//! The settings the `Setting` members of 3dSen 2.0 stand for: the rounding of AMD-040 §3, the
//! style identifiers of AMD-055 REQ-SCN-078 and the defaults of the new keys
//! (AMD-040 REQ-LUA-114, AMD-043 REQ-SCN-066, AMD-059 REQ-LUA-116).

use rstest::rstest;
use zax_settings::catalogue::{self, SettingId};
use zax_settings::model::SettingsModel;
use zax_settings::styles;
use zax_settings::values::SettingValue;

/// AMD-040 §3: a value is bounded, then put on the nearest rung of the step, ties going to the
/// even rung. The four values the amendment names, plus the bounds.
#[rstest]
#[case(500.0, 180.0)]
#[case(-500.0, -180.0)]
#[case(40.0, 30.0)]
#[case(45.0, 60.0)]
#[case(15.0, 0.0)]
#[case(-15.0, 0.0)]
#[case(-45.0, -60.0)]
#[case(0.0, 0.0)]
fn a_curvature_value_is_rounded_as_amd_040_requires(#[case] written: f64, #[case] expected: f64) {
    let domain = catalogue::descriptor(SettingId::SceneBendLr).domain();
    assert!((domain.snap(written) - expected).abs() < 1e-9, "{written}");
}

/// The same rounding reached through the model, which is what a write from the options window
/// does (AMD-043 REQ-SCN-066: writing 45 gives 60).
#[test]
fn a_model_write_of_a_curvature_value_is_rounded_too() {
    let mut model = SettingsModel::new();
    model.set(SettingId::SceneBendLr, SettingValue::Int(45));
    assert_eq!(60, model.get(SettingId::SceneBendLr).as_int());
    model.set(SettingId::SceneBendLr, SettingValue::Int(500));
    assert_eq!(180, model.get(SettingId::SceneBendLr).as_int());
}

/// AMD-055 REQ-SCN-079, the normalisation of the graded parameters: 9 gives 5, 0 gives 1.
#[rstest]
#[case(SettingId::RenderStyleAoStrength, 9, 5)]
#[case(SettingId::RenderStyleAoStrength, 0, 1)]
#[case(SettingId::RenderStyleAoStrength, 3, 3)]
#[case(SettingId::RenderStyleVoxelGrid, 4, 3)]
#[case(SettingId::RenderStyleAo, 1, 1)]
#[case(SettingId::RenderStyleAo, 7, 2)]
fn a_style_parameter_is_bounded_by_its_ladder(
    #[case] id: SettingId,
    #[case] written: i32,
    #[case] expected: i32,
) {
    let mut model = SettingsModel::new();
    model.set(id, SettingValue::Int(written));
    assert_eq!(expected, model.get(id).as_int());
}

/// AMD-055 REQ-SCN-078: 1 is written for Enhanced and read back as 4; anything else gives 0,
/// wherever it comes from.
#[rstest]
#[case(4, 4)]
#[case(1, 4)]
#[case(0, 0)]
#[case(2, 0)]
#[case(7, 0)]
#[case(-1, 0)]
fn the_visual_style_keeps_four_and_takes_one_for_it(#[case] written: i32, #[case] expected: i32) {
    let mut model = SettingsModel::new();
    model.set(SettingId::RenderVisualStyle, SettingValue::Int(written));
    assert_eq!(expected, model.get(SettingId::RenderVisualStyle).as_int());
}

/// REQ-LUA-114: every parameter identifier of the Enhanced style names a setting, and the
/// identifiers of the other styles name none.
#[test]
fn every_parameter_identifier_of_enhanced_names_a_setting() {
    for (name, id) in styles::parameters_of(styles::ENHANCED_STYLE) {
        assert_eq!(
            Some(*id),
            styles::parameter(styles::ENHANCED_STYLE, name),
            "{name}"
        );
        assert_eq!(
            Some(*id),
            styles::parameter(styles::ENHANCED_STYLE_ALIAS, name),
            "{name} through the alias"
        );
    }
    assert_eq!(None, styles::parameter(styles::CLASSIC_STYLE, "spl.ao"));
    assert_eq!(None, styles::parameter(7, "spl.ao"));
}

/// The defaults of the new keys, as the amendments give them.
#[rstest]
#[case(SettingId::VideoMaxQueuedFrames, 2)]
#[case(SettingId::VideoMsaa20, 2)]
#[case(SettingId::EmuRunAhead, 1)]
#[case(SettingId::UiLibraryView, 0)]
#[case(SettingId::VrRefreshRate, 90)]
#[case(SettingId::RenderVisualStyle, 0)]
#[case(SettingId::SceneBendLr, 0)]
#[case(SettingId::SceneArcCenter, 0)]
#[case(SettingId::RenderStyleAoStrength, 3)]
#[case(SettingId::RenderStyleAoQuality, 1)]
#[case(SettingId::RenderStyleSsrQuality, 2)]
fn the_new_keys_start_at_the_default_of_their_amendment(
    #[case] id: SettingId,
    #[case] expected: i32,
) {
    assert_eq!(expected, SettingsModel::new().get(id).as_int());
}

/// AMD-059 REQ-LUA-116: `ui.library_view` has no domain at all — any value is kept.
#[test]
fn the_library_view_keeps_whatever_is_written() {
    let mut model = SettingsModel::new();
    model.set(SettingId::UiLibraryView, SettingValue::Int(7));
    assert_eq!(7, model.get(SettingId::UiLibraryView).as_int());
    model.set(SettingId::UiLibraryView, SettingValue::Int(-3));
    assert_eq!(-3, model.get(SettingId::UiLibraryView).as_int());
}

/// REQ-PERS-081: each new setting publishes the finite ordered list of its values, except the
/// library view, which has no domain.
#[test]
fn the_new_settings_publish_their_values() {
    for id in [
        SettingId::VideoMaxQueuedFrames,
        SettingId::VideoMsaa20,
        SettingId::EmuRunAhead,
        SettingId::VrRefreshRate,
        SettingId::RenderVisualStyle,
        SettingId::SceneBendLr,
        SettingId::RenderStyleAo,
    ] {
        let descriptor = catalogue::descriptor(id);
        assert!(
            descriptor.domain().options().is_some_and(|o| !o.is_empty()),
            "{}",
            descriptor.key()
        );
    }
    assert!(catalogue::descriptor(SettingId::UiLibraryView)
        .domain()
        .options()
        .is_none());
}
