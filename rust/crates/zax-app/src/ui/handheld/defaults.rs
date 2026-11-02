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
fn defaults() -> [(SettingId, SettingValue); 10] {
    [
        // Run-ahead (AMD-041, exposed as `Setting.RunAheadFrames` by AMD-040 REQ-LUA-114) costs
        // a whole extra emulated frame, plus a state saved and loaded, for every frame shown:
        // off on a handheld, whose budget is the frame itself. The desktop keeps the 1 of
        // AMD-041 §3. Nothing reads the setting until RU-3 makes it act, but the default is set
        // here so that the lot that does inherits the right one.
        (SettingId::EmuRunAhead, SettingValue::Int(0)),
        (SettingId::RenderShadow, SettingValue::Bool(false)),
        (SettingId::VideoAntiAlias, SettingValue::Enum(0)),
        // Swap every refresh: 60 frames per second on the 60 Hz screens of the handhelds.
        (SettingId::VideoVSync, SettingValue::Enum(1)),
        // The optimised Lua, and the scripts of a late frame beside the drawing (owner request).
        (SettingId::EmuLuaOptimized, SettingValue::Bool(true)),
        // And the experimental paths over them (owner request).
        (SettingId::EmuLuaExperimental, SettingValue::Bool(true)),
        (SettingId::EmuPipeline, SettingValue::Enum(PIPELINE_AUTO)),
        // No conversion and no frame hooks while the 2D picture is shown (owner request).
        (SettingId::EmuPause3dIn2d, SettingValue::Bool(true)),
        // Only the shown frame of a catch-up is converted (owner request).
        (SettingId::EmuCatchUpLast3d, SettingValue::Bool(true)),
        // AMD-052 REQ-LOOP-074 takes the desktop catch-up to 2; the handheld keeps 5 (owner
        // decision). The history is in `docs/PERFORMANCE-HISTORY.md` §5.5 and §7: a limit of 2
        // measured worse there than both None and 5 once the core had its own thread, because
        // the second frame of a pair waits for the emulation thread in series.
        (SettingId::EmuCatchUp, SettingValue::Enum(5)),
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
        assert!(model.get(SettingId::EmuLuaExperimental).as_bool());
        assert_eq!(PIPELINE_AUTO, model.get(SettingId::EmuPipeline).as_int());
        assert!(model.get(SettingId::EmuPause3dIn2d).as_bool());
        assert!(model.get(SettingId::EmuCatchUpLast3d).as_bool());
        // Offered in the options, off by default (owner decision).
        assert!(!model.get(SettingId::VideoHalfRate3d).as_bool());
        // The run-ahead of AMD-041 is off on a handheld and 1 on the desktop.
        assert_eq!(0, model.get(SettingId::EmuRunAhead).as_int());
        assert_eq!(1, SettingsModel::new().get(SettingId::EmuRunAhead).as_int());
        // The settings of the other 2.0 members cost nothing — they have no effect at all — so
        // they keep the default of their amendment here (AMD-059 REQ-LUA-116, AMD-055 §4).
        assert_eq!(2, model.get(SettingId::VideoMaxQueuedFrames).as_int());
        assert_eq!(2, model.get(SettingId::VideoMsaa20).as_int());
        assert_eq!(90, model.get(SettingId::VrRefreshRate).as_int());
        assert_eq!(0, model.get(SettingId::UiLibraryView).as_int());
        assert_eq!(0, model.get(SettingId::RenderVisualStyle).as_int());
        assert_eq!(3, model.get(SettingId::RenderStyleAoStrength).as_int());
        // AMD-043 REQ-SCN-066: the four curvature settings are 0 on a handheld as on the
        // desktop, which is their own default. At 0 the curvature costs nothing anywhere — no
        // shader variant is compiled, no surface is subdivided — so there is nothing to turn off
        // here; the lot's own measurements are in `docs/PERFORMANCE-HISTORY.md`.
        for id in [
            SettingId::SceneBendLr,
            SettingId::SceneBendTb,
            SettingId::SceneArcH,
            SettingId::SceneArcCenter,
        ] {
            assert_eq!(0, model.get(id).as_int(), "{id:?}");
            assert_eq!(0, SettingsModel::new().get(id).as_int(), "{id:?} desktop");
        }
        // AMD-046 REQ-SCN-074 keeps its own default on both platforms: the rectangle of the 2D
        // mode bends with the scene, and a handheld only pays for it while a curvature is set.
        assert!(model.get(SettingId::SceneBend2dMode).as_bool());
        assert!(SettingsModel::new()
            .get(SettingId::SceneBend2dMode)
            .as_bool());
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
