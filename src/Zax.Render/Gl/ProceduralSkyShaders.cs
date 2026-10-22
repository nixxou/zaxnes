namespace Zax.Render.Gl;

/// <summary>
/// The GLSL that replaces the skybox cubemaps of SPEC-06 by code (AMD-024 REQ-PUB-003): the star
/// field and the twinkle mask of the Gradient sky, the three nebula fields of the Fantasy sky,
/// and the procedural sun, moon and cloud layer of <see cref="SkyBodySettings"/>.
/// </summary>
/// <remarks>
/// <para>
/// The fragment shaders of <see cref="SkyboxShaders"/> never sample a cubemap directly: they call
/// the five functions that <see cref="Textured"/> and <see cref="Procedural"/> both define, so
/// the colour formulas of REQ-SCN-050 and REQ-SCN-051 are written once and the only thing the
/// publishable build changes is where the five fields come from. The textured block is what
/// <c>--assets</c> uses when the user supplies their own cubemaps, and it is also what the
/// acceptance criteria of REQ-SCN-051 inject uniform test cubemaps into.
/// </para>
/// <para>
/// No file is read either way: the procedural block is plain arithmetic on the view direction.
/// </para>
/// </remarks>
internal static class ProceduralSkyShaders
{
    /// <summary>
    /// The noise library both blocks build on: a hash, trilinear value noise, a five-octave
    /// sum and one sparse layer of round stars.
    /// </summary>
    public const string Noise = """
        // A cheap, well-behaved 3D hash in [0, 1). No trigonometry, so no precision cliff when
        // the lattice coordinate grows.
        float zaxSkyHash(vec3 p)
        {
            p = fract(p * vec3(0.1031, 0.1030, 0.0973));
            p += dot(p, p.yzx + 33.33);
            return fract((p.x + p.y) * p.z);
        }

        // Trilinear value noise with a smoothstep fade, mean 0.5.
        float zaxSkyNoise(vec3 x)
        {
            vec3 i = floor(x);
            vec3 f = x - i;
            vec3 u = f * f * (3.0 - 2.0 * f);

            float n000 = zaxSkyHash(i + vec3(0.0, 0.0, 0.0));
            float n100 = zaxSkyHash(i + vec3(1.0, 0.0, 0.0));
            float n010 = zaxSkyHash(i + vec3(0.0, 1.0, 0.0));
            float n110 = zaxSkyHash(i + vec3(1.0, 1.0, 0.0));
            float n001 = zaxSkyHash(i + vec3(0.0, 0.0, 1.0));
            float n101 = zaxSkyHash(i + vec3(1.0, 0.0, 1.0));
            float n011 = zaxSkyHash(i + vec3(0.0, 1.0, 1.0));
            float n111 = zaxSkyHash(i + vec3(1.0, 1.0, 1.0));

            return mix(
                mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y),
                mix(mix(n001, n101, u.x), mix(n011, n111, u.x), u.y),
                u.z);
        }

        // Five octaves of value noise, normalised back to [0, 1].
        float zaxSkyFbm(vec3 x)
        {
            float sum = 0.0;
            float weight = 0.0;
            float amplitude = 0.5;
            vec3 at = x;
            for (int octave = 0; octave < 5; octave++)
            {
                sum += amplitude * zaxSkyNoise(at);
                weight += amplitude;
                amplitude *= 0.5;
                at *= 2.03;
            }

            return sum / weight;
        }

        // One sparse layer of round stars: the direction is cut into cells, a fraction of them
        // holds a star, and each star is a soft disc inside its cell. 'radius' is the disc's
        // size in cells — a star has to stay a point, so a coarse layer takes a small
        // fraction of its cell and a fine one a larger one.
        float zaxSkyStarLayer(vec3 d, float scale, float density, float radius, float seed)
        {
            vec3 p = d * scale + vec3(seed);
            vec3 cell = floor(p);
            vec3 f = p - cell;

            if (zaxSkyHash(cell + vec3(seed + 1.7)) > density)
            {
                return 0.0;
            }

            vec3 centre = vec3(
                zaxSkyHash(cell + vec3(seed + 11.3)),
                zaxSkyHash(cell + vec3(seed + 23.9)),
                zaxSkyHash(cell + vec3(seed + 37.1)));
            float at = length(f - centre);

            // The brightest stars are the rarest: the square of a uniform draw. The floor
            // matters — REQ-SCN-050 multiplies the field by at most 0.2, so a star that
            // starts dim never reaches the image at all.
            float draw = zaxSkyHash(cell + vec3(seed + 53.7));
            float brightness = 0.40 + 0.60 * draw * draw;

            // A star of this layer is between half and one radius across.
            float size = radius * (0.5 + 0.5 * zaxSkyHash(cell + vec3(seed + 71.3)));
            return brightness * (1.0 - smoothstep(0.25 * size, size, at));
        }

        // A band of denser stars across the sky, the way a galactic plane reads: 1 on the
        // great circle normal to 'axis', fading to 0 a few tens of degrees away.
        float zaxSkyBand(vec3 d, vec3 axis, float width)
        {
            float t = dot(normalize(d), normalize(axis));
            return exp(-(t * t) / max(width * width, 1e-4));
        }
        """;

    /// <summary>
    /// The five sky fields read from the cubemaps SPEC-06 names: <c>Stars_LP_01</c> for the
    /// Gradient stars and <c>Starfield</c>, <c>NebulaDensity</c> and <c>NebulaDiffusion</c> for
    /// the Fantasy nebula.
    /// </summary>
    public const string Textured = """
        uniform samplerCube uStarMap;
        uniform samplerCube uStarfield;
        uniform samplerCube uDensity;
        uniform samplerCube uDiffusion;

        // REQ-SCN-050: s is the red channel of Stars_LP_01 in d times its blue channel in the
        // turned direction.
        float zaxSkyStarsR(vec3 d) { return texture(uStarMap, d).r; }
        float zaxSkyStarsB(vec3 d) { return texture(uStarMap, d).b; }

        float zaxSkyStarfield(vec3 d) { return texture(uStarfield, d).r; }
        float zaxSkyDensityR(vec3 d) { return texture(uDensity, d).r; }
        vec3 zaxSkyDiffusion(vec3 d) { return texture(uDiffusion, d).rgb; }

        // With real cubemaps the colours of REQ-SCN-051 are used exactly as the formula
        // gives them; the hue field below belongs to the procedural sky alone.
        vec3 zaxSkyNebulaColor(vec3 d, vec3 c) { return c; }
        """;

    /// <summary>
    /// The same five fields, computed from the direction alone: no sampler, no file.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The star field and the twinkle mask are deliberately asymmetric: the red field is the
    /// sparse disc of stars, the blue one a smooth field between 0.35 and 1. REQ-SCN-050 reads
    /// the sky as <c>s = R(d) × B(R_Y(b)·d)</c>, so with two sparse fields a star would only
    /// light up when both samples happened to fall on one — almost never. With a smooth blue
    /// field every star keeps its place and its brightness breathes as the mask turns past it at
    /// 3° per second, which is the « étoiles fines qui scintillent lentement » the reference view
    /// R4 of SPEC-06 §6 describes. The formula itself is untouched.
    /// </para>
    /// <para>
    /// Nothing here depends on time; the sky is fixed in the scene frame (REQ-SCN-049).
    /// </para>
    /// </remarks>
    public const string Procedural = """
        // The plane the two star fields crowd along, in scene coordinates.
        const vec3 ZAX_SKY_AXIS = vec3(0.37, 0.88, -0.30);

        float zaxSkyStarsR(vec3 d)
        {
            // Four layers of unequal size and density, crowded along a band: a few big
            // bright stars over a dust of small ones, which is what the eye reads as depth.
            float band = 0.55 + 0.75 * zaxSkyBand(d, ZAX_SKY_AXIS, 0.42);
            return clamp(
                // The radii grow with the scale so that every layer draws a star of about the
                // same angular size — a tenth of a degree, which is a pixel or two at 1080p.
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
            // bias leaves a good half of the sky below 0.4 — open, dark space.
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
        // (AMD-024, règle de création des assets de remplacement). The colours of
        // REQ-SCN-051 keep their place — the frame's palette still sets how bright and how
        // saturated the sky is — but their hue turns with the field.
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
        """;

    /// <summary>
    /// The sun disc, the moon disc and the cloud layer of REQ-PUB-003, composited over the sky
    /// colour. Every gain is zero by default, which is the « sun, moon and clouds are disabled »
    /// of REQ-SCN-050 (Q-064).
    /// </summary>
    /// <remarks>
    /// The clouds are a band around the horizon that thins out towards the poles; the moon
    /// carries a procedural crater pattern so that it does not read as a flat disc. Both discs
    /// are feathered by <see cref="SkyBodySettings.DiscSoftness"/>.
    /// </remarks>
    public const string Bodies = """
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
        """;
}
