namespace Zax.Render.Gl;

/// <summary>
/// The GLSL sources of the renderer, written against SPEC-06 §3.9 to §3.11.
/// </summary>
/// <remarks>
/// <para>
/// The original's shaders are DirectX bytecode and were not extracted (see
/// <c>assets-neutral/INVENTORY.md</c>); these are written from the specification. OpenGL 3.3
/// core, no extension (REQ-SCN-056).
/// </para>
/// <para>
/// AMD-021 R2-009: every colour these shaders write is <em>linear</em>. The blending of
/// REQ-SCN-040 therefore happens on linear colours, and the sRGB encode is the job of
/// <see cref="ResolveFragment"/>, the single pass that copies
/// <see cref="LinearColorBuffer"/> into the host framebuffer.
/// </para>
/// </remarks>
internal static class SceneShaders
{
    /// <summary>The sRGB transfer functions of SPEC-06 §2, shared by every shader.</summary>
    public const string Transfer = """
        float zaxToLinear(float c)
        {
            return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
        }

        float zaxToSrgb(float c)
        {
            return c <= 0.0031308 ? c * 12.92 : 1.055 * pow(c, 1.0 / 2.4) - 0.055;
        }

        vec3 zaxToLinear3(vec3 c)
        {
            return vec3(zaxToLinear(c.r), zaxToLinear(c.g), zaxToLinear(c.b));
        }

        vec3 zaxToSrgb3(vec3 c)
        {
            return vec3(zaxToSrgb(c.r), zaxToSrgb(c.g), zaxToSrgb(c.b));
        }
        """;

    /// <summary>
    /// The animated deformation of SPEC-04B REQ-E3D-568, applied in the object's local frame
    /// before the scale and the rotation (DEC-04B-Q2, REQ-SCN-043).
    /// </summary>
    private const string Deformation = """
        uniform int uDeform;
        uniform vec3 uDeformAmplitude;
        uniform vec3 uDeformSize;
        uniform vec3 uDeformTile;
        uniform vec4 uDeformSpeed[3];
        uniform vec4 uDeformPhase[3];
        uniform float uTime;

        float zaxDeformRow(float amplitude, float size, vec3 n, vec4 speed, vec4 phase)
        {
            return amplitude * size
                 * sin(n.x * speed.x + phase.x)
                 * sin(n.y * speed.y + phase.y)
                 * sin(n.z * speed.z + phase.z)
                 * sin(uTime * speed.w + phase.w);
        }

        vec3 zaxDeformation(vec3 v)
        {
            vec3 p = uDeformTile + v;
            vec3 n = vec3(
                uDeformSize.x == 0.0 ? 0.0 : p.x / uDeformSize.x,
                uDeformSize.y == 0.0 ? 0.0 : p.y / uDeformSize.y,
                uDeformSize.z == 0.0 ? 0.0 : p.z / uDeformSize.z);
            return vec3(
                zaxDeformRow(uDeformAmplitude.x, uDeformSize.x, n, uDeformSpeed[0], uDeformPhase[0]),
                zaxDeformRow(uDeformAmplitude.y, uDeformSize.y, n, uDeformSpeed[1], uDeformPhase[1]),
                zaxDeformRow(uDeformAmplitude.z, uDeformSize.z, n, uDeformSpeed[2], uDeformPhase[2]));
        }
        """;

    /// <summary>
    /// Vertex shader of the scene pass. The scene frame is left-handed (REQ-E3D-400), so the
    /// clip position is built from <c>(x, y, −z)</c> (REQ-SCN-001).
    /// </summary>
    public static string SceneVertex { get; } = $$"""
        #version 330 core
        layout(location = 0) in vec3 aPosition;
        layout(location = 1) in vec3 aNormal;
        layout(location = 2) in vec2 aUv;

        uniform mat4 uModel;
        uniform mat3 uNormalMatrix;
        uniform float uNormalSign;
        uniform mat4 uViewProjection;

        {{Deformation}}

        out vec3 vScene;
        out vec3 vNormal;
        out vec2 vUv;

        void main()
        {
            vec3 local = aPosition;
            if (uDeform != 0)
            {
                local += zaxDeformation(aPosition);
            }

            vec4 scene = uModel * vec4(local, 1.0);
            vScene = scene.xyz;
            // REQ-SCN-028: the normal is the mesh normal, unchanged by the deformation.
            // uNormalSign is −1 for a mirrored placement, whose cofactor matrix points the
            // normal inwards (Q-069).
            vNormal = uNormalSign * (uNormalMatrix * aNormal);
            vUv = aUv;
            gl_Position = uViewProjection * vec4(scene.x, scene.y, -scene.z, 1.0);
        }
        """;

    /// <summary>
    /// Fragment shader of the scene pass: albedo of REQ-SCN-034, visible-rectangle clipping of
    /// REQ-SCN-036, hard shadow of REQ-SCN-030/031 and the Lambert shading of REQ-SCN-028.
    /// </summary>
    public static string SceneFragment { get; } = $$"""
        #version 330 core
        in vec3 vScene;
        in vec3 vNormal;
        in vec2 vUv;
        out vec4 oColor;

        // 0 = indexed atlas, 1 = direct colour atlas, 2 = flat colour (backdrop).
        uniform int uColorMode;
        uniform usampler2D uIndexTexture;
        uniform sampler2D uColorTexture;
        uniform sampler2D uPalette;
        uniform int uPaletteTable;
        uniform vec2 uTextureSize;
        uniform vec2 uTexOffset;
        uniform vec2 uTexTilting;
        uniform vec3 uFlatColor;

        uniform int uClip;
        uniform vec4 uClipRect;

        uniform vec3 uLightDirection;
        uniform vec3 uLightLinear;
        uniform float uAmbient;
        uniform float uAlpha;

        uniform int uReceiveShadow;
        uniform sampler2D uShadowMap;
        uniform mat4 uLightViewProjection;
        uniform float uShadowBias;
        uniform float uNormalOffset;
        uniform float uShadowStrength;

        {{Transfer}}

        void main()
        {
            // REQ-SCN-036: a fragment whose scene (x, y) leaves R is dropped, whatever its depth.
            if (uClip != 0 && (vScene.x < uClipRect.x || vScene.x > uClipRect.z
                            || vScene.y < uClipRect.y || vScene.y > uClipRect.w))
            {
                discard;
            }

            vec3 albedo;
            float texelAlpha = 1.0;
            if (uColorMode == 2)
            {
                albedo = uFlatColor;
            }
            else
            {
                // REQ-E3D-563: nearest neighbour, no mipmap, texel coordinates taken modulo
                // the texture size.
                vec2 uv = vec2(
                    vUv.x * uTexTilting.x + uTexOffset.x * uTextureSize.x,
                    vUv.y * uTexTilting.y + uTexOffset.y * uTextureSize.y);
                ivec2 texel = ivec2(mod(floor(uv), uTextureSize));
                if (uColorMode == 0)
                {
                    // REQ-SCN-034: the index selects 4 * P + (t mod 4) in the frame palette.
                    uint value = texelFetch(uIndexTexture, texel, 0).r;
                    int entry = clamp(uPaletteTable, 0, 15) * 4 + int(value & 3u);
                    albedo = texelFetch(uPalette, ivec2(entry, 0), 0).rgb;
                }
                else
                {
                    vec4 direct = texelFetch(uColorTexture, texel, 0);
                    albedo = direct.rgb;
                    texelAlpha = direct.a;
                }
            }

            vec3 n = normalize(vNormal);
            vec3 l = normalize(-uLightDirection);
            float ndl = max(0.0, dot(n, l));

            float shadow = 1.0;
            if (uReceiveShadow != 0)
            {
                // REQ-SCN-031: a normal offset and a depth bias of about 0.02 unit.
                vec3 biased = vScene + n * uNormalOffset;
                vec4 light = uLightViewProjection * vec4(biased.x, biased.y, -biased.z, 1.0);
                vec3 projected = light.xyz / light.w * 0.5 + 0.5;
                if (projected.x >= 0.0 && projected.x <= 1.0
                    && projected.y >= 0.0 && projected.y <= 1.0
                    && projected.z <= 1.0)
                {
                    float stored = texture(uShadowMap, projected.xy).r;
                    if (projected.z - uShadowBias > stored)
                    {
                        shadow = uShadowStrength;
                    }
                }
            }

            // REQ-SCN-028: albedo_lin * (ambient_lin + max(0, N.L) * light_lin * S).
            // R2-009: written linear, so that the blending of REQ-SCN-040 is linear too.
            vec3 lit = zaxToLinear3(albedo) * (vec3(uAmbient) + ndl * uLightLinear * shadow);
            oColor = vec4(min(lit, vec3(1.0)), uAlpha * texelAlpha);
        }
        """;

    /// <summary>
    /// Vertex shader of the resolve pass: one full-screen triangle, built from the vertex
    /// index alone.
    /// </summary>
    public const string ResolveVertex = """
        #version 330 core
        out vec2 vUv;

        void main()
        {
            vec2 corner = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
            vUv = corner;
            gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
        }
        """;

    /// <summary>
    /// Fragment shader of the resolve pass: the sRGB encode of SPEC-06 §2, applied once, after
    /// the blending (AMD-021 R2-009).
    /// </summary>
    public static string ResolveFragment { get; } = $$"""
        #version 330 core
        in vec2 vUv;
        out vec4 oColor;

        uniform sampler2D uLinear;

        {{Transfer}}

        void main()
        {
            vec4 linear = texture(uLinear, vUv);

            // REQ-SCN-035: the alpha channel is never converted.
            oColor = vec4(zaxToSrgb3(clamp(linear.rgb, 0.0, 1.0)), linear.a);
        }
        """;

    /// <summary>Vertex shader of the shadow pass: the same placement, depth only.</summary>
    public static string ShadowVertex { get; } = $$"""
        #version 330 core
        layout(location = 0) in vec3 aPosition;
        layout(location = 1) in vec3 aNormal;
        layout(location = 2) in vec2 aUv;

        uniform mat4 uModel;
        uniform mat4 uLightViewProjection;

        {{Deformation}}

        void main()
        {
            vec3 local = aPosition;
            if (uDeform != 0)
            {
                local += zaxDeformation(aPosition);
            }

            vec4 scene = uModel * vec4(local, 1.0);
            gl_Position = uLightViewProjection * vec4(scene.x, scene.y, -scene.z, 1.0);
        }
        """;

    /// <summary>Fragment shader of the shadow pass: writes nothing but the depth.</summary>
    public const string ShadowFragment = """
        #version 330 core

        void main()
        {
        }
        """;
}
