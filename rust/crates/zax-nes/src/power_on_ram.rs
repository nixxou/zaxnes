//! What the 2 KiB of work RAM hold at power-on (lot CARTDB step 2).
//!
//! A real console's RAM starts with whatever its cells settled to. Most games clear it first; a
//! few read it before writing it, which is why FCEUX and NesHawk fill it with
//! `00 00 00 00 FF FF FF FF` (NesHawk's `NES.Core.cs` names *Cybernoid*, *Minna no Taabou no
//! Nakayoshi Daisakusen* and *Huang Di*) and why Mesen lets the player choose. Zaxnes keeps zeros
//! by default, as REQ-CORE-004 has it.

/// The power-on contents of the work RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerOnRam {
    /// All `$00` (REQ-CORE-004, the default).
    #[default]
    Zero,
    /// `00 00 00 00 FF FF FF FF` repeating: FCEUX's default and NesHawk's.
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
