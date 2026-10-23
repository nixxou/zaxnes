in vec3 vDirection;
out vec4 oColor;

// F0 to F3: the universal background colour and colours 1 to 3 of background
// palette 0 of the frame, in sRGB 0..1.
uniform vec3 uF0;
uniform vec3 uF1;
uniform vec3 uF2;
uniform vec3 uF3;
uniform float uGamma;

const float SRGB_DOUBLE = ZAX_SRGB_DOUBLE;

// smoothed(u) = 3u^2 - 2u^3 on u saturated in [0, 1].
float zaxSmoothed(float u)
{
    u = clamp(u, 0.0, 1.0);
    return u * u * (3.0 - 2.0 * u);
}

// u(x) = (dens(x) - 0.4) / 0.4, with the low threshold 0.4 and the high one 0.8.
float zaxDensity(vec3 x)
{
    return (zaxSkyDensityR(normalize(x)) - 0.4) / 0.4;
}

void main()
{
    vec3 d = normalize(vDirection);
    vec3 delta = zaxSkyDiffusion(d);
    float u0 = zaxDensity(d);

    // 1. Background F0, mixed towards the star tint. Q-024: the division and the
    // averages of tints are read as sRGB-space operations, linearised afterwards.
    vec3 color = zaxToLinear3(uF0);
    float alpha = uGamma > 0.4 ? 0.0 : 1.0 - uGamma;
    float field = zaxSkyStarfield(d);
    float weight = clamp(clamp(field, 0.0, 1.0) * alpha, 0.0, 1.0);
    color = mix(color, zaxToLinear3(uF1 * 0.5) * SRGB_DOUBLE, weight);

    // 2. Ambient halo, strong outside the dense areas.
    vec3 halo = zaxToLinear3(zaxSkyNebulaColor(d, uF0)) * SRGB_DOUBLE * delta.g;
    color = mix(color, halo, clamp(2.0 * delta.g * zaxSmoothed(1.0 - u0), 0.0, 1.0));

    // 3. Base, inside the dense areas.
    float mean = (delta.g + delta.b) * 0.5;
    vec3 base = zaxToLinear3(zaxSkyNebulaColor(d, (uF2 + uF3) * 0.5)) * SRGB_DOUBLE * mean;
    color = mix(
        color,
        base,
        clamp(2.0 * mean * (1.0 - uGamma) * 0.25 * zaxSmoothed(u0), 0.0, 1.0));

    // 4. Two ripple layers.
    vec3 d1 = d + 0.75 * vec3(0.2 * delta.b, -0.1 * delta.b, 1.0 * delta.b);
    float rho1 = zaxSkyDiffusion(normalize(d1)).r;
    color = mix(
        color,
        zaxToLinear3(zaxSkyNebulaColor(d1, uF2)) * SRGB_DOUBLE * rho1,
        clamp(rho1 * 2.0 * (5.0 / 255.0) * zaxSmoothed(zaxDensity(d1)), 0.0, 1.0));

    vec3 d2 = d + vec3(1.0, 0.1, 0.2) * delta.g;
    float rho2 = zaxSkyDiffusion(normalize(d2)).r;
    color = mix(
        color,
        zaxToLinear3(zaxSkyNebulaColor(d2, uF3)) * SRGB_DOUBLE * rho2,
        clamp(rho2 * 2.0 * (5.0 / 255.0) * zaxSmoothed(zaxDensity(d2)), 0.0, 1.0));

    // REQ-PUB-003 (Q-064): the sun, the moon and the clouds, all off by default.
    color = zaxSkyBodies(d, color);

    // 5. Exposure 1.
    oColor = vec4(clamp(color, 0.0, 1.0), 1.0);
}
