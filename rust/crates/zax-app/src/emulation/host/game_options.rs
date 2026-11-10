//! The options of a game (Rust-only, owner request): the few settings the user set for one ROM
//! file, kept in `games.json` beside the profile chosen for it, put in force while it plays.

use std::path::Path;

use zax_core_api::EngineKind;
use zax_settings::catalogue::SettingId;
use zax_settings::games::GameOptions;

use super::{EmulationHost, GameView};
use crate::emulation::fceux_validated;
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

    /// The engine the game in `game_id` plays on (lot FCEUX-23 step 5): its own option where it
    /// has one, the global default otherwise — both `zaxnes`, and there is no "auto" (§4 of
    /// `docs/review/FCEUX-ENGINE.md`: an automatic choice would need a quality judgement the
    /// application cannot make).
    ///
    /// Read from the **store** rather than from the model, because the launch has to know before
    /// `apply_game_options` has put the game's options in force: that is the moment it decides
    /// whether the session can be kept, and a changed engine means a new machine.
    ///
    /// Always Zaxnes in a build without the `fceux-core` feature, whatever the stored value says
    /// — the setting is then read and written without error and ignored, which is what lets a
    /// `games.json` cross between builds.
    #[must_use]
    pub fn engine_for_game(&self, game_id: &str) -> EngineKind {
        if !cfg!(feature = "fceux-core") {
            return EngineKind::Zaxnes;
        }

        let stored = self
            .games
            .rom_path(game_id)
            .filter(|path| !path.is_empty())
            .map(|path| self.game_options(&path))
            .unwrap_or_default()
            .into_iter()
            .find(|(id, _)| *id == SettingId::EmuEngine)
            .map(|(_, value)| value.as_int());
        let value = stored.unwrap_or_else(|| {
            // The global default, which is the preference with no game layer over it.
            let mut global = self.settings.clone();
            global.clear_session_layer();
            global.clear_game_values();
            global.get(SettingId::EmuEngine).as_int()
        });
        u8::try_from(value)
            .ok()
            .and_then(EngineKind::from_u8)
            .unwrap_or_default()
    }

    /// The engine in force for the game in play, which is the model's value: the one the *next*
    /// load will use, and what the options window greys its rows from.
    #[must_use]
    pub fn chosen_engine(&self) -> EngineKind {
        if !cfg!(feature = "fceux-core") {
            return EngineKind::Zaxnes;
        }
        u8::try_from(self.settings.get(SettingId::EmuEngine).as_int())
            .ok()
            .and_then(EngineKind::from_u8)
            .unwrap_or_default()
    }

    /// The engine the machine in play is **actually** running, for the marker the status line and
    /// the game's options show. `None` with no session.
    #[must_use]
    pub fn running_engine(&self) -> Option<EngineKind> {
        self.session.as_ref().map(|session| session.core().kind())
    }

    /// The CRC-32 of the data of the game whose options are in force — the identity lot FCEUX-48
    /// step 2's table of validated games is keyed by, and the one the built-in patches already
    /// use. Read from the **cache**, which a launch always fills (`certify_rom`, before the job
    /// begins), so that the interface reads no file while it draws a frame; `None` with no game,
    /// or for a ROM the cache has not seen as it is.
    #[must_use]
    pub fn current_rom_data_crc(&self) -> Option<u32> {
        self.known_rom_crc(Path::new(self.current_rom_path()?))
    }

    /// Whether the game in play is one lot FCEUX-6's sweep validated for **3D on FCEUX** (lot
    /// FCEUX-48 step 2). It is a marker and not a gate: the engine option works for every game,
    /// and this is what the engine row, the notification at load and the performance overlay say
    /// about it.
    ///
    /// `Unknown` rather than `NotValidated` when there is no game or the CRC was never read —
    /// "nothing is known" must not be printed as "not validated".
    #[must_use]
    pub fn fceux_validation(&self) -> fceux_validated::Status {
        fceux_validated::status_of(self.current_rom_data_crc())
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
