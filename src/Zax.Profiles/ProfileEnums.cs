namespace Zax.Profiles;

/// <summary>
/// Mirroring flags of a tile (SPEC-03 REQ-PROF-031): bit 0 horizontal, bit 1 vertical.
/// </summary>
[Flags]
public enum TileFlip
{
    /// <summary>No mirroring.</summary>
    None = 0,

    /// <summary>Mirror horizontally (columns reversed).</summary>
    Horizontal = 1,

    /// <summary>Mirror vertically (rows reversed).</summary>
    Vertical = 2,
}

/// <summary>
/// Colours present in a pattern (SPEC-03 §4.3, <c>colorFlags</c>). Colour 0 is transparent and
/// has no flag.
/// </summary>
[Flags]
public enum PatternColorFlags
{
    /// <summary>No colour recorded.</summary>
    None = 0,

    /// <summary>Colour 1 occurs in the pattern.</summary>
    Color1 = 2,

    /// <summary>Colour 2 occurs in the pattern.</summary>
    Color2 = 4,

    /// <summary>Colour 3 occurs in the pattern.</summary>
    Color3 = 8,
}

/// <summary>
/// Stored density of a pattern, in tiles (<c>tDensity</c>) or in pixels (<c>pDensity</c>),
/// SPEC-03 §4.3.
/// </summary>
/// <remarks>
/// The built-in profiles also hold the undefined values 6 and 8, once each; they are kept as
/// read, so a value outside this enumeration is legal and must not be normalised.
/// </remarks>
public enum PatternDensity
{
    /// <summary>Empty.</summary>
    Empty = 0,

    /// <summary>Low.</summary>
    Low = 1,

    /// <summary>Average.</summary>
    Average = 2,

    /// <summary>Medium.</summary>
    Medium = 3,

    /// <summary>High.</summary>
    High = 4,

    /// <summary>Solid.</summary>
    Solid = 5,
}

/// <summary>Geometry family of a 3D variant (SPEC-03 §4.4, <c>geo</c>).</summary>
public enum VariantGeometry
{
    /// <summary>Default geometry, derived from the tiles.</summary>
    Default = 0,

    /// <summary>Horizontal cylinder.</summary>
    HorizontalCylinder = 1,

    /// <summary>Vertical cylinder.</summary>
    VerticalCylinder = 2,

    /// <summary>Cube.</summary>
    Cube = 3,

    /// <summary>Half horizontal cylinder.</summary>
    HalfHorizontalCylinder = 4,

    /// <summary>Half vertical cylinder.</summary>
    HalfVerticalCylinder = 5,
}

/// <summary>
/// Depth layer of a 3D variant and index of the six stored layer parameter sets
/// (SPEC-03 §4.4 and §4.6, SPEC-09 §4.1.6).
/// </summary>
public enum ProfileLayer
{
    /// <summary>Not identified.</summary>
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

/// <summary>Depth alignment of a layer (SPEC-03 §4.6, <c>Aligned</c>).</summary>
public enum LayerAlignment
{
    /// <summary>Front.</summary>
    Front = 0,

    /// <summary>Centre.</summary>
    Center = 1,

    /// <summary>Back.</summary>
    Back = 2,
}

/// <summary>
/// Mesh algorithm. Used both as the per-variant override (SPEC-03 §4.4, <c>rm</c>, where 0
/// means "use the global setting") and as the global setting (<c>RenderMode</c>, §4.6, where
/// the three algorithms are numbered from 0).
/// </summary>
public enum MeshAlgorithm
{
    /// <summary>In a variant: use the global setting. Not a value of the global setting.</summary>
    FromSettings = 0,

    /// <summary>Greedy meshing.</summary>
    Greedy = 1,

    /// <summary>Marching cubes.</summary>
    Marching = 2,

    /// <summary>Greedy meshing, multicolour.</summary>
    GreedyMulticolor = 3,
}

/// <summary>Global mesh algorithm setting (SPEC-03 §4.6 field 13, <c>RenderMode</c>).</summary>
public enum GlobalMeshAlgorithm
{
    /// <summary>Greedy meshing.</summary>
    Greedy = 0,

    /// <summary>Marching cubes.</summary>
    Marching = 1,

    /// <summary>Greedy meshing, multicolour.</summary>
    GreedyMulticolor = 2,
}

/// <summary>How a 3D variant is drawn (SPEC-03 §4.4, <c>renderStyle</c>).</summary>
public enum VariantRenderStyle
{
    /// <summary>Hidden.</summary>
    Hidden = 0,

    /// <summary>Drawn normally.</summary>
    Normal = 1,

    /// <summary>Shadow only.</summary>
    ShadowOnly = 2,
}

/// <summary>Pixel format of an explicit variant texture (SPEC-03 REQ-PROF-035).</summary>
public enum TextureFormat
{
    /// <summary>Palette index, three bits per pixel, split between <c>data</c> and <c>extra</c>.</summary>
    Indexed = 0,

    /// <summary>One packed ARGB integer per pixel.</summary>
    Direct = 1,
}

/// <summary>Television region of the embedded settings (SPEC-03 §4.6 field 34).</summary>
public enum TvRegionMode
{
    /// <summary>NTSC.</summary>
    Ntsc = 0,

    /// <summary>PAL.</summary>
    Pal = 1,

    /// <summary>Multi-region.</summary>
    Multi = 2,

    /// <summary>Dendy.</summary>
    Dendy = 3,
}

/// <summary>
/// Format version of a profile, after the version string has been filtered
/// (SPEC-03 REQ-PROF-021).
/// </summary>
public enum ProfileFormatVersion
{
    /// <summary>Version <c>"1"</c>: not supported, the profile is refused.</summary>
    Unsupported = 1,

    /// <summary>
    /// Version <c>"2"</c>: the "V2 format" flag is set, which moves the reference point of the
    /// deformation pivot (SPEC-04B) and is exposed to scripts as <c>IsV2Format</c> (SPEC-05).
    /// </summary>
    V2 = 2,

    /// <summary>Any other value, including <c>"3"</c> and an absent version: current format.</summary>
    Current = 3,
}
