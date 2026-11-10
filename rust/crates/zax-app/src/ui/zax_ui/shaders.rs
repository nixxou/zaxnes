//! The shaders window (Rust-only, owner request): one tab per display mode (2D, 2D widescreen,
//! 3D, 3D widescreen), each with its slang preset, the quick presets, a copy from another tab,
//! the lines and area of the picture, and a slider per parameter of the preset.

use imgui::{Condition, TabItem, Ui, WindowFlags};
use zax_settings::catalogue::SettingId;
#[cfg(feature = "shaders")]
use zax_settings::shader_slots;
use zax_settings::shader_slots::{
    ShaderSlot, AREA_AUTO, AREA_PICTURE, AREA_WINDOW, FOLLOW, OFF, PRESET,
};
use zax_settings::values::SettingValue;

use super::{centre, play, BrowserPurpose, ZaxUi};
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::ui::shader_panel::{self, ShaderState};
use crate::ui::strings;

/// The window's default size.
const WINDOW_SIZE: [f32; 2] = [700.0, 640.0];

/// The line counts the lines choice offers (0: the window's own).
const LINE_CHOICES: [i32; 5] = [0, 240, 480, 720, 1080];

const GREEN: [f32; 4] = [0.45, 0.85, 0.45, 1.0];
const YELLOW: [f32; 4] = [0.95, 0.8, 0.35, 1.0];
const RED: [f32; 4] = [0.95, 0.45, 0.4, 1.0];
#[cfg(feature = "shaders")]
const HEADING: [f32; 4] = [0.6, 0.75, 1.0, 1.0];

/// The fields of the window that live between frames.
#[derive(Default)]
pub struct ShaderWindow {
    /// The preset field of the tab shown, while it is being typed.
    preset_text: String,
    preset_editing: bool,
    /// The folder field, while it is being typed.
    library_text: String,
    library_editing: bool,
    /// The filter of the parameter list.
    #[cfg(feature = "shaders")]
    filter: String,
    /// The slot whose preset the file dialog chooses.
    browsing: Option<ShaderSlot>,
    /// The parameters of the preset shown.
    #[cfg(feature = "shaders")]
    parameters: shader_panel::ParameterCache,
}

fn tr(key: &str) -> String {
    strings::get(key)
}

fn slot_label(slot: ShaderSlot) -> String {
    tr(match slot {
        ShaderSlot::TwoD => "shaders.slot.2d",
        ShaderSlot::TwoDWide => "shaders.slot.2d_ws",
        ShaderSlot::ThreeD => "shaders.slot.3d",
        ShaderSlot::ThreeDWide => "shaders.slot.3d_ws",
    })
}

/// A combo over `values` with `labels`; the value chosen when it changed.
fn choice(ui: &Ui, label: &str, current: i32, values: &[i32], labels: &[String]) -> Option<i32> {
    let mut index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    if ui.combo(label, &mut index, labels, |label| label.as_str().into()) {
        return values.get(index).copied().filter(|value| *value != current);
    }
    None
}

impl ZaxUi {
    pub(super) fn draw_shaders(&mut self, ui: &Ui, host: &mut AppHost) {
        let mut open = self.state.shaders;
        let window = ui
            .window(tr("shaders.title"))
            .size(WINDOW_SIZE, Condition::FirstUseEver)
            .position(
                centre(ui, WINDOW_SIZE[0], WINDOW_SIZE[1]),
                Condition::FirstUseEver,
            )
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if let Some(_window) = window {
            if cfg!(feature = "shaders") {
                self.draw_shader_contents(ui, host);
            } else {
                ui.text_wrapped(tr("shaders.not_built"));
            }
        }
        self.state.shaders = open;
    }

    fn draw_shader_contents(&mut self, ui: &Ui, host: &mut AppHost) {
        let report = host.shader_report.clone();
        if report.state == ShaderState::NoWgpu {
            ui.text_colored(YELLOW, tr("shaders.needs_wgpu"));
            if host.settings().get(SettingId::VideoBackend).as_int() == 0 {
                if ui.button(tr("shaders.use_vulkan")) {
                    host.update_settings(|model| {
                        model.set(SettingId::VideoBackend, SettingValue::Enum(1))
                    });
                    play(host, UiSound::Confirm);
                }
            } else {
                ui.text_wrapped(tr("shaders.restart"));
            }
            ui.separator();
        }
        self.draw_library_row(ui, host);
        ui.separator();
        if let Some(_bar) = ui.tab_bar("##shader_slots") {
            for slot in ShaderSlot::ALL {
                let mut label = slot_label(slot);
                if report.mode_slot == Some(slot) {
                    label = strings::format(&tr("shaders.slot.on_screen"), &[&label]);
                }
                let Some(_tab) = TabItem::new(format!("{label}##{}", slot.label())).begin(ui)
                else {
                    continue;
                };
                self.draw_slot(ui, host, slot, &report);
            }
        }
    }

    fn draw_library_row(&mut self, ui: &Ui, host: &mut AppHost) {
        let stored = host
            .settings()
            .get(SettingId::ShaderLibrary)
            .as_text()
            .to_owned();
        if !self.shader_window.library_editing {
            self.shader_window.library_text.clone_from(&stored);
        }
        let found = shader_panel::library_folder(host.settings(), &host.paths().root);
        let hint = found.as_ref().map_or_else(
            || tr("shaders.library_hint"),
            |path| path.display().to_string(),
        );
        let width = ui.push_item_width(-ui.calc_text_size(tr("shaders.browse"))[0] - 160.0);
        let entered = ui
            .input_text(tr("shaders.library"), &mut self.shader_window.library_text)
            .hint(&hint)
            .enter_returns_true(true)
            .build();
        drop(width);
        self.shader_window.library_editing = ui.is_item_active();
        let left = ui.is_item_deactivated_after_edit();
        if (entered || left) && self.shader_window.library_text.trim() != stored {
            let value = self.shader_window.library_text.trim().to_owned();
            host.update_settings(|model| model.set(SettingId::ShaderLibrary, value.as_str()));
        }
        ui.same_line();
        if ui.button(format!("{}##library", tr("shaders.browse"))) {
            self.open_browser(host, BrowserPurpose::ShaderLibrary);
        }
        if found.is_none() {
            ui.text_wrapped(tr("shaders.library_missing"));
        }
    }

    fn draw_slot(
        &mut self,
        ui: &Ui,
        host: &mut AppHost,
        slot: ShaderSlot,
        report: &shader_panel::ShaderReport,
    ) {
        let ids = slot.ids();
        let mode = host.settings().get(ids.mode).as_int();
        let (values, labels): (Vec<i32>, Vec<String>) = match slot.fallback() {
            None => (
                vec![OFF, PRESET],
                vec![tr("shaders.mode.off"), tr("shaders.mode.preset")],
            ),
            Some(parent) => (
                vec![FOLLOW, OFF, PRESET],
                vec![
                    strings::format(&tr("shaders.mode.follow"), &[&slot_label(parent)]),
                    tr("shaders.mode.off"),
                    tr("shaders.mode.preset"),
                ],
            ),
        };
        if let Some(value) = choice(ui, &tr("shaders.mode"), mode, &values, &labels) {
            host.update_settings(|model| model.set(ids.mode, SettingValue::Enum(value)));
        }
        Self::draw_copy_row(ui, host, slot);
        let lines = host.settings().get(ids.lines).as_int();
        let line_labels: Vec<String> = LINE_CHOICES
            .iter()
            .map(|lines| {
                if *lines == 0 {
                    tr("shaders.lines.window")
                } else {
                    lines.to_string()
                }
            })
            .collect();
        if let Some(value) = choice(ui, &tr("shaders.lines"), lines, &LINE_CHOICES, &line_labels) {
            host.update_settings(|model| model.set(ids.lines, SettingValue::Int(value)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(tr("shaders.lines.help"));
        }
        ui.separator();
        match host.settings().get(ids.mode).as_int() {
            FOLLOW => {
                if let Some(parent) = slot.fallback() {
                    ui.text_wrapped(strings::format(
                        &tr("shaders.follows"),
                        &[&slot_label(parent)],
                    ));
                }
            }
            PRESET => self.draw_preset(ui, host, slot, report),
            _ => ui.text_disabled(tr("shaders.none")),
        }
    }

    fn draw_copy_row(ui: &Ui, host: &mut AppHost, slot: ShaderSlot) {
        let others: Vec<ShaderSlot> = ShaderSlot::ALL
            .into_iter()
            .filter(|other| *other != slot)
            .collect();
        let mut labels = vec![tr("shaders.copy.none")];
        labels.extend(others.iter().map(|other| slot_label(*other)));
        let mut index = 0;
        if ui.combo(tr("shaders.copy"), &mut index, &labels, |label| {
            label.as_str().into()
        }) {
            if let Some(from) = index.checked_sub(1).and_then(|index| others.get(index)) {
                host.update_settings(|model| shader_panel::copy_slot(model, *from, slot));
                play(host, UiSound::Confirm);
            }
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(tr("shaders.copy.help"));
        }
    }

    fn draw_preset(
        &mut self,
        ui: &Ui,
        host: &mut AppHost,
        slot: ShaderSlot,
        report: &shader_panel::ShaderReport,
    ) {
        let ids = slot.ids();
        let library = shader_panel::library_folder(host.settings(), &host.paths().root);
        let quick = shader_panel::available_quick_presets(library.as_deref());
        let stored = host.settings().get(ids.preset).as_text().to_owned();
        if !quick.is_empty() {
            ui.text(tr("shaders.quick"));
            for (index, (preset, path)) in quick.iter().enumerate() {
                if index > 0 {
                    ui.same_line();
                }
                let path = path.display().to_string();
                let label = format!("{}##quick{index}", tr(preset.label));
                let chosen = path == stored;
                let _style = chosen
                    .then(|| ui.push_style_color(imgui::StyleColor::Button, [0.25, 0.5, 0.3, 1.0]));
                if ui.button(label) && !chosen {
                    host.update_settings(|model| {
                        model.set(ids.preset, path.as_str());
                        model.set(ids.params, "");
                    });
                    play(host, UiSound::Confirm);
                }
            }
        }
        if !self.shader_window.preset_editing {
            self.shader_window.preset_text =
                shader_panel::display_path(&stored, library.as_deref());
        }
        let width = ui.push_item_width(-ui.calc_text_size(tr("shaders.browse"))[0] - 120.0);
        let entered = ui
            .input_text(tr("shaders.preset"), &mut self.shader_window.preset_text)
            .enter_returns_true(true)
            .build();
        drop(width);
        self.shader_window.preset_editing = ui.is_item_active();
        let left = ui.is_item_deactivated_after_edit();
        if entered || left {
            let typed = self.shader_window.preset_text.trim().to_owned();
            let full = match &library {
                Some(library) if !std::path::Path::new(&typed).is_absolute() => {
                    library.join(&typed).display().to_string()
                }
                _ => typed,
            };
            if full != stored {
                host.update_settings(|model| {
                    model.set(ids.preset, full.as_str());
                    model.set(ids.params, "");
                });
            }
        }
        ui.same_line();
        if ui.button(format!("{}##preset", tr("shaders.browse"))) {
            self.shader_window.browsing = Some(slot);
            self.open_browser(host, BrowserPurpose::ShaderPreset);
        }
        let area = host.settings().get(ids.area).as_int();
        if let Some(value) = choice(
            ui,
            &tr("shaders.area"),
            area,
            &[AREA_AUTO, AREA_PICTURE, AREA_WINDOW],
            &[
                tr("shaders.area.auto"),
                tr("shaders.area.picture"),
                tr("shaders.area.window"),
            ],
        ) {
            host.update_settings(|model| model.set(ids.area, SettingValue::Enum(value)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(tr("shaders.area.help"));
        }
        if !stored.is_empty() && report.preset == stored {
            match report.state {
                ShaderState::Compiling => ui.text_colored(YELLOW, tr("shaders.state.compiling")),
                ShaderState::Active => ui.text_colored(GREEN, tr("shaders.state.active")),
                ShaderState::Refused => {
                    ui.text_colored(RED, tr("shaders.state.refused"));
                    ui.text_wrapped(&report.problem);
                }
                ShaderState::None | ShaderState::NoWgpu => {}
            }
        }
        if !stored.is_empty() {
            self.draw_parameters(ui, host, slot, &stored);
        }
    }

    #[cfg(not(feature = "shaders"))]
    #[allow(clippy::unused_self)]
    fn draw_parameters(&mut self, _ui: &Ui, _host: &mut AppHost, _slot: ShaderSlot, _: &str) {}

    #[cfg(feature = "shaders")]
    fn draw_parameters(&mut self, ui: &Ui, host: &mut AppHost, slot: ShaderSlot, preset: &str) {
        let ids = slot.ids();
        self.shader_window
            .parameters
            .want(std::path::Path::new(preset));
        let overrides = shader_slots::parse_parameters(host.settings().get(ids.params).as_text());
        let title = strings::format(&tr("shaders.parameters"), &[&overrides.len().to_string()]);
        if !ui.collapsing_header(
            format!("{title}###parameters"),
            imgui::TreeNodeFlags::DEFAULT_OPEN,
        ) {
            return;
        }
        let parameters = match self.shader_window.parameters.result() {
            None => {
                ui.text_disabled(tr("shaders.parameters.reading"));
                return;
            }
            Some(Err(problem)) => {
                ui.text_colored(RED, problem);
                return;
            }
            Some(Ok(parameters)) => parameters.clone(),
        };
        if parameters.is_empty() {
            ui.text_disabled(tr("shaders.parameters.none"));
            return;
        }
        let width = ui.push_item_width(-ui.calc_text_size(tr("shaders.reset_all"))[0] - 90.0);
        ui.input_text(tr("shaders.filter"), &mut self.shader_window.filter)
            .build();
        drop(width);
        ui.same_line();
        if ui.button(tr("shaders.reset_all")) && !overrides.is_empty() {
            host.update_settings(|model| model.set(ids.params, ""));
            play(host, UiSound::Confirm);
        }
        let filter = self.shader_window.filter.trim().to_lowercase();
        let Some(_child) = ui.child_window("##parameters").begin() else {
            return;
        };
        let mut text = host.settings().get(ids.params).as_text().to_owned();
        let mut changed = false;
        for info in &parameters {
            if !filter.is_empty()
                && !info.description.to_lowercase().contains(&filter)
                && !info.name.to_lowercase().contains(&filter)
            {
                continue;
            }
            if info.is_heading() {
                if !info.description.is_empty() {
                    ui.text_colored(HEADING, &info.description);
                }
                continue;
            }
            let current = shader_panel::parameter_value(&overrides, &info.name, info.initial);
            let mut value = current;
            let modified = overrides.iter().any(|(name, _)| *name == info.name);
            let _id = ui.push_id(info.name.as_str());
            let width = ui.push_item_width(-150.0);
            let label = if info.description.is_empty() {
                info.name.clone()
            } else {
                info.description.clone()
            };
            let moved = ui
                .slider_config(&label, info.minimum, info.maximum)
                .display_format(shader_panel::slider_format(info.step))
                .build(&mut value);
            drop(width);
            if ui.is_item_hovered() {
                ui.tooltip_text(format!(
                    "{}\n{} {}",
                    info.name,
                    tr("shaders.parameters.default"),
                    info.initial
                ));
            }
            if moved {
                let value = shader_panel::snap(value, info.minimum, info.maximum, info.step);
                if (value - current).abs() > f32::EPSILON {
                    let kept = ((value - info.initial).abs() > f32::EPSILON).then_some(value);
                    text = shader_slots::with_parameter(&text, &info.name, kept);
                    changed = true;
                }
            }
            if modified {
                ui.same_line();
                if ui.small_button(tr("shaders.reset")) {
                    text = shader_slots::with_parameter(&text, &info.name, None);
                    changed = true;
                }
            }
        }
        if changed {
            host.update_settings(|model| model.set(ids.params, text.as_str()));
        }
    }

    /// The file dialog answered with the preset of the slot it was opened for.
    pub(super) fn shader_preset_chosen(&mut self, host: &mut AppHost, path: &str) {
        let Some(slot) = self.shader_window.browsing.take() else {
            return;
        };
        let ids = slot.ids();
        host.update_settings(|model| {
            model.set(ids.mode, SettingValue::Enum(PRESET));
            model.set(ids.preset, path);
            model.set(ids.params, "");
        });
    }
}
