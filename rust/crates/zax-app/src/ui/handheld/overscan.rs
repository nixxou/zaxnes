//! The overscan's own screen (Rust-only, owner request), opened from a row of the options: there
//! the sides cut for the NTSC games and for the PAL ones, and the mode (Fit or Resize); from a
//! game's options its four sides ("Global": its region's) and its mode.

use zax_settings::catalogue::{self, SettingId};
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::pad::PadButton;
use crate::ui::text;

/// The rows from the options.
pub const GLOBAL_ROWS: [SettingId; 9] = [
    SettingId::VideoOverscanNtscTop,
    SettingId::VideoOverscanNtscBottom,
    SettingId::VideoOverscanNtscLeft,
    SettingId::VideoOverscanNtscRight,
    SettingId::VideoOverscanPalTop,
    SettingId::VideoOverscanPalBottom,
    SettingId::VideoOverscanPalLeft,
    SettingId::VideoOverscanPalRight,
    SettingId::VideoOverscanMode,
];

/// The rows from a game's options.
pub const GAME_ROWS: [SettingId; 5] = [
    SettingId::VideoOverscanTop,
    SettingId::VideoOverscanBottom,
    SettingId::VideoOverscanLeft,
    SettingId::VideoOverscanRight,
    SettingId::VideoOverscanMode,
];

/// What the screen asks of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverscanCommand {
    /// A value changed.
    Changed,
    /// Back to the options.
    Close,
}

/// The rows and the selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OverscanModel {
    /// A game's four sides rather than the NTSC and PAL ones.
    game: bool,
    selected: usize,
}

impl OverscanModel {
    /// The screen on its first row, for a game (`game`) or for the options.
    #[must_use]
    pub const fn new(game: bool) -> Self {
        Self { game, selected: 0 }
    }

    /// Whether the rows are a game's.
    #[must_use]
    pub const fn is_game(&self) -> bool {
        self.game
    }

    /// The rows.
    #[must_use]
    pub const fn rows(&self) -> &'static [SettingId] {
        if self.game {
            &GAME_ROWS
        } else {
            &GLOBAL_ROWS
        }
    }

    /// The index of the selected row.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// One press of a button; `step` steps a setting by its value.
    pub fn handle(
        &mut self,
        button: PadButton,
        mut step: impl FnMut(SettingId, i32),
    ) -> Option<OverscanCommand> {
        let count = self.rows().len();
        let id = self.rows()[self.selected];
        match button {
            PadButton::Up => self.selected = (self.selected + count - 1) % count,
            PadButton::Down => self.selected = (self.selected + 1) % count,
            PadButton::Left => {
                step(id, -1);
                return Some(OverscanCommand::Changed);
            }
            PadButton::Right | PadButton::A => {
                step(id, 1);
                return Some(OverscanCommand::Changed);
            }
            PadButton::B | PadButton::Menu => return Some(OverscanCommand::Close),
            _ => {}
        }
        None
    }
}

/// The values `id` takes, in order.
fn values(id: SettingId) -> Vec<i32> {
    catalogue::descriptor(id)
        .domain()
        .options()
        .unwrap_or_default()
        .iter()
        .map(|option| {
            #[allow(clippy::cast_possible_truncation)] // -1 to 32.
            let value = option.value as i32;
            value
        })
        .collect()
}

/// Steps `id` by `step` in `settings`, stopping at the first and last values.
pub fn step(id: SettingId, settings: &mut SettingsModel, step: i32) {
    let values = values(id);
    let current = settings.get(id).as_int();
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    let last = values.len().saturating_sub(1);
    let next = if step < 0 {
        index.saturating_sub(1)
    } else {
        (index + 1).min(last)
    };
    if let Some(value) = values.get(next) {
        settings.set(id, SettingValue::Enum(*value));
    }
}

/// The label of the row `id`.
#[must_use]
pub fn label(id: SettingId) -> String {
    text::label_of(catalogue::descriptor(id))
}

/// The value of `id` in `settings` as the screen shows it.
#[must_use]
pub fn value_text(id: SettingId, settings: &SettingsModel) -> String {
    let current = f64::from(settings.get(id).as_int());
    catalogue::descriptor(id)
        .domain()
        .options()
        .unwrap_or_default()
        .iter()
        .find(|option| (option.value - current).abs() < f64::EPSILON)
        .map_or_else(String::new, |option| text::label_of_option(&option.label))
}

/// The overscan in one line, for the row of the options that opens the screen: a game's own
/// sides (top, bottom, left, right, "G" for its region's) when it has any, else the NTSC and PAL
/// ones, then the mode.
#[must_use]
pub fn summary(settings: &SettingsModel) -> String {
    let sides = |ids: &[SettingId]| {
        ids.iter()
            .map(|id| match settings.get(*id).as_int() {
                -1 => "G".to_owned(),
                pixels => pixels.to_string(),
            })
            .collect::<Vec<_>>()
            .join("/")
    };
    let mode = value_text(SettingId::VideoOverscanMode, settings);
    let own = &GAME_ROWS[..4];
    if own.iter().any(|id| settings.get(*id).as_int() >= 0) {
        format!("{} · {mode}", sides(own))
    } else {
        format!(
            "NTSC {} · PAL {} · {mode}",
            sides(&GLOBAL_ROWS[..4]),
            sides(&GLOBAL_ROWS[4..8])
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_side_steps_from_global_to_32_pixels_and_stops_there() {
        let mut settings = SettingsModel::new();
        let id = SettingId::VideoOverscanTop;
        assert_eq!(-1, settings.get(id).as_int(), "Global by default");
        step(id, &mut settings, -1);
        assert_eq!(-1, settings.get(id).as_int());
        step(id, &mut settings, 1);
        assert_eq!(0, settings.get(id).as_int());
        for _ in 0..40 {
            step(id, &mut settings, 1);
        }
        assert_eq!(32, settings.get(id).as_int());
        assert_eq!("32 px", value_text(id, &settings));
    }

    #[test]
    fn the_screen_steps_the_selected_row_and_goes_back_with_b() {
        let mut settings = SettingsModel::new();
        let mut model = OverscanModel::new(false);
        model.handle(PadButton::Up, |_, _| {});
        assert_eq!(
            SettingId::VideoOverscanMode,
            GLOBAL_ROWS[model.selected_index()]
        );
        model.handle(PadButton::Left, |id, by| step(id, &mut settings, by));
        assert_eq!("Fit", value_text(SettingId::VideoOverscanMode, &settings));
        assert_eq!(
            Some(OverscanCommand::Close),
            model.handle(PadButton::B, |_, _| {})
        );
    }

    #[test]
    fn the_summary_shows_a_games_own_sides() {
        let mut settings = SettingsModel::new();
        assert_eq!("NTSC 0/0/0/0 · PAL 0/0/0/0 · Resize", summary(&settings));
        settings.set(SettingId::VideoOverscanTop, SettingValue::Enum(8));
        assert_eq!("8/G/G/G · Resize", summary(&settings));
    }
}
