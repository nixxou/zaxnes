//! The four slots of the slang shaders: their defaults, the fallbacks and the parameter text.

use zax_settings::catalogue::{self, SettingId};
use zax_settings::model::SettingsModel;
use zax_settings::shader_slots::{
    format_parameters, parse_parameters, resolve, with_parameter, ShaderSlot, FOLLOW, OFF, PRESET,
};
use zax_settings::values::SettingValue;

fn set_preset(model: &mut SettingsModel, slot: ShaderSlot, preset: &str) {
    let ids = slot.ids();
    model.set(ids.mode, SettingValue::Enum(PRESET));
    model.set(ids.preset, preset);
}

#[test]
fn by_default_no_mode_has_a_shader() {
    let model = SettingsModel::new();
    for (three_d, wide) in [(false, false), (false, true), (true, false), (true, true)] {
        assert_eq!(resolve(&model, three_d, wide), None);
    }
    assert_eq!(model.get(SettingId::Shader2dMode).as_int(), OFF);
    assert_eq!(model.get(SettingId::Shader3dWsMode).as_int(), FOLLOW);
    assert_eq!(model.get(SettingId::Shader2dLines).as_int(), 240);
    assert_eq!(model.get(SettingId::Shader3dLines).as_int(), 0);
}

#[test]
fn a_2d_preset_covers_every_mode_that_follows() {
    let mut model = SettingsModel::new();
    set_preset(&mut model, ShaderSlot::TwoD, "crt/crt-easymode.slangp");
    for (three_d, wide) in [(false, false), (false, true), (true, false), (true, true)] {
        let choice = resolve(&model, three_d, wide).unwrap();
        assert_eq!(choice.slot, ShaderSlot::TwoD);
        assert_eq!(choice.preset, "crt/crt-easymode.slangp");
        // The lines stay the mode's own: the console's in 2D, the window's in 3D.
        assert_eq!(choice.lines, if three_d { 0 } else { 240 });
    }
}

#[test]
fn a_slot_of_its_own_wins_and_the_wide_3d_follows_the_3d() {
    let mut model = SettingsModel::new();
    set_preset(&mut model, ShaderSlot::TwoD, "a.slangp");
    set_preset(&mut model, ShaderSlot::ThreeD, "b.slangp");
    model.set(SettingId::Shader3dLines, SettingValue::Int(480));
    assert_eq!(resolve(&model, false, true).unwrap().preset, "a.slangp");
    let wide_3d = resolve(&model, true, true).unwrap();
    assert_eq!(
        (wide_3d.slot, wide_3d.preset.as_str(), wide_3d.lines),
        (ShaderSlot::ThreeD, "b.slangp", 0)
    );
    assert_eq!(resolve(&model, true, false).unwrap().lines, 480);
}

#[test]
fn a_slot_turned_off_stops_the_fallback_and_an_empty_preset_is_none() {
    let mut model = SettingsModel::new();
    set_preset(&mut model, ShaderSlot::TwoD, "a.slangp");
    model.set(SettingId::Shader2dWsMode, SettingValue::Enum(OFF));
    assert_eq!(resolve(&model, false, true), None);
    assert!(resolve(&model, true, true).is_some());
    model.set(SettingId::Shader3dMode, SettingValue::Enum(PRESET));
    assert_eq!(resolve(&model, true, false), None);
    assert_eq!(resolve(&model, true, true), None);
}

#[test]
fn the_2d_slot_cannot_follow() {
    let mut model = SettingsModel::new();
    model.set(SettingId::Shader2dMode, SettingValue::Enum(FOLLOW));
    assert_ne!(model.get(SettingId::Shader2dMode).as_int(), FOLLOW);
    assert_eq!(resolve(&model, false, false), None);
}

#[test]
fn the_parameters_come_back_with_the_slot() {
    let mut model = SettingsModel::new();
    set_preset(&mut model, ShaderSlot::TwoD, "a.slangp");
    model.set(SettingId::Shader2dParams, "MASK_STRENGTH=0.3;SCANLINES=1");
    let choice = resolve(&model, true, false).unwrap();
    assert_eq!(
        choice.parameters,
        [
            ("MASK_STRENGTH".to_owned(), 0.3),
            ("SCANLINES".to_owned(), 1.0)
        ]
    );
}

#[test]
fn parameter_text_skips_what_does_not_read_and_keeps_the_last_value() {
    let parsed = parse_parameters(" A = 1.5 ;bad;B=x;C-D=2;E=inf;;A=2;F_1=-0.25");
    assert_eq!(parsed, [("A".to_owned(), 2.0), ("F_1".to_owned(), -0.25)]);
    assert_eq!(format_parameters(&parsed), "A=2;F_1=-0.25");
    assert_eq!(parse_parameters(""), []);
}

#[test]
fn a_parameter_is_set_changed_and_reset() {
    let text = with_parameter("", "GAMMA", Some(2.2));
    assert_eq!(text, "GAMMA=2.2");
    let text = with_parameter(&text, "MASK", Some(0.5));
    let text = with_parameter(&text, "GAMMA", Some(2.4));
    assert_eq!(text, "GAMMA=2.4;MASK=0.5");
    assert_eq!(with_parameter(&text, "GAMMA", None), "MASK=0.5");
    assert_eq!(with_parameter(&text, "UNKNOWN", None), text);
}

#[test]
fn the_twenty_four_keys_are_in_the_catalogue() {
    let keys: Vec<String> = catalogue::descriptors()
        .map(|descriptor| descriptor.key().into_owned())
        .filter(|key| key.starts_with("shader."))
        .collect();
    assert_eq!(keys.len(), 24);
    for slot in ["2d", "2d_ws", "3d", "3d_ws"] {
        for part in ["mode", "preset", "lines", "params", "area", "frame"] {
            assert!(
                keys.contains(&format!("shader.{slot}.{part}")),
                "{slot}.{part}"
            );
        }
    }
}

#[test]
fn a_bezel_preset_fills_the_window_unless_told_otherwise() {
    use zax_settings::shader_slots::{fills_window, AREA_AUTO, AREA_PICTURE, AREA_WINDOW};
    let bezel = "G:/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__3__STD.slangp";
    let crt = "G:/slang-shaders/crt/crt-easymode.slangp";
    assert!(fills_window(AREA_AUTO, bezel));
    assert!(!fills_window(AREA_AUTO, crt));
    assert!(!fills_window(AREA_PICTURE, bezel));
    assert!(fills_window(AREA_WINDOW, crt));
    assert!(fills_window(
        AREA_AUTO,
        "C:\\shaders\\bezel\\koko-aio\\x.slangp"
    ));
}

/// The framing of the 3D scene: Auto frames the presets that fill the window on the picture.
#[test]
fn framing_follows_the_presets_that_fill_the_window() {
    use zax_settings::shader_slots::{frames_picture, FRAME_AUTO, FRAME_PICTURE, FRAME_WINDOW};
    assert!(frames_picture(FRAME_AUTO, true));
    assert!(!frames_picture(FRAME_AUTO, false));
    assert!(frames_picture(FRAME_PICTURE, false));
    assert!(!frames_picture(FRAME_WINDOW, true));
    let mut model = SettingsModel::new();
    set_preset(
        &mut model,
        ShaderSlot::TwoD,
        "bezel/Mega_Bezel/Presets/MBZ__3__STD.slangp",
    );
    let choice = resolve(&model, true, false).expect("3D follows 2D");
    assert!(choice.fills_window && choice.frames_picture);
    model.set(SettingId::Shader3dFrame, SettingValue::Enum(FRAME_WINDOW));
    assert!(!resolve(&model, true, false).expect("3D").frames_picture);
}
