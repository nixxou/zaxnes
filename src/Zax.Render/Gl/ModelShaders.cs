namespace Zax.Render.Gl;

/// <summary>
/// The GLSL of the decorative television of SPEC-06 REQ-SCN-054 and AMD-025.
/// </summary>
/// <remarks>
/// <para>
/// AMD-025 ships one glTF 2.0 model, so the shading is the metallic-roughness model of
/// glTF 2.0 §B: a Lambert diffuse term over <c>(1 − metallic)</c> of the base colour, and a
/// Cook-Torrance specular term with the GGX distribution, the Smith height-correlated visibility
/// and the Schlick Fresnel, whose F0 is 0.04 for a dielectric and the base colour for a metal.
/// The diffuse term keeps the ambient and the light of REQ-SCN-028, so the cabinet shades under
/// the same light as a tile.
/// </para>
/// <para>
/// AMD-021 R2-014, which AMD-025 keeps: the cabinet neither casts nor receives a shadow, the
/// buttons do both.
/// </para>
/// <para>
/// The model carries tangents, but reading them would mean a fourth vertex attribute across the
/// whole mesh pipeline; the tangent frame of the normal map is rebuilt from the screen-space
/// derivatives of the position and of the texture coordinates instead — the usual cotangent
/// frame, which needs nothing beyond GLSL 330.
/// </para>
/// </remarks>
internal static class ModelShaders
{
    /// <summary>Vertex shader: the scene placement of REQ-SCN-001, plus the texture coordinates.</summary>
    public const string Vertex = """
        #version 330 core
        layout(location = 0) in vec3 aPosition;
        layout(location = 1) in vec3 aNormal;
        layout(location = 2) in vec2 aUv;

        uniform mat4 uModel;
        uniform mat3 uNormalMatrix;
        uniform mat4 uViewProjection;
        // −1 when the placement mirrors the model, which turns the cofactor matrix into the
        // inverse transpose the lighting needs.
        uniform float uNormalSign;

        out vec3 vScene;
        out vec3 vNormal;
        out vec2 vUv;

        void main()
        {
            vec4 scene = uModel * vec4(aPosition, 1.0);
            vScene = scene.xyz;
            vNormal = uNormalSign * (uNormalMatrix * aNormal);
            // glTF texture coordinates have v = 0 at the top row, like the decoded images, so
            // they are used as they are.
            vUv = aUv;
            gl_Position = uViewProjection * vec4(scene.x, scene.y, -scene.z, 1.0);
        }
        """;

    /// <summary>Fragment shader: base colour, normal map, metallic-roughness, emission.</summary>
    public static string Fragment { get; } = $$"""
        #version 330 core
        in vec3 vScene;
        in vec3 vNormal;
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uBaseColor;
        uniform sampler2D uNormalMap;
        uniform sampler2D uMetallicRoughness;
        uniform sampler2D uEmissive;

        uniform int uHasNormalMap;
        uniform int uHasMetallicRoughness;
        uniform int uHasEmissive;

        uniform vec4 uBaseColorFactor;
        uniform float uNormalScale;
        uniform float uMetallic;
        uniform float uRoughness;
        uniform vec3 uEmissiveColor;

        uniform vec3 uLightDirection;
        uniform vec3 uLightLinear;
        uniform float uAmbient;
        uniform vec3 uEye;

        uniform int uAlphaCutout;
        uniform float uAlphaCutoff;

        // AMD-021 R2-014: only the buttons receive a shadow.
        uniform int uReceiveShadow;
        uniform sampler2D uShadowMap;
        uniform mat4 uLightViewProjection;
        uniform float uShadowBias;
        uniform float uNormalOffset;
        uniform float uShadowStrength;

        {{SceneShaders.Transfer}}

        mat3 zaxCotangentFrame(vec3 n, vec3 p, vec2 uv)
        {
            vec3 dp1 = dFdx(p);
            vec3 dp2 = dFdy(p);
            vec2 duv1 = dFdx(uv);
            vec2 duv2 = dFdy(uv);

            vec3 dp2perp = cross(dp2, n);
            vec3 dp1perp = cross(n, dp1);
            vec3 t = dp2perp * duv1.x + dp1perp * duv2.x;
            vec3 b = dp2perp * duv1.y + dp1perp * duv2.y;
            float invmax = inversesqrt(max(dot(t, t), dot(b, b)) + 1e-12);
            return mat3(t * invmax, b * invmax, n);
        }

        // Trowbridge-Reitz (GGX) normal distribution, glTF 2.0 §B.
        float zaxGgx(float ndh, float alpha)
        {
            float a2 = alpha * alpha;
            float d = ndh * ndh * (a2 - 1.0) + 1.0;
            return a2 / max(3.14159265 * d * d, 1e-6);
        }

        // Smith height-correlated visibility, already divided by 4·NdL·NdV.
        float zaxVisibility(float ndl, float ndv, float alpha)
        {
            float a2 = alpha * alpha;
            float l = ndv * sqrt(ndl * ndl * (1.0 - a2) + a2);
            float v = ndl * sqrt(ndv * ndv * (1.0 - a2) + a2);
            return 0.5 / max(l + v, 1e-6);
        }

        void main()
        {
            vec4 sampled = texture(uBaseColor, vUv) * uBaseColorFactor;
            if (uAlphaCutout != 0 && sampled.a < uAlphaCutoff)
            {
                discard;
            }

            vec3 base = zaxToLinear3(sampled.rgb);

            vec3 n = normalize(vNormal);
            if (uHasNormalMap != 0)
            {
                vec3 map = texture(uNormalMap, vUv).rgb * 2.0 - 1.0;
                map.xy *= uNormalScale;
                n = normalize(zaxCotangentFrame(n, vScene, vUv) * map);
            }

            // glTF 2.0 §3.9.6: roughness in green, metalness in blue, both linear.
            float roughness = uRoughness;
            float metallic = uMetallic;
            if (uHasMetallicRoughness != 0)
            {
                vec3 mr = texture(uMetallicRoughness, vUv).rgb;
                roughness *= mr.g;
                metallic *= mr.b;
            }

            roughness = clamp(roughness, 0.04, 1.0);
            metallic = clamp(metallic, 0.0, 1.0);
            float alpha = roughness * roughness;

            vec3 l = normalize(-uLightDirection);
            vec3 v = normalize(uEye - vScene);
            vec3 h = normalize(l + v);
            float ndl = max(dot(n, l), 0.0);
            float ndv = max(dot(n, v), 1e-4);
            float ndh = max(dot(n, h), 0.0);
            float vdh = max(dot(v, h), 0.0);

            float shadow = 1.0;
            if (uReceiveShadow != 0)
            {
                // REQ-SCN-031, with the same normal offset and depth bias as the scene pass.
                vec3 biased = vScene + n * uNormalOffset;
                vec4 light = uLightViewProjection * vec4(biased.x, biased.y, -biased.z, 1.0);
                vec3 projected = light.xyz / light.w * 0.5 + 0.5;
                if (projected.x >= 0.0 && projected.x <= 1.0
                    && projected.y >= 0.0 && projected.y <= 1.0
                    && projected.z <= 1.0
                    && projected.z - uShadowBias > texture(uShadowMap, projected.xy).r)
                {
                    shadow = uShadowStrength;
                }
            }

            // REQ-SCN-028 for the diffuse half, with the metal rule of glTF 2.0 §B.
            vec3 diffuse = base * (1.0 - metallic);
            vec3 color = diffuse * (vec3(uAmbient) + ndl * uLightLinear * shadow);

            vec3 f0 = mix(vec3(0.04), base, metallic);
            vec3 fresnel = f0 + (vec3(1.0) - f0) * pow(1.0 - vdh, 5.0);
            vec3 specular = fresnel * zaxGgx(ndh, alpha) * zaxVisibility(ndl, ndv, alpha);
            color += uLightLinear * specular * ndl * shadow;

            // A metal lit by one directional light alone reads as black; the ambient term of
            // REQ-SCN-028 stands in for the missing environment.
            color += f0 * uAmbient * (1.0 - roughness) * metallic;

            if (uHasEmissive != 0)
            {
                color += zaxToLinear3(texture(uEmissive, vUv).rgb) * uEmissiveColor;
            }
            else
            {
                color += uEmissiveColor;
            }

            // R2-009: linear out; the sRGB encode is the resolve pass.
            oColor = vec4(clamp(color, 0.0, 1.0), 1.0);
        }
        """;

    /// <summary>Vertex shader of the unlit tunnel boxes of REQ-SCN-053.</summary>
    public const string TunnelVertex = """
        #version 330 core
        layout(location = 0) in vec3 aPosition;
        layout(location = 1) in vec3 aNormal;
        layout(location = 2) in vec2 aUv;

        uniform mat4 uModel;
        uniform mat4 uViewProjection;

        void main()
        {
            vec4 scene = uModel * vec4(aPosition, 1.0);
            gl_Position = uViewProjection * vec4(scene.x, scene.y, -scene.z, 1.0);
        }
        """;

    /// <summary>
    /// Fragment shader of the tunnel: one flat, unlit colour (REQ-SCN-053). <c>uColor</c> is
    /// the linear colour, like every other colour these shaders write (R2-009).
    /// </summary>
    public const string TunnelFragment = """
        #version 330 core
        out vec4 oColor;
        uniform vec3 uColor;

        void main()
        {
            oColor = vec4(uColor, 1.0);
        }
        """;
}
