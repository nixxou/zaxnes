//! The settings file (REQ-PERS-015, REQ-PERS-020 to REQ-PERS-024, REQ-PERS-073).

#[path = "common/close.rs"]
mod close;
#[path = "common/exact.rs"]
mod exact;

use close::assert_close;
use exact::assert_exact;
use rstest::rstest;
use serde_json::{Map, Value};
use time::macros::datetime;
use time::OffsetDateTime;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::persistence::{SettingsFile, SettingsLoadStatus, SettingsMigration};
use zax_settings::store::{MemoryStorage, SettingsStorage};
use zax_settings::values::{Color, Rect, SettingValue};

const NAME: &str = SettingsFile::<MemoryStorage>::DEFAULT_NAME;

fn now() -> OffsetDateTime {
    datetime!(2026-10-21 14:30:05 UTC)
}

/// The `values` object of the settings file as `storage` holds it.
fn stored_values(storage: &MemoryStorage) -> Option<Map<String, Value>> {
    let root: Value = serde_json::from_str(&storage.text(NAME)?).ok()?;
    root["values"].as_object().cloned()
}

/// A file with `text` under the settings name.
fn storage_with(text: &str) -> MemoryStorage {
    let mut storage = MemoryStorage::new();
    storage.put(NAME, text);
    storage
}

/// REQ-PERS-020: the user preferences survive a round trip.
#[test]
fn user_preferences_are_written_and_read_back() {
    let mut written = SettingsModel::new();
    written.set(SettingId::CameraFov, 85.0_f32);
    written.set(SettingId::AudioVolume, 0.25_f32);
    written.set(SettingId::SceneSkybox, SettingValue::Int(2));
    written.set(SettingId::UiFullscreen, true);
    written.set(SettingId::UiLastBrowseDir, r"D:\roms");
    written.set(
        SettingId::SceneCustomBgColor,
        Color::new(1.0, 0.0, 0.0, 1.0),
    );
    let mut file = SettingsFile::new(MemoryStorage::new());
    file.save(&written).expect("the store accepts the write");
    let mut read = SettingsModel::new();
    let mut again = SettingsFile::new(file.storage().clone());
    let result = again.load(&mut read, now());
    assert_eq!(result.status, SettingsLoadStatus::Loaded);
    assert_exact(read.get(SettingId::CameraFov).as_float(), 85.0);
    assert_close(read.get(SettingId::AudioVolume).as_float(), 0.25, 4);
    assert_eq!(read.get(SettingId::SceneSkybox).as_int(), 2);
    assert!(read.get(SettingId::UiFullscreen).as_bool());
    assert_eq!(read.get(SettingId::UiLastBrowseDir).as_text(), r"D:\roms");
    assert_eq!(
        read.get(SettingId::SceneCustomBgColor).as_color(),
        Color::new(1.0, 0.0, 0.0, 1.0)
    );
}

/// REQ-PERS-002: an absent file leaves every default in place.
#[test]
fn an_absent_file_leaves_the_defaults() {
    let mut model = SettingsModel::new();
    let mut file = SettingsFile::new(MemoryStorage::new());
    let result = file.load(&mut model, now());
    assert_eq!(result.status, SettingsLoadStatus::Absent);
    assert_eq!(result.message, None);
    assert_exact(model.get(SettingId::CameraFov).as_float(), 70.0);
}

/// REQ-PERS-020: the P and S settings are not needed at start-up, and their presence in the file is
/// ignored.
#[test]
fn profile_and_session_settings_in_the_file_are_ignored() {
    let storage = storage_with(
        r#"{
  "schemaVersion": 1,
  "values": { "conv.empty_tile": true, "camera.smooth_time": 9, "camera.fov": 55 }
}"#,
    );
    let mut model = SettingsModel::new();
    let mut file = SettingsFile::new(storage);
    assert_eq!(
        file.load(&mut model, now()).status,
        SettingsLoadStatus::Loaded
    );
    assert!(!model.get(SettingId::ConvEmptyTile).as_bool());
    assert_close(model.get(SettingId::CameraSmoothTime).as_float(), 0.4, 4);
    assert_exact(model.get(SettingId::CameraFov).as_float(), 55.0);
}

/// REQ-PERS-022: an unreadable file takes the model back to the defaults, is set aside and asks for the
/// notification, word for word.
#[test]
fn an_unreadable_file_is_set_aside_and_notified() {
    let mut file = SettingsFile::new(storage_with("{{{"));
    let mut model = SettingsModel::new();
    let result = file.load(&mut model, now());
    assert_eq!(result.status, SettingsLoadStatus::Unreadable);
    assert_eq!(
        result.message,
        Some("Settings file was unreadable and has been reset.")
    );
    assert!(!file.storage().exists(NAME));
    assert!(file
        .storage()
        .names()
        .any(|name| name == "settings.json.corrupt-20261021-143005"));
    assert_exact(model.get(SettingId::CameraFov).as_float(), 70.0);
}

/// REQ-PERS-022: a root that is not an object counts as unreadable.
#[test]
fn a_file_whose_root_is_not_an_object_counts_as_unreadable() {
    let mut file = SettingsFile::new(storage_with("[1, 2, 3]"));
    let result = file.load(&mut SettingsModel::new(), now());
    assert_eq!(result.status, SettingsLoadStatus::Unreadable);
}

/// REQ-PERS-022: a single value of the wrong type is replaced on its own, silently, and the other
/// values are kept.
#[test]
fn one_invalid_value_is_replaced_silently() {
    let mut file = SettingsFile::new(storage_with(
        r#"{ "schemaVersion": 1, "values": { "camera.fov": "abc", "audio.volume": 0.5 } }"#,
    ));
    let mut model = SettingsModel::new();
    let result = file.load(&mut model, now());
    assert_eq!(result.status, SettingsLoadStatus::Loaded);
    assert_eq!(result.message, None);
    assert_exact(model.get(SettingId::CameraFov).as_float(), 70.0);
    assert_close(model.get(SettingId::AudioVolume).as_float(), 0.5, 4);
    assert!(file.storage().exists(NAME));
}

/// REQ-PERS-003: a value in the file that is outside its domain is brought back in, exactly as a change
/// from the UI would be.
#[test]
fn a_value_outside_its_domain_in_the_file_is_brought_back_in() {
    let mut file = SettingsFile::new(storage_with(
        r#"{ "schemaVersion": 1, "values": { "video.frame_rate": 75, "emu.game_speed": 12 } }"#,
    ));
    let mut model = SettingsModel::new();
    file.load(&mut model, now());
    assert_eq!(model.get(SettingId::VideoFrameRate).as_int(), 90);
    assert_exact(model.get(SettingId::EmuGameSpeed).as_float(), 8.0);
}

/// REQ-PERS-023: an unknown key survives a rewrite.
#[test]
fn an_unknown_key_survives_a_rewrite() {
    let mut file = SettingsFile::new(storage_with(
        r#"{ "schemaVersion": 1, "values": { "future.x": 3, "audio.volume": 0.5 } }"#,
    ));
    let mut model = SettingsModel::new();
    file.load(&mut model, now());
    model.set(SettingId::AudioVolume, 0.8_f32);
    file.save(&model).expect("the store accepts the write");
    let values = stored_values(file.storage()).expect("the values object");
    assert_eq!(values["future.x"].as_i64(), Some(3));
    let volume = values["audio.volume"]
        .as_f64()
        .expect("the volume is a number");
    assert!((volume - 0.8).abs() < 1e-4);
}

struct RenameFovMigration;

impl SettingsMigration for RenameFovMigration {
    fn source_version(&self) -> i64 {
        0
    }

    fn apply(&self, values: &mut Map<String, Value>) {
        if let Some(node) = values.remove("legacy.fov") {
            values.insert("camera.fov".to_owned(), node);
        }
    }
}

/// REQ-PERS-024: a file from an earlier version is migrated, a `.vN.bak` copy is kept and the file is
/// written again at the current version.
#[test]
fn an_older_file_is_migrated_after_a_backup_copy() {
    let original = r#"{ "schemaVersion": 0, "values": { "legacy.fov": 85 } }"#;
    let mut file = SettingsFile::new(storage_with(original))
        .with_migrations(vec![
            Box::new(RenameFovMigration) as Box<dyn SettingsMigration>
        ]);
    let mut model = SettingsModel::new();
    let result = file.load(&mut model, now());
    assert_eq!(result.status, SettingsLoadStatus::Migrated);
    assert_eq!(result.version, 0);
    assert_eq!(
        file.storage().text("settings.json.v0.bak").as_deref(),
        Some(original)
    );
    assert_exact(model.get(SettingId::CameraFov).as_float(), 85.0);
    file.save(&model).expect("the store accepts the write");
    let root: Value =
        serde_json::from_str(&file.storage().text(NAME).expect("written")).expect("JSON");
    assert_eq!(
        root["schemaVersion"].as_i64(),
        Some(SettingsFile::<MemoryStorage>::CURRENT_SCHEMA_VERSION)
    );
    assert_eq!(root["values"]["camera.fov"].as_i64(), Some(85));
}

/// REQ-PERS-024: a file from a newer version is read at best and left untouched for the rest of the
/// session, with its own notification.
#[test]
fn a_newer_file_is_read_at_best_and_never_rewritten() {
    let mut file = SettingsFile::new(storage_with(
        r#"{ "schemaVersion": 99, "values": { "camera.fov": 45, "unheard.of": true } }"#,
    ));
    let mut model = SettingsModel::new();
    let result = file.load(&mut model, now());
    assert_eq!(result.status, SettingsLoadStatus::NewerVersion);
    assert_eq!(
        result.message,
        Some("Settings were created by a newer version.")
    );
    assert!(result.is_read_only_session());
    assert_exact(model.get(SettingId::CameraFov).as_float(), 45.0);
    model.set(SettingId::CameraFov, 70.0_f32);
    assert!(file.save(&model).is_err());
    assert!(file
        .storage()
        .text(NAME)
        .is_some_and(|text| text.contains("\"schemaVersion\": 99")));
}

/// REQ-PERS-073: a store that refuses writes does not stop anything.
#[test]
fn a_read_only_store_refuses_the_write_without_throwing() {
    let mut model = SettingsModel::new();
    model.set(SettingId::CameraFov, 50.0_f32);
    let mut file = SettingsFile::new(MemoryStorage::read_only());
    assert!(file.save(&model).is_err());
    assert_eq!(file.storage().names().count(), 0);
}

/// REQ-PERS-015: `Setting:Load()` re-reads the file and applies only its U settings, so a session value a
/// script put on top is dropped in favour of the file's preference.
#[test]
fn reloading_applies_only_the_user_settings_of_the_file() {
    let mut file = SettingsFile::new(storage_with(
        r#"{ "schemaVersion": 1, "values": { "audio.volume": 0.3 } }"#,
    ));
    let mut model = SettingsModel::new();
    file.load(&mut model, now());
    assert_close(model.get(SettingId::AudioVolume).as_float(), 0.3, 4);
    model.set(SettingId::AudioVolume, 0.9_f32);
    file.load(&mut model, now());
    assert_close(model.get(SettingId::AudioVolume).as_float(), 0.3, 4);
}

/// §4.5.1: enumerations are written as the label of their domain and read back from either form.
#[test]
fn enumerations_are_written_as_labels_and_read_from_either_form() {
    let mut written = SettingsModel::new();
    written.set(SettingId::SceneSkybox, SettingValue::Int(1));
    let mut file = SettingsFile::new(MemoryStorage::new());
    file.save(&written).expect("the store accepts the write");
    assert_eq!(
        stored_values(file.storage()).expect("the values object")["scene.skybox"].as_str(),
        Some("Gradient")
    );
    let mut file = SettingsFile::new(storage_with(
        r#"{ "schemaVersion": 1, "values": { "scene.skybox": 2 } }"#,
    ));
    let mut read = SettingsModel::new();
    file.load(&mut read, now());
    assert_eq!(read.get(SettingId::SceneSkybox).as_int(), 2);
}

/// AMD-031 §3: a value stored for the TV box key is read without error, has no effect, and is not written
/// back, unlike the unknown keys of REQ-PERS-023, which survive a rewrite.
#[rstest]
#[case("1")]
#[case("\"TV\"")]
#[case("\"TV 3\"")]
#[case("3")]
fn amd_031_a_stored_tv_box_value_is_ignored_and_dropped(#[case] stored: &str) {
    let text = format!(
        r#"{{ "schemaVersion": 1, "values": {{ "scene.skybox": 1, "scene.tv_box": {stored}, "vr.unknown": 7 }} }}"#
    );
    let mut file = SettingsFile::new(storage_with(&text));
    let mut model = SettingsModel::new();
    let result = file.load(&mut model, now());
    // Read without error, and the rest of the file is applied as usual.
    assert_eq!(result.status, SettingsLoadStatus::Loaded);
    assert_eq!(model.get(SettingId::SceneSkybox).as_int(), 1);
    assert!(SettingId::from_key("scene.tv_box").is_none());
    file.save(&model).expect("the store accepts the write");
    let values = stored_values(file.storage()).expect("the values object");
    // The retired key is gone; a genuinely unknown key is still kept (REQ-PERS-023).
    assert!(!values.contains_key("scene.tv_box"));
    assert_eq!(values["vr.unknown"].as_i64(), Some(7));
}

/// §4.5.1: a vector is an array of numbers, and a rectangle likewise.
#[test]
fn vectors_and_rectangles_are_arrays_of_numbers() {
    let mut written = SettingsModel::new();
    written.set(SettingId::UiWindowRect, Rect::new(10, 20, 1920, 1080));
    let mut file = SettingsFile::new(MemoryStorage::new());
    file.save(&written).expect("the store accepts the write");
    assert!(stored_values(file.storage()).expect("the values object")["ui.window_rect"].is_array());
    let mut read = SettingsModel::new();
    let mut again = SettingsFile::new(file.storage().clone());
    again.load(&mut read, now());
    assert_eq!(
        read.get(SettingId::UiWindowRect).as_rect(),
        Rect::new(10, 20, 1920, 1080)
    );
}

/// A preference back at its default is dropped from the file.
#[test]
fn a_preference_back_at_its_default_leaves_the_file() {
    let mut model = SettingsModel::new();
    let mut file = SettingsFile::new(MemoryStorage::new());
    model.set(SettingId::CameraFov, 50.0_f32);
    file.save(&model).expect("the store accepts the write");
    assert!(file
        .storage()
        .text(NAME)
        .is_some_and(|text| text.contains("camera.fov")));
    model.reset(SettingId::CameraFov);
    file.save(&model).expect("the store accepts the write");
    assert!(!file
        .storage()
        .text(NAME)
        .is_some_and(|text| text.contains("camera.fov")));
}

/// Rust-only: the widescreen choice once called "Window" is "Auto" now; a file written before
/// still reads.
#[test]
fn the_former_window_widescreen_reads_as_auto() {
    let storage = storage_with(
        r#"{
  "schemaVersion": 1,
  "values": { "video.widescreen": "Window" }
}"#,
    );
    let mut model = SettingsModel::new();
    let mut file = SettingsFile::new(storage);
    file.load(&mut model, now());
    assert_eq!(1, model.get(SettingId::VideoWidescreen).as_int());
}
