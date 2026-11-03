//! AMD-055 REQ-SCN-082: the pixel shadows.
//!
//! On the faces of **opaque tile** meshes, the shadow of REQ-SCN-030 and REQ-SCN-031 is read not
//! at the fragment but at the centre of the fragment's **voxel**, so it changes by squares of one
//! voxel — the pixel grid of the game. The snapping happens in the mesh's local frame of
//! REQ-E3D-504, before the deformation, the object's transform and the curvature; the displaced
//! point is then placed like a vertex and the bias of REQ-SCN-031 is applied **to it**.
//!
//! What the criterion of the amendment discriminates is its **second** bullet: the shadows of
//! REQ-SCN-030 are already hard, so the sharpness of an edge tells the two styles apart not at
//! all. What tells them apart is that with the effect on **no voxel cell of the floor holds both
//! lit and shadowed pixels**, and with it off at least one cell of each edge does. That is what
//! the two GPU tests below measure, cell by cell, by projecting each cell of the floor's local
//! voxel grid through the frame's own camera.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

use common::{solid, GlFixture, SceneFixture};
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::ProfileLayer;
use zax_render::camera::CameraView;
use zax_render::numerics::{Matrix4x4, Vector3};
use zax_render::shaders;
use zax_render::style::{self, StyleSettings};

/// A small whole number as a coordinate. The fixtures below count in voxels, so every value is
/// far inside the exactly representable range.
#[allow(clippy::cast_precision_loss)]
fn at(value: i32) -> f32 {
    value as f32
}

// --------------------------------------------------------------- the CPU twin

/// REQ-SCN-082 point 2: on a GREEDY face the two axes that move are exactly the two whose
/// `|n_a|` is 0, and the third — the one the normal points along — is left alone, which is what
/// keeps the displaced point **on the face**.
#[test]
fn req_scn_082_a_greedy_face_snaps_the_two_axes_of_its_plane() {
    let p = [1.3_f32, -2.8, 3.5];
    // Normal ±z: x and y move, z is untouched.
    for normal in [[0.0, 0.0, 1.0], [0.0, 0.0, -1.0_f32]] {
        let snapped = style::voxel_center(p, normal, 0.0);
        assert_eq!([1.5, -2.5, 3.5], snapped, "normal {normal:?}");
    }
    // Normal ±x: y and z move, x is untouched.
    for normal in [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0_f32]] {
        let snapped = style::voxel_center(p, normal, 0.0);
        assert_eq!([1.3, -2.5, 3.5], snapped, "normal {normal:?}");
    }
    // Normal ±y: x and z move, y is untouched.
    for normal in [[0.0, 1.0, 0.0], [0.0, -1.0, 0.0_f32]] {
        let snapped = style::voxel_center(p, normal, 0.0);
        assert_eq!([1.5, -2.8, 3.5], snapped, "normal {normal:?}");
    }
}

/// REQ-SCN-082 point 2, the tie rule: the two axes of smallest `|n_a|`, **ties broken by the
/// smaller index** in the order `x`, `y`, `z`. So on a face at 45° between `x` and `y` the two
/// smallest are `z` (0) and then `x` — the smaller index of the tie — and `y` is the one left
/// alone. A MARCHING surface always has two axes to snap, so there is no plate where the effect
/// vanishes (REQ-E3D-522).
#[test]
fn req_scn_082_the_tie_is_broken_by_the_smaller_index() {
    let p = [1.3_f32, -2.8, 3.2];
    let half = std::f32::consts::FRAC_1_SQRT_2;

    // 45° between x and y: x and z move, y stays.
    let snapped = style::voxel_center(p, [half, half, 0.0], 0.0);
    assert_eq!([1.5, -2.8, 3.5], snapped);

    // 45° between y and z: x and y move, z stays.
    let snapped = style::voxel_center(p, [0.0, half, half], 0.0);
    assert_eq!([1.5, -2.5, 3.2], snapped);

    // The degenerate normal of a zero-area triangle (REQ-E3D-505): every `|n_a|` ties at 0, so
    // the two smallest are x and y and the point still lands on a voxel centre in two axes.
    let snapped = style::voxel_center(p, [0.0, 0.0, 0.0], 0.0);
    assert_eq!([1.5, -2.5, 3.2], snapped);

    // An oblique MARCHING normal with no tie at all: the largest component is y, so y stays.
    let snapped = style::voxel_center(p, [0.3, 0.9, 0.31], 0.0);
    assert_eq!([1.5, -2.8, 3.5], snapped);
}

/// REQ-SCN-082 point 2 on `z`: the voxel edges of the local frame are offset by `D/2`
/// (REQ-E3D-504), so an **even** construction depth snaps to "whole part + 0,5" as `x` and `y` do,
/// and an **odd** one snaps to the whole coordinates. Only `D/2 mod 1` matters, which is what
/// `voxel_z_offset` reduces a depth to.
#[test]
fn req_scn_082_the_z_axis_follows_the_construction_depth() {
    for depth in [0, 2, 4, 8, 96] {
        assert_eq!(0.0, style::voxel_z_offset(depth), "depth {depth}");
    }
    for depth in [1, 3, 5, 7, 95] {
        assert_eq!(0.5, style::voxel_z_offset(depth), "depth {depth}");
    }

    // A face of normal ±x, so `z` is one of the two axes that move. Even depth: the centres are
    // at the half coordinates.
    let even = style::voxel_center([0.0, 0.0, 3.2], [1.0, 0.0, 0.0], style::voxel_z_offset(8));
    assert_eq!(3.5, even[2]);
    // Odd depth: the voxel of a `sizeZ` = 1 plate spans [−0.5, 0.5], so its centre is 0.
    let odd = style::voxel_center([0.0, 0.0, 0.2], [1.0, 0.0, 0.0], style::voxel_z_offset(1));
    assert_eq!(0.0, odd[2]);
    let odd = style::voxel_center([0.0, 0.0, -0.2], [1.0, 0.0, 0.0], style::voxel_z_offset(1));
    assert_eq!(0.0, odd[2]);
    // And the whole plate of depth 1 reads one single voxel centre, whatever the point.
    for z in [-0.49_f32, -0.2, 0.0, 0.3, 0.49] {
        let centre = style::voxel_center([0.0, 0.0, z], [1.0, 0.0, 0.0], style::voxel_z_offset(1));
        assert_eq!(0.0, centre[2], "z = {z}");
    }
}

/// REQ-SCN-082, Notes: "le point d'évaluation se déplace de 0,7 unité au plus le long de la
/// face" — half a voxel on each of the two axes, so `√2 / 2 = 0.707`, and never off the face.
#[test]
fn req_scn_082_the_point_moves_at_most_seven_tenths_of_a_unit() {
    let mut worst = 0.0_f32;
    for step in 0..40 {
        for other in 0..40 {
            let p = [at(step) * 0.05 - 1.0, at(other) * 0.05 - 1.0, 0.5];
            let snapped = style::voxel_center(p, [0.0, 0.0, -1.0], 0.0);
            // The axis the normal points along never moves: the point stays on the face.
            assert_eq!(p[2], snapped[2]);
            let moved = ((snapped[0] - p[0]).powi(2) + (snapped[1] - p[1]).powi(2)).sqrt();
            worst = worst.max(moved);
        }
    }
    assert!(
        worst <= style::VOXEL_CENTRE_REACH + 1e-6,
        "the point moved {worst} units"
    );
    assert!(worst > 0.6, "the fixture must reach the corner of a voxel");
}

/// The grid of REQ-E3D-504 is the grid of REQ-SCN-081 with `N` = 1 (REQ-SCN-082, Notes): a voxel
/// centre is half a unit from the two cell edges that surround it, so the snapped coordinate is
/// always at distance 0.5 from a whole coordinate.
#[test]
fn req_scn_082_the_cells_are_those_of_the_voxel_grid_at_one() {
    for step in -80..80 {
        let x = at(step) * 0.1;
        let snapped = style::voxel_center([x, 0.25, 0.5], [0.0, 0.0, 1.0], 0.0)[0];
        let distance = (snapped - snapped.round()).abs();
        assert!((distance - 0.5).abs() < 1e-5, "x = {x} gave {snapped}");
        assert!(
            (snapped - x).abs() <= 0.5 + 1e-6,
            "x = {x} left its own voxel for {snapped}"
        );
    }
}

// --------------------------------------------------------------- the variants

/// The brief's point 1: no Classic variant carries anything of the pixel shadows. Preprocesses
/// the assembled source the way the GLSL compiler would, over every variant that leaves the two
/// AMD-055 defines out — the shape `req_scn_081_no_classic_variant_carries_anything_of_the_grid`
/// established, with the same reason: a `#ifdef` line is text, so the files differ where the
/// compiled code does not.
#[test]
fn req_scn_082_no_classic_variant_carries_anything_of_the_pixel_shadow() {
    const SHADOW_ONLY: [&str; 5] = [
        "zaxVoxelShadowPoint",
        "zaxVoxelCenter",
        "uVoxelZOffset",
        "vModelLinear",
        "vDeformTile",
    ];

    for embedded in [false, true] {
        for solid in [false, true] {
            for srgb_palette in [false, true] {
                for bend in [false, true] {
                    for centroid in [false, true] {
                        for voxel_grid in [false, true] {
                            let source = shaders::scene(shaders::SceneVariant {
                                solid,
                                srgb_palette,
                                bend,
                                centroid,
                                voxel_grid,
                                pixel_shadow: false,
                            });
                            for (fragment, stage) in
                                [(false, &source.vertex), (true, &source.fragment)]
                            {
                                let compiled =
                                    preprocessed(&shaders::assemble(stage, fragment, embedded));
                                for forbidden in SHADOW_ONLY {
                                    assert!(
                                        !common::mentions(&compiled, forbidden),
                                        "{}: the Classic variant is given {forbidden}",
                                        source.name
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // And the other way round, so the test is worth something.
    let snapped = shaders::scene(shaders::SceneVariant {
        pixel_shadow: true,
        ..shaders::SceneVariant::FLAT
    });
    let compiled = shaders::assemble(&snapped.fragment, true, false)
        + &shaders::assemble(&snapped.vertex, false, false);
    for wanted in SHADOW_ONLY {
        assert!(
            common::mentions(&compiled, wanted),
            "the pixel-shadow variant must carry {wanted}"
        );
    }
}

/// REQ-SCN-082 is about **tile meshes**: the requirement says it is without object on the
/// backdrop, so [`shaders::backdrop`] has no such axis, and the pixel-shadow bit is cleared from
/// a backdrop key. Neither backdrop program carries anything of the snapping.
#[test]
fn req_scn_082_the_backdrop_program_has_no_pixel_shadow_axis() {
    for embedded in [false, true] {
        for bend in [false, true] {
            for voxel_grid in [false, true] {
                let source = shaders::backdrop(bend, voxel_grid);
                for (fragment, stage) in [(false, &source.vertex), (true, &source.fragment)] {
                    let compiled = preprocessed(&shaders::assemble(stage, fragment, embedded));
                    for forbidden in ["zaxVoxelShadowPoint", "uVoxelZOffset", "vModelLinear"] {
                        assert!(
                            !common::mentions(&compiled, forbidden),
                            "{}: the backdrop is given {forbidden}",
                            source.name
                        );
                    }
                }
            }
        }
    }
}

/// The deformation and the curvature reach the **fragment** stage only in the pixel-shadow
/// variants, which is what REQ-SCN-082 point 3 asks for (the voxel centre is placed like a
/// vertex) and nothing else needs.
#[test]
fn req_scn_082_the_fragment_stage_takes_the_placement_only_for_the_snapping() {
    for bend in [false, true] {
        let plain = shaders::scene(shaders::SceneVariant {
            bend,
            ..shaders::SceneVariant::FLAT
        });
        let fragment = shaders::assemble(&plain.fragment, true, false);
        assert!(
            !common::mentions(&fragment, "zaxDeformation"),
            "{}: the fragment stage must not carry the deformation",
            plain.name
        );
        assert!(
            !common::mentions(&fragment, "zaxBend"),
            "{}: the fragment stage must not carry the curvature",
            plain.name
        );

        let snapped = shaders::scene(shaders::SceneVariant {
            bend,
            pixel_shadow: true,
            ..shaders::SceneVariant::FLAT
        });
        let fragment = shaders::assemble(&snapped.fragment, true, false);
        assert!(
            common::mentions(&fragment, "zaxDeformation"),
            "{}",
            snapped.name
        );
        // The curvature only where the frame bends: a flat frame's snapping needs none. Read on
        // `zaxWrap`, which the chunk of REQ-SCN-067 declares outside any guard — `zaxBend` itself
        // is *named* inside the snapping chunk, under a `#ifdef` the Classic stages never see.
        assert_eq!(
            bend,
            common::mentions(&fragment, "zaxWrap"),
            "{}: the curvature belongs in the fragment stage only when the frame bends",
            snapped.name
        );
    }
}

/// The assembled source as the GLSL compiler sees it with the two AMD-055 defines **out**: the
/// blocks they guard dropped, line comments removed.
///
/// The guarded blocks of this amendment are never **nested** inside one another, which this
/// relies on: a `#if` of its own inside a skipped block would not be counted (the chunk of
/// REQ-SCN-082 does hold a `#ifdef ZAX_BEND`, but it is a chunk the Classic stages do not carry
/// at all).
fn preprocessed(source: &str) -> String {
    let defined = |line: &str| line.contains("ZAX_VOXEL_GRID") || line.contains("ZAX_PIXEL_SHADOW");
    let mut out = String::new();
    let mut skipping = 0_i32;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#if") && defined(trimmed) {
            skipping += 1;
            continue;
        }
        if skipping > 0 {
            if trimmed.starts_with("#endif") {
                skipping -= 1;
            }
            continue;
        }
        let code = line.split("//").next().unwrap_or("");
        if !code.trim().is_empty() {
            out.push_str(code);
            out.push('\n');
        }
    }
    out
}

// --------------------------------------------------------------- the GPU side

/// The acceptance fixture: a floor of 4 × 4 full tiles that receives the shadows, and one plate
/// above it that casts, lit by a light oblique on two axes so that the shadow's edges fall
/// **inside** voxels rather than on their edges.
///
/// `rotated` turns the floor by `rot = (90, 0, 0)` as the criterion of REQ-SCN-082 does, which
/// makes its scene normal (0, 1, 0). The rotation is an **object** transform, so it changes
/// nothing of what the snapping does — the local frame is the same — and that is exactly what the
/// second fixture is there to show.
fn floor_and_caster(pixel_shadow: bool, rotated: bool) -> common::Built {
    let mut built = SceneFixture::new()
        .background(0x0F)
        // A mid grey for the floor, not a white: a white one saturates (`min(lit, 1)` in
        // REQ-SCN-028) and then the lit floor and the lit caster are the **same** colour, which
        // leaves no way to tell a floor pixel from a caster pixel.
        .palette(1, 0x00)
        .palette(2, 0x16)
        // The floor: 4 × 4 full tiles, flat (depth 1), which receives the shadow.
        .pattern_block(solid(1, 0), 4, 4, 1, ProfileLayer::L2, 1.0)
        .rect(solid(1, 0), 14, 11, 4, 4)
        // The caster: one full tile, deep, over the middle of the floor.
        .pattern(solid(2, 0), 8, ProfileLayer::L1)
        .tile(solid(2, 0), 15 * 8, 12 * 8, 0)
        .build();
    built.scene.settings.shadows = true;
    built.scene.settings.style = StyleSettings {
        enhanced: true,
        pixel_shadow,
        ..StyleSettings::default()
    };
    // A light whose ratios are not simple fractions, so the shadow's edges fall **inside** voxels
    // whatever whole number of units separates the caster from the floor — which is the point of
    // the 29 units and the (1, −3, 1) of the criterion, whose shadow lands at 9,667 units.
    built.scene.light.direction = Vector3::new(1.0, -0.7, 2.6);
    built.scene.camera.current_mut().angles = Vector3::ZERO;
    built.scene.camera.current_mut().set_distance(30.0);
    built.scene.camera.snap_to_setpoints();

    if rotated {
        let floor = built.instances(is_floor);
        for id in floor {
            built.instance_mut(id).rotation = Vec3::new(90.0, 0.0, 0.0);
        }
        // The floor is horizontal now, so the light of the criterion itself is the one that puts
        // the shadow's edges inside voxels: (1, −3, 1) displaces the shadow by a third of the
        // caster's height on each of the floor's two axes.
        built.scene.light.direction = Vector3::new(1.0, -3.0, 1.0);
        // And the caster goes **29 units** above the floor, the height of the criterion: the
        // shadow then lands 29/3 = 9,667 units away from the plate on both axes, well clear of
        // it and with its edges in the middle of a voxel.
        let caster = built.instances(|tile| !is_floor(tile));
        for id in caster {
            built.instance_mut(id).offset = Vec3::new(0.0, 29.0, 0.0);
        }
        built.rebuild_draw_list();
        // Seen from the front a floor turned flat is edge on; the criterion looks at it from 45°
        // of elevation, which is also what gives each of its voxel cells a measurable area.
        built.scene.camera.current_mut().angles = Vector3::new(45.0, 0.0, 0.0);
        built.scene.camera.current_mut().set_distance(40.0);
        built.scene.camera.snap_to_setpoints();
    }
    built
}

/// Whether a drawn tile belongs to the floor: its mesh was built at depth 1, the caster's at 8.
fn is_floor(tile: &zax_engine3d::draw_list::DrawTile) -> bool {
    tile.mesh
        .as_ref()
        .is_some_and(|mesh| mesh.build_depth() == 1)
}

/// The view the renderer builds for this frame, so a scene point can be projected exactly as the
/// vertex stage projects it.
fn view_of(built: &common::Built, width: i32, height: i32) -> CameraView {
    #[allow(clippy::cast_precision_loss)] // Target sizes are small.
    let aspect = width as f32 / height as f32;
    built
        .scene
        .camera
        .build_view(built.scene.view_clipping(), aspect)
}

/// The pixel a **scene** point lands on, or `None` when it falls outside the capture.
///
/// The curvature is applied first, as the vertex stage does (REQ-SCN-067, and REQ-SCN-082 places
/// the voxel centre like a vertex); the clip position is then built from `(x, y, −z)`, the
/// left-handed scene frame of REQ-SCN-001, with the row-vector matrices `scene.vert` uses.
fn project(
    view: &CameraView,
    bend: &SpaceBend,
    point: Vector3,
    width: i32,
    height: i32,
) -> Option<(i32, i32)> {
    let point = Vector3::from(bend.apply(Vec3::from(point)));
    let m: Matrix4x4 = view.view_projection();
    let (x, y, z) = (point.x, point.y, -point.z);
    let clip = [
        x * m.m11 + y * m.m21 + z * m.m31 + m.m41,
        x * m.m12 + y * m.m22 + z * m.m32 + m.m42,
        x * m.m14 + y * m.m24 + z * m.m34 + m.m44,
    ];
    if clip[2] <= 0.0 {
        return None;
    }
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let pixel = (
        ((clip[0] / clip[2] * 0.5 + 0.5) * width as f32).floor() as i32,
        ((0.5 - clip[1] / clip[2] * 0.5) * height as f32).floor() as i32,
    );
    (pixel.0 >= 0 && pixel.0 < width && pixel.1 >= 0 && pixel.1 < height).then_some(pixel)
}

/// The two colours the floor shows, lit and shadowed, taken from the capture **where the floor
/// is**: the colours found at the centre of every voxel cell of every floor tile, less those the
/// caster also shows — it stands in front of the floor, so its own pixels are sampled too.
///
/// The floor carries one lighting term over its whole surface, so a pixel of it is one of exactly
/// two values; that is what makes "this cell holds both" a measurement rather than an estimate.
/// Reading the two values off the capture rather than computing them is deliberate: what the
/// criterion is about is which pixels take which of the two, not what the two are.
fn floor_shades(built: &common::Built, capture: &common::Capture) -> (i32, i32) {
    let floor = sampled_colors(built, capture, true);
    let caster = sampled_colors(built, capture, false);
    let mut ordered: Vec<(i32, usize)> = floor
        .into_iter()
        .filter(|&(color, _)| caster.get(&color).copied().unwrap_or(0) < 8)
        .collect();
    ordered.sort_by_key(|&(color, count)| (std::cmp::Reverse(count), color));
    assert!(
        ordered.len() >= 2 && ordered[1].1 >= 8,
        "the floor must show a lit and a shadowed shade, found {ordered:?}"
    );
    let (first, second) = (ordered[0].0, ordered[1].0);
    if (first >> 16) >= (second >> 16) {
        (first, second)
    } else {
        (second, first)
    }
}

/// The colours the capture holds at the centre of every voxel cell of the floor's tiles
/// (`floor`) or of every other tile of the draw list, with how many cells each covers.
fn sampled_colors(
    built: &common::Built,
    capture: &common::Capture,
    floor: bool,
) -> std::collections::HashMap<i32, usize> {
    let view = view_of(built, capture.width, capture.height);
    let bend = built.scene.effective_bend();
    let mut counts = std::collections::HashMap::new();
    for tile in built.draw_list.tiles() {
        let Some(mesh) = tile.mesh.as_ref() else {
            continue;
        };
        if is_floor(tile) != floor {
            continue;
        }
        let face = visible_face(tile, mesh, &view);
        for cell_y in 0..8_i32 {
            for cell_x in 0..8_i32 {
                let local = Vec3::new(at(cell_x) - 3.5, at(cell_y) - 3.5, face);
                let scene = Vector3::from(tile.transform.transform_point(local));
                if let Some((x, y)) = project(&view, &bend, scene, capture.width, capture.height) {
                    *counts.entry(capture.packed(x, y)).or_default() += 1;
                }
            }
        }
    }
    counts
}

/// How each 1-voxel cell of the floor reads in `capture`: (cells wholly lit, cells wholly
/// shadowed, cells that hold **both**, named by their tile of the draw list and their indices).
///
/// Each cell is sampled at the nine points of the **middle third** of its local square (the
/// fractions 0.35, 0.5 and 0.65 of each axis), projected through the frame's own camera and
/// curvature; a sample counts only when it lands on one of the two floor shades, and a cell counts
/// only when at least six of its nine samples did — which leaves out the cells the caster hides
/// and the cells on the floor's silhouette.
///
/// The **middle third** and not the middle half, because a cell of the floor seen from 45° of
/// elevation is some six pixels across: a sample that lands a pixel outside its own cell reports
/// a boundary that is not there, and widening the sampling to 1/6 … 5/6 did exactly that (twelve
/// such cells, all of them on a cell edge of the shadow).
fn cell_census(
    built: &common::Built,
    capture: &common::Capture,
    shades: (i32, i32),
) -> (usize, usize, Vec<(usize, i32, i32)>) {
    let (lit, shadowed) = shades;
    let view = view_of(built, capture.width, capture.height);
    let bend = built.scene.effective_bend();
    let (mut all_lit, mut all_dark, mut mixed) = (0, 0, Vec::new());

    for (index, tile) in built.draw_list.tiles().iter().enumerate() {
        let Some(mesh) = tile.mesh.as_ref() else {
            continue;
        };
        if !is_floor(tile) {
            continue;
        }
        let face = visible_face(tile, mesh, &view);

        // REQ-SCN-082's criterion measures the cells of the floor that are **entirely visible**.
        // The outer ring of each tile box is not: a box is closed on itself (REQ-E3D-501), so at
        // the joint with its neighbour the lateral wall of one stands one unit from the face of
        // the other, and a wall is a face of its own — with its own two snapped axes. Seen from
        // an elevation that ring is a band of wall pixels mixed in with the face's.
        for cell_y in 1..7_i32 {
            for cell_x in 1..7_i32 {
                let (mut found, mut light, mut dark) = (0, 0, 0);
                for step_y in 1..=3 {
                    for step_x in 1..=3 {
                        let local = Vec3::new(
                            at(cell_x) - 4.0 + 0.2 + at(step_x) * 0.15,
                            at(cell_y) - 4.0 + 0.2 + at(step_y) * 0.15,
                            face,
                        );
                        let scene = Vector3::from(tile.transform.transform_point(local));
                        let Some((x, y)) =
                            project(&view, &bend, scene, capture.width, capture.height)
                        else {
                            continue;
                        };
                        let packed = capture.packed(x, y);
                        if packed == lit {
                            found += 1;
                            light += 1;
                        } else if packed == shadowed {
                            found += 1;
                            dark += 1;
                        }
                    }
                }

                if found < 6 {
                    continue;
                }
                match (light > 0, dark > 0) {
                    (true, true) => mixed.push((index, cell_x, cell_y)),
                    (true, false) => all_lit += 1,
                    (false, true) => all_dark += 1,
                    (false, false) => {}
                }
            }
        }
    }

    (all_lit, all_dark, mixed)
}

/// The local `z` of the face of a tile plate that looks at the eye, of the two the box has
/// (REQ-E3D-501). Taken from the geometry and not from the capture: a box one voxel deep has its
/// two faces one unit apart, and sampling the wrong one shifts every cell of the grid by the
/// perspective of that unit — which is how this test first read mixed cells where there were
/// none.
fn visible_face(
    tile: &zax_engine3d::draw_list::DrawTile,
    mesh: &zax_engine3d::geometry::tile_mesh::TileMesh,
    view: &CameraView,
) -> f32 {
    let centre = tile.transform.transform_point(Vec3::ZERO);
    let along_z = tile.transform.transform_point(Vec3::new(0.0, 0.0, 1.0));
    let normal = Vector3::new(
        along_z.x - centre.x,
        along_z.y - centre.y,
        along_z.z - centre.z,
    );
    let to_eye = Vector3::new(
        view.eye.x - centre.x,
        view.eye.y - centre.y,
        view.eye.z - centre.z,
    );
    if Vector3::dot(normal, to_eye) >= 0.0 {
        mesh.bounds_max().z
    } else {
        mesh.bounds_min().z
    }
}

/// REQ-SCN-082's acceptance criterion, both bullets: with `spl.pixel_shadow` = 1 **no** voxel
/// cell of the floor holds both lit and shadowed pixels, and with it 0 — in Classic as in
/// Enhanced — several cells of each edge of the shadow do.
#[test]
fn req_scn_082_the_shadow_changes_by_whole_voxel_cells() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let off = floor_and_caster(false, false);
    let plain = gl.render_built(&mut renderer, &off);
    let shades = floor_shades(&off, &plain);
    let (lit_cells, dark_cells, mixed) = cell_census(&off, &plain, shades);
    assert!(
        lit_cells > 50 && dark_cells > 10,
        "the fixture must show a shadow on the floor: {lit_cells} lit, {dark_cells} shadowed cells"
    );
    assert!(
        mixed.len() >= 4,
        "with the effect off the shadow cuts through cells: {} mixed",
        mixed.len()
    );
    // The second bullet names "at least one cell of **each** edge": the cells the boundary cuts
    // have to run along both directions of the shadow's border, not one.
    let columns: std::collections::HashSet<i32> = mixed.iter().map(|&(_, x, _)| x).collect();
    let rows: std::collections::HashSet<i32> = mixed.iter().map(|&(_, _, y)| y).collect();
    assert!(
        columns.len() > 1 && rows.len() > 1,
        "both edges of the shadow must cut cells: {mixed:?}"
    );

    let on = floor_and_caster(true, false);
    let snapped = gl.render_built(&mut renderer, &on);
    let (lit_on, dark_on, mixed_on) = cell_census(&on, &snapped, shades);
    assert!(
        mixed_on.is_empty(),
        "with the effect on no cell may hold both: {mixed_on:?}"
    );
    assert!(
        lit_on > 50 && dark_on > 10,
        "and the shadow is still there: {lit_on} lit, {dark_on} shadowed cells"
    );
    // The picture did move, and only by cells: the shadow keeps about its extent.
    assert_ne!(
        plain.pixels, snapped.pixels,
        "the snapping changes the frame"
    );
    let before = i64::try_from(dark_cells).unwrap();
    let after = i64::try_from(dark_on).unwrap();
    assert!(
        (before - after).abs() <= before / 2,
        "the shadow kept its extent: {before} cells became {after}"
    );
}

/// The same, on the criterion's own geometry: the floor turned by `rot = (90, 0, 0)`, so its
/// scene normal is (0, 1, 0) and the shadow falls on a horizontal plane seen from 45° of
/// elevation. The rotation is an object transform, so the cells the shadow is quantised on are
/// the same ones — which is the order clause of REQ-SCN-080 for this effect.
#[test]
fn req_scn_082_the_rotated_floor_of_the_criterion_quantises_too() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let off = floor_and_caster(false, true);
    let plain = gl.render_built(&mut renderer, &off);
    let shades = floor_shades(&off, &plain);
    let (lit_cells, dark_cells, mixed) = cell_census(&off, &plain, shades);
    assert!(
        lit_cells > 20 && dark_cells > 5,
        "the rotated floor must show a shadow: {lit_cells} lit, {dark_cells} shadowed"
    );
    assert!(
        mixed.len() >= 2,
        "with the effect off the shadow cuts through cells: {mixed:?}"
    );

    let on = floor_and_caster(true, true);
    let snapped = gl.render_built(&mut renderer, &on);
    let (_, dark_on, mixed_on) = cell_census(&on, &snapped, shades);
    assert!(
        mixed_on.is_empty(),
        "with the effect on no cell may hold both: {mixed_on:?}"
    );
    assert!(
        dark_on > 5,
        "and the shadow is still there: {dark_on} cells"
    );
}

/// REQ-SCN-082's scope and the brief's points 1 and 2: at `spl.pixel_shadow` = 0 the capture is
/// the Classic one **to the pixel** and no program is compiled for the effect; Classic with the
/// parameter turned up is Classic too.
#[test]
fn req_scn_082_the_snapping_is_off_at_zero_and_in_classic() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    let mut classic = floor_and_caster(false, false);
    classic.scene.settings.style.enhanced = false;
    let plain = gl.render_built(&mut renderer, &classic);
    assert_eq!(0, renderer.statistics().style_programs);

    let enhanced_off = floor_and_caster(false, false);
    let same = gl.render_built(&mut renderer, &enhanced_off);
    assert_eq!(
        plain.pixels, same.pixels,
        "Enhanced at spl.pixel_shadow = 0 is Classic"
    );
    assert_eq!(0, renderer.statistics().style_programs);

    // Classic with the parameter on: still Classic, still nothing compiled (the style is the
    // gate, so a player who goes back to Classic keeps his parameters and pays nothing).
    let mut turned_up = floor_and_caster(true, false);
    turned_up.scene.settings.style.enhanced = false;
    let still = gl.render_built(&mut renderer, &turned_up);
    assert_eq!(
        plain.pixels, still.pixels,
        "Classic ignores spl.pixel_shadow"
    );
    assert_eq!(0, renderer.statistics().style_programs);

    // And on: the programs appear and the picture moves.
    let on = floor_and_caster(true, false);
    let snapped = gl.render_built(&mut renderer, &on);
    assert!(renderer.statistics().style_programs >= 2);
    assert_ne!(plain.pixels, snapped.pixels);
}

/// REQ-SCN-082: the effect is without object on the **backdrop**, which with
/// `render.backdrop_lighting` off draws through the scene program and does receive shadows. A
/// frame whose only shadow receiver is the backdrop must therefore be the same picture with the
/// effect on and off — to the pixel.
///
/// Without the guard the backdrop would take the snapping in the unit box's own local frame,
/// where a whole face reads one single voxel centre: the caster's shadow would jump to the middle
/// of the box or vanish altogether.
#[test]
fn req_scn_082_the_backdrop_keeps_its_own_shadows() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    // The caster alone, with a backdrop behind it that receives its shadow.
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x16)
        .pattern(solid(1, 0), 8, ProfileLayer::L1)
        .tile(solid(1, 0), 16 * 8, 14 * 8, 0)
        .build();
    built.scene.settings.shadows = true;
    built.scene.settings.backdrop_lighting = false;
    built.scene.backdrop.enabled = true;
    built.scene.backdrop.receive_shadow = true;
    built.scene.backdrop.color = zax_engine3d::palette::Rgb24::new(188, 188, 188);
    built.scene.light.direction = Vector3::new(1.0, -1.0, 1.0);
    built.scene.settings.style = StyleSettings {
        enhanced: true,
        ..StyleSettings::default()
    };

    let plain = gl.render_built(&mut renderer, &built);
    built.scene.settings.style.pixel_shadow = true;
    let snapped = gl.render_built(&mut renderer, &built);
    assert!(
        renderer.statistics().style_programs >= 2,
        "the variant is in use, so the comparison is worth something"
    );
    assert_eq!(
        plain.pixels, snapped.pixels,
        "the backdrop must not take the snapping (REQ-SCN-082: without object on the backdrop)"
    );
}

/// REQ-SCN-080's order clause for this effect: the snapping is evaluated in the mesh's local
/// frame, **before** the curvature, and the displaced point is then bent — so a bent frame still
/// shows a shadow quantised on the mesh's own cells, and the shadow map it reads holds the bent
/// geometry (REQ-SCN-068).
#[test]
fn req_scn_080_the_snapping_happens_before_the_curvature() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    // Ten degrees of left-right curvature: enough to move every pixel of the floor, and little
    // enough that the shadow of the fixture still falls across the middle of the floor, where
    // its edges can be seen to cut cells with the effect off.
    let bend = SpaceBend::new(10.0, 0.0, 0.0, 0.0, 1.0);

    let mut bent_off = floor_and_caster(false, false);
    bent_off.scene.bend = bend;
    let plain = gl.render_built(&mut renderer, &bent_off);
    let shades = floor_shades(&bent_off, &plain);
    let (_, dark_cells, mixed) = cell_census(&bent_off, &plain, shades);
    assert!(
        dark_cells > 5 && mixed.len() >= 2,
        "the bent fixture must show a shadow that cuts cells: {dark_cells} dark, {mixed:?}"
    );

    let mut bent_on = floor_and_caster(true, false);
    bent_on.scene.bend = bend;
    let snapped = gl.render_built(&mut renderer, &bent_on);
    let (_, dark_on, mixed_on) = cell_census(&bent_on, &snapped, shades);
    assert!(
        mixed_on.is_empty(),
        "a bent frame quantises on the same cells: {mixed_on:?}"
    );
    assert!(dark_on > 5, "and keeps its shadow: {dark_on} cells");
    assert_ne!(plain.pixels, snapped.pixels);

    // And the curvature is still what it was: the flat frame and the bent one differ.
    let flat = floor_and_caster(true, false);
    let unbent = gl.render_built(&mut renderer, &flat);
    assert_ne!(
        unbent.pixels, snapped.pixels,
        "the curvature still moves the frame with the snapping on"
    );
}

/// REQ-SCN-082's remaining exclusions: the **HUD** and the **transparent objects**, neither of
/// which receives a shadow (REQ-SCN-040 for the second), and the rectangle of the **2D mode**
/// (AMD-046 §2), which is the only object of a 2D-mode frame and therefore draws with the plain
/// variant throughout.
///
/// Measured where it costs nothing to be exact: a frame of those three alone is the same picture
/// with the effect on and off, to the pixel.
#[test]
fn req_scn_082_the_hud_the_transparent_objects_and_the_two_dimensional_mode_are_left_out() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    // A HUD tile and a transparent object over an opaque floor that casts on itself.
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x00)
        .palette(5, 0x16)
        .pattern(solid(1, 0), 4, ProfileLayer::L1)
        .tile(solid(1, 0), 16 * 8, 2 * 8, 0)
        .pattern_with(
            solid(1, 1),
            8,
            ProfileLayer::L1,
            common::PatternOptions {
                alpha: 0.5,
                ..common::PatternOptions::default()
            },
        )
        .tile(solid(1, 1), 14 * 8, 14 * 8, 1)
        .build();
    built.scene.settings.shadows = true;
    built.scene.light.direction = Vector3::new(1.0, -0.7, 2.6);
    built.scene.settings.style = StyleSettings {
        enhanced: true,
        ..StyleSettings::default()
    };

    let plain = gl.render_built(&mut renderer, &built);
    built.scene.settings.style.pixel_shadow = true;
    let snapped = gl.render_built(&mut renderer, &built);
    assert!(
        renderer.statistics().style_programs >= 2,
        "the variant is in use, so the comparison is worth something"
    );
    assert_eq!(
        plain.pixels, snapped.pixels,
        "neither the HUD nor a transparent object takes the snapping"
    );

    // And the 2D mode: the rectangle is the whole frame, and the variant is not even selected.
    let mut flat = SceneFixture::new();
    flat.two_dimensional_mode = true;
    let mut flat = flat
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 4, ProfileLayer::L1)
        .tile(solid(1, 0), 8 * 8, 8 * 8, 0)
        .build();
    flat.scene.two_dimensional_mode = true;
    flat.scene.settings.style = StyleSettings {
        enhanced: true,
        pixel_shadow: true,
        ..StyleSettings::default()
    };
    let picture = gl.render_built(&mut renderer, &flat);
    flat.scene.settings.style.pixel_shadow = false;
    let plain_picture = gl.render_built(&mut renderer, &flat);
    assert_eq!(
        plain_picture.pixels, picture.pixels,
        "the 2D-mode rectangle draws the same in both"
    );
}
