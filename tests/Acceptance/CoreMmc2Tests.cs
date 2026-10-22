using Zax.Nes;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The synthetic MMC2 vector of REQ-CORE-039, and the MMC2 record ordering of
/// REQ-CORE-044.
/// </summary>
public sealed class CoreMmc2Tests
{
    private const int ChrBankSize = 4 * 1024;

    /// <summary>
    /// A mapper 9 ROM built like SPEC-02 §5.1, with four distinct 4 KiB CHR banks. In bank
    /// <c>b</c>, tile <c>n</c> has plane 0 = eight bytes of <c>n ^ (0x5A * b)</c> and
    /// plane 1 = its complement, so the same tile number reads differently per bank.
    /// </summary>
    private static byte[] BuildMmc2()
    {
        byte[] image = new byte[16 + (32 * 1024) + (4 * ChrBankSize)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 0x02;      // 32 KiB PRG
        image[5] = 0x02;      // 16 KiB CHR
        image[6] = 0x91;      // mapper 9 low nibble, vertical mirroring
        image[7] = 0x00;

        // The program sits in the last 8 KiB bank, which MMC2 keeps fixed at $E000.
        int prg = 16;
        int lastBank = prg + (3 * 8 * 1024);
        image[lastBank + 0] = 0x4C;
        image[lastBank + 1] = 0x00;
        image[lastBank + 2] = 0xE0;
        for (int vector = 0xFFFA; vector <= 0xFFFE; vector += 2)
        {
            image[prg + (vector - 0x8000)] = 0x00;
            image[prg + (vector - 0x8000) + 1] = 0xE0;
        }

        int chr = prg + (32 * 1024);
        for (int bank = 0; bank < 4; bank++)
        {
            for (int n = 0; n < 256; n++)
            {
                byte value = (byte)(n ^ (0x5A * bank));
                int at = chr + (bank * ChrBankSize) + (n * 16);
                for (int row = 0; row < 8; row++)
                {
                    image[at + row] = value;
                    image[at + 8 + row] = (byte)~value;
                }
            }
        }

        return image;
    }

    private static ulong Plane0(int bank, int n) => 0x0101010101010101UL * (byte)(n ^ (0x5A * bank));

    private static ulong Plane1(int bank, int n) => 0x0101010101010101UL * (byte)~(byte)(n ^ (0x5A * bank));

    private static NesCore Prepare()
    {
        NesCore core = new();
        Assert.True(core.LoadRom(BuildMmc2(), "Mmc2.nes").Success);
        for (int i = 0; i < 3; i++)
        {
            core.RunFrame(0);
        }

        // CHR banking for PPU $0000-$0FFF: latch $FD selects bank 0, latch $FE selects bank 1.
        core.WriteMemory(0xB000, 0);
        core.WriteMemory(0xC000, 1);

        // PPU $1000-$1FFF: banks 2 and 3, whichever latch.
        core.WriteMemory(0xD000, 2);
        core.WriteMemory(0xE000, 3);
        return core;
    }

    [Fact] // REQ-CORE-039 (the synthetic MMC2 vector of the criterion)
    public void Mmc2_latches_choose_the_bank_of_the_background_tiles()
    {
        NesCore core = Prepare();
        SpecRom.FillOam(core, 0xFF);

        // Row 0 of the nametable: $FE in column 2, $41 in 5 and 12, $FD in column 8.
        byte[] row = new byte[32];
        row[2] = 0xFE;
        row[5] = 0x41;
        row[8] = 0xFD;
        row[12] = 0x41;
        SpecRom.Vram(core, 0x2000, row);

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);
        TileRecord[] first = [.. batch.Background.Where(t => t.Y == 0).OrderBy(t => t.X)];
        Assert.Equal(32, first.Length);

        // Column 5 is still under the $FE latch set by column 2; column 12 under the $FD
        // latch set by column 8.
        Assert.Equal(Plane0(1, 0x41), first[5].Plane0);
        Assert.Equal(Plane1(1, 0x41), first[5].Plane1);
        Assert.Equal(Plane0(0, 0x41), first[12].Plane0);
        Assert.Equal(Plane1(0, 0x41), first[12].Plane1);
        Assert.NotEqual(first[5].Plane0, first[12].Plane0);
    }

    [Fact] // REQ-CORE-033 (same RAM and same batches with and without the callback, MMC2)
    public void The_mmc2_rom_is_unaffected_by_the_video_callback()
    {
        static (byte[] Ram, List<TileRecord[]> Batches, long Cycles) Run(bool withCallback)
        {
            NesCore core = Prepare();
            SpecRom.FillOam(core, 0xFF);
            SpecRom.Oam(core, 0, 0x40, 0x41, 0x00, 0x30);

            byte[] row = new byte[32];
            row[2] = 0xFE;
            row[5] = 0x41;
            row[8] = 0xFD;
            row[12] = 0x41;
            for (int r = 0; r < 30; r++)
            {
                SpecRom.Vram(core, 0x2000 + (32 * r), row);
            }

            core.WriteMemory(0x2000, 0x00);
            core.WriteMemory(0x2001, 0x1E);

            List<TileRecord[]> batches = [];
            if (withCallback)
            {
                core.VideoCallback = (in FrameBatch b) => batches.Add(b.Records.ToArray());
            }

            for (int i = 0; i < 600; i++)
            {
                core.RunFrame((ushort)i);
            }

            byte[] ram = new byte[0x800];
            for (int a = 0; a < 0x800; a++)
            {
                ram[a] = core.ReadMemory((ushort)a);
            }

            return (ram, batches, core.CycleCount);
        }

        (byte[] withRam, List<TileRecord[]> withBatches, long withCycles) = Run(true);
        (byte[] withoutRam, _, long withoutCycles) = Run(false);

        Assert.Equal(withoutRam, withRam);
        Assert.Equal(withoutCycles, withCycles);
        Assert.Equal(600, withBatches.Count);
    }

    [Fact] // REQ-CORE-044 (MMC2 exception: sprites of one capture come out by increasing X)
    public void Mmc2_sprites_of_one_capture_are_ordered_by_x()
    {
        NesCore core = Prepare();
        SpecRom.FillOam(core, 0xFF);

        // Four sprites on the same capture line, OAM order deliberately not X order.
        SpecRom.Oam(core, 0, y: 0x40, tile: 0x10, attribute: 0x00, x: 0xC0);
        SpecRom.Oam(core, 1, y: 0x40, tile: 0x11, attribute: 0x00, x: 0x20);
        SpecRom.Oam(core, 2, y: 0x40, tile: 0x12, attribute: 0x00, x: 0x80);
        SpecRom.Oam(core, 3, y: 0x40, tile: 0x13, attribute: 0x00, x: 0x40);

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x08, ppuMask: 0x1E);
        TileRecord[] sprites = batch.Sprites;
        Assert.Equal(4, sprites.Length);
        Assert.Equal([0x20, 0x40, 0x80, 0xC0], [.. sprites.Select(s => (int)s.X)]);
        Assert.Equal([1, 3, 2, 0], [.. sprites.Select(s => s.Index)]);
    }

    [Fact] // REQ-CORE-044 (ties on X are broken by OAM number, as REQ-ARCH-012 demands)
    public void Mmc2_sprites_tie_broken_by_oam_number()
    {
        NesCore core = Prepare();
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 5, y: 0x40, tile: 0x10, attribute: 0x00, x: 0x50);
        SpecRom.Oam(core, 2, y: 0x40, tile: 0x11, attribute: 0x00, x: 0x50);
        SpecRom.Oam(core, 9, y: 0x40, tile: 0x12, attribute: 0x00, x: 0x50);

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x08, ppuMask: 0x1E);
        Assert.Equal([2, 5, 9], [.. batch.Sprites.Select(s => s.Index)]);
    }

    [Fact] // REQ-CORE-044 (the default order, on a non-MMC2 cartridge, is by OAM number)
    public void Non_mmc2_sprites_keep_the_oam_order()
    {
        NesCore core = SpecRom.Warm();
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 0, y: 0x40, tile: 0x10, attribute: 0x00, x: 0xC0);
        SpecRom.Oam(core, 1, y: 0x40, tile: 0x11, attribute: 0x00, x: 0x20);
        SpecRom.Oam(core, 2, y: 0x40, tile: 0x12, attribute: 0x00, x: 0x80);

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x08, ppuMask: 0x1E);
        Assert.Equal([0, 1, 2], [.. batch.Sprites.Select(s => s.Index)]);
    }
}
