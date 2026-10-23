//! PPU rendering seen from the outside: the optional 256 x 240 indexed image
//! (REQ-CORE-048), nametable mirroring, the scroll registers, $2007 and the PPUSTATUS flags
//! (REQ-CORE-112, REQ-CORE-116).

mod common;

use common::TileFixture;
use zax_nes::ppu::SCREEN_WIDTH;
use zax_nes::Machine;

/// Pattern $FF: plane 0 all ones, plane 1 all zeros, so every pixel is colour 1.
const SOLID_TILE: u8 = 0xFF;

fn pixel_at(core: &Machine, x: usize, y: usize) -> u8 {
    core.pixel_frame()[(y * SCREEN_WIDTH) + x]
}

#[test]
fn req_core_048_a_disabled_screen_is_filled_with_the_universal_backdrop() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x21]);
    fixture.set_mask(0x00);
    fixture.next_batch();

    assert_eq!(256 * 240, fixture.core.pixel_frame().len());
    assert!(fixture
        .core
        .pixel_frame()
        .iter()
        .all(|&color| color == 0x21));
}

#[test]
fn req_core_048_a_solid_background_fills_the_image_with_its_palette_colour() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F01, &[0x30]);
    fixture.fill_vram(0x2000, 960, SOLID_TILE);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);
    fixture.next_batch();

    assert_eq!(0x30, pixel_at(&fixture.core, 0, 0));
    assert_eq!(0x30, pixel_at(&fixture.core, 128, 120));
    assert_eq!(0x30, pixel_at(&fixture.core, 255, 239));
}

#[test]
fn req_core_048_the_left_column_mask_blanks_the_first_eight_background_pixels() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F01, &[0x30]);
    fixture.fill_vram(0x2000, 960, SOLID_TILE);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x08); // background on, leftmost column hidden.
    fixture.next_batch();

    for x in 0..8 {
        assert_eq!(0x0F, pixel_at(&fixture.core, x, 100));
    }
    assert_eq!(0x30, pixel_at(&fixture.core, 8, 100));

    // The records themselves are untouched by the mask: it only shows up in maskX
    // (REQ-CORE-041, notes; REQ-CORE-046).
    assert_eq!(8, fixture.core.ppu().mask_x());
    assert_eq!(32, fixture.core.tile_records().len() / 30);
}

#[test]
fn req_core_047_the_greyscale_bit_only_changes_the_image() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F01, &[0x16]);
    fixture.fill_vram(0x2000, 960, SOLID_TILE);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x0B); // background on, greyscale on.
    let batch = fixture.next_batch();

    assert_eq!(0x10, pixel_at(&fixture.core, 100, 100)); // $16 & $30
    assert_eq!(0x16, batch.palette[1]); // the batch palette is untouched
}

#[test]
fn req_core_048_a_sprite_is_drawn_over_the_background_and_behind_it_when_hidden() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F01, &[0x01]); // background colour 1
    fixture.write_vram(0x3F11, &[0x02]); // sprite sub-palette 0, colour 1
    fixture.fill_vram(0x2000, 960, SOLID_TILE);
    fixture.set_sprite(0, 99, SOLID_TILE, 0x00, 100); // in front
    fixture.set_sprite(1, 99, SOLID_TILE, 0x20, 120); // behind the background
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x1E);
    fixture.next_batch();

    assert_eq!(0x02, pixel_at(&fixture.core, 103, 103));
    assert_eq!(0x01, pixel_at(&fixture.core, 123, 103));
}

#[test]
fn req_core_112_sprite_zero_hit_needs_a_solid_pixel_in_both_layers() {
    fn hit(mask: u8, background_tile: u8) -> bool {
        let mut fixture = TileFixture::new();
        fixture.fill_vram(0x2000, 960, background_tile);
        fixture.set_sprite(0, 99, SOLID_TILE, 0x00, 100);
        fixture.set_scroll(0, 0);
        fixture.set_control(0x00);
        fixture.set_mask(mask);
        fixture.core.run_frame();
        fixture.core.run_frame();
        fixture.core.ppu().sprite_zero_hit()
    }

    assert!(hit(0x1E, SOLID_TILE));
    assert!(!hit(0x16, SOLID_TILE)); // sprites off
    assert!(!hit(0x0E, SOLID_TILE)); // background off
    assert!(!hit(0x1E, 0x00)); // transparent background
}

#[test]
fn req_core_116_the_overflow_flag_rises_once_nine_sprites_share_a_line() {
    fn overflow(sprites: u8) -> bool {
        let mut fixture = TileFixture::new();
        for n in 0..sprites {
            fixture.set_sprite(n, 99, SOLID_TILE, 0x00, 8 * n);
        }
        fixture.set_scroll(0, 0);
        fixture.set_control(0x00);
        fixture.set_mask(0x1E);
        fixture.core.run_frame();
        fixture.core.run_frame();
        fixture.core.ppu().sprite_overflow()
    }

    assert!(!overflow(8));
    assert!(overflow(9));
}

#[test]
fn req_core_041_every_sprite_of_a_line_is_emitted_even_though_only_8_are_drawn() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F11, &[0x30]);

    // 32 sprites side by side on the same line, 8 pixels apart.
    for n in 0..32 {
        fixture.set_sprite(n, 99, SOLID_TILE, 0x00, 8 * n);
    }
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x1E);
    let sprites = fixture.next_batch().sprites();

    // All 32 are emitted: the hardware limit of 8 per line does not apply to the records
    // (REQ-CORE-041, notes).
    assert_eq!(32, sprites.len());

    // The drawn image, on the other hand, shows the hardware's 8.
    for x in 0..64 {
        assert_eq!(0x30, pixel_at(&fixture.core, x, 103));
    }
    for x in 64..256 {
        assert_eq!(0x0F, pixel_at(&fixture.core, x, 103));
    }
}

#[test]
fn req_core_012_the_header_mirroring_folds_the_four_logical_nametables() {
    for (header_byte6, first, second) in [
        (0x00, 0x2000, 0x2400), // horizontal: $2000 = $2400
        (0x00, 0x2800, 0x2C00),
        (0x01, 0x2000, 0x2800), // vertical: $2000 = $2800
        (0x01, 0x2400, 0x2C00),
    ] {
        let mut fixture = TileFixture::with(header_byte6, None);
        fixture.write_vram(first, &[0x5A]);

        let core = &fixture.core;
        assert_eq!(core.map_nametable(first), core.map_nametable(second));
        assert_eq!(0x5A, core.ppu().vram()[core.map_nametable(second)]);
    }
}

#[test]
fn req_core_012_four_screen_mirroring_keeps_the_four_nametables_apart() {
    let mut fixture = TileFixture::with(0x08, None);
    fixture.write_vram(0x2000, &[0x01]);
    fixture.write_vram(0x2400, &[0x02]);
    fixture.write_vram(0x2800, &[0x03]);
    fixture.write_vram(0x2C00, &[0x04]);

    let core = &fixture.core;
    let vram = core.ppu().vram();
    assert_eq!(0x01, vram[core.map_nametable(0x2000)]);
    assert_eq!(0x02, vram[core.map_nametable(0x2400)]);
    assert_eq!(0x03, vram[core.map_nametable(0x2800)]);
    assert_eq!(0x04, vram[core.map_nametable(0x2C00)]);

    // $3000-$3EFF mirrors $2000-$2EFF.
    assert_eq!(core.map_nametable(0x2400), core.map_nametable(0x3400));
}

#[test]
fn req_core_112_reads_of_2007_outside_the_palette_go_through_a_one_byte_buffer() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x2000, &[0x11, 0x22, 0x33]);

    // Point the address back at $2000: the first read returns the stale buffer, the
    // following ones lag one byte behind (nesdev, "PPU registers"; vram_access).
    fixture.core.write_memory(0x2006, 0x20);
    fixture.core.write_memory(0x2006, 0x00);
    fixture.core.ppu_read(7);
    assert_eq!(0x11, fixture.core.ppu_read(7));
    assert_eq!(0x22, fixture.core.ppu_read(7));
    assert_eq!(0x33, fixture.core.ppu_read(7));
}

#[test]
fn req_core_112_palette_reads_are_immediate_and_six_bits_wide() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F01, &[0xFF]);
    fixture.core.write_memory(0x2006, 0x3F);
    fixture.core.write_memory(0x2006, 0x01);

    // Only six bits are stored, and no buffer gets in the way.
    assert_eq!(0x3F, fixture.core.ppu_read(7) & 0x3F);
}

#[test]
fn req_core_112_the_vram_address_increments_by_1_or_by_32() {
    let mut fixture = TileFixture::new();
    fixture.core.write_memory(0x2000, 0x00);
    fixture.write_vram(0x2000, &[0xA0, 0xA1]);
    {
        let core = &fixture.core;
        assert_eq!(0xA0, core.ppu().vram()[core.map_nametable(0x2000)]);
        assert_eq!(0xA1, core.ppu().vram()[core.map_nametable(0x2001)]);
    }

    fixture.core.write_memory(0x2000, 0x04); // increment by 32
    fixture.write_vram(0x2100, &[0xB0, 0xB1]);
    let core = &fixture.core;
    assert_eq!(0xB0, core.ppu().vram()[core.map_nametable(0x2100)]);
    assert_eq!(0xB1, core.ppu().vram()[core.map_nametable(0x2120)]);
}

#[test]
fn req_core_112_the_scroll_registers_follow_the_documented_loopy_layout() {
    let mut fixture = TileFixture::new();

    // The first $2005 write latches fine X straight away; the rest of the scroll goes to
    // the temporary register t and only reaches the internal address on the pre-render
    // line, which is what the V4 to V6 vectors of the tile contract exercise.
    fixture.set_scroll(13, 200);
    assert_eq!(5, fixture.core.ppu().fine_x());

    // A pair of $2006 writes loads the whole address at once.
    fixture.core.write_memory(0x2006, 0x2C);
    fixture.core.write_memory(0x2006, 0x15);
    let ppu = fixture.core.ppu();
    assert_eq!(0x15 & 0x1F, ppu.coarse_x());
    assert_eq!((0x2C15 >> 5) & 0x1F, ppu.coarse_y());
    assert_eq!((0x2C15 >> 10) & 0x03, ppu.nametable_select());
    assert_eq!((0x2C15 >> 12) & 0x07, ppu.fine_y());
}

#[test]
fn req_core_112_reading_2002_clears_the_vblank_flag_and_the_write_toggle() {
    let mut fixture = TileFixture::new();
    fixture.core.run_frame();

    assert!(fixture.core.ppu().vblank_flag());
    assert_eq!(0x80, fixture.core.ppu_read(2) & 0x80);
    assert!(!fixture.core.ppu().vblank_flag());
    assert_eq!(0x00, fixture.core.ppu_read(2) & 0x80);

    // One $2006 write then a read of $2002 resets the toggle, so the next $2006 write is
    // treated as a first write again.
    fixture.core.write_memory(0x2006, 0x21);
    fixture.core.ppu_read(2);
    fixture.core.write_memory(0x2006, 0x2C);
    fixture.core.write_memory(0x2006, 0x15);
    assert_eq!((0x2C15 >> 10) & 0x03, fixture.core.ppu().nametable_select());
}
