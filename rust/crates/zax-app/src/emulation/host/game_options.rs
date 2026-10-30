//! The options of a game (Rust-only, owner request): the few settings the user set for one ROM
//! file, kept in `games.json` beside the profile chosen for it, put in force while it plays.

use zax_settings::games::GameOptions;

use super::{EmulationHost, GameView};
use crate::emulation::game_catalog::GameCatalog;

/// The key of a ROM file in `games.json`: its absolute path, as a launch records it.
#[must_use]
fn game_options_key(rom_path: &str) -> String {
    std::path::absolute(rom_path).map_or_else(
        |_| rom_path.to_owned(),
        |path| path.to_string_lossy().into_owned(),
    )
}

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// The ROM file of the game in play, whose options are in force.
    #[must_use]
    pub fn current_rom_path(&self) -> Option<&str> {
        self.game_options_rom.as_deref()
    }

    /// The options the user set for the game in `rom_path`; empty when none.
    #[must_use]
    pub fn game_options(&self, rom_path: &str) -> GameOptions {
        self.settings_store
            .as_ref()
            .map(|store| {
                store
                    .games()
                    .options_for(&game_options_key(rom_path))
                    .to_vec()
            })
            .unwrap_or_default()
    }

    /// Remembers the options of the game in `rom_path` (none forgets them), in force at once
    /// when that game is the one in play.
    pub fn set_game_options(&mut self, rom_path: &str, options: GameOptions) {
        let key = game_options_key(rom_path);
        if let Some(store) = &mut self.settings_store {
            store.games_mut().set_options_for(&key, options.clone());
            store.schedule_write(self.elapsed);
        }
        if self.game_options_rom.as_deref() == Some(key.as_str()) {
            self.update_settings(|settings| settings.set_game_values(options));
        }
    }

    /// Puts in force the options of the game the job starts, before its session is made: some
    /// of them (the Lua paths) are read only then.
    pub(super) fn apply_game_options(&mut self, game_id: &str) {
        let rom = self
            .games
            .rom_path(game_id)
            .filter(|path| !path.is_empty())
            .map(|path| game_options_key(&path));
        let options = rom
            .as_deref()
            .map(|rom| self.game_options(rom))
            .unwrap_or_default();
        self.game_options_rom = rom;
        self.settings.set_game_values(options);
        self.choose_slots(game_id);
    }

    /// Writes down an option of the game in play the user changed in the options (it went to
    /// the game, not to the preference).
    pub(super) fn keep_game_options_edits(&mut self) {
        if !self.settings.take_game_edited() {
            return;
        }
        let Some(rom) = self.game_options_rom.clone() else {
            return;
        };
        let options: GameOptions = self
            .settings
            .game_values()
            .iter()
            .map(|(id, value)| (*id, value.clone()))
            .collect();
        if let Some(store) = &mut self.settings_store {
            store.games_mut().set_options_for(&rom, options);
            store.schedule_write(self.elapsed);
        }
    }
}
