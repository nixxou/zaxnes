//! The render defaults of a handheld console (Mali-G31 class GPU, 1 GB of memory), and its
//! pipelined frames, applied as environment defaults: they hold only while the user has saved
//! no value of their own.
//!
//! The scene is drawn at the screen's own resolution, as on the desktop (owner decision: the
//! 2/3 scale and the automatic resolution stay choices of the options). Gaps against the
//! research notes (`docs/HANDHELD-RESEARCH.md` §5.4), reported rather than invented: the shadow
//! map size and the post-processing chain are not settings (the scene keeps the one shadow map
//! size, and no post-processing pass exists); the frame cap is the vsync interval (every refresh
//! of a 60 Hz screen), since the frame rate setting has no 30.

use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

/// The choice "Auto" of `emu.pipeline`.
const PIPELINE_AUTO: i32 = 1;

/// Every switch one handheld default sets.
fn defaults() -> [(SettingId, SettingValue); 5] {
    [
        (SettingId::RenderShadow, SettingValue::Bool(false)),
        (SettingId::VideoAntiAlias, SettingValue::Enum(0)),
        // Swap every refresh: 60 frames per second on the 60 Hz screens of the handhelds.
        (SettingId::VideoVSync, SettingValue::Enum(1)),
        // The optimised Lua, and the scripts of a late frame beside the drawing (owner request).
        (SettingId::EmuLuaOptimized, SettingValue::Bool(true)),
        (SettingId::EmuPipeline, SettingValue::Enum(PIPELINE_AUTO)),
    ]
}

/// Makes the handheld values the defaults of `model`.
pub fn apply(model: &mut SettingsModel) {
    for (id, value) in defaults() {
        model.set_environment_default(id, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::app_settings::AppSettings as _;

    #[test]
    fn a_fresh_model_gets_the_handheld_render_defaults() {
        let mut model = SettingsModel::new();
        assert!(model.shadow(), "the desktop default has shadows on");
        apply(&mut model);
        assert!(!model.shadow());
        assert_eq!(crate::render_scaling::DIRECT_METHOD, model.render_method());
        assert!((model.res_scale_factor() - 1.0).abs() < 1.0e-6);
        assert!(!model.get(SettingId::VideoDynamicResolution).as_bool());
        assert_eq!(0, model.anti_alias());
        assert_eq!(1, model.vsync());
        assert!(model.get(SettingId::EmuLuaOptimized).as_bool());
        assert_eq!(PIPELINE_AUTO, model.get(SettingId::EmuPipeline).as_int());
    }

    #[test]
    fn a_value_the_user_set_wins() {
        let mut model = SettingsModel::new();
        apply(&mut model);
        model.set(SettingId::RenderShadow, true);
        model.set(SettingId::VideoVSync, SettingValue::Enum(2));
        assert!(model.shadow());
        assert_eq!(2, model.vsync());
    }
}
