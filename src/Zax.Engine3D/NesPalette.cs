namespace Zax.Engine3D;

/// <summary>An opaque 24-bit colour.</summary>
/// <param name="R">Red component.</param>
/// <param name="G">Green component.</param>
/// <param name="B">Blue component.</param>
public readonly record struct Rgb24(byte R, byte G, byte B)
{
    /// <summary>Builds a colour from a packed <c>0xRRGGBB</c> integer.</summary>
    public static Rgb24 FromPacked(int packed)
        => new((byte)(packed >> 16), (byte)(packed >> 8), (byte)packed);

    /// <summary>The colour packed as <c>0xRRGGBB</c>.</summary>
    public int Packed => (R << 16) | (G << 8) | B;

    /// <inheritdoc/>
    public override string ToString() => $"#{R:X2}{G:X2}{B:X2}";
}

/// <summary>
/// The fixed table of the 64 NES colours (SPEC-04 §4.1, REQ-E3D-251). It is owned by SPEC-04
/// and used by the geometry, the renderer and the scene background (DEC-04B-Q1).
/// </summary>
public static class NesPalette
{
    /// <summary>Number of NES colours.</summary>
    public const int ColorCount = 64;

    private static readonly int[] Packed =
    [
        0x788084, 0x0000FC, 0x0000C4, 0x4028C4, 0x94008C, 0xAC0028, 0xAC1000, 0x8C1800,
        0x503000, 0x007800, 0x006800, 0x005800, 0x004058, 0x000000, 0x000000, 0x000000,
        0xBCC0C4, 0x0078FC, 0x0088FC, 0x6848FC, 0xDC00D4, 0xE40060, 0xFC3800, 0xE46018,
        0xAC8000, 0x00B800, 0x00A800, 0x00A848, 0x008894, 0x2C2C2C, 0x000000, 0x000000,
        0xFCF8FC, 0x38C0FC, 0x6888FC, 0x9C78FC, 0xFC78FC, 0xFC589C, 0xFC7858, 0xFCA048,
        0xFCB800, 0xBCF818, 0x58D858, 0x58F89C, 0x00E8E4, 0x606060, 0x000000, 0x000000,
        0xFCF8FC, 0xA4E8FC, 0xBCB8FC, 0xDCB8FC, 0xFCB8FC, 0xF4C0E0, 0xF4D0B4, 0xFCE0B4,
        0xFCD884, 0xDCF878, 0xB8F878, 0xB0F0D8, 0x00F8FC, 0xC8C0C0, 0x000000, 0x000000,
    ];

    private static readonly Rgb24[] Colors = BuildColors();

    /// <summary>The RGB colour of NES colour index <paramref name="index"/> (six bits used).</summary>
    public static Rgb24 Color(int index) => Colors[index & 0x3F];

    private static Rgb24[] BuildColors()
    {
        Rgb24[] colors = new Rgb24[ColorCount];
        for (int i = 0; i < ColorCount; i++)
        {
            colors[i] = Rgb24.FromPacked(Packed[i]);
        }

        return colors;
    }
}

/// <summary>
/// The palette of one frame: 16 tables of 4 NES colour indices (REQ-E3D-250).
/// </summary>
/// <remarks>
/// Entry <c>e</c> is <c>p[0]</c> when <c>e mod 4 = 0</c> and <c>p[e mod 32]</c> otherwise,
/// where <c>p</c> is the 32-byte PPU palette of the batch (SPEC-02 REQ-CORE-045). Tables 0 to
/// 3 are the background sub-palettes, 4 to 7 the sprite ones, and tables 8 to 15 repeat
/// tables 0 to 7 so that a script can use them as custom palettes.
/// </remarks>
public sealed class FramePalette
{
    /// <summary>Number of entries: 16 tables × 4 columns.</summary>
    public const int EntryCount = 64;

    private readonly byte[] _entries = new byte[EntryCount];

    /// <summary>NES colour index (0 to 63) of entry <paramref name="entry"/>.</summary>
    /// <exception cref="ArgumentOutOfRangeException">The entry is outside 0..63.</exception>
    public byte this[int entry]
    {
        get
        {
            ArgumentOutOfRangeException.ThrowIfGreaterThan((uint)entry, 63u, nameof(entry));
            return _entries[entry];
        }

        set
        {
            ArgumentOutOfRangeException.ThrowIfGreaterThan((uint)entry, 63u, nameof(entry));
            _entries[entry] = (byte)(value & 0x3F);
        }
    }

    /// <summary>The RGB colour of entry <paramref name="entry"/> (REQ-E3D-251).</summary>
    public Rgb24 Color(int entry) => NesPalette.Color(this[entry]);

    /// <summary>Rebuilds the 64 entries from the 32 palette bytes of a frame batch.</summary>
    /// <exception cref="ArgumentException">Fewer than 32 bytes were given.</exception>
    public void LoadFrom(ReadOnlySpan<byte> ppuPalette)
    {
        if (ppuPalette.Length < 32)
        {
            throw new ArgumentException("A frame palette needs the 32 PPU palette bytes.", nameof(ppuPalette));
        }

        byte universal = (byte)(ppuPalette[0] & 0x3F);
        for (int e = 0; e < EntryCount; e++)
        {
            _entries[e] = (e & 3) == 0 ? universal : (byte)(ppuPalette[e & 31] & 0x3F);
        }
    }

    /// <summary>Copies the entries of <paramref name="source"/> into this palette.</summary>
    public void CopyFrom(FramePalette source)
    {
        ArgumentNullException.ThrowIfNull(source);
        source._entries.CopyTo(_entries, 0);
    }

    /// <summary>Writes the 64 resolved RGB colours of the frame (SPEC-04B REQ-E3D-641).</summary>
    public void CopyColorsTo(Span<Rgb24> destination)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(destination.Length, EntryCount, nameof(destination));
        for (int e = 0; e < EntryCount; e++)
        {
            destination[e] = NesPalette.Color(_entries[e]);
        }
    }

    /// <summary>Sets every entry to zero.</summary>
    public void Clear() => Array.Clear(_entries);
}
