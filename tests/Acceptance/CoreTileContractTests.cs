using Zax.Nes;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for the tile contract of SPEC-02 §3.5 and the vectors V1 to
/// V10 of §5.3.
/// </summary>
public sealed class CoreTileContractTests
{
    [Fact] // REQ-CORE-031
    public void Video_info_is_33_31_128()
        => Assert.Equal(new VideoInfo(33, 31, 128), NesCore.VideoInfo);

    [Fact] // REQ-CORE-034
    public void Plane_packing_and_source_pixels()
    {
        ulong plane0 = TileRecord.PackPlane([0x80, 0, 0, 0, 0, 0, 0, 0x01]);
        Assert.Equal(0x0100000000000080UL, plane0);

        TileRecord record = new(plane0, 0, 0, 0, 0, false, false, false, 0, true);
        Assert.Equal(1, record.SourcePixel(0, 0));
        Assert.Equal(1, record.SourcePixel(7, 7));
        for (int r = 0; r < 8; r++)
        {
            for (int c = 0; c < 8; c++)
            {
                if ((r, c) is (0, 0) or (7, 7))
                {
                    continue;
                }

                Assert.Equal(0, record.SourcePixel(c, r));
            }
        }
    }

    [Fact] // REQ-CORE-034, displayed pixel under flips
    public void Flips_select_the_displayed_source_pixel()
    {
        ulong plane0 = TileRecord.PackPlane([0x80, 0, 0, 0, 0, 0, 0, 0]);
        TileRecord flipped = new(plane0, 0, 0, 0, 0, mirrorH: true, mirrorV: false, false, 0, false);
        Assert.Equal(1, flipped.DisplayedPixel(7, 0));
        Assert.Equal(0, flipped.DisplayedPixel(0, 0));
    }

    // ------------------------------------------------------------------ V1

    [Fact] // REQ-CORE-036, REQ-CORE-037, REQ-CORE-038, REQ-CORE-040, REQ-CORE-046 (V1)
    public void V1_bare_background()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);

        Assert.Equal(960, batch.Records.Length);
        Assert.All(batch.Records, r => Assert.True(r.Background));
        Assert.Empty(batch.Sprites);

        for (int r = 0; r < 30; r++)
        {
            for (int k = 0; k < 32; k++)
            {
                TileRecord tile = batch.Records[(r * 32) + k];
                Assert.Equal(8 * k, tile.X);
                Assert.Equal(8 * r, tile.Y);
                Assert.Equal((33 * r) + k, tile.Index);
                Assert.Equal(0, tile.Palette);
                Assert.Equal(0UL, tile.Plane0);
                Assert.Equal(0UL, tile.Plane1);
                Assert.False(tile.MirrorH);
                Assert.False(tile.MirrorV);
                Assert.False(tile.Hidden);
            }
        }

        // r = 1, k = 4 -> x = 32, y = 8, index 37.
        TileRecord sample = batch.Records[32 + 4];
        Assert.Equal(32, sample.X);
        Assert.Equal(8, sample.Y);
        Assert.Equal(37, sample.Index);

        Assert.Equal(8, batch.MaskX);
        Assert.Equal(8, batch.MaskY);
    }

    // ------------------------------------------------------------------ V2

    [Fact] // REQ-CORE-041, REQ-CORE-042, REQ-CORE-044 (V2)
    public void V2_eight_by_eight_sprite()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 5, y: 0x50, tile: 0x21, attribute: 0x63, x: 0x80);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x08, ppuMask: 0x1E);

        TileRecord sprite = Assert.Single(batch.Sprites);
        Assert.False(sprite.Background);
        Assert.Equal(128, sprite.X);
        Assert.Equal(81, sprite.Y);
        Assert.Equal(5, sprite.Index);
        Assert.Equal(3, sprite.Palette);
        Assert.True(sprite.Hidden);
        Assert.True(sprite.MirrorH);
        Assert.False(sprite.MirrorV);
        Assert.Equal(0x2121212121212121UL, sprite.Plane0);
        Assert.Equal(0xDEDEDEDEDEDEDEDEUL, sprite.Plane1);

        // It follows the background row captured at L = 81 and precedes the one of L = 89.
        int at = Array.IndexOf(batch.Records, sprite);
        Assert.Equal(80, batch.Records[at - 1].Y);  // row captured at L = 81 -> y = 80
        Assert.Equal(88, batch.Records[at + 1].Y);  // row captured at L = 89 -> y = 88
    }

    // ------------------------------------------------------------------ V3

    [Fact] // REQ-CORE-042, REQ-CORE-043 (V3)
    public void V3_eight_by_sixteen_sprite()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 2, y: 0x20, tile: 0x15, attribute: 0x80, x: 0x10);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x20, ppuMask: 0x1E);

        TileRecord[] sprites = batch.Sprites;
        Assert.Equal(2, sprites.Length);

        Assert.Equal(2, sprites[0].Index);
        Assert.Equal(16, sprites[0].X);
        Assert.Equal(33, sprites[0].Y);
        Assert.True(sprites[0].MirrorV);
        Assert.Equal(SpecRom.Plane0(1, 0x15), sprites[0].Plane0);
        Assert.Equal(SpecRom.Plane1(1, 0x15), sprites[0].Plane1);

        Assert.Equal(66, sprites[1].Index);
        Assert.Equal(16, sprites[1].X);
        Assert.Equal(41, sprites[1].Y);
        Assert.True(sprites[1].MirrorV);
        Assert.Equal(SpecRom.Plane0(1, 0x14), sprites[1].Plane0);
        Assert.Equal(SpecRom.Plane1(1, 0x14), sprites[1].Plane1);

        // The two records are consecutive in the batch.
        Assert.Equal(1, Array.IndexOf(batch.Records, sprites[1]) - Array.IndexOf(batch.Records, sprites[0]));
    }

    [Fact] // REQ-CORE-042 (V3 without the vertical flip)
    public void V3_without_vertical_flip()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 2, y: 0x20, tile: 0x15, attribute: 0x00, x: 0x10);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x20, ppuMask: 0x1E);

        TileRecord[] sprites = batch.Sprites;
        Assert.Equal(2, sprites.Length);
        Assert.Equal(SpecRom.Plane0(1, 0x14), sprites[0].Plane0);
        Assert.Equal(SpecRom.Plane0(1, 0x15), sprites[1].Plane0);
    }

    [Fact] // REQ-CORE-042 (V3, bottom half off-screen)
    public void V3_bottom_half_is_dropped_at_240()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 2, y: 0xE7, tile: 0x15, attribute: 0x80, x: 0x10);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x20, ppuMask: 0x1E);

        TileRecord sprite = Assert.Single(batch.Sprites);
        Assert.Equal(2, sprite.Index);
    }

    [Fact] // REQ-CORE-041 (Y = $EF and Y = $FF are never emitted; Y = 0 gives y = 1)
    public void Sprite_vertical_bounds()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 1, y: 0xEF, tile: 0x10, attribute: 0x00, x: 0x20);
        SpecRom.Oam(core, 3, y: 0x00, tile: 0x10, attribute: 0x00, x: 0x20);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x1E);

        TileRecord sprite = Assert.Single(batch.Sprites);
        Assert.Equal(3, sprite.Index);
        Assert.Equal(1, sprite.Y);
    }

    // ------------------------------------------------------------------ V4

    [Fact] // REQ-CORE-038 (V4)
    public void V4_horizontal_scroll_of_thirteen()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        for (int row = 0; row < 30; row++)
        {
            byte[] left = new byte[32];
            byte[] right = new byte[32];
            for (int c = 0; c < 32; c++)
            {
                left[c] = (byte)c;
                right[c] = (byte)(32 + c);
            }

            SpecRom.Vram(core, 0x2000 + (32 * row), left);
            SpecRom.Vram(core, 0x2400 + (32 * row), right);
        }

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A, scrollX: 13);

        TileRecord[] firstRow = [.. batch.Background.Where(t => t.Y == 0)];
        Assert.Equal(33, firstRow.Length);
        for (int k = 0; k < 33; k++)
        {
            Assert.Equal((8 * k) - 5, firstRow[k].X);
        }

        Assert.Equal(SpecRom.Plane0(0, 1), firstRow[0].Plane0);
        Assert.Equal(SpecRom.Plane0(0, 31), firstRow[30].Plane0);
        Assert.Equal(SpecRom.Plane0(0, 32), firstRow[31].Plane0);
        Assert.Equal(SpecRom.Plane0(0, 33), firstRow[32].Plane0);
    }

    // ------------------------------------------------------------------ V5

    [Fact] // REQ-CORE-037, REQ-CORE-038 (V5)
    public void V5_vertical_scroll_of_three()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        for (int row = 0; row < 30; row++)
        {
            byte[] cells = new byte[32];
            Array.Fill(cells, (byte)row);
            SpecRom.Vram(core, 0x2000 + (32 * row), cells);
        }

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A, scrollY: 3);

        int[] rowYs = [.. batch.Background.Select(t => (int)t.Y).Distinct().Order()];
        Assert.Equal(30, rowYs.Length);
        for (int j = 0; j < 30; j++)
        {
            Assert.Equal((8 * j) + 5, rowYs[j]);
        }

        TileRecord first = batch.Background.First(t => t.Y == 5);
        Assert.Equal(SpecRom.Plane0(0, 1), first.Plane0);
        Assert.Equal(0, first.Index / 33); // capture line 6 -> row index 0

        TileRecord last = batch.Background.First(t => t.Y == 237);
        Assert.Equal(SpecRom.Plane0(0, 0), last.Plane0);
        Assert.Equal(0UL, last.Plane0);
    }

    // ------------------------------------------------------------------ V6

    [Fact] // REQ-CORE-037 (V6, the vertical-mirroring exception)
    public void V6_scroll_y_one_vertical_mirroring_skips_line_zero()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A, scrollY: 1);

        Assert.DoesNotContain(batch.Background, t => t.Y == -1);
        Assert.Equal(7, batch.Background.Min(t => (int)t.Y));
    }

    [Fact] // REQ-CORE-037 (V6, horizontal mirroring keeps the row of line 0)
    public void V6_scroll_y_one_horizontal_mirroring_keeps_line_zero()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Horizontal);
        SpecRom.FillOam(core, 0xFF);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A, scrollY: 1);

        TileRecord[] top = [.. batch.Background.Where(t => t.Y == -1)];
        Assert.Equal(32, top.Length);
        Assert.Equal([.. Enumerable.Range(0, 32)], [.. top.Select(t => t.Index)]);
    }

    // ------------------------------------------------------------------ V7

    [Fact] // REQ-CORE-045 (V7)
    public void V7_palette()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Vram(core, 0x3F00, 0x0F);
        SpecRom.Vram(core, 0x3F01, 0x30);
        SpecRom.Vram(core, 0x3F11, 0x16);

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);
        Assert.Equal(32, batch.Palette.Length);
        Assert.Equal(0x0F, batch.Palette[0]);
        Assert.Equal(0x30, batch.Palette[1]);
        Assert.Equal(0x16, batch.Palette[17]);

        SpecRom.Vram(core, 0x3F10, 0x21);
        CapturedBatch after = CapturedBatch.Run(core);
        Assert.Equal(0x21, after.Palette[0]);
        Assert.Equal(0x21, after.Palette[16]);
    }

    // ------------------------------------------------------------------ V8

    [Fact] // REQ-CORE-044 (V8)
    public void V8_record_order()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        SpecRom.Oam(core, 0, y: 0x00, tile: 0x01, attribute: 0x00, x: 0x08);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x1E);

        for (int k = 0; k < 32; k++)
        {
            Assert.True(batch.Records[k].Background);
            Assert.Equal(0, batch.Records[k].Y);
        }

        TileRecord sprite = batch.Records[32];
        Assert.False(sprite.Background);
        Assert.Equal(0, sprite.Index);
        Assert.Equal(1, sprite.Y);

        for (int k = 0; k < 32; k++)
        {
            Assert.True(batch.Records[33 + k].Background);
            Assert.Equal(8, batch.Records[33 + k].Y);
        }
    }

    // ------------------------------------------------------------------ V9

    [Theory] // REQ-CORE-046 (V9)
    [InlineData(SpecRom.Mirroring.Horizontal, 0x1E, 8, 0)]
    [InlineData(SpecRom.Mirroring.Vertical, 0x1E, 0, 8)]
    [InlineData(SpecRom.Mirroring.Vertical, 0x18, 8, 8)]
    [InlineData(SpecRom.Mirroring.FourScreen, 0x1E, 0, 0)]
    [InlineData(SpecRom.Mirroring.FourScreen, 0x1A, 8, 0)]
    public void V9_masks(SpecRom.Mirroring mirroring, int ppuMask, int expectedX, int expectedY)
    {
        NesCore core = SpecRom.Warm(mirroring);
        SpecRom.FillOam(core, 0xFF);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: (byte)ppuMask);

        Assert.Equal(expectedX, batch.MaskX);
        Assert.Equal(expectedY, batch.MaskY);
    }

    // ------------------------------------------------------------------ V10

    [Fact] // REQ-CORE-047 (V10)
    public void V10_greyscale_and_emphasis_change_nothing()
    {
        static CapturedBatch Run(byte mask)
        {
            NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
            SpecRom.FillOam(core, 0xFF);
            SpecRom.Oam(core, 5, y: 0x50, tile: 0x21, attribute: 0x63, x: 0x80);
            return SpecRom.Capture(core, ppuCtrl: 0x08, ppuMask: mask);
        }

        CapturedBatch plain = Run(0x1E);
        CapturedBatch fancy = Run(0xFF);

        Assert.Equal(plain.Records, fancy.Records);
        Assert.Equal(plain.Palette, fancy.Palette);
        Assert.Equal(plain.MaskX, fancy.MaskX);
        Assert.Equal(plain.MaskY, fancy.MaskY);
    }

    // ------------------------------------------------------------------ capacity and callback

    [Fact] // REQ-CORE-030
    public void The_callback_receives_records_palette_and_masks()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x0A);

        Assert.NotEmpty(batch.Records);
        Assert.Equal(32, batch.Palette.Length);
        Assert.InRange(batch.MaskX, 0, 8);
        Assert.InRange(batch.MaskY, 0, 8);
    }

    [Fact] // REQ-CORE-032
    public void A_batch_never_exceeds_its_capacity()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        core.WriteMemory(0x2003, 0);
        for (int n = 0; n < 64; n++)
        {
            core.WriteMemory(0x2004, (byte)(8 * (n % 29)));
            core.WriteMemory(0x2004, 0x10);
            core.WriteMemory(0x2004, 0x00);
            core.WriteMemory(0x2004, (byte)(4 * n));
        }

        CapturedBatch batch = SpecRom.Capture(core, ppuCtrl: 0x00, ppuMask: 0x1E);
        Assert.InRange(batch.Background.Length, 0, VideoInfo.MaxBackgroundRecords);
        Assert.InRange(batch.Sprites.Length, 0, VideoInfo.MaxSpriteRecords);
        Assert.InRange(batch.Records.Length, 0, VideoInfo.MaxRecords);
    }

    [Fact] // REQ-CORE-032 (a raster effect that forces a row capture on every line)
    public void An_overflowing_raster_frame_is_capped_without_a_crash()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        SpecRom.FillOam(core, 0xFF);
        for (int n = 0; n < 64; n++)
        {
            SpecRom.Oam(core, n, (byte)(8 * (n % 29)), 0x12, 0x00, (byte)(4 * n));
        }

        core.WriteMemory(0x2000, 0x00);
        core.WriteMemory(0x2001, 0x1E);
        core.RunFrame(0);

        // Walk one frame by hand, rewriting the scroll with fine Y = 1 at the start of
        // every visible line, so the core wants to emit a row for each of the 240 lines.
        for (int line = 0; line < RegionTiming.VisibleScanlines; line++)
        {
            for (int guard = 0; guard < 2000 && core.Ppu.Scanline != line; guard++)
            {
                core.Bus.Tick();
            }

            core.WriteMemory(0x2006, 0x10); // fine Y = 1, nametable 0, coarse Y = 0
            core.WriteMemory(0x2006, 0x00);
        }

        for (int guard = 0; guard < 200_000 && core.Ppu.Scanline < RegionTiming.VisibleScanlines; guard++)
        {
            core.Bus.Tick();
        }

        ReadOnlySpan<TileRecord> records = core.TileRecords;
        int background = 0;
        int sprites = 0;
        foreach (TileRecord record in records)
        {
            if (record.Background)
            {
                background++;
            }
            else
            {
                sprites++;
            }
        }

        Assert.True(background > 960, $"the raster effect produced only {background} background records");
        Assert.InRange(background, 0, VideoInfo.MaxBackgroundRecords);
        Assert.InRange(sprites, 0, VideoInfo.MaxSpriteRecords);
        Assert.InRange(records.Length, 0, VideoInfo.MaxRecords);

        // The sprites are still there, and the emulation carries on.
        Assert.True(sprites > 0);
        for (int i = 0; i < 10; i++)
        {
            core.RunFrame(0);
        }

        Assert.True(core.IsRunning);
    }

    [Fact] // REQ-CORE-033 (the "auto" part: emulation is identical with and without the callback)
    public void Registering_the_callback_changes_nothing()
    {
        static (byte[] Ram, long Cycles) Run(bool withCallback)
        {
            NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
            SpecRom.Oam(core, 0, 0x30, 0x12, 0x00, 0x40);
            core.WriteMemory(0x2000, 0x08);
            core.WriteMemory(0x2001, 0x1E);
            if (withCallback)
            {
                core.VideoCallback = (in FrameBatch _) => { };
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

            return (ram, core.CycleCount);
        }

        (byte[] withRam, long withCycles) = Run(true);
        (byte[] withoutRam, long withoutCycles) = Run(false);

        Assert.Equal(withoutRam, withRam);
        Assert.Equal(withoutCycles, withCycles);
    }

    [Fact] // REQ-CORE-048
    public void The_optional_2d_picture_is_offered()
    {
        NesCore core = SpecRom.Warm(SpecRom.Mirroring.Vertical);
        core.RunFrame(0);
        ReadOnlySpan<byte> picture = core.PixelFrame;
        Assert.True(picture.IsEmpty || picture.Length == 256 * 240);
    }
}
