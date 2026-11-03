//! AMD-055 REQ-SCN-084, *définition du sol*: the half of the floor reflections that is decided
//! on the CPU, where the placement is.
//!
//! The requirement asks for **two** tests at once — a face whose scene-space normal before
//! curvature has `+Y ≥ 0,9`, belonging to an instance whose bounding box in the floor plane
//! measures at least `6 × échelle de scène` in both dimensions — and the invariants of §2.5 are
//! what forces them to stay two:
//!
//! - **Variante R-s** puts a block of 8 × 8 × 8 units and the same block at
//!   `scale = (1 ; 1 ; 0,5)` side by side: the first reflects, the second must not. Both present
//!   the same pixels of the same depth buffer with the same normal, so no reconstruction from
//!   depth can tell them apart — the extent has to be measured on the instance, on the CPU.
//! - **Variante R-m** is the same 8 × 8-tile shape *without* rotation: its `xz` box measures
//!   64 × 8 and therefore **passes** the extent test, and not one pixel of its front face (scene
//!   normal (0, 0, −1)) may be modified — so the normal has to be tested per fragment, and a
//!   per-instance mask alone would reflect the wall.
//!
//! Everything measured here is the extent half and the predicate of the normal half; the pass
//! that puts them together on the GPU is step 6b of this lot. The measurements run on
//! `scene_export::objects`, the same `FramePlan::collect` the GPU path runs, without a GL context.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use common::{assert_close, solid, SceneFixture};
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::ProfileLayer;
use zax_render::numerics::Vector3;
use zax_render::scene_export::{self, ExportedObject};
use zax_render::style;

/// A shape of `columns` × `rows` full tiles built at depth `depth`, drawn in a scene layer: the
/// floor, the wall and the block of the §2.5 variants are all this shape under another transform.
fn block(columns: i32, rows: i32, depth: i32) -> common::Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x10)
        .pattern_block(solid(1, 0), columns, rows, depth, ProfileLayer::L2, 1.0)
        .rect(solid(1, 0), 12, 11, columns, rows)
        .build()
}

/// The same shape with every instance turned by `rotation` and scaled by `scale`.
fn placed(mut built: common::Built, rotation: Vec3, scale: Vec3) -> common::Built {
    for id in built.instances(|_| true) {
        let instance = built.instance_mut(id);
        instance.rotation = rotation;
        instance.scale = scale;
    }
    built.rebuild_draw_list();
    built
}

const NO_ROTATION: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const FLAT_ON_THE_GROUND: Vec3 = Vec3::new(90.0, 0.0, 0.0);
const UNSCALED: Vec3 = Vec3::new(1.0, 1.0, 1.0);

/// The objects of the frame as the renderer would place them.
fn export(built: &mut common::Built) -> Vec<ExportedObject> {
    built.scene.camera.snap_to_setpoints();
    let view = built
        .scene
        .camera
        .build_view(built.scene.view_clipping(), 4.0 / 3.0);
    scene_export::objects(&built.scene, &built.frame(), &view)
}

/// The object's bounding box in the scene frame before the curvature.
fn scene_box(object: &ExportedObject) -> ([f32; 3], [f32; 3]) {
    boxes(std::slice::from_ref(object))
}

/// The union of the boxes of `objects`, in the scene frame before the curvature — which for the
/// objects of one shape is the box REQ-SCN-084 measures, because that box is the **instance's**
/// and a shape of many tiles is drawn as one object per tile record.
fn boxes(objects: &[ExportedObject]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for object in objects {
        for position in &object.unbent {
            for (axis, value) in [position.x, position.y, position.z].into_iter().enumerate() {
                min[axis] = min[axis].min(value);
                max[axis] = max[axis].max(value);
            }
        }
    }
    (min, max)
}

/// The one floor decision the objects of a single-shape frame share. The decision is per
/// instance, so a shape whose tiles are drawn one by one must answer the same for every one of
/// them — which this asserts on the way.
#[track_caller]
fn floor_flag(exported: &[ExportedObject]) -> bool {
    assert!(!exported.is_empty(), "the shape is drawn");
    let first = exported[0].floor;
    assert!(
        exported.iter().all(|object| object.floor == first),
        "the decision is per instance, so every tile of the shape shares it"
    );
    first
}

/// Whether any object of the frame carries a face the normal half of the definition accepts.
fn any_floor_face(exported: &[ExportedObject], local_camera: bool) -> bool {
    exported
        .iter()
        .any(|object| has_a_floor_face(object, local_camera))
}

/// Whether any object of the frame carries a face of scene normal `wanted`.
fn any_face(exported: &[ExportedObject], wanted: Vec3) -> bool {
    exported.iter().any(|object| has_face(object, wanted))
}

/// Whether the object carries a face the normal half of the definition accepts.
fn has_a_floor_face(object: &ExportedObject, local_camera: bool) -> bool {
    object
        .normals
        .iter()
        .any(|normal| style::is_floor_normal([normal.x, normal.y, normal.z], local_camera))
}

/// Whether the object carries a face of scene normal `wanted`.
fn has_face(object: &ExportedObject, wanted: Vec3) -> bool {
    object.normals.iter().any(|normal| {
        (normal.x - wanted.x).abs() < 1e-4
            && (normal.y - wanted.y).abs() < 1e-4
            && (normal.z - wanted.z).abs() < 1e-4
    })
}

/// The floor of the close-up R: 8 × 8 full tiles of `D` = 8 (64 × 64 units) turned by
/// `rot = (90, 0, 0)`, so its visible face has the scene normal (0, 1, 0) and its `xz` box
/// measures 64 × 64 — "c'est donc une surface de sol".
#[test]
fn req_scn_084_the_floor_of_the_close_up_is_a_floor_surface() {
    let mut built = placed(block(8, 8, 8), FLAT_ON_THE_GROUND, UNSCALED);
    let exported = export(&mut built);

    let (min, max) = boxes(&exported);
    assert_close(64.0, f64::from(max[0] - min[0]), 1e-3);
    assert_close(64.0, f64::from(max[2] - min[2]), 1e-3);
    assert_close(8.0, f64::from(max[1] - min[1]), 1e-3);

    assert!(
        floor_flag(&exported),
        "the xz box measures 64 × 64, both over 6"
    );
    assert!(
        any_face(&exported, Vec3::new(0.0, 1.0, 0.0)),
        "the rotation turns the front face up"
    );
    assert!(any_floor_face(&exported, false));
}

/// The box is the **instance's**, not the tile's. A floor of 8 × 1 full tiles at
/// `scale = (0,5 ; 1 ; 1)` has tiles 4 units wide — under the threshold of 6 — and an instance 32
/// units wide, which is over it: only the per-instance accumulation gives the right answer.
#[test]
fn req_scn_084_the_box_is_the_instances_and_not_the_tiles() {
    let mut built = placed(block(8, 1, 8), NO_ROTATION, Vec3::new(0.5, 1.0, 1.0));
    let exported = export(&mut built);
    assert_eq!(8, exported.len(), "eight tile records, drawn one by one");

    let (min, max) = scene_box(&exported[0]);
    assert_close(4.0, f64::from(max[0] - min[0]), 1e-3);
    assert!(
        !style::is_floor_extent(min, max, false, 1.0),
        "one tile alone is 4 × 8 and would not reflect"
    );

    let (min, max) = boxes(&exported);
    assert_close(32.0, f64::from(max[0] - min[0]), 1e-3);
    assert!(floor_flag(&exported), "the instance is 32 × 8 and does");
}

/// Variante R-s, both halves of it: the block of 8 × 8 × 8 units is a floor, the same block at
/// `scale = (1 ; 1 ; 0,5)` — 4 units on `z` — is not, and neither is the same shape built at
/// `D` = 3, the amendment's other example.
#[test]
fn req_scn_084_the_block_of_eight_units_reflects_and_a_shallower_one_does_not() {
    let mut full = block(1, 1, 8);
    let exported = export(&mut full);
    let (min, max) = boxes(&exported);
    for axis in 0..3 {
        assert_close(8.0, f64::from(max[axis] - min[axis]), 1e-3);
    }
    assert!(floor_flag(&exported), "8 × 8 in xz");
    assert!(
        any_face(&exported, Vec3::new(0.0, 1.0, 0.0)),
        "the top face of the block faces up"
    );

    // "la même shape … avec `scale = (1 ; 1 ; 0,5)`, mesure … 4 unités en `z` et ne réfléchit
    // pas". The normal of its top face is unchanged — which is why the test cannot be made on
    // the depth buffer.
    let mut halved = placed(block(1, 1, 8), NO_ROTATION, Vec3::new(1.0, 1.0, 0.5));
    let exported = export(&mut halved);
    let (min, max) = boxes(&exported);
    assert_close(4.0, f64::from(max[2] - min[2]), 1e-3);
    assert!(
        !floor_flag(&exported),
        "4 units on z is under the threshold of 6"
    );
    assert!(any_face(&exported, Vec3::new(0.0, 1.0, 0.0)));

    // "la même shape avec `D` = 3 … ne réfléchit pas": the depth is the object's own, and it
    // reaches the box through the mesh, which runs −D/2 … +D/2 (REQ-E3D-504).
    let mut shallow = block(1, 1, 3);
    let exported = export(&mut shallow);
    let (min, max) = boxes(&exported);
    assert_close(3.0, f64::from(max[2] - min[2]), 1e-3);
    assert!(
        !floor_flag(&exported),
        "3 units on z is under the threshold of 6"
    );
}

/// Variante R-m: the wall is the same 8 × 8-tile shape without rotation. Its `xz` box measures
/// 64 × 8, so it **passes** the extent test — and not one pixel of its front face may be
/// modified, which only the per-fragment normal test can deliver.
#[test]
fn req_scn_084_the_wall_passes_the_extent_and_is_excluded_by_its_face() {
    let mut built = block(8, 8, 8);
    let exported = export(&mut built);

    let (min, max) = boxes(&exported);
    assert_close(64.0, f64::from(max[0] - min[0]), 1e-3);
    assert_close(64.0, f64::from(max[1] - min[1]), 1e-3);
    assert_close(8.0, f64::from(max[2] - min[2]), 1e-3);
    assert!(
        floor_flag(&exported),
        "64 × 8 in xz: a per-instance mask alone would reflect the wall"
    );

    // The invariant: no pixel of the face of scene normal (0, 0, −1) is modified.
    assert!(
        any_face(&exported, Vec3::new(0.0, 0.0, -1.0)),
        "the front face"
    );
    assert!(!style::is_floor_normal([0.0, 0.0, -1.0], false));
    // What the wall does carry is the 64 × 8 strip of its top face, which is a floor face: the
    // exclusion of the front face is the normal's doing and nothing else's.
    assert!(any_floor_face(&exported, false));
}

/// The local camera of REQ-SCN-009, as v1.4 of the amendment reads it (Q-096): the NES plane is
/// laid flat and stands in for the floor, so the vertical becomes the image plane's normal
/// `(0, 0, −1)` and the footprint is measured in `xy`.
///
/// The discriminating pair is **one** shape at **one** depth: 8 × 8 tiles of `D` = 3 is a floor
/// under the local camera (its `xy` box is 64 × 64) and not under the ordinary one (its `xz` box
/// is 64 × 3); turned flat by `rot = (90, 0, 0)` it is exactly the other way round.
#[test]
fn req_scn_084_the_local_camera_lays_the_image_plane_flat() {
    let mut plane = block(8, 8, 3);
    assert!(
        !floor_flag(&export(&mut plane)),
        "64 × 3 in xz is not a floor for an ordinary camera"
    );
    plane.scene.camera.local_camera = true;
    let exported = export(&mut plane);
    assert!(
        floor_flag(&exported),
        "64 × 64 in xy is a floor for the local camera"
    );
    assert!(
        any_face(&exported, Vec3::new(0.0, 0.0, -1.0)) && any_floor_face(&exported, true),
        "and the face turned towards the camera is the floor face"
    );

    let mut turned = placed(block(8, 8, 3), FLAT_ON_THE_GROUND, UNSCALED);
    assert!(
        floor_flag(&export(&mut turned)),
        "64 × 64 in xz is a floor for an ordinary camera"
    );
    turned.scene.camera.local_camera = true;
    assert!(
        !floor_flag(&export(&mut turned)),
        "64 × 3 in xy is not a floor for the local camera"
    );
    // The clause holds "quelle que soit l'inclinaison de la caméra": it is the profile's flag
    // that swaps the vertical, not the camera's angles, so tilting the camera changes nothing.
    turned.scene.camera.current_mut().angles = Vector3::new(-60.0, 0.0, 0.0);
    assert!(!floor_flag(&export(&mut turned)));
}

/// Q-093 (a), answered: the backdrop carries no instance and is measured as an object of its own.
/// At its default size of 256 × 240 × 1 its `z` dimension is 1, under the threshold of 6, so by
/// default it is not a floor; Lua deepening it to 6 units makes it one.
#[test]
fn req_scn_084_the_backdrop_is_an_object_of_its_own() {
    let mut built = block(1, 1, 8);
    built.scene.backdrop.enabled = true;

    let exported = export(&mut built);
    let backdrop = backdrop_of(&exported);
    let (min, max) = scene_box(backdrop);
    assert_close(256.0, f64::from(max[0] - min[0]), 1e-3);
    assert_close(1.0, f64::from(max[2] - min[2]), 1e-3);
    assert!(!backdrop.floor, "256 × 1 in xz: z is under the threshold");
    // And the block beside it is unaffected by the backdrop sharing the bucket.
    assert_eq!(1, exported.iter().filter(|object| object.floor).count());

    built.scene.backdrop.size.z = 6.0;
    let exported = export(&mut built);
    assert!(backdrop_of(&exported).floor, "256 × 6 in xz reaches it");
}

/// The backdrop among the exported objects: the one 256 units wide (REQ-SCN-048's default size).
fn backdrop_of(exported: &[ExportedObject]) -> &ExportedObject {
    exported
        .iter()
        .find(|object| {
            let (min, max) = scene_box(object);
            (max[0] - min[0]) > 200.0
        })
        .expect("the backdrop is drawn")
}

/// REQ-SCN-084's order clause, the half of it that is decided here: the box is the one of the
/// scene frame **before** curvature, so a bent frame marks exactly the same instances. (That the
/// pass then works on the *rendered*, bent depth is step 6b's subject.)
#[test]
fn req_scn_080_the_floor_definition_is_measured_before_the_curvature() {
    let flags = |bend: SpaceBend| -> Vec<bool> {
        let mut built = placed(block(8, 8, 8), FLAT_ON_THE_GROUND, UNSCALED);
        built.scene.bend = bend;
        export(&mut built)
            .iter()
            .map(|object| object.floor)
            .collect()
    };

    let flat = flags(SpaceBend::default());
    assert_eq!(64, flat.len(), "the floor's sixty-four tile records");
    assert!(flat.iter().all(|floor| *floor));
    assert_eq!(flat, flags(SpaceBend::new(60.0, 0.0, 0.0, false, 1.0)));
    assert_eq!(flat, flags(SpaceBend::new(0.0, 90.0, 0.0, false, 1.0)));
}

/// REQ-SCN-080, 4th bullet, and the scope clause of REQ-SCN-084: the transparent objects do not
/// carry reflections and the HUD is never a floor surface — both are drawn after the pass, so the
/// marking never reaches them whatever their shape.
#[test]
fn req_scn_080_a_transparent_object_and_the_hud_are_never_floor_surfaces() {
    // The same floor of the close-up R, made translucent: it lands in the transparent bucket.
    let mut translucent = placed(
        SceneFixture::new()
            .background(0x0F)
            .palette(1, 0x10)
            .pattern_block(solid(1, 0), 8, 8, 8, ProfileLayer::L2, 0.5)
            .rect(solid(1, 0), 12, 11, 8, 8)
            .build(),
        FLAT_ON_THE_GROUND,
        UNSCALED,
    );
    let exported = export(&mut translucent);
    assert!(!exported.is_empty(), "the translucent floor is drawn");
    assert!(exported.iter().all(|object| !object.floor));

    // And the same shape on the interface layer, which is the HUD bucket (REQ-SCN-044).
    let mut hud = placed(
        SceneFixture::new()
            .background(0x0F)
            .palette(1, 0x10)
            .pattern_block(solid(1, 0), 8, 8, 8, ProfileLayer::Ui, 1.0)
            .rect(solid(1, 0), 12, 11, 8, 8)
            .build(),
        FLAT_ON_THE_GROUND,
        UNSCALED,
    );
    let exported = export(&mut hud);
    assert!(!exported.is_empty(), "the HUD shape is drawn");
    assert!(exported.iter().all(|object| !object.floor));
    assert!(exported.iter().any(|object| object.hud));
}

/// "Un sol de jeu, qui s'étend sur de nombreuses tiles, dépasse le seuil dans les deux dimensions
/// dès que sa profondeur atteint 6 unités": a row of 8 tiles reaches the threshold on `x` by its
/// width alone, and crosses it on `z` exactly at `D` = 6.
#[test]
fn req_scn_084_a_game_floor_crosses_the_threshold_at_a_depth_of_six() {
    for (depth, floor) in [(5, false), (6, true), (8, true)] {
        let mut built = block(8, 1, depth);
        let exported = export(&mut built);
        let (min, max) = boxes(&exported);
        assert_close(64.0, f64::from(max[0] - min[0]), 1e-3);
        assert_close(f64::from(depth), f64::from(max[2] - min[2]), 1e-3);
        assert_eq!(floor, floor_flag(&exported), "a floor of depth {depth}");
    }
}
