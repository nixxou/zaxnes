//! The *Shaders* page of the console options (owner request): a row per slot that steps through
//! following, nothing and the presets of the catalogue, as the desktop's window sets them.

use zax_app::ui::handheld::options::{step_row, value_text, OptionRow, OptionsPage, ADVANCED_HUB};
use zax_app::ui::handheld::overlay::CameraPreset;
use zax_app::ui::shader_panel::{self, SlotPick, CATALOGUE};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::shader_slots::{resolve, ShaderSlot, FOLLOW, OFF, PRESET};

#[test]
fn the_page_hangs_on_the_advanced_hub_with_the_four_slots() {
    assert!(ADVANCED_HUB.contains(&OptionRow::Page(OptionsPage::Shaders)));
    assert_eq!(
        OptionsPage::Shaders.rows(),
        [
            OptionRow::Shader(ShaderSlot::TwoD),
            OptionRow::Shader(ShaderSlot::TwoDWide),
            OptionRow::Shader(ShaderSlot::ThreeD),
            OptionRow::Shader(ShaderSlot::ThreeDWide),
        ]
    );
}

#[test]
fn the_2d_row_steps_from_off_to_the_first_preset_and_back() {
    let mut settings = SettingsModel::new();
    let row = OptionRow::Shader(ShaderSlot::TwoD);
    assert_eq!(settings.get(SettingId::Shader2dMode).as_int(), OFF);
    step_row(row, &mut settings, 1);
    assert_eq!(settings.get(SettingId::Shader2dMode).as_int(), PRESET);
    assert_eq!(
        settings.get(SettingId::Shader2dPreset).as_text(),
        CATALOGUE[0].path
    );
    assert_eq!(
        value_text(row, &settings, CameraPreset::Orbit),
        shader_panel::preset_label(CATALOGUE[0].path)
    );
    // Every 3D and widescreen mode follows the 2D slot by default.
    assert_eq!(
        resolve(&settings, true, true).map(|choice| choice.preset),
        Some(CATALOGUE[0].path.to_owned())
    );
    step_row(row, &mut settings, -1);
    assert_eq!(settings.get(SettingId::Shader2dMode).as_int(), OFF);
    // The 2D slot has no slot to follow: back once more wraps to the last preset.
    step_row(row, &mut settings, -1);
    assert_eq!(
        shader_panel::slot_pick(&settings, ShaderSlot::TwoD),
        SlotPick::Preset(CATALOGUE[CATALOGUE.len() - 1].path.to_owned())
    );
}

#[test]
fn a_following_slot_starts_on_follow_and_can_be_turned_off() {
    let mut settings = SettingsModel::new();
    let row = OptionRow::Shader(ShaderSlot::ThreeDWide);
    assert_eq!(settings.get(SettingId::Shader3dWsMode).as_int(), FOLLOW);
    step_row(row, &mut settings, 1);
    assert_eq!(settings.get(SettingId::Shader3dWsMode).as_int(), OFF);
    step_row(row, &mut settings, 1);
    assert_eq!(settings.get(SettingId::Shader3dWsMode).as_int(), PRESET);
    step_row(row, &mut settings, -2);
    assert_eq!(settings.get(SettingId::Shader3dWsMode).as_int(), FOLLOW);
}

#[test]
fn a_preset_of_the_catalogue_brings_its_parameters_and_area() {
    let mut settings = SettingsModel::new();
    let screen = CATALOGUE
        .iter()
        .find(|preset| preset.path.contains("SCREEN-ONLY"))
        .expect("a Mega Bezel screen in the catalogue");
    shader_panel::set_slot_pick(
        &mut settings,
        ShaderSlot::TwoDWide,
        &SlotPick::Preset(screen.path.to_owned()),
    );
    assert_eq!(
        settings.get(SettingId::Shader2dWsArea).as_int(),
        screen.area
    );
    assert!(settings
        .get(SettingId::Shader2dWsParams)
        .as_text()
        .contains("HSM_ASPECT_RATIO_MODE=6"));
}
