//! The tile contract of SPEC-02 §3.5 through its test vectors V1 to V10 (§5.3), the batch
//! plumbing and AMD-021 R2-005.

// Test indices and bytes are small: the narrowing casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

mod common;

use common::{capture_batch, machine_with, minimal_rom, pattern, BatchCopy, TileFixture};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use zax_nes::VideoInfo;

// ------------------------------------------------------------------ V1

#[test]
fn req_core_037_v1_a_bare_background_gives_960_records_in_30_rows() {
    let mut fixture = TileFixture::new();
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);
    let batch = fixture.next_batch();

    assert_eq!(960, batch.records.len());
    assert!(batch.records.iter().all(|record| record.background));

    for row in 0..30 {
        for k in 0..32 {
            let record = batch.records[(row * 32) + k];
            assert_eq!(8 * k as i16, record.x);
            assert_eq!(8 * row as i16, record.y);
            assert_eq!((33 * row + k) as i32, record.index);
            assert_eq!(0, record.palette);
            assert_eq!(0, record.plane0);
            assert_eq!(0, record.plane1);
            assert!(!record.mirror_h);
            assert!(!record.mirror_v);
            assert!(!record.hidden);
        }
    }

    // The worked example of REQ-CORE-040: row 1, k = 4 is x = 32, y = 8, index 37.
    let sample = batch.records[32 + 4];
    assert_eq!(32, sample.x);
    assert_eq!(8, sample.y);
    assert_eq!(37, sample.index);

    // maskX is 8 because bit 2 of PPUMASK is clear; maskY is 8 because the mirroring is
    // vertical (REQ-CORE-046).
    assert_eq!(8, batch.mask_x);
    assert_eq!(8, batch.mask_y);
}

#[test]
fn req_core_036_v1_empty_patterns_are_still_emitted() {
    let mut fixture = TileFixture::new();
    fixture.set_scroll(0, 0);
    fixture.set_mask(0x0A);
    let batch = fixture.next_batch();

    assert_eq!(960, batch.records.len());
    assert!(batch.records.iter().all(zax_nes::TileRecord::is_empty));
}

// ------------------------------------------------------------------ V2

#[test]
fn req_core_042_v2_an_8x8_sprite_carries_its_attributes_and_planes() {
    let mut fixture = TileFixture::new();
    fixture.set_sprite(5, 0x50, 0x21, 0x63, 0x80);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x08);
    fixture.set_mask(0x1E);
    let sprites = fixture.next_batch().sprites();

    assert_eq!(1, sprites.len());
    let sprite = sprites[0];
    assert!(!sprite.background);
    assert_eq!(128, sprite.x);
    assert_eq!(81, sprite.y);
    assert_eq!(5, sprite.index);
    assert_eq!(3, sprite.palette);
    assert!(sprite.hidden);
    assert!(sprite.mirror_h);
    assert!(!sprite.mirror_v);

    let (plane0, plane1) = pattern(1, 0x21);
    assert_eq!(0x2121_2121_2121_2121, plane0);
    assert_eq!(0xDEDE_DEDE_DEDE_DEDE, plane1);
    assert_eq!(plane0, sprite.plane0);
    assert_eq!(plane1, sprite.plane1);
}

#[test]
fn req_core_044_v2_the_sprite_sits_between_the_rows_of_lines_81_and_89() {
    let mut fixture = TileFixture::new();
    fixture.set_sprite(5, 0x50, 0x21, 0x63, 0x80);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x08);
    fixture.set_mask(0x1E);
    let batch = fixture.next_batch();

    let sprite_at = batch
        .records
        .iter()
        .position(|record| !record.background)
        .unwrap();
    assert!(sprite_at > 0);

    // The sprite is captured on line 87, so the row before it is the one of line 81
    // (y = 80) and the row after it is the one of line 89 (y = 88).
    assert_eq!(80, batch.records[sprite_at - 1].y);
    assert!(batch.records[sprite_at - 1].background);
    assert_eq!(88, batch.records[sprite_at + 1].y);
    assert!(batch.records[sprite_at + 1].background);
}

// ------------------------------------------------------------------ V3

#[test]
fn req_core_042_v3_an_8x16_sprite_emits_two_halves_swapped_by_the_vertical_flip() {
    let mut fixture = TileFixture::new();
    fixture.set_sprite(2, 0x20, 0x15, 0x80, 0x10);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x20);
    fixture.set_mask(0x1E);
    let sprites = fixture.next_batch().sprites();

    assert_eq!(2, sprites.len());

    assert_eq!(2, sprites[0].index);
    assert_eq!(16, sprites[0].x);
    assert_eq!(33, sprites[0].y);
    assert!(sprites[0].mirror_v);
    assert_eq!(pattern(1, 0x15), (sprites[0].plane0, sprites[0].plane1));

    assert_eq!(66, sprites[1].index);
    assert_eq!(16, sprites[1].x);
    assert_eq!(41, sprites[1].y);
    assert!(sprites[1].mirror_v);
    assert_eq!(pattern(1, 0x14), (sprites[1].plane0, sprites[1].plane1));
}

#[test]
fn req_core_042_v3_without_the_vertical_flip_the_halves_keep_their_order() {
    let mut fixture = TileFixture::new();
    fixture.set_sprite(2, 0x20, 0x15, 0x00, 0x10);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x20);
    fixture.set_mask(0x1E);
    let sprites = fixture.next_batch().sprites();

    assert_eq!(2, sprites.len());
    assert_eq!(2, sprites[0].index);
    assert_eq!(pattern(1, 0x14).0, sprites[0].plane0);
    assert_eq!(66, sprites[1].index);
    assert_eq!(pattern(1, 0x15).0, sprites[1].plane0);
}

#[test]
fn req_core_042_v3_the_lower_half_is_dropped_when_it_would_fall_off_the_screen() {
    let mut fixture = TileFixture::new();
    fixture.set_sprite(2, 0xE7, 0x15, 0x80, 0x10);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x20);
    fixture.set_mask(0x1E);
    let sprites = fixture.next_batch().sprites();

    assert_eq!(1, sprites.len());
    assert_eq!(2, sprites[0].index);
    assert_eq!(232, sprites[0].y);
}

#[test]
fn req_core_041_sprites_at_or_below_line_240_are_never_emitted() {
    let mut fixture = TileFixture::new();
    fixture.set_sprite(0, 0xEF, 0x01, 0x00, 0x10); // Ytop = 240.
    fixture.set_sprite(1, 0xFF, 0x01, 0x00, 0x20); // Ytop = 256.
    fixture.set_sprite(2, 0x00, 0x01, 0x00, 0x30); // Ytop = 1: emitted, y = 1.
    fixture.set_scroll(0, 0);
    fixture.set_mask(0x1E);
    let sprites = fixture.next_batch().sprites();

    assert_eq!(1, sprites.len());
    assert_eq!(2, sprites[0].index);
    assert_eq!(1, sprites[0].y);
}

// ------------------------------------------------------------------ V4

#[test]
fn req_core_038_v4_a_fine_x_scroll_adds_a_33rd_record_from_the_neighbouring_nametable() {
    let mut fixture = TileFixture::new();

    // Nametable $2000: column c holds tile c. Nametable $2400: column c holds tile 32+c.
    let left: Vec<u8> = (0..32).collect();
    let right: Vec<u8> = (32..64).collect();
    for row in 0..30u16 {
        fixture.write_vram(0x2000 + (row * 32), &left);
        fixture.write_vram(0x2400 + (row * 32), &right);
    }

    fixture.set_scroll(13, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);
    let batch = fixture.next_batch();

    assert_eq!(30 * 33, batch.records.len());
    for k in 0..=32 {
        let record = batch.records[k];
        assert_eq!((8 * k as i16) - 5, record.x);
        assert_eq!(0, record.y);
    }

    assert_eq!(pattern(0, 1).0, batch.records[0].plane0);
    assert_eq!(pattern(0, 31).0, batch.records[30].plane0);
    assert_eq!(pattern(0, 32).0, batch.records[31].plane0);
    assert_eq!(pattern(0, 33).0, batch.records[32].plane0);
}

// ------------------------------------------------------------------ V5

#[test]
fn req_core_037_v5_a_fine_y_scroll_of_3_captures_rows_on_lines_8j_plus_6() {
    let mut fixture = TileFixture::new();
    for row in 0..30u8 {
        fixture.fill_vram(0x2000 + (u16::from(row) * 32), 32, row);
    }

    fixture.set_scroll(0, 3);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);
    let batch = fixture.next_batch();

    assert_eq!(30 * 32, batch.records.len());
    for j in 0..30 {
        assert_eq!((8 * j as i16) + 5, batch.records[j * 32].y);
    }

    // The first captured row is nametable row 1: row 0 would sit at y = -3 and is not
    // emitted. Its line index is floor(6 / 8) = 0.
    assert_eq!(pattern(0, 1).0, batch.records[0].plane0);
    assert_eq!(0, batch.records[0].index);

    // The last one (line 238, y = 237) comes from row 0 of the vertically neighbouring
    // nametable $2800, which mirrors $2000 under vertical mirroring: tile 0.
    let last = batch.records[29 * 32];
    assert_eq!(237, last.y);
    assert_eq!(0, last.plane0);
    assert_eq!(0, last.plane1);
}

// ------------------------------------------------------------------ V6

#[test]
fn req_core_037_v6_vertical_mirroring_drops_the_row_of_line_0() {
    let mut fixture = TileFixture::new();
    fixture.set_scroll(0, 1);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);
    let batch = fixture.next_batch();

    assert_eq!(7, batch.records[0].y);
    assert!(!batch.records.iter().any(|record| record.y == -1));
}

#[test]
fn req_core_037_v6_horizontal_mirroring_keeps_the_row_of_line_0() {
    let mut fixture = TileFixture::with(0x00, None);
    fixture.set_scroll(0, 1);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);
    let batch = fixture.next_batch();

    for k in 0..32 {
        assert_eq!(-1, batch.records[k].y);
        assert_eq!(k as i32, batch.records[k].index);
    }
}

// ------------------------------------------------------------------ V7

#[test]
fn req_core_045_v7_the_batch_palette_is_the_32_bytes_of_palette_ram() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F01, &[0x30]);
    fixture.write_vram(0x3F11, &[0x16]);
    fixture.set_scroll(0, 0);
    fixture.set_mask(0x0A);
    let batch = fixture.next_batch();

    assert_eq!(32, batch.palette.len());
    assert_eq!(0x0F, batch.palette[0]);
    assert_eq!(0x30, batch.palette[1]);
    assert_eq!(0x16, batch.palette[17]);

    fixture.write_vram(0x3F10, &[0x21]);
    let batch = fixture.next_batch();
    assert_eq!(0x21, batch.palette[0]);
    assert_eq!(0x21, batch.palette[16]);
}

// ------------------------------------------------------------------ V8

#[test]
fn req_core_044_v8_records_follow_the_chronological_order_of_the_capture_lines() {
    let mut fixture = TileFixture::new();
    fixture.set_sprite(0, 0x00, 0x01, 0x00, 0x08);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x1E);
    let batch = fixture.next_batch();

    // 32 background records of line 1 (y = 0), then the sprite captured on line 7, then
    // the 32 background records of line 9 (y = 8).
    for k in 0..32 {
        assert!(batch.records[k].background);
        assert_eq!(0, batch.records[k].y);
    }

    let sprite = batch.records[32];
    assert!(!sprite.background);
    assert_eq!(0, sprite.index);
    assert_eq!(1, sprite.y);
    assert_eq!(8, sprite.x);

    for k in 0..32 {
        assert!(batch.records[33 + k].background);
        assert_eq!(8, batch.records[33 + k].y);
    }
}

// ------------------------------------------------------------------ V9

#[test]
fn req_core_046_v9_the_margins_follow_the_mirroring_and_the_left_column_mask() {
    for (header_byte6, mask, expected_mask_x, expected_mask_y) in [
        (0x00, 0x1E, 8, 0), // horizontal
        (0x01, 0x1E, 0, 8), // vertical
        (0x01, 0x18, 8, 8), // vertical, left column masked
        (0x08, 0x1E, 0, 0), // four-screen
        (0x08, 0x1A, 8, 0), // four-screen, sprites masked on the left
    ] {
        let mut fixture = TileFixture::with(header_byte6, None);
        fixture.set_scroll(0, 0);
        fixture.set_control(0x00);
        fixture.set_mask(mask);
        let batch = fixture.next_batch();

        assert_eq!(
            expected_mask_x, batch.mask_x,
            "header {header_byte6:02X} mask {mask:02X}"
        );
        assert_eq!(
            expected_mask_y, batch.mask_y,
            "header {header_byte6:02X} mask {mask:02X}"
        );
    }
}

// ------------------------------------------------------------------ V10

#[test]
fn req_core_047_v10_greyscale_and_colour_emphasis_change_nothing_in_the_batch() {
    fn run(mask: u8) -> BatchCopy {
        let mut fixture = TileFixture::new();
        fixture.set_sprite(5, 0x50, 0x21, 0x63, 0x80);
        fixture.set_scroll(0, 0);
        fixture.set_control(0x08);
        fixture.set_mask(mask);
        fixture.next_batch()
    }

    assert_eq!(run(0x1E), run(0xFF));
}

// ------------------------------------------------------------------ batch plumbing

#[test]
fn req_core_021_the_callback_fires_once_per_frame_even_with_an_empty_batch() {
    // There is no callback at this layer: the batch of every frame is read after it, and
    // it is there, empty, even with rendering off.
    let mut fixture = TileFixture::new();
    fixture.set_mask(0x00);
    fixture.core.run_frame(0);

    for _ in 0..5 {
        let batch = capture_batch(&mut fixture.core);
        assert_eq!(0, batch.records.len());
    }
}

#[test]
fn req_core_031_the_video_info_is_33_by_31_with_128_sprite_slots() {
    let info = VideoInfo::STANDARD;
    assert_eq!(33, info.width);
    assert_eq!(31, info.height);
    assert_eq!(128, info.sprite_count);
}

#[test]
fn req_core_032_the_batch_is_capped_at_1023_background_and_128_sprite_records() {
    // A program that rewrites $2006 in a tight loop with fine Y = 1, so that every one of
    // the 240 visible lines wants to emit a row: 7 680 background records, of which only
    // 1 023 fit. Sprites have their own budget and must survive.
    let program = [
        0xA9, 0x18, // LDA #$18        background and sprites on
        0x8D, 0x01, 0x20, // STA $2001
        0xA9, 0x10, // LDA #$10        fine Y = 1, coarse = 0
        0x8D, 0x06, 0x20, // STA $2006
        0xA9, 0x00, // LDA #$00
        0x8D, 0x06, 0x20, // STA $2006
        0x4C, 0x05, 0x80, // JMP $8005
    ];
    let mut core = machine_with(&minimal_rom(0x01, Some(&program), None), "Overflow.nes");

    // OAM is cleared at power-on, so all 64 sprites sit at Ytop = 1 and are captured
    // together on line 7, well before the background budget runs out.
    let mut batch = capture_batch(&mut core);
    for _ in 0..4 {
        batch = capture_batch(&mut core);
    }

    assert_eq!(1023, batch.background().len());
    assert_eq!(64, batch.sprites().len());
    assert_eq!(1023 + 64, batch.records.len());
    assert!(core.ppu().dropped_background_records() > 0);
}

#[test]
fn req_core_033_registering_the_video_callback_changes_nothing_in_the_emulation() {
    // Reading the batch after every frame (what a registered callback does) must not change
    // the emulation.
    let program = [
        0xA9, 0x1E, // LDA #$1E
        0x8D, 0x01, 0x20, // STA $2001
        0xE6, 0x20, // INC $20
        0xA5, 0x20, // LDA $20
        0x8D, 0x00, 0x03, // STA $0300
        0x4C, 0x05, 0x80, // JMP $8005
    ];
    let rom = minimal_rom(0x01, Some(&program), None);
    let mut silent = machine_with(&rom, "T.nes");
    let mut watched = machine_with(&rom, "T.nes");

    let mut copies = 0;
    for _ in 0..120 {
        silent.run_frame(0);
        copies += capture_batch(&mut watched).records.len();
    }
    assert!(copies > 0);

    assert_eq!(silent.cycle_count(), watched.cycle_count());
    for address in 0..0x800u16 {
        assert_eq!(silent.read_memory(address), watched.read_memory(address));
    }
    assert_eq!(silent.tile_records(), watched.tile_records());
}

// ------------------------------------------------------------------ AMD-021 R2-005

fn write_address_on_line(fixture: &mut TileFixture, line: i32, address: u16) -> BatchCopy {
    let core = &mut fixture.core;
    core.run_frame(0);

    let mut guard = 0;
    while (core.ppu().scanline() != line || core.ppu().dot() < 120) && guard < 400_000 {
        let (cpu, bus) = core.cpu_and_bus_mut();
        cpu.step(bus);
        guard += 1;
    }
    assert_eq!(line, core.ppu().scanline());

    core.write_memory(0x2006, (address >> 8) as u8);
    core.write_memory(0x2006, address as u8);
    capture_batch(core)
}

#[test] // AMD-021 R2-005, criterion 1, first sentence
fn amd_021_r2_005_a_2006_write_during_line_100_emits_the_row_of_line_100() {
    let mut fixture = TileFixture::with(0x00, None);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);

    // Fine Y 1, coarse Y 12, coarse X 0, nametable 0: v = $1180.
    let batch = write_address_on_line(&mut fixture, 100, 0x1180);

    // The row of line 100 carries y = line - 1 = 99 (REQ-CORE-038). Before R2-005 the write
    // only reached the capture of line 101, which would have emitted y = 100.
    assert!(batch.background().iter().any(|record| record.y == 99));
    assert!(!batch.background().iter().any(|record| record.y == 100));
}

#[test] // AMD-021 R2-005, criterion 1, second sentence
fn amd_021_r2_005_writing_fine_y_2_during_a_fine_y_1_line_drops_its_row() {
    let mut fixture = TileFixture::with(0x00, None);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);

    // Line 97 has fine Y 1 with no scroll, so it would normally emit the row y = 96.
    let reference = write_address_on_line(&mut fixture, 97, 0x1180);
    assert!(reference.background().iter().any(|record| record.y == 96));

    // Writing fine Y 2 during that same line removes the row entirely: the condition
    // "fine Y = 1" is evaluated on the state at the end of the line.
    let mut again = TileFixture::with(0x00, None);
    again.set_scroll(0, 0);
    again.set_control(0x00);
    again.set_mask(0x0A);
    let batch = write_address_on_line(&mut again, 97, 0x2180);
    assert!(!batch.background().iter().any(|record| record.y == 96));
}

#[test] // AMD-021 R2-005 (the two end-of-line reloads stay out of the row of the line)
fn amd_021_r2_005_the_end_of_line_reloads_only_reach_the_next_line() {
    // With no mid-line write at all, the rows are exactly the ones of V1: the vertical
    // increment of dot 256 and the horizontal reload of dot 257 of line L must land on line
    // L + 1, not on the row captured for line L.
    let mut fixture = TileFixture::with(0x00, None);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);
    let background = fixture.next_batch().background();

    assert_eq!(960, background.len());
    for row in 0..30 {
        assert_eq!(8 * row as i16, background[row * 32].y);
        assert_eq!(0, background[row * 32].x);
    }
}

const SEED: u64 = 1_469_598_103_934_665_603;

fn mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(1_099_511_628_211)
}

#[test]
fn req_core_092_the_frame_batches_survive_a_savestate_round_trip() {
    // A program that rewrites OAM through DMA with changing data, so that every frame
    // produces a different batch.
    let program = [
        0xA9, 0x1E, // LDA #$1E
        0x8D, 0x01, 0x20, // STA $2001
        0xEE, 0x00, 0x02, // INC $0200       loop
        0xEE, 0x03, 0x02, // INC $0203
        0xA9, 0x02, // LDA #$02
        0x8D, 0x14, 0x40, // STA $4014       OAM DMA from page 2
        0x4C, 0x05, 0x80, // JMP $8005
    ];

    let run = |save_restore_at_frame: Option<u16>| {
        let mut core = machine_with(&minimal_rom(0x01, Some(&program), None), "Batches.nes");

        let hash = Arc::new(AtomicU64::new(SEED));
        let sink = Arc::clone(&hash);
        core.set_video_callback(Some(Box::new(move |batch| {
            let mut h = sink.load(Ordering::Relaxed);
            for record in batch.records {
                h = mix(h, record.plane0);
                h = mix(h, record.plane1);
                h = mix(h, u64::from(record.x.cast_unsigned()));
                h = mix(h, u64::from(record.y.cast_unsigned()));
                h = mix(h, u64::from(record.index.cast_unsigned()));
                h = mix(h, u64::from(record.palette));
            }
            for &entry in &batch.palette {
                h = mix(h, u64::from(entry));
            }
            h = mix(h, u64::from(batch.mask_x.cast_unsigned()));
            h = mix(h, u64::from(batch.mask_y.cast_unsigned()));
            sink.store(h, Ordering::Relaxed);
        })));

        for frame in 0..60u16 {
            core.run_frame(frame);

            if save_restore_at_frame == Some(frame) {
                let state = core.save_state(true).unwrap();
                core.try_load_state(&state, 0).unwrap();
            }
        }

        hash.load(Ordering::Relaxed)
    };

    let straight = run(None);
    assert_ne!(SEED, straight);
    assert_eq!(straight, run(Some(30)));
}
