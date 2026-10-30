//! Movies (Rust-only, owner request): recording the game being played from its current state,
//! playing a recorded one back, and notes written on the frame on screen. While recording,
//! loading a save state (or rolling back) takes the movie back to that state's frame: its inputs
//! from there are dropped and recording goes on (see `zax_nes::movie::MovieRun`). Movies go to
//! the `movies` folder of the user data, named after the ROM (owner request): `<rom>.mv<n>`, the
//! next number; only the movies of the game chosen are listed. The notes written while one is
//! played are written back to its file.

use std::path::{Path, PathBuf};

use zax_nes::movie::{Movie, MovieMode};

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
        if finished {
            self.finish_movie();
            self.notify(&text("Movie finished", "Film terminé"));
        }
    }

    /// Ends the movie being recorded or played, if any: a recording is saved to a new file, the
    /// notes written on a played one to its file.
    pub(super) fn finish_movie(&mut self) {
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
