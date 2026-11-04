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
    // The harness builds the host without the one-off mirror the application runs at start-up
    // (`publish_settings_to_scene`), so its scene begins at one sample whatever the setting's
    // default is; what this test measures is that a *change* reaches the scene at once. The
    // setting now starts at 1 (AMD-052 REQ-PERS-089), so the first change made here is to 0 —
    // writing the value a setting already holds is not a change and dispatches nothing.
    assert_eq!(1, harness.host.scene().settings.anti_alias_samples());
    harness
        .host
        .update_settings(|model| model.set_anti_alias(0));
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

/// The domain is 0..3 since AMD-060 §2 ("0..2 → 0..3"), and a value outside it is brought back
/// "comme aujourd'hui une valeur hors 0..2" — which for this setter is the clamp, and for a value
/// arriving through the settings model is the default (`Domain::coerce` on a choice domain whose
/// labels do not cover it).
#[test]
fn req_pers_017_the_anti_alias_option_stays_in_its_domain() {
    let mut settings = SettingsModel::new();
    // The default is 1 since AMD-052 REQ-PERS-089.
    assert_eq!(1, settings.anti_alias());
    settings.set_anti_alias(0);
    assert_eq!(0, settings.anti_alias());
    settings.set_anti_alias(2);
    assert_eq!(2, settings.anti_alias());
    settings.set_anti_alias(3);
    assert_eq!(3, settings.anti_alias());
    settings.set_anti_alias(7);
    assert_eq!(3, settings.anti_alias());
    settings.set_anti_alias(-4);
    assert_eq!(0, settings.anti_alias());
}

/// AMD-060 REQ-SCN-085's first clause where the renderer reads it: level 3 is **one** sample.
///
/// This is also AMD-060 §2's "en lecture, la valeur 3 se lit comme « aucun multi-échantillonnage »"
/// at the one place the setting becomes a sample count, and the reason the scene can carry the
/// level without the multisampled attachments of AMD-032 §2 ever being allocated.
#[test]
fn req_scn_085_the_fourth_level_asks_for_no_multisampling() {
    let mut harness = LoopHarness::create();
    harness
        .host
        .update_settings(|model| model.set_anti_alias(3));
    assert_eq!(1, harness.host.scene().settings.anti_alias_samples());
    assert_eq!(3, harness.host.settings().anti_alias());
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

/// AMD-032 §5: only the options that print and exit write to the console without a window. A
/// normal launch must not, or the window shares the shell it was started from.
#[rstest]
#[case(&["--help"])]
#[case(&["-h"])]
#[case(&["--list-profiles"])]
#[case(&["--import-3dsen", "C:/3dSen"])]
fn only_the_options_that_print_and_exit_attach_to_the_parent_console(#[case] arguments: &[&str]) {
    assert!(AppOptions::parse(arguments)
        .expect("the arguments parse")
        .prints_and_exits());
}

/// Every launch that opens a window leaves the terminal alone.
#[rstest]
#[case(&[])]
#[case(&["--no-audio"])]
#[case(&["--smoke-test"])]
#[case(&["--vsync", "0"])]
fn a_launch_that_opens_a_window_does_not_attach(#[case] arguments: &[&str]) {
    assert!(!AppOptions::parse(arguments)
        .expect("the arguments parse")
        .prints_and_exits());
}
