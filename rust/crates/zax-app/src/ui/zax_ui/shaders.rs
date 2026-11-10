//! The shaders window (Rust-only, owner request): one tab per display mode (2D, 2D widescreen,
//! 3D, 3D widescreen), each with its slang preset picked from the shader pack Zaxnes carries
//! (the recommended ones of the mode as buttons, a menu of the catalogue by family, and a menu of
//! every preset of the pack), a copy from another tab, the lines and area of the picture, and a
//! slider per parameter of the preset.

use imgui::{Condition, TabItem, Ui, WindowFlags};
use zax_settings::catalogue::SettingId;
#[cfg(feature = "shaders")]
use zax_settings::shader_slots;
use zax_settings::shader_slots::{
    ShaderSlot, AREA_AUTO, AREA_PICTURE, AREA_WINDOW, FOLLOW, OFF, PRESET,
};
use zax_settings::values::SettingValue;

use super::{centre, play, ZaxUi};
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::shader_pack;
use crate::ui::shader_panel::{self, Family, PresetFolder, ShaderState, CATALOGUE};
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
    /// The filter of the parameter list.
    #[cfg(feature = "shaders")]
    filter: String,
    /// The presets of the pack, read when their menu first opens.
    pack: Option<PresetFolder>,
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

/// The name a stored preset is shown by: its catalogue name, or its path in the pack.
fn preset_label(stored: &str) -> String {
    shader_panel::catalogue_entry(stored)
        .map_or_else(|| stored.to_owned(), |preset| tr(preset.label))
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

/// The menu of a folder of the pack: its folders as submenus, then its presets; the path of
/// the preset clicked.
fn folder_menu(ui: &Ui, folder: &PresetFolder, stored: &str) -> Option<String> {
    let mut chosen = None;
    for child in &folder.folders {
        if let Some(_menu) = ui.begin_menu(&child.name) {
            if let Some(path) = folder_menu(ui, child, stored) {
                chosen = Some(path);
            }
        }
    }
    for (name, path) in &folder.presets {
        if ui
            .menu_item_config(format!("{name}##{path}"))
            .selected(path == stored)
            .build()
        {
            chosen = Some(path.clone());
        }
    }
    chosen
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
            if !cfg!(feature = "shaders") {
                ui.text_wrapped(tr("shaders.not_built"));
            } else if !shader_pack::carried() {
                ui.text_wrapped(tr("shaders.pack_missing"));
            } else {
                self.draw_shader_contents(ui, host);
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
            .map(|lines| match *lines {
                0 => tr("shaders.lines.window"),
                240 => tr("shaders.lines.console"),
                lines => lines.to_string(),
            })
            .collect();
        if let Some(value) = choice(ui, &tr("shaders.lines"), lines, &LINE_CHOICES, &line_labels) {
            host.update_settings(|model| model.set(ids.lines, SettingValue::Int(value)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(tr("shaders.lines.help"));
        }
        ui.separator();
        Self::draw_quick_row(ui, host, slot);
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

    /// The presets the catalogue recommends for the mode of `slot`, one button each; a click
    /// gives the slot that preset.
    fn draw_quick_row(ui: &Ui, host: &mut AppHost, slot: ShaderSlot) {
        let ids = slot.ids();
        let using = host.settings().get(ids.mode).as_int() == PRESET;
        let stored = host.settings().get(ids.preset).as_text().to_owned();
        ui.text(tr("shaders.quick"));
        for (index, preset) in CATALOGUE
            .iter()
            .filter(|preset| shader_panel::featured_for(preset, slot))
            .enumerate()
        {
            ui.same_line();
            let chosen = using && stored == preset.path;
            let _style = chosen
                .then(|| ui.push_style_color(imgui::StyleColor::Button, [0.25, 0.5, 0.3, 1.0]));
            if ui.button(format!("{}##quick{index}", tr(preset.label))) && !chosen {
                host.update_settings(|model| {
                    shader_panel::choose_preset(model, slot, preset.path);
                });
                play(host, UiSound::Confirm);
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(format!(
                    "{}\n{}",
                    tr(&format!("{}.help", preset.label)),
                    tr(preset.cost.label())
                ));
            }
        }
    }

    /// The preset of the slot, and the menu that changes it.
    fn draw_preset_menu(&mut self, ui: &Ui, host: &mut AppHost, slot: ShaderSlot) {
        let ids = slot.ids();
        let stored = host.settings().get(ids.preset).as_text().to_owned();
        let shown = if stored.is_empty() {
            tr("shaders.preset.choose")
        } else {
            preset_label(&stored)
        };
        let combo = ui.begin_combo(tr("shaders.preset"), &shown);
        if combo.is_none() && ui.is_item_hovered() && !stored.is_empty() {
            ui.tooltip_text(&stored);
        }
        let Some(_combo) = combo else {
            return;
        };
        let mut chosen: Option<String> = None;
        ui.text_disabled(tr("shaders.menu.recommended"));
        for preset in CATALOGUE
            .iter()
            .filter(|preset| shader_panel::featured_for(preset, slot))
        {
            if ui
                .menu_item_config(format!("{}##rec{}", tr(preset.label), preset.path))
                .selected(stored == preset.path)
                .build()
            {
                chosen = Some(preset.path.to_owned());
            }
        }
        ui.separator();
        for family in Family::ALL {
            let Some(_menu) = ui.begin_menu(tr(family.label())) else {
                continue;
            };
            for preset in CATALOGUE.iter().filter(|preset| preset.family == family) {
                let clicked = ui
                    .menu_item_config(format!("{}##cat{}", tr(preset.label), preset.path))
                    .shortcut(tr(preset.cost.label()))
                    .selected(stored == preset.path)
                    .build();
                if ui.is_item_hovered() {
                    ui.tooltip_text(tr(&format!("{}.help", preset.label)));
                }
                if clicked {
                    chosen = Some(preset.path.to_owned());
                }
            }
        }
        ui.separator();
        if let Some(_menu) = ui.begin_menu(tr("shaders.menu.all")) {
            let pack = self.shader_window.pack.get_or_insert_with(|| {
                PresetFolder::read(&shader_pack::folder(&host.paths().root), "")
            });
            if pack.is_empty() {
                ui.text_disabled(tr("shaders.menu.empty"));
            } else if let Some(path) = folder_menu(ui, pack, &stored) {
                chosen = Some(path);
            }
        }
        if let Some(path) = chosen {
            if path != stored {
                host.update_settings(|model| shader_panel::choose_preset(model, slot, &path));
                play(host, UiSound::Confirm);
            }
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
        self.draw_preset_menu(ui, host, slot);
        let stored = host.settings().get(ids.preset).as_text().to_owned();
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
        let file = shader_pack::resolve(&host.paths().root, &stored)
            .display()
            .to_string();
        if !stored.is_empty() && report.preset == file {
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
            self.draw_parameters(ui, host, slot, &stored, &file);
        }
    }

    #[cfg(not(feature = "shaders"))]
    #[allow(clippy::unused_self)]
    fn draw_parameters(&mut self, _: &Ui, _: &mut AppHost, _: ShaderSlot, _: &str, _: &str) {}

    #[cfg(feature = "shaders")]
    #[allow(clippy::too_many_lines)] // The list of sliders and its buttons.
    fn draw_parameters(
        &mut self,
        ui: &Ui,
        host: &mut AppHost,
        slot: ShaderSlot,
        stored: &str,
        file: &str,
    ) {
        let ids = slot.ids();
        self.shader_window
            .parameters
            .want(std::path::Path::new(file));
        let overrides = shader_slots::parse_parameters(host.settings().get(ids.params).as_text());
        let starting = shader_slots::parse_parameters(&shader_panel::starting_parameters(stored));
        let changed_count = overrides
            .iter()
            .filter(|entry| !starting.contains(entry))
            .count()
            + starting
                .iter()
                .filter(|(name, _)| !overrides.iter().any(|(known, _)| known == name))
                .count();
        let title = strings::format(&tr("shaders.parameters"), &[&changed_count.to_string()]);
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
        if ui.button(tr("shaders.reset_all")) && changed_count > 0 {
            let text = shader_panel::starting_parameters(stored);
            host.update_settings(|model| model.set(ids.params, text.as_str()));
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
            // The value a reset returns to: the catalogue's, or the preset's own.
            let default = shader_panel::parameter_value(&starting, &info.name, info.initial);
            let current = shader_panel::parameter_value(&overrides, &info.name, info.initial);
            let mut value = current;
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
                    default
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
            if (current - default).abs() > f32::EPSILON {
                ui.same_line();
                if ui.small_button(tr("shaders.reset")) {
                    let kept =
                        ((default - info.initial).abs() > f32::EPSILON).then_some(default);
                    text = shader_slots::with_parameter(&text, &info.name, kept);
                    changed = true;
                }
            }
        }
        if changed {
            host.update_settings(|model| model.set(ids.params, text.as_str()));
        }
    }
}
