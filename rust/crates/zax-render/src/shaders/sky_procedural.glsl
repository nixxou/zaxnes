// The plane the two star fields crowd along, in scene coordinates.
const vec3 ZAX_SKY_AXIS = vec3(0.37, 0.88, -0.30);

float zaxSkyStarsR(vec3 d)
{
    // Four layers of unequal size and density, crowded along a band: a few big
    // bright stars over a dust of small ones, which is what the eye reads as depth.
    float band = 0.55 + 0.75 * zaxSkyBand(d, ZAX_SKY_AXIS, 0.42);
    return clamp(
        // The radii grow with the scale so that every layer draws a star of about the
        // same angular size - a tenth of a degree, which is a pixel or two at 1080p.
        zaxSkyStarLayer(d, 42.0, 0.030 * band, 0.13, 0.0)
            + zaxSkyStarLayer(d, 78.0, 0.085 * band, 0.20, 7.0)
            + zaxSkyStarLayer(d, 131.0, 0.110 * band, 0.32, 19.0)
            + zaxSkyStarLayer(d, 197.0, 0.130 * band, 0.46, 43.0),
        0.0,
        1.0);
}

float zaxSkyStarsB(vec3 d)
{
    // The twinkle mask: smooth, always on, between 0.35 and 1.
    return 0.35 + 0.65 * zaxSkyFbm(d * 9.0 + vec3(31.0, 17.0, 5.0));
}

float zaxSkyStarfield(vec3 d)
{
    float band = 0.6 + 0.8 * zaxSkyBand(d, ZAX_SKY_AXIS, 0.45);
    return clamp(
        zaxSkyStarLayer(d, 36.0, 0.022 * band, 0.09, 3.0)
            + zaxSkyStarLayer(d, 71.0, 0.070 * band, 0.15, 19.0)
            + zaxSkyStarLayer(d, 128.0, 0.105 * band, 0.25, 61.0)
            + zaxSkyStarLayer(d, 211.0, 0.120 * band, 0.38, 97.0),
        0.0,
        1.0);
}

float zaxSkyDensityR(vec3 d)
{
    // REQ-SCN-051 reads this through (dens - 0.4) / 0.4, so the field has to straddle
    // the 0.4 / 0.8 thresholds. Two octaves of a ridged field over a slow one give
    // the dense parts the filaments of a nebula instead of round blobs, and the
    // bias leaves a good half of the sky below 0.4 - open, dark space.
    float slow = zaxSkyFbm(d * 1.35 + vec3(4.0, 1.0, 9.0));
    float ridge = 1.0 - abs(2.0 * zaxSkyFbm(d * 3.1 + vec3(13.0, 7.0, 2.0)) - 1.0);
    float fine = 1.0 - abs(2.0 * zaxSkyFbm(d * 6.7 + vec3(27.0, 5.0, 31.0)) - 1.0);
    float mass = slow * 0.55 + ridge * 0.30 + fine * 0.15;
    return clamp((mass - 0.57) * 3.00 + 0.46, 0.0, 1.0);
}

vec3 zaxSkyDiffusion(vec3 d)
{
    // r drives the two ripple layers, g the ambient halo, b the base mass; they are
    // three different fields so that the ripples do not follow the masses exactly.
    // The halo is the term REQ-SCN-051 paints the widest, so it is the one that is
    // cut hardest: smoothstep leaves the quiet sky at zero and lets the clouds glow.
    float r = zaxSkyFbm(d * 3.4 + vec3(21.0, 3.0, 11.0));
    float g = zaxSkyFbm(d * 1.55 + vec3(2.0, 29.0, 6.0));
    float b = zaxSkyFbm(d * 2.2 + vec3(8.0, 14.0, 27.0));
    return clamp(
        vec3(
            r * 1.15 - 0.08,
            smoothstep(0.49, 0.85, g),
            smoothstep(0.44, 0.92, b)),
        vec3(0.0),
        vec3(1.0));
}

// The hue of the nebula, which the publishable build owns: slow magenta, violet and
// teal masses in the mood of the original cubemaps, computed, never copied
// (AMD-024, regle de creation des assets de remplacement). The colours of
// REQ-SCN-051 keep their place - the frame's palette still sets how bright and how
// saturated the sky is - but their hue turns with the field.
vec3 zaxSkyHue(vec3 d)
{
    float a = zaxSkyFbm(d * 1.05 + vec3(11.0, 2.0, 7.0));
    float b = zaxSkyFbm(d * 0.74 + vec3(5.0, 19.0, 23.0));
    vec3 hue = mix(vec3(0.95, 0.16, 0.58), vec3(0.45, 0.20, 0.95), smoothstep(0.34, 0.70, a));
    return mix(hue, vec3(0.10, 0.80, 0.74), smoothstep(0.50, 0.86, b));
}

vec3 zaxSkyNebulaColor(vec3 d, vec3 c)
{
    float level = max(max(c.r, c.g), c.b);
    return mix(c, zaxSkyHue(d) * level, 0.70);
}
