//! Sizes of the handheld screens, derived from the height of the display so that 640x480,
//! 720x480 and 720x720 all read the same, and the control hints of each screen.

use super::Screen;
use crate::ui::fonts::FontSizes;
use crate::ui::keys;

/// Body text height, in pixels, on a 480-pixel-high display.
const BODY_AT_480: f32 = 22.0;
/// Smallest row, in pixels: a thumb-sized target on a 3.5 inch screen.
pub const MINIMUM_ROW: f32 = 36.0;

/// The sizes of one display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// The body font, in pixels.
    pub font: f32,
    /// The heading font, in pixels.
    pub heading: f32,
    /// The height of a list row, never under [`MINIMUM_ROW`].
    pub row: f32,
    /// The space around the content.
    pub padding: f32,
}

impl Metrics {
    /// The sizes for a display `height` pixels high.
    #[must_use]
    pub fn for_height(height: f32) -> Self {
        let font = (BODY_AT_480 * height / 480.0)
            .round()
            .max(BODY_AT_480 - 2.0);
        Self {
            font,
            heading: (font * 1.25).round(),
            row: (font * 1.75).round().max(MINIMUM_ROW),
            padding: (font * 0.6).round(),
        }
    }

    /// The font sizes to bake into the atlas.
    #[must_use]
    pub const fn font_sizes(&self) -> FontSizes {
        FontSizes {
            body: self.font,
            heading: self.heading,
        }
    }

    /// The height of the title band.
    #[must_use]
    pub fn title_band(&self) -> f32 {
        self.heading + 2.0 * self.padding
    }

    /// The height of the bar of control hints.
    #[must_use]
    pub fn hint_band(&self) -> f32 {
        self.font + 2.0 * self.padding
    }

    /// How many rows fit between the title and the hints on a display `height` pixels high.
    #[must_use]
    pub fn visible_rows(&self, height: f32) -> usize {
        let room = height - self.title_band() - self.hint_band() - self.padding;
        let mut rows = 0;
        let mut used = self.row;
        while used <= room {
            rows += 1;
            used += self.row;
        }
        // A display too small for one row still shows one.
        rows.max(1)
    }
}

/// A control hint: the button, and the key of its meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hint {
    /// What is printed for the button.
    pub button: &'static str,
    /// The key of the text for what it does.
    pub meaning: &'static str,
}

const fn hint(button: &'static str, meaning: &'static str) -> Hint {
    Hint { button, meaning }
}

const HOME_HINTS: &[Hint] = &[
    hint("A", keys::HANDHELD_HINT_PLAY),
    hint("X", keys::HANDHELD_HINT_PROFILE),
    hint("Y", keys::HANDHELD_HINT_OPTIONS),
    hint("Select", keys::HANDHELD_HINT_GAME_OPTIONS),
];
const HACKS_HINTS: &[Hint] = &[
    hint("</>", keys::HANDHELD_HINT_CHANGE),
    hint("B", keys::HANDHELD_HINT_BACK),
];
const PROFILES_HINTS: &[Hint] = &[
    hint("A", keys::HANDHELD_HINT_CHOOSE),
    hint("B", keys::HANDHELD_HINT_BACK),
];
const OPTIONS_HINTS: &[Hint] = &[
    hint("</>", keys::HANDHELD_HINT_CHANGE),
    hint("B", keys::HANDHELD_HINT_BACK),
];
const GAME_OPTIONS_HINTS: &[Hint] = &[
    hint("</>", keys::HANDHELD_HINT_CHANGE),
    hint("X", keys::HANDHELD_HINT_RESET_ALL),
    hint("B", keys::HANDHELD_HINT_BACK),
];
const CONTROLS_HINTS: &[Hint] = &[
    hint("A", keys::HANDHELD_HINT_REBIND),
    hint("B", keys::HANDHELD_HINT_BACK),
];
const STATES_SAVE_HINTS: &[Hint] = &[
    hint("A", keys::HANDHELD_HINT_SAVE_HERE),
    hint("</>", keys::HANDHELD_HINT_PAGE),
    hint("B", keys::HANDHELD_HINT_BACK),
];
const STATES_LOAD_HINTS: &[Hint] = &[
    hint("A", keys::HANDHELD_HINT_LOAD),
    hint("</>", keys::HANDHELD_HINT_PAGE),
    hint("B", keys::HANDHELD_HINT_BACK),
];
const OVERLAY_HINTS: &[Hint] = &[
    hint("A", keys::HANDHELD_HINT_CHOOSE),
    hint("</>", keys::HANDHELD_HINT_CHANGE),
    hint("B", keys::HANDHELD_MENU_RESUME),
];

const ACHIEVEMENTS_HINTS: &[Hint] = &[
    hint("A", keys::HANDHELD_HINT_CHOOSE),
    hint("</>", keys::HANDHELD_HINT_CHANGE),
    hint("B", keys::HANDHELD_HINT_BACK),
];
const KEYBOARD_HINTS: &[Hint] = &[
    hint("A", keys::ACH_KEY_TYPE),
    hint("X", keys::ACH_KEY_DELETE),
    hint("Y", keys::ACH_KEY_SHIFT),
    hint("Start", keys::ACH_KEY_DONE),
    hint("B", keys::HANDHELD_HINT_CANCEL),
];
const ACHIEVEMENT_LIST_HINTS: &[Hint] = &[
    hint("L/R", keys::HANDHELD_HINT_PAGE),
    hint("B", keys::HANDHELD_HINT_BACK),
];

const ACHIEVEMENT_GRID_HINTS: &[Hint] = &[hint("B", keys::HANDHELD_HINT_BACK)];

/// The hints at the bottom of `screen`.
#[must_use]
pub const fn hints(screen: Screen) -> &'static [Hint] {
    match screen {
        Screen::Home => HOME_HINTS,
        Screen::Profiles => PROFILES_HINTS,
        Screen::Options | Screen::Overscan => OPTIONS_HINTS,
        Screen::GameOptions => GAME_OPTIONS_HINTS,
        Screen::Hacks => HACKS_HINTS,
        Screen::Controls => CONTROLS_HINTS,
        Screen::Overlay => OVERLAY_HINTS,
        Screen::States => STATES_SAVE_HINTS,
        Screen::Achievements => ACHIEVEMENTS_HINTS,
        Screen::Keyboard => KEYBOARD_HINTS,
        Screen::AchievementList => ACHIEVEMENT_LIST_HINTS,
        Screen::AchievementGrid => ACHIEVEMENT_GRID_HINTS,
        Screen::InGame => &[],
    }
}

/// The hints of the save state screen, which say whether A saves or loads.
#[must_use]
pub const fn states_hints(mode: super::states::StatesMode) -> &'static [Hint] {
    match mode {
        super::states::StatesMode::Save => STATES_SAVE_HINTS,
        super::states::StatesMode::Load => STATES_LOAD_HINTS,
    }
}

/// The hints while a binding waits for a button: only the way out.
pub const CAPTURE_HINTS: &[Hint] = &[hint("Menu", keys::HANDHELD_HINT_CANCEL)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::strings::{self, UiLanguage};

    #[test]
    fn the_sizes_follow_the_height_of_the_display() {
        let small = Metrics::for_height(480.0);
        assert!((small.font - 22.0).abs() < f32::EPSILON);
        assert!(small.row >= MINIMUM_ROW);
        let square = Metrics::for_height(720.0);
        assert!(square.font > small.font && square.row > small.row);
        // Even a tiny display keeps a readable font and a row to press.
        let tiny = Metrics::for_height(240.0);
        assert!(tiny.font >= 20.0 && tiny.row >= MINIMUM_ROW);
    }

    #[test]
    fn a_480_high_display_shows_a_screenful_of_rows() {
        let metrics = Metrics::for_height(480.0);
        let rows = metrics.visible_rows(480.0);
        assert!((6..=10).contains(&rows), "{rows}");
        let rows_height = f32::from(u16::try_from(rows).unwrap()) * metrics.row;
        let used = metrics.title_band() + rows_height + metrics.hint_band();
        assert!(used <= 480.0);
        assert_eq!(1, metrics.visible_rows(10.0));
    }

    #[test]
    fn every_hint_has_a_text_in_both_languages() {
        for screen in [
            Screen::Home,
            Screen::Profiles,
            Screen::Options,
            Screen::Controls,
            Screen::Overlay,
            Screen::States,
        ] {
            assert!(!hints(screen).is_empty(), "{screen:?}");
        }
        for hint in [
            hints(Screen::Home),
            hints(Screen::Overlay),
            hints(Screen::Options),
            STATES_SAVE_HINTS,
            STATES_LOAD_HINTS,
            CAPTURE_HINTS,
        ]
        .concat()
        {
            for language in [UiLanguage::English, UiLanguage::French] {
                assert!(
                    strings::lookup(language, hint.meaning).is_some(),
                    "{hint:?}"
                );
            }
        }
    }
}
