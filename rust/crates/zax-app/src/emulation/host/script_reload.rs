//! The script reload of EDITOR-1 (`tools.reload_script`, F5): the profile of the game being
//! played is read again from its file and a new session is made of it, while the game stays
//! where it is.
//!
//! * **Fast** (the default): the core state of the frame on screen goes into the new session,
//!   whose `Start` hook runs there. Instant, for what does not accumulate (a camera offset, a
//!   colour, a block raised).
//! * **Exact** (with a movie only): the new session goes back to the movie's start and runs every
//!   frame with its scripts up to the frame on screen, through the seek of the movie timeline, so
//!   that what the scripts smooth or accumulate is as if they had always been the new ones.
//!
//! The guard of the design note: when the old or the new scripts change the core (`WriteMem`,
//! `SetInputType`) or drive it (`InputProcess` and the other members of `CORE_MEMBERS`), the
//! core of a frame depends on the scripts, so a fast reload would show a core the new scripts
//! never made: with a movie the reload is then exact, and the checkpoints taken with the old
//! scripts are forgotten.

use super::{EmulationHost, GameView};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::game_catalog::GameCatalog;
use crate::game_session::GameSession;
use crate::persistence::app_settings::AppSettings as _;
use crate::ui::strings::{self, UiLanguage};

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// Reloads the profile's scripts, `exact` or fast (see the module).
    pub(super) fn reload_script(&mut self, exact: bool) {
        self.complete_frame_in_flight();
        let Some(game_id) = self.current_game_id().map(str::to_owned) else {
            return;
        };
        // The editor mode: a profile made from a project is built again from it first.
        if self.settings.editor_mode() {
            if let Some(Err(message)) = self.games.rebuild_from_project(&game_id) {
                self.log(&format!(
                    "script reload: the project does not build: {message}"
                ));
                self.notifications.show_error(&message);
                return;
            }
        }
        self.games.forget_profile(&game_id);
        let Some(game) = self.games.resolve(&game_id) else {
            return;
        };
        let own_movie = self.own_movie_playing();
        let Ok(rom) = self.rom_for_reload(&game) else {
            return;
        };
        let Some(mut old) = self.session.take() else {
            return;
        };
        let old_drives_the_core = !old.core_follows_the_inputs() || old.scripts_change_the_core();
        let state = old.core().save_state(true);
        let run = old.take_movie_run();
        let frame_count = old.core().frame_count();
        let ports = [old.core().port_type(0), old.core().port_type(1)];
        let margin = old.core().widescreen_margin();
        let Ok(state) = state else {
            self.session = Some(old);
            return;
        };
        old.end(&mut self.settings, &mut self.scene);
        // `dispose` takes the session **by value**, so the old engine is freed here and not at
        // the end of this function. That is what makes a reload work on an engine keeping one
        // machine per process (lot FCEUX-48 step 4): `ensure_session` below would otherwise find
        // the old one alive and fall back to Zaxnes's core with a line in the log, silently
        // changing engines under an author editing a profile during an `.fm2` playback.
        old.dispose(&mut self.settings, &mut self.scene);

        if let Err(message) = self.ensure_session(&game, true) {
            self.log(&format!("script reload failed: {message}"));
            self.notifications.show_error(&message);
            self.current_game = None;
            self.set_state(EmulationState::Stopped);
            return;
        }
        if let Some(rom) = rom.as_ref() {
            if let Err(message) = self.load_rom_into_new_session(&game, rom) {
                self.log(&format!("script reload: the ROM does not load: {message}"));
                self.notifications.show_error(&message);
                return;
            }
        }
        let builtin_patches = self.builtin_patches_wanted();
        let advanced = self.advanced_patches_wanted();
        let forced = self.forced_patch();
        let Some(session) = self.session.as_mut() else {
            return;
        };
        session.core_mut().set_builtin_patches(builtin_patches);
        session.core_mut().set_advanced_patches(advanced);
        session.core_mut().set_forced_patch(forced);
        if let Err(error) = session.restore_core(&state) {
            self.log(&format!(
                "script reload: the core state does not load: {error}"
            ));
            return;
        }
        for (port, kind) in ports.into_iter().enumerate() {
            if let Ok(kind) = kind {
                let _ = session.core_mut().set_port_type(port, kind);
            }
        }
        session.core_mut().set_widescreen_margin(margin);
        let mut has_movie = run.is_some();
        session.put_movie_run(run);
        let new_drives_the_core =
            !session.core_follows_the_inputs() || session.scripts_change_the_core();
        // A movie the engine played itself: the state restore above brought the machine back but
        // not the movie, FCEUX's block being the machine and not the movie mode. It is loaded
        // again, which re-powers the machine and puts its clock at 0 — so the reload is **always
        // exact** for one of these, the frame on screen being reachable only by replaying.
        let own_movie_again = own_movie && self.replay_own_movie_after_rebuild().is_some();
        has_movie = has_movie || own_movie_again;
        if own_movie && !own_movie_again {
            self.movie_timeline = None;
            self.movie_file = None;
            self.notifications.show_error(&text(
                "The movie could not be put back after the reload.",
                "Le film n'a pas pu être remis en place après le rechargement.",
            ));
        }
        self.apply_profile_scene();
        if let Some(session) = self.session.as_mut() {
            session.start(&mut self.settings, &mut self.scene);
        }
        self.rollback.clear();
        self.frame_steps.clear();
        self.finish_reload(
            exact || own_movie_again,
            old_drives_the_core || new_drives_the_core,
            has_movie,
            frame_count,
        );
    }

    /// The ROM to put back into the session a reload makes, when the engine's savestate does not
    /// carry the cartridge — every engine but Zaxnes's own (lot FCEUX-48 step 4). `None` means
    /// the state carries it, as it always did; `Err` means the file could not be read, which is
    /// reported here and leaves the game alone.
    fn rom_for_reload(
        &mut self,
        game: &crate::emulation::game_catalog::ResolvedGame,
    ) -> Result<Option<crate::emulation::rom_file::RomData>, ()> {
        let carries_its_rom = self
            .session
            .as_ref()
            .is_some_and(|session| session.core().kind() == zax_core_api::EngineKind::Zaxnes);
        if carries_its_rom {
            return Ok(None);
        }
        match self.read_rom_of(game) {
            Ok(rom) => Ok(Some(rom)),
            Err(message) => {
                self.log(&format!("script reload: the ROM cannot be read: {message}"));
                self.notifications.show_error(&message);
                Err(())
            }
        }
    }

    /// The last act of a reload: whether it is exact, what that does to the timeline, and what to
    /// say about it. Its own function so that `reload_script` stays inside clippy's line budget.
    ///
    /// `exact` is what the caller asked for **or** what a movie the engine plays itself forces
    /// (lot FCEUX-48 step 4): that movie was loaded again by the rebuild, which puts its clock at
    /// 0, so the frame the author was on is reachable only by replaying to it.
    fn finish_reload(
        &mut self,
        exact: bool,
        drives_the_core: bool,
        has_movie: bool,
        frame_count: i64,
    ) {
        let exact = has_movie && (exact || drives_the_core);
        if let Some(timeline) = self.movie_timeline.as_mut() {
            if drives_the_core {
                timeline.forget_checkpoints();
            }
            if exact {
                timeline.seek_exactly(frame_count);
            }
        }
        if drives_the_core {
            self.movie_explorer = None;
        }
        self.log(&format!(
            "scripts reloaded ({}) at frame {frame_count}",
            if exact { "exact" } else { "fast" }
        ));
        let message = match (exact, drives_the_core && has_movie) {
            (true, true) => text(
                "Scripts reloaded: exact (they change the core), replaying from the movie's start",
                "Scripts rechargés : exact (ils modifient le cœur), on rejoue depuis le début du film",
            ),
            (true, false) => text(
                "Scripts reloaded: exact, replaying from the movie's start",
                "Scripts rechargés : exact, on rejoue depuis le début du film",
            ),
            (false, _) => text("Scripts reloaded", "Scripts rechargés"),
        };
        self.notify(&message);
    }

    /// Whether the session of the game being played drives or changes the core from its scripts,
    /// in which case only an exact reload is right (see the module).
    #[must_use]
    pub fn scripts_drive_the_core(&self) -> bool {
        self.session.as_ref().is_some_and(|session: &GameSession| {
            !session.core_follows_the_inputs() || session.scripts_change_the_core()
        })
    }
}

/// `english`, or `french` when the interface is in French.
fn text(english: &str, french: &str) -> String {
    if strings::language() == UiLanguage::French {
        french.to_owned()
    } else {
        english.to_owned()
    }
}
