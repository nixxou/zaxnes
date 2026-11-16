//! Console models of the menu bar (lot FIDPLUS-1, G1).

use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut, TvRegionSetting};
use zax_app::persistence::console_model::ConsoleModel;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

#[test]
fn fidplus_g1_a_model_sets_its_values_and_turns_detection_off() {
    let mut model = SettingsModel::new();
    model.set_auto_tv_mode(true);

    ConsoleModel::Ntsc2C02BeforeG.apply(&mut model);
    assert!(!model.auto_tv_mode());
    assert_eq!(TvRegionSetting::Ntsc, model.tv_mode());
    assert_eq!(1, model.get(SettingId::EmuPpuRevision).as_int());
    assert!(!model.get(SettingId::EmuOamaddrBug).as_bool());
    assert!(ConsoleModel::Ntsc2C02BeforeG.is_selected(&model));
    assert!(!ConsoleModel::Ntsc2C02G.is_selected(&model));

    ConsoleModel::Ntsc2C02G.apply(&mut model);
    assert_eq!(0, model.get(SettingId::EmuPpuRevision).as_int());
    assert!(model.get(SettingId::EmuOamaddrBug).as_bool());
    assert!(ConsoleModel::Ntsc2C02G.is_selected(&model));
}

#[test]
fn fidplus_g1_clones_set_their_region_only() {
    let mut model = SettingsModel::new();
    ConsoleModel::Ntsc2C02BeforeG.apply(&mut model);

    ConsoleModel::PalN.apply(&mut model);
    assert_eq!(TvRegionSetting::PalN, model.tv_mode());
    // The revision options are left as they were.
    assert_eq!(1, model.get(SettingId::EmuPpuRevision).as_int());
    assert!(!model.get(SettingId::EmuOamaddrBug).as_bool());
    assert!(ConsoleModel::PalN.is_selected(&model));
}

#[test]
fn fidplus_g1_changing_an_option_by_hand_unticks_the_model() {
    let mut model = SettingsModel::new();
    ConsoleModel::Pal2C07.apply(&mut model);
    assert!(ConsoleModel::Pal2C07.is_selected(&model));

    model.set(SettingId::EmuOamaddrBug, SettingValue::Bool(true));
    assert!(ConsoleModel::ALL
        .iter()
        .all(|console| !console.is_selected(&model)));

    model.set_auto_tv_mode(true);
    ConsoleModel::Pal2C07.apply(&mut model);
    assert!(ConsoleModel::Pal2C07.is_selected(&model));
    model.set_auto_tv_mode(true);
    assert!(!ConsoleModel::Pal2C07.is_selected(&model));
}

#[test]
fn fidplus_g1_numbers_round_trip() {
    for console in ConsoleModel::ALL {
        assert_eq!(Some(console), ConsoleModel::from_raw(console as i32));
    }
    assert_eq!(None, ConsoleModel::from_raw(6));
    assert_eq!(None, ConsoleModel::from_raw(-1));
}
