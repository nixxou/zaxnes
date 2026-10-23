//! The camera of SPEC-06 §3.2 to §3.6 and the test vectors of §5.1 (port of `CameraTests.cs`).

mod common;

use common::assert_close;
use zax_profiles::values::Vec4F;
use zax_render::camera::{CameraMode, CameraModeState, CameraStateData, CameraView, SceneCamera};
use zax_render::controls::{self, CameraAction};
use zax_render::geometry::{self, VisibleRect};
use zax_render::numerics::Vector3;
use zax_render::smoothing::CriticalSmoothing;

/// Clip margins of vector C1, the defaults of REQ-SCN-003.
const DEFAULT_CLIPPING: Vec4F = Vec4F::new(0.1, 8.1, 8.1, 0.1);

/// Clip margins of vector C2, those of the Super Mario Bros profile.
const MARIO_CLIPPING: Vec4F = Vec4F::new(0.0, 7.9, 0.0, 0.1);

fn close(expected: f32, actual: f32, tolerance: f64) {
    assert_close(f64::from(expected), f64::from(actual), tolerance);
}

fn steady(clipping: Vec4F) -> CameraView {
    let mut camera = SceneCamera::new();
    camera.snap_to_setpoints();
    camera.build_view(clipping, 16.0 / 9.0)
}

fn advance(camera: &mut SceneCamera, seconds: f32) {
    let steps = (seconds * 600.0).round_ties_even() as i32;
    for _ in 0..steps {
        camera.advance(1.0 / 600.0);
    }
}

#[test]
fn req_scn_012_default_distance_is_104_over_atan_of_half_the_fov() {
    for (fov, expected) in [(70.0, 189.65), (45.0, 277.93), (95.0, 150.25), (90.0, 156.21)] {
        close(expected, geometry::default_distance(fov), 0.05);
    }
}

#[test]
fn req_scn_045_hud_plane_distance_is_60_over_atan_of_half_the_fov() {
    for (fov, expected) in [(70.0, 109.42), (90.0, 90.12)] {
        close(expected, geometry::hud_distance(fov), 0.05);
    }
}

#[test]
fn req_scn_004_visible_height_in_the_nes_plane_matches_the_acceptance_criterion() {
    // 16:9, FOV 70, distance 189.65 from the NES plane: 265.6 high and 472.2 wide (±1 %).
    let distance = 189.65_f32;
    let height = 2.0 * distance * (70.0_f32 * std::f32::consts::PI / 180.0 / 2.0).tan();
    let width = height * 16.0 / 9.0;

    close(265.6, height, 2.7);
    close(472.2, width, 4.8);
}

#[test]
fn req_scn_007_forward_is_cos_rx_sin_ry_minus_sin_rx_cos_rx_cos_ry() {
    let f = geometry::forward(Vector3::new(30.0, -20.0, 0.0), false);

    close(-0.29620, f.x, 1e-4);
    close(-0.5, f.y, 1e-4);
    close(0.81380, f.z, 1e-4);
}

#[test]
fn req_scn_007_rz_has_no_effect_on_the_forward_direction() {
    let without = geometry::forward(Vector3::new(12.0, 34.0, 0.0), false);
    let with = geometry::forward(Vector3::new(12.0, 34.0, 57.0), false);

    close(without.x, with.x, 1e-5);
    close(without.y, with.y, 1e-5);
    close(without.z, with.z, 1e-5);
}

#[test]
fn req_scn_007_effective_target_recentres_on_the_visible_rectangle() {
    let target = geometry::effective_target(geometry::CENTER, DEFAULT_CLIPPING);

    // Default margins: T' = (124, 124, 0), the centre of R (REQ-E3D-410).
    close(124.0, target.x, 1e-4);
    close(124.0, target.y, 1e-4);
    close(0.0, target.z, 1e-4);
    let centre = VisibleRect::from_clipping(DEFAULT_CLIPPING).center();
    close(centre.x, target.x, 1e-4);
    close(centre.y, target.y, 1e-4);
}

#[test]
fn vector_c1_default_view_with_the_default_margins() {
    let view = steady(DEFAULT_CLIPPING);

    close(189.65, view.distance, 0.05);
    close(124.0, view.target.x, 0.5);
    close(124.0, view.target.y, 0.5);
    close(121.32, view.eye.x, 0.5);
    close(126.68, view.eye.y, 0.5);
    close(-189.61, view.eye.z, 0.5);
    close(123.4, view.aim_point.x, 0.6);
    close(124.6, view.aim_point.y, 0.6);
}

#[test]
fn vector_c2_default_view_with_the_super_mario_bros_margins() {
    let view = steady(MARIO_CLIPPING);

    close(128.0, view.target.x, 0.5);
    close(123.9, view.target.y, 0.5);
    close(125.32, view.eye.x, 0.5);
    close(126.58, view.eye.y, 0.5);
    close(-189.61, view.eye.z, 0.5);
    close(127.3, view.aim_point.x, 0.6);
    close(124.5, view.aim_point.y, 0.6);
}

#[test]
fn vector_c3_orbited_view_places_the_eye_above_and_to_the_right() {
    let mut camera = SceneCamera::new();
    camera.current_mut().angles = Vector3::new(30.0, -20.0, 0.0);
    camera.snap_to_setpoints();
    let view = camera.build_view(MARIO_CLIPPING, 16.0 / 9.0);

    close(184.17, view.eye.x, 0.5);
    close(218.73, view.eye.y, 0.5);
    close(-154.34, view.eye.z, 0.5);
    close(145.1, view.aim_point.x, 0.6);
    close(145.7, view.aim_point.y, 0.6);
}

#[test]
fn vector_c4_a_fov_change_resets_the_view_and_the_hud_distance() {
    let mut camera = SceneCamera::new();
    camera.current_mut().set_distance(42.0);
    camera.set_fov(90.0);

    close(156.21, camera.current().distance(), 0.05);
    close(90.12, geometry::hud_distance(camera.fov()), 0.05);
    assert_eq!(geometry::DEFAULT_ANGLES, camera.current().angles);
}

#[test]
fn vector_c5_zoom_in_held_three_seconds_bottoms_out_at_the_bound() {
    let mut camera = SceneCamera::new();
    for _ in 0..180 {
        controls::apply(&mut camera, CameraAction::ZOOM_IN, 1.0 / 60.0);
    }

    close(0.0, camera.current().distance(), 0.0);
}

#[test]
fn req_scn_011_the_setpoint_distance_is_bounded_but_the_angles_are_not() {
    let mut camera = SceneCamera::new();

    camera.current_mut().set_distance(1000.0);
    close(512.0, camera.current().distance(), 0.0);
    camera.current_mut().set_distance(-5.0);
    close(0.0, camera.current().distance(), 0.0);

    // Holding rotate down for 13 s takes rx past 360° (REQ-SCN-011).
    for _ in 0..13 * 60 {
        controls::apply(&mut camera, CameraAction::ROTATE_DOWN, 1.0 / 60.0);
    }

    assert!(camera.current().angles.x > 360.0);
}

#[test]
fn req_scn_014_rotate_right_held_three_seconds_adds_ninety_degrees_at_any_frame_rate() {
    for frame_rate in [30.0_f32, 60.0, 144.0] {
        let mut camera = SceneCamera::new();
        let start = camera.current().angles.y;
        let steps = (3.0 * frame_rate) as i32;
        for _ in 0..steps {
            controls::apply(&mut camera, CameraAction::ROTATE_RIGHT, 1.0 / frame_rate);
        }

        close(start + 90.0, camera.current().angles.y, 90.0 * 0.02);
    }
}

#[test]
fn req_scn_014_opposite_rotations_do_not_cancel_each_other() {
    let mut camera = SceneCamera::new();
    let start_y = camera.current().angles.y;
    let start_x = camera.current().angles.x;

    controls::apply(
        &mut camera,
        CameraAction::ROTATE_LEFT
            | CameraAction::ROTATE_RIGHT
            | CameraAction::ROTATE_UP
            | CameraAction::ROTATE_DOWN,
        1.0,
    );

    close(start_y - 30.0, camera.current().angles.y, 1e-4);
    close(start_x + 30.0, camera.current().angles.x, 1e-4);
}

#[test]
fn req_scn_015_zoom_in_held_one_second_from_the_default_view_reaches_119_65() {
    let mut camera = SceneCamera::new();
    for _ in 0..60 {
        controls::apply(&mut camera, CameraAction::ZOOM_IN, 1.0 / 60.0);
    }

    close(119.65, camera.current().distance(), 119.65 * 0.02);
}

#[test]
fn req_scn_016_ten_wheel_notches_forward_take_the_distance_to_139_65() {
    let mut camera = SceneCamera::new();
    controls::apply_wheel(&mut camera, 10.0, CameraAction::NONE);

    close(139.65, camera.current().distance(), 1.0);
}

#[test]
fn req_scn_016_the_wheel_is_ignored_while_a_zoom_action_is_held() {
    let mut camera = SceneCamera::new();
    let before = camera.current().distance();
    controls::apply_wheel(&mut camera, 10.0, CameraAction::ZOOM_IN);

    close(before, camera.current().distance(), 0.0);
}

#[test]
fn req_scn_017_dragging_300_pixels_right_adds_thirty_degrees_to_ry() {
    let mut camera = SceneCamera::new();
    let start = camera.current().angles.y;
    controls::apply_drag(&mut camera, 300.0, 0.0);

    close(start + 30.0, camera.current().angles.y, 1e-3);
}

#[test]
fn req_scn_019_a_distance_step_of_one_hundred_decays_as_the_criterion_says() {
    let mut camera = SceneCamera::new();
    camera.snap_to_setpoints();
    let target = camera.current().distance();
    camera.current_mut().set_distance(target + 100.0);

    advance(&mut camera, 0.4);
    close(40.6, (camera.current().distance() - camera.distance()).abs(), 3.0);

    advance(&mut camera, 0.8);
    assert!((camera.current().distance() - camera.distance()).abs() < 2.0);
}

#[test]
fn req_scn_019_the_filter_never_overshoots_its_setpoint() {
    let mut filter = CriticalSmoothing::default();
    let mut value = 0.0;
    for _ in 0..600 {
        value = filter.step(value, 100.0, 0.4, 1.0 / 60.0);
        assert!((0.0..=100.0).contains(&value));
    }

    close(100.0, value, 1e-3);
}

#[test]
fn req_scn_006_each_mode_keeps_its_own_distance_setpoint() {
    let mut camera = SceneCamera::new();
    camera.current_mut().set_distance(150.0);

    camera.set_mode(CameraMode::Fpv);
    camera.current_mut().set_distance(40.0);
    close(40.0, camera.current().distance(), 0.0);
    close(
        geometry::MODE_TRANSITION_SECONDS,
        camera.transition_remaining(),
        0.0,
    );

    camera.set_mode(CameraMode::Tpv);
    close(150.0, camera.current().distance(), 0.0);
}

#[test]
fn req_scn_012_a_view_reset_only_changes_the_distance_of_the_current_mode() {
    let mut camera = SceneCamera::new();
    camera.set_mode(CameraMode::Fpv);
    camera.current_mut().set_distance(40.0);
    camera.set_mode(CameraMode::Tpv);
    camera.current_mut().set_distance(11.0);

    camera.reset_view();

    close(
        geometry::default_distance(70.0),
        camera.setpoints(CameraMode::Tpv).distance(),
        0.05,
    );
    close(40.0, camera.setpoints(CameraMode::Fpv).distance(), 0.0);
    assert_eq!(geometry::DEFAULT_ANGLES, camera.setpoints(CameraMode::Fpv).angles);
    assert_eq!(geometry::CENTER, camera.setpoints(CameraMode::Fpv).target);
}

#[test]
fn req_scn_012_the_fpv_distance_is_zero_at_application_launch() {
    close(
        0.0,
        SceneCamera::new().setpoints(CameraMode::Fpv).distance(),
        0.0,
    );
}

#[test]
fn req_scn_020_in_fpv_the_target_is_followed_instantly_once_the_window_is_over() {
    let mut camera = SceneCamera::new();
    camera.set_mode(CameraMode::Fpv);
    advance(&mut camera, 5.0);

    camera.current_mut().target = Vector3::new(40.0, 50.0, 60.0);
    camera.advance(1.0 / 60.0);

    close(40.0, camera.target().x, 1e-3);
    close(50.0, camera.target().y, 1e-3);
    close(60.0, camera.target().z, 1e-3);
}

#[test]
fn req_scn_020_in_tpv_the_target_is_always_smoothed() {
    let mut camera = SceneCamera::new();
    advance(&mut camera, 5.0);

    camera.current_mut().target = Vector3::new(128.0, 180.0, 0.0);
    camera.advance(1.0 / 60.0);

    assert!(camera.target().y > 120.0);
    assert!(camera.target().y < 180.0);
}

#[test]
fn req_scn_013_ending_a_game_returns_to_tpv_and_to_a_smooth_time_of_zero_point_four() {
    let mut camera = SceneCamera::new();
    camera.smooth_time = 2.0;
    camera.set_mode(CameraMode::Fpv);

    camera.end_game();

    assert_eq!(CameraMode::Tpv, camera.mode());
    close(CriticalSmoothing::DEFAULT_SMOOTH_TIME, camera.smooth_time, 0.0);
    close(
        geometry::default_distance(70.0),
        camera.current().distance(),
        0.05,
    );
}

#[test]
fn req_scn_009_the_local_camera_rotates_the_picture_around_the_view_axis() {
    let mut camera = SceneCamera::new();
    camera.local_camera = true;
    camera.current_mut().angles = Vector3::new(0.0, 90.0, 0.0);
    camera.snap_to_setpoints();

    let view = camera.build_view(DEFAULT_CLIPPING, 16.0 / 9.0);

    // q = R_Z(90°) · R_X(0): the camera's right axis is scene +Y, so the top of the NES
    // picture appears on the right of the screen, a quarter turn (REQ-SCN-009 criterion).
    close(0.0, view.forward.x, 1e-4);
    close(0.0, view.forward.y, 1e-4);
    close(1.0, view.forward.z, 1e-4);
    close(1.0, view.right.y, 1e-4);
    close(-1.0, view.up.x, 1e-4);

    // The view is still centred on the effective target (REQ-SCN-009).
    let expected = geometry::effective_target(geometry::CENTER, DEFAULT_CLIPPING);
    let eye = expected - (view.forward * view.distance);
    close(eye.x, view.eye.x, 1e-3);
    close(eye.y, view.eye.y, 1e-3);
}

#[test]
fn req_scn_010_in_fpv_the_target_is_exactly_at_the_centre_of_the_picture() {
    let mut camera = SceneCamera::new();
    camera.set_mode(CameraMode::Fpv);
    camera.current_mut().angles = Vector3::ZERO;
    camera.current_mut().set_distance(100.0);
    camera.current_mut().target = Vector3::new(60.0, 70.0, 80.0);
    camera.snap_to_setpoints();

    let view = camera.build_view(DEFAULT_CLIPPING, 16.0 / 9.0);

    // No recentring on the visible rectangle in FPV (REQ-SCN-010).
    close(60.0, view.target.x, 1e-3);
    close(70.0, view.target.y, 1e-3);
    close(60.0, view.eye.x, 1e-3);
    close(70.0, view.eye.y, 1e-3);
    close(-20.0, view.eye.z, 1e-3);
}

/// REQ-SCN-008 as amended by AMD-019 (Q-046): the degenerate case is the eye on **one** of the
/// two vertices — at least one distance zero — and the point is then the midpoint of the
/// segment, not the vertex the general formula would give.
#[test] // REQ-SCN-008, Q-046
fn req_scn_008_a_degenerate_segment_falls_back_to_its_midpoint() {
    let p = Vector3::new(10.0, 20.0, 0.0);
    let q = Vector3::new(30.0, 20.0, 0.0);

    close(20.0, geometry::bisector(p, q, p).x, 1e-4);
    close(20.0, geometry::bisector(p, q, q).x, 1e-4);

    // The segment reduced to a point stays that point.
    close(10.0, geometry::bisector(p, p, p).x, 1e-4);

    // Away from the vertices the general formula still holds: an eye equidistant from both
    // ends lands on the middle.
    let middle = geometry::bisector(p, q, Vector3::new(20.0, 0.0, 0.0));
    close(20.0, middle.x, 1e-4);
}

#[test]
fn req_scn_045_a_hud_element_at_the_nes_centre_sits_four_units_above_the_screen_centre() {
    let view = steady(DEFAULT_CLIPPING);

    let centre = view.hud_point(geometry::CENTER);
    let element = view.hud_point(Vector3::new(128.0, 128.0, 0.0));

    close(4.0, Vector3::dot(element - centre, view.up), 1e-3);
    close(0.0, Vector3::dot(element - centre, view.right), 1e-3);
    close(
        view.hud_distance,
        Vector3::dot(centre - view.eye, view.forward),
        1e-3,
    );
}

#[test]
fn req_scn_045_the_hud_frame_fills_about_78_percent_of_the_picture_height() {
    let view = steady(DEFAULT_CLIPPING);

    let visible = 2.0 * view.hud_distance * (view.fov * std::f32::consts::PI / 180.0 / 2.0).tan();
    let hud_height = geometry::FRAME_HEIGHT * geometry::HUD_SCALE;

    close(0.78, hud_height / visible, 0.01);
}

#[test]
fn req_scn_029_the_hud_light_direction_does_not_move_when_the_camera_orbits() {
    let direction = Vector3::new(1.0, -1.0, 1.0);

    let mut front = SceneCamera::new();
    front.snap_to_setpoints();
    let front_view = front.build_view(DEFAULT_CLIPPING, 1.6);
    let front_light = front_view.hud_light_direction(direction);

    let mut orbited = SceneCamera::new();
    orbited.current_mut().angles = Vector3::new(0.0, 90.0, 0.0);
    orbited.snap_to_setpoints();
    let orbited_view = orbited.build_view(DEFAULT_CLIPPING, 1.6);
    let orbited_light = orbited_view.hud_light_direction(direction);

    // Expressed in the camera frame the direction is the same, so N·L on a HUD quad facing
    // the camera is unchanged.
    let front_normal = -front_view.forward;
    let orbited_normal = -orbited_view.forward;
    let front_ndl = Vector3::dot(front_normal.normalize(), (-front_light).normalize());
    let orbited_ndl = Vector3::dot(orbited_normal.normalize(), (-orbited_light).normalize());

    close(front_ndl, orbited_ndl, 1e-4);
}

#[test]
fn req_scn_024_the_captured_state_holds_the_mode_both_setpoints_and_the_window() {
    let mut camera = SceneCamera::new();
    camera.setpoints_mut(CameraMode::Tpv).set_distance(150.0);
    camera.set_mode(CameraMode::Fpv);
    camera.current_mut().set_distance(40.0);
    camera.current_mut().angles = Vector3::new(12.0, -34.0, 5.0);
    camera.current_mut().target = Vector3::new(10.0, 20.0, 30.0);
    camera.advance(1.0);

    let state = camera.capture();

    assert_eq!(CameraMode::Fpv, state.mode);
    close(150.0, state.tpv.distance, 5e-4);
    close(40.0, state.fpv.distance, 5e-4);
    assert_eq!(Vector3::new(12.0, -34.0, 5.0), state.fpv.angles);
    assert_eq!(Vector3::new(10.0, 20.0, 30.0), state.fpv.target);
    close(2.0, state.transition_remaining, 5e-3);
}

#[test]
fn req_scn_024_restoring_brings_back_fpv_with_the_distance_of_the_save() {
    let mut saved = SceneCamera::new();
    saved.set_mode(CameraMode::Fpv);
    saved.current_mut().set_distance(40.0);
    let state = saved.capture();

    // The criterion of REQ-SCN-024: save in FPV, go back to TPV and zoom, reload.
    let mut camera = SceneCamera::new();
    camera.set_mode(CameraMode::Tpv);
    camera.current_mut().set_distance(300.0);
    camera.snap_to_setpoints();

    camera.restore(&state);

    assert_eq!(CameraMode::Fpv, camera.mode());
    close(40.0, camera.current().distance(), 5e-4);

    // The displayed values are not snapped: they join the setpoint by the smoothing of
    // REQ-SCN-019.
    close(300.0, camera.distance(), 5e-4);
    advance(&mut camera, 3.0);
    close(40.0, camera.distance(), 0.05);
}

#[test]
fn req_scn_024_the_restored_distances_stay_inside_their_bounds() {
    let mut camera = SceneCamera::new();
    camera.restore(&CameraStateData {
        mode: CameraMode::Tpv,
        tpv: CameraModeState {
            distance: 9000.0,
            angles: Vector3::ZERO,
            target: Vector3::ZERO,
        },
        fpv: CameraModeState {
            distance: -20.0,
            angles: Vector3::ZERO,
            target: Vector3::ZERO,
        },
        transition_remaining: -5.0,
    });

    close(
        geometry::MAX_DISTANCE,
        camera.setpoints(CameraMode::Tpv).distance(),
        5e-4,
    );
    close(0.0, camera.setpoints(CameraMode::Fpv).distance(), 5e-4);
    close(0.0, camera.transition_remaining(), 5e-4);
}
