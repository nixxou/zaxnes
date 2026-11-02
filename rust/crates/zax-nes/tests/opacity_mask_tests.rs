//! AMD-050 REQ-CORE-049: the opacity mask, vector V11 of SPEC-02 §5.3, and REQ-CORE-048 as the
//! amendment restates it.
//!
//! The mask rides in bit 6 of each byte of the picture (`OPAQUE_PIXEL`), beside the NES colour
//! index of REQ-CORE-048 in bits 0-5 (`PIXEL_COLOR`). That is the internal layout the
//! requirement's note allows, and the one that costs the PPU nothing: the bit is already in the
//! palette entry the pixel resolves to.

mod common;

use common::TileFixture;
use zax_nes::ppu::{OPAQUE_PIXEL, PIXEL_COLOR, SCREEN_WIDTH};
use zax_nes::NesCore;

/// Pattern $FF: plane 0 all ones, plane 1 all zeros, so every pixel is colour 1.
const SOLID_TILE: u8 = 0xFF;

/// The NES colour index at `(x, y)`, the opacity bit masked off.
fn pixel_at(core: &NesCore, x: usize, y: usize) -> u8 {
    core.pixel_frame_with_mask()[(y * SCREEN_WIDTH) + x] & PIXEL_COLOR
}

/// Whether the pixel at `(x, y)` carries at least one opaque pixel.
fn opaque_at(core: &NesCore, x: usize, y: usize) -> bool {
    core.pixel_frame_with_mask()[(y * SCREEN_WIDTH) + x] & OPAQUE_PIXEL != 0
}

/// A fixture whose nametable is filled with `tile` and whose palette entries 0 and 1 are
/// `backdrop` and `colour`.
fn background(tile: u8, backdrop: u8, colour: u8, mask: u8) -> TileFixture {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[backdrop]);
    fixture.write_vram(0x3F01, &[colour]);
    fixture.fill_vram(0x2000, 960, tile);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(mask);
    fixture.next_batch();
    fixture
}

/// V11, first case: a nametable filled with a non-empty tile, the background rendered with its
/// left column shown (PPUMASK `$0A`), gives a mask that is **entirely** 1.
#[test]
fn req_core_049_v11_a_full_background_is_opaque_everywhere() {
    let fixture = background(SOLID_TILE, 0x0F, 0x30, 0x0A);

    assert!(
        fixture
            .core
            .pixel_frame_with_mask()
            .iter()
            .all(|byte| byte & OPAQUE_PIXEL != 0),
        "every one of the 61 440 pixels carries an opaque pixel"
    );
    assert_eq!(0x30, pixel_at(&fixture.core, 100, 100));
}

/// V11, second case — the one the image alone cannot express: with the palette colours equal to
/// the universal backdrop's, the **image** is uniformly `$0F` and the **mask** is still
/// entirely 1.
#[test]
fn req_core_049_v11_an_opaque_pixel_of_the_backdrop_colour_is_still_opaque() {
    let fixture = background(SOLID_TILE, 0x0F, 0x0F, 0x0A);

    let picture = fixture.core.pixel_frame_with_mask();
    assert!(
        picture.iter().all(|byte| byte & PIXEL_COLOR == 0x0F),
        "the image is uniformly the universal backdrop's colour"
    );
    assert!(
        picture.iter().all(|byte| byte & OPAQUE_PIXEL != 0),
        "and the mask is still 1: these pixels are opaque, they only look like the backdrop"
    );
}

/// V11, third case: PPUMASK `$08` masks the eight leftmost background pixels, so each row of the
/// mask starts with eight zeros and carries 248 ones.
#[test]
fn req_core_049_v11_the_left_column_mask_clears_eight_bits_a_row() {
    let fixture = background(SOLID_TILE, 0x0F, 0x30, 0x08);

    for y in 0..240 {
        for x in 0..8 {
            assert!(!opaque_at(&fixture.core, x, y), "({x}, {y}) is masked");
        }
        for x in [8, 9, 128, 254, 255] {
            assert!(opaque_at(&fixture.core, x, y), "({x}, {y}) is drawn");
        }
    }
}

/// V11, fourth case: PPUMASK `$00` — both layers off — gives a mask entirely 0 and an image
/// uniformly the universal backdrop's colour. REQ-SCN-076's note: the 2D rectangle is then
/// wholly transparent, which is what the original program shows too (it has no geometry either).
#[test]
fn req_core_049_v11_a_blanked_screen_has_an_empty_mask() {
    let fixture = background(SOLID_TILE, 0x21, 0x30, 0x00);

    let picture = fixture.core.pixel_frame_with_mask();
    assert!(picture.iter().all(|byte| byte & OPAQUE_PIXEL == 0));
    assert!(picture.iter().all(|byte| byte & PIXEL_COLOR == 0x21));
}

/// V11, fifth case: with an empty background and one sprite, the mask is 1 exactly on the
/// sprite's 8 × 8 rectangle and 0 everywhere else.
#[test]
fn req_core_049_v11_a_lone_sprite_is_the_only_opaque_rectangle() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F11, &[0x30]);
    // The nametable keeps tile 0, whose pattern is empty, so no background pixel is opaque.
    fixture.fill_vram(0x2000, 960, 0x00);
    fixture.set_sprite(0, 0x20, SOLID_TILE, 0x00, 0x10);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x1E);
    fixture.next_batch();

    // A sprite whose OAM y is `$20` is drawn on the lines 33 to 40 (nesdev), at x 16 to 23.
    for y in 0..240 {
        for x in 0..256 {
            let inside = (16..=23).contains(&x) && (33..=40).contains(&y);
            assert_eq!(
                inside,
                opaque_at(&fixture.core, x, y),
                "({x}, {y}) should be {}",
                if inside { "opaque" } else { "transparent" }
            );
        }
    }
}

/// REQ-CORE-049: a sprite pixel counts "quel que soit son drapeau « caché »" — a sprite the
/// engine marks hidden (REQ-CORE-042) still makes its pixels opaque, because the PPU draws it.
#[test]
fn req_core_049_a_sprite_behind_the_background_still_marks_its_pixels() {
    let mut fixture = TileFixture::new();
    fixture.write_vram(0x3F00, &[0x0F]);
    fixture.write_vram(0x3F11, &[0x30]);
    fixture.fill_vram(0x2000, 960, 0x00);
    // Attribute $20: the sprite is behind the background, which is transparent here.
    fixture.set_sprite(0, 0x20, SOLID_TILE, 0x20, 0x10);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x1E);
    fixture.next_batch();

    assert!(opaque_at(&fixture.core, 18, 36), "the sprite is opaque");
    assert_eq!(0x30, pixel_at(&fixture.core, 18, 36));
    assert!(!opaque_at(&fixture.core, 100, 100), "and nothing else is");
}

/// REQ-CORE-049: the sprite layer's own PPUMASK switches gate the bit as the background's do —
/// sprites off, and the left-column mask for `x < 8`.
#[test]
fn req_core_049_the_sprite_switches_of_ppumask_gate_the_bit() {
    let sprite_at_left = |mask: u8| {
        let mut fixture = TileFixture::new();
        fixture.write_vram(0x3F00, &[0x0F]);
        fixture.write_vram(0x3F11, &[0x30]);
        fixture.fill_vram(0x2000, 960, 0x00);
        fixture.set_sprite(0, 0x20, SOLID_TILE, 0x00, 0x00);
        fixture.set_scroll(0, 0);
        fixture.set_control(0x00);
        fixture.set_mask(mask);
        fixture.next_batch();
        opaque_at(&fixture.core, 4, 36)
    };

    assert!(sprite_at_left(0x1E), "sprites on, left column shown");
    assert!(!sprite_at_left(0x18), "bit 2 off: the left 8 columns go");
    assert!(!sprite_at_left(0x0E), "bit 4 off: no sprite at all");
}

/// REQ-CORE-049: the greyscale bit of PPUMASK is a video effect on the **colour** and must not
/// touch the mask (REQ-CORE-047 read with the new bit).
#[test]
fn req_core_049_the_greyscale_bit_leaves_the_mask_alone() {
    let fixture = background(SOLID_TILE, 0x0F, 0x16, 0x0B);

    assert_eq!(0x10, pixel_at(&fixture.core, 100, 100), "$16 & $30");
    assert!(opaque_at(&fixture.core, 100, 100), "still an opaque pixel");
}

/// REQ-CORE-048 as AMD-050 amends it (P1, acquired independently of the mask): the index image
/// is the NES colour index the PPU shows, 0 to 63, and nothing else.
#[test]
fn req_core_048_the_index_image_is_the_colour_indices_alone() {
    let fixture = background(SOLID_TILE, 0x0F, 0x30, 0x0A);

    let mut indices = Vec::new();
    fixture.core.pixel_frame_indices_into(&mut indices);
    assert_eq!(256 * 240, indices.len());
    assert!(
        indices.iter().all(|&index| index <= 63),
        "every byte is a NES colour index"
    );
    assert!(indices.iter().all(|&index| index == 0x30));

    // And it is the same image the stored bytes carry, bit 6 aside.
    let stored: Vec<u8> = fixture
        .core
        .pixel_frame_with_mask()
        .iter()
        .map(|byte| byte & PIXEL_COLOR)
        .collect();
    assert_eq!(stored, indices);
}

/// REQ-CORE-049: the bit is derived from the palette **index**, so a state restore — which
/// replaces the whole palette without going through a register write — rebuilds the shadow
/// table and goes on producing the right bit.
#[test]
fn req_core_049_the_mask_survives_a_state_restore() {
    let fixture = background(SOLID_TILE, 0x0F, 0x30, 0x0A);
    let state = fixture.core.save_state(true).expect("a ROM is loaded");

    let mut restored = TileFixture::new();
    restored
        .core
        .try_load_state(&state, 0)
        .expect("a valid state");
    restored.core.run_frame(0);
    assert!(
        restored
            .core
            .pixel_frame_with_mask()
            .iter()
            .all(|byte| byte & OPAQUE_PIXEL != 0),
        "the shadow palette was rebuilt on restore"
    );
    assert_eq!(0x30, pixel_at(&restored.core, 100, 100));
}
