//! The options of the handheld interface as data: a short list of rows with stepped values (no
//! slider, no text). The values are the settings of the desktop options window; the rows only
//! choose which ones a handheld needs and how they are stepped.

use zax_settings::catalogue::{self, SettingId};
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::overlay::CameraPreset;
use super::pad::PadButton;
use crate::persistence::app_settings::{AppSettings as _, AppSettingsMut as _, TvRegionSetting};
use crate::render_scaling::{DIRECT_METHOD, RENDER_TEXTURE_METHOD};
use crate::ui::{keys, strings, text};

/// The volumes the row steps through.
const VOLUME_LEVELS: [f32; 11] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

/// The frame caps the row offers, in frames per second.
pub const FRAME_CAPS: [i32; 2] = [60, 30];

/// A row of the options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionRow {
    /// The language of the interface.
    Language,
    /// The volume.
    Volume,
    /// The resolution the scene is drawn at.
    RenderScale,
    /// The shadows.
    Shadows,
    /// The table the NES colours are shown in (`video.nes_palette`).
    Palette,
    /// The frame cap: 60 every refresh, 30 every other refresh of a 60 Hz screen.
    FrameCap,
    /// The frame counter.
    ShowFps,
    /// The frame profiler.
    Profiler,
    /// The camera preset.
    Camera,
    /// How many emulated frames a displayed frame may run when the game is late.
    CatchUp,
    /// The optimised Lua paths (`unsafe`), on or off.
    LuaOptimized,
    /// The experimental Lua paths (they need the optimised ones), on or off.
    LuaExperimental,
    /// The scripts beside the drawing: off, automatic (while late) or always.
    Pipeline,
    /// The TV region: automatic (from the ROM header, at the next game start), NTSC or PAL.
    Region,
    /// The gamepad bindings.
    Controls,
}

/// The rows, in order.
pub const ROWS: [OptionRow; 15] = [
    OptionRow::Language,
    OptionRow::Volume,
    OptionRow::RenderScale,
    OptionRow::Shadows,
    OptionRow::Palette,
    OptionRow::FrameCap,
    OptionRow::ShowFps,
    OptionRow::Profiler,
    OptionRow::Camera,
    OptionRow::CatchUp,
    OptionRow::LuaOptimized,
    OptionRow::LuaExperimental,
    OptionRow::Pipeline,
    OptionRow::Region,
    OptionRow::Controls,
];

/// The values of the palette row (`video.nes_palette`): the setting's choices.
fn palettes() -> Vec<i32> {
    catalogue::descriptor(SettingId::VideoNesPalette)
        .domain()
        .options()
        .unwrap_or_default()
        .iter()
        .map(|option| {
            #[allow(clippy::cast_possible_truncation)] // A choice's index.
            let value = option.value as i32;
            value
        })
        .collect()
}

fn palette(settings: &SettingsModel) -> i32 {
    settings.get(SettingId::VideoNesPalette).as_int()
}

/// The values of the catch-up row (`emu.catch_up`): none, then a limit of 1 to 5 frames.
const CATCH_UPS: [i32; 6] = [0, 1, 2, 3, 4, 5];

fn catch_up(settings: &SettingsModel) -> i32 {
    settings.get(SettingId::EmuCatchUp).as_int()
}

/// The values of the pipeline row (`emu.pipeline`): off, auto, always.
const PIPELINES: [i32; 3] = [0, 1, 2];

fn pipeline(settings: &SettingsModel) -> i32 {
    settings.get(SettingId::EmuPipeline).as_int()
}

/// The choices of the region row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionChoice {
    /// `emu.auto_tv_mode` on: the region of the ROM header, applied when a game starts.
    Auto,
    /// NTSC, at once.
    Ntsc,
    /// PAL, at once.
    Pal,
}

impl RegionChoice {
    /// The choices, in the order the row cycles through them.
    pub const ALL: [Self; 3] = [Self::Auto, Self::Ntsc, Self::Pal];

    /// The choice `settings` stand for: Auto while the automatic mode is on, else the region.
    #[must_use]
    pub fn of(settings: &SettingsModel) -> Self {
        if settings.auto_tv_mode() {
            Self::Auto
        } else if settings.tv_mode() == TvRegionSetting::Pal {
            Self::Pal
        } else {
            Self::Ntsc
        }
    }

    fn apply(self, settings: &mut SettingsModel) {
        settings.set_auto_tv_mode(self == Self::Auto);
        match self {
            Self::Auto => {}
            Self::Ntsc => settings.set_tv_mode(TvRegionSetting::Ntsc),
            Self::Pal => settings.set_tv_mode(TvRegionSetting::Pal),
        }
    }
}

/// What the options ask of the application beyond the settings they change themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsCommand {
    /// Leave the options.
    Close,
    /// Show the list of the gamepad bindings.
    OpenControls,
    /// Step the camera preset.
    StepCamera(i32),
}

/// The list and its selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HandheldOptionsModel {
    selected: usize,
}

impl HandheldOptionsModel {
    /// The options on their first row.
    #[must_use]
    pub const fn new() -> Self {
        Self { selected: 0 }
    }

    /// The selected row.
    #[must_use]
    pub fn selected(&self) -> OptionRow {
        ROWS[self.selected]
    }

    /// The index of the selected row.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// One press of a button; a value row changes `settings` itself.
    pub fn handle(
        &mut self,
        button: PadButton,
        settings: &mut SettingsModel,
    ) -> Option<OptionsCommand> {
        match button {
            PadButton::Up => self.selected = (self.selected + ROWS.len() - 1) % ROWS.len(),
            PadButton::Down => self.selected = (self.selected + 1) % ROWS.len(),
            PadButton::Left => return self.step(settings, -1),
            PadButton::Right => return self.step(settings, 1),
            PadButton::A => {
                return if self.selected() == OptionRow::Controls {
                    Some(OptionsCommand::OpenControls)
                } else {
                    self.step(settings, 1)
                };
            }
            PadButton::B => return Some(OptionsCommand::Close),
            _ => {}
        }
        None
    }

    fn step(self, settings: &mut SettingsModel, step: i32) -> Option<OptionsCommand> {
        step_row(self.selected(), settings, step)
    }
}

/// Steps `row` by `step` in `settings`: the value rows change the settings themselves, the camera
/// row asks the application.
pub fn step_row(row: OptionRow, settings: &mut SettingsModel, step: i32) -> Option<OptionsCommand> {
    match row {
        OptionRow::Language => {
            let codes = language_codes();
            let current = settings.language();
            let next = *cycled(&codes, &current.as_str(), step);
            settings.set_language(next);
        }
        OptionRow::Volume => settings.set_volume(stepped_volume(settings.volume(), step)),
        OptionRow::RenderScale => set_render_scale(settings, step),
        OptionRow::Shadows => {
            let on = settings.shadow();
            settings.set_shadow(!on);
        }
        OptionRow::Palette => {
            let next = *cycled(&palettes(), &palette(settings), step);
            settings.set(SettingId::VideoNesPalette, SettingValue::Enum(next));
        }
        OptionRow::FrameCap => {
            let next = *cycled(&FRAME_CAPS, &frame_cap(settings), step);
            settings.set_vsync(if next == 30 { 2 } else { 1 });
        }
        OptionRow::Profiler => {
            let on = settings.get(SettingId::UiProfiler).as_bool();
            settings.set(SettingId::UiProfiler, SettingValue::Bool(!on));
        }
        OptionRow::ShowFps => {
            let on = settings.show_fps();
            settings.set_show_fps(!on);
        }
        OptionRow::Camera => return Some(OptionsCommand::StepCamera(step)),
        OptionRow::CatchUp => {
            let next = *cycled(&CATCH_UPS, &catch_up(settings), step);
            settings.set(SettingId::EmuCatchUp, SettingValue::Enum(next));
        }
        OptionRow::LuaOptimized => {
            let on = settings.get(SettingId::EmuLuaOptimized).as_bool();
            settings.set(SettingId::EmuLuaOptimized, SettingValue::Bool(!on));
        }
        OptionRow::LuaExperimental => {
            let on = settings.get(SettingId::EmuLuaExperimental).as_bool();
            settings.set(SettingId::EmuLuaExperimental, SettingValue::Bool(!on));
        }
        OptionRow::Pipeline => {
            let next = *cycled(&PIPELINES, &pipeline(settings), step);
            settings.set(SettingId::EmuPipeline, SettingValue::Enum(next));
        }
        OptionRow::Region => {
            cycled(&RegionChoice::ALL, &RegionChoice::of(settings), step).apply(settings);
        }
        OptionRow::Controls => {}
    }
    None
}

/// The label of `row`, in the language in force.
#[must_use]
pub fn label(row: OptionRow) -> String {
    match row {
        OptionRow::Language => text::label_of(catalogue::descriptor(SettingId::UiLanguage)),
        OptionRow::Volume => text::label_of(catalogue::descriptor(SettingId::AudioVolume)),
        OptionRow::RenderScale => strings::get(keys::HANDHELD_OPTION_RENDER_SCALE),
        OptionRow::Shadows => text::label_of(catalogue::descriptor(SettingId::RenderShadow)),
        OptionRow::Palette => text::label_of(catalogue::descriptor(SettingId::VideoNesPalette)),
        OptionRow::FrameCap => strings::get(keys::HANDHELD_OPTION_FRAME_CAP),
        OptionRow::ShowFps => text::label_of(catalogue::descriptor(SettingId::UiShowFps)),
        OptionRow::Profiler => text::label_of(catalogue::descriptor(SettingId::UiProfiler)),
        OptionRow::Camera => strings::get(keys::HANDHELD_OPTION_CAMERA),
        OptionRow::CatchUp => strings::get(keys::HANDHELD_OPTION_CATCH_UP),
        OptionRow::LuaOptimized => {
            text::label_of(catalogue::descriptor(SettingId::EmuLuaOptimized))
        }
        OptionRow::LuaExperimental => {
            text::label_of(catalogue::descriptor(SettingId::EmuLuaExperimental))
        }
        OptionRow::Pipeline => strings::get(keys::HANDHELD_OPTION_PIPELINE),
        OptionRow::Region => strings::get(keys::HANDHELD_OPTION_REGION),
        OptionRow::Controls => strings::get(keys::HANDHELD_OPTION_CONTROLS),
    }
}

/// The value of `row` as the screen shows it; empty for a row that opens a list.
#[must_use]
pub fn value_text(row: OptionRow, settings: &SettingsModel, camera: CameraPreset) -> String {
    let switch = |on: bool| text::label_of_option(if on { "enabled" } else { "disabled" });
    match row {
        OptionRow::Language => text::label_of_option(&settings.language()),
        OptionRow::Volume => format!("{} %", volume_percent(settings.volume())),
        OptionRow::RenderScale => render_scale_options()
            .get(render_scale_index(settings))
            .map_or_else(
                || text::label_of_option("Auto"),
                |label| text::label_of_option(label),
            ),
        OptionRow::Shadows => switch(settings.shadow()),
        OptionRow::Palette => catalogue::descriptor(SettingId::VideoNesPalette)
            .domain()
            .options()
            .unwrap_or_default()
            .get(usize::try_from(palette(settings)).unwrap_or(0))
            .map_or_else(String::new, |option| text::label_of_option(&option.label)),
        OptionRow::FrameCap => format!("{} fps", frame_cap(settings)),
        OptionRow::ShowFps => switch(settings.show_fps()),
        OptionRow::Profiler => switch(settings.get(SettingId::UiProfiler).as_bool()),
        OptionRow::Camera => camera_label(camera),
        OptionRow::CatchUp => match catch_up(settings) {
            0 => text::label_of_option("None"),
            frames => frames.to_string(),
        },
        OptionRow::LuaOptimized => switch(settings.get(SettingId::EmuLuaOptimized).as_bool()),
        OptionRow::LuaExperimental => switch(settings.get(SettingId::EmuLuaExperimental).as_bool()),
        OptionRow::Pipeline => text::label_of_option(match pipeline(settings) {
            1 => "Auto",
            2 => "Always",
            _ => "Off",
        }),
        OptionRow::Region => match RegionChoice::of(settings) {
            RegionChoice::Auto => strings::get(keys::HANDHELD_REGION_AUTO),
            RegionChoice::Ntsc => "NTSC".to_owned(),
            RegionChoice::Pal => "PAL".to_owned(),
        },
        OptionRow::Controls => String::new(),
    }
}

/// The name of a camera preset, in the language in force.
#[must_use]
pub fn camera_label(camera: CameraPreset) -> String {
    strings::get(match camera {
        CameraPreset::Orbit => keys::MENU_CAMERA_ORBITAL,
        CameraPreset::FirstPerson => keys::MENU_CAMERA_FIRST_PERSON,
        CameraPreset::ResetView => keys::HANDHELD_CAMERA_RESET,
    })
}

/// The frame cap in force: 30 with a swap every other refresh, 60 otherwise.
#[must_use]
pub fn frame_cap(settings: &SettingsModel) -> i32 {
    if settings.vsync() == 2 {
        30
    } else {
        60
    }
}

/// The value after `current`, `step` places on, wrapping round; the first when `current` is not
/// in the list.
fn cycled<'a, T: PartialEq>(values: &'a [T], current: &T, step: i32) -> &'a T {
    let count = i32::try_from(values.len()).unwrap_or(1).max(1);
    let here = values
        .iter()
        .position(|value| value == current)
        .and_then(|index| i32::try_from(index).ok())
        .unwrap_or(0);
    let next = usize::try_from((here + step).rem_euclid(count)).unwrap_or(0);
    &values[next]
}

fn language_codes() -> Vec<&'static str> {
    catalogue::descriptor(SettingId::UiLanguage)
        .text_options()
        .map_or_else(Vec::new, <[&str]>::to_vec)
}

fn nearest_volume_level(volume: f32) -> usize {
    VOLUME_LEVELS
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - volume).abs().total_cmp(&(*b - volume).abs()))
        .map_or(0, |(index, _)| index)
}

fn stepped_volume(volume: f32, step: i32) -> f32 {
    let last = VOLUME_LEVELS.len() - 1;
    let index = nearest_volume_level(volume);
    let next = if step < 0 {
        index.saturating_sub(1)
    } else {
        (index + 1).min(last)
    };
    VOLUME_LEVELS[next]
}

fn volume_percent(volume: f32) -> usize {
    nearest_volume_level(volume) * 10
}

fn render_scale_options() -> Vec<&'static str> {
    catalogue::descriptor(SettingId::VideoResScale)
        .domain()
        .options()
        .map_or_else(Vec::new, |options| {
            options.iter().map(|option| option.label.as_str()).collect()
        })
}

/// The index of the resolution scale in force: the scale only applies to the "Render Tex"
/// method, so the other method is "Native"; the automatic resolution comes after the scales.
fn render_scale_index(settings: &SettingsModel) -> usize {
    if settings.get(SettingId::VideoDynamicResolution).as_bool() {
        return render_scale_options().len();
    }
    if settings.render_method() == RENDER_TEXTURE_METHOD {
        usize::try_from(settings.get(SettingId::VideoResScale).as_int()).unwrap_or(0)
    } else {
        0
    }
}

fn set_render_scale(settings: &mut SettingsModel, step: i32) {
    let scales = i32::try_from(render_scale_options().len())
        .unwrap_or(1)
        .max(1);
    let here = i32::try_from(render_scale_index(settings)).unwrap_or(0);
    let next = (here + step).rem_euclid(scales + 1);
    let automatic = next == scales;
    settings.set(
        SettingId::VideoDynamicResolution,
        SettingValue::Bool(automatic),
    );
    if automatic {
        settings.set_render_method(RENDER_TEXTURE_METHOD);
        return;
    }
    settings.set(SettingId::VideoResScale, SettingValue::Enum(next));
    settings.set_render_method(if next > 0 {
        RENDER_TEXTURE_METHOD
    } else {
        DIRECT_METHOD
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go_to(model: &mut HandheldOptionsModel, row: OptionRow, settings: &mut SettingsModel) {
        while model.selected() != row {
            model.handle(PadButton::Down, settings);
        }
    }

    #[test]
    fn the_region_row_cycles_auto_ntsc_and_pal() {
        let mut settings = SettingsModel::new();
        settings.set_auto_tv_mode(true);
        let mut model = HandheldOptionsModel::new();
        go_to(&mut model, OptionRow::Region, &mut settings);
        assert_eq!(RegionChoice::Auto, RegionChoice::of(&settings));
        model.handle(PadButton::Right, &mut settings);
        assert!(!settings.auto_tv_mode());
        assert_eq!(TvRegionSetting::Ntsc, settings.tv_mode());
        model.handle(PadButton::Right, &mut settings);
        assert_eq!(TvRegionSetting::Pal, settings.tv_mode());
        assert_eq!(
            "PAL",
            value_text(OptionRow::Region, &settings, CameraPreset::Orbit)
        );
        model.handle(PadButton::Right, &mut settings);
        assert!(settings.auto_tv_mode());
        model.handle(PadButton::Left, &mut settings);
        assert_eq!(RegionChoice::Pal, RegionChoice::of(&settings));
    }

    #[test]
    fn the_volume_steps_by_ten_percent_and_stops_at_the_ends() {
        let mut settings = SettingsModel::new();
        let mut model = HandheldOptionsModel::new();
        go_to(&mut model, OptionRow::Volume, &mut settings);
        model.handle(PadButton::Right, &mut settings);
        assert_eq!(100, volume_percent(settings.volume()));
        model.handle(PadButton::Left, &mut settings);
        model.handle(PadButton::Left, &mut settings);
        assert_eq!(80, volume_percent(settings.volume()));
        for _ in 0..20 {
            model.handle(PadButton::Left, &mut settings);
        }
        assert_eq!(0, volume_percent(settings.volume()));
    }

    #[test]
    fn the_render_scale_row_switches_the_render_method_with_it() {
        let mut settings = SettingsModel::new();
        let mut model = HandheldOptionsModel::new();
        go_to(&mut model, OptionRow::RenderScale, &mut settings);
        assert_eq!(0, render_scale_index(&settings));
        model.handle(PadButton::Right, &mut settings);
        assert_eq!(RENDER_TEXTURE_METHOD, settings.render_method());
        assert_eq!(1, settings.get(SettingId::VideoResScale).as_int());
        assert!((settings.res_scale_factor() - 2.0 / 3.0).abs() < 1.0e-6);
        model.handle(PadButton::Left, &mut settings);
        assert_eq!(DIRECT_METHOD, settings.render_method());
        assert!((settings.res_scale_factor() - 1.0).abs() < 1.0e-6);
        // Before Native comes the automatic resolution, then the smallest scale.
        model.handle(PadButton::Left, &mut settings);
        assert!(settings.get(SettingId::VideoDynamicResolution).as_bool());
        assert_eq!(RENDER_TEXTURE_METHOD, settings.render_method());
        assert_eq!(
            "Auto",
            value_text(OptionRow::RenderScale, &settings, CameraPreset::Orbit)
        );
        model.handle(PadButton::Left, &mut settings);
        assert!(!settings.get(SettingId::VideoDynamicResolution).as_bool());
        assert_eq!(
            4,
            settings.get(SettingId::VideoResScale).as_int(),
            "wraps round"
        );
    }

    #[test]
    fn switches_and_the_frame_cap_toggle() {
        let mut settings = SettingsModel::new();
        let mut model = HandheldOptionsModel::new();
        go_to(&mut model, OptionRow::Shadows, &mut settings);
        let shadows = settings.shadow();
        model.handle(PadButton::A, &mut settings);
        assert_eq!(!shadows, settings.shadow());
        go_to(&mut model, OptionRow::FrameCap, &mut settings);
        model.handle(PadButton::A, &mut settings);
        assert_eq!(30, frame_cap(&settings));
        assert_eq!(2, settings.vsync());
        model.handle(PadButton::A, &mut settings);
        assert_eq!(60, frame_cap(&settings));
        assert_eq!(1, settings.vsync());
        go_to(&mut model, OptionRow::ShowFps, &mut settings);
        model.handle(PadButton::Right, &mut settings);
        assert!(settings.show_fps());
    }

    #[test]
    fn the_language_row_cycles_the_catalogue_languages() {
        let mut settings = SettingsModel::new();
        let mut model = HandheldOptionsModel::new();
        assert_eq!(OptionRow::Language, model.selected());
        model.handle(PadButton::Right, &mut settings);
        assert_eq!("fr", settings.language());
        model.handle(PadButton::Right, &mut settings);
        assert_eq!("en", settings.language());
    }

    #[test]
    fn the_camera_and_controls_rows_ask_the_application() {
        let mut settings = SettingsModel::new();
        let mut model = HandheldOptionsModel::new();
        go_to(&mut model, OptionRow::Camera, &mut settings);
        assert_eq!(
            Some(OptionsCommand::StepCamera(-1)),
            model.handle(PadButton::Left, &mut settings)
        );
        assert_eq!(
            Some(OptionsCommand::StepCamera(1)),
            model.handle(PadButton::A, &mut settings)
        );
        go_to(&mut model, OptionRow::Controls, &mut settings);
        assert_eq!(
            Some(OptionsCommand::OpenControls),
            model.handle(PadButton::A, &mut settings)
        );
        assert_eq!(None, model.handle(PadButton::Right, &mut settings));
        assert_eq!(
            Some(OptionsCommand::Close),
            model.handle(PadButton::B, &mut settings)
        );
    }
}
