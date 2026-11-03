// The smoothing of AMD-055 REQ-SCN-083, "un lissage qui respecte les discontinuités de
// profondeur": the WGSL twin of shaders/ao_blur.frag — a 4 x 4 box over the raw occlusion, each
// tap weighted by how near its surface is in depth to the one under the pixel.
//
// One pass rather than a separable pair, because the kernel is small and a second pass would be a
// second target and a second full-screen fill. The radius and the threshold of the smoothing are
// free implementation choices (REQ-SCN-083), so the sharpness below is a constant of this file
// and not a parameter.
//
// `zax_source0` is the raw occlusion, read texel by texel (GL binds it NEAREST).

// How fast a tap loses its weight with the depth difference, **relative to the distance of the
// pixel**: at a difference of 1/ZAX_AO_BLUR_SHARPNESS of the distance a tap counts for 1/e.
//
// Relative and not absolute on purpose. An absolute threshold in scene units either rejects the
// neighbours of a floor seen at a grazing angle — leaving the kernel's own noise unsmoothed,
// which REQ-SCN-083 forbids — or lets a silhouette bleed across. A smooth exponential rather than
// a cut for the same reason: a tap never stops counting altogether, so there is always something
// to average.
const ZAX_AO_BLUR_SHARPNESS: f32 = 20.0;

@fragment
fn fs_main(in: ScreenVertex) -> @location(0) vec4<f32> {
    let center = zax_view_sample(in.uv).z;
    let falloff = ZAX_AO_BLUR_SHARPNESS / max(abs(center), 1.0);
    let texel = zax_texel();

    var sum = 0.0;
    var total = 0.0;
    for (var y = -2; y <= 1; y++) {
        for (var x = -2; x <= 1; x++) {
            // -1.5, -0.5, +0.5 and +1.5 texels: a 4 x 4 box centred on the pixel.
            let uv = in.uv + (vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) * texel);
            let weight = exp(-abs(zax_view_sample(uv).z - center) * falloff);
            sum += zax_load0(uv).r * weight;
            total += weight;
        }
    }

    return vec4<f32>(sum / max(total, 1e-4), 0.0, 0.0, 1.0);
}
