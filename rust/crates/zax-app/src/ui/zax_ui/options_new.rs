//! The new options window (lot REWRITE-UI, `docs/progress/REWRITE-UI.md`): a simple mode (the
//! Essentials tab and the Controllers tab) and an advanced mode (every tab of the classic
//! window as well). On Essentials, a performance ↔ fidelity slider of five presets, a box saying
//! what the chosen one turns on, and the preset's own check boxes. The classic window stays,
//! one button away, so that the two can be compared.

use imgui::{Condition, StyleColor, TabBarFlags, TabItem, Ui, WindowFlags};
use zax_settings::catalogue::SettingId;
use zax_settings::meta::SettingEffect;
use zax_settings::values::SettingValue;

use super::{centre, play, ZaxUi};
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::emulation::host::hack_options;
use crate::ui::handheld::hacks::HacksView;
use crate::ui::options::{self, OptionsGroup, OptionsTab, TabEntry};
use crate::ui::presets::{self, matches, reset_with, PRESETS};
use crate::ui::{strings, text};

/// The window's default size, as the classic one's.
const WINDOW_SIZE: [f32; 2] = [820.0, 640.0];

/// The accent of the interface (the console mode's yellow).
const ACCENT: [f32; 4] = [1.0, 0.835, 0.18, 1.0];

/// The amber of a "takes effect later" note.
const LATER: [f32; 4] = [1.0, 0.82, 0.48, 1.0];

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
    /// The buttons that open the widescreen and the hacks of the game in play.
    GameHacks,
    /// The widescreen of every game by default: a check box, and the ratio beside it when the
    /// field holds.
    WidescreenDefault(bool),
    /// The common default of a hack pack option (`hack_options::COMMON_OPTIONS`).
    HackDefault(&'static str),
    /// The video signal: off (the palette), composite, S-Video or RF, one list over
    /// `video.composite_signal` and `video.signal_kind` (lot REWRITE-UI).
    Signal,
    /// A heading (a string key).
    Heading(&'static str),
    /// The button that puts the four knobs of the television back at rest.
    TvNeutral,
}

/// The television's knobs (`video.tv_*`).
const TV_KNOBS: [SettingId; 4] = [
    SettingId::VideoTvHue,
    SettingId::VideoTvSaturation,
    SettingId::VideoTvContrast,
    SettingId::VideoTvBrightness,
];

/// What the advanced mode shows of a page.
#[derive(Clone, Copy)]
enum Advanced {
    /// Classic tabs, one under the other.
    Tabs(&'static [OptionsTab]),
    /// A list of its own, for a page no classic tab matches.
    Items(&'static [Simple]),
}

/// One tab of the new window: what its advanced mode draws, and what its simple mode shows
/// (nothing: the tab is hidden in that mode).
struct Page {
    /// The classic tab whose title it takes and whose groups the simple mode reads.
    tab: OptionsTab,
    /// A title of its own (a string key) instead of the classic tab's.
    title: Option<&'static str>,
    simple: &'static [Simple],
    advanced: Advanced,
}

impl Page {
    fn title(&self) -> String {
        self.title
            .map_or_else(|| options::title_of(self.tab), strings::get)
    }
}

/// The tabs between Essentials and Controllers (owner's selection, 2026-11-17). The 3D render
/// takes the light and the camera, which are the 3D scene's alone; the widescreen and the hacks
/// have a tab of their own (owner, 2026-11-18).
const PAGES: [Page; 8] = [
    Page {
        tab: OptionsTab::Display,
        title: None,
        simple: &[
            Simple::Row(SettingId::UiFullscreen),
            Simple::Row(SettingId::VideoVSync),
            Simple::Row(SettingId::VideoAntiAlias),
            Simple::Row(SettingId::VideoSkipFrame),
            Simple::Row(SettingId::VideoBackend),
            Simple::Shaders,
        ],
        advanced: Advanced::Tabs(&[OptionsTab::Display]),
    },
    Page {
        tab: OptionsTab::NesPicture,
        title: None,
        // The signal, the television and the palette (owner, 2026-11-18).
        simple: &[
            Simple::Signal,
            Simple::Row(SettingId::VideoTvModel),
            Simple::Row(SettingId::VideoNesPalette),
            Simple::Group(OptionsGroup::Overscan),
        ],
        // The classic tab without its widescreen group, which went to Widescreen & Hacks.
        advanced: Advanced::Items(&[
            Simple::Signal,
            Simple::Row(SettingId::VideoTvModel),
            Simple::Row(SettingId::VideoNesPalette),
            Simple::Row(SettingId::VideoEmphasis3d),
            Simple::Heading("options.new.signal.decoder"),
            Simple::Row(SettingId::VideoDecoderSharpness),
            Simple::Row(SettingId::VideoCombFilter),
            Simple::Row(SettingId::VideoFrozenDotCrawl),
            Simple::Row(SettingId::VideoRfNoise),
            Simple::Row(SettingId::VideoRfGhost),
            Simple::Heading("options.new.signal.knobs"),
            Simple::TvNeutral,
            Simple::Row(SettingId::VideoTvHue),
            Simple::Row(SettingId::VideoTvSaturation),
            Simple::Row(SettingId::VideoTvContrast),
            Simple::Row(SettingId::VideoTvBrightness),
            Simple::Row(SettingId::VideoRemoveSpriteLimit),
            Simple::Group(OptionsGroup::Overscan),
        ]),
    },
    Page {
        tab: OptionsTab::NesPicture,
        title: Some("options.new.tab.widescreen"),
        // The defaults of every game whose ROM has a widescreen patch; a game's own choices, in
        // its hacks window, come first (owner, 2026-11-18).
        // The simple mode: the widescreen on or off, nothing more (owner, 2026-11-18).
        simple: &[Simple::WidescreenDefault(false), Simple::GameHacks],
        advanced: Advanced::Items(&[
            Simple::WidescreenDefault(true),
            Simple::Row(SettingId::VideoWidescreenExtra),
            Simple::Row(SettingId::VideoWidescreenAdvanced),
            Simple::HackDefault("no_slowdown"),
            Simple::HackDefault("invincible"),
            Simple::HackDefault("level_select"),
            Simple::HackDefault("margin_enemies"),
            Simple::HackDefault("margin_freeze"),
            Simple::HackDefault("margin_freeze_tint"),
            Simple::Row(SettingId::VideoWidescreenStrictEdges),
            Simple::Row(SettingId::EmuBuiltinPatches),
            Simple::GameHacks,
        ]),
    },
    Page {
        tab: OptionsTab::Render3d,
        title: None,
        simple: &[
            Simple::Row(SettingId::RenderVisualStyle),
            Simple::Row(SettingId::RenderStyleReflections),
            Simple::Group(OptionsGroup::Geometry),
            Simple::Row(SettingId::LightIntensity),
            Simple::Row(SettingId::RenderShadow),
            Simple::Row(SettingId::CameraFov),
        ],
        advanced: Advanced::Tabs(&[OptionsTab::Render3d, OptionsTab::Light, OptionsTab::Camera]),
    },
    Page {
        tab: OptionsTab::Audio,
        title: None,
        simple: &[
            Simple::Row(SettingId::AudioVolume),
            Simple::Row(SettingId::AudioConsoleProfile),
        ],
        advanced: Advanced::Tabs(&[OptionsTab::Audio]),
    },
    Page {
        tab: OptionsTab::Emulation,
        title: None,
        simple: &[
            Simple::Row(SettingId::EmuRunAhead),
            Simple::Row(SettingId::EmuSmoothScrollAhead),
            Simple::Row(SettingId::EmuSmoothScroll3d),
            Simple::Row(SettingId::EmuRainbowNetwork),
            Simple::Row(SettingId::EmuExactRate),
            Simple::Row(SettingId::EmuRealtimeInput),
        ],
        advanced: Advanced::Tabs(&[OptionsTab::Emulation]),
    },
    Page {
        tab: OptionsTab::Interface,
        title: None,
        simple: &[
            Simple::Row(SettingId::UiLanguage),
            Simple::Folder(SettingId::UiRomsFolder),
            Simple::Row(SettingId::UiShowFps),
        ],
        advanced: Advanced::Tabs(&[OptionsTab::Interface]),
    },
    Page {
        tab: OptionsTab::Achievements,
        title: None,
        simple: &[Simple::All],
        advanced: Advanced::Tabs(&[OptionsTab::Achievements]),
    },
];

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
        // The mode and the preset are settings, shared with the console screens.
        self.options_advanced = presets::advanced(host.settings());
        self.options_preset = presets::chosen(host.settings());
        if let Some(_window) = window {
            if ui.checkbox(
                strings::get("options.new.advanced"),
                &mut self.options_advanced,
            ) {
                let advanced = self.options_advanced;
                host.update_settings(|model| {
                    model.set(SettingId::UiOptionsAdvanced, SettingValue::Bool(advanced));
                });
            }
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
                    if !self.options_advanced && page.simple.is_empty() {
                        continue;
                    }
                    let Some(_tab) = TabItem::new(page.title()).begin(ui) else {
                        continue;
                    };
                    self.draw_page_reset(ui, host, page);
                    match page.advanced {
                        Advanced::Tabs(tabs) if self.options_advanced => {
                            for (index, &tab) in tabs.iter().enumerate() {
                                if index > 0 {
                                    ui.spacing();
                                    ui.separator();
                                    ui.text_colored(ACCENT, options::title_of(tab));
                                }
                                let _id = ui.push_id_usize(index);
                                self.draw_tab_entries(ui, host, tab);
                            }
                        }
                        Advanced::Items(items) if self.options_advanced => {
                            self.draw_items(ui, host, page.tab, items);
                        }
                        _ => self.draw_items(ui, host, page.tab, page.simple),
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

    /// The settings a page shows in the mode in force, the ones its Default button resets.
    fn page_ids(&self, page: &Page) -> Vec<SettingId> {
        let items = match page.advanced {
            Advanced::Tabs(tabs) if self.options_advanced => {
                return tabs
                    .iter()
                    .flat_map(|&tab| self.options.rows(tab))
                    .map(|row| row.descriptor.id())
                    .collect();
            }
            Advanced::Items(items) if self.options_advanced => items,
            _ => page.simple,
        };
        let mut ids = Vec::new();
        for item in items {
            match *item {
                Simple::Row(id) | Simple::Folder(id) => ids.push(id),
                Simple::WidescreenDefault(_) => ids.push(SettingId::VideoWidescreen),
                Simple::Signal => {
                    ids.extend([SettingId::VideoCompositeSignal, SettingId::VideoSignalKind]);
                }
                Simple::All => ids.extend(
                    self.options
                        .rows(page.tab)
                        .iter()
                        .map(|row| row.descriptor.id()),
                ),
                Simple::Group(group) => {
                    let mut inside = false;
                    for entry in self.options.entries(page.tab) {
                        match entry {
                            TabEntry::Group(heading) => inside = *heading == group,
                            TabEntry::Row(row) if inside => ids.push(row.descriptor.id()),
                            TabEntry::Compact(_, cells) if inside => {
                                ids.extend(cells.iter().map(|row| row.descriptor.id()));
                            }
                            _ => {}
                        }
                    }
                }
                // The common hack defaults live in `emu.hack_options` with the games' own
                // choices: the Default button takes out the common ones only.
                Simple::Shaders
                | Simple::GameHacks
                | Simple::HackDefault(_)
                | Simple::TvNeutral
                | Simple::Heading(_) => {}
            }
        }
        ids
    }

    /// A page's Default button: what the page shows goes back to the chosen preset's values,
    /// and to the settings' own defaults where the preset has none. The preset stays chosen.
    fn draw_page_reset(&mut self, ui: &Ui, host: &mut AppHost, page: &Page) {
        if ui.button(text::reset_defaults()) {
            let ids = self.page_ids(page);
            let common = self.shows_hack_defaults(page);
            let preset = &PRESETS[self.options_preset.min(PRESETS.len() - 1)];
            host.update_settings(|model| {
                reset_with(model, preset, &ids);
                if common {
                    let stored = model.get(SettingId::EmuHackOptions);
                    let text = hack_options::without_common(stored.as_text());
                    model.set(SettingId::EmuHackOptions, SettingValue::Text(text));
                }
            });
            play(host, UiSound::Confirm);
        }
        ui.separator();
    }

    /// Whether the page shows common hack defaults in the mode in force.
    fn shows_hack_defaults(&self, page: &Page) -> bool {
        let items = match page.advanced {
            Advanced::Items(items) if self.options_advanced => items,
            Advanced::Tabs(_) if self.options_advanced => return false,
            _ => page.simple,
        };
        items
            .iter()
            .any(|item| matches!(item, Simple::HackDefault(_)))
    }

    /// The widescreen of every game by default: on or off, and its ratio when on.
    fn draw_widescreen_default(ui: &Ui, host: &mut AppHost, with_ratio: bool) {
        ui.text_wrapped(strings::get("options.new.widescreen_line"));
        ui.spacing();
        let value = host.settings().get(SettingId::VideoWidescreen).as_int();
        let mut on = value != 0;
        if ui.checkbox(strings::get("options.new.widescreen_default"), &mut on) {
            host.update_settings(|model| {
                model.set(
                    SettingId::VideoWidescreen,
                    SettingValue::Enum(i32::from(on)),
                );
            });
            play(host, UiSound::Confirm);
        }
        if on && with_ratio {
            let descriptor = zax_settings::catalogue::descriptor(SettingId::VideoWidescreen);
            // The ratios: every choice but Off.
            let ratios: Vec<(i32, String)> = (1..=4)
                .map(|ratio| {
                    (
                        ratio,
                        descriptor
                            .domain()
                            .label_of(f64::from(ratio))
                            .unwrap_or_default()
                            .to_owned(),
                    )
                })
                .collect();
            let mut index = ratios
                .iter()
                .position(|&(ratio, _)| ratio == value)
                .unwrap_or(0);
            ui.same_line();
            ui.set_next_item_width(140.0);
            if ui.combo(
                strings::get("options.new.widescreen_ratio"),
                &mut index,
                &ratios,
                |(_, label)| label.as_str().into(),
            ) {
                let ratio = ratios[index].0;
                host.update_settings(|model| {
                    model.set(SettingId::VideoWidescreen, SettingValue::Enum(ratio));
                });
            }
        }
    }

    /// The common default of a hack pack option, for every pack that has it.
    fn draw_hack_default(ui: &Ui, host: &mut AppHost, key: &'static str) {
        let Some(&(_, pack_default)) = hack_options::COMMON_OPTIONS
            .iter()
            .find(|&&(common, _)| common == key)
        else {
            return;
        };
        let stored = host.settings().get(SettingId::EmuHackOptions);
        let mut on = hack_options::common_value(stored.as_text(), key).unwrap_or(pack_default);
        if ui.checkbox(strings::get(&format!("options.new.hack.{key}")), &mut on) {
            let text = hack_options::with_common(stored.as_text(), key, on);
            host.update_settings(|model| {
                model.set(SettingId::EmuHackOptions, SettingValue::Text(text));
            });
            play(host, UiSound::Confirm);
        }
    }

    /// The buttons that open the widescreen and the hacks of the game in play (its own hacks
    /// window), greyed without a game.
    fn draw_game_hacks(&mut self, ui: &Ui, host: &mut AppHost) {
        ui.spacing();
        ui.separator();
        ui.text_disabled(strings::get("options.new.game_hacks_line"));
        let no_game = host.current_rom_path().is_none();
        let _disabled = ui.begin_disabled(no_game);
        if ui.button(strings::get("options.new.game_widescreen")) {
            self.open_hacks(host, HacksView::Widescreen);
            play(host, UiSound::Confirm);
        }
        ui.same_line();
        if ui.button(strings::get("options.new.game_hacks")) {
            self.open_hacks(host, HacksView::Aids);
            play(host, UiSound::Confirm);
        }
        if no_game {
            ui.text_disabled(strings::get("options.new.no_game"));
        }
    }

    /// The items of a page: the rows the owner chose, a group of the classic tab, or all of
    /// it.
    fn draw_items(&mut self, ui: &Ui, host: &mut AppHost, tab: OptionsTab, items: &[Simple]) {
        let run_ahead_forced = host
            .session()
            .is_some_and(crate::game_session::GameSession::forces_run_ahead_to_zero);
        let fceux_validation = host.fceux_validation();
        for item in items {
            match *item {
                Simple::All => self.draw_tab_entries(ui, host, tab),
                Simple::GameHacks => self.draw_game_hacks(ui, host),
                Simple::WidescreenDefault(with_ratio) => {
                    Self::draw_widescreen_default(ui, host, with_ratio);
                }
                Simple::HackDefault(key) => Self::draw_hack_default(ui, host, key),
                Simple::Signal => Self::draw_signal(ui, host),
                Simple::Heading(key) => {
                    ui.spacing();
                    ui.text_disabled(strings::get(key));
                }
                Simple::TvNeutral => {
                    if ui.small_button(strings::get("options.new.signal.neutral")) {
                        host.update_settings(|model| {
                            for id in TV_KNOBS {
                                model.reset(id);
                            }
                        });
                        play(host, UiSound::Confirm);
                    }
                }
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
                    for entry in self.options.entries(tab).to_vec() {
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

    /// The video signal (lot REWRITE-UI): one list over `video.composite_signal` and
    /// `video.signal_kind`.
    fn draw_signal(ui: &Ui, host: &mut AppHost) {
        let on = host
            .settings()
            .get(SettingId::VideoCompositeSignal)
            .as_bool();
        let kind = host.settings().get(SettingId::VideoSignalKind).as_int();
        let current = if on {
            usize::try_from(kind.clamp(0, 2) + 1).unwrap_or(1)
        } else {
            0
        };
        let names = [
            strings::get("options.new.signal.off"),
            text::label_of_option("Composite"),
            text::label_of_option("S-Video"),
            text::label_of_option("RF"),
        ];
        let mut chosen = current;
        if ui.combo_simple_string(strings::get("options.new.signal"), &mut chosen, &names)
            && chosen != current
        {
            host.update_settings(|model| {
                crate::ui::handheld::options::signal_choose(model, chosen);
            });
            play(host, UiSound::Confirm);
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(strings::get("options.new.signal.help"));
        }
        if on {
            ui.text_disabled(strings::get("options.new.signal.palette_note"));
        }
    }

    /// The Essentials tab: the slider, its box and check boxes, then the essential settings.
    fn draw_essentials(&mut self, ui: &Ui, host: &mut AppHost) {
        if ui.button(text::reset_defaults()) {
            let preset = &PRESETS[self.options_preset.min(PRESETS.len() - 1)];
            host.update_settings(|model| {
                for &(id, value, _) in preset.values {
                    model.set(id, value.value());
                }
            });
            play(host, UiSound::Confirm);
        }
        ui.separator();
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
                host.update_settings(|model| presets::choose(model, index));
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
    }
}
