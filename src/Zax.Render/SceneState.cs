using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Render;

/// <summary>Skybox setting, stored as an integer (SPEC-06 REQ-SCN-049).</summary>
public enum SkyboxKind
{
    /// <summary>Flat background, black outside a game (REQ-SCN-046).</summary>
    Default = -1,

    /// <summary>Flat background, always the custom colour.</summary>
    Color = 0,

    /// <summary>Gradient sky derived from the frame background colour (REQ-SCN-050).</summary>
    Gradient = 1,

    /// <summary>Nebula sky (REQ-SCN-051).</summary>
    Fantasy = 2,
}

/// <summary>Ambient occlusion setting <c>spl.ao</c> (AMD-055 REQ-SCN-079).</summary>
public enum AmbientOcclusionMode
{
    /// <summary>No occlusion pass at all.</summary>
    Off = 0,

    /// <summary>
    /// The multi-scale occlusion of REQ-SCN-083, rendered exactly like <see cref="Depth"/>
    /// while the MiniEngine port is not available — declared fallback <b>(D)</b>.
    /// </summary>
    Msvo = 1,

    /// <summary>Screen-space occlusion reconstructed from the depth (REQ-SCN-083).</summary>
    Depth = 2,
}

/// <summary>
/// The twelve parameters of the Enhanced style (AMD-055 REQ-SCN-079), plus the style itself
/// (REQ-SCN-078). They are global user settings, never taken from the profile, and a change
/// applies to the next rendered frame without reloading the game.
/// </summary>
/// <remarks>
/// <para>
/// Every <c>Effective*</c> member answers Classic with "off": REQ-SCN-080 has an Enhanced
/// capture with all effects off be identical to a Classic one, so Classic is simply the whole
/// set of effects switched off, and the renderer reads the effective members alone.
/// </para>
/// <para>
/// The curvature of AMD-043 is deliberately absent: it is <b>not</b> an effect of the style and
/// acts in Classic as in Enhanced <b>(D)</b>, which is why it stays on
/// <see cref="RenderSettings.Bend"/> (REQ-SCN-078).
/// </para>
/// </remarks>
public sealed class StyleSettings
{
    private int _ambientOcclusion;
    private int _aoStrength = 3;
    private int _aoRadius = 3;
    private int _aoThickness = 3;
    private int _aoQuality = 1;
    private int _voxelGrid;
    private int _ssrShine = 3;
    private int _ssrClarity = 3;
    private int _ssrLength = 3;
    private int _ssrQuality = 2;

    /// <summary>
    /// Whether the Enhanced style is in force. Classic is the rendering of SPEC-06, unchanged
    /// (REQ-SCN-078).
    /// </summary>
    public bool Enhanced { get; set; }

    /// <summary><c>spl.ao</c>: 0 Off, 1 MSVO, 2 Depth.</summary>
    public int AmbientOcclusion
    {
        get => _ambientOcclusion;
        set => _ambientOcclusion = Math.Clamp(value, 0, 2);
    }

    /// <summary><c>ao.strength</c>, 1 to 5.</summary>
    public int AoStrength
    {
        get => _aoStrength;
        set => _aoStrength = Math.Clamp(value, 1, 5);
    }

    /// <summary><c>ao.radius</c>, 1 to 5.</summary>
    public int AoRadius
    {
        get => _aoRadius;
        set => _aoRadius = Math.Clamp(value, 1, 5);
    }

    /// <summary><c>ao.thickness</c>, 1 to 5; no effect outside the MSVO mode.</summary>
    public int AoThickness
    {
        get => _aoThickness;
        set => _aoThickness = Math.Clamp(value, 1, 5);
    }

    /// <summary><c>ao.quality</c>: 0 Low, 1 Medium, 2 High.</summary>
    public int AoQuality
    {
        get => _aoQuality;
        set => _aoQuality = Math.Clamp(value, 0, 2);
    }

    /// <summary><c>spl.pixel_shadow</c> (REQ-SCN-082).</summary>
    public bool PixelShadow { get; set; }

    /// <summary><c>spl.voxel_grid</c>: 0 Off, 1, 2 or 3 (REQ-SCN-081).</summary>
    public int VoxelGrid
    {
        get => _voxelGrid;
        set => _voxelGrid = Math.Clamp(value, 0, 3);
    }

    /// <summary><c>spl.reflections</c> (REQ-SCN-084).</summary>
    public bool Reflections { get; set; }

    /// <summary><c>ssr.shine</c>, 1 to 5.</summary>
    public int SsrShine
    {
        get => _ssrShine;
        set => _ssrShine = Math.Clamp(value, 1, 5);
    }

    /// <summary><c>ssr.clarity</c>, 1 to 5.</summary>
    public int SsrClarity
    {
        get => _ssrClarity;
        set => _ssrClarity = Math.Clamp(value, 1, 5);
    }

    /// <summary><c>ssr.length</c>, 1 to 5.</summary>
    public int SsrLength
    {
        get => _ssrLength;
        set => _ssrLength = Math.Clamp(value, 1, 5);
    }

    /// <summary><c>ssr.quality</c>: 0 Low, 1 Medium, 2 High, 3 Ultra.</summary>
    public int SsrQuality
    {
        get => _ssrQuality;
        set => _ssrQuality = Math.Clamp(value, 0, 3);
    }

    /// <summary>
    /// The occlusion mode the frame really runs, <see cref="AmbientOcclusionMode.Off"/> in
    /// Classic.
    /// </summary>
    public AmbientOcclusionMode EffectiveOcclusion
        => Enhanced ? (AmbientOcclusionMode)AmbientOcclusion : AmbientOcclusionMode.Off;

    /// <summary>
    /// The cell side <c>N = 2^spl.voxel_grid</c> in voxels, and 0 when the grid is off or the
    /// style is Classic (REQ-SCN-081).
    /// </summary>
    public int EffectiveGridCell
        => Enhanced && VoxelGrid > 0 ? 1 << VoxelGrid : 0;

    /// <summary>Whether the pixel shadows of REQ-SCN-082 are in force this frame.</summary>
    public bool EffectivePixelShadow => Enhanced && PixelShadow;

    /// <summary>Whether the floor reflections of REQ-SCN-084 are in force this frame.</summary>
    public bool EffectiveReflections => Enhanced && Reflections;

    /// <summary>
    /// The occlusion radius <c>r</c> in scene units: <c>(8 + 6 × (ao.radius − 1)) × s</c>
    /// (REQ-SCN-083).
    /// </summary>
    /// <param name="sceneScale">The scene scale <c>s</c> of REQ-SCN-002.</param>
    public float OcclusionRadius(float sceneScale)
        => (8f + (6f * (AoRadius - 1))) * sceneScale;

    /// <summary>
    /// The darkening ceiling <c>O ≤ 0.1 × ao.strength</c> of REQ-SCN-083: 0.1 to 0.5.
    /// </summary>
    public float OcclusionCeiling => 0.1f * AoStrength;

    /// <summary>
    /// The maximum ray length <c>40 × n(ssr.length) × s</c> in scene units (REQ-SCN-084): 40,
    /// 120, 200, 280 and 360 for the settings 1 to 5.
    /// </summary>
    /// <param name="sceneScale">The scene scale <c>s</c> of REQ-SCN-002.</param>
    public float ReflectionLength(float sceneScale) => 40f * Odd(SsrLength) * sceneScale;

    /// <summary>
    /// The reflection intensity <c>0.09 × n(ssr.shine)</c> of REQ-SCN-084 — the share of the
    /// reflected colour mixed into the surface: 0.09, 0.27, 0.45, 0.63 and 0.81.
    /// </summary>
    public float ReflectionIntensity => 0.09f * Odd(SsrShine);

    /// <summary>
    /// The blur radius at the maximum ray length, as a fraction of the screen height:
    /// <c>0.003 × (10 − n(ssr.clarity))</c> (REQ-SCN-084), so 0.027 down to 0.003.
    /// </summary>
    public float ReflectionBlur => 0.003f * (10 - Odd(SsrClarity));

    /// <summary>
    /// <c>n(niveau) = 2 × niveau − 1</c> of REQ-SCN-084: the odd numbers 1 to 9 for the levels
    /// 1 to 5.
    /// </summary>
    private static int Odd(int level) => (2 * level) - 1;
}

/// <summary>
/// The user settings of SPEC-06 §4.1 the renderer reads. Their persistence belongs to SPEC-09;
/// the bounds and defaults here are those of §4.1.
/// </summary>
public sealed class RenderSettings
{
    private float _lightIntensity = 1f;
    private int _shadowMapSize = SceneConstants.ShadowMapSize;
    private int _antiAliasSamples = 1;
    private int _vsync;
    private int _targetFrameRate = 90;
    private int _bendLeftRight;
    private int _bendTopBottom;
    private int _arcHorizontal;
    private bool _arcAtEdge;
    private float _sceneScale = 1f;
    private SpaceBend _bend = SpaceBend.None;

    /// <summary>Vertical field of view in degrees, 45 to 95 by steps of 5 (REQ-SCN-005).</summary>
    public float Fov { get; set; } = SceneConstants.DefaultFov;

    /// <summary>Light intensity, 0.50 to 1.50 by steps of 0.01 (REQ-SCN-026).</summary>
    public float LightIntensity
    {
        get => _lightIntensity;
        set => _lightIntensity = Math.Clamp(value, 0.5f, 1.5f);
    }

    /// <summary>Whether shadows are enabled at all (REQ-SCN-030).</summary>
    public bool Shadows { get; set; } = true;

    /// <summary>Skybox setting (REQ-SCN-049).</summary>
    public SkyboxKind Skybox { get; set; } = SkyboxKind.Default;

    /// <summary>The custom background colour, black by default (REQ-SCN-046).</summary>
    public Rgb24 CustomBackgroundColor { get; set; }

    /// <summary>
    /// The sun, the moon and the clouds of the procedural skyboxes (AMD-024 REQ-PUB-003). All
    /// three are off by default, which is what REQ-SCN-050 asks for (Q-064).
    /// </summary>
    // Q-064
    public SkyBodySettings SkyBodies { get; } = new();

    /// <summary>
    /// Shadow map side in texels, 4096 by default (AMD-034 §2, replacing the 1024 of
    /// DEC-R2-D4). A size the hardware refuses is lowered by the renderer, not here.
    /// </summary>
    public int ShadowMapSize
    {
        get => _shadowMapSize;
        set => _shadowMapSize = Math.Clamp(value, 64, 8192);
    }

    /// <summary>
    /// Samples per pixel the scene is drawn with (AMD-032 §2): 1 for no anti-aliasing, 4 or 8
    /// for the two levels of <c>video.anti_alias</c>. The shadow map is never multisampled.
    /// A value the hardware refuses is lowered by the renderer, not here.
    /// </summary>
    public int AntiAliasSamples
    {
        get => _antiAliasSamples;
        set => _antiAliasSamples = Math.Clamp(value, 1, SceneConstants.MaxAntiAliasSamples);
    }

    /// <summary>
    /// Render skip (REQ-SCN-060), enabled by default: when no new emulated frame has been
    /// produced since the last render and the game is running, the host may keep the previous
    /// image instead of drawing the scene again.
    /// </summary>
    public bool SkipRender { get; set; } = true;

    /// <summary>
    /// VSync (REQ-SCN-061, SPEC-09): 0 off, 1 every refresh, 2 every other refresh. Applying it
    /// is the host's business; the renderer accepts any rate.
    /// </summary>
    public int VSync
    {
        get => _vsync;
        set => _vsync = Math.Clamp(value, 0, 2);
    }

    /// <summary>
    /// Target frame rate (REQ-SCN-061): 60, 90 or 120 frames per second, 90 by default. Any
    /// other persisted value comes back as 90.
    /// </summary>
    public int TargetFrameRate
    {
        get => _targetFrameRate;
        set => _targetFrameRate = value is 60 or 90 or 120 ? value : 90;
    }

    /// <summary>
    /// Whether back faces are culled. Kept as a switch so that a capture can show the raw
    /// meshes; it has no effect on closed geometry (REQ-E3D-505).
    /// </summary>
    public bool CullBackFaces { get; set; } = true;

    /// <summary>
    /// Whether a skybox is drawn instead of a flat background, which changes the background
    /// colour rules of REQ-SCN-046.
    /// </summary>
    public bool SkyboxDrawn => Skybox is SkyboxKind.Gradient or SkyboxKind.Fantasy;

    /// <summary>
    /// Left-right bend <c>scene.bend_lr</c> in degrees, −180 to 180 (AMD-043 REQ-SCN-066).
    /// </summary>
    /// <remarks>
    /// The renderer only bounds the angle; the step of 30° is the settings model's business
    /// (AMD-040 §3), so a script or a test may hand over any angle inside the bounds and get
    /// the continuous transform REQ-SCN-067 defines.
    /// </remarks>
    public int BendLeftRight
    {
        get => _bendLeftRight;
        set => Rebuild(ref _bendLeftRight, Math.Clamp(value, -180, 180));
    }

    /// <summary>Top-bottom bend <c>scene.bend_tb</c> in degrees, −180 to 180 (REQ-SCN-066).</summary>
    public int BendTopBottom
    {
        get => _bendTopBottom;
        set => Rebuild(ref _bendTopBottom, Math.Clamp(value, -180, 180));
    }

    /// <summary>Horizontal arc <c>scene.arc_h</c> in degrees, −180 to 180 (REQ-SCN-066).</summary>
    public int ArcHorizontal
    {
        get => _arcHorizontal;
        set => Rebuild(ref _arcHorizontal, Math.Clamp(value, -180, 180));
    }

    /// <summary>
    /// <c>scene.arc_center</c>: false for the screen centre, true for the bottom edge of the
    /// reference frame (REQ-SCN-066).
    /// </summary>
    public bool ArcAtEdge
    {
        get => _arcAtEdge;
        set
        {
            if (_arcAtEdge == value)
            {
                return;
            }

            _arcAtEdge = value;
            RebuildBend();
        }
    }

    /// <summary>
    /// <c>scene.bend_2d_mode</c>: whether the curvature reaches the 2D-mode rectangle, true by
    /// default (AMD-046 REQ-SCN-074). It is read at every frame, so a change is in force at the
    /// next rendered one; in 3D mode nothing reads it.
    /// </summary>
    public bool Bend2dMode { get; set; } = true;

    /// <summary>
    /// The visual style of AMD-055 REQ-SCN-078 and its twelve parameters. They are global
    /// settings, never taken from the profile, and apply to the next rendered frame.
    /// </summary>
    public StyleSettings Style { get; } = new();

    /// <summary>The scene scale <c>s</c> of REQ-SCN-002; 1 on desktop.</summary>
    public float SceneScale
    {
        get => _sceneScale;
        set
        {
            float scale = value is > 0f and <= float.MaxValue ? value : 1f;
            if (_sceneScale.Equals(scale))
            {
                return;
            }

            _sceneScale = scale;
            RebuildBend();
        }
    }

    /// <summary>
    /// The transform <c>B</c> of REQ-SCN-067 the four settings above define. It is rebuilt only
    /// when one of them changes, so the per-frame cost is nil.
    /// </summary>
    public SpaceBend Bend => _bend;

    private void Rebuild(ref int field, int value)
    {
        if (field == value)
        {
            return;
        }

        field = value;
        RebuildBend();
    }

    private void RebuildBend() => _bend = new SpaceBend(
        _bendLeftRight,
        _bendTopBottom,
        _arcHorizontal,
        _arcAtEdge,
        _sceneScale);
}

/// <summary>
/// The directional light of SPEC-06 §3.9: a profile setting that Lua can change during a game
/// (REQ-SCN-025, REQ-SCN-026).
/// </summary>
public sealed class LightState
{
    /// <summary>
    /// Propagation direction of the rays, normalised at use. The default <c>(1, −1, 1)</c> is
    /// light coming from above, from the left and from the camera side (REQ-SCN-025).
    /// </summary>
    public Vector3 Direction { get; set; } = new(1f, -1f, 1f);

    /// <summary>Light colour, white by default (REQ-SCN-025).</summary>
    public Rgb24 Color { get; set; } = new(255, 255, 255);

    /// <summary>Puts the profile defaults back (REQ-SCN-025).</summary>
    public void Reset()
    {
        Direction = new Vector3(1f, -1f, 1f);
        Color = new Rgb24(255, 255, 255);
    }
}

/// <summary>
/// The backdrop of SPEC-06 REQ-SCN-048: an optional axis-aligned box driven only by Lua. Every
/// property goes back to its default at the end of each game.
/// </summary>
public sealed class BackdropState
{
    /// <summary>Whether the box is displayed; false by default.</summary>
    public bool Enabled { get; set; }

    /// <summary>Colour of the box, black by default.</summary>
    public Rgb24 Color { get; set; }

    /// <summary>Size of the box, <c>(256, 240, 1)</c> by default.</summary>
    public Vector3 Size { get; set; } = new(256f, 240f, 1f);

    /// <summary>Position of the box centre relative to <c>C</c>, zero by default.</summary>
    public Vector3 Offset { get; set; }

    /// <summary>Whether the box receives shadows; true by default.</summary>
    public bool ReceiveShadow { get; set; } = true;

    /// <summary>Puts every property back to its default (REQ-SCN-048).</summary>
    public void Reset()
    {
        Enabled = false;
        Color = default;
        Size = new Vector3(256f, 240f, 1f);
        Offset = Vector3.Zero;
        ReceiveShadow = true;
    }
}

/// <summary>
/// Everything the renderer needs for one rendered frame: the settings, the camera, the clip
/// margins, the light, the backdrop and the tile list produced by the 3D engine
/// (SPEC-04B REQ-E3D-641).
/// </summary>
/// <remarks>
/// The host owns this object and mutates it between frames; the renderer only reads it, holds no
/// process-wide state and never assumes it owns the window (REQ-SCN-057, REQ-ARCH-003).
/// </remarks>
public sealed class SceneState
{
    /// <summary>The user settings of §4.1.</summary>
    public RenderSettings Settings { get; } = new();

    /// <summary>The directional light.</summary>
    public LightState Light { get; } = new();

    /// <summary>The backdrop.</summary>
    public BackdropState Backdrop { get; } = new();

    /// <summary>The camera.</summary>
    public SceneCamera Camera { get; } = new();

    /// <summary>
    /// The four clip margins <c>(c0, c1, c2, c3)</c> in scene units (REQ-SCN-003, DEC-R2-T1).
    /// They come from the profile at game load and Lua can change them during the game.
    /// </summary>
    public Vec4F Clipping { get; set; } = ConversionSettings.DefaultClipping;

    /// <summary>
    /// Whether a game is running. Outside that state the 3D scene is not displayed: only the
    /// background or the skybox stays visible behind the UI (REQ-SCN-047).
    /// </summary>
    public bool GameRunning { get; set; }

    /// <summary>
    /// The profile setting <c>Setting.BgColor</c>, forced to true at every profile load
    /// (REQ-SCN-046); Lua can change it afterwards.
    /// </summary>
    public bool UseNesBackgroundColor { get; set; } = true;

    /// <summary>
    /// The universal NES background colour of the <b>displayed</b> frame, entry 0 of the
    /// palette (REQ-SCN-046, REQ-SCN-052).
    /// </summary>
    public Rgb24 NesBackgroundColor { get; set; }

    /// <summary>The tile list of the displayed frame, or null when there is nothing to draw.</summary>
    public FrameDrawList? DrawList { get; set; }

    /// <summary>
    /// True when the "2D Mode" profile is loaded, that is when the profile holds no pattern at
    /// all (REQ-SCN-055). The scene then shows the core's pixel frame on the NES plane instead
    /// of the recomposed tiles (AMD-009, Q-022).
    /// </summary>
    public bool TwoDimensionalMode { get; set; }

    /// <summary>
    /// The 256 × 240 pixel frame the core produced for the <b>displayed</b> frame, one NES
    /// colour index per pixel, row 0 first (SPEC-02 REQ-CORE-048). Only read in 2D mode.
    /// </summary>
    public ReadOnlyMemory<byte> NesPicture { get; set; }

    /// <summary>
    /// The opacity mask of the same frame, one bit per pixel, 32 bytes per image row, the
    /// least significant bit of a byte being its leftmost pixel (SPEC-02 REQ-CORE-049). In 2D
    /// mode a pixel whose bit is 0 is not drawn at all (REQ-SCN-076); left empty, every pixel
    /// of the rectangle is drawn, which is what a host that produces no mask gets.
    /// </summary>
    public ReadOnlyMemory<byte> NesOpacityMask { get; set; }

    /// <summary>
    /// The shape the subjective camera follows, or null (REQ-SCN-021). In FPV it decides which
    /// tiles count as occluders and are drawn at half opacity (REQ-SCN-022).
    /// </summary>
    public FrameShape? FollowedShape { get; set; }

    /// <summary>
    /// Real time elapsed since the renderer started, in seconds. The star twinkle of
    /// REQ-SCN-050 uses it; it never follows the emulated clock (REQ-SCN-059).
    /// </summary>
    public float RealTime { get; set; }

    /// <summary>
    /// The frame time in seconds the animated deformation uses: emulated frames divided by 60
    /// (REQ-SCN-043, SPEC-04B REQ-E3D-568). It freezes while the game is paused.
    /// </summary>
    public float EmulatedTime { get; set; }

    /// <summary>
    /// The background colour of REQ-SCN-046, recomputed every rendered frame from the current
    /// settings and the displayed frame's palette.
    /// </summary>
    public Rgb24 BackgroundColor => ResolveBackground(
        Settings.Skybox,
        GameRunning,
        UseNesBackgroundColor,
        NesBackgroundColor,
        Settings.CustomBackgroundColor);

    /// <summary>The visible rectangle of the current clip margins (REQ-SCN-039).</summary>
    public VisibleRect VisibleRectangle => VisibleRect.From(Clipping);

    /// <summary>
    /// The background colour table of REQ-SCN-046.
    /// </summary>
    /// <param name="skybox">The skybox setting.</param>
    /// <param name="gameRunning">Whether a game is running.</param>
    /// <param name="useNesBackground">The profile setting <c>Setting.BgColor</c>.</param>
    /// <param name="nesBackground">Entry 0 of the displayed frame's palette.</param>
    /// <param name="custom">The user setting <c>Setting.CustomBgColor</c>.</param>
    public static Rgb24 ResolveBackground(
        SkyboxKind skybox,
        bool gameRunning,
        bool useNesBackground,
        Rgb24 nesBackground,
        Rgb24 custom)
    {
        if (skybox == SkyboxKind.Color)
        {
            return custom;
        }

        if (!gameRunning)
        {
            return default;
        }

        return useNesBackground ? nesBackground : custom;
    }
}
