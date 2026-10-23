//! The setting `ui.language` of AMD-035 §1: user scope, persisted, `en` by default, `en` or `fr` as
//! values, and anything else read as `en` without an error.

use rstest::rstest;
use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::{SettingEffect, SettingKind, SettingScope, SettingTheme};
use zax_settings::model::SettingsModel;
use zax_settings::persistence::{SettingsFile, SettingsLoadStatus};
use zax_settings::store::MemoryStorage;
use zax_settings::values::SettingValue;

/// AMD-035 §1: the setting is a persisted user preference, defaulting to English.
#[test]
fn the_setting_is_a_persisted_user_preference_defaulting_to_english() {
    let descriptor = catalogue::descriptor(SettingId::UiLanguage);
    assert_eq!(descriptor.kind(), SettingKind::Text);
    assert_eq!(descriptor.scope(), SettingScope::User);
    assert_eq!(descriptor.effect(), SettingEffect::Immediate);
    assert_eq!(descriptor.theme(), SettingTheme::Interface);
    assert!(descriptor.is_persisted());
    assert_eq!(descriptor.default_value().as_text(), "en");
    assert_eq!(descriptor.text_options(), Some(&["en", "fr"][..]));
}

/// AMD-035 §4: a fresh user folder is English.
#[test]
fn a_model_with_no_file_reads_english() {
    let model = SettingsModel::new();
    assert_eq!(model.get(SettingId::UiLanguage).as_text(), "en");
}

/// AMD-035 §1: an unknown value falls back to `en`, without an error.
#[rstest]
#[case("fr", "fr")]
#[case("FR", "fr")]
#[case("  fr  ", "fr")]
#[case("en", "en")]
#[case("", "en")]
#[case("de", "en")]
#[case("français", "en")]
fn an_unknown_value_reads_as_english(#[case] stored: &str, #[case] expected: &str) {
    let descriptor = catalogue::descriptor(SettingId::UiLanguage);
    assert_eq!(descriptor.coerce_text(stored), expected);
}

/// AMD-035 §1: the choice survives a restart.
#[test]
fn the_choice_is_written_to_the_settings_file_and_read_back() {
    let mut written = SettingsModel::new();
    written.set(SettingId::UiLanguage, "fr");
    let mut file = SettingsFile::new(MemoryStorage::new());
    file.save(&written).expect("the store accepts the write");
    let mut read = SettingsModel::new();
    let mut again = SettingsFile::new(file.storage().clone());
    let result = again.load(&mut read, time::OffsetDateTime::UNIX_EPOCH);
    assert_eq!(result.status, SettingsLoadStatus::Loaded);
    assert_eq!(read.get(SettingId::UiLanguage), SettingValue::from("fr"));
}

/// The environment default (the language of the operating system) applies only while the user
/// holds no value, and a choice equal to it is still written so that it keeps winning.
#[test]
fn an_environment_default_gives_way_to_the_users_choice() {
    let mut model = SettingsModel::new();
    model.set_environment_default(SettingId::UiLanguage, "fr");
    assert_eq!(model.get(SettingId::UiLanguage).as_text(), "fr");
    assert_eq!(model.user_preferences().count(), 0);

    model.set(SettingId::UiLanguage, "en");
    assert_eq!(model.get(SettingId::UiLanguage).as_text(), "en");
    assert_eq!(model.user_preferences().count(), 1);

    let mut file = SettingsFile::new(MemoryStorage::new());
    file.save(&model).expect("the store accepts the write");

    // A later launch on a French system: the saved English wins.
    let mut next = SettingsModel::new();
    next.set_environment_default(SettingId::UiLanguage, "fr");
    let mut again = SettingsFile::new(file.storage().clone());
    again.load(&mut next, time::OffsetDateTime::UNIX_EPOCH);
    assert_eq!(next.get(SettingId::UiLanguage).as_text(), "en");

    // With nothing saved the system language shows, and nothing is written for it.
    let mut fresh = SettingsModel::new();
    fresh.set_environment_default(SettingId::UiLanguage, "fr");
    let mut empty = SettingsFile::new(MemoryStorage::new());
    empty.save(&fresh).expect("the store accepts the write");
    let mut read = SettingsModel::new();
    SettingsFile::new(empty.storage().clone()).load(&mut read, time::OffsetDateTime::UNIX_EPOCH);
    assert!(read.user_preferences().next().is_none());
}
