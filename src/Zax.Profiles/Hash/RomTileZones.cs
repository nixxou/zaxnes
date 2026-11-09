namespace Zax.Profiles.Hash;

/// <summary>
/// The PRG and CHR zones of an iNES file, as AMD-062 REQ-PROF-078 step 1 delimits them for the
/// resolution of a « 3dh » profile's tiles.
/// </summary>
/// <remarks>
/// This repeats the little of the iNES header that locating the two zones needs; it is a file
/// layout concern, not emulation, which is why it lives here rather than in the core (the
/// profile library knows nothing of <c>Zax.Nes</c>). Both zones are truncated to the length of
/// the file, so a short or padded image is scanned for what it actually holds.
/// </remarks>
/// <param name="PrgStart">Offset of the PRG zone.</param>
/// <param name="PrgLength">Length of the PRG zone, already truncated to the file.</param>
/// <param name="ChrStart">Offset of the CHR zone.</param>
/// <param name="ChrLength">Length of the CHR zone, already truncated to the file.</param>
public readonly record struct RomTileZones(int PrgStart, int PrgLength, int ChrStart, int ChrLength)
{
    /// <summary>Length of an iNES header.</summary>
    public const int HeaderSize = 16;

    /// <summary>Length of the optional trainer.</summary>
    public const int TrainerSize = 512;

    private static readonly byte[] Signature = [0x4E, 0x45, 0x53, 0x1A];

    /// <summary>True when neither zone holds a single tile.</summary>
    public bool IsEmpty => PrgLength < RawTile.ByteCount && ChrLength < RawTile.ByteCount;

    /// <summary>
    /// Delimits the two zones of <paramref name="file"/>. A file that is not iNES gives no zone
    /// at all, and therefore resolves nothing (REQ-PROF-078).
    /// </summary>
    public static RomTileZones Of(ReadOnlySpan<byte> file)
    {
        if (file.Length < HeaderSize || !file[..Signature.Length].SequenceEqual(Signature))
        {
            return default;
        }

        bool isNes20 = (file[7] & 0x0C) == 0x08;
        int prgLength = isNes20
            ? Nes20Size(file[4], file[9] & 0x0F, 16 * 1024)
            : file[4] * 16 * 1024;
        int chrLength = isNes20
            ? Nes20Size(file[5], file[9] >> 4, 8 * 1024)
            : file[5] * 8 * 1024;

        int prgStart = HeaderSize + ((file[6] & 0x04) != 0 ? TrainerSize : 0);
        prgLength = Truncate(file.Length, prgStart, prgLength);
        int chrStart = prgStart + prgLength;
        chrLength = Truncate(file.Length, chrStart, chrLength);
        return new RomTileZones(prgStart, prgLength, chrStart, chrLength);
    }

    private static int Truncate(int fileLength, int start, int length)
    {
        if (start >= fileLength || length <= 0)
        {
            return 0;
        }

        return Math.Min(length, fileLength - start);
    }

    /// <summary>
    /// The NES 2.0 size notation, including the exponent-multiplier form the high nibble 15
    /// selects: <c>(2 × MM + 1) × 2^E</c> for a byte read as <c>EEEEEEMM</c>.
    /// </summary>
    private static int Nes20Size(byte low, int highNibble, int unit)
    {
        if (highNibble == 0x0F)
        {
            int exponent = low >> 2;
            int multiplier = ((low & 0x03) * 2) + 1;
            return exponent < 31 ? multiplier << exponent : 0;
        }

        return ((highNibble << 8) | low) * unit;
    }
}
