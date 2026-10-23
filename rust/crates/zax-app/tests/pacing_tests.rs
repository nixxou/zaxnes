//! Port of `PacingTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::LoopHarness;
use rstest::rstest;
use zax_app::emulation::fast_forward_state::{FastForwardState, BURST_FRAMES};
use zax_app::emulation::frame_pacer::{Clock, FramePacer, MAX_BACKLOG};
use zax_app::emulation::speed_setting;
use zax_app::input::InputAction;
use zax_app::persistence::app_settings::AppSettingsMut;

fn manual() -> (Rc<Cell<f64>>, Clock) {
    let now = Rc::new(Cell::new(0.0));
    let source = Rc::clone(&now);
    (now, Box::new(move || source.get()))
}

#[rstest]
#[case(-8, 1.0 / 3.0)]
#[case(-4, 0.5)]
#[case(-1, 0.8)]
#[case(0, 1.0)]
#[case(1, 1.25)]
#[case(2, 1.5)]
#[case(4, 2.0)]
#[case(8, 3.0)]
fn req_loop_015_the_speed_setting_maps_to_a_factor(#[case] setting: i32, #[case] factor: f64) {
    assert!((speed_setting::factor(setting) - factor).abs() < 1e-9);
}

#[test]
fn req_pers_003_the_speed_setting_is_clamped() {
    assert_eq!(-8, speed_setting::clamp(-9));
    assert_eq!(8, speed_setting::clamp(12));
    assert!((speed_setting::fast_forward_factor() - 1.5).abs() < 1e-9);
}

#[rstest]
#[case(60.0, 0, 10.0, 600)]
#[case(50.0, -4, 10.0, 250)]
#[case(60.0, 8, 1.0, 180)]
fn req_loop_010_the_clock_hands_out_the_nominal_rate(
    #[case] nominal: f64,
    #[case] setting: i32,
    #[case] seconds: f64,
    #[case] expected: u32,
) {
    let (now, clock) = manual();
    let mut pacer = FramePacer::new(nominal, clock);
    pacer.speed = speed_setting::factor(setting);
    let mut frames = 0;
    while now.get() < seconds {
        now.set(now.get() + 0.001);
        frames += pacer.due_frames();
    }
    assert!(((expected - 1)..=(expected + 1)).contains(&frames), "{frames}");
}

#[test]
fn req_loop_012_the_backlog_is_limited_to_five_frames() {
    let (now, clock) = manual();
    let mut pacer = FramePacer::new(60.0, clock);
    assert_eq!(1, pacer.due_frames());
    now.set(now.get() + 1.0);
    assert_eq!(MAX_BACKLOG, pacer.due_frames());
    assert!((50..=60).contains(&pacer.dropped_frames()), "{}", pacer.dropped_frames());
}

#[test]
fn req_loop_013_resetting_the_clock_drops_the_backlog() {
    let (now, clock) = manual();
    let mut pacer = FramePacer::new(60.0, clock);
    pacer.due_frames();
    now.set(now.get() + 30.0);
    pacer.reset();
    // Frame 0 is due at t0 itself (REQ-LOOP-011), so one frame is handed out and the thirty
    // seconds of backlog are gone.
    assert_eq!(1, pacer.due_frames());
    let mut frames = 1;
    let limit = now.get() + 1.0;
    while now.get() < limit {
        now.set(now.get() + 0.001);
        frames += pacer.due_frames();
    }
    assert!((59..=61).contains(&frames), "{frames}");
}

#[test]
fn req_loop_016_one_request_arms_thirty_frames() {
    let mut fast = FastForwardState::new();
    assert!(fast.request());
    assert_eq!(BURST_FRAMES, fast.remaining());
    // A second request during the burst is ignored.
    assert!(!fast.request());
    assert_eq!(BURST_FRAMES, fast.remaining());
    for _ in 0..BURST_FRAMES {
        assert!(fast.is_active());
        fast.on_frame_run();
    }
    assert!(!fast.is_active());
    assert!(fast.request());
}

#[test]
fn req_loop_016_holding_the_hotkey_keeps_the_rate_at_one_and_a_half() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    let before = harness.host.emulated_frames();
    let mut frames = 0;
    for _ in 0..90 {
        // 90 rendered frames of 1/90 s: one second of real time, the hotkey held.
        frames += harness.tick_actions(1.0 / 90.0, InputAction::FAST_FORWARD, InputAction::NONE);
    }
    let ran = harness.host.emulated_frames() - before;
    assert_eq!(i64::from(frames), ran);
    assert!((81..=99).contains(&ran), "{ran}");
    // REQ-LOOP-016: the notification is shown while the key is held.
    let current = harness.host.notifications.current().unwrap();
    assert_eq!("Fast Forward", current.text);
    assert!((current.seconds - 1.0).abs() < f64::EPSILON);
    // On release, between 0 and 30 frames of the burst remain.
    assert!(harness.host.fast_forward.remaining() <= BURST_FRAMES);
}

#[test]
fn req_loop_015_a_speed_change_takes_effect_on_the_next_frame() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    let frames = harness.host.session().unwrap().core().frame_count();
    harness.host.update_settings(|model| model.set_game_speed(4));
    harness.tick_frame();
    assert!((harness.host.pacer.speed - 2.0).abs() < 1e-9);
    let session = harness.host.session().unwrap();
    assert!((session.core().speed() - 2.0).abs() < 1e-9);
    assert!(session.core().frame_count() >= frames);
}

#[test]
fn req_loop_011_the_interval_between_frames_stays_close_to_the_period() {
    let (now, clock) = manual();
    let mut pacer = FramePacer::new(60.0, clock);
    pacer.due_frames();
    let mut intervals = Vec::new();
    let mut last = now.get();
    while intervals.len() < 300 {
        now.set(now.get() + 0.001);
        for _ in 0..pacer.due_frames() {
            intervals.push(now.get() - last);
            last = now.get();
        }
    }
    let inside = intervals
        .iter()
        .filter(|interval| (0.0146..=0.0186).contains(*interval))
        .count();
    assert!(inside >= intervals.len() * 95 / 100, "only {inside}/{} inside", intervals.len());
}
