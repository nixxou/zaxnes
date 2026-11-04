// The second pass of AMD-060 REQ-SCN-085: the blending weight of every pixel the first pass marked.
// `SMAABlendingWeightCalculationPS` of `third_party/smaa/SMAA.hlsl` (MIT), with its diagonal
// search, its horizontal and vertical searches through the two precomputed tables, and its corner
// patterns — the whole of it, at the reference's own High preset.
//
// Read, not transcribed. Where a line here differs from the reference's, it is for one of three
// reasons and the comment at that line says which:
//
//  - **the y axis**: `uRtMetrics.y` is negative here (`smaa.glsl` point 1), so the reference's own
//    `± offset · SMAA_RT_METRICS.y` arithmetic means the same thing it means in Direct3D, and every
//    texel offset is written as that multiply-add rather than as an integer `textureLodOffset`;
//  - **`out` parameters**: GLSL has them, so `SMAASearchDiag1/2` keep theirs;
//  - **`SMAAMovc`**: the reference's conditional move is `mix(a, b, bvec)` here, which is one
//    instruction and says the same thing.
//
// Output: `rg` the horizontal pair of areas and `ba` the vertical one, as the third pass reads
// them. Every pixel is written — the pass does not `discard` — so the target needs no clear.

in vec2 vUv;
out vec4 oWeights;

// The edges of the first pass, `r` west and `g` north, read with **bilinear** filtering: the
// searches below fetch four edges at a time through it, which is the reference's
// `@PSEUDO_GATHER4`, and a `NEAREST` edge texture would silently break them.
uniform sampler2D uEdges;

// The two precomputed tables of the reference, taken to the byte (`src/assets/smaa/`). They are
// indexed in their **own** coordinate space, never through `uRtMetrics`.
uniform sampler2D uArea;
uniform sampler2D uSearch;

// The reference's High preset (`SMAA_PRESET_HIGH`): 16 steps for the horizontal and vertical
// patterns, 8 for the diagonals, corners rounded at 25 %. The threshold is not here — this pass
// does not threshold anything; the first one did.
//
// High rather than Ultra (32 and 16) because the preset is a cost, the mode's whole argument is
// that it costs one sample where MSAA 4x costs four, and 16 steps handle a line 64 pixels long
// perfectly. Ultra is for a resolution far above what a 1080p frame of 256 x 240 voxels shows.
const int ZAX_SMAA_MAX_SEARCH_STEPS = 16;
const int ZAX_SMAA_MAX_SEARCH_STEPS_DIAG = 8;
const float ZAX_SMAA_CORNER_ROUNDING_NORM = 0.25;

// `SMAA_AREATEX_MAX_DISTANCE`, `..._DIAG`, `SMAA_AREATEX_PIXEL_SIZE` and `..._SUBTEX_SIZE` of the
// reference, and the two sizes of the search table — the second being the **cropped** shape the
// header ships (`SMAA_SEARCHTEX_PACKED_SIZE`), which is why the two differ.
const vec2 ZAX_AREATEX_MAX_DISTANCE = vec2(16.0, 16.0);
const vec2 ZAX_AREATEX_MAX_DISTANCE_DIAG = vec2(20.0, 20.0);
const vec2 ZAX_AREATEX_PIXEL_SIZE = vec2(1.0 / 160.0, 1.0 / 560.0);
const float ZAX_AREATEX_SUBTEX_SIZE = 1.0 / 7.0;
const vec2 ZAX_SEARCHTEX_SIZE = vec2(66.0, 33.0);
const vec2 ZAX_SEARCHTEX_PACKED_SIZE = vec2(64.0, 16.0);

// `SMAADecodeDiagBilinearAccess`: two binary values out of one bilinear read. The reference's own
// comment explains the constants — a bilinear fetch of `e` carries a 0,25 offset, so an enabled
// red edge reads 0,25 or 1 and an enabled green one 0,75 or 1, and `x · |5x − 3,75|` then rounds
// to the pair.
vec2 zaxSmaaDecodeDiag2(vec2 e)
{
    e.r = e.r * abs((5.0 * e.r) - (5.0 * 0.75));
    return round(e);
}

vec4 zaxSmaaDecodeDiag4(vec4 e)
{
    e.rb = e.rb * abs((5.0 * e.rb) - (5.0 * 0.75));
    return round(e);
}

// `SMAASearchDiag1`: walks the diagonal one pixel at a time until the line ends.
vec2 zaxSmaaSearchDiag1(vec2 texcoord, vec2 dir, out vec2 e)
{
    vec4 coord = vec4(texcoord, -1.0, 1.0);
    vec3 t = vec3(uRtMetrics.xy, 1.0);
    while (coord.z < float(ZAX_SMAA_MAX_SEARCH_STEPS_DIAG - 1) && coord.w > 0.9)
    {
        coord.xyz = (t * vec3(dir, 1.0)) + coord.xyz;
        e = zaxSmaaFetch(uEdges, coord.xy).rg;
        coord.w = dot(e, vec2(0.5, 0.5));
    }
    return coord.zw;
}

// `SMAASearchDiag2`, with the reference's `@SearchDiag2Optimization`: both edges at once through
// one bilinear read, decoded by [`zaxSmaaDecodeDiag2`].
vec2 zaxSmaaSearchDiag2(vec2 texcoord, vec2 dir, out vec2 e)
{
    vec4 coord = vec4(texcoord, -1.0, 1.0);
    coord.x += 0.25 * uRtMetrics.x;
    vec3 t = vec3(uRtMetrics.xy, 1.0);
    while (coord.z < float(ZAX_SMAA_MAX_SEARCH_STEPS_DIAG - 1) && coord.w > 0.9)
    {
        coord.xyz = (t * vec3(dir, 1.0)) + coord.xyz;
        e = zaxSmaaDecodeDiag2(zaxSmaaFetch(uEdges, coord.xy).rg);
        coord.w = dot(e, vec2(0.5, 0.5));
    }
    return coord.zw;
}

// `SMAAAreaDiag`: the area of a diagonal line of distances `dist` with crossing edges `e`. The
// diagonal half of the table is its right half, hence the `+ 0,5`.
vec2 zaxSmaaAreaDiag(vec2 dist, vec2 e, float offset)
{
    vec2 texcoord = (ZAX_AREATEX_MAX_DISTANCE_DIAG * e) + dist;
    texcoord = (ZAX_AREATEX_PIXEL_SIZE * texcoord) + (0.5 * ZAX_AREATEX_PIXEL_SIZE);
    texcoord.x += 0.5;
    texcoord.y += ZAX_AREATEX_SUBTEX_SIZE * offset;
    return texture(uArea, texcoord).rg;
}

// `SMAACalculateDiagWeights`: the diagonal patterns, which have both a north and a west edge, so
// looking for them on one boundary is enough. `subsampleIndices` is zero throughout — SMAA 1x,
// which is what REQ-SCN-085 asks for ("les trois passes du SMAA 1x").
vec2 zaxSmaaDiagWeights(vec2 texcoord, vec2 e)
{
    vec2 weights = vec2(0.0, 0.0);

    vec4 d;
    vec2 end;
    if (e.r > 0.0)
    {
        d.xz = zaxSmaaSearchDiag1(texcoord, vec2(-1.0, 1.0), end);
        d.x += float(end.y > 0.9);
    }
    else
    {
        d.xz = vec2(0.0, 0.0);
    }
    d.yw = zaxSmaaSearchDiag1(texcoord, vec2(1.0, -1.0), end);

    if ((d.x + d.y) > 2.0)
    {
        vec4 coords = (vec4(-d.x + 0.25, d.x, d.y, -d.y - 0.25) * uRtMetrics.xyxy) + texcoord.xyxy;
        vec4 c;
        c.xy = zaxSmaaFetchOffset(uEdges, coords.xy, vec2(-1.0, 0.0)).rg;
        c.zw = zaxSmaaFetchOffset(uEdges, coords.zw, vec2(1.0, 0.0)).rg;
        c.yxwz = zaxSmaaDecodeDiag4(c.xyzw);
        vec2 cc = (vec2(2.0, 2.0) * c.xz) + c.yw;
        // `SMAAMovc(bool2(step(0.9, d.zw)), cc, 0)`: the crossing edge goes where the line's end
        // was not found.
        cc = mix(cc, vec2(0.0, 0.0), greaterThanEqual(d.zw, vec2(0.9, 0.9)));
        weights += zaxSmaaAreaDiag(d.xy, cc, 0.0);
    }

    d.xz = zaxSmaaSearchDiag2(texcoord, vec2(-1.0, -1.0), end);
    if (zaxSmaaFetchOffset(uEdges, texcoord, vec2(1.0, 0.0)).r > 0.0)
    {
        d.yw = zaxSmaaSearchDiag2(texcoord, vec2(1.0, 1.0), end);
        d.y += float(end.y > 0.9);
    }
    else
    {
        d.yw = vec2(0.0, 0.0);
    }

    if ((d.x + d.y) > 2.0)
    {
        vec4 coords = (vec4(-d.x, -d.x, d.y, d.y) * uRtMetrics.xyxy) + texcoord.xyxy;
        vec4 c;
        c.x = zaxSmaaFetchOffset(uEdges, coords.xy, vec2(-1.0, 0.0)).g;
        c.y = zaxSmaaFetchOffset(uEdges, coords.xy, vec2(0.0, -1.0)).r;
        c.zw = zaxSmaaFetchOffset(uEdges, coords.zw, vec2(1.0, 0.0)).gr;
        vec2 cc = (vec2(2.0, 2.0) * c.xz) + c.yw;
        cc = mix(cc, vec2(0.0, 0.0), greaterThanEqual(d.zw, vec2(0.9, 0.9)));
        weights += zaxSmaaAreaDiag(d.xy, cc, 0.0).gr;
    }

    return weights;
}

// `SMAASearchLength`: how much length the last step of a search adds, 0, 1 or 2, read off the
// search table. The table is stored flipped vertically and cropped, which the two scales below
// undo — that flip is the **table's** own encoding and has nothing to do with the host's texture
// origin, so it is kept exactly as the reference writes it.
float zaxSmaaSearchLength(vec2 e, float offset)
{
    vec2 scale = ZAX_SEARCHTEX_SIZE * vec2(0.5, -1.0);
    vec2 bias = ZAX_SEARCHTEX_SIZE * vec2(offset, 1.0);
    scale += vec2(-1.0, 1.0);
    bias += vec2(0.5, -0.5);
    scale *= 1.0 / ZAX_SEARCHTEX_PACKED_SIZE;
    bias *= 1.0 / ZAX_SEARCHTEX_PACKED_SIZE;
    return texture(uSearch, (scale * e) + bias).r;
}

// `SMAASearchXLeft` / `XRight` / `YUp` / `YDown`: the four searches of the second pass. Each walks
// two pixels at a time over the bilinearly-fetched edges until some edge is not activated
// (`e.g > 0.8281`) or a crossing edge breaks the line (`e.r == 0`).
float zaxSmaaSearchXLeft(vec2 texcoord, float end)
{
    vec2 e = vec2(0.0, 1.0);
    while (texcoord.x > end && e.g > 0.8281 && e.r == 0.0)
    {
        e = zaxSmaaFetch(uEdges, texcoord).rg;
        texcoord = (-vec2(2.0, 0.0) * uRtMetrics.xy) + texcoord;
    }
    float offset = (-(255.0 / 127.0) * zaxSmaaSearchLength(e, 0.0)) + 3.25;
    return (uRtMetrics.x * offset) + texcoord.x;
}

float zaxSmaaSearchXRight(vec2 texcoord, float end)
{
    vec2 e = vec2(0.0, 1.0);
    while (texcoord.x < end && e.g > 0.8281 && e.r == 0.0)
    {
        e = zaxSmaaFetch(uEdges, texcoord).rg;
        texcoord = (vec2(2.0, 0.0) * uRtMetrics.xy) + texcoord;
    }
    float offset = (-(255.0 / 127.0) * zaxSmaaSearchLength(e, 0.5)) + 3.25;
    return (-uRtMetrics.x * offset) + texcoord.x;
}

// "Up" and "down" are **screen** directions, and they come out right because `uRtMetrics.y` is
// negative: the reference's `texcoord − (0, 2) · metrics.xy` is `+2/h` here, which in an OpenGL
// texture is up on screen. The `end` bounds the caller passes carry the same sign.
float zaxSmaaSearchYUp(vec2 texcoord, float end)
{
    vec2 e = vec2(1.0, 0.0);
    while (texcoord.y > end && e.r > 0.8281 && e.g == 0.0)
    {
        e = zaxSmaaFetch(uEdges, texcoord).rg;
        texcoord = (-vec2(0.0, 2.0) * uRtMetrics.xy) + texcoord;
    }
    float offset = (-(255.0 / 127.0) * zaxSmaaSearchLength(e.gr, 0.0)) + 3.25;
    return (uRtMetrics.y * offset) + texcoord.y;
}

float zaxSmaaSearchYDown(vec2 texcoord, float end)
{
    vec2 e = vec2(1.0, 0.0);
    while (texcoord.y < end && e.r > 0.8281 && e.g == 0.0)
    {
        e = zaxSmaaFetch(uEdges, texcoord).rg;
        texcoord = (vec2(0.0, 2.0) * uRtMetrics.xy) + texcoord;
    }
    float offset = (-(255.0 / 127.0) * zaxSmaaSearchLength(e.gr, 0.5)) + 3.25;
    return (-uRtMetrics.y * offset) + texcoord.y;
}

// `SMAAArea`: the areas on either side of the edge, for a distance pair and two crossing edges.
// The `round(4 · e)` is the reference's own guard against the bilinear filter's rounding.
vec2 zaxSmaaArea(vec2 dist, float e1, float e2, float offset)
{
    vec2 texcoord = (ZAX_AREATEX_MAX_DISTANCE * round(4.0 * vec2(e1, e2))) + dist;
    texcoord = (ZAX_AREATEX_PIXEL_SIZE * texcoord) + (0.5 * ZAX_AREATEX_PIXEL_SIZE);
    texcoord.y = (ZAX_AREATEX_SUBTEX_SIZE * offset) + texcoord.y;
    return texture(uArea, texcoord).rg;
}

// `SMAADetectHorizontalCornerPattern` and its vertical twin: how much a sharp corner is rounded.
void zaxSmaaHorizontalCorner(inout vec2 weights, vec4 texcoord, vec2 d)
{
    vec2 leftRight = step(d.xy, d.yx);
    vec2 rounding = (1.0 - ZAX_SMAA_CORNER_ROUNDING_NORM) * leftRight;
    rounding /= leftRight.x + leftRight.y;

    vec2 factor = vec2(1.0, 1.0);
    factor.x -= rounding.x * zaxSmaaFetchOffset(uEdges, texcoord.xy, vec2(0.0, 1.0)).r;
    factor.x -= rounding.y * zaxSmaaFetchOffset(uEdges, texcoord.zw, vec2(1.0, 1.0)).r;
    factor.y -= rounding.x * zaxSmaaFetchOffset(uEdges, texcoord.xy, vec2(0.0, -2.0)).r;
    factor.y -= rounding.y * zaxSmaaFetchOffset(uEdges, texcoord.zw, vec2(1.0, -2.0)).r;

    weights *= clamp(factor, 0.0, 1.0);
}

void zaxSmaaVerticalCorner(inout vec2 weights, vec4 texcoord, vec2 d)
{
    vec2 leftRight = step(d.xy, d.yx);
    vec2 rounding = (1.0 - ZAX_SMAA_CORNER_ROUNDING_NORM) * leftRight;
    rounding /= leftRight.x + leftRight.y;

    vec2 factor = vec2(1.0, 1.0);
    factor.x -= rounding.x * zaxSmaaFetchOffset(uEdges, texcoord.xy, vec2(1.0, 0.0)).g;
    factor.x -= rounding.y * zaxSmaaFetchOffset(uEdges, texcoord.zw, vec2(1.0, 1.0)).g;
    factor.y -= rounding.x * zaxSmaaFetchOffset(uEdges, texcoord.xy, vec2(-2.0, 0.0)).g;
    factor.y -= rounding.y * zaxSmaaFetchOffset(uEdges, texcoord.zw, vec2(-2.0, 1.0)).g;

    weights *= clamp(factor, 0.0, 1.0);
}

void main()
{
    vec4 weights = vec4(0.0, 0.0, 0.0, 0.0);

    // `SMAABlendingWeightCalculationVS`'s three offsets and its `pixcoord`, computed here rather
    // than interpolated (`smaa.glsl` point 2).
    vec2 pixcoord = vUv * uRtMetrics.zw;
    vec4 offset0 = (uRtMetrics.xyxy * vec4(-0.25, -0.125, 1.25, -0.125)) + vUv.xyxy;
    vec4 offset1 = (uRtMetrics.xyxy * vec4(-0.125, -0.25, -0.125, 1.25)) + vUv.xyxy;
    vec4 offset2 = (uRtMetrics.xxyy * (vec4(-2.0, 2.0, -2.0, 2.0) * float(ZAX_SMAA_MAX_SEARCH_STEPS)))
        + vec4(offset0.xz, offset1.yw);

    vec2 e = zaxSmaaFetch(uEdges, vUv).rg;

    if (e.g > 0.0)
    {
        weights.rg = zaxSmaaDiagWeights(vUv, e);

        if (weights.r == -weights.g)
        {
            vec2 d;
            vec3 coords;
            coords.x = zaxSmaaSearchXLeft(offset0.xy, offset2.x);
            coords.y = offset1.y;
            d.x = coords.x;

            float e1 = zaxSmaaFetch(uEdges, coords.xy).r;

            coords.z = zaxSmaaSearchXRight(offset0.zw, offset2.y);
            d.y = coords.z;

            d = abs(round((uRtMetrics.zz * d) - pixcoord.xx));
            vec2 sqrtD = sqrt(d);

            float e2 = zaxSmaaFetchOffset(uEdges, coords.zy, vec2(1.0, 0.0)).r;

            weights.rg = zaxSmaaArea(sqrtD, e1, e2, 0.0);

            coords.y = vUv.y;
            zaxSmaaHorizontalCorner(weights.rg, coords.xyzy, d);
        }
        else
        {
            e.r = 0.0;
        }
    }

    if (e.r > 0.0)
    {
        vec2 d;
        vec3 coords;
        coords.y = zaxSmaaSearchYUp(offset1.xy, offset2.z);
        coords.x = offset0.x;
        d.x = coords.y;

        float e1 = zaxSmaaFetch(uEdges, coords.xy).g;

        coords.z = zaxSmaaSearchYDown(offset1.zw, offset2.w);
        d.y = coords.z;

        // The reference's `d = abs(round(mad(RT.ww, d, -pixcoord.yy)))`. `uRtMetrics.w` is the
        // height and **positive** — the mirror lives in `.y`, not in `.w` — so the two distances
        // come out in pixels here exactly as they do there, and `abs` makes their order immaterial.
        d = abs(round((uRtMetrics.ww * d) - pixcoord.yy));
        vec2 sqrtD = sqrt(d);

        float e2 = zaxSmaaFetchOffset(uEdges, coords.xz, vec2(0.0, 1.0)).g;

        weights.ba = zaxSmaaArea(sqrtD, e1, e2, 0.0);

        coords.x = vUv.x;
        zaxSmaaVerticalCorner(weights.ba, coords.xyxz, d);
    }

    oWeights = weights;
}
