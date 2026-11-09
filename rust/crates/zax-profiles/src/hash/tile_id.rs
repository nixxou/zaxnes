//! The content identifier of a tile in a "3dh" profile (AMD-062 REQ-PROF-075 to REQ-PROF-077):
//! a pair of 64-bit integers derived from the sixteen raw bytes of the tile.
//!
//! [`TileId::low`] carries the mixing prefix of REQ-PROF-077 in its low 32 bits and the first
//! four bytes of the digest in its high 32 bits; [`TileId::high`] carries bytes 4 to 11 of the
//! digest. Because the prefix is a cheap function of the two planes, a scan can compute it for
//! every window of a ROM and only hash the windows whose prefix is in the profile's manifest.
//!
//! The **binary form** (envelope field 12, tile field 5) is `low` then `high`, each
//! little-endian. The **text form**, which is what every comparison to the manifest uses, is 32
//! lowercase hexadecimal digits, `high` first.

use sha2::{Digest, Sha256};

use crate::hash::raw_tile::{self, RawTile};

/// Number of bytes of the binary form.
pub const BYTE_COUNT: usize = 16;

/// Number of hexadecimal digits of the text form.
pub const TEXT_LENGTH: usize = 32;

/// Number of hexadecimal digits of a legacy digest (REQ-PROF-076).
pub const LEGACY_DIGEST_LENGTH: usize = 64;

/// The domain string the digest is prefixed with; a datum of the format, not a convenience name.
pub const DOMAIN_STRING: &str = "3dSen.TileId.v1";

/// The content identifier of a raw tile (REQ-PROF-075).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileId {
    /// First half, `L`: the prefix and the first four digest bytes.
    pub low: u64,
    /// Second half, `H`: digest bytes 4 to 11.
    pub high: u64,
}

impl TileId {
    /// Builds an identifier from its two halves.
    #[must_use]
    pub const fn new(low: u64, high: u64) -> Self {
        Self { low, high }
    }

    /// The mixing prefix of REQ-PROF-077, which is the low 32 bits of `L`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // The prefix is by definition the low 32 bits.
    pub const fn prefix(self) -> u32 {
        self.low as u32
    }

    /// The identifier of a raw tile (REQ-PROF-075).
    #[must_use]
    pub fn compute(raw: &RawTile) -> Self {
        let (low, high) = raw_tile::to_planes(raw);
        Self::compute_with_prefix(raw, compute_prefix(low, high))
    }

    /// The identifier of a raw tile whose prefix has already been computed, which is what a scan
    /// does once the prefix has passed the manifest filter.
    #[must_use]
    pub fn compute_with_prefix(raw: &RawTile, prefix: u32) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_STRING.as_bytes());
        hasher.update(raw);
        let digest = hasher.finalize();

        let mut first = [0u8; 4];
        first.copy_from_slice(&digest[0..4]);
        let mut second = [0u8; 8];
        second.copy_from_slice(&digest[4..12]);

        Self {
            low: u64::from(prefix) | (u64::from(u32::from_le_bytes(first)) << 32),
            high: u64::from_le_bytes(second),
        }
    }

    /// Reads the binary form: `L` then `H`, each little-endian. None when the slice is shorter
    /// than sixteen bytes.
    #[must_use]
    pub fn from_binary(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < BYTE_COUNT {
            return None;
        }

        let mut low = [0u8; 8];
        low.copy_from_slice(&bytes[0..8]);
        let mut high = [0u8; 8];
        high.copy_from_slice(&bytes[8..16]);
        Some(Self {
            low: u64::from_le_bytes(low),
            high: u64::from_le_bytes(high),
        })
    }

    /// The sixteen bytes of the binary form.
    #[must_use]
    pub fn to_binary(self) -> [u8; BYTE_COUNT] {
        let mut bytes = [0u8; BYTE_COUNT];
        bytes[..8].copy_from_slice(&self.low.to_le_bytes());
        bytes[8..].copy_from_slice(&self.high.to_le_bytes());
        bytes
    }

    /// The text form: 32 lowercase hexadecimal digits, `H` on the first sixteen and `L` on the
    /// last sixteen. This is the form the manifest holds.
    #[must_use]
    pub fn to_text(self) -> String {
        format!("{:016x}{:016x}", self.high, self.low)
    }

    /// Reads a text form. Accepts exactly 32 lowercase hexadecimal digits, which is what the
    /// format writes; an entry in any other shape is not an identifier.
    #[must_use]
    pub fn parse_text(text: &str) -> Option<Self> {
        if text.len() != TEXT_LENGTH || !is_lowercase_hex(text) {
            return None;
        }

        let high = u64::from_str_radix(&text[0..16], 16).ok()?;
        let low = u64::from_str_radix(&text[16..32], 16).ok()?;
        Some(Self { low, high })
    }
}

impl std::fmt::Display for TileId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:016x}{:016x}", self.high, self.low)
    }
}

/// The 32-bit mixing prefix `P` of REQ-PROF-077, a function of the two planes in clear.
///
/// The six steps are a measured mixing function with no meaning of their own; they are
/// interoperability constants, like `K`, and are reproduced bit for bit.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // Step 6 is the low 32 bits by definition.
pub const fn compute_prefix(low: u64, high: u64) -> u32 {
    let product = (low ^ high ^ 0x5997_3F00_3336_2349) as u128 * 0x9E37_79B1_85EB_CA87_u64 as u128;
    let mut mixed_low = (product as u64).wrapping_add(0x03C0_0000_0000_0000);
    let mut mixed_high = (product >> 64) as u64;

    let h2 = high ^ 0xC202_7976_92D6_3D58;
    mixed_high = mixed_high
        .wrapping_add(h2)
        .wrapping_add((h2 & 0xFFFF_FFFF).wrapping_mul(0x85EB_CA76));

    mixed_low ^= mixed_high.swap_bytes();

    let mut v = mixed_low.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    v ^= v >> 37;
    v = v.wrapping_mul(0x1656_6791_9E37_79F9);
    v ^= v >> 32;
    v as u32
}

/// The prefix of a raw tile.
#[must_use]
pub fn prefix_of(raw: &RawTile) -> u32 {
    let (low, high) = raw_tile::to_planes(raw);
    compute_prefix(low, high)
}

/// True when every character is a digit or a lowercase hexadecimal letter.
#[must_use]
pub fn is_lowercase_hex(text: &str) -> bool {
    text.bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// True when `text` is 64 lowercase hexadecimal digits (REQ-PROF-076).
#[must_use]
pub fn is_legacy_digest(text: &str) -> bool {
    text.len() == LEGACY_DIGEST_LENGTH && is_lowercase_hex(text)
}

/// True when `text` is 32 lowercase hexadecimal digits (REQ-PROF-075, text form).
#[must_use]
pub fn is_identifier_text(text: &str) -> bool {
    text.len() == TEXT_LENGTH && is_lowercase_hex(text)
}
