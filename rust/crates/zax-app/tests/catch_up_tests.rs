//! The catch-up limit of the frame clock (Rust-only, handhelds): `--catch-up`,
//! `ZAXNES_CATCH_UP`, none by default on a handheld, REQ-LOOP-012 elsewhere.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::LoopHarness;
use zax_app::app_options::{parse_catch_up, AppOptions, OptionsError};
use zax_app::emulation::frame_pacer::{CatchUp, Clock, FramePacer, MAX_BACKLOG};

fn manual() -> (Rc<Cell<f64>>, Clock) {
    let now = Rc::new(Cell::new(0.0));
    let source = Rc::clone(&now);
    (now, Box::new(move || source.get()))
}

#[test]
fn the_values_of_the_option_are_zero_to_five() {
    assert_eq!(CatchUp::Off, parse_catch_up("0").unwrap());
    assert_eq!(CatchUp::Limit(1), parse_catch_up(" 1 ").unwrap());
    assert_eq!(CatchUp::Limit(MAX_BACKLOG), parse_catch_up("5").unwrap());
    for wrong in ["6", "-1", "two", ""] {
        assert_eq!(
            Err(OptionsError::CatchUp {
                value: wrong.to_owned()
            }),
            parse_catch_up(wrong)
        );
    }
    let options = AppOptions::parse(&["--catch-up", "2"]).unwrap();
    assert_eq!(Some(CatchUp::Limit(2)), options.catch_up);
    assert_eq!(None, AppOptions::parse(&[]).unwrap().catch_up);
}

#[test]
fn without_catch_up_a_displayed_frame_runs_the_frames_it_stands_for() {
    // Every refresh of a 60 Hz screen, every other one, and a 59.94 Hz game on a 60 Hz screen.
    assert_eq!(1, CatchUp::Off.frames_per_display(60.0, 60.0));
    assert_eq!(2, CatchUp::Off.frames_per_display(60.0, 30.0));
    assert_eq!(1, CatchUp::Off.frames_per_display(60.0988, 60.0));
    // Fast forward (x1.5) and the fastest speed setting (x2) still get their frames.
    assert_eq!(2, CatchUp::Off.frames_per_display(90.0, 60.0));
    assert_eq!(4, CatchUp::Off.frames_per_display(120.0, 30.0));
    // Never more than REQ-LOOP-012 allows, and the whole allowance when the display is unknown.
    assert_eq!(MAX_BACKLOG, CatchUp::Off.frames_per_display(600.0, 60.0));
    assert_eq!(MAX_BACKLOG, CatchUp::Off.frames_per_display(60.0, 0.0));
    assert_eq!(3, CatchUp::Limit(3).frames_per_display(60.0, 60.0));
}

#[test]
fn a_late_display_drops_the_backlog_beyond_the_limit() {
    let (now, clock) = manual();
    let mut pacer = FramePacer::new(60.0, clock);
    pacer.max_backlog = 1;
    assert_eq!(1, pacer.due_frames());
    // A displayed frame that took 40 ms: one frame runs, the rest of the backlog is dropped, so
    // the next frame is due a period later instead of at once.
    now.set(0.040);
    assert_eq!(1, pacer.due_frames());
    assert_eq!(1, pacer.dropped_frames());
    now.set(0.041);
    assert_eq!(0, pacer.due_frames());
}

#[test]
fn the_host_applies_the_limit() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.tick_frame();
    harness.host.catch_up = CatchUp::Off;
    assert_eq!(1, harness.tick(0.5));
    harness.host.catch_up = CatchUp::Limit(MAX_BACKLOG);
    assert_eq!(MAX_BACKLOG, harness.tick(0.5));
}
