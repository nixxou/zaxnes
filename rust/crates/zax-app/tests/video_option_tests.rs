//! Port of the application tests of `VideoOptionTests.cs`. The console-attach tests belong to
//! the command-line front end.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::LoopHarness;
use rstest::rstest;
use zax_app::app_options::AppOptions;
use zax_app::application::should_open_fullscreen;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_app::render_scaling;
use zax_platform::HostWindowSettings;
use zax_render::geometry::MAX_ANTI_ALIAS_SAMPLES;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

#[test]
fn req_pers_017_a_change_of_anti_alias_reaches_the_scene_at_once() {
    let mut harness = LoopHarness::create();
    assert_eq!(1, harness.host.scene().settings.anti_alias_samples());
    harness
        .host
        .update_settings(|model| model.set_anti_alias(1));
    assert_eq!(4, harness.host.scene().settings.anti_alias_samples());
    harness
        .host
        .update_settings(|model| model.set_anti_alias(2));
    assert_eq!(
        MAX_ANTI_ALIAS_SAMPLES,
        harness.host.scene().settings.anti_alias_samples()
    );
    harness
        .host
        .update_settings(|model| model.set_anti_alias(0));
    assert_eq!(1, harness.host.scene().settings.anti_alias_samples());
}

#[test]
fn req_pers_017_the_anti_alias_option_stays_in_its_domain() {
    let mut settings = SettingsModel::new();
    assert_eq!(0, settings.anti_alias());
    settings.set_anti_alias(2);
    assert_eq!(2, settings.anti_alias());
    settings.set_anti_alias(7);
    assert_eq!(2, settings.anti_alias());
}

#[rstest]
// index, expected width, expected height for a 1920 x 1080 view
#[case(0, 1920, 1080)]
#[case(1, 1280, 720)]
#[case(2, 960, 540)]
#[case(3, 640, 360)]
#[case(4, 480, 270)]
fn req_scn_063_the_render_texture_method_scales_the_scene_buffer(
    #[case] index: i32,
    #[case] expected_width: i32,
    #[case] expected_height: i32,
) {
    let mut model = SettingsModel::new();
    model.set_render_method(render_scaling::RENDER_TEXTURE_METHOD);
    model.set(SettingId::VideoResScale, SettingValue::Enum(index));
    let (width, height) =
        render_scaling::scene_size(1920, 1080, model.render_method(), model.res_scale_factor());
    assert_eq!(expected_width, width);
    assert_eq!(expected_height, height);
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
fn the_direct_method_ignores_the_resolution_scale(#[case] index: i32) {
    let mut model = SettingsModel::new();
    model.set_render_method(render_scaling::DIRECT_METHOD);
    model.set(SettingId::VideoResScale, SettingValue::Enum(index));
    assert!(!render_scaling::scale_applies(model.render_method()));
    assert_eq!(
        (1920, 1080),
        render_scaling::scene_size(1920, 1080, model.render_method(), model.res_scale_factor())
    );
}

#[test]
fn the_scaled_buffer_never_collapses_to_nothing() {
    assert_eq!(
        (1, 1),
        render_scaling::scene_size(2, 2, render_scaling::RENDER_TEXTURE_METHOD, 0.25)
    );
    assert_eq!(
        (1, 1),
        render_scaling::scene_size(0, -4, render_scaling::RENDER_TEXTURE_METHOD, 1.0)
    );
}

#[rstest]
#[case(false, false)]
#[case(true, true)]
fn req_pers_067_the_window_opens_in_the_persisted_state(
    #[case] persisted: bool,
    #[case] expected: bool,
) {
    assert_eq!(
        expected,
        should_open_fullscreen(&AppOptions::default(), persisted)
    );
}

#[rstest]
#[case(true, false)]
#[case(false, true)]
fn a_capture_run_stays_windowed(#[case] smoke: bool, #[case] shots: bool) {
    let options = AppOptions {
        smoke_test: smoke,
        ui_shot_directory: shots.then(|| "shots".to_owned()),
        ..AppOptions::default()
    };
    assert!(!should_open_fullscreen(&options, true));
}

#[test]
fn req_pers_067_the_window_settings_carry_the_full_screen_state() {
    let windowed = HostWindowSettings {
        vsync: 1,
        ..HostWindowSettings::default()
    };
    assert!(!windowed.fullscreen);
    let full = HostWindowSettings {
        fullscreen: true,
        ..windowed
    };
    assert!(full.fullscreen);
    assert_eq!(1280, full.width);
}
