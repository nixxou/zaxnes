//! AMD-051 REQ-LOOP-073: a load that stops advancing is reported and abandoned instead of
//! freezing the program.
//!
//! The acceptance criteria say "le harnais bloque l'étape 6 de REQ-LOOP-021 (attente des
//! maillages)". The seam that does it is `EmulationHost::load_progress_stall`, which holds the
//! loading progress of REQ-E3D-600 where the engine would report it; `LoopHarness::tick` drives
//! the real clock, so the ten seconds are exact and no test waits for them.

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::{LoopHarness, SyntheticRom};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::emulation::host::{LOADING_TIMED_OUT_MESSAGE, LOADING_TIMEOUT_SECONDS};
use zax_app::emulation::notification_center::NotificationSeverity;

/// A harness with a game, and a launch under way whose mesh wait is held at `progress`.
fn launching_with_progress(progress: f32) -> LoopHarness {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("g1", Some(&rom));
    harness.host.load_progress_stall = Some(progress);
    harness
        .host
        .enqueue(Command::LaunchGame, CommandArgs::of_text("g1"));
    harness.tick_frame();
    assert!(harness.host.is_loading(), "the launch has begun");
    harness
}

/// Ticks `seconds` of real time in sixtieths, as the loop does, and stops as soon as the loading
/// indicator clears. Returns the real time that elapsed.
fn tick_until_not_loading(harness: &mut LoopHarness, seconds: f64) -> f64 {
    let step = 1.0 / 60.0;
    let mut elapsed = 0.0;
    while elapsed < seconds && harness.host.is_loading() {
        harness.tick(step);
        elapsed += step;
    }
    elapsed
}

#[test]
fn req_loop_073_a_mesh_wait_that_stops_advancing_times_out_after_ten_seconds() {
    let mut harness = launching_with_progress(0.5);
    let elapsed = tick_until_not_loading(&mut harness, 15.0);

    assert!(
        !harness.host.is_loading(),
        "the loading indicator is down after {elapsed:.2} s"
    );
    assert!(
        (elapsed - LOADING_TIMEOUT_SECONDS).abs() <= 1.0,
        "the notification appears at {LOADING_TIMEOUT_SECONDS} s plus or minus one, not {elapsed:.2}"
    );

    // Point 2 of the requirement, clause by clause.
    assert_eq!(EmulationState::Stopped, harness.host.state());
    assert!(harness.host.current_game_id().is_none());
    let notification = harness.host.notifications.current().expect("a message");
    assert_eq!(LOADING_TIMED_OUT_MESSAGE, notification.text);
}

#[test]
fn req_loop_073_the_notification_is_an_error_so_it_is_never_dropped() {
    // Q-105: the gravity is the one the amendment's line of SPEC-07 §4 gives, not the "normale"
    // of its point 1. The difference is observable through REQ-UI-081, which drops a normal
    // notification while an error is on screen. If R0 answers "normale", this is the assertion
    // to invert.
    let mut harness = launching_with_progress(0.25);
    tick_until_not_loading(&mut harness, 15.0);
    let notification = harness.host.notifications.current().expect("a message");
    assert_eq!(NotificationSeverity::Error, notification.severity);
}

#[test]
fn req_loop_073_a_command_runs_within_a_second_of_the_timeout() {
    let mut harness = launching_with_progress(0.5);
    tick_until_not_loading(&mut harness, 15.0);
    assert!(!harness.host.is_loading());

    // The second criterion: the command model of AMD-010 REQ-UI-205 answers again at once.
    harness.host.load_progress_stall = None;
    harness
        .host
        .enqueue(Command::LaunchGame, CommandArgs::of_text("g1"));
    let mut ticked = 0.0;
    while ticked < 1.0 && harness.host.current_game_id().is_none() {
        harness.tick(1.0 / 60.0);
        ticked += 1.0 / 60.0;
    }
    assert_eq!(
        Some("g1"),
        harness.host.current_game_id(),
        "a command executes within the second that follows"
    );
}

#[test]
fn req_loop_073_a_wait_whose_progress_keeps_moving_is_never_interrupted() {
    // REQ-ACC-031 excludes the initial mesh build from every budget, and REQ-E3D-600 builds every
    // object of every pattern of a profile: a load that is advancing must survive any duration.
    // This is the point 51-N1 of R2-review-AMD-050-053-v1.1.
    let mut harness = launching_with_progress(0.0);
    let step = 1.0 / 60.0;
    let mut progress = 0.0f32;
    // Thirty seconds, three times the timeout, with the measure creeping forward.
    for frame in 0..1800 {
        if frame % 120 == 0 {
            progress += 0.001;
            harness.host.load_progress_stall = Some(progress);
        }
        harness.tick(step);
        assert!(
            harness.host.is_loading(),
            "a load that advances is never interrupted (frame {frame})"
        );
    }

    // And it still finishes when the meshes are there.
    harness.host.load_progress_stall = Some(1.0);
    tick_until_not_loading(&mut harness, 5.0);
    assert_eq!(EmulationState::Running, harness.host.state());
    assert_eq!(Some("g1"), harness.host.current_game_id());
}

#[test]
fn req_loop_073_an_ordinary_launch_is_untouched() {
    // The watchdog must not change a load that works: the stall is `None` and every stage
    // completes in one tick, so the clock is reset at every step.
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("g1", Some(&rom));
    harness.launch("g1");
    assert_eq!(EmulationState::Running, harness.host.state());
    assert!(harness.host.notifications.current().is_none());
}
