//! Patches built into the emulator and applied in memory to the ROMs they were made for.
//!
//! A known ROM is recognised by the CRC-32 of its data without the iNES header (the header of
//! one dump differs from another's), then patched with a BPS patch made against that data; the
//! player's header is kept. The patches carry the hacks' own bytes only: everything else is
//! copied from the player's ROM. A ROM nothing matches is loaded as it is.
//!
//! | ROM | Patch | Source |
//! |---|---|---|
//! | Zelda II - The Adventure of Link (USA) | widescreen (see `docs/WIDESCREEN-EXTENSION.md`) | a fork of the FiendsOfTheElements Zelda II disassembly (ca65), `tools/make_bps.py` |

/// One built-in patch.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinPatch {
    /// What the patch does, for the log.
    pub name: &'static str,
    /// CRC-32 of the ROM data the patch applies to, iNES header excluded.
    pub source_crc: u32,
    /// The BPS patch.
    pub bps: &'static [u8],
}

/// The built-in patches.
pub const PATCHES: [BuiltinPatch; 1] = [BuiltinPatch {
    name: "Zelda II (USA) widescreen",
    source_crc: 0xBA32_2865,
    bps: include_bytes!("../patches/zelda2-usa-widescreen.bps"),
}];

/// A ROM image with the built-in patch it matched applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchedRom {
    /// The patched image, iNES header included when the original had one.
    pub image: Vec<u8>,
    /// The patch's name.
    pub name: &'static str,
}

/// Size of an iNES header.
const HEADER_SIZE: usize = 16;

/// The image with the built-in patch made for it, or `None` when no patch matches (or the one
/// that matches does not apply cleanly, which its checksums would tell).
#[must_use]
pub fn apply(image: &[u8]) -> Option<PatchedRom> {
    let header_size = if image.starts_with(b"NES\x1A") && image.len() > HEADER_SIZE {
        HEADER_SIZE
    } else {
        0
    };
    let (header, data) = image.split_at(header_size);
    let crc = crc32(data);
    PATCHES
        .iter()
        .filter(|patch| patch.source_crc == crc)
        .find_map(|patch| {
            let patched = apply_bps(patch.bps, data).ok()?;
            let mut image = Vec::with_capacity(header.len() + patched.len());
            image.extend_from_slice(header);
            image.extend_from_slice(&patched);
            Some(PatchedRom {
                image,
                name: patch.name,
            })
        })
}

/// Whether a built-in patch is made for `image` (iNES header or not).
#[must_use]
pub fn matches(image: &[u8]) -> bool {
    let header_size = if image.starts_with(b"NES") && image.len() > HEADER_SIZE {
        HEADER_SIZE
    } else {
        0
    };
    let crc = crc32(&image[header_size..]);
    PATCHES.iter().any(|patch| patch.source_crc == crc)
}

/// Why a BPS patch did not apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpsError {
    /// Not a BPS patch, or one cut short.
    Malformed,
    /// The source is not the one the patch was made for.
    WrongSource,
    /// The result does not have the checksum the patch promises.
    WrongTarget,
}

/// Applies the BPS patch `patch` to `source` and returns the target.
///
/// # Errors
/// [`BpsError`] when the patch is malformed, made for another source, or produces a target
/// whose checksum differs from the one it carries.
pub fn apply_bps(patch: &[u8], source: &[u8]) -> Result<Vec<u8>, BpsError> {
    if patch.len() < 4 + 12 || !patch.starts_with(b"BPS1") {
        return Err(BpsError::Malformed);
    }

    let footer_at = patch.len() - 12;
    let footer = |index: usize| {
        let bytes = patch.get(footer_at + (4 * index)..footer_at + (4 * index) + 4);
        bytes
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map(u32::from_le_bytes)
            .ok_or(BpsError::Malformed)
    };
    if crc32(&patch[..footer_at + 8]) != footer(2)? {
        return Err(BpsError::Malformed);
    }
    if crc32(source) != footer(0)? {
        return Err(BpsError::WrongSource);
    }

    let mut reader = Reader {
        bytes: &patch[..footer_at],
        position: 4,
    };
    let source_size = reader.number()?;
    let target_size = reader.number()?;
    let metadata_size = reader.number()?;
    if source_size != source.len() {
        return Err(BpsError::WrongSource);
    }
    reader.skip(metadata_size)?;

    let mut target = Vec::with_capacity(target_size);
    let mut source_offset: usize = 0;
    let mut target_offset: usize = 0;
    while reader.position < reader.bytes.len() {
        let action = reader.number()?;
        let length = (action >> 2) + 1;
        match action & 3 {
            0 => {
                let start = target.len();
                let bytes = source
                    .get(start..start + length)
                    .ok_or(BpsError::Malformed)?;
                target.extend_from_slice(bytes);
            }
            1 => {
                let bytes = reader.take(length)?;
                target.extend_from_slice(bytes);
            }
            2 => {
                source_offset = offset(source_offset, reader.number()?)?;
                let bytes = source
                    .get(source_offset..source_offset + length)
                    .ok_or(BpsError::Malformed)?;
                target.extend_from_slice(bytes);
                source_offset += length;
            }
            _ => {
                target_offset = offset(target_offset, reader.number()?)?;
                for _ in 0..length {
                    // Byte by byte: the copy may overlap the bytes it is writing.
                    let byte = *target.get(target_offset).ok_or(BpsError::Malformed)?;
                    target.push(byte);
                    target_offset += 1;
                }
            }
        }
        if target.len() > target_size {
            return Err(BpsError::Malformed);
        }
    }

    if target.len() != target_size || crc32(&target) != footer(1)? {
        return Err(BpsError::WrongTarget);
    }
    Ok(target)
}

/// A relative BPS offset: bit 0 is the sign, the rest the distance.
fn offset(base: usize, encoded: usize) -> Result<usize, BpsError> {
    let distance = encoded >> 1;
    if encoded & 1 == 0 {
        base.checked_add(distance)
    } else {
        base.checked_sub(distance)
    }
    .ok_or(BpsError::Malformed)
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, BpsError> {
        let byte = *self.bytes.get(self.position).ok_or(BpsError::Malformed)?;
        self.position += 1;
        Ok(byte)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], BpsError> {
        let bytes = self
            .bytes
            .get(self.position..self.position + length)
            .ok_or(BpsError::Malformed)?;
        self.position += length;
        Ok(bytes)
    }

    fn skip(&mut self, length: usize) -> Result<(), BpsError> {
        self.take(length).map(|_| ())
    }

    /// A BPS number: seven bits per byte, the last byte flagged, each continuation adding one.
    fn number(&mut self) -> Result<usize, BpsError> {
        let mut value: usize = 0;
        let mut shift: usize = 1;
        loop {
            let byte = self.byte()?;
            value = usize::from(byte & 0x7F)
                .checked_mul(shift)
                .and_then(|part| value.checked_add(part))
                .ok_or(BpsError::Malformed)?;
            if byte & 0x80 != 0 {
                return Ok(value);
            }
            shift = shift.checked_mul(128).ok_or(BpsError::Malformed)?;
            value = value.checked_add(shift).ok_or(BpsError::Malformed)?;
        }
    }
}

/// CRC-32 (IEEE, the one of zlib and of the ROM databases).
#[must_use]
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 0 {
                crc >> 1
            } else {
                (crc >> 1) ^ 0xEDB8_8320
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BPS number as the format writes it.
    fn number(mut value: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            #[allow(clippy::cast_possible_truncation)] // Masked to seven bits.
            let low = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                out.push(0x80 | low);
                return out;
            }
            out.push(low);
            value -= 1;
        }
    }

    fn patch(source: &[u8], target: &[u8], actions: &[u8]) -> Vec<u8> {
        let mut patch = b"BPS1".to_vec();
        patch.extend(number(source.len()));
        patch.extend(number(target.len()));
        patch.extend(number(0));
        patch.extend_from_slice(actions);
        patch.extend(crc32(source).to_le_bytes());
        patch.extend(crc32(target).to_le_bytes());
        let crc = crc32(&patch);
        patch.extend(crc.to_le_bytes());
        patch
    }

    #[test]
    fn crc32_matches_the_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn numbers_round_trip() {
        for value in [0, 1, 127, 128, 129, 16_511, 16_512, 1 << 20] {
            let bytes = number(value);
            let mut reader = Reader {
                bytes: &bytes,
                position: 0,
            };
            assert_eq!(reader.number(), Ok(value));
        }
    }

    #[test]
    fn source_read_target_read_and_copies() {
        let source = b"ABCDEFGH";
        let target = b"ABxyEFGHGH";
        let mut actions = Vec::new();
        actions.extend(number(1 << 2)); // SourceRead 2: "AB"
        actions.extend(number((1 << 2) | 1)); // TargetRead 2: "xy"
        actions.extend(b"xy");
        actions.extend(number((3 << 2) | 2)); // SourceCopy 4 from +4: "EFGH"
        actions.extend(number(4 << 1));
        actions.extend(number((1 << 2) | 3)); // TargetCopy 2 from +6: "GH"
        actions.extend(number(6 << 1));
        let patch = patch(source, target, &actions);
        assert_eq!(apply_bps(&patch, source), Ok(target.to_vec()));
        assert_eq!(apply_bps(&patch, b"ABCDEFGX"), Err(BpsError::WrongSource));
    }

    #[test]
    fn unknown_roms_are_left_alone() {
        let mut image = b"NES\x1A".to_vec();
        image.resize(16 + 1024, 0);
        assert_eq!(apply(&image), None);
    }

    /// Needs `ZAXNES_ROMS` with `Zelda II - The Adventure of Link (USA).nes`.
    #[test]
    #[ignore = "needs a commercial ROM (ZAXNES_ROMS)"]
    fn the_zelda_ii_patch_applies_to_the_usa_rom() {
        let Some(folder) = std::env::var_os("ZAXNES_ROMS") else {
            return;
        };
        let path = std::path::Path::new(&folder).join("Zelda II - The Adventure of Link (USA).nes");
        let Ok(image) = std::fs::read(path) else {
            return;
        };
        // `apply` checks the source and target checksums of the patch: a `Some` is the build.
        let patched = apply(&image).map(|rom| rom.image).unwrap_or_default();
        assert_eq!(patched.len(), image.len());
        assert_eq!(patched[..16], image[..16], "the player's header is kept");
    }

    #[test]
    fn the_built_in_patches_are_well_formed() {
        for patch in PATCHES {
            assert!(patch.bps.starts_with(b"BPS1"), "{}", patch.name);
            let footer = patch.bps.len() - 12;
            let source_crc = u32::from_le_bytes([
                patch.bps[footer],
                patch.bps[footer + 1],
                patch.bps[footer + 2],
                patch.bps[footer + 3],
            ]);
            assert_eq!(source_crc, patch.source_crc, "{}", patch.name);
        }
    }
}
