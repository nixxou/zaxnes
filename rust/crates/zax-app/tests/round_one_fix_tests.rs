//! Port of the non-interface tests of `RoundOneFixTests.cs`. The tests that drive the menu bar,
//! the dialogues and the font atlas belong to the interface; the deletion flows are checked here
//! through the host's pending confirmation, which is what the dialogue shows.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::{LoopHarness, SyntheticRom, TemporaryRoot};
use zax_app::audio::{AudioMixer, TrackRandom};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::emulation::host::{EmulationHost, EmulationHostOptions};
use zax_app::library_game_catalog::LibraryGameCatalog;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_app::persistence::game_art;
use zax_app::single_instance::SingleInstance;
use zax_app::user_data_paths::UserDataPaths;
use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::profile::GameProfile;
use zax_render::assets::png_image;
use zax_render::camera::CameraMode;
use zax_render::geometry as render_geometry;
use zax_render::numerics::Vector3;
use zax_render::scene::SceneState;
use zax_render::smoothing::CriticalSmoothing;

#[test]
fn req_scn_013_stopping_a_game_resets_the_view_and_returns_to_tpv() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    assert_eq!(EmulationState::Running, harness.host.state());
    {
        let scene = harness.host.scene_mut();
        scene.camera.set_mode(CameraMode::Fpv);
        scene.camera.current_mut().set_distance(42.0);
        scene.camera.current_mut().angles = Vector3::new(45.0, 45.0, 0.0);
        scene.camera.smooth_time = 2.0;
    }
    harness
        .host
        .update_settings(|model| model.set_game_speed(4));
    harness.run(Command::Stop);
    let scene = harness.host.scene();
    let camera = &scene.camera;
    assert_eq!(CameraMode::Tpv, camera.mode());
    assert!((camera.current().angles.x - render_geometry::DEFAULT_ANGLES.x).abs() < 1e-3);
    assert!((camera.current().angles.y - render_geometry::DEFAULT_ANGLES.y).abs() < 1e-3);
    assert!(
        (camera.current().distance() - render_geometry::default_distance(camera.fov())).abs()
            < 1e-3
    );
    assert!((camera.smooth_time - CriticalSmoothing::DEFAULT_SMOOTH_TIME).abs() < 1e-4);
    assert_eq!(0, harness.host.settings().game_speed());
}

#[test]
fn req_aud_022_the_menu_music_leaves_suspension_when_a_game_ends() {
    let mut harness = LoopHarness::create();
    let mixer = harness.host.mixer_mut().unwrap();
    mixer.set_menu_music_enabled(true);
    // In the menu the track plays; entering the game fades it out and suspends it.
    mixer.set_emulation_audible(true);
    mixer.advance(5.0);
    assert!(mixer.music_suspended());
    // Back to the menu: the track is playable again and its gain climbs back.
    mixer.set_emulation_audible(false);
    assert!(!mixer.music_suspended());
    mixer.advance(5.0);
    assert!(!mixer.music_suspended());
    assert!(mixer.music_gain() > 0.0);
}

fn empty_profile(id: &str, title: &str) -> GameProfile {
    let mut profile = GameProfile::create_empty();
    profile.id = Some(id.to_owned());
    profile.title = Some(title.to_owned());
    zax_profiles::export::normalize(&mut profile, None).unwrap();
    profile
}

struct LibraryHarness {
    _root: TemporaryRoot,
    paths: UserDataPaths,
    host: EmulationHost<LibraryGameCatalog>,
}

fn library_harness() -> LibraryHarness {
    let root = TemporaryRoot::new("zax-r1-");
    let paths = UserDataPaths::resolve(Some(root.path()), None, None);
    let profile_file = paths.profiles_directory.join("mine.3dn");
    std::fs::write(
        &profile_file,
        format::save(&empty_profile("mine", "Mine"), FieldPresence::All),
    )
    .unwrap();
    let catalog = LibraryGameCatalog::open(None, &paths, Box::new(|_| {}));
    let mut options = EmulationHostOptions::new(
        paths.clone(),
        zax_settings::model::SettingsModel::new(),
        SceneState::default(),
        catalog,
    );
    options.mixer = Some(AudioMixer::new(48_000, None, TrackRandom::new(1)));
    options.confirm_with_interface = true;
    options.capture_thumbnail = Some(Box::new(|| Some(vec![0x30; 256 * 192 * 3])));
    LibraryHarness {
        host: EmulationHost::new(options),
        paths,
        _root: root,
    }
}

impl LibraryHarness {
    fn settle(&mut self) {
        for _ in 0..64 {
            if !self.host.is_loading() {
                break;
            }
            self.host.tick_actions(
                0.001,
                zax_app::input::InputAction::NONE,
                zax_app::input::InputAction::NONE,
            );
        }
    }

    fn run(&mut self, command: Command, args: CommandArgs) {
        self.host.enqueue(command, args);
        self.host.apply_commands();
        self.settle();
    }

    fn launch_mine(&mut self) {
        let rom = self.paths.root.join("game.nes");
        std::fs::write(&rom, SyntheticRom::counter()).unwrap();
        self.host
            .launch_rom_with_profile(&rom.to_string_lossy(), "mine");
        self.settle();
        assert_eq!(EmulationState::Running, self.host.state());
    }
}

#[test]
fn req_pers_042_a_user_game_is_deleted_with_its_states_its_file_and_its_cover() {
    let mut harness = library_harness();
    let profile_file = harness.paths.profiles_directory.join("mine.3dn");
    harness.launch_mine();
    harness.run(Command::SaveSlot, CommandArgs::of_number(3));
    harness.run(Command::QuickSave, CommandArgs::default());
    assert!(harness.host.slots.find("3").unwrap().is_occupied());
    assert!(harness.host.slots.find("quick").unwrap().is_occupied());
    // A cover, so that its file can be shown to go too.
    let cover = game_art::cover_file(&harness.paths, "Mine", "mine");
    std::fs::create_dir_all(&harness.paths.covers_directory).unwrap();
    std::fs::write(&cover, png_image::encode_rgb(2, 2, &[0; 12]).unwrap()).unwrap();
    {
        use zax_app::emulation::game_catalog::GameCatalog as _;
        harness
            .host
            .games_mut()
            .set_cover("mine", Some(&cover.file_name().unwrap().to_string_lossy()));
    }
    harness.run(Command::DeleteGame, CommandArgs::of_text("mine"));
    // The interface asked first, with the exact text of SPEC-10 REQ-UI-037.
    let request = harness.host.pending_confirmation().unwrap().clone();
    assert_eq!(
        "Do you want to delete the game and its save states ?",
        request.message
    );
    assert_eq!("Mine", request.title);
    assert_eq!("Yes", request.confirm_label);
    assert_eq!("No", request.cancel_label);
    assert!(harness.host.confirm_pending());
    assert_eq!(EmulationState::Stopped, harness.host.state());
    assert!(harness.host.current_game_id().is_none());
    assert!(!harness.host.slots.find("3").unwrap().is_occupied());
    assert!(!harness.host.slots.find("quick").unwrap().is_occupied());
    assert!(!harness.paths.slot_state_file("3").exists());
    assert!(!harness.paths.slot_state_file("quick").exists());
    assert!(!profile_file.exists());
    assert!(!cover.exists());
    assert!(harness.host.games().library().find("mine").is_none());
}

#[test]
fn req_pers_042_a_built_in_profile_is_never_deletable_and_no_changes_nothing() {
    let mut harness = library_harness();
    let profile_file = harness.paths.profiles_directory.join("mine.3dn");
    let two_dimensional = {
        use zax_app::emulation::game_catalog::GameCatalog as _;
        harness.host.games().two_dimensional_mode_id().unwrap()
    };
    assert!(!harness.host.request_delete_game(&two_dimensional));
    assert!(!harness.host.delete_game(&two_dimensional));
    assert!(harness
        .host
        .games()
        .library()
        .find(&two_dimensional)
        .is_some());
    assert!(harness.host.request_delete_game("mine"));
    harness.host.cancel_pending();
    assert!(harness.host.games().library().find("mine").is_some());
    assert!(profile_file.is_file());
}

#[test]
fn req_ui_052_deleting_a_slot_asks_the_documented_question() {
    let mut harness = library_harness();
    harness.launch_mine();
    harness.run(Command::SaveSlot, CommandArgs::of_number(5));
    assert!(harness.host.slots.find("5").unwrap().is_occupied());
    harness.run(Command::DeleteSlot, CommandArgs::of_number(5));
    let request = harness.host.pending_confirmation().unwrap().clone();
    assert_eq!("Do you want to delete the slot 5 ?", request.message);
    assert_eq!("Mine", request.title);
    // "No" keeps the slot.
    harness.host.cancel_pending();
    assert!(harness.host.slots.find("5").unwrap().is_occupied());
    harness.run(Command::DeleteSlot, CommandArgs::of_number(5));
    assert!(harness.host.confirm_pending());
    assert!(!harness.host.slots.find("5").unwrap().is_occupied());
    assert!(!harness.paths.slot_state_file("5").exists());
}

#[test]
fn req_pers_036_the_pause_capture_can_become_the_cover() {
    let mut harness = library_harness();
    harness.launch_mine();
    harness.run(Command::CreateGameArt, CommandArgs::default());
    let cover = harness
        .host
        .games()
        .library()
        .find("mine")
        .unwrap()
        .cover
        .clone();
    assert_eq!(Some("Mine_3dsen_mine.png".to_owned()), cover);
    assert!(harness
        .paths
        .covers_directory
        .join(cover.unwrap())
        .is_file());
    assert!(harness.host.scene().game_running);
}

#[test]
fn req_pers_074_a_derived_cover_name_is_sanitised_and_carries_the_identifier() {
    assert_eq!(
        "A_B_C__3dsen_42.png",
        game_art::cover_file_name("A/B:C?", "42")
    );
    assert_eq!("A_B_3dsen_42.png", game_art::cover_file_name("A_B", "42"));
    assert!(!game_art::sanitise_file_name(r"a\b").contains('\\'));
    assert_eq!("a_b", game_art::sanitise_file_name(r"a\b"));
    assert_eq!("a_b_c", game_art::sanitise_file_name("a<b>c"));
    // The identifier is what distinguishes two games of the same title.
    assert_ne!(
        game_art::cover_file_name("Same", "1"),
        game_art::cover_file_name("Same", "2")
    );
}

#[test]
fn req_pers_036_an_imported_cover_is_reduced_to_fit_the_documented_size() {
    assert_eq!((192, 192), game_art::fitted_size(1024, 1024));
    assert_eq!((256, 48), game_art::fitted_size(1024, 192));
    assert_eq!((64, 48), game_art::fitted_size(64, 48));
    let root = TemporaryRoot::new("zax-art-");
    let paths = UserDataPaths::resolve(Some(root.path()), None, None);
    let source = root.path().join("art.png");
    std::fs::write(
        &source,
        png_image::encode_rgb(1024, 1024, &vec![0x40; 1024 * 1024 * 3]).unwrap(),
    )
    .unwrap();
    let name = game_art::import_cover(&paths, "A/B", "7", &source).unwrap();
    assert_eq!("A_B_3dsen_7.png", name);
    let written = png_image::load(&paths.covers_directory.join(&name)).unwrap();
    assert_eq!(192, written.width);
    assert_eq!(192, written.height);
    assert_eq!(0x40, written.pixel(10, 10).0);
    assert_eq!(255, written.pixel(10, 10).3);
}

#[test]
fn req_pers_036_an_undecodable_cover_is_refused() {
    let root = TemporaryRoot::new("zax-art-");
    let paths = UserDataPaths::resolve(Some(root.path()), None, None);
    let source = root.path().join("art.jpg");
    std::fs::write(&source, [0xFF, 0xD8, 0xFF, 0xE0, 0, 0]).unwrap();
    let error = game_art::import_cover(&paths, "T", "1", &source).unwrap_err();
    assert_eq!(game_art::UNSUPPORTED_SOURCE_MESSAGE, error);
    assert_eq!(
        0,
        std::fs::read_dir(&paths.covers_directory).unwrap().count()
    );
}

fn acquire_elsewhere(root: &std::path::Path) -> bool {
    let root = root.to_path_buf();
    std::thread::spawn(move || SingleInstance::acquire(Some(&root)).is_owner())
        .join()
        .unwrap()
}

#[test]
fn req_loop_054_a_second_instance_on_the_same_root_does_not_own_it() {
    let first = TemporaryRoot::new("zax-single-");
    let second = TemporaryRoot::new("zax-single-");
    let owner = SingleInstance::acquire(Some(first.path()));
    assert!(owner.is_owner());
    // A lock file is held per open file description, so a second launch is simulated on another
    // thread with its own handle: the real second launch is another process, which is the same
    // thing to the kernel. The guard dropped on that thread would release only its own handle.
    assert!(!acquire_elsewhere(first.path()));
    assert!(acquire_elsewhere(second.path()));
    // The name depends on the root alone, and is stable across spellings of it.
    let spelled = format!("{}{}", first.path().display(), std::path::MAIN_SEPARATOR);
    assert_eq!(
        SingleInstance::name_for(Some(first.path())),
        SingleInstance::name_for(Some(std::path::Path::new(&spelled)))
    );
    assert_ne!(
        SingleInstance::name_for(Some(first.path())),
        SingleInstance::name_for(Some(second.path()))
    );
    // Releasing the guard lets the next instance in.
    drop(owner);
    assert!(acquire_elsewhere(first.path()));
}
