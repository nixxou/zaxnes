//! The settings the loop and the interface read by name: typed accessors over the model
//! (SPEC-09 §4). The model itself is the single source of truth (REQ-PERS-007).

use zax_nes::NesRegion;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use crate::emulation::speed_setting;

/// The default value of the speed setting.
pub const SPEED_SETTING_DEFAULT: i32 = 0;
/// The frame rates the options window offers.
pub const FRAME_RATE_DOMAIN: [i32; 3] = [60, 90, 120];

/// The TV mode setting (`emu.tv_mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TvRegionSetting {
    /// 60 Hz.
    Ntsc = 0,
    /// 50 Hz.
    Pal = 1,
    /// Not offered by the interface; treated as NTSC timing (SPEC-02).
    Multi = 2,
    /// 50 Hz with the Dendy timing.
    Dendy = 3,
}

impl TvRegionSetting {
    /// The setting for the number the model holds; unknown numbers read as NTSC.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Pal,
            2 => Self::Multi,
            3 => Self::Dendy,
            _ => Self::Ntsc,
        }
    }

    /// The region the core runs in.
    #[must_use]
    pub const fn region(self) -> NesRegion {
        match self {
            Self::Pal => NesRegion::Pal,
            Self::Dendy => NesRegion::Dendy,
            Self::Ntsc | Self::Multi => NesRegion::Ntsc,
        }
    }

    /// The setting that stands for a detected region.
    #[must_use]
    pub const fn of_region(region: NesRegion) -> Self {
        match region {
            NesRegion::Pal => Self::Pal,
            NesRegion::Dendy => Self::Dendy,
            NesRegion::Ntsc | NesRegion::Multi => Self::Ntsc,
        }
    }
}

/// A frame rate the options window offers, or 90 for any other value.
#[must_use]
pub const fn validate_frame_rate(value: i32) -> i32 {
    match value {
        60 | 90 | 120 => value,
        _ => 90,
    }
}

/// Typed reads of the settings the application uses.
pub trait AppSettings {
    /// `emu.game_speed`, the integer speed setting.
    fn game_speed(&self) -> i32;
    /// `emu.tv_mode`.
    fn tv_mode(&self) -> TvRegionSetting;
    /// `emu.auto_tv_mode`.
    fn auto_tv_mode(&self) -> bool;
    /// `audio.volume`, 0 to 1.
    fn volume(&self) -> f32;
    /// `audio.bg_music`: the menu music.
    fn menu_music(&self) -> bool;
    /// `audio.mute`.
    fn mute(&self) -> bool;
    /// The volume the mixer plays at: 0 when muted.
    fn effective_volume(&self) -> f32;
    /// `video.skip_frame`.
    fn skip_frame(&self) -> bool;
    /// `video.frame_rate`.
    fn frame_rate(&self) -> i32;
    /// `video.vsync`: 0 off, 1 every refresh, 2 every other refresh.
    fn vsync(&self) -> i32;
    /// `video.anti_alias`.
    fn anti_alias(&self) -> i32;
    /// `video.render_method`.
    fn render_method(&self) -> i32;
    /// `video.res_scale_factor`.
    fn res_scale_factor(&self) -> f32;
    /// `render.shadow`.
    fn shadow(&self) -> bool;
    /// `scene.skybox`.
    fn skybox(&self) -> i32;
    /// `camera.fov`.
    fn fov(&self) -> f32;
    /// `light.intensity`.
    fn light_intensity(&self) -> f32;
    /// `ui.fullscreen`.
    fn fullscreen(&self) -> bool;
    /// `ui.show_fps`.
    fn show_fps(&self) -> bool;
    /// `ui.sort_option`.
    fn sort_option(&self) -> i32;
    /// `ui.language`, one of the codes the catalogue offers.
    fn language(&self) -> String;
    /// `ui.last_browse_dir`.
    fn last_browse_directory(&self) -> String;
    /// `camera.local_cam`.
    fn local_camera(&self) -> bool;
    /// The factor `k` of the speed setting.
    fn speed_factor(&self) -> f64;
    /// The region the TV mode stands for.
    fn region(&self) -> NesRegion;
}

impl AppSettings for SettingsModel {
    fn game_speed(&self) -> i32 {
        self.get(SettingId::EmuGameSpeed).as_int()
    }

    fn tv_mode(&self) -> TvRegionSetting {
        TvRegionSetting::from_raw(self.get(SettingId::EmuTvMode).as_int())
    }

    fn auto_tv_mode(&self) -> bool {
        self.get(SettingId::EmuAutoTvMode).as_bool()
    }

    fn volume(&self) -> f32 {
        self.get(SettingId::AudioVolume).as_float()
    }

    fn menu_music(&self) -> bool {
        self.get(SettingId::AudioBgMusic).as_bool()
    }

    fn mute(&self) -> bool {
        self.get(SettingId::AudioMute).as_bool()
    }

    fn effective_volume(&self) -> f32 {
        if self.mute() {
            0.0
        } else {
            self.volume()
        }
    }

    fn skip_frame(&self) -> bool {
        self.get(SettingId::VideoSkipFrame).as_bool()
    }

    fn frame_rate(&self) -> i32 {
        self.get(SettingId::VideoFrameRate).as_int()
    }

    fn vsync(&self) -> i32 {
        self.get(SettingId::VideoVSync).as_int()
    }

    fn anti_alias(&self) -> i32 {
        self.get(SettingId::VideoAntiAlias).as_int()
    }

    fn render_method(&self) -> i32 {
        self.get(SettingId::VideoRenderMethod).as_int()
    }

    fn res_scale_factor(&self) -> f32 {
        self.get(SettingId::VideoResScaleFactor).as_float()
    }

    fn shadow(&self) -> bool {
        self.get(SettingId::RenderShadow).as_bool()
    }

    fn skybox(&self) -> i32 {
        self.get(SettingId::SceneSkybox).as_int()
    }

    fn fov(&self) -> f32 {
        self.get(SettingId::CameraFov).as_float()
    }

    fn light_intensity(&self) -> f32 {
        self.get(SettingId::LightIntensity).as_float()
    }

    fn fullscreen(&self) -> bool {
        self.get(SettingId::UiFullscreen).as_bool()
    }

    fn show_fps(&self) -> bool {
        self.get(SettingId::UiShowFps).as_bool()
    }

    fn sort_option(&self) -> i32 {
        self.get(SettingId::UiSortOption).as_int()
    }

    fn language(&self) -> String {
        zax_settings::catalogue::descriptor(SettingId::UiLanguage)
            .coerce_text(self.get(SettingId::UiLanguage).as_text())
    }

    fn last_browse_directory(&self) -> String {
        self.get(SettingId::UiLastBrowseDir).as_text().to_owned()
    }

    fn local_camera(&self) -> bool {
        self.get(SettingId::CameraLocalCam).as_bool()
    }

    fn speed_factor(&self) -> f64 {
        speed_setting::factor(self.game_speed())
    }

    fn region(&self) -> NesRegion {
        self.tv_mode().region()
    }
}

/// Typed writes of the settings the application uses, in the user layer.
pub trait AppSettingsMut {
    /// Sets `emu.game_speed`.
    fn set_game_speed(&mut self, value: i32);
    /// Sets `emu.tv_mode`.
    fn set_tv_mode(&mut self, value: TvRegionSetting);
    /// Sets `emu.auto_tv_mode`.
    fn set_auto_tv_mode(&mut self, value: bool);
    /// Sets `audio.volume`.
    fn set_volume(&mut self, value: f32);
    /// Sets `audio.bg_music`.
    fn set_menu_music(&mut self, value: bool);
    /// Sets `audio.mute`.
    fn set_mute(&mut self, value: bool);
    /// Sets `video.skip_frame`.
    fn set_skip_frame(&mut self, value: bool);
    /// Sets `video.frame_rate`.
    fn set_frame_rate(&mut self, value: i32);
    /// Sets `video.vsync`, brought into 0 to 2.
    fn set_vsync(&mut self, value: i32);
    /// Sets `video.anti_alias`, brought into 0 to 2.
    fn set_anti_alias(&mut self, value: i32);
    /// Sets `video.render_method`, brought into 0 to 1.
    fn set_render_method(&mut self, value: i32);
    /// Sets `render.shadow`.
    fn set_shadow(&mut self, value: bool);
    /// Sets `scene.skybox`.
    fn set_skybox(&mut self, value: i32);
    /// Sets `camera.fov`.
    fn set_fov(&mut self, value: f32);
    /// Sets `light.intensity`.
    fn set_light_intensity(&mut self, value: f32);
    /// Sets `ui.fullscreen`.
    fn set_fullscreen(&mut self, value: bool);
    /// Sets `ui.show_fps`.
    fn set_show_fps(&mut self, value: bool);
    /// Sets `ui.sort_option`, brought into 0 to 1.
    fn set_sort_option(&mut self, value: i32);
    /// Sets `ui.language`.
    fn set_language(&mut self, value: &str);
    /// Sets `ui.last_browse_dir`.
    fn set_last_browse_directory(&mut self, value: &str);
    /// Sets `camera.local_cam`.
    fn set_local_camera(&mut self, value: bool);
    /// Takes the TV mode of a detected region.
    fn apply_detected_region(&mut self, region: NesRegion);
}

impl AppSettingsMut for SettingsModel {
    fn set_game_speed(&mut self, value: i32) {
        self.set(SettingId::EmuGameSpeed, SettingValue::Int(value));
    }

    fn set_tv_mode(&mut self, value: TvRegionSetting) {
        self.set(SettingId::EmuTvMode, SettingValue::Enum(value as i32));
    }

    fn set_auto_tv_mode(&mut self, value: bool) {
        self.set(SettingId::EmuAutoTvMode, value);
    }

    fn set_volume(&mut self, value: f32) {
        self.set(SettingId::AudioVolume, value);
    }

    fn set_menu_music(&mut self, value: bool) {
        self.set(SettingId::AudioBgMusic, value);
    }

    fn set_mute(&mut self, value: bool) {
        self.set(SettingId::AudioMute, value);
    }

    fn set_skip_frame(&mut self, value: bool) {
        self.set(SettingId::VideoSkipFrame, value);
    }

    fn set_frame_rate(&mut self, value: i32) {
        self.set(SettingId::VideoFrameRate, SettingValue::Int(value));
    }

    fn set_vsync(&mut self, value: i32) {
        self.set(SettingId::VideoVSync, SettingValue::Enum(value.clamp(0, 2)));
    }

    fn set_anti_alias(&mut self, value: i32) {
        self.set(
            SettingId::VideoAntiAlias,
            SettingValue::Enum(value.clamp(0, 2)),
        );
    }

    fn set_render_method(&mut self, value: i32) {
        self.set(
            SettingId::VideoRenderMethod,
            SettingValue::Enum(value.clamp(0, 1)),
        );
    }

    fn set_shadow(&mut self, value: bool) {
        self.set(SettingId::RenderShadow, value);
    }

    fn set_skybox(&mut self, value: i32) {
        self.set(SettingId::SceneSkybox, SettingValue::Enum(value));
    }

    fn set_fov(&mut self, value: f32) {
        self.set(SettingId::CameraFov, value);
    }

    fn set_light_intensity(&mut self, value: f32) {
        self.set(SettingId::LightIntensity, value);
    }

    fn set_fullscreen(&mut self, value: bool) {
        self.set(SettingId::UiFullscreen, value);
    }

    fn set_show_fps(&mut self, value: bool) {
        self.set(SettingId::UiShowFps, value);
    }

    fn set_sort_option(&mut self, value: i32) {
        self.set(
            SettingId::UiSortOption,
            SettingValue::Enum(value.clamp(0, 1)),
        );
    }

    fn set_language(&mut self, value: &str) {
        self.set(SettingId::UiLanguage, value);
    }

    fn set_last_browse_directory(&mut self, value: &str) {
        self.set(SettingId::UiLastBrowseDir, value);
    }

    fn set_local_camera(&mut self, value: bool) {
        self.set(SettingId::CameraLocalCam, value);
    }

    fn apply_detected_region(&mut self, region: NesRegion) {
        self.set_tv_mode(TvRegionSetting::of_region(region));
    }
}
