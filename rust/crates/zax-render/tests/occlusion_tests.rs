//! The occlusion transparency of SPEC-06 REQ-SCN-022: in FPV, an opaque tile of another shape
//! that crosses the segment from the eye to the followed target is drawn at 0.5 for that frame
//! (port of `OcclusionTests.cs`).

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;
use zax_render::camera::CameraMode;
use zax_render::geometry;
use zax_render::numerics::Vector3;
use zax_render::occlusion::{self, Occlusion};
use zax_render::scene::SceneState;

#[test]
fn req_scn_022_the_opacity_of_an_occluder_is_one_half() {
    assert!((geometry::OCCLUDER_ALPHA - 0.5).abs() < 5e-6);
}

#[test]
fn a_segment_that_crosses_a_box_is_detected_and_one_that_stops_short_is_not() {
    let min = Vector3::new(-1.0, -1.0, 4.0);
    let max = Vector3::new(1.0, 1.0, 6.0);

    assert!(occlusion::segment_hits_box(
        Vector3::ZERO,
        Vector3::new(0.0, 0.0, 10.0),
        min,
        max
    ));
    assert!(!occlusion::segment_hits_box(
        Vector3::ZERO,
        Vector3::new(0.0, 0.0, 3.0),
        min,
        max
    ));
    assert!(!occlusion::segment_hits_box(
        Vector3::new(5.0, 0.0, 0.0),
        Vector3::new(5.0, 0.0, 10.0),
        min,
        max
    ));

    // A segment that lies inside the box meets it.
    assert!(occlusion::segment_hits_box(
        Vector3::new(0.0, 0.0, 4.5),
        Vector3::new(0.0, 0.0, 5.5),
        min,
        max
    ));
}

#[test]
fn req_scn_022_the_test_is_off_outside_fpv_and_without_a_followed_shape() {
    let mut scene = SceneState::default();
    scene.camera.snap_to_setpoints();
    let view = scene.camera.build_view(scene.clipping, 1.0);

    assert!(!Occlusion::for_scene(&scene, &view).enabled());

    scene.camera.set_mode(CameraMode::Fpv);
    assert!(!Occlusion::for_scene(&scene, &view).enabled());
}

fn fpv(scene: &mut SceneState, target: Vector3, distance: f32) {
    scene.settings.shadows = false;
    scene.camera.set_mode(CameraMode::Fpv);
    scene.camera.current_mut().target = target;
    scene.camera.current_mut().set_distance(distance);
    scene.camera.current_mut().angles = Vector3::ZERO;
    scene.camera.snap_to_setpoints();
}

#[test]
fn req_scn_022_a_wall_in_front_of_the_followed_shape_becomes_transparent() {
    let gl = GlFixture::create(128, 128);
    let mut renderer = gl.renderer();

    // A background wall in L1 at z = 0 and a sprite "character" behind it in L2.
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(21, 0x16)
        .pattern(solid(1, 0), 4, ProfileLayer::L1)
        .rect(solid(1, 0), 12, 12, 6, 6)
        .sprite(solid(1, 3), 112, 112, 1, false)
        .build();
    fpv(&mut built.scene, Vector3::new(116.0, 128.0, 20.0), 120.0);

    // Without a followed shape nothing fades.
    gl.render_built(&mut renderer, &built);
    assert_eq!(0, renderer.statistics().occluders);

    // Following the sprite makes the wall tiles on the line of sight transparent.
    let published = built.engine.published_frame().expect("published frame");
    built.scene.followed_shape = published.shapes().last().copied();
    gl.render_built(&mut renderer, &built);

    let statistics = renderer.statistics();
    assert!(
        statistics.occluders > 0,
        "the wall should hide the followed shape"
    );
    assert!(
        statistics.transparent_tiles >= statistics.occluders,
        "every occluder is drawn with the transparent objects"
    );
}

#[test]
fn req_scn_022_the_tiles_of_the_followed_shape_itself_never_fade() {
    let gl = GlFixture::create(96, 96);
    let mut renderer = gl.renderer();

    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 4, ProfileLayer::L1)
        .rect(solid(1, 0), 12, 12, 4, 4)
        .build();
    fpv(&mut built.scene, Vector3::new(128.0, 128.0, 0.0), 150.0);

    // The only shape of the frame is the followed one, so nothing can occlude it.
    let published = built.engine.published_frame().expect("published frame");
    built.scene.followed_shape = published.shapes().first().copied();
    gl.render_built(&mut renderer, &built);

    assert_eq!(0, renderer.statistics().occluders);
}
