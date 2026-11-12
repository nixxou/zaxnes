//! BizHawk's `.bk2` movies in the application (owner request, 2026-11-12).
//!
//! A `.bk2` only stays in sync on the core that recorded it, and the one BizHawk NES core this
//! build carries is NesHawk's port (`zax-neshawk`): a movie whose header names `NesHawk` switches
//! the **session** to that engine, as an `.fm2` switches it to FCEUX (`own_movies.rs`), and leaves
//! the game's own `emu.engine` alone. A movie of another core (`QuickNes`) or of none plays on the
//! engine in force, with a warning that it may drift.
//!
//! The frames go through Zaxnes's own movie machinery ([`zax_nes::movie::MovieRun`]): the
//! timeline, the seek, the movie bar, the 3D conversion and the profile's hooks all work as they
//! do for a Zaxnes movie. The `.bk2`'s **resets** are the one thing that machinery has no column
//! for, so the session applies them itself ([`crate::game_session::GameSession::set_movie_commands`]),
//! at the frame `BizHawk` applies them: after the flagged frame, before the next.

use std::path::{Path, PathBuf};

use zax_core_api::{EngineKind, NesRegion};
use zax_nes::movie::tas::{FrameCommand, MovieFile};
use zax_nes::movie::Movie;

use super::{EmulationHost, GameView};
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::notification_center::{NotificationSeverity, DEFAULT_SECONDS};
use crate::ui::strings::{self, UiLanguage};

/// Whether `path` names a `.bk2`.
#[must_use]
pub fn is_bk2(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("bk2"))
}

/// The machine commands of `movie`, each at the movie frame it is applied **before**, in order.
///
/// A `.bk2`'s Reset column means "after this frame" (`MovieFile::reset_after_frame`), so its
/// reset lands before the next frame. A power cycle on the very first frame is how a movie says
/// it starts from power-on, which the playback does anyway.
#[must_use]
pub fn movie_commands(movie: &MovieFile) -> Vec<(u32, FrameCommand)> {
    let shift = u32::from(movie.reset_after_frame);
    movie
        .frames
        .iter()
        .enumerate()
        .filter(|(index, frame)| {
            frame.command != FrameCommand::None
                && !(*index == 0 && frame.command == FrameCommand::HardReset)
        })
        .filter_map(|(index, frame)| {
            u32::try_from(index)
                .ok()
                .map(|index| (index + shift, frame.command))
        })
        .collect()
}

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// Plays the `.bk2` in `path` from power-on, on the engine that recorded it when this build
    /// has it.
    pub(super) fn play_bk2(&mut self, path: &str) {
        let Some(movie) = self.read_bk2(path) else {
            return;
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
        let mut warnings = Vec::new();
        let wanted = movie.recorded_engine();
        if wanted.is_none() {
            let core = movie
                .header
                .get("Core")
                .map_or_else(|| text("no core", "aucun cœur"), Clone::clone);
            warnings.push(format!(
                "{} ({core})",
                text(
                    "the movie was recorded on a core this build has not got and may drift",
                    "le film a été enregistré sur un cœur absent de cette version et peut se désynchroniser"
                )
            ));
        }

        // The engine the movie needs, for this session only; the session is rebuilt on it.
        let current = self.session.as_ref().map(|session| session.core().kind());
        if let Some(engine) = wanted.filter(|engine| current != Some(*engine)) {
            self.movie_engine = Some(engine);
            if let Err(message) = self.rebuild_session_for_the_movie(&game, &rom) {
                self.movie_engine = None;
                self.log(&format!(
                    "bk2 movie: the engine could not be made: {message}"
                ));
                self.notifications.show_error(&message);
                return;
            }
        }

        let Some(session) = self.session.as_mut() else {
            return;
        };
        if let Some(expected) = movie.rom_sha1 {
            let data = rom.image.get(16..).unwrap_or_default();
            if zax_nes::movie::bk2::sha1(&rom.image) != expected
                && zax_nes::movie::bk2::sha1(data) != expected
            {
                warnings.push(text(
                    "the movie names another ROM",
                    "le film désigne une autre ROM",
                ));
            }
        }
        // The devices of the two ports before the power-on, which keeps them.
        for (port, kind) in movie.ports.into_iter().enumerate() {
            let _ = session.core_mut().set_port_type(port, kind);
        }
        session.reset(&mut self.settings, &mut self.scene);
        if movie.pal_stated {
            let pal = matches!(session.core().region(), NesRegion::Pal | NesRegion::Dendy);
            if pal != movie.pal {
                warnings.push(text(
                    "the movie was made for another region",
                    "le film a été fait pour une autre région",
                ));
            }
        }
        let Ok(start_state) = session.core().save_state(true) else {
            return;
        };
        let header = serde_json::json!({
            "bk2": Path::new(path).file_name().map(|name| name.to_string_lossy()),
            "core": movie.header.get("Core"),
            "rerecordCount": movie.header.get("rerecordCount"),
        });
        let inputs: Vec<u16> = movie.frames.iter().map(|frame| frame.word).collect();
        let playback = Movie {
            header: header.to_string(),
            start_state,
            inputs,
            annotations: Vec::new(),
        };
        if let Err(error) = session.start_movie_playback(playback) {
            self.log(&format!("bk2 movie refused: {error}"));
            self.notifications.show_error(&format!(
                "{}: {error}",
                text("Movie refused", "Film refusé")
            ));
            return;
        }
        session.set_movie_commands(movie_commands(&movie));
        let engine = session.core().kind();
        self.movie_file = Some(PathBuf::from(path));
        self.start_movie_timeline();
        self.rollback.clear();
        self.frame_steps.clear();
        self.report_bk2_started(path, &movie, engine, &warnings);
    }

    /// Reads the `.bk2` in `path`; a refusal is reported here.
    fn read_bk2(&mut self, path: &str) -> Option<MovieFile> {
        match std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| zax_nes::movie::bk2::read(&bytes))
        {
            Ok(movie) => Some(movie),
            Err(error) => {
                self.log(&format!("bk2 movie refused: {error}"));
                self.notifications.show_error(&format!(
                    "{}: {error}",
                    text("Movie refused", "Film refusé")
                ));
                None
            }
        }
    }

    /// What the log and the notification say once the movie plays on `engine`.
    fn report_bk2_started(
        &mut self,
        path: &str,
        movie: &MovieFile,
        engine: EngineKind,
        warnings: &[String],
    ) {
        let frames = movie.frames.len();
        self.log(&format!(
            "bk2 movie on the {engine} engine: {path}, {frames} frames"
        ));
        for note in &movie.notes {
            self.log(&format!("bk2 movie: {note}"));
        }
        let message = format!(
            "{} {engine} ({frames} {})",
            text(
                "Playing the movie on the engine",
                "Lecture du film sur le moteur"
            ),
            text("frames", "trames")
        );
        if warnings.is_empty() {
            self.notify(&message);
        } else {
            for warning in warnings {
                self.log(&format!("bk2 movie: {warning}"));
            }
            self.notifications.show(
                &format!("{message} - {}", warnings.join("; ")),
                DEFAULT_SECONDS,
                NotificationSeverity::Warning,
            );
        }
        if movie.recorded_engine() == Some(engine) && engine != EngineKind::Zaxnes {
            // The session stays on that engine; the game's own comes back at the next load, as
            // for an `.fm2` on FCEUX.
            self.log("bk2 movie: the game's own engine comes back at the next load");
        }
    }
}

/// `english` or `french`, whichever the interface speaks.
fn text(english: &str, french: &str) -> String {
    if strings::language() == UiLanguage::French {
        french.to_owned()
    } else {
        english.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zax_nes::movie::tas::FrameInput;

    fn frame(command: FrameCommand) -> FrameInput {
        FrameInput { word: 0, command }
    }

    #[test]
    fn a_bk2_reset_lands_before_the_next_frame_and_a_first_power_is_dropped() {
        let movie = MovieFile {
            kind: "bk2",
            reset_after_frame: true,
            frames: vec![
                frame(FrameCommand::HardReset),
                frame(FrameCommand::None),
                frame(FrameCommand::SoftReset),
                frame(FrameCommand::None),
            ],
            ..MovieFile::default()
        };
        assert_eq!(vec![(3, FrameCommand::SoftReset)], movie_commands(&movie));
        let before = MovieFile {
            reset_after_frame: false,
            ..movie
        };
        assert_eq!(vec![(2, FrameCommand::SoftReset)], movie_commands(&before));
    }

    #[test]
    fn only_the_bk2_extension_is_taken() {
        assert!(is_bk2(Path::new("run.bk2")));
        assert!(is_bk2(Path::new("RUN.BK2")));
        assert!(!is_bk2(Path::new("run.fm2")));
    }
}
