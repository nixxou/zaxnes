namespace Zax.Nes.Tests;

/// <summary>
/// PPU rendering seen from the outside: the optional 256 x 240 indexed image
/// (REQ-CORE-048), nametable mirroring, the scroll registers, $2007 and the PPUSTATUS flags
/// (REQ-CORE-112, REQ-CORE-116).
/// </summary>
public sealed class PpuRenderingTests
{
    /// <summary>Pattern $FF: plane 0 all ones, plane 1 all zeros, so every pixel is colour 1.</summary>
    private const byte SolidTile = 0xFF;

    private static byte PixelAt(NesCore core, int x, int y)
        => core.PixelFrame[(y * Ppu.Ppu2C02.ScreenWidth) + x];

    [Fact]
    public void REQ_CORE_048_a_disabled_screen_is_filled_with_the_universal_backdrop()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F00, 0x21);
        fixture.SetMask(0x00);

        fixture.NextBatch();

        Assert.Equal(256 * 240, fixture.Core.PixelFrame.Length);
        Assert.All(fixture.Core.PixelFrame.ToArray(), static color => Assert.Equal(0x21, color));
    }

    [Fact]
    public void REQ_CORE_048_a_solid_background_fills_the_image_with_its_palette_colour()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F00, 0x0F);
        fixture.WriteVram(0x3F01, 0x30);
        fixture.FillVram(0x2000, 960, SolidTile);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0A);

        fixture.NextBatch();

        Assert.Equal(0x30, PixelAt(fixture.Core, 0, 0));
        Assert.Equal(0x30, PixelAt(fixture.Core, 128, 120));
        Assert.Equal(0x30, PixelAt(fixture.Core, 255, 239));
    }

    [Fact]
    public void REQ_CORE_048_the_left_column_mask_blanks_the_first_eight_background_pixels()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F00, 0x0F);
        fixture.WriteVram(0x3F01, 0x30);
        fixture.FillVram(0x2000, 960, SolidTile);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x08); // background on, leftmost column hidden.

        fixture.NextBatch();

        for (int x = 0; x < 8; x++)
        {
            Assert.Equal(0x0F, PixelAt(fixture.Core, x, 100));
        }

        Assert.Equal(0x30, PixelAt(fixture.Core, 8, 100));

        // The records themselves are untouched by the mask: it only shows up in maskX
        // (REQ-CORE-041, notes; REQ-CORE-046).
        Assert.Equal(8, fixture.Core.MaskX);
        Assert.Equal(32, fixture.Core.TileRecords.Length / 30);
    }

    [Fact]
    public void REQ_CORE_047_the_greyscale_bit_only_changes_the_image()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F00, 0x0F);
        fixture.WriteVram(0x3F01, 0x16);
        fixture.FillVram(0x2000, 960, SolidTile);
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x0B); // background on, greyscale on.

        BatchCopy batch = fixture.NextBatch();

        Assert.Equal(0x10, PixelAt(fixture.Core, 100, 100)); // $16 & $30
        Assert.Equal(0x16, batch.Palette[1]);                // the batch palette is untouched
    }

    [Fact]
    public void REQ_CORE_048_a_sprite_is_drawn_over_the_background_and_behind_it_when_hidden()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F00, 0x0F);
        fixture.WriteVram(0x3F01, 0x01); // background colour 1
        fixture.WriteVram(0x3F11, 0x02); // sprite sub-palette 0, colour 1
        fixture.FillVram(0x2000, 960, SolidTile);

        fixture.SetSprite(0, 99, SolidTile, 0x00, 100); // in front
        fixture.SetSprite(1, 99, SolidTile, 0x20, 120); // behind the background
        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x1E);

        fixture.NextBatch();

        Assert.Equal(0x02, PixelAt(fixture.Core, 103, 103));
        Assert.Equal(0x01, PixelAt(fixture.Core, 123, 103));
    }

    [Fact]
    public void REQ_CORE_112_sprite_zero_hit_needs_a_solid_pixel_in_both_layers()
    {
        static bool Hit(byte mask, byte backgroundTile)
        {
            var fixture = new TileFixture();
            fixture.FillVram(0x2000, 960, backgroundTile);
            fixture.SetSprite(0, 99, SolidTile, 0x00, 100);
            fixture.SetScroll(0, 0);
            fixture.SetControl(0x00);
            fixture.SetMask(mask);

            fixture.Core.RunFrame(0);
            fixture.Core.RunFrame(0);
            return fixture.Core.Ppu.SpriteZeroHit;
        }

        Assert.True(Hit(0x1E, SolidTile));
        Assert.False(Hit(0x16, SolidTile)); // sprites off
        Assert.False(Hit(0x0E, SolidTile)); // background off
        Assert.False(Hit(0x1E, 0x00));      // transparent background
    }

    [Fact]
    public void REQ_CORE_116_the_overflow_flag_rises_once_nine_sprites_share_a_line()
    {
        static bool Overflow(int sprites)
        {
            var fixture = new TileFixture();
            for (int n = 0; n < sprites; n++)
            {
                fixture.SetSprite(n, 99, SolidTile, 0x00, (byte)(8 * n));
            }

            fixture.SetScroll(0, 0);
            fixture.SetControl(0x00);
            fixture.SetMask(0x1E);

            fixture.Core.RunFrame(0);
            fixture.Core.RunFrame(0);
            return fixture.Core.Ppu.SpriteOverflow;
        }

        Assert.False(Overflow(8));
        Assert.True(Overflow(9));
    }

    [Fact]
    public void REQ_CORE_041_every_sprite_of_a_line_is_emitted_even_though_only_8_are_drawn()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F00, 0x0F);
        fixture.WriteVram(0x3F11, 0x30);

        // 32 sprites side by side on the same line, 8 pixels apart.
        for (int n = 0; n < 32; n++)
        {
            fixture.SetSprite(n, 99, SolidTile, 0x00, (byte)(8 * n));
        }

        fixture.SetScroll(0, 0);
        fixture.SetControl(0x00);
        fixture.SetMask(0x1E);

        TileRecord[] sprites = fixture.NextBatch().Sprites;

        // All 32 are emitted: the hardware limit of 8 per line does not apply to the records
        // (REQ-CORE-041, notes).
        Assert.Equal(32, sprites.Length);

        // The drawn image, on the other hand, shows the hardware's 8.
        for (int x = 0; x < 64; x++)
        {
            Assert.Equal(0x30, PixelAt(fixture.Core, x, 103));
        }

        for (int x = 64; x < 256; x++)
        {
            Assert.Equal(0x0F, PixelAt(fixture.Core, x, 103));
        }
    }

    [Theory]
    [InlineData(0x00, 0x2000, 0x2400)] // horizontal: $2000 = $2400
    [InlineData(0x00, 0x2800, 0x2C00)]
    [InlineData(0x01, 0x2000, 0x2800)] // vertical: $2000 = $2800
    [InlineData(0x01, 0x2400, 0x2C00)]
    public void REQ_CORE_012_the_header_mirroring_folds_the_four_logical_nametables(
        byte headerByte6,
        int first,
        int second)
    {
        var fixture = new TileFixture(headerByte6);

        fixture.WriteVram((ushort)first, 0x5A);

        Assert.Equal(
            fixture.Core.Ppu.MapNametable((ushort)first),
            fixture.Core.Ppu.MapNametable((ushort)second));
        Assert.Equal(0x5A, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable((ushort)second)]);
    }

    [Fact]
    public void REQ_CORE_012_four_screen_mirroring_keeps_the_four_nametables_apart()
    {
        var fixture = new TileFixture(headerByte6: 0x08);

        fixture.WriteVram(0x2000, 0x01);
        fixture.WriteVram(0x2400, 0x02);
        fixture.WriteVram(0x2800, 0x03);
        fixture.WriteVram(0x2C00, 0x04);

        Assert.Equal(0x01, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable(0x2000)]);
        Assert.Equal(0x02, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable(0x2400)]);
        Assert.Equal(0x03, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable(0x2800)]);
        Assert.Equal(0x04, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable(0x2C00)]);

        // $3000-$3EFF mirrors $2000-$2EFF.
        Assert.Equal(
            fixture.Core.Ppu.MapNametable(0x2400),
            fixture.Core.Ppu.MapNametable(0x3400));
    }

    [Fact]
    public void REQ_CORE_112_reads_of_2007_outside_the_palette_go_through_a_one_byte_buffer()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x2000, 0x11, 0x22, 0x33);

        // Point the address back at $2000: the first read returns the stale buffer, the
        // following ones lag one byte behind (nesdev, "PPU registers"; vram_access).
        fixture.Core.WriteMemory(0x2006, 0x20);
        fixture.Core.WriteMemory(0x2006, 0x00);

        fixture.Core.Ppu.Read(7);
        Assert.Equal(0x11, fixture.Core.Ppu.Read(7));
        Assert.Equal(0x22, fixture.Core.Ppu.Read(7));
        Assert.Equal(0x33, fixture.Core.Ppu.Read(7));
    }

    [Fact]
    public void REQ_CORE_112_palette_reads_are_immediate_and_six_bits_wide()
    {
        var fixture = new TileFixture();
        fixture.WriteVram(0x3F01, 0xFF);

        fixture.Core.WriteMemory(0x2006, 0x3F);
        fixture.Core.WriteMemory(0x2006, 0x01);

        // Only six bits are stored, and no buffer gets in the way.
        Assert.Equal(0x3F, fixture.Core.Ppu.Read(7) & 0x3F);
    }

    [Fact]
    public void REQ_CORE_112_the_vram_address_increments_by_1_or_by_32()
    {
        var fixture = new TileFixture();

        fixture.Core.WriteMemory(0x2000, 0x00);
        fixture.WriteVram(0x2000, 0xA0, 0xA1);
        Assert.Equal(0xA0, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable(0x2000)]);
        Assert.Equal(0xA1, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable(0x2001)]);

        fixture.Core.WriteMemory(0x2000, 0x04); // increment by 32
        fixture.WriteVram(0x2100, 0xB0, 0xB1);
        Assert.Equal(0xB0, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable(0x2100)]);
        Assert.Equal(0xB1, fixture.Core.Ppu.Vram[fixture.Core.Ppu.MapNametable(0x2120)]);
    }

    [Fact]
    public void REQ_CORE_112_the_scroll_registers_follow_the_documented_loopy_layout()
    {
        var fixture = new TileFixture();
        Ppu.Ppu2C02 ppu = fixture.Core.Ppu;

        // The first $2005 write latches fine X straight away; the rest of the scroll goes to
        // the temporary register t and only reaches the internal address on the pre-render
        // line, which is what the V4 to V6 vectors of the tile contract exercise.
        fixture.SetScroll(13, 200);
        Assert.Equal(5, ppu.FineX);

        // A pair of $2006 writes loads the whole address at once.
        fixture.Core.WriteMemory(0x2006, 0x2C);
        fixture.Core.WriteMemory(0x2006, 0x15);
        Assert.Equal(0x15 & 0x1F, ppu.CoarseX);
        Assert.Equal(((0x2C15 >> 5) & 0x1F), ppu.CoarseY);
        Assert.Equal((0x2C15 >> 10) & 0x03, ppu.NametableSelect);
        Assert.Equal((0x2C15 >> 12) & 0x07, ppu.FineY);
    }

    [Fact]
    public void REQ_CORE_112_reading_2002_clears_the_vblank_flag_and_the_write_toggle()
    {
        var fixture = new TileFixture();
        Ppu.Ppu2C02 ppu = fixture.Core.Ppu;

        fixture.Core.RunFrame(0);
        Assert.True(ppu.VBlankFlag);
        Assert.Equal(0x80, ppu.Read(2) & 0x80);
        Assert.False(ppu.VBlankFlag);
        Assert.Equal(0x00, ppu.Read(2) & 0x80);

        // One $2006 write then a read of $2002 resets the toggle, so the next $2006 write is
        // treated as a first write again.
        fixture.Core.WriteMemory(0x2006, 0x21);
        ppu.Read(2);
        fixture.Core.WriteMemory(0x2006, 0x2C);
        fixture.Core.WriteMemory(0x2006, 0x15);
        Assert.Equal((0x2C15 >> 10) & 0x03, ppu.NametableSelect);
    }
}
