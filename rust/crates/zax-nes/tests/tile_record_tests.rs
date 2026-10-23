//! The bit-plane encoding of a tile record, which is also the recognition key of the
//! conversion engine and must be respected to the bit (REQ-CORE-034, REQ-CORE-036).

mod common;

use common::pattern;
use zax_nes::TileRecord;

fn record(plane0: u64, plane1: u64, background: bool) -> TileRecord {
    TileRecord::new(plane0, plane1, 0, 0, 0, false, false, false, 0, background)
}

#[test]
fn req_core_034_the_acceptance_vector_of_the_plane_encoding() {
    // Plane 0 = 80 00 00 00 00 00 00 01, plane 1 = 00 x 8.
    let plane0 = TileRecord::pack_plane([0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
    let plane1 = TileRecord::pack_plane([0; 8]);
    assert_eq!(0x0100_0000_0000_0080, plane0);
    assert_eq!(0, plane1);

    let record = record(plane0, plane1, true);

    // Source pixel (0,0) is 1, (7,7) is 1, everything else is 0.
    assert_eq!(Some(1), record.source_pixel(0, 0));
    assert_eq!(Some(1), record.source_pixel(7, 7));
    for row in 0..8 {
        for column in 0..8 {
            let lit = matches!((row, column), (0, 0) | (7, 7));
            assert_eq!(Some(u8::from(lit)), record.source_pixel(column, row));
        }
    }
}

#[test]
fn req_core_034_bit_7_of_a_row_byte_is_the_leftmost_pixel() {
    // Row 0 = $80: only column 0 is set.
    let mut record = record(TileRecord::pack_plane([0x80, 0, 0, 0, 0, 0, 0, 0]), 0, true);
    assert_eq!(Some(1), record.source_pixel(0, 0));
    assert_eq!(Some(0), record.source_pixel(7, 0));

    // Row 0 = $01: only column 7.
    record.plane0 = TileRecord::pack_plane([0x01, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(Some(0), record.source_pixel(0, 0));
    assert_eq!(Some(1), record.source_pixel(7, 0));
}

#[test]
fn req_core_034_the_two_planes_give_colours_zero_to_three() {
    // Row 0: plane 0 = %11000000, plane 1 = %10100000 → colours 3, 1, 2, 0, ...
    let plane0 = TileRecord::pack_plane([0b1100_0000, 0, 0, 0, 0, 0, 0, 0]);
    let plane1 = TileRecord::pack_plane([0b1010_0000, 0, 0, 0, 0, 0, 0, 0]);
    let record = record(plane0, plane1, true);

    assert_eq!(Some(3), record.source_pixel(0, 0));
    assert_eq!(Some(1), record.source_pixel(1, 0));
    assert_eq!(Some(2), record.source_pixel(2, 0));
    assert_eq!(Some(0), record.source_pixel(3, 0));
}

#[test]
fn req_core_034_planes_are_stored_without_flips_applied() {
    let plain = record(
        TileRecord::pack_plane([0x80, 0, 0, 0, 0, 0, 0, 0]),
        0,
        false,
    );
    let flipped_h = TileRecord {
        mirror_h: true,
        ..plain
    };
    let flipped_v = TileRecord {
        mirror_v: true,
        ..plain
    };

    // The stored planes are identical: only the lookup changes.
    assert_eq!(plain.plane0, flipped_h.plane0);
    assert_eq!(plain.plane0, flipped_v.plane0);

    assert_eq!(Some(1), plain.displayed_pixel(0, 0));
    assert_eq!(Some(1), flipped_h.displayed_pixel(7, 0));
    assert_eq!(Some(0), flipped_h.displayed_pixel(0, 0));
    assert_eq!(Some(1), flipped_v.displayed_pixel(0, 7));
    assert_eq!(Some(0), flipped_v.displayed_pixel(0, 0));

    let flipped_both = TileRecord {
        mirror_h: true,
        mirror_v: true,
        ..plain
    };
    assert_eq!(Some(1), flipped_both.displayed_pixel(7, 7));
}

#[test]
fn req_core_036_an_empty_record_is_still_a_record() {
    let empty = TileRecord::new(0, 0, 8, 16, 5, false, false, false, 0, true);
    assert!(empty.is_empty());
    assert_eq!(8, empty.x);
    assert_eq!(16, empty.y);
    assert_eq!(5, empty.index);
}

#[test]
fn req_core_034_the_minimal_rom_patterns_match_the_notation_of_section_5_1() {
    // M($0000, 3): plane 0 = $0303030303030303, plane 1 = $FCFC...
    assert_eq!(
        (0x0303_0303_0303_0303, 0xFCFC_FCFC_FCFC_FCFC),
        pattern(0, 3)
    );

    // M($1000, $21): plane 0 = $2121..., plane 1 = $DEDE... (vector V2).
    assert_eq!(
        (0x2121_2121_2121_2121, 0xDEDE_DEDE_DEDE_DEDE),
        pattern(1, 0x21)
    );

    // Tile 0 of table $0000 is entirely zero.
    assert_eq!((0, 0), pattern(0, 0));
}

#[test]
fn req_core_034_out_of_range_coordinates_are_rejected() {
    let record = record(0, 0, true);
    assert_eq!(None, record.source_pixel(8, 0));
    assert_eq!(None, record.source_pixel(0, 8));
    assert_eq!(None, record.displayed_pixel(8, 0));
    // `pack_plane` takes exactly eight bytes, so a seven-byte plane does not compile.
}
