//! Port of `SettingsWiringTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::{LoopHarness, SyntheticRom, TemporaryRoot};
use time::OffsetDateTime;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_app::persistence::settings_store::SettingsStore;
use zax_app::user_data_paths::UserDataPaths;
use zax_engine3d::settings::ZLayer;
use zax_settings::catalogue::{Layer, Port, SettingId};
use zax_settings::model::{SettingLayer, SettingsModel};
use zax_settings::values::SettingValue;

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

struct Root {
    _folder: TemporaryRoot,
    paths: UserDataPaths,
}

fn root() -> Root {
    let folder = TemporaryRoot::new("zax-settings-");
    let paths = UserDataPaths::resolve(Some(folder.path()), None, None);
    Root {
        _folder: folder,
        paths,
    }
}

#[test]
fn the_host_view_reads_and_writes_the_model() {
    let mut model = SettingsModel::new();
    model.set_fov(85.0);
    assert!((model.get(SettingId::CameraFov).as_float() - 85.0).abs() < f32::EPSILON);
    model.set(SettingId::AudioVolume, 0.25_f32);
    assert!((model.volume() - 0.25).abs() < 1e-4);
}

#[test]
fn a_change_reaches_the_host_with_its_stable_key() {
    let mut model = SettingsModel::new();
    model.set_volume(0.5);
    model.set_volume(0.5);
    let seen: Vec<String> = model
        .drain_changes()
        .map(|change| change.id.key().into_owned())
        .collect();
    assert_eq!(vec!["audio.volume".to_owned()], seen);
}

#[test]
fn the_host_view_validates_through_the_model() {
    let mut model = SettingsModel::new();
    model.set_frame_rate(75);
    model.set_game_speed(12);
    model.set_skybox(7);
    assert_eq!(90, model.frame_rate());
    assert_eq!(8, model.game_speed());
    assert_eq!(-1, model.skybox());
}

#[test]
fn the_end_of_a_game_resets_the_speed_and_the_session_layer() {
    let mut model = SettingsModel::new();
    model.set_game_speed(4);
    model.set_layer(SettingId::CameraSmoothTime, 0.9_f32, SettingLayer::Session);
    model.end_of_game();
    assert_eq!(0, model.game_speed());
    assert!((model.get(SettingId::CameraSmoothTime).as_float() - 0.4).abs() < 1e-4);
}

#[test]
fn the_preferences_survive_a_restart() {
    let root = root();
    let mut first = SettingsModel::new();
    let mut store = SettingsStore::new(&root.paths);
    store.load(&mut first, at(0));
    assert!((first.fov() - 70.0).abs() < f32::EPSILON);
    first.set_fov(55.0);
    first.set_volume(0.4);
    first.set_fullscreen(true);
    store.save(&first);
    let mut second = SettingsModel::new();
    SettingsStore::new(&root.paths).load(&mut second, at(0));
    assert!((second.fov() - 55.0).abs() < f32::EPSILON);
    assert!((second.volume() - 0.4).abs() < 1e-4);
    assert!(second.fullscreen());
}

#[test]
fn a_script_write_on_a_preference_is_never_persisted() {
    let root = root();
    let mut settings = SettingsModel::new();
    let mut store = SettingsStore::new(&root.paths);
    store.load(&mut settings, at(0));
    settings.set_volume(0.8);
    settings.set_layer(SettingId::AudioVolume, 0.1_f32, SettingLayer::Session);
    assert!((settings.volume() - 0.1).abs() < 1e-4);
    store.save(&settings);
    settings.end_of_game();
    assert!((settings.volume() - 0.8).abs() < 1e-4);
    let mut reread = SettingsModel::new();
    SettingsStore::new(&root.paths).load(&mut reread, at(0));
    assert!((reread.volume() - 0.8).abs() < 1e-4);
}

#[test]
fn a_change_from_the_ui_is_written_within_two_seconds() {
    let root = root();
    let mut settings = SettingsModel::new();
    let mut store = SettingsStore::new(&root.paths);
    store.load(&mut settings, at(0));
    assert!(!store.has_pending_write());
    settings.set_fov(45.0);
    for change in settings.drain_changes().collect::<Vec<_>>() {
        store.on_setting_changed(&settings, change.id);
    }
    assert!(store.has_pending_write());
    store.tick(&settings, 10.0);
    store.tick(&settings, 12.1);
    assert!(!store.has_pending_write());
    let mut reread = SettingsModel::new();
    SettingsStore::new(&root.paths).load(&mut reread, at(0));
    assert!((reread.fov() - 45.0).abs() < f32::EPSILON);
}

#[test]
fn an_unreadable_file_reports_its_notification_to_the_host() {
    let root = root();
    std::fs::write(root.paths.root.join("settings.json"), "{{{").unwrap();
    let mut settings = SettingsModel::new();
    let mut store = SettingsStore::new(&root.paths);
    store.load(&mut settings, at(1_760_950_000));
    assert_eq!(
        Some("Settings file was unreadable and has been reset."),
        store.message()
    );
    assert!((settings.fov() - 70.0).abs() < f32::EPSILON);
    assert!(!root.paths.root.join("settings.json").exists());
    let set_aside = std::fs::read_dir(&root.paths.root)
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("settings.json.corrupt-")
        });
    assert!(set_aside);
}

#[test]
fn the_recent_roms_and_the_rom_profile_choices_survive_a_restart() {
    let root = root();
    let mut settings = SettingsModel::new();
    let mut store = SettingsStore::new(&root.paths);
    store.load(&mut settings, at(0));
    store.games_mut().note_rom_opened(r"D:\roms\a.nes");
    store.games_mut().note_rom_opened(r"D:\roms\b.nes");
    store.games_mut().set_profile_for(r"D:\roms\b.nes", "42");
    store.save(&settings);
    assert!(root.paths.root.join("games.json").is_file());
    let mut reread = SettingsStore::new(&root.paths);
    reread.load(&mut SettingsModel::new(), at(0));
    assert_eq!(
        vec![r"D:\roms\b.nes", r"D:\roms\a.nes"],
        reread.games().recent_roms()
    );
    assert_eq!(Some("42"), reread.games().profile_for(r"D:\roms\b.nes"));
}

#[test]
fn the_game_list_coexists_with_the_library() {
    let root = root();
    std::fs::write(
        &root.paths.library_file,
        r#"{ "schemaVersion": 1, "games": [] }"#,
    )
    .unwrap();
    let mut settings = SettingsModel::new();
    let mut store = SettingsStore::new(&root.paths);
    store.load(&mut settings, at(0));
    store.games_mut().note_rom_opened(r"D:\roms\a.nes");
    store.save(&settings);
    assert!(std::fs::read_to_string(&root.paths.library_file)
        .unwrap()
        .contains("\"games\": []"));
    assert!(std::fs::read_to_string(root.paths.root.join("games.json"))
        .unwrap()
        .contains("recentRoms"));
}

#[test]
fn a_read_only_root_does_not_stop_the_program() {
    let root = root();
    // A plain file where the root folder should be: creating the layout fails, so the program
    // keeps its settings in memory and the writes are refused (REQ-PERS-073).
    let blocked = root.paths.root.join("blocked");
    std::fs::write(&blocked, "not a folder").unwrap();
    let paths = UserDataPaths::resolve(Some(&blocked), None, None);
    assert!(!paths.is_writable);
    assert_eq!(
        Some(format!(
            "User data folder is not writable: {}",
            blocked.display()
        )),
        paths.unwritable_message
    );
    let mut settings = SettingsModel::new();
    let mut store = SettingsStore::new(&paths);
    store.load(&mut settings, at(0));
    settings.set_fov(50.0);
    store.save(&settings);
    assert!((settings.fov() - 50.0).abs() < f32::EPSILON);
}

fn empty_tile(harness: &LoopHarness) -> bool {
    harness
        .host
        .session()
        .unwrap()
        .engine()
        .settings()
        .empty_tile
}

#[test]
fn launching_a_different_game_reapplies_the_profile_settings() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("a", Some(&rom));
    harness.catalog().add("b", Some(&rom));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("a"));
    // A script changes a profile setting during the game.
    harness.host.update_settings(|model| {
        model.set_layer(SettingId::ConvEmptyTile, true, SettingLayer::Session)
    });
    assert!(harness
        .host
        .settings()
        .get(SettingId::ConvEmptyTile)
        .as_bool());
    assert!(empty_tile(&harness));
    // Relaunching the same game keeps it (REQ-PERS-006).
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("a"));
    assert!(harness
        .host
        .settings()
        .get(SettingId::ConvEmptyTile)
        .as_bool());
    // A different game takes the profile's value back (REQ-PERS-005).
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("b"));
    assert!(!harness
        .host
        .settings()
        .get(SettingId::ConvEmptyTile)
        .as_bool());
    assert!(!empty_tile(&harness));
}

#[test]
fn a_game_load_forces_the_background_flag_and_resets_the_layer_shadows() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("a", Some(&rom));
    harness.host.update_settings(|model| {
        model.set_layer(SettingId::SceneUseNesBgColor, false, SettingLayer::Profile);
        for layer in Layer::ALL {
            model.set_layer(
                SettingId::LayerCastShadow(layer),
                true,
                SettingLayer::Profile,
            );
        }
    });
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("a"));
    let model = harness.host.settings();
    assert!(model.get(SettingId::SceneUseNesBgColor).as_bool());
    let casts: Vec<bool> = Layer::ALL
        .iter()
        .map(|layer| model.get(SettingId::LayerCastShadow(*layer)).as_bool())
        .collect();
    assert_eq!(vec![false, false, true, false, false, false], casts);
}

#[test]
fn a_write_reaches_the_scene_and_the_conversion_slice() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("a", Some(&rom));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("a"));
    harness.host.update_settings(|model| {
        model.set_layer(SettingId::ClipBottom, 16.1_f32, SettingLayer::Profile)
    });
    let clipping = harness.host.session().unwrap().engine().settings().clipping;
    assert!((clipping.y - 16.1).abs() < 1e-4);
    assert!((harness.host.scene().clipping.y - 16.1).abs() < 1e-4);
    harness.host.update_settings(|model| {
        model.set_layer(
            SettingId::LayerOffset(Layer::ALL[3]),
            24.0_f32,
            SettingLayer::Profile,
        );
    });
    let offset = harness
        .host
        .session()
        .unwrap()
        .engine()
        .settings()
        .layer(ZLayer::from_raw(3))
        .offset;
    assert!((offset - 24.0).abs() < f32::EPSILON);
    harness.host.update_settings(|model| model.set_fov(50.0));
    assert!((harness.host.scene().settings.fov - 50.0).abs() < f32::EPSILON);
    harness
        .host
        .update_settings(|model| model.set_light_intensity(1.4));
    assert!((harness.host.scene().settings.light_intensity() - 1.4).abs() < 1e-4);
}

#[test]
fn a_game_load_puts_both_ports_back_to_gamepad() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("a", Some(&rom));
    harness.host.update_settings(|model| {
        model.set(
            SettingId::InputPortType(Port::ALL[0]),
            SettingValue::Enum(1),
        );
    });
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("a"));
    let model = harness.host.settings();
    assert_eq!(
        0,
        model.get(SettingId::InputPortType(Port::ALL[0])).as_int()
    );
    assert_eq!(
        0,
        model.get(SettingId::InputPortType(Port::ALL[1])).as_int()
    );
}
