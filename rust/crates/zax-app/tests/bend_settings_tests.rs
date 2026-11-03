//! The five curvature settings from the model to the scene: AMD-043 REQ-SCN-066 (the four
//! angles, their rounding and their persistence) and AMD-046 REQ-SCN-074 (the "Bend 2D Mode"
//! switch), read **when they change** and never per frame.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::TemporaryRoot;
use time::OffsetDateTime;
use zax_app::persistence::settings_store::SettingsStore;
use zax_app::scene_script_bridge::{self, bend_of};
use zax_app::user_data_paths::UserDataPaths;
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::Vec3;
use zax_render::scene::SceneState;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

/// REQ-SCN-066: the four settings default to 0, so the curvature of a fresh model is the
/// identity — which is what makes it free (the renderer tests `is_identity` once per frame).
#[test]
fn req_scn_066_the_default_curvature_is_the_identity() {
    let model = SettingsModel::new();
    assert!(bend_of(&model).is_identity());
    assert_eq!(SpaceBend::NONE, bend_of(&model));

    let mut scene = SceneState::default();
    assert!(
        scene.bend.is_identity(),
        "and so is the scene's own default"
    );
    assert!(
        scene.bend_two_dimensional_mode,
        "REQ-SCN-074 is on by default"
    );
    scene_script_bridge::push_settings(&model, &mut scene);
    assert!(scene.bend.is_identity());
    assert!(scene.bend_two_dimensional_mode);
}

/// REQ-SCN-066, acceptance criterion: writing 45 in Bend Left-Right gives 60 (the rounding of
/// AMD-040 §3, which the setting's stepped domain applies), and the curvature the scene receives
/// is the one of the **rounded** value.
#[test]
fn req_scn_066_writing_45_gives_60_and_the_scene_gets_that_curvature() {
    let mut model = SettingsModel::new();
    model.set(SettingId::SceneBendLr, SettingValue::Int(45));
    assert_eq!(60, model.get(SettingId::SceneBendLr).as_int());

    let mut scene = SceneState::default();
    scene_script_bridge::push_settings(&model, &mut scene);
    assert_eq!(SpaceBend::new(60.0, 0.0, 0.0, 0.0, 1.0), scene.bend);

    // The vector of REQ-SCN-067 for (60, 0, 0, middle) and P = (256, 120, 0).
    let image = scene.bend.apply(Vec3::new(256.0, 120.0, 0.0));
    assert!((image.x - 251.64).abs() <= 0.05, "x {}", image.x);
    assert!((image.z - 24.70).abs() <= 0.05, "z {}", image.z);
}

/// REQ-SCN-066 and REQ-SCN-074: a change of any of the five settings reaches the scene through
/// `apply_setting`, which is the one path the host takes when a setting changes — never per
/// frame.
#[test]
fn req_scn_066_a_change_of_each_setting_reaches_the_scene() {
    let mut model = SettingsModel::new();
    let mut scene = SceneState::default();
    scene_script_bridge::push_settings(&model, &mut scene);

    for (id, value, expected) in [
        (
            SettingId::SceneBendLr,
            90,
            SpaceBend::new(90.0, 0.0, 0.0, 0.0, 1.0),
        ),
        (
            SettingId::SceneBendTb,
            -60,
            SpaceBend::new(90.0, -60.0, 0.0, 0.0, 1.0),
        ),
        (
            SettingId::SceneArcH,
            30,
            SpaceBend::new(90.0, -60.0, 30.0, 0.0, 1.0),
        ),
        (
            SettingId::SceneArcCenter,
            1,
            SpaceBend::new(90.0, -60.0, 30.0, 1.0, 1.0),
        ),
    ] {
        model.set(id, SettingValue::Int(value));
        scene_script_bridge::apply_setting(id, &model, &mut scene);
        assert_eq!(expected, scene.bend, "{id:?}");
    }

    model.set(SettingId::SceneBend2dMode, false);
    scene_script_bridge::apply_setting(SettingId::SceneBend2dMode, &model, &mut scene);
    assert!(!scene.bend_two_dimensional_mode);
    model.set(SettingId::SceneBend2dMode, true);
    scene_script_bridge::apply_setting(SettingId::SceneBend2dMode, &model, &mut scene);
    assert!(scene.bend_two_dimensional_mode);

    // Back to all zero, and the scene's curvature is the identity again (the settings apply hot,
    // from the next rendered frame).
    for id in [
        SettingId::SceneBendLr,
        SettingId::SceneBendTb,
        SettingId::SceneArcH,
    ] {
        model.set(id, SettingValue::Int(0));
        scene_script_bridge::apply_setting(id, &model, &mut scene);
    }
    assert!(scene.bend.is_identity());
}

/// REQ-SCN-066 and REQ-SCN-074, acceptance criterion: the values survive a restart.
#[test]
fn req_scn_066_the_five_settings_survive_a_restart() {
    let folder = TemporaryRoot::new("zax-bend-settings-");
    let paths = UserDataPaths::resolve(Some(folder.path()), None, None);

    let mut model = SettingsModel::new();
    model.set(SettingId::SceneBendLr, SettingValue::Int(45));
    model.set(SettingId::SceneBendTb, SettingValue::Int(-180));
    model.set(SettingId::SceneArcH, SettingValue::Int(120));
    model.set(SettingId::SceneArcCenter, SettingValue::Int(1));
    model.set(SettingId::SceneBend2dMode, false);
    SettingsStore::new(&paths).save(&model);

    let mut restarted = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut restarted, at(0));
    assert_eq!(
        60,
        restarted.get(SettingId::SceneBendLr).as_int(),
        "45 → 60"
    );
    assert_eq!(-180, restarted.get(SettingId::SceneBendTb).as_int());
    assert_eq!(120, restarted.get(SettingId::SceneArcH).as_int());
    assert_eq!(1, restarted.get(SettingId::SceneArcCenter).as_int());
    assert!(!restarted.get(SettingId::SceneBend2dMode).as_bool());
    assert_eq!(bend_of(&model), bend_of(&restarted));
}

/// RO-1 point 7: a settings file written **before** this lot loads unchanged — every rung of the
/// amendment's 30° grid is still a rung — and a value off the new grid is brought onto the nearest
/// one rather than refused or defaulted.
#[test]
fn ro_1_a_stored_curvature_loads_unchanged_or_snaps_to_the_nearest_rung() {
    let folder = TemporaryRoot::new("zax-bend-grid-");
    let paths = UserDataPaths::resolve(Some(folder.path()), None, None);

    // Written as a file from before the lot: the three angles on the 30° grid.
    let mut before = SettingsModel::new();
    before.set(SettingId::SceneBendLr, SettingValue::Int(-150));
    before.set(SettingId::SceneBendTb, SettingValue::Int(90));
    before.set(SettingId::SceneArcH, SettingValue::Int(30));
    SettingsStore::new(&paths).save(&before);

    let mut restarted = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut restarted, at(0));
    assert_eq!(-150, restarted.get(SettingId::SceneBendLr).as_int());
    assert_eq!(90, restarted.get(SettingId::SceneBendTb).as_int());
    assert_eq!(30, restarted.get(SettingId::SceneArcH).as_int());
    assert_eq!(bend_of(&before), bend_of(&restarted));

    // The fine rungs the lot adds survive a restart as themselves.
    let mut fine = SettingsModel::new();
    fine.set(SettingId::SceneBendLr, SettingValue::Int(15));
    fine.set(SettingId::SceneBendTb, SettingValue::Int(-5));
    fine.set(SettingId::SceneArcH, SettingValue::Int(25));
    SettingsStore::new(&paths).save(&fine);
    let mut reloaded = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut reloaded, at(0));
    assert_eq!(15, reloaded.get(SettingId::SceneBendLr).as_int());
    assert_eq!(-5, reloaded.get(SettingId::SceneBendTb).as_int());
    assert_eq!(25, reloaded.get(SettingId::SceneArcH).as_int());
    assert_eq!(bend_of(&fine), bend_of(&reloaded));

    // A hand-edited value off the grid lands on the nearest rung, and the scene gets that one.
    let mut edited = SettingsModel::new();
    edited.set(SettingId::SceneBendLr, SettingValue::Int(13));
    edited.set(SettingId::SceneArcH, SettingValue::Int(-99));
    assert_eq!(15, edited.get(SettingId::SceneBendLr).as_int());
    assert_eq!(-90, edited.get(SettingId::SceneArcH).as_int());
    assert_eq!(SpaceBend::new(15.0, 0.0, -90.0, 0.0, 1.0), bend_of(&edited));
}

/// RO-5 part 2: a settings file written by an older Zaxnes, which could only ever hold 0 or 1 for
/// the arc centre, loads **unchanged** — the key's JSON is a number either way, and the two values
/// AMD-043 names are still rungs — and the three quarters the lot adds survive a restart as
/// themselves, giving the pivot of their own height.
#[test]
fn ro_5_a_stored_arc_centre_loads_unchanged_and_the_quarters_survive_a_restart() {
    let folder = TemporaryRoot::new("zax-arc-centre-");
    let paths = UserDataPaths::resolve(Some(folder.path()), None, None);

    // A file from before the lot: the arc centre written as the whole number 1, and the arc on so
    // that the pivot is read at all.
    let mut before = SettingsModel::new();
    before.set(SettingId::SceneArcH, SettingValue::Int(60));
    before.set(SettingId::SceneArcCenter, SettingValue::Int(1));
    SettingsStore::new(&paths).save(&before);
    let text = std::fs::read_to_string(&paths.settings_file).expect("the file was written");
    assert!(
        text.contains("\"scene.arc_center\": 1"),
        "the file format of the key is unchanged: {text}"
    );

    let mut restarted = SettingsModel::new();
    SettingsStore::new(&paths).load(&mut restarted, at(0));
    assert_eq!(1, restarted.get(SettingId::SceneArcCenter).as_int());
    assert_eq!(bend_of(&before), bend_of(&restarted));
    assert_eq!(
        zax_engine3d::space_bend::EDGE_PIVOT_OFFSET,
        bend_of(&restarted).arc_pivot()
    );

    // Each quarter, written, reloaded and turned into the pivot of its own height.
    for (chosen, pivot) in [(0.25_f32, -30.0_f32), (0.5, -60.0), (0.75, -90.0)] {
        let mut model = SettingsModel::new();
        model.set(SettingId::SceneArcH, SettingValue::Int(60));
        model.set(SettingId::SceneArcCenter, SettingValue::Float(chosen));
        SettingsStore::new(&paths).save(&model);
        let mut reloaded = SettingsModel::new();
        SettingsStore::new(&paths).load(&mut reloaded, at(0));
        assert!(
            (reloaded.get(SettingId::SceneArcCenter).as_float() - chosen).abs() < f32::EPSILON,
            "{chosen} read back as {:?}",
            reloaded.get(SettingId::SceneArcCenter)
        );
        assert_eq!(bend_of(&model), bend_of(&reloaded));
        assert_eq!(pivot, bend_of(&reloaded).arc_pivot(), "{chosen}");
    }

    // A hand-edited value off the grid lands on the nearest quarter, ties away from zero.
    let mut edited = SettingsModel::new();
    edited.set(SettingId::SceneArcCenter, SettingValue::Float(0.4));
    assert!((edited.get(SettingId::SceneArcCenter).as_float() - 0.5).abs() < f32::EPSILON);
    edited.set(SettingId::SceneArcCenter, SettingValue::Float(0.125));
    assert!((edited.get(SettingId::SceneArcCenter).as_float() - 0.25).abs() < f32::EPSILON);
}

/// REQ-SCN-074 note: the setting has **no Lua access** — the original program has none — so it
/// is neither a style parameter of REQ-LUA-114 nor a `Setting` member.
#[test]
fn req_scn_074_the_setting_has_no_lua_access() {
    let descriptor = zax_settings::catalogue::descriptor(SettingId::SceneBend2dMode);
    assert_eq!(None, descriptor.lua_path());
    for style in [0, 1, 4, 7] {
        assert_eq!(
            None,
            zax_settings::styles::parameter(style, "spl.bend_2d_mode")
        );
    }
}
