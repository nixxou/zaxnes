//! `.fm2` playback on the engine that recorded it (lot FCEUX-48 step 4, §5 of
//! `docs/review/FCEUX-ENGINE.md`).
//!
//! The owner's reason for the whole FCEUX engine: **play a complete TAS and edit the 3dn profile
//! on the fly**. A `.fm2` only stays in sync on the engine that recorded it, so opening one plays
//! it on FCEUX — through FCEUX's own movie mode, which is the only way a movie's `RAMInitSeed`
//! can be honoured at all ([`EngineHandle::plays_its_own_movies`] says why).
//!
//! # What this module does, and what it deliberately leaves alone
//!
//! It is a *second* source of movie frames beside [`zax_nes::movie::MovieRun`], not a replacement.
//! Everything around the frames is the machinery EDITOR-1 already built and this module reuses
//! untouched: the [`crate::emulation::movie_timeline::MovieTimeline`] for the checkpoints and the
//! seek, the movie bar, the pause and the frame step, the 3D conversion and the profile's Lua
//! hooks (all of which hang off `run_frame`, which playback does not change), and
//! `script_reload` for the editor.
//!
//! Three things are **not** the same as on Zaxnes's own movie path, each for a stated reason:
//!
//! * **no movie explorer.** It replays the movie in a *second* core, and FCEUX keeps one machine
//!   per process (`movie_explorer.rs` says so itself). A Zaxnes explorer would replay the inputs
//!   on the wrong machine, so the checkpoints come from playback alone;
//! * **no notes.** An annotation belongs to a Zaxnes movie file, and an `.fm2` is never written
//!   over. `Ctrl+M` says so rather than doing nothing;
//! * **no input hook of ours.** FCEUX's `FCEUMOV_AddInputState` loads each frame's record over the
//!   pad state itself, so the loop feeds its usual word and the engine ignores it.
//!
//! # The engine the playback forces
//!
//! Opening an `.fm2` switches the **session's** engine to the one that plays it, and leaves the
//! game's own `emu.engine` alone: the choice is the user's and a movie is not a change of mind.
//! Stopping the movie does **not** switch back, and that is deliberate — §4's rule is that an
//! engine change means re-powering the machine and never happens mid-game. A movie load re-powers
//! anyway, so forcing the engine for it costs nothing; stopping does not, so switching back would
//! throw away the frame the author is looking at. The game goes back to its own engine at the next
//! load, and the message says so.

use std::path::{Path, PathBuf};

use zax_core_api::EngineKind;

use super::{EmulationHost, GameView};
use crate::emulation::engine_handle::{EngineHandle, OwnMovieHeader};
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::movie_timeline::MovieTimeline;
use crate::emulation::notification_center::{NotificationSeverity, DEFAULT_SECONDS};
use crate::emulation::rom_file::{self, INVALID_ROM_MESSAGE};
use crate::ui::strings::{self, UiLanguage};

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// Whether the engine of the session is playing a movie of its own.
    #[must_use]
    pub fn own_movie_playing(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.core().own_movie_position().is_some())
    }

    /// The movie frame on screen and the movie's length, from the engine's own clock. `None`
    /// unless the engine is playing a movie of its own.
    ///
    /// The engine's clock counts past the end, so the frame is clamped to the length here: a
    /// movie bar must not show frame 5 003 of 5 000.
    #[must_use]
    pub fn own_movie_position(&self) -> Option<(u32, u32)> {
        let (frame, length) = self.session.as_ref()?.core().own_movie_position()?;
        Some((frame.min(length), length))
    }

    /// Whether an `.fm2` should be played by the engine rather than imported: only with an engine
    /// that plays its own movies in the build, and only for `.fm2` — FCEUX's own movie code
    /// refuses the FCE Ultra `.fcm` format outright ("please use Tools > Convert FCM"), so a
    /// `.fcm` keeps Zaxnes's clean-room importer.
    pub(super) fn plays_fm2_on_its_own_engine(path: &Path) -> bool {
        cfg!(feature = "fceux-core") && zax_nes::movie::fceu_kind(path) == Some("fm2")
    }

    /// Plays the `.fm2` in `path` on the engine that can replay it.
    ///
    /// Two orders matter here, and both are the only right one.
    ///
    /// **The movie is judged before anything is committed.** Switching the session's engine
    /// re-powers the machine, so a refusal decided afterwards would cost the player the game they
    /// were playing — which is the contract `fm2_movie_tests`'s *a refused fm2 leaves the game
    /// alone* states, and which this path broke until step 7 of the lot found it.
    ///
    /// **The session is rebuilt before the movie is loaded**, so that the profile's `Start` hook
    /// has run and the 3D scene is the game's; the movie's own load then re-powers the machine,
    /// which is what a movie needs and what wipes anything a `Start` hook wrote.
    pub(super) fn play_fm2_on_its_own_engine(&mut self, path: &str) {
        let movie = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.notifications.show_error(&format!(
                    "{}: {error}",
                    text("Movie refused", "Film refuse")
                ));
                return;
            }
        };
        let Some(game_id) = self.current_game_id().map(str::to_owned) else {
            return;
        };
        let Some(game) = self.games.resolve(&game_id) else {
            return;
        };
        let rom = match self.read_rom_of(&game) {
            Ok(rom) => rom,
            Err(message) => {
                self.notifications.show_error(&message);
                return;
            }
        };
        // With that engine already in play there is nothing to lose, so it is asked directly;
        // otherwise the question goes to a throw-away engine of that kind with this game's ROM in
        // it, which also answers whether the ROM loads there at all.
        let already = self
            .session
            .as_ref()
            .is_some_and(|session| session.core().kind() == EngineKind::Fceux);
        if !self.movie_is_for_its_own_engine(&rom, &movie, already, path) {
            return;
        }

        // Only now is anything committed: the engine the movie needs, in force for this session
        // only, set before the rebuild because `ensure_session` reads it.
        self.movie_engine = Some(EngineKind::Fceux);
        if !already {
            if let Err(message) = self.rebuild_session_for_the_movie(&game, &rom) {
                self.movie_engine = None;
                self.log(&format!(
                    "fm2 movie: the engine could not be made: {message}"
                ));
                self.notifications.show_error(&message);
                return;
            }
        }

        let Some(session) = self.session.as_mut() else {
            self.movie_engine = None;
            return;
        };
        match session.core_mut().play_own_movie(&movie) {
            Some(Ok(header)) => {
                self.movie_file = Some(PathBuf::from(path));
                self.start_own_movie_timeline(header.frames);
                self.rollback.clear();
                self.frame_steps.clear();
                self.report_own_movie_started(path, &header);
            }
            other => {
                // A backstop. The judgement above already refused everything the engine would,
                // so reaching here means the two disagreed, which is worth a line rather than a
                // silent difference.
                let why = match other {
                    Some(Err(why)) => why,
                    _ => "the engine would not take the movie".to_owned(),
                };
                self.log(&format!(
                    "fm2 movie refused after the judgement passed: {why}"
                ));
                self.notifications
                    .show_error(&format!("{}: {why}", text("Movie refused", "Film refuse")));
                self.movie_engine = None;
            }
        }
    }

    /// Whether the movie may be played on the engine that plays its own, judged **without
    /// touching the machine**; the refusal or the fall-back is reported here.
    ///
    /// A `false` means the caller must do nothing more: either the movie was refused, or it has
    /// already been handed to the other engine's own reader. The second case is the one worth
    /// naming — a ROM that will not load on that engine is about the **game** and not about the
    /// movie, so refusing would leave a playable movie unplayable; the other reader takes it with
    /// a warning, a reader that is not the one the movie was recorded on being able to drift.
    fn movie_is_for_its_own_engine(
        &mut self,
        rom: &rom_file::RomData,
        movie: &[u8],
        already: bool,
        path: &str,
    ) -> bool {
        let judged = if already {
            self.session
                .as_ref()
                .and_then(|session| session.core().inspect_own_movie(movie))
        } else {
            EngineHandle::judge_own_movie(&rom.image, &rom.name, movie)
        };
        match judged {
            Some(Ok(_)) => true,
            Some(Err(why)) if EngineHandle::judgement_is_about_the_rom(&why) => {
                self.log(&format!(
                    "fm2 movie: the FCEUX engine cannot load this ROM ({why}); Zaxnes's own reader takes the movie"
                ));
                self.notifications.show(
                    &text(
                        "The FCEUX engine cannot load this ROM: the movie plays on Zaxnes's own reader and may drift out of sync.",
                        "Le moteur FCEUX ne peut pas charger cette ROM : le film est lu par le lecteur de Zaxnes et peut se desynchroniser.",
                    ),
                    DEFAULT_SECONDS,
                    NotificationSeverity::Warning,
                );
                self.play_fm2(path);
                false
            }
            Some(Err(why)) => {
                self.log(&format!("fm2 movie refused: {why}"));
                self.notifications
                    .show_error(&format!("{}: {why}", text("Movie refused", "Film refuse")));
                false
            }
            None => {
                self.log("fm2 movie: the engine that plays its own movies could not be made");
                self.notifications.show_error(&text(
                    "The FCEUX engine is not available, so this movie cannot be played in sync.",
                    "Le moteur FCEUX n'est pas disponible, ce film ne peut pas etre lu en phase.",
                ));
                false
            }
        }
    }

    /// What the log and the notification say once the movie is playing. A movie that states no
    /// checksum claims nothing, so there is nothing to warn about there; one that states the wrong
    /// one never reaches this.
    fn report_own_movie_started(&mut self, path: &str, header: &OwnMovieHeader) {
        self.log(&format!(
            "fm2 movie on the {} engine: {path}, {} frames, {:?}, {} re-records",
            EngineKind::Fceux,
            header.frames,
            header.region,
            header.rerecord_count,
        ));
        let message = format!(
            "{} ({} {})",
            text(
                "Playing the movie on the FCEUX engine",
                "Lecture du film sur le moteur FCEUX"
            ),
            header.frames,
            text("frames", "trames")
        );
        if header.from_savestate {
            self.notifications.show(
                &format!(
                    "{message} - {}",
                    text(
                        "it starts from a savestate it carries, not from power-on",
                        "il demarre d'une sauvegarde d'etat qu'il contient, pas de l'allumage"
                    )
                ),
                DEFAULT_SECONDS,
                NotificationSeverity::Warning,
            );
        } else {
            self.notify(&message);
        }
    }

    /// Rebuilds the session of `game` on the engine [`EmulationHost::movie_engine`] asks for,
    /// with the game where it is otherwise: the same profile, read again, and its `Start` hook
    /// run. Modelled on `script_reload`, without the state restore a movie's own power-on makes
    /// pointless.
    pub(super) fn rebuild_session_for_the_movie(
        &mut self,
        game: &crate::emulation::game_catalog::ResolvedGame,
        rom: &rom_file::RomData,
    ) -> Result<(), String> {
        // The ROM is handed in rather than carried over in a savestate: a state belongs to the
        // engine that wrote it and is refused by the other one, which is the whole point of the
        // engine stamp lot FCEUX-23 step 5 put in the block. `script_reload` can carry a state
        // across because the engine there does not change.

        if let Some(mut old) = self.session.take() {
            old.end(&mut self.settings, &mut self.scene);
            // `dispose` takes the session by value, so the old engine is **freed here**. That is
            // what makes the next line able to make the other engine at all: FCEUX keeps one
            // machine per process, and `EngineHandle::of_kind` falls back to Zaxnes's core with a
            // line in the log when a second is refused — which would play the movie on the engine
            // it is not in sync on.
            old.dispose(&mut self.settings, &mut self.scene);
        }
        self.ensure_session(game, true, Some(&rom.image))?;

        self.load_rom_into_new_session(game, rom)?;
        self.apply_profile_scene();
        if let Some(session) = self.session.as_mut() {
            session.start(&mut self.settings, &mut self.scene);
        }
        Ok(())
    }

    /// Puts `rom` into the session just made, as the launch's own stage 4 does and in the same
    /// order: the patch settings, the ROM, the region it reports, the battery.
    ///
    /// **Needed by any engine whose savestate does not carry the cartridge**, which is every
    /// engine but Zaxnes's own: `FceuxEngine::save_state_to` answers `include_rom_image` as "no"
    /// whatever it is asked, FCEUX's block holding the machine and not the cartridge. So a path
    /// that rebuilds a session and restores a state — this one, and `script_reload` — has to
    /// load the ROM again itself rather than letting the state carry it.
    pub(super) fn load_rom_into_new_session(
        &mut self,
        game: &crate::emulation::game_catalog::ResolvedGame,
        rom: &rom_file::RomData,
    ) -> Result<(), String> {
        let builtin_patches = self.builtin_patches_wanted();
        let advanced = self.advanced_patches_wanted();
        let forced = self.forced_patch();
        let result = match self.session.as_mut() {
            Some(session) => {
                session.core_mut().set_builtin_patches(builtin_patches);
                session.core_mut().set_advanced_patches(advanced);
                session.core_mut().set_forced_patch(forced);
                session.load_rom(&rom.image, &rom.name)
            }
            None => return Err(INVALID_ROM_MESSAGE.to_owned()),
        };
        if !result.success {
            return Err(INVALID_ROM_MESSAGE.to_owned());
        }
        self.apply_region_on_load(result.region, true, game);
        self.attach_battery(true);
        Ok(())
    }

    /// The ROM file of `game`, read as a launch reads it.
    pub(super) fn read_rom_of(
        &self,
        game: &crate::emulation::game_catalog::ResolvedGame,
    ) -> Result<rom_file::RomData, String> {
        let path = self
            .games
            .rom_path(&game.id)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| INVALID_ROM_MESSAGE.to_owned())?;
        rom_file::read(Path::new(&path))
    }

    /// Plays the `.fm2` of [`EmulationHost::movie_file`] again on the session just made, for a
    /// path that had to rebuild the session while a movie was playing (`script_reload`).
    ///
    /// Gives the movie's length, or `None` when there was no movie to put back or it could no
    /// longer be played — in which case the caller has a session without a movie and says so.
    pub(super) fn replay_own_movie_after_rebuild(&mut self) -> Option<u32> {
        let path = self.movie_file.clone()?;
        let movie = std::fs::read(&path).ok()?;
        let session = self.session.as_mut()?;
        match session.core_mut().play_own_movie(&movie) {
            Some(Ok(header)) => Some(header.frames),
            Some(Err(why)) => {
                self.log(&format!("fm2 movie could not be put back: {why}"));
                None
            }
            None => None,
        }
    }

    /// The timeline of a movie the engine plays itself: the same one EDITOR-1 built, with the
    /// length from the movie's header and the state at its first frame as the first checkpoint.
    ///
    /// **No explorer**, for the reason in the module docs. `core_only_allowed` is the session's
    /// own answer, as on Zaxnes: it is about the profile's scripts, not about the engine.
    fn start_own_movie_timeline(&mut self, frames: u32) {
        self.movie_explorer = None;
        self.movie_timeline = self.session.as_ref().and_then(|session| {
            let core = session.core();
            Some(MovieTimeline::new(
                core.frame_count(),
                frames,
                core.save_state(false).ok()?,
                session.core_follows_the_inputs(),
            ))
        });
    }

    /// Stops the movie the engine is playing: the machine goes on from the frame it is at, and
    /// the engine stays the one it was until the next load (see the module docs).
    pub(super) fn finish_own_movie(&mut self) {
        let forced = self.movie_engine.take();
        if !self.own_movie_playing() {
            return;
        }
        if let Some(session) = self.session.as_mut() {
            session.core_mut().stop_own_movie();
        }
        self.movie_file = None;
        self.rollback.clear();
        self.frame_steps.clear();
        self.log("fm2 movie stopped");
        if forced.is_some() {
            self.notify(&text(
                "Movie stopped; the game stays on the FCEUX engine until the next load",
                "Film arrêté ; le jeu reste sur le moteur FCEUX jusqu'au prochain chargement",
            ));
        } else {
            self.notify(&text("Movie stopped", "Film arrêté"));
        }
    }

    /// Once a tick: a movie the engine played to its end holds on its last frame, paused, like
    /// the Zaxnes path does with a timeline. Returns whether the engine is playing a movie at
    /// all, so that the caller knows this end was this module's to judge.
    ///
    /// The engine's own clock counts **past** the end, so the test is `frame >= length` and the
    /// seek brings the picture back to the last frame when a catch-up overran it.
    pub(super) fn check_own_movie_end(&mut self) -> bool {
        let Some((frame, length)) = self
            .session
            .as_ref()
            .and_then(|session| session.core().own_movie_position())
        else {
            return false;
        };
        if frame < length {
            return true;
        }
        let Some(timeline) = self.movie_timeline.as_mut() else {
            return true;
        };
        if timeline.seek_target().is_some() {
            return true;
        }
        timeline.seek(timeline.length());
        self.pause();
        self.notify(&text("End of the movie", "Fin du film"));
        true
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
