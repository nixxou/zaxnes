using System.Numerics;
using Zax.Profiles;

namespace Zax.Render;

/// <summary>
/// The visible rectangle <c>R</c> of SPEC-06 REQ-SCN-003 and SPEC-04B REQ-E3D-410:
/// <c>R = [c0, 256 − c2] × [c1, 240 − c3]</c>, in scene units.
/// </summary>
/// <param name="MinX">Left edge, <c>c0</c>.</param>
/// <param name="MinY">Bottom edge, <c>c1</c>.</param>
/// <param name="MaxX">Right edge, <c>256 − c2</c>.</param>
/// <param name="MaxY">Top edge, <c>240 − c3</c>.</param>
public readonly record struct VisibleRect(float MinX, float MinY, float MaxX, float MaxY)
{
    /// <summary>Builds the rectangle from the four clip margins (DEC-R2-T1).</summary>
    public static VisibleRect From(Vec4F clipping) => new(
        clipping.X,
        clipping.Y,
        SceneConstants.FrameWidth - clipping.Z,
        SceneConstants.FrameHeight - clipping.W);

    /// <summary>Width of the rectangle.</summary>
    public float Width => MaxX - MinX;

    /// <summary>Height of the rectangle.</summary>
    public float Height => MaxY - MinY;

    /// <summary>Centre of the rectangle, in the <c>z = 0</c> plane.</summary>
    public Vector3 Center => new((MinX + MaxX) * 0.5f, (MinY + MaxY) * 0.5f, 0f);

    /// <summary>Whether the scene point <c>(x, y)</c> survives the clipping of REQ-SCN-036.</summary>
    public bool Contains(float x, float y) => x >= MinX && x <= MaxX && y >= MinY && y <= MaxY;
}

/// <summary>
/// The scene constants of SPEC-06 §4.3 and the closed-form geometry they imply: the reference
/// frame, the default camera placement, the corrected aim point and the HUD plane.
/// </summary>
public static class SceneConstants
{
    /// <summary>Width of the reference frame, 256 scene units (REQ-E3D-410).</summary>
    public const float FrameWidth = 256f;

    /// <summary>Height of the reference frame, 240 scene units (REQ-E3D-410).</summary>
    public const float FrameHeight = 240f;

    /// <summary>Height of the conversion grid, <c>Hs = 248</c> (REQ-E3D-410).</summary>
    public const float SceneHeight = 248f;

    /// <summary>The centre <c>C = (128, 120, 0)</c> of the reference frame (REQ-E3D-410).</summary>
    public static Vector3 Center { get; } = new(128f, 120f, 0f);

    /// <summary>The default setpoint angles, <c>(0.81°, 0.81°, 0)</c> (REQ-SCN-012).</summary>
    public static Vector3 DefaultAngles { get; } = new(0.81f, 0.81f, 0f);

    /// <summary>Near plane, 0.3 unit (REQ-SCN-004).</summary>
    public const float NearPlane = 0.3f;

    /// <summary>Far plane, 5000 units (REQ-SCN-004).</summary>
    public const float FarPlane = 5000f;

    /// <summary>Default field of view in degrees (REQ-SCN-005).</summary>
    public const float DefaultFov = 70f;

    /// <summary>Lowest field of view in degrees (REQ-SCN-005).</summary>
    public const float MinFov = 45f;

    /// <summary>Highest field of view in degrees (REQ-SCN-005).</summary>
    public const float MaxFov = 95f;

    /// <summary>Highest camera setpoint distance (REQ-SCN-011).</summary>
    public const float MaxDistance = 512f;

    /// <summary>Rotation speed of the extended actions, 30°/s (REQ-SCN-014).</summary>
    public const float RotationDegreesPerSecond = 30f;

    /// <summary>Zoom speed of the extended actions, 70 units/s (REQ-SCN-015).</summary>
    public const float ZoomUnitsPerSecond = 70f;

    /// <summary>Distance change of one mouse wheel notch, 5 units (REQ-SCN-016, DEC-08-3).</summary>
    public const float WheelUnitsPerNotch = 5f;

    /// <summary>Rotation per pixel of mouse drag, 0.1° (REQ-SCN-017).</summary>
    public const float DragDegreesPerPixel = 0.1f;

    /// <summary>Length of the mode transition window, 3 s (REQ-SCN-006, REQ-SCN-020).</summary>
    public const float ModeTransitionSeconds = 3f;

    /// <summary>Radius of the scene the shadow map has to cover, 512 units (REQ-SCN-030).</summary>
    public const float ShadowDistance = 512f;

    /// <summary>
    /// Default shadow map side in texels: 4096 since AMD-034 §2, which replaces the 1024 of
    /// DEC-R2-D4.
    /// </summary>
    /// <remarks>
    /// At 1024 texels one texel spans about 0.4 unit on the world map of Super Mario Bros. 3,
    /// more than a status-bar glyph is wide, so the shadow of a one-unit stroke breaks into
    /// dots: 920 shadowed pixels in the status bar against 5 188 in the original (AMD-034 §1).
    /// The covered area, the single map and the hard edges of REQ-SCN-030 do not change.
    /// </remarks>
    public const int ShadowMapSize = 4096;

    /// <summary>
    /// Smallest shadow map the fallback of AMD-034 §2 ever settles for. A size the hardware
    /// refuses is halved — 4096, then 2048, then 1024 — but a map asked for explicitly below
    /// this floor (the seam tests use 128) is still honoured.
    /// </summary>
    public const int MinShadowMapSize = 64;

    /// <summary>
    /// The shadow map sizes to try, largest first, for a map of <paramref name="requested"/>
    /// texels on hardware that advertises <paramref name="maxTextureSize"/> (AMD-034 §2).
    /// </summary>
    /// <param name="requested">The size asked for, in texels.</param>
    /// <param name="maxTextureSize">What <c>GL_MAX_TEXTURE_SIZE</c> answered, in texels.</param>
    /// <returns>
    /// The requested size capped by the hardware limit, then halved — 4096, 2048, 1024 for the
    /// default — down to <see cref="MinShadowMapSize"/>.
    /// </returns>
    /// <remarks>
    /// OpenGL 3.3 only guarantees <c>GL_MAX_TEXTURE_SIZE</c> ≥ 1024, so a 4096-texel map is not
    /// a size every implementation has to accept. The cap skips the sizes the driver already
    /// says it cannot hold; the rest of the chain is there for the ones it accepts to allocate
    /// but whose framebuffer comes out incomplete.
    /// </remarks>
    public static IEnumerable<int> ShadowMapSizes(int requested, int maxTextureSize)
    {
        int start = Math.Min(
            Math.Max(MinShadowMapSize, requested),
            Math.Max(MinShadowMapSize, maxTextureSize));
        for (int size = start; size >= MinShadowMapSize; size /= 2)
        {
            yield return size;
        }
    }

    /// <summary>
    /// The depth bias of the shadow test in scene units: <see cref="ShadowBiasTexels"/> texels
    /// of the map in use, never under <see cref="ShadowDepthBias"/> (AMD-034 §2, amending
    /// Q-032).
    /// </summary>
    /// <param name="texelSize">Side of one shadow-map texel, in scene units.</param>
    public static float ShadowBias(float texelSize)
        => MathF.Max(ShadowDepthBias, ShadowBiasTexels * texelSize);

    /// <summary>
    /// The depth bias of REQ-SCN-031, about 0.02 unit; the floor of <c>ShadowBias</c>, which
    /// AMD-034 §2 keeps as the value the texel-sized bias may never go under.
    /// </summary>
    public const float ShadowDepthBias = 0.02f;

    /// <summary>The normal offset of REQ-SCN-031, about 0.02 unit.</summary>
    public const float ShadowNormalOffset = 0.02f;

    /// <summary>
    /// How many shadow-map texels the depth bias has to cover: one texel since AMD-034 §2,
    /// which lowers the 1.5 of Q-032.
    /// </summary>
    /// <remarks>
    /// A receiving fragment and the texel it samples are up to one texel apart on the surface,
    /// and the light crosses that texel at an angle, so the depth they disagree by is of the
    /// order of the texel (Q-032 took its diagonal). AMD-034 §2 takes the texel itself: the bias
    /// is subtracted from every shadow, so on a map four times finer it was shortening the very
    /// shadows of thin strokes the amendment is about, and a 4096-texel map makes the margin the
    /// extra half texel bought unnecessary.
    /// </remarks>
    public const float ShadowBiasTexels = 1f;

    /// <summary>Highest sample count the anti-aliasing of AMD-032 §2 ever asks for.</summary>
    public const int MaxAntiAliasSamples = 8;

    /// <summary>
    /// Samples per pixel the <c>video.anti_alias</c> option of SPEC-09 REQ-PERS-017 stands for:
    /// 0 off, 1 four samples, 2 eight samples (AMD-032 §2).
    /// </summary>
    /// <param name="option">The stored option, 0 to 2; anything else means off.</param>
    /// <returns>1, 4 or 8.</returns>
    /// <remarks>
    /// REQ-PERS-017 lets the reimplementation apply a full-screen anti-aliasing and names FXAA
    /// and SMAA for the two levels. AMD-032 takes multisampling instead: the scene is a mass of
    /// hard-edged flat faces with nearest-neighbour texturing (REQ-SCN-034), so a
    /// post-processing filter would blur the pixel art it is supposed to keep crisp, while
    /// multisampling only touches the geometric edges.
    /// </remarks>
    public static int AntiAliasSamples(int option) => option switch
    {
        1 => 4,
        2 => MaxAntiAliasSamples,
        _ => 1,
    };

    /// <summary>
    /// Rounds a requested sample count down to a power of two no greater than
    /// <paramref name="supported"/>, which is what a multisampled attachment can be asked for.
    /// </summary>
    /// <param name="samples">Requested samples per pixel.</param>
    /// <param name="supported">What the implementation reports as its maximum.</param>
    /// <returns>1 when anti-aliasing is off or nothing above one sample is supported.</returns>
    public static int ClampAntiAliasSamples(int samples, int supported)
    {
        int limit = Math.Min(Math.Max(samples, 1), Math.Max(supported, 1));
        int granted = 1;
        while (granted * 2 <= limit)
        {
            granted *= 2;
        }

        return granted;
    }

    /// <summary>Scale factor of the HUD plane, 0.5 (REQ-SCN-045).</summary>
    public const float HudScale = 0.5f;

    /// <summary>Depth nudge applied by the renderer to sprite tiles, ±0.1 (REQ-SCN-037).</summary>
    public const float SpriteNudge = 0.1f;

    /// <summary>Opacity of a tile that hides the followed shape in FPV (REQ-SCN-022).</summary>
    public const float OccluderAlpha = 0.5f;

    /// <summary>
    /// The default setpoint distance <c>d₀ = 104 / atan(FOV_rad / 2)</c> (REQ-SCN-012). The
    /// arc tangent really is taken of half the field of view expressed in radians.
    /// </summary>
    public static float DefaultDistance(float fovDegrees)
        => 104f / MathF.Atan(fovDegrees * MathF.PI / 180f / 2f);

    /// <summary>The HUD plane distance <c>d_HUD = 60 / atan(FOV_rad / 2)</c> (REQ-SCN-045).</summary>
    public static float HudDistance(float fovDegrees)
        => 60f / MathF.Atan(fovDegrees * MathF.PI / 180f / 2f);

    /// <summary>
    /// The rotation <c>q = R_Y(ry) · R_X(rx) · R_Z(rz)</c> of REQ-SCN-007 step 1, in the
    /// left-handed scene frame, or <c>q = R_Z(ry) · R_X(rx)</c> when the local camera setting
    /// is on (REQ-SCN-009).
    /// </summary>
    /// <param name="angles">Angles <c>(rx, ry, rz)</c> in degrees.</param>
    /// <param name="localCamera">Whether the local camera setting is on.</param>
    /// <returns>The three columns of <c>q</c>: the images of X, Y and Z.</returns>
    public static (Vector3 Right, Vector3 Up, Vector3 Forward) Rotation(Vector3 angles, bool localCamera)
    {
        Matrix4x4 x = RotX(angles.X);
        Matrix4x4 q = localCamera
            ? Multiply(RotZ(angles.Y), x)
            : Multiply(Multiply(RotY(angles.Y), x), RotZ(angles.Z));

        return (Column(q, 0), Column(q, 1), Column(q, 2));
    }

    /// <summary>
    /// The forward direction <c>f = q · (0, 0, 1)</c> of REQ-SCN-007 step 3, which for
    /// <c>rz = 0</c> is <c>(cos rx · sin ry, −sin rx, cos rx · cos ry)</c>.
    /// </summary>
    public static Vector3 Forward(Vector3 angles, bool localCamera)
        => Rotation(angles, localCamera).Forward;

    /// <summary>
    /// The effective target <c>T′ = T + ((c0 − c2) / 2, (c1 − c3) / 2, 0)</c> of REQ-SCN-007
    /// step 2: the target recentred on the visible rectangle. FPV does not recentre
    /// (REQ-SCN-010).
    /// </summary>
    public static Vector3 EffectiveTarget(Vector3 target, Vec4F clipping) => target + new Vector3(
        (clipping.X - clipping.Z) * 0.5f,
        (clipping.Y - clipping.W) * 0.5f,
        0f);

    /// <summary>
    /// The aimed point <c>P*</c> of REQ-SCN-008: in the <c>z = 0</c> plane, the intersection of
    /// <c>M1M2</c> and <c>M3M4</c>, where each <c>Mi</c> is where the bisector issued from the
    /// eye of the angle under which the eye sees the segment lands.
    /// </summary>
    /// <param name="eye">The eye <c>E</c>.</param>
    /// <param name="rect">The visible rectangle.</param>
    /// <param name="fallback">
    /// The point returned when the two lines are parallel, in practice the effective target.
    /// </param>
    public static Vector3 AimPoint(Vector3 eye, VisibleRect rect, Vector3 fallback)
    {
        Vector3 a = new(rect.MinX, rect.MinY, 0f);
        Vector3 b = new(rect.MaxX, rect.MinY, 0f);
        Vector3 c = new(rect.MinX, rect.MaxY, 0f);
        Vector3 d = new(rect.MaxX, rect.MaxY, 0f);

        Vector3 m1 = Bisector(a, b, eye);
        Vector3 m2 = Bisector(c, d, eye);
        Vector3 m3 = Bisector(a, d, eye);
        Vector3 m4 = Bisector(b, c, eye);

        // Intersection on the x and y components only (REQ-SCN-008).
        float r1x = m2.X - m1.X;
        float r1y = m2.Y - m1.Y;
        float r2x = m4.X - m3.X;
        float r2y = m4.Y - m3.Y;
        float denominator = (r1x * r2y) - (r1y * r2x);
        if (MathF.Abs(denominator) < 1e-6f)
        {
            return new Vector3(fallback.X, fallback.Y, 0f);
        }

        float t = (((m3.X - m1.X) * r2y) - ((m3.Y - m1.Y) * r2x)) / denominator;
        return new Vector3(m1.X + (t * r1x), m1.Y + (t * r1y), 0f);
    }

    /// <summary>
    /// Where the bisector issued from <paramref name="eye"/> of the angle under which the eye
    /// sees <c>[p, q]</c> lands: <c>p + (q − p) · |p − E| / (|p − E| + |q − E|)</c>. A
    /// degenerate case (the eye on a vertex) gives the midpoint (REQ-SCN-008).
    /// </summary>
    /// <remarks>
    /// AMD-019 (Q-046) settles what "degenerate" means: the eye coincides with one of the two
    /// vertices, that is <b>at least one</b> of the two distances is zero. The general formula
    /// stays defined there — it returns the vertex itself — so the midpoint has to be taken
    /// explicitly.
    /// </remarks>
    public static Vector3 Bisector(Vector3 p, Vector3 q, Vector3 eye)
    {
        float dp = Vector3.Distance(p, eye);
        float dq = Vector3.Distance(q, eye);
        float sum = dp + dq;

        // Q-046: either distance zero — not only both — is the degenerate case.
        float t = dp <= 1e-6f || dq <= 1e-6f || sum <= 1e-6f ? 0.5f : dp / sum;
        return p + ((q - p) * t);
    }

    private static Matrix4x4 RotX(float degrees)
    {
        float r = degrees * MathF.PI / 180f;
        float c = MathF.Cos(r);
        float s = MathF.Sin(r);
        return new Matrix4x4(
            1f, 0f, 0f, 0f,
            0f, c, -s, 0f,
            0f, s, c, 0f,
            0f, 0f, 0f, 1f);
    }

    private static Matrix4x4 RotY(float degrees)
    {
        float r = degrees * MathF.PI / 180f;
        float c = MathF.Cos(r);
        float s = MathF.Sin(r);
        return new Matrix4x4(
            c, 0f, s, 0f,
            0f, 1f, 0f, 0f,
            -s, 0f, c, 0f,
            0f, 0f, 0f, 1f);
    }

    private static Matrix4x4 RotZ(float degrees)
    {
        float r = degrees * MathF.PI / 180f;
        float c = MathF.Cos(r);
        float s = MathF.Sin(r);
        return new Matrix4x4(
            c, -s, 0f, 0f,
            s, c, 0f, 0f,
            0f, 0f, 1f, 0f,
            0f, 0f, 0f, 1f);
    }

    /// <summary>
    /// Product of two matrices whose cells are stored row by row, applied to column vectors —
    /// the convention of REQ-SCN-007 and REQ-E3D-407. <c>Matrix4x4.Multiply</c> multiplies the
    /// stored cells, so it is the right product for that convention too.
    /// </summary>
    private static Matrix4x4 Multiply(Matrix4x4 a, Matrix4x4 b) => Matrix4x4.Multiply(a, b);

    /// <summary>
    /// Column <paramref name="column"/> of a matrix stored row by row, that is the image of
    /// the corresponding basis vector. <c>Mij</c> is the cell of row <c>i</c>, column <c>j</c>.
    /// </summary>
    private static Vector3 Column(Matrix4x4 m, int column) => column switch
    {
        0 => new Vector3(m.M11, m.M21, m.M31),
        1 => new Vector3(m.M12, m.M22, m.M32),
        _ => new Vector3(m.M13, m.M23, m.M33),
    };
}
