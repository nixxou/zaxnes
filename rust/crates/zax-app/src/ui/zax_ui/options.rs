//! The options window (REQ-UI-202): one tab per theme, one widget per setting.

use imgui::{ColorEditFlags, Condition, TabItem, TabItemFlags, Ui, WindowFlags};
use zax_settings::meta::{SettingEffect, SettingKind};
use zax_settings::values::{Color, SettingValue};

use super::{centre, play, BrowserPurpose, ZaxUi};
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::persistence::app_settings::AppSettings as _;
use crate::render_scaling;
use crate::ui::options::{self, OptionsRow};
use crate::ui::text;
use zax_settings::catalogue::SettingId;

impl ZaxUi {
    pub(super) fn draw_options(&mut self, ui: &Ui, host: &mut AppHost) {
        let mut open = self.state.options;
        let window = ui
            .window(text::options_title())
            .size([560.0, 520.0], Condition::FirstUseEver)
            .position(centre(ui, 560.0, 520.0), Condition::FirstUseEver)
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if let Some(_window) = window {
            if let Some(_bar) = ui.tab_bar("##options") {
                // The capture aid of --ui-shots: asking for a tab is the same thing a click on it
                // would do. It is honoured on one frame only, so the tab the shot photographs has
                // no stray close button left on it.
                let forced = self.force_options_tab.take();
                for tab in options::TABS {
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
                        continue;
                    };
                    if ui.button(text::reset_defaults()) {
                        host.update_settings(|model| self.options.reset_tab(model, tab));
                        play(host, UiSound::Confirm);
                    }
                    ui.separator();
                    for row in self.options.rows(tab).to_vec() {
                        if options::is_folder(&row) {
                            self.draw_folder_row(ui, host, &row);
                        } else {
                            draw_row(ui, host, &row);
                        }
                    }
                }
            }
        }
        self.state.options = open;
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
        ui.same_line();
        if ui.button(text::browse()) {
            self.open_browser(host, BrowserPurpose::RomsFolder);
        }
        ui.text_disabled(text::roms_folder_hint(&default));
    }
}

fn draw_row(ui: &Ui, host: &mut AppHost, row: &OptionsRow) {
    let descriptor = row.descriptor;
    let id = descriptor.id();
    let _id = ui.push_id(descriptor.key());
    // AMD-032 §3: the resolution scale only exists for the render-texture method; under the
    // direct method the scene is always drawn at the native resolution, so the row is shown
    // greyed out rather than hidden, with the reason spelled out beside it.
    let inert = id == SettingId::VideoResScale
        && !render_scaling::scale_applies(host.settings().render_method());
    let disabled = ui.begin_disabled(inert);
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
    if inert {
        ui.same_line();
        ui.text_disabled(text::res_scale_needs_render_texture());
    }
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
    if !ui.combo(row.label(), &mut index, &labels, |label| {
        label.as_str().into()
    }) {
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
