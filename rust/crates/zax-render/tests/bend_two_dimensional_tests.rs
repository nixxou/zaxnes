//! AMD-046 REQ-SCN-074: the rectangle of the 2D mode bends like the game scene by default, and
//! the setting "Bend 2D Mode" keeps it flat.
//!
//! The acceptance criteria are written against the exported vertices of the rectangle
//! (`zax_render::scene_export`), so they run without a GL context; the last two render on the GPU.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{GlFixture, SceneFixture};
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::Vec3;
use zax_render::camera::{CameraView, SceneCamera};
use zax_render::geometry;
use zax_render::interop;
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::scene_export::{self, ExportedObject};

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

/// A curvature of 90° left-right, the setting every criterion of REQ-SCN-074 uses.
fn bend_90() -> SpaceBend {
    SpaceBend::new(90.0, 0.0, 0.0, 0.0, 1.0)
}

/// A picture of `index` everywhere, every pixel **opaque**: AMD-050 REQ-CORE-049 puts the
/// opacity bit in bit 6 of each byte, and REQ-SCN-076 does not draw a pixel without it. These
/// tests are about the geometry of the rectangle, so its pixels have to be drawn at all.
fn picture(index: u8) -> Vec<u8> {
    vec![(index & interop::PIXEL_COLOR) | interop::OPAQUE_PIXEL; WIDTH * HEIGHT]
}

fn view(scene: &SceneState) -> CameraView {
    let mut camera = SceneCamera::new();
    camera.snap_to_setpoints();
    camera.build_view(scene.view_clipping(), 4.0 / 3.0)
}

/// A 2D-mode scene with the curvature `bend` and the setting `bend_rectangle`.
fn two_dimensional(bend: SpaceBend, bend_rectangle: bool) -> SceneState {
    let mut scene = SceneState {
        game_running: true,
        two_dimensional_mode: true,
        bend,
        bend_two_dimensional_mode: bend_rectangle,
        ..SceneState::default()
    };
    scene.settings.shadows = false;
    scene
}

/// A 2D-mode frame carrying `picture` and no draw list.
fn frame(picture: &[u8]) -> SceneFrame<'_> {
    SceneFrame {
        draw_list: None,
        nes_picture: picture,
        nes_picture_height: 0,
    }
}

/// The one exported object of a 2D-mode frame: the rectangle.
fn rectangle(scene: &SceneState, picture: &[u8]) -> ExportedObject {
    let mut objects = scene_export::objects(scene, &frame(picture), &view(scene));
    assert_eq!(1, objects.len(), "the rectangle is the only object");
    objects.remove(0)
}

/// REQ-SCN-074, first criterion, setting **active**: every exported vertex of the rectangle, of
/// flat position `P`, is within 0.05 unit of `B(P)`, and the rectangle counts at least 1089
/// distinct vertices (a grid of at least 33 × 33).
#[test]
fn req_scn_074_the_rectangle_bends_and_is_subdivided_when_the_setting_is_on() {
    let picture = picture(0x30);
    let scene = two_dimensional(bend_90(), true);
    let exported = rectangle(&scene, &picture);

    assert!(exported.bent, "the rectangle bends");
    assert!(
        exported.positions.len() >= 1089,
        "at least a 33 × 33 grid, got {}",
        exported.positions.len()
    );

    let transform = bend_90();
    let mut moved = 0;
    for (flat, bent) in exported.unbent.iter().zip(&exported.positions) {
        let expected = transform.apply(*flat);
        assert!(
            Vec3::distance(expected, *bent) <= 0.05,
            "{bent:?} is not B({flat:?}) = {expected:?}"
        );
        if Vec3::distance(*flat, *bent) > 0.5 {
            moved += 1;
        }
    }
    assert!(moved > 0, "the curvature moved the rectangle");

    // "au moins 1 089 sommets distincts": the grid holds no duplicate.
    let mut keys: Vec<(u32, u32)> = exported
        .unbent
        .iter()
        .map(|point| (point.x.to_bits(), point.y.to_bits()))
        .collect();
    keys.sort_unstable();
    let total = keys.len();
    keys.dedup();
    assert_eq!(total, keys.len(), "every vertex of the grid is its own");
}

/// REQ-SCN-074, first criterion, setting **inactive**: every exported vertex is in the plane
/// `z = 0`, at its flat position to within 10⁻³ unit, **whatever** the settings of REQ-SCN-066.
#[test]
fn req_scn_074_the_rectangle_stays_flat_when_the_setting_is_off() {
    let picture = picture(0x30);
    for bend in [
        bend_90(),
        SpaceBend::new(180.0, -150.0, 120.0, 1.0, 1.0),
        SpaceBend::new(-30.0, 30.0, -30.0, 0.0, 1.0),
    ] {
        let scene = two_dimensional(bend, false);
        let exported = rectangle(&scene, &picture);
        assert!(!exported.bent, "the rectangle does not bend");
        for (flat, placed) in exported.unbent.iter().zip(&exported.positions) {
            assert!(placed.z.abs() <= 1e-3, "z = {} is not the plane", placed.z);
            assert!(
                Vec3::distance(*flat, *placed) <= 1e-3,
                "{placed:?} is not the flat {flat:?}"
            );
        }
    }
}

/// REQ-SCN-074, first criterion, last clause: the visible NES rows and columns are the same with
/// the setting on and off, for the default margins — which is the order clause of REQ-SCN-068
/// read through the rectangle: the clipping is judged on the position **before** the curvature.
#[test]
fn req_scn_074_the_visible_rows_do_not_depend_on_the_setting() {
    let picture = picture(0x30);
    let rect = SceneState::default().visible_rectangle();
    // The NES rows the rectangle shows: those whose centre line falls both inside the
    // rectangle's **flat** span — the positions the fragment shader judges (REQ-SCN-036) — and
    // inside R. The curvature must not enter this count, which is the clause under test.
    let visible = |exported: &ExportedObject| {
        let lowest = exported
            .unbent
            .iter()
            .fold(f32::MAX, |lowest, point| lowest.min(point.y));
        let highest = exported
            .unbent
            .iter()
            .fold(f32::MIN, |highest, point| highest.max(point.y));
        (0..240)
            .filter(|row| {
                #[allow(clippy::cast_precision_loss)] // 0 to 239.
                let y = geometry::SCENE_HEIGHT - (*row as f32) - 0.5;
                y >= rect.min_y && y <= rect.max_y && y >= lowest && y <= highest
            })
            .count()
    };

    let on = rectangle(&two_dimensional(bend_90(), true), &picture);
    let off = rectangle(&two_dimensional(bend_90(), false), &picture);
    assert_eq!(visible(&on), visible(&off));
    assert!(visible(&on) > 0, "some rows are visible at all");

    // And the flat positions themselves span the same rectangle in both cases, which is what the
    // clipping reads.
    let span = |exported: &ExportedObject| {
        let min = exported
            .unbent
            .iter()
            .fold(f32::MAX, |lowest, point| lowest.min(point.y));
        let max = exported
            .unbent
            .iter()
            .fold(f32::MIN, |highest, point| highest.max(point.y));
        (min.to_bits(), max.to_bits())
    };
    assert_eq!(span(&on), span(&off));
    assert_eq!(on.clipped, off.clipped);
}

/// REQ-SCN-074: in 3D mode the setting has no effect — no exported vertex moves when it is
/// toggled, with or without a curvature.
#[test]
fn req_scn_074_the_setting_has_no_effect_in_three_dimensional_mode() {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(
            common::solid(1, 0),
            1,
            zax_profiles::enums::ProfileLayer::L1,
        )
        .rect(common::solid(1, 0), 10, 12, 4, 4)
        .build();
    built.scene.settings.shadows = false;
    let frame = SceneFrame::with_draw_list(&built.draw_list);

    for bend in [SpaceBend::NONE, bend_90()] {
        built.scene.bend = bend;
        built.scene.bend_two_dimensional_mode = true;
        let on = scene_export::objects(&built.scene, &frame, &view(&built.scene));
        built.scene.bend_two_dimensional_mode = false;
        let off = scene_export::objects(&built.scene, &frame, &view(&built.scene));

        assert!(!on.is_empty(), "the tile is exported");
        assert_eq!(on.len(), off.len());
        for (on, off) in on.iter().zip(&off) {
            assert_eq!(on.bent, off.bent, "3D mode ignores the setting");
            assert_eq!(on.positions, off.positions);
            assert_eq!(on.normals, off.normals);
        }
    }
}

/// On the GPU: with the setting on, a bent 2D-mode frame is a different picture from the flat
/// one and the rectangle is one draw call of 2 × 32 × 32 triangles; with the setting off, it is
/// the flat picture again and the rectangle is back to two triangles.
#[test]
fn req_scn_074_the_setting_decides_the_picture_and_the_mesh() {
    let gl = GlFixture::create(320, 300);
    let mut renderer = gl.renderer();
    let picture = picture(0x30);

    let flat_scene = two_dimensional(SpaceBend::NONE, true);
    let flat = gl.render(&mut renderer, &flat_scene, &frame(&picture));
    let flat_draws = renderer.statistics().draw_calls;
    assert_eq!(2, renderer.statistics().triangles, "a single quad");

    let bent_scene = two_dimensional(bend_90(), true);
    let bent = gl.render(&mut renderer, &bent_scene, &frame(&picture));
    assert_eq!(
        flat_draws,
        renderer.statistics().draw_calls,
        "still one draw call: the rectangle is subdivided, not split"
    );
    assert_eq!(
        2 * 32 * 32,
        renderer.statistics().triangles,
        "32 × 32 cells of two triangles"
    );

    let off_scene = two_dimensional(bend_90(), false);
    let off = gl.render(&mut renderer, &off_scene, &frame(&picture));
    assert_eq!(
        2,
        renderer.statistics().triangles,
        "the setting off puts the single quad back"
    );

    let differing = |left: &common::Capture, right: &common::Capture| {
        (0..left.width)
            .flat_map(|x| (0..left.height).map(move |y| (x, y)))
            .filter(|&(x, y)| left.at(x, y) != right.at(x, y))
            .count()
    };
    assert!(
        differing(&flat, &bent) > 0,
        "the curvature changed the picture"
    );
    assert_eq!(
        0,
        differing(&flat, &off),
        "the setting off gives the picture of no curvature at all"
    );
}
