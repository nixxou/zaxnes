namespace Zax.Nes.Rom;

/// <summary>
/// A validated ROM image: its header, its PRG and CHR banks and the raw bytes it came
/// from (REQ-CORE-010, REQ-CORE-011).
/// </summary>
/// <remarks>
/// The raw image is kept because a savestate must carry it (REQ-CORE-090): the reference
/// host restores a save slot without reloading the ROM, so the state block has to be
/// self-contained.
/// </remarks>
public sealed class RomImage
{
    private readonly byte[] _raw;
    private readonly byte[] _prgRom;
    private readonly byte[] _chrRom;
    private readonly byte[]? _trainer;

    private RomImage(byte[] raw, InesHeader header, byte[] prgRom, byte[] chrRom, byte[]? trainer, string name)
    {
        _raw = raw;
        Header = header;
        _prgRom = prgRom;
        _chrRom = chrRom;
        _trainer = trainer;
        Name = name;
    }

    /// <summary>The parsed header.</summary>
    public InesHeader Header { get; }

    /// <summary>
    /// ROM name supplied at load time, typically the file name. It is kept by the
    /// instance, stored in savestates and returned on restore (REQ-CORE-014).
    /// </summary>
    public string Name { get; }

    /// <summary>PRG-ROM bytes. Never empty.</summary>
    public ReadOnlySpan<byte> PrgRom => _prgRom;

    /// <summary>CHR-ROM bytes. Empty when the cartridge uses CHR-RAM.</summary>
    public ReadOnlySpan<byte> ChrRom => _chrRom;

    /// <summary>The 512 trainer bytes, or an empty span when there is no trainer.</summary>
    public ReadOnlySpan<byte> Trainer => _trainer ?? [];

    /// <summary>The exact bytes the image was loaded from.</summary>
    public ReadOnlySpan<byte> Raw => _raw;

    /// <summary>iNES mapper number.</summary>
    public int Mapper => Header.Mapper;

    /// <summary>Region announced by the header (REQ-CORE-013).</summary>
    public NesRegion AnnouncedRegion => Header.Region;

    /// <summary>
    /// Validates <paramref name="image"/> and splits it into banks.
    /// </summary>
    /// <param name="image">The bytes of a <c>.nes</c> file.</param>
    /// <param name="name">ROM name to keep (REQ-CORE-014).</param>
    /// <param name="rom">The loaded image on success.</param>
    /// <param name="error">A message naming the cause on failure (REQ-CORE-011).</param>
    /// <remarks>
    /// Rejects a missing signature, a file shorter than the size the header announces and
    /// a header that announces no PRG-ROM. Unsupported mapper numbers are rejected one
    /// level up, when the cartridge is built (REQ-CORE-011, REQ-CORE-102).
    /// Trailing bytes beyond the announced size are ignored: many dumps carry a title
    /// block there.
    /// </remarks>
    public static bool TryLoad(ReadOnlySpan<byte> image, string name, out RomImage? rom, out string? error)
        => TryLoad(image, name, out rom, out _, out error);

    /// <summary>
    /// Validates <paramref name="image"/> and splits it into banks, reporting the typed
    /// cause of a failure as well as its message (AMD-002, answer to Q-001).
    /// </summary>
    public static bool TryLoad(
        ReadOnlySpan<byte> image,
        string name,
        out RomImage? rom,
        out RomLoadFailure failure,
        out string? error)
    {
        rom = null;
        failure = RomLoadFailure.None;

        if (!InesHeader.TryParse(image, out InesHeader header, out error))
        {
            failure = RomLoadFailure.CorruptHeader;
            return false;
        }

        if (header.PrgRomSize == 0)
        {
            failure = RomLoadFailure.CorruptHeader;
            error = "Corrupt header: the header announces no PRG-ROM.";
            return false;
        }

        if (image.Length < header.AnnouncedSize)
        {
            // The reference does not detect truncated files; failing here is an accepted
            // divergence (REQ-CORE-011, notes).
            failure = RomLoadFailure.TruncatedFile;
            error = $"Truncated file: the header announces {header.AnnouncedSize} bytes, the image has {image.Length}.";
            return false;
        }

        byte[] trainer = header.HasTrainer
            ? image.Slice(InesHeader.Size, InesHeader.TrainerSize).ToArray()
            : [];

        rom = new RomImage(
            image.ToArray(),
            header,
            image.Slice(header.PrgRomOffset, header.PrgRomSize).ToArray(),
            header.ChrRomSize == 0 ? [] : image.Slice(header.ChrRomOffset, header.ChrRomSize).ToArray(),
            header.HasTrainer ? trainer : null,
            name);

        error = null;
        return true;
    }
}
