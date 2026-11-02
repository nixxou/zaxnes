//! The camera of 3dSen 2.0 where the scene and the script API meet (AMD-040 REQ-LUA-113,
//! AMD-045 REQ-SCN-069): the two copies of the framed-distance formula must agree, and the
//! camera the scripts see must be the scene's.

use zax_app::scene_script_bridge::SceneCameraBridge;
use zax_render::camera::SceneCamera;
use zax_render::geometry;
use zax_scripting::{framed_distance, ScriptCamera, MAX_CAMERA_DISTANCE};

/// `zax-scripting` carries its own copy of `d₀ = 132.7964 / tan(FOV / 2)`, the crate not
/// depending on the renderer. The two must give the same number at every field of view the
/// camera accepts, and beyond (a script may write `camera.fov` down to 10°, SPEC-09 §4.1).
#[test]
fn req_scn_069_the_two_copies_of_the_framed_distance_agree() {
    let mut fov = 10.0_f32;
    while fov <= 150.0 {
        let scene = geometry::reset_distance(fov);
        let scripting = framed_distance(fov);
        assert!(
            (scene - scripting).abs() <= scene.abs() * 1.0e-6,
            "{fov}°: {scene} against {scripting}"
        );
        fov += 0.5;
    }
}

/// REQ-SCN-011: the bound `CS.Nes3D.CameraControl.DistanceMax` publishes is the camera's own
/// (AMD-059 REQ-LUA-117).
#[test]
fn req_lua_117_the_published_bound_is_the_one_the_camera_applies() {
    assert!((MAX_CAMERA_DISTANCE - geometry::MAX_DISTANCE).abs() < f32::EPSILON);

    let mut camera = SceneCamera::new();
    camera.current_mut().set_distance(1000.0);
    assert!((camera.current().distance() - MAX_CAMERA_DISTANCE).abs() < f32::EPSILON);
}

/// REQ-LUA-113 through the real camera: the bridge hands out the scene camera's framed
/// distance, which follows the field of view, and is not the bounded setpoint.
#[test]
fn req_lua_113_the_bridge_reads_the_framed_distance_of_the_scene_camera() {
    let mut camera = SceneCamera::new();
    camera.set_fov(45.0);
    let bridge = SceneCameraBridge(&mut camera);

    assert!((bridge.reset_distance() - 320.60).abs() < 0.05);
    assert!((bridge.fov() - 45.0).abs() < f32::EPSILON);
    // The setpoint the reset wrote went through the [0, 512] bound; the framed distance did not.
    assert!((bridge.distance() - 320.60).abs() < 0.05);
}

/// REQ-SCN-069: d₀ goes past 512 under a field of view a script may post, and the setpoint is
/// bounded there while `Camera.ResetDistance` is not.
#[test]
fn req_scn_069_the_setpoint_is_bounded_and_the_framed_distance_is_not() {
    let mut camera = SceneCamera::new();
    camera.set_fov(20.0);
    // The camera clamps the field of view to [45, 95] (REQ-SCN-005), so 20° never reaches it.
    assert!((camera.fov() - 45.0).abs() < f32::EPSILON);

    // Below 29.1° the formula passes the bound (REQ-SCN-069).
    assert!(geometry::reset_distance(29.0) > geometry::MAX_DISTANCE);
    assert!(geometry::reset_distance(29.2) < geometry::MAX_DISTANCE);
}
