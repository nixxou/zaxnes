// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// The **bilateral upsample** of `AoBlurAndUpsampleCS.hlsli`, the WGSL twin of
// shaders/msvo_upsample.frag and the step that makes the estimate multi-scale (AMD-055
// REQ-SCN-083 *Mode MSVO*: "l'occultation est estimée à plusieurs résolutions de la profondeur
// puis combinée"): the coarse level is carried into the finer one with weights taken from how well
// the two depths agree, then **multiplied** by the finer level's own occlusion. Run from the
// coarsest pair down to the finest, each step folding everything coarser into the level below it.
//
// The **last** step is also where the access becomes the occlusion `O` the composite reads, and
// where the ceiling of REQ-SCN-083 is applied — the same convention `ao.wgsl` follows for the
// Depth estimator, so that `ao_composite.wgsl` stays a bare `1 - O` shared by both modes.
//
// The four sources, in the order the OpenGL twin binds its samplers: `zax_source0` the fine depth
// and `zax_source1` the coarse depth, both read texel by texel (`NEAREST`, and the bilateral
// upsample wants no filtering — it gathers nine *distinct* coarse texels with weights of its own);
// `zax_source2` the coarse occlusion and `zax_source3` the fine one, both through the filtered
// sampler, which is what the OpenGL `R8` levels carry.
//
// The nine taps run from -1 to +1 on each axis and the kernel is symmetric, so the mirrored row
// order of this API changes nothing here.

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    let coarse_texel = screen.msvo_c.zw;
    let headroom = screen.msvo_b.x;
    let share = screen.msvo_b.z;
    let ceiling = screen.msvo_b.w;
    let far_distance = screen.msvo_d.x;
    let final_step = screen.msvo_counts.y;
    let scale = screen.drawn.xy;

    let fine = zax_load0(in.uv).r;

    // The tolerance of the original, expressed on a depth kept in scene units: a tap whose depth
    // is within a hundredth of this pixel's distance is a good match.
    let tolerance = (0.01 * fine) + 1e-3;

    var total = 0.0;
    var weight = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let uv = in.uv + (vec2<f32>(f32(x), f32(y)) * coarse_texel);
            let depth = zax_load1(uv).r;
            let tent = select(1.0, 2.0, x == 0) * select(1.0, 2.0, y == 0);
            let w = tent / (abs(fine - depth) + tolerance);
            total += textureSampleLevel(zax_source2, zax_smooth, uv * scale, 0.0).r * w;
            weight += w;
        }
    }

    let coarse = select(1.0 / headroom, total / weight, weight > 0.0) * headroom;
    let own = textureSampleLevel(zax_source3, zax_smooth, in.uv * scale, 0.0).r * headroom;
    let access = own * coarse;

    // Only the last step turns the access into the occlusion `O`, and only there is it bounded:
    // an access above 1 is "less than half occluded", which the subtraction reads as no darkening,
    // and the ceiling of REQ-SCN-083 is applied in the same expression.
    let occlusion = select(
        clamp((1.0 - access) / max(share, 1e-4), 0.0, 1.0),
        0.0,
        fine >= far_distance);
    let value = select(
        clamp(access, 0.0, headroom) / headroom,
        ceiling * occlusion,
        final_step != 0);
    return vec4<f32>(value, 0.0, 0.0, 1.0);
}
