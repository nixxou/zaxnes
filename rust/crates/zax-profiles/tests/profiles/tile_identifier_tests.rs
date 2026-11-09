//! The tile identifier, the legacy digest, the mixing prefix and the mirrorings of a raw tile
//! (AMD-062 REQ-PROF-075, 076, 077, 079), on the spec's own vectors.

use rstest::rstest;
use zax_profiles::enums::TileFlip;
use zax_profiles::hash::raw_tile::{self, RawTile};
use zax_profiles::hash::tile_id::{self, TileId};
use zax_profiles::tile::ProfileTile;

/// The sixteen bytes a hexadecimal vector stands for.
fn raw_of(hex: &str) -> RawTile {
    let bytes = from_hex(hex);
    let mut raw = [0u8; 16];
    raw.copy_from_slice(&bytes);
    raw
}

fn from_hex(hex: &str) -> Vec<u8> {
    assert!(
        hex.len() % 2 == 0,
        "a hexadecimal string has an even length"
    );
    (0..hex.len() / 2)
        .map(|index| u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap())
        .collect()
}

// REQ-PROF-075 and REQ-PROF-077: the four vectors, prefix, binary form and text form.
#[rstest]
#[case(
    "00000000000000000000000000000000",
    0x06e2_8b2d,
    "2d8be206b548f95b38bf8a3e2d7f50f9",
    "f9507f2d3e8abf385bf948b506e28b2d"
)]
#[case(
    "ffffffffffffffffffffffffffffffff",
    0x5343_027f,
    "7f024353624f541ff1301725c2461716",
    "161746c2251730f11f544f625343027f"
)]
#[case(
    "000102030405060708090a0b0c0d0e0f",
    0x870d_cae2,
    "e2ca0d87ba7804d4c5cccdc426124e91",
    "914e1226c4cdccc5d40478ba870dcae2"
)]
#[case(
    "ffffffffffffffff0000000000000000",
    0x1dbb_ccea,
    "eaccbb1d67be65f246453106c3c0a5d3",
    "d3a5c0c306314546f265be671dbbccea"
)]
fn the_identifier_of_a_raw_tile_follows_the_vectors(
    #[case] raw_hex: &str,
    #[case] prefix: u32,
    #[case] binary_hex: &str,
    #[case] text: &str,
) {
    let raw = raw_of(raw_hex);
    let (low, high) = raw_tile::to_planes(&raw);

    assert_eq!(prefix, tile_id::compute_prefix(low, high));
    assert_eq!(prefix, tile_id::prefix_of(&raw));

    let id = TileId::compute(&raw);
    assert_eq!(prefix, id.prefix());
    assert_eq!(binary_hex, raw_tile::hex_lowercase(&id.to_binary()));
    assert_eq!(text, id.to_text());
    assert_eq!(text, id.to_string());

    // The binary form and the text form are two readings of the same pair.
    assert_eq!(Some(id), TileId::from_binary(&id.to_binary()));
    assert_eq!(Some(id), TileId::parse_text(text));

    // The prefix filter of REQ-PROF-077: hashing with a prefix already known gives the same
    // identifier as hashing from the raw bytes alone.
    assert_eq!(id, TileId::compute_with_prefix(&raw, prefix));
}

// REQ-PROF-076: the legacy digest of the null tile and of 00 01 ... 0f.
#[rstest]
#[case(
    "00000000000000000000000000000000",
    "374708fff7719dd5979ec875d56cd2286f6d3cf7ec317a3b25632aab28ec37bb"
)]
#[case(
    "000102030405060708090a0b0c0d0e0f",
    "be45cb2605bf36bebde684841a28f0fd43c69850a3dce5fedba69928ee3a8991"
)]
fn the_legacy_digest_follows_the_vectors(#[case] raw_hex: &str, #[case] digest: &str) {
    assert_eq!(digest, raw_tile::legacy_digest(&raw_of(raw_hex)));
    assert!(tile_id::is_legacy_digest(digest));
}

// REQ-PROF-079: the three mirrorings of 00 01 ... 0f.
#[rstest]
#[case(TileFlip::HORIZONTAL, "008040c020a060e0109050d030b070f0")]
#[case(TileFlip::VERTICAL, "07060504030201000f0e0d0c0b0a0908")]
#[case(TileFlip::from_bits(3), "e060a020c0408000f070b030d0509010")]
fn the_mirrorings_follow_the_vectors(#[case] flip: TileFlip, #[case] expected: &str) {
    let raw = raw_of("000102030405060708090a0b0c0d0e0f");
    assert_eq!(
        expected,
        raw_tile::hex_lowercase(&raw_tile::flipped(&raw, flip))
    );
}

#[test]
fn a_mirroring_of_zero_leaves_the_tile_alone() {
    let raw = raw_of("000102030405060708090a0b0c0d0e0f");
    assert_eq!(raw, raw_tile::flipped(&raw, TileFlip::NONE));
}

#[test]
fn every_mirroring_is_its_own_inverse() {
    let raw = raw_of("123456789abcdef0fedcba9876543210");
    for bits in 0..4 {
        let flip = TileFlip::from_bits(bits);
        assert_eq!(raw, raw_tile::flipped(&raw_tile::flipped(&raw, flip), flip));
    }
}

// REQ-PROF-079: the four flips of a window, in the order of the flip bits, are what a ROM scan
// confronts to the manifest (REQ-PROF-078 point 2).
#[test]
fn the_four_flips_are_in_the_order_of_the_flip_bits() {
    let raw = raw_of("000102030405060708090a0b0c0d0e0f");
    let flips = raw_tile::all_flips(&raw);
    for bits in 0..4 {
        let flip = TileFlip::from_bits(bits);
        assert_eq!(
            raw_tile::flipped(&raw, flip),
            flips[usize::try_from(bits).unwrap()]
        );
    }
}

// REQ-PROF-075: the planes read from the raw bytes are the planes in clear of SPEC-03, so a raw
// tile and the profile tile of the same content show the same pixels.
#[test]
fn the_planes_of_a_raw_tile_are_the_planes_in_clear() {
    let raw = raw_of("ff00000000000000ff00000000000000");
    let (low, high) = raw_tile::to_planes(&raw);
    let tile = ProfileTile::new(low, high, TileFlip::NONE);

    for x in 0..8 {
        assert_eq!(Some(3), tile.color_at(x, 0));
        assert_eq!(Some(0), tile.color_at(x, 1));
    }

    assert_eq!(raw, raw_tile::from_planes(low, high));
}

#[test]
fn the_text_form_only_accepts_thirty_two_lowercase_digits() {
    assert_eq!(None, TileId::parse_text(""));
    assert_eq!(None, TileId::parse_text(&"0".repeat(31)));
    assert_eq!(None, TileId::parse_text(&"0".repeat(33)));
    assert_eq!(None, TileId::parse_text(&"A".repeat(32)));
    assert!(TileId::parse_text(&"a".repeat(32)).is_some());

    assert!(tile_id::is_identifier_text(&"a".repeat(32)));
    assert!(!tile_id::is_identifier_text(&"A".repeat(32)));
    assert!(!tile_id::is_identifier_text(&"a".repeat(64)));
    assert!(!tile_id::is_legacy_digest(&"a".repeat(32)));
    assert!(!tile_id::is_legacy_digest(&"g".repeat(64)));
}

#[test]
fn a_blank_tile_is_recognised_and_the_domain_string_is_the_format_datum() {
    assert!(raw_tile::is_blank(&[0u8; 16]));
    assert!(!raw_tile::is_blank(&raw_of(
        "000102030405060708090a0b0c0d0e0f"
    )));
    assert_eq!("3dSen.TileId.v1", tile_id::DOMAIN_STRING);
    assert_eq!(15, tile_id::DOMAIN_STRING.len());
}
