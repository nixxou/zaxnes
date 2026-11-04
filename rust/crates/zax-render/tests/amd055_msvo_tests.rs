//! AMD-055 REQ-SCN-083, *Mode MSVO* (`spl.ao` = 1): the multi-scale occlusion ported from the
//! MIT-licensed effect of Microsoft's `MiniEngine` (`gl/msvo.rs`).
//!
//! §2.4 says in so many words that "les invariants et les critères de revue de cette section valent
//! **aussi** pour ce mode, à la seule exception de l'invariant d'identité entre `spl.ao` = 1 et
//! `spl.ao` = 2, qui ne vaut que sous le repli". So the five invariants are asserted here a second
//! time, against the multi-scale estimator, on the same close-up A fixture
//! `amd055_occlusion_tests.rs` uses for the hemisphere one — plus the two clauses that are the
//! mode's own: `ao.thickness` **acts**, and the fallback **(D)** is still reachable when an
//! implementation refuses the pyramid.
//!
//! **What is R0's and why**, unchanged from the Depth file: the close-up A wants a custom texture
//! in the direct RGB format (188, 188, 188) at 1920 × 1080, which the synthetic fixture cannot
//! make. The albedo is reached through NES colour `0x10`, which *is* (188, 188, 188); the capture
//! stays at 640 × 480 and the floor is the frame's own plane. Every invariant below is a **ratio**
//! between two captures of one scene, so neither limit changes what it measures.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

use common::{solid, Capture, GlFixture, SceneFixture};
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::ProfileLayer;
use zax_render::camera::CameraView;
use zax_render::numerics::{Matrix4x4, Vector3};
use zax_render::style::{self, AmbientOcclusion, StyleSettings};

/// The scene `z` of the plane's front face: the 12 × 12 slab is centred at `z` = 16 and is `D` = 8
/// deep, so its face — the "sol" of the close-up A — is at 12.
const PLANE_Z: f32 = 12.0;

/// The centre of that face, which is also where the cube stands and what the camera aims at.
const CENTRE: Vector3 = Vector3::new(128.0, 128.0, PLANE_Z);

/// Half the side of the plane: 12 tiles of 8 units.
const PLANE_HALF: f32 = 48.0;

/// How far from the centre the second invariant reads "the plane far from the cube": 44 units from
/// the centre of an 8-unit cube is 40 units from its surface, which is `2r` at the default
/// `ao.radius` = 3 — and inside the plane's own half-side.
const FAR_FROM_THE_CUBE: f32 = 44.0;
const _: () = assert!(PLANE_HALF > FAR_FROM_THE_CUBE);

/// The close-up A of §2.4, as closely as the synthetic fixture reaches it; `cube` false is the
/// first invariant's "un plan seul, sans autre géométrie".
fn close_up_a(cube: bool) -> common::Built {
    let mut fixture = SceneFixture::new()
        .background(0x0F)
        // NES colour 0x10 is (188, 188, 188): the albedo of the close-up A, 0,503 linear.
        .palette(1, 0x10)
        .palette(2, 0x10)
        .pattern_block(solid(1, 0), 12, 12, 8, ProfileLayer::L2, 1.0)
        .rect(solid(1, 0), 10, 9, 12, 12);
    if cube {
        fixture =
            fixture
                .pattern(solid(2, 7), 8, ProfileLayer::L1)
                .tile(solid(2, 7), 15 * 8, 14 * 8, 1);
    }

    let mut built = fixture.build();
    if cube {
        for id in built.instances(is_cube) {
            built.instance_mut(id).offset = Vec3::new(4.0, -4.0, 4.0);
        }
        built.rebuild_draw_list();
    }

    built.scene.light.direction = Vector3::new(1.0, -1.0, 1.0);
    built.scene.camera.set_fov(70.0);
    let setpoint = built.scene.camera.current_mut();
    setpoint.angles = Vector3::new(30.0, 35.0, 0.0);
    setpoint.target = CENTRE;
    setpoint.set_distance(80.0);
    built.scene.camera.snap_to_setpoints();
    built
}

/// The same floor with a **kerb** of 64 × 8 × 8 units across it in place of the close-up's single
/// cube: the scene the .NET lot V2-25 added for the large-scale question, and the only one of the
/// two that asks it.
///
/// The occluder has to be **laterally wide** for a multi-scale estimate to have anything to find:
/// the pattern of each level is a seven-entry checkerboard scaled by that level's sphere radius, so
/// a coarse level samples sparsely in absolute terms and an 8-unit cube falls between its samples
/// whatever the pyramid does. Eight tiles of the close-up's own pattern, side by side across the
/// middle row, are 64 units wide and 8 tall — which the coarsest sphere of `1,75 r` = 35 units sees
/// whole.
fn close_up_kerb() -> common::Built {
    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x10)
        .palette(2, 0x10)
        .pattern_block(solid(1, 0), 12, 12, 8, ProfileLayer::L2, 1.0)
        .rect(solid(1, 0), 10, 9, 12, 12)
        .pattern(solid(2, 7), 8, ProfileLayer::L1);
    // Each record replaces the floor tile of its own cell, as the close-up's cube does: the kerb
    // stands where that tile was and hides it.
    for column in KERB_COLUMNS {
        fixture = fixture.tile(solid(2, 7), column * 8, 14 * 8, 1);
    }

    let mut built = fixture.build();
    for id in built.instances(is_cube) {
        built.instance_mut(id).offset = Vec3::new(4.0, -4.0, 4.0);
    }
    built.rebuild_draw_list();

    built.scene.light.direction = Vector3::new(1.0, -1.0, 1.0);
    built.scene.camera.set_fov(70.0);
    let setpoint = built.scene.camera.current_mut();
    setpoint.angles = Vector3::new(30.0, 35.0, 0.0);
    setpoint.target = CENTRE;
    setpoint.set_distance(80.0);
    built.scene.camera.snap_to_setpoints();
    built
}

/// The tile columns the kerb occupies. The close-up's cube is column 15 of row 14 and its footprint
/// is centred on the floor's centre, so column `c` of that row is centred on
/// `CENTRE.x + (c − 15) × 8`: columns 12 to 19 span 64 units about the centre.
const KERB_COLUMNS: std::ops::Range<i32> = 12..20;

/// The scene `x` the kerb spans, less half a tile at each end so that every sample of
/// [`kerb_profile`] faces the kerb rather than its corner.
const KERB_X: (f32, f32) = (CENTRE.x - 24.0, CENTRE.x + 32.0);

/// Whether a drawn tile belongs to the cube or to the kerb rather than to the plane: either is an
/// instance of a single tile, so its mesh spans 8 units where the plane's spans 96.
fn is_cube(tile: &zax_engine3d::draw_list::DrawTile) -> bool {
    tile.mesh.as_ref().is_some_and(|mesh| {
        let span = mesh.bounds_max().x - mesh.bounds_min().x;
        span < 16.0 && mesh.build_depth() == 8
    })
}

/// The Enhanced style with the occlusion in `mode`, every other effect at the default of
/// REQ-SCN-079.
fn with_occlusion(mode: AmbientOcclusion) -> StyleSettings {
    StyleSettings {
        enhanced: true,
        ao: mode,
        ..StyleSettings::default()
    }
}

/// The linear value of one sRGB byte.
fn linear(value: i32) -> f64 {
    #[allow(clippy::cast_precision_loss)] // 0 to 255.
    f64::from(zax_render::color::to_linear(value as f32 / 255.0))
}

/// The sum of the three linear channels of a pixel — the quantity the invariants of §2.4 take the
/// ratio of ("valeurs lues en sRGB 8 bits puis converties en linéaire").
fn linear_sum(capture: &Capture, x: i32, y: i32) -> f64 {
    let (red, green, blue) = capture.at(x, y);
    linear(red) + linear(green) + linear(blue)
}

/// The ratio (on / off) at one pixel.
fn ratio_at(off: &Capture, on: &Capture, x: i32, y: i32) -> f64 {
    let before = linear_sum(off, x, y);
    assert!(
        before > 1e-6,
        "({x}, {y}) is black in the reference capture"
    );
    linear_sum(on, x, y) / before
}

/// The lowest ratio over the pixels the reference capture shows the plane at, and where it is.
fn worst_plane_ratio(off: &Capture, on: &Capture, shade: i32) -> (f64, i32, i32, usize) {
    let (mut worst, mut at, mut count) = (2.0_f64, (0, 0), 0);
    for y in 0..off.height {
        for x in 0..off.width {
            if off.packed(x, y) != shade {
                continue;
            }
            count += 1;
            let ratio = ratio_at(off, on, x, y);
            if ratio < worst {
                worst = ratio;
                at = (x, y);
            }
        }
    }
    assert!(
        count > 1000,
        "the plane must fill the capture, found {count}"
    );
    (worst, at.0, at.1, count)
}

/// The shade the plane shows: the commonest colour of a capture that is not the background.
fn plane_shade(capture: &Capture) -> i32 {
    let mut counts = std::collections::HashMap::new();
    for y in 0..capture.height {
        for x in 0..capture.width {
            let packed = capture.packed(x, y);
            if packed != 0 {
                *counts.entry(packed).or_insert(0_usize) += 1;
            }
        }
    }
    counts
        .into_iter()
        .max_by_key(|&(color, count)| (count, color))
        .map_or(0, |(color, _)| color)
}

/// The pixel a scene point lands on, projected exactly as the vertex stage projects it
/// (REQ-SCN-001: the clip position is built from `(x, y, −z)`, with the row-vector matrices).
fn project(view: &CameraView, point: Vector3, width: i32, height: i32) -> Option<(i32, i32)> {
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

/// Renders the fixture once with `style` and gives back the capture and the statistics.
fn shot(
    gl: &GlFixture,
    renderer: &mut zax_render::SceneRenderer,
    built: &mut common::Built,
    style: StyleSettings,
) -> (Capture, zax_render::statistics::RenderStatistics) {
    built.scene.settings.style = style;
    let capture = gl.render_built(renderer, built);
    (capture, renderer.statistics().clone())
}

/// The mean darkening of the plane, in sRGB levels, by 4-unit bands of distance to the cube's
/// footprint — the profile Q-100 measures the two estimators against each other with.
///
/// The bands are walked in the **scene** and projected through the frame's own camera, not read off
/// a rectangle of the capture: a band of constant scene width covers a varying number of pixels
/// under perspective, so a mean over pixels would weight the near end of the plane several times
/// more than the far one.
fn profile(off: &Capture, on: &Capture, view: &CameraView, shade: i32) -> [f64; 5] {
    let mut bands = [(0.0_f64, 0_usize); 5];
    for step_y in 0..=384 {
        for step_x in 0..=384 {
            #[allow(clippy::cast_precision_loss)] // 0 to 384.
            let (x, y) = (
                CENTRE.x - PLANE_HALF + step_x as f32 * 0.25,
                CENTRE.y - PLANE_HALF + step_y as f32 * 0.25,
            );
            // The cube's footprint is the 8-unit square centred on the plane's centre.
            let distance = (x - CENTRE.x).abs().max((y - CENTRE.y).abs()) - 4.0;
            if distance < 0.0 {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Bounded below.
            let band = (distance / 4.0) as usize;
            if band >= bands.len() {
                continue;
            }
            let Some((px, py)) = project(view, Vector3::new(x, y, PLANE_Z), off.width, off.height)
            else {
                continue;
            };
            if off.packed(px, py) != shade {
                continue;
            }
            let (r0, g0, b0) = off.at(px, py);
            let (r1, g1, b1) = on.at(px, py);
            bands[band].0 += f64::from((r0 - r1) + (g0 - g1) + (b0 - b1)) / 3.0;
            bands[band].1 += 1;
        }
    }
    bands.map(|(sum, count)| {
        #[allow(clippy::cast_precision_loss)] // Counts far below 2^53.
        if count > 0 {
            sum / count as f64
        } else {
            0.0
        }
    })
}

/// The mean darkening of the floor of [`close_up_kerb`], in sRGB levels, by 4-unit bands of
/// distance to the kerb — and **on the near side of it only**.
///
/// The side matters, and finding out why is worth the sentence: the kerb stands 8 units out of the
/// floor, so from a camera 30° above it the floor **behind** the kerb is hidden for the first 12
/// units and comes back into view beyond that, at a grazing angle where the kerb genuinely blocks
/// much more of its hemisphere. Averaging the two sides therefore reads a profile that *rises*
/// again at 12 to 16 units — 5,1 then 13,9 levels at 1920 × 1080 — which looks exactly like a ring
/// of banding and is nothing of the kind. The near side is the one the whole profile is visible on.
fn kerb_profile(off: &Capture, on: &Capture, view: &CameraView, shade: i32) -> [f64; 8] {
    let mut bands = [(0.0_f64, 0_usize); 8];
    for side in [1.0_f32, -1.0] {
        let mut found = [(0.0_f64, 0_usize); 8];
        for step_y in 0..=384 {
            for step_x in 0..=224 {
                #[allow(clippy::cast_precision_loss)] // 0 to 384.
                let (x, y) = (
                    KERB_X.0 + step_x as f32 * 0.25,
                    CENTRE.y - PLANE_HALF + step_y as f32 * 0.25,
                );
                if (y - CENTRE.y) * side < 0.0 {
                    continue;
                }
                // The kerb is 8 units deep about the floor's centre row.
                let distance = (y - CENTRE.y).abs() - 4.0;
                if distance < 0.0 {
                    continue;
                }
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Bounded.
                let band = (distance / 4.0) as usize;
                if band >= found.len() {
                    continue;
                }
                let Some((px, py)) =
                    project(view, Vector3::new(x, y, PLANE_Z), off.width, off.height)
                else {
                    continue;
                };
                if off.packed(px, py) != shade {
                    continue;
                }
                let (r0, g0, b0) = off.at(px, py);
                let (r1, g1, b1) = on.at(px, py);
                found[band].0 += f64::from((r0 - r1) + (g0 - g1) + (b0 - b1)) / 3.0;
                found[band].1 += 1;
            }
        }
        // The near side is the one whose contact band the camera sees at all.
        if found[0].1 > 0 {
            bands = found;
            break;
        }
    }
    assert!(
        bands[0].1 > 1000,
        "the near side of the kerb must be visible, found {} samples",
        bands[0].1
    );
    bands.map(|(sum, count)| {
        #[allow(clippy::cast_precision_loss)] // Counts far below 2^53.
        if count > 0 {
            sum / count as f64
        } else {
            0.0
        }
    })
}

/// The view the fixture's camera builds, for [`profile`] and for the far-point projection.
fn view_of(built: &common::Built) -> CameraView {
    built
        .scene
        .camera
        .build_view(built.scene.view_clipping(), 640.0 / 480.0)
}

/// REQ-SCN-083, **first invariant**, in the MSVO mode: "Gros plan A **sans le cube** (un plan seul,
/// sans autre géométrie) : le rapport (occlusion On / Off) vaut 1,00 ± 0,02 en tout pixel du sol."
///
/// It comes out **exactly 1** at each of the 144 094 pixels of the plane, at every quality and every
/// `ao.radius` — which is what the three choices recorded in `gl/msvo.rs` are for, and what finding
/// this invariant broken is what found them: the access is stored **with headroom** so that the
/// smoothing averages the estimator's noise both ways instead of a clamp rectifying it, the pyramid
/// is point-sampled so that interpolating a perspective depth adds no ripple, and a sample outside
/// the drawn rectangle does not occlude.
#[test]
fn req_scn_083_msvo_leaves_a_plane_alone_undarkened() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(false);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);

    // Every quality and every radius: the three sampling choices above all bite harder as the
    // sphere covers fewer texels, so the extreme of the two tables is the case that matters.
    for quality in 0..=2 {
        for radius in [1, 3, 5] {
            let style = StyleSettings {
                ao_quality: quality,
                ao_radius: radius,
                ..with_occlusion(AmbientOcclusion::Msvo)
            };
            let (on, statistics) = shot(&gl, &mut renderer, &mut built, style);
            assert_eq!(
                style::MSVO_LEVELS,
                i32::try_from(statistics.occlusion_levels).unwrap(),
                "the multi-scale estimator ran"
            );
            let (worst, x, y, count) = worst_plane_ratio(&off, &on, shade);
            assert!(
                worst >= 0.98,
                "ao.quality {quality}, ao.radius {radius}: a plane alone is not darkened, \
                 got {worst:.5} at ({x}, {y}) over {count} pixels"
            );
        }
    }
}

/// REQ-SCN-083, **second invariant**, in the MSVO mode: at the foot of the cube the ratio is
/// strictly below 1 and at least 0,10 — the ceiling `min(0,95 ; 0,3 × ao.strength)` = 0,9 at the
/// default — and at a pixel of the plane `2r` = 40 units from the cube it is 1,00 ± 0,02.
#[test]
fn req_scn_083_msvo_darkens_the_foot_of_the_cube_and_not_the_far_plane() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);
    let (on, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );

    let (worst, x, y, _) = worst_plane_ratio(&off, &on, shade);
    assert!(
        worst < 1.0,
        "the foot of the cube is darkened, got {worst:.5}"
    );
    assert!(
        worst >= 0.10,
        "and never below the ceiling: {worst:.5} at ({x}, {y})"
    );

    let view = view_of(&built);
    let mut far = 0;
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        let point = Vector3::new(
            CENTRE.x + dx * FAR_FROM_THE_CUBE,
            CENTRE.y + dy * FAR_FROM_THE_CUBE,
            PLANE_Z,
        );
        let Some((px, py)) = project(&view, point, off.width, off.height) else {
            continue;
        };
        if off.packed(px, py) != shade {
            continue;
        }
        far += 1;
        let ratio = ratio_at(&off, &on, px, py);
        assert!(
            (ratio - 1.0).abs() <= 0.02,
            "40 units from the cube the plane is untouched, got {ratio:.5} at ({px}, {py})"
        );
    }
    assert!(
        far >= 2,
        "at least two far points must be visible, saw {far}"
    );
}

/// REQ-SCN-083, **third invariant**, in the MSVO mode: `ao.strength` = 1 leaves the ratio everywhere
/// ≥ 0,70 and `ao.strength` = 5 everywhere ≥ 0,05 — the ceiling read as a floor — and at the pixel
/// of contact the ratio is non-increasing in the setting and strictly decreasing from 1 to 4.
///
/// The floor is compared with the same one-sRGB-level slack the Depth file uses, which is the
/// tolerance §2.4 states for every invariant of the section since Q-097: the ceiling is reached
/// exactly at the contact, and a value read back through an 8-bit encode cannot land on it.
#[test]
fn req_scn_083_msvo_keeps_the_strength_as_the_darkening_ceiling() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);

    // The pixel of contact: the darkest pixel of the plane at the default strength, so that the
    // monotonicity below is read at one pixel and not at a moving minimum.
    let (_, contact_x, contact_y, _) = {
        let (on, _) = shot(
            &gl,
            &mut renderer,
            &mut built,
            with_occlusion(AmbientOcclusion::Msvo),
        );
        worst_plane_ratio(&off, &on, shade)
    };

    let mut at_contact = Vec::new();
    for strength in 1..=5 {
        let style = StyleSettings {
            ao_strength: strength,
            ..with_occlusion(AmbientOcclusion::Msvo)
        };
        let (on, _) = shot(&gl, &mut renderer, &mut built, style);
        let (worst, x, y, _) = worst_plane_ratio(&off, &on, shade);
        let floor = 1.0 - f64::from(style.ao_ceiling());
        assert!(
            worst >= floor - 0.006,
            "ao.strength {strength}: the ratio must stay at or above {floor:.2}, \
             got {worst:.5} at ({x}, {y})"
        );
        at_contact.push(ratio_at(&off, &on, contact_x, contact_y));
    }

    for rung in 1..5 {
        assert!(
            at_contact[rung] <= at_contact[rung - 1] + 1e-9,
            "the ratio at the contact is non-increasing in ao.strength: {at_contact:?}"
        );
    }
    for rung in 1..4 {
        assert!(
            at_contact[rung] < at_contact[rung - 1],
            "and strictly decreasing from 1 to 4: {at_contact:?}"
        );
    }
}

/// REQ-SCN-083, **fourth invariant** in the form the port gives it, and the clause of REQ-SCN-079
/// that `ao.thickness` acts in this mode and in this mode alone: "avec le portage MSVO disponible :
/// `spl.ao` = 1, changer `ao.thickness` de 1 à 5 modifie au moins un pixel de plus de 2 niveaux
/// sRGB, et le rapport au pixel de contact ne **croît** pas".
///
/// And the *Mode MSVO* paragraph's own requirement with it: "à géométrie fixe, l'assombrissement NE
/// DOIT PAS décroître quand `ao.thickness` croît" — asserted over all five rungs, not just the two
/// ends, because the monotonicity is what makes the setting mean "thickness" at all.
#[test]
fn req_scn_083_the_thickness_acts_in_the_msvo_mode_and_never_lightens() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);
    let (default, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );
    let (_, contact_x, contact_y, _) = worst_plane_ratio(&off, &default, shade);

    let mut captures = Vec::new();
    let mut at_contact = Vec::new();
    for thickness in 1..=5 {
        let style = StyleSettings {
            ao_thickness: thickness,
            ..with_occlusion(AmbientOcclusion::Msvo)
        };
        let (on, _) = shot(&gl, &mut renderer, &mut built, style);
        at_contact.push(ratio_at(&off, &on, contact_x, contact_y));
        captures.push(on);
    }

    // At least one pixel moves by more than 2 sRGB levels between the two ends.
    let mut most = 0;
    for y in 0..off.height {
        for x in 0..off.width {
            let (r0, g0, b0) = captures[0].at(x, y);
            let (r1, g1, b1) = captures[4].at(x, y);
            most = most.max((r0 - r1).abs().max((g0 - g1).abs()).max((b0 - b1).abs()));
        }
    }
    assert!(
        most > 2,
        "ao.thickness 1 to 5 must move a pixel by more than 2 sRGB levels, most was {most}"
    );

    // And the darkening never decreases as the thickness grows: the ratio is non-increasing.
    for rung in 1..5 {
        assert!(
            at_contact[rung] <= at_contact[rung - 1] + 1e-9,
            "the darkening does not decrease as ao.thickness grows: {at_contact:?}"
        );
    }
    assert!(
        at_contact[4] < at_contact[0],
        "and it does grow over the five rungs: {at_contact:?}"
    );
}

/// REQ-SCN-083, **fifth invariant** and the brief's "defaults unchanged", in the MSVO mode:
/// `spl.ao` = 0 is the Classic capture to the pixel, and the mode compiles, allocates and submits
/// nothing until it is chosen — then releases all of it when the style goes back.
#[test]
fn req_scn_083_msvo_is_off_at_zero_and_in_classic() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (classic, before) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    assert_eq!(0, before.style_programs);
    assert_eq!(0, before.screen_effect_bytes);
    assert_eq!(0, before.screen_effect_passes);
    assert_eq!(0, before.occlusion_levels);

    // Classic with the MSVO mode and every parameter turned up cannot act: the style is the one
    // gate (§3 point 4).
    let (loaded, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        StyleSettings {
            ao: AmbientOcclusion::Msvo,
            ao_strength: 5,
            ao_thickness: 5,
            ..StyleSettings::default()
        },
    );
    assert_eq!(classic.pixels, loaded.pixels);
    assert_eq!(0, statistics.screen_effect_passes);
    assert_eq!(0, statistics.occlusion_levels);

    let (_, on) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );
    assert!(on.screen_effect_bytes > 0, "the pyramid is allocated");
    assert_eq!(
        style::MSVO_LEVELS,
        i32::try_from(on.occlusion_levels).unwrap()
    );

    let (again, back) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    assert_eq!(classic.pixels, again.pixels);
    assert_eq!(0, back.style_programs, "the programs are released");
    assert_eq!(0, back.screen_effect_bytes, "and so is the pyramid");
    assert_eq!(0, back.occlusion_levels);
}

/// The brief's point 2, one level down: **one estimator or the other is built, never both**. The
/// multi-scale mode holds the shared composite and the five programs of `gl/msvo.rs`; the
/// hemisphere mode holds the shared composite and its own two. Switching between them releases
/// what is leaving, pyramid and all.
#[test]
fn req_scn_083_each_mode_builds_only_its_own_estimator() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (_, depth) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(3, depth.style_programs, "the composite and its two passes");
    assert_eq!(0, depth.occlusion_levels, "and no pyramid");
    assert_eq!(3, depth.screen_effect_passes);
    let depth_bytes = depth.screen_effect_bytes;

    let (_, msvo) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );
    assert_eq!(
        6, msvo.style_programs,
        "the composite and the five of the port"
    );
    assert_eq!(
        style::MSVO_LEVELS,
        i32::try_from(msvo.occlusion_levels).unwrap()
    );
    // The pyramid is deeper than the pair of targets the hemisphere estimator owns, and the two
    // never coexist: the byte count is the one or the other, never their sum.
    assert!(msvo.screen_effect_bytes > depth_bytes);

    let (_, again) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(3, again.style_programs, "the port's five are released");
    assert_eq!(0, again.occlusion_levels);
    assert_eq!(
        depth_bytes, again.screen_effect_bytes,
        "and so is its pyramid"
    );
}

/// REQ-SCN-083's quality rungs in the MSVO mode: the three give the same general aspect, and only
/// High computes at full resolution ("à la moitié de chaque dimension de la cible de rendu courante
/// sauf en High").
///
/// The **number of scales does not vary** — [`style::MSVO_LEVELS`] is 4 at every quality, which is
/// what Q-097 settled: the coarsest level carries the wide half of a multi-scale estimate, and
/// dropping it at Low made the mode *lighter* there, where REQ-SCN-083 asks for "le même aspect
/// général, Low pouvant être plus bruité". What varies at Low is the sample pattern.
#[test]
fn req_scn_083_the_msvo_qualities_agree_and_only_high_is_full_resolution() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);

    let mut contacts = Vec::new();
    let mut bytes = Vec::new();
    for quality in 0..=2 {
        let style = StyleSettings {
            ao_quality: quality,
            ..with_occlusion(AmbientOcclusion::Msvo)
        };
        let (on, statistics) = shot(&gl, &mut renderer, &mut built, style);
        assert_eq!(
            style::MSVO_LEVELS,
            i32::try_from(statistics.occlusion_levels).unwrap(),
            "ao.quality {quality} keeps all four scales"
        );
        assert_eq!(
            style.msvo_pattern_entries(),
            if quality == 0 {
                style::MSVO_LOW_ENTRIES
            } else {
                style::MSVO_ENTRIES
            }
        );
        let (worst, _, _, _) = worst_plane_ratio(&off, &on, shade);
        contacts.push(worst);
        bytes.push(statistics.screen_effect_bytes);
    }

    // Half of each dimension except at High: a quarter of the texels, so a quarter of the bytes.
    assert_eq!(bytes[0], bytes[1], "Low and Medium are both at half");
    assert_eq!(bytes[2], bytes[1] * 4, "and High at full");

    // The same general aspect: the three agree to within a quarter at the contact, which is the
    // tolerance the Depth file's own quality test uses.
    let (low, high) = (
        contacts.iter().copied().fold(f64::MAX, f64::min),
        contacts.iter().copied().fold(f64::MIN, f64::max),
    );
    assert!(
        high - low <= 0.25,
        "the three qualities give the same general aspect: {contacts:?}"
    );
    for contact in &contacts {
        assert!(*contact < 0.9, "and each of them darkens: {contacts:?}");
    }
}

/// REQ-SCN-083's fallback **(D)**, still a real path: "tant que ce portage n'est pas disponible,
/// `spl.ao` = 1 DOIT être rendu exactement comme `spl.ao` = 2".
///
/// An implementation that refuses the pyramid — the one thing the mode needs that the rest of the
/// renderer does not, an `R16F` colour target — falls back on the Depth estimator, says so **once**,
/// and does not try again. The refusal is injected (`SceneRenderer::refuse_msvo`), the same kind of
/// seam `fail_screen_effects` is and for the same reason: no driver on this side refuses `R16F`, and
/// an unreachable fallback is an untested one.
#[test]
fn req_scn_083_an_implementation_that_refuses_the_pyramid_renders_msvo_as_depth() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (depth, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    let (msvo, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );
    assert_ne!(
        depth.pixels, msvo.pixels,
        "the two estimators draw different pictures when the port runs"
    );
    assert_eq!(
        style::MSVO_LEVELS,
        i32::try_from(statistics.occlusion_levels).unwrap()
    );
    renderer.take_diagnostics();

    assert!(
        renderer.refuse_msvo(),
        "the frame had an estimator to refuse"
    );
    let (fallback, refused) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );
    assert_eq!(
        depth.pixels, fallback.pixels,
        "REQ-SCN-083 fallback (D): MSVO is then rendered exactly as Depth"
    );
    assert_eq!(0, refused.occlusion_levels, "no pyramid is built");
    assert_eq!(3, refused.style_programs, "the Depth estimator is in place");
    assert!(renderer.msvo_refused());

    let lines = renderer.take_diagnostics();
    assert_eq!(1, lines.len(), "one line says so: {lines:?}");
    assert!(
        lines[0].contains("MSVO") && lines[0].contains("REQ-SCN-083"),
        "{lines:?}"
    );

    // And it is not retried: a second frame draws the same Depth picture and says nothing more.
    let (again, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );
    assert_eq!(depth.pixels, again.pixels);
    assert!(renderer.take_diagnostics().is_empty(), "and only once");
}

/// Q-100, the Rust side: the **multi-scale** mode is the deeper and the wider-reaching of the two at
/// the contact and in the bands that follow it, which is the whole point of estimating the
/// occlusion at several resolutions ("ce qui donne à la fois un contact fin et un assombrissement
/// large à coût modéré").
///
/// The figures and the discussion are in `docs/progress/RU-6.md`; what is pinned here is the
/// ordering, which is what Q-100 is about and what no calibration constant may silently invert.
#[test]
fn req_scn_083_the_multi_scale_mode_reaches_further_than_the_single_scale_one() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);
    let view = view_of(&built);
    let (depth_shot, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    let (msvo_shot, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );

    let depth = profile(&off, &depth_shot, &view, shade);
    let msvo = profile(&off, &msvo_shot, &view, shade);
    for band in 0..3 {
        assert!(
            msvo[band] > depth[band],
            "band {band}: the multi-scale mode darkens more, {msvo:?} against {depth:?}"
        );
    }
    // And both die out well inside `2 r` = 40 units, which the second invariant measures at.
    assert!(
        msvo[4] < 1.0 && depth[4] < 1.0,
        "both fade by 16 to 20 units: {msvo:?}, {depth:?}"
    );
}

/// REQ-SCN-083 *Mode MSVO*, the large scale, on the kerb scene — the regression test the .NET lot
/// V2-25 wrote for the same clause, with the same four bounds and the same shape: both modes'
/// profiles read off two captures of one scene, each bound held so that it can be broken in either
/// direction.
///
/// Every bound is a figure measured in **this** lot and not a figure of the reference: the
/// requirement is P3 and judged by review, and `Q-100` says what the reference's own figures would
/// need. The figures themselves, side by side with the .NET's, are in `docs/progress/RU-6.md`.
#[test]
fn req_scn_083_the_msvo_mode_finds_a_laterally_wide_occluder() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_kerb();

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);
    let view = view_of(&built);
    let (depth_shot, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    let (msvo_shot, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );
    assert_eq!(
        style::MSVO_LEVELS,
        i32::try_from(statistics.occlusion_levels).unwrap()
    );

    let depth = kerb_profile(&off, &depth_shot, &view, shade);
    let msvo = kerb_profile(&off, &msvo_shot, &view, shade);

    // 1. The contact band darkens, and well past the noise floor.
    assert!(
        msvo[0] > 20.0,
        "the contact band of the kerb darkens: {msvo:?}"
    );

    // 2. The mid field — `0,25 r` to `0,75 r`, which is the 4 to 12 units of the two bands that
    //    follow the contact — is a real share of it, which is what "multi-scale" buys.
    let mid = f64::midpoint(msvo[1], msvo[2]);
    let ratio = mid / msvo[0];
    assert!(ratio > 0.12, "mid / contact is {ratio:.4}: {msvo:?}");

    // 3. And it is a **larger** share than the single-scale estimator's on the same scene. The
    //    .NET port measured 0,466 of the Depth mode's ratio here and `Q-100` recorded the shortfall
    //    against the reference's 1,38; this one measures above 1, so the ordering is the
    //    reference's. The bound is the .NET's own, so that a regression in either port's direction
    //    is caught.
    let depth_ratio = f64::midpoint(depth[1], depth[2]) / depth[0];
    assert!(
        ratio / depth_ratio > 0.30,
        "mid / contact against the Depth mode's: {:.4} ({ratio:.4} over {depth_ratio:.4})",
        ratio / depth_ratio
    );

    // 4. §2.4's second invariant on this scene: beyond `2 r` = 40 units nothing is darkened.
    assert!(
        msvo[7] < 1.0 && depth[7] < 1.0,
        "beyond 2 r the floor is untouched: {msvo:?}, {depth:?}"
    );
}

/// RU-7, the three invariants of §2.4 read off a **`wgpu`** capture in the **MSVO** mode, with the
/// OpenGL ones beside them: the isolated plane at 1,00 over every quality and every radius, the
/// contact ratio `< 1` and `>= 0,10`, and the five `ao.strength` rungs.
///
/// The first of the three is the one that found the port's three sampling choices on the OpenGL
/// side (the access headroom, the point-sampled pyramid, the sample outside the rectangle), so it
/// is also the one that says whether this backend inherited all three — and the two row mirrors of
/// `msvo_downsample.wgsl` and `msvo_occlude.wgsl` with them.
///
/// Skips with the reason printed where no adapter answers, as every `wgpu` test does.
#[cfg(feature = "wgpu")]
#[test]
// The three invariants of §2.4 in one body, on each reachable API: they share the two captures
// the ratios are taken between, and splitting them would render the reference twice per API.
#[allow(clippy::too_many_lines)]
fn req_scn_083_the_msvo_invariants_of_2_4_hold_on_the_wgpu_backend_too() {
    use zax_render::wgpu_backend::headless::GpuApi;

    let apis: &[(&str, GpuApi)] = if cfg!(windows) {
        &[("vulkan", GpuApi::Vulkan), ("dx12", GpuApi::Dx12)]
    } else {
        &[("vulkan", GpuApi::Vulkan)]
    };
    for &(name, api) in apis {
        let Some(mut fixture) = common::WgpuFixture::create(api, 640, 480) else {
            return;
        };

        // The first invariant, over the extremes of the two tables: the sampling choices of the
        // port all bite harder as the sphere covers fewer texels.
        let mut alone = close_up_a(false);
        alone.scene.settings.style = StyleSettings::default();
        let off = fixture.render_built(&alone);
        let shade = plane_shade(&off);
        let mut worst_alone = 2.0_f64;
        for quality in 0..=2 {
            for radius in [1, 3, 5] {
                alone.scene.settings.style = StyleSettings {
                    ao_quality: quality,
                    ao_radius: radius,
                    ..with_occlusion(AmbientOcclusion::Msvo)
                };
                let on = fixture.render_built(&alone);
                assert_eq!(
                    style::MSVO_LEVELS,
                    i32::try_from(fixture.statistics().occlusion_levels).unwrap(),
                    "{name}: the multi-scale estimator ran"
                );
                let (worst, x, y, count) = worst_plane_ratio(&off, &on, shade);
                assert!(
                    worst >= 0.98,
                    "{name}, ao.quality {quality}, ao.radius {radius}: a plane alone is not \
                     darkened, got {worst:.5} at ({x}, {y}) over {count} pixels"
                );
                worst_alone = worst_alone.min(worst);
            }
        }
        println!("{name}: a plane alone reads {worst_alone:.5} over the nine configurations");

        // The second: the foot of the cube, and the plane `2r` away.
        let mut built = close_up_a(true);
        built.scene.settings.style = StyleSettings::default();
        let off = fixture.render_built(&built);
        let shade = plane_shade(&off);
        built.scene.settings.style = with_occlusion(AmbientOcclusion::Msvo);
        let on = fixture.render_built(&built);
        let (contact, contact_x, contact_y, _) = worst_plane_ratio(&off, &on, shade);
        println!("{name}: the contact reads {contact:.4} at ({contact_x}, {contact_y})");
        assert!(
            contact < 1.0,
            "{name}: the foot is darkened, got {contact:.4}"
        );
        assert!(
            contact >= 0.10,
            "{name}: and never below the ceiling: {contact:.4}"
        );

        let view = built
            .scene
            .camera
            .build_view(built.scene.view_clipping(), 640.0 / 480.0);
        let mut far = 0;
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let point = Vector3::new(
                CENTRE.x + dx * FAR_FROM_THE_CUBE,
                CENTRE.y + dy * FAR_FROM_THE_CUBE,
                PLANE_Z,
            );
            let Some((px, py)) = project(&view, point, off.width, off.height) else {
                continue;
            };
            if off.packed(px, py) != shade {
                continue;
            }
            far += 1;
            let ratio = ratio_at(&off, &on, px, py);
            assert!(
                (ratio - 1.0).abs() <= 0.02,
                "{name}: 40 units from the cube the plane is untouched, got {ratio:.4}"
            );
        }
        assert!(far >= 2, "{name}: at least two far points, saw {far}");

        // The third: the five rungs of `ao.strength`.
        let mut at_contact = Vec::new();
        for strength in 1..=5 {
            let style = StyleSettings {
                ao_strength: strength,
                ..with_occlusion(AmbientOcclusion::Msvo)
            };
            built.scene.settings.style = style;
            let on = fixture.render_built(&built);
            let (worst, x, y, _) = worst_plane_ratio(&off, &on, shade);
            let floor = 1.0 - f64::from(style.ao_ceiling());
            assert!(
                worst >= floor - 0.006,
                "{name}: ao.strength {strength} must stay at or above {floor:.2}, \
                 got {worst:.4} at ({x}, {y})"
            );
            at_contact.push(ratio_at(&off, &on, contact_x, contact_y));
        }
        println!("{name}: the five ao.strength rungs read {at_contact:?}");
        for rung in 1..5 {
            assert!(
                at_contact[rung] <= at_contact[rung - 1] + 1e-9,
                "{name}: non-increasing in ao.strength: {at_contact:?}"
            );
        }
        for rung in 1..4 {
            assert!(
                at_contact[rung] < at_contact[rung - 1],
                "{name}: and strictly decreasing from 1 to 4: {at_contact:?}"
            );
        }
    }
}
