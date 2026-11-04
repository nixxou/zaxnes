// The third pass of AMD-060 REQ-SCN-085: the neighbourhood blending.
// `SMAANeighborhoodBlendingPS` of `third_party/smaa/SMAA.hlsl` (MIT).
//
// It reads the frame's **linear** colour (AMD-021 R2-009) and writes the filtered colour into a
// target of its own — this is the one pass of the three that cannot work in place, so it cannot be
// a blend state the way REQ-SCN-083's composite is. The space is linear and before the sRGB encode,
// and `renderer.rs::run_smaa` carries the three reasons.
//
// **A pixel with no blending weight is written back unchanged**, through the reference's own
// early-out. That is the second acceptance criterion of REQ-SCN-085 — "sur une scène dont une seule
// face plane porte une texture contrastée, les pixels intérieurs à la face sont identiques à
// l'octet à ceux de la capture à 0" — and it holds to the bit: the source and this target are both
// `RGBA16F`, and a `NEAREST` read at a pixel centre is the texel itself.

in vec2 vUv;
out vec4 oColor;

// The frame's linear colour, as the resolve pass of AMD-021 R2-009 reads it.
uniform sampler2D uColor;

// The blending weights of the second pass: `rg` the horizontal pair, `ba` the vertical one.
uniform sampler2D uWeights;

// One of the reference's two colour taps, done by hand.
//
// The reference "exploits bilinear filtering to mix current pixel with the chosen neighbor": it
// samples at `texcoord + fraction · metrics`, and the hardware filter then returns
// `(1 − fraction) · here + fraction · there`. The frame's colour texture is **`NEAREST`** here
// (`gl/linear_buffer.rs` has always allocated it so, and the resolve pass samples it at exact texel
// centres where the two filters agree), so the mix is written out instead of changing a sampler
// state the whole renderer shares. Same two texels, same weight, one `mix`.
vec4 zaxSmaaBlendToward(vec2 towards, float fraction)
{
    vec4 here = zaxSmaaFetch(uColor, vUv);
    vec4 there = zaxSmaaFetchOffset(uColor, vUv, towards);
    return mix(here, there, fraction);
}

void main()
{
    // The reference's four weights: the right neighbour's `a`, the neighbour one step along `+y`
    // and its own `x` and `z`. `+y` is *down* on screen — `uRtMetrics.y` is negative here, so
    // `vec2(0, 1)` means what `SMAANeighborhoodBlendingVS`'s `float4(1, 0, 0, 1)` means in Direct3D
    // (`smaa.glsl` point 1).
    vec4 a;
    a.x = zaxSmaaFetchOffset(uWeights, vUv, vec2(1.0, 0.0)).a;
    a.y = zaxSmaaFetchOffset(uWeights, vUv, vec2(0.0, 1.0)).g;
    a.wz = zaxSmaaFetch(uWeights, vUv).xz;

    // The reference's own early-out, and with it the byte-identity of everything the first pass did
    // not mark.
    if (dot(a, vec4(1.0, 1.0, 1.0, 1.0)) < 1e-5)
    {
        oColor = zaxSmaaFetch(uColor, vUv);
        return;
    }

    // Which axis carries the larger weight, and the pair that goes with it. The reference does this
    // with two `SMAAMovc`s over a `float4` of offsets; written as the branch it is, because the two
    // arms differ only in which texel pair they mix and GLSL has no `inout` conditional move worth
    // the obscurity.
    bool horizontal = max(a.x, a.z) > max(a.y, a.w);
    vec2 weight = horizontal ? a.xz : a.yw;
    weight /= dot(weight, vec2(1.0, 1.0));

    vec4 first;
    vec4 second;
    if (horizontal)
    {
        first = zaxSmaaBlendToward(vec2(1.0, 0.0), a.x);
        second = zaxSmaaBlendToward(vec2(-1.0, 0.0), a.z);
    }
    else
    {
        first = zaxSmaaBlendToward(vec2(0.0, 1.0), a.y);
        second = zaxSmaaBlendToward(vec2(0.0, -1.0), a.w);
    }

    oColor = (weight.x * first) + (weight.y * second);
}
