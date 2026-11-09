using Zax.Profiles.Hash;

namespace Zax.Profiles.Tests;

/// <summary>
/// Synthetic iNES images for the tile resolution of AMD-062 REQ-PROF-078: a header of SPEC-02
/// §5.1, an empty PRG bank and a CHR zone that holds the raw bytes of chosen tiles, each at a
/// position that is <b>not</b> aligned on sixteen.
/// </summary>
internal static class SyntheticRom
{
    /// <summary>Offset of the first tile inside the CHR zone; 8 is not a multiple of sixteen.</summary>
    private const int FirstTileOffset = 8;

    /// <summary>Distance between two tiles, so that every one of them starts at offset 8 mod 16.</summary>
    private const int TileStride = 32;

    private const int PrgBankSize = 16 * 1024;
    private const int ChrBankSize = 8 * 1024;

    /// <summary>
    /// Builds an iNES file whose CHR zone holds <paramref name="tiles"/>, each sixteen bytes at
    /// a position that is not a multiple of sixteen.
    /// </summary>
    /// <remarks>
    /// The space between two tiles is filled with a deterministic high-entropy filler rather
    /// than zeros. A window straddling a tile and its padding is then no tile of any profile,
    /// which is what the second part of the criterion needs: the file without one tile must
    /// really not hold that tile. The PRG bank stays zero, so the all-zero tile is found there
    /// as well — the accepted divergence (D) of AMD-062 §9.
    /// </remarks>
    /// <param name="tiles">The raw tiles to place, sixteen bytes each.</param>
    /// <param name="flip">A mirroring applied to every tile before it is placed.</param>
    internal static byte[] WithTiles(IEnumerable<byte[]> tiles, TileFlip flip = TileFlip.None)
    {
        List<byte[]> list = [.. tiles];
        int needed = list.Count == 0
            ? ChrBankSize
            : FirstTileOffset + ((list.Count - 1) * TileStride) + RawTile.ByteCount;
        int chrBanks = Math.Max(1, (needed + ChrBankSize - 1) / ChrBankSize);
        Assert.True(chrBanks <= 255, $"{chrBanks} CHR banks do not fit in an iNES header.");

        int chrStart = 16 + PrgBankSize;
        byte[] file = new byte[chrStart + (chrBanks * ChrBankSize)];
        WriteHeader(file, prgBanks: 1, chrBanks: (byte)chrBanks);
        FillChr(file.AsSpan(chrStart));

        for (int index = 0; index < list.Count; index++)
        {
            Span<byte> destination = file.AsSpan(chrStart + FirstTileOffset + (index * TileStride), 16);
            RawTile.Flip(list[index], flip, destination);
        }

        return file;
    }

    /// <summary>A deterministic filler: a 64-bit xorshift, so the bytes look like nothing.</summary>
    private static void FillChr(Span<byte> chr)
    {
        ulong state = 0x9E37_79B9_7F4A_7C15UL;
        for (int offset = 0; offset < chr.Length; offset++)
        {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            chr[offset] = (byte)(state >> 24);
        }
    }

    /// <summary>
    /// Builds an iNES file in mapper 0 with no CHR-ROM, which gives the cartridge 32 KiB of
    /// CHR-RAM (SPEC-02 REQ-CORE-012), and the given PRG bytes.
    /// </summary>
    internal static byte[] WithChrRam(ReadOnlySpan<byte> program, int prgBanks = 1)
    {
        byte[] file = new byte[16 + (prgBanks * PrgBankSize)];
        WriteHeader(file, (byte)prgBanks, chrBanks: 0);
        program.CopyTo(file.AsSpan(16));
        return file;
    }

    /// <summary>The raw bytes of the distinct tile contents of a profile, in file order.</summary>
    internal static List<byte[]> DistinctRawTiles(GameProfile profile)
    {
        HashSet<(ulong Low, ulong High)> seen = [];
        List<byte[]> raw = [];
        foreach (ProfilePattern pattern in HashProfileReaderTests.AllPatterns(profile, includeSlots: true))
        {
            foreach (ProfileTile? cell in pattern.Grid?.Cells ?? [])
            {
                if (cell is null || cell.IsPending)
                {
                    continue;
                }

                if (seen.Add((cell.LowPlane, cell.HighPlane)))
                {
                    raw.Add(RawTile.FromPlanes(cell.LowPlane, cell.HighPlane));
                }
            }
        }

        return raw;
    }

    private static void WriteHeader(byte[] file, byte prgBanks, byte chrBanks)
    {
        file[0] = 0x4E;
        file[1] = 0x45;
        file[2] = 0x53;
        file[3] = 0x1A;
        file[4] = prgBanks;
        file[5] = chrBanks;
    }
}
