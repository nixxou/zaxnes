namespace Zax.Profiles;

/// <summary>Main light of the embedded settings (SPEC-03 §4.6, <c>Light</c>).</summary>
public sealed class LightSettings
{
    /// <summary>Direction, or <see langword="null"/> when the field is absent.</summary>
    public Vec3F? Direction { get; set; }

    /// <summary>Colour, or <see langword="null"/> when the field is absent.</summary>
    public ArgbColor? Color { get; set; }

    /// <summary>
    /// Intensity. Of scope U (SPEC-09 §4.1.3): the user's value is kept when a profile loads,
    /// unlike the direction and the colour.
    /// </summary>
    public float Intensity { get; set; }
}

/// <summary>
/// Parameters of one depth layer (SPEC-03 §4.6, <c>LayerParams</c>, SPEC-09 §4.1.6).
/// </summary>
/// <remarks>
/// The two shadow flags are kept <b>as stored</b>, because a profile must be rewritable without
/// loss (REQ-PROF-004). What the engine sees after a load is not the stored value: layer i has
/// <c>CastShadow = (i == 2)</c> and <c>RecvShadow = (i &gt;= 2)</c> whatever the file says
/// (REQ-PROF-042) — see <see cref="LayerSettingsSet.EffectiveCastShadow"/>.
/// </remarks>
public sealed class LayerSettings
{
    /// <summary>Depth offset in scene units (NES pixels).</summary>
    public float Offset { get; set; }

    /// <summary>Depth alignment of the volumes of the layer.</summary>
    public LayerAlignment Aligned { get; set; }

    /// <summary>The <c>CastShadow</c> flag as stored in the file.</summary>
    public bool StoredCastShadow { get; set; }

    /// <summary>The <c>RecvShadow</c> flag as stored in the file.</summary>
    public bool StoredRecvShadow { get; set; }

    /// <summary>
    /// Depth coefficient derived from the alignment (SPEC-09 REQ-PERS-013 point 8): 1 for
    /// front, 0 for centre, −1 for back.
    /// </summary>
    public float Coefficient => Aligned switch
    {
        LayerAlignment.Front => 1f,
        LayerAlignment.Center => 0f,
        LayerAlignment.Back => -1f,
        _ => 1f,
    };
}

/// <summary>
/// The six depth layers of the embedded settings (SPEC-03 §4.6, <c>Layers</c>), indexed by
/// <see cref="ProfileLayer"/>.
/// </summary>
public sealed class LayerSettingsSet
{
    /// <summary>Number of layers a valid profile holds (REQ-PROF-043).</summary>
    public const int LayerCount = 6;

    /// <summary>Layer parameters, in file order, indexed by layer value.</summary>
    public IList<LayerSettings> Layers { get; } = [];

    /// <summary>Layer given to patterns that do not name one.</summary>
    public ProfileLayer DefaultLayer { get; set; }

    /// <summary>Kept for data compatibility; without effect (SPEC-04 REQ-E3D-342).</summary>
    public bool Auto { get; set; }

    /// <summary>Layer parameters by index, or <see langword="null"/> when out of range.</summary>
    public LayerSettings? this[int index]
        => (uint)index < (uint)Layers.Count ? Layers[index] : null;

    /// <summary>
    /// Whether layer <paramref name="index"/> casts shadows after a profile load: true for
    /// layer 2 only, whatever the file stores (REQ-PROF-042, SPEC-09 REQ-PERS-005 step 4).
    /// </summary>
    public static bool EffectiveCastShadow(int index) => index == 2;

    /// <summary>
    /// Whether layer <paramref name="index"/> receives shadows after a profile load: true from
    /// layer 2 on, whatever the file stores (REQ-PROF-042).
    /// </summary>
    public static bool EffectiveRecvShadow(int index) => index >= 2;

    /// <summary>Builds the six layers with the default values of SPEC-09 §4.1.6.</summary>
    public static LayerSettingsSet CreateDefault()
    {
        LayerSettingsSet set = new() { DefaultLayer = ProfileLayer.L1 };
        float[] offsets = [0f, 0f, 0f, 12f, 24f, 36f];
        for (int i = 0; i < LayerCount; i++)
        {
            set.Layers.Add(new LayerSettings
            {
                Offset = offsets[i],
                Aligned = i == 1 ? LayerAlignment.Center : LayerAlignment.Front,
                StoredCastShadow = EffectiveCastShadow(i),
                StoredRecvShadow = EffectiveRecvShadow(i),
            });
        }

        return set;
    }
}

/// <summary>
/// The settings embedded in a profile (SPEC-03 §4.6): a complete copy of the application
/// settings as they were when the profile was written.
/// </summary>
/// <remarks>
/// <para>
/// Only part of it is applied when a profile loads; which part is fixed by REQ-PROF-041 and
/// listed by <see cref="ProfileSettingApplication"/>. The rest is data to be preserved
/// (REQ-PROF-004), including the editor presets of fields 14 and 15.
/// </para>
/// <para>
/// Every property starts at the default of REQ-PROF-040 for a missing field:
/// <c>Fov = 70</c>, <c>Skybox = −1</c>, <c>Mirroring = true</c>, <c>SceneRatio = 1</c>,
/// <c>SkipFrame = true</c>, <c>BgMusic = true</c>, everything else 0 or false.
/// </para>
/// </remarks>
public sealed class ProfileSettings
{
    /// <summary>Main light (field 1). Required: a profile without it is refused.</summary>
    public LightSettings? Light { get; set; }

    /// <summary>
    /// Use the universal colour of the NES palette as the background (field 2). Forced to true
    /// when a profile is applied (REQ-PROF-041).
    /// </summary>
    public bool UseNesBackgroundColor { get; set; }

    /// <summary>Custom background colour (field 3), or <see langword="null"/> when absent.</summary>
    public ArgbColor? CustomBackgroundColor { get; set; }

    /// <summary>
    /// Image clipping margins (field 4), indices 0 to 3 = left, bottom, right, top
    /// (DEC-R2-T1). Required: a profile without it is refused.
    /// </summary>
    public Vec4F? Clipping { get; set; }

    /// <summary>Without observable effect (field 5, SPEC-09 REQ-PERS-017).</summary>
    public bool MapEnable { get; set; }

    /// <summary>The six depth layers (field 6). Required: a profile without it is refused.</summary>
    public LayerSettingsSet? Layers { get; set; }

    /// <summary>Graphic quality (field 7): 0 low, 1 medium, 2 high.</summary>
    public int GraphicQuality { get; set; }

    /// <summary>VR runtime (field 8). Always read as 0 on desktop (SPEC-09 REQ-PERS-011).</summary>
    public int VrRuntime { get; set; }

    /// <summary>Vertical sync mode (field 9).</summary>
    public int VSync { get; set; }

    /// <summary>Frame-rate cap (field 10).</summary>
    public int FrameRate { get; set; }

    /// <summary>Game speed (field 11), −8 to 8.</summary>
    public float GameSpeed { get; set; }

    /// <summary>Master volume (field 12), 0 to 1.</summary>
    public float Volume { get; set; }

    /// <summary>Global mesh algorithm (field 13).</summary>
    public GlobalMeshAlgorithm MeshAlgorithm { get; set; }

    /// <summary>The editor's five shape presets (field 14, REQ-PROF-044).</summary>
    public IList<RecordedPatternSlot> SavedPatterns { get; } = [];

    /// <summary>The editor's five deformation presets (field 15, REQ-PROF-044).</summary>
    public IList<RecordedPatternSlot> SavedAnimPatterns { get; } = [];

    /// <summary>VR control method (field 16). Stored, without effect.</summary>
    public int ControlMethod { get; set; }

    /// <summary>Global shadow switch (field 17).</summary>
    public bool Shadow { get; set; }

    /// <summary>Recognise background tiles regardless of palette (field 18).</summary>
    public bool SameBackgroundPalette { get; set; }

    /// <summary>Sprite grouping tolerance (field 19).</summary>
    public int SpriteTolerance { get; set; }

    /// <summary>Tile scan direction (field 20): 0 by rows, 1 by columns.</summary>
    public int ScanDirection { get; set; }

    /// <summary>Skybox (field 21): −1 none, 0 colour, 1 gradient, 2 fantasy.</summary>
    public int Skybox { get; set; } = -1;

    /// <summary>Core index (field 22).</summary>
    public int Core { get; set; }

    /// <summary>VR mirroring (field 23). Stored, without effect.</summary>
    public bool Mirroring { get; set; } = true;

    /// <summary>Include empty background tiles in the conversion (field 24).</summary>
    public bool EmptyTile { get; set; }

    /// <summary>Top-down camera mode (field 25).</summary>
    public bool LocalCamera { get; set; }

    /// <summary>VR scene ratio (field 26). Read as 1 on desktop.</summary>
    public float SceneRatio { get; set; } = 1f;

    /// <summary>"Menu = Select + Start" (field 27). Stored, without effect (DEC-08-6).</summary>
    public bool EmulatedMenuButton { get; set; }

    /// <summary>Render method (field 28): 0 direct, 1 render texture.</summary>
    public int RenderMethod { get; set; }

    /// <summary>Resolution scale index (field 29).</summary>
    public int ResolutionScaleIndex { get; set; }

    /// <summary>Skip rendering when no new frame was emulated (field 30).</summary>
    public bool SkipFrame { get; set; } = true;

    /// <summary>Menu background music (field 31).</summary>
    public bool BackgroundMusic { get; set; } = true;

    /// <summary>Anti-aliasing (field 32): 0 none, 1 FXAA-like, 2 SMAA-like.</summary>
    public int AntiAlias { get; set; }

    /// <summary>Take the region from the ROM header when loading (field 33).</summary>
    public bool AutoTvMode { get; set; }

    /// <summary>Video region (field 34).</summary>
    public TvRegionMode TvMode { get; set; }

    /// <summary>
    /// The decorative TV box of the original product (field 35). AMD-031 removed the feature
    /// and kept the field: it is decoded and written back as it was found, byte for byte, and
    /// has no effect on anything.
    /// </summary>
    public int TvBox { get; set; }

    /// <summary>Library sort option (field 36): 0 A-Z, 1 recent.</summary>
    public int SortOption { get; set; }

    /// <summary>Recognise sprite tiles regardless of palette (field 37).</summary>
    public bool SameSpritePalette { get; set; }

    /// <summary>
    /// Vertical field of view in degrees (field 38). Never present in the built-in profiles,
    /// so it is 70 for all of them.
    /// </summary>
    public float Fov { get; set; } = 70f;
}
