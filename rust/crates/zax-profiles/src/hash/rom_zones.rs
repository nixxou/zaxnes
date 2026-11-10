//! The PRG and CHR zones of an iNES file (AMD-062 REQ-PROF-078 point 1), where the tiles of a
//! "3dh" profile are looked for.
//!
//! The rule is the amendment's own and is deliberately self-contained: the profile module reads a
//! ROM file only to find sixteen-byte windows in it, and depends on no emulation core
//! (REQ-ARCH-002). What it computes is the same header arithmetic the core's own header parser
//! does, bounded by the file's length, which a test checks against that parser.

use std::ops::Range;

/// The `NES<EOF>` signature of an iNES file.
const SIGNATURE: [u8; 4] = [b'N', b'E', b'S', 0x1A];

/// Length of the header.
const HEADER_SIZE: usize = 16;

/// Length of the trainer, skipped when bit 2 of byte 6 is set.
const TRAINER_SIZE: usize = 512;

/// Size unit of the PRG zone.
const PRG_UNIT: usize = 16 * 1024;

/// Size unit of the CHR zone.
const CHR_UNIT: usize = 8 * 1024;

/// The two zones of an iNES file, as byte ranges of the file, each truncated to its length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomZones {
    /// The PRG zone.
    pub prg: Range<usize>,
    /// The CHR zone, empty for a cartridge whose character memory is RAM.
    pub chr: Range<usize>,
}

impl RomZones {
    /// The zones of `file`, or none when it is not an iNES file — which resolves nothing, every
    /// tile staying as it is (REQ-PROF-078).
    #[must_use]
    pub fn of(file: &[u8]) -> Option<Self> {
        if file.len() < HEADER_SIZE || file[..4] != SIGNATURE {
            return None;
        }

        let is_nes20 = (file[7] & 0x0C) == 0x08;
        let (prg_size, chr_size) = if is_nes20 {
            (
                nes20_size(file[4], file[9] & 0x0F, PRG_UNIT),
                nes20_size(file[5], file[9] >> 4, CHR_UNIT),
            )
        } else {
            (
                usize::from(file[4]) * PRG_UNIT,
                usize::from(file[5]) * CHR_UNIT,
            )
        };

        let trainer = if file[6] & 0x04 == 0 { 0 } else { TRAINER_SIZE };
        let prg_start = HEADER_SIZE.saturating_add(trainer);
        let prg_end = prg_start.saturating_add(prg_size);
        let chr_end = prg_end.saturating_add(chr_size);

        Some(Self {
            prg: zone(prg_start, prg_end, file.len()),
            chr: zone(prg_end, chr_end, file.len()),
        })
    }

    /// The two zones in the order REQ-PROF-078 scans them.
    #[must_use]
    pub fn in_scan_order(&self) -> [Range<usize>; 2] {
        [self.prg.clone(), self.chr.clone()]
    }
}

/// A size written as NES 2.0 does: the twelve-bit count of units, or — when the high nibble is
/// 15 — the exponent-multiplier notation `(low & 3) * 2 + 1` shifted left by `low >> 2`.
fn nes20_size(low: u8, high_nibble: u8, unit: usize) -> usize {
    if high_nibble == 0x0F {
        let multiplier = usize::from(low & 0x03) * 2 + 1;
        return multiplier
            .checked_shl(u32::from(low >> 2))
            .unwrap_or(usize::MAX);
    }

    ((usize::from(high_nibble) << 8) | usize::from(low)) * unit
}

/// The range `start..end` brought inside `0..length`; empty when it starts past the end.
fn zone(start: usize, end: usize, length: usize) -> Range<usize> {
    let start = start.min(length);
    start..end.clamp(start, length)
}
