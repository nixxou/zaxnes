//! The layers of values and what a profile load does to them (REQ-PERS-005 to REQ-PERS-013,
//! REQ-PERS-016, REQ-PERS-017, REQ-PERS-082).

#[path = "common/close.rs"]
mod close;
#[path = "common/exact.rs"]
mod exact;

use close::assert_close;
use exact::assert_exact;
use zax_settings::catalogue::{self, Layer, Port, SettingId};
use zax_settings::meta::{SettingEffect, SettingScope, SettingTheme};
use zax_settings::model::{SettingLayer, SettingsModel};
use zax_settings::values::{Color, SettingValue, Vector3};

/// The user preference of `id` that would be persisted, if any.
fn preference(model: &SettingsModel, id: SettingId) -> Vec<SettingValue> {
    model
        .user_preferences()
        .filter(|(key, _)| *key == id)
        .map(|(_, value)| value.clone())
        .collect()
}

/// The effective cast flags of the six layers.
fn cast_flags(model: &SettingsModel) -> Vec<bool> {
    Layer::ALL
        .iter()
        .map(|layer| model.get(SettingId::LayerCastShadow(*layer)).as_bool())
        .collect()
}

/// The effective receive flags of the six layers.
fn recv_flags(model: &SettingsModel) -> Vec<bool> {
    Layer::ALL
        .iter()
        .map(|layer| model.get(SettingId::LayerRecvShadow(*layer)).as_bool())
        .collect()
}

/// REQ-PERS-005 and §5.4: loading a different game replaces every P setting with the profile's value,
/// keeps every U setting, forces `BgColor` true and resets the six pairs of layer shadow flags.
#[test]
fn loading_a_different_game_replaces_the_profile_settings_and_keeps_the_preferences() {
    let mut model = SettingsModel::new();
    // State before: the user's preferences and the previous game's profile values.
    model.set(SettingId::LightIntensity, 1.2_f32);
    model.set(SettingId::SceneSkybox, SettingValue::Int(1));
    model.set(SettingId::CameraFov, 80.0_f32);
    model.set_layer(SettingId::ConvEmptyTile, true, SettingLayer::Profile);
    model.set_layer(
        SettingId::EmuTvMode,
        SettingValue::Int(1),
        SettingLayer::Profile,
    );
    model.set_layer(
        SettingId::VideoGraphicQuality,
        SettingValue::Int(2),
        SettingLayer::Profile,
    );
    model.apply_profile([
        (SettingId::LightIntensity, SettingValue::Float(1.0)),
        (
            SettingId::LightDirection,
            Vector3::new(-1.0, -1.0, 1.0).into(),
        ),
        (SettingId::ConvEmptyTile, false.into()),
        (SettingId::EmuTvMode, SettingValue::Enum(0)),
        (SettingId::VideoGraphicQuality, SettingValue::Enum(0)),
        (SettingId::SceneUseNesBgColor, false.into()),
        (SettingId::SceneSkybox, SettingValue::Enum(0)),
        (SettingId::CameraFov, SettingValue::Float(60.0)),
    ]);
    // Preferences survive, the light intensity included (REQ-PERS-005 point 2).
    assert_close(model.get(SettingId::LightIntensity).as_float(), 1.2, 4);
    assert_eq!(model.get(SettingId::SceneSkybox).as_int(), 1);
    assert_exact(model.get(SettingId::CameraFov).as_float(), 80.0);
    // Profile settings are replaced.
    assert_eq!(
        model.get(SettingId::LightDirection).as_vector3(),
        Vector3::new(-1.0, -1.0, 1.0)
    );
    assert!(!model.get(SettingId::ConvEmptyTile).as_bool());
    assert_eq!(model.get(SettingId::EmuTvMode).as_int(), 0);
    assert_eq!(model.get(SettingId::VideoGraphicQuality).as_int(), 0);
    // Forced true whatever the profile says (point 3).
    assert!(model.get(SettingId::SceneUseNesBgColor).as_bool());
    // Shadow flags reset whatever the profile stores (point 4).
    assert_eq!(
        cast_flags(&model),
        [false, false, true, false, false, false]
    );
    assert_eq!(recv_flags(&model), [false, false, true, true, true, true]);
}

/// REQ-PERS-005 acceptance criterion: a profile that sets `CastShadow` on every layer still ends up
/// with layer 2 alone casting.
#[test]
fn a_profile_that_sets_every_cast_shadow_flag_is_still_reset() {
    let mut model = SettingsModel::new();
    model.set(
        SettingId::SceneCustomBgColor,
        Color::new(1.0, 0.0, 0.0, 1.0),
    );
    model.set(SettingId::LightIntensity, 1.3_f32);
    let mut values = vec![
        (
            SettingId::LightDirection,
            SettingValue::from(Vector3::new(0.0, -1.0, 1.0)),
        ),
        (SettingId::SceneUseNesBgColor, false.into()),
    ];
    for layer in Layer::ALL {
        values.push((SettingId::LayerCastShadow(layer), true.into()));
    }
    model.apply_profile(values);
    assert_close(model.get(SettingId::LightIntensity).as_float(), 1.3, 4);
    assert_eq!(
        model.get(SettingId::LightDirection).as_vector3(),
        Vector3::new(0.0, -1.0, 1.0)
    );
    assert_eq!(
        model.get(SettingId::SceneCustomBgColor).as_color(),
        Color::new(1.0, 0.0, 0.0, 1.0)
    );
    assert!(model.get(SettingId::SceneUseNesBgColor).as_bool());
    assert_eq!(
        cast_flags(&model),
        [false, false, true, false, false, false]
    );
}

/// REQ-PERS-005 note: a profile setting the file does not carry takes the default of REQ-PROF-040, not
/// the value the previous game left behind.
#[test]
fn a_setting_absent_from_the_profile_takes_its_default_not_the_previous_value() {
    let mut model = SettingsModel::new();
    model.set_layer(SettingId::ConvEmptyTile, true, SettingLayer::Profile);
    model.set_layer(SettingId::ClipBottom, 16.1_f32, SettingLayer::Profile);
    model.apply_profile([(SettingId::CameraLocalCam, SettingValue::Bool(true))]);
    assert!(!model.get(SettingId::ConvEmptyTile).as_bool());
    assert_close(model.get(SettingId::ClipBottom).as_float(), 8.1, 4);
    assert!(model.get(SettingId::CameraLocalCam).as_bool());
}

/// REQ-PERS-006: relaunching the same game does not reapply the profile, so what the scripts changed
/// during the previous session of that game survives.
#[test]
fn relaunching_the_same_game_keeps_what_the_scripts_changed() {
    let mut model = SettingsModel::new();
    model.apply_profile([(SettingId::ConvEmptyTile, SettingValue::Bool(false))]);
    // A script turns it on during the game, outside the start-up hook.
    model.set_layer(SettingId::ConvEmptyTile, true, SettingLayer::Session);
    model.end_of_game();
    // Same game again: no profile application, so the value survives.
    assert!(model.get(SettingId::ConvEmptyTile).as_bool());
    // Game B then game A: the profile's value is back.
    model.apply_profile([(SettingId::ConvEmptyTile, SettingValue::Bool(false))]);
    assert!(!model.get(SettingId::ConvEmptyTile).as_bool());
}

/// REQ-PERS-009: an effective change raises one event with its key; writing the same value raises none.
#[test]
fn only_an_effective_change_raises_an_event() {
    let mut model = SettingsModel::new();
    model.set(SettingId::CameraFov, 80.0_f32);
    let seen: Vec<_> = model.drain_changes().collect();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].id, SettingId::CameraFov);
    assert_eq!(seen[0].effect, SettingEffect::Immediate);
    assert_exact(seen[0].value.as_float(), 80.0);
    model.set(SettingId::CameraFov, 80.0_f32);
    assert_eq!(model.drain_changes().count(), 0);
}

/// REQ-PERS-010 and DEC-09-Q3: a script's write on a user preference is a session value; it is what
/// everyone reads until the game ends, and the preference comes back afterwards.
#[test]
fn a_script_write_on_a_preference_lives_in_the_session_layer() {
    let mut model = SettingsModel::new();
    let blue = Color::new(0.0, 0.0, 1.0, 1.0);
    let scripted = Color::new(0.2, 0.2, 0.2, 1.0);
    model.set(SettingId::SceneCustomBgColor, blue);
    model.set_layer(
        SettingId::SceneCustomBgColor,
        scripted,
        SettingLayer::Session,
    );
    assert_eq!(
        model.get(SettingId::SceneCustomBgColor).as_color(),
        scripted
    );
    assert!(model.has_session_value(SettingId::SceneCustomBgColor));
    // What gets persisted is still the preference, never the script's value.
    assert_eq!(
        preference(&model, SettingId::SceneCustomBgColor),
        [SettingValue::from(blue)]
    );
    model.end_of_game();
    assert_eq!(model.get(SettingId::SceneCustomBgColor).as_color(), blue);
}

/// REQ-PERS-011: the VR runtime reads 0 and a write is ignored without error.
#[test]
fn the_vr_runtime_always_reads_zero() {
    let mut model = SettingsModel::new();
    assert!(!model.set(SettingId::VrRuntime, SettingValue::Int(1)));
    assert_eq!(model.get(SettingId::VrRuntime).as_int(), 0);
}

/// REQ-PERS-011: the scene ratio reads 1, the written value is kept, and a change of the kept value is
/// still notified so the scene can run its view reset.
#[test]
fn the_scene_ratio_reads_one_but_keeps_what_was_written() {
    let mut model = SettingsModel::new();
    assert!(model.set(SettingId::VrSceneRatio, 5.0_f32));
    assert_exact(model.get(SettingId::VrSceneRatio).as_float(), 1.0);
    assert_exact(model.get_stored(SettingId::VrSceneRatio).as_float(), 5.0);
    assert!(model
        .drain_changes()
        .any(|change| change.id == SettingId::VrSceneRatio));
    // A second write of the same value changes nothing.
    assert!(!model.set(SettingId::VrSceneRatio, 5.0_f32));
    assert_eq!(model.drain_changes().count(), 0);
}

/// REQ-PERS-013 point 3: writing `SceneScale` writes the scene ratio.
#[test]
fn writing_the_scene_scale_writes_the_scene_ratio() {
    let mut model = SettingsModel::new();
    model.set(SettingId::SceneSceneScale, 3.0_f32);
    assert_exact(model.get_stored(SettingId::VrSceneRatio).as_float(), 3.0);
    // On desktop the ratio reads 1, so the scale stays RootScale × 1.
    assert_exact(model.get(SettingId::SceneSceneScale).as_float(), 1.0);
}

/// REQ-PERS-012: at the end of a game the speed goes back to 0 and the smoothing time to 0.4.
#[test]
fn the_end_of_a_game_resets_the_speed_and_the_smoothing_time() {
    let mut model = SettingsModel::new();
    model.set(SettingId::EmuGameSpeed, 4.0_f32);
    model.set_layer(SettingId::CameraSmoothTime, 0.7_f32, SettingLayer::Session);
    model.end_of_game();
    assert_exact(model.get(SettingId::EmuGameSpeed).as_float(), 0.0);
    assert_close(model.get(SettingId::CameraSmoothTime).as_float(), 0.4, 4);
}

/// REQ-PERS-012 and DEC-09-Q6: both ports go back to Gamepad at every load of a different game, so a
/// Zapper profile cannot leak into the next game.
#[test]
fn loading_a_different_game_puts_both_ports_back_to_gamepad() {
    let mut model = SettingsModel::new();
    model.set(SettingId::InputPortType(Port::ALL[0]), SettingValue::Int(1));
    assert!(model.get(SettingId::InputZapperEnable).as_bool());
    model.apply_profile([]);
    assert_eq!(
        model.get(SettingId::InputPortType(Port::ALL[0])).as_int(),
        0
    );
    assert_eq!(
        model.get(SettingId::InputPortType(Port::ALL[1])).as_int(),
        0
    );
    assert!(!model.get(SettingId::InputZapperEnable).as_bool());
}

/// REQ-PERS-013 points 1 and 2: the target frame rate and the frame time.
#[test]
fn the_target_frame_rate_follows_vsync_then_the_render_rate() {
    let mut model = SettingsModel::default();
    model.set_display_refresh_rate(144.0);
    model.set(SettingId::VideoVSync, SettingValue::Int(0));
    model.set(SettingId::VideoFrameRate, SettingValue::Int(120));
    assert_exact(model.get(SettingId::VideoTargetFramerate).as_float(), 120.0);
    assert_close(
        model.get(SettingId::VideoTargetTimePerFrame).as_float(),
        0.008_333,
        5,
    );
    model.set(SettingId::VideoVSync, SettingValue::Int(1));
    assert_exact(model.get(SettingId::VideoTargetFramerate).as_float(), 144.0);
    model.set(SettingId::VideoVSync, SettingValue::Int(2));
    assert_exact(model.get(SettingId::VideoTargetFramerate).as_float(), 72.0);
}

/// REQ-PERS-013 points 4 to 8: the other derived values.
#[test]
fn the_derived_values_follow_the_settings_that_feed_them() {
    let mut model = SettingsModel::new();
    assert!(!model.get(SettingId::SceneSkyboxEnable).as_bool());
    model.set(SettingId::SceneSkybox, SettingValue::Int(2));
    assert!(model.get(SettingId::SceneSkyboxEnable).as_bool());
    assert_exact(
        model.get(SettingId::LayerCoef(Layer::ALL[1])).as_float(),
        0.0,
    );
    assert_exact(
        model.get(SettingId::LayerCoef(Layer::ALL[2])).as_float(),
        1.0,
    );
    for (index, factor) in [
        (0, 1.0_f32),
        (1, 2.0 / 3.0),
        (2, 0.5),
        (3, 1.0 / 3.0),
        (4, 0.25),
    ] {
        model.set(SettingId::VideoResScale, SettingValue::Int(index));
        assert_close(
            model.get(SettingId::VideoResScaleFactor).as_float(),
            factor,
            5,
        );
    }
}

/// REQ-PERS-013: a change of a feeding setting also announces the derived value, so a subsystem can
/// watch the derived key alone.
#[test]
fn a_change_announces_the_derived_values_it_makes_stale() {
    let mut model = SettingsModel::new();
    model.set(SettingId::SceneSkybox, SettingValue::Int(1));
    assert!(model
        .drain_changes()
        .any(|c| c.id == SettingId::SceneSkyboxEnable));
    model.set(SettingId::LayerAligned(Layer::ALL[3]), SettingValue::Int(2));
    assert!(model
        .drain_changes()
        .any(|c| c.id == SettingId::LayerCoef(Layer::ALL[3])));
}

/// REQ-PERS-016: the movie mode stays at NONE.
#[test]
fn the_movie_mode_is_frozen_at_none() {
    let mut model = SettingsModel::new();
    assert!(!model.set(SettingId::MovieMode, SettingValue::Int(2)));
    assert_eq!(model.get(SettingId::MovieMode).as_int(), 0);
}

/// REQ-PERS-017: the three settings with no observable effect are still stored and handed back, and
/// they say so through their effect metadata.
#[test]
fn the_settings_without_effect_are_still_stored() {
    for id in [
        SettingId::SceneMapEnable,
        SettingId::InputEmulatedMenuButton,
    ] {
        let descriptor = catalogue::descriptor(id);
        assert_eq!(descriptor.effect(), SettingEffect::NoEffect);
        let layer = if descriptor.scope() == SettingScope::Profile {
            SettingLayer::Profile
        } else {
            SettingLayer::User
        };
        let mut model = SettingsModel::new();
        model.set_layer(id, true, layer);
        assert!(model.get(id).as_bool(), "{}", descriptor.key());
    }
}

/// REQ-PERS-017: `video.anti_alias` is stored and exposed; it may drive a full-screen pass, so its
/// effect is immediate rather than none.
#[test]
fn the_anti_aliasing_setting_is_stored_and_exposed() {
    let mut model = SettingsModel::new();
    model.set(SettingId::VideoAntiAlias, SettingValue::Int(2));
    assert_eq!(model.get(SettingId::VideoAntiAlias).as_int(), 2);
    assert!(catalogue::descriptor(SettingId::VideoAntiAlias).is_libretro_exposed());
}

/// REQ-PERS-082: two models in the same process are independent and nothing touches a disk.
#[test]
fn two_models_are_independent() {
    let mut first = SettingsModel::new();
    let second = SettingsModel::new();
    first.set(SettingId::CameraFov, 95.0_f32);
    first.set_layer(SettingId::ConvEmptyTile, true, SettingLayer::Profile);
    assert_exact(second.get(SettingId::CameraFov).as_float(), 70.0);
    assert!(!second.get(SettingId::ConvEmptyTile).as_bool());
}

/// Q-066: a command-line override masks the preference without replacing it, so the preferences that
/// get persisted are untouched (REQ-PERS-020); the user's own change ends the override, and a script's
/// write still masks it while a game runs (REQ-PERS-010).
#[test]
fn a_command_line_override_masks_the_preference_without_replacing_it() {
    let mut model = SettingsModel::new();
    model.set(SettingId::VideoVSync, SettingValue::Enum(2));
    model.set_layer(
        SettingId::VideoVSync,
        SettingValue::Enum(0),
        SettingLayer::CommandLine,
    );
    assert_eq!(model.get(SettingId::VideoVSync).as_int(), 0);
    assert!(model.has_command_line_value(SettingId::VideoVSync));
    // Only the user layer is persisted, and it still holds what the user chose.
    assert_eq!(
        preference(&model, SettingId::VideoVSync),
        [SettingValue::Enum(2)]
    );
    // A script's write wins while the game runs and is undone at the end of it; the override is then
    // back in force.
    model.set_layer(
        SettingId::VideoVSync,
        SettingValue::Enum(1),
        SettingLayer::Session,
    );
    assert_eq!(model.get(SettingId::VideoVSync).as_int(), 1);
    model.clear_session_layer();
    assert_eq!(model.get(SettingId::VideoVSync).as_int(), 0);
    // The user's own change ends the override for that key, and is persisted.
    model.set(SettingId::VideoVSync, SettingValue::Enum(1));
    assert_eq!(model.get(SettingId::VideoVSync).as_int(), 1);
    assert!(!model.has_command_line_value(SettingId::VideoVSync));
    assert!(model.drain_changes().any(|c| c.id == SettingId::VideoVSync));
    assert_eq!(
        preference(&model, SettingId::VideoVSync),
        [SettingValue::Enum(1)]
    );
}

/// AMD-010 REQ-UI-202: the per-theme "Default values" button.
#[test]
fn resetting_a_theme_takes_its_settings_back_to_their_defaults() {
    let mut model = SettingsModel::new();
    model.set(SettingId::CameraFov, 95.0_f32);
    model.set(SettingId::AudioVolume, 0.2_f32);
    model.reset_theme(SettingTheme::Camera);
    assert_exact(model.get(SettingId::CameraFov).as_float(), 70.0);
    assert_close(model.get(SettingId::AudioVolume).as_float(), 0.2, 4);
}

/// The options of a game (Rust-only) mask the preference and the profile while it plays, a
/// script still masks them, the user's choice on such a setting changes the game's value and
/// not the preference, and the end of the game drops them.
#[test]
fn the_options_of_a_game_sit_between_the_preferences_and_the_scripts() {
    // Shadows are on by default: the preference is off, the game wants them.
    let mut model = SettingsModel::new();
    model.set(SettingId::RenderShadow, false);
    model.set(SettingId::EmuCatchUp, SettingValue::Enum(1));
    model.drain_changes().for_each(drop);

    model.set_game_values([(SettingId::RenderShadow, SettingValue::Bool(true))]);
    assert!(model.get(SettingId::RenderShadow).as_bool());
    assert!(model.has_game_value(SettingId::RenderShadow));
    let changed: Vec<SettingId> = model.drain_changes().map(|change| change.id).collect();
    assert_eq!(changed, [SettingId::RenderShadow]);
    assert_eq!(
        preference(&model, SettingId::RenderShadow),
        [SettingValue::Bool(false)]
    );

    model.set_layer(SettingId::RenderShadow, false, SettingLayer::Session);
    assert!(
        !model.get(SettingId::RenderShadow).as_bool(),
        "a script wins"
    );
    model.clear_session_layer();
    assert!(model.get(SettingId::RenderShadow).as_bool());

    assert!(!model.take_game_edited());
    model.set(SettingId::RenderShadow, false);
    assert!(model.take_game_edited());
    assert_eq!(
        model.game_values().get(&SettingId::RenderShadow),
        Some(&SettingValue::Bool(false))
    );
    model.set(SettingId::EmuCatchUp, SettingValue::Enum(2));
    assert!(
        !model.take_game_edited(),
        "a setting the game has no value for"
    );
    assert_eq!(
        preference(&model, SettingId::EmuCatchUp),
        [SettingValue::Enum(2)]
    );

    model.set_game_values([(SettingId::RenderShadow, SettingValue::Bool(true))]);
    model.end_of_game();
    assert_eq!(0, model.game_values().len());
    assert!(
        !model.get(SettingId::RenderShadow).as_bool(),
        "the preference again"
    );
}
