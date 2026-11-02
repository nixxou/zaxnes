//! Movies (Rust-only, owner request): recording the game being played from its current state,
//! playing a recorded one back, and notes written on the frame on screen. While recording,
//! loading a save state (or rolling back) takes the movie back to that state's frame: its inputs
//! from there are dropped and recording goes on (see `zax_nes::movie::MovieRun`). Movies go to
//! the `movies` folder of the user data, named after the ROM (owner request): `<rom>.mv<n>`, the
//! next number; only the movies of the game chosen are listed. The notes written while one is
//! played are written back to its file.

use std::path::{Path, PathBuf};

use zax_nes::movie::fm2::Fm2Movie;
use zax_nes::movie::{Movie, MovieMode};
use zax_nes::NesRegion;

use crate::emulation::movie_timeline::{MovieTimeline, WARM_FRAMES};
use crate::input::InputAction;

use super::{EmulationHost, GameView};
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::notification_center::{NotificationSeverity, DEFAULT_SECONDS};
use crate::game_session::GameSession;
use crate::persistence::save_state_file;
use crate::ui::strings::{self, UiLanguage};

/// The movies listed in the menu, newest first.
pub const LISTED_MOVIES: usize = 15;

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// The movie the session is recording or playing, if any.
    #[must_use]
    pub fn movie_mode(&self) -> Option<MovieMode> {
        self.session
            .as_ref()
            .and_then(GameSession::movie)
            .map(zax_nes::movie::MovieRun::mode)
    }

    /// Starts recording from the current state, or stops and saves the recording.
    pub(super) fn toggle_movie_recording(&mut self) {
        self.complete_frame_in_flight();
        // Recording again while a movie plays takes the game over from the frame on screen.
        let recording = self.movie_mode() == Some(MovieMode::Recording);
        self.finish_movie();
        if recording {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            return;
        };
        match session.start_movie_recording() {
            Ok(()) => self.notify(&text("Recording a movie", "Enregistrement d'un film")),
            Err(error) => {
                self.notifications.show_error(&error);
            }
        }
    }

    /// Plays the movie file `path` from its start state.
    pub(super) fn play_movie(&mut self, path: &str) {
        self.complete_frame_in_flight();
        self.finish_movie();
        if is_fm2(Path::new(path)) {
            self.play_fm2(path);
            return;
        }
        let Some(movie) = std::fs::read(path)
            .ok()
            .and_then(|bytes| Movie::read(&bytes))
        else {
            self.notifications.show_error(&text(
                "Not a movie file.",
                "Ce n'est pas un fichier de film.",
            ));
            return;
        };
        let wanted_options = serde_json::from_str::<serde_json::Value>(&movie.header)
            .ok()
            .and_then(|header| header.get("hackOptions")?.as_u64());
        let Some(session) = self.session.as_mut() else {
            return;
        };
        match session.start_movie_playback(movie) {
            Ok(()) => {
                self.movie_file = Some(PathBuf::from(path));
                self.start_movie_timeline();
                self.rollback.clear();
                self.frame_steps.clear();
                if wanted_options.is_some_and(|bits| bits != u64::from(self.hack_option_bits())) {
                    self.notifications.show(
                        &text(
                            "Playing a movie recorded with other hack options",
                            "Lecture d'un film enregistré avec d'autres options du hack",
                        ),
                        DEFAULT_SECONDS,
                        NotificationSeverity::Warning,
                    );
                } else {
                    self.notify(&text("Playing a movie", "Lecture d'un film"));
                }
            }
            Err(error) => {
                self.log(&format!("movie refused: {error}"));
                self.notifications
                    .show_error(save_state_file::CORRUPTED_MESSAGE);
            }
        }
    }

    /// Plays the `.fm2` movie `path` (FCEUX's text format): the game is reset as on a power
    /// cycle (`End`, the core powered on, `Start`) and the movie plays from there. A region or
    /// ROM that differs from the movie's is warned about, not refused: the movie may desync.
    fn play_fm2(&mut self, path: &str) {
        let parsed = std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| Fm2Movie::parse(&bytes).map_err(|error| error.to_string()));
        let fm2 = match parsed {
            Ok(fm2) => fm2,
            Err(error) => {
                self.log(&format!("fm2 movie refused: {error}"));
                self.notifications.show_error(&format!(
                    "{}: {error}",
                    text("Movie refused", "Film refusé")
                ));
                return;
            }
        };
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut warnings = Vec::new();
        let core = session.core();
        let pal = matches!(core.region(), NesRegion::Pal | NesRegion::Dendy);
        if fm2.header.pal != pal {
            warnings.push(text(
                "the movie was made for another region",
                "le film a été fait pour une autre région",
            ));
        }
        let rom = core
            .cartridge()
            .map(|cartridge| (cartridge.rom().prg_rom(), cartridge.rom().chr_rom()));
        if rom.and_then(|(prg, chr)| fm2.rom_data_matches(prg, chr)) == Some(false) {
            warnings.push(text(
                "the movie was made with another ROM",
                "le film a été fait avec une autre ROM",
            ));
        }
        let resets = fm2.resets();
        if let Some(&(frame, _)) = resets.first() {
            warnings.push(format!(
                "{} {frame}",
                text(
                    "the movie's resets are not replayed, the first at frame",
                    "les resets du film ne sont pas rejoués, le premier à la frame"
                )
            ));
        }
        session.reset(&mut self.settings, &mut self.scene);
        let Ok(start_state) = session.core().save_state(true) else {
            return;
        };
        let header = serde_json::json!({
            "fm2": Path::new(path).file_name().map(|name| name.to_string_lossy()),
            "romFilename": fm2.header.rom_filename,
            "rerecordCount": fm2.header.rerecord_count,
        });
        let movie = Movie {
            header: header.to_string(),
            start_state,
            inputs: fm2.words(),
            annotations: Vec::new(),
        };
        let frames = movie.inputs.len();
        if let Err(error) = session.start_movie_playback(movie) {
            self.log(&format!("fm2 movie refused: {error}"));
            return;
        }
        self.movie_file = Some(PathBuf::from(path));
        self.start_movie_timeline();
        self.rollback.clear();
        self.frame_steps.clear();
        self.log(&format!("fm2 movie: {path}, {frames} frames"));
        if warnings.is_empty() {
            self.notify(&text("Playing a movie", "Lecture d'un film"));
        } else {
            for warning in &warnings {
                self.log(&format!("fm2 movie: {warning}"));
            }
            self.notifications.show(
                &warnings.join("; "),
                DEFAULT_SECONDS,
                NotificationSeverity::Warning,
            );
        }
    }

    /// The timeline of the movie that has just started playing: its start state is the first
    /// checkpoint (EDITOR-1).
    fn start_movie_timeline(&mut self) {
        self.movie_timeline = self.session.as_ref().and_then(|session| {
            let movie = session.movie()?;
            let core = session.core();
            let start_frame = core.frame_count();
            let state = core.save_state(false).ok()?;
            Some(MovieTimeline::new(
                start_frame,
                u32::try_from(movie.movie().inputs.len()).unwrap_or(u32::MAX),
                state,
                session.core_follows_the_inputs(),
            ))
        });
    }

    /// Drops the timeline, the seek under way with it (the sound given back).
    fn end_movie_timeline(&mut self) {
        let seeking = self
            .movie_timeline
            .take()
            .is_some_and(|timeline| timeline.seek_target().is_some());
        if seeking {
            self.end_seek_sound();
        }
    }

    /// Goes to the movie frame `frame` (`tools.seek_movie`): the seek runs over the next ticks.
    pub(super) fn seek_movie(&mut self, frame: i32) {
        if let Some(timeline) = self.movie_timeline.as_mut() {
            timeline.seek(u32::try_from(frame).unwrap_or(0));
        }
    }

    /// The movie frame on screen and the movie's length, while a movie plays with a timeline.
    #[must_use]
    pub fn movie_position(&self) -> Option<(u32, u32)> {
        let timeline = self.movie_timeline.as_ref()?;
        let frame_count = self.session.as_ref()?.core().frame_count();
        Some((timeline.movie_frame(frame_count), timeline.length()))
    }

    /// The seek under way, as the movie frame it is heading for.
    #[must_use]
    pub fn movie_seek_target(&self) -> Option<u32> {
        let timeline = self.movie_timeline.as_ref()?;
        timeline
            .seek_target()
            .map(|target| timeline.movie_frame(target))
    }

    /// The furthest movie frame that has a checkpoint, for the bar.
    #[must_use]
    pub fn movie_explored(&self) -> Option<u32> {
        let timeline = self.movie_timeline.as_ref()?;
        Some(timeline.movie_frame(timeline.furthest_checkpoint()))
    }

    /// Once a tick, before the frames: advances the seek under way for at most
    /// [`SEEK_BUDGET`] — from a checkpoint, the core alone up to [`WARM_FRAMES`] before the
    /// target when the scripts leave the core alone, then whole frames (input hook, conversion,
    /// frame hooks). The sound is off meanwhile. Returns whether a seek took the tick.
    pub(super) fn advance_movie_seek(&mut self) -> bool {
        let Some(target) = self
            .movie_timeline
            .as_ref()
            .and_then(MovieTimeline::seek_target)
        else {
            return false;
        };
        self.complete_frame_in_flight();
        let (Some(session), Some(timeline)) = (self.session.as_mut(), self.movie_timeline.as_mut())
        else {
            return false;
        };
        session
            .core_mut()
            .bus_mut()
            .apu_mut()
            .set_output_enabled(false);
        let current = session.core().frame_count();
        if let Some((_, state)) = timeline.seek_origin(current, target) {
            if let Err(error) = session.restore_core(state) {
                timeline.end_seek();
                self.end_seek_sound();
                self.log(&format!("movie seek failed: {error}"));
                return false;
            }
        }
        session.rewind_movie();
        let deadline = std::time::Instant::now() + SEEK_BUDGET;
        loop {
            let (Some(session), Some(timeline)) =
                (self.session.as_mut(), self.movie_timeline.as_mut())
            else {
                return false;
            };
            let frame_count = session.core().frame_count();
            if frame_count >= target {
                break;
            }
            if timeline.wants(frame_count) {
                if let Ok(state) = session.core().save_state(false) {
                    timeline.record(frame_count, state);
                }
            }
            if timeline.core_only_allowed() && frame_count < target - WARM_FRAMES {
                if !session.run_movie_frame_core_only() {
                    break;
                }
            } else {
                self.run_session_frame(InputAction::NONE, InputAction::NONE);
                self.complete_frame_in_flight();
            }
            if std::time::Instant::now() >= deadline {
                return true;
            }
        }
        if let Some(timeline) = self.movie_timeline.as_mut() {
            timeline.end_seek();
        }
        self.end_seek_sound();
        self.rollback.clear();
        self.frame_steps.clear();
        self.pacer.reset();
        true
    }

    /// The sound comes back after a seek, without the samples of the frames it ran.
    fn end_seek_sound(&mut self) {
        if let Some(session) = self.session.as_mut() {
            let apu = session.core_mut().bus_mut().apu_mut();
            apu.set_output_enabled(true);
            apu.drop_pending_samples();
        }
    }

    /// Stops the movie being recorded or played, saving it.
    pub(super) fn stop_movie(&mut self) {
        self.complete_frame_in_flight();
        self.finish_movie();
    }

    /// Writes a note on the frame on screen: `args.text`, or, without one, the game pauses and
    /// the interface asks for it.
    pub(super) fn annotate_movie(&mut self, args: &CommandArgs) {
        let Some(note) = args.text.as_deref() else {
            self.pause();
            self.request_ui(Command::AnnotateMovie, args);
            return;
        };
        let note = note.trim();
        if note.is_empty() {
            return;
        }
        self.complete_frame_in_flight();
        let frame = self
            .session
            .as_mut()
            .and_then(|session| session.annotate_movie(note));
        if let Some(frame) = frame {
            self.log(&format!("movie note at frame {frame}: {note}"));
            self.notify(&format!(
                "{} {frame}",
                text("Note written at frame", "Note écrite à la frame")
            ));
        }
    }

    /// Once a tick: a movie played to its end gives the game back to the player.
    pub(super) fn check_movie_end(&mut self) {
        let finished = self
            .session
            .as_ref()
            .and_then(GameSession::movie)
            .is_some_and(zax_nes::movie::MovieRun::played_out);
        if finished && self.movie_timeline.is_some() {
            // With a timeline (EDITOR-1) the movie stops on its last frame like a video, paused,
            // until it is stopped; a catch-up may have run a frame past the end, the seek brings
            // it back.
            if let Some(timeline) = self.movie_timeline.as_mut() {
                timeline.seek(timeline.length());
            }
            self.pause();
            self.notify(&text("End of the movie", "Fin du film"));
            return;
        }
        if finished {
            self.finish_movie();
            self.notify(&text("Movie finished", "Film terminé"));
        }
    }

    /// Ends the movie being recorded or played, if any: a recording is saved to a new file, the
    /// notes written on a played one to its file.
    pub(super) fn finish_movie(&mut self) {
        self.end_movie_timeline();
        let mode = self.movie_mode();
        let header = self.movie_header();
        let Some(run) = self.session.as_mut().and_then(|session| {
            let annotated = session.movie()?.annotated();
            session.stop_movie().map(|movie| (movie, annotated))
        }) else {
            return;
        };
        let (mut movie, annotated) = run;
        let played_file = self.movie_file.take();
        let file = match mode {
            Some(MovieMode::Recording) => {
                movie.header = header;
                self.new_movie_file()
            }
            _ => match played_file {
                // An `.fm2` is never written over: its notes go to a new Zaxnes movie.
                Some(file) if annotated && is_fm2(&file) => self.new_movie_file(),
                Some(file) if annotated => file,
                _ => {
                    self.notify(&text("Movie stopped", "Film arrêté"));
                    return;
                }
            },
        };
        let written = std::fs::create_dir_all(&self.paths.movies_directory)
            .and_then(|()| std::fs::write(&file, movie.write()));
        match written {
            Ok(()) => {
                let name = file
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.log(&format!("movie saved: {}", file.display()));
                self.notify(&format!(
                    "{} {name}",
                    text("Movie saved:", "Film enregistré :")
                ));
            }
            Err(error) => {
                self.notifications.show_error(&error.to_string());
            }
        }
    }

    /// The movie files of the game chosen, newest first, at most [`LISTED_MOVIES`]; none
    /// without a game (owner request).
    #[must_use]
    pub fn movie_files(&self) -> Vec<PathBuf> {
        let Some(rom) = self.slots.rom() else {
            return Vec::new();
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> =
            movies_of(&self.paths.movies_directory, rom)
                .into_iter()
                .map(|(_, path)| {
                    let modified = std::fs::metadata(&path)
                        .and_then(|metadata| metadata.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    (modified, path)
                })
                .collect();
        files.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        files
            .into_iter()
            .take(LISTED_MOVIES)
            .map(|(_, path)| path)
            .collect()
    }

    /// Renames the movies of `game_id` named as they were before (`<game> <date>.zmv`, the game
    /// named in their header) after its ROM `rom`, oldest first.
    pub(super) fn rename_legacy_movies(&mut self, game_id: &str, rom: &str) {
        let Some(title) = self.games.title(game_id) else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(&self.paths.movies_directory) else {
            return;
        };
        let mut legacy: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "zmv"))
            .filter(|path| {
                std::fs::read(path)
                    .ok()
                    .and_then(|bytes| Movie::read(&bytes))
                    .and_then(|movie| serde_json::from_str::<serde_json::Value>(&movie.header).ok())
                    .is_some_and(|header| {
                        header.get("game").and_then(|game| game.as_str()) == Some(title.as_str())
                    })
            })
            .map(|path| {
                let modified = std::fs::metadata(&path)
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                (modified, path)
            })
            .collect();
        legacy.sort_by_key(|entry| entry.0);
        for (_, path) in legacy {
            let target = next_movie_file(&self.paths, rom);
            if std::fs::rename(&path, &target).is_err() {
                self.log(&format!(
                    "the movie {} could not be renamed after the ROM.",
                    path.display()
                ));
            }
        }
    }

    fn movie_header(&self) -> String {
        let saved_at = (self.now)().unix_timestamp();
        let patch = self
            .session
            .as_ref()
            .and_then(|session| session.core().original_image())
            .and_then(zax_nes::builtin_patches::patch_for);
        serde_json::json!({
            "game": self.current_game_title().unwrap_or_default(),
            "romSha1": self.current_rom_fingerprint().unwrap_or_default(),
            "hackName": patch.map(|patch| patch.name),
            "hackVersion": patch.map(|patch| format!("{:08X}", patch.version())),
            "hackOptions": self.hack_option_bits(),
            // The hack reads the margin ($5002): where enemies spawn depends on it.
            "widescreenMargin": self.session.as_ref().map(|session| session.core().widescreen_margin()),
            "appVersion": env!("CARGO_PKG_VERSION"),
            "savedAtLocal": save_state_file::local_date_time(saved_at),
        })
        .to_string()
    }

    fn new_movie_file(&self) -> PathBuf {
        let rom = self
            .slots
            .rom()
            .map_or_else(|| "movie".to_owned(), str::to_owned);
        next_movie_file(&self.paths, &rom)
    }

    fn notify(&mut self, message: &str) {
        self.notifications.show_normal(message);
    }
}

/// The movies of the ROM `rom` in `directory` (`<rom>.mv<n>`), with their number.
fn movies_of(directory: &Path, rom: &str) -> Vec<(u32, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let prefix = format!("{rom}.mv");
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = name.strip_prefix(&prefix)?.parse::<u32>().ok()?;
            Some((number, entry.path()))
        })
        .collect()
}

/// The file of the next movie of the ROM `rom`: `<rom>.mv<n>`, one past the highest number.
fn next_movie_file(paths: &crate::user_data_paths::UserDataPaths, rom: &str) -> PathBuf {
    let next = movies_of(&paths.movies_directory, rom)
        .iter()
        .map(|(number, _)| *number)
        .max()
        .unwrap_or(0)
        + 1;
    paths.movie_file(rom, next)
}

/// `english`, or `french` when the interface is in French.
fn text(english: &str, french: &str) -> String {
    if strings::language() == UiLanguage::French {
        french.to_owned()
    } else {
        english.to_owned()
    }
}

/// The time a seek may take from one tick, so that the window stays responsive while it runs
/// through thousands of frames.
const SEEK_BUDGET: std::time::Duration = std::time::Duration::from_millis(12);

/// Whether `path` names an `.fm2` movie (FCEUX's text format), by its extension.
fn is_fm2(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("fm2"))
}
