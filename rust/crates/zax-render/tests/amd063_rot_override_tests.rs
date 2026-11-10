//! The forced camera rotation of AMD-063 REQ-LUA-119: it becomes the setpoint of the current
//! mode and the displayed rotation at once, without the smoothing of REQ-SCN-019, and it is a
//! session value no savestate carries.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use zax_render::camera::{CameraMode, SceneCamera};
use zax_render::numerics::Vector3;

/// A camera settled on its default view, one real second of smoothing away from anything.
fn settled() -> SceneCamera {
    let mut camera = SceneCamera::new();
    camera.snap_to_setpoints();
    camera
}

fn close(expected: Vector3, actual: Vector3) {
    for (expected, actual) in [
        (expected.x, actual.x),
        (expected.y, actual.y),
        (expected.z, actual.z),
    ] {
        common::assert_close(f64::from(expected), f64::from(actual), 1e-4);
    }
}

// REQ-LUA-119, acceptance criterion: the forced rotation is the setpoint of the current mode and
// the rotation shown from the very next update, with no smoothing in between.
#[test]
fn req_lua_119_the_forced_rotation_is_shown_at_once() {
    let mut camera = settled();
    let forced = Vector3::new(-60.0, 10.0, 0.0);
    assert_eq!(None, camera.rot_override());

    camera.set_rot_override(Some(forced));
    camera.advance(1.0 / 60.0);

    close(forced, camera.current().angles);
    close(forced, camera.angles());

    // Every later update keeps it there, whatever a setpoint write asks for in between.
    camera.current_mut().angles = Vector3::new(0.0, 0.0, 0.0);
    camera.advance(1.0 / 60.0);
    close(forced, camera.current().angles);
    close(forced, camera.angles());
}

// REQ-LUA-119: put back to none, the camera carries on from the setpoint it was left at — the
// last forced value — and the smoothing works again from there.
#[test]
fn req_lua_119_clearing_it_leaves_the_camera_where_it_was() {
    let mut camera = settled();
    let forced = Vector3::new(-60.0, 10.0, 0.0);
    camera.set_rot_override(Some(forced));
    camera.advance(1.0 / 60.0);

    camera.set_rot_override(None);
    camera.advance(1.0 / 60.0);
    close(forced, camera.current().angles);
    close(forced, camera.angles());

    // A new setpoint is now reached through the smoothing: after one frame the displayed angle
    // has moved towards it without reaching it (REQ-SCN-019).
    camera.current_mut().angles = Vector3::new(0.0, 10.0, 0.0);
    camera.advance(1.0 / 60.0);
    assert!(
        camera.angles().x > forced.x && camera.angles().x < 0.0,
        "{} should be between {} and 0",
        camera.angles().x,
        forced.x
    );
}

// REQ-LUA-119: whatever the elapsed time, including a frame of none, the forced rotation is
// applied; and an angle outside ]-180, 180] is brought back by whole turns like any setpoint
// (AMD-045 REQ-SCN-071), the orientation shown being the one asked for.
#[test]
fn req_lua_119_the_forced_rotation_survives_a_still_frame_and_is_wrapped() {
    let mut camera = settled();
    camera.set_rot_override(Some(Vector3::new(-60.0, 10.0, 0.0)));
    camera.advance(0.0);
    close(Vector3::new(-60.0, 10.0, 0.0), camera.angles());

    camera.set_rot_override(Some(Vector3::new(400.0, 0.0, 0.0)));
    camera.advance(1.0 / 60.0);
    // 400 is 40 past a whole turn: the setpoint is wrapped, and the displayed angle follows it
    // by the same whole turns, so the orientation is the one asked for.
    common::assert_close(40.0, f64::from(camera.current().angles.x), 1e-4);
    common::assert_close(40.0, f64::from(camera.angles().x), 1e-4);
}

// REQ-LUA-119: it is a session value. REQ-SCN-024 is unchanged, so neither the capture nor the
// restore of the camera state touches it, and the end of a game clears it.
#[test]
fn req_lua_119_no_savestate_carries_the_forced_rotation() {
    let mut camera = settled();
    camera.set_rot_override(Some(Vector3::new(-60.0, 10.0, 0.0)));
    camera.advance(1.0 / 60.0);

    let state = camera.capture();

    let mut other = settled();
    other.restore(&state);
    assert_eq!(None, other.rot_override());
    // The setpoint the forced value wrote is part of the camera state and does come back.
    close(Vector3::new(-60.0, 10.0, 0.0), other.current().angles);

    camera.end_game();
    assert_eq!(None, camera.rot_override());
}

// REQ-LUA-119: the forced rotation is the **current** mode's. Switching mode leaves the other
// mode's setpoint alone until it becomes current.
#[test]
fn req_lua_119_the_forced_rotation_is_the_current_modes() {
    let mut camera = settled();
    let other = camera.setpoints(CameraMode::Fpv).angles;
    camera.set_rot_override(Some(Vector3::new(-60.0, 10.0, 0.0)));
    camera.advance(1.0 / 60.0);

    close(other, camera.setpoints(CameraMode::Fpv).angles);
    close(Vector3::new(-60.0, 10.0, 0.0), camera.angles());
}
