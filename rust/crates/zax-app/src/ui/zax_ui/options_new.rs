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
use crate::ui::options::{self, OptionsGroup, OptionsTab, TabEntry};
use crate::ui::{strings, text};

/// The window's default size, as the classic one's.
const WINDOW_SIZE: [f32; 2] = [820.0, 640.0];

/// The accent of the interface (the console mode's yellow).
const ACCENT: [f32; 4] = [1.0, 0.835, 0.18, 1.0];

/// The amber of a "takes effect later" note.
const LATER: [f32; 4] = [1.0, 0.82, 0.48, 1.0];

/// A value a preset writes (`SettingValue` holds a `String` variant, so it cannot sit in a
/// constant table itself).
#[derive(Clone, Copy)]
enum V {
    B(bool),
    I(i32),
    E(i32),
}

impl V {
    const fn value(self) -> SettingValue {
        match self {
            Self::B(on) => SettingValue::Bool(on),
            Self::I(number) => SettingValue::Int(number),
            Self::E(number) => SettingValue::Enum(number),
        }
    }
}

/// One preset of the slider. Every preset writes the whole set of settings the slider manages,
/// so that a mode puts back what another one changed; each value is either proposed (shown
/// under the slider, to be changed) or set and hidden (owner, 2026-11-18).
struct Preset {
    /// Its key in the string tables: `options.preset.<key>.name`, `.summary`, `.acts`.
    key: &'static str,
    /// The values the preset writes, in the order the proposed ones are shown, each with
    /// whether it is proposed.
    values: &'static [(SettingId, V, bool)],
}

/// The five presets, from the fastest to the most faithful (owner's table,
/// `docs/design/PRESETS.csv`, 2026-11-18).
const PRESETS: [Preset; 5] = [
    Preset {
        key: "fastest",
        values: &[
            (SettingId::EmuFastCore, V::B(true), true),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(0), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(true), true),
            (SettingId::EmuPipeline, V::E(1), true),
            (SettingId::EmuLuaOptimized, V::B(true), true),
            (SettingId::EmuLuaExperimental, V::B(true), true),
            (SettingId::VideoDynamicResolution, V::B(true), true),
            (SettingId::VideoResScale, V::E(0), true),
            (SettingId::VideoHalfRate3d, V::B(false), true),
            (SettingId::RenderShadow, V::B(false), true),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(0), true),
            (SettingId::RenderStyleReflections, V::I(0), false),
            (SettingId::RenderStyleSsrQuality, V::I(0), false),
            (SettingId::RenderStyleAo, V::I(0), false),
            (SettingId::RenderStyleAoQuality, V::I(0), false),
            (SettingId::RenderStylePixelShadow, V::I(0), false),
        ],
    },
    Preset {
        key: "fast",
        values: &[
            (SettingId::EmuFastCore, V::B(true), true),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(true), true),
            (SettingId::EmuPipeline, V::E(0), true),
            (SettingId::EmuLuaOptimized, V::B(false), true),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(true), true),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::RenderShadow, V::B(true), true),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(0), true),
            (SettingId::RenderStyleReflections, V::I(0), false),
            (SettingId::RenderStyleSsrQuality, V::I(0), false),
            (SettingId::RenderStyleAo, V::I(0), false),
            (SettingId::RenderStyleAoQuality, V::I(0), false),
            (SettingId::RenderStylePixelShadow, V::I(0), false),
        ],
    },
    Preset {
        key: "balanced",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(1), false),
            (SettingId::RenderStyleAo, V::I(0), true),
            (SettingId::RenderStyleAoQuality, V::I(1), false),
            (SettingId::RenderStylePixelShadow, V::I(0), true),
        ],
    },
    Preset {
        key: "faithful",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(false), true),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), true),
            (SettingId::EmuLatePresent, V::B(false), true),
            (SettingId::EmuCatchUpLast3d, V::B(false), true),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(2), true),
            (SettingId::RenderStyleAo, V::I(1), true),
            (SettingId::RenderStyleAoQuality, V::I(1), true),
            (SettingId::RenderStylePixelShadow, V::I(1), true),
        ],
    },
    Preset {
        key: "most_faithful",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(true), true),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(true), true),
            (SettingId::EmuLatePresent, V::B(false), true),
            (SettingId::EmuCatchUpLast3d, V::B(false), false),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), true),
            (SettingId::EmuOamaddrWriteCorruption, V::B(true), true),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(2), true),
            (SettingId::RenderStyleAo, V::I(1), true),
            (SettingId::RenderStyleAoQuality, V::I(2), true),
            (SettingId::RenderStylePixelShadow, V::I(1), true),
        ],
    },
];

/// The settings of the Essentials tab, under the slider.
const ESSENTIALS: [SettingId; 1] = [SettingId::VideoWidescreen];

/// What the simple mode shows of a page.
#[derive(Clone, Copy)]
enum Simple {
    /// One setting.
    Row(SettingId),
    /// Every row of a group of the classic tab (its compact rows included).
    Group(OptionsGroup),
    /// The whole classic tab.
    All,
    /// The button that opens the shaders window.
    Shaders,
    /// A folder setting, drawn as the classic window draws it.
    Folder(SettingId),
}

/// One tab of the new window: the classic tabs its advanced mode draws one under the other, and
/// what its simple mode shows (nothing: the tab is hidden in that mode).
struct Page {
    /// The classic tab whose title it takes and whose groups the simple mode reads.
    tab: OptionsTab,
    simple: &'static [Simple],
    advanced: &'static [OptionsTab],
}

/// The tabs between Essentials and Controllers (owner's selection, 2026-11-17). The 3D render
/// takes the light and the camera, which are the 3D scene's alone.
const PAGES: [Page; 7] = [
    Page {
        tab: OptionsTab::Display,
        simple: &[
            Simple::Row(SettingId::UiFullscreen),
            Simple::Row(SettingId::VideoVSync),
            Simple::Row(SettingId::VideoAntiAlias),
            Simple::Row(SettingId::VideoSkipFrame),
            Simple::Row(SettingId::VideoBackend),
            Simple::Shaders,
        ],
        advanced: &[OptionsTab::Display],
    },
    Page {
        tab: OptionsTab::NesPicture,
        simple: &[
            Simple::Row(SettingId::VideoNesPalette),
            Simple::Row(SettingId::VideoCompositeSignal),
            Simple::Group(OptionsGroup::Overscan),
        ],
        advanced: &[OptionsTab::NesPicture],
    },
    Page {
        tab: OptionsTab::Render3d,
        simple: &[
            Simple::Row(SettingId::RenderVisualStyle),
            Simple::Row(SettingId::RenderStyleReflections),
            Simple::Group(OptionsGroup::Geometry),
            Simple::Row(SettingId::LightIntensity),
            Simple::Row(SettingId::RenderShadow),
            Simple::Row(SettingId::CameraFov),
        ],
        advanced: &[OptionsTab::Render3d, OptionsTab::Light, OptionsTab::Camera],
    },
    Page {
        tab: OptionsTab::Audio,
        simple: &[
            Simple::Row(SettingId::AudioVolume),
            Simple::Row(SettingId::AudioConsoleProfile),
        ],
        advanced: &[OptionsTab::Audio],
    },
    Page {
        tab: OptionsTab::Emulation,
        simple: &[
            Simple::Row(SettingId::EmuRunAhead),
            Simple::Row(SettingId::EmuSmoothScrollAhead),
            Simple::Row(SettingId::EmuRainbowNetwork),
            Simple::Row(SettingId::EmuExactRate),
            Simple::Row(SettingId::EmuRealtimeInput),
        ],
        advanced: &[OptionsTab::Emulation],
    },
    Page {
        tab: OptionsTab::Interface,
        simple: &[
            Simple::Row(SettingId::UiLanguage),
            Simple::Folder(SettingId::UiRomsFolder),
            Simple::Row(SettingId::UiShowFps),
        ],
        advanced: &[OptionsTab::Interface],
    },
    Page {
        tab: OptionsTab::Achievements,
        simple: &[Simple::All],
        advanced: &[OptionsTab::Achievements],
    },
];

/// Whether the settings still hold every value `preset` writes.
fn matches(model: &SettingsModel, preset: &Preset) -> bool {
    preset
        .values
        .iter()
        .all(|&(id, value, _)| model.get(id) == value.value())
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
                for page in &PAGES {
                    if self.options_advanced && page.advanced.is_empty() {
                        continue;
                    }
                    if !self.options_advanced && page.simple.is_empty() {
                        continue;
                    }
                    let Some(_tab) = TabItem::new(options::title_of(page.tab)).begin(ui) else {
                        continue;
                    };
                    if self.options_advanced {
                        for (index, &tab) in page.advanced.iter().enumerate() {
                            if index > 0 {
                                ui.spacing();
                                ui.separator();
                                ui.text_colored(ACCENT, options::title_of(tab));
                            }
                            let _id = ui.push_id_usize(index);
                            self.draw_tab_body(ui, host, tab);
                        }
                    } else {
                        self.draw_simple_page(ui, host, page);
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

    /// A page in the simple mode: the rows the owner chose, a group of the classic tab, or all
    /// of it.
    fn draw_simple_page(&mut self, ui: &Ui, host: &mut AppHost, page: &Page) {
        let run_ahead_forced = host
            .session()
            .is_some_and(crate::game_session::GameSession::forces_run_ahead_to_zero);
        let fceux_validation = host.fceux_validation();
        for item in page.simple {
            match *item {
                Simple::All => self.draw_tab_body(ui, host, page.tab),
                Simple::Folder(id) => {
                    let row = options::row_of(id);
                    self.draw_folder_row(ui, host, &row);
                }
                Simple::Shaders => {
                    if ui.button(strings::get("options.new.shaders_open")) {
                        self.state.shaders = true;
                        play(host, UiSound::Confirm);
                    }
                }
                Simple::Row(id) if !options::is_shown(zax_settings::catalogue::descriptor(id)) => {}
                Simple::Row(id) => {
                    let row = options::row_of(id);
                    super::options::draw_row(ui, host, &row, run_ahead_forced, fceux_validation);
                    if let Some(note) = later_note(id) {
                        ui.same_line();
                        ui.text_colored(LATER, note);
                    }
                }
                Simple::Group(group) => {
                    ui.spacing();
                    ui.text_disabled(group.title());
                    let mut inside = false;
                    for entry in self.options.entries(page.tab).to_vec() {
                        match entry {
                            TabEntry::Group(heading) => inside = heading == group,
                            TabEntry::Row(row) if inside => super::options::draw_row(
                                ui,
                                host,
                                &row,
                                run_ahead_forced,
                                fceux_validation,
                            ),
                            TabEntry::Compact(compact, cells) if inside => {
                                super::options::draw_compact_row(ui, host, compact, &cells);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
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
                    for &(id, value, _) in preset.values {
                        model.set(id, value.value());
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
        for &(id, _, shown) in preset.values {
            if !shown {
                continue;
            }
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
    }
}
