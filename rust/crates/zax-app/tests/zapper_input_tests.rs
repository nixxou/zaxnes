//! Port of `ZapperInputTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::{LoopHarness, SyntheticRom};
use rstest::rstest;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::input::routing;
use zax_app::input::zapper::{self, ZapperInput, ZapperState, PRESSED_BYTE, RELEASED_BYTE};
use zax_app::input::{InputAction, InputSnapshot};
use zax_core_api::ControllerPortType;
use zax_engine3d::settings::ConversionSettings;
use zax_engine3d::space_bend::SpaceBend;
use zax_nes::{NesCore, ZapperPosition};
use zax_profiles::values::Vec4F;
use zax_render::camera::{to_gl, CameraView, SceneCamera};
use zax_render::numerics::Vector3;

const VIEWPORT: [f32; 2] = [1280.0, 720.0];

fn default_view(clipping: Option<Vec4F>) -> CameraView {
    let mut camera = SceneCamera::new();
    camera.reset_view();
    camera.snap_to_setpoints();
    camera.build_view(
        clipping.unwrap_or(ConversionSettings::DEFAULT_CLIPPING),
        VIEWPORT[0] / VIEWPORT[1],
    )
}

fn project(view: &CameraView, scene_point: Vector3) -> [f32; 2] {
    let gl = to_gl(scene_point);
    let m = view.view_projection();
    let x = gl.x * m.m11 + gl.y * m.m21 + gl.z * m.m31 + m.m41;
    let y = gl.x * m.m12 + gl.y * m.m22 + gl.z * m.m32 + m.m42;
    let w = gl.x * m.m14 + gl.y * m.m24 + gl.z * m.m34 + m.m44;
    let (ndc_x, ndc_y) = (x / w, y / w);
    [
        ndc_x.midpoint(1.0) * VIEWPORT[0],
        (1.0 - ndc_y) * 0.5 * VIEWPORT[1],
    ]
}

fn update(
    zapper: &mut ZapperState,
    active: bool,
    view: &CameraView,
    pointer: [f32; 2],
    right_button: bool,
) {
    zapper.update(
        active,
        &ZapperInput {
            view,
            pointer,
            size: VIEWPORT,
            right_button,
            offset: Vector3::ZERO,
            scene_scale: 1.0,
            bend: SpaceBend::NONE,
        },
    );
}

#[rstest]
// Scene point x, scene point y, expected px, expected py (SPEC-08 §5.3).
#[case(128.0, 120.0, 128, 128)]
#[case(128.0, 128.0, 128, 120)]
#[case(0.0, 248.0, 0, 0)]
#[case(0.0, 8.0, 0, 240)]
#[case(256.0, 248.0, 256, 0)]
fn req_inp_063_the_aim_is_the_nes_pixel_under_the_pointer(
    #[case] scene_x: f32,
    #[case] scene_y: f32,
    #[case] expected_x: i32,
    #[case] expected_y: i32,
) {
    let view = default_view(None);
    let pointer = project(&view, Vector3::new(scene_x, scene_y, 0.0));
    let position = zapper::try_aim(&view, pointer, VIEWPORT, Vector3::ZERO, 1.0).unwrap();
    assert!(
        (expected_x - 1..=expected_x + 1).contains(&position.x),
        "{position:?}"
    );
    assert!(
        (expected_y - 1..=expected_y + 1).contains(&position.y),
        "{position:?}"
    );
}

#[test]
fn section_5_3_the_top_left_corner_of_the_visible_rectangle() {
    let view = default_view(Some(Vec4F::new(0.0, 7.9, 0.0, 0.1)));
    let pointer = project(&view, Vector3::new(0.0, 239.9, 0.0));
    let position = zapper::try_aim(&view, pointer, VIEWPORT, Vector3::ZERO, 1.0).unwrap();
    assert!((-1..=1).contains(&position.x), "{position:?}");
    assert!((7..=9).contains(&position.y), "{position:?}");
}

#[test]
fn req_inp_063_the_zapper_offset_shifts_the_aimed_pixel() {
    let view = default_view(None);
    let offset = Vector3::new(10.0, 0.0, 0.0);
    let pointer = project(&view, Vector3::new(128.0, 120.0, 0.0));
    let position = zapper::try_aim(&view, pointer, VIEWPORT, offset, 1.0).unwrap();
    assert!((117..=119).contains(&position.x), "{position:?}");
    assert!((127..=129).contains(&position.y), "{position:?}");
    // REQ-INP-063, notes: a positive ZapperOffset.y means the image moved up, so the same
    // screen point aims at a NES pixel further down.
    let centre = project(&view, Vector3::new(128.0, 128.0, 0.0));
    let shifted =
        zapper::try_aim(&view, centre, VIEWPORT, Vector3::new(0.0, 10.0, 0.0), 1.0).unwrap();
    assert!((129..=131).contains(&shifted.y), "{shifted:?}");
}

#[test]
fn dec_08_4_the_aim_accounts_for_the_scene_scale() {
    // At scale 2, the scene point (256, 240) is the pixel (128, 128): the division by e happens
    // before the flip constant is applied.
    let scaled = zapper::to_nes_pixel([256.0, 240.0], Vector3::ZERO, 2.0);
    assert_eq!((128, 128), (scaled.x, scaled.y));
    // At scale 1 the same point reads (256, 8).
    let unscaled = zapper::to_nes_pixel([256.0, 240.0], Vector3::ZERO, 1.0);
    assert_eq!((256, 8), (unscaled.x, unscaled.y));
}

#[test]
fn req_inp_063_a_rotated_camera_still_aims_at_the_scene_point() {
    let mut camera = SceneCamera::new();
    camera.reset_view();
    camera.current_mut().angles = Vector3::new(0.0, 30.0, 0.0);
    camera.snap_to_setpoints();
    let view = camera.build_view(
        ConversionSettings::DEFAULT_CLIPPING,
        VIEWPORT[0] / VIEWPORT[1],
    );
    for point in [
        Vector3::new(128.0, 120.0, 0.0),
        Vector3::new(128.0, 128.0, 0.0),
        Vector3::new(64.0, 180.0, 0.0),
        Vector3::new(200.0, 40.0, 0.0),
    ] {
        let pointer = project(&view, point);
        let position = zapper::try_aim(&view, pointer, VIEWPORT, Vector3::ZERO, 1.0).unwrap();
        let expected = zapper::to_nes_pixel([point.x, point.y], Vector3::ZERO, 1.0);
        assert!(
            (expected.x - 1..=expected.x + 1).contains(&position.x),
            "{position:?} {expected:?}"
        );
        assert!(
            (expected.y - 1..=expected.y + 1).contains(&position.y),
            "{position:?} {expected:?}"
        );
    }
}

#[test]
fn req_inp_064_an_off_screen_shot_is_not_clamped() {
    let view = default_view(None);
    let pointer = project(&view, Vector3::new(-20.0, 120.0, 0.0));
    let position = zapper::try_aim(&view, pointer, VIEWPORT, Vector3::ZERO, 1.0).unwrap();
    assert!((-21..=-19).contains(&position.x), "{position:?}");
    assert!(!position.is_on_screen());
    // Below the image too: y = 248 - (-1) = 249, outside [0, 240).
    let below = zapper::to_nes_pixel([10.0, -1.0], Vector3::ZERO, 1.0);
    assert_eq!(249, below.y);
    assert!(!below.is_on_screen());
}

#[test]
fn req_inp_064_a_ray_that_misses_the_plane_keeps_the_last_position() {
    let mut zapper = ZapperState::new();
    let view = default_view(None);
    let pointer = project(&view, Vector3::new(128.0, 120.0, 0.0));
    update(&mut zapper, true, &view, pointer, false);
    let valid = zapper.position();
    assert!(zapper.has_position());
    // A camera looking along the plane: the forward axis has no z component at all.
    let parallel = CameraView {
        forward: Vector3::new(1.0, 0.0, 0.0),
        right: Vector3::new(0.0, 0.0, 1.0),
        up: Vector3::new(0.0, 1.0, 0.0),
        ..default_view(None)
    };
    // Aiming at the very centre of that view: the ray runs along the plane.
    let centre = [VIEWPORT[0] * 0.5, VIEWPORT[1] * 0.5];
    assert!(zapper::try_intersect(&parallel, centre, VIEWPORT, Vector3::ZERO).is_none());
    update(&mut zapper, true, &parallel, centre, false);
    assert_eq!(valid, zapper.position());
}

#[test]
fn req_inp_062_the_trigger_is_the_right_button_with_the_dec_02_q1_polarity() {
    let mut zapper = ZapperState::new();
    let view = default_view(None);
    let pointer = project(&view, Vector3::new(128.0, 120.0, 0.0));
    update(&mut zapper, true, &view, pointer, false);
    assert!(!zapper.trigger_held());
    assert_eq!(0x10, zapper.port_byte());
    update(&mut zapper, true, &view, pointer, true);
    assert!(zapper.trigger_held());
    assert_eq!(0x00, zapper.port_byte());
    // The word the core receives carries that byte on the Zapper port and nothing else.
    let word = routing::core_word(
        InputAction::NONE,
        InputAction::A,
        ControllerPortType::Controller,
        ControllerPortType::Zapper,
        zapper.port_byte(),
    );
    assert_eq!(0x00, (word >> 8) & 0xFF);
}

#[test]
fn req_inp_061_an_inactive_zapper_ignores_the_mouse() {
    let mut zapper = ZapperState::new();
    let view = default_view(None);
    update(
        &mut zapper,
        true,
        &view,
        project(&view, Vector3::new(128.0, 120.0, 0.0)),
        false,
    );
    let frozen = zapper.position();
    update(
        &mut zapper,
        false,
        &view,
        project(&view, Vector3::new(32.0, 32.0, 0.0)),
        true,
    );
    assert_eq!(frozen, zapper.position());
    assert!(!zapper.trigger_held());
}

#[test]
fn req_inp_065_the_core_reads_the_zapper_byte_the_host_composed() {
    let mut core = NesCore::new();
    core.load_rom(&SyntheticRom::build(None, false), "zapper.nes");
    core.set_port_type(1, ControllerPortType::Zapper).unwrap();
    core.set_zapper_position_source(Some(Box::new(|_| ZapperPosition::new(-20, 120))));
    core.run_frame(routing::core_word(
        InputAction::NONE,
        InputAction::NONE,
        ControllerPortType::Controller,
        ControllerPortType::Zapper,
        RELEASED_BYTE,
    ));
    assert_eq!(0x18, core.bus_mut().read_input_port(1, 0x00));
    core.run_frame(routing::core_word(
        InputAction::NONE,
        InputAction::NONE,
        ControllerPortType::Controller,
        ControllerPortType::Zapper,
        PRESSED_BYTE,
    ));
    assert_eq!(0x08, core.bus_mut().read_input_port(1, 0x00));
}

#[test]
fn req_inp_060_a_different_game_puts_both_ports_back_on_controllers() {
    let mut harness = LoopHarness::create();
    let rom = harness.write_rom(&SyntheticRom::counter(), "game.nes");
    harness
        .catalog()
        .add_with("duck", Some(&rom), None, false, Some("Duck Hunt"));
    harness
        .catalog()
        .add_with("other", Some(&rom), None, false, Some("Other Game"));
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("duck"));
    assert_eq!(ControllerPortType::Controller, harness.host.port_type(0));
    assert_eq!(ControllerPortType::Controller, harness.host.port_type(1));
    // A Zapper profile would do this from its Start hook (SPEC-05 REQ-LUA-032).
    harness
        .host
        .session_mut()
        .unwrap()
        .set_port_type(1, ControllerPortType::Zapper);
    assert_eq!(ControllerPortType::Zapper, harness.host.port_type(1));
    assert!(harness.host.zapper_active());
    // Relaunching the same game is not a different game: the types are left alone.
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("duck"));
    assert_eq!(ControllerPortType::Zapper, harness.host.port_type(1));
    // Another game sends both ports back to Controller.
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("other"));
    assert_eq!(ControllerPortType::Controller, harness.host.port_type(0));
    assert_eq!(ControllerPortType::Controller, harness.host.port_type(1));
    assert!(!harness.host.zapper_active());
}

#[test]
fn req_inp_004_a_zapper_port_shows_the_trigger_and_no_button_to_the_core() {
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
    // Pointer over the centre of the image, right button held.
    let view = {
        let scene = harness.host.scene();
        scene
            .camera
            .build_view(scene.clipping, VIEWPORT[0] / VIEWPORT[1])
    };
    let pointer = project(&view, Vector3::new(128.0, 128.0, 0.0));
    let held = InputSnapshot {
        player_two: InputAction::A,
        pointer,
        pointer_size: VIEWPORT,
        right_button: true,
        ..InputSnapshot::EMPTY
    };
    harness.tick_snapshot(1.0 / 60.0, &held);
    let port_byte = |harness: &LoopHarness| {
        harness
            .host
            .session()
            .unwrap()
            .core()
            .bus()
            .input()
            .port_byte(1)
    };
    assert_eq!(0x00, port_byte(&harness));
    assert!(harness.host.zapper.trigger_held());
    let position = harness.host.zapper.position();
    assert!((127..=129).contains(&position.x), "{position:?}");
    assert!((119..=121).contains(&position.y), "{position:?}");
    // Releasing the button gives 0x10, and J2's A still never reaches the port.
    let released = InputSnapshot {
        right_button: false,
        ..held
    };
    harness.tick_snapshot(1.0 / 60.0, &released);
    assert_eq!(0x10, port_byte(&harness));
}

// ------------------------------------------------- AMD-043 REQ-INP-124: aiming at a bent scene

/// A curvature of 90° left-right, the one the ROM criterion of REQ-INP-124 uses.
fn bend_90() -> SpaceBend {
    SpaceBend::new(90.0, 0.0, 0.0, 0.0, 1.0)
}

/// Where the pointer has to be for the ray to pass through the scene point `target`.
fn pointer_at(view: &CameraView, target: zax_engine3d::vectors::Vec3) -> [f32; 2] {
    project(view, Vector3::new(target.x, target.y, target.z))
}

/// REQ-INP-124, acceptance criterion: for each unbounded vector of REQ-SCN-067, a ray that
/// passes through `B(P)` gives an aimed point within 0.5 scene unit of `P` (with z₀ = z of P).
#[rstest]
#[case(60.0, 0.0, 0.0, false, (256.0, 120.0, 0.0))]
#[case(60.0, 0.0, 0.0, false, (0.0, 120.0, 0.0))]
#[case(60.0, 0.0, 0.0, false, (128.0, 120.0, 0.0))]
#[case(-60.0, 0.0, 0.0, false, (256.0, 120.0, 0.0))]
#[case(0.0, 60.0, 0.0, false, (128.0, 240.0, 0.0))]
#[case(0.0, 60.0, 0.0, false, (128.0, 0.0, 0.0))]
#[case(0.0, 0.0, 60.0, false, (0.0, 120.0, 0.0))]
#[case(0.0, 0.0, 60.0, false, (128.0, 240.0, 0.0))]
#[case(0.0, 0.0, 60.0, true, (0.0, 0.0, 0.0))]
#[case(0.0, 0.0, 60.0, true, (128.0, 240.0, 0.0))]
#[case(90.0, 0.0, 0.0, false, (224.0, 120.0, 0.0))]
fn req_inp_124_a_ray_through_the_bent_image_aims_at_the_point_it_came_from(
    #[case] bend_lr: f32,
    #[case] bend_tb: f32,
    #[case] arc_h: f32,
    #[case] at_edge: bool,
    #[case] point: (f32, f32, f32),
) {
    // The amendment's two heights; RO-5 part 2 made the parameter a fraction, and the inverse of
    // REQ-INP-124 is a function of the pivot alone, so a quarter needs no case of its own here.
    let bend = SpaceBend::new(bend_lr, bend_tb, arc_h, f32::from(u8::from(at_edge)), 1.0);
    let point = zax_engine3d::vectors::Vec3::new(point.0, point.1, point.2);
    let view = default_view(None);
    // The aim plane is z₀ = z of P, which the Zapper offset carries.
    let offset = Vector3::new(0.0, 0.0, point.z);
    let pointer = pointer_at(&view, bend.apply(point));

    let aimed = zapper::try_intersect_bent(&view, pointer, VIEWPORT, offset, 1.0, bend)
        .expect("the ray meets a candidate");
    let distance = ((aimed[0] - point.x).powi(2) + (aimed[1] - point.y).powi(2)).sqrt();
    assert!(
        distance <= 0.5,
        "aimed at ({}, {}), expected ({}, {}): {distance} unit away",
        aimed[0],
        aimed[1],
        point.x,
        point.y
    );
}

/// REQ-INP-124: without a curvature the aim is the one of REQ-INP-063, to the bit — the same
/// function, not a special case that happens to agree.
#[test]
fn req_inp_124_no_curvature_aims_as_req_inp_063() {
    let view = default_view(None);
    for pointer in [[640.0, 360.0], [100.0, 100.0], [1200.0, 700.0], [0.0, 0.0]] {
        let flat = zapper::try_intersect(&view, pointer, VIEWPORT, Vector3::ZERO);
        let bent = zapper::try_intersect_bent(
            &view,
            pointer,
            VIEWPORT,
            Vector3::ZERO,
            1.0,
            SpaceBend::NONE,
        );
        assert_eq!(flat, bent, "pointer {pointer:?}");
    }
}

/// REQ-INP-124: a candidate only outside the aiming window — |x − 128| ≤ 144, |y − 128| ≤ 136 —
/// is handed over **unbounded**, as the off-screen shot of REQ-INP-064.
#[test]
fn req_inp_124_a_candidate_outside_the_window_is_passed_unbounded() {
    assert!(zapper::in_aiming_window([128.0, 128.0], 1.0));
    assert!(zapper::in_aiming_window([-16.0, -8.0], 1.0), "the corner");
    assert!(zapper::in_aiming_window([272.0, 264.0], 1.0));
    assert!(!zapper::in_aiming_window([-17.0, 128.0], 1.0));
    assert!(!zapper::in_aiming_window([128.0, 265.0], 1.0));
    // The window scales with the scene scale of REQ-SCN-002.
    assert!(!zapper::in_aiming_window([-16.0, 128.0], 0.5));

    // A point well outside the window still produces an aim, and it is **not** clamped into the
    // window. A gentle curvature is used on purpose: at 90° the scene wraps far enough that a
    // ray aimed at the far left also crosses the plane inside the window, and REQ-INP-124 then
    // prefers that nearer in-window candidate — which is the requirement, not a failure.
    let bend = SpaceBend::new(30.0, 0.0, 0.0, 0.0, 1.0);
    let view = default_view(None);
    let far = zax_engine3d::vectors::Vec3::new(-220.0, 120.0, 0.0);
    let pointer = pointer_at(&view, bend.apply(far));
    let aimed = zapper::try_intersect_bent(&view, pointer, VIEWPORT, Vector3::ZERO, 1.0, bend)
        .expect("the ray meets a candidate");
    assert!(
        !zapper::in_aiming_window(aimed, 1.0),
        "the only candidate is outside the window and is passed as it is: {aimed:?}"
    );
    assert!(
        (aimed[0] - far.x).abs() <= 1.0,
        "and it is the point aimed at, unbounded: {aimed:?}"
    );

    // No candidate at all — a camera whose ray runs along the aim plane — keeps the last valid
    // position (REQ-INP-064), which `update` does by leaving the state alone.
    let parallel = CameraView {
        forward: Vector3::new(1.0, 0.0, 0.0),
        right: Vector3::new(0.0, 0.0, 1.0),
        up: Vector3::new(0.0, 1.0, 0.0),
        ..default_view(None)
    };
    let mut zapper = ZapperState::new();
    let centre = [VIEWPORT[0] * 0.5, VIEWPORT[1] * 0.5];
    zapper.update(
        true,
        &ZapperInput {
            view: &parallel,
            pointer: centre,
            size: VIEWPORT,
            right_button: false,
            offset: Vector3::ZERO,
            scene_scale: 1.0,
            bend,
        },
    );
    assert!(!zapper.has_position(), "nothing was ever aimed at");
}

/// REQ-INP-124 through the state: the curved aim is recomputed only when one of its inputs
/// changes. REQ-INP-061 refreshes the Zapper once per displayed frame, and the march of the
/// inverse has no business running on a frame where nothing moved.
#[test]
fn req_inp_124_the_curved_aim_is_recomputed_only_on_a_change() {
    let view = default_view(None);
    let bend = bend_90();
    let centre = zax_engine3d::vectors::Vec3::new(128.0, 120.0, 0.0);
    let pointer = pointer_at(&view, bend.apply(centre));

    let mut zapper = ZapperState::new();
    let aim = |pointer: [f32; 2]| ZapperInput {
        view: &view,
        pointer,
        size: VIEWPORT,
        right_button: false,
        offset: Vector3::ZERO,
        scene_scale: 1.0,
        bend,
    };
    zapper.update(true, &aim(pointer));
    assert!(zapper.has_position());
    let first = zapper.position();

    // The same inputs again: the position stands.
    zapper.update(true, &aim(pointer));
    assert_eq!(first, zapper.position());

    // A pointer that moved: a new aim, and it is a different pixel.
    let moved = pointer_at(
        &view,
        bend.apply(zax_engine3d::vectors::Vec3::new(64.0, 120.0, 0.0)),
    );
    zapper.update(true, &aim(moved));
    assert_ne!(first, zapper.position());

    // A reset forgets the memo as well as the position.
    zapper.reset();
    assert!(!zapper.has_position());
    zapper.update(true, &aim(moved));
    assert!(zapper.has_position());
}

/// AMD-046's amendment of REQ-INP-124, read through `SceneState::effective_bend`: in 2D mode
/// with "Bend 2D Mode" off, the aim is the one of REQ-INP-063; with it on, REQ-INP-124 applies.
#[test]
fn req_scn_074_a_flat_two_dimensional_mode_aims_as_at_curvature_zero() {
    let mut scene = zax_render::scene::SceneState {
        two_dimensional_mode: true,
        bend: bend_90(),
        bend_two_dimensional_mode: false,
        ..zax_render::scene::SceneState::default()
    };
    assert!(scene.effective_bend().is_identity(), "REQ-INP-063 applies");

    scene.bend_two_dimensional_mode = true;
    assert_eq!(bend_90(), scene.effective_bend(), "REQ-INP-124 applies");

    // In 3D mode the setting changes nothing either way.
    scene.two_dimensional_mode = false;
    for setting in [false, true] {
        scene.bend_two_dimensional_mode = setting;
        assert_eq!(bend_90(), scene.effective_bend());
    }
}
