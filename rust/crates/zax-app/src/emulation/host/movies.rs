//! Movies (Rust-only, owner request): recording the game being played from its current state,
//! playing a recorded one back, and notes written on the frame on screen. While recording,
//! loading a save state (or rolling back) takes the movie back to that state's frame: its inputs
//! from there are dropped and recording goes on (see `zax_nes::movie::MovieRun`). Movies go to
//! the `movies` folder of the user data, `<game> <date>.zmv`; the notes written while one is
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

    /// The movie files of the movies folder, newest first, at most [`LISTED_MOVIES`].
    #[must_use]
    pub fn movie_files(&self) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(&self.paths.movies_directory) else {
            return Vec::new();
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "zmv"))
            .map(|path| {
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
        let title: String = self
            .current_game_title()
            .unwrap_or("movie")
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || " -_()".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let stamp = save_state_file::local_date_time((self.now)().unix_timestamp())
            .replace([':', ' '], "-");
        unique(
            &self.paths.movies_directory,
            &format!("{} {stamp}", title.trim()),
        )
    }

    fn notify(&mut self, message: &str) {
        self.notifications.show_normal(message);
    }
}

/// `<directory>/<stem>.zmv`, numbered when it exists already.
fn unique(directory: &Path, stem: &str) -> PathBuf {
    let mut path = directory.join(format!("{stem}.zmv"));
    let mut number = 2;
    while path.exists() {
        path = directory.join(format!("{stem} ({number}).zmv"));
        number += 1;
    }
    path
}

/// `english`, or `french` when the interface is in French.
fn text(english: &str, french: &str) -> String {
    if strings::language() == UiLanguage::French {
        french.to_owned()
    } else {
        english.to_owned()
    }
}
