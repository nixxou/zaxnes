//! AMD-052: the catch-up limit of REQ-LOOP-074, the clock reset on a speed change of
//! REQ-LOOP-075, and the part of the GPU frame queue of REQ-LOOP-077 that lives outside OpenGL —
//! the setting itself and its effect on the emulated cadence. Port of `Amd052CadenceTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::LoopHarness;
use zax_app::emulation::commands::Command;
use zax_app::emulation::frame_pacer::{Clock, FramePacer};
use zax_app::input::input_action::InputAction;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::{SettingEffect, SettingKind, SettingScope};
use zax_settings::values::SettingValue;

fn manual() -> (Rc<Cell<f64>>, Clock) {
    let now = Rc::new(Cell::new(0.0));
    let source = Rc::clone(&now);
    (now, Box::new(move || source.get()))
}

/// REQ-LOOP-074, amended criterion of REQ-LOOP-012: a 500 ms block at 60 Hz runs at most two
/// frames in a burst, then the normal cadence resumes, and the frame counter is about 28 frames
/// short of an unblocked run.
#[test]
fn req_loop_074_a_five_hundred_millisecond_block_costs_about_twenty_eight_frames() {
    // The reference run: one second of 1 ms steps, nothing in the way.
    let (free, clock) = manual();
    let mut plain = FramePacer::new(60.0, clock);
    let mut unblocked = 0;
    while free.get() < 1.0 - 1e-9 {
        free.set(free.get() + 0.001);
        unblocked += plain.due_frames();
    }

    // The same second, with the loop stuck for 500 ms in the middle of it.
    let (now, clock) = manual();
    let mut pacer = FramePacer::new(60.0, clock);
    let mut ran = 0;
    while now.get() < 0.25 - 1e-9 {
        now.set(now.get() + 0.001);
        ran += pacer.due_frames();
    }

    now.set(now.get() + 0.5);
    let burst = pacer.due_frames();
    ran += burst;

    // Then the normal cadence resumes: one frame per period, no second burst.
    while now.get() < 1.0 - 1e-9 {
        now.set(now.get() + 0.001);
        let due = pacer.due_frames();
        assert!(due <= 1, "a second burst of {due} frames");
        ran += due;
    }

    assert_eq!(2, burst);
    assert!(
        (27..=29).contains(&(unblocked - ran)),
        "{} frames lost",
        unblocked - ran
    );
}

/// REQ-LOOP-074, the amended vector T5: a one-second block in NTSC catches at most 2 frames up
/// and loses about 58.
#[test]
fn req_loop_074_the_t5_vector_loses_about_fifty_eight_frames() {
    let (now, clock) = manual();
    let mut pacer = FramePacer::new(60.0, clock);
    pacer.due_frames();

    now.set(1.0);
    let caught = pacer.due_frames();

    assert_eq!(2, caught);
    assert!(
        (57..=59).contains(&pacer.dropped_frames()),
        "{} frames dropped",
        pacer.dropped_frames()
    );
}

/// REQ-LOOP-075, first criterion: going from the normal speed to x2 and back produces no burst
/// of catch-up frames, because each change of `emu.game_speed` resets the cadence clock
/// (REQ-LOOP-013 with the line AMD-052 §3 point 3 adds to it).
#[test]
fn req_loop_075_a_speed_change_resets_the_clock_and_produces_no_burst() {
    // What the reset buys, on the clock alone: half a second of lateness is abandoned at the
    // change instead of being run as a burst at the new period. Without the reset the same call
    // would hand out the two frames of REQ-LOOP-074 and abandon 28.
    let (now, clock) = manual();
    let mut pacer = FramePacer::new(60.0, clock);
    pacer.due_frames();
    now.set(now.get() + 0.5);

    pacer.speed = 2.0;
    pacer.reset();

    assert_eq!(1, pacer.due_frames());
    assert_eq!(0, pacer.dropped_frames());

    // And in the loop: every change of `emu.game_speed` resets the clock — the ±1 shortcuts of
    // REQ-LOOP-015 included, since they write the same setting — and no tick that follows runs a
    // catch-up burst.
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.tick(1.0 / 60.0);

    let resets = harness.host.pacer.clock_resets();

    harness
        .host
        .update_settings(|model| model.set_game_speed(4));
    harness.host.dispatch_setting_changes();
    assert!((harness.host.pacer.speed - 2.0).abs() < 1e-9);
    assert_eq!(resets + 1, harness.host.pacer.clock_resets());
    assert_no_burst(&mut harness, 1.0 / 120.0);

    harness.run(Command::SpeedDown);
    assert_eq!(3, harness.host.settings().game_speed());
    assert_eq!(resets + 2, harness.host.pacer.clock_resets());

    harness
        .host
        .update_settings(|model| model.set_game_speed(0));
    harness.host.dispatch_setting_changes();
    assert!((harness.host.pacer.speed - 1.0).abs() < 1e-9);
    assert_eq!(resets + 3, harness.host.pacer.clock_resets());
    assert_no_burst(&mut harness, 1.0 / 60.0);
}

/// REQ-LOOP-075, second criterion: NTSC, `s = 0`, the fast-forward hotkey held for 5 s. The
/// clock-reset counter stays where it was for the whole hold, the criterion of REQ-LOOP-016
/// still holds (about 450 emulated frames ±5 %), and the gap between two consecutive emulated
/// frames stays inside `1/f ± 2 ms` with `f = 90` while the key is held and with `f = 60` after
/// it is released (statement of REQ-LOOP-011).
///
/// Arming, expiring and re-arming a burst happens about six times a second — a burst of 30
/// emulated frames at x1.5 lasts about 0.33 s — which is why REQ-LOOP-075 excludes the cycle of
/// REQ-LOOP-016 from the resets of REQ-LOOP-013.
#[test]
fn req_loop_075_holding_the_fast_forward_hotkey_never_resets_the_clock() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");

    let resets = harness.host.pacer.clock_resets();
    let (held, frames) = intervals(&mut harness, 5.0, InputAction::FAST_FORWARD);

    // The hold itself: no clock reset, and the about 450 frames of REQ-LOOP-016 ±5 %.
    assert_eq!(resets, harness.host.pacer.clock_resets());
    assert!((427..=473).contains(&frames), "{frames} emulated frames");
    assert_intervals_near(&held, 1.0 / 90.0);

    // On release, between 0 and 30 emulated frames of the last burst remain (REQ-LOOP-016);
    // they are drained before the cadence at f = 60 is measured.
    intervals(&mut harness, 0.5, InputAction::NONE);
    let (released, _) = intervals(&mut harness, 2.0, InputAction::NONE);

    assert_eq!(resets, harness.host.pacer.clock_resets());
    assert_intervals_near(&released, 1.0 / 60.0);
}

/// REQ-LOOP-077: the setting of SPEC-09 §4.1.1 AMD-052 §3 point 7 adds — a whole number, 1 or 2,
/// default 2, scope U, immediate — and nothing outside the domain gets through.
#[test]
fn req_loop_077_the_queued_frame_setting_is_a_user_preference_of_one_or_two() {
    let descriptor = catalogue::descriptor(SettingId::VideoMaxQueuedFrames);

    assert_eq!("video.max_queued_frames", descriptor.key());
    assert_eq!(SettingKind::Int, descriptor.kind());
    assert_eq!(SettingScope::User, descriptor.scope());
    assert_eq!(SettingEffect::Immediate, descriptor.effect());
    assert_eq!(&SettingValue::Int(2), descriptor.default_value());
    assert!(descriptor.is_persisted());

    // The presentation chain belongs to the host in libretro (REQ-PERS-086), so the key is not
    // a core option — AMD-052 §3 point 8 lists it beside `video.vsync`.
    assert!(!descriptor.is_libretro_exposed());
    assert_eq!(None, descriptor.profile_field());

    let mut model = zax_settings::model::SettingsModel::new();
    assert_eq!(2, model.max_queued_frames());

    model.set_max_queued_frames(1);
    assert_eq!(1, model.max_queued_frames());

    // Out of domain falls back to the default, as REQ-PERS-003 asks of an enumerated domain.
    model.set(SettingId::VideoMaxQueuedFrames, SettingValue::Int(7));
    assert_eq!(2, model.max_queued_frames());
}

/// REQ-LOOP-077, last criterion: at `video.max_queued_frames` = 1, NTSC, `k = 1`, ten seconds
/// still hand out 600 ± 1 emulated frames — the criterion of REQ-LOOP-010 is unchanged —
/// including when the rendering is deliberately slowed down, here to 30 rendered frames a
/// second. The wait on the GPU is not a delay in the sense of REQ-LOOP-074: the frame clock is
/// absolute, so no frame is abandoned as long as the backlog stays inside the limit of two.
#[test]
fn req_loop_077_a_slowed_render_does_not_move_the_emulated_cadence() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness
        .host
        .update_settings(|model| model.set_max_queued_frames(1));

    let resets = harness.host.pacer.clock_resets();
    let before = harness.host.pacer.frame_count();

    // 300 rendered frames of 1/30 s: ten seconds of real time with the display path half as
    // fast as the emulation.
    for _ in 0..300 {
        harness.tick(1.0 / 30.0);
    }

    let ran = harness.host.pacer.frame_count() - before;

    assert!((599..=601).contains(&ran), "{ran} emulated frames");
    assert_eq!(resets, harness.host.pacer.clock_resets());

    // The 1 ms early start of REQ-LOOP-011 can round one frame of the ten seconds onto the
    // wrong side of a rendered frame; the limit of REQ-LOOP-074 is never reached.
    // Q-087
    assert!(
        harness.host.pacer.dropped_frames() <= 1,
        "{} frames dropped",
        harness.host.pacer.dropped_frames()
    );
}

/// Asserts that half a second of ticks at half `period` never runs more than one emulated frame
/// at a time and abandons nothing: no catch-up burst.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 60 to 120 ticks.
fn assert_no_burst(harness: &mut LoopHarness, period: f64) {
    let dropped = harness.host.pacer.dropped_frames();
    for _ in 0..(1.0 / period).round() as u32 {
        let due = harness.tick(period / 2.0);
        assert!(due <= 1, "a burst of {due} frames");
    }
    assert_eq!(dropped, harness.host.pacer.dropped_frames());
}

/// Runs `seconds` of real time as 1 ms rendered frames — fine enough to time the emulated frames
/// one by one — and returns the gaps between consecutive emulated frames, with their count.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // At most 5000 ticks.
fn intervals(harness: &mut LoopHarness, seconds: f64, actions: InputAction) -> (Vec<f64>, u32) {
    const STEP: f64 = 1.0 / 1000.0;
    let mut gaps = Vec::new();
    let mut last = harness.clock();
    let mut frames = 0;

    for _ in 0..(seconds / STEP).round() as u32 {
        let due = harness.tick_actions(STEP, actions, InputAction::NONE);
        frames += due;
        for _ in 0..due {
            gaps.push(harness.clock() - last);
            last = harness.clock();
        }
    }

    (gaps, frames)
}

/// Asserts that the overwhelming majority of `gaps` sit inside `period ± 2 ms`, which is what
/// the statement of REQ-LOOP-011 asks of the gap between two consecutive emulated frames.
fn assert_intervals_near(gaps: &[f64], period: f64) {
    assert!(!gaps.is_empty(), "no emulated frame was timed");
    let inside = gaps
        .iter()
        .filter(|gap| (*gap - period).abs() <= 0.002)
        .count();
    assert!(
        inside >= gaps.len() * 95 / 100,
        "only {inside} of {} gaps are inside {:.2} ms ± 2 ms",
        gaps.len(),
        period * 1000.0
    );
}
