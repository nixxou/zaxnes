// The second pass of AMD-060 REQ-SCN-085 on this backend: the blending weight of every pixel the
// first pass marked. The WGSL twin of shaders/smaa_weights.frag — the whole of
// `SMAABlendingWeightCalculationPS` of `third_party/smaa/SMAA.hlsl` (MIT) at the reference's own
// **High** preset: the diagonal search and its bilinear decode, the four horizontal and vertical
// searches through the search table, `SMAAArea` and `SMAAAreaDiag` through the area table, and the
// two corner patterns.
//
// Read against the reference and against the GLSL, not transcribed from either. Where a line
// differs from the GLSL's it is for one of three reasons, and all three are this language's:
//
//  - **no `out` parameters**: the two diagonal searches hand back a struct instead;
//  - **no swizzle assignment**: `c.yxwz = decode(c)` and `weights.rg = ...` become explicit
//    components, which is also why the two weight pairs are two `vec2` here rather than one
//    `vec4`;
//  - **no `inout`**: the corner patterns take the weights and return them multiplied.
//
// The arithmetic is identical, including the one thing that matters most: every texel offset is
// the multiply-add `uv + texels · metrics.xy`, so the sign of `metrics.y` carries (`smaa.wgsl`).
//
// Output: `rg` the horizontal pair of areas and `ba` the vertical one. Every pixel is written —
// the pass does not `discard` — so its target needs no clear.

// The reference's High preset (`SMAA_PRESET_HIGH`): 16 steps for the horizontal and vertical
// patterns, 8 for the diagonals, corners rounded at 25 %. The same preset the OpenGL path uses,
// and for the reason that file gives: the preset is a cost, the mode's whole argument is that it
// costs one sample where MSAA 4x costs four, and 16 steps handle a line 64 pixels long perfectly.
const ZAX_SMAA_MAX_SEARCH_STEPS: i32 = 16;
const ZAX_SMAA_MAX_SEARCH_STEPS_DIAG: i32 = 8;
const ZAX_SMAA_CORNER_ROUNDING_NORM: f32 = 0.25;

// `SMAA_AREATEX_MAX_DISTANCE`, `..._DIAG`, `SMAA_AREATEX_PIXEL_SIZE` and `..._SUBTEX_SIZE` of the
// reference, and the two sizes of the search table — the second being the **cropped** shape the
// header ships (`SMAA_SEARCHTEX_PACKED_SIZE`), which is why the two differ.
const ZAX_AREATEX_MAX_DISTANCE: vec2<f32> = vec2<f32>(16.0, 16.0);
const ZAX_AREATEX_MAX_DISTANCE_DIAG: vec2<f32> = vec2<f32>(20.0, 20.0);
const ZAX_AREATEX_PIXEL_SIZE: vec2<f32> = vec2<f32>(1.0 / 160.0, 1.0 / 560.0);
const ZAX_AREATEX_SUBTEX_SIZE: f32 = 1.0 / 7.0;
const ZAX_SEARCHTEX_SIZE: vec2<f32> = vec2<f32>(66.0, 33.0);
const ZAX_SEARCHTEX_PACKED_SIZE: vec2<f32> = vec2<f32>(64.0, 16.0);

// What a diagonal search hands back: the reference's return value and its `out` edge pair.
struct ZaxSmaaDiagonal {
    d: vec2<f32>,
    e: vec2<f32>,
}

// `SMAADecodeDiagBilinearAccess`: two binary values out of one bilinear read. The reference's own
// comment explains the constants — a bilinear fetch of `e` carries a 0,25 offset, so an enabled
// red edge reads 0,25 or 1 and an enabled green one 0,75 or 1, and `x · |5x − 3,75|` then rounds
// to the pair.
fn zax_smaa_decode_diag2(e: vec2<f32>) -> vec2<f32> {
    let red = e.r * abs((5.0 * e.r) - (5.0 * 0.75));
    return round(vec2<f32>(red, e.g));
}

fn zax_smaa_decode_diag4(e: vec4<f32>) -> vec4<f32> {
    let rb = e.rb * abs((5.0 * e.rb) - vec2<f32>(5.0 * 0.75));
    return round(vec4<f32>(rb.x, e.g, rb.y, e.a));
}

// `SMAASearchDiag1`: walks the diagonal one pixel at a time until the line ends.
fn zax_smaa_search_diag1(texcoord: vec2<f32>, dir: vec2<f32>) -> ZaxSmaaDiagonal {
    var coord = vec4<f32>(texcoord, -1.0, 1.0);
    var e = vec2<f32>(0.0, 0.0);
    let t = vec3<f32>(smaa.metrics.xy, 1.0);
    while (coord.z < f32(ZAX_SMAA_MAX_SEARCH_STEPS_DIAG - 1) && coord.w > 0.9) {
        let next = (t * vec3<f32>(dir, 1.0)) + coord.xyz;
        coord.x = next.x;
        coord.y = next.y;
        coord.z = next.z;
        e = zax_smaa_edges(coord.xy);
        coord.w = dot(e, vec2<f32>(0.5, 0.5));
    }
    var out: ZaxSmaaDiagonal;
    out.d = coord.zw;
    out.e = e;
    return out;
}

// `SMAASearchDiag2`, with the reference's `@SearchDiag2Optimization`: both edges at once through
// one bilinear read, decoded by [`zax_smaa_decode_diag2`].
fn zax_smaa_search_diag2(texcoord: vec2<f32>, dir: vec2<f32>) -> ZaxSmaaDiagonal {
    var coord = vec4<f32>(texcoord, -1.0, 1.0);
    coord.x = coord.x + (0.25 * smaa.metrics.x);
    var e = vec2<f32>(0.0, 0.0);
    let t = vec3<f32>(smaa.metrics.xy, 1.0);
    while (coord.z < f32(ZAX_SMAA_MAX_SEARCH_STEPS_DIAG - 1) && coord.w > 0.9) {
        let next = (t * vec3<f32>(dir, 1.0)) + coord.xyz;
        coord.x = next.x;
        coord.y = next.y;
        coord.z = next.z;
        e = zax_smaa_decode_diag2(zax_smaa_edges(coord.xy));
        coord.w = dot(e, vec2<f32>(0.5, 0.5));
    }
    var out: ZaxSmaaDiagonal;
    out.d = coord.zw;
    out.e = e;
    return out;
}

// `SMAAAreaDiag`: the area of a diagonal line of distances `dist` with crossing edges `e`. The
// diagonal half of the table is its right half, hence the `+ 0,5`.
fn zax_smaa_area_diag(dist: vec2<f32>, e: vec2<f32>, offset: f32) -> vec2<f32> {
    var texcoord = (ZAX_AREATEX_MAX_DISTANCE_DIAG * e) + dist;
    texcoord = (ZAX_AREATEX_PIXEL_SIZE * texcoord) + (0.5 * ZAX_AREATEX_PIXEL_SIZE);
    texcoord.x = texcoord.x + 0.5;
    texcoord.y = texcoord.y + (ZAX_AREATEX_SUBTEX_SIZE * offset);
    return zax_smaa_table_area(texcoord);
}

// `SMAACalculateDiagWeights`: the diagonal patterns, which have both a north and a west edge, so
// looking for them on one boundary is enough. `subsampleIndices` is zero throughout — SMAA 1x,
// which is what REQ-SCN-085 asks for ("les trois passes du SMAA 1x").
fn zax_smaa_diag_weights(texcoord: vec2<f32>, e: vec2<f32>) -> vec2<f32> {
    var weights = vec2<f32>(0.0, 0.0);
    var d = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    if (e.r > 0.0) {
        let found = zax_smaa_search_diag1(texcoord, vec2<f32>(-1.0, 1.0));
        d.x = found.d.x + select(0.0, 1.0, found.e.y > 0.9);
        d.z = found.d.y;
    }
    let down = zax_smaa_search_diag1(texcoord, vec2<f32>(1.0, -1.0));
    d.y = down.d.x;
    d.w = down.d.y;

    if ((d.x + d.y) > 2.0) {
        let coords = (vec4<f32>(-d.x + 0.25, d.x, d.y, -d.y - 0.25) * smaa.metrics.xyxy)
            + vec4<f32>(texcoord, texcoord);
        let left = zax_smaa_edges_offset(coords.xy, vec2<f32>(-1.0, 0.0));
        let right = zax_smaa_edges_offset(coords.zw, vec2<f32>(1.0, 0.0));
        // The reference's `c.yxwz = SMAADecodeDiagBilinearAccess(c.xyzw)`, with the crossing pairs
        // put back where it puts them.
        let decoded = zax_smaa_decode_diag4(vec4<f32>(left, right));
        let c = vec4<f32>(decoded.y, decoded.x, decoded.w, decoded.z);
        var cc = (vec2<f32>(2.0, 2.0) * vec2<f32>(c.x, c.z)) + vec2<f32>(c.y, c.w);
        // `SMAAMovc(bool2(step(0.9, d.zw)), cc, 0)`: the crossing edge goes where the line's end
        // was not found.
        cc = select(cc, vec2<f32>(0.0, 0.0), d.zw >= vec2<f32>(0.9, 0.9));
        weights = weights + zax_smaa_area_diag(d.xy, cc, 0.0);
    }

    let up = zax_smaa_search_diag2(texcoord, vec2<f32>(-1.0, -1.0));
    d.x = up.d.x;
    d.z = up.d.y;
    d.y = 0.0;
    d.w = 0.0;
    if (zax_smaa_edges_offset(texcoord, vec2<f32>(1.0, 0.0)).r > 0.0) {
        let other = zax_smaa_search_diag2(texcoord, vec2<f32>(1.0, 1.0));
        d.y = other.d.x + select(0.0, 1.0, other.e.y > 0.9);
        d.w = other.d.y;
    }

    if ((d.x + d.y) > 2.0) {
        let coords = (vec4<f32>(-d.x, -d.x, d.y, d.y) * smaa.metrics.xyxy)
            + vec4<f32>(texcoord, texcoord);
        let west = zax_smaa_edges_offset(coords.xy, vec2<f32>(-1.0, 0.0)).g;
        let north = zax_smaa_edges_offset(coords.xy, vec2<f32>(0.0, -1.0)).r;
        let far = zax_smaa_edges_offset(coords.zw, vec2<f32>(1.0, 0.0));
        let c = vec4<f32>(west, north, far.g, far.r);
        var cc = (vec2<f32>(2.0, 2.0) * vec2<f32>(c.x, c.z)) + vec2<f32>(c.y, c.w);
        cc = select(cc, vec2<f32>(0.0, 0.0), d.zw >= vec2<f32>(0.9, 0.9));
        let area = zax_smaa_area_diag(d.xy, cc, 0.0);
        weights = weights + vec2<f32>(area.y, area.x);
    }

    return weights;
}

// `SMAASearchLength`: how much length the last step of a search adds, 0, 1 or 2, read off the
// search table. The table is stored flipped vertically and cropped, which the two scales below
// undo — that flip is the **table's** own encoding and has nothing to do with the host's texture
// origin, so it is kept exactly as the reference writes it, on both backends.
fn zax_smaa_search_length(e: vec2<f32>, offset: f32) -> f32 {
    var scale = ZAX_SEARCHTEX_SIZE * vec2<f32>(0.5, -1.0);
    var bias = ZAX_SEARCHTEX_SIZE * vec2<f32>(offset, 1.0);
    scale = scale + vec2<f32>(-1.0, 1.0);
    bias = bias + vec2<f32>(0.5, -0.5);
    scale = scale * (vec2<f32>(1.0, 1.0) / ZAX_SEARCHTEX_PACKED_SIZE);
    bias = bias * (vec2<f32>(1.0, 1.0) / ZAX_SEARCHTEX_PACKED_SIZE);
    return zax_smaa_table_search((scale * e) + bias);
}

// `SMAASearchXLeft` / `XRight` / `YUp` / `YDown`: the four searches of the second pass. Each walks
// two pixels at a time over the bilinearly-fetched edges until some edge is not activated
// (`e.g > 0.8281`) or a crossing edge breaks the line (`e.r == 0`).
fn zax_smaa_search_x_left(start: vec2<f32>, end: f32) -> f32 {
    var texcoord = start;
    var e = vec2<f32>(0.0, 1.0);
    while (texcoord.x > end && e.g > 0.8281 && e.r == 0.0) {
        e = zax_smaa_edges(texcoord);
        texcoord = (-vec2<f32>(2.0, 0.0) * smaa.metrics.xy) + texcoord;
    }
    let offset = (-(255.0 / 127.0) * zax_smaa_search_length(e, 0.0)) + 3.25;
    return (smaa.metrics.x * offset) + texcoord.x;
}

fn zax_smaa_search_x_right(start: vec2<f32>, end: f32) -> f32 {
    var texcoord = start;
    var e = vec2<f32>(0.0, 1.0);
    while (texcoord.x < end && e.g > 0.8281 && e.r == 0.0) {
        e = zax_smaa_edges(texcoord);
        texcoord = (vec2<f32>(2.0, 0.0) * smaa.metrics.xy) + texcoord;
    }
    let offset = (-(255.0 / 127.0) * zax_smaa_search_length(e, 0.5)) + 3.25;
    return (-smaa.metrics.x * offset) + texcoord.x;
}

// "Up" and "down" are **screen** directions, and they come out right because `metrics.y` has the
// sign of the host's texture origin: here it is positive and a wgpu framebuffer's rows run
// downward, so the reference's `texcoord − (0, 2) · metrics.xy` is up on screen, exactly as the
// negative `y` makes it on the OpenGL side. The `end` bounds the caller passes carry the same
// sign.
//
// **And the comparison against them is written multiplied through by `metrics.y`**, which here is
// the reference's own condition — `metrics.y` being positive — and on the OpenGL side is the
// mirrored one. It is the one place the sign does not carry itself: the walk and the bound both
// flip, the operator `>` does not, and the GLSL twin's two searches did nothing at all until this
// was found by comparing the two backends' pictures (`smaa_weights.frag` carries the
// measurement). Written the same way in both sources so that neither can drift back.
fn zax_smaa_search_y_up(start: vec2<f32>, end: f32) -> f32 {
    var texcoord = start;
    var e = vec2<f32>(1.0, 0.0);
    while (((texcoord.y - end) * smaa.metrics.y) > 0.0 && e.r > 0.8281 && e.g == 0.0) {
        e = zax_smaa_edges(texcoord);
        texcoord = (-vec2<f32>(0.0, 2.0) * smaa.metrics.xy) + texcoord;
    }
    let offset = (-(255.0 / 127.0) * zax_smaa_search_length(vec2<f32>(e.g, e.r), 0.0)) + 3.25;
    return (smaa.metrics.y * offset) + texcoord.y;
}

fn zax_smaa_search_y_down(start: vec2<f32>, end: f32) -> f32 {
    var texcoord = start;
    var e = vec2<f32>(1.0, 0.0);
    while (((texcoord.y - end) * smaa.metrics.y) < 0.0 && e.r > 0.8281 && e.g == 0.0) {
        e = zax_smaa_edges(texcoord);
        texcoord = (vec2<f32>(0.0, 2.0) * smaa.metrics.xy) + texcoord;
    }
    let offset = (-(255.0 / 127.0) * zax_smaa_search_length(vec2<f32>(e.g, e.r), 0.5)) + 3.25;
    return (-smaa.metrics.y * offset) + texcoord.y;
}

// `SMAAArea`: the areas on either side of the edge, for a distance pair and two crossing edges.
// The `round(4 · e)` is the reference's own guard against the bilinear filter's rounding.
fn zax_smaa_area(dist: vec2<f32>, e1: f32, e2: f32, offset: f32) -> vec2<f32> {
    var texcoord = (ZAX_AREATEX_MAX_DISTANCE * round(4.0 * vec2<f32>(e1, e2))) + dist;
    texcoord = (ZAX_AREATEX_PIXEL_SIZE * texcoord) + (0.5 * ZAX_AREATEX_PIXEL_SIZE);
    texcoord.y = (ZAX_AREATEX_SUBTEX_SIZE * offset) + texcoord.y;
    return zax_smaa_table_area(texcoord);
}

// `SMAADetectHorizontalCornerPattern`: how much a sharp corner is rounded. The reference's `inout`
// is a value in and a value out here.
fn zax_smaa_horizontal_corner(weights: vec2<f32>, texcoord: vec4<f32>, d: vec2<f32>) -> vec2<f32> {
    let left_right = step(d.xy, vec2<f32>(d.y, d.x));
    var rounding = (1.0 - ZAX_SMAA_CORNER_ROUNDING_NORM) * left_right;
    rounding = rounding / (left_right.x + left_right.y);

    var factor = vec2<f32>(1.0, 1.0);
    factor.x = factor.x
        - (rounding.x * zax_smaa_edges_offset(texcoord.xy, vec2<f32>(0.0, 1.0)).r);
    factor.x = factor.x
        - (rounding.y * zax_smaa_edges_offset(texcoord.zw, vec2<f32>(1.0, 1.0)).r);
    factor.y = factor.y
        - (rounding.x * zax_smaa_edges_offset(texcoord.xy, vec2<f32>(0.0, -2.0)).r);
    factor.y = factor.y
        - (rounding.y * zax_smaa_edges_offset(texcoord.zw, vec2<f32>(1.0, -2.0)).r);

    return weights * clamp(factor, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
}

// And its vertical twin.
fn zax_smaa_vertical_corner(weights: vec2<f32>, texcoord: vec4<f32>, d: vec2<f32>) -> vec2<f32> {
    let left_right = step(d.xy, vec2<f32>(d.y, d.x));
    var rounding = (1.0 - ZAX_SMAA_CORNER_ROUNDING_NORM) * left_right;
    rounding = rounding / (left_right.x + left_right.y);

    var factor = vec2<f32>(1.0, 1.0);
    factor.x = factor.x
        - (rounding.x * zax_smaa_edges_offset(texcoord.xy, vec2<f32>(1.0, 0.0)).g);
    factor.x = factor.x
        - (rounding.y * zax_smaa_edges_offset(texcoord.zw, vec2<f32>(1.0, 1.0)).g);
    factor.y = factor.y
        - (rounding.x * zax_smaa_edges_offset(texcoord.xy, vec2<f32>(-2.0, 0.0)).g);
    factor.y = factor.y
        - (rounding.y * zax_smaa_edges_offset(texcoord.zw, vec2<f32>(-2.0, 1.0)).g);

    return weights * clamp(factor, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
}

@fragment
fn fs_main(in: SmaaVertex) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var horizontal = vec2<f32>(0.0, 0.0);
    var vertical = vec2<f32>(0.0, 0.0);

    // `SMAABlendingWeightCalculationVS`'s three offsets and its `pixcoord`, computed here rather
    // than interpolated — this backend's screen passes share one attribute-less vertex stage, as
    // the OpenGL ones do (`smaa.glsl` point 2).
    let pixcoord = uv * smaa.metrics.zw;
    let offset0 = (smaa.metrics.xyxy * vec4<f32>(-0.25, -0.125, 1.25, -0.125))
        + vec4<f32>(uv, uv);
    let offset1 = (smaa.metrics.xyxy * vec4<f32>(-0.125, -0.25, -0.125, 1.25))
        + vec4<f32>(uv, uv);
    let offset2 = (smaa.metrics.xxyy
        * (vec4<f32>(-2.0, 2.0, -2.0, 2.0) * f32(ZAX_SMAA_MAX_SEARCH_STEPS)))
        + vec4<f32>(offset0.x, offset0.z, offset1.y, offset1.w);

    let e = zax_smaa_edges(uv);
    var west = e.r;

    if (e.g > 0.0) {
        horizontal = zax_smaa_diag_weights(uv, e);

        // The reference's own test for "no diagonal was found": both weights are zero, and both
        // are non-negative.
        if (horizontal.x == -horizontal.y) {
            var d = vec2<f32>(0.0, 0.0);
            let left = zax_smaa_search_x_left(offset0.xy, offset2.x);
            d.x = left;
            let e1 = zax_smaa_edges(vec2<f32>(left, offset1.y)).r;

            let right = zax_smaa_search_x_right(offset0.zw, offset2.y);
            d.y = right;

            d = abs(round((smaa.metrics.zz * d) - pixcoord.xx));
            let sqrt_d = sqrt(d);

            let e2 = zax_smaa_edges_offset(
                vec2<f32>(right, offset1.y),
                vec2<f32>(1.0, 0.0),
            ).r;

            horizontal = zax_smaa_area(sqrt_d, e1, e2, 0.0);
            horizontal = zax_smaa_horizontal_corner(
                horizontal,
                vec4<f32>(left, uv.y, right, uv.y),
                d,
            );
        } else {
            west = 0.0;
        }
    }

    if (west > 0.0) {
        var d = vec2<f32>(0.0, 0.0);
        let up = zax_smaa_search_y_up(offset1.xy, offset2.z);
        d.x = up;
        let e1 = zax_smaa_edges(vec2<f32>(offset0.x, up)).g;

        let down = zax_smaa_search_y_down(offset1.zw, offset2.w);
        d.y = down;

        // The reference's `d = abs(round(mad(RT.ww, d, -pixcoord.yy)))`. `metrics.w` is the height
        // and positive on both backends — the orientation lives in `.y`, not in `.w` — so the two
        // distances come out in pixels here exactly as they do there, and `abs` makes their order
        // immaterial.
        d = abs(round((smaa.metrics.ww * d) - pixcoord.yy));
        let sqrt_d = sqrt(d);

        let e2 = zax_smaa_edges_offset(
            vec2<f32>(offset0.x, down),
            vec2<f32>(0.0, 1.0),
        ).g;

        vertical = zax_smaa_area(sqrt_d, e1, e2, 0.0);
        vertical = zax_smaa_vertical_corner(vertical, vec4<f32>(uv.x, up, uv.x, down), d);
    }

    return vec4<f32>(horizontal, vertical);
}
