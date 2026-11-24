//! Cheat codes and a RAM search for the NES, with no engine dependency.
//!
//! - [`code`]: reading and writing Game Genie, Pro Action Replay and raw codes.
//! - [`search`]: the RAM search that finds the address of a value.
//! - [`import`]: the cheat files of Mesen, RetroArch and FCEUX.
//! - [`CheatList`]: a game's cheats, what the engines apply each frame, and its file.
//!
//! See `docs/progress/CHEATS.md` for the survey of other emulators and the plan.

pub mod code;
pub mod import;
pub mod search;

use serde::{Deserialize, Serialize};

pub use code::{CheatCode, CheatEffect, CodeError, CodeFormat};

/// One cheat as the player sees it: a description and one or more codes, on or off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cheat {
    /// What it does; may hold a note on a second line.
    pub description: String,
    /// The codes, all applied together.
    pub codes: Vec<CheatCode>,
    /// Whether it is applied.
    pub enabled: bool,
}

/// A ROM patch the CPU read path applies: what Game Genie codes become.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RomPatch {
    /// CPU address, `$8000-$FFFF`.
    pub address: u16,
    /// Byte returned.
    pub value: u8,
    /// Byte the cartridge must hold for the patch to apply.
    pub compare: Option<u8>,
}

impl RomPatch {
    /// The byte the CPU reads at `address` when the cartridge holds `original` there, if this
    /// patch applies at that address.
    #[must_use]
    pub fn apply(&self, address: u16, original: u8) -> Option<u8> {
        (address == self.address && self.compare.is_none_or(|compare| compare == original))
            .then_some(self.value)
    }
}

/// A game's cheats.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheatList {
    /// In the order the player sees them.
    pub cheats: Vec<Cheat>,
}

/// The file form of one cheat: codes as text, so the file stays readable and editable.
#[derive(Serialize, Deserialize)]
struct StoredCheat {
    description: String,
    codes: String,
    enabled: bool,
}

#[derive(Serialize, Deserialize)]
struct StoredList {
    version: u32,
    cheats: Vec<StoredCheat>,
}

fn enabled_codes(cheats: &[Cheat]) -> impl Iterator<Item = &CheatCode> {
    cheats.iter().filter(|cheat| cheat.enabled).flat_map(|cheat| &cheat.codes)
}

impl CheatList {
    /// Whether any cheat is on.
    #[must_use]
    pub fn any_enabled(&self) -> bool {
        self.cheats.iter().any(|cheat| cheat.enabled && !cheat.codes.is_empty())
    }

    /// The ROM patches of the cheats that are on, for the engine's read path; empty when none
    /// is on, which is what lets an engine skip the test entirely.
    #[must_use]
    pub fn rom_patches(&self) -> Vec<RomPatch> {
        enabled_codes(&self.cheats)
            .filter_map(|code| match code.effect {
                CheatEffect::RomRead { address, value, compare } => {
                    Some(RomPatch { address, value, compare })
                }
                CheatEffect::RamWrite { .. } => None,
            })
            .collect()
    }

    /// Applies the RAM writes of the cheats that are on, through the engine's memory access;
    /// called once a frame, after the frame (so the next one starts with the values set).
    pub fn apply_ram_writes(&self, read: impl Fn(u16) -> u8, mut write: impl FnMut(u16, u8)) {
        for code in enabled_codes(&self.cheats) {
            if let CheatEffect::RamWrite { address, value, compare } = code.effect {
                if compare.is_none_or(|compare| read(address) == compare) {
                    write(address, value);
                }
            }
        }
    }

    /// The list as Zaxnes's cheat file (JSON).
    ///
    /// # Errors
    ///
    /// Only if serialisation fails, which it does not for these types.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let stored = StoredList {
            version: 1,
            cheats: self
                .cheats
                .iter()
                .map(|cheat| StoredCheat {
                    description: cheat.description.clone(),
                    codes: cheat.codes.iter().map(ToString::to_string).collect::<Vec<_>>().join("+"),
                    enabled: cheat.enabled,
                })
                .collect(),
        };
        serde_json::to_string_pretty(&stored)
    }

    /// Reads Zaxnes's cheat file; cheats whose codes do not read are reported, not dropped
    /// silently.
    ///
    /// # Errors
    ///
    /// The JSON error when the file is not one.
    pub fn from_json(text: &str) -> Result<import::Imported, serde_json::Error> {
        let stored: StoredList = serde_json::from_str(text)?;
        let mut imported = import::Imported::default();
        for cheat in stored.cheats {
            match CheatCode::parse_list(&cheat.codes) {
                Ok(codes) => imported.cheats.push(Cheat {
                    description: cheat.description,
                    codes,
                    enabled: cheat.enabled,
                }),
                Err(error) => imported.skipped.push((cheat.description, error)),
            }
        }
        Ok(imported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn list() -> CheatList {
        let cheat = |text: &str, enabled| Cheat {
            description: text.to_owned(),
            codes: CheatCode::parse_list(text).unwrap_or_default(),
            enabled,
        };
        CheatList {
            cheats: vec![cheat("GOSSIP+0010:05", true), cheat("0011?01:07", true), cheat("0012:09", false)],
        }
    }

    #[test]
    fn rom_patches_and_ram_writes() {
        let list = list();
        let patches = list.rom_patches();
        assert_eq!(patches, [RomPatch { address: 0xD1DD, value: 0x14, compare: None }]);
        assert_eq!(patches[0].apply(0xD1DD, 0x99), Some(0x14));
        assert_eq!(patches[0].apply(0xD1DE, 0x99), None);
        let ram = RefCell::new([0u8; 0x20]);
        list.apply_ram_writes(|a| ram.borrow()[usize::from(a)], |a, v| ram.borrow_mut()[usize::from(a)] = v);
        assert_eq!(ram.borrow()[0x10..0x13], [5, 0, 0]); // 0x11 waits for its compare value
        ram.borrow_mut()[0x11] = 1;
        list.apply_ram_writes(|a| ram.borrow()[usize::from(a)], |a, v| ram.borrow_mut()[usize::from(a)] = v);
        assert_eq!(ram.borrow()[0x11], 7);
    }

    #[test]
    fn json_round_trip() {
        let list = list();
        let text = list.to_json().unwrap_or_default();
        let back = CheatList::from_json(&text).map(|i| i.cheats).unwrap_or_default();
        assert_eq!(back, list.cheats);
    }
}
