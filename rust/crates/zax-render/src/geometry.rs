//! The scene constants of SPEC-06 §4.3 and the closed-form geometry they imply: the reference
//! frame, the default camera placement, the corrected aim point and the HUD plane.

use zax_profiles::values::Vec4F;

use crate::numerics::{Matrix4x4, Vector3};

/// Width of the reference frame, 256 scene units (REQ-E3D-410).
pub const FRAME_WIDTH: f32 = 256.0;

/// Height of the reference frame, 240 scene units (REQ-E3D-410).
pub const FRAME_HEIGHT: f32 = 240.0;

/// Height of the conversion grid, `Hs = 248` (REQ-E3D-410).
pub const SCENE_HEIGHT: f32 = 248.0;

/// The centre `C = (128, 120, 0)` of the reference frame (REQ-E3D-410).
pub const CENTER: Vector3 = Vector3::new(128.0, 120.0, 0.0);

/// The default setpoint angles, `(0.81°, 0.81°, 0)` (REQ-SCN-012).
pub const DEFAULT_ANGLES: Vector3 = Vector3::new(0.81, 0.81, 0.0);

/// Near plane, 0.3 unit (REQ-SCN-004).
pub const NEAR_PLANE: f32 = 0.3;

/// Far plane, 5000 units (REQ-SCN-004).
pub const FAR_PLANE: f32 = 5000.0;

/// Default field of view in degrees (REQ-SCN-005).
pub const DEFAULT_FOV: f32 = 70.0;

/// Lowest field of view in degrees (REQ-SCN-005).
pub const MIN_FOV: f32 = 45.0;

/// Highest field of view in degrees (REQ-SCN-005).
pub const MAX_FOV: f32 = 95.0;

/// Highest camera setpoint distance (REQ-SCN-011).
pub const MAX_DISTANCE: f32 = 512.0;

/// Rotation speed of the extended actions, 30°/s (REQ-SCN-014).
pub const ROTATION_DEGREES_PER_SECOND: f32 = 30.0;

/// Zoom speed of the extended actions, 70 units/s (REQ-SCN-015).
pub const ZOOM_UNITS_PER_SECOND: f32 = 70.0;

/// Distance change of one mouse wheel notch, 5 units (REQ-SCN-016, DEC-08-3).
pub const WHEEL_UNITS_PER_NOTCH: f32 = 5.0;

/// Rotation per pixel of mouse drag, 0.1° (REQ-SCN-017).
pub const DRAG_DEGREES_PER_PIXEL: f32 = 0.1;

/// Length of the mode transition window, 3 s (REQ-SCN-006, REQ-SCN-020).
pub const MODE_TRANSITION_SECONDS: f32 = 3.0;

/// Radius of the scene the shadow map has to cover, 512 units (REQ-SCN-030).
pub const SHADOW_DISTANCE: f32 = 512.0;

/// Default shadow map side in texels: 4096 since AMD-034 §2, which replaces the 1024 of
/// DEC-R2-D4.
///
/// At 1024 texels one texel spans about 0.4 unit on the world map of Super Mario Bros. 3, more
/// than a status-bar glyph is wide, so the shadow of a one-unit stroke breaks into dots: 920
/// shadowed pixels in the status bar against 5 188 in the original (AMD-034 §1). The covered
/// area, the single map and the hard edges of REQ-SCN-030 do not change.
pub const SHADOW_MAP_SIZE: i32 = 4096;

/// Smallest shadow map the fallback of AMD-034 §2 ever settles for. A size the hardware refuses
/// is halved — 4096, then 2048, then 1024 — but a map asked for explicitly below this floor
/// (the seam tests use 128) is still honoured.
pub const MIN_SHADOW_MAP_SIZE: i32 = 64;

/// The depth bias of REQ-SCN-031, about 0.02 unit; the floor of [`shadow_bias`], which AMD-034
/// §2 keeps as the value the texel-sized bias may never go under.
pub const SHADOW_DEPTH_BIAS: f32 = 0.02;

/// The normal offset of REQ-SCN-031, about 0.02 unit.
pub const SHADOW_NORMAL_OFFSET: f32 = 0.02;

/// How many shadow-map texels the depth bias has to cover: one texel since AMD-034 §2, which
/// lowers the 1.5 of Q-032.
///
/// A receiving fragment and the texel it samples are up to one texel apart on the surface, and
/// the light crosses that texel at an angle, so the depth they disagree by is of the order of
/// the texel (Q-032 took its diagonal). AMD-034 §2 takes the texel itself: the bias is
/// subtracted from every shadow, so on a map four times finer it was shortening the very shadows
/// of thin strokes the amendment is about.
pub const SHADOW_BIAS_TEXELS: f32 = 1.0;

/// Highest sample count the anti-aliasing of AMD-032 §2 ever asks for.
pub const MAX_ANTI_ALIAS_SAMPLES: i32 = 8;

/// Scale factor of the HUD plane, 0.5 (REQ-SCN-045).
pub const HUD_SCALE: f32 = 0.5;

/// Depth nudge applied by the renderer to sprite tiles, ±0.1 (REQ-SCN-037).
pub const SPRITE_NUDGE: f32 = 0.1;

/// Opacity of a tile that hides the followed shape in FPV (REQ-SCN-022).
pub const OCCLUDER_ALPHA: f32 = 0.5;

/// The visible rectangle `R` of SPEC-06 REQ-SCN-003 and SPEC-04B REQ-E3D-410:
/// `R = [c0, 256 − c2] × [c1, 240 − c3]`, in scene units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VisibleRect {
    /// Left edge, `c0`.
    pub min_x: f32,
    /// Bottom edge, `c1`.
    pub min_y: f32,
    /// Right edge, `256 − c2`.
    pub max_x: f32,
    /// Top edge, `240 − c3`.
    pub max_y: f32,
}

impl VisibleRect {
    /// Builds the rectangle from the four clip margins (DEC-R2-T1).
    #[must_use]
    pub fn from_clipping(clipping: Vec4F) -> Self {
        Self {
            min_x: clipping.x,
            min_y: clipping.y,
            max_x: FRAME_WIDTH - clipping.z,
            max_y: FRAME_HEIGHT - clipping.w,
        }
    }

    /// Width of the rectangle.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    /// Height of the rectangle.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }

    /// Centre of the rectangle, in the `z = 0` plane.
    #[must_use]
    pub fn center(&self) -> Vector3 {
        Vector3::new(
            f32::midpoint(self.min_x, self.max_x),
            f32::midpoint(self.min_y, self.max_y),
            0.0,
        )
    }

    /// Whether the scene point `(x, y)` survives the clipping of REQ-SCN-036.
    #[must_use]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

/// The shadow map sizes to try, largest first, for a map of `requested` texels on hardware that
/// advertises `max_texture_size` (AMD-034 §2): the requested size capped by the hardware limit,
/// then halved — 4096, 2048, 1024 for the default — down to [`MIN_SHADOW_MAP_SIZE`].
///
/// OpenGL 3.3 only guarantees `GL_MAX_TEXTURE_SIZE` ≥ 1024, so a 4096-texel map is not a size
/// every implementation has to accept. The cap skips the sizes the driver already says it cannot
/// hold; the rest of the chain is there for the ones it accepts to allocate but whose
/// framebuffer comes out incomplete.
pub fn shadow_map_sizes(requested: i32, max_texture_size: i32) -> impl Iterator<Item = i32> {
    let start = requested
        .max(MIN_SHADOW_MAP_SIZE)
        .min(max_texture_size.max(MIN_SHADOW_MAP_SIZE));
    std::iter::successors(Some(start), |size| Some(size / 2))
        .take_while(|&size| size >= MIN_SHADOW_MAP_SIZE)
}

/// The depth bias of the shadow test in scene units: [`SHADOW_BIAS_TEXELS`] texels of the map in
/// use, never under [`SHADOW_DEPTH_BIAS`] (AMD-034 §2, amending Q-032). `texel_size` is the side
/// of one shadow-map texel, in scene units.
#[must_use]
pub fn shadow_bias(texel_size: f32) -> f32 {
    SHADOW_DEPTH_BIAS.max(SHADOW_BIAS_TEXELS * texel_size)
}

/// Samples per pixel the `video.anti_alias` option of SPEC-09 REQ-PERS-017 stands for: 0 off,
/// 1 four samples, 2 eight samples (AMD-032 §2); anything else means off.
///
/// REQ-PERS-017 lets the reimplementation apply a full-screen anti-aliasing and names FXAA and
/// SMAA for the two levels. AMD-032 takes multisampling instead: the scene is a mass of
/// hard-edged flat faces with nearest-neighbour texturing (REQ-SCN-034), so a post-processing
/// filter would blur the pixel art it is supposed to keep crisp, while multisampling only
/// touches the geometric edges.
#[must_use]
pub const fn anti_alias_samples(option: i32) -> i32 {
    match option {
        1 => 4,
        2 => MAX_ANTI_ALIAS_SAMPLES,
        _ => 1,
    }
}

/// Rounds a requested sample count down to a power of two no greater than `supported`, which is
/// what a multisampled attachment can be asked for; 1 when anti-aliasing is off or nothing above
/// one sample is supported.
#[must_use]
pub fn clamp_anti_alias_samples(samples: i32, supported: i32) -> i32 {
    let limit = samples.max(1).min(supported.max(1));
    let mut granted = 1;
    while granted * 2 <= limit {
        granted *= 2;
    }

    granted
}

/// The default setpoint distance `d₀ = 104 / atan(FOV_rad / 2)` (REQ-SCN-012). The arc tangent
/// really is taken of half the field of view expressed in radians.
#[must_use]
pub fn default_distance(fov_degrees: f32) -> f32 {
    104.0 / (fov_degrees * std::f32::consts::PI / 180.0 / 2.0).atan()
}

/// The HUD plane distance `d_HUD = 60 / atan(FOV_rad / 2)` (REQ-SCN-045).
#[must_use]
pub fn hud_distance(fov_degrees: f32) -> f32 {
    60.0 / (fov_degrees * std::f32::consts::PI / 180.0 / 2.0).atan()
}

/// The three columns of a rotation: the images of X, Y and Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Basis {
    /// The image of X.
    pub right: Vector3,
    /// The image of Y.
    pub up: Vector3,
    /// The image of Z.
    pub forward: Vector3,
}

/// The rotation `q = R_Y(ry) · R_X(rx) · R_Z(rz)` of REQ-SCN-007 step 1, in the left-handed
/// scene frame, or `q = R_Z(ry) · R_X(rx)` when the local camera setting is on (REQ-SCN-009).
/// `angles` are `(rx, ry, rz)` in degrees.
#[must_use]
pub fn rotation(angles: Vector3, local_camera: bool) -> Basis {
    let x = rot_x(angles.x);
    let q = if local_camera {
        Matrix4x4::multiply(&rot_z(angles.y), &x)
    } else {
        Matrix4x4::multiply(&Matrix4x4::multiply(&rot_y(angles.y), &x), &rot_z(angles.z))
    };

    // The matrices are stored row by row and applied to column vectors here (REQ-SCN-007,
    // REQ-E3D-407); the product of the stored cells is the right one for that convention too.
    Basis {
        right: Vector3::new(q.m11, q.m21, q.m31),
        up: Vector3::new(q.m12, q.m22, q.m32),
        forward: Vector3::new(q.m13, q.m23, q.m33),
    }
}

/// The forward direction `f = q · (0, 0, 1)` of REQ-SCN-007 step 3, which for `rz = 0` is
/// `(cos rx · sin ry, −sin rx, cos rx · cos ry)`.
#[must_use]
pub fn forward(angles: Vector3, local_camera: bool) -> Vector3 {
    rotation(angles, local_camera).forward
}

/// The effective target `T′ = T + ((c0 − c2) / 2, (c1 − c3) / 2, 0)` of REQ-SCN-007 step 2: the
/// target recentred on the visible rectangle. FPV does not recentre (REQ-SCN-010).
#[must_use]
pub fn effective_target(target: Vector3, clipping: Vec4F) -> Vector3 {
    target
        + Vector3::new(
            (clipping.x - clipping.z) * 0.5,
            (clipping.y - clipping.w) * 0.5,
            0.0,
        )
}

/// The aimed point `P*` of REQ-SCN-008: in the `z = 0` plane, the intersection of `M1M2` and
/// `M3M4`, where each `Mi` is where the bisector issued from the eye of the angle under which the
/// eye sees the segment lands. `fallback` is returned when the two lines are parallel, in
/// practice the effective target.
#[must_use]
#[allow(clippy::many_single_char_names)] // The corners A to D and points of REQ-SCN-008.
pub fn aim_point(eye: Vector3, rect: VisibleRect, fallback: Vector3) -> Vector3 {
    let a = Vector3::new(rect.min_x, rect.min_y, 0.0);
    let b = Vector3::new(rect.max_x, rect.min_y, 0.0);
    let c = Vector3::new(rect.min_x, rect.max_y, 0.0);
    let d = Vector3::new(rect.max_x, rect.max_y, 0.0);

    let m1 = bisector(a, b, eye);
    let m2 = bisector(c, d, eye);
    let m3 = bisector(a, d, eye);
    let m4 = bisector(b, c, eye);

    // Intersection on the x and y components only (REQ-SCN-008).
    let r1x = m2.x - m1.x;
    let r1y = m2.y - m1.y;
    let r2x = m4.x - m3.x;
    let r2y = m4.y - m3.y;
    let denominator = (r1x * r2y) - (r1y * r2x);
    if denominator.abs() < 1e-6 {
        return Vector3::new(fallback.x, fallback.y, 0.0);
    }

    let t = (((m3.x - m1.x) * r2y) - ((m3.y - m1.y) * r2x)) / denominator;
    Vector3::new(m1.x + (t * r1x), m1.y + (t * r1y), 0.0)
}

/// Where the bisector issued from `eye` of the angle under which the eye sees `[p, q]` lands:
/// `p + (q − p) · |p − E| / (|p − E| + |q − E|)`. A degenerate case (the eye on a vertex) gives
/// the midpoint (REQ-SCN-008).
///
/// AMD-019 (Q-046) settles what "degenerate" means: the eye coincides with one of the two
/// vertices, that is **at least one** of the two distances is zero. The general formula stays
/// defined there — it returns the vertex itself — so the midpoint has to be taken explicitly.
#[must_use]
pub fn bisector(p: Vector3, q: Vector3, eye: Vector3) -> Vector3 {
    let dp = Vector3::distance(p, eye);
    let dq = Vector3::distance(q, eye);
    let sum = dp + dq;

    // Q-046: either distance zero — not only both — is the degenerate case.
    let t = if dp <= 1e-6 || dq <= 1e-6 || sum <= 1e-6 {
        0.5
    } else {
        dp / sum
    };
    p + ((q - p) * t)
}

fn sin_cos(degrees: f32) -> (f32, f32) {
    let r = degrees * std::f32::consts::PI / 180.0;
    (r.sin(), r.cos())
}

fn rot_x(degrees: f32) -> Matrix4x4 {
    let (s, c) = sin_cos(degrees);
    Matrix4x4::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, c, -s, 0.0],
        [0.0, s, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

fn rot_y(degrees: f32) -> Matrix4x4 {
    let (s, c) = sin_cos(degrees);
    Matrix4x4::from_rows([
        [c, 0.0, s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-s, 0.0, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

fn rot_z(degrees: f32) -> Matrix4x4 {
    let (s, c) = sin_cos(degrees);
    Matrix4x4::from_rows([
        [c, -s, 0.0, 0.0],
        [s, c, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
}
