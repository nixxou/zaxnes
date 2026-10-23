//! Port of `RollbackTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::LoopHarness;
use rstest::rstest;
use walkdir::WalkDir;
use zax_app::emulation::commands::Command;
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::emulation::rollback_history::{RollbackHistory, CAPACITY};

#[test]
fn req_loop_030_the_history_keeps_thirty_snapshots() {
    let mut history = RollbackHistory::new();
    for index in 0..40_u8 {
        history.capture(vec![index]);
    }
    assert_eq!(CAPACITY, history.count());
    // The newest two are 39 and 38; a rollback restores the second-to-last, that is 38.
    assert_eq!(Some(vec![38]), history.take_rollback_target());
    assert_eq!(CAPACITY - 2, history.count());
}

#[test]
fn a_full_history_lends_its_oldest_buffer_to_the_next_snapshot() {
    let mut history = RollbackHistory::new();
    for index in 0..CAPACITY {
        let byte = u8::try_from(index).unwrap();
        history
            .capture_with(|buffer| {
                assert!(buffer.is_empty(), "no buffer to recycle before the history is full");
                buffer.push(byte);
                Ok::<(), ()>(())
            })
            .unwrap();
    }
    history
        .capture_with(|buffer| {
            assert_eq!(&[0], buffer.as_slice(), "the oldest snapshot is the one recycled");
            buffer.clear();
            buffer.push(99);
            Ok::<(), ()>(())
        })
        .unwrap();
    assert_eq!(CAPACITY, history.count());
    let last = u8::try_from(CAPACITY - 1).unwrap();
    assert_eq!(Some(vec![last]), history.take_rollback_target());
    assert_eq!(Some(vec![1]), history.take_at(0));
}

#[test]
fn a_failed_snapshot_leaves_the_history_as_it_was() {
    let mut history = RollbackHistory::new();
    for index in 0..40_u8 {
        history.capture(vec![index]);
    }
    assert_eq!(Err("no ROM"), history.capture_with(|_| Err("no ROM")));
    assert_eq!(CAPACITY, history.count());
    assert_eq!(Some(vec![10]), history.take_at(0));
    let mut empty = RollbackHistory::new();
    assert_eq!(Err(()), empty.capture_with(|_| Err(())));
    assert!(empty.is_empty());
}

#[rstest]
#[case(0, true)]
#[case(59, false)]
#[case(60, true)]
#[case(600, true)]
#[case(601, false)]
fn req_loop_030_snapshots_land_on_multiples_of_sixty(#[case] frame: i64, #[case] expected: bool) {
    assert_eq!(expected, RollbackHistory::is_snapshot_frame(frame));
}

#[test]
fn req_loop_032_the_edge_cases_are_fixed() {
    let mut history = RollbackHistory::new();
    assert_eq!(None, history.take_rollback_target());
    history.capture(vec![7]);
    assert_eq!(Some(vec![7]), history.take_rollback_target());
    assert!(history.is_empty());
}

#[test]
fn req_loop_033_the_history_panel_keeps_only_older_entries() {
    let mut history = RollbackHistory::new();
    for index in 0..10_u8 {
        history.capture(vec![index]);
    }
    assert_eq!(Some(vec![4]), history.take_at(4));
    assert_eq!(4, history.count());
    assert_eq!(None, history.take_at(9));
}

#[test]
fn req_loop_032_a_rollback_restores_the_second_to_last_snapshot() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(100);
    assert_eq!(100, harness.host.session().unwrap().core().frame_count());
    assert_eq!(2, harness.host.rollback.count());
    harness.run(Command::Rollback);
    assert_eq!(0, harness.host.session().unwrap().core().frame_count());
    assert_eq!(EmulationState::Running, harness.host.state());
    let current = harness.host.notifications.current().unwrap();
    assert_eq!("Roll Back", current.text);
    assert!((current.seconds - 1.0).abs() < f64::EPSILON);
    // Frame 0 is a multiple of 60, so it is captured again straight away.
    harness.run_frames(1);
    assert_eq!(1, harness.host.rollback.count());
}

#[test]
fn req_loop_032_each_press_goes_back_one_to_two_seconds() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(630);
    assert_eq!(630, harness.host.session().unwrap().core().frame_count());
    harness.run(Command::Rollback);
    assert_eq!(540, harness.host.session().unwrap().core().frame_count());
    // The restored snapshot sits on a multiple of 60, so it is captured again on the very next
    // frame (REQ-LOOP-032, last line); the next press therefore lands on 480.
    harness.run_frames(1);
    harness.run(Command::Rollback);
    assert_eq!(480, harness.host.session().unwrap().core().frame_count());
}

#[test]
fn req_loop_031_only_a_launch_load_or_reset_clears_the_history() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(200);
    let snapshots = harness.host.rollback.count();
    assert!(snapshots >= 3);
    harness.run(Command::Pause);
    assert_eq!(snapshots, harness.host.rollback.count());
    harness.run(Command::QuickSave);
    assert_eq!(snapshots, harness.host.rollback.count());
    harness.run(Command::Reset);
    assert_eq!(0, harness.host.rollback.count());
}

#[test]
fn req_loop_031_a_rollback_right_after_a_load_restores_nothing() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(120);
    harness.run(Command::QuickSave);
    harness.run_frames(120);
    harness.run(Command::QuickLoad);
    assert_eq!(0, harness.host.rollback.count());
    let frames = harness.host.session().unwrap().core().frame_count();
    harness.run(Command::Rollback);
    assert_eq!(frames, harness.host.session().unwrap().core().frame_count());
    assert_eq!(EmulationState::Running, harness.host.state());
}

#[test]
fn dec_07_q3_a_rollback_leaves_the_camera_alone() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(120);
    {
        let scene = harness.host.scene_mut();
        scene.camera.current_mut().set_distance(123.0);
        scene.camera.snap_to_setpoints();
    }
    let distance = harness.host.scene().camera.distance();
    harness.run(Command::Rollback);
    assert!((harness.host.scene().camera.distance() - distance).abs() < 1e-3);
}

#[test]
fn req_pers_058_the_history_is_not_persisted() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(200);
    harness.host.shutdown();
    let found = WalkDir::new(harness.root.path())
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .contains("rollback")
        });
    assert!(!found);
}
