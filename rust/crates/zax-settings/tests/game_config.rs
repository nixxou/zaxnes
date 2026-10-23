//! The per-ROM choices beside the library: the remembered profile and the ten recent ROMs (AMD-010
//! REQ-UI-200, REQ-UI-201; REQ-PERS-021, REQ-PERS-022, REQ-PERS-024).

use time::macros::datetime;
use time::OffsetDateTime;
use zax_settings::catalogue::{self};
use zax_settings::games::GameConfigFile;
use zax_settings::persistence::SettingsLoadStatus;
use zax_settings::store::MemoryStorage;

fn now() -> OffsetDateTime {
    datetime!(2026-10-21 09:00:00 UTC)
}

/// AMD-010 REQ-UI-200: the File menu lists the ten last ROMs, most recent first.
#[test]
fn the_recent_list_keeps_the_ten_last_roms_most_recent_first() {
    let mut file = GameConfigFile::new(MemoryStorage::new());
    for i in 1..=12 {
        file.note_rom_opened(&format!(r"D:\roms\game{i}.nes"));
    }
    assert_eq!(
        file.recent_roms().len(),
        GameConfigFile::<MemoryStorage>::RECENT_ROM_LIMIT
    );
    assert_eq!(file.recent_roms()[0], r"D:\roms\game12.nes");
    assert_eq!(file.recent_roms()[9], r"D:\roms\game3.nes");
    assert!(!file
        .recent_roms()
        .contains(&r"D:\roms\game1.nes".to_owned()));
}

/// Opening a ROM again moves it to the head instead of adding a second entry.
#[test]
fn opening_a_rom_again_moves_it_to_the_head() {
    let mut file = GameConfigFile::new(MemoryStorage::new());
    file.note_rom_opened(r"D:\roms\a.nes");
    file.note_rom_opened(r"D:\roms\b.nes");
    file.note_rom_opened(r"D:\ROMS\A.NES");
    assert_eq!(file.recent_roms(), [r"D:\ROMS\A.NES", r"D:\roms\b.nes"]);
}

/// AMD-010 REQ-UI-201: the second opening of the same file starts straight away with the remembered
/// profile.
#[test]
fn the_profile_chosen_for_a_rom_file_is_remembered() {
    let mut first = GameConfigFile::new(MemoryStorage::new());
    assert_eq!(first.profile_for(r"D:\roms\digdug.nes"), None);
    first.set_profile_for(r"D:\roms\digdug.nes", "10");
    first.note_rom_opened(r"D:\roms\digdug.nes");
    first.save().expect("the store accepts the write");
    let mut second = GameConfigFile::new(first.storage().clone());
    assert_eq!(second.load(now()).status, SettingsLoadStatus::Loaded);
    assert_eq!(second.profile_for(r"D:\roms\digdug.nes"), Some("10"));
    assert_eq!(second.recent_roms(), [r"D:\roms\digdug.nes"]);
}

/// A disappeared ROM can be dropped from both lists.
#[test]
fn a_rom_can_be_forgotten() {
    let mut file = GameConfigFile::new(MemoryStorage::new());
    file.note_rom_opened(r"D:\roms\gone.nes");
    file.set_profile_for(r"D:\roms\gone.nes", "7");
    assert!(file.forget_rom(r"D:\roms\gone.nes"));
    assert!(file.recent_roms().is_empty());
    assert_eq!(file.profile_for(r"D:\roms\gone.nes"), None);
    assert!(!file.forget_rom(r"D:\roms\gone.nes"));
}

/// REQ-PERS-022: an unreadable file is set aside and the lists start empty.
#[test]
fn an_unreadable_file_is_set_aside() {
    let mut storage = MemoryStorage::new();
    storage.put(
        GameConfigFile::<MemoryStorage>::DEFAULT_NAME,
        "not json at all",
    );
    let mut file = GameConfigFile::new(storage);
    let result = file.load(now());
    assert_eq!(result.status, SettingsLoadStatus::Unreadable);
    assert_eq!(
        result.message,
        Some(GameConfigFile::<MemoryStorage>::UNREADABLE_MESSAGE)
    );
    assert!(file
        .storage()
        .names()
        .any(|name| name == "games.json.corrupt-20261021-090000"));
    assert!(file.recent_roms().is_empty());
}

/// REQ-PERS-024: a file from a newer version is read at best and left alone.
#[test]
fn a_newer_file_is_read_at_best_and_never_rewritten() {
    let mut storage = MemoryStorage::new();
    storage.put(
        GameConfigFile::<MemoryStorage>::DEFAULT_NAME,
        r#"{ "schemaVersion": 99, "recentRoms": ["D:\\a.nes"], "romProfiles": { "D:\\a.nes": "3" } }"#,
    );
    let mut file = GameConfigFile::new(storage);
    let result = file.load(now());
    assert_eq!(result.status, SettingsLoadStatus::NewerVersion);
    assert_eq!(file.recent_roms(), [r"D:\a.nes"]);
    assert_eq!(file.profile_for(r"D:\a.nes"), Some("3"));
    file.note_rom_opened(r"D:\b.nes");
    assert!(file.save().is_err());
    let text = file
        .storage()
        .text(GameConfigFile::<MemoryStorage>::DEFAULT_NAME)
        .expect("the file is still there");
    assert!(text.contains("\"schemaVersion\": 99"));
}

/// REQ-PERS-024: an older file leaves a `.vN.bak` copy behind.
#[test]
fn an_older_file_leaves_a_backup_copy() {
    let mut storage = MemoryStorage::new();
    storage.put(
        GameConfigFile::<MemoryStorage>::DEFAULT_NAME,
        r#"{ "schemaVersion": 0, "recentRoms": [] }"#,
    );
    let mut file = GameConfigFile::new(storage);
    assert_eq!(file.load(now()).status, SettingsLoadStatus::Migrated);
    assert!(file
        .storage()
        .names()
        .any(|name| name == "games.json.v0.bak"));
}

/// REQ-PERS-075: an absent file is not an error: the lists simply start empty, with no notification.
#[test]
fn an_absent_file_starts_empty_without_a_notification() {
    let mut file = GameConfigFile::new(MemoryStorage::new());
    let result = file.load(now());
    assert_eq!(result.status, SettingsLoadStatus::Absent);
    assert_eq!(result.message, None);
    assert!(file.recent_roms().is_empty());
}

/// REQ-PERS-031: no setting is remembered per game. The file beside the library carries the recent list
/// and the ROM-to-profile choices and nothing else.
#[test]
fn the_file_carries_no_settings() {
    let mut file = GameConfigFile::new(MemoryStorage::new());
    file.note_rom_opened(r"D:\roms\a.nes");
    file.set_profile_for(r"D:\roms\a.nes", "4");
    file.save().expect("the store accepts the write");
    let written = file
        .storage()
        .text(GameConfigFile::<MemoryStorage>::DEFAULT_NAME)
        .expect("the file was written");
    for descriptor in catalogue::descriptors() {
        assert!(
            !written.contains(descriptor.key().as_ref()),
            "{}",
            descriptor.key()
        );
    }
}
