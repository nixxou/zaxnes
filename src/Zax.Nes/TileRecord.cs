namespace Zax.Nes;

/// <summary>
/// One 8x8 tile displayed during a frame (REQ-CORE-034, REQ-CORE-035, SPEC-02 §4.1).
/// </summary>
/// <remarks>
/// <para>
/// The two bit planes hold the 16 CHR bytes of the pattern <em>without flips applied</em>.
/// The byte of row <c>r</c> (0 = top) occupies bits <c>8r</c> to <c>8r+7</c>; inside that
/// byte bit 7 is the leftmost pixel (column 0) and bit 0 is column 7. The colour of the
/// source pixel at (column c, row r) is
/// <c>bit(Plane0, 8r + 7 - c) + 2 * bit(Plane1, 8r + 7 - c)</c>, between 0 and 3, where 0
/// is transparent. The pixel shown at tile position (px, py) is the source pixel of
/// column <c>MirrorH ? 7 - px : px</c> and row <c>MirrorV ? 7 - py : py</c>.
/// </para>
/// <para>
/// This encoding is also the one used by the pattern planes stored in profiles (SPEC-03),
/// which are the recognition key of the conversion engine (SPEC-04); it must be respected
/// to the bit.
/// </para>
/// </remarks>
public readonly record struct TileRecord
{
    /// <summary>Creates a tile record.</summary>
    public TileRecord(
        ulong plane0,
        ulong plane1,
        short x,
        short y,
        int index,
        bool mirrorH,
        bool mirrorV,
        bool hidden,
        byte palette,
        bool background)
    {
        Plane0 = plane0;
        Plane1 = plane1;
        X = x;
        Y = y;
        Index = index;
        MirrorH = mirrorH;
        MirrorV = mirrorV;
        Hidden = hidden;
        Palette = palette;
        Background = background;
    }

    /// <summary>Low bit plane, 8 CHR bytes packed little-endian by row.</summary>
    public ulong Plane0 { get; init; }

    /// <summary>High bit plane, 8 CHR bytes packed little-endian by row.</summary>
    public ulong Plane1 { get; init; }

    /// <summary>Screen x of the tile's top-left pixel. Background: -7..255. Sprite: 0..255.</summary>
    public short X { get; init; }

    /// <summary>Screen y of the tile's top-left pixel. Background: -1..238. Sprite: 1..239.</summary>
    public short Y { get; init; }

    /// <summary>Background: REQ-CORE-040. Sprite: 0..127 (REQ-CORE-043).</summary>
    public int Index { get; init; }

    /// <summary>Horizontal flip. Sprite: bit 6 of the OAM attribute. Background: false.</summary>
    public bool MirrorH { get; init; }

    /// <summary>Vertical flip. Sprite: bit 7 of the OAM attribute. Background: false.</summary>
    public bool MirrorV { get; init; }

    /// <summary>Sprite priority bit "behind background" (bit 5 of the attribute). Always false for background.</summary>
    public bool Hidden { get; init; }

    /// <summary>Sub-palette number, 0 to 3; background sub-palette for background tiles.</summary>
    public byte Palette { get; init; }

    /// <summary>True for a background tile, false for a sprite tile.</summary>
    public bool Background { get; init; }

    /// <summary>
    /// Colour (0 to 3) of the <em>source</em> pixel at (<paramref name="column"/>,
    /// <paramref name="row"/>), flips not applied. 0 is transparent.
    /// </summary>
    public int SourcePixel(int column, int row)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThan((uint)column, 7u, nameof(column));
        ArgumentOutOfRangeException.ThrowIfGreaterThan((uint)row, 7u, nameof(row));

        int bit = (8 * row) + 7 - column;
        return (int)((Plane0 >> bit) & 1UL) + (2 * (int)((Plane1 >> bit) & 1UL));
    }

    /// <summary>
    /// Colour (0 to 3) of the pixel <em>displayed</em> at tile position
    /// (<paramref name="px"/>, <paramref name="py"/>), flips applied.
    /// </summary>
    public int DisplayedPixel(int px, int py)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThan((uint)px, 7u, nameof(px));
        ArgumentOutOfRangeException.ThrowIfGreaterThan((uint)py, 7u, nameof(py));

        return SourcePixel(MirrorH ? 7 - px : px, MirrorV ? 7 - py : py);
    }

    /// <summary>True when both planes are zero, i.e. the tile draws nothing.</summary>
    /// <remarks>
    /// Empty records are still emitted by the core (REQ-CORE-036); filtering is the
    /// consumer's job (SPEC-04).
    /// </remarks>
    public bool IsEmpty => (Plane0 | Plane1) == 0UL;

    /// <summary>
    /// Packs eight CHR bytes of one plane into the 64-bit representation of
    /// REQ-CORE-034: the byte of row <c>r</c> goes to bits <c>8r</c>..<c>8r+7</c>.
    /// </summary>
    public static ulong PackPlane(ReadOnlySpan<byte> rows)
    {
        ArgumentOutOfRangeException.ThrowIfNotEqual(rows.Length, 8, nameof(rows));

        ulong plane = 0;
        for (int r = 0; r < 8; r++)
        {
            plane |= (ulong)rows[r] << (8 * r);
        }

        return plane;
    }
}
