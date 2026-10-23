//! Value validation (REQ-PERS-003) against the vectors of SPEC-09 §5.1 and §5.2.

#[path = "common/close.rs"]
mod close;
#[path = "common/exact.rs"]
mod exact;

use close::assert_close;
use exact::assert_exact;
use rstest::rstest;
use zax_settings::catalogue::{Layer, SettingId};
use zax_settings::model::SettingsModel;
use zax_settings::values::{SettingValue, Vector3};

/// REQ-PERS-003 rule 1 and §5.1: a render rate outside {60, 90, 120} becomes 90.
#[rstest]
#[case(60, 60)]
#[case(90, 90)]
#[case(120, 120)]
#[case(75, 90)]
#[case(0, 90)]
fn render_rate_outside_the_three_values_becomes_ninety(
    #[case] written: i32,
    #[case] expected: i32,
) {
    let mut model = SettingsModel::new();
    model.set(SettingId::VideoFrameRate, SettingValue::Int(written));
    assert_eq!(model.get(SettingId::VideoFrameRate).as_int(), expected);
}

/// REQ-PERS-003 rule 3 and §5.1: the speed index is bounded to [−8, 8].
#[rstest]
#[case(-9.0, -8.0)]
#[case(3.4, 3.4)]
#[case(8.0, 8.0)]
#[case(12.0, 8.0)]
fn game_speed_is_bounded(#[case] written: f32, #[case] expected: f32) {
    let mut model = SettingsModel::new();
    model.set(SettingId::EmuGameSpeed, written);
    assert_close(model.get(SettingId::EmuGameSpeed).as_float(), expected, 4);
}

/// REQ-PERS-003 rule 4 and §5.1: a negative sprite tolerance becomes its absolute value.
#[test]
fn sprite_tolerance_drops_its_sign() {
    let mut model = SettingsModel::new();
    model.set(SettingId::ConvSpriteTolerance, SettingValue::Int(-3));
    assert_eq!(model.get(SettingId::ConvSpriteTolerance).as_int(), 3);
}

/// §5.1: a skybox outside its four values falls back to the default.
#[test]
fn an_unknown_skybox_falls_back_to_the_default() {
    let mut model = SettingsModel::new();
    model.set(SettingId::SceneSkybox, SettingValue::Int(7));
    assert_eq!(model.get(SettingId::SceneSkybox).as_int(), -1);
}

/// §5.1: a resolution scale index outside 0 to 4 falls back to the default.
#[test]
fn an_unknown_resolution_scale_falls_back_to_the_default() {
    let mut model = SettingsModel::new();
    model.set(SettingId::VideoResScale, SettingValue::Int(5));
    assert_eq!(model.get(SettingId::VideoResScale).as_int(), 0);
}

/// REQ-PERS-003 rule 2 and §5.1: the core index is the remainder of the division by the number of
/// cores, which is 1 in v1, so the effective value is always 0.
#[test]
fn the_core_index_is_taken_modulo_the_number_of_cores() {
    let mut single = SettingsModel::new();
    single.set(SettingId::EmuCore, SettingValue::Int(3));
    assert_eq!(single.get(SettingId::EmuCore).as_int(), 0);
    let mut dual = SettingsModel::new();
    dual.set_core_count(2);
    dual.set(SettingId::EmuCore, SettingValue::Int(3));
    assert_eq!(dual.get(SettingId::EmuCore).as_int(), 1);
}

/// §4.1.5: a clipping margin outside its interval is clamped.
#[rstest]
#[case(SettingId::ClipLeft, -4.0, 0.0)]
#[case(SettingId::ClipLeft, 300.0, 256.0)]
#[case(SettingId::ClipBottom, 300.0, 248.0)]
#[case(SettingId::ClipTop, 300.0, 248.0)]
#[case(SettingId::ClipRight, 300.0, 256.0)]
fn a_clipping_margin_is_clamped(
    #[case] id: SettingId,
    #[case] written: f32,
    #[case] expected: f32,
) {
    let mut model = SettingsModel::new();
    model.set(id, written);
    assert_exact(model.get(id).as_float(), expected);
}

/// §4.1.2: a script's field of view is bounded to [10, 150].
#[rstest]
#[case(5.0, 10.0)]
#[case(80.0, 80.0)]
#[case(400.0, 150.0)]
fn the_field_of_view_is_bounded(#[case] written: f32, #[case] expected: f32) {
    let mut model = SettingsModel::new();
    model.set(SettingId::CameraFov, written);
    assert_exact(model.get(SettingId::CameraFov).as_float(), expected);
}

/// §4.1.9: the volume is bounded to [0, 1].
#[rstest]
#[case(-1.0, 0.0)]
#[case(0.37, 0.37)]
#[case(4.0, 1.0)]
fn the_volume_is_bounded(#[case] written: f32, #[case] expected: f32) {
    let mut model = SettingsModel::new();
    model.set(SettingId::AudioVolume, written);
    assert_close(model.get(SettingId::AudioVolume).as_float(), expected, 4);
}

/// §4.1.2: the smoothing time's domain is "> 0" with no repair rule, so a value outside it takes the
/// default (REQ-PERS-003).
#[rstest]
#[case(0.7, 0.7)]
#[case(0.0, 0.4)]
#[case(-2.0, 0.4)]
fn a_non_positive_smoothing_time_takes_the_default(#[case] written: f32, #[case] expected: f32) {
    let mut model = SettingsModel::new();
    model.set(SettingId::CameraSmoothTime, written);
    assert_close(
        model.get(SettingId::CameraSmoothTime).as_float(),
        expected,
        4,
    );
}

/// §4.1.3: the light direction's domain is "non nul".
#[test]
fn a_zero_light_direction_takes_the_default() {
    let mut model = SettingsModel::new();
    model.set(SettingId::LightDirection, Vector3::new(0.0, 0.0, 0.0));
    assert_eq!(
        model.get(SettingId::LightDirection).as_vector3(),
        Vector3::new(1.0, -1.0, 1.0)
    );
    model.set(SettingId::LightDirection, Vector3::new(0.0, -1.0, 1.0));
    assert_eq!(
        model.get(SettingId::LightDirection).as_vector3(),
        Vector3::new(0.0, -1.0, 1.0)
    );
}

/// SPEC-03 REQ-PROF-041 acceptance criterion: a light intensity of 2 survives, so the domain is not the
/// slider's 0.5 to 1.5.
#[test]
fn the_light_intensity_is_not_bounded_to_the_slider_range() {
    let mut model = SettingsModel::new();
    model.set(SettingId::LightIntensity, 2.0_f32);
    assert_exact(model.get(SettingId::LightIntensity).as_float(), 2.0);
}

/// REQ-PERS-004: a setting accepts the number of its enumeration.
#[test]
fn an_enumerated_setting_accepts_its_number() {
    let mut model = SettingsModel::new();
    // Setting.Layer[2].Aligned = 2 gives layer 2 the BACK alignment.
    model.set(SettingId::LayerAligned(Layer::ALL[2]), SettingValue::Int(2));
    assert_eq!(
        model.get(SettingId::LayerAligned(Layer::ALL[2])).as_int(),
        2
    );
    assert_exact(
        model.get(SettingId::LayerCoef(Layer::ALL[2])).as_float(),
        -1.0,
    );
}

/// §5.2: the UI slider positions map to the documented values.
#[rstest]
#[case(-5.0, 45.0)]
#[case(0.0, 70.0)]
#[case(5.0, 95.0)]
fn the_field_of_view_slider_positions_map_to_their_values(
    #[case] position: f32,
    #[case] expected: f32,
) {
    let mut model = SettingsModel::new();
    model.set(SettingId::CameraFov, 70.0 + 5.0 * position);
    assert_exact(model.get(SettingId::CameraFov).as_float(), expected);
}

/// §5.2: the light slider runs 0 to 100 for 0.5 to 1.5.
#[rstest]
#[case(0.0, 0.5)]
#[case(50.0, 1.0)]
#[case(100.0, 1.5)]
fn the_light_slider_positions_map_to_their_values(#[case] position: f32, #[case] expected: f32) {
    let mut model = SettingsModel::new();
    model.set(SettingId::LightIntensity, 0.5 + position / 100.0);
    assert_close(model.get(SettingId::LightIntensity).as_float(), expected, 4);
}
