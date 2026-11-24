//! Reading the cheat files of other emulators.
//!
//! | Format | Who writes it | Layout |
//! |---|---|---|
//! | Mesen JSON | Mesen, Mesen2, BeesNES's built-in database | `{"Cheats":[{"Description","Type":"NesGameGenie"/"NesProActionReplay"/"NesCustom","Enabled","Codes"}]}` |
//! | libretro `.cht` | RetroArch, libretro-database `cht/` | `cheats = N`, then `cheatI_desc`, `cheatI_code`, `cheatI_enable` |
//! | FCEUX `.cht` | FCEUX | one line per cheat: `[S][C][:]AAAA:VV[:CC]:Name`, `:` before the address when off, `C` when a compare value follows, `S` for a substitute cheat |
//!
//! A cheat whose codes do not read is skipped and reported, never guessed.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::code::{CheatCode, CheatEffect, CodeError, CodeFormat};
use crate::Cheat;

/// What an import read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Imported {
    /// The cheats read.
    pub cheats: Vec<Cheat>,
    /// The cheats skipped, with the reason.
    pub skipped: Vec<(String, CodeError)>,
}

impl Imported {
    fn push(&mut self, description: String, codes: Result<Vec<CheatCode>, CodeError>, enabled: bool) {
        match codes {
            Ok(codes) => self.cheats.push(Cheat { description, codes, enabled }),
            Err(error) => self.skipped.push((description, error)),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MesenFile {
    #[serde(default)]
    cheats: Vec<MesenCheat>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MesenCheat {
    #[serde(default)]
    description: String,
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    codes: String,
}

/// A Mesen description may carry a note in `{{...}}`; it is kept, on its own line.
fn mesen_description(text: &str) -> String {
    match text.split_once("{{") {
        Some((name, rest)) => {
            let note = rest.trim_end_matches("}}");
            format!("{}\n{}", name.trim(), note.trim())
        }
        None => text.trim().to_owned(),
    }
}

/// Doubles the backslashes that do not start a JSON escape: some hand-edited files of the
/// BeesNES database hold a stray `\R` in a note.
fn repair_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '\\' {
            match chars.peek() {
                Some('"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u') => {
                    if let Some(next) = chars.next() {
                        out.push(next);
                    }
                }
                _ => out.push('\\'),
            }
        }
    }
    out
}

/// Reads a Mesen cheat file (JSON). A file with stray backslashes in its text is read after
/// they are escaped.
///
/// # Errors
///
/// The JSON error when the file is not Mesen's JSON at all.
pub fn mesen_json(text: &str) -> Result<Imported, serde_json::Error> {
    let file: MesenFile = match serde_json::from_str(text) {
        Ok(file) => file,
        Err(error) => serde_json::from_str(&repair_escapes(text)).map_err(|_| error)?,
    };
    let mut imported = Imported::default();
    for cheat in file.cheats {
        let codes = CheatCode::parse_list(&cheat.codes).and_then(|codes| {
            // the type says which format the codes are in; check it agrees with what was read
            let expected = match cheat.r#type.as_str() {
                "NesGameGenie" => Some(CodeFormat::GameGenie),
                "NesProActionReplay" => Some(CodeFormat::ProActionReplay),
                "NesCustom" => Some(CodeFormat::Raw),
                _ => None,
            };
            match expected {
                Some(format) if codes.iter().any(|code| code.format != format) => {
                    Err(CodeError::Invalid(cheat.codes.clone()))
                }
                _ => Ok(codes),
            }
        });
        imported.push(mesen_description(&cheat.description), codes, cheat.enabled);
    }
    Ok(imported)
}

/// Reads a libretro `.cht` file. Lines that are not `key = value` are ignored.
#[must_use]
pub fn libretro_cht(text: &str) -> Imported {
    let mut keys = BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim();
            let value = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(value);
            keys.insert(key.trim().to_owned(), value.to_owned());
        }
    }
    let count: usize = keys.get("cheats").and_then(|n| n.parse().ok()).unwrap_or(0);
    let mut imported = Imported::default();
    for index in 0..count {
        let get = |field: &str| keys.get(&format!("cheat{index}_{field}"));
        let description = get("desc").cloned().unwrap_or_else(|| format!("Cheat {index}"));
        let enabled = get("enable").is_some_and(|v| v == "true");
        let codes = get("code").map_or(Err(CodeError::Empty), |code| CheatCode::parse_list(code));
        imported.push(description, codes, enabled);
    }
    imported
}

/// Reads an FCEUX `.cht` file.
#[must_use]
pub fn fceux_cht(text: &str) -> Imported {
    let mut imported = Imported::default();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let mut rest = line;
        let substitute = rest.starts_with('S');
        if substitute {
            rest = &rest[1..];
        }
        let compared = rest.starts_with('C');
        if compared {
            rest = &rest[1..];
        }
        let enabled = !rest.starts_with(':');
        if !enabled {
            rest = &rest[1..];
        }
        let fields = if compared { 4 } else { 3 };
        let parts: Vec<&str> = rest.splitn(fields, ':').collect();
        let name = parts.last().map_or("", |name| name.trim()).to_owned();
        let parsed = (parts.len() == fields)
            .then(|| {
                let address = u16::from_str_radix(parts[0], 16).ok()?;
                let value = u8::from_str_radix(parts[1], 16).ok()?;
                let compare = if compared { Some(u8::from_str_radix(parts[2], 16).ok()?) } else { None };
                // a substitute cheat replaces what the CPU reads, which only differs from a
                // write for ROM; a plain one is written every frame
                let effect = if substitute && address >= 0x8000 {
                    CheatEffect::RomRead { address, value, compare }
                } else {
                    CheatEffect::RamWrite { address, value, compare }
                };
                Some(vec![CheatCode { effect, format: CodeFormat::Raw }])
            })
            .flatten()
            .ok_or_else(|| CodeError::Invalid(line.to_owned()));
        imported.push(name, parsed, enabled);
    }
    imported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesen() {
        let text = r#"{"Cheats":[
            {"Description":"Balloons{{Use caution.}}","Type":"NesCustom","Enabled":false,"Codes":"050F:80"},
            {"Description":"No damage","Type":"NesGameGenie","Enabled":true,"Codes":"SXKEZPVG"},
            {"Description":"Wrong type","Type":"NesGameGenie","Enabled":false,"Codes":"050F:80"}]}"#;
        let imported = mesen_json(text).ok().unwrap_or_default();
        assert_eq!(imported.cheats.len(), 2);
        assert_eq!(imported.cheats[0].description, "Balloons\nUse caution.");
        assert!(imported.cheats[1].enabled);
        assert_eq!(imported.skipped.len(), 1);
        let stray = r#"{"Cheats":[{"Description":"a\Rb","Type":"NesCustom","Codes":"0001:02"}]}"#;
        let imported = mesen_json(stray).ok().unwrap_or_default();
        assert_eq!(imported.cheats[0].description, "a\\Rb");
    }

    #[test]
    fn libretro() {
        let text = "cheats = 2\n\ncheat0_desc = \"Infinite lives\"\ncheat0_code = \"SXIOPO\"\ncheat0_enable = false\n\
                    cheat1_desc = \"Two\"\ncheat1_code = \"GOSSIP+AAAAAA\"\ncheat1_enable = true\n";
        let imported = libretro_cht(text);
        assert_eq!(imported.cheats.len(), 2);
        assert_eq!(imported.cheats[1].codes.len(), 2);
        assert!(imported.cheats[1].enabled);
    }

    #[test]
    fn fceux() {
        let imported = fceux_cht("075a:09:Lives\nC:0100:01:02:Off\nSC8123:EA:4C:Patch\nbad line\n");
        assert_eq!(imported.cheats.len(), 3);
        assert!(imported.cheats[0].enabled);
        assert!(!imported.cheats[1].enabled);
        assert_eq!(
            imported.cheats[1].codes[0].effect,
            CheatEffect::RamWrite { address: 0x100, value: 1, compare: Some(2) }
        );
        assert_eq!(
            imported.cheats[2].codes[0].effect,
            CheatEffect::RomRead { address: 0x8123, value: 0xEA, compare: Some(0x4C) }
        );
        assert_eq!(imported.skipped.len(), 1);
    }
}
