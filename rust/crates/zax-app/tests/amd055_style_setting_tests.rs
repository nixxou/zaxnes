//! The visual style of AMD-055 REQ-SCN-078 and its twelve parameters of REQ-SCN-079, from the
//! settings model to the scene: read **when they change** and never per frame.
//!
//! RU-2 created the thirteen settings and their Lua surface; what this lot adds is that they
//! reach the renderer. The round trips of the style identifier (1 written, 4 read back; anything
//! else 0) are asserted here on the real model, because REQ-SCN-078 states them of the setting
//! and not only of the Lua method.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::TemporaryRoot;
use time::OffsetDateTime;
use zax_app::persistence::settings_store::SettingsStore;
use zax_app::scene_script_bridge;
use zax_app::user_data_paths::UserDataPaths;
use zax_render::scene::SceneState;
use zax_render::style::{AmbientOcclusion, StyleSettings};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

/// The thirteen settings of AMD-055 §4, style first.
const STYLE_SETTINGS: [SettingId; 13] = [
    SettingId::RenderVisualStyle,
    SettingId::RenderStyleAo,
    SettingId::RenderStyleAoStrength,
    SettingId::RenderStyleAoRadius,
    SettingId::RenderStyleAoThickness,
    SettingId::RenderStyleAoQuality,
    SettingId::RenderStylePixelShadow,
    SettingId::RenderStyleVoxelGrid,
    SettingId::RenderStyleReflections,
    SettingId::RenderStyleSsrShine,
    SettingId::RenderStyleSsrClarity,
    SettingId::RenderStyleSsrLength,
    SettingId::RenderStyleSsrQuality,
];

/// REQ-SCN-078, the first acceptance criterion, on the **setting**: 4 reads back 4, 1 reads back
/// 4, 2 reads back 0, and a value out of the domain — including one that came from the settings
/// file (REQ-PERS-003) — reads back 0.
#[test]
fn req_scn_078_the_style_identifier_round_trips() {
    let mut model = SettingsModel::new();
    assert_eq!(
        0,
        model.get(SettingId::RenderVisualStyle).as_int(),
        "default"
    );

    for (written, read_back) in [
        (4.0, 4),
        (1.0, 4),
        (0.0, 0),
        (2.0, 0),
        (7.0, 0),
        (-1.0, 0),
        (5.0, 0),
    ] {
        model.set(SettingId::RenderVisualStyle, written);
        let scene_style = scene_script_bridge::style_of(&model);
        assert_eq!(
            read_back,
            scene_style.identifier(),
            "render.visual_style = {written}"
        );
        assert_eq!(read_back == 4, scene_style.enhanced);
    }
}

/// REQ-SCN-078: the style and each of the twelve reach the scene through the one bridge every
/// other scene setting uses, so nothing reads the model per frame (REQ-PERS-007: the change
/// applies at the next rendered frame, without reloading the game).
#[test]
fn req_scn_078_the_style_and_its_parameters_reach_the_scene_when_they_change() {
    let mut model = SettingsModel::new();
    let mut scene = SceneState::default();
    assert_eq!(StyleSettings::default(), scene.settings.style);

    model.set(SettingId::RenderVisualStyle, 4.0);
    scene_script_bridge::apply_setting(SettingId::RenderVisualStyle, &model, &mut scene);
    assert!(scene.settings.style.enhanced);

    // Each parameter, one at a time, at a value that is not its default.
    let changes: [(SettingId, f32); 12] = [
        (SettingId::RenderStyleAo, 2.0),
        (SettingId::RenderStyleAoStrength, 5.0),
        (SettingId::RenderStyleAoRadius, 1.0),
        (SettingId::RenderStyleAoThickness, 4.0),
        (SettingId::RenderStyleAoQuality, 2.0),
        (SettingId::RenderStylePixelShadow, 1.0),
        (SettingId::RenderStyleVoxelGrid, 3.0),
        (SettingId::RenderStyleReflections, 1.0),
        (SettingId::RenderStyleSsrShine, 1.0),
        (SettingId::RenderStyleSsrClarity, 5.0),
        (SettingId::RenderStyleSsrLength, 4.0),
        (SettingId::RenderStyleSsrQuality, 3.0),
    ];
    for (id, value) in changes {
        model.set(id, value);
        scene_script_bridge::apply_setting(id, &model, &mut scene);
    }
    let style = scene.settings.style;
    assert_eq!(AmbientOcclusion::Depth, style.ao);
    assert_eq!(5, style.ao_strength);
    assert_eq!(1, style.ao_radius);
    assert_eq!(4, style.ao_thickness);
    assert_eq!(2, style.ao_quality);
    assert!(style.pixel_shadow);
    assert_eq!(3, style.voxel_grid);
    assert!(style.reflections);
    assert_eq!(1, style.ssr_shine);
    assert_eq!(5, style.ssr_clarity);
    assert_eq!(4, style.ssr_length);
    assert_eq!(3, style.ssr_quality);

    // And the wholesale push a profile load and a start-up take gives the same value.
    let mut fresh = SceneState::default();
    scene_script_bridge::push_settings(&model, &mut fresh);
    assert_eq!(style, fresh.settings.style);

    // Back to Classic: every parameter is kept, and every effect is off (REQ-SCN-078, §3 point 4
    // — a change of style never loses a parameter).
    model.set(SettingId::RenderVisualStyle, 0.0);
    scene_script_bridge::apply_setting(SettingId::RenderVisualStyle, &model, &mut scene);
    assert!(!scene.settings.style.enhanced);
    assert_eq!(3, scene.settings.style.voxel_grid, "the parameter is kept");
    assert!(!scene.settings.style.voxel_grid_active());
    assert!(!scene.settings.style.screen_effects_active());
}

/// REQ-SCN-078: the curvature is **not** a style effect. A change of style leaves the four
/// curvature settings and the scene's curvature exactly where they were (AMD-055 §3 point 4,
/// reading A).
#[test]
fn req_scn_078_a_change_of_style_keeps_the_curvature() {
    let mut model = SettingsModel::new();
    let mut scene = SceneState::default();
    model.set(SettingId::SceneBendLr, 60.0);
    scene_script_bridge::apply_setting(SettingId::SceneBendLr, &model, &mut scene);
    let bent = scene.bend;
    assert!(!bent.is_identity(), "60° bends");

    for style in [4.0, 0.0, 1.0, 7.0] {
        model.set(SettingId::RenderVisualStyle, style);
        scene_script_bridge::apply_setting(SettingId::RenderVisualStyle, &model, &mut scene);
        assert_eq!(
            bent, scene.bend,
            "style {style} must not touch the curvature"
        );
    }
}

/// REQ-SCN-079, first acceptance criterion: each parameter written from the settings model
/// survives a restart, and the normalisation of AMD-040 §3 bounds then snaps it — `ao.strength`
/// 9 → 5, 1.4 → 1, 0 → 1; `spl.voxel_grid` 4 → 3; `spl.ao` 1.4 → 1.
#[test]
fn req_scn_079_the_parameters_are_bounded_snapped_and_persisted() {
    let folder = TemporaryRoot::new("zax-amd055-style-");
    let paths = UserDataPaths::resolve(Some(folder.path()), None, None);
    let mut model = SettingsModel::new();

    for (id, written, expected) in [
        (SettingId::RenderStyleAoStrength, 9.0, 5),
        (SettingId::RenderStyleAoStrength, 1.4, 1),
        (SettingId::RenderStyleAoStrength, 0.0, 1),
        (SettingId::RenderStyleVoxelGrid, 4.0, 3),
        (SettingId::RenderStyleAo, 1.4, 1),
        (SettingId::RenderStyleSsrQuality, 9.0, 3),
        (SettingId::RenderStyleSsrShine, -2.0, 1),
    ] {
        model.set(id, written);
        assert_eq!(expected, model.get(id).as_int(), "{id:?} written {written}");
    }

    // A value for every one of the thirteen, then a restart.
    let values: [f32; 13] = [
        4.0, 2.0, 5.0, 1.0, 4.0, 2.0, 1.0, 3.0, 1.0, 1.0, 5.0, 4.0, 3.0,
    ];
    for (id, value) in STYLE_SETTINGS.iter().zip(values) {
        model.set(*id, value);
    }
    SettingsStore::new(&paths).save(&model);

    let mut restarted = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut restarted, at(0));
    for (id, value) in STYLE_SETTINGS.iter().zip(values) {
        assert!(
            (value - restarted.get(*id).as_float()).abs() < 1e-6,
            "{id:?} across a restart"
        );
    }
}

/// REQ-SCN-078: a settings **file** holding 1 reads back 4 and one holding 7 reads back 0 — the
/// rule of the "Domaine" column of AMD-055 §4, which REQ-PERS-003 applies on load.
#[test]
fn req_scn_078_a_settings_file_of_one_reads_back_four() {
    for (stored, read_back) in [(1.0, 4), (4.0, 4), (7.0, 0), (2.0, 0), (0.0, 0)] {
        let folder = TemporaryRoot::new("zax-amd055-file-");
        let paths = UserDataPaths::resolve(Some(folder.path()), None, None);
        let mut model = SettingsModel::new();
        model.set(SettingId::RenderVisualStyle, stored);
        SettingsStore::new(&paths).save(&model);

        let mut restarted = SettingsModel::new();
        SettingsStore::new(&paths).load(&mut restarted, at(0));
        let style = scene_script_bridge::style_of(&restarted);
        assert_eq!(read_back, style.identifier(), "file holding {stored}");
    }
}
