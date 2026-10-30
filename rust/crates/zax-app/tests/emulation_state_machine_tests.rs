//! Port of `EmulationStateMachineTests.cs`.

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::{LoopHarness, SyntheticRom, START_UNIX_SECONDS};
use time::OffsetDateTime;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::emulation::notification_center::NotificationSeverity;
use zax_app::emulation::rom_file::INVALID_ROM_MESSAGE;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut, TvRegionSetting};
use zax_app::persistence::save_slots::QUICK_SLOT_ID;
use zax_app::persistence::settings_store::SettingsStore;
use zax_nes::NesRegion;
use zax_settings::model::SettingsModel;

fn launch(harness: &mut LoopHarness, game_id: &str) {
    harness.launch(game_id);
    assert_eq!(EmulationState::Running, harness.host.state());
}

#[test]
fn req_loop_001_the_host_starts_stopped_with_no_current_game() {
    let harness = LoopHarness::create();
    assert_eq!(EmulationState::Stopped, harness.host.state());
    assert!(!harness.host.is_loading());
    assert!(harness.host.current_game_id().is_none());
    assert!(harness.host.session().is_none());
}

#[test]
fn req_loop_021_a_launch_runs_no_frame_before_running() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("g1", Some(&rom));
    harness
        .host
        .enqueue(Command::LaunchGame, CommandArgs::of_text("g1"));
    harness.tick_frame();
    assert!(harness.host.is_loading());
    let mut seen = Vec::new();
    while harness.host.is_loading() {
        harness.tick(1.0 / 1000.0);
        seen.push(harness.host.state());
    }
    assert!(seen.contains(&EmulationState::Paused));
    assert_eq!(EmulationState::Running, harness.host.state());
    assert_eq!(Some("g1"), harness.host.current_game_id());
    assert_eq!(
        0,
        harness
            .host
            .session()
            .expect("a session")
            .core()
            .frame_count()
    );
    assert_eq!(0, harness.host.emulated_frames());
    // REQ-PERS-033: the launch date is stamped on the entry.
    assert_eq!(
        vec![("g1".to_owned(), START_UNIX_SECONDS)],
        harness.catalog().played
    );
}

#[test]
fn req_loop_004_pause_and_resume_only_act_in_their_own_state() {
    let mut harness = LoopHarness::create();
    // Stopped: both are inert.
    harness.run(Command::Resume);
    assert_eq!(EmulationState::Stopped, harness.host.state());
    harness.run(Command::Pause);
    assert_eq!(EmulationState::Stopped, harness.host.state());
    launch(&mut harness, "g1");
    assert_eq!(EmulationState::Running, harness.host.state());
    harness.run(Command::PauseResume);
    assert_eq!(EmulationState::Paused, harness.host.state());
    // A second Pause in Paused changes nothing and takes no new capture.
    harness.set_thumbnail(Some(vec![1, 2, 3]));
    harness.run(Command::Pause);
    assert_eq!(EmulationState::Paused, harness.host.state());
    harness.run(Command::PauseResume);
    assert_eq!(EmulationState::Running, harness.host.state());
}

#[test]
fn req_loop_006_pausing_captures_the_game_view() {
    let mut harness = LoopHarness::create();
    launch(&mut harness, "g1");
    let mut capture = vec![0_u8; 256 * 192 * 3];
    capture[0] = 0x7F;
    harness.set_thumbnail(Some(capture));
    harness.run(Command::Pause);
    assert_eq!(EmulationState::Paused, harness.host.state());
    assert_eq!(0x7F, harness.host.pause_thumbnail().expect("a capture")[0]);
}

#[test]
fn req_loop_007_commands_are_deferred_during_a_load() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("g1", Some(&rom));
    harness
        .host
        .enqueue(Command::LaunchGame, CommandArgs::of_text("g1"));
    harness.tick_frame();
    assert!(harness.host.is_loading());
    // A quick save asked for in the middle of the load.
    harness
        .host
        .enqueue(Command::QuickSave, CommandArgs::default());
    assert_eq!(1, harness.host.pending_commands());
    harness.tick(1.0 / 1000.0);
    assert_eq!(1, harness.host.pending_commands());
    harness.settle();
    assert!(!harness.host.is_loading());
    harness.tick(1.0 / 1000.0);
    assert_eq!(0, harness.host.pending_commands());
    // The save happened, and it holds the post-load state.
    let quick = harness
        .host
        .slots
        .find(QUICK_SLOT_ID)
        .expect("the quick slot");
    assert!(quick.is_occupied());
    assert_eq!("g1", quick.game_id);
    assert_eq!(EmulationState::Running, harness.host.state());
}

#[test]
fn req_loop_023_an_invalid_rom_reports_the_exact_message() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(b"this is not a ROM", "fake.nes");
    harness.catalog().add("bad", Some(&rom));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("bad"));
    assert_eq!(EmulationState::Stopped, harness.host.state());
    assert!(harness.host.current_game_id().is_none());
    assert!(!harness.host.is_loading());
    let current = harness
        .host
        .notifications
        .current()
        .expect("a notification");
    assert_eq!(INVALID_ROM_MESSAGE, current.text);
    assert_eq!(NotificationSeverity::Error, current.severity);
}

#[test]
fn req_loop_018_a_failed_open_keeps_the_running_game() {
    let mut harness = LoopHarness::create();
    launch(&mut harness, "good");
    harness.run_frames(30);
    let session = harness.host.session().expect("a session");
    let frames = session.core().frame_count();
    let counter = session.core().read_memory(0x0010);
    assert!(frames > 0);
    let truncated = harness.write_rom(&[0x4E, 0x45, 0x53, 0x1A, 0x02], "truncated.nes");
    harness.catalog().add("bad", Some(&truncated));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("bad"));
    assert_eq!(EmulationState::Running, harness.host.state());
    assert_eq!(Some("good"), harness.host.current_game_id());
    let session = harness.host.session().expect("a session");
    assert_eq!(frames, session.core().frame_count());
    assert_eq!(counter, session.core().read_memory(0x0010));
    assert_eq!(
        INVALID_ROM_MESSAGE,
        harness
            .host
            .notifications
            .current()
            .expect("a notification")
            .text
    );
}

#[test]
fn req_loop_050_a_reset_cold_starts_the_core() {
    let mut harness = LoopHarness::create();
    launch(&mut harness, "g1");
    harness.run_frames(40);
    let session = harness.host.session().expect("a session");
    assert!(session.core().frame_count() > 0);
    let before = std::ptr::from_ref(session.runtime().expect("a runtime")) as usize;
    harness.run(Command::Reset);
    assert_eq!(EmulationState::Running, harness.host.state());
    let session = harness.host.session().expect("a session");
    assert_eq!(0, session.core().frame_count());
    // The profile, and therefore the Lua state, was not reloaded (REQ-LOOP-021 note).
    assert_eq!(
        before,
        std::ptr::from_ref(session.runtime().expect("a runtime")) as usize
    );
}

#[test]
fn req_loop_051_stopping_clears_the_current_game() {
    let mut harness = LoopHarness::create();
    launch(&mut harness, "g1");
    harness
        .host
        .update_settings(|model| model.set_game_speed(4));
    harness.run(Command::Stop);
    assert_eq!(EmulationState::Stopped, harness.host.state());
    assert!(harness.host.current_game_id().is_none());
    assert!(harness.host.session().is_none());
    assert_eq!(0, harness.host.settings().game_speed());
}

#[test]
fn req_loop_053_only_an_explicit_suspension_pauses() {
    let mut harness = LoopHarness::create();
    launch(&mut harness, "g1");
    harness.host.suspend();
    assert_eq!(EmulationState::Paused, harness.host.state());
    harness.host.resume_from_suspend();
    harness.tick_frame();
    assert_eq!(EmulationState::Running, harness.host.state());
    // Suspended while already paused: the resume must not start the game.
    harness.run(Command::Pause);
    harness.host.suspend();
    harness.host.resume_from_suspend();
    assert_eq!(EmulationState::Paused, harness.host.state());
}

#[test]
fn req_loop_052_the_exit_persists_settings_and_slots() {
    let mut harness = LoopHarness::create_with(|options| {
        options.settings_store = Some(SettingsStore::new(&options.paths));
    });
    launch(&mut harness, "g1");
    harness.host.update_settings(|model| {
        model.set_volume(0.25);
        model.set_auto_tv_mode(true);
    });
    let rom = harness.host.rom_file_name("g1");
    harness.run(Command::Quit);
    assert!(harness.host.quit_requested());
    harness.host.shutdown();
    assert!(harness.paths.settings_file.is_file());
    // Owner request: a ROM's states are its files; there is no index of them to write.
    assert!(!harness.paths.slot_state_file(&rom, QUICK_SLOT_ID).exists());
    let mut reread = SettingsModel::new();
    let now = OffsetDateTime::from_unix_timestamp(harness.now()).expect("a valid instant");
    SettingsStore::new(&harness.paths).load(&mut reread, now);
    assert!((reread.volume() - 0.25).abs() < 1e-6);
    assert!(reread.auto_tv_mode());
}

#[test]
fn req_loop_017_automatic_region_follows_the_rom_header() {
    let mut harness = LoopHarness::create();
    // Byte 9 bit 0 = 1 marks PAL in the iNES header.
    let mut image = SyntheticRom::counter();
    image[9] = 0x01;
    let rom = harness.write_rom(&image, "pal.nes");
    harness.catalog().add("pal", Some(&rom));
    harness
        .host
        .update_settings(|model| model.set_auto_tv_mode(true));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("pal"));
    assert_eq!(TvRegionSetting::Pal, harness.host.settings().tv_mode());
    assert_eq!(
        NesRegion::Pal,
        harness.host.session().expect("a session").core().region()
    );
    assert!((harness.host.pacer.nominal_rate - 50.0).abs() < f64::EPSILON);
    harness
        .host
        .update_settings(|model| model.set_tv_mode(TvRegionSetting::Ntsc));
    assert_eq!(
        NesRegion::Ntsc,
        harness.host.session().expect("a session").core().region()
    );
    harness.tick_frame();
    assert!((harness.host.pacer.nominal_rate - 60.0).abs() < f64::EPSILON);
}

#[test]
fn req_loop_024_a_mesh_algorithm_change_keeps_the_state_and_the_core() {
    let mut harness = LoopHarness::create();
    launch(&mut harness, "g1");
    harness.run_frames(60);
    harness.run(Command::Pause);
    let frames = harness
        .host
        .session()
        .expect("a session")
        .core()
        .frame_count();
    harness
        .host
        .change_mesh_algorithm(zax_engine3d::settings::RenderAlgorithm::Marching);
    assert!(harness.host.is_loading());
    harness.settle();
    assert!(!harness.host.is_loading());
    assert_eq!(EmulationState::Paused, harness.host.state());
    assert_eq!(
        frames,
        harness
            .host
            .session()
            .expect("a session")
            .core()
            .frame_count()
    );
}
