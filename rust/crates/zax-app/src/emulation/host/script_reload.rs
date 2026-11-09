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
        old.dispose(&mut self.settings, &mut self.scene);

        if let Err(message) = self.ensure_session(&game, true) {
            self.log(&format!("script reload failed: {message}"));
            self.notifications.show_error(&message);
            self.current_game = None;
            self.set_state(EmulationState::Stopped);
            return;
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
        let has_movie = run.is_some();
        session.put_movie_run(run);
        let new_drives_the_core =
            !session.core_follows_the_inputs() || session.scripts_change_the_core();
        self.apply_profile_scene();
        if let Some(session) = self.session.as_mut() {
            session.start(&mut self.settings, &mut self.scene);
        }
        self.rollback.clear();
        self.frame_steps.clear();

        let drives_the_core = old_drives_the_core || new_drives_the_core;
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
