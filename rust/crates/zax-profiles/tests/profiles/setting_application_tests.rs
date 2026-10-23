use std::collections::HashSet;

use zax_profiles::enums::{LayerAlignment, ProfileLayer, TvRegionMode};
use zax_profiles::setting_application::{
    applied_keys, preserved_keys, values_to_apply, AppliedValue,
};
use zax_profiles::settings::{LayerSettings, LayerSettingsSet, ProfileSettings};
use zax_profiles::values::{ArgbColor, Vec3F};
use zax_settings::catalogue::{Layer, SettingId};

use super::common::neutral_assets;

fn keys(ids: &[SettingId]) -> Vec<String> {
    ids.iter().map(|id| id.key().into_owned()).collect()
}

#[test]
fn req_prof_041_the_applied_and_preserved_keys_are_two_disjoint_sets() {
    let applied = applied_keys();
    let preserved = preserved_keys();
    let applied_set: HashSet<SettingId> = applied.iter().copied().collect();
    let preserved_set: HashSet<SettingId> = preserved.iter().copied().collect();

    assert!(applied_set.is_disjoint(&preserved_set));
    assert_eq!(applied.len(), applied_set.len());
    assert_eq!(preserved.len(), preserved_set.len());
}

#[test]
fn req_prof_041_the_light_direction_and_colour_are_applied_but_not_its_intensity() {
    let applied = applied_keys();
    assert!(applied.contains(&SettingId::LightDirection));
    assert!(applied.contains(&SettingId::LightColor));
    assert!(preserved_keys().contains(&SettingId::LightIntensity));
    assert!(!applied.contains(&SettingId::LightIntensity));
}

#[test]
fn req_prof_041_the_user_settings_of_the_second_list_are_all_preserved() {
    let expected = [
        "light.intensity", "render.mesh_algo", "vr.runtime", "scene.skybox", "emu.core",
        "emu.game_speed", "render.shadow", "audio.volume", "video.frame_rate", "video.vsync",
        "vr.control_method", "vr.scene_ratio", "input.emulated_menu_button",
        "video.render_method", "video.res_scale", "video.skip_frame", "audio.bg_music",
        // scene.tv_box stood between emu.auto_tv_mode and ui.sort_option; AMD-031 retired the
        // setting with the TV box, so there is no preference left to preserve.
        "video.anti_alias", "vr.mirroring", "emu.auto_tv_mode",
        "ui.sort_option", "scene.custom_bg_color", "camera.fov",
    ];

    assert_eq!(expected.to_vec(), keys(&preserved_keys()));
}

#[test]
fn req_prof_041_the_applied_keys_cover_the_whole_first_list() {
    let expected = [
        "light.direction", "light.color",
        "clip.left", "clip.bottom", "clip.right", "clip.top",
        "scene.map_enable", "layer.default_layer", "layer.auto",
        "video.graphic_quality", "editor.saved_patterns", "editor.saved_anim_patterns",
        "conv.same_bg_palette", "conv.same_sp_palette", "conv.sprite_tolerance",
        "conv.scan_direction", "conv.empty_tile", "camera.local_cam", "emu.tv_mode",
        "scene.use_nes_bg_color",
    ];

    let applied = keys(&applied_keys());
    for key in expected {
        assert!(applied.iter().any(|applied| applied == key), "{key}");
    }

    for index in 0..LayerSettingsSet::LAYER_COUNT {
        for field in ["offset", "aligned", "cast_shadow", "recv_shadow"] {
            let key = format!("layer[{index}].{field}");
            assert!(applied.iter().any(|applied| *applied == key), "{key}");
        }
    }

    assert_eq!(expected.len() + LayerSettingsSet::LAYER_COUNT * 4, applied.len());
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_041_loading_empty_bytes_applies_the_documented_values() {
    let document = neutral_assets::load_profile("empty.bytes");
    let settings = document.profile.settings.as_ref().unwrap();
    let values = values_to_apply(settings);

    assert!(matches!(
        values[&SettingId::LightDirection],
        Some(AppliedValue::Vector(direction)) if direction == Vec3F::new(1.0, 0.2, 1.0)
    ));
    assert!(matches!(
        values[&SettingId::LightColor],
        Some(AppliedValue::Color(color)) if color == ArgbColor::new(-1)
    ));
    for (key, expected) in [
        (SettingId::ClipLeft, 4.1_f32),
        (SettingId::ClipBottom, 16.1),
        (SettingId::ClipRight, 4.1),
        (SettingId::ClipTop, 0.1),
    ] {
        assert!(matches!(values[&key], Some(AppliedValue::Float(value)) if value == expected), "{key:?}");
    }
    assert!(matches!(values[&SettingId::VideoGraphicQuality], Some(AppliedValue::Int(2))));
    assert!(matches!(
        values[&SettingId::LayerDefaultLayer],
        Some(AppliedValue::Layer(ProfileLayer::L1))
    ));
    assert!(matches!(values[&SettingId::EmuTvMode], Some(AppliedValue::TvMode(TvRegionMode::Ntsc))));

    // Forced true whatever the profile says.
    assert!(matches!(values[&SettingId::SceneUseNesBgColor], Some(AppliedValue::Bool(true))));

    // Never applied, so never offered here.
    assert!(!values.contains_key(&SettingId::LightIntensity));
    assert!(!values.contains_key(&SettingId::AudioVolume));
    assert!(!values.contains_key(&SettingId::CameraFov));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_042_the_applied_shadow_flags_ignore_what_the_profile_stores() {
    let document = neutral_assets::load_profile("empty.bytes");
    let settings = document.profile.settings.as_ref().unwrap();

    // The file stores (F,F) (F,F) (T,T) (F,T) (F,T) (F,T).
    let layers = settings.layers.as_ref().unwrap();
    assert!(layers.layers[2].stored_cast_shadow);
    assert!(!layers.layers[3].stored_cast_shadow);
    assert!(!layers.layers[0].stored_recv_shadow);

    let values = values_to_apply(settings);
    let expected_cast = [false, false, true, false, false, false];
    let expected_recv = [false, false, true, true, true, true];

    for (index, layer) in Layer::ALL.into_iter().enumerate() {
        assert!(matches!(
            values[&SettingId::LayerCastShadow(layer)],
            Some(AppliedValue::Bool(value)) if value == expected_cast[index]
        ));
        assert!(matches!(
            values[&SettingId::LayerRecvShadow(layer)],
            Some(AppliedValue::Bool(value)) if value == expected_recv[index]
        ));
    }
}

#[test]
fn req_prof_040_a_settings_message_starts_at_the_documented_defaults() {
    let settings = ProfileSettings::default();

    assert!((settings.fov - 70.0).abs() < f32::EPSILON);
    assert_eq!(-1, settings.skybox);
    assert!(settings.mirroring);
    assert!((settings.scene_ratio - 1.0).abs() < f32::EPSILON);
    assert!(settings.skip_frame);
    assert!(settings.background_music);

    assert!(!settings.empty_tile);
    assert!(!settings.use_nes_background_color);
    assert_eq!(0, settings.graphic_quality);
    assert!(settings.volume.abs() < f32::EPSILON);
    assert_eq!(TvRegionMode::Ntsc, settings.tv_mode);
}

#[test]
fn req_prof_040_a_layer_without_shadow_flags_stores_them_as_false() {
    let layer = LayerSettings::default();

    assert!(!layer.stored_cast_shadow);
    assert!(!layer.stored_recv_shadow);
    assert_eq!(LayerAlignment::Front, layer.aligned);
    assert!((layer.coefficient() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn req_pers_013_the_layer_coefficient_follows_the_alignment() {
    let coefficient = |aligned| LayerSettings { aligned, ..LayerSettings::default() }.coefficient();

    assert!((coefficient(LayerAlignment::Front) - 1.0).abs() < f32::EPSILON);
    assert!(coefficient(LayerAlignment::Center).abs() < f32::EPSILON);
    assert!((coefficient(LayerAlignment::Back) + 1.0).abs() < f32::EPSILON);
}

#[test]
fn req_pers_002_the_default_layer_set_has_the_documented_offsets() {
    let layers = LayerSettingsSet::create_default();
    let offset = |index| layers.get(index).unwrap().offset;

    assert_eq!(LayerSettingsSet::LAYER_COUNT, layers.layers.len());
    assert_eq!(ProfileLayer::L1, layers.default_layer);
    assert!(offset(2).abs() < f32::EPSILON);
    assert!((offset(3) - 12.0).abs() < f32::EPSILON);
    assert!((offset(4) - 24.0).abs() < f32::EPSILON);
    assert!((offset(5) - 36.0).abs() < f32::EPSILON);
    assert_eq!(LayerAlignment::Center, layers.get(1).unwrap().aligned);
    assert_eq!(LayerAlignment::Front, layers.get(2).unwrap().aligned);
    assert!(layers.get(6).is_none());
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_044_the_memorised_slots_are_kept_with_their_names_and_patterns() {
    let document = neutral_assets::load_profile("empty.bytes");
    let settings = document.profile.settings.as_ref().unwrap();

    assert_eq!(5, settings.saved_patterns.len());
    assert_eq!(5, settings.saved_anim_patterns.len());

    for slot in &settings.saved_patterns {
        let pattern = slot.pattern.as_ref().unwrap();
        assert!(pattern.grid.is_none());
        assert_eq!(Some(zax_profiles::values::Vec2I::new(0, 0)), pattern.tile_size);
        assert_eq!(1, pattern.variants.len());
    }
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_044_every_built_in_profile_carries_ten_slots() {
    let mut slots = 0;
    for path in neutral_assets::profile_files() {
        let document = neutral_assets::load_profile(&path.file_name().unwrap().to_string_lossy());
        let settings = document.profile.settings.as_ref().unwrap();
        assert_eq!(5, settings.saved_patterns.len());
        assert_eq!(5, settings.saved_anim_patterns.len());
        slots += settings.saved_patterns.len() + settings.saved_anim_patterns.len();
    }

    assert_eq!(1010, slots);
}
