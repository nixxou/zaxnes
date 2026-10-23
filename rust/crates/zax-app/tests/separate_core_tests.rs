//! "Emulation on a separate core" (Rust-only, handheld default): each frame runs on the
//! emulation thread while the previous one is drawn, and the game must come out exactly as on
//! one thread for the same inputs.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::LoopHarness;

/// What a run leaves: emulated frames, the counter the ROM increments, the picture.
fn run(separate_core: bool, ticks: usize) -> (i64, u8, Vec<u8>) {
    let mut harness = LoopHarness::create();
    harness.host.separate_core = separate_core;
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
