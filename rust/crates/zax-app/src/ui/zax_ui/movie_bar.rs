//! The movie bar (EDITOR-1, owner request): while a movie plays with its timeline, a bar along
//! the bottom of the window to go anywhere in it, like a video — a slider over the movie's
//! frames, play / pause, a frame back and forward, the start and the end, the frame and the time.
//!
//! It lies over the view, as the menu bar does during a game: shown while the pointer is near the
//! bottom of the window, while the game is paused, while a seek is under way, and while the
//! slider is held. Dragging the slider asks for a seek at every change; the host runs it over the
//! next ticks from the nearest checkpoint (see `emulation::movie_timeline`).

use imgui::{Condition, Ui, WindowFlags};

use super::ZaxUi;
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::emulation_state::EmulationState;
use crate::ui::{keys, strings};

/// The band at the bottom of the window that reveals the bar, in pixels.
const REVEAL_BAND: f32 = 72.0;
/// The height of the bar.
const BAR_HEIGHT: f32 = 40.0;

/// What the bar remembers between frames.
#[derive(Debug, Clone, Default)]
pub struct MovieBar {
    /// Whether the slider is held: the bar stays, and the value shown is the one dragged to.
    dragging: bool,
    /// The frame the slider was last dragged to, so a seek is asked only when it moves.
    dragged_to: Option<u32>,
}

impl ZaxUi {
    /// Draws the movie bar when a movie plays with a timeline and the bar is called for.
    pub(super) fn draw_movie_bar(&mut self, ui: &Ui, host: &mut AppHost) {
        let Some((position, length)) = host.movie_position() else {
            self.movie_bar = MovieBar::default();
            return;
        };
        let seeking = host.movie_seek_target();
        let paused = host.state() == EmulationState::Paused;
        let [width, height] = ui.io().display_size;
        let [x, y] = ui.io().mouse_pos;
        let near = (height - REVEAL_BAND..=height).contains(&y) && (0.0..=width).contains(&x);
        if !(near || paused || seeking.is_some() || self.movie_bar.dragging) {
            return;
        }
        let flags = WindowFlags::NO_DECORATION
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_SAVED_SETTINGS
            | WindowFlags::NO_FOCUS_ON_APPEARING
            | WindowFlags::NO_NAV
            | WindowFlags::NO_SCROLLBAR;
        let window = ui
            .window("##movie_bar")
            .position([0.0, height - BAR_HEIGHT], Condition::Always)
            .size([width, BAR_HEIGHT], Condition::Always)
            .bg_alpha(0.75)
            .flags(flags)
            .begin();
        let Some(_window) = window else {
            return;
        };
        let rate = host
            .session()
            .map_or(60, crate::game_session::GameSession::nominal_frame_rate)
            .max(1);
        let shown = self
            .movie_bar
            .dragged_to
            .filter(|_| self.movie_bar.dragging)
            .or(seeking)
            .unwrap_or(position);

        let mut seek_to = None;
        if ui.button("|<") {
            seek_to = Some(0);
        }
        ui.same_line();
        if ui.button("<") {
            seek_to = Some(shown.saturating_sub(1));
            host.enqueue(Command::Pause, CommandArgs::default());
        }
        ui.same_line();
        let play_label = if paused {
            strings::get(keys::MOVIE_BAR_PLAY)
        } else {
            strings::get(keys::MOVIE_BAR_PAUSE)
        };
        if ui.button(format!("{play_label}##play")) {
            // Play on the last frame starts the movie over.
            if paused && position >= length {
                seek_to = Some(0);
            }
            host.enqueue(Command::PauseResume, CommandArgs::default());
        }
        ui.same_line();
        if ui.button(">") {
            seek_to = Some((shown + 1).min(length));
            host.enqueue(Command::Pause, CommandArgs::default());
        }
        ui.same_line();
        if ui.button(">|") {
            seek_to = Some(length);
        }
        ui.same_line();

        let label = position_label(shown, length, rate, seeking.is_some());
        let label_width = ui.calc_text_size(&label)[0];
        let slider_width = (ui.content_region_avail()[0] - label_width - 12.0).max(80.0);
        ui.set_next_item_width(slider_width);
        let mut value = shown;
        if ui
            .slider_config("##movie_frame", 0, length)
            .display_format("")
            .build(&mut value)
            && self.movie_bar.dragged_to != Some(value)
        {
            self.movie_bar.dragged_to = Some(value);
            seek_to = Some(value);
        }
        self.movie_bar.dragging = ui.is_item_active();
        if !self.movie_bar.dragging {
            self.movie_bar.dragged_to = None;
        }
        ui.same_line();
        ui.text(&label);

        if let Some(frame) = seek_to {
            host.enqueue(
                Command::SeekMovie,
                CommandArgs::of_number(i32::try_from(frame).unwrap_or(i32::MAX)),
            );
        }
    }
}

/// `frame / length  m:ss.cc`, and the frame heading for while a seek runs.
fn position_label(frame: u32, length: u32, rate: i32, seeking: bool) -> String {
    let rate = u32::try_from(rate).unwrap_or(60).max(1);
    let seconds = frame / rate;
    let hundredths = (frame % rate) * 100 / rate;
    let arrow = if seeking {
        format!("{} ", strings::get(keys::MOVIE_BAR_SEEKING))
    } else {
        String::new()
    };
    format!(
        "{arrow}{frame} / {length}  {}:{:02}.{hundredths:02}",
        seconds / 60,
        seconds % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_label_gives_the_frame_and_the_time() {
        assert_eq!(position_label(0, 100, 60, false), "0 / 100  0:00.00");
        assert_eq!(
            position_label(3690, 47454, 60, false),
            "3690 / 47454  1:01.50"
        );
        assert_eq!(position_label(125, 1000, 50, false), "125 / 1000  0:02.50");
    }
}
