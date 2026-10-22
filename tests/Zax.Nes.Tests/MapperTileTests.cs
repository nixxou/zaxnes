namespace Zax.Nes.Tests;

/// <summary>
/// The tile contract where it meets the mappers: the MMC2 latch vector of REQ-CORE-039, the
/// side-effect-free read path of REQ-CORE-033 and the sprite order of REQ-CORE-044.
/// </summary>
public sealed class MapperTileTests
{
    /// <summary>
    /// Builds the MMC2 board of the REQ-CORE-039 acceptance criterion: 64 KiB of CHR whose
    /// every kilobyte is filled with its own bank number, so the pattern of a tile says which
    /// bank it came from.
    /// </summary>
    private static NesCore LoadMmc2()
    {
        NesCore core = MapperFixture.Load(9, prgBanks8k: 8, chrBanks1k: 64);

        for (int frame = 0; frame < 3; frame++)
        {
            core.RunFrame(0);
        }

        // CHR for the $0000 window: 4 KiB bank 1 under latch $FD, bank 2 under latch $FE.
        core.MapperWrite(0xB000, 0x01);
        core.MapperWrite(0xC000, 0x02);

        return core;
    }

    /// <summary>Writes bytes to PPU memory the way a game does, through $2006 and $2007.</summary>
    private static void WriteVram(NesCore core, ushort address, params byte[] values)
    {
        core.WriteMemory(0x2006, (byte)(address >> 8));
        core.WriteMemory(0x2006, (byte)address);
        foreach (byte value in values)
        {
            core.WriteMemory(0x2007, value);
        }
    }

    /// <summary>
    /// The pattern byte of tile $41 in 4 KiB CHR bank <paramref name="bank"/>: the tile sits
    /// in the second kilobyte of the bank, so every one of its bytes reads back as
    /// 4 x bank + 1.
    /// </summary>
    private static ulong Tile41Plane(int bank)
    {
        ulong fill = (ulong)((4 * bank) + 1);
        ulong plane = 0;
        for (int row = 0; row < 8; row++)
        {
            plane |= fill << (8 * row);
        }

        return plane;
    }

    [Fact]
    public void REQ_CORE_039_an_mmc2_row_carries_the_pattern_each_latch_selected()
    {
        NesCore core = LoadMmc2();

        // The row of the acceptance criterion: $FE in column 2, $FD in column 8, $41 in
        // columns 5 and 12.
        var row = new byte[32];
        row[2] = 0xFE;
        row[5] = 0x41;
        row[8] = 0xFD;
        row[12] = 0x41;
        WriteVram(core, 0x2000, row);

        // The $2006 writes left the fine Y scroll at 2, as they always do for an address in
        // $2xxx; the pair of $2005 writes of the SPEC-02 §5.1 recipe puts it back to 0 so
        // that the row of line 0 is captured on line 1 (REQ-CORE-037).
        core.WriteMemory(0x2005, 0x00);
        core.WriteMemory(0x2005, 0x00);

        core.WriteMemory(0x2000, 0x00); // Background patterns at $0000.
        core.WriteMemory(0x2001, 0x0A); // Background on, left column shown.

        core.RunFrame(0);
        BatchCopy batch = TileFixture.CaptureBatch(core);

        TileRecord[] first = [.. batch.Background.Where(static record => record.Y == 0)];
        Assert.Equal(32, first.Length);

        // Column 5 follows the $FE in column 2, column 12 the $FD in column 8.
        Assert.Equal(Tile41Plane(2), first[5].Plane0);
        Assert.Equal(Tile41Plane(1), first[12].Plane0);
    }

    [Fact]
    public void REQ_CORE_033_capturing_records_leaves_the_mmc2_latch_alone()
    {
        // Same board and same row, run twice: once with a video callback registered and once
        // without. The machine must end up in exactly the same state (REQ-CORE-033).
        static byte[] Run(bool withCallback)
        {
            NesCore core = LoadMmc2();

            var row = new byte[32];
            row[2] = 0xFE;
            row[5] = 0x41;
            row[8] = 0xFD;
            row[12] = 0x41;
            WriteVram(core, 0x2000, row);
            core.WriteMemory(0x2005, 0x00);
            core.WriteMemory(0x2005, 0x00);
            core.WriteMemory(0x2000, 0x00);
            core.WriteMemory(0x2001, 0x1E);

            if (withCallback)
            {
                core.VideoCallback = static (in FrameBatch batch) => { };
            }

            for (int frame = 0; frame < 600; frame++)
            {
                core.RunFrame(0);
            }

            core.VideoCallback = null;
            return core.SaveState();
        }

        Assert.Equal(Run(withCallback: false), Run(withCallback: true));
    }

    [RomFact("mmc3_test_2/rom_singles/4-scanline_timing.nes")]
    public void REQ_CORE_033_capturing_records_does_not_clock_the_mmc3_counter()
    {
        // The criterion of REQ-CORE-033: the mmc3_test suite gives the same result with and
        // without a video callback. The whole machine state is compared, which also covers
        // the IRQ counter itself.
        static byte[] Run(bool withCallback)
        {
            string path = TestRomLibrary.Find("mmc3_test_2/rom_singles/4-scanline_timing.nes")!;
            var core = new NesCore();
            Assert.True(core.LoadRom(File.ReadAllBytes(path), "4-scanline_timing.nes").Success);

            if (withCallback)
            {
                core.VideoCallback = static (in FrameBatch batch) => { };
            }

            for (int frame = 0; frame < 400; frame++)
            {
                core.RunFrame(0);
            }

            core.VideoCallback = null;
            return core.SaveState();
        }

        Assert.Equal(Run(withCallback: false), Run(withCallback: true));
    }

    [Theory]
    [InlineData(9)]
    [InlineData(10)]
    public void REQ_CORE_044_mappers_9_and_10_emit_the_sprites_of_a_capture_by_x(int mapper)
    {
        NesCore core = MapperFixture.Load(mapper, prgBanks8k: 8, chrBanks1k: 64);
        for (int frame = 0; frame < 3; frame++)
        {
            core.RunFrame(0);
        }

        // Four sprites in the same eight-line band, in decreasing x: the records must come
        // out by increasing x, OAM number breaking ties.
        core.WriteMemory(0x2003, 0x00);
        byte[] xs = [0x60, 0x40, 0x40, 0x20];
        for (int sprite = 0; sprite < xs.Length; sprite++)
        {
            core.WriteMemory(0x2004, 0x00); // y = 0, so the band of line 7.
            core.WriteMemory(0x2004, 0x41);
            core.WriteMemory(0x2004, 0x00);
            core.WriteMemory(0x2004, xs[sprite]);
        }

        for (int sprite = xs.Length; sprite < 64; sprite++)
        {
            core.WriteMemory(0x2004, 0xFF);
            core.WriteMemory(0x2004, 0x00);
            core.WriteMemory(0x2004, 0x00);
            core.WriteMemory(0x2004, 0x00);
        }

        core.WriteMemory(0x2001, 0x1E);
        core.RunFrame(0);
        BatchCopy batch = TileFixture.CaptureBatch(core);

        TileRecord[] sprites = batch.Sprites;
        Assert.Equal(4, sprites.Length);
        Assert.Equal<int[]>([0x20, 0x40, 0x40, 0x60], [.. sprites.Select(static record => (int)record.X)]);

        // Equal x keeps the OAM order: sprite 1 before sprite 2.
        Assert.Equal<int[]>([3, 1, 2, 0], [.. sprites.Select(static record => record.Index)]);
    }

    [Fact]
    public void REQ_CORE_044_other_mappers_keep_the_oam_order()
    {
        NesCore core = MapperFixture.Load(4, prgBanks8k: 8, chrBanks1k: 64);
        for (int frame = 0; frame < 3; frame++)
        {
            core.RunFrame(0);
        }

        core.WriteMemory(0x2003, 0x00);
        byte[] xs = [0x60, 0x40, 0x20];
        for (int sprite = 0; sprite < xs.Length; sprite++)
        {
            core.WriteMemory(0x2004, 0x00);
            core.WriteMemory(0x2004, 0x41);
            core.WriteMemory(0x2004, 0x00);
            core.WriteMemory(0x2004, xs[sprite]);
        }

        for (int sprite = xs.Length; sprite < 64; sprite++)
        {
            core.WriteMemory(0x2004, 0xFF);
            core.WriteMemory(0x2004, 0x00);
            core.WriteMemory(0x2004, 0x00);
            core.WriteMemory(0x2004, 0x00);
        }

        core.WriteMemory(0x2001, 0x1E);
        core.RunFrame(0);
        BatchCopy batch = TileFixture.CaptureBatch(core);

        Assert.Equal<int[]>([0, 1, 2], [.. batch.Sprites.Select(static record => record.Index)]);
    }
}
