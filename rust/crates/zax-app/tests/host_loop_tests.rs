//! Port of the non-command-line tests of `HostLoopTests.cs`. The `AppOptions` parsing tests
//! belong to the command-line front end and the `RenderSettings` test to `zax-render`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

use std::time::{Duration, Instant};

use zax_app::emulation::frame_pacer::{system_clock, FramePacer, MAX_BACKLOG};
use zax_app::input::press_tracker::{PressKind, PressTracker};
use zax_app::render_skip;

fn block(seconds: f64) {
    let start = Instant::now();
    while start.elapsed().as_secs_f64() < seconds {
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn req_loop_012_the_backlog_never_exceeds_five_emulated_frames() {
    let mut pacer = FramePacer::new(60.0, system_clock());
    // Frame 0 is due at t0 itself (REQ-LOOP-011), so the first call hands out one frame.
    assert_eq!(1, pacer.due_frames());
    // Block the loop for half a second: at most five frames are run in a burst and the rest of
    // the backlog is abandoned.
    block(0.5);
    let due = pacer.due_frames();
    assert_eq!(MAX_BACKLOG, due);
    assert!(pacer.dropped_frames() > 20, "only {} frames were dropped", pacer.dropped_frames());
}

#[test]
fn req_loop_013_resetting_the_clock_drops_the_backlog() {
    let mut pacer = FramePacer::new(60.0, system_clock());
    block(0.2);
    pacer.reset();
    // The twelve frames the block made late are gone: only the frame due at t0 is handed out, so
    // the resume shows no burst.
    assert_eq!(1, pacer.due_frames());
}

#[test]
fn req_loop_015_the_speed_factor_multiplies_the_nominal_rate() {
    let mut pacer = FramePacer::new(60.0, system_clock());
    pacer.speed = 1.5;
    assert!((pacer.effective_rate() - 90.0).abs() < 1e-9);
    pacer.speed = 0.5;
    assert!((pacer.effective_rate() - 30.0).abs() < 1e-9);
}

#[test]
fn req_loop_010_the_clock_hands_out_about_the_nominal_rate() {
    let mut pacer = FramePacer::new(60.0, system_clock());
    let start = Instant::now();
    let mut total = 0;
    while start.elapsed().as_secs_f64() < 0.5 {
        total += pacer.due_frames();
        std::thread::sleep(Duration::from_millis(1));
    }
    // 0.5 s at 60 frames/s, plus the frame due at t0, with a margin for a loaded machine.
    assert!((25..=34).contains(&total), "{total}");
}

#[test]
fn section_2_a_press_released_under_0_3_s_is_short_and_a_longer_one_is_long() {
    let mut tracker = PressTracker::new();
    assert_eq!(PressKind::None, tracker.update(true, 0.1));
    assert_eq!(PressKind::Short, tracker.update(false, 0.0));
    assert_eq!(PressKind::None, tracker.update(true, 0.1));
    assert_eq!(PressKind::None, tracker.update(true, 0.1));
    assert_eq!(PressKind::Long, tracker.update(true, 0.2));
    // A long press is reported once, and the release that follows is not a short press.
    assert_eq!(PressKind::None, tracker.update(true, 0.2));
    assert_eq!(PressKind::None, tracker.update(false, 0.0));
}

#[test]
fn req_scn_060_the_scene_is_only_skipped_when_no_emulated_frame_arrived() {
    // Enabled, running, nothing new, same target: the previous image is kept.
    assert!(render_skip::should_skip(true, true, 0, true));
    // A new emulated frame, a stopped game, a resize or the setting off all force a draw.
    assert!(!render_skip::should_skip(true, true, 1, true));
    assert!(!render_skip::should_skip(true, false, 0, true));
    assert!(!render_skip::should_skip(true, true, 0, false));
    assert!(!render_skip::should_skip(false, true, 0, true));
}
