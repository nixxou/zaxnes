using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>Where an instance is attached in the scene (SPEC-04B REQ-E3D-409, REQ-E3D-641).</summary>
public enum SceneContainer
{
    /// <summary>The game container.</summary>
    Game = 0,

    /// <summary>The interface container, for the UI layer.</summary>
    Interface = 1,
}

/// <summary>Material class of a tile to draw (SPEC-04B REQ-E3D-642).</summary>
public enum MaterialClass
{
    /// <summary>Opaque.</summary>
    Opaque = 0,

    /// <summary>Alpha blended, casts no shadow.</summary>
    Transparent = 1,

    /// <summary>UI layer.</summary>
    Interface = 2,

    /// <summary>Invisible but casts its shadow.</summary>
    ShadowOnly = 3,
}

/// <summary>The animated deformation parameters of a tile (SPEC-04B REQ-E3D-568).</summary>
/// <param name="Amplitude"><c>deltaAmp</c>.</param>
/// <param name="Speed"><c>deltaSpeed</c>.</param>
/// <param name="Phase"><c>deltaOffset</c>.</param>
/// <param name="Size"><c>sz = (pixel width, pixel height, D)</c>.</param>
/// <param name="TileRelativeToPivot"><c>(tx + 4, Hs − ty − 4, 0) − Pd</c>.</param>
public readonly record struct DeformParameters(
    Vec3 Amplitude,
    Mat4 Speed,
    Mat4 Phase,
    Vec3 Size,
    Vec3 TileRelativeToPivot)
{
    /// <summary>
    /// The displacement <c>δ</c> of the local mesh vertex <paramref name="vertex"/> at
    /// <paramref name="time"/> seconds (REQ-E3D-568). The renderer may evaluate this in a
    /// shader instead; the result must be the same.
    /// </summary>
    /// <remarks>
    /// <c>δ</c> is added to the vertex in the object's local frame, <b>before</b> the scale and
    /// the rotation (REQ-E3D-406, DEC-04B-Q2). Row 3 of the two matrices is free script data
    /// and has no effect here (REQ-E3D-569).
    /// </remarks>
    public Vec3 Displacement(Vec3 vertex, float time)
    {
        Vec3 point = TileRelativeToPivot + vertex;
        Vec3 normalised = new(
            Size.X == 0f ? 0f : point.X / Size.X,
            Size.Y == 0f ? 0f : point.Y / Size.Y,
            Size.Z == 0f ? 0f : point.Z / Size.Z);

        return new Vec3(
            Component(0, Amplitude.X, Size.X, normalised, time),
            Component(1, Amplitude.Y, Size.Y, normalised, time),
            Component(2, Amplitude.Z, Size.Z, normalised, time));
    }

    private float Component(int row, float amplitude, float size, Vec3 normalised, float time)
        => amplitude
        * size
        * MathF.Sin((normalised.X * Speed[row, 0]) + Phase[row, 0])
        * MathF.Sin((normalised.Y * Speed[row, 1]) + Phase[row, 1])
        * MathF.Sin((normalised.Z * Speed[row, 2]) + Phase[row, 2])
        * MathF.Sin((time * Speed[row, 3]) + Phase[row, 3]);
}

/// <summary>
/// One tile to draw, in the abstract form the renderer consumes (SPEC-04B REQ-E3D-641). The
/// conversion pipeline fills everything but the mesh itself, which lot L4b produces.
/// </summary>
public readonly record struct DrawTile
{
    /// <summary>The frame instance the tile belongs to.</summary>
    public required FrameInstance Instance { get; init; }

    /// <summary>The tile of the shape.</summary>
    public required FrameTile Tile { get; init; }

    /// <summary>
    /// The mesh of this tile image for this instance, or null when no geometry builder is
    /// plugged in (SPEC-04B REQ-E3D-640).
    /// </summary>
    public TileMesh? Mesh { get; init; }

    /// <summary>
    /// The complete transformation of REQ-E3D-406, scene transformation excluded: it maps a
    /// local mesh vertex to its scene position. The sprite depth nudge <c>n</c> and the
    /// deformation are <b>not</b> included (DEC-R2-T4).
    /// </summary>
    public required Mat4 Transform { get; init; }

    /// <summary>The anchor point <c>Pos</c> of the instance (REQ-E3D-205).</summary>
    public required Vec3 Anchor { get; init; }

    /// <summary>The tile centre <c>T</c> of SPEC-04B REQ-E3D-405.</summary>
    public required Vec3 TileCenter { get; init; }

    /// <summary>Where the tile is attached in the scene.</summary>
    public required SceneContainer Container { get; init; }

    /// <summary>Material class (SPEC-04B REQ-E3D-642).</summary>
    public required MaterialClass Material { get; init; }

    /// <summary>Opacity, 0 to 1.</summary>
    public required float Alpha { get; init; }

    /// <summary>
    /// The texture of the 3D object, resolved by the geometry builder — the custom texture of
    /// the profile or the pattern image (SPEC-04B REQ-E3D-426, REQ-E3D-641). Null when no
    /// geometry builder is plugged in.
    /// </summary>
    public ObjectTexture? Texture { get; init; }

    /// <summary>Texture format: indexed or direct.</summary>
    public required TextureFormat TextureFormat { get; init; }

    /// <summary>
    /// The palette table <c>P</c> of SPEC-04B REQ-E3D-560, or −1 for a direct texture, which
    /// needs none.
    /// </summary>
    public required int PaletteTable { get; init; }

    /// <summary>Texture offset, a fraction of the texture size.</summary>
    public required Vec2 TexOffset { get; init; }

    /// <summary>Texture repetition.</summary>
    public required Vec2 TexTilting { get; init; }

    /// <summary>Whether the tile casts a shadow (SPEC-04B REQ-E3D-567).</summary>
    public required bool CastShadow { get; init; }

    /// <summary>Whether the tile receives shadows (SPEC-04B REQ-E3D-567).</summary>
    public required bool ReceiveShadow { get; init; }

    /// <summary>
    /// True for a tile of a sprite shape. With <see cref="BehindBackground"/> it lets the
    /// renderer pick the depth nudge <c>n</c> (SPEC-04B REQ-E3D-405).
    /// </summary>
    public required bool Sprite { get; init; }

    /// <summary>True for a sprite tile whose priority bit puts it behind the background.</summary>
    public required bool BehindBackground { get; init; }

    /// <summary>The deformation parameters, or null when no deformation is active.</summary>
    public DeformParameters? Deform { get; init; }
}

/// <summary>
/// Everything the renderer needs for one displayed frame (SPEC-04B REQ-E3D-641): the ordered
/// tiles to draw, the 64 resolved palette colours and the frame time.
/// </summary>
public sealed class FrameDrawList
{
    private readonly List<DrawTile> _tiles = [];
    private readonly Rgb24[] _palette = new Rgb24[FramePalette.EntryCount];

    /// <summary>The ordered tiles to draw.</summary>
    public IReadOnlyList<DrawTile> Tiles => _tiles;

    /// <summary>The 64 resolved RGB colours of the frame palette.</summary>
    public ReadOnlySpan<Rgb24> Palette => _palette;

    /// <summary>The frame counter of the frame (REQ-E3D-043).</summary>
    public int FrameCounter { get; private set; }

    /// <summary>The frame time in seconds (REQ-E3D-043).</summary>
    public float Time { get; private set; }

    internal void Begin(ConversionFrame frame)
    {
        _tiles.Clear();
        FrameCounter = frame.Counter;
        Time = frame.Time;
        frame.Palette.CopyColorsTo(_palette);
    }

    internal void Add(DrawTile tile) => _tiles.Add(tile);
}
