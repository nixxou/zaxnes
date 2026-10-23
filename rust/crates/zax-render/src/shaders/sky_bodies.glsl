uniform vec3 uSunDirection;
uniform vec3 uSunLinear;
uniform float uSunGain;
uniform vec2 uSunCos;

uniform vec3 uMoonDirection;
uniform vec3 uMoonLinear;
uniform float uMoonGain;
uniform vec2 uMoonCos;

uniform vec3 uCloudLinear;
uniform float uCloudGain;
uniform float uCloudCoverage;
uniform float uCloudScale;

// A disc centred on 'direction': 1 inside the inner cone, 0 outside the outer one.
// x holds the cosine of the inner angle, y that of the outer one.
float zaxSkyDisc(vec3 d, vec3 direction, vec2 cosines)
{
    return smoothstep(cosines.y, cosines.x, dot(d, direction));
}

vec3 zaxSkyBodies(vec3 d, vec3 color)
{
    if (uCloudGain > 0.0)
    {
        // A horizon band: strongest at y = 0, gone at the poles.
        float band = 1.0 - smoothstep(0.08, 0.72, abs(d.y));
        float scale = max(uCloudScale, 0.05);

        // Two octaves an octave and a half apart, the fine one carried by the
        // coarse one: the edges of a cloud fray instead of being one noise contour.
        float coarse = zaxSkyFbm(d * scale + vec3(57.0, 13.0, 41.0));
        float fine = zaxSkyFbm(d * scale * 2.7 + vec3(9.0, 83.0, 17.0));
        float field = coarse * 0.72 + fine * 0.28;
        float threshold = 1.0 - uCloudCoverage;
        float mass = smoothstep(threshold - 0.06, min(threshold + 0.40, 1.0), field);

        // Thicker in the middle than at the rim, so the layer reads as volume.
        mass = mass * mass * (3.0 - 2.0 * mass);
        color = mix(color, uCloudLinear, clamp(mass * band * uCloudGain, 0.0, 1.0));
    }

    if (uMoonGain > 0.0)
    {
        float disc = zaxSkyDisc(d, uMoonDirection, uMoonCos);
        float craters = 0.78 + 0.22 * zaxSkyFbm(d * 180.0 + vec3(3.0, 71.0, 23.0));
        color = mix(color, uMoonLinear * craters, clamp(disc * uMoonGain, 0.0, 1.0));
    }

    if (uSunGain > 0.0)
    {
        // A faint glow around the disc, then the disc itself.
        float glow = zaxSkyDisc(
            d, uSunDirection, vec2(uSunCos.x, mix(uSunCos.y, -1.0, 0.04)));
        color = mix(color, uSunLinear, clamp(0.22 * glow * glow * uSunGain, 0.0, 1.0));

        float disc = zaxSkyDisc(d, uSunDirection, uSunCos);
        color = mix(color, uSunLinear, clamp(disc * uSunGain, 0.0, 1.0));
    }

    return color;
}
