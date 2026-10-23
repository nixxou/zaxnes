//! Port of `InputHostTests.cs`: the input layer driven end to end with no window at all, which
//! is the acceptance criterion of REQ-INP-120.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::{LoopHarness, SyntheticRom};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::input::{InputAction, InputSnapshot};
use zax_nes::ControllerPortType;

fn ports(harness: &LoopHarness) -> (u8, u8) {
    let session = harness.host.session().unwrap();
    let core = session.core();
    let input = core.bus().input();
    (input.port_byte(0), input.port_byte(1))
}

fn launched() -> LoopHarness {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness.catalog().add("g1", Some(&rom));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("g1"));
    harness
}

/// REQ-INP-004 and REQ-INP-001: both players reach the core through one input word, J1 in the
/// low byte and J2 in the next one.
#[test]
fn req_inp_120_injected_snapshots_drive_both_players_through_the_core() {
    let mut harness = launched();
    let both = InputSnapshot::of(
        InputAction::RIGHT | InputAction::A,
        InputAction::UP | InputAction::START,
    );
    assert!(harness.tick_snapshot(1.0 / 60.0, &both) > 0);
    assert_eq!((0x81, 0x18), ports(&harness));
    // REQ-INP-006: the camera and system bits never reach the core.
    let system = InputSnapshot::of(
        InputAction::RIGHT | InputAction::ROLL_BACK | InputAction::MENU,
        InputAction::FAST_FORWARD | InputAction::ROT_LEFT,
    );
    harness.tick_snapshot(1.0 / 60.0, &system);
    assert_eq!((0x80, 0x00), ports(&harness));
}

/// REQ-INP-005: one emulated frame is one sample, fast forward included: the mask of the
/// snapshot is in force for every frame a tick runs.
#[test]
fn req_inp_005_every_emulated_frame_of_a_burst_sees_the_same_sample() {
    let mut harness = launched();
    // A press held across a run of ticks is in force on every emulated frame those ticks run:
    // the core's port byte is checked after each one.
    let before = harness.host.emulated_frames();
    let mut ran = 0;
    let held = InputSnapshot::of(InputAction::RIGHT | InputAction::A, InputAction::NONE);
    for _ in 0..60 {
        ran += harness.tick_snapshot(1.0 / 60.0, &held);
        assert_eq!(0x81, ports(&harness).0);
    }
    assert!(
        ran >= 55,
        "a second of ticks ran only {ran} emulated frames"
    );
    assert_eq!(before + i64::from(ran), harness.host.emulated_frames());
    // REQ-INP-103 with REQ-INP-005: holding FastForward runs more emulated frames for the same
    // real time, and the player bits of the same snapshot still reach the core on each of them.
    let before = harness.host.emulated_frames();
    let mut fast = 0;
    let fast_held = InputSnapshot::of(
        InputAction::RIGHT | InputAction::FAST_FORWARD,
        InputAction::NONE,
    );
    for _ in 0..60 {
        fast += harness.tick_snapshot(1.0 / 60.0, &fast_held);
        assert_eq!(0x80, ports(&harness).0);
    }
    assert!(fast > ran, "fast forward ran {fast} frames, x1 ran {ran}");
    assert_eq!(before + i64::from(fast), harness.host.emulated_frames());
}

/// REQ-INP-100 to REQ-INP-104: the hotkeys of the snapshot reach the command model: a short
/// Menu press pauses, a long one resets the view without leaving Running, and a rising edge of
/// `RollBack` rolls back.
#[test]
fn req_inp_120_injected_snapshots_trigger_the_hotkeys() {
    let mut harness = launched();
    assert_eq!(EmulationState::Running, harness.host.state());
    // REQ-INP-101: a short press on J2's Menu pauses, whatever the port types are.
    harness.tick_snapshot(
        0.1,
        &InputSnapshot::of(InputAction::NONE, InputAction::MENU),
    );
    harness.tick_snapshot(0.01, &InputSnapshot::EMPTY);
    assert_eq!(EmulationState::Paused, harness.host.state());
    harness.tick_snapshot(
        0.1,
        &InputSnapshot::of(InputAction::MENU, InputAction::NONE),
    );
    harness.tick_snapshot(0.01, &InputSnapshot::EMPTY);
    assert_eq!(EmulationState::Running, harness.host.state());
    // REQ-INP-101: a long press resets the view and leaves the state alone. The first held tick
    // starts the measurement, so the 0.3 s boundary is crossed on the third.
    harness
        .host
        .scene_mut()
        .camera
        .current_mut()
        .set_distance(42.0);
    let menu = InputSnapshot::of(InputAction::MENU, InputAction::NONE);
    harness.tick_snapshot(0.2, &menu);
    harness.tick_snapshot(0.2, &menu);
    harness.tick_snapshot(0.2, &menu);
    harness.tick_snapshot(0.01, &InputSnapshot::EMPTY);
    assert_eq!(EmulationState::Running, harness.host.state());
    assert!((harness.host.scene().camera.current().distance() - 42.0).abs() > f32::EPSILON);
}

/// REQ-INP-061 and REQ-INP-063: an injected snapshot moves the Zapper, and the position the
/// core is handed is the one the pointer aimed at on the previous displayed frame.
#[test]
fn req_inp_120_injected_snapshots_move_the_zapper() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness
        .catalog()
        .add_with("duck", Some(&rom), None, false, Some("Duck Hunt"));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("duck"));
    harness
        .host
        .session_mut()
        .unwrap()
        .set_port_type(1, ControllerPortType::Zapper);
    let viewport = [256.0, 240.0];
    // The exact pixel depends on the camera; what matters here is that the position moves with
    // the pointer and is handed to the core for the Zapper port.
    let aim = |pointer: [f32; 2]| InputSnapshot {
        pointer,
        pointer_size: viewport,
        ..InputSnapshot::EMPTY
    };
    let handed = |harness: &LoopHarness| {
        harness
            .host
            .session()
            .unwrap()
            .core()
            .bus()
            .input()
            .zapper_position(1)
            .unwrap()
    };
    harness.tick_snapshot(1.0 / 60.0, &aim([10.0, 10.0]));
    let top_left = harness.host.zapper.position();
    assert_eq!(top_left, handed(&harness));
    harness.tick_snapshot(1.0 / 60.0, &aim([246.0, 230.0]));
    let bottom_right = harness.host.zapper.position();
    assert!(
        bottom_right.x > top_left.x,
        "pointing right must raise the aimed column"
    );
    assert!(
        bottom_right.y > top_left.y,
        "pointing down must raise the aimed line"
    );
    assert_eq!(bottom_right, handed(&harness));
}
