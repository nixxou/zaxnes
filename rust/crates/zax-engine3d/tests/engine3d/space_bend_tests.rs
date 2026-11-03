//! AMD-043 REQ-SCN-067: the space bending `B`, its thirteen test vectors, its inverse (the one
//! REQ-INP-124 aims through) and the normal REQ-SCN-068 transports.

use rstest::rstest;
use zax_engine3d::space_bend::{self, SpaceBend};
use zax_engine3d::vectors::Vec3;

/// The tolerance of the vector table of REQ-SCN-067, ±0.05 unit.
const TOLERANCE: f32 = 0.05;

/// The amendment's own two-value view of the arc centre: REQ-SCN-066 gives it the screen centre
/// and the bottom edge, and the thirteen test vectors of REQ-SCN-067 are stated for those two. The
/// fractions RO-5 part 2 adds have a test of their own
/// ([`ro_5_the_pivot_is_linear_between_the_two_heights`]).
fn bend(lr: f32, tb: f32, arc: f32, at_edge: bool) -> SpaceBend {
    SpaceBend::new(lr, tb, arc, if at_edge { 1.0 } else { 0.0 }, 1.0)
}

fn close(expected: f32, actual: f32, tolerance: f32, what: &str) {
    assert!(
        (expected - actual).abs() <= tolerance,
        "{what}: expected {expected}, got {actual}"
    );
}

fn close_point(expected: Vec3, actual: Vec3, tolerance: f32) {
    close(expected.x, actual.x, tolerance, "x");
    close(expected.y, actual.y, tolerance, "y");
    close(expected.z, actual.z, tolerance, "z");
}

/// The thirteen vectors of REQ-SCN-067, in the order the table gives them. The last column of
/// each row says whether the depth bound of step 2 is active, which is where `B` stops being
/// invertible (AMD-043 §4) and so which rows the inverse criterion below leaves out.
#[rstest]
#[case(60.0, 0.0, 0.0, false, (128.0, 120.0, 0.0), (128.0, 120.0, -8.43), false)]
#[case(60.0, 0.0, 0.0, false, (256.0, 120.0, 0.0), (251.64, 120.0, 24.70), false)]
#[case(60.0, 0.0, 0.0, false, (0.0, 120.0, 0.0), (4.36, 120.0, 24.70), false)]
#[case(60.0, 0.0, 0.0, false, (256.0, 120.0, 10.0), (246.58, 120.0, 33.46), false)]
#[case(-60.0, 0.0, 0.0, false, (256.0, 120.0, 0.0), (251.64, 120.0, -24.70), false)]
#[case(-180.0, 0.0, 0.0, false, (256.0, 120.0, -200.0), (132.53, 120.0, -64.00), true)]
#[case(0.0, 60.0, 0.0, false, (128.0, 240.0, 0.0), (128.0, 235.91, 23.16), false)]
#[case(0.0, 60.0, 0.0, false, (128.0, 0.0, 0.0), (128.0, 4.09, 23.16), false)]
#[case(0.0, 0.0, 60.0, false, (0.0, 120.0, 0.0), (4.36, 95.30, 0.0), false)]
#[case(0.0, 0.0, 60.0, false, (128.0, 240.0, 0.0), (128.0, 249.81, 0.0), false)]
#[case(0.0, 0.0, 60.0, true, (0.0, 0.0, 0.0), (4.36, -24.70, 0.0), false)]
#[case(0.0, 0.0, 60.0, true, (128.0, 240.0, 0.0), (128.0, 251.19, 0.0), false)]
#[case(0.0, 0.0, -180.0, false, (128.0, 240.0, 0.0), (128.0, 179.47, 0.0), true)]
fn req_scn_067_the_thirteen_vectors_of_the_curvature(
    #[case] bend_lr: f32,
    #[case] bend_tb: f32,
    #[case] arc_h: f32,
    #[case] at_edge: bool,
    #[case] point: (f32, f32, f32),
    #[case] expected: (f32, f32, f32),
    #[case] bounded: bool,
) {
    let transform = bend(bend_lr, bend_tb, arc_h, at_edge);
    assert!(!transform.is_identity(), "one axis bends");
    let point = Vec3::new(point.0, point.1, point.2);
    let expected = Vec3::new(expected.0, expected.1, expected.2);

    close_point(expected, transform.apply(point), TOLERANCE);

    // AMD-043 §4, last paragraph: `B` is invertible wherever the depth bound does not bite, and
    // the aim is unspecified beyond. The rows marked `bounded` are exactly the two the table
    // itself annotates "(bornage actif)".
    if bounded {
        return;
    }
    let back = transform
        .invert(expected)
        .expect("B is invertible where the depth bound is idle");
    // The vector table is rounded to 0.01, so the preimage of the *rounded* image is within a
    // few hundredths of P rather than exactly P.
    close_point(point, back, 0.05);
}

/// REQ-SCN-067: `B` is the identity at θ = 0 on every axis, and the default settings are exactly
/// that — which is what makes the curvature free when nobody asks for it.
#[test]
fn req_scn_067_no_angle_at_all_is_the_identity() {
    let none = bend(0.0, 0.0, 0.0, false);
    assert!(none.is_identity());
    assert_eq!(SpaceBend::NONE, none);
    assert_eq!(SpaceBend::default(), none);
    assert!(SpaceBend::new(0.0, 0.0, 0.0, 1.0, 1.0).is_identity());

    for point in [
        Vec3::new(128.0, 120.0, 0.0),
        Vec3::new(0.0, 8.0, -50.0),
        Vec3::new(256.0, 248.0, 400.0),
    ] {
        assert_eq!(point, none.apply(point));
        assert_eq!(Some(point), none.invert(point));
        let normal = Vec3::new(0.0, 0.0, -1.0);
        assert_eq!(normal, none.transform_normal(point, normal));
    }
}

/// REQ-SCN-067: an implementation may treat as zero any |θ| below a threshold of its own, not
/// above 10⁻⁴ rad. Ours is that threshold exactly; just above it the axis bends.
#[test]
fn req_scn_067_an_angle_under_the_threshold_bends_nothing() {
    let degrees = |radians: f32| radians * 180.0 / std::f32::consts::PI;
    let under = degrees(space_bend::ANGLE_THRESHOLD * 0.5);
    let over = degrees(space_bend::ANGLE_THRESHOLD * 10.0);

    // The allowance of §3: "un seuil de son choix ne dépassant pas 10⁻⁴ rad".
    const { assert!(space_bend::ANGLE_THRESHOLD <= 1e-4) };
    assert!(bend(under, under, under, false).is_identity());
    assert!(!bend(over, 0.0, 0.0, false).is_identity());
    assert!(!bend(0.0, over, 0.0, false).is_identity());
    assert!(!bend(0.0, 0.0, over, false).is_identity());
}

/// REQ-SCN-067: `B` is continuous in θ, so a small angle moves a point by a small distance. Each
/// of the three axes is walked from 0 up to its 180° rung, and no step may jump.
#[test]
fn req_scn_067_the_curvature_is_continuous_in_the_angle() {
    let point = Vec3::new(256.0, 240.0, 20.0);
    for axis in 0..3 {
        let mut previous = Vec3::new(point.x, point.y, point.z);
        let mut degrees = 0.0_f32;
        while degrees <= 180.0 {
            let transform = match axis {
                0 => bend(degrees, 0.0, 0.0, false),
                1 => bend(0.0, degrees, 0.0, false),
                _ => bend(0.0, 0.0, degrees, false),
            };
            let image = transform.apply(point);
            assert!(
                Vec3::distance(previous, image) < 4.0,
                "axis {axis} jumped at {degrees}°: {previous:?} → {image:?}"
            );
            previous = image;
            degrees += 0.25;
        }
    }
}

/// REQ-SCN-067 step 1, the note: `φ = θ / 4` keeps the bent picture about the size of the flat
/// one — at 60° left-right the half-width goes from 128 to 123.6. The amendment's own figure,
/// which pins the recentring factor without restating the formula.
#[test]
fn req_scn_067_the_recentring_keeps_the_picture_about_its_size() {
    let transform = bend(60.0, 0.0, 0.0, false);
    let right = transform.apply(Vec3::new(256.0, 120.0, 0.0));
    close(123.6, right.x - 128.0, 0.05, "half-width at 60°");
}

/// REQ-SCN-067 step 2: the depth is held at 95 % of the radius, in both directions of curvature.
/// Past that point a whole half-line shares one image, which is why `invert` refuses there.
#[test]
fn req_scn_067_the_depth_is_held_at_95_percent_of_the_radius() {
    for degrees in [180.0_f32, -180.0] {
        let transform = bend(degrees, 0.0, 0.0, false);
        // k = θ / 256, so the radius is 256 / |θ| ≈ 81.5 units and the bound sits at ≈ 77.4.
        let radius = space_bend::HORIZONTAL_LENGTH / degrees.abs().to_radians();
        let held = space_bend::DEPTH_LIMIT * radius;
        let far = Vec3::new(128.0, 120.0, (held * 4.0).copysign(degrees));
        let further = Vec3::new(128.0, 120.0, (held * 8.0).copysign(degrees));
        assert_eq!(
            transform.apply(far),
            transform.apply(further),
            "both are past the bound at {degrees}°, so both share one image"
        );
        assert_eq!(None, transform.invert(transform.apply(far)));
    }
}

/// REQ-INP-124's own criterion, inverted: for each unbounded vector of REQ-SCN-067, the preimage
/// of `B(P)` computed at full precision is `P` to well under the 0.5 unit the aim is allowed.
#[rstest]
#[case(60.0, 0.0, 0.0, false, (256.0, 120.0, 0.0))]
#[case(60.0, 0.0, 0.0, false, (0.0, 120.0, 0.0))]
#[case(60.0, 0.0, 0.0, false, (256.0, 120.0, 10.0))]
#[case(-60.0, 0.0, 0.0, false, (256.0, 120.0, 0.0))]
#[case(0.0, 60.0, 0.0, false, (128.0, 240.0, 0.0))]
#[case(0.0, 60.0, 0.0, false, (128.0, 0.0, 0.0))]
#[case(0.0, 0.0, 60.0, false, (0.0, 120.0, 0.0))]
#[case(0.0, 0.0, 60.0, true, (0.0, 0.0, 0.0))]
#[case(90.0, 0.0, 0.0, false, (0.0, 8.0, 0.0))]
#[case(90.0, -60.0, 30.0, true, (40.0, 200.0, -12.0))]
#[case(-150.0, 120.0, -90.0, false, (200.0, 30.0, 6.0))]
fn req_inp_124_the_inverse_recovers_the_point_it_came_from(
    #[case] bend_lr: f32,
    #[case] bend_tb: f32,
    #[case] arc_h: f32,
    #[case] at_edge: bool,
    #[case] point: (f32, f32, f32),
) {
    let transform = bend(bend_lr, bend_tb, arc_h, at_edge);
    let point = Vec3::new(point.0, point.1, point.2);
    let back = transform
        .invert(transform.apply(point))
        .expect("no depth bound on these vectors");
    assert!(
        Vec3::distance(point, back) < 0.01,
        "B⁻¹(B(P)) = {back:?}, expected {point:?}"
    );
}

/// REQ-SCN-068, last point (REQ-SCN-028 amended): the normal is the mesh normal **transported**
/// by the curvature. At the fixed point `C` it does not turn at all; at the edge it turns with
/// the surface, by the angle `a = u · k` the wrapping of step 2 rolls that column through.
#[test]
fn req_scn_068_the_normal_is_transported_by_the_curvature() {
    let transform = bend(90.0, 0.0, 0.0, false);
    // The NES plane faces the default camera, which looks towards growing z (REQ-SCN-001).
    let flat = Vec3::new(0.0, 0.0, -1.0);

    // At C the curvature is locally a scaling along the axis, so the normal keeps its direction.
    let at_centre = transform.transform_normal(space_bend::CENTER, flat);
    close(0.0, at_centre.x, 1e-3, "x at C");
    close(-1.0, at_centre.z, 1e-3, "z at C");

    // A positive angle pushes the edges **away** from the camera: at 90° the right column lands
    // at a larger z, so the surface is convex towards the viewer and the outward normal of the
    // right edge leans to the right. The turn is a = u · k = 128 · (π/2) / 256 = 45°.
    let right = Vec3::new(256.0, 120.0, 0.0);
    assert!(transform.apply(right).z > 0.0, "the right edge moves away");
    let at_edge = transform.transform_normal(right, flat);
    close(1.0, at_edge.squared_length(), 1e-3, "unit length");
    close(
        45.0_f32.to_radians().sin(),
        at_edge.x,
        1e-3,
        "sin 45° to the right",
    );
    close(
        -45.0_f32.to_radians().cos(),
        at_edge.z,
        1e-3,
        "cos 45° forward",
    );
    close(0.0, at_edge.y, 1e-3, "a left-right bend leaves y alone");

    // A negative angle wraps the scene around the player, so the same normal leans the other way.
    let wrapping = bend(-90.0, 0.0, 0.0, false);
    assert!(wrapping.apply(right).z < 0.0, "the right edge comes closer");
    assert!(wrapping.transform_normal(right, flat).x < -0.5);

    // And the three axes at 0 leave every normal exactly as it was.
    let identity = bend(0.0, 0.0, 0.0, false);
    assert_eq!(
        flat,
        identity.transform_normal(Vec3::new(10.0, 20.0, 30.0), flat)
    );
}

/// REQ-SCN-067: the scene scale `s` divides the curvature, so a scene drawn at half size bends
/// by the same angle over its own width. A scale that is not a positive finite number reads as 1.
#[test]
fn req_scn_067_the_scene_scale_divides_the_curvature() {
    let desktop = SpaceBend::new(90.0, 0.0, 0.0, 0.0, 1.0);
    let half = SpaceBend::new(90.0, 0.0, 0.0, 0.0, 0.5);
    // k = θ / (L · s): halving the scale doubles the curvature, so the same angle is swept over
    // half the width.
    assert!(
        (half.curvature().x - 2.0 * desktop.curvature().x).abs() < 1e-6,
        "k = θ / (L · s): {} against {}",
        half.curvature().x,
        desktop.curvature().x
    );

    for bad in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            desktop.curvature().x,
            SpaceBend::new(90.0, 0.0, 0.0, 0.0, bad).curvature().x,
            "a scale of {bad} reads as 1"
        );
    }
}

/// REQ-SCN-066: the arc centre picks the pivot — the screen centre, or `y = 0`, eight units below
/// the bottom of the NES image, which is `−120` from `C`.
#[test]
fn req_scn_066_the_arc_centre_picks_the_pivot() {
    assert_eq!(0.0, bend(0.0, 0.0, 60.0, false).arc_pivot());
    assert_eq!(-120.0, bend(0.0, 0.0, 60.0, true).arc_pivot());
    assert_eq!(-60.0, SpaceBend::new(0.0, 0.0, 60.0, 1.0, 0.5).arc_pivot());
}

/// RO-5 part 2 (owner decision): the arc centre is a **fraction** of the way between the two
/// heights REQ-SCN-066 names, so the pivot is linear in it and monotone — and the two ends are the
/// very bit patterns the boolean produced, so a capture at 0 or at 1 is pixel-identical to the one
/// of before this lot.
#[test]
fn ro_5_the_pivot_is_linear_between_the_two_heights() {
    // The two ends, bit for bit (`to_bits`, not `==`, which would let `−0.0` through).
    assert_eq!(
        0.0_f32.to_bits(),
        SpaceBend::new(0.0, 0.0, 60.0, 0.0, 1.0)
            .arc_pivot()
            .to_bits()
    );
    assert_eq!(
        (space_bend::EDGE_PIVOT_OFFSET).to_bits(),
        SpaceBend::new(0.0, 0.0, 60.0, 1.0, 1.0)
            .arc_pivot()
            .to_bits()
    );
    // And the whole struct is equal to what the two values gave before, so every uniform the
    // renderer uploads — not the pivot alone — is unchanged at both ends.
    assert_eq!(
        bend(10.0, -20.0, 60.0, false),
        SpaceBend::new(10.0, -20.0, 60.0, 0.0, 1.0)
    );
    assert_eq!(
        bend(10.0, -20.0, 60.0, true),
        SpaceBend::new(10.0, -20.0, 60.0, 1.0, 1.0)
    );

    // The five rungs the setting offers, at the desktop scale: linear, strictly monotone.
    let pivots: Vec<f32> = [0.0, 0.25, 0.5, 0.75, 1.0]
        .into_iter()
        .map(|centre| SpaceBend::new(0.0, 0.0, 60.0, centre, 1.0).arc_pivot())
        .collect();
    assert_eq!(vec![0.0, -30.0, -60.0, -90.0, -120.0], pivots);
    for pair in pivots.windows(2) {
        assert!(
            pair[1] < pair[0],
            "the pivot only ever goes down: {pivots:?}"
        );
    }

    // The scene scale divides it as it divides the two ends (REQ-SCN-067).
    assert_eq!(-30.0, SpaceBend::new(0.0, 0.0, 60.0, 0.5, 0.5).arc_pivot());

    // A value outside the range, or not a number, is brought back in: `--bend` and a Lua write
    // reach this constructor without the domain's snapping.
    for (centre, expected) in [
        (-1.0_f32, 0.0_f32),
        (2.0, space_bend::EDGE_PIVOT_OFFSET),
        (f32::NAN, 0.0),
        (f32::INFINITY, 0.0),
    ] {
        assert_eq!(
            expected,
            SpaceBend::new(0.0, 0.0, 60.0, centre, 1.0).arc_pivot(),
            "a centre of {centre}"
        );
    }

    // An intermediate pivot really moves the picture: the arc is wrapped around `p − v`, so the
    // image of a point differs at every rung.
    let point = Vec3::new(40.0, 60.0, 0.0);
    let images: Vec<Vec3> = [0.0, 0.25, 0.5, 0.75, 1.0]
        .into_iter()
        .map(|centre| SpaceBend::new(0.0, 0.0, 60.0, centre, 1.0).apply(point))
        .collect();
    for (first, second) in images.iter().zip(images.iter().skip(1)) {
        assert!(
            Vec3::distance(*first, *second) > 0.5,
            "two neighbouring rungs give the same picture: {images:?}"
        );
    }
}

/// REQ-SCN-067 step 1: the per-axis constants the shader reads are `k = θ / (L · s)`,
/// `f = φ / sin φ` and `h = 2 sin²(φ / 2) / k`, and an idle axis publishes `k = 0`, `f = 1`,
/// `h = 0` so that the GLSL mirror of this file needs no special case.
#[test]
fn req_scn_067_the_uniforms_an_idle_axis_publishes_are_the_neutral_ones() {
    let only_arc = bend(0.0, 0.0, 60.0, false);
    assert_eq!(Vec3::new(0.0, 0.0, 1.0), only_arc.active());
    assert_eq!(0.0, only_arc.curvature().x);
    assert_eq!(1.0, only_arc.recentring().x);
    assert_eq!(0.0, only_arc.offset().x);

    // The arc's own three, recomputed from the formulas of step 1.
    let theta = 60.0_f32.to_radians();
    let phi = theta / 4.0;
    let k = theta / space_bend::HORIZONTAL_LENGTH;
    close(k, only_arc.curvature().z, 1e-6, "k");
    close(phi / phi.sin(), only_arc.recentring().z, 1e-6, "f");
    let half = (phi / 2.0).sin();
    close(2.0 * half * half / k, only_arc.offset().z, 1e-5, "h");
}
