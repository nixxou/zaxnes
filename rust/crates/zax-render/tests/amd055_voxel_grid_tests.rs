//! AMD-055 REQ-SCN-081: the voxel grid.
//!
//! The grid modulates the **albedo** of a tile mesh's faces and of the backdrop box's faces,
//! before any lighting, in the mesh's local frame of REQ-E3D-504 for a tile and in scene units
//! for the backdrop — so before the animated deformation, before the object's transform and
//! before the curvature. The grooves therefore follow the object's scale, its rotation and the
//! curved surface, which is the order clause of REQ-SCN-080.
//!
//! The amendment's four-point table is a table of **ratios of linear values** between a capture
//! with the grid and one without it, so it does not depend on the albedo: it is asserted here on
//! the CPU twin at the albedo of the close-up G (0.503) *and* on the GPU with the fixture's own
//! NES colours. What this side cannot build is the close-up G itself — it wants a custom texture
//! in the direct RGB format (188, 188, 188), which the synthetic fixture has no way to make — so
//! the sRGB columns of the table (133, 166, 169, 107) are checked against the twin and left to
//! R0 on the GPU.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

use common::{solid, GlFixture, SceneFixture};
use zax_engine3d::palette::Rgb24;
use zax_engine3d::space_bend::SpaceBend;
use zax_profiles::enums::ProfileLayer;
use zax_render::numerics::Vector3;
use zax_render::shaders;
use zax_render::style::{self, StyleSettings};

/// The albedo of the close-up G: the direct colour (188, 188, 188) linearised.
const CLOSE_UP_ALBEDO: f32 = 0.503;

/// The normal of the measured face of the close-up G, in the mesh's local frame: it is a GREEDY
/// face, so `m = 1`, `t = 1`, `k = 0.62` and `e = 0.10`, and the weight of the `z` axis is 0.
const FRONT_FACE: [f32; 3] = [0.0, 0.0, -1.0];

/// sRGB from a linear value, the encode of SPEC-06 §2.
fn to_srgb(linear: f32) -> i32 {
    let encoded = if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    #[allow(clippy::cast_possible_truncation)] // 0..=255 by construction.
    {
        (encoded.clamp(0.0, 1.0) * 255.0).round() as i32
    }
}

// --------------------------------------------------------------- the CPU twin

/// REQ-SCN-081, the acceptance table, point by point: the ratio of linear values (grid on / grid
/// off) at the four points of the tile adjacent to the face centre on the `+x`, `+y` side, with
/// `spl.voxel_grid` = 2 (so `N` = 4).
///
/// The sRGB columns are recovered from the ratios at the albedo of the close-up G, which is the
/// other half of what the amendment prints: 166 off, and 133 / 166 / 169 / 107 on.
#[test]
fn req_scn_081_the_four_points_of_the_acceptance_table() {
    // The close-up G is lit head on by a light of direction (1, −1, 1) at intensity 1; what the
    // table fixes is the **ratio**, so the lighting cancels and the sRGB columns follow from the
    // albedo alone — the face of the close-up is fully lit (sRGB 166 at albedo 0.503).
    // The close-up G is lit by a white light of direction (1, −1, 1) at intensity 1 on a face of
    // normal (0, 0, −1): `N·L` = 1/√3 = 0.577 (REQ-SCN-025), on top of the ambient of
    // REQ-SCN-028, so the lit value is `0.503 × (0.1812 + 0.577)` — which is sRGB **166**, the
    // "grid off" column of the amendment's own table. Recovering that number is the check that
    // the fixture of the criterion is understood before its ratios are asserted.
    let lit = CLOSE_UP_ALBEDO * (zax_render::color::ambient_linear() + 1.0 / 3.0_f32.sqrt());
    assert_eq!(166, to_srgb(lit), "the 'grid off' column of the table");

    let cases = [
        (
            "centre of a groove, one axis only",
            [0.0, 2.0],
            0.62,
            3,
            133,
        ),
        ("centre of a cell", [2.0, 2.0], 1.00, 1, 166),
        ("bright edge, d = 10 %", [0.4, 2.0], 1.048, 2, 169),
        ("crossing of two grooves", [0.0, 0.0], 0.384, 3, 107),
    ];
    for (what, point, ratio, tolerance, srgb) in cases {
        let factor = style::voxel_grid_factor([point[0], point[1], -4.0], FRONT_FACE, 4.0);
        assert!(
            (factor - ratio).abs() <= 0.03,
            "{what}: expected a ratio of {ratio}, found {factor}"
        );
        let found = to_srgb(lit * factor);
        assert!(
            (found - srgb).abs() <= tolerance,
            "{what}: expected sRGB {srgb} ± {tolerance}, found {found}"
        );
    }

    // The amendment's own arithmetic for the crossing: 0.62² = 0.384, the corner of a cell being
    // darker than a groove on purpose (Notes).
    let groove = style::voxel_grid_factor([0.0, 2.0, -4.0], FRONT_FACE, 4.0);
    let crossing = style::voxel_grid_factor([0.0, 0.0, -4.0], FRONT_FACE, 4.0);
    assert!((crossing - groove * groove).abs() < 1e-5);
}

/// REQ-SCN-081: the grid is off outside the two bands. At a cell centre and anywhere past 16 % of
/// a cell from an edge the factor is exactly 1, so the grid cannot darken a flat surface as a
/// whole — the check that the groove is a groove and not a tint.
#[test]
fn req_scn_081_the_factor_is_exactly_one_away_from_an_edge() {
    for cell in [2.0_f32, 4.0, 8.0] {
        for fraction in [0.17_f32, 0.25, 0.3, 0.4, 0.5] {
            let point = [fraction * cell, 0.5 * cell, -4.0];
            let factor = style::voxel_grid_factor(point, FRONT_FACE, cell);
            assert!(
                (factor - 1.0).abs() < 1e-6,
                "cell {cell}, d = {fraction}: factor {factor}"
            );
        }
    }
}

/// REQ-SCN-081: an axis perpendicular to the face has weight 0 and **cuts no groove**. On the
/// front face of a tile, the `z` axis is that axis: moving the point across a `z` cell edge
/// changes nothing at all.
#[test]
fn req_scn_081_an_axis_perpendicular_to_the_face_cuts_no_groove() {
    let reference = style::voxel_grid_factor([2.0, 2.0, -4.0], FRONT_FACE, 4.0);
    for z in [-4.0_f32, -3.5, 0.0, 0.001, 4.0] {
        let factor = style::voxel_grid_factor([2.0, 2.0, z], FRONT_FACE, 4.0);
        assert!(
            (factor - reference).abs() < 1e-6,
            "z = {z} changed the factor"
        );
    }
    // And the groove of an axis **in** the plane is unchanged by it too.
    let groove = style::voxel_grid_factor([0.0, 2.0, -4.0], FRONT_FACE, 4.0);
    let moved = style::voxel_grid_factor([0.0, 2.0, 1.7], FRONT_FACE, 4.0);
    assert!((groove - moved).abs() < 1e-6);
}

/// REQ-SCN-081, the alignment interpolation: a face aligned on an axis (`m ≥ 0.9`) gives
/// `k = 0.62` and `e = 0.10`; a face at 45° or more oblique (`m ≤ 0.8`) gives `k = 0.38` and
/// `e = 0.20`; between the two they are linear in `t`.
#[test]
fn req_scn_081_the_alignment_interpolates_k_and_e() {
    // An aligned face: the groove of a single axis is 1 − 1 × (1 − 0.62) = 0.62.
    let aligned = style::voxel_grid_factor([0.0, 2.0, 0.0], [0.0, 0.0, 1.0], 4.0);
    assert!((aligned - 0.62).abs() < 1e-5, "aligned: {aligned}");

    // A face at 45°, normal (0, 1, 1)/√2, so m = 0.7071 ≤ 0.8 and t = 0: k = 0.38. Its weights
    // are w_x = 1 and w_y = w_z = clamp(4 (1 − 0.7071)) = 1 as well, so a point on an x edge and
    // at a cell centre in y and z gives 1 − 1 × (1 − 0.38) = 0.38.
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let oblique = style::voxel_grid_factor([0.0, 2.0, 2.0], [0.0, half, half], 4.0);
    assert!((oblique - 0.38).abs() < 1e-5, "45°: {oblique}");

    // Halfway, m = 0.85 so t = 0.5: k = 0.38 + 0.12 = 0.50. The normal (0, 0.85, …) is not a unit
    // vector, which is deliberate — the formula reads |n_a| and the renderer normalises first; a
    // unit normal with m = 0.85 is built here.
    let rest = (1.0_f32 - 0.85 * 0.85).sqrt();
    let between = style::voxel_grid_factor([0.0, 2.0, 2.0], [0.0, 0.85, rest], 4.0);
    let weight_x = 1.0_f32;
    let expected = 1.0 - weight_x * (1.0 - 0.5);
    assert!(
        (between - expected).abs() < 0.02,
        "m = 0.85: expected about {expected}, found {between}"
    );
}

/// REQ-SCN-081: the cell edges are the coordinates that are multiples of `N`, so for `N` = 8 and
/// `N` = 4 an edge passes through the **centre** of a tile (local 0), and for `N` = 2 the edges
/// are at the even coordinates. Measured as "where is the factor at its lowest".
#[test]
fn req_scn_081_the_edges_are_the_multiples_of_the_cell() {
    for (setting, cell) in [(1, 2.0_f32), (2, 4.0), (3, 8.0)] {
        let style = StyleSettings {
            voxel_grid: setting,
            ..StyleSettings::default()
        };
        #[allow(clippy::cast_precision_loss)] // 2, 4, 8.
        let published = style.voxel_cell() as f32;
        assert!((published - cell).abs() < 1e-6);

        // An edge through the centre of the tile, for every cell size.
        let centre = style::voxel_grid_factor([0.0, cell * 0.5, 0.0], FRONT_FACE, cell);
        assert!(
            (centre - 0.62).abs() < 0.01,
            "cell {cell} at the tile centre"
        );
        // And one at each multiple of the cell, and none between two of them.
        let mut edges = Vec::new();
        for step in 0_i16..=160 {
            let x = f32::from(step) * 0.05 - 4.0;
            let factor = style::voxel_grid_factor([x, cell * 0.5, 0.0], FRONT_FACE, cell);
            if factor < 0.7 {
                edges.push(x);
            }
        }
        // The grooves found, reduced to their centres: one every `cell` units.
        let mut centres = Vec::new();
        for &x in &edges {
            if centres
                .last()
                .is_none_or(|last: &f32| (x - last).abs() > 1.0)
            {
                centres.push(x);
            }
        }
        // The multiples of `cell` inside the tile's 8 units, which run from −4 to +4.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 1, 3 or 5.
        let expected = (4.0 / cell).floor() as usize * 2 + 1;
        assert_eq!(
            expected,
            centres.len(),
            "cell {cell}: grooves at {centres:?} over the 8 units of a tile"
        );
    }
}

// --------------------------------------------------------------- the GPU side

/// A 4 × 4 shape of full tiles seen head on and lit from the eye: the shape of the close-up G,
/// with the fixture's own NES colours instead of its direct texture. The palette entry is a mid
/// grey on purpose — a white one saturates (`min(lit, 1)` in REQ-SCN-028), and a clipped value
/// cannot carry a ratio.
fn surface(grid: i32) -> common::Built {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x00)
        .pattern_block(solid(1, 1), 4, 4, 8, ProfileLayer::L1, 1.0)
        .rect(solid(1, 1), 14, 11, 4, 4)
        .build();
    built.scene.settings.shadows = false;
    built.scene.settings.style = StyleSettings {
        enhanced: true,
        voxel_grid: grid,
        ..StyleSettings::default()
    };
    // Head on, so the measured face is the front face of normal (0, 0, −1), and lit from the eye
    // so the whole face carries one lighting term and a change of shade is the grid's.
    built.scene.light.direction = Vector3::new(0.0, 0.0, 1.0);
    built.scene.light.color = Rgb24::new(255, 255, 255);
    built.scene.camera.current_mut().angles = Vector3::ZERO;
    built.scene.camera.current_mut().set_distance(30.0);
    built.scene.camera.snap_to_setpoints();
    built
}

/// The relative linear value of a packed sRGB colour. These fixtures are grey, so the three
/// channels are equal and this is the channel value in linear space.
fn linear(packed: i32) -> f64 {
    let channel = f64::from((packed >> 16) & 0xFF) / 255.0;
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// The colour that covers the most pixels of `capture`, background aside, and those pixels.
///
/// **Why a mask and not a window.** The ratios of REQ-SCN-081 are ratios "grille On / grille Off"
/// *at a point of one surface*, and this fixture shows two surfaces (the measured front face and
/// a dimmer one at its edge). Measuring a rectangle of the capture mixes them, and the minimum
/// then reads 0.08 instead of the 0.384 the amendment publishes — which is how this test was
/// first written, and wrong. The mask is the pixels that were the front face in the capture
/// **without** the grid, so every ratio below is one surface against itself.
fn face_mask(capture: &common::Capture) -> (f64, Vec<(i32, i32)>) {
    let mut counts: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
    for y in 0..capture.height {
        for x in 0..capture.width {
            let packed = capture.packed(x, y);
            if packed != 0 {
                *counts.entry(packed).or_default() += 1;
            }
        }
    }
    let modal = counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .expect("a lit surface")
        .0;
    let pixels = (0..capture.height)
        .flat_map(|y| (0..capture.width).map(move |x| (x, y)))
        .filter(|&(x, y)| capture.packed(x, y) == modal)
        .collect();
    (linear(modal), pixels)
}

/// REQ-SCN-081 on the GPU: the acceptance table's **ratios** are what the shader produces. The
/// ratio column is the one that does not depend on the albedo, so the fixture's own colours carry
/// it; what is left for R0 is the sRGB column, which wants the close-up G's direct texture.
#[test]
fn req_scn_081_the_published_ratios_are_what_the_shader_draws() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let off = surface(0);
    let plain = gl.render_built(&mut renderer, &off);
    let (flat, mask) = face_mask(&plain);
    assert!(
        mask.len() > 10_000,
        "the face fills the view: {}",
        mask.len()
    );

    for setting in [1, 2, 3] {
        let on = surface(setting);
        let gridded = gl.render_built(&mut renderer, &on);

        let ratios: Vec<f64> = mask
            .iter()
            .map(|&(x, y)| linear(gridded.packed(x, y)) / flat)
            .collect();
        let lowest = ratios.iter().copied().fold(f64::MAX, f64::min);
        let highest = ratios.iter().copied().fold(f64::MIN, f64::max);

        // The crossing of two grooves: 0.62² = 0.384, the darkest the formula can reach on an
        // axis-aligned face, since the third axis has weight 0.
        assert!(
            (lowest - 0.384).abs() <= 0.03,
            "spl.voxel_grid = {setting}: the darkest pixel should be the crossing, found {lowest}"
        );
        // A single groove, 0.62, and a cell interior, exactly 1, both have to be there.
        assert!(
            ratios.iter().any(|&r| (r - 0.62).abs() <= 0.03),
            "spl.voxel_grid = {setting}: no pixel at the single-groove ratio"
        );
        assert!(
            ratios.iter().filter(|&&r| (r - 1.0).abs() <= 0.01).count() > mask.len() / 4,
            "spl.voxel_grid = {setting}: the cell interiors must be untouched"
        );
        // The bright edge of REQ-SCN-081 reaches 1 + e = 1.10 on one axis, so 1.10² = 1.21 where
        // two bright edges meet at a cell corner, and no more.
        assert!(
            (1.02..=1.21).contains(&highest),
            "spl.voxel_grid = {setting}: the brightest pixel should be a bright edge, {highest}"
        );
    }
}

/// REQ-SCN-081, third acceptance criterion: `spl.voxel_grid` = 1, 2 and 3 space the groove lines
/// 2, 4 and 8 scene units apart. Measured on the screen, as the criterion asks, by counting the
/// groove **lines** that cross one row of the face.
///
/// Counting the groove *area* instead does not work, and the reason is the formula: the groove is
/// a band of 3.5 % of a **cell** on each side of an edge, so halving the cell halves the width of
/// every line as well as doubling their number. The area is therefore the same at the three
/// settings — measured here as 8213, 8823 and 8727 pixels, which is how this test was first
/// written and why it is written this way now.
#[test]
fn req_scn_081_the_three_settings_space_the_grooves_two_four_and_eight_units_apart() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let off = surface(0);
    let plain = gl.render_built(&mut renderer, &off);
    let (flat, mask) = face_mask(&plain);

    let mut counts = Vec::new();
    for setting in [1, 2, 3] {
        let on = surface(setting);
        let gridded = gl.render_built(&mut renderer, &on);
        counts.push(groove_lines(&gridded, flat, &mask));
    }

    // 32 units of face, so about 17, 9 and 5 lines; what the criterion fixes is the spacing, and
    // halving the cell doubles the number of lines that cross the row.
    let (fine, medium, coarse) = (counts[0], counts[1], counts[2]);
    assert!(
        fine > medium && medium > coarse,
        "a smaller cell gives more groove lines: {counts:?}"
    );
    #[allow(clippy::cast_precision_loss)] // Line counts under 100.
    let ratio = |a: usize, b: usize| a as f64 / b as f64;
    assert!(
        (1.5..2.6).contains(&ratio(fine, medium)),
        "N = 2 should give about twice the lines of N = 4: {counts:?}"
    );
    assert!(
        (1.5..2.6).contains(&ratio(medium, coarse)),
        "N = 4 should give about twice the lines of N = 8: {counts:?}"
    );
}

/// How many groove lines cross the face on one row of `capture`.
///
/// The row is chosen as the one whose mean ratio is highest, that is the one **least** inside a
/// groove of the other axis: a row that happens to sit on a horizontal groove is dark from end to
/// end and would count no vertical line at all. (That is a real reading of the formula, not an
/// artefact — the two axes multiply.)
fn groove_lines(capture: &common::Capture, flat: f64, mask: &[(i32, i32)]) -> usize {
    let mut rows: std::collections::BTreeMap<i32, Vec<i32>> = std::collections::BTreeMap::new();
    for &(x, y) in mask {
        rows.entry(y).or_default().push(x);
    }
    let best = rows
        .iter()
        .filter(|(_, columns)| columns.len() > 200)
        .max_by(|a, b| {
            let mean = |columns: &Vec<i32>, y: i32| {
                let total: f64 = columns
                    .iter()
                    .map(|&x| linear(capture.packed(x, y)) / flat)
                    .sum();
                #[allow(clippy::cast_precision_loss)] // Column counts under 2^24.
                {
                    total / columns.len() as f64
                }
            };
            mean(a.1, *a.0)
                .partial_cmp(&mean(b.1, *b.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("a row of the face");

    let (y, columns) = (*best.0, best.1);
    let mut sorted = columns.clone();
    sorted.sort_unstable();
    let mut lines = 0;
    let mut inside = false;
    for &x in &sorted {
        let dark = linear(capture.packed(x, y)) / flat < 0.8;
        if dark && !inside {
            lines += 1;
        }
        inside = dark;
    }
    lines
}

/// REQ-SCN-081, fourth acceptance criterion: the grid follows the **object**, because it is
/// evaluated in the mesh's local frame, before the object's transform. With `scale` doubled on
/// `x` a groove that was `N` units apart in the mesh covers `2 N` units of the scene; with
/// `rot = (0, 0, 30)` the grooves turn with the face.
#[test]
fn req_scn_081_the_grid_follows_the_object() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let straight = gl.render_built(&mut renderer, &surface(2));

    let mut scaled = surface(2);
    for id in scaled.instances(|_| true) {
        scaled.instance_mut(id).scale = zax_engine3d::vectors::Vec3::new(2.0, 1.0, 1.0);
    }
    scaled.rebuild_draw_list();
    let widened = gl.render_built(&mut renderer, &scaled);

    let mut turned = surface(2);
    for id in turned.instances(|_| true) {
        turned.instance_mut(id).rotation = zax_engine3d::vectors::Vec3::new(0.0, 0.0, 30.0);
    }
    turned.rebuild_draw_list();
    let rotated = gl.render_built(&mut renderer, &turned);

    assert_ne!(
        straight.pixels, widened.pixels,
        "scale 2 on x spaces the grooves out"
    );
    assert_ne!(
        straight.pixels, rotated.pixels,
        "a rotation of 30 degrees turns the grooves with the face"
    );
}

/// The distinct non-background colours of a capture.
fn shades(capture: &common::Capture) -> Vec<i32> {
    let mut found: Vec<i32> = capture
        .distinct_colors()
        .into_iter()
        .filter(|&color| color != 0)
        .collect();
    found.sort_unstable();
    found
}

/// REQ-SCN-081's scope, and the brief's point 2: with `spl.voxel_grid` = 0 the capture is the
/// Classic one **to the pixel**, no program is compiled for the grid, and the HUD and the skybox
/// are untouched when it is on.
#[test]
fn req_scn_081_the_grid_is_off_at_zero_and_never_reaches_the_hud_or_the_sky() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    // Off: Classic to the pixel, and nothing compiled.
    let mut classic = surface(0);
    classic.scene.settings.style.enhanced = false;
    let plain = gl.render_built(&mut renderer, &classic);
    assert_eq!(0, renderer.statistics().style_programs);

    let enhanced_off = surface(0);
    let same = gl.render_built(&mut renderer, &enhanced_off);
    assert_eq!(plain.pixels, same.pixels, "spl.voxel_grid = 0 is Classic");
    assert_eq!(0, renderer.statistics().style_programs);

    // On: the programs appear, and the picture moves.
    let on = surface(2);
    let gridded = gl.render_built(&mut renderer, &on);
    assert!(renderer.statistics().style_programs >= 2);
    assert_ne!(plain.pixels, gridded.pixels);

    // A scene of a HUD pattern and a gradient sky: neither takes the grid (REQ-SCN-081).
    let mut hud = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 1), 4, ProfileLayer::L1)
        .tile(solid(1, 1), 16, 2, 0)
        .build();
    hud.scene.settings.shadows = false;
    hud.scene.settings.skybox = zax_render::scene::SkyboxKind::Gradient;
    let before = gl.render_built(&mut renderer, &hud);
    hud.scene.settings.style = StyleSettings {
        enhanced: true,
        voxel_grid: 3,
        ..StyleSettings::default()
    };
    let after = gl.render_built(&mut renderer, &hud);
    // The sky fills most of the picture and the HUD sits above the visible rectangle; the one
    // thing that may move is the tile, and this fixture's tile is in the scene bucket.
    let moved = before
        .pixels
        .chunks(4)
        .zip(after.pixels.chunks(4))
        .filter(|(a, b)| a != b)
        .count();
    let total = before.pixels.len() / 4;
    assert!(
        moved * 100 < total * 30,
        "the sky and the HUD must not take the grid: {moved} of {total} pixels moved"
    );
}

/// AMD-055 §3 point 5: the grid applies to the **backdrop** too, modulating the albedo `c` of
/// REQ-SCN-075 before the model, so the specular term is not darkened by a groove.
#[test]
fn req_scn_075_the_backdrop_takes_the_grid_on_its_albedo() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern(solid(3, 0), 4, ProfileLayer::L1)
        .build();
    built.scene.settings.shadows = false;
    built.scene.backdrop.enabled = true;
    built.scene.backdrop.color = Rgb24::new(188, 188, 188);
    built.scene.backdrop.receive_shadow = false;
    built.scene.light.direction = Vector3::new(0.0, 0.0, 1.0);
    built.scene.camera.current_mut().angles = Vector3::ZERO;
    built.scene.camera.snap_to_setpoints();

    let plain = gl.render_built(&mut renderer, &built);
    let plain_shades = shades(&plain);

    built.scene.settings.style = StyleSettings {
        enhanced: true,
        voxel_grid: 3,
        ..StyleSettings::default()
    };
    let gridded = gl.render_built(&mut renderer, &built);
    assert_ne!(
        plain.pixels, gridded.pixels,
        "the backdrop takes the grid (AMD-055 §3 point 5)"
    );
    assert!(
        shades(&gridded).len() > plain_shades.len(),
        "and it cuts the face into more shades"
    );

    // The same with the lighting setting off, where the backdrop draws through the scene program
    // at colour mode 2: the grid still reaches it, in scene units.
    built.scene.settings.backdrop_lighting = false;
    let unlit_grid = gl.render_built(&mut renderer, &built);
    built.scene.settings.style.voxel_grid = 0;
    let unlit_plain = gl.render_built(&mut renderer, &built);
    assert_ne!(
        unlit_plain.pixels, unlit_grid.pixels,
        "and through the scene program too"
    );
}

/// The order clause of REQ-SCN-080: the grid is evaluated **before** the curvature, so a bent
/// surface still carries grooves and they follow the bend. What the criterion of REQ-SCN-080
/// discriminates is that the grid is painted on the geometry rather than in screen space — and a
/// screen-space grid would give the same straight, evenly spaced lines in both captures.
#[test]
fn req_scn_080_the_grid_is_painted_on_the_geometry_not_on_the_screen() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let flat = surface(2);
    let straight = gl.render_built(&mut renderer, &flat);

    let mut bent = surface(2);
    bent.scene.bend = SpaceBend::new(90.0, 0.0, 0.0, 0.0, 1.0);
    let curved = gl.render_built(&mut renderer, &bent);

    assert_ne!(
        straight.pixels, curved.pixels,
        "the curvature moves the grooved surface"
    );
    // And the grooves survive the bend: a bent capture still holds several shades of the face.
    assert!(
        shades(&curved).len() > 3,
        "the grooves follow the curved surface"
    );

    // The grid variant and the curvature variant are the same program, so both are compiled:
    // `style_programs` counts the grid's two, and the curvature's are AMD-043's.
    assert!(renderer.statistics().style_programs >= 2);
}

/// The brief's point 1: no Classic variant carries anything of the grid. The check preprocesses
/// the assembled source the way the GLSL compiler would, over every variant that leaves the two
/// AMD-055 defines out — the shape `req_scn_067_the_flat_variant_carries_nothing_of_the_curvature`
/// established for AMD-043.
#[test]
fn req_scn_081_no_classic_variant_carries_anything_of_the_grid() {
    const GRID_ONLY: [&str; 4] = ["zaxVoxelGrid", "uVoxelCell", "vLocal", "vLocalNormal"];

    for embedded in [false, true] {
        for solid in [false, true] {
            for srgb_palette in [false, true] {
                for bend in [false, true] {
                    for centroid in [false, true] {
                        let source = shaders::scene(shaders::SceneVariant {
                            solid,
                            srgb_palette,
                            bend,
                            centroid,
                            voxel_grid: false,
                            pixel_shadow: false,
                        });
                        for (fragment, stage) in [(false, &source.vertex), (true, &source.fragment)]
                        {
                            let compiled =
                                preprocessed(&shaders::assemble(stage, fragment, embedded));
                            for forbidden in GRID_ONLY {
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

        // And the backdrop program without the grid axis carries none of it either.
        for bend in [false, true] {
            let source = shaders::backdrop(bend, false);
            for (fragment, stage) in [(false, &source.vertex), (true, &source.fragment)] {
                let compiled = preprocessed(&shaders::assemble(stage, fragment, embedded));
                for forbidden in GRID_ONLY {
                    assert!(
                        !common::mentions(&compiled, forbidden),
                        "{}: the backdrop without the grid is given {forbidden}",
                        source.name
                    );
                }
            }
        }
    }

    // And the other way round, so the test is worth something: the grid variant does carry it.
    let gridded = shaders::scene(shaders::SceneVariant {
        voxel_grid: true,
        ..shaders::SceneVariant::FLAT
    });
    // The raw assembled source, not the preprocessed one: `preprocessed` drops exactly the
    // blocks this half is looking for.
    let compiled = shaders::assemble(&gridded.fragment, true, false)
        + &shaders::assemble(&gridded.vertex, false, false);
    for wanted in GRID_ONLY {
        assert!(
            common::mentions(&compiled, wanted),
            "the grid variant must carry {wanted}"
        );
    }
}

/// The assembled source as the GLSL compiler sees it with the two AMD-055 defines **out**:
/// the blocks they guard dropped, line comments removed. The guards are text, so the file
/// changes even where the compiled code does not (the finding AMD-058 recorded for
/// `ZAX_CENTROID`), which is why this preprocesses rather than greps.
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
