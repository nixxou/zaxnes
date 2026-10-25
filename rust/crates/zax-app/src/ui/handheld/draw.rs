//! The drawing of the handheld screens: thin, since the models decide what is shown. Rows are
//! painted on the window's draw list rather than as widgets, because the selection belongs to the
//! models and the gamepad moves it; the focus is a bright bar with dark text.

use imgui::{Condition, DrawListMut, StyleColor, StyleVar, Ui, WindowFlags, WindowToken};
use zax_platform::input::labels;

use super::game_options::{GameOptionsModel, GAME_ROWS};
use super::layout::{self, Hint, Metrics};
use super::options::{self, ROWS};
use super::overlay::{OverlayItem, ITEMS};
use super::rom_list::{first_visible, ProfileMatch, RomEntry, RomListModel};
use super::states::{SlotState, StatesMode};
use super::{slot_state, HandheldUi, Screen, STATE_PICTURE_ID};
use crate::input::BindingDeviceKind;
use crate::persistence::save_slots::numbered_slot_id;
use crate::ui::confirm_dialog::ConfirmationView;
use crate::ui::menu_bar;
use crate::ui::zax_ui::{UiFrame, ZaxUi};
use crate::ui::{key_config, keys, strings, text};

const BACKGROUND: [f32; 4] = [0.03, 0.03, 0.06, 1.0];
const PANEL: [f32; 4] = [0.05, 0.05, 0.09, 0.96];
const TEXT: [f32; 4] = [0.96, 0.96, 0.96, 1.0];
const DIM: [f32; 4] = [0.68, 0.72, 0.8, 1.0];
const FOCUS: [f32; 4] = [1.0, 0.82, 0.16, 1.0];
const ON_FOCUS: [f32; 4] = [0.04, 0.04, 0.04, 1.0];
const PROFILE: [f32; 4] = [0.45, 0.92, 0.55, 1.0];
const PROFILE_ON_FOCUS: [f32; 4] = [0.02, 0.32, 0.08, 1.0];
const BAR: [f32; 4] = [0.1, 0.1, 0.16, 1.0];
const WARNING: [f32; 4] = [1.0, 0.45, 0.35, 1.0];
const WARNING_ON_FOCUS: [f32; 4] = [0.6, 0.06, 0.02, 1.0];
/// A value a game has of its own, and the hint of the list when the game has some.
const GAME_OWN: [f32; 4] = [0.3, 0.9, 1.0, 1.0];
const GAME_OWN_ON_FOCUS: [f32; 4] = [0.0, 0.3, 0.4, 1.0];
/// The save state screen when it saves, and when it loads: two colours apart from each other and
/// from the orange and red of the states that do not load as they are.
const SAVE_ACCENT: [f32; 4] = [0.84, 0.46, 1.0, 1.0];
const LOAD_ACCENT: [f32; 4] = [0.36, 0.68, 1.0, 1.0];
/// A state made with the built-in patches set the other way (it loads once they are switched).
const OTHER_MODE: [f32; 4] = [1.0, 0.62, 0.2, 1.0];

/// A few games for the screenshots of a machine with no ROM folder.
pub(super) fn sample_roms() -> RomListModel {
    let entry = |name: &str, profile: Option<&str>| {
        let mut entry = RomEntry::new(std::path::PathBuf::from(format!("/roms/{name}.nes")));
        entry.profile = profile.map(|title| ProfileMatch {
            id: title.to_lowercase(),
            title: title.to_owned(),
            two_dimensional: false,
        });
        entry
    };
    RomListModel::new(vec![
        entry("Adventure Island", Some("Adventure Island")),
        entry("Brick Breaker", None),
        entry("Castle Quest (U)", Some("Castle Quest")),
        entry("Dungeon Dash", None),
        entry("Escape From Mars", Some("Escape From Mars")),
        entry("Forest Runner", None),
        entry("Galaxy Raiders", Some("Galaxy Raiders")),
        entry("Haunted Halls", None),
        entry("Ice Climber Jr", None),
        entry("Jungle Jump", Some("Jungle Jump")),
    ])
}

/// What the painting of one screen needs.
struct Painter<'ui> {
    ui: &'ui Ui,
    list: DrawListMut<'ui>,
    metrics: Metrics,
    size: [f32; 2],
    heading: Option<imgui::FontId>,
}

impl Painter<'_> {
    fn text(&self, x: f32, y: f32, colour: [f32; 4], text: &str) {
        self.list.add_text([x, y], colour, text);
    }

    fn width(&self, text: &str) -> f32 {
        self.ui.calc_text_size(text)[0]
    }

    /// The title band: the heading on the left, `right` in the body face on the right.
    fn title(&self, title: &str, right: &str) {
        let metrics = &self.metrics;
        self.list
            .add_rect([0.0, 0.0], [self.size[0], metrics.title_band()], BAR)
            .filled(true)
            .build();
        {
            let _heading = self.heading.map(|font| self.ui.push_font(font));
            self.text(metrics.padding, metrics.padding, FOCUS, title);
        }
        if !right.is_empty() {
            let x = self.size[0] - metrics.padding - self.width(right);
            let y = (metrics.title_band() - metrics.font) * 0.5;
            self.text(x, y, DIM, right);
        }
    }

    /// The top of the row `slot` of a list that starts under the title.
    fn row_top(&self, slot: usize) -> f32 {
        let slots = f32::from(u16::try_from(slot).unwrap_or(u16::MAX));
        self.metrics.title_band() + self.metrics.padding * 0.5 + slots * self.metrics.row
    }

    /// One row of a list: `left` clipped to leave room for `right`.
    fn row(&self, slot: usize, selected: bool, left: &str, right: (&str, [f32; 4])) {
        let x1 = self.size[0] - self.metrics.padding * 0.5;
        self.row_to(slot, (selected, FOCUS), left, right, x1);
    }

    /// One row of a list that ends at `x1`, its focus bar of `focus` when selected.
    fn row_to(
        &self,
        slot: usize,
        (selected, focus): (bool, [f32; 4]),
        left: &str,
        right: (&str, [f32; 4]),
        x1: f32,
    ) {
        self.row_parts(slot, (selected, focus), left, None, right, x1);
    }

    /// One row of a list with `badge` before `right` (the [W] of a game a widescreen hack is
    /// made for).
    fn row_badged(
        &self,
        slot: usize,
        selected: bool,
        left: &str,
        badge: (&str, [f32; 4]),
        right: (&str, [f32; 4]),
    ) {
        let x1 = self.size[0] - self.metrics.padding * 0.5;
        self.row_parts(slot, (selected, FOCUS), left, Some(badge), right, x1);
    }

    /// One row of a list that ends at `x1`: `left`, then `badge` if any, then `right`.
    fn row_parts(
        &self,
        slot: usize,
        (selected, focus): (bool, [f32; 4]),
        left: &str,
        badge: Option<(&str, [f32; 4])>,
        right: (&str, [f32; 4]),
        x1: f32,
    ) {
        let metrics = &self.metrics;
        let top = self.row_top(slot);
        let x0 = metrics.padding * 0.5;
        if selected {
            self.list
                .add_rect([x0, top], [x1, top + metrics.row], focus)
                .filled(true)
                .rounding(metrics.padding * 0.4)
                .build();
        }
        let y = top + (metrics.row - metrics.font) * 0.5;
        // Grey on the bright bar would not read: the focus keeps dark text.
        let (main, side) = if selected {
            (ON_FOCUS, if right.1 == DIM { ON_FOCUS } else { right.1 })
        } else {
            (TEXT, right.1)
        };
        let right_width = self.width(right.0);
        let right_x = x1 - metrics.padding - right_width;
        if !right.0.is_empty() {
            self.text(right_x, y, side, right.0);
        }
        let mut clip_to = if right.0.is_empty() {
            x1
        } else {
            right_x - metrics.padding
        };
        if let Some((badge, colour)) = badge {
            let badge_x = clip_to - self.width(badge);
            let colour = if !selected {
                colour
            } else if colour == GAME_OWN {
                GAME_OWN_ON_FOCUS
            } else if colour == WARNING {
                WARNING_ON_FOCUS
            } else {
                ON_FOCUS
            };
            self.text(badge_x, y, colour, badge);
            clip_to = badge_x - metrics.padding;
        }
        self.list
            .with_clip_rect_intersect([x0, top], [clip_to, top + metrics.row], || {
                self.text(x0 + metrics.padding, y, main, left);
            });
    }

    /// The bar of hints at the bottom.
    fn hints(&self, hints: &[Hint]) {
        let texts: Vec<(&str, String)> = hints
            .iter()
            .map(|hint| (hint.button, strings::get(hint.meaning)))
            .collect();
        self.hint_texts(&texts);
    }

    /// The bar of hints at the bottom, each a button and what it does, already in words.
    fn hint_texts(&self, hints: &[(&str, String)]) {
        let entries: Vec<(&str, String, [f32; 4])> = hints
            .iter()
            .map(|(button, meaning)| (*button, meaning.clone(), TEXT))
            .collect();
        self.hint_entries(&entries);
    }

    /// The same, each meaning in a colour of its own.
    fn hint_entries(&self, hints: &[(&str, String, [f32; 4])]) {
        let metrics = &self.metrics;
        let top = self.size[1] - metrics.hint_band();
        self.list
            .add_rect([0.0, top], self.size, BAR)
            .filled(true)
            .build();
        let y = top + metrics.padding;
        let mut x = metrics.padding;
        for (button, meaning, colour) in hints {
            self.text(x, y, FOCUS, button);
            x += self.width(button) + metrics.padding * 0.5;
            self.text(x, y, *colour, meaning);
            x += self.width(meaning) + metrics.padding * 1.6;
        }
    }

    /// A paragraph in small print under the row `slot` of the list area.
    fn note(&self, slot: usize, text: &str) {
        let metrics = &self.metrics;
        let width = self.size[0] - 4.0 * metrics.padding;
        self.ui
            .set_cursor_pos([2.0 * metrics.padding, self.row_top(slot) + metrics.padding]);
        let _wrap = self
            .ui
            .push_text_wrap_pos_with_pos(2.0 * metrics.padding + width);
        let _colour = self.ui.push_style_color(StyleColor::Text, DIM);
        self.ui.text_wrapped(text);
    }

    /// A paragraph in the middle of the list area.
    fn message(&self, text: &str, colour: [f32; 4]) {
        let metrics = &self.metrics;
        let width = self.size[0] - 4.0 * metrics.padding;
        self.ui
            .set_cursor_pos([2.0 * metrics.padding, self.row_top(1)]);
        let _wrap = self
            .ui
            .push_text_wrap_pos_with_pos(2.0 * metrics.padding + width);
        let _colour = self.ui.push_style_color(StyleColor::Text, colour);
        self.ui.text_wrapped(text);
    }
}

/// A window over the whole display, with no frame, no scrolling and no focus of its own.
fn full_screen<'ui>(ui: &'ui Ui, name: &str, background: [f32; 4]) -> Option<WindowToken<'ui>> {
    full_screen_with(
        ui,
        name,
        background,
        WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS,
    )
}

/// The same, over every other window: Dear `ImGui` files a window that is not brought to the
/// front behind all the others when it first appears, which would hide a question.
fn full_screen_over<'ui>(
    ui: &'ui Ui,
    name: &str,
    background: [f32; 4],
) -> Option<WindowToken<'ui>> {
    full_screen_with(ui, name, background, WindowFlags::empty())
}

fn full_screen_with<'ui>(
    ui: &'ui Ui,
    name: &str,
    background: [f32; 4],
    extra: WindowFlags,
) -> Option<WindowToken<'ui>> {
    let _padding = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
    let _border = ui.push_style_var(StyleVar::WindowBorderSize(0.0));
    let _rounding = ui.push_style_var(StyleVar::WindowRounding(0.0));
    let _background = ui.push_style_color(StyleColor::WindowBg, background);
    ui.window(name)
        .position([0.0, 0.0], Condition::Always)
        .size(ui.io().display_size, Condition::Always)
        .flags(
            WindowFlags::NO_DECORATION
                | WindowFlags::NO_MOVE
                | WindowFlags::NO_SAVED_SETTINGS
                | WindowFlags::NO_NAV
                | extra,
        )
        .begin()
}

impl HandheldUi {
    /// Draws the screen in front, the hold of the exit combination and the notification.
    pub fn draw(&mut self, ui: &Ui, frame: &UiFrame<'_>) {
        let size = ui.io().display_size;
        self.metrics = self.metrics_for(size[1]);
        let _body = self.body_font().map(|font| ui.push_font(font));
        self.follow_the_state_picture(frame.host);
        match self.screen {
            Screen::Home => self.draw_home(ui, frame),
            Screen::GameOptions => self.draw_game_options(ui),
            Screen::Hacks => self.draw_hacks(ui),
            Screen::Profiles => self.draw_profiles(ui),
            Screen::Options => self.draw_options(ui, frame),
            Screen::Controls => self.draw_controls(ui, frame),
            Screen::Overlay => self.draw_overlay(ui, frame),
            Screen::States => self.draw_states(ui, frame),
            Screen::InGame => {}
        }
        if let Some(question) = ZaxUi::confirmation(frame.host) {
            self.draw_confirmation(ui, question);
        }
        self.draw_exit_hold(ui);
        let margin = self.metrics.hint_band() + self.metrics.padding;
        ZaxUi::draw_notification(ui, frame.host, margin);
    }

    fn painter<'ui>(&self, ui: &'ui Ui) -> Painter<'ui> {
        Painter {
            ui,
            list: ui.get_window_draw_list(),
            metrics: self.metrics,
            size: ui.io().display_size,
            heading: self.heading_font,
        }
    }

    fn draw_home(&self, ui: &Ui, frame: &UiFrame<'_>) {
        let Some(_window) = full_screen(ui, "##home", BACKGROUND) else {
            return;
        };
        let painter = self.painter(ui);
        let total = self.roms.entries().len();
        let position = if total == 0 {
            String::new()
        } else {
            let initial = self.roms.selected_entry().map_or('#', RomEntry::initial);
            format!("{initial}  {}/{total}", self.roms.selected() + 1)
        };
        painter.title(text::PRODUCT_NAME, &position);
        if total == 0 {
            let folder = self.roms_directory.display().to_string();
            let message = strings::format(&strings::get(keys::HANDHELD_ROMS_EMPTY), &[&folder]);
            painter.message(&message, WARNING);
        }
        let rows = self.metrics.visible_rows(painter.size[1]);
        let first = first_visible(self.roms.selected(), rows, total);
        for (slot, (index, entry)) in self
            .roms
            .entries()
            .iter()
            .enumerate()
            .skip(first)
            .take(rows)
            .enumerate()
        {
            let selected = index == self.roms.selected();
            let tag = profile_tag(entry);
            let colour = if selected && tag.1 == PROFILE {
                PROFILE_ON_FOCUS
            } else {
                tag.1
            };
            if entry.widescreen || entry.widescreen_other.is_some() {
                // Grey while off, coloured once on, red for a version of the game the hack is
                // not made for (owner request).
                // Orange once forced and on.
                let badge = if entry.widescreen_forced && entry.widescreen_on {
                    OTHER_MODE
                } else if entry.widescreen_other.is_some() {
                    WARNING
                } else if entry.widescreen_on {
                    GAME_OWN
                } else {
                    DIM
                };
                painter.row_badged(
                    slot,
                    selected,
                    &entry.name,
                    ("[W]", badge),
                    (&tag.0, colour),
                );
            } else {
                painter.row(slot, selected, &entry.name, (&tag.0, colour));
            }
        }
        // "Game options" in cyan when the selected game has some (owner request).
        let own = self
            .roms
            .selected_entry()
            .is_some_and(|entry| Self::has_game_options(frame.host, &entry.path));
        let mut hints: Vec<(&str, String, [f32; 4])> = layout::hints(Screen::Home)
            .iter()
            .map(|hint| {
                let colour = if own && hint.meaning == keys::HANDHELD_HINT_GAME_OPTIONS {
                    GAME_OWN
                } else {
                    TEXT
                };
                (hint.button, strings::get(hint.meaning), colour)
            })
            .collect();
        // L2 turns the widescreen of the selected game on or off, Start opens its hacks.
        if let Some(entry) = self
            .roms
            .selected_entry()
            .filter(|entry| entry.widescreen || entry.widescreen_other.is_some())
        {
            let colour = if entry.widescreen_forced && entry.widescreen_on {
                OTHER_MODE
            } else if entry.widescreen_other.is_some() {
                WARNING
            } else if entry.widescreen_on {
                GAME_OWN
            } else {
                TEXT
            };
            hints.push(("L2", strings::get(keys::HANDHELD_HINT_WIDESCREEN), colour));
            hints.push(("Start", strings::get(keys::HANDHELD_HINT_HACKS), TEXT));
        }
        painter.hint_entries(&hints);
    }

    fn draw_game_options(&self, ui: &Ui) {
        let (Some(_window), Some(options)) = (
            full_screen(ui, "##game-options", BACKGROUND),
            &self.game_options,
        ) else {
            return;
        };
        let painter = self.painter(ui);
        let title = strings::format(
            &strings::get(keys::HANDHELD_GAME_OPTIONS_TITLE),
            &[options.name()],
        );
        painter.title(&title, "");
        let rows = self.metrics.visible_rows(painter.size[1]);
        let selected = options.selected_index();
        let total = GameOptionsModel::ROW_COUNT;
        let first = first_visible(selected, rows, total);
        for (slot, index) in (0..total).skip(first).take(rows).enumerate() {
            let focused = index == selected;
            if index == GameOptionsModel::HACKS_ROW {
                painter.row(
                    slot,
                    focused,
                    &strings::get(keys::HANDHELD_HINT_HACKS),
                    (">", DIM),
                );
                continue;
            }
            let Some(row) = index.checked_sub(1).map(|row| GAME_ROWS[row]) else {
                let colour = if options.has_options() { GAME_OWN } else { DIM };
                let colour = if focused && colour == GAME_OWN {
                    GAME_OWN_ON_FOCUS
                } else {
                    colour
                };
                painter.row(
                    slot,
                    focused,
                    &strings::get(keys::HANDHELD_GAME_OPTIONS_RESET),
                    ("", colour),
                );
                continue;
            };
            let own = options.is_own(row);
            let colour = match (own, focused) {
                (true, true) => GAME_OWN_ON_FOCUS,
                (true, false) => GAME_OWN,
                (false, _) => DIM,
            };
            let shown = format!("< {} >", options.value_text(row));
            painter.row(slot, focused, &options::label(row), (&shown, colour));
        }
        painter.hints(layout::hints(Screen::GameOptions));
    }

    /// The hacks of one game: the widescreen, its ratio and margin, the options of its patch.
    fn draw_hacks(&self, ui: &Ui) {
        let (Some(_window), Some(hacks)) = (full_screen(ui, "##hacks", BACKGROUND), &self.hacks)
        else {
            return;
        };
        let painter = self.painter(ui);
        let title = strings::format(&strings::get(keys::HACKS_TITLE), &[hacks.name()]);
        painter.title(&title, "");
        let rows = hacks.rows();
        if rows.is_empty() {
            match hacks.other_rom_message() {
                Some(message) => painter.message(&message, WARNING),
                None => painter.message(&strings::get(keys::HACKS_NONE), DIM),
            }
        }
        // Another version of the game: what it is, under the rows that force the patch.
        let other = hacks.other_rom_message().filter(|_| !rows.is_empty());
        // The note on the patch takes the place of a row or two at the bottom.
        let visible = self.metrics.visible_rows(painter.size[1]);
        let visible = if other.is_some() {
            visible.saturating_sub(3).max(1)
        } else if hacks.patch().is_some() {
            visible.saturating_sub(2).max(1)
        } else {
            visible
        };
        let selected = hacks.selected_index();
        let first = first_visible(selected, visible, rows.len());
        for (slot, (index, row)) in rows
            .iter()
            .enumerate()
            .skip(first)
            .take(visible)
            .enumerate()
        {
            let focused = index == selected;
            let colour = match (hacks.is_own(*row), focused) {
                (true, true) => GAME_OWN_ON_FOCUS,
                (true, false) => GAME_OWN,
                (false, _) => DIM,
            };
            let shown = format!("< {} >", hacks.value_text(*row));
            painter.row(slot, focused, &hacks.label(*row), (&shown, colour));
        }
        if let Some(other) = other {
            painter.note(visible, &other);
        } else if hacks.patch().is_some() {
            painter.note(visible, &strings::get(keys::HACKS_PATCH_NOTE));
        }
        painter.hints(layout::hints(Screen::Hacks));
    }

    fn draw_profiles(&self, ui: &Ui) {
        let (Some(_window), Some(profiles)) =
            (full_screen(ui, "##profiles", BACKGROUND), &self.profiles)
        else {
            return;
        };
        let painter = self.painter(ui);
        let title = strings::format(
            &strings::get(keys::HANDHELD_PROFILE_FOR),
            &[profiles.name()],
        );
        painter.title(&title, "");
        let rows = self.metrics.visible_rows(painter.size[1]);
        let total = profiles.choices().len();
        let first = first_visible(profiles.selected(), rows, total);
        for (slot, (index, choice)) in profiles
            .choices()
            .iter()
            .enumerate()
            .skip(first)
            .take(rows)
            .enumerate()
        {
            painter.row(
                slot,
                index == profiles.selected(),
                &choice.title,
                (&choice.source_label(), DIM),
            );
        }
        painter.hints(layout::hints(Screen::Profiles));
    }

    fn draw_options(&self, ui: &Ui, frame: &UiFrame<'_>) {
        let Some(_window) = full_screen(ui, "##options", BACKGROUND) else {
            return;
        };
        let painter = self.painter(ui);
        painter.title(&text::options_title(), "");
        // The list scrolls with the selection when it is longer than the screen.
        let rows = self.metrics.visible_rows(painter.size[1]);
        let selected = self.options.selected_index();
        let first = first_visible(selected, rows, ROWS.len());
        for (slot, (index, row)) in ROWS.iter().enumerate().skip(first).take(rows).enumerate() {
            let value = options::value_text(*row, frame.host.settings(), self.overlay.camera());
            let shown = match row {
                options::OptionRow::Controls => String::new(),
                _ => format!("< {value} >"),
            };
            painter.row(
                slot,
                index == selected,
                &options::label(*row),
                (&shown, DIM),
            );
        }
        painter.hints(layout::hints(Screen::Options));
    }

    fn draw_controls(&mut self, ui: &Ui, frame: &UiFrame<'_>) {
        let Some(_window) = full_screen(ui, "##controls", BACKGROUND) else {
            return;
        };
        let rows = self.control_rows();
        let painter = self.painter(ui);
        painter.title(&strings::get(keys::HANDHELD_CONTROLS_PLAYER), "");
        let session = frame.input.rebinding();
        let set = key_config::set_of(
            frame.input.bindings(),
            frame.input.pads(),
            0,
            BindingDeviceKind::Gamepad,
        );
        if set.is_none() {
            painter.message(&text::no_gamepad(), WARNING);
            painter.hints(layout::hints(Screen::Controls));
            return;
        }
        // The list scrolls with the selection when it is longer than the screen.
        let visible = self.metrics.visible_rows(painter.size[1]);
        let first = first_visible(self.controls.selected, visible, rows.len());
        for (slot, (index, row)) in rows
            .iter()
            .enumerate()
            .skip(first)
            .take(visible)
            .enumerate()
        {
            let listening = key_config::is_listening_for(session, row, BindingDeviceKind::Gamepad);
            let binding = if listening {
                text::press_a_control()
            } else {
                set.map_or_else(String::new, |set| labels::of(set.get(&row.target)))
            };
            painter.row(
                slot,
                index == self.controls.selected,
                &row.label,
                (&binding, DIM),
            );
        }
        let hints = if self.controls.capture_left.is_some() {
            layout::CAPTURE_HINTS
        } else {
            layout::hints(Screen::Controls)
        };
        painter.hints(hints);
    }

    fn draw_overlay(&self, ui: &Ui, frame: &UiFrame<'_>) {
        let size = ui.io().display_size;
        ui.get_background_draw_list()
            .add_rect([0.0, 0.0], size, [0.0, 0.0, 0.0, 0.6])
            .filled(true)
            .build();
        let Some(_window) = full_screen(ui, "##overlay", [0.0, 0.0, 0.0, 0.0]) else {
            return;
        };
        let painter = self.painter(ui);
        let metrics = &self.metrics;
        // The menu is a panel in the middle, so the game stays visible around it.
        let rows = ITEMS.len();
        let panel_height = metrics.title_band()
            + metrics.row * f32::from(u16::try_from(rows).unwrap_or(u16::MAX))
            + metrics.padding;
        let panel_width = (size[0] - 4.0 * metrics.padding).min(metrics.font * 24.0);
        let left = (size[0] - panel_width) * 0.5;
        let top = ((size[1] - metrics.hint_band() - panel_height) * 0.5).max(0.0);
        painter
            .list
            .add_rect([left, top], [left + panel_width, top + panel_height], PANEL)
            .filled(true)
            .rounding(metrics.padding)
            .build();
        self.draw_overlay_rows(&painter, frame, left, top, panel_width);
        painter.hints(layout::hints(Screen::Overlay));
    }

    fn draw_overlay_rows(
        &self,
        painter: &Painter<'_>,
        frame: &UiFrame<'_>,
        left: f32,
        top: f32,
        width: f32,
    ) {
        let metrics = &self.metrics;
        {
            let _heading = painter.heading.map(|font| painter.ui.push_font(font));
            painter.text(
                left + metrics.padding,
                top + metrics.padding,
                FOCUS,
                &strings::get(keys::HANDHELD_MENU_TITLE),
            );
        }
        for (index, item) in ITEMS.iter().enumerate() {
            let slot = f32::from(u16::try_from(index).unwrap_or(0));
            let row_top = top + metrics.title_band() + slot * metrics.row;
            let selected = index == self.overlay.selected_index();
            if selected {
                painter
                    .list
                    .add_rect(
                        [left + metrics.padding * 0.5, row_top],
                        [left + width - metrics.padding * 0.5, row_top + metrics.row],
                        FOCUS,
                    )
                    .filled(true)
                    .rounding(metrics.padding * 0.4)
                    .build();
            }
            let colour = if selected { ON_FOCUS } else { TEXT };
            let y = row_top + (metrics.row - metrics.font) * 0.5;
            painter.text(
                left + metrics.padding * 1.5,
                y,
                colour,
                &Self::overlay_label(*item),
            );
            let value = self.overlay_value(*item, frame);
            if !value.is_empty() {
                let x = left + width - metrics.padding * 1.5 - painter.width(&value);
                painter.text(x, y, if selected { ON_FOCUS } else { DIM }, &value);
            }
        }
    }

    fn overlay_label(item: OverlayItem) -> String {
        strings::get(match item {
            OverlayItem::Resume => keys::HANDHELD_MENU_RESUME,
            OverlayItem::SaveState => keys::HANDHELD_MENU_SAVE_STATE,
            OverlayItem::LoadState => keys::HANDHELD_MENU_LOAD_STATE,
            OverlayItem::FastForward => keys::HANDHELD_MENU_FAST_FORWARD,
            OverlayItem::Camera => keys::HANDHELD_OPTION_CAMERA,
            OverlayItem::Options => keys::WINDOW_OPTIONS,
            OverlayItem::Hacks => keys::HANDHELD_HINT_HACKS,
            OverlayItem::QuitToList => keys::HANDHELD_MENU_QUIT_TO_LIST,
            OverlayItem::ExitToLauncher => keys::HANDHELD_MENU_EXIT,
        })
    }

    fn overlay_value(&self, item: OverlayItem, frame: &UiFrame<'_>) -> String {
        match item {
            OverlayItem::SaveState | OverlayItem::LoadState => {
                let slot = self.overlay.slot();
                let record = frame.host.slots.find(&numbered_slot_id(slot));
                format!("< {} >", menu_bar::slot_label(slot, record))
            }
            OverlayItem::Camera => format!("< {} >", options::camera_label(self.overlay.camera())),
            _ => String::new(),
        }
    }

    /// The save state screen: the slots on the left, the thumbnail of the selected one on the
    /// right, in the colour of what A does (save or load), which the title says too.
    fn draw_states(&self, ui: &Ui, frame: &UiFrame<'_>) {
        let Some(_window) = full_screen(ui, "##states", BACKGROUND) else {
            return;
        };
        let painter = self.painter(ui);
        let metrics = &self.metrics;
        let states = self.states();
        let (accent, title) = match states.mode() {
            StatesMode::Save => (SAVE_ACCENT, keys::HANDHELD_STATES_SAVE_TITLE),
            StatesMode::Load => (LOAD_ACCENT, keys::HANDHELD_STATES_LOAD_TITLE),
        };
        let size = painter.size;
        let band = metrics.title_band();
        painter
            .list
            .add_rect([0.0, 0.0], [size[0], band], BAR)
            .filled(true)
            .build();
        painter
            .list
            .add_rect([0.0, band - metrics.padding * 0.3], [size[0], band], accent)
            .filled(true)
            .build();
        {
            let _heading = painter.heading.map(|font| ui.push_font(font));
            painter.text(
                metrics.padding,
                metrics.padding,
                accent,
                &strings::get(title),
            );
        }

        // The slots, on the left.
        let split = (size[0] * 0.56).round();
        let total = crate::persistence::save_slots::EXPOSED_SLOT_COUNT;
        let rows = metrics.visible_rows(size[1]);
        let first = first_visible(states.selected_index(), rows, total);
        for (row, index) in (first..total).take(rows).enumerate() {
            let slot = i32::try_from(index + 1).unwrap_or(1);
            let record = frame.host.slots.find(&numbered_slot_id(slot));
            let colour = match slot_state(frame.host, slot) {
                SlotState::Empty => DIM,
                SlotState::Fits => TEXT,
                SlotState::OtherMode => OTHER_MODE,
                SlotState::OtherVersion => WARNING,
            };
            let label = menu_bar::slot_label(slot, record);
            let (name, date) = label.split_once(" — ").unwrap_or((label.as_str(), ""));
            painter.row_to(
                row,
                (index == states.selected_index(), accent),
                name,
                (date, colour),
                split - metrics.padding * 0.5,
            );
        }

        self.draw_state_preview(&painter, frame, accent, split);
        painter.hints(layout::states_hints(states.mode()));
    }

    /// The thumbnail of the selected slot, framed in `accent`, right of `split`; under it, why
    /// the state does not load as it is, when it does not.
    fn draw_state_preview(
        &self,
        painter: &Painter<'_>,
        frame: &UiFrame<'_>,
        accent: [f32; 4],
        split: f32,
    ) {
        let ui = painter.ui;
        let metrics = &self.metrics;
        let size = painter.size;
        let states = self.states();
        let left = split + metrics.padding * 0.5;
        let right = size[0] - metrics.padding;
        let top = painter.row_top(0);
        let bottom = size[1] - metrics.hint_band() - metrics.padding;
        let width = (right - left).max(1.0);
        let height = (width * 0.75).min((bottom - top - 2.0 * metrics.row).max(1.0));
        let width = height / 0.75;
        let x = left + ((right - left) - width) * 0.5;
        painter
            .list
            .add_rect(
                [x - 3.0, top - 3.0],
                [x + width + 3.0, top + height + 3.0],
                accent,
            )
            .rounding(metrics.padding * 0.3)
            .thickness(3.0)
            .build();
        let slot = states.slot();
        if self.state_picture_shown() {
            painter
                .list
                .add_image(
                    imgui::TextureId::new(STATE_PICTURE_ID),
                    [x, top],
                    [x + width, top + height],
                )
                .build();
        } else {
            let empty = !frame.host.slots.is_occupied(&numbered_slot_id(slot));
            let message = if empty {
                text::empty_slot()
            } else {
                strings::get(keys::HANDHELD_STATES_NO_PICTURE)
            };
            let message_x = x + (width - painter.width(&message)) * 0.5;
            painter.text(
                message_x,
                top + (height - metrics.font) * 0.5,
                DIM,
                &message,
            );
        }
        // Under it, why the state does not load as it is, when it does not.
        let caption = match slot_state(frame.host, slot) {
            SlotState::OtherMode => Some((keys::SLOT_OTHER_PATCH, OTHER_MODE)),
            SlotState::OtherVersion => Some((keys::SLOT_OTHER_HACK_VERSION, WARNING)),
            SlotState::Empty | SlotState::Fits => None,
        };
        if let Some((key, colour)) = caption {
            ui.set_cursor_pos([left, top + height + metrics.padding]);
            let _wrap = ui.push_text_wrap_pos_with_pos(right);
            let _colour = ui.push_style_color(StyleColor::Text, colour);
            ui.text_wrapped(strings::get(key));
        }
    }

    /// A question of the host, over the screen: its title and text, A to agree, B to decline.
    fn draw_confirmation(&self, ui: &Ui, question: ConfirmationView<'_>) {
        let size = ui.io().display_size;
        let metrics = &self.metrics;
        let Some(_window) = full_screen_over(ui, "##question", [0.0, 0.0, 0.0, 0.75]) else {
            return;
        };
        let painter = self.painter(ui);
        let width = (size[0] - 4.0 * metrics.padding).min(metrics.font * 26.0);
        let left = (size[0] - width) * 0.5;
        let top = size[1] * 0.2;
        let bottom = size[1] - metrics.hint_band() - 2.0 * metrics.padding;
        painter
            .list
            .add_rect([left, top], [left + width, bottom], PANEL)
            .filled(true)
            .rounding(metrics.padding)
            .build();
        {
            let _heading = painter.heading.map(|font| ui.push_font(font));
            painter.text(
                left + metrics.padding,
                top + metrics.padding,
                FOCUS,
                question.title(),
            );
        }
        ui.set_cursor_pos([left + metrics.padding, top + metrics.title_band()]);
        let _wrap = ui.push_text_wrap_pos_with_pos(left + width - metrics.padding);
        let _colour = ui.push_style_color(StyleColor::Text, TEXT);
        ui.text_wrapped(question.message());
        // A message to acknowledge has no cancelling button.
        let cancel = question.cancel_label();
        if cancel.is_empty() {
            painter.hint_texts(&[("A", question.confirm_label())]);
        } else {
            painter.hint_texts(&[("A", question.confirm_label()), ("B", cancel)]);
        }
    }

    /// While Select + Start is held: how long until the application quits.
    fn draw_exit_hold(&self, ui: &Ui) {
        let progress = self.combo.progress();
        if progress <= 0.0 {
            return;
        }
        let list = ui.get_foreground_draw_list();
        let size = ui.io().display_size;
        let metrics = &self.metrics;
        let bottom = size[1] - metrics.hint_band() - metrics.padding;
        let top = bottom - metrics.row;
        list.add_rect([0.0, top], [size[0], bottom], [0.0, 0.0, 0.0, 0.85])
            .filled(true)
            .build();
        list.add_rect([0.0, top], [size[0] * progress, bottom], WARNING)
            .filled(true)
            .build();
        let message = strings::get(keys::HANDHELD_EXIT_HOLD);
        let x = (size[0] - ui.calc_text_size(&message)[0]) * 0.5;
        list.add_text([x, top + (metrics.row - metrics.font) * 0.5], TEXT, message);
    }
}

/// The tag at the right of a game: its 3D profile, or "2D" when it has none.
fn profile_tag(entry: &RomEntry) -> (String, [f32; 4]) {
    match &entry.profile {
        Some(profile) if !profile.two_dimensional => (profile.title.clone(), PROFILE),
        _ => (strings::get(keys::HANDHELD_TAG_2D), DIM),
    }
}
