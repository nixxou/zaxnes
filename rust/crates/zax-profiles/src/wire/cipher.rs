//! The reversible masking of the tile planes (SPEC-03 REQ-PROF-030): a stored plane is
//! `rotr64(plane XOR key, rotation)`.

/// The XOR key of the planes.
pub const KEY: u64 = 0x0007_08FA_CD88_4A5A;

/// The rotation, in bits: the low six bits of the key.
pub const ROTATION: u32 = (KEY & 0x3F) as u32;

/// The plane in clear of a stored (masked) plane.
#[must_use]
pub fn decode(stored: i64) -> u64 {
    // The file stores the plane as a signed varint; the bits are the plane's.
    u64::from_le_bytes(stored.to_le_bytes()).rotate_left(ROTATION) ^ KEY
}

/// The stored (masked) plane of a plane in clear.
#[must_use]
pub fn encode(plane: u64) -> i64 {
    i64::from_le_bytes((plane ^ KEY).rotate_right(ROTATION).to_le_bytes())
}
