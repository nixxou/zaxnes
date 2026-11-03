//! The options window (REQ-UI-202): one tab per theme, one widget per setting.

use imgui::{
    ColorEditFlags, Condition, ItemHoveredFlags, TabBarFlags, TabItem, TabItemFlags, TreeNodeFlags,
    Ui, WindowFlags,
};
use zax_settings::catalogue::SettingId;
use zax_settings::meta::{SettingEffect, SettingKind};
use zax_settings::values::{Color, SettingValue};

use super::{centre, play, BrowserPurpose, ZaxUi};
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::ui::options::{self, CompactRow, OptionsGroup, OptionsRow, TabEntry};
use crate::ui::{strings, text};

/// The window's default size. RO-1 widened it: nine tabs and the long French titles need the room,
/// and the grid of a reorganised tab is no longer 41 rows deep (the old 560 × 520 was sized for a
/// window the player had to scroll).
const WINDOW_SIZE: [f32; 2] = [820.0, 600.0];

/// How wide a detailed tooltip is allowed to grow before its text wraps (RO-1 point 3).
const TOOLTIP_WIDTH: f32 = 420.0;

/// How wide one cell of a compact row is (RO-1 point 6): enough for "32 px" and the arrow, so
/// that the four cells and their captions fit on one line of the window.
const COMPACT_CELL_WIDTH: f32 = 70.0;

impl ZaxUi {
    pub(super) fn draw_options(&mut self, ui: &Ui, host: &mut AppHost) {
        let mut open = self.state.options;
        let window = ui
            .window(text::options_title())
            .size(WINDOW_SIZE, Condition::FirstUseEver)
            .position(
                centre(ui, WINDOW_SIZE[0], WINDOW_SIZE[1]),
                Condition::FirstUseEver,
            )
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if let Some(_window) = window {
            // RO-1: nine tabs. Scrolling keeps every title readable where the window is too narrow
            // for all of them; the default policy would shrink them to an ellipsis instead.
            if let Some(_bar) =
                ui.tab_bar_with_flags("##options", TabBarFlags::FITTING_POLICY_SCROLL)
            {
                // The capture aid of --ui-shots: asking for a tab is the same thing a click on it
                // would do. It is honoured on one frame only, so the tab the shot photographs has
                // no stray close button left on it.
                let forced = self.force_options_tab.take();
                for tab in options::TABS {
                    self.draw_options_tab(ui, host, tab, forced);
                }
            }
        }
        self.state.options = open;
    }

    /// One tab: its reset button, then its entries in the order [`options::LAYOUT`] gives them.
    fn draw_options_tab(
        &mut self,
        ui: &Ui,
        host: &mut AppHost,
        tab: options::OptionsTab,
        forced: Option<options::OptionsTab>,
    ) {
        let mut selected = true;
        let item = TabItem::new(options::title_of(tab));
        let shown = if forced == Some(tab) {
            item.flags(TabItemFlags::SET_SELECTED)
                .opened(&mut selected)
                .begin(ui)
        } else {
            item.begin(ui)
        };
        let Some(_item) = shown else {
            return;
        };
        if ui.button(text::reset_defaults()) {
            host.update_settings(|model| self.options.reset_tab(model, tab));
            play(host, UiSound::Confirm);
        }
        ui.separator();
        if tab == options::OptionsTab::Achievements
            && crate::ui::achievements_view::draw_account(ui, host, &mut self.sign_in)
        {
            play(host, UiSound::Confirm);
        }
        // Q-103: whether the game being played holds Run-ahead at 0, read once for the whole tab
        // rather than per row.
        let run_ahead_forced = host
            .session()
            .is_some_and(crate::game_session::GameSession::forces_run_ahead_to_zero);
        // AMD-055 §3 point 6: the style parameters live in a collapsible group, whose rows are
        // drawn only while the style is Enhanced. Read once for the whole tab.
        let style_note = options::style_group_note(host.settings());
        // Whether the rows of the group being drawn are shown at all: a collapsed header hides
        // them, and a Style group in Classic shows none (REQ-SCN-079).
        let mut group_open = true;
        for entry in self.options.entries(tab).to_vec() {
            match entry {
                TabEntry::Group(group) => {
                    group_open = draw_group_heading(ui, group, style_note.as_deref());
                }
                TabEntry::Row(row) => {
                    let id = row.descriptor.id();
                    if !group_open || !options::style_row_visible(id, host.settings()) {
                        continue;
                    }
                    if options::is_folder(&row) {
                        self.draw_folder_row(ui, host, &row);
                    } else {
                        draw_row(ui, host, &row, run_ahead_forced);
                    }
                }
                TabEntry::Compact(compact, cells) => {
                    if group_open {
                        draw_compact_row(ui, host, compact, &cells);
                    }
                }
            }
        }
    }

    /// A folder setting: a field written on Enter or when it loses the focus, and a button
    /// that picks the folder (Rust-only, the ROM folder of the console mode).
    fn draw_folder_row(&mut self, ui: &Ui, host: &mut AppHost, row: &OptionsRow) {
        let id = row.descriptor.id();
        let _id = ui.push_id(row.descriptor.key());
        let stored = host.settings().get(id).as_text().to_owned();
        if !self.folder_editing {
            self.folder_text.clone_from(&stored);
        }
        let default = host.paths().root.join("roms").display().to_string();
        let width = ui.push_item_width(-ui.calc_text_size(text::browse())[0] - 140.0);
        let entered = ui
            .input_text(row.label(), &mut self.folder_text)
            .hint(&default)
            .enter_returns_true(true)
            .build();
        drop(width);
        self.folder_editing = ui.is_item_active();
        let left = ui.is_item_deactivated_after_edit();
        if (entered || left) && self.folder_text.trim() != stored {
            let value = self.folder_text.trim().to_owned();
            host.update_settings(|model| model.set(id, value.as_str()));
            play(host, UiSound::Confirm);
        }
        let hovered = hovered(ui);
        ui.same_line();
        if ui.button(text::browse()) {
            self.open_browser(host, BrowserPurpose::RomsFolder);
        }
        ui.text_disabled(text::roms_folder_hint(&default));
        if hovered {
            draw_tooltip(ui, id);
        }
    }
}

/// Whether the pointer is over the item just drawn, **including** a greyed one: a row the player
/// cannot change is the row whose explanation is worth the most, and `ImGui` reports no hover over
/// a disabled item unless it is asked to (RO-1 point 3).
fn hovered(ui: &Ui) -> bool {
    ui.is_item_hovered_with_flags(ItemHoveredFlags::ALLOW_WHEN_DISABLED)
}

/// The detailed tooltip of a row (RO-1 point 3), wrapped so that a paragraph of a few sentences
/// stays a readable block rather than one line across the screen.
///
/// Built here and nowhere else: `tooltip_of` is called only for the row under the pointer.
fn draw_tooltip(ui: &Ui, id: SettingId) {
    let text = options::tooltip_of(id);
    if text.is_empty() {
        return;
    }
    ui.tooltip(|| {
        let wrap = ui.push_text_wrap_pos_with_pos(TOOLTIP_WIDTH);
        ui.text_wrapped(text);
        drop(wrap);
    });
}

/// The heading of a group, and whether the rows under it are drawn.
///
/// A long family (RO-1: three rows or more) folds away behind a collapsing header; a short one is
/// a separator and a dim title, which is what AMD-043 §6 asked of the Geometry group. `style_note`
/// is [`options::style_group_note`]'s answer: a Style group in Classic draws its heading and that
/// one sentence, and none of its rows (REQ-SCN-079).
fn draw_group_heading(ui: &Ui, group: OptionsGroup, style_note: Option<&str>) -> bool {
    let note = (group == OptionsGroup::Style)
        .then_some(style_note)
        .flatten();
    if group.is_collapsible() {
        // Open by default: a player who has just picked Enhanced is looking for exactly these
        // rows, and a group nobody opens is a group nobody finds. It stays collapsible, which is
        // what AMD-055 §3 point 6 asks for.
        let open = ui.collapsing_header(group.title(), TreeNodeFlags::DEFAULT_OPEN);
        if let Some(note) = note {
            ui.text_disabled(note);
            return false;
        }
        return open;
    }
    ui.separator();
    ui.text_disabled(group.title());
    if let Some(note) = note {
        ui.text_disabled(note);
        return false;
    }
    true
}

/// A compact row (RO-1 point 6): the title of the row, then the four values of the quantity side
/// by side, each under its own caption and each with its own tooltip.
///
/// Every one of the four is still its own setting, edited by its own widget and written through
/// the same path a row of its own used: a change plays the validation sound, the reset button of
/// the tab reaches all four, and the settings file is untouched. Only the place changes.
fn draw_compact_row(ui: &Ui, host: &mut AppHost, compact: CompactRow, cells: &[OptionsRow; 4]) {
    let _id = ui.push_id(compact.title());
    ui.text(compact.title());
    for (cell, caption) in cells.iter().zip(options::COMPACT_CAPTIONS) {
        let id = cell.descriptor.id();
        let _cell = ui.push_id(cell.descriptor.key());
        ui.same_line();
        let width = ui.push_item_width(COMPACT_CELL_WIDTH);
        let changed = draw_enum_labelled(ui, host, cell, &strings::get(caption));
        drop(width);
        let hovered = hovered(ui);
        if let Some(value) = changed {
            host.update_settings(|model| model.set(id, value));
            if options::plays_validation_sound(id) {
                play(host, UiSound::Confirm);
            }
        }
        if hovered {
            draw_tooltip(ui, id);
        }
    }
}

/// One row of a tab. `run_ahead_forced` is Q-103's verdict on the running game, which only the
/// Run-ahead row shows.
fn draw_row(ui: &Ui, host: &mut AppHost, row: &OptionsRow, run_ahead_forced: bool) {
    let descriptor = row.descriptor;
    let id = descriptor.id();
    let _id = ui.push_id(descriptor.key());
    // RO-1 point 1: a row whose setting cannot act while another one is set a certain way is drawn
    // greyed out with the reason beside it, never hidden. The setting itself is kept and persists,
    // and acts again as soon as the other one changes. `options::inert_note` holds every case and
    // names the code path each one was read from.
    let inert_note = options::inert_note(id, host.settings());
    let disabled = ui.begin_disabled(inert_note.is_some());
    // A setting whose domain is a finite list gets a drop-down whatever its shape:
    // video.frame_rate is a whole number out of {60, 90, 120} (SPEC-09 §4.1.1), and SPEC-10
    // REQ-UI-061 shows it as a list.
    let enumerated = descriptor
        .domain()
        .options()
        .is_some_and(|options| !options.is_empty())
        && matches!(descriptor.kind(), SettingKind::Enum | SettingKind::Int);
    let changed = if descriptor.kind() == SettingKind::Text {
        draw_text(ui, host, row)
    } else if options::SWITCHES.contains(&id) {
        // AMD-055 §4 shows `spl.pixel_shadow` and `spl.reflections` as a check box; the model
        // keeps them as the whole numbers 0 and 1, which is what Lua reads and writes.
        draw_switch(ui, host, row)
    } else if enumerated {
        draw_enum(ui, host, row)
    } else {
        match descriptor.kind() {
            SettingKind::Bool => draw_bool(ui, host, row),
            SettingKind::Enum => draw_enum(ui, host, row),
            SettingKind::Color => draw_color(ui, host, row),
            SettingKind::Int => draw_int(ui, host, row),
            _ => draw_float(ui, host, row),
        }
    };
    // RO-1 point 3: the detailed tooltip, on hover over the label or the widget — ImGui's item
    // covers both. It is drawn after the disabled scope is dropped, so a greyed row's explanation
    // is not itself greyed; the hover has to be read here, while the item is still the current one.
    let hovered = hovered(ui);
    if let Some(value) = changed {
        host.update_settings(|model| model.set(id, value));
        if options::plays_validation_sound(id) {
            // AMD-021 R2-013 (replacing the note and the table of REQ-AUD-041): every setting of
            // the page plays the validation sound once per change of value, except Speed and
            // FOV, which are silent.
            play(host, UiSound::Confirm);
        }
    }
    draw_row_hint(ui, row);
    drop(disabled);
    if let Some(note) = inert_note {
        ui.same_line();
        ui.text_disabled(note);
    }
    // Q-103: the Run-ahead row says so while the running game holds it at 0. The row stays live —
    // the setting is kept and applies again to the next game.
    if let Some(note) = options::forced_run_ahead_note(id, run_ahead_forced) {
        ui.same_line();
        ui.text_disabled(note);
    }
    if hovered {
        draw_tooltip(ui, id);
    }
}

/// A whole number of 0 or 1 drawn as a check box (AMD-055 §4).
fn draw_switch(ui: &Ui, host: &AppHost, row: &OptionsRow) -> Option<SettingValue> {
    let mut on = host.settings().get(row.descriptor.id()).as_int() != 0;
    ui.checkbox(row.label(), &mut on)
        .then_some(SettingValue::Int(i32::from(on)))
}

fn draw_row_hint(ui: &Ui, row: &OptionsRow) {
    match row.descriptor.effect() {
        SettingEffect::Restart => {
            ui.same_line();
            ui.text_disabled(text::at_next_start());
        }
        SettingEffect::NextGameLoad | SettingEffect::NextRomLoad => {
            ui.same_line();
            ui.text_disabled(text::at_next_load());
        }
        _ => {}
    }
    if row.is_profile_scoped() {
        ui.same_line();
        ui.text_disabled(text::profile_setting());
    }
}

fn draw_text(ui: &Ui, host: &AppHost, row: &OptionsRow) -> Option<SettingValue> {
    let descriptor = row.descriptor;
    let options = descriptor
        .text_options()
        .filter(|options| !options.is_empty())?;
    let stored = host.settings().get(descriptor.id());
    let current = descriptor.coerce_text(stored.as_text());
    let mut index = options
        .iter()
        .position(|option| *option == current)
        .unwrap_or(0);
    let labels: Vec<String> = options
        .iter()
        .map(|option| text::label_of_option(option))
        .collect();
    ui.combo(row.label(), &mut index, &labels, |label| {
        label.as_str().into()
    })
    .then(|| SettingValue::Text(options.get(index).copied().unwrap_or_default().to_owned()))
}

fn draw_bool(ui: &Ui, host: &AppHost, row: &OptionsRow) -> Option<SettingValue> {
    let mut value = host.settings().get(row.descriptor.id()).as_bool();
    ui.checkbox(row.label(), &mut value)
        .then_some(SettingValue::Bool(value))
}

fn draw_enum(ui: &Ui, host: &AppHost, row: &OptionsRow) -> Option<SettingValue> {
    draw_enum_labelled(ui, host, row, &row.label())
}

/// The same drop-down under a label of the caller's choosing: a compact row (RO-1 point 6) labels
/// each of its cells with a caption instead of with the setting's own label.
fn draw_enum_labelled(
    ui: &Ui,
    host: &AppHost,
    row: &OptionsRow,
    label: &str,
) -> Option<SettingValue> {
    let descriptor = row.descriptor;
    let Some(options) = descriptor
        .domain()
        .options()
        .filter(|options| !options.is_empty())
    else {
        return draw_int(ui, host, row);
    };
    let current = host.settings().get(descriptor.id()).as_double();
    let mut index = options
        .iter()
        .position(|option| (option.value - current).abs() < 1e-9)
        .unwrap_or(0);
    let labels: Vec<String> = options
        .iter()
        .map(|option| text::label_of_option(&option.label))
        .collect();
    if !ui.combo(label, &mut index, &labels, |label| label.as_str().into()) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)] // the C# `(int)options[index].Value`
    let picked = options.get(index).map_or(0, |option| option.value as i32);
    Some(if descriptor.kind() == SettingKind::Int {
        SettingValue::Int(picked)
    } else {
        SettingValue::Enum(picked)
    })
}

#[allow(clippy::cast_possible_truncation)] // the C# `(int)Math.Round(bound)`
fn whole_bounds(row: &OptionsRow) -> (i32, i32) {
    (row.minimum.round() as i32, row.maximum.round() as i32)
}

fn draw_int(ui: &Ui, host: &AppHost, row: &OptionsRow) -> Option<SettingValue> {
    let mut value = host.settings().get(row.descriptor.id()).as_int();
    let changed = if row.maximum > row.minimum {
        let (low, high) = whole_bounds(row);
        ui.slider(row.label(), low, high, &mut value)
    } else {
        ui.input_int(row.label(), &mut value).build()
    };
    changed.then_some(SettingValue::Int(value))
}

fn draw_float(ui: &Ui, host: &AppHost, row: &OptionsRow) -> Option<SettingValue> {
    let mut value = host.settings().get(row.descriptor.id()).as_float();
    if row.maximum <= row.minimum {
        return ui
            .input_float(row.label(), &mut value)
            .build()
            .then_some(SettingValue::Float(value));
    }
    // A step of one or more means the setting is whole in the interface (the speed of REQ-LOOP-015
    // and the field of view of SPEC-10 REQ-UI-061), so the slider is an integer one and the value
    // lands exactly on a step.
    if row.step >= 1.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        // the C# `(int)MathF.Round(value / step) * (int)step`
        let mut whole = (value / row.step as f32).round() as i32 * row.step as i32;
        let (low, high) = whole_bounds(row);
        #[allow(clippy::cast_precision_loss)] // a slider value
        return ui
            .slider(row.label(), low, high, &mut whole)
            .then_some(SettingValue::Float(whole as f32));
    }
    #[allow(clippy::cast_possible_truncation)] // the C# `(float)row.Minimum`
    let (low, high) = (row.minimum as f32, row.maximum as f32);
    ui.slider_config(row.label(), low, high)
        .display_format("%.2f")
        .build(&mut value)
        .then_some(SettingValue::Float(value))
}

fn draw_color(ui: &Ui, host: &AppHost, row: &OptionsRow) -> Option<SettingValue> {
    let stored = host.settings().get(row.descriptor.id()).as_color();
    let mut value = [stored.r, stored.g, stored.b, stored.a];
    ui.color_edit4_config(row.label(), &mut value)
        .flags(ColorEditFlags::NO_INPUTS | ColorEditFlags::ALPHA_PREVIEW)
        .build()
        .then(|| SettingValue::Color(Color::new(value[0], value[1], value[2], value[3])))
}
