using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>Depth layer of an instance (SPEC-04 §4.3, SPEC-04B REQ-E3D-402).</summary>
public enum ZLayer
{
    /// <summary>Not identified; resolved per shape by REQ-E3D-205.</summary>
    Unidentified = 0,

    /// <summary>Game interface.</summary>
    Ui = 1,

    /// <summary>First depth layer.</summary>
    L1 = 2,

    /// <summary>Second depth layer.</summary>
    L2 = 3,

    /// <summary>Third depth layer.</summary>
    L3 = 4,

    /// <summary>Fourth depth layer.</summary>
    L4 = 5,
}

/// <summary>Depth alignment of a layer (REQ-E3D-205, SPEC-04B REQ-E3D-403).</summary>
public enum ZAlignment
{
    /// <summary>Front faces aligned; coefficient 1.</summary>
    Front = 0,

    /// <summary>Centred; coefficient 0.</summary>
    Center = 1,

    /// <summary>Back faces aligned; coefficient −1.</summary>
    Back = 2,
}

/// <summary>Geometry family of an instance (SPEC-04 §4.3, semantics in SPEC-04B §3.3).</summary>
public enum Geo3DType
{
    /// <summary>Extrusion of the pattern image.</summary>
    Default = 0,

    /// <summary>Cylinder with a horizontal axis.</summary>
    HCylinder = 1,

    /// <summary>Cylinder with a vertical axis.</summary>
    VCylinder = 2,

    /// <summary>Textured box.</summary>
    Cube = 3,

    /// <summary>Half cylinder with a horizontal axis.</summary>
    HalfHCylinder = 4,

    /// <summary>Half cylinder with a vertical axis.</summary>
    HalfVCylinder = 5,
}

/// <summary>Mesh algorithm (REQ-E3D-340; geometry in SPEC-04B §3.4).</summary>
public enum RenderAlgorithm
{
    /// <summary>Rectangular faces merged without looking at colour.</summary>
    Greedy = 0,

    /// <summary>Smoothed surface, marching cubes.</summary>
    Marching = 1,

    /// <summary>Greedy meshing limited to equal colours.</summary>
    GreedyMulticolor = 2,
}

/// <summary>Parameters of one depth layer (REQ-E3D-205, SPEC-04B REQ-E3D-403).</summary>
public sealed class LayerSetting
{
    /// <summary>Depth offset in scene units.</summary>
    public float Offset { get; set; }

    /// <summary>Depth alignment.</summary>
    public ZAlignment Alignment { get; set; }

    /// <summary>Whether the layer lets its objects cast shadows (SPEC-04B REQ-E3D-567).</summary>
    public bool CastShadow { get; set; }

    /// <summary>Whether the layer lets its objects receive shadows.</summary>
    public bool ReceiveShadow { get; set; }

    /// <summary>The alignment coefficient <c>a</c>: 1 front, 0 centre, −1 back.</summary>
    public float Coefficient => Alignment switch
    {
        ZAlignment.Front => 1f,
        ZAlignment.Center => 0f,
        ZAlignment.Back => -1f,
        _ => 1f,
    };
}

/// <summary>
/// The settings the conversion pipeline consumes (SPEC-04 §4.2), with the factory defaults of
/// that table.
/// </summary>
/// <remarks>
/// The settings model itself belongs to SPEC-09; this is the slice the pipeline reads, so that
/// the engine stays independent of the persistence layer (REQ-ARCH-002, REQ-ARCH-021). A
/// profile supplies some of these values (SPEC-03); <see cref="ApplyProfile"/> does that.
/// </remarks>
public sealed class ConversionSettings
{
    /// <summary>Number of layers: one per <see cref="ZLayer"/> value.</summary>
    public const int LayerCount = 6;

    private readonly LayerSetting[] _layers = BuildDefaultLayers();

    /// <summary>
    /// Clip margins <c>(left, bottom, right, top)</c> in pixels, indices 0 to 3 of the
    /// profile's vector 4 and of the Lua object (DEC-R2-T1). Default
    /// <c>(0.1, 8.1, 8.1, 0.1)</c>.
    /// </summary>
    public Vec4F Clipping { get; set; } = DefaultClipping;

    /// <summary>The factory clip margins of SPEC-04 §4.2.</summary>
    public static Vec4F DefaultClipping => new(0.1f, 8.1f, 8.1f, 0.1f);

    /// <summary>Whether an empty background tile is kept as an ordinary tile (REQ-E3D-004).</summary>
    public bool EmptyTile { get; set; }

    /// <summary>
    /// Germination order of background tiles: 0 = by y then x, 1 = by x then y
    /// (REQ-E3D-006).
    /// </summary>
    public int ScanDirection { get; set; }

    /// <summary>Whether background growth requires an equal palette attribute (REQ-E3D-090).</summary>
    public bool SameBackgroundPalette { get; set; }

    /// <summary>Whether sprite pass S2 requires an equal palette attribute (REQ-E3D-131).</summary>
    public bool SameSpritePalette { get; set; }

    /// <summary>Adjacency tolerance in pixels of sprite pass S2 (REQ-E3D-130, REQ-E3D-131).</summary>
    public int SpriteTolerance { get; set; }

    /// <summary>
    /// Global mesh algorithm (REQ-E3D-340). A user setting: it is never taken from a profile.
    /// </summary>
    public RenderAlgorithm RenderAlgorithm { get; set; } = RenderAlgorithm.Greedy;

    /// <summary>Layer given to a background shape whose instance layer is Unidentified.</summary>
    public ZLayer DefaultLayer { get; set; } = ZLayer.L1;

    /// <summary>Kept for data compatibility; without effect (REQ-E3D-342).</summary>
    public bool AutoLayer { get; set; }

    /// <summary>Whether shadows are enabled at all (SPEC-04B REQ-E3D-567).</summary>
    public bool Shadow { get; set; } = true;

    /// <summary>Whether the scene background uses the NES background colour (REQ-E3D-254).</summary>
    public bool UseNesBackgroundColor { get; set; } = true;

    /// <summary>The custom scene background colour used when <see cref="UseNesBackgroundColor"/> is false.</summary>
    public Rgb24 CustomBackgroundColor { get; set; }

    /// <summary>Parameters of layer <paramref name="layer"/>.</summary>
    public LayerSetting Layer(ZLayer layer) => _layers[(int)layer];

    /// <summary>Parameters of layer <paramref name="index"/> (0 to 5).</summary>
    public LayerSetting Layer(int index) => _layers[index];

    /// <summary>
    /// Applies the settings a profile carries (SPEC-03 §4.6) to the pipeline slice.
    /// </summary>
    /// <remarks>
    /// Only the keys REQ-PROF-041 lists as applied are taken, which
    /// <see cref="ProfileSettingApplication"/> is authoritative on: the clip margins, the five
    /// conversion settings, the layers — with the shadow permissions reset per REQ-PROF-042 —
    /// and the NES background flag, forced to true. The global mesh algorithm
    /// (REQ-E3D-340), the shadow switch and the custom background colour are user settings and
    /// a profile load leaves them alone.
    /// </remarks>
    public void ApplyProfile(ProfileSettings? settings)
    {
        if (settings is null)
        {
            return;
        }

        if (settings.Clipping is { } clipping)
        {
            Clipping = clipping;
        }

        EmptyTile = settings.EmptyTile;
        ScanDirection = settings.ScanDirection;
        SameBackgroundPalette = settings.SameBackgroundPalette;
        SameSpritePalette = settings.SameSpritePalette;
        SpriteTolerance = settings.SpriteTolerance;
        UseNesBackgroundColor = true;

        if (settings.Layers is { } layers)
        {
            DefaultLayer = (ZLayer)(int)layers.DefaultLayer;
            AutoLayer = layers.Auto;
            for (int i = 0; i < LayerCount && i < layers.Layers.Count; i++)
            {
                LayerSettings source = layers.Layers[i];
                _layers[i].Offset = source.Offset;
                _layers[i].Alignment = (ZAlignment)(int)source.Aligned;

                // REQ-PROF-042: whatever the file stores, after a load layer 2 casts and
                // layers 2 and up receive.
                _layers[i].CastShadow = LayerSettingsSet.EffectiveCastShadow(i);
                _layers[i].ReceiveShadow = LayerSettingsSet.EffectiveRecvShadow(i);
            }
        }
    }

    /// <summary>The scene background colour of the frame whose palette is given (REQ-E3D-254).</summary>
    public Rgb24 BackgroundColor(FramePalette palette, bool skyboxActive)
    {
        ArgumentNullException.ThrowIfNull(palette);
        return UseNesBackgroundColor && !skyboxActive ? palette.Color(0) : CustomBackgroundColor;
    }

    private static LayerSetting[] BuildDefaultLayers()
    {
        // REQ-E3D-205 notes: UI offset 0 CENTER; L1 0, L2 12, L3 24, L4 36, all FRONT.
        // Shadow permissions: L1 casts and receives, L2..L4 receive only, UI neither
        // (SPEC-04B REQ-E3D-567).
        float[] offsets = [0f, 0f, 0f, 12f, 24f, 36f];
        LayerSetting[] layers = new LayerSetting[LayerCount];
        for (int i = 0; i < LayerCount; i++)
        {
            layers[i] = new LayerSetting
            {
                Offset = offsets[i],
                Alignment = i == (int)ZLayer.Ui ? ZAlignment.Center : ZAlignment.Front,
                CastShadow = i == (int)ZLayer.L1,
                ReceiveShadow = i >= (int)ZLayer.L1,
            };
        }

        return layers;
    }
}
