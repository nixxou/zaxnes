//! The resume point of AMD-053 REQ-LOOP-076: the savestate the normal exit writes, the entry the
//! next launch offers, and the three erasure rules. The store itself is
//! [`crate::persistence::resume_point`].
//!
//! `-noresume` — which `-slot` implies — suspends the whole of it for one run: the point is
//! neither read, nor written, nor erased. The amendment states the first two and leaves its first
//! erasure rule in contradiction with the third; R1 confirmed the reading that makes the option
//! agree with its name (Q-088).

use super::{EmulationHost, GameView};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::game_catalog::GameCatalog;
use crate::persistence::resume_point::{self, ResumePoint};

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// What the start-up does with the resume point (REQ-LOOP-076, « Au lancement suivant » and
    /// the three erasure rules). Called once, when the host is built.
    pub(super) fn prepare_resume_point(&mut self) {
        if self.suspend_resume {
            // Q-088: nothing is read and nothing is touched this run.
            self.resume = None;
            return;
        }

        let found = resume_point::peek(&self.paths);
        if found.is_none() {
            // Rule 3: an unreadable or headerless file is erased, the start-up is normal, no
            // notification is shown, and the fact is journalled.
            if resume_point::file_exists(&self.paths) {
                self.log(
                    "the resume point was discarded: its file could not be read (REQ-LOOP-076).",
                );
                resume_point::clear(&self.paths);
            }
            return;
        }

        // Rule 3 again, with REQ-PERS-055 as §3 point 7 extends it: the game must still be in the
        // library.
        let found = found.filter(|point| self.games.exists(&point.game_id));
        if found.is_none() {
            self.log(
                "the resume point was discarded: its game is no longer in the library \
                 (REQ-PERS-055).",
            );
            resume_point::clear(&self.paths);
        }
        self.resume = found;
    }

    /// Whether the `Resume` entry is offered at this launch (REQ-LOOP-076).
    #[must_use]
    pub const fn resume_available(&self) -> bool {
        self.resume.is_some()
    }

    /// What the resume point holds, when the entry is offered.
    #[must_use]
    pub const fn resume_point(&self) -> Option<&ResumePoint> {
        self.resume.as_ref()
    }

    /// The title the `Resume` entry carries: the game the resume point holds, or none when the
    /// entry is not offered.
    #[must_use]
    pub fn resume_game_title(&self) -> Option<String> {
        let point = self.resume.as_ref()?;
        Some(
            self.games
                .title(&point.game_id)
                .unwrap_or_else(|| point.game_id.clone()),
        )
    }

    /// The thumbnail file of the resume point, or none when there is none to show
    /// (REQ-LOOP-076, « portant sa vignette »).
    #[must_use]
    pub fn resume_thumbnail_file(&self) -> Option<std::path::PathBuf> {
        self.resume
            .as_ref()
            .filter(|point| point.has_thumbnail)
            .map(|_| self.paths.resume_thumbnail_file())
            .filter(|file| file.is_file())
    }

    /// Loads the resume point, exactly as a slot load does (REQ-LOOP-044). It is what the
    /// `resume.last` command runs, and the only path that ever reads it. A file that passed the
    /// start-up check and still fails here falls under REQ-LOOP-046 — the `Corrupted save state.`
    /// notification and the Stopped state — like any other slot.
    ///
    /// Ignoring the entry during a session does not erase it (erasure rule 2): it stays offered
    /// until the next exit, so the point is kept here even once it has been loaded.
    pub fn resume_last_game(&mut self) {
        if self.resume.is_some() {
            self.load_resume_point();
        }
    }

    /// Steps 1 and 2 of the exit (REQ-LOOP-076): the `Save` hook of every active script, then the
    /// capture and the write of the resume savestate — or the erasure of the point when the state
    /// is Stopped.
    ///
    /// Returns whether a savestate was written. The clause of REQ-LOOP-041 asking for the write to
    /// happen « between two emulated frames » is satisfied by construction: no emulated frame runs
    /// any more at this point.
    pub(super) fn write_or_clear_resume_point(&mut self) -> bool {
        if self.suspend_resume {
            // Q-088: -noresume suspends the erasure rule as well, otherwise launching a slot,
            // quitting from Stopped and losing the resume point would be the common case.
            return false;
        }

        // Rule 1, which wins over rule 2: quitting from Stopped erases the point, so ignoring the
        // Resume entry and then quitting without playing does clear it.
        if self.state == EmulationState::Stopped {
            self.clear_resume_point();
            return false;
        }

        // The condition of the requirement, in the vocabulary of SPEC-07: Running or Paused with
        // the loading indicator down. A loading exit writes nothing and erases nothing.
        if self.loading
            || self.current_game.is_none()
            || !self
                .session
                .as_ref()
                .is_some_and(crate::game_session::GameSession::has_rom)
        {
            return false;
        }
        if !self.paths.is_writable {
            return false;
        }

        // The capture taken at the last pause when there is one — it always belongs to the current
        // game, being cleared with it — and a capture of the game view otherwise, which is the
        // usual case of an exit made from Running.
        let picture = self.thumbnail_for_slot();
        let Some(content) = self.capture_save_state(resume_point::SLOT_ID) else {
            return false;
        };
        match resume_point::write(&self.paths, &content, picture.as_deref()) {
            Ok(()) => {
                self.log(&format!(
                    "resume point written for '{}' (REQ-LOOP-076).",
                    content.header.game_id
                ));
                true
            }
            Err(error) => {
                self.log(&format!("the resume point could not be written: {error}"));
                false
            }
        }
    }

    /// Erases the resume point and forgets its entry.
    fn clear_resume_point(&mut self) {
        let had_one = self.resume.take().is_some() || resume_point::file_exists(&self.paths);
        resume_point::clear(&self.paths);
        if had_one {
            self.log(
                "the resume point was cleared: the program exited with no game running \
                 (REQ-LOOP-076).",
            );
        }
    }

    // --------------------------------------------- favourites and search (REQ-UI-210, REQ-UI-211)

    /// Whether the enumeration of the games keeps only the favourites
    /// (`library.filter_favorites`).
    #[must_use]
    pub const fn favorites_only(&self) -> bool {
        self.favorites_only
    }

    /// Ticks or unticks the favourites filter.
    pub fn set_favorites_only(&mut self, favorites_only: bool) {
        self.favorites_only = favorites_only;
    }

    /// The text typed in the search of the enumeration of the games (`library.search`).
    #[must_use]
    pub fn library_search(&self) -> &str {
        &self.library_search
    }

    /// Sets the search text; a blank text excludes nothing (REQ-UI-211).
    pub fn set_library_search(&mut self, text: &str) {
        text.clone_into(&mut self.library_search);
    }

    /// Whether `game_id` is marked as a favourite (REQ-UI-210).
    #[must_use]
    pub fn is_favorite(&self, game_id: &str) -> bool {
        self.games.is_favorite(game_id)
    }

    /// Flips the favourite mark of `game_id` and persists it (`favorite.toggle`); returns its new
    /// value, or none when the game does not exist.
    pub fn toggle_favorite(&mut self, game_id: &str) -> Option<bool> {
        self.games.toggle_favorite(game_id)
    }
}
