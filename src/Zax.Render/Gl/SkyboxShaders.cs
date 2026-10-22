namespace Zax.Render.Gl;

/// <summary>
/// The GLSL of the two skyboxes of SPEC-06: Gradient (REQ-SCN-050) and Fantasy (REQ-SCN-051).
/// </summary>
/// <remarks>
/// <para>
/// Both are drawn as one full-screen triangle right after the clear, with the depth test off, so
/// that the whole scene is painted over them (REQ-SCN-049, REQ-SCN-058). The fragment's view
/// direction is rebuilt from the camera basis expressed in <b>scene</b> coordinates, which is
/// also the frame the sky fields are authored in, so the direction is used as the sampling vector
/// unchanged.
/// </para>
/// <para>
/// The colour formulas are written once; where the five sky fields come from is a parameter
/// (AMD-024 REQ-PUB-003). <see cref="ProceduralSkyShaders.Procedural"/> computes them from the
/// direction, which is what the publishable build uses — no image file at all —  and
/// <see cref="ProceduralSkyShaders.Textured"/> samples the cubemaps of a user-supplied asset
/// folder.
/// </para>
/// </remarks>
internal static class SkyboxShaders
{
    /// <summary>
    /// Full-screen triangle. No vertex buffer: the three corners come from
    /// <c>gl_VertexID</c>, which a GL 3.3 core context provides.
    /// </summary>
    public const string Vertex = """
        #version 330 core

        uniform vec3 uRight;
        uniform vec3 uUp;
        uniform vec3 uForward;
        uniform vec2 uTanHalf;

        out vec3 vDirection;

        void main()
        {
            vec2 ndc = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2) * 2.0 - 1.0;
            vDirection = uForward + uRight * (ndc.x * uTanHalf.x) + uUp * (ndc.y * uTanHalf.y);
            gl_Position = vec4(ndc, 1.0, 1.0);
        }
        """;

    /// <summary>The Gradient sky of REQ-SCN-050, over the sky fields of <paramref name="fields"/>.</summary>
    /// <param name="fields">
    /// <see cref="ProceduralSkyShaders.Procedural"/> or
    /// <see cref="ProceduralSkyShaders.Textured"/>.
    /// </param>
    public static string GradientFragment(string fields) => $$"""
        #version 330 core
        in vec3 vDirection;
        out vec4 oColor;

        // M, the universal NES background colour of the current frame, in sRGB 0..1.
        uniform vec3 uBackground;
        uniform float uStars;
        uniform float uStarAngle;

        {{SceneShaders.Transfer}}

        {{ProceduralSkyShaders.Noise}}

        {{fields}}

        {{ProceduralSkyShaders.Bodies}}

        void main()
        {
            vec3 d = normalize(vDirection);
            float y = d.y;

            // REQ-SCN-050: Hc = 1.4 M and Bc = 0.6 M, never saturated before the conversion.
            vec3 m = uBackground;
            vec3 c = m + (1.4 * m - m) * pow(max(y, 0.0), 2.5);
            vec3 g = c + (0.6 * m - c) * pow(max(-y, 0.0), 2.5);
            vec3 lit = zaxToLinear3(g);

            if (uStars > 0.5)
            {
                // The second sample is taken in d turned about Y, which is the rotation
                // R_Y(b) of REQ-SCN-007 in the left-handed scene frame.
                float ca = cos(uStarAngle);
                float sa = sin(uStarAngle);
                vec3 turned = vec3(d.x * ca + d.z * sa, d.y, -d.x * sa + d.z * ca);
                float s = zaxSkyStarsR(d) * zaxSkyStarsB(turned);
                float k = clamp(s * min(abs(0.2 * y), 1.0), 0.0, 1.0);
                // E, a slightly grey white: sRGB 0.5 doubled, about 0.98 in linear.
                float e = zaxToLinear(0.5) * {{SkyGradient.SrgbDouble}};
                lit = lit * (1.0 - k) + k * vec3(e);
            }

            // REQ-PUB-003 (Q-064): the sun, the moon and the clouds, all off by default.
            lit = zaxSkyBodies(d, lit);

            // R2-009: linear out; the sRGB encode is the resolve pass.
            oColor = vec4(clamp(lit, 0.0, 1.0), 1.0);
        }
        """;

    /// <summary>The Fantasy nebula sky of REQ-SCN-051, over the sky fields of <paramref name="fields"/>.</summary>
    /// <param name="fields">
    /// <see cref="ProceduralSkyShaders.Procedural"/> or
    /// <see cref="ProceduralSkyShaders.Textured"/>.
    /// </param>
    public static string FantasyFragment(string fields) => $$"""
        #version 330 core
        in vec3 vDirection;
        out vec4 oColor;

        // F0 to F3: the universal background colour and colours 1 to 3 of background
        // palette 0 of the frame, in sRGB 0..1.
        uniform vec3 uF0;
        uniform vec3 uF1;
        uniform vec3 uF2;
        uniform vec3 uF3;
        uniform float uGamma;

        {{SceneShaders.Transfer}}

        {{ProceduralSkyShaders.Noise}}

        {{fields}}

        {{ProceduralSkyShaders.Bodies}}

        const float SRGB_DOUBLE = {{SkyGradient.SrgbDouble}};

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
        """;
}
