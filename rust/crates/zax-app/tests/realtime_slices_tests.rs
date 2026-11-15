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
