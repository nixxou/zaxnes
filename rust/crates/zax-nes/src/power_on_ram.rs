//! What the 2 KiB of work RAM hold at power-on (lot CARTDB step 2).
//!
//! A real console's RAM starts with whatever its cells settled to. Most games clear it first; a
//! few read it before writing it, which is why FCEUX and NesHawk fill it with
//! `00 00 00 00 FF FF FF FF` (NesHawk's `NES.Core.cs` names *Cybernoid*, *Minna no Taabou no
//! Nakayoshi Daisakusen* and *Huang Di*) and why Mesen lets the player choose (zeros by default
//! there). Zaxnes takes FCEUX's pattern by default since the owner's decision of 2026-11-13;
//! REQ-CORE-004's zeros stay a choice.

/// The power-on contents of the work RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerOnRam {
    /// All `$00` (REQ-CORE-004).
    Zero,
    /// `00 00 00 00 FF FF FF FF` repeating: FCEUX's default and NesHawk's, and Zaxnes's.
    #[default]
    Pattern,
    /// All `$FF`.
    Ff,
    /// Pseudo-random bytes from the seed (`xorshift64*`), so a run can be repeated.
    Random(u64),
}

impl PowerOnRam {
    /// Writes the pattern over `ram`.
    #[allow(clippy::cast_possible_truncation)] // One byte of each draw is kept, by design.
    pub fn fill(self, ram: &mut [u8]) {
        match self {
            Self::Zero => ram.fill(0),
            Self::Ff => ram.fill(0xFF),
            Self::Pattern => {
                for (index, byte) in ram.iter_mut().enumerate() {
                    *byte = if index & 4 != 0 { 0xFF } else { 0x00 };
                }
            }
            Self::Random(seed) => {
                let mut state = seed | 1;
                for byte in ram.iter_mut() {
                    state ^= state >> 12;
                    state ^= state << 25;
                    state ^= state >> 27;
                    *byte = (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 56) as u8;
                }
            }
        }
    }
}

/// What the PPU's own memories — palette RAM, OAM and the 2 KiB of nametable RAM — hold at
/// power-on (lot FIDELITY step 36, `docs/review/FIDELITY-LEADS.md` B3). Like the work RAM they
/// start with whatever their cells settled to, and that differs from one console to another:
/// blargg's `power_up_palette` test reads the palette of his console, which `NesHawk` copies and
/// `TriCNES` keeps as an alternative to its author's own, a different one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerOnPpuRam {
    /// All `$00` (REQ-CORE-004): this core's power-on so far.
    #[default]
    Zero,
    /// The palette blargg's `power_up_palette` expects (his console's); OAM and the nametable
    /// RAM at `$00`.
    Blargg,
    /// Pseudo-random palette, OAM and nametable RAM from the seed, so a run can be repeated.
    Random(u64),
}

/// The palette of blargg's console at power-on, as `power_up_palette` checks it.
pub const BLARGG_POWER_UP_PALETTE: [u8; 32] = [
    0x09, 0x01, 0x00, 0x01, 0x00, 0x02, 0x02, 0x0D, 0x08, 0x10, 0x08, 0x24, 0x00, 0x00, 0x04, 0x2C,
    0x09, 0x01, 0x34, 0x03, 0x00, 0x04, 0x00, 0x14, 0x08, 0x3A, 0x00, 0x02, 0x00, 0x20, 0x2C, 0x08,
];

impl PowerOnPpuRam {
    /// The palette, OAM and nametable RAM contents, or `None` for all zeros.
    #[must_use]
    pub fn contents(self) -> Option<([u8; 32], [u8; 256], [u8; 2048])> {
        match self {
            Self::Zero => None,
            Self::Blargg => Some((BLARGG_POWER_UP_PALETTE, [0; 256], [0; 2048])),
            Self::Random(seed) => {
                let mut palette = [0u8; 32];
                let mut oam = [0u8; 256];
                let mut vram = [0u8; 2048];
                PowerOnRam::Random(seed).fill(&mut palette);
                PowerOnRam::Random(seed ^ 0x5A5A).fill(&mut oam);
                PowerOnRam::Random(seed ^ 0xA5A5).fill(&mut vram);
                Some((palette, oam, vram))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pattern_is_fceuxs() {
        let mut ram = [0x55u8; 16];
        PowerOnRam::Pattern.fill(&mut ram);
        assert_eq!(&ram[..8], &[0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn a_seed_repeats() {
        let (mut a, mut b) = ([0u8; 64], [0u8; 64]);
        PowerOnRam::Random(7).fill(&mut a);
        PowerOnRam::Random(7).fill(&mut b);
        assert_eq!(a, b);
        assert!(a.iter().any(|&byte| byte != a[0]));
    }
}
