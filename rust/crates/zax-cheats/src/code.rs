//! One cheat code: parsing the formats players type, and writing them back.
//!
//! Two different things hide behind "cheat code" on the NES:
//!
//! - a **ROM patch** ([`CheatEffect::RomRead`]): what a Game Genie does. It sits between the
//!   cartridge and the console and replaces the byte the CPU *reads* at `$8000-$FFFF`, optionally
//!   only when the cartridge's byte equals a compare value (8-letter codes), which keeps a code
//!   from hitting the wrong bank on a mapper. It must be applied on the read path, never by
//!   writing memory.
//! - a **RAM write** ([`CheatEffect::RamWrite`]): what a Pro Action Replay does, and the raw
//!   `AAAA:VV` codes of FCEUX, Mesen and RetroArch. The byte is written again every frame,
//!   optionally only while the current value equals a compare value.

use std::fmt;

/// What a code does to the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheatEffect {
    /// Replace the byte the CPU reads at `address` (`$8000-$FFFF`) with `value`, only when the
    /// cartridge's byte is `compare` if there is one.
    RomRead {
        /// CPU address, `$8000-$FFFF`.
        address: u16,
        /// Byte returned to the CPU.
        value: u8,
        /// Byte the cartridge must hold there for the patch to apply.
        compare: Option<u8>,
    },
    /// Write `value` at `address` every frame, only while the byte there is `compare` if there
    /// is one.
    RamWrite {
        /// CPU address.
        address: u16,
        /// Byte written.
        value: u8,
        /// Byte that must be there for the write to happen.
        compare: Option<u8>,
    },
}

/// The format a code was typed in, kept so it is shown back the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeFormat {
    /// 6 or 8 Game Genie letters.
    GameGenie,
    /// 8 hexadecimal digits `00AAAAVV`.
    ProActionReplay,
    /// `AAAA:VV` or `AAAA?CC:VV`.
    Raw,
}

/// Why a code could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeError {
    /// Nothing to read.
    Empty,
    /// A format was recognised but a character or a length is wrong.
    Invalid(String),
}

impl fmt::Display for CodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty code"),
            Self::Invalid(code) => write!(f, "not a Game Genie, Pro Action Replay or raw code: {code}"),
        }
    }
}

impl std::error::Error for CodeError {}

/// One code: its effect and the format it came in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CheatCode {
    /// What it does.
    pub effect: CheatEffect,
    /// How it was written.
    pub format: CodeFormat,
}

/// The Game Genie alphabet: the letter at index `n` stands for the nibble `n`.
const GENIE_LETTERS: &[u8; 16] = b"APZLGITYEOXUKSVN";

fn genie_nibble(letter: u8) -> Option<u8> {
    let upper = letter.to_ascii_uppercase();
    GENIE_LETTERS
        .iter()
        .position(|&l| l == upper)
        .and_then(|n| u8::try_from(n).ok())
}

/// Decodes 6 or 8 Game Genie letters (the nesdev wiki's bit layout).
///
/// # Errors
///
/// [`CodeError::Invalid`] for another length or a letter outside the alphabet.
pub fn decode_game_genie(code: &str) -> Result<CheatEffect, CodeError> {
    let bytes = code.as_bytes();
    if bytes.len() != 6 && bytes.len() != 8 {
        return Err(CodeError::Invalid(code.to_owned()));
    }
    let mut n = [0u8; 8];
    for (slot, &letter) in n.iter_mut().zip(bytes) {
        *slot = genie_nibble(letter).ok_or_else(|| CodeError::Invalid(code.to_owned()))?;
    }
    let address = 0x8000
        | (u16::from(n[3] & 7) << 12)
        | (u16::from(n[5] & 7) << 8)
        | (u16::from(n[4] & 8) << 8)
        | (u16::from(n[2] & 7) << 4)
        | (u16::from(n[1] & 8) << 4)
        | u16::from(n[4] & 7)
        | u16::from(n[3] & 8);
    let low = ((n[1] & 7) << 4) | ((n[0] & 8) << 4) | (n[0] & 7);
    if bytes.len() == 6 {
        Ok(CheatEffect::RomRead { address, value: low | (n[5] & 8), compare: None })
    } else {
        let compare = ((n[7] & 7) << 4) | ((n[6] & 8) << 4) | (n[6] & 7) | (n[5] & 8);
        Ok(CheatEffect::RomRead { address, value: low | (n[7] & 8), compare: Some(compare) })
    }
}

/// Encodes a ROM patch as Game Genie letters: 8 with a compare value, 6 without.
#[must_use]
pub fn encode_game_genie(address: u16, value: u8, compare: Option<u8>) -> String {
    let low = |x: u16| x.to_le_bytes()[0];
    let mut n = [0u8; 8];
    n[0] = (value & 7) | ((value >> 4) & 8);
    n[1] = ((value >> 4) & 7) | low((address >> 4) & 8);
    n[2] = low((address >> 4) & 7);
    n[3] = low(((address >> 12) & 7) | (address & 8));
    n[4] = low((address & 7) | ((address >> 8) & 8));
    n[5] = low((address >> 8) & 7);
    let len = if let Some(compare) = compare {
        // the third letter's high bit tells the device an 8-letter code follows
        n[2] |= 8;
        n[5] |= compare & 8;
        n[6] = (compare & 7) | ((compare >> 4) & 8);
        n[7] = ((compare >> 4) & 7) | (value & 8);
        8
    } else {
        n[5] |= value & 8;
        6
    };
    n[..len].iter().map(|&nibble| char::from(GENIE_LETTERS[usize::from(nibble)])).collect()
}

fn hex_u16(text: &str) -> Option<u16> {
    if text.is_empty() || text.len() > 4 {
        return None;
    }
    u16::from_str_radix(text, 16).ok()
}

fn hex_u8(text: &str) -> Option<u8> {
    if text.is_empty() || text.len() > 2 {
        return None;
    }
    u8::from_str_radix(text, 16).ok()
}

/// `AAAA:VV`, `AAAA?CC:VV` (FCEUX, RetroArch) or `AAAA:VV:CC` (Mesen); a `0x` or `$` before the
/// address is accepted.
fn parse_raw(code: &str) -> Option<CheatEffect> {
    let (left, value) = code.split_once(':')?;
    let (value, trailing_compare) = match value.split_once(':') {
        Some((value, compare)) => (value, Some(hex_u8(compare.trim())?)),
        None => (value, None),
    };
    let value = hex_u8(value.trim())?;
    let (address, compare) = match left.split_once('?') {
        Some(_) if trailing_compare.is_some() => return None,
        Some((address, compare)) => (address, Some(hex_u8(compare.trim())?)),
        None => (left, trailing_compare),
    };
    let address = address.trim();
    let address = address
        .strip_prefix("0x")
        .or_else(|| address.strip_prefix("0X"))
        .or_else(|| address.strip_prefix('$'))
        .unwrap_or(address);
    let address = hex_u16(address)?;
    Some(if address >= 0x8000 {
        CheatEffect::RomRead { address, value, compare }
    } else {
        CheatEffect::RamWrite { address, value, compare }
    })
}

/// `00AAAAVV`: the first byte is not used by the NES device.
fn parse_par(code: &str) -> Option<CheatEffect> {
    if code.len() != 8 || !code.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let word = u32::from_str_radix(code, 16).ok()?;
    let address = u16::try_from((word >> 8) & 0xFFFF).ok()?;
    let value = u8::try_from(word & 0xFF).ok()?;
    Some(CheatEffect::RamWrite { address, value, compare: None })
}

impl CheatCode {
    /// Reads one code in any format: raw when it holds a `:`, Pro Action Replay for 8 hex
    /// digits, Game Genie for 6 or 8 letters of its alphabet. Spaces and dashes are ignored.
    ///
    /// An 8-character code made only of letters that are both hex digits and Game Genie letters
    /// (`A`, `E` only) is read as Pro Action Replay; such codes do not occur in practice.
    ///
    /// # Errors
    ///
    /// [`CodeError::Empty`] or [`CodeError::Invalid`].
    pub fn parse(text: &str) -> Result<Self, CodeError> {
        let code: String = text.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
        if code.is_empty() {
            return Err(CodeError::Empty);
        }
        if code.contains(':') {
            return parse_raw(&code)
                .map(|effect| Self { effect, format: CodeFormat::Raw })
                .ok_or(CodeError::Invalid(code));
        }
        if let Some(effect) = parse_par(&code) {
            return Ok(Self { effect, format: CodeFormat::ProActionReplay });
        }
        decode_game_genie(&code).map(|effect| Self { effect, format: CodeFormat::GameGenie })
    }

    /// Several codes joined by `+`, `,`, `;` or new lines, as cheat files store a cheat made of
    /// more than one code.
    ///
    /// # Errors
    ///
    /// The first code that does not read.
    pub fn parse_list(text: &str) -> Result<Vec<Self>, CodeError> {
        let codes: Vec<Self> = text
            .split(['+', ',', ';', '\n', '\r'])
            .filter(|part| !part.trim().is_empty())
            .map(Self::parse)
            .collect::<Result<_, _>>()?;
        if codes.is_empty() {
            Err(CodeError::Empty)
        } else {
            Ok(codes)
        }
    }
}

impl fmt::Display for CheatCode {
    /// The code in the format it was typed in.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.format, self.effect) {
            (CodeFormat::GameGenie, CheatEffect::RomRead { address, value, compare }) => {
                f.write_str(&encode_game_genie(address, value, compare))
            }
            (CodeFormat::ProActionReplay, CheatEffect::RamWrite { address, value, compare: None }) => {
                write!(f, "00{address:04X}{value:02X}")
            }
            (_, CheatEffect::RomRead { address, value, compare } | CheatEffect::RamWrite { address, value, compare }) => {
                match compare {
                    Some(compare) => write!(f, "{address:04X}?{compare:02X}:{value:02X}"),
                    None => write!(f, "{address:04X}:{value:02X}"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gossip_is_the_wiki_example() {
        assert_eq!(
            decode_game_genie("GOSSIP"),
            Ok(CheatEffect::RomRead { address: 0xD1DD, value: 0x14, compare: None })
        );
    }

    #[test]
    fn game_genie_round_trips() {
        for address in (0x8000u16..=0xFFFF).step_by(37) {
            for value in (0u8..=255).step_by(17) {
                for compare in [None, Some(value ^ 0x5A)] {
                    let text = encode_game_genie(address, value, compare);
                    assert_eq!(
                        decode_game_genie(&text),
                        Ok(CheatEffect::RomRead { address, value, compare }),
                        "{text}"
                    );
                }
            }
        }
    }

    #[test]
    fn formats() {
        let raw = CheatCode::parse("050F:80").ok();
        assert_eq!(raw.map(|c| c.effect), Some(CheatEffect::RamWrite { address: 0x050F, value: 0x80, compare: None }));
        let cmp = CheatCode::parse("075A?03:09").ok();
        assert_eq!(cmp.map(|c| c.effect), Some(CheatEffect::RamWrite { address: 0x075A, value: 0x09, compare: Some(3) }));
        let mesen = CheatCode::parse("C1C6:20:00").ok();
        assert_eq!(mesen.map(|c| c.effect), Some(CheatEffect::RomRead { address: 0xC1C6, value: 0x20, compare: Some(0) }));
        let par = CheatCode::parse("00075A09").ok();
        assert_eq!(par.map(|c| c.effect), Some(CheatEffect::RamWrite { address: 0x075A, value: 0x09, compare: None }));
        let gg = CheatCode::parse("sxkez-pvg").ok();
        assert!(matches!(gg.map(|c| c.effect), Some(CheatEffect::RomRead { compare: Some(_), .. })));
        assert_eq!(gg.map(|c| c.to_string()), Some("SXKEZPVG".to_owned()));
        assert!(CheatCode::parse("HELLO").is_err());
        assert_eq!(CheatCode::parse_list("GOSSIP+050F:80").map(|l| l.len()), Ok(2));
    }
}
