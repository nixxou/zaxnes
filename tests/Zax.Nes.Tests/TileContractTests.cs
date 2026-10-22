namespace Zax.Nes.Tests;

/// <summary>
/// The 3dSen tile emission contract: every worked example of SPEC-02 §5.3 (V1 to V10) plus
/// the requirements they anchor (REQ-CORE-030 to REQ-CORE-047).
/// </summary>
public sealed class TileContractTests
{
    // ------------------------------------------------------------------ V1

    [Fact]
    public void REQ_CORE_037_V1_a_bare_background_gives_960_records_in_30_rows()
    {
        var fixture = new TileFixture();
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(960, batch.Records.Length);
        Assert.All(batch.Records, static record => Assert.True(record.Background));

        for (int row = 0; row < 30; row++)
        {
            for (int k = 0; k < 32; k++)
            {
                TileRecord record = batch.Records[(row * 32) + k];
                Assert.Equal(8 * k, record.X);
                Assert.Equal(8 * row, record.Y);
                Assert.Equal((33 * row) + k, record.Index);
                Assert.Equal(0, record.Palette);
                Assert.Equal(0UL, record.Plane0);
                Assert.Equal(0UL, record.Plane1);
                Assert.False(record.MirrorH);
                Assert.False(record.MirrorV);
                Assert.False(record.Hidden);
            }
        }

        // The worked example of REQ-CORE-040: row 1, k = 4 is x = 32, y = 8, index 37.
        TileRecord sample = batch.Records[32 + 4];
        Assert.Equal(32, sample.X);
        Assert.Equal(8, sample.Y);
        Assert.Equal(37, sample.Index);

        // maskX is 8 because bit 2 of PPUMASK is clear; maskY is 8 because the mirroring is
        // vertical (REQ-CORE-046).
        Assert.Equal(8, batch.MaskX);
        Assert.Equal(8, batch.MaskY);
    }

    [Fact]
    public void REQ_CORE_036_V1_empty_patterns_are_still_emitted()
    {
        var fixture = new TileFixture();
        fixture.SetScroll(0, 0);
        fixture.SetMask(0x0A);

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(960, batch.Records.Length);
        Assert.All(batch.Records, static record => Assert.True(record.IsEmpty));
    }

    // ------------------------------------------------------------------ V2

    [Fact]
    public void REQ_CORE_042_V2_an_8x8_sprite_carries_its_attributes_and_planes()
    {
        var fixture = new TileFixture();
        fixture.SetSprite(5, 0x50, 0x21, 0x63, 0x80);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x08);
        fixture.SetMask(0x1E);

        BatchCopy batch = fixture.NextBatch();
        TileRecord[] sprites = batch.Sprites;

        TileRecord sprite = Assert.Single(sprites);
        Assert.False(sprite.Background);
        Assert.Equal(128, sprite.X);
        Assert.Equal(81, sprite.Y);
        Assert.Equal(5, sprite.Index);
        Assert.Equal(3, sprite.Palette);
        Assert.True(sprite.Hidden);
        Assert.True(sprite.MirrorH);
        Assert.False(sprite.MirrorV);

        (ulong plane0, ulong plane1) = TestRoms.Pattern(1, 0x21);
        Assert.Equal(0x2121212121212121UL, plane0);
        Assert.Equal(0xDEDEDEDEDEDEDEDEUL, plane1);
        Assert.Equal(plane0, sprite.Plane0);
        Assert.Equal(plane1, sprite.Plane1);
    }

    [Fact]
    public void REQ_CORE_044_V2_the_sprite_sits_between_the_rows_of_lines_81_and_89()
    {
        var fixture = new TileFixture();
        fixture.SetSprite(5, 0x50, 0x21, 0x63, 0x80);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x08);
        fixture.SetMask(0x1E);

        BatchCopy batch = fixture.NextBatch();

        int spriteAt = Array.FindIndex(batch.Records, static record => !record.Background);
        Assert.True(spriteAt > 0);

        // The sprite is captured on line 87, so the row before it is the one of line 81
        // (y = 80) and the row after it is the one of line 89 (y = 88).
        Assert.Equal(80, batch.Records[spriteAt - 1].Y);
        Assert.True(batch.Records[spriteAt - 1].Background);
        Assert.Equal(88, batch.Records[spriteAt + 1].Y);
        Assert.True(batch.Records[spriteAt + 1].Background);
    }

    // ------------------------------------------------------------------ V3

    [Fact]
    public void REQ_CORE_042_V3_an_8x16_sprite_emits_two_halves_swapped_by_the_vertical_flip()
    {
        var fixture = new TileFixture();
        fixture.SetSprite(2, 0x20, 0x15, 0x80, 0x10);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x20);
        fixture.SetMask(0x1E);

        TileRecord[] sprites = fixture.NextBatch().Sprites;

        Assert.Equal(2, sprites.Length);

        Assert.Equal(2, sprites[0].Index);
        Assert.Equal(16, sprites[0].X);
        Assert.Equal(33, sprites[0].Y);
        Assert.True(sprites[0].MirrorV);
        Assert.Equal(TestRoms.Pattern(1, 0x15).Plane0, sprites[0].Plane0);
        Assert.Equal(TestRoms.Pattern(1, 0x15).Plane1, sprites[0].Plane1);

        Assert.Equal(66, sprites[1].Index);
        Assert.Equal(16, sprites[1].X);
        Assert.Equal(41, sprites[1].Y);
        Assert.True(sprites[1].MirrorV);
        Assert.Equal(TestRoms.Pattern(1, 0x14).Plane0, sprites[1].Plane0);
        Assert.Equal(TestRoms.Pattern(1, 0x14).Plane1, sprites[1].Plane1);
    }

    [Fact]
    public void REQ_CORE_042_V3_without_the_vertical_flip_the_halves_keep_their_order()
    {
        var fixture = new TileFixture();
        fixture.SetSprite(2, 0x20, 0x15, 0x00, 0x10);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x20);
        fixture.SetMask(0x1E);

        TileRecord[] sprites = fixture.NextBatch().Sprites;

        Assert.Equal(2, sprites.Length);
        Assert.Equal(2, sprites[0].Index);
        Assert.Equal(TestRoms.Pattern(1, 0x14).Plane0, sprites[0].Plane0);
        Assert.Equal(66, sprites[1].Index);
        Assert.Equal(TestRoms.Pattern(1, 0x15).Plane0, sprites[1].Plane0);
    }

    [Fact]
    public void REQ_CORE_042_V3_the_lower_half_is_dropped_when_it_would_fall_off_the_screen()
    {
        var fixture = new TileFixture();
        fixture.SetSprite(2, 0xE7, 0x15, 0x80, 0x10);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x20);
        fixture.SetMask(0x1E);

        TileRecord[] sprites = fixture.NextBatch().Sprites;

        TileRecord sprite = Assert.Single(sprites);
        Assert.Equal(2, sprite.Index);
        Assert.Equal(232, sprite.Y);
    }

    [Fact]
    public void REQ_CORE_041_sprites_at_or_below_line_240_are_never_emitted()
    {
        var fixture = new TileFixture();
        fixture.SetSprite(0, 0xEF, 0x01, 0x00, 0x10); // Ytop = 240.
        fixture.SetSprite(1, 0xFF, 0x01, 0x00, 0x20); // Ytop = 256.
        fixture.SetSprite(2, 0x00, 0x01, 0x00, 0x30); // Ytop = 1: emitted, y = 1.
        fixture.SetScroll(0, 0);
        fixture.SetMask(0x1E);

        TileRecord[] sprites = fixture.NextBatch().Sprites;

        TileRecord sprite = Assert.Single(sprites);
        Assert.Equal(2, sprite.Index);
        Assert.Equal(1, sprite.Y);
    }

    // ------------------------------------------------------------------ V4

    [Fact]
    public void REQ_CORE_038_V4_a_fine_x_scroll_adds_a_33rd_record_from_the_neighbouring_nametable()
    {
        var fixture = new TileFixture();

        // Nametable $2000: column c holds tile c. Nametable $2400: column c holds tile 32+c.
        for (int row = 0; row < 30; row++)
        {
            var columns = new byte[32];
            for (int c = 0; c < 32; c++)
            {
                columns[c] = (byte)c;
            }

            fixture.WriteVram((ushort)(0x2000 + (row * 32)), columns);

            for (int c = 0; c < 32; c++)
            {
                columns[c] = (byte)(32 + c);
            }

            fixture.WriteVram((ushort)(0x2400 + (row * 32)), columns);
        }

        fixture.SetScroll(13, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(30 * 33, batch.Records.Length);

        for (int k = 0; k <= 32; k++)
        {
            TileRecord record = batch.Records[k];
            Assert.Equal((8 * k) - 5, record.X);
            Assert.Equal(0, record.Y);
        }

        Assert.Equal(TestRoms.Pattern(0, 1).Plane0, batch.Records[0].Plane0);
        Assert.Equal(TestRoms.Pattern(0, 31).Plane0, batch.Records[30].Plane0);
        Assert.Equal(TestRoms.Pattern(0, 32).Plane0, batch.Records[31].Plane0);
        Assert.Equal(TestRoms.Pattern(0, 33).Plane0, batch.Records[32].Plane0);
    }

    // ------------------------------------------------------------------ V5

    [Fact]
    public void REQ_CORE_037_V5_a_fine_y_scroll_of_3_captures_rows_on_lines_8j_plus_6()
    {
        var fixture = new TileFixture();

        for (int row = 0; row < 30; row++)
        {
            fixture.FillVram((ushort)(0x2000 + (row * 32)), 32, (byte)row);
        }

        fixture.SetScroll(0, 3);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(30 * 32, batch.Records.Length);

        for (int j = 0; j < 30; j++)
        {
            TileRecord record = batch.Records[j * 32];
            Assert.Equal((8 * j) + 5, record.Y);
        }

        // The first captured row is nametable row 1: row 0 would sit at y = -3 and is not
        // emitted. Its line index is floor(6 / 8) = 0.
        Assert.Equal(TestRoms.Pattern(0, 1).Plane0, batch.Records[0].Plane0);
        Assert.Equal(0, batch.Records[0].Index);

        // The last one (line 238, y = 237) comes from row 0 of the vertically neighbouring
        // nametable $2800, which mirrors $2000 under vertical mirroring: tile 0.
        TileRecord last = batch.Records[29 * 32];
        Assert.Equal(237, last.Y);
        Assert.Equal(0UL, last.Plane0);
        Assert.Equal(0UL, last.Plane1);
    }

    // ------------------------------------------------------------------ V6

    [Fact]
    public void REQ_CORE_037_V6_vertical_mirroring_drops_the_row_of_line_0()
    {
        var fixture = new TileFixture();
        fixture.SetScroll(0, 1);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(7, batch.Records[0].Y);
        Assert.DoesNotContain(batch.Records, static record => record.Y == -1);
    }

    [Fact]
    public void REQ_CORE_037_V6_horizontal_mirroring_keeps_the_row_of_line_0()
    {
        var fixture = new TileFixture(headerByte6: 0x00);
        fixture.SetScroll(0, 1);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        BatchCopy batch = fixture.NextBatch();

        for (int k = 0; k < 32; k++)
        {
            Assert.Equal(-1, batch.Records[k].Y);
            Assert.Equal(k, batch.Records[k].Index);
        }
    }

    // ------------------------------------------------------------------ V7

    [Fact]
    public void REQ_CORE_045_V7_the_batch_palette_is_the_32_bytes_of_palette_ram()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F00, 0x0F);
        fixture.WriteVram(0x3F01, 0x30);
        fixture.WriteVram(0x3F11, 0x16);
        fixture.SetScroll(0, 0);
        fixture.SetMask(0x0A);

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(32, batch.Palette.Length);
        Assert.Equal(0x0F, batch.Palette[0]);
        Assert.Equal(0x30, batch.Palette[1]);
        Assert.Equal(0x16, batch.Palette[17]);

        fixture.WriteVram(0x3F10, 0x21);
        batch = fixture.NextBatch();

        Assert.Equal(0x21, batch.Palette[0]);
        Assert.Equal(0x21, batch.Palette[16]);
    }

    // ------------------------------------------------------------------ V8

    [Fact]
    public void REQ_CORE_044_V8_records_follow_the_chronological_order_of_the_capture_lines()
    {
        var fixture = new TileFixture();
        fixture.SetSprite(0, 0x00, 0x01, 0x00, 0x08);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x1E);

        BatchCopy batch = fixture.NextBatch();

        // 32 background records of line 1 (y = 0), then the sprite captured on line 7,
        // then the 32 background records of line 9 (y = 8).
        for (int k = 0; k < 32; k++)
        {
            Assert.True(batch.Records[k].Background);
            Assert.Equal(0, batch.Records[k].Y);
        }

        TileRecord sprite = batch.Records[32];
        Assert.False(sprite.Background);
        Assert.Equal(0, sprite.Index);
        Assert.Equal(1, sprite.Y);
        Assert.Equal(8, sprite.X);

        for (int k = 0; k < 32; k++)
        {
            Assert.True(batch.Records[33 + k].Background);
            Assert.Equal(8, batch.Records[33 + k].Y);
        }
    }

    // ------------------------------------------------------------------ V9

    [Theory]
    [InlineData(0x00, 0x1E, 8, 0)] // horizontal
    [InlineData(0x01, 0x1E, 0, 8)] // vertical
    [InlineData(0x01, 0x18, 8, 8)] // vertical, left column masked
    [InlineData(0x08, 0x1E, 0, 0)] // four-screen
    [InlineData(0x08, 0x1A, 8, 0)] // four-screen, sprites masked on the left
    public void REQ_CORE_046_V9_the_margins_follow_the_mirroring_and_the_left_column_mask(
        byte headerByte6,
        byte mask,
        int expectedMaskX,
        int expectedMaskY)
    {
        var fixture = new TileFixture(headerByte6);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(mask);

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(expectedMaskX, batch.MaskX);
        Assert.Equal(expectedMaskY, batch.MaskY);
    }

    // ------------------------------------------------------------------ V10

    [Fact]
    public void REQ_CORE_047_V10_greyscale_and_colour_emphasis_change_nothing_in_the_batch()
    {
        BatchCopy Run(byte mask)
        {
            var fixture = new TileFixture();
            fixture.SetSprite(5, 0x50, 0x21, 0x63, 0x80);
            fixture.SetScroll(0, 0);
            fixture.SetControl(0x08);
            fixture.SetMask(mask);
            return fixture.NextBatch();
        }

        BatchCopy plain = Run(0x1E);
        BatchCopy decorated = Run(0xFF);

        Assert.Equal<TileRecord[]>(plain.Records, decorated.Records);
        Assert.Equal<byte[]>(plain.Palette, decorated.Palette);
        Assert.Equal(plain.MaskX, decorated.MaskX);
        Assert.Equal(plain.MaskY, decorated.MaskY);
    }

    // ------------------------------------------------------------------ batch plumbing

    [Fact]
    public void REQ_CORE_021_the_callback_fires_once_per_frame_even_with_an_empty_batch()
    {
        var fixture = new TileFixture();
        fixture.SetMask(0x00);
        fixture.Core.RunFrame(0);

        int calls = 0;
        int records = -1;
        fixture.Core.VideoCallback = (in FrameBatch batch) =>
        {
            calls++;
            records = batch.Records.Length;
        };

        for (int i = 0; i < 5; i++)
        {
            fixture.Core.RunFrame(0);
        }

        Assert.Equal(5, calls);
        Assert.Equal(0, records);
    }

    [Fact]
    public void REQ_CORE_031_the_video_info_is_33_by_31_with_128_sprite_slots()
    {
        VideoInfo info = NesCore.VideoInfo;

        Assert.Equal(33, info.Width);
        Assert.Equal(31, info.Height);
        Assert.Equal(128, info.SpriteCount);
    }

    [Fact]
    public void REQ_CORE_032_the_batch_is_capped_at_1023_background_and_128_sprite_records()
    {
        // A program that rewrites $2006 in a tight loop with fine Y = 1, so that every one of
        // the 240 visible lines wants to emit a row: 7 680 background records, of which only
        // 1 023 fit. Sprites have their own budget and must survive.
        byte[] program =
        [
            0xA9, 0x18,             // LDA #$18        background and sprites on
            0x8D, 0x01, 0x20,       // STA $2001
            0xA9, 0x10,             // LDA #$10        fine Y = 1, coarse = 0
            0x8D, 0x06, 0x20,       // STA $2006
            0xA9, 0x00,             // LDA #$00
            0x8D, 0x06, 0x20,       // STA $2006
            0x4C, 0x05, 0x80,       // JMP $8005
        ];

        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(0x01, program), "Overflow.nes").Success);

        // OAM is cleared at power-on, so all 64 sprites sit at Ytop = 1 and are captured
        // together on line 7, well before the background budget runs out.
        BatchCopy batch = TileFixture.CaptureBatch(core);
        for (int i = 0; i < 4; i++)
        {
            batch = TileFixture.CaptureBatch(core);
        }

        Assert.Equal(1023, batch.Background.Length);
        Assert.Equal(64, batch.Sprites.Length);
        Assert.Equal(1023 + 64, batch.Records.Length);
        Assert.True(core.Ppu.DroppedBackgroundRecords > 0);
    }

    [Fact]
    public void REQ_CORE_092_the_frame_batches_survive_a_savestate_round_trip()
    {
        // A program that rewrites OAM through DMA with changing data, so that every frame
        // produces a different batch.
        byte[] program =
        [
            0xA9, 0x1E,             // LDA #$1E
            0x8D, 0x01, 0x20,       // STA $2001
            0xEE, 0x00, 0x02,       // INC $0200       loop
            0xEE, 0x03, 0x02,       // INC $0203
            0xA9, 0x02,             // LDA #$02
            0x8D, 0x14, 0x40,       // STA $4014       OAM DMA from page 2
            0x4C, 0x05, 0x80,       // JMP $8005
        ];

        ulong Run(int saveRestoreAtFrame)
        {
            var core = new NesCore();
            Assert.True(core.LoadRom(TestRoms.Minimal(0x01, program), "Batches.nes").Success);

            ulong hash = 1469598103934665603UL;
            core.VideoCallback = (in FrameBatch batch) =>
            {
                foreach (TileRecord record in batch.Records)
                {
                    hash = Mix(hash, record.Plane0);
                    hash = Mix(hash, record.Plane1);
                    hash = Mix(hash, (ulong)(ushort)record.X);
                    hash = Mix(hash, (ulong)(ushort)record.Y);
                    hash = Mix(hash, (ulong)record.Index);
                    hash = Mix(hash, record.Palette);
                }

                foreach (byte entry in batch.Palette)
                {
                    hash = Mix(hash, entry);
                }

                hash = Mix(hash, (ulong)batch.MaskX);
                hash = Mix(hash, (ulong)batch.MaskY);
            };

            for (int frame = 0; frame < 60; frame++)
            {
                core.RunFrame((ushort)frame);

                if (frame == saveRestoreAtFrame)
                {
                    byte[] state = core.SaveState();
                    Assert.True(core.TryLoadState(state, 0, out _, out _, out string? error), error);
                }
            }

            return hash;
        }

        ulong straight = Run(-1);
        Assert.NotEqual(1469598103934665603UL, straight);
        Assert.Equal(straight, Run(30));
    }

    private static ulong Mix(ulong hash, ulong value)
    {
        for (int i = 0; i < 8; i++)
        {
            hash = (hash ^ (byte)(value >> (8 * i))) * 1099511628211UL;
        }

        return hash;
    }

    [Fact]
    public void REQ_CORE_033_registering_the_video_callback_changes_nothing_in_the_emulation()
    {
        byte[] program =
        [
            0xA9, 0x1E,             // LDA #$1E
            0x8D, 0x01, 0x20,       // STA $2001
            0xE6, 0x20,             // INC $20
            0xA5, 0x20,             // LDA $20
            0x8D, 0x00, 0x03,       // STA $0300
            0x4C, 0x05, 0x80,       // JMP $8005
        ];

        byte[] rom = TestRoms.Minimal(0x01, program);

        var silent = new NesCore();
        var watched = new NesCore();
        Assert.True(silent.LoadRom(rom, "T.nes").Success);
        Assert.True(watched.LoadRom(rom, "T.nes").Success);
        watched.VideoCallback = static (in FrameBatch _) => { };

        for (int i = 0; i < 120; i++)
        {
            silent.RunFrame(0x00FF);
            watched.RunFrame(0x00FF);
        }

        Assert.Equal(silent.CycleCount, watched.CycleCount);
        for (int address = 0; address < 0x800; address++)
        {
            Assert.Equal(silent.ReadMemory((ushort)address), watched.ReadMemory((ushort)address));
        }

        Assert.Equal<TileRecord[]>([.. silent.TileRecords], [.. watched.TileRecords]);
    }

    // ------------------------------------------------------------------ AMD-021 R2-005

    /// <summary>
    /// Runs to the middle of visible line <paramref name="line"/> of the next frame, writes
    /// <paramref name="address"/> to $2006, and returns the batch of that frame.
    /// </summary>
    /// <remarks>
    /// The write is a host memory write (REQ-CORE-082), which takes the same path through the
    /// PPU as a <c>STA $2006</c> would and leaves the PPU clock where it is: that is what lets
    /// the write be placed on an exact line without counting cycles.
    /// </remarks>
    private static BatchCopy WriteAddressOnLine(TileFixture fixture, int line, ushort address)
    {
        NesCore core = fixture.Core;
        core.RunFrame(0);

        int guard = 0;
        while ((core.Ppu.Scanline != line || core.Ppu.Dot < 120) && guard++ < 400_000)
        {
            core.Cpu.Step();
        }

        Assert.Equal(line, core.Ppu.Scanline);

        core.WriteMemory(0x2006, (byte)(address >> 8));
        core.WriteMemory(0x2006, (byte)address);

        BatchCopy? copy = null;
        core.VideoCallback = (in FrameBatch batch) => copy = new BatchCopy
        {
            Records = batch.Records.ToArray(),
            Palette = batch.Palette.ToArray(),
            MaskX = batch.MaskX,
            MaskY = batch.MaskY,
        };

        core.RunFrame(0);
        core.VideoCallback = null;
        Assert.NotNull(copy);
        return copy;
    }

    [Fact] // AMD-021 R2-005, criterion 1, first sentence
    public void AMD_021_R2_005_a_2006_write_during_line_100_emits_the_row_of_line_100()
    {
        var fixture = new TileFixture(headerByte6: 0x00);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        // Fine Y 1, coarse Y 12, coarse X 0, nametable 0: v = $1180.
        BatchCopy batch = WriteAddressOnLine(fixture, 100, 0x1180);

        // The row of line 100 carries y = line - 1 = 99 (REQ-CORE-038). Before R2-005 the
        // write only reached the capture of line 101, which would have emitted y = 100.
        Assert.Contains(batch.Background, static record => record.Y == 99);
        Assert.DoesNotContain(batch.Background, static record => record.Y == 100);
    }

    [Fact] // AMD-021 R2-005, criterion 1, second sentence
    public void AMD_021_R2_005_writing_fine_y_2_during_a_fine_y_1_line_drops_its_row()
    {
        var fixture = new TileFixture(headerByte6: 0x00);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        // Line 97 has fine Y 1 with no scroll, so it would normally emit the row y = 96.
        BatchCopy reference = WriteAddressOnLine(fixture, 97, 0x1180);
        Assert.Contains(reference.Background, static record => record.Y == 96);

        // Writing fine Y 2 during that same line removes the row entirely: the condition
        // "fine Y = 1" is evaluated on the state at the end of the line.
        var again = new TileFixture(headerByte6: 0x00);
        again.SetScroll(0, 0);
        again.SetControl(0x00);
        again.SetMask(0x0A);

        BatchCopy batch = WriteAddressOnLine(again, 97, 0x2180);
        Assert.DoesNotContain(batch.Background, static record => record.Y == 96);
    }

    [Fact] // AMD-021 R2-005 (the two end-of-line reloads stay out of the row of the line)
    public void AMD_021_R2_005_the_end_of_line_reloads_only_reach_the_next_line()
    {
        // With no mid-line write at all, the rows are exactly the ones of V1: the vertical
        // increment of dot 256 and the horizontal reload of dot 257 of line L must land on
        // line L + 1, not on the row captured for line L.
        var fixture = new TileFixture(headerByte6: 0x00);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(960, batch.Background.Length);
        for (int row = 0; row < 30; row++)
        {
            Assert.Equal(8 * row, batch.Background[row * 32].Y);
            Assert.Equal(0, batch.Background[row * 32].X);
        }
    }
}
