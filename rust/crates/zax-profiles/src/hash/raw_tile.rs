//! The sixteen bytes of a tile as they sit in a ROM, and the operations a "3dh" profile needs on
//! them (AMD-062 §5, §6).
//!
//! Low plane in `b[0..7]`, high plane in `b[8..15]`. Read as little-endian 64-bit integers the
//! two halves are exactly the planes in clear of SPEC-03 REQ-PROF-030: bit `y * 8 + (7 - x)` of a
//! plane is the bit of pixel (x, y). A tile's `flip` is stored apart in the profile, so a raw
//! tile is always a **non-flipped** tile.

use sha2::{Digest, Sha256};

use crate::enums::TileFlip;

/// Number of bytes of a raw tile.
pub const BYTE_COUNT: usize = 16;

/// Number of bytes of one bit plane.
pub const PLANE_BYTE_COUNT: usize = 8;

/// The sixteen bytes of a tile, low plane first (AMD-062 §5).
pub type RawTile = [u8; BYTE_COUNT];

/// The two planes in clear of a raw tile.
#[must_use]
pub fn to_planes(raw: &RawTile) -> (u64, u64) {
    let (low, high) = raw.split_at(PLANE_BYTE_COUNT);
    (read_u64_le(low), read_u64_le(high))
}

/// The sixteen raw bytes of a tile given by its planes in clear.
#[must_use]
pub fn from_planes(low: u64, high: u64) -> RawTile {
    let mut raw = [0u8; BYTE_COUNT];
    raw[..PLANE_BYTE_COUNT].copy_from_slice(&low.to_le_bytes());
    raw[PLANE_BYTE_COUNT..].copy_from_slice(&high.to_le_bytes());
    raw
}

/// The mirrored copy of a raw tile (REQ-PROF-079): the horizontal mirror reverses the eight bits
/// of every byte, the vertical mirror reverses the order of the eight bytes **of each plane**.
#[must_use]
pub fn flipped(source: &RawTile, flip: TileFlip) -> RawTile {
    let horizontal = flip.contains(TileFlip::HORIZONTAL);
    let vertical = flip.contains(TileFlip::VERTICAL);
    let mut result = [0u8; BYTE_COUNT];
    for plane in 0..2 {
        let origin = plane * PLANE_BYTE_COUNT;
        for row in 0..PLANE_BYTE_COUNT {
            let source_row = if vertical {
                PLANE_BYTE_COUNT - 1 - row
            } else {
                row
            };
            let value = source[origin + source_row];
            result[origin + row] = if horizontal {
                value.reverse_bits()
            } else {
                value
            };
        }
    }

    result
}

/// The four flips of REQ-PROF-079, in the order of the `flip` bits (0, 1, 2, 3).
#[must_use]
pub fn all_flips(source: &RawTile) -> [RawTile; 4] {
    [
        *source,
        flipped(source, TileFlip::HORIZONTAL),
        flipped(source, TileFlip::VERTICAL),
        flipped(source, TileFlip::HORIZONTAL | TileFlip::VERTICAL),
    ]
}

/// The legacy digest of a raw tile (REQ-PROF-076): `SHA-256(b[0..15])` written as 64 lowercase
/// hexadecimal digits.
#[must_use]
pub fn legacy_digest(raw: &RawTile) -> String {
    hex_lowercase(Sha256::digest(raw).as_slice())
}

/// True when every one of the sixteen bytes is zero.
#[must_use]
pub fn is_blank(raw: &RawTile) -> bool {
    raw.iter().all(|&byte| byte == 0)
}

/// Lowercase hexadecimal form of a byte string.
#[must_use]
pub fn hex_lowercase(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing into a String never fails.
        let _ = write!(text, "{byte:02x}");
    }

    text
}

/// Reads eight bytes as a little-endian 64-bit integer; a shorter slice is zero-extended, which
/// the callers of this module never produce (every slice comes from a fixed-size array).
fn read_u64_le(bytes: &[u8]) -> u64 {
    let mut buffer = [0u8; 8];
    let taken = bytes.len().min(8);
    buffer[..taken].copy_from_slice(&bytes[..taken]);
    u64::from_le_bytes(buffer)
}
