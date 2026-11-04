// The third pass of AMD-060 REQ-SCN-085 on this backend: the neighbourhood blending. The WGSL
// twin of shaders/smaa_blend.frag — `SMAANeighborhoodBlendingPS` of
// `third_party/smaa/SMAA.hlsl` (MIT).
//
// It reads the frame's **linear** colour (AMD-021 R2-009) and writes the filtered colour into a
// target of its own: this is the one pass of the three that cannot work in place, so it cannot be
// a blend state the way REQ-SCN-083's composite is. The space is linear and before the sRGB
// encode, and `renderer.rs::run_smaa` carries the reasons on both backends.
//
// **A pixel with no blending weight is written back unchanged**, through the reference's own
// early-out — the second acceptance criterion of REQ-SCN-085, and it holds to the bit here for
// the same reason it holds on the other side: the source and the target are both `Rgba16Float`,
// and a texel read at its own centre is the texel itself.

// One of the reference's two colour taps, done by hand.
//
// The reference "exploits bilinear filtering to mix current pixel with the chosen neighbor": it
// samples at `texcoord + fraction · metrics` and lets the hardware filter return
// `(1 − fraction) · here + fraction · there`. The frame's colour is read texel by texel here, as
// the OpenGL backend's `NEAREST` reads it, so the mix is written out instead. Same two texels,
// same weight, one `mix`.
fn zax_smaa_blend_toward(uv: vec2<f32>, towards: vec2<f32>, fraction: f32) -> vec4<f32> {
    let here = zax_smaa_color(uv);
    let there = zax_smaa_color_offset(uv, towards);
    return mix(here, there, fraction);
}

@fragment
fn fs_main(in: SmaaVertex) -> @location(0) vec4<f32> {
    let uv = in.uv;
    // The reference's four weights: the right neighbour's `a`, the neighbour one step along `+y`
    // and its own `x` and `z`. `+y` is *down* on screen on this backend, which is what
    // `SMAANeighborhoodBlendingVS`'s `float4(1, 0, 0, 1)` means in Direct3D (`smaa.wgsl`).
    let own = zax_smaa_weights(uv);
    var a: vec4<f32>;
    a.x = zax_smaa_weights_offset(uv, vec2<f32>(1.0, 0.0)).a;
    a.y = zax_smaa_weights_offset(uv, vec2<f32>(0.0, 1.0)).g;
    a.w = own.x;
    a.z = own.z;

    // The reference's own early-out, and with it the byte-identity of everything the first pass
    // did not mark.
    if (dot(a, vec4<f32>(1.0, 1.0, 1.0, 1.0)) < 1e-5) {
        return zax_smaa_color(uv);
    }

    // Which axis carries the larger weight, and the pair that goes with it. The reference does
    // this with two `SMAAMovc`s over a `float4` of offsets; written as the branch it is, because
    // the two arms differ only in which texel pair they mix.
    let horizontal = max(a.x, a.z) > max(a.y, a.w);
    var weight = select(vec2<f32>(a.y, a.w), vec2<f32>(a.x, a.z), horizontal);
    weight = weight / dot(weight, vec2<f32>(1.0, 1.0));

    var first: vec4<f32>;
    var second: vec4<f32>;
    if (horizontal) {
        first = zax_smaa_blend_toward(uv, vec2<f32>(1.0, 0.0), a.x);
        second = zax_smaa_blend_toward(uv, vec2<f32>(-1.0, 0.0), a.z);
    } else {
        first = zax_smaa_blend_toward(uv, vec2<f32>(0.0, 1.0), a.y);
        second = zax_smaa_blend_toward(uv, vec2<f32>(0.0, -1.0), a.w);
    }

    return (weight.x * first) + (weight.y * second);
}
