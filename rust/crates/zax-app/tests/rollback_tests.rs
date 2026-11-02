//! Port of `RollbackTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::LoopHarness;
use rstest::rstest;
use walkdir::WalkDir;
use zax_app::emulation::commands::Command;
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::emulation::rollback_history::{RollbackHistory, SnapshotExtras, CAPACITY};
use zax_render::camera::SceneCamera;
use zax_render::numerics::Vector3;

#[test]
fn req_loop_030_the_history_keeps_thirty_snapshots() {
    let mut history = RollbackHistory::new();
    for index in 0..40_u8 {
        history.capture(vec![index], SnapshotExtras::NONE);
    }
    assert_eq!(CAPACITY, history.count());
    // The newest two are 39 and 38; a rollback restores the second-to-last, that is 38.
    assert_eq!(
        Some(vec![38]),
        history.take_rollback_target().map(|s| s.core)
    );
    assert_eq!(CAPACITY - 2, history.count());
}

#[test]
fn a_full_history_lends_its_oldest_buffer_to_the_next_snapshot() {
    let mut history = RollbackHistory::new();
    for index in 0..CAPACITY {
        let byte = u8::try_from(index).unwrap();
        history
            .capture_with(SnapshotExtras::NONE, |buffer| {
                assert!(
                    buffer.is_empty(),
                    "no buffer to recycle before the history is full"
                );
                buffer.push(byte);
                Ok::<(), ()>(())
            })
            .unwrap();
    }
    history
        .capture_with(SnapshotExtras::NONE, |buffer| {
            assert_eq!(
                &[0],
                buffer.as_slice(),
                "the oldest snapshot is the one recycled"
            );
            buffer.clear();
            buffer.push(99);
            Ok::<(), ()>(())
        })
        .unwrap();
    assert_eq!(CAPACITY, history.count());
    let last = u8::try_from(CAPACITY - 1).unwrap();
    assert_eq!(
        Some(vec![last]),
        history.take_rollback_target().map(|s| s.core)
    );
    assert_eq!(Some(vec![1]), history.take_at(0).map(|s| s.core));
}

#[test]
fn a_failed_snapshot_leaves_the_history_as_it_was() {
    let mut history = RollbackHistory::new();
    for index in 0..40_u8 {
        history.capture(vec![index], SnapshotExtras::NONE);
    }
    assert_eq!(
        Err("no ROM"),
        history.capture_with(SnapshotExtras::NONE, |_| Err("no ROM"))
    );
    assert_eq!(CAPACITY, history.count());
    assert_eq!(Some(vec![10]), history.take_at(0).map(|s| s.core));
    let mut empty = RollbackHistory::new();
    assert_eq!(
        Err(()),
        empty.capture_with(SnapshotExtras::NONE, |_| Err(()))
    );
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
    assert!(history.take_rollback_target().is_none());
    history.capture(vec![7], SnapshotExtras::NONE);
    assert_eq!(
        Some(vec![7]),
        history.take_rollback_target().map(|s| s.core)
    );
    assert!(history.is_empty());
}

#[test]
fn req_loop_033_the_history_panel_keeps_only_older_entries() {
    let mut history = RollbackHistory::new();
    for index in 0..10_u8 {
        history.capture(vec![index], SnapshotExtras::NONE);
    }
    assert_eq!(Some(vec![4]), history.take_at(4).map(|s| s.core));
    assert_eq!(4, history.count());
    assert!(history.take_at(9).is_none());
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

/// AMD-042 REQ-LOOP-069 and REQ-LOOP-070 replace DEC-07-Q3: a snapshot carries the camera, and
/// the restored camera is shown **straight** at its setpoints — not reached by the smoothing
/// (REQ-SCN-073).
#[test]
fn req_loop_070_a_rollback_puts_the_camera_back_where_it_was() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(120);

    // The orientation and the distance at the moment of the snapshot that will be restored.
    let before = harness.host.scene().camera.capture();
    harness.run_frames(60);

    // And now the camera turns by 90° and zooms, with the smoothing caught up.
    {
        let camera = &mut harness.host.scene_mut().camera;
        camera.current_mut().angles = Vector3::new(0.81, 90.81, 0.0);
        camera.current_mut().set_distance(42.0);
        camera.snap_to_setpoints();
    }
    assert!((harness.host.scene().camera.angles().y - 90.81).abs() < 1e-3);

    harness.run(Command::Rollback);

    // The setpoints are the ones of the capture...
    let after = harness.host.scene().camera.capture();
    assert_eq!(before.tpv.angles, after.tpv.angles);
    assert!((before.tpv.distance - after.tpv.distance).abs() < 1e-3);
    assert_eq!(before.mode, after.mode);
    // ... and the displayed camera is already there, with no frame of smoothing (REQ-SCN-073).
    let camera = &harness.host.scene().camera;
    assert!((camera.angles().y - before.tpv.angles.y).abs() < 1e-3);
    assert!((camera.distance() - before.tpv.distance).abs() < 1e-3);
}

/// AMD-042 REQ-LOOP-069 and REQ-LOOP-070 acceptance: a profile writing the frame counter into
/// `PData[0]` at every `Update`; after a rollback the entry has the value it had at the moment
/// the restored snapshot was captured, not the one of the frame the game had reached.
#[test]
fn req_loop_070_a_rollback_puts_the_persistent_data_back() {
    let mut harness = LoopHarness::create();
    harness.launch_with_script("g1", "function Update()\n  SetPData(0, frame)\nend\n");

    // 10.5 s of NTSC: the snapshot a rollback restores is the one of the counter 540.
    harness.run_frames(630);
    // `frame` is the counter of the frame the hook runs for, so the last `Update` wrote 629.
    assert_eq!(629.0, pdata(&mut harness, 0));

    harness.run(Command::Rollback);

    assert_eq!(540, harness.host.session().unwrap().core().frame_count());
    // The snapshot was taken at the **start** of the frame 540, before it ran (REQ-LOOP-030), so
    // the entry holds what the `Update` of the frame before it wrote — and no longer 629.
    assert_eq!(539.0, pdata(&mut harness, 0));
}

/// REQ-LOOP-070: the rollback calls neither `Save` nor `Load` — they stay reserved for the
/// savestates and for the serialisation (REQ-LUA-074, REQ-LUA-075).
#[test]
fn req_loop_070_a_rollback_calls_no_save_or_load_hook() {
    let mut harness = LoopHarness::create();
    harness.launch_with_script(
        "g1",
        "function Save()\n  print('save')\nend\nfunction Load()\n  print('load')\nend\n",
    );
    harness.run_frames(120);
    harness.clear_script_lines();

    harness.run(Command::Rollback);

    assert_eq!(Vec::<String>::new(), harness.script_lines());
}

/// AMD-042 REQ-LOOP-071 acceptance: 10 s of play gives at least nine snapshots, and the end of
/// the game empties the history at once, before any new launch.
#[test]
fn req_loop_071_the_end_of_a_game_empties_the_history() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(600);
    assert!(
        harness.host.rollback.count() >= 9,
        "only {} snapshots",
        harness.host.rollback.count()
    );

    harness.run(Command::Stop);

    assert_eq!(EmulationState::Stopped, harness.host.state());
    assert_eq!(0, harness.host.rollback.count());
}

/// REQ-LOOP-069: the depth is still 30 snapshots once they carry three things instead of one,
/// and the steady state still recycles the oldest core buffer.
#[test]
fn req_loop_069_the_extras_travel_with_the_core_buffer_that_is_recycled() {
    let mut history = RollbackHistory::new();
    let camera = SceneCamera::new().capture();
    for index in 0..CAPACITY {
        let byte = u8::try_from(index).unwrap();
        history
            .capture_with(
                SnapshotExtras {
                    persistent_data: Some([byte; 256]),
                    camera: Some(camera),
                },
                |buffer| {
                    buffer.push(byte);
                    Ok::<(), ()>(())
                },
            )
            .unwrap();
    }
    assert_eq!(CAPACITY, history.count());

    // The oldest buffer is lent to the next snapshot, and its extras are replaced, not kept.
    history
        .capture_with(
            SnapshotExtras {
                persistent_data: Some([99; 256]),
                camera: None,
            },
            |buffer| {
                assert_eq!(&[0], buffer.as_slice());
                buffer.clear();
                buffer.push(99);
                Ok::<(), ()>(())
            },
        )
        .unwrap();
    let last = history.take_at(CAPACITY - 1).unwrap();
    assert_eq!(vec![99], last.core);
    assert_eq!(Some([99; 256]), last.extras.persistent_data);
    assert_eq!(None, last.extras.camera);
}

/// The value of `PData[index]`, as `GetPData(index)` reads it.
fn pdata(harness: &mut LoopHarness, index: i64) -> f64 {
    harness
        .host
        .session_mut()
        .unwrap()
        .read_persistent_data(index + 1)
        .unwrap()
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
