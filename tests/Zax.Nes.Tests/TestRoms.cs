namespace Zax.Nes.Tests;

/// <summary>
/// Builds the minimal test ROM of SPEC-02 §5.1 and the header vectors of §5.2.
/// </summary>
internal static class TestRoms
{
    /// <summary>Base address the PRG-ROM is mapped at.</summary>
    public const ushort PrgBase = 0x8000;

    /// <summary>
    /// Builds the minimal ROM of SPEC-02 §5.1: 2 x 16 KiB of PRG, 1 x 8 KiB of CHR,
    /// mapper 0, an infinite loop at $8000 and all three vectors pointing at it.
    /// </summary>
    /// <param name="headerByte6">
    /// Byte 6 of the header. <c>0x01</c> (the default) is vertical mirroring, <c>0x00</c>
    /// horizontal, <c>0x08</c> four-screen.
    /// </param>
    /// <param name="program">
    /// Code to place at $8000, replacing the infinite loop. The vectors still point at
    /// $8000.
    /// </param>
    /// <param name="nmiVector">
    /// Address to put in the NMI vector instead of $8000, for the frame-boundary test of
    /// REQ-CORE-020.
    /// </param>
    public static byte[] Minimal(byte headerByte6 = 0x01, byte[]? program = null, ushort? nmiVector = null)
    {
        const int prgSize = 2 * 16 * 1024;
        const int chrSize = 8 * 1024;
        var image = new byte[16 + prgSize + chrSize];

        // Header: "NES\x1A", 2 x 16 KiB PRG, 1 x 8 KiB CHR, mapper 0.
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 0x02;
        image[5] = 0x01;
        image[6] = headerByte6;

        // PRG: JMP $8000 at $8000, unless the caller supplied its own code.
        byte[] code = program is { Length: > 0 } ? program : [0x4C, 0x00, 0x80];
        code.CopyTo(image, 16);

        // Vectors at $FFFA (NMI), $FFFC (RESET) and $FFFE (IRQ), all pointing at $8000.
        int vectors = 16 + prgSize - 6;
        for (int i = 0; i < 3; i++)
        {
            image[vectors + (2 * i)] = 0x00;
            image[vectors + (2 * i) + 1] = 0x80;
        }

        if (nmiVector is { } nmi)
        {
            image[vectors] = (byte)nmi;
            image[vectors + 1] = (byte)(nmi >> 8);
        }

        // CHR: tile n has plane 0 = eight bytes of n and plane 1 = eight bytes of 255 - n;
        // tile 0 of table $0000 is entirely zero.
        int chr = 16 + prgSize;
        for (int table = 0; table < 2; table++)
        {
            for (int tile = 0; tile < 256; tile++)
            {
                int offset = chr + (table * 4096) + (tile * 16);
                if (table == 0 && tile == 0)
                {
                    continue;
                }

                for (int row = 0; row < 8; row++)
                {
                    image[offset + row] = (byte)tile;
                    image[offset + 8 + row] = (byte)(255 - tile);
                }
            }
        }

        return image;
    }

    /// <summary>
    /// Returns the 16 CHR bytes of pattern <paramref name="tile"/> of
    /// <paramref name="table"/> (0 for $0000, 1 for $1000), as M(table, n) of §5.1.
    /// </summary>
    public static (ulong Plane0, ulong Plane1) Pattern(int table, int tile)
    {
        if (table == 0 && tile == 0)
        {
            return (0UL, 0UL);
        }

        ulong plane0 = 0;
        ulong plane1 = 0;
        for (int row = 0; row < 8; row++)
        {
            plane0 |= (ulong)(byte)tile << (8 * row);
            plane1 |= (ulong)(byte)(255 - tile) << (8 * row);
        }

        return (plane0, plane1);
    }

    /// <summary>
    /// Builds a 16-byte header from bytes 4 to 15, the form the vector table of SPEC-02
    /// §5.2 uses, followed by enough payload to satisfy the announced sizes.
    /// </summary>
    public static byte[] FromHeaderVector(params byte[] bytes4To15)
    {
        ArgumentOutOfRangeException.ThrowIfNotEqual(bytes4To15.Length, 12, nameof(bytes4To15));

        var header = new byte[16];
        header[0] = 0x4E;
        header[1] = 0x45;
        header[2] = 0x53;
        header[3] = 0x1A;
        bytes4To15.CopyTo(header, 4);
        return header;
    }
}
