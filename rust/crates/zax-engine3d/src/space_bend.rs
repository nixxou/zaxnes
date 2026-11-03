//! The space bending `B` of SPEC-06 REQ-SCN-067 (AMD-043): the purely geometric curvature the
//! renderer applies to the game scene — the bulged glass of a cathode-ray television, or a
//! side-scrolling level rolled onto a tiny planet.
//!
//! It lives in this platform-free geometry crate for three reasons: it is a closed-form function
//! of a point that has to be unit-tested without a GL context; `zax-render` mirrors it in GLSL
//! (`shaders/bend.glsl`) and builds its per-axis uniforms from here; and `zax-app` inverts it for
//! the Zapper aim of REQ-INP-124. It takes part in **no** conversion: AMD-043 §1 keeps the
//! curvature out of the profiles and out of the 2D → 3D conversion, and nothing in this crate
//! calls it from a per-tile path.
//!
//! **Not** the animated deformation of REQ-SCN-043: that one moves the vertices of a motif in its
//! own local frame, before the object transform; this one is applied last, to the final scene
//! position (REQ-SCN-068).

use crate::vectors::Vec3;

/// Reference length of the left-right bend and of the horizontal arc, 256 (REQ-SCN-067 step 1).
pub const HORIZONTAL_LENGTH: f32 = 256.0;

/// Reference length of the top-bottom bend, 240 (REQ-SCN-067 step 1).
pub const VERTICAL_LENGTH: f32 = 240.0;

/// How much of the cylinder radius a depth may reach before it is held back, 0.95
/// (REQ-SCN-067 step 2). It bounds both directions of curvature.
pub const DEPTH_LIMIT: f32 = 0.95;

/// Angles under this many radians are treated as zero, which REQ-SCN-067 allows up to
/// 10⁻⁴ rad. `B` is continuous in θ and is the identity at θ = 0.
pub const ANGLE_THRESHOLD: f32 = 1e-4;

/// The arc pivot when the centre is the bottom edge of the reference frame: `−120 · s` relative
/// to `C`, that is `y = 0`, eight units below the bottom of the NES image (REQ-SCN-066).
///
/// RO-5 part 2 (owner decision): `scene.arc_center` is a **fraction** of this offset, so the pivot
/// is `EDGE_PIVOT_OFFSET · s · c` with `c` one of 0, 0.25, 0.5, 0.75, 1. At `c = 0` and `c = 1`
/// the arithmetic is bit-exact, so the two heights REQ-SCN-066 names are unchanged to the pixel.
pub const EDGE_PIVOT_OFFSET: f32 = -120.0;

/// The centre `C = (128, 120, 0)` of the reference frame, which is the fixed point of every axis
/// (REQ-E3D-410, REQ-SCN-067) and also the default camera target.
pub const CENTER: Vec3 = Vec3::new(128.0, 120.0, 0.0);

/// The curvature of the whole scene, built from the four settings of REQ-SCN-066.
///
/// [`SpaceBend::default`] is the identity, which is what the default settings (all four at 0)
/// give; [`SpaceBend::is_identity`] is what every caller tests before paying for anything.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpaceBend {
    left_right: Axis,
    top_bottom: Axis,
    arc: Axis,
    /// The arc pivot `p`, relative to `C`.
    pivot: f32,
}

/// The four partial derivatives of one wrapping `E`, in the order `∂u/∂u`, `∂u/∂w`, `∂w/∂u`,
/// `∂w/∂w`.
#[derive(Debug, Clone, Copy, Default)]
struct PairDerivatives {
    uu: f32,
    uw: f32,
    wu: f32,
    ww: f32,
}

impl SpaceBend {
    /// The identity: no curvature at all.
    pub const NONE: Self = Self {
        left_right: Axis::NONE,
        top_bottom: Axis::NONE,
        arc: Axis::NONE,
        pivot: 0.0,
    };

    /// The transform of the four settings of REQ-SCN-066.
    ///
    /// `bend_left_right` (`scene.bend_lr`), `bend_top_bottom` (`scene.bend_tb`) and
    /// `arc_horizontal` (`scene.arc_h`) are in degrees: a positive angle pushes the edges away
    /// (bulged), a negative one brings them closer (wrapping around the player).
    /// `arc_center` is `scene.arc_center`: 0 for the screen centre, 1 for the bottom edge of the
    /// reference frame, and since RO-5 part 2 any fraction between the two, which places the pivot
    /// at the matching linear interpolation of the two heights. A value outside `[0, 1]`, or one
    /// that is not a number, is brought back into the range — the setting's domain never produces
    /// one, but `--bend` and a Lua write reach this constructor too.
    /// `scene_scale` is the scene scale `s` of REQ-SCN-002, 1 on desktop.
    #[must_use]
    pub fn new(
        bend_left_right: f32,
        bend_top_bottom: f32,
        arc_horizontal: f32,
        arc_center: f32,
        scene_scale: f32,
    ) -> Self {
        let scale = if scene_scale > 0.0 && scene_scale <= f32::MAX {
            scene_scale
        } else {
            1.0
        };
        let fraction = if arc_center.is_finite() {
            arc_center.clamp(0.0, 1.0)
        } else {
            0.0
        };
        // `x * 1.0 == x`, so the far end is bit for bit the `−120 · s` the boolean of AMD-043
        // REQ-SCN-066 gave. The near end is written out rather than multiplied, a product by zero
        // of a negative number being `−0.0`: numerically the same pivot everywhere it is used, but
        // this way the uniform the renderer uploads is the very bit pattern it uploaded before.
        // The branch costs nothing — this runs when a curvature setting changes, never per frame.
        let pivot = if fraction <= 0.0 {
            0.0
        } else {
            EDGE_PIVOT_OFFSET * scale * fraction
        };
        Self {
            left_right: Axis::new(bend_left_right, HORIZONTAL_LENGTH, scale),
            top_bottom: Axis::new(bend_top_bottom, VERTICAL_LENGTH, scale),
            arc: Axis::new(arc_horizontal, HORIZONTAL_LENGTH, scale),
            pivot,
        }
    }

    /// Whether no axis bends anything, in which case `B` is the identity and every caller — the
    /// renderer's shader choice, the subdivision of a flat surface, the Zapper aim — takes its
    /// cheap path.
    #[must_use]
    pub const fn is_identity(&self) -> bool {
        !self.left_right.active && !self.top_bottom.active && !self.arc.active
    }

    /// The image `B(P)` of a scene point (REQ-SCN-067).
    #[must_use]
    pub fn apply(&self, point: Vec3) -> Vec3 {
        if self.is_identity() {
            return point;
        }

        let mut u = point.x - CENTER.x;
        let mut v = point.y - CENTER.y;
        let mut w = point.z - CENTER.z;

        // REQ-SCN-067 step 3, in this order: left-right on (u, w), top-bottom on (v, w), then
        // the arc on (u, p − v).
        if self.left_right.active {
            let (next_u, next_w, _) = wrap(&self.left_right, u, w);
            u = next_u;
            w = next_w;
        }
        if self.top_bottom.active {
            let (next_v, next_w, _) = wrap(&self.top_bottom, v, w);
            v = next_v;
            w = next_w;
        }
        if self.arc.active {
            let (next_u, next_s, _) = wrap(&self.arc, u, self.pivot - v);
            u = next_u;
            v = self.pivot - next_s;
        }

        Vec3::new(u + CENTER.x, v + CENTER.y, w + CENTER.z)
    }

    /// The preimage of `bent`: the point `P` such that `B(P) = bent` (REQ-INP-124).
    ///
    /// `None` where the depth limit of REQ-SCN-067 step 2 was reached, which is exactly where
    /// `B` stops being injective and where AMD-043 §4 leaves the aim unspecified.
    #[must_use]
    pub fn invert(&self, bent: Vec3) -> Option<Vec3> {
        if self.is_identity() {
            return Some(bent);
        }

        let mut u = bent.x - CENTER.x;
        let mut v = bent.y - CENTER.y;
        let mut w = bent.z - CENTER.z;

        // The three steps are undone in the reverse order of REQ-SCN-067 step 3. Each one acts on
        // a pair of components and leaves the third alone, so the chain unwinds exactly.
        if self.arc.active {
            let (next_u, next_s) = unwrap(&self.arc, u, self.pivot - v)?;
            u = next_u;
            v = self.pivot - next_s;
        }
        if self.top_bottom.active {
            let (next_v, next_w) = unwrap(&self.top_bottom, v, w)?;
            v = next_v;
            w = next_w;
        }
        if self.left_right.active {
            let (next_u, next_w) = unwrap(&self.left_right, u, w)?;
            u = next_u;
            w = next_w;
        }

        Some(Vec3::new(u + CENTER.x, v + CENTER.y, w + CENTER.z))
    }

    /// The Jacobian of `B` at `point`, row by row: cell `3 · i + j` is `∂Bᵢ/∂Pⱼ`.
    #[must_use]
    // u, v, w are the names REQ-SCN-067 gives the three components, j the Jacobian and d the
    // four derivatives of one wrapping.
    #[allow(clippy::many_single_char_names)]
    pub fn jacobian(&self, point: Vec3) -> [f32; 9] {
        // Identity to start with, then one 3 × 3 factor per active axis, applied on the left in
        // the order the axes are applied.
        let mut j = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let mut u = point.x - CENTER.x;
        let mut v = point.y - CENTER.y;
        let mut w = point.z - CENTER.z;

        if self.left_right.active {
            let (next_u, next_w, d) = wrap(&self.left_right, u, w);
            u = next_u;
            w = next_w;
            // (u, w) move together; v passes through.
            compose(&[d.uu, 0.0, d.uw, 0.0, 1.0, 0.0, d.wu, 0.0, d.ww], &mut j);
        }
        if self.top_bottom.active {
            // The depth `w` the top-bottom axis produces is the last use of it: the arc wraps
            // (u, p − v) and leaves the depth alone.
            let (next_v, _, d) = wrap(&self.top_bottom, v, w);
            v = next_v;
            compose(&[1.0, 0.0, 0.0, 0.0, d.uu, d.uw, 0.0, d.wu, d.ww], &mut j);
        }
        if self.arc.active {
            let (_, _, d) = wrap(&self.arc, u, self.pivot - v);
            // The pair is (u, p − v) and the result is read back as v ← p − s, so the two cross
            // terms change sign and the second diagonal term does not.
            compose(&[d.uu, -d.uw, 0.0, -d.wu, d.ww, 0.0, 0.0, 0.0, 1.0], &mut j);
        }
        j
    }

    /// The mesh normal `normal` transported by the curvature (REQ-SCN-028 as REQ-SCN-068 amends
    /// it): the cofactor matrix of the Jacobian applied to it, normalised.
    ///
    /// `point` is the scene point the normal belongs to, **before** bending.
    #[must_use]
    pub fn transform_normal(&self, point: Vec3, normal: Vec3) -> Vec3 {
        if self.is_identity() {
            return normal;
        }

        let j = self.jacobian(point);
        // The cofactor matrix is det(J) · J⁻ᵗ, built here from the cross products of the columns
        // so that the singular Jacobian the depth limit produces needs no special case.
        let c0 = Vec3::new(j[0], j[3], j[6]);
        let c1 = Vec3::new(j[1], j[4], j[7]);
        let c2 = Vec3::new(j[2], j[5], j[8]);
        let k0 = Vec3::cross(c1, c2);
        let k1 = Vec3::cross(c2, c0);
        let k2 = Vec3::cross(c0, c1);

        let transported = Vec3::new(
            (k0.x * normal.x) + (k1.x * normal.y) + (k2.x * normal.z),
            (k0.y * normal.x) + (k1.y * normal.y) + (k2.y * normal.z),
            (k0.z * normal.x) + (k1.z * normal.y) + (k2.z * normal.z),
        );
        let length = transported.squared_length().sqrt();
        if length <= 1e-12 {
            normal
        } else {
            Vec3::new(
                transported.x / length,
                transported.y / length,
                transported.z / length,
            )
        }
    }

    /// The curvatures `k` of the three axes, for the shader uniforms.
    #[must_use]
    pub const fn curvature(&self) -> Vec3 {
        Vec3::new(self.left_right.k, self.top_bottom.k, self.arc.k)
    }

    /// The recentring factors `f` of the three axes.
    #[must_use]
    pub const fn recentring(&self) -> Vec3 {
        Vec3::new(self.left_right.f, self.top_bottom.f, self.arc.f)
    }

    /// The offsets `h` of the three axes.
    #[must_use]
    pub const fn offset(&self) -> Vec3 {
        Vec3::new(self.left_right.h, self.top_bottom.h, self.arc.h)
    }

    /// 1 for an axis that bends, 0 for one that does not — the switch the shader reads so that an
    /// inactive axis costs it a comparison and nothing more.
    #[must_use]
    pub fn active(&self) -> Vec3 {
        let flag = |active: bool| if active { 1.0 } else { 0.0 };
        Vec3::new(
            flag(self.left_right.active),
            flag(self.top_bottom.active),
            flag(self.arc.active),
        )
    }

    /// The arc pivot `p`, relative to `C`.
    #[must_use]
    pub const fn arc_pivot(&self) -> f32 {
        self.pivot
    }
}

/// The wrapping `E(k, f, h)` of REQ-SCN-067 step 2: the pair `(u, w)` — the coordinate along the
/// bent axis and the depth — rolled onto a cylinder of radius `1 / k`. Returns the new pair and
/// the four partial derivatives.
// u, w, k, f, h and a are the names REQ-SCN-067 step 2 gives them.
#[allow(clippy::many_single_char_names)]
fn wrap(axis: &Axis, u: f32, w: f32) -> (f32, f32, PairDerivatives) {
    let (k, f) = (axis.k, axis.f);

    // Neither direction of curvature goes past 95 % of the radius.
    let held = w * k > DEPTH_LIMIT;
    let depth = if held { DEPTH_LIMIT / k } else { w };
    let slope = if held { 0.0 } else { 1.0 };

    let a = u * k;
    let (sin, cos) = (a.sin(), a.cos());
    let radial = 1.0 - (depth * k);

    let derivatives = PairDerivatives {
        uu: f * radial * cos,
        uw: -f * slope * sin,
        wu: f * radial * sin,
        ww: f * slope * cos,
    };
    (
        f * radial * sin / k,
        f * (((1.0 - cos) / k) + (depth * cos) - axis.h),
        derivatives,
    )
}

/// The inverse of [`wrap`]. With `r = 1 / k` and `ρ = r − w`, the pair comes out as
/// `(f ρ sin a, f (r − ρ cos a) − f h)`, so `ρ` and `a` are read back with a square root and an
/// arc tangent. `None` where the forward pass held the depth back and the inverse does not exist.
// Same names as `wrap`, plus the radius r and the angle a.
#[allow(clippy::many_single_char_names)]
fn unwrap(axis: &Axis, u: f32, w: f32) -> Option<(f32, f32)> {
    let (k, f) = (axis.k, axis.f);
    let r = 1.0 / k;

    let along = u / f;
    let deep = r - axis.h - (w / f);
    let magnitude = ((along * along) + (deep * deep)).sqrt();
    if magnitude <= 1e-9 {
        return None;
    }

    let sign = if k < 0.0 { -1.0 } else { 1.0 };
    let radius = magnitude * sign;
    let depth = r - radius;

    // Where the forward pass held the depth back, a whole half-line maps to the same image: the
    // inverse does not exist there (AMD-043 §4).
    if depth * k >= DEPTH_LIMIT - 1e-5 {
        return None;
    }

    let a = (along * sign).atan2(deep * sign);
    Some((a / k, depth))
}

/// Left-multiplies `target` by `step`, in place; both are row-major 3 × 3.
fn compose(step: &[f32; 9], target: &mut [f32; 9]) {
    let mut product = [0.0f32; 9];
    for row in 0..3 {
        for column in 0..3 {
            let mut sum = 0.0;
            for inner in 0..3 {
                sum += step[(row * 3) + inner] * target[(inner * 3) + column];
            }
            product[(row * 3) + column] = sum;
        }
    }
    *target = product;
}

/// One bent axis: the curvature `k = θ / (L · s)`, the recentring factor `f = φ / sin φ` and the
/// offset `h = 2 sin²(φ / 2) / k`, with `φ = θ / 4` (REQ-SCN-067 step 1).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Axis {
    /// Curvature `k`; zero when the axis bends nothing.
    k: f32,
    /// Recentring factor `f`; one when the axis bends nothing.
    f: f32,
    /// Offset `h`; zero when the axis bends nothing.
    h: f32,
    /// Whether the axis bends anything at all.
    active: bool,
}

impl Default for Axis {
    fn default() -> Self {
        Self::NONE
    }
}

impl Axis {
    const NONE: Self = Self {
        k: 0.0,
        f: 1.0,
        h: 0.0,
        active: false,
    };

    fn new(degrees: f32, length: f32, scene_scale: f32) -> Self {
        let theta = degrees * std::f32::consts::PI / 180.0;
        if !theta.is_finite() || theta.abs() < ANGLE_THRESHOLD || length <= 0.0 {
            return Self::NONE;
        }

        let phi = theta / 4.0;
        let k = theta / (length * scene_scale);
        let half = (phi / 2.0).sin();
        Self {
            k,
            f: phi / phi.sin(),
            h: 2.0 * half * half / k,
            active: true,
        }
    }
}
