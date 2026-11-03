// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// The `SmartBlur` of `AoBlurAndUpsampleCS.hlsli`, the WGSL twin of shaders/msvo_blur.frag: one
// separable pass of the five-tap depth-respecting blur the original runs over **every** level of
// its chain before that level is combined with the next.
//
// **This pass is what a port without it is missing** (Q-100 correction 1). In the original, every
// `BlurAndUpsample` step blurs its low-resolution input over ±2 texels and the chain ends with one
// more step at full resolution, so *no* level ever reaches the output unblurred. It is also the
// "lissage qui respecte les discontinuités de profondeur" REQ-SCN-083 asks for.
//
// `zax_source0` is the occlusion being blurred, read through the **filtered** sampler as the
// OpenGL level is; `zax_source1` is this level's depth, read texel by texel as `NEAREST`.
//
// The step is one texel along the axis being blurred, and its sign does not matter: the kernel runs
// from -2 to +2 and is symmetric, so the mirrored row order of this API changes nothing here.

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    let step_uv = screen.msvo_c.xy;
    let sharpness = screen.msvo_b.y;

    let center = zax_load1(in.uv).r;
    let falloff = sharpness / max(center, 1e-3);

    var total = 0.0;
    var weight = 0.0;
    for (var tap = -2; tap <= 2; tap++) {
        let uv = in.uv + (step_uv * f32(tap));
        let depth = zax_load1(uv).r;

        // The kernel of `SmartBlur`: the two outer taps weigh half of the three inner.
        let tent = select(1.0, 0.5, abs(tap) == 2);
        let w = tent / (1.0 + (abs(center - depth) * falloff));
        total += textureSampleLevel(zax_source0, zax_smooth, uv * screen.drawn.xy, 0.0).r * w;
        weight += w;
    }

    let blurred = select(
        textureSampleLevel(zax_source0, zax_smooth, in.uv * screen.drawn.xy, 0.0).r,
        total / weight,
        weight > 0.0);
    return vec4<f32>(blurred, 0.0, 0.0, 1.0);
}
