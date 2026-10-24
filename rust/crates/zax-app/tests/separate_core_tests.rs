//! "Emulation on a separate core" (Rust-only, handheld default): each frame runs on the
//! emulation thread while the previous one is drawn, and the game must come out exactly as on
//! one thread for the same inputs.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::{LoopHarness, SyntheticRom};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::persistence::app_settings::AppSettings as _;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;

/// What a run leaves: emulated frames, the counter the ROM increments, the picture.
fn run(separate_core: bool, ticks: usize) -> (i64, u8, Vec<u8>) {
    run_with(separate_core, true, ticks)
}

fn run_with(separate_core: bool, ahead: bool, ticks: usize) -> (i64, u8, Vec<u8>) {
    let mut harness = LoopHarness::create();
    harness.host.separate_core = separate_core;
    harness.host.core_ahead = ahead;
    harness.launch("g1");
    for _ in 0..ticks {
        harness.tick_frame();
    }
    harness.host.complete_frame_in_flight();
    let core = harness.host.session().expect("a session").core();
    (
        core.frame_count(),
        core.read_memory(0x0010),
        core.pixel_frame().to_vec(),
    )
}

#[test]
fn the_game_runs_the_same_frames_on_the_emulation_thread() {
    let alone = run(false, 150);
    let threaded = run(true, 150);
    assert!(alone.0 > 100, "{}", alone.0);
    assert_eq!(alone, threaded);
}

#[test]
fn the_frames_are_the_same_with_and_without_emulation_ahead() {
    // The test profile has no script, so it may run ahead.
    let serial = run_with(true, false, 150);
    let ahead = run_with(true, true, 150);
    assert_eq!(run(false, 150), serial);
    assert_eq!(serial, ahead);
}

#[test]
fn the_core_is_away_during_the_drawing_and_back_at_the_next_tick() {
    let mut harness = LoopHarness::create();
    harness.host.separate_core = true;
    harness.launch("g1");
    for _ in 0..30 {
        harness.tick_frame();
    }
    // Between two ticks the session holds the stand-in core; the picture of the last finished
    // frame is still there for the 2D mode.
    let session = harness.host.session_mut().expect("a session");
    assert_eq!(0, session.core().frame_count());
    let mut picture = Vec::new();
    session.copy_nes_picture(&mut picture);
    assert_eq!(256 * 240, picture.len());
    harness.host.complete_frame_in_flight();
    assert!(harness.host.session().unwrap().core().frame_count() > 20);
}

#[test]
fn the_shutdown_takes_the_core_back_first() {
    let mut harness = LoopHarness::create();
    harness.host.separate_core = true;
    harness.launch("g1");
    for _ in 0..10 {
        harness.tick_frame();
    }
    harness.host.shutdown();
    assert!(harness.host.session().is_none());
}

/// The camera's field of view after each tick, set by a script from the RAM byte the ROM keeps
/// incrementing: what the scripts read of the core, frame after frame.
fn script_reads(ahead: bool, ticks: usize) -> Vec<u32> {
    let mut harness = LoopHarness::create();
    harness.host.separate_core = true;
    harness.host.core_ahead = ahead;
    let mut profile = GameProfile::create_empty();
    profile.scripts.push(ProfileScript {
        name: Some("reader".to_owned()),
        enabled: true,
        source: Some(
            "function LateUpdate()
  Setting.Fov = 45 + Script:ReadMem(0x10) % 40
end
"
            .to_owned(),
        ),
        ..ProfileScript::default()
    });
    let rom = harness.write_rom(&SyntheticRom::counter(), "g1.nes");
    harness
        .catalog()
        .add_with("g1", Some(&rom), Some(profile), false, None);
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("g1"));
    (0..ticks)
        .map(|_| {
            harness.tick_frame();
            harness.host.settings().fov().to_bits()
        })
        .collect()
}

#[test]
fn a_script_reading_memory_sees_the_same_bytes_ahead() {
    let serial = script_reads(false, 120);
    let ahead = script_reads(true, 120);
    // The ROM moves the byte all the time, so the values do change from frame to frame.
    assert!(
        serial.windows(2).any(|pair| pair[0] != pair[1]),
        "{serial:?}"
    );
    assert_eq!(serial, ahead);
}
