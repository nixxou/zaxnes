//! The settings a command-line option imposes for one run (Q-066): they mask the persisted
//! value without replacing it.

use zax_platform::pacing::MAXIMUM_VSYNC;
use zax_settings::catalogue::SettingId;
use zax_settings::model::{SettingLayer, SettingsModel};
use zax_settings::values::SettingValue;

use crate::app_options::AppOptions;

/// The settings the command line can override.
pub const OVERRIDABLE_KEYS: [SettingId; 6] = [
    SettingId::CameraFov,
    SettingId::RenderShadow,
    SettingId::SceneSkybox,
    SettingId::VideoSkipFrame,
    SettingId::VideoFrameRate,
    SettingId::VideoVSync,
];

/// Writes the options the command line carries into the command-line layer of `model`.
pub fn apply(options: &AppOptions, model: &mut SettingsModel) {
    let layer = SettingLayer::CommandLine;
    if let Some(fov) = options.fov {
        model.set_layer(SettingId::CameraFov, fov, layer);
    }
    if let Some(shadows) = options.shadows {
        model.set_layer(SettingId::RenderShadow, shadows, layer);
    }
    if let Some(skybox) = options.skybox {
        model.set_layer(SettingId::SceneSkybox, SettingValue::Enum(skybox), layer);
    }
    if let Some(skip_render) = options.skip_render {
        model.set_layer(SettingId::VideoSkipFrame, skip_render, layer);
    }
    if let Some(frame_rate) = options.frame_rate {
        model.set_layer(
            SettingId::VideoFrameRate,
            SettingValue::Int(frame_rate),
            layer,
        );
    }
    if let Some(vsync) = options.vsync {
        model.set_layer(
            SettingId::VideoVSync,
            SettingValue::Enum(vsync.clamp(0, MAXIMUM_VSYNC)),
            layer,
        );
    }
}
