//! The libretro core option table generated from the model (REQ-PERS-081, REQ-PERS-083, REQ-PERS-085,
//! SPEC-09 §4.6).

#[path = "common/exact.rs"]
mod exact;

use exact::assert_exact;
use rstest::rstest;
use zax_settings::catalogue::{self, SettingId};
use zax_settings::core_options;
use zax_settings::model::SettingsModel;
use zax_settings::values::{Color, SettingValue};

/// REQ-PERS-083: the key mapping of the acceptance criterion.
#[rstest]
#[case("camera.fov", "zaxnes_camera_fov")]
#[case("layer[3].offset", "zaxnes_layer_3__offset")]
#[case("input.port_type[0]", "zaxnes_input_port_type_0_")]
fn the_core_option_key_replaces_the_separators(#[case] key: &str, #[case] expected: &str) {
    assert_eq!(core_options::core_option_key(key), expected);
}

/// REQ-PERS-083 acceptance criterion: the generated table holds `zaxnes_camera_fov` with the values 45
/// to 95 and the default 70.
#[test]
fn the_field_of_view_option_is_generated_from_the_model() {
    let option = core_options::find("zaxnes_camera_fov").expect("zaxnes_camera_fov");
    assert_eq!(option.setting, SettingId::CameraFov);
    assert_eq!(
        option.values,
        ["45", "50", "55", "60", "65", "70", "75", "80", "85", "90", "95"]
    );
    assert_eq!(option.default_value, "70");
    assert_eq!(option.category, "Camera");
}

/// SPEC-09 §4.6: the normative extract, entry by entry. The ten entries are what the generator must
/// produce, with `video.graphic_quality` and `emu.tv_mode` withdrawn by v1.1 and `scene.tv_box` by
/// AMD-031.
#[test]
fn the_table_is_the_normative_extract_of_the_specification() {
    let keys: Vec<&str> = core_options::all()
        .iter()
        .map(|option| option.key.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "zaxnes_video_anti_alias",
            "zaxnes_render_mesh_algo",
            "zaxnes_render_shadow",
            "zaxnes_camera_fov",
            "zaxnes_light_intensity",
            "zaxnes_scene_skybox",
            "zaxnes_scene_custom_bg_color",
            // AMD-031 retired scene.tv_box, so the extract is one entry shorter.
            "zaxnes_emu_game_speed",
            "zaxnes_emu_auto_tv_mode",
            "zaxnes_audio_volume",
        ]
    );
}

/// SPEC-09 §4.6: the values and defaults of the four single-line entries.
#[rstest]
#[case("zaxnes_scene_skybox", "Default", "Default,Color,Gradient,Fantasy")]
#[case(
    "zaxnes_render_mesh_algo",
    "GREEDY",
    "GREEDY,MARCHING,GREEDY_MULTICOLOR"
)]
#[case("zaxnes_render_shadow", "enabled", "enabled,disabled")]
#[case("zaxnes_emu_auto_tv_mode", "disabled", "enabled,disabled")]
fn the_single_line_entries_match(#[case] key: &str, #[case] default: &str, #[case] values: &str) {
    let option = core_options::find(key).expect(key);
    assert_eq!(option.default_value, default);
    assert_eq!(option.values, values.split(',').collect::<Vec<_>>());
}

/// SPEC-09 §4.6: the light intensity runs 0.50 to 1.50 in steps of 0.05.
#[test]
fn the_light_intensity_is_discretised_in_steps_of_five_hundredths() {
    let option = core_options::find("zaxnes_light_intensity").expect("zaxnes_light_intensity");
    assert_eq!(option.values.len(), 21);
    assert_eq!(option.values[0], "0.50");
    assert_eq!(option.values[20], "1.50");
    assert_eq!(option.default_value, "1.00");
}

/// SPEC-09 §4.6: the speed runs −8 to 8.
#[test]
fn the_game_speed_runs_from_minus_eight_to_eight() {
    let option = core_options::find("zaxnes_emu_game_speed").expect("zaxnes_emu_game_speed");
    assert_eq!(option.values.len(), 17);
    assert_eq!(option.values[0], "-8");
    assert_eq!(option.values[16], "8");
    assert_eq!(option.default_value, "0");
}

/// DEC-09-Q5: the background colour is exposed as eight presets.
#[test]
fn the_background_colour_is_exposed_as_eight_presets() {
    let option =
        core_options::find("zaxnes_scene_custom_bg_color").expect("zaxnes_scene_custom_bg_color");
    assert_eq!(
        option.values,
        [
            "Black",
            "White",
            "Gray",
            "NES Sky Blue",
            "Dark Blue",
            "Green",
            "Red",
            "Purple"
        ]
    );
    assert_eq!(option.default_value, "Black");
}

/// REQ-PERS-081: every exposed option offers a non-empty list, without duplicates, that holds its default.
#[test]
fn every_option_offers_a_clean_list_holding_its_default() {
    assert_ne!(core_options::all(), []);
    for option in core_options::all() {
        assert_ne!(option.values, [] as [std::string::String; 0]);
        let mut distinct = option.values.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), option.values.len(), "{}", option.key);
        assert!(
            option.values.contains(&option.default_value),
            "{}",
            option.key
        );
        assert!(option.key.starts_with("zaxnes_"), "{}", option.key);
    }
}

/// REQ-PERS-085: no P, S or D setting is exposed, and the keys §4.6 lists as host-owned are absent.
#[rstest]
#[case(SettingId::VideoVSync)]
#[case(SettingId::VideoFrameRate)]
#[case(SettingId::VideoSkipFrame)]
#[case(SettingId::VideoRenderMethod)]
#[case(SettingId::VideoResScale)]
#[case(SettingId::AudioBgMusic)]
#[case(SettingId::InputEmulatedMenuButton)]
#[case(SettingId::EmuCore)]
#[case(SettingId::UiSortOption)]
#[case(SettingId::UiFullscreen)]
#[case(SettingId::VideoGraphicQuality)]
#[case(SettingId::EmuTvMode)]
#[case(SettingId::ConvEmptyTile)]
#[case(SettingId::CameraSmoothTime)]
#[case(SettingId::VrRuntime)]
fn the_settings_the_host_owns_are_not_exposed(#[case] id: SettingId) {
    assert!(!catalogue::descriptor(id).is_libretro_exposed());
    assert!(core_options::all()
        .iter()
        .all(|option| option.setting != id));
}

/// REQ-PERS-085: no exposed option belongs to a scope other than U.
#[test]
fn every_exposed_option_is_a_user_preference() {
    for option in core_options::all() {
        assert_eq!(
            catalogue::descriptor(option.setting).scope(),
            zax_settings::meta::SettingScope::User,
            "{}",
            option.key
        );
    }
}

/// REQ-PERS-085: a change from the host takes the same path as one from the UI, so it is validated and
/// notified.
#[test]
fn a_change_from_the_host_goes_through_the_model() {
    let mut model = SettingsModel::new();
    assert!(core_options::try_apply(
        &mut model,
        "zaxnes_camera_fov",
        "85"
    ));
    assert_exact(model.get(SettingId::CameraFov).as_float(), 85.0);
    assert!(model
        .drain_changes()
        .any(|change| change.id == SettingId::CameraFov));
    assert!(core_options::try_apply(
        &mut model,
        "zaxnes_render_shadow",
        "disabled"
    ));
    assert!(!model.get(SettingId::RenderShadow).as_bool());
    assert!(core_options::try_apply(
        &mut model,
        "zaxnes_audio_volume",
        "35%"
    ));
    assert!((model.get(SettingId::AudioVolume).as_float() - 0.35).abs() < 1e-4);
    assert!(core_options::try_apply(
        &mut model,
        "zaxnes_scene_custom_bg_color",
        "NES Sky Blue"
    ));
    assert_eq!(
        model.get(SettingId::SceneCustomBgColor).as_color(),
        Color::from_rgb(0x0000_78FC)
    );
}

/// An unknown key or label changes nothing.
#[test]
fn an_unknown_option_or_label_is_refused() {
    let mut model = SettingsModel::new();
    assert!(!core_options::try_apply(
        &mut model,
        "zaxnes_not_a_setting",
        "1"
    ));
    assert!(!core_options::try_apply(
        &mut model,
        "zaxnes_camera_fov",
        "123"
    ));
    assert_exact(model.get(SettingId::CameraFov).as_float(), 70.0);
    assert_eq!(
        model.get(SettingId::SceneCustomBgColor),
        SettingValue::from(Color::BLACK)
    );
}
