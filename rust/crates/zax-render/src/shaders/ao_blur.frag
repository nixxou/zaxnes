// The smoothing of AMD-055 REQ-SCN-083, "un lissage qui respecte les discontinuités de
// profondeur": a 4 × 4 box over the raw occlusion, each tap weighted by how near its surface is in
// depth to the one under the pixel.
//
// One pass rather than a separable pair, because the kernel is small and a second pass would be a
// second target and a second full-screen fill. The radius and the threshold of the smoothing are
// free implementation choices (REQ-SCN-083), so the sharpness below is a constant of this file and
// not a parameter.

in vec2 vUv;
out vec4 oOcclusion;

uniform sampler2D uOcclusion;
// One texel of the occlusion target, in viewport coordinates.
uniform vec2 uTexel;

// How fast a tap loses its weight with the depth difference, **relative to the distance of the
// pixel**: at a difference of 1/ZAX_AO_BLUR_SHARPNESS of the distance a tap counts for 1/e.
//
// Relative and not absolute on purpose. An absolute threshold in scene units either rejects the
// neighbours of a floor seen at a grazing angle — leaving the kernel's own noise unsmoothed, which
// REQ-SCN-083 forbids — or lets a silhouette bleed across. A smooth exponential rather than a cut
// for the same reason: a tap never stops counting altogether, so there is always something to
// average.
const float ZAX_AO_BLUR_SHARPNESS = 20.0;

void main()
{
    float center = zaxViewSample(vUv).z;
    float falloff = ZAX_AO_BLUR_SHARPNESS / max(abs(center), 1.0);

    float sum = 0.0;
    float total = 0.0;
    for (int y = -2; y <= 1; ++y)
    {
        for (int x = -2; x <= 1; ++x)
        {
            // −1.5, −0.5, +0.5 and +1.5 texels: a 4 × 4 box centred on the pixel.
            vec2 uv = vUv + (vec2(float(x) + 0.5, float(y) + 0.5) * uTexel);
            float weight = exp(-abs(zaxViewSample(uv).z - center) * falloff);
            sum += texture(uOcclusion, uv * uDrawnScale).r * weight;
            total += weight;
        }
    }

    oOcclusion = vec4(sum / max(total, 1e-4), 0.0, 0.0, 1.0);
}
