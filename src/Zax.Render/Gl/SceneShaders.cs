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
    /// The space bending of SPEC-06 REQ-SCN-067 (AMD-043), the GLSL counterpart of
    /// <see cref="Zax.Engine3D.SpaceBend"/>. The per-axis constants <c>k</c>, <c>f</c> and
    /// <c>h</c> are computed once per frame on the CPU and handed over as uniforms, so a vertex
    /// only pays for the wrapping itself.
    /// </summary>
    /// <remarks>
    /// The Jacobian is carried along because REQ-SCN-068 has the shading normal transported by
    /// the curvature (amending REQ-SCN-028). Its cofactor matrix is built from the cross
    /// products of its columns, which needs neither <c>inverse</c> nor a special case for the
    /// singular Jacobian the depth limit produces.
    /// </remarks>
    private const string Bend = """
        uniform int uBend;
        uniform vec3 uBendK;
        uniform vec3 uBendF;
        uniform vec3 uBendH;
        uniform vec3 uBendOn;
        uniform float uBendPivot;

        const vec3 zaxBendCenter = vec3(128.0, 120.0, 0.0);
        const float zaxBendDepthLimit = 0.95;

        // E(k, f, h) of REQ-SCN-067 step 2, with its four partial derivatives in
        // d = (du/du, du/dw, dw/du, dw/dw).
        vec2 zaxWrap(float k, float f, float h, vec2 p, out vec4 d)
        {
            bool held = p.y * k > zaxBendDepthLimit;
            float depth = held ? zaxBendDepthLimit / k : p.y;
            float slope = held ? 0.0 : 1.0;

            float a = p.x * k;
            float s = sin(a);
            float c = cos(a);
            float radial = 1.0 - depth * k;

            d = vec4(f * radial * c, -f * slope * s, f * radial * s, f * slope * c);
            return vec2(f * radial * s / k, f * ((1.0 - c) / k + depth * c - h));
        }

        vec3 zaxBend(vec3 scene, out mat3 jacobian)
        {
            vec3 q = scene - zaxBendCenter;
            jacobian = mat3(1.0);
            vec4 d;

            if (uBendOn.x != 0.0)
            {
                vec2 r = zaxWrap(uBendK.x, uBendF.x, uBendH.x, vec2(q.x, q.z), d);
                q.x = r.x;
                q.z = r.y;
                jacobian = mat3(d.x, 0.0, d.z, 0.0, 1.0, 0.0, d.y, 0.0, d.w) * jacobian;
            }

            if (uBendOn.y != 0.0)
            {
                vec2 r = zaxWrap(uBendK.y, uBendF.y, uBendH.y, vec2(q.y, q.z), d);
                q.y = r.x;
                q.z = r.y;
                jacobian = mat3(1.0, 0.0, 0.0, 0.0, d.x, d.z, 0.0, d.y, d.w) * jacobian;
            }

            if (uBendOn.z != 0.0)
            {
                // The arc wraps (u, p − v) and reads the second component back as p − s.
                vec2 r = zaxWrap(uBendK.z, uBendF.z, uBendH.z, vec2(q.x, uBendPivot - q.y), d);
                q.x = r.x;
                q.y = uBendPivot - r.y;
                jacobian = mat3(d.x, -d.z, 0.0, -d.y, d.w, 0.0, 0.0, 0.0, 1.0) * jacobian;
            }

            return q + zaxBendCenter;
        }

        vec3 zaxBendNormal(mat3 jacobian, vec3 n)
        {
            vec3 k0 = cross(jacobian[1], jacobian[2]);
            vec3 k1 = cross(jacobian[2], jacobian[0]);
            vec3 k2 = cross(jacobian[0], jacobian[1]);
            vec3 t = k0 * n.x + k1 * n.y + k2 * n.z;
            return dot(t, t) > 1e-12 ? t : n;
        }
        """;

    /// <summary>
    /// The voxel grid of SPEC-06 REQ-SCN-081 (AMD-055): grooves on the cell edges of the frame
    /// the grid is evaluated in, which modulate the <b>albedo</b> before any lighting.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The frame is the one REQ-SCN-081 names, and it is the <see cref="SceneVertex"/> that
    /// chooses it: the local mesh frame of the tile (REQ-E3D-504, REQ-E3D-522) for a tile, the
    /// scene frame for the backdrop, both taken <b>before</b> the object transform and before
    /// the curvature, so the grooves are painted on the geometry and follow it.
    /// </para>
    /// <para>
    /// <c>uGridCell</c> is the cell side: <c>N = 2^spl.voxel_grid</c> voxels for a tile, and
    /// the same <c>N</c> in scene units for the backdrop. The amendment writes the backdrop
    /// edges at multiples of <c>N × scene scale</c> for a position in units of
    /// <c>scene × scale</c>; the renderer keeps positions in unscaled scene units, so the
    /// scale cancels out and the cell is <c>N</c> either way.
    /// </para>
    /// </remarks>
    private const string VoxelGrid = """
        uniform float uGridCell;

        // REQ-SCN-081: the groove covers 3.5 % of the cell on each side of an edge, the bright
        // edge runs from there to 16 %, and the factors of the three axes multiply.
        const float zaxGrooveHalfWidth = 0.035;
        const float zaxBrightEdge = 0.16;

        float zaxGridAxis(float p, float n, float k, float e)
        {
            // Distance to the nearest edge, as a fraction of the cell: d in [0, 0.5]. It is
            // computed before any test, so that fwidth() stays in uniform control flow.
            float cells = p / uGridCell;
            float d = abs(cells - floor(cells + 0.5));
            float pixel = max(fwidth(d) * 0.5, 1e-6);

            // An axis perpendicular to the face has a zero weight and makes no groove — both
            // factors below are then 1 — and the axes of the plane of the face have a weight
            // of 1.
            float w = clamp(4.0 * (1.0 - abs(n)), 0.0, 1.0);

            float groove = 1.0 - w * (1.0 - k);
            float outer = d <= zaxBrightEdge
                ? 1.0 + w * e * (zaxBrightEdge - d) / (zaxBrightEdge - zaxGrooveHalfWidth)
                : 1.0;

            // The step from the groove to the bright edge is smoothed over about one screen
            // pixel, as the requirement asks; at d = 0.16 the factor is already 1, so that
            // transition is continuous on its own.
            float t = smoothstep(zaxGrooveHalfWidth - pixel, zaxGrooveHalfWidth + pixel, d);
            return mix(groove, outer, t);
        }

        float zaxVoxelGrid(vec3 p, vec3 n)
        {
            // uGridCell is 0 wherever the grid does not apply, and the whole draw call shares
            // that value: the branch is uniform, so leaving early does not disturb fwidth().
            if (uGridCell <= 0.0)
            {
                return 1.0;
            }

            // Alignment on an axis: m >= 0.9 gives t = 1 — every GREEDY face (REQ-E3D-505) —
            // and m <= 0.8 gives t = 0, a face at 45° or more oblique.
            vec3 a = abs(n);
            float m = max(a.x, max(a.y, a.z));
            float t = clamp((m - 0.8) / 0.1, 0.0, 1.0);
            float k = 0.38 + 0.24 * t;
            float e = 0.20 - 0.10 * t;

            return zaxGridAxis(p.x, n.x, k, e)
                 * zaxGridAxis(p.y, n.y, k, e)
                 * zaxGridAxis(p.z, n.z, k, e);
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

        // AMD-055 REQ-SCN-081: 0 to take the grid frame from the local mesh frame of the tile
        // (REQ-E3D-504), 1 to take it from the scene frame, which is what the backdrop uses.
        uniform int uGridScene;

        {{Deformation}}

        {{Bend}}

        out vec3 vScene;
        out vec3 vBent;
        out vec3 vNormal;
        out vec2 vUv;
        out vec3 vGridPos;
        out vec3 vGridNormal;
        out vec3 vLocal;
        out vec3 vLocalNormal;

        void main()
        {
            vec3 local = aPosition;
            if (uDeform != 0)
            {
                local += zaxDeformation(aPosition);
            }

            vec4 scene = uModel * vec4(local, 1.0);

            // REQ-SCN-068: the clipping of REQ-SCN-036 is evaluated on the position *before*
            // bending, so a tile the margins keep stays visible wherever its bent image lands.
            vScene = scene.xyz;

            // REQ-SCN-028: the normal is the mesh normal, unchanged by the deformation.
            // uNormalSign is −1 for a mirrored placement, whose cofactor matrix points the
            // normal inwards (Q-069).
            vec3 sceneNormal = uNormalSign * (uNormalMatrix * aNormal);
            vNormal = sceneNormal;

            // AMD-055 REQ-SCN-081 and REQ-SCN-082: the raw mesh position and normal, before
            // the animated deformation, before the object transform and before the curvature.
            // In MARCHING the attribute already carries the flat normal of the triangle
            // (REQ-E3D-522), so one varying covers both algorithms.
            vLocal = aPosition;
            vLocalNormal = aNormal;

            // REQ-SCN-081: the grid of a tile lives in that local frame, the grid of the
            // backdrop in the scene frame — in both cases before the curvature.
            vGridPos = uGridScene != 0 ? scene.xyz : aPosition;
            vGridNormal = uGridScene != 0 ? sceneNormal : aNormal;

            vec3 placed = scene.xyz;
            if (uBend != 0)
            {
                // REQ-SCN-067, applied last: after the animated deformation and after the
                // object's own transform. REQ-SCN-068 transports the normal with it.
                mat3 jacobian;
                placed = zaxBend(scene.xyz, jacobian);
                vNormal = zaxBendNormal(jacobian, vNormal);
            }

            vBent = placed;
            vUv = aUv;
            gl_Position = uViewProjection * vec4(placed.x, placed.y, -placed.z, 1.0);
        }
        """;

    /// <summary>
    /// The backdrop material of SPEC-06 REQ-SCN-075 (AMD-049): the Standard reflection model of
    /// a non-metallic, almost matt surface, which replaces for the backdrop — and for it alone —
    /// the diffuse term REQ-SCN-048 used to inherit from REQ-SCN-028.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The model is given in full by the requirement, which sources it on the public shader
    /// sources of Unity; the CPU twin of this function is
    /// <see cref="Zax.Render.SceneColor.ShadeBackdropLinear"/> and the two are checked against
    /// the same test vectors.
    /// </para>
    /// <para>
    /// There is no environment reflection: in shadow the specular part is reduced by the very
    /// same factor <c>s</c> as the direct diffuse part, so the colour does not depend on the
    /// Skybox setting (REQ-SCN-049).
    /// </para>
    /// </remarks>
    private const string Backdrop = """
        const float zaxBackdropRoughness = 0.99;
        const float zaxPi = 3.14159265358979;

        float zaxPow5(float x)
        {
            float square = x * x;
            return square * square * x;
        }

        vec3 zaxBackdrop(vec3 albedo, vec3 n, vec3 v, vec3 l, vec3 lightLinear, float ambient, float shadow)
        {
            // REQ-SCN-075: H = (L + V) normalised, and H = N when L + V = 0.
            vec3 sum = l + v;
            vec3 h = dot(sum, sum) > 0.0 ? normalize(sum) : n;

            float nl = max(dot(n, l), 0.0);
            float nv = abs(dot(n, v));
            float nh = max(dot(n, h), 0.0);
            float lh = max(dot(l, h), 0.0);

            float rough = zaxBackdropRoughness;
            float alpha = rough * rough;
            float alpha2 = alpha * alpha;

            float f90 = 0.5 + 2.0 * lh * lh * rough;
            float fd = (1.0 + (f90 - 1.0) * zaxPow5(1.0 - nl))
                     * (1.0 + (f90 - 1.0) * zaxPow5(1.0 - nv));

            float vis = 0.5 / (nl * (nv * (1.0 - alpha) + alpha)
                             + nv * (nl * (1.0 - alpha) + alpha) + 1e-5);
            float denominator = nh * nh * (alpha2 - 1.0) + 1.0;
            float d = alpha2 / (zaxPi * denominator * denominator);
            float spec = vis * d * zaxPi * nl;
            float f = 0.04 + 0.96 * zaxPow5(1.0 - lh);

            return albedo * 0.96 * (vec3(ambient) + shadow * lightLinear * fd * nl)
                 + shadow * lightLinear * spec * f;
        }
        """;

    /// <summary>
    /// The pixel shadows of SPEC-06 REQ-SCN-082 (AMD-055): the shadow is read not at the
    /// fragment but at the centre of the fragment's voxel, so it changes by squares of one
    /// voxel, like the pixel grid of the game.
    /// </summary>
    /// <remarks>
    /// The point is moved in the local mesh frame, then transported <b>as a vertex</b> — the
    /// animated deformation, then the object's scale, rotation and position in the order of
    /// REQ-E3D-406, then the curvature — which is why the fragment shader carries the
    /// deformation and bending code the vertex shader uses.
    /// </remarks>
    private const string PixelShadow = """
        uniform int uPixelShadow;
        uniform float uBuildDepth;

        vec3 zaxVoxelCentre(vec3 p, vec3 n)
        {
            // REQ-SCN-082 step 2: the two axes of the plane of the face are the two of
            // smallest |n_a|, ties going to the smallest index in the order x, y, z — so the
            // axis left alone is the one of largest |n_a|, ties going to the largest index.
            vec3 a = abs(n);

            // "half" is a reserved word in GLSL, hence the name.
            float halfDepth = uBuildDepth * 0.5;
            vec3 centre = vec3(
                floor(p.x) + 0.5,
                floor(p.y) + 0.5,

                // On z the voxel boundaries are offset by D/2 (REQ-E3D-504).
                floor(p.z + halfDepth) + 0.5 - halfDepth);

            if (a.z >= a.x && a.z >= a.y)
            {
                centre.z = p.z;
            }
            else if (a.y >= a.x)
            {
                centre.y = p.y;
            }
            else
            {
                centre.x = p.x;
            }

            return centre;
        }

        // REQ-SCN-082 step 3: the displaced point, carried to the scene like a vertex.
        vec3 zaxPlaceLikeVertex(vec3 local)
        {
            vec3 v = local;
            if (uDeform != 0)
            {
                v += zaxDeformation(local);
            }

            vec3 scene = (uModel * vec4(v, 1.0)).xyz;
            if (uBend != 0)
            {
                mat3 jacobian;
                scene = zaxBend(scene, jacobian);
            }

            return scene;
        }
        """;

    /// <summary>
    /// Fragment shader of the scene pass: albedo of REQ-SCN-034, visible-rectangle clipping of
    /// REQ-SCN-036, the fragment rejection of REQ-SCN-076, hard shadow of REQ-SCN-030/031, the
    /// pixel shadows of REQ-SCN-082 and the voxel grid of REQ-SCN-081, the Lambert shading of
    /// REQ-SCN-028 for the tiles and the HUD, and the Standard model of REQ-SCN-075 for the
    /// backdrop.
    /// </summary>
    public static string SceneFragment { get; } = $$"""
        #version 330 core
        in vec3 vScene;
        in vec3 vBent;
        in vec3 vNormal;
        in vec2 vUv;
        in vec3 vGridPos;
        in vec3 vGridNormal;
        in vec3 vLocal;
        in vec3 vLocalNormal;
        out vec4 oColor;

        uniform mat4 uModel;

        // 0 = indexed atlas, 1 = direct colour atlas, 2 = flat colour (backdrop).
        uniform int uColorMode;

        // REQ-SCN-076: non-zero on the 2D-mode rectangle alone, where a texel of alpha 0
        // rejects the fragment instead of being blended.
        uniform int uCutout;
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

        // REQ-SCN-075: the backdrop's model needs the eye, the only view-dependent term of the
        // scene pass.
        uniform vec3 uEye;

        uniform int uReceiveShadow;
        uniform sampler2D uShadowMap;
        uniform mat4 uLightViewProjection;
        uniform float uShadowBias;
        uniform float uNormalOffset;
        uniform float uShadowStrength;

        {{Transfer}}

        {{Backdrop}}

        {{Deformation}}

        {{Bend}}

        {{VoxelGrid}}

        {{PixelShadow}}

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

            // AMD-050 REQ-SCN-076: a pixel of the 2D-mode rectangle whose opacity bit is 0 is
            // not drawn at all. Rejecting the fragment — rather than blending it — keeps the
            // rectangle in the opaque pass of REQ-SCN-058, out of the transparent class of
            // REQ-SCN-040, and makes the result independent of video.anti_alias: the test is
            // binary per fragment, so no sprite edge comes out partly transparent.
            if (uCutout != 0 && texelAlpha < 0.5)
            {
                discard;
            }

            vec3 n = normalize(vNormal);
            vec3 l = normalize(-uLightDirection);
            float ndl = max(0.0, dot(n, l));

            // AMD-055 REQ-SCN-081: the grid modulates the albedo before any lighting — the
            // term c of REQ-SCN-075 for the backdrop, so a groove never darkens the specular
            // part. It multiplies the *linear* albedo, which is what REQ-SCN-028 calls
            // albedo_lin and what the acceptance ratios of the amendment are read on.
            // uGridCell is 0 wherever the grid does not apply: the HUD, the skybox and the
            // 2D-mode rectangle (and in Classic, where no effect of the style exists).
            vec3 albedoLinear = zaxToLinear3(albedo)
                * zaxVoxelGrid(vGridPos, normalize(vGridNormal));

            float shadow = 1.0;
            if (uReceiveShadow != 0)
            {
                // AMD-055 REQ-SCN-082: with the pixel shadows on, the lookup point is the
                // centre of the fragment's voxel, carried to the scene like a vertex, so the
                // shadow changes by squares of one voxel. The bias below is applied to that
                // displaced point and not to the fragment, which keeps shadow acne away.
                vec3 lookup = uPixelShadow != 0
                    ? zaxPlaceLikeVertex(zaxVoxelCentre(vLocal, vLocalNormal))
                    : vBent;

                // REQ-SCN-031: a normal offset and a depth bias of about 0.02 unit. The
                // shadow map holds the bent geometry (REQ-SCN-068), so the lookup uses the
                // bent position.
                vec3 biased = lookup + n * uNormalOffset;
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

            vec3 lit;
            if (uColorMode == 2)
            {
                // REQ-SCN-075 (AMD-049): the backdrop follows the Standard model, which is the
                // only place in the scene pass where the eye matters. V points at the eye from
                // the bent position, the one the geometry really occupies (REQ-SCN-068).
                vec3 toEye = uEye - vBent;
                vec3 v = dot(toEye, toEye) > 0.0 ? normalize(toEye) : n;
                lit = zaxBackdrop(albedoLinear, n, v, l, uLightLinear, uAmbient, shadow);
            }
            else
            {
                // REQ-SCN-028: albedo_lin * (ambient_lin + max(0, N.L) * light_lin * S).
                // R2-009: written linear, so that the blending of REQ-SCN-040 is linear too.
                lit = albedoLinear * (vec3(uAmbient) + ndl * uLightLinear * shadow);
            }

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

        {{Bend}}

        void main()
        {
            vec3 local = aPosition;
            if (uDeform != 0)
            {
                local += zaxDeformation(aPosition);
            }

            vec4 scene = uModel * vec4(local, 1.0);

            // REQ-SCN-068: the shadow pass is bent too, so the shadows follow the curved
            // surface instead of sliding off it.
            vec3 placed = scene.xyz;
            if (uBend != 0)
            {
                mat3 jacobian;
                placed = zaxBend(scene.xyz, jacobian);
            }

            gl_Position = uLightViewProjection * vec4(placed.x, placed.y, -placed.z, 1.0);
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
