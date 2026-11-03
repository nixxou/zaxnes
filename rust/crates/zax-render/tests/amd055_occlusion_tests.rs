//! AMD-055 REQ-SCN-083, the ambient occlusion, and the clauses of REQ-SCN-080 that place it.
//!
//! REQ-SCN-083 fixes an **intention**, normative **parameters** and **invariants**, not an
//! algorithm (§1 of the amendment, v1.2): what is asserted here is therefore the parameters and
//! the invariants, never a particular kernel. Every invariant of §2.4 is a **ratio of linear
//! values** between two captures of the same scene that differ only by the setting named, which is
//! what makes them measurable without the absolute column the close-up A publishes.
//!
//! **What is R0's and why.** The close-up A wants a custom texture in the direct RGB format
//! (188, 188, 188) at 1920 × 1080; the synthetic fixture of these tests cannot make one — the same
//! limit steps 3 and 4 of this lot recorded for the grid's close-up G and the pixel shadows' sRGB
//! column. The albedo is reached another way here: NES colour `0x10` *is* (188, 188, 188), so the
//! fixture carries the close-up's own albedo of 0,503 while the capture stays at the fixture's
//! 640 × 480. The floor is the frame's own plane (normal (0, 0, −1)) rather than the criterion's
//! floor turned by `rot = (90, 0, 0)`: the invariants are ratios between two captures of one
//! scene, so the plane's orientation changes nothing of what they measure, and the criterion's own
//! configuration with its absolute column is R0's.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

use common::{solid, Capture, GlFixture, SceneFixture};
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::ProfileLayer;
use zax_render::camera::CameraView;
use zax_render::numerics::{Matrix4x4, Vector3};
use zax_render::style::{AmbientOcclusion, StyleSettings};

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

/// The close-up A of §2.4, as closely as the synthetic fixture reaches it: a plane of 12 × 12 full
/// tiles of `D` = 8 (96 × 96 units) of albedo 0,503, the cube of `sizeZ` = 8 standing on the
/// centre of it, `render.shadow` false, a black `Color` skybox, the light (1, −1, 1), a camera at
/// 80 units and 30° of elevation and a field of view of 70°.
///
/// `cube` false is the invariant's "Gros plan A **sans le cube** (un plan seul, sans autre
/// géométrie)".
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
        // The cube's footprint on the centre of the plane, and its back face against that plane:
        // the tile grid cannot centre an 8-unit footprint on 128, so the placement does it.
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

/// Whether a drawn tile belongs to the cube rather than to the plane or to the HUD: the cube is
/// the one instance of a single tile, so its mesh spans 8 units where the plane's spans 96.
///
/// **Not the build depth and not the transform.** The plane is built at `D` = 8 like the cube, so
/// a depth test selects both; and the transform is what this predicate exists to *set*, so
/// reading it would make the selection depend on whether it has already run.
fn is_cube(tile: &zax_engine3d::draw_list::DrawTile) -> bool {
    tile.mesh.as_ref().is_some_and(|mesh| {
        let span = mesh.bounds_max().x - mesh.bounds_min().x;
        span < 16.0 && mesh.build_depth() == 8
    })
}

/// The Enhanced style with the occlusion on, every other effect at the default of REQ-SCN-079.
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
///
/// The plane carries one lighting term over its whole surface and the scene has no shadow, so
/// "the plane's pixels" is exactly "the pixels of its one shade" — which is also how the pixels of
/// the cube, of its own two shades, are left out.
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

/// The lowest ratio over every pixel the reference capture drew geometry at, and where it is.
///
/// The mask of one shade that [`worst_plane_ratio`] uses rests on the plane carrying a single
/// lighting term, which a **bent** frame does not: the curvature of AMD-043 turns the surface
/// through the frame, so its shade varies continuously and the commonest one is a narrow band that
/// need not touch the cube at all. Over a bent frame the mask is therefore "not the background",
/// which takes in the cube's own pixels too — and the invariants asserted on it (`< 1` somewhere,
/// never under `1 − ceiling`) hold on a darkened surface whatever it is.
fn worst_geometry_ratio(off: &Capture, on: &Capture) -> (f64, i32, i32, usize) {
    let (mut worst, mut at, mut count) = (2.0_f64, (0, 0), 0);
    for y in 0..off.height {
        for x in 0..off.width {
            if off.packed(x, y) == 0 {
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
        "the geometry must fill the capture, found {count}"
    );
    (worst, at.0, at.1, count)
}

/// The shade the plane shows: the commonest colour of a capture of this fixture **that is not the
/// background**.
///
/// The exclusion is not a detail. The plane covers 144 094 of the 640 × 480 pixels and the black
/// background the remaining 163 106, so the commonest colour of the whole capture is the
/// background — and every ratio would then be taken on black, which `ratio_at` refuses to divide
/// by. The background of these fixtures is NES colour `0x0F`, which is pure black, so "not the
/// background" is "not packed 0".
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

/// The pixel a scene point of the plane lands on, projected exactly as the vertex stage projects
/// it (REQ-SCN-001: the clip position is built from `(x, y, −z)`, with the row-vector matrices).
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

/// REQ-SCN-083, first invariant: "Gros plan A **sans le cube** (un plan seul, sans autre
/// géométrie) : le rapport (occlusion On / Off) vaut 1,00 ± 0,02 en tout pixel du sol."
///
/// It comes out **exactly 1** at every one of the 144 094 pixels of the plane, which is what the
/// composite of `ao_composite.frag` is built to give: where the occlusion is 0 the factor is 1
/// and `dst × 1` is `dst`. Finding the three pixels that were not is what added the "no normal to
/// reconstruct" arm of `ao.frag` (see its comment): at the sharp corner of the plane's silhouette
/// an axis has no neighbour on the surface at all, and the pass now leaves such a pixel alone
/// rather than invent a normal for it.
#[test]
fn req_scn_083_a_plane_alone_is_never_darkened() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(false);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);
    let (on, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(3, statistics.screen_effect_passes, "the pass did run");

    let (worst, x, y, count) = worst_plane_ratio(&off, &on, shade);
    assert!(
        worst >= 0.98,
        "a plane alone is not darkened: {worst:.4} at ({x}, {y}) over {count} pixels"
    );
}

/// REQ-SCN-083, second invariant: on the pixels of the plane at the foot of the cube the ratio is
/// **strictly below 1** and **at least 0,10** (the ceiling `min(0,95 ; 0,3 × ao.strength)` = 0,9
/// at the default `ao.strength` = 3), and at a pixel of the plane at least `2r` = 40 units from
/// the cube it is 1,00 ± 0,02.
#[test]
fn req_scn_083_the_foot_of_the_cube_is_darkened_and_the_far_plane_is_not() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);
    let (on, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );

    let (worst, x, y, _) = worst_plane_ratio(&off, &on, shade);
    assert!(
        worst < 1.0,
        "the foot of the cube is darkened, got {worst:.4}"
    );
    assert!(
        worst >= 0.10,
        "and never below the ceiling: {worst:.4} at ({x}, {y})"
    );

    // And the plane far from the cube is untouched. 44 units from the centre of an 8-unit cube is
    // 40 units from its surface, which is `2r` at the default `ao.radius` = 3.
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
            "40 units from the cube the plane is untouched, got {ratio:.4} at ({px}, {py})"
        );
    }
    assert!(
        far >= 2,
        "at least two far points must be visible, saw {far}"
    );
}

/// REQ-SCN-083, third invariant: `ao.strength` = 5 leaves the ratio everywhere ≥ 0,05 and
/// `ao.strength` = 1 everywhere ≥ 0,70 — that is `1 − min(0,95 ; 0,3 × ao.strength)`, the
/// darkening ceiling read as a floor on the ratio — and at the pixel of contact the ratio is
/// non-increasing in the setting and **strictly decreasing from 1 to 4**.
#[test]
fn req_scn_083_the_strength_is_the_darkening_ceiling() {
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
            with_occlusion(AmbientOcclusion::Depth),
        );
        worst_plane_ratio(&off, &on, shade)
    };

    let mut at_contact = Vec::new();
    for strength in 1..=5 {
        let style = StyleSettings {
            ao_strength: strength,
            ..with_occlusion(AmbientOcclusion::Depth)
        };
        let (on, _) = shot(&gl, &mut renderer, &mut built, style);
        let (worst, x, y, _) = worst_plane_ratio(&off, &on, shade);
        let floor = 1.0 - f64::from(style.ao_ceiling());
        assert!(
            worst >= floor - 0.006,
            "ao.strength {strength}: the ratio must stay at or above {floor:.2}, \
             got {worst:.4} at ({x}, {y})"
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

/// REQ-SCN-083, fourth invariant, in its fallback form: "Sous le repli **(D)**, `spl.ao` = 1 et
/// `spl.ao` = 2 donnent des captures identiques à ±2 niveaux sRGB."
///
/// Asserted at tolerance 0: the MSVO port of REQ-SCN-083 does not exist, so `spl.ao` = 1 selects
/// the very same pass with the very same uniforms. REQ-SCN-079's own criterion rides on the same
/// fact — `ao.thickness` acts in MSVO and in no other mode, so under the fallback changing it from
/// 1 to 5 changes no pixel.
#[test]
fn req_scn_083_msvo_is_rendered_as_depth_and_the_thickness_does_nothing() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (depth, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    let (msvo, _) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Msvo),
    );
    assert_eq!(
        depth.pixels, msvo.pixels,
        "REQ-SCN-083 fallback (D): MSVO is rendered exactly as Depth"
    );

    for thickness in [1, 5] {
        let style = StyleSettings {
            ao_thickness: thickness,
            ..with_occlusion(AmbientOcclusion::Depth)
        };
        let (shot, _) = shot(&gl, &mut renderer, &mut built, style);
        assert_eq!(
            depth.pixels, shot.pixels,
            "REQ-SCN-079: ao.thickness has no effect outside an effective MSVO mode"
        );
    }
}

/// REQ-SCN-083, fifth invariant, and point 2 of this lot's brief: `spl.ao` = 0 is the Classic
/// capture — asserted at tolerance 0 rather than the ±2 sRGB levels the requirement allows,
/// because the same programs draw the frame with the same uniforms — and nothing is compiled,
/// allocated or submitted for it.
#[test]
fn req_scn_083_the_occlusion_is_off_at_zero_and_in_classic() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (classic, before) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    assert_eq!(0, before.style_programs);
    assert_eq!(0, before.screen_effect_bytes);
    assert_eq!(0, before.screen_effect_passes);

    let (enhanced_off, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        StyleSettings {
            enhanced: true,
            ..StyleSettings::default()
        },
    );
    assert_eq!(classic.pixels, enhanced_off.pixels);
    assert_eq!(0, statistics.style_programs);
    assert_eq!(0, statistics.screen_effect_bytes);
    assert_eq!(0, statistics.screen_effect_passes);

    // Classic with the occlusion turned up cannot act: the style is the one gate (§3 point 4).
    let (loaded, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        StyleSettings {
            ao: AmbientOcclusion::Depth,
            ao_strength: 5,
            ..StyleSettings::default()
        },
    );
    assert_eq!(classic.pixels, loaded.pixels);
    assert_eq!(0, statistics.screen_effect_passes);

    // And the targets and the programs are **released** when the style goes back to Classic,
    // which is the lifecycle point 1 of the brief asks for.
    let (_, on) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(3, on.style_programs);
    assert!(on.screen_effect_bytes > 0);
    let (again, back) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    assert_eq!(classic.pixels, again.pixels);
    assert_eq!(0, back.style_programs, "the programs are released");
    assert_eq!(0, back.screen_effect_bytes, "and so are the targets");
}

/// REQ-SCN-083's scope, the A-h variant: "les pixels de la skybox et du HUD sont identiques à ±1
/// niveau sRGB entre `spl.ao` = 0 et `spl.ao` = 2".
///
/// Asserted at tolerance 0 again: the composite writes a factor of exactly 1 wherever nothing was
/// drawn, and the HUD is drawn **after** the pass (REQ-SCN-080's frame order), so neither can
/// move by a level. The skybox is the Gradient of the variant, so the comparison is on a sky with
/// structure rather than on one flat colour.
#[test]
fn req_scn_083_the_skybox_and_the_hud_are_left_alone() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    // The geometry mask: the same scene over a **black** background, where every pixel that is not
    // black carries the plane or the cube. The pass may only touch those.
    let mut plain = close_up_a(true);
    plain.scene.settings.skybox = zax_render::scene::SkyboxKind::Color;
    let (black, _) = shot(&gl, &mut renderer, &mut plain, StyleSettings::default());
    let geometry: Vec<bool> = (0..black.height)
        .flat_map(|y| (0..black.width).map(move |x| (x, y)))
        .map(|(x, y)| black.packed(x, y) != 0)
        .collect();

    // The A-h variant: the Gradient sky of REQ-SCN-050 and a HUD pattern (REQ-SCN-044).
    let mut built = hud_close_up_a(true);
    built.scene.settings.skybox = zax_render::scene::SkyboxKind::Gradient;
    let (off, reference) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    assert!(reference.skybox_pass, "the Gradient skybox is drawn");
    assert!(reference.hud_tiles >= 1, "and a HUD tile with it");

    let (on, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(3, statistics.screen_effect_passes);

    // The HUD's own pixels: those a frame without it draws differently.
    let mut without = hud_close_up_a(false);
    without.scene.settings.skybox = zax_render::scene::SkyboxKind::Gradient;
    let (no_hud, _) = shot(&gl, &mut renderer, &mut without, StyleSettings::default());

    let (mut sky, mut hud) = (0, 0);
    for y in 0..off.height {
        for x in 0..off.width {
            let index = usize::try_from(y * off.width + x).unwrap();
            let on_hud = off.packed(x, y) != no_hud.packed(x, y);
            if on_hud {
                hud += 1;
            } else if geometry[index] {
                continue;
            } else {
                sky += 1;
            }
            assert_eq!(
                off.at(x, y),
                on.at(x, y),
                "REQ-SCN-083: neither the skybox nor the HUD is modified, at ({x}, {y})"
            );
        }
    }
    assert!(sky > 100, "the sky must be visible, saw {sky} pixels");
    assert!(hud > 100, "and the HUD with it, saw {hud} pixels");
}

/// The close-up A with a HUD pattern over it, the A-h variant of §2.4.
fn hud_close_up_a(hud: bool) -> common::Built {
    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x10)
        .palette(2, 0x10)
        .palette(3, 0x16)
        .pattern_block(solid(1, 0), 12, 12, 8, ProfileLayer::L2, 1.0)
        .rect(solid(1, 0), 10, 9, 12, 12)
        .pattern(solid(2, 7), 8, ProfileLayer::L1)
        .tile(solid(2, 7), 15 * 8, 14 * 8, 1);
    if hud {
        // The HUD bucket is the **`Ui` layer**, not a screen position: `frame_plan` files a tile
        // under `SceneContainer::Interface` when its pattern's layer is `ProfileLayer::Ui`
        // (REQ-SCN-044). The tile is put above the visible rectangle as well, so that it lands on
        // the sky and the comparison below is not confused with the plane's own pixels.
        fixture = fixture
            .pattern(solid(3, 9), 4, ProfileLayer::Ui)
            .rect(solid(3, 9), 12, 0, 8, 1);
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

/// The close-up A's plane with a slab of `alpha` standing in front of its centre: the opaque
/// occluder at `alpha` = 1, and the transparent object of REQ-SCN-040 below it.
///
/// **The slab is a sprite, and that is what makes the fixture valid.** A *background* record at a
/// cell of the plane's rectangle replaces the plane's own tile there (one screen cell carries one
/// background tile), which leaves a hole whose rim is real geometry at a right angle — and the
/// occlusion darkens that rim correctly, in the transparent capture as in the opaque one, which
/// would hide the very thing this test is about. A sprite record coexists with the background tile
/// it covers (REQ-CORE-043), so the plane keeps all 144 of its tiles either way and the slab is the
/// *only* difference between the two captures.
fn slab_on_the_plane(alpha: f32) -> common::Built {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x10)
        .palette(2, 0x16)
        .pattern_block(solid(1, 0), 12, 12, 8, ProfileLayer::L2, 1.0)
        .rect(solid(1, 0), 10, 9, 12, 12)
        .pattern_with(
            solid(2, 7),
            8,
            ProfileLayer::L1,
            common::PatternOptions {
                alpha,
                ..common::PatternOptions::default()
            },
        )
        .sprite(solid(2, 7), 15 * 8, 14 * 8, 1, false)
        .build();
    built.scene.light.direction = Vector3::new(1.0, -1.0, 1.0);
    built.scene.camera.set_fov(70.0);
    let setpoint = built.scene.camera.current_mut();
    setpoint.angles = Vector3::new(30.0, 35.0, 0.0);
    setpoint.target = CENTRE;
    setpoint.set_distance(80.0);
    built.scene.camera.snap_to_setpoints();
    built
}

/// REQ-SCN-080's frame order, where it is observable: the occlusion runs **after** the opaque
/// objects and **before** the transparent ones, so a transparent object is not an occluder for it
/// (4th bullet: "comme ils sont dessinés après les deux passes d'écran … ils n'occultent pas pour
/// l'occlusion ambiante").
///
/// **Measured against the same slab made opaque**, which is what makes the assertion discriminate:
/// the identical geometry in the identical place leaves the plane's face at exactly ratio 1 over
/// all 141 527 of its pixels when it is transparent, and darkens 12 411 of them to 0,558 when it is
/// opaque.
///
/// Asserting instead that the *whole capture* does not move would be wrong, and for a reason worth
/// recording: the plane is a closed slab of `D` = 8 (REQ-E3D-501), so its own lateral walls meet its
/// face at a right angle and the occlusion darkens that corner — correctly, and in both captures.
/// The plane's **face** is the surface the clause is about, and `plane_shade` is exactly its mask.
#[test]
fn req_scn_080_a_transparent_object_neither_occludes_nor_is_darkened() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = slab_on_the_plane(0.5);

    let (off, reference) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    assert!(
        reference.transparent_objects >= 1,
        "the fixture must hold a transparent object"
    );
    let shade = plane_shade(&off);
    let (on, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(3, statistics.screen_effect_passes);
    assert_eq!(
        reference.transparent_objects, statistics.transparent_objects,
        "the pass changes nothing of what is drawn"
    );

    // The transparent slab is not in the depth the pass reads, so it occludes nothing: the plane's
    // face is lit exactly as in Classic, at its foot as everywhere else.
    let (worst, x, y, _) = worst_plane_ratio(&off, &on, shade);
    assert!(
        worst >= 0.98,
        "REQ-SCN-080: a transparent object does not occlude, got {worst:.4} at ({x}, {y})"
    );

    // And the same slab made opaque does darken it, which is what the clause above rules out.
    let mut opaque = slab_on_the_plane(1.0);
    let (opaque_off, opaque_reference) =
        shot(&gl, &mut renderer, &mut opaque, StyleSettings::default());
    assert_eq!(
        0, opaque_reference.transparent_objects,
        "the control fixture is wholly opaque"
    );
    let opaque_shade = plane_shade(&opaque_off);
    let (opaque_on, _) = shot(
        &gl,
        &mut renderer,
        &mut opaque,
        with_occlusion(AmbientOcclusion::Depth),
    );
    let (opaque_worst, ox, oy, _) = worst_plane_ratio(&opaque_off, &opaque_on, opaque_shade);
    assert!(
        opaque_worst < 0.98,
        "the same slab made opaque occludes, got {opaque_worst:.4} at ({ox}, {oy})"
    );
}

/// REQ-SCN-080's order clause for the screen-space effects: they work on the **rendered** depth,
/// so after the curvature. A bent frame therefore darkens where the bent geometry is, which is a
/// different picture from the flat one — and the pass still runs.
#[test]
fn req_scn_080_the_occlusion_works_on_the_bent_depth() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);
    built.scene.bend = SpaceBend::new(40.0, 0.0, 0.0, 0.0, 1.0);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let (on, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(3, statistics.screen_effect_passes, "the pass runs bent too");
    assert_ne!(off.pixels, on.pixels, "and darkens the bent frame");

    let (worst, x, y, _) = worst_geometry_ratio(&off, &on);
    assert!(
        (0.10..1.0).contains(&worst),
        "the bent surface is darkened where its geometry meets: {worst:.4} at ({x}, {y})"
    );
}

/// AMD-032 §2 read with REQ-SCN-080's last bullet: with anti-aliasing on, the two screen passes
/// work on the **resolved** colour and depth. The pass runs, it darkens, and the frame still
/// carries the samples the implementation granted.
#[test]
fn req_scn_080_the_occlusion_works_on_the_resolved_frame() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);
    built.scene.settings.set_anti_alias_samples(4);

    let (off, reference) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    if reference.anti_alias_samples < 2 {
        // A context that granted no multisampling tells this test nothing.
        return;
    }
    let shade = plane_shade(&off);
    let (on, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(
        reference.anti_alias_samples, statistics.anti_alias_samples,
        "the anti-aliasing is unchanged by the pass"
    );
    assert_eq!(3, statistics.screen_effect_passes);

    let (worst, x, y, _) = worst_plane_ratio(&off, &on, shade);
    assert!(
        (0.10..1.0).contains(&worst),
        "the resolved frame is darkened at the foot of the cube: {worst:.4} at ({x}, {y})"
    );
}

/// REQ-SCN-083's quality rungs: the three values "donnent le même aspect général", and the one
/// clause of them that **is** observable is the resolution — half of each dimension except at
/// High, which is four times the bytes.
#[test]
fn req_scn_083_the_three_qualities_agree_and_only_high_is_full_resolution() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (off, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    let shade = plane_shade(&off);

    let mut bytes = Vec::new();
    let mut worsts = Vec::new();
    for quality in 0..=2 {
        let style = StyleSettings {
            ao_quality: quality,
            ..with_occlusion(AmbientOcclusion::Depth)
        };
        let (on, statistics) = shot(&gl, &mut renderer, &mut built, style);
        bytes.push(statistics.screen_effect_bytes);
        let (worst, _, _, _) = worst_plane_ratio(&off, &on, shade);
        worsts.push(worst);
    }

    // Low and Medium at half of each dimension, High at the full one: two targets of one byte a
    // texel either way.
    assert_eq!(2 * 320 * 240, bytes[0], "Low is half resolution");
    assert_eq!(bytes[0], bytes[1], "Medium too");
    assert_eq!(4 * bytes[0], bytes[2], "and High is four times as many");

    // "Le même aspect général, Low pouvant être plus bruité": the three darken the contact to
    // within a quarter of each other, and none of them leaves it undarkened.
    for (quality, worst) in worsts.iter().enumerate() {
        assert!(
            *worst < 1.0 && *worst >= 0.10,
            "ao.quality {quality} darkens the contact: {worst:.4}"
        );
    }
    let spread = worsts.iter().copied().fold(0.0_f64, f64::max)
        - worsts.iter().copied().fold(1.0_f64, f64::min);
    assert!(spread < 0.25, "the three qualities agree: {worsts:?}");
}

/// REQ-SCN-083's scope in 2D mode: the rectangle of AMD-046 §2 is excluded from being modified
/// while being allowed to occlude, and in 2D mode it is the only object of the frame — so not
/// running the pass satisfies both clauses at once, and costs nothing.
#[test]
fn req_scn_083_the_two_dimensional_mode_runs_no_pass() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut fixture = SceneFixture::new();
    fixture.two_dimensional_mode = true;
    let mut built = fixture
        .background(0x0F)
        .palette(1, 0x10)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .rect(solid(1, 0), 4, 4, 8, 8)
        .build();
    built.scene.two_dimensional_mode = true;

    let (off, before) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    assert_eq!(0, before.screen_effect_passes);
    let (on, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );

    assert_eq!(0, statistics.screen_effect_passes, "no pass in 2D mode");
    assert_eq!(0, statistics.screen_effect_bytes, "and no target");
    assert_eq!(0, statistics.style_programs, "and no program");
    assert_eq!(off.pixels, on.pixels, "so the picture is the Classic one");
}

/// Point 1 of this lot's brief, on the shaders: no program that draws geometry carries anything of
/// the screen-space passes. The occlusion is three **separate** programs, not a variant axis, so
/// this is a structural fact rather than a preprocessing argument — and asserting it is what keeps
/// it one.
#[test]
fn req_scn_083_no_geometry_program_carries_anything_of_the_occlusion() {
    let names = ["uOcclusion", "zaxViewSample", "zaxViewDepth", "zaxDither"];
    let mut sources = Vec::new();
    for embedded in [false, true] {
        for bend in [false, true] {
            for voxel_grid in [false, true] {
                let backdrop = zax_render::shaders::backdrop(bend, voxel_grid);
                sources.push(zax_render::shaders::assemble(
                    &backdrop.fragment,
                    true,
                    embedded,
                ));
                sources.push(zax_render::shaders::assemble(
                    &backdrop.vertex,
                    false,
                    embedded,
                ));
            }
            let shadow = zax_render::shaders::shadow(bend);
            sources.push(zax_render::shaders::assemble(
                &shadow.fragment,
                true,
                embedded,
            ));
        }
        for solid in [false, true] {
            for srgb_palette in [false, true] {
                for bend in [false, true] {
                    for centroid in [false, true] {
                        for voxel_grid in [false, true] {
                            for pixel_shadow in [false, true] {
                                let scene =
                                    zax_render::shaders::scene(zax_render::shaders::SceneVariant {
                                        solid,
                                        srgb_palette,
                                        bend,
                                        centroid,
                                        voxel_grid,
                                        pixel_shadow,
                                    });
                                sources.push(zax_render::shaders::assemble(
                                    &scene.fragment,
                                    true,
                                    embedded,
                                ));
                                sources.push(zax_render::shaders::assemble(
                                    &scene.vertex,
                                    false,
                                    embedded,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    for source in &sources {
        for name in names {
            assert!(
                !common::mentions(source, name),
                "no geometry program carries '{name}' of the screen passes"
            );
        }
    }

    // And in reverse, so that the assertion above is worth something: the three programs of the
    // occlusion do carry them.
    let occlusion = zax_render::shaders::assemble(&zax_render::shaders::AO.fragment, true, false);
    assert!(common::mentions(&occlusion, "zaxViewSample"));
    assert!(common::mentions(&occlusion, "zaxDither"));
    let blur = zax_render::shaders::assemble(&zax_render::shaders::AO_BLUR.fragment, true, false);
    assert!(common::mentions(&blur, "uOcclusion"));
    let composite =
        zax_render::shaders::assemble(&zax_render::shaders::AO_COMPOSITE.fragment, true, false);
    assert!(common::mentions(&composite, "uOcclusion"));
    // The composite reads one texture and nothing else: it needs no depth at all.
    assert!(!common::mentions(&composite, "uDepth"));
}

/// Point 3 of this lot's brief, the fallback half: an implementation that cannot give the screen
/// passes what they need draws the frame **without** them — the Classic picture, which is cheaper
/// and never more expensive — says so once, and gives them up for the session rather than trying
/// again on every frame.
///
/// No driver this side refuses either of the two formats the passes use (`R8` and `RGBA16F`, the
/// two the renderer already depends on — see `gl::screen_effects`), which is why there is nothing
/// to probe and why the refusal has to be injected here to be observed at all
/// (`SceneRenderer::fail_screen_effects`, the same kind of test seam as `set_wall_depth_bias`).
#[test]
fn req_scn_080_an_implementation_that_refuses_the_passes_draws_the_classic_frame() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = close_up_a(true);

    let (classic, _) = shot(&gl, &mut renderer, &mut built, StyleSettings::default());
    assert!(
        renderer.take_diagnostics().is_empty(),
        "a Classic frame says nothing"
    );

    renderer.fail_screen_effects(true);
    let (refused, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(
        classic.pixels, refused.pixels,
        "the refused frame is the Classic frame, to the pixel"
    );
    assert_eq!(0, statistics.style_programs);
    assert_eq!(0, statistics.screen_effect_bytes);
    assert_eq!(0, statistics.screen_effect_passes);
    assert!(renderer.screen_effects_given_up());

    let lines = renderer.take_diagnostics();
    assert_eq!(1, lines.len(), "{lines:?}");
    assert!(lines[0].contains("REQ-SCN-080"), "{}", lines[0]);
    assert!(lines[0].contains("without them"), "{}", lines[0]);

    // Given up for the session: a second frame says nothing more and tries nothing more, even
    // once the refusal is lifted — the two formats are required ones, so a refusal is a property
    // of the implementation and not of the frame.
    renderer.fail_screen_effects(false);
    let (again, statistics) = shot(
        &gl,
        &mut renderer,
        &mut built,
        with_occlusion(AmbientOcclusion::Depth),
    );
    assert_eq!(classic.pixels, again.pixels);
    assert_eq!(0, statistics.screen_effect_passes);
    assert!(renderer.take_diagnostics().is_empty(), "one line, once");
}
