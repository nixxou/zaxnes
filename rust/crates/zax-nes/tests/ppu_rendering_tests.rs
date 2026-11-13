//! PPU rendering seen from the outside: the optional 256 x 240 indexed image
//! (REQ-CORE-048), nametable mirroring, the scroll registers, $2007 and the PPUSTATUS flags
//! (REQ-CORE-112, REQ-CORE-116).

mod common;

use common::{loaded_core, TileFixture};
use zax_nes::cpu::CpuBus;
use zax_nes::ppu::{SpriteLimit, PIXEL_COLOR, SCREEN_WIDTH};
use zax_nes::NesCore;

/// Pattern $FF: plane 0 all ones, plane 1 all zeros, so every pixel is colour 1.
const SOLID_TILE: u8 = 0xFF;

/// The NES colour index of REQ-CORE-048 at `(x, y)`: the stored byte with the opacity bit of
/// AMD-050 REQ-CORE-049 masked off (`PIXEL_COLOR`).
fn pixel_at(core: &NesCore, x: usize, y: usize) -> u8 {
    core.pixel_frame_with_mask()[(y * SCREEN_WIDTH) + x] & PIXEL_COLOR
}

#[test]
fn req_core_048_a_disabled_screen_is_filled_with_the_universal_backdrop() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x21]);
    fixture.set_mask(0x00);
    fixture.next_batch();

    assert_eq!(256 * 240, fixture.core.pixel_frame_with_mask().len());
    assert!(
        fixture
            .core
            .pixel_frame_with_mask()
            .iter()
            .all(|&byte| byte == 0x21),
        "rendering off: the backdrop, and the opacity bit clear"
    );
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
        fixture.core.run_frame(0);
        fixture.core.run_frame(0);
        fixture.core.ppu().sprite_zero_hit()
    }

    assert!(hit(0x1E, SOLID_TILE));
    assert!(!hit(0x16, SOLID_TILE)); // sprites off
    assert!(!hit(0x0E, SOLID_TILE)); // background off
    assert!(!hit(0x1E, 0x00)); // transparent background
}

/// `AccuracyCoin`'s `$2002 flag timing` (REQ-CORE-112, REQ-CORE-116): the sprite flags and the
/// vblank flag are all cleared on dot 1 of the pre-render line, but a `$2002` read latches the
/// vblank flag at its **start** and samples the sprite flags at its **end**, about 1.9 dots
/// later. So a read landing on dot 0 of that line comes back with bit 7 still set and bits 5 and
/// 6 already gone, and a read one dot earlier still has all three.
#[test]
fn req_core_112_a_2002_read_on_the_pre_render_dot_0_already_sees_the_sprite_flags_cleared() {
    /// Sets both sprite flags, then reads `$2002` on the cycle whose PPU position is
    /// `(line, dot)`. The read's own cycle advances the PPU three dots, so the alignment is
    /// found three dots ahead of the target; the achieved position is asserted, so a phase that
    /// cannot land on it fails loudly rather than measuring the wrong dot.
    fn status_read_at(line: i32, dot: i32) -> u8 {
        let mut fixture = TileFixture::new();
        // Sprite 0 over a solid background sets the hit; nine sprites on the line set overflow.
        fixture.fill_vram(0x2000, 960, SOLID_TILE);
        for n in 0..9u8 {
            fixture.set_sprite(n, 99, SOLID_TILE, 0x00, 8 * n);
        }
        fixture.set_sprite(0, 99, SOLID_TILE, 0x00, 100);
        fixture.set_scroll(0, 0);
        fixture.set_control(0x00);
        fixture.set_mask(0x1E);
        fixture.core.run_frame(0);

        // Three dots before the target, which one read cycle then covers.
        let (want_line, want_dot) = if dot >= 3 {
            (line, dot - 3)
        } else {
            (line - 1, dot + 341 - 3)
        };

        let bus = fixture.core.bus_mut();
        let mut guard = 0;
        loop {
            bus.sync_ppu();
            if bus.ppu().scanline() == want_line && bus.ppu().dot() == want_dot {
                break;
            }
            bus.tick();
            guard += 1;
            assert!(guard < 2_000_000, "never aligned on {want_line}/{want_dot}");
        }

        let status = bus.read(0x2002);
        assert_eq!(
            (line, dot),
            (bus.ppu().scanline(), bus.ppu().dot()),
            "the read did not land where the test meant it to"
        );
        status & 0xE0
    }

    let pre_render = TileFixture::new().core.ppu().pre_render_scanline();

    // Dot 0: the vblank bit is the one this dot still had, the sprite flags are sampled past the
    // dot-1 clear. The ROM's third reading, `$80`.
    assert_eq!(0x80, status_read_at(pre_render, 0));

    // The last dot of the line before: 1.9 dots short of the clear, so all three stand. The
    // ROM's second reading, `$E0`.
    assert_eq!(0xE0, status_read_at(pre_render - 1, 340));
}

/// Runs the machine until the PPU stands on `scanline`, so that a register access made straight
/// on the PPU afterwards happens at that position with no dot in between.
fn park_on_scanline(fixture: &mut TileFixture, scanline: i32) {
    let bus = fixture.core.bus_mut();
    let mut guard = 0;
    loop {
        bus.sync_ppu();
        if bus.ppu().scanline() == scanline {
            return;
        }
        bus.tick();
        guard += 1;
        assert!(guard < 2_000_000, "never reached scanline {scanline}");
    }
}

/// A `$2007` access made while the PPU is fetching does not add the 1 or 32 of PPUCTRL: it
/// triggers a coarse X increment and a Y increment **at once** (nesdev, "PPU scrolling", "$2007
/// (PPUDATA) reads and writes"; `AccuracyCoin`'s `$2007 read w/ rendering` 2, REQ-CORE-112).
///
/// **This test was inverted by lot MF-4d-fix and lot MF-4e flipped it back**, which is the
/// deferral Q-125 asked for closing where the rule returns. It asserts `(6, 8, 3)` on the visible
/// line again. What makes the rule compatible with AMD-021 R2-005 is not the dot the access lands
/// on — the page's condition has no dot in it — but that the borrowed increment no longer reaches
/// `v_row_start`: see [`zax_nes::ppu::Ppu2C02::increment_vram_address`] and Q-137, and
/// `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line` for the other half of the pair.
///
/// **Behind `ppu-2007-render-read` since lot MF-4e-gate** (Q-139), with the rule it pins. The
/// default build runs
/// `req_core_112_a_2007_read_while_fetching_still_steps_by_the_ppuctrl_amount_q_125` below in its
/// place, so one of the two states the shipped behaviour in every configuration and neither build
/// can change this path silently.
///
/// The access is made on the PPU directly so that no dot passes between the two readings of `v`
/// and the difference is the increment itself and nothing else.
#[cfg(feature = "ppu-2007-render-read")]
#[test]
fn req_core_112_a_2007_read_while_fetching_increments_coarse_x_and_y_at_once() {
    // v = $2105: coarse X 5, coarse Y 8, fine Y 2.
    fn v_after_a_2007_read(scanline: i32) -> (i32, i32, i32) {
        let mut fixture = TileFixture::new();
        fixture.set_control(0x00); // Step of 1.
        fixture.set_mask(0x1E); // Rendering enabled.
        park_on_scanline(&mut fixture, scanline);

        let ppu = fixture.core.bus_mut().ppu_mut();
        ppu.write(6, 0x21, None);
        ppu.write(6, 0x05, None);
        assert_eq!((5, 8, 2), (ppu.coarse_x(), ppu.coarse_y(), ppu.fine_y()));

        ppu.read(7, None);
        (ppu.coarse_x(), ppu.coarse_y(), ppu.fine_y())
    }

    // On a visible line, with rendering on: both increments, so coarse X moves **and** fine Y
    // does. Coarse Y stays at 8 because fine Y was 2 and only carries out of 7.
    assert_eq!((6, 8, 3), v_after_a_2007_read(100));

    // In vblank nothing is fetched, so the PPUCTRL step of 1 applies and fine Y stands.
    assert_eq!((6, 8, 2), v_after_a_2007_read(250));
}

/// **The default build's half of the pair: the rule is off, so a `$2007` access steps by PPUCTRL's
/// amount on every line** — `11407930`'s behaviour, and v0.4.2's (Q-139).
///
/// This is the test lot MF-4d-fix inverted and lot MF-4e flipped back, restored here for the
/// configuration that does not ship the rule. It is `#[cfg]`'d rather than `#[ignore]`d for
/// MF-4d-fix's own reason: an ignored test pins nothing and would let the rule come back by
/// accident, whereas exactly one of this test and
/// `req_core_112_a_2007_read_while_fetching_increments_coarse_x_and_y_at_once` runs in every
/// configuration, each stating the behaviour its build actually ships.
///
/// Why the rule is behind a feature at all is not that it is wrong — nesdev's "PPU scrolling" page
/// and `AccuracyCoin`'s `$2007 read w/ rendering` 2 both have it, and
/// [`zax_nes::ppu::Ppu2C02::increment_vram_address`] carries the whole derivation. It is that R0's
/// A/B of the 3D packs found it moving the picture of Super Mario Bros. 3 and Zelda II, which the
/// owner validates by eye before it is the default.
///
/// **Both row fields are asserted beside the increment**, which the inverted test of MF-4d-fix
/// could not do because the accessors did not exist yet: with the rule off, a `$2007` access is an
/// ordinary CPU address change on every line and so reaches `v_row_start` as well as `v_row_next`.
/// That is what puts Super Mario Bros. 3's banner row back on line 195 and lets
/// `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line` keep its `smallest_gap >= 3`.
#[cfg(not(feature = "ppu-2007-render-read"))]
#[test]
fn req_core_112_a_2007_read_while_fetching_still_steps_by_the_ppuctrl_amount_q_125() {
    // v = $2105: coarse X 5, coarse Y 8, fine Y 2.
    fn v_after_a_2007_read(scanline: i32) -> (i32, i32, i32) {
        let mut fixture = TileFixture::new();
        fixture.set_control(0x00); // Step of 1.
        fixture.set_mask(0x1E); // Rendering enabled.
        park_on_scanline(&mut fixture, scanline);

        let ppu = fixture.core.bus_mut().ppu_mut();
        ppu.write(6, 0x21, None);
        ppu.write(6, 0x05, None);
        assert_eq!((5, 8, 2), (ppu.coarse_x(), ppu.coarse_y(), ppu.fine_y()));

        ppu.read(7, None);
        (ppu.coarse_x(), ppu.coarse_y(), ppu.fine_y())
    }

    // The three fine Ys the captured rows are decided on, from a `v` whose fine Y is 0.
    fn fine_ys(scanline: i32) -> (i32, i32, i32) {
        let mut fixture = TileFixture::new();
        fixture.set_control(0x00);
        fixture.set_mask(0x1E);
        park_on_scanline(&mut fixture, scanline);

        let ppu = fixture.core.bus_mut().ppu_mut();
        ppu.write(6, 0x01, None); // v = $0105: coarse X 5, coarse Y 8, fine Y 0.
        ppu.write(6, 0x05, None);
        ppu.read(7, None);
        (
            ppu.fine_y(),
            ppu.captured_row_fine_y(),
            ppu.next_row_fine_y(),
        )
    }

    // On a visible line, with rendering on: the PPUCTRL step of 1 applies, so coarse X moves and
    // fine Y does **not**. The feature build gives (6, 8, 3) — Q-125, Q-137, Q-139.
    assert_eq!((6, 8, 2), v_after_a_2007_read(100));

    // In vblank, the same, and here it is the hardware's own answer: nothing is fetched.
    assert_eq!((6, 8, 2), v_after_a_2007_read(250));

    // And the captured rows: a CPU address change reaches both fields on every line, fetching or
    // not, so nothing moves off the fine Y the `$2006` write left. The feature build gives
    // (1, 0, 1) on the visible line — see
    // `amd_021_r2_005_a_2007_read_while_fetching_moves_the_next_lines_row_and_not_this_lines`.
    assert_eq!((0, 0, 0), fine_ys(100));
    assert_eq!((0, 0, 0), fine_ys(250));
}

/// The increment a `$2007` access borrows from the fetch pipeline reaches the row of the **next**
/// line and not the row of the line it is made in (AMD-021 R2-005 criterion 2, Q-137) — the
/// unit-scale statement of what `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`
/// measures over 5 000 frames of a commercial ROM.
///
/// Both halves of criterion 2's sentence are here. "N'émet aucune rangée de fond à la ligne où le
/// split écrit `$2006`" is the middle figure staying at the fine Y the `$2006` write left; "la
/// première rangée du bandeau est émise à la ligne suivante" is the third moving to the 1 that
/// emits, which is what makes the row appear on the **next** line rather than being lost. The same
/// access made outside rendering is a CPU address change and reaches both.
///
/// **Behind `ppu-2007-render-read` since lot MF-4e-gate** (Q-139): it pins the rule, so it runs
/// where the rule does. The default build's statement of the same three fields is the last block
/// of `req_core_112_a_2007_read_while_fetching_still_steps_by_the_ppuctrl_amount_q_125`.
#[cfg(feature = "ppu-2007-render-read")]
#[test]
fn amd_021_r2_005_a_2007_read_while_fetching_moves_the_next_lines_row_and_not_this_lines() {
    // fine Y 0 going in, so the pipeline's Y increment takes it to the 1 that emits a row.
    fn fine_ys(scanline: i32) -> (i32, i32, i32) {
        let mut fixture = TileFixture::new();
        fixture.set_control(0x00);
        fixture.set_mask(0x1E);
        park_on_scanline(&mut fixture, scanline);

        let ppu = fixture.core.bus_mut().ppu_mut();
        ppu.write(6, 0x01, None); // v = $0105: coarse X 5, coarse Y 8, fine Y 0.
        ppu.write(6, 0x05, None);
        ppu.read(7, None);
        (
            ppu.fine_y(),
            ppu.captured_row_fine_y(),
            ppu.next_row_fine_y(),
        )
    }

    // Fetching: `v` reaches fine Y 1 and so does the next line's row, while this line's row stays
    // at the 0 the `$2006` write left. Had this line's row followed `v`, Super Mario Bros. 3's
    // split line would emit a spurious row; had the next line's row not followed it, that row
    // would be lost instead of moving down one line.
    assert_eq!((1, 0, 1), fine_ys(100));

    // In vblank the PPUCTRL step applies and both rows follow `v`, all three still at fine Y 0.
    assert_eq!((0, 0, 0), fine_ys(250));
}

/// `AccuracyCoin`'s `Address $2004 behavior` 6, 7 and A (REQ-CORE-112): a `$2004` write made
/// while the PPU is fetching does not reach OAM and bumps the OAM address by four, realigned
/// with `& $FC`.
#[test]
fn req_core_112_a_2004_write_while_fetching_bumps_the_address_by_four_and_writes_nothing() {
    fn write_2004(scanline: i32, from: u8) -> (u8, u8) {
        let mut fixture = TileFixture::new();
        fixture.set_mask(0x1E); // Rendering enabled.
        park_on_scanline(&mut fixture, scanline);

        let ppu = fixture.core.bus_mut().ppu_mut();
        ppu.write(3, from, None); // OAMADDR
        let before = ppu.oam()[usize::from(from)];
        ppu.write(4, before.wrapping_add(1), None); // A byte that cannot already be there.
        (ppu.oam_address(), ppu.oam()[usize::from(from)])
    }

    // Fetching, from an aligned address: +4, and OAM untouched.
    let (address, byte) = write_2004(100, 0);
    assert_eq!(4, address);
    assert_eq!(0xFF, byte, "OAM was written while the PPU was fetching");

    // Fetching, from an unaligned one: (1 + 4) & $FC is 4, not 5 — the ROM's code A.
    assert_eq!(4, write_2004(100, 1).0);

    // In vblank the write is an ordinary one: OAM takes the byte and the address steps by 1.
    let mut fixture = TileFixture::new();
    fixture.set_mask(0x1E);
    park_on_scanline(&mut fixture, 250);
    let ppu = fixture.core.bus_mut().ppu_mut();
    ppu.write(3, 0x00, None);
    ppu.write(4, 0x5A, None);
    assert_eq!(1, ppu.oam_address());
    assert_eq!(0x5A, ppu.oam()[0]);
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
        fixture.core.run_frame(0);
        fixture.core.run_frame(0);
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

/// The buffer a `$2007` read fetches is only there for a read **two** CPU cycles later: a read
/// one cycle behind still returns the byte the buffer held before. That is the pair
/// `dmc_dma_during_read4/double_2007_read` makes with `lda $20F7,x`, X=$10 — the dummy read of
/// `$2007` and the read of `$2107` — and `AccuracyCoin`'s `$2007 Stress Test` states the rule:
/// "Reading from $2007 should set up the PPU Read Buffer two ppu cycles after the CPU Read
/// ends" (REQ-CORE-112).
#[test]
fn req_core_112_a_2007_read_one_cycle_after_another_still_sees_the_older_buffer() {
    /// One cycle on PRG, so that the next `$2007` read is far enough to see the last refill.
    fn slack(bus: &mut zax_nes::NesBus) {
        bus.read(0x8000);
    }

    let (_, mut bus) = loaded_core(None);

    // VRAM $2000.. = 11 22 33 44 55 66 77, written with the host accessor so that no cycle runs
    // and the read buffer is untouched, then the address back to $2001 as the ROM leaves it.
    bus.poke(0x2006, 0x20);
    bus.poke(0x2006, 0x00);
    for value in [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77] {
        bus.poke(0x2007, value);
    }
    bus.poke(0x2006, 0x20);
    bus.poke(0x2006, 0x01);
    bus.read(0x2007); // Fills the buffer with $22 and leaves v at $2002.
    slack(&mut bus);

    // The pair one cycle apart: both reads return $22, and the buffer is left holding the byte
    // the *second* one fetched ($44 from $2003), not the first one's ($33 from $2002).
    assert_eq!(0x22, bus.read(0x2007));
    assert_eq!(0x22, bus.read(0x2107));
    slack(&mut bus);
    assert_eq!(0x44, bus.read(0x2007));

    // Reads two cycles apart are the ordinary one-byte lag, with nothing lost.
    slack(&mut bus);
    assert_eq!(0x55, bus.read(0x2007));
    slack(&mut bus);
    assert_eq!(0x66, bus.read(0x2007));
    slack(&mut bus);
    assert_eq!(0x77, bus.read(0x2007));
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
    fixture.core.run_frame(0);

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

/// A frame with the sprites `place` sets (sprite palette colour 1 = $02 over a $0F backdrop, no
/// background), drawn with `limit`.
fn sprite_frame(limit: SpriteLimit, place: impl Fn(&mut TileFixture)) -> TileFixture {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F11, &[0x02]);
    for n in 0..64 {
        fixture.set_sprite(n, 0xF0, 0, 0, 0);
    }
    place(&mut fixture);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x14);
    fixture.core.set_sprite_limit(limit);
    fixture.core.run_frame(0);
    fixture.core.run_frame(0);
    fixture
}

#[test]
fn the_sprites_past_the_eighth_of_a_line_show_once_the_limit_is_removed() {
    let ten_in_a_row = |fixture: &mut TileFixture| {
        for n in 0..10 {
            fixture.set_sprite(n, 99, SOLID_TILE, 0x00, 16 * n);
        }
    };
    for (limit, ninth_shows) in [
        (SpriteLimit::Kept, false),
        (SpriteLimit::Removed, true),
        (SpriteLimit::Smart, true),
    ] {
        let fixture = sprite_frame(limit, ten_in_a_row);
        assert_eq!(
            0x02,
            pixel_at(&fixture.core, (16 * 7) + 3, 103),
            "{limit:?}"
        );
        let ninth = if ninth_shows { 0x02 } else { 0x0F };
        assert_eq!(
            ninth,
            pixel_at(&fixture.core, (16 * 8) + 3, 103),
            "{limit:?}"
        );
        assert_eq!(
            ninth,
            pixel_at(&fixture.core, (16 * 9) + 3, 103),
            "{limit:?}"
        );
        // The flag is the hardware's whatever the picture shows.
        assert!(fixture.core.ppu().sprite_overflow(), "{limit:?}");
    }
}

#[test]
fn smart_keeps_the_limit_on_the_lines_a_game_masks_with_eight_stacked_sprites() {
    // The Legend of Zelda at a door: eight blank sprites at one spot, in front of the OAM, hide
    // the sprites after them on their lines (here the lines 100 to 107); others of the same
    // y, but not on one spot, mask nothing.
    let mask = |fixture: &mut TileFixture| {
        for n in 0..8 {
            fixture.set_sprite(n, 99, 0x00, 0x20, 0);
        }
        fixture.set_sprite(8, 103, SOLID_TILE, 0x00, 100);
    };
    for (limit, masked) in [
        (SpriteLimit::Kept, true),
        (SpriteLimit::Removed, false),
        (SpriteLimit::Smart, true),
    ] {
        let fixture = sprite_frame(limit, mask);
        let hidden = if masked { 0x0F } else { 0x02 };
        assert_eq!(hidden, pixel_at(&fixture.core, 103, 106), "{limit:?}");
        // Below the mask the sprite shows in every mode.
        assert_eq!(0x02, pixel_at(&fixture.core, 103, 109), "{limit:?}");
    }

    let spread = |fixture: &mut TileFixture| {
        for n in 0..8 {
            fixture.set_sprite(n, 99, 0x00, 0x20, 8 * n);
        }
        fixture.set_sprite(8, 103, SOLID_TILE, 0x00, 100);
    };
    let fixture = sprite_frame(SpriteLimit::Smart, spread);
    assert_eq!(0x02, pixel_at(&fixture.core, 103, 106));
}

/// With PPUMASK's greyscale bit set, the palette RAM drives only the two colour bits of a grey
/// onto the PPU's data bus, so a `$2007` read of `$3F00-$3FFF` returns the entry masked with
/// `$30` (REQ-CORE-047, REQ-CORE-112). The byte **stored** in palette RAM is untouched.
///
/// `AccuracyCoin`'s `Palette RAM Quirks`, code 6: "With Greyscale Mode enabled, the lower four
/// bits of the value read should all be zero"; code 7: "with Greyscale Mode enabled, the lower
/// four bits of the value written should be unaffected".
#[test]
fn req_core_047_a_palette_read_under_greyscale_returns_the_value_anded_with_30() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x21, 0x0F, 0x3F]);

    // Greyscale off: the six stored bits, with the two upper bits of the PPU's I/O latch.
    fixture.set_mask(0x00);
    for (address, stored) in [(0x3F00u16, 0x21u8), (0x3F01, 0x0F), (0x3F02, 0x3F)] {
        let [high, low] = address.to_be_bytes();
        fixture.core.write_memory(0x2006, high);
        fixture.core.write_memory(0x2006, low);
        assert_eq!(
            stored,
            fixture.core.ppu_read(7) & 0x3F,
            "greyscale off, ${address:04X}"
        );
    }

    // Greyscale on: the same entries, ANDed with $30 — and the write of $2001 is the last byte
    // on the latch, so the upper two bits are its own.
    fixture.set_mask(0x01);
    for (address, masked) in [(0x3F00u16, 0x20u8), (0x3F01, 0x00), (0x3F02, 0x30)] {
        let [high, low] = address.to_be_bytes();
        fixture.core.write_memory(0x2006, high);
        fixture.core.write_memory(0x2006, low);
        assert_eq!(
            masked,
            fixture.core.ppu_read(7) & 0x3F,
            "greyscale on, ${address:04X}"
        );
    }

    // Code 7: nothing was stored masked. Turning greyscale back off gives all three entries
    // back, so the mask only ever touched what the palette drove onto the bus.
    fixture.set_mask(0x00);
    for (address, stored) in [(0x3F00u16, 0x21u8), (0x3F01, 0x0F), (0x3F02, 0x3F)] {
        let [high, low] = address.to_be_bytes();
        fixture.core.write_memory(0x2006, high);
        fixture.core.write_memory(0x2006, low);
        assert_eq!(
            stored,
            fixture.core.ppu_read(7) & 0x3F,
            "the stored byte of ${address:04X} is unaffected"
        );
    }
}

/// The 2C02G/H's OAMADDR defect (lot FIDELITY step 19): "if OAMADDR is not less than eight when
/// rendering starts, the eight bytes starting at OAMADDR & 0xF8 are copied to the first eight
/// bytes of OAM" (nesdev, "PPU registers"). Not on a PAL 2C07, and not with the setting off.
#[test]
fn fidelity_oamaddr_at_8_or_more_copies_its_row_over_oam_0_when_rendering_starts() {
    fn oam_head_after_start(oamaddr: u8, region: Option<zax_nes::NesRegion>, bug: bool) -> Vec<u8> {
        let mut fixture = TileFixture::new();
        for sprite in 0..8u8 {
            fixture.set_sprite(sprite, 0xF0, sprite, 0, sprite * 16);
        }
        if let Some(region) = region {
            let timing = zax_nes::RegionTiming::for_region(region);
            fixture.core.bus_mut().ppu_mut().set_timing(timing);
        }
        fixture.core.set_oamaddr_bug(bug);
        fixture.set_mask(0x18);
        park_on_scanline(&mut fixture, 250);
        fixture.core.bus_mut().ppu_mut().write(3, oamaddr, None);
        park_on_scanline(&mut fixture, 0);
        fixture.core.bus().ppu().oam()[0..8].to_vec()
    }

    let untouched = vec![0xF0, 0, 0, 0, 0xF0, 1, 0, 16];
    let row_2 = vec![0xF0, 4, 0, 64, 0xF0, 5, 0, 80];
    assert_eq!(
        untouched,
        oam_head_after_start(0x07, None, true),
        "below 8: nothing"
    );
    assert_eq!(
        row_2,
        oam_head_after_start(0x13, None, true),
        "OAMADDR & $F8 = $10"
    );
    assert_eq!(
        untouched,
        oam_head_after_start(0x13, None, false),
        "setting off"
    );
    assert_eq!(
        untouched,
        oam_head_after_start(0x13, Some(zax_nes::NesRegion::Pal), true),
        "the 2C07 does not have it"
    );
}

/// Lot FIDELITY step 20: with the rendering off for more than 8 NTSC frames, an OAM row nothing
/// accessed decays (each byte reads as its own address, as `BeesNES` leaves it); a row the CPU
/// touched in the meantime, or the rendering on all along, keeps its bytes.
#[test]
fn fidelity_oam_decays_after_8_frames_with_the_rendering_off() {
    fn oam_after_blank(frames: usize, decay: bool, touch_row_1: bool) -> Vec<u8> {
        let mut fixture = TileFixture::new();
        for sprite in 0..4u8 {
            fixture.set_sprite(sprite, 0x40, 0x11, 0x01, 0x22);
        }
        fixture.core.set_oam_decay(decay);
        fixture.set_mask(0x18);
        park_on_scanline(&mut fixture, 10);
        fixture.set_mask(0x00);
        for frame in 0..frames {
            park_on_scanline(&mut fixture, 100);
            park_on_scanline(&mut fixture, 0);
            if touch_row_1 && frame % 4 == 0 {
                let ppu = fixture.core.bus_mut().ppu_mut();
                ppu.write(3, 0x08, None);
                ppu.read(4, None);
            }
        }
        fixture.set_mask(0x18);
        fixture.core.bus().ppu().oam()[0..16].to_vec()
    }

    let kept = vec![
        0x40, 0x11, 0x01, 0x22, 0x40, 0x11, 0x01, 0x22, 0x40, 0x11, 0x01, 0x22, 0x40, 0x11, 0x01,
        0x22,
    ];
    let row_0_decayed = [0, 1, 2, 3, 4, 5, 0x02, 7];
    assert_eq!(kept, oam_after_blank(12, false, false), "setting off");
    assert_eq!(kept, oam_after_blank(6, true, false), "6 frames: kept");
    let both = oam_after_blank(12, true, false);
    assert_eq!(row_0_decayed, both[0..8], "12 frames: row 0 decayed");
    assert_eq!(
        [8, 9, 0x02, 11, 12, 13, 0x02, 15],
        both[8..16],
        "12 frames: row 1 decayed"
    );
    let touched = oam_after_blank(12, true, true);
    assert_eq!(row_0_decayed, touched[0..8], "row 0 untouched");
    assert_eq!(kept[8..16], touched[8..16], "row 1 read every 4 frames");
}

/// Plan step 2: with `set_power_on_top`, a power-on puts the PPU at the top of the picture (nesdev,
/// "PPU power up state"; dot 11 as `BeesNES`), and without it at the start of vblank as before.
#[test]
fn fidelity_power_on_top_starts_the_ppu_at_line_0_dot_11() {
    let mut fixture = TileFixture::new();
    fixture.core.reset();
    let ppu = fixture.core.bus().ppu();
    assert_ne!(0, ppu.scanline(), "default: vblank");

    fixture.core.set_power_on_top(true);
    fixture.core.reset();
    let ppu = fixture.core.bus().ppu();
    // Dot 11 when the CPU's reset starts; its seven cycles (21 NTSC dots) have run since.
    assert_eq!((0, 11 + 21), (ppu.scanline(), ppu.dot()));
}
