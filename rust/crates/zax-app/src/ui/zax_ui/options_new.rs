//! The new options window (lot REWRITE-UI, `docs/progress/REWRITE-UI.md`): a simple mode (the
//! Essentials tab and the Controllers tab) and an advanced mode (every tab of the classic
//! window as well). On Essentials, a performance ↔ fidelity slider of five presets, a box saying
//! what the chosen one turns on, and the preset's own check boxes. The classic window stays,
//! one button away, so that the two can be compared.

use imgui::{Condition, StyleColor, TabBarFlags, TabItem, Ui, WindowFlags};
use zax_settings::catalogue::SettingId;
use zax_settings::meta::SettingEffect;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::{centre, play, ZaxUi};
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::ui::options;
use crate::ui::{strings, text};

/// The window's default size, as the classic one's.
const WINDOW_SIZE: [f32; 2] = [820.0, 640.0];

/// The accent of the interface (the console mode's yellow).
const ACCENT: [f32; 4] = [1.0, 0.835, 0.18, 1.0];

/// The amber of a "takes effect later" note.
const LATER: [f32; 4] = [1.0, 0.82, 0.48, 1.0];

/// One preset of the slider: the values it sets, and the check boxes it shows under it (each
/// with the value the preset gives it).
struct Preset {
    /// Its key in the string tables: `options.preset.<key>.name`, `.summary`, `.acts`.
    key: &'static str,
    /// The values the preset writes, the check boxes' included.
    values: &'static [(SettingId, bool)],
    /// The check boxes shown under the preset.
    boxes: &'static [SettingId],
}

/// The five presets, from the fastest to the most faithful. The middle one is the defaults.
const PRESETS: [Preset; 5] = [
    Preset {
        key: "fastest",
        values: &[
            (SettingId::EmuFastCore, true),
            (SettingId::EmuFidelityPlus, false),
            (SettingId::EmuLuaOptimized, true),
            (SettingId::EmuCatchUpLast3d, true),
            (SettingId::EmuExactRate, false),
        ],
        boxes: &[SettingId::EmuFastCore, SettingId::EmuLuaOptimized],
    },
    Preset {
        key: "fast",
        values: &[
            (SettingId::EmuFastCore, true),
            (SettingId::EmuFidelityPlus, false),
            (SettingId::EmuLuaOptimized, false),
            (SettingId::EmuCatchUpLast3d, false),
            (SettingId::EmuExactRate, false),
        ],
        boxes: &[SettingId::EmuFastCore, SettingId::EmuLuaOptimized],
    },
    Preset {
        key: "balanced",
        values: &[
            (SettingId::EmuFastCore, false),
            (SettingId::EmuFidelityPlus, false),
            (SettingId::EmuLuaOptimized, false),
            (SettingId::EmuCatchUpLast3d, false),
            (SettingId::EmuExactRate, false),
            (SettingId::EmuRealtimeInput, true),
        ],
        boxes: &[SettingId::EmuRealtimeInput],
    },
    Preset {
        key: "faithful",
        values: &[
            (SettingId::EmuFastCore, false),
            (SettingId::EmuFidelityPlus, false),
            (SettingId::EmuExactRate, true),
            (SettingId::EmuRealtimeInput, true),
            (SettingId::EmuLatePresent, false),
        ],
        boxes: &[SettingId::EmuFidelityPlus, SettingId::EmuLatePresent],
    },
    Preset {
        key: "most_faithful",
        values: &[
            (SettingId::EmuFastCore, false),
            (SettingId::EmuFidelityPlus, true),
            (SettingId::EmuExactRate, true),
            (SettingId::EmuRealtimeInput, true),
            (SettingId::EmuOamDecay, false),
            (SettingId::EmuOamaddrWriteCorruption, false),
        ],
        boxes: &[
            SettingId::EmuFidelityPlus,
            SettingId::EmuOamDecay,
            SettingId::EmuOamaddrWriteCorruption,
        ],
    },
];

/// The settings of the Essentials tab, under the slider.
const ESSENTIALS: [SettingId; 5] = [
    SettingId::UiFullscreen,
    SettingId::VideoWidescreen,
    SettingId::AudioVolume,
    SettingId::EmuRunAhead,
    SettingId::UiLanguage,
];

/// Whether the settings still hold every value `preset` writes.
fn matches(model: &SettingsModel, preset: &Preset) -> bool {
    preset
        .values
        .iter()
        .all(|&(id, value)| model.get(id).as_bool() == value)
}

/// The note of a setting that takes effect later; none for one that acts at once.
fn later_note(id: SettingId) -> Option<String> {
    match zax_settings::catalogue::descriptor(id).effect() {
        SettingEffect::NextGameLoad | SettingEffect::NextRomLoad => {
            Some(strings::get("options.new.next_game"))
        }
        SettingEffect::Restart => Some(strings::get("options.new.next_start")),
        SettingEffect::Immediate | SettingEffect::NoEffect => None,
    }
}

impl ZaxUi {
    /// The new options window.
    pub(super) fn draw_options_new(&mut self, ui: &Ui, host: &mut AppHost) {
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
            ui.checkbox(
                strings::get("options.new.advanced"),
                &mut self.options_advanced,
            );
            ui.same_line();
            ui.text_disabled(if self.options_advanced {
                strings::get("options.new.advanced_line")
            } else {
                strings::get("options.new.simple_line")
            });
            let label = strings::get("options.new.to_classic");
            let width = ui.calc_text_size(&label)[0] + 16.0;
            ui.same_line_with_pos(ui.window_content_region_max()[0] - width);
            if ui.button(&label) {
                self.options_classic = true;
            }
            ui.separator();
            if let Some(_bar) =
                ui.tab_bar_with_flags("##options_new", TabBarFlags::FITTING_POLICY_SCROLL)
            {
                if let Some(_tab) =
                    TabItem::new(strings::get("options.new.tab.essentials")).begin(ui)
                {
                    self.draw_essentials(ui, host);
                }
                if self.options_advanced {
                    for tab in options::TABS {
                        self.draw_options_tab(ui, host, tab, None);
                    }
                }
                if let Some(_tab) =
                    TabItem::new(strings::get("options.new.tab.controllers")).begin(ui)
                {
                    ui.text_wrapped(strings::get("options.new.controllers_line"));
                    ui.spacing();
                    if ui.button(strings::get("options.new.controllers_open")) {
                        self.state.key_config = true;
                        play(host, UiSound::Confirm);
                    }
                }
            }
        }
        self.state.options = open;
    }

    /// The Essentials tab: the slider, its box and check boxes, then the essential settings.
    fn draw_essentials(&mut self, ui: &Ui, host: &mut AppHost) {
        ui.spacing();
        ui.text(strings::get("options.new.preset_title"));
        let chosen = self.options_preset.min(PRESETS.len() - 1);
        let modified = !matches(host.settings(), &PRESETS[chosen]);
        ui.same_line();
        let name = strings::get(&format!("options.preset.{}.name", PRESETS[chosen].key));
        ui.text_colored(ACCENT, &name);
        if modified {
            ui.same_line();
            ui.text_disabled(strings::get("options.new.modified"));
        }

        // The slider: five steps side by side, the chosen one in the accent.
        let gap = 6.0;
        let full = ui.content_region_avail()[0];
        #[allow(clippy::cast_precision_loss)] // Five steps.
        let step_width = (full - gap * (PRESETS.len() as f32 - 1.0)) / PRESETS.len() as f32;
        for (index, preset) in PRESETS.iter().enumerate() {
            if index > 0 {
                ui.same_line_with_spacing(0.0, gap);
            }
            let selected = index == chosen;
            let colors = selected.then(|| {
                (
                    ui.push_style_color(StyleColor::Button, ACCENT),
                    ui.push_style_color(StyleColor::ButtonHovered, ACCENT),
                    ui.push_style_color(StyleColor::Text, [0.08, 0.08, 0.12, 1.0]),
                )
            });
            let label = strings::get(&format!("options.preset.{}.name", preset.key));
            if ui.button_with_size(format!("{label}##preset{index}"), [step_width, 34.0]) {
                self.options_preset = index;
                host.update_settings(|model| {
                    for &(id, value) in preset.values {
                        model.set(id, SettingValue::Bool(value));
                    }
                });
                play(host, UiSound::Confirm);
            }
            drop(colors);
        }
        ui.text_disabled(strings::get("options.new.faster"));
        let right = strings::get("options.new.truer");
        ui.same_line_with_pos(ui.window_content_region_max()[0] - ui.calc_text_size(&right)[0]);
        ui.text_disabled(&right);
        ui.spacing();

        // What the preset turns on.
        let preset = &PRESETS[chosen];
        let summary = strings::get(&format!("options.preset.{}.summary", preset.key));
        let acts = strings::get(&format!("options.preset.{}.acts", preset.key));
        let lines = acts.lines().count();
        #[allow(clippy::cast_precision_loss)] // A few lines.
        let height = ui.text_line_height_with_spacing() * (lines as f32 + 3.0) + 12.0;
        if let Some(_child) = ui
            .child_window("##preset_box")
            .size([0.0, height])
            .border(true)
            .begin()
        {
            ui.text_wrapped(&summary);
            for act in acts.lines() {
                ui.bullet_text(act);
            }
        }
        ui.spacing();

        // The preset's own check boxes: the real settings, with a note when one waits.
        ui.text_disabled(strings::get("options.new.preset_options"));
        let run_ahead_forced = host
            .session()
            .is_some_and(crate::game_session::GameSession::forces_run_ahead_to_zero);
        let fceux_validation = host.fceux_validation();
        for &id in preset.boxes {
            let row = options::row_of(id);
            super::options::draw_row(ui, host, &row, run_ahead_forced, fceux_validation);
            if let Some(note) = later_note(id) {
                ui.same_line();
                ui.text_colored(LATER, note);
            }
        }

        ui.spacing();
        ui.separator();
        ui.text(strings::get("options.new.essentials"));
        for id in ESSENTIALS {
            let row = options::row_of(id);
            super::options::draw_row(ui, host, &row, run_ahead_forced, fceux_validation);
            if let Some(note) = later_note(id) {
                ui.same_line();
                ui.text_colored(LATER, note);
            }
        }
        let folder = options::row_of(SettingId::UiRomsFolder);
        self.draw_folder_row(ui, host, &folder);
    }
}
