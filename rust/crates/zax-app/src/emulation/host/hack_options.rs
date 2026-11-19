//! The options of the built-in patch of the game being played (Rust-only, owner request): kept in
//! `emu.hack_options` as `<patch>/<option>=0|1` entries separated by `;`, only for the options the
//! player changed, and told to the core every frame.

use zax_nes::builtin_patches::HackOption;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

use super::{EmulationHost, GameView};
use crate::emulation::game_catalog::GameCatalog;

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// The options of the built-in patch of the game being played, each with whether it is on.
    #[must_use]
    pub fn hack_options(&self) -> Vec<(HackOption, bool)> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        let core = session.core();
        let Some(patch) = core.options_patch() else {
            return Vec::new();
        };
        let stored = self.settings.get(SettingId::EmuHackOptions);
        core.hack_options()
            .iter()
            .map(|option| (*option, is_on(stored.as_text(), patch, option)))
            .collect()
    }

    /// The option bits for the options as they are (the low byte: `$500A`).
    #[must_use]
    pub fn hack_option_bits(&self) -> u16 {
        // No aid while a `.bk2` plays: its hooks would change the game it replays.
        if self.plain_for_bk2() {
            return 0;
        }
        self.hack_options()
            .iter()
            .filter(|(_, on)| *on)
            .fold(0, |bits, (option, _)| bits | (1 << (option.bit & 15)))
    }

    /// Turns option `index` of the game's patch the other way.
    pub(super) fn toggle_hack_option(&mut self, index: i32) {
        let Some(patch) = self
            .session
            .as_ref()
            .and_then(|session| session.core().options_patch())
        else {
            return;
        };
        let options = self.hack_options();
        let Some((option, on)) = usize::try_from(index)
            .ok()
            .and_then(|index| options.get(index))
        else {
            return;
        };
        let text = with_option(
            self.settings.get(SettingId::EmuHackOptions).as_text(),
            patch,
            option.key,
            !on,
        );
        self.update_settings(|model| {
            model.set(SettingId::EmuHackOptions, SettingValue::Text(text))
        });
    }
}

fn entry_key(patch: &str, key: &str) -> String {
    format!("{patch}/{key}")
}

/// The patch name of the defaults common to every hack pack (lot REWRITE-UI, owner,
/// 2026-11-18): `*/no_slowdown=1` is the default of every pack's `no_slowdown` the player did
/// not set for that pack.
const COMMON: &str = "*";

pub use zax_nes::builtin_patches::COMMON_OPTIONS;

/// The stored value of `key` of `patch`, when the player set it.
fn stored_value(stored: &str, patch: &str, key: &str) -> Option<bool> {
    let wanted = entry_key(patch, key);
    stored
        .split(';')
        .filter_map(|entry| entry.split_once('='))
        .find(|(name, _)| *name == wanted)
        .map(|(_, value)| value.trim() == "1")
}

/// Whether `option` of `patch` is on in the stored text: the player's choice for that patch,
/// else the common default of the options window, else the pack's own default.
pub(super) fn is_on(stored: &str, patch: &str, option: &HackOption) -> bool {
    stored_value(stored, patch, option.key)
        .or_else(|| stored_value(stored, COMMON, option.key))
        .unwrap_or(option.default)
}

/// The common default of `key` in the stored text, when one was set.
#[must_use]
pub fn common_value(stored: &str, key: &str) -> Option<bool> {
    stored_value(stored, COMMON, key)
}

/// The stored text with the common default of `key` set to `on`.
#[must_use]
pub fn with_common(stored: &str, key: &str, on: bool) -> String {
    with_option(stored, COMMON, key, on)
}

/// The stored text without any common default: each pack back to its own.
#[must_use]
pub fn without_common(stored: &str) -> String {
    let prefix = format!("{COMMON}/");
    stored
        .split(';')
        .filter(|entry| !entry.is_empty() && !entry.starts_with(&prefix))
        .collect::<Vec<_>>()
        .join(";")
}

/// The stored text with `key` of `patch` set to `on`.
pub(super) fn with_option(stored: &str, patch: &str, key: &str, on: bool) -> String {
    let wanted = entry_key(patch, key);
    let mut entries: Vec<String> = stored
        .split(';')
        .filter(|entry| !entry.is_empty())
        .filter(|entry| entry.split_once('=').is_none_or(|(name, _)| name != wanted))
        .map(str::to_owned)
        .collect();
    entries.push(format!("{wanted}={}", u8::from(on)));
    entries.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTION: HackOption = HackOption {
        key: "enemies",
        label: "Enemies",
        label_fr: "Ennemis",
        bit: 0,
        default: true,
    };

    #[test]
    fn options_are_stored_per_patch() {
        assert!(is_on("", "P", &OPTION), "the default");
        let stored = with_option("", "P", "enemies", false);
        assert_eq!(stored, "P/enemies=0");
        assert!(!is_on(&stored, "P", &OPTION));
        assert!(is_on(&stored, "Q", &OPTION), "another patch");
        let stored = with_option(&stored, "P", "enemies", true);
        assert_eq!(stored, "P/enemies=1");
        assert!(is_on(&stored, "P", &OPTION));
    }

    #[test]
    fn a_common_default_stands_where_the_pack_was_not_set() {
        let stored = with_common("", "enemies", false);
        assert!(!is_on(&stored, "P", &OPTION), "the common default");
        let stored = with_option(&stored, "P", "enemies", true);
        assert!(is_on(&stored, "P", &OPTION), "the pack's own choice first");
        assert!(!is_on(&stored, "Q", &OPTION));
        assert_eq!(common_value(&stored, "enemies"), Some(false));
        let stored = without_common(&stored);
        assert_eq!(stored, "P/enemies=1");
        assert!(is_on(&stored, "Q", &OPTION), "the pack's default again");
    }
}
