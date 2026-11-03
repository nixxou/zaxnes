//! AMD-055 REQ-SCN-084, the floor reflections, and the clauses of REQ-SCN-080 that place them.
//!
//! REQ-SCN-084 fixes an **intention**, normative **parameters** and **invariants**, not an
//! algorithm (§1 of the amendment, v1.2): what is asserted here is the parameters and the
//! invariants, never a particular march. Every invariant of §2.5 is a comparison of two captures
//! of one scene that differ only by the setting named, which is what makes them measurable
//! without the absolute column the close-up R publishes.
//!
//! **What is R0's and why.** The close-up R wants two custom textures in the direct RGB format —
//! the floor at (124, 124, 124) and the cube at (255, 0, 0) — at 1920 × 1080; the synthetic
//! fixture of these tests cannot make one, the same limit steps 3 to 5 of this lot recorded. The
//! albedi are reached another way: NES colour `0x00` is (120, 128, 132), a floor as dark as the
//! criterion's, and `0x16` is (252, 56, 0), a red whose linear red channel is 0.95. The invariant
//! of the red channel is a **ratio**, so it is measured here; the absolute column is R0's.
//!
//! Two of the six automatic invariants are **not measurable in the close-up R as the amendment
//! writes it**, and `Q-106` records the measurements that show it: every ray reflected by an
//! 8-unit cube resting on the floor travels about fifteen units, far under the forty the shortest
//! `ssr.length` already allows, so neither the length nor the blur — which is proportional to the
//! distance travelled — reaches the configuration's resolution. What is asserted here instead is
//! stated test by test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test code

mod common;

use std::collections::HashMap;

use common::{close_up_r, is_cube, solid, Capture, GlFixture, SceneFixture};
use zax_engine3d::draw_list::DrawTile;
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::Vec3;
use zax_profiles::enums::ProfileLayer;
use zax_render::numerics::Vector3;
use zax_render::style::{AmbientOcclusion, StyleSettings};

/// The Enhanced style with the reflections on, every other effect at the default of REQ-SCN-079.
fn with_reflections(style: StyleSettings) -> StyleSettings {
    StyleSettings {
        enhanced: true,
        reflections: true,
        ..style
    }
}

/// Enhanced with every effect at its default: no reflection, no occlusion, no variant.
const ENHANCED_OFF: StyleSettings = StyleSettings {
    enhanced: true,
    ao: AmbientOcclusion::Off,
    ao_strength: 3,
    ao_radius: 3,
    ao_thickness: 3,
    ao_quality: 1,
    pixel_shadow: false,
    voxel_grid: 0,
    reflections: false,
    ssr_shine: 3,
    ssr_clarity: 3,
    ssr_length: 3,
    ssr_quality: 2,
};

/// The linear value of one sRGB byte.
fn linear(value: i32) -> f64 {
    #[allow(clippy::cast_precision_loss)] // 0 to 255.
    f64::from(zax_render::color::to_linear(value as f32 / 255.0))
}

/// How many pixels of the capture differ at all.
fn changed(off: &Capture, on: &Capture) -> usize {
    let mut count = 0;
    for y in 0..off.height {
        for x in 0..off.width {
            if off.packed(x, y) != on.packed(x, y) {
                count += 1;
            }
        }
    }
    count
}

/// The commonest colour of the capture that is not the cleared background — the one shade a flat,
/// unshadowed surface of one orientation carries, which is how the floor's own pixels are
/// separated from the cube's and from the background's (the helper of step 5 of this lot).
fn dominant_shade(capture: &Capture, background: i32) -> i32 {
    let mut counts: HashMap<i32, usize> = HashMap::new();
    for y in 0..capture.height {
        for x in 0..capture.width {
            let colour = capture.packed(x, y);
            if colour != background {
                *counts.entry(colour).or_default() += 1;
            }
        }
    }
    let (shade, count) = counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .expect("the capture holds geometry");
    assert!(count > 1000, "the dominant surface must fill the capture");
    shade
}

/// The pixels the reference capture shows at `shade`, and over them the largest red-channel ratio
/// and how many moved at all.
fn red_over_shade(off: &Capture, on: &Capture, shade: i32) -> (f64, usize, usize) {
    let (mut best, mut moved, mut total) = (1.0_f64, 0, 0);
    for y in 0..off.height {
        for x in 0..off.width {
            if off.packed(x, y) != shade {
                continue;
            }
            total += 1;
            if on.packed(x, y) != shade {
                moved += 1;
            }
            let before = linear(off.at(x, y).0);
            if before > 1e-6 {
                best = best.max(linear(on.at(x, y).0) / before);
            }
        }
    }
    (best, moved, total)
}

/// The sum over the capture of |on − off| on the three linear channels: the "écart entre les
/// captures" the monotonicity invariant of §2.5 speaks of.
fn gap(off: &Capture, on: &Capture) -> f64 {
    let mut sum = 0.0;
    for y in 0..off.height {
        for x in 0..off.width {
            let (r0, g0, b0) = off.at(x, y);
            let (r1, g1, b1) = on.at(x, y);
            sum += (linear(r1) - linear(r0)).abs()
                + (linear(g1) - linear(g0)).abs()
                + (linear(b1) - linear(b0)).abs();
        }
    }
    sum
}

/// The first invariant of §2.5: on the floor pixels at the foot of the cube, the **linear red
/// channel** grows by at least 10 % against the same capture at `spl.reflections` = 0.
///
/// Measured over the floor's own shade, which is what separates its pixels from the cube's: the
/// floor carries one lighting term and the scene has no shadow. The margin the amendment computes
/// — a factor of 4,9 between the reflected red and the floor's own, so 30 % of the full red
/// suffices — comes out as a ratio of **1,94** here, well past the 1,10 the invariant asks.
#[test]
fn req_scn_084_the_floor_reflects_the_red_cube_at_its_foot() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);

    let off = gl.render_built(&mut renderer, &built);
    let floor = dominant_shade(&off, 0);
    built.scene.settings.style = with_reflections(StyleSettings::default());
    let on = gl.render_built(&mut renderer, &built);

    let (best, moved, total) = red_over_shade(&off, &on, floor);
    assert!(
        total > 10_000,
        "the floor must fill the capture, found {total} pixels"
    );
    assert!(
        moved > 100,
        "the reflection must be visible, {moved} floor pixels moved"
    );
    assert!(
        best >= 1.10,
        "the red channel must grow by 10 % at the contact, best ratio {best:.3}"
    );
}

/// Fourth invariant of §2.5, first clause: `ssr.shine` rising from 1 to 5 makes the gap between
/// the captures with and without reflections grow monotonically. The five intensities are
/// `0,045 × n`, so the gap should grow about linearly — and does: the sums come out 139, 417, 680,
/// 951 and 1223.
#[test]
fn req_scn_084_the_shine_grows_the_gap_monotonically() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    let off = gl.render_built(&mut renderer, &built);

    let mut previous = 0.0;
    for shine in 1..=5 {
        built.scene.settings.style = with_reflections(StyleSettings {
            ssr_shine: shine,
            ..StyleSettings::default()
        });
        let on = gl.render_built(&mut renderer, &built);
        let gap = gap(&off, &on);
        assert!(
            gap > previous,
            "ssr.shine {shine} must widen the gap: {gap:.3} after {previous:.3}"
        );
        previous = gap;
    }
}

/// Fourth invariant, third clause, as far as this side can take it (`Q-106`): `ssr.clarity` rising
/// narrows the blur, so the reflection touches **no more** pixels at a higher setting than at a
/// lower one, and strictly fewer at 5 than at 1.
///
/// The amendment measures a *transition width* on a reflection's edge, which needs rays that have
/// travelled a sizeable fraction of their maximum — the blur radius is proportional to the
/// distance travelled, and in the close-up R every ray travels about fifteen units of the two
/// hundred the default allows, so the radius is a fraction of a pixel. Q-093 (c) already recorded
/// that defect of the configuration and R1 accepted a declared variant for it; `Q-106` records
/// that the same holds here and at every fixture this side can build. What is left measurable is
/// the **spread**, and it is monotone.
#[test]
fn req_scn_084_the_clarity_narrows_the_blur() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    let off = gl.render_built(&mut renderer, &built);

    let mut widths = Vec::new();
    for clarity in 1..=5 {
        built.scene.settings.style = with_reflections(StyleSettings {
            ssr_clarity: clarity,
            ..StyleSettings::default()
        });
        let on = gl.render_built(&mut renderer, &built);
        widths.push(changed(&off, &on));
    }

    for pair in widths.windows(2) {
        assert!(
            pair[1] <= pair[0],
            "a clearer reflection must not spread further: {widths:?}"
        );
    }
    // And the measurement that says the configuration cannot discriminate: in the close-up R the
    // five settings give the **same** area to the pixel, because the blur radius is a fraction of
    // one texel at every one of them. `Q-106` carries the figures and the configurations tried.
    assert!(
        widths.iter().all(|width| *width == widths[0]),
        "the close-up R cannot read the clarity: {widths:?}"
    );
}

/// Fourth invariant, second clause, as far as this side can take it (`Q-106`): `ssr.length` never
/// **shortens** the reflected area, and every setting reflects.
///
/// The invariant as written — "`ssr.length` croissant allonge la zone modifiée" — cannot be read
/// in the close-up R: the cube rests **on** the floor, so every ray that reaches it has travelled
/// about fifteen units, and even `ssr.length` = 1 allows forty. Measured here at four camera
/// elevations down to 5° and at floors of up to 24 tiles, the modified area does not grow at any
/// setting; what it does is stay the same to within the blur's own spread. `Q-106` carries the
/// figures. The march's positions are fitted to the **longest** ray any setting can ask for
/// (`style::ssr_longest`) exactly so that the area can never shrink, and that is what is asserted.
#[test]
fn req_scn_084_the_length_never_shortens_the_reflected_area() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    let off = gl.render_built(&mut renderer, &built);

    let mut captures = Vec::new();
    for length in 1..=5 {
        built.scene.settings.style = with_reflections(StyleSettings {
            ssr_length: length,
            ..StyleSettings::default()
        });
        let on = gl.render_built(&mut renderer, &built);
        let area = changed(&off, &on);
        assert!(
            area > 100,
            "ssr.length {length} must reflect, {area} pixels"
        );
        captures.push(area);
    }

    // From the second rung on, the maximum length is past every ray of this configuration, so the
    // area is identical: no setting loses a reflection a shorter one found.
    for pair in captures[1..].windows(2) {
        assert_eq!(pair[0], pair[1], "the area must not move: {captures:?}");
    }
}

/// Fifth invariant of §2.5 and point 2 of the lot's brief: `spl.reflections` = 0 is the Classic
/// picture **to the pixel**, nothing is compiled, allocated or submitted for it — and all of it is
/// released when the style goes back to Classic.
#[test]
fn req_scn_084_the_reflections_are_off_at_zero_and_in_classic() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);

    let classic = gl.render_built(&mut renderer, &built);
    let draws = renderer.statistics().draw_calls;
    assert_eq!(0, renderer.statistics().style_programs);
    assert_eq!(0, renderer.statistics().screen_effect_bytes);
    assert_eq!(0, renderer.statistics().screen_effect_passes);

    // Enhanced with every effect at its default: the same picture, and still nothing.
    built.scene.settings.style = ENHANCED_OFF;
    let enhanced = gl.render_built(&mut renderer, &built);
    assert_eq!(0, changed(&classic, &enhanced), "Enhanced-off is Classic");
    assert_eq!(0, renderer.statistics().style_programs);
    assert_eq!(0, renderer.statistics().screen_effect_bytes);
    assert_eq!(0, renderer.statistics().screen_effect_passes);
    assert_eq!(draws, renderer.statistics().draw_calls);

    // And Classic with every `ssr.*` turned up is still the Classic picture.
    built.scene.settings.style = StyleSettings {
        reflections: true,
        ssr_shine: 5,
        ssr_length: 5,
        ssr_clarity: 1,
        ssr_quality: 3,
        ..StyleSettings::default()
    };
    let turned_up = gl.render_built(&mut renderer, &built);
    assert_eq!(0, changed(&classic, &turned_up), "Classic ignores ssr.*");
    assert_eq!(0, renderer.statistics().style_programs);
    assert_eq!(0, renderer.statistics().screen_effect_bytes);

    // On: the two programs of the trace and its composite, the mask program, the three passes and
    // the two targets.
    built.scene.settings.style = with_reflections(StyleSettings::default());
    let _ = gl.render_built(&mut renderer, &built);
    assert_eq!(3, renderer.statistics().style_programs);
    assert_eq!(3, renderer.statistics().screen_effect_passes);
    assert!(renderer.statistics().screen_effect_bytes > 0);
    assert!(
        renderer.statistics().draw_calls > draws,
        "the mask pass draws"
    );

    // Back to Classic: released.
    built.scene.settings.style = StyleSettings::default();
    let again = gl.render_built(&mut renderer, &built);
    assert_eq!(0, changed(&classic, &again));
    assert_eq!(0, renderer.statistics().screen_effect_bytes);
    assert_eq!(0, renderer.statistics().screen_effect_passes);
    assert_eq!(draws, renderer.statistics().draw_calls);
}

/// Second invariant of §2.5, the **variant R-m**: the wall is the same shape without rotation, so
/// its `xz` box measures 64 × 8 and passes the extent test — and not one pixel of its front face,
/// of scene normal (0, 0, −1), may be modified.
///
/// That is the invariant that forbids a per-instance mask from standing alone, and the one the
/// per-fragment `+Y ≥ 0,9` of `floor_mask.frag` exists for. The front face is measured by its own
/// shade: it carries one lighting term, and the thin top strip of the box — which *is* a floor
/// surface, 64 × 8 units of normal (0, 1, 0) — carries another.
#[test]
fn req_scn_084_no_pixel_of_the_wall_is_modified() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(false, 8, 1.0, 0.0);

    let off = gl.render_built(&mut renderer, &built);
    let front = dominant_shade(&off, 0);
    built.scene.settings.style = with_reflections(StyleSettings::default());
    let on = gl.render_built(&mut renderer, &built);

    let (best, moved, total) = red_over_shade(&off, &on, front);
    assert!(total > 10_000, "the wall must fill the capture: {total}");
    assert_eq!(
        0, moved,
        "no pixel of the face of normal (0, 0, −1) may move, {moved} of {total} did"
    );
    assert!((best - 1.0).abs() < 1e-9);
}

/// Third invariant of §2.5, the **variant R-s**: the block of 8 × 8 × 8 units is modified on at
/// least one pixel of its top face; the same block at `scale = (1 ; 1 ; 0,5)` — four units on `z`,
/// so under `6 × échelle de scène` in one of its two dimensions — is modified on **no** pixel.
#[test]
fn req_scn_084_the_eight_unit_block_reflects_and_the_four_unit_one_does_not() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let mut deep = variant_r_s(1.0);
    let off = gl.render_built(&mut renderer, &deep);
    deep.scene.settings.style = with_reflections(StyleSettings::default());
    let on = gl.render_built(&mut renderer, &deep);
    assert!(
        changed(&off, &on) > 0,
        "the 8-unit block must be modified on at least one pixel"
    );

    let mut shallow = variant_r_s(0.5);
    let off = gl.render_built(&mut renderer, &shallow);
    shallow.scene.settings.style = with_reflections(StyleSettings::default());
    let on = gl.render_built(&mut renderer, &shallow);
    assert_eq!(
        0,
        changed(&off, &on),
        "the 4-unit block is not a floor and must not be modified on any pixel"
    );
}

/// The **variant R-s**: a block of one full tile, `D` = 8, `scale = (1 ; 1 ; scale_z)`, and the
/// red cube beside it — its base level with the block's top face, its `(−1, 0, 0)` face two units
/// from the block's `+x` edge, so that the reflection of the red face reaches the top face.
///
/// Both blocks are **background tiles of the same layer**, which is what puts them at the same
/// depth: a sprite is pinned to the sprite plane and a pattern of another layer is placed at that
/// layer's depth, and in either case the cube ends up twelve units in front of the block, too far
/// for its reflection to land on an eight-unit face. That cost three measurements.
fn variant_r_s(scale_z: f32) -> common::Built {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x00)
        .palette(2, 0x16)
        .pattern(solid(1, 0), 8, ProfileLayer::L2)
        .tile(solid(1, 0), 15 * 8, 14 * 8, 0)
        .pattern(solid(2, 0), 8, ProfileLayer::L2)
        .tile(solid(2, 0), 17 * 8, 14 * 8, 0)
        .build();

    let cube = |tile: &DrawTile| tile.tile_center.x > 132.0;
    for id in built.instances(cube) {
        // 10 units of the cube's own cell back towards the block, and 8 up onto its top face.
        built.instance_mut(id).offset = Vec3::new(-2.0, 12.0, 4.0);
    }
    for id in built.instances(|tile| !cube(tile)) {
        let instance = built.instance_mut(id);
        instance.scale = Vec3::new(1.0, 1.0, scale_z);
        instance.offset = Vec3::new(4.0, 4.0, 4.0);
    }
    built.rebuild_draw_list();

    built.scene.settings.shadows = false;
    built.scene.light.direction = Vector3::new(1.0, -1.0, 1.0);
    built.scene.camera.set_fov(70.0);
    let setpoint = built.scene.camera.current_mut();
    setpoint.angles = Vector3::new(30.0, 35.0, 0.0);
    setpoint.target = Vector3::new(128.0, 128.0, 4.0);
    setpoint.set_distance(30.0);
    built.scene.camera.snap_to_setpoints();
    built
}

/// Sixth invariant of §2.5, the **variant R-h**: with the Gradient skybox and a HUD pattern in
/// view, neither the skybox's pixels nor the HUD's move between `spl.reflections` = 0 and 1.
///
/// They cost nothing to exclude: the skybox leaves the far plane in the depth buffer, so the mask
/// is 0 there and the trace leaves at its first line; the HUD is drawn **after** the pass
/// (REQ-SCN-080's order), so it is neither reflective nor reflected.
#[test]
fn req_scn_084_the_skybox_and_the_hud_are_left_alone() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x00)
        .palette(5, 0x30)
        .palette(22, 0x16)
        .pattern_block(solid(1, 0), 8, 8, 8, ProfileLayer::L2, 1.0)
        .rect(solid(1, 0), 12, 11, 8, 8)
        .pattern(solid(1, 3), 8, ProfileLayer::Ui)
        .tile(solid(1, 3), 15 * 8, 8, 1)
        .pattern(solid(2, 7), 8, ProfileLayer::L1)
        .sprite(solid(2, 7), 15 * 8, 14 * 8, 1, false)
        .build();
    for id in built.instances(is_cube) {
        built.instance_mut(id).offset = Vec3::new(4.0, 4.0, 4.0);
    }
    for id in built.instances(|tile| !is_cube(tile)) {
        built.instance_mut(id).rotation = Vec3::new(90.0, 0.0, 0.0);
    }
    built.rebuild_draw_list();
    built.scene.settings.shadows = false;
    built.scene.settings.skybox = zax_render::scene::SkyboxKind::Gradient;
    built.scene.light.direction = Vector3::new(1.0, -1.0, 1.0);
    built.scene.camera.set_fov(70.0);
    let setpoint = built.scene.camera.current_mut();
    setpoint.angles = Vector3::new(30.0, 35.0, 0.0);
    setpoint.target = Vector3::new(128.0, 128.0, 12.0);
    setpoint.set_distance(40.0);
    built.scene.camera.snap_to_setpoints();

    let off = gl.render_built(&mut renderer, &built);
    built.scene.settings.style = with_reflections(StyleSettings::default());
    let on = gl.render_built(&mut renderer, &built);

    // The floor is what moves; the sky above the horizon and the HUD patch do not.
    assert!(changed(&off, &on) > 100, "the floor must still reflect");
    let hud_and_sky = (0..120).flat_map(|y| (0..off.width).map(move |x| (x, y)));
    for (x, y) in hud_and_sky {
        assert_eq!(
            off.packed(x, y),
            on.packed(x, y),
            "the skybox and the HUD must not move, ({x}, {y}) did"
        );
    }
}

/// REQ-SCN-080's order clause: the pass works on the **rendered** depth and colour, so after the
/// curvature. A bent frame therefore still reflects, and the reflection follows the bent floor.
#[test]
fn req_scn_080_the_reflections_work_on_the_bent_depth() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.bend = SpaceBend::new(20.0, 0.0, 0.0, 0.0, 1.0);

    let off = gl.render_built(&mut renderer, &built);
    built.scene.settings.style = with_reflections(StyleSettings::default());
    let on = gl.render_built(&mut renderer, &built);
    assert!(
        changed(&off, &on) > 100,
        "a bent floor reflects, on the depth the curvature left"
    );
}

/// REQ-SCN-080's last bullet: with anti-aliasing on, the pass works on the resolved frame, and the
/// sample count of the frame is unchanged.
#[test]
fn req_scn_080_the_reflections_work_on_the_resolved_frame() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.settings.set_anti_alias_samples(4);

    let off = gl.render_built(&mut renderer, &built);
    let samples = renderer.statistics().anti_alias_samples;
    assert!(samples > 1, "the fixture must be multisampled");
    built.scene.settings.style = with_reflections(StyleSettings::default());
    let on = gl.render_built(&mut renderer, &built);
    assert_eq!(samples, renderer.statistics().anti_alias_samples);
    assert!(changed(&off, &on) > 100, "a multisampled frame reflects");
}

/// REQ-SCN-084's scope clause and REQ-SCN-080's 4th bullet: the rectangle of the 2D mode is never
/// a floor surface, so a 2D-mode frame runs no pass, allocates no target and compiles no program.
#[test]
fn req_scn_080_the_two_dimensional_mode_runs_no_pass() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    built.scene.two_dimensional_mode = true;

    let classic = gl.render_built(&mut renderer, &built);
    built.scene.settings.style = with_reflections(StyleSettings::default());
    let enhanced = gl.render_built(&mut renderer, &built);

    assert_eq!(0, changed(&classic, &enhanced));
    assert_eq!(0, renderer.statistics().style_programs);
    assert_eq!(0, renderer.statistics().screen_effect_bytes);
    assert_eq!(0, renderer.statistics().screen_effect_passes);
}

/// REQ-SCN-058 as the amendment orders it: the reflections run **first**, then the occlusion. Both
/// on, the frame submits the three passes of REQ-SCN-084 and the three of REQ-SCN-083, and holds
/// the five programs of the two.
#[test]
fn req_scn_080_the_reflections_run_before_the_occlusion() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let mut built = close_up_r(true, 8, 1.0, 0.0);
    let off = gl.render_built(&mut renderer, &built);

    built.scene.settings.style = with_reflections(StyleSettings {
        ao: AmbientOcclusion::Depth,
        ..StyleSettings::default()
    });
    let both = gl.render_built(&mut renderer, &built);
    // Three programs of the occlusion, two of the trace, one of the mask.
    assert_eq!(6, renderer.statistics().style_programs);
    // The mask, the trace and its composite; the occlusion, its blur and its composite.
    assert_eq!(6, renderer.statistics().screen_effect_passes);
    assert!(changed(&off, &both) > 100);

    // Each effect alone costs its own share, which is point 2 of the brief for the pair of
    // passes. On a renderer of its own each time: a **program** of the scene family, which the
    // mask is, stays resident once it is built — as the grid's variants of step 3 do — so a
    // count taken after another style has drawn would include it.
    for (style, programs) in [
        (with_reflections(StyleSettings::default()), 3),
        (
            StyleSettings {
                enhanced: true,
                ao: AmbientOcclusion::Depth,
                ..StyleSettings::default()
            },
            3,
        ),
    ] {
        let mut alone = gl.renderer();
        built.scene.settings.style = style;
        let _ = gl.render_built(&mut alone, &built);
        assert_eq!(programs, alone.statistics().style_programs);
        assert_eq!(3, alone.statistics().screen_effect_passes);
    }
}

/// Point 1 of the lot's brief, on the shaders: the reflections are **separate programs**, not an
/// axis of the scene pass, so no variant any other draw uses can carry anything of them. The one
/// program that shares the scene's vertex stage is the mask, and its own define is what brings the
/// pre-curvature normal in.
#[test]
fn req_scn_084_no_geometry_program_carries_anything_of_the_reflections() {
    use zax_render::shaders::{self, SceneVariant};

    const BROUGHT_IN: [&str; 6] = [
        "vSceneNormal",
        "uFloorUp",
        "uIntensity",
        "uMaxLength",
        "uReferenceLength",
        "zaxBlurredColour",
    ];

    let mut sources = Vec::new();
    for bits in 0..64_u8 {
        let variant = SceneVariant {
            solid: bits & 1 != 0,
            srgb_palette: bits & 2 != 0,
            bend: bits & 4 != 0,
            centroid: bits & 8 != 0,
            voxel_grid: bits & 16 != 0,
            pixel_shadow: bits & 32 != 0,
        };
        sources.push(shaders::scene(variant));
    }
    for bend in [false, true] {
        for grid in [false, true] {
            sources.push(shaders::backdrop(bend, grid));
        }
        sources.push(shaders::shadow(bend));
    }

    for source in &sources {
        for embedded in [false, true] {
            for (fragment, stage) in [(false, &source.vertex), (true, &source.fragment)] {
                // Preprocessed as the compiler sees it: the one line `scene.vert` gained for the
                // mask is a `#ifdef ZAX_FLOOR_MASK` block, and a `#ifdef` line is text — the same
                // situation steps 3 and 4 of this lot recorded for the grid and the snapping.
                let assembled = without_the_mask(&shaders::assemble(stage, fragment, embedded));
                for name in BROUGHT_IN {
                    assert!(
                        !common::mentions(&assembled, name),
                        "{} carries {name}",
                        source.name
                    );
                }
            }
        }
    }

    // And the mask's own vertex stage does carry the one identifier it needs, in both contexts.
    for bend in [false, true] {
        let mask = shaders::floor_mask(bend);
        let vertex = shaders::assemble(&mask.vertex, false, false);
        assert!(common::mentions(&vertex, "vSceneNormal"));
        let fragment = shaders::assemble(&mask.fragment, true, false);
        assert!(common::mentions(&fragment, "uFloorUp"));
        // The mask reads no texture and writes one channel: none of the scene's other axes
        // reaches it.
        for name in ["uColorTexture", "uPalette", "zaxVoxelGrid", "uVoxelCell"] {
            assert!(
                !common::mentions(&fragment, name),
                "the mask must not carry {name}"
            );
        }
    }
}

/// `source` as the compiler sees it when `ZAX_FLOOR_MASK` is **not** defined: the guarded blocks
/// dropped, and the comments with them (a comment that names an identifier is not code).
fn without_the_mask(source: &str) -> String {
    let mut out = String::new();
    let mut skipping = 0_i32;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#if") && trimmed.contains("ZAX_FLOOR_MASK") {
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
