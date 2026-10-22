using Zax.Nes;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The minimal test ROM of SPEC-02 §5.1, built by the verifier from the specification text
/// alone, plus the set-up procedure the same section prescribes.
/// </summary>
/// <remarks>
/// Header <c>4E 45 53 1A 02 01 01 00 00…</c>: 2 x 16 KiB PRG, 1 x 8 KiB CHR, vertical
/// mirroring, mapper 0. PRG: <c>$8000</c> holds <c>4C 00 80</c> and the NMI, RESET and IRQ
/// vectors all point at <c>$8000</c>. CHR: tile n of each table has plane 0 = eight bytes
/// equal to n and plane 1 = eight bytes equal to 255 - n, except tile 0 of table
/// <c>$0000</c>, which is all zero.
/// </remarks>
public static class SpecRom
{
    /// <summary>Mirroring to encode in byte 6 of the header.</summary>
    public enum Mirroring
    {
        /// <summary>Byte 6 = <c>0x00</c>.</summary>
        Horizontal,

        /// <summary>Byte 6 = <c>0x01</c>.</summary>
        Vertical,

        /// <summary>Byte 6 = <c>0x08</c>.</summary>
        FourScreen,
    }

    /// <summary>Builds the minimal ROM image.</summary>
    /// <param name="mirroring">Value to encode in byte 6.</param>
    /// <param name="program">
    /// Optional machine code to place at <c>$8000</c> instead of the infinite loop.
    /// </param>
    /// <param name="nmi">Optional NMI handler, placed at <c>$9000</c>.</param>
    public static byte[] Build(
        Mirroring mirroring = Mirroring.Vertical,
        byte[]? program = null,
        byte[]? nmi = null)
    {
        byte flags6 = mirroring switch
        {
            Mirroring.Horizontal => 0x00,
            Mirroring.Vertical => 0x01,
            _ => 0x08,
        };

        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 0x02;
        image[5] = 0x01;
        image[6] = flags6;
        image[7] = 0x00;

        // PRG: JMP $8000 at $8000, and the three vectors pointing at it.
        int prg = 16;
        byte[] code = program ?? [0x4C, 0x00, 0x80];
        code.CopyTo(image, prg);
        for (int vector = 0xFFFA; vector <= 0xFFFE; vector += 2)
        {
            image[prg + (vector - 0x8000)] = 0x00;
            image[prg + (vector - 0x8000) + 1] = 0x80;
        }

        if (nmi is not null)
        {
            nmi.CopyTo(image, prg + 0x1000);
            image[prg + (0xFFFA - 0x8000)] = 0x00;
            image[prg + (0xFFFA - 0x8000) + 1] = 0x90;
        }

        // CHR: two tables of 256 tiles.
        int chr = prg + (32 * 1024);
        for (int table = 0; table < 2; table++)
        {
            for (int n = 0; n < 256; n++)
            {
                int at = chr + (table * 4096) + (n * 16);
                bool zero = table == 0 && n == 0;
                for (int row = 0; row < 8; row++)
                {
                    image[at + row] = zero ? (byte)0 : (byte)n;
                    image[at + 8 + row] = zero ? (byte)0 : (byte)(255 - n);
                }
            }
        }

        return image;
    }

    /// <summary>Plane 0 of pattern <paramref name="n"/> of <paramref name="table"/>.</summary>
    public static ulong Plane0(int table, int n)
        => table == 0 && n == 0 ? 0UL : 0x0101010101010101UL * (ulong)n;

    /// <summary>Plane 1 of pattern <paramref name="n"/> of <paramref name="table"/>.</summary>
    public static ulong Plane1(int table, int n)
        => table == 0 && n == 0 ? 0UL : 0x0101010101010101UL * (ulong)(255 - n);

    /// <summary>
    /// Machine code that strobes the controllers and stores ten reads of <c>$4016</c> at
    /// <c>$0200</c> and ten reads of <c>$4017</c> at <c>$0210</c>, over and over. The
    /// register reads of REQ-CORE-022, REQ-CORE-024 and REQ-CORE-061 have to come from the
    /// emulated CPU: the host-side "read a memory byte" is side-effect-free by REQ-CORE-081
    /// and must not shift the serial registers.
    /// </summary>
    public static readonly byte[] PortReader =
    [
        0xA9, 0x01,             // LDA #$01
        0x8D, 0x16, 0x40,       // STA $4016
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x16, 0x40,       // STA $4016
        0xA2, 0x00,             // LDX #$00
        0xAD, 0x16, 0x40,       // L1: LDA $4016
        0x9D, 0x00, 0x02,       //     STA $0200,X
        0xE8,                   //     INX
        0xE0, 0x0A,             //     CPX #$0A
        0xD0, 0xF5,             //     BNE L1
        0xA2, 0x00,             // LDX #$00
        0xAD, 0x17, 0x40,       // L2: LDA $4017
        0x9D, 0x10, 0x02,       //     STA $0210,X
        0xE8,                   //     INX
        0xE0, 0x0A,             //     CPX #$0A
        0xD0, 0xF5,             //     BNE L2
        0x4C, 0x00, 0x80,       // JMP $8000
    ];

    /// <summary>
    /// A core with the minimal ROM loaded and the PPU warmed up (step 1 of §5.1: three
    /// frames, about 29 658 CPU cycles).
    /// </summary>
    public static NesCore Warm(
        Mirroring mirroring = Mirroring.Vertical,
        string name = "Minimal.nes",
        byte[]? program = null)
    {
        NesCore core = new();
        Assert.True(core.LoadRom(Build(mirroring, program), name).Success);
        for (int i = 0; i < 3; i++)
        {
            core.RunFrame(0);
        }

        return core;
    }

    /// <summary>Writes <paramref name="value"/> to VRAM address <paramref name="address"/>.</summary>
    public static void Vram(NesCore core, int address, params byte[] value)
    {
        core.WriteMemory(0x2006, (byte)(address >> 8));
        core.WriteMemory(0x2006, (byte)(address & 0xFF));
        foreach (byte b in value)
        {
            core.WriteMemory(0x2007, b);
        }
    }

    /// <summary>Writes one OAM entry (sprite <paramref name="n"/>).</summary>
    public static void Oam(NesCore core, int n, byte y, byte tile, byte attribute, byte x)
    {
        core.WriteMemory(0x2003, (byte)(n * 4));
        core.WriteMemory(0x2004, y);
        core.WriteMemory(0x2004, tile);
        core.WriteMemory(0x2004, attribute);
        core.WriteMemory(0x2004, x);
    }

    /// <summary>Fills the whole OAM with <paramref name="value"/>.</summary>
    public static void FillOam(NesCore core, byte value)
    {
        core.WriteMemory(0x2003, 0);
        for (int i = 0; i < 256; i++)
        {
            core.WriteMemory(0x2004, value);
        }
    }

    /// <summary>
    /// Sets the scroll with the two <c>$2005</c> writes §5.1 prescribes, then PPUCTRL and
    /// PPUMASK, runs one frame, and returns the batch of the <em>following</em> frame.
    /// </summary>
    public static CapturedBatch Capture(NesCore core, byte ppuCtrl, byte ppuMask, int scrollX = 0, int scrollY = 0)
    {
        core.WriteMemory(0x2005, (byte)scrollX);
        core.WriteMemory(0x2005, (byte)scrollY);
        core.WriteMemory(0x2000, ppuCtrl);
        core.WriteMemory(0x2001, ppuMask);
        core.RunFrame(0);
        return CapturedBatch.Run(core);
    }
}

/// <summary>A copy of one frame batch, taken because the spans are only valid in the callback.</summary>
public sealed class CapturedBatch
{
    private CapturedBatch(TileRecord[] records, byte[] palette, int maskX, int maskY)
    {
        Records = records;
        Palette = palette;
        MaskX = maskX;
        MaskY = maskY;
    }

    /// <summary>All records, in the order the core emitted them.</summary>
    public TileRecord[] Records { get; }

    /// <summary>The 32 bytes of palette RAM.</summary>
    public byte[] Palette { get; }

    /// <summary>Unreliable left/right band width.</summary>
    public int MaskX { get; }

    /// <summary>Unreliable top/bottom band width.</summary>
    public int MaskY { get; }

    /// <summary>Background records only.</summary>
    public TileRecord[] Background => [.. Records.Where(r => r.Background)];

    /// <summary>Sprite records only.</summary>
    public TileRecord[] Sprites => [.. Records.Where(r => !r.Background)];

    /// <summary>Runs one frame through the video callback and keeps a copy of the batch.</summary>
    public static CapturedBatch Run(NesCore core, ushort input = 0)
    {
        CapturedBatch? captured = null;
        int calls = 0;
        VideoCallback? previous = core.VideoCallback;
        core.VideoCallback = (in FrameBatch batch) =>
        {
            calls++;
            captured = new CapturedBatch(batch.Records.ToArray(), batch.Palette.ToArray(), batch.MaskX, batch.MaskY);
        };

        core.RunFrame(input);
        core.VideoCallback = previous;

        Assert.Equal(1, calls);
        return captured!;
    }
}
