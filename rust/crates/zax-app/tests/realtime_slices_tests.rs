//! Lot FIDPLUS-B step 3 (`docs/design/REALTIME-PACING.md`): with `emu.realtime_input` on, the
//! host runs the machine in slices of real time, and the frames still come at the console's rate.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::LoopHarness;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

fn harness(slices: bool) -> LoopHarness {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    // Run-ahead (on by default) keeps whole frames: off here.
    harness.host.update_settings(|settings| {
        settings.set(SettingId::EmuRunAhead, SettingValue::Int(0));
        settings.set(SettingId::EmuRealtimeInput, SettingValue::Bool(slices))
    });
    harness.tick_frame();
    harness
}

fn frame_count(harness: &LoopHarness) -> i64 {
    harness.host.session().expect("a game").core().frame_count()
}

fn in_progress(harness: &LoopHarness) -> bool {
    harness.host.session().expect("a game").frame_in_progress()
}

#[test]
fn fidplus_b3_ten_seconds_of_slices_make_six_hundred_frames() {
    let mut harness = harness(true);
    let start = frame_count(&harness);
    let mut ended = 0;
    for _ in 0..600 {
        ended += harness.tick(1.0 / 60.0);
    }
    let made = frame_count(&harness) - start;
    assert_eq!(i64::from(ended), made);
    assert!((598..=601).contains(&made), "{made}");
}

#[test]
fn fidplus_b3_a_frame_spreads_over_two_half_frame_ticks() {
    let mut harness = harness(true);
    let start = frame_count(&harness);
    let mut quiet = 0;
    for _ in 0..120 {
        let ended = harness.tick(1.0 / 120.0);
        assert!(ended <= 1, "{ended} frames in half a frame of time");
        if ended == 0 {
            // Half a frame ran: the frame goes on at the next tick.
            quiet += 1;
            assert!(in_progress(&harness));
        }
    }
    let made = frame_count(&harness) - start;
    assert!((58..=61).contains(&made), "{made}");
    assert!(quiet >= 55, "{quiet}");
}

#[test]
fn fidplus_b3_whole_frames_again_finish_the_sliced_one() {
    let mut harness = harness(true);
    harness.tick(1.0 / 120.0);
    while !in_progress(&harness) {
        harness.tick(1.0 / 120.0);
    }
    let start = frame_count(&harness);
    harness.host.update_settings(|settings| {
        settings.set(SettingId::EmuRealtimeInput, SettingValue::Bool(false))
    });
    harness.tick_frame();
    assert!(!in_progress(&harness));
    assert!(frame_count(&harness) > start);
}

#[test]
fn fidplus_b3_run_ahead_keeps_whole_frames() {
    let mut harness = harness(true);
    harness
        .host
        .update_settings(|settings| settings.set(SettingId::EmuRunAhead, SettingValue::Int(1)));
    for _ in 0..10 {
        harness.tick(1.0 / 120.0);
        assert!(!in_progress(&harness));
    }
}

/// Lot FIDPLUS-B step 4, the rate judge of `docs/design/REALTIME-PACING.md`: with
/// `emu.exact_rate` the host paces at the console's rate, and ten minutes are 36 059 frames, not
/// 36 000 (the pacer alone, on a simulated clock: 36 000 emulated frames would take a debug build
/// minutes).
#[test]
fn fidplus_b4_ten_minutes_at_the_exact_rate_are_36059_frames() {
    use std::cell::Cell;
    use std::rc::Rc;
    use zax_app::emulation::frame_pacer::FramePacer;

    let mut harness = harness(false);
    harness.host.update_settings(|settings| {
        settings.set(SettingId::EmuExactRate, SettingValue::Bool(true))
    });
    harness.tick_frame();
    let rate = harness.host.pacer.nominal_rate;
    assert!((rate - 60.098_8).abs() < 1e-3, "{rate}");

    // The pacer hands a frame out at once and starts frames a little early: counted the same way
    // at the nominal rate, ten minutes differ by the 59 frames of the console's own rate.
    let count = |rate: f64| {
        let now = Rc::new(Cell::new(0.0_f64));
        let source = Rc::clone(&now);
        let mut pacer = FramePacer::new(rate, Box::new(move || source.get()));
        let mut frames = 0_u64;
        while now.get() < 600.0 {
            now.set(now.get() + 1.0 / 60.0);
            frames += u64::from(pacer.due_frames());
        }
        frames
    };
    let exact = count(rate);
    let nominal = count(60.0);
    assert!((36_059..=36_061).contains(&exact), "{exact}");
    assert!(
        (58..=60).contains(&(exact - nominal)),
        "{exact} against {nominal}"
    );
}
