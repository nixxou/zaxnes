//! Port of `PersistenceTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::io::Write;
use std::path::Path;

use common::{LoopHarness, SyntheticRom, TemporaryRoot};
use time::OffsetDateTime;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::rom_file::{
    self, CORRUPTED_ZIP_MESSAGE, INVALID_ROM_MESSAGE, UNCOMPRESSION_FAILED_MESSAGE,
};
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_app::persistence::save_slots::SaveSlotIndex;
use zax_app::persistence::settings_store::{
    SettingsStore, NEWER_VERSION_MESSAGE, UNREADABLE_MESSAGE,
};
use zax_app::user_data_paths::{
    try_copy_legacy_root, UserDataPaths, UserDataRootKind, LEGACY_PRODUCT_NAME,
    PORTABLE_FOLDER_NAME, PORTABLE_MARKER_NAME, PRODUCT_NAME,
};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

fn files_matching(folder: &Path, prefix: &str) -> usize {
    std::fs::read_dir(folder)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .count()
}

#[test]
fn req_pers_072_the_layout_is_created_on_the_first_start() {
    let root = TemporaryRoot::new("zax-pers-");
    let paths = UserDataPaths::resolve(Some(&root.path().join("userdata")), None, None);
    assert!(paths.is_writable);
    for folder in [
        &paths.saves_directory,
        &paths.battery_directory,
        &paths.profiles_directory,
        // REWRITE-UI step 9: no covers any more, so no covers folder.
        &paths.logs_directory,
    ] {
        assert!(folder.is_dir());
    }
    assert!(paths
        .save_index_file
        .to_string_lossy()
        .ends_with("index.json"));
}

#[test]
fn req_pers_071_portable_mode_uses_the_userdata_folder() {
    let root = TemporaryRoot::new("zax-pers-");
    let executable = root.path().join("bin");
    std::fs::create_dir_all(&executable).unwrap();
    std::fs::write(executable.join(PORTABLE_MARKER_NAME), "").unwrap();
    let portable = UserDataPaths::resolve(None, Some(&executable), Some(&root.path().join("none")));
    assert_eq!(UserDataRootKind::Portable, portable.kind);
    assert_eq!(executable.join(PORTABLE_FOLDER_NAME), portable.root);
    let forced = root.path().join("forced");
    let overridden = UserDataPaths::resolve(Some(&forced), Some(&executable), None);
    assert_eq!(UserDataRootKind::Explicit, overridden.kind);
    assert_eq!(std::path::absolute(&forced).unwrap(), overridden.root);
}

/// Owner request (2026-11-12): a portable folder starts on its own; nothing of the per-user root
/// is read or copied into it.
#[test]
fn a_portable_start_reads_nothing_of_the_per_user_root() {
    let root = TemporaryRoot::new("zax-pers-");
    let executable = root.path().join("bin");
    std::fs::create_dir_all(&executable).unwrap();
    std::fs::write(executable.join(PORTABLE_MARKER_NAME), "").unwrap();
    let local = root.path().join("local");
    std::fs::create_dir_all(local.join(PRODUCT_NAME)).unwrap();
    std::fs::write(local.join(PRODUCT_NAME).join("settings.json"), "mine").unwrap();
    let paths = UserDataPaths::resolve(None, Some(&executable), Some(&local));
    assert_eq!(UserDataRootKind::Portable, paths.kind);
    assert!(paths.migrated_from.is_none());
    assert!(!paths.settings_file.exists());
}

/// Owner request: a portable folder that takes no writes (Program Files) leaves the data in the
/// per-user root.
#[test]
fn an_unwritable_portable_folder_falls_back_to_the_per_user_root() {
    let root = TemporaryRoot::new("zax-pers-");
    let executable = root.path().join("bin");
    std::fs::create_dir_all(&executable).unwrap();
    std::fs::write(executable.join(PORTABLE_MARKER_NAME), "").unwrap();
    // A file where the folder would be: nothing can be created under it.
    std::fs::write(executable.join(PORTABLE_FOLDER_NAME), "").unwrap();
    let local = root.path().join("local");
    let paths = UserDataPaths::resolve(None, Some(&executable), Some(&local));
    assert_eq!(UserDataRootKind::LocalAppData, paths.kind);
    assert_eq!(local.join(PRODUCT_NAME), paths.root);
    assert!(paths.is_writable);
}

#[test]
fn amd_029_the_first_run_copies_the_root_of_the_old_name() {
    let root = TemporaryRoot::new("zax-pers-");
    let local = root.path().join("local");
    let legacy = local.join(LEGACY_PRODUCT_NAME);
    std::fs::create_dir_all(legacy.join("saves")).unwrap();
    std::fs::create_dir_all(legacy.join("profiles")).unwrap();
    std::fs::write(legacy.join("settings.json"), r#"{ "schemaVersion": 1 }"#).unwrap();
    std::fs::write(legacy.join("saves").join("quick.state"), [1, 2, 3, 4]).unwrap();
    std::fs::write(legacy.join("profiles").join("a.3dn"), [9]).unwrap();
    let paths = UserDataPaths::resolve(None, None, Some(&local));
    assert_eq!(UserDataRootKind::LocalAppData, paths.kind);
    assert_eq!(local.join(PRODUCT_NAME), paths.root);
    assert_eq!(Some(legacy.clone()), paths.migrated_from);
    assert_eq!(
        r#"{ "schemaVersion": 1 }"#,
        std::fs::read_to_string(&paths.settings_file).unwrap()
    );
    assert_eq!(
        vec![1, 2, 3, 4],
        std::fs::read(paths.legacy_slot_state_file("quick")).unwrap()
    );
    assert_eq!(
        vec![9],
        std::fs::read(paths.profiles_directory.join("a.3dn")).unwrap()
    );
    // The old root is read, never written to, moved or deleted.
    assert!(legacy.is_dir());
    assert_eq!(
        vec![1, 2, 3, 4],
        std::fs::read(legacy.join("saves").join("quick.state")).unwrap()
    );
    assert_eq!(3, std::fs::read_dir(&legacy).unwrap().count());
}

#[test]
fn amd_029_an_existing_root_is_never_overwritten() {
    let root = TemporaryRoot::new("zax-pers-");
    let local = root.path().join("second-run");
    let legacy = local.join(LEGACY_PRODUCT_NAME);
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("settings.json"), "old").unwrap();
    let current = local.join(PRODUCT_NAME);
    std::fs::create_dir_all(&current).unwrap();
    std::fs::write(current.join("settings.json"), "mine").unwrap();
    let paths = UserDataPaths::resolve(None, None, Some(&local));
    assert!(paths.migrated_from.is_none());
    assert_eq!(
        "mine",
        std::fs::read_to_string(&paths.settings_file).unwrap()
    );
    // No old root at all: a plain first start, with nothing to say.
    let fresh = root.path().join("fresh");
    std::fs::create_dir_all(&fresh).unwrap();
    let none = UserDataPaths::resolve(None, None, Some(&fresh));
    assert!(none.migrated_from.is_none());
    assert!(none.is_writable);
    // And the explicit and portable roots are not migrated: §2 only moves the default one.
    let explicit = UserDataPaths::resolve(Some(&root.path().join("explicit")), None, Some(&local));
    assert_eq!(UserDataRootKind::Explicit, explicit.kind);
    assert!(explicit.migrated_from.is_none());
}

#[test]
fn amd_029_the_copy_refuses_the_cases_that_are_not_a_migration() {
    let root = TemporaryRoot::new("zax-pers-");
    let legacy = root.path().join("edge").join("Sen");
    let target = root.path().join("edge").join("Zaxnes");
    // No old root.
    assert!(!try_copy_legacy_root(&legacy, &target));
    assert!(!target.exists());
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("library.json"), "[]").unwrap();
    // The same folder under two spellings.
    let spelled = format!("{}{}", legacy.display(), std::path::MAIN_SEPARATOR);
    assert!(!try_copy_legacy_root(&legacy, Path::new(&spelled)));
    assert!(try_copy_legacy_root(&legacy, &target));
    assert_eq!(
        "[]",
        std::fs::read_to_string(target.join("library.json")).unwrap()
    );
    // Second call: the target now exists, so nothing is copied over it.
    std::fs::write(legacy.join("library.json"), "changed").unwrap();
    assert!(!try_copy_legacy_root(&legacy, &target));
    assert_eq!(
        "[]",
        std::fs::read_to_string(target.join("library.json")).unwrap()
    );
}

#[test]
fn req_pers_023_unknown_keys_survive_a_rewrite() {
    let root = TemporaryRoot::new("zax-pers-");
    let paths = UserDataPaths::resolve(Some(&root.path().join("settings")), None, None);
    std::fs::write(
        &paths.settings_file,
        r#"{ "schemaVersion": 1, "values": { "audio.volume": 0.5, "future.x": 3 } }"#,
    )
    .unwrap();
    let mut model = SettingsModel::new();
    let mut store = SettingsStore::new(&paths);
    store.load(&mut model, at(0));
    assert!((model.volume() - 0.5).abs() < 1e-6);
    assert!(store.message().is_none());
    model.set_volume(0.75);
    store.save(&model);
    let json = std::fs::read_to_string(&paths.settings_file).unwrap();
    assert!(json.contains("\"future.x\": 3"), "{json}");
    assert!(json.contains("0.75"), "{json}");
}

#[test]
fn req_pers_022_an_unreadable_settings_file_is_set_aside() {
    let root = TemporaryRoot::new("zax-pers-");
    let paths = UserDataPaths::resolve(Some(&root.path().join("corrupt")), None, None);
    std::fs::write(&paths.settings_file, "{{{").unwrap();
    let mut model = SettingsModel::new();
    let mut store = SettingsStore::new(&paths);
    store.load(&mut model, at(1_700_000_000));
    assert_eq!(Some(UNREADABLE_MESSAGE), store.message());
    assert!((model.volume() - 1.0).abs() < 1e-6);
    assert!((model.fov() - 70.0).abs() < 1e-6);
    assert!(!paths.settings_file.exists());
    assert_eq!(1, files_matching(&paths.root, "settings.json.corrupt-"));
}

#[test]
fn req_pers_003_an_invalid_value_falls_back_alone() {
    let root = TemporaryRoot::new("zax-pers-");
    let paths = UserDataPaths::resolve(Some(&root.path().join("domain")), None, None);
    std::fs::write(
        &paths.settings_file,
        r#"{ "schemaVersion": 1, "values": { "camera.fov": "abc", "video.frame_rate": 75,
              "emu.game_speed": 12, "audio.volume": 0.5 } }"#,
    )
    .unwrap();
    let mut model = SettingsModel::new();
    let mut store = SettingsStore::new(&paths);
    store.load(&mut model, at(0));
    assert!(store.message().is_none());
    assert!((model.fov() - 70.0).abs() < 1e-6);
    // The default, Auto: the display's rate, 60 until it is known.
    assert_eq!(60, model.frame_rate());
    assert_eq!(8, model.game_speed());
    assert!((model.volume() - 0.5).abs() < 1e-6);
}

#[test]
fn req_pers_024_a_newer_file_is_left_alone() {
    let root = TemporaryRoot::new("zax-pers-");
    let paths = UserDataPaths::resolve(Some(&root.path().join("newer")), None, None);
    let content = r#"{ "schemaVersion": 99, "values": { "audio.volume": 0.25 } }"#;
    std::fs::write(&paths.settings_file, content).unwrap();
    let mut model = SettingsModel::new();
    let mut store = SettingsStore::new(&paths);
    store.load(&mut model, at(0));
    assert_eq!(Some(NEWER_VERSION_MESSAGE), store.message());
    assert!((model.volume() - 0.25).abs() < 1e-6);
    model.set_volume(1.0);
    store.save(&model);
    assert_eq!(
        content,
        std::fs::read_to_string(&paths.settings_file).unwrap()
    );
}

#[test]
fn req_pers_009_only_a_real_change_raises_an_event() {
    let mut model = SettingsModel::new();
    model.set_fov(80.0);
    model.set_fov(80.0);
    model.set_volume(1.0);
    let changes: Vec<SettingId> = model.drain_changes().map(|change| change.id).collect();
    assert_eq!(vec![SettingId::CameraFov], changes);
}

#[test]
fn req_pers_055_the_slots_are_reconciled_at_start_up() {
    let root = TemporaryRoot::new("zax-pers-");
    let paths = UserDataPaths::resolve(Some(&root.path().join("slots")), None, None);
    let mut index = SaveSlotIndex::new();
    index.occupy("1", "alive", 10, false);
    index.occupy("2", "alive", 20, false);
    index.occupy("3", "gone", 30, false);
    std::fs::write(paths.legacy_slot_state_file("1"), [1]).unwrap();
    std::fs::write(paths.legacy_slot_state_file("3"), [3]).unwrap();
    index.save(&paths).unwrap();
    let mut reread = SaveSlotIndex::load(&paths, at(0));
    let cleared = reread.reconcile(&paths, |id| id == "alive");
    assert_eq!(2, cleared);
    assert!(reread.find("1").unwrap().is_occupied());
    assert!(!reread.find("2").unwrap().is_occupied());
    assert!(!reread.find("3").unwrap().is_occupied());
    assert!(!paths.legacy_slot_state_file("3").exists());
}

#[test]
fn req_pers_055_a_corrupt_index_is_rebuilt_from_the_files() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(9));
    let rom = harness.host.rom_file_name("g1");
    harness.host.shutdown();
    // The shared slots of before: a state of theirs, and their index unreadable.
    std::fs::rename(
        harness.paths.slot_state_file(&rom, "9"),
        harness.paths.legacy_slot_state_file("9"),
    )
    .unwrap();
    std::fs::write(&harness.paths.save_index_file, "not json at all").unwrap();
    let rebuilt = SaveSlotIndex::load(&harness.paths, at(harness.now()));
    assert!(rebuilt.was_recovered);
    assert!(rebuilt.find("9").unwrap().is_occupied());
    assert_eq!("g1", rebuilt.find("9").unwrap().game_id);
    assert_eq!(
        1,
        files_matching(&harness.paths.saves_directory, "index.json.corrupt-")
    );
}

#[test]
fn req_pers_073_an_unwritable_root_is_reported() {
    let root = TemporaryRoot::new("zax-pers-");
    // A path whose parent is a file can never be created.
    let file = root.path().join("blocker");
    std::fs::write(&file, "x").unwrap();
    let paths = UserDataPaths::resolve(Some(&file.join("root")), None, None);
    assert!(!paths.is_writable);
    assert_eq!(
        Some(format!(
            "User data folder is not writable: {}",
            paths.root.display()
        )),
        paths.unwritable_message
    );
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    for (name, contents) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(contents).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn req_loop_022_a_zip_uses_its_first_entry() {
    let root = TemporaryRoot::new("zax-pers-");
    let archive = root.path().join("Foo.zip");
    write_zip(&archive, &[("bar.nes", &SyntheticRom::build(None, false))]);
    let result = rom_file::read(&archive).unwrap();
    assert_eq!("Foo.nes", result.name);
    assert_eq!(16 + 32 * 1024 + 8 * 1024, result.image.len());
}

#[test]
fn req_loop_023_archive_failures_report_the_exact_messages() {
    let root = TemporaryRoot::new("zax-pers-");
    let broken = root.path().join("broken.zip");
    std::fs::write(&broken, [0x50, 0x4B, 0x03, 0x04, 0xFF, 0xFF]).unwrap();
    assert_eq!(CORRUPTED_ZIP_MESSAGE, rom_file::read(&broken).unwrap_err());
    let empty = root.path().join("empty.zip");
    write_zip(&empty, &[]);
    assert_eq!(
        UNCOMPRESSION_FAILED_MESSAGE,
        rom_file::read(&empty).unwrap_err()
    );
    // A damaged 7z (owner request: 7z archives are read) is refused cleanly.
    let seven = root.path().join("game.7z");
    std::fs::write(&seven, [0x37, 0x7A]).unwrap();
    assert_eq!(
        UNCOMPRESSION_FAILED_MESSAGE,
        rom_file::read(&seven).unwrap_err()
    );
}

#[test]
fn req_loop_023_a_missing_file_reports_the_system_message() {
    let root = TemporaryRoot::new("zax-pers-");
    let message = rom_file::read(&root.path().join("absent.nes")).unwrap_err();
    assert_ne!(message.as_str(), "");
    assert_ne!(INVALID_ROM_MESSAGE, message);
}

#[test]
fn req_pers_060_the_battery_ram_survives_a_restart() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::battery_writer(0x5A), "battery.nes");
    harness.catalog().add("bat", Some(&rom));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("bat"));
    harness.run_frames(10);
    assert_eq!(
        0x5A,
        harness.host.session().unwrap().core().battery_ram()[0]
    );
    harness.run(Command::Stop);
    let files: Vec<_> = std::fs::read_dir(&harness.paths.battery_directory)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "srm")
        })
        .collect();
    assert_eq!(1, files.len());
    assert_eq!(0x5A, std::fs::read(files[0].path()).unwrap()[0]);
    // A second launch reads the file back before the first frame runs.
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("bat"));
    assert_eq!(
        0x5A,
        harness.host.session().unwrap().core().battery_ram()[0]
    );
}

#[test]
fn req_pers_060_a_rom_without_a_battery_writes_nothing() {
    let mut harness = LoopHarness::create();
    harness.launch("plain");
    harness.run_frames(10);
    harness.run(Command::Stop);
    assert_eq!(
        0,
        std::fs::read_dir(&harness.paths.battery_directory)
            .unwrap()
            .count()
    );
}

/// Lot MAPPERS-P: a `.studybox` tape loads with `StudyBox.bin` found beside it, packaged as the
/// NES 2.0 image the core runs (mapper 186); without the BIOS the user is told where to put it.
#[test]
fn a_study_box_tape_is_read_with_its_bios() {
    let root = TemporaryRoot::new("zax-pers-");
    let mut tape = b"STBX".to_vec();
    tape.extend_from_slice(&4u32.to_le_bytes());
    tape.extend_from_slice(&0x100u32.to_le_bytes());
    tape.extend_from_slice(b"PAGE");
    tape.extend_from_slice(&14u32.to_le_bytes());
    tape.extend_from_slice(&[0, 0, 0, 0, 4, 0, 0, 0, 0xC5, 1, 2, 3, 4, 0]);
    let path = root.path().join("lesson.studybox");
    std::fs::write(&path, &tape).unwrap();
    assert_eq!(
        zax_app::emulation::rom_file::STUDY_BOX_BIOS_MISSING_MESSAGE,
        rom_file::read(&path).unwrap_err()
    );
    std::fs::write(root.path().join("StudyBox.bin"), vec![0u8; 0x4_0000]).unwrap();
    let rom = rom_file::read(&path).unwrap();
    assert_eq!("lesson.studybox", rom.name);
    assert_eq!(16 + 0x4_0000 + tape.len(), rom.image.len());
    assert_eq!(&rom.image[..4], b"NES\x1A");
}

/// Lot FDS: a `.fds` disk image loads with `disksys.rom` found beside it, packaged as the NES 2.0
/// image the core runs (mapper 20: the BIOS twice, then the disk); without the BIOS the user is
/// told where to put it, and a `.qd` is read as a Quick Disk image.
#[test]
fn a_disk_image_is_read_with_the_disk_system_bios() {
    let root = TemporaryRoot::new("zax-pers-");
    let mut disk = b"FDS\x1A\x01".to_vec();
    disk.resize(16, 0);
    let mut side = vec![0u8; 65_500];
    side[0] = 0x01;
    side[1..15].copy_from_slice(b"*NINTENDO-HVC*");
    disk.extend_from_slice(&side);
    let path = root.path().join("game.fds");
    std::fs::write(&path, &disk).unwrap();
    assert_eq!(
        zax_app::emulation::rom_file::FDS_BIOS_MISSING_MESSAGE,
        rom_file::read(&path).unwrap_err()
    );
    std::fs::write(root.path().join("disksys.rom"), vec![0u8; 0x2000]).unwrap();
    let rom = rom_file::read(&path).unwrap();
    assert_eq!("game.fds", rom.name);
    assert_eq!(16 + 2 * 0x2000 + disk.len(), rom.image.len());
    assert_eq!(&rom.image[..4], b"NES\x1A");

    let mut core = zax_nes::NesCore::new();
    assert!(core.load_rom(&rom.image, &rom.name).success);
    assert_eq!(1, core.disk_sides());
}

/// Lot MAPPERS-P: a board whose save medium is its flash (here the COOLGIRL's save area) keeps
/// it in `<rom>.flash` beside the RAM files, written when the game stops and read back before
/// the first frame of the next launch.
#[test]
fn a_flash_save_survives_a_restart() {
    // A COOLGIRL image (mapper 342, NES 2.0): 128 KiB of PRG, CHR-RAM. The reset code at $E000
    // copies a routine to $0300 (moving the PRG base moves every window) that maps the save area
    // at $8000, enables flash writes and programs $5A at $8000 by the JEDEC sequence.
    let mut routine = Vec::new();
    for (value, address) in [
        (0xFE, 0x5000u16),
        (0x00, 0x5001),
        (0x04, 0x5007),
        (0xAA, 0x8AAA),
        (0x55, 0x8555),
        (0xA0, 0x8AAA),
        (0x5A, 0x8000),
    ] {
        routine.extend_from_slice(&[0xA9, value, 0x8D]);
        routine.extend_from_slice(&address.to_le_bytes());
    }
    let here = 0x0300 + u16::try_from(routine.len()).unwrap();
    routine.push(0x4C);
    routine.extend_from_slice(&here.to_le_bytes());
    let length = u8::try_from(routine.len() - 1).unwrap();
    let mut code = vec![
        0xA2, length, 0xBD, 0x10, 0xE0, 0x9D, 0x00, 0x03, 0xCA, 0x10, 0xF7,
    ];
    code.extend_from_slice(&[0x4C, 0x00, 0x03]);
    code.resize(0x10, 0xEA);
    code.extend_from_slice(&routine);

    let prg = 128 * 1024;
    let mut image = vec![0u8; 16 + prg];
    image[..16].copy_from_slice(&[
        0x4E, 0x45, 0x53, 0x1A, 8, 0, 0x60, 0x58, 0x01, 0, 0, 0, 0, 0, 0, 0,
    ]);
    let last_bank = 16 + prg - 0x2000;
    image[last_bank..last_bank + code.len()].copy_from_slice(&code);
    for vector in 0..3 {
        image[16 + prg - 6 + vector * 2] = 0x00;
        image[16 + prg - 5 + vector * 2] = 0xE0;
    }

    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&image, "coolgirl.nes");
    harness.catalog().add("cg", Some(&rom));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("cg"));
    harness.run_frames(5);
    let saved = harness.host.session().unwrap().core().export_flash_save();
    assert_eq!(b"ZXCG", &saved[..4]);
    harness.run(Command::Stop);
    let files: Vec<_> = std::fs::read_dir(&harness.paths.battery_directory)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "flash")
        })
        .collect();
    assert_eq!(1, files.len());
    assert_eq!(saved, std::fs::read(&files[0]).unwrap());
    // The next launch reads it back before the first frame runs.
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("cg"));
    assert_eq!(
        saved,
        harness.host.session().unwrap().core().export_flash_save()
    );
}
