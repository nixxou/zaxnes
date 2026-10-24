//! The catalogue against the transcription of SPEC-09 §4.1 (REQ-PERS-001, REQ-PERS-002,
//! REQ-PERS-008, REQ-PERS-080, REQ-PERS-084, REQ-ARCH-021).

#[path = "common/catalogue_table.rs"]
mod catalogue_table;
#[path = "common/exact.rs"]
mod exact;

use exact::assert_exact;

use std::collections::HashSet;

use catalogue_table::{describe, rows};
use rstest::rstest;
use zax_settings::catalogue::{self, Layer, SettingId};
use zax_settings::meta::{SettingEffect, SettingScope};
use zax_settings::model::SettingsModel;

/// REQ-PERS-001: every setting of §4.1 is in the model.
#[test]
fn every_setting_of_the_tables_is_present_with_its_kind_default_scope_and_effect() {
    for row in rows() {
        let id = SettingId::from_key(&row.key).expect(&row.key);
        let descriptor = catalogue::descriptor(id);
        assert_eq!(row.kind, descriptor.kind(), "{}", row.key);
        assert_eq!(row.scope, descriptor.scope(), "{}", row.key);
        assert_eq!(row.effect, descriptor.effect(), "{}", row.key);
        assert_eq!(row.lua.as_deref(), descriptor.lua_path(), "{}", row.key);
        assert_eq!(
            row.default,
            describe(descriptor.default_value()),
            "{}",
            row.key
        );
    }
}

/// REQ-PERS-001: the enumeration returns at least the keys of §4.1. Anything beyond them is a
/// setting of this reimplementation, which must carry a new key.
#[test]
fn the_enumeration_covers_the_tables_and_names_its_additions() {
    let specified: HashSet<String> = rows().into_iter().map(|row| row.key).collect();
    let model: HashSet<String> = catalogue::descriptors()
        .map(|d| d.key().into_owned())
        .collect();
    assert!(specified.is_subset(&model));
    let mut additions: Vec<String> = model.difference(&specified).cloned().collect();
    additions.sort();
    // emu.builtin_patches, emu.catch_up, emu.lua_optimized, emu.lua_smooth_gc, emu.pipeline,
    // ui.profiler, video.backend, video.dynamic_resolution, video.widescreen,
    // video.widescreen_extra and video.widescreen_strict_edges: Rust-only settings (owner
    // request).
    assert_eq!(
        additions,
        [
            "audio.mute",
            "emu.builtin_patches",
            "emu.catch_up",
            "emu.lua_optimized",
            "emu.lua_smooth_gc",
            "emu.pipeline",
            "ui.language",
            "ui.profiler",
            "ui.show_fps",
            "video.backend",
            "video.dynamic_resolution",
            "video.widescreen",
            "video.widescreen_extra",
            "video.widescreen_strict_edges"
        ]
    );
}

/// REQ-PERS-080: every key matches the published grammar and appears once.
#[test]
fn keys_are_stable_and_unique() {
    let keys: Vec<String> = catalogue::descriptors()
        .map(|d| d.key().into_owned())
        .collect();
    let distinct: HashSet<&String> = keys.iter().collect();
    assert_eq!(keys.len(), distinct.len());
    for key in &keys {
        assert!(catalogue::is_valid_key(key), "{key}");
    }
}

/// REQ-PERS-080: the key set against the versioned reference list. A key that disappears without
/// being recorded as withdrawn fails here.
#[test]
fn no_published_key_disappeared() {
    let reference: Vec<&str> = include_str!("data/SettingKeys.reference.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_ne!(reference, [] as [&str; 0]);
    let keys: HashSet<String> = catalogue::descriptors()
        .map(|d| d.key().into_owned())
        .collect();
    let missing: Vec<&&str> = reference
        .iter()
        .filter(|key| !keys.contains(**key))
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
}

/// REQ-PERS-002: on a first launch, with no settings file, the acceptance criterion's values hold.
#[test]
fn first_launch_values_are_the_defaults_of_the_tables() {
    let model = SettingsModel::new();
    assert_exact(model.get(SettingId::CameraFov).as_float(), 70.0);
    assert_eq!(model.get(SettingId::VideoFrameRate).as_int(), 90);
    assert_eq!(model.get(SettingId::SceneSkybox).as_int(), -1);
    assert_exact(model.get(SettingId::AudioVolume).as_float(), 1.0);
    assert!(model.get(SettingId::RenderShadow).as_bool());
    assert_eq!(
        model.get(SettingId::LightDirection).as_vector3(),
        zax_settings::values::Vector3::new(1.0, -1.0, 1.0)
    );
    assert_exact(
        model.get(SettingId::LayerOffset(Layer::ALL[2])).as_float(),
        0.0,
    );
    assert_exact(
        model.get(SettingId::LayerOffset(Layer::ALL[5])).as_float(),
        36.0,
    );
}

/// REQ-PERS-008: the Lua aliases are exposed as well as the main names.
#[rstest]
#[case("SamePalette", SettingId::ConvSameBgPalette)]
#[case("SameBgPalette", SettingId::ConvSameBgPalette)]
#[case("TopDownCam", SettingId::CameraLocalCam)]
#[case("LocalCam", SettingId::CameraLocalCam)]
fn lua_aliases_designate_the_same_setting(#[case] path: &str, #[case] id: SettingId) {
    assert_eq!(SettingId::from_lua_path(path), Some(id));
}

/// REQ-PERS-008, §5.6: the access paths the built-in profiles use all resolve. The method calls
/// `GetInputType` and `SetInputType` are not properties and are checked by SPEC-08's lot instead.
#[rstest]
#[case("EmptyTile")]
#[case("Clipping[0]")]
#[case("Clipping[1]")]
#[case("Clipping[2]")]
#[case("Clipping[3]")]
#[case("Layer[2].CastShadow")]
#[case("Layer[3].CastShadow")]
#[case("Layer[4].CastShadow")]
#[case("Layer[5].CastShadow")]
#[case("BgColor")]
#[case("Light.Direction")]
#[case("CustomBgColor")]
#[case("SamePalette")]
#[case("Light.Color")]
#[case("LocalCam")]
#[case("GameSpeed")]
#[case("VrRuntime")]
#[case("Layer[3].Offset")]
#[case("Layer[3].Aligned")]
#[case("TopDownCam")]
#[case("Layer[2].RecvShadow")]
#[case("SmoothTime")]
#[case("SameBgPalette")]
#[case("SceneRatio")]
fn the_paths_the_built_in_profiles_use_all_resolve(#[case] path: &str) {
    assert!(SettingId::from_lua_path(path).is_some(), "{path}");
}

/// REQ-PERS-007: the members marked "lecture" in §4.2 are not assignable.
#[rstest]
#[case(SettingId::VideoTargetFramerate)]
#[case(SettingId::VideoTargetTimePerFrame)]
#[case(SettingId::SceneSkyboxEnable)]
#[case(SettingId::InputZapperEnable)]
#[case(SettingId::VideoResScaleFactor)]
#[case(SettingId::EmuTvData)]
fn read_only_members_are_not_writable_from_lua(#[case] id: SettingId) {
    assert!(!catalogue::descriptor(id).is_lua_writable());
}

/// REQ-PERS-007: `SceneScale` is the one derived member Lua may assign, because writing it writes
/// `SceneRatio` (REQ-PERS-013 point 3).
#[test]
fn scene_scale_is_the_one_writable_derived_member() {
    let writable: Vec<String> = catalogue::of_scope(SettingScope::Derived)
        .filter(|d| d.is_lua_writable())
        .map(|d| d.key().into_owned())
        .collect();
    assert_eq!(writable, ["scene.scene_scale"]);
}

/// REQ-PERS-084: the three examples of the acceptance criterion.
#[test]
fn the_moment_of_effect_is_exposed_as_metadata() {
    assert_eq!(
        catalogue::descriptor(SettingId::CameraFov).effect(),
        SettingEffect::Immediate
    );
    assert_eq!(
        catalogue::descriptor(SettingId::EmuAutoTvMode).effect(),
        SettingEffect::NextRomLoad
    );
    assert_eq!(
        catalogue::descriptor(SettingId::UiWindowRect).effect(),
        SettingEffect::Restart
    );
}

/// REQ-PERS-020: scope U, and `ui.fullscreen` among them, is what gets persisted; nothing else is.
#[test]
fn only_user_preferences_are_persisted() {
    assert!(catalogue::descriptor(SettingId::UiFullscreen).is_persisted());
    for descriptor in catalogue::descriptors() {
        assert_eq!(
            descriptor.is_persisted(),
            descriptor.scope() == SettingScope::User,
            "{}",
            descriptor.key()
        );
    }
}

/// REQ-PROF-041: the profile-scoped settings of §4.1 are exactly the list SPEC-03 publishes, which is
/// authoritative where the two differ.
#[test]
fn the_profile_scoped_settings_are_the_list_of_req_prof_041() {
    let mut expected: Vec<SettingId> = vec![
        SettingId::LightDirection,
        SettingId::LightColor,
        SettingId::ClipLeft,
        SettingId::ClipBottom,
        SettingId::ClipRight,
        SettingId::ClipTop,
        SettingId::SceneMapEnable,
        SettingId::LayerDefaultLayer,
        SettingId::LayerAuto,
        SettingId::VideoGraphicQuality,
        SettingId::EditorSavedPatterns,
        SettingId::EditorSavedAnimPatterns,
        SettingId::ConvSameBgPalette,
        SettingId::ConvSameSpPalette,
        SettingId::ConvSpriteTolerance,
        SettingId::ConvScanDirection,
        SettingId::ConvEmptyTile,
        SettingId::CameraLocalCam,
        SettingId::EmuTvMode,
        SettingId::SceneUseNesBgColor,
    ];
    for layer in Layer::ALL {
        expected.extend([
            SettingId::LayerOffset(layer),
            SettingId::LayerAligned(layer),
            SettingId::LayerCastShadow(layer),
            SettingId::LayerRecvShadow(layer),
        ]);
    }
    let mut expected: Vec<String> = expected
        .into_iter()
        .map(|id| id.key().into_owned())
        .collect();
    expected.sort();
    let mut actual: Vec<String> = catalogue::of_scope(SettingScope::Profile)
        .map(|d| d.key().into_owned())
        .collect();
    actual.sort();
    assert_eq!(actual, expected);
}

/// REQ-PROF-041 second list: the user keeps these across a profile load, the light intensity included.
#[rstest]
#[case(SettingId::LightIntensity)]
#[case(SettingId::RenderMeshAlgo)]
#[case(SettingId::SceneSkybox)]
#[case(SettingId::EmuGameSpeed)]
#[case(SettingId::RenderShadow)]
#[case(SettingId::AudioVolume)]
#[case(SettingId::SceneCustomBgColor)]
#[case(SettingId::CameraFov)]
fn the_preserved_settings_are_user_scoped(#[case] id: SettingId) {
    assert_eq!(catalogue::descriptor(id).scope(), SettingScope::User);
}

/// REQ-ARCH-021: the enumeration gives key, type, values or bounds and default for every setting.
#[test]
fn every_setting_announces_a_type_and_a_domain_or_bounds() {
    for descriptor in catalogue::descriptors() {
        assert!(!descriptor.key().is_empty());
        assert_eq!(
            descriptor.default_value().kind(),
            descriptor.kind(),
            "{}",
            descriptor.key()
        );
    }
}
