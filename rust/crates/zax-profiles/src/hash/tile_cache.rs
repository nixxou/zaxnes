//! The cache of resolved tiles of a "3dh" profile (AMD-062 REQ-PROF-082).
//!
//! It holds nothing but pairs "manifest string → sixteen bytes", each of which recomputes from
//! its bytes, so a wrong entry can always be told from a right one and the cache can never make a
//! profile mean something the ROM does not say. The format is this module's own ("format libre"):
//! a header line, then one line per pair. A file without the header, or a line that does not
//! parse or does not recompute, is ignored — a cache is an optimisation, never an authority.
//!
//! The cache is kept per profile `Id` in the user's data folder; where that folder is belongs to
//! the application. It has no effect on the bytes of the profile file.

use crate::hash::raw_tile::{self, RawTile};
use crate::hash::tiles;

/// First line of a cache file, which also carries its version.
const HEADER: &str = "zaxnes-tile-cache 1";

/// Extension of a cache file.
pub const EXTENSION: &str = "tiles";

/// The file name of the cache of the profile `id`.
///
/// Every character a file name cannot be sure to carry is replaced by `_`: the derived
/// identifier of AMD-037 is standard base64 and may hold `/` and `+`. Two identifiers can then
/// share a file, which costs nothing — an entry a profile does not wait for is ignored, and an
/// entry it does wait for carries the bytes that string recomputes from, whichever profile wrote
/// it (REQ-PROF-082).
#[must_use]
pub fn file_name(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    format!("{safe}.{EXTENSION}")
}

/// Writes the cache of the resolved tiles `entries` gives, in the order it gives them.
#[must_use]
pub fn write<'a>(entries: impl IntoIterator<Item = (&'a str, &'a RawTile)>) -> String {
    let mut text = String::from(HEADER);
    for (key, raw) in entries {
        text.push('\n');
        text.push_str(key);
        text.push(' ');
        text.push_str(&raw_tile::hex_lowercase(raw));
    }
    text.push('\n');
    text
}

/// Reads a cache file. An unreadable file gives no entry at all, and an entry whose string does
/// not recompute from its bytes is left out (REQ-PROF-082).
#[must_use]
pub fn read(text: &str) -> Vec<(String, RawTile)> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Vec::new();
    }

    lines.filter_map(entry_of).collect()
}

/// True when `text` is a cache that holds at least one entry, which is what makes it worth
/// writing: a profile that resolved nothing leaves no file behind.
#[must_use]
pub fn has_entry(text: &str) -> bool {
    !read(text).is_empty()
}

/// Reads a cache file held as bytes; bytes that are not UTF-8 give no entry.
#[must_use]
pub fn read_bytes(bytes: &[u8]) -> Vec<(String, RawTile)> {
    std::str::from_utf8(bytes).map(read).unwrap_or_default()
}

fn entry_of(line: &str) -> Option<(String, RawTile)> {
    let (key, hex) = line.split_once(' ')?;
    let raw = raw_tile_of(hex)?;
    tiles::recomputes(key, &raw).then(|| (key.to_owned(), raw))
}

/// The sixteen bytes of a string of 32 lowercase hexadecimal digits.
fn raw_tile_of(hex: &str) -> Option<RawTile> {
    if hex.len() != raw_tile::BYTE_COUNT * 2 {
        return None;
    }

    let mut raw = [0_u8; raw_tile::BYTE_COUNT];
    for (byte, digits) in raw.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
        let text = std::str::from_utf8(digits).ok()?;
        *byte = u8::from_str_radix(text, 16).ok()?;
        // Upper case would still parse; the written form is lower case, and a cache that was not
        // written here is not one to trust.
        if text.bytes().any(|digit| digit.is_ascii_uppercase()) {
            return None;
        }
    }

    Some(raw)
}
