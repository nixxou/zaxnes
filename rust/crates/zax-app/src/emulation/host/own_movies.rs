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
    /// The order matters and is the only order that is right: the session is rebuilt on that
    /// engine first (so the profile's `Start` hook has run and the 3D scene is the game's), and
    /// the movie is loaded second — its load re-powers the machine, which is what a movie needs
    /// and what wipes anything a `Start` hook wrote. A refusal leaves the machine as the rebuild
    /// left it, which is the game at power-on on the forced engine, and says which thing the
    /// movie needed.
    pub(super) fn play_fm2_on_its_own_engine(&mut self, path: &str) {
        let movie = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.notifications.show_error(&format!(
                    "{}: {error}",
                    text("Movie refused", "Film refusé")
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

        // The engine the movie needs, in force for this session only. Set before the rebuild,
        // because `ensure_session` reads it.
        self.movie_engine = Some(EngineKind::Fceux);
        let already = self
            .session
            .as_ref()
            .is_some_and(|session| session.core().kind() == EngineKind::Fceux);
        if !already {
            if let Err(message) = self.rebuild_session_for_the_movie(&game) {
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
        let played = session.core_mut().play_own_movie(&movie);
        match played {
            Some(Ok(header)) => {
                self.movie_file = Some(PathBuf::from(path));
                self.start_own_movie_timeline(header.frames);
                self.rollback.clear();
                self.frame_steps.clear();
                self.log(&format!(
                    "fm2 movie on the {} engine: {path}, {} frames, {:?}, {} re-records",
                    EngineKind::Fceux,
                    header.frames,
                    header.region,
                    header.rerecord_count,
                ));
                // A movie that states no checksum claims nothing, so there is nothing to warn
                // about; one that states the wrong one was refused before it got here.
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
                            "{message} — {}",
                            text(
                                "it starts from a savestate it carries, not from power-on",
                                "il démarre d'une sauvegarde d'état qu'il contient, pas de l'allumage"
                            )
                        ),
                        DEFAULT_SECONDS,
                        NotificationSeverity::Warning,
                    );
                } else {
                    self.notify(&message);
                }
            }
            Some(Err(why)) => {
                self.log(&format!("fm2 movie refused: {why}"));
                self.notifications
                    .show_error(&format!("{}: {why}", text("Movie refused", "Film refusé")));
                // The engine stays forced only while a movie plays; a refusal is not a playback.
                // The game is left where the rebuild left it — at power-on on that engine — which
                // the next load puts back to its own choice.
                self.movie_engine = None;
            }
            None => {
                // The engine turned out not to play its own movies after all, which only a
                // fallback to Zaxnes can cause (one FCEUX per process). Say so rather than
                // silently importing the movie on the other engine.
                self.movie_engine = None;
                self.log("fm2 movie: the engine that plays its own movies could not be made");
                self.notifications.show_error(&text(
                    "The FCEUX engine is not available, so this movie cannot be played in sync.",
                    "Le moteur FCEUX n'est pas disponible, ce film ne peut pas être lu en phase.",
                ));
            }
        }
    }

    /// Rebuilds the session of `game` on the engine [`EmulationHost::movie_engine`] asks for,
    /// with the game where it is otherwise: the same profile, read again, and its `Start` hook
    /// run. Modelled on `script_reload`, without the state restore a movie's own power-on makes
    /// pointless.
    fn rebuild_session_for_the_movie(
        &mut self,
        game: &crate::emulation::game_catalog::ResolvedGame,
    ) -> Result<(), String> {
        // The ROM is read from its file rather than carried over in a savestate: a state belongs
        // to the engine that wrote it and is refused by the other one, which is the whole point
        // of the engine stamp lot FCEUX-23 step 5 put in the block. `script_reload` can carry a
        // state across because the engine there does not change.
        let rom = self.read_rom_of(game)?;

        if let Some(mut old) = self.session.take() {
            old.end(&mut self.settings, &mut self.scene);
            // `dispose` takes the session by value, so the old engine is **freed here**. That is
            // what makes the next line able to make the other engine at all: FCEUX keeps one
            // machine per process, and `EngineHandle::of_kind` falls back to Zaxnes's core with a
            // line in the log when a second is refused — which would play the movie on the engine
            // it is not in sync on.
            old.dispose(&mut self.settings, &mut self.scene);
        }
        self.ensure_session(game, true)?;

        self.load_rom_into_new_session(game, &rom)?;
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
