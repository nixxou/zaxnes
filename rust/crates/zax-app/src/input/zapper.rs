//! The Zapper: the pointer aimed through the camera at the picture plane (SPEC-08
//! REQ-INP-060 to REQ-INP-064).

use zax_engine3d::space_bend::{self, SpaceBend};
use zax_engine3d::vectors::Vec3;
use zax_nes::ZapperPosition;
use zax_render::camera::CameraView;
use zax_render::numerics::Vector3;

/// Height of the picture plane in scene units; the NES image is 240 lines inside it.
pub const SCENE_HEIGHT: f32 = 248.0;
/// What the port reads while the trigger is released.
pub const RELEASED_BYTE: u8 = 0x10;
/// What the port reads while the trigger is held.
pub const PRESSED_BYTE: u8 = 0x00;

/// The ray through the pointer: its origin and its direction.
#[must_use]
pub fn ray(view: &CameraView, pointer: [f32; 2], size: [f32; 2]) -> (Vector3, Vector3) {
    let width = size[0].max(1.0);
    let height = size[1].max(1.0);
    // Normalised device coordinates: x to the right, y upwards, so the screen y is flipped.
    let ndc_x = (2.0 * pointer[0] / width) - 1.0;
    let ndc_y = 1.0 - (2.0 * pointer[1] / height);
    let tan_half_fov = (view.fov * std::f32::consts::PI / 360.0).tan();
    let direction = view.forward
        + (view.right * (ndc_x * tan_half_fov * view.aspect))
        + (view.up * (ndc_y * tan_half_fov));
    (view.eye, direction)
}

/// Where the ray through the pointer meets the plane at the depth of `offset`, in scene
/// coordinates, or `None` when it does not (parallel, behind the eye, not finite).
#[must_use]
pub fn try_intersect(
    view: &CameraView,
    pointer: [f32; 2],
    size: [f32; 2],
    offset: Vector3,
) -> Option<[f32; 2]> {
    let (origin, direction) = ray(view, pointer, size);
    if direction.z.abs() < 1e-6 {
        return None;
    }
    let t = (offset.z - origin.z) / direction.z;
    if t <= 0.0 || !t.is_finite() {
        return None;
    }
    let point = [origin.x + (t * direction.x), origin.y + (t * direction.y)];
    (point[0].is_finite() && point[1].is_finite()).then_some(point)
}

/// The NES pixel under `point`, truncating towards zero as REQ-INP-063 spells out: a point left
/// of or above the image gives a negative pixel, which is the unclamped off-screen shot of
/// REQ-INP-064.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // the C# cast: the truncated scene coordinate
pub fn to_nes_pixel(point: [f32; 2], offset: Vector3, scene_scale: f32) -> ZapperPosition {
    let scale = if scene_scale > 0.0 && scene_scale <= f32::MAX {
        scene_scale
    } else {
        1.0
    };
    let x = (point[0] - offset.x) / scale;
    let y = SCENE_HEIGHT - ((point[1] - offset.y) / scale);
    ZapperPosition::new(x.trunc() as i32, y.trunc() as i32)
}

/// Aims the pointer through `view`: the NES pixel it points at, or `None` when the ray misses
/// the plane.
#[must_use]
pub fn try_aim(
    view: &CameraView,
    pointer: [f32; 2],
    size: [f32; 2],
    offset: Vector3,
    scene_scale: f32,
) -> Option<ZapperPosition> {
    try_intersect(view, pointer, size, offset).map(|point| to_nes_pixel(point, offset, scene_scale))
}

/// Half-width of the aiming window of REQ-INP-124, 144 scene units: the NES image of
/// REQ-SCN-001 widened by 16 units on each of its four edges.
pub const WINDOW_HALF_WIDTH: f32 = 144.0;

/// Half-height of the aiming window of REQ-INP-124, 136 scene units.
pub const WINDOW_HALF_HEIGHT: f32 = 136.0;

/// Radius of the sphere around `C` the candidates are looked for in. The bent image of the
/// aiming window never leaves it: the curvature of REQ-SCN-067 rolls the scene up, it never
/// spreads it out.
const SEARCH_RADIUS: f32 = 1024.0;

/// Distance between two samples of the ray, in scene units. `B` is close to an isometry (`f`
/// stays within 12 % of 1 over the whole range of angles), so a one-unit sample cannot step over
/// a crossing of a plane.
const MARCH_STEP: f32 = 1.0;

/// How many halvings the bracket is cut by; 32 takes a one-unit span under 10⁻⁹.
const BISECTIONS: u32 = 32;

/// How far a candidate's image may be from the ray before it is rejected.
const IMAGE_TOLERANCE: f32 = 0.25;

/// Whether `point` is inside the aiming window of REQ-INP-124.
#[must_use]
pub fn in_aiming_window(point: [f32; 2], scene_scale: f32) -> bool {
    (point[0] - 128.0).abs() <= WINDOW_HALF_WIDTH * scene_scale
        && (point[1] - 128.0).abs() <= WINDOW_HALF_HEIGHT * scene_scale
}

/// The point of the plane `z = offset.z` the pointer designates on a bent scene (REQ-INP-124):
/// the candidates are the points whose **image** `B(P)` lies on the aiming ray, in front of the
/// camera; the one taken is the nearest to the ray's origin among those inside the aiming
/// window, or, failing that, the nearest of all.
///
/// `B` is invertible wherever the depth bound of REQ-SCN-067 does not bite (AMD-043 §4), so the
/// candidates are read off the **ray** rather than searched for on the plane: the ray is walked
/// forward, the preimage of each sample is taken, and a change of sign of `P.z − z₀` between two
/// samples brackets a candidate, which a bisection then locates. Walking forward is what makes
/// the first candidate found the nearest one. Each candidate is checked against its own image,
/// which rejects the ones an arc-tangent branch would invent.
///
/// At curvature 0 this is [`try_intersect`], unchanged: one ray-plane intersection.
#[must_use]
pub fn try_intersect_bent(
    view: &CameraView,
    pointer: [f32; 2],
    size: [f32; 2],
    offset: Vector3,
    scene_scale: f32,
    bend: SpaceBend,
) -> Option<[f32; 2]> {
    if bend.is_identity() {
        return try_intersect(view, pointer, size, offset);
    }

    let (origin, direction) = ray(view, pointer, size);
    let length = direction.length();
    if length <= 1e-6 || !length.is_finite() {
        return None;
    }
    let step = direction * (1.0 / length);
    let (near, far) = span(origin, step)?;
    let scale = if scene_scale > 0.0 && scene_scale <= f32::MAX {
        scene_scale
    } else {
        1.0
    };

    let mut nearest = None;
    let mut previous: Option<(f32, f32)> = None;
    let mut t = near;
    while t <= far {
        match preimage(bend, origin, step, t) {
            None => {
                // The depth bound of REQ-SCN-067 reached: no preimage here, and no bracket
                // across the gap either.
                previous = None;
            }
            Some(point) => {
                let depth = point.z - offset.z;
                if let Some((previous_t, previous_depth)) = previous {
                    let crosses = depth == 0.0
                        || previous_depth == 0.0
                        || (depth < 0.0) != (previous_depth < 0.0);
                    if crosses {
                        if let Some(candidate) = refine(bend, origin, step, offset.z, previous_t, t)
                        {
                            if nearest.is_none() {
                                nearest = Some(candidate);
                            }
                            if in_aiming_window(candidate, scale) {
                                return Some(candidate);
                            }
                        }
                    }
                }
                previous = Some((t, depth));
            }
        }
        t += MARCH_STEP;
    }

    // Only candidates outside the window: the nearest is handed over unbounded, as the
    // off-screen shot of REQ-INP-064 (REQ-INP-124).
    nearest
}

/// [`try_aim`] on a scene the curvature of AMD-043 has bent (REQ-INP-124). The identity falls
/// back on REQ-INP-063.
#[must_use]
pub fn try_aim_bent(
    view: &CameraView,
    pointer: [f32; 2],
    size: [f32; 2],
    offset: Vector3,
    scene_scale: f32,
    bend: SpaceBend,
) -> Option<ZapperPosition> {
    try_intersect_bent(view, pointer, size, offset, scene_scale, bend)
        .map(|point| to_nes_pixel(point, offset, scene_scale))
}

/// The preimage of the ray sample at `t`, or `None` where `B` is not invertible.
fn preimage(bend: SpaceBend, origin: Vector3, step: Vector3, t: f32) -> Option<Vec3> {
    let q = origin + (step * t);
    let found = bend.invert(Vec3::new(q.x, q.y, q.z))?;
    (found.x.is_finite() && found.y.is_finite() && found.z.is_finite()).then_some(found)
}

/// Bisects `[low, high]`, where the preimage crosses the plane `z = plane`, and checks the point
/// found against its own image.
fn refine(
    bend: SpaceBend,
    origin: Vector3,
    step: Vector3,
    plane: f32,
    low: f32,
    high: f32,
) -> Option<[f32; 2]> {
    let start = preimage(bend, origin, step, low)?;
    let below = start.z - plane < 0.0;
    let (mut low, mut high) = (low, high);
    let mut middle = low;
    let mut found = start;
    for _ in 0..BISECTIONS {
        middle = f32::midpoint(low, high);
        found = preimage(bend, origin, step, middle)?;
        if (found.z - plane < 0.0) == below {
            low = middle;
        } else {
            high = middle;
        }
    }

    // The arc tangent of the inverse is taken on a principal branch; a candidate whose image is
    // not on the ray comes from the wrong one and is dropped.
    let image = origin + (step * middle);
    let curved = bend.apply(found);
    let distance = Vec3::distance(curved, Vec3::new(image.x, image.y, image.z));
    if distance > IMAGE_TOLERANCE {
        return None;
    }
    (found.x.is_finite() && found.y.is_finite()).then_some([found.x, found.y])
}

/// The part of the ray that is both in front of the camera and inside the search sphere.
fn span(origin: Vector3, step: Vector3) -> Option<(f32, f32)> {
    let centre = space_bend::CENTER;
    let to_origin = origin - Vector3::new(centre.x, centre.y, centre.z);
    let b = 2.0 * Vector3::dot(step, to_origin);
    let c = Vector3::dot(to_origin, to_origin) - (SEARCH_RADIUS * SEARCH_RADIUS);
    let discriminant = (b * b) - (4.0 * c);
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let near = f32::midpoint(-b, -root).max(1e-3);
    let far = f32::midpoint(-b, root);
    (far > near).then_some((near, far))
}

/// What the Zapper needs of a displayed frame: the camera and the pointer.
#[derive(Debug, Clone, Copy)]
pub struct ZapperInput<'a> {
    /// The camera view the pointer aims through.
    pub view: &'a CameraView,
    /// Pointer position in the render target, in pixels.
    pub pointer: [f32; 2],
    /// Size of the render target, in pixels.
    pub size: [f32; 2],
    /// Whether the right button, the trigger, is held.
    pub right_button: bool,
    /// Offset of the picture plane (the script's aim offset).
    pub offset: Vector3,
    /// Scene scale, 1 on the desktop.
    pub scene_scale: f32,
    /// The curvature the frame applies (`SceneState::effective_bend`), the identity by default.
    /// At the identity the aim is the ray-plane intersection of REQ-INP-063, unchanged; only a
    /// real curvature takes the inverse of REQ-INP-124.
    pub bend: SpaceBend,
}

/// What the curved aim of REQ-INP-124 was last computed from, so that it is computed again only
/// when one of them changes.
///
/// REQ-INP-061 refreshes the Zapper once per **displayed frame**, and the inverse of REQ-INP-124
/// walks the aiming ray one scene unit at a time: running it on a frame where neither the
/// pointer nor the camera nor the aim plane moved would be pure waste. This is only kept, and
/// only compared, while a curvature is active — at curvature 0 `update` is the two lines it was
/// before AMD-043.
#[derive(Debug, Clone, Copy, PartialEq)]
struct AimInputs {
    pointer: [f32; 2],
    size: [f32; 2],
    offset: Vector3,
    scene_scale: f32,
    bend: SpaceBend,
    view: CameraView,
}

/// The Zapper as the loop tracks it: the trigger and the last aimed position.
#[derive(Debug, Clone, Copy)]
pub struct ZapperState {
    trigger_held: bool,
    position: ZapperPosition,
    has_position: bool,
    /// What the last **curved** aim was computed from (REQ-INP-124); `None` while no curvature
    /// is active and after a reset.
    last_curved_aim: Option<AimInputs>,
}

impl Default for ZapperState {
    fn default() -> Self {
        Self::new()
    }
}

impl ZapperState {
    /// Creates a Zapper pointing off screen.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            trigger_held: false,
            position: ZapperPosition::OFF_SCREEN,
            has_position: false,
            last_curved_aim: None,
        }
    }

    /// Whether the trigger is held.
    #[must_use]
    pub const fn trigger_held(&self) -> bool {
        self.trigger_held
    }

    /// Where the gun was last aimed.
    #[must_use]
    pub const fn position(&self) -> ZapperPosition {
        self.position
    }

    /// Whether the gun was ever aimed at the plane since the last reset.
    #[must_use]
    pub const fn has_position(&self) -> bool {
        self.has_position
    }

    /// The byte the port reads.
    #[must_use]
    pub const fn port_byte(&self) -> u8 {
        if self.trigger_held {
            PRESSED_BYTE
        } else {
            RELEASED_BYTE
        }
    }

    /// Refreshes the Zapper from the pointer: the trigger follows the right button and the aim
    /// follows the pointer when it meets the plane. Does nothing when not `active`.
    ///
    /// On a bent scene the aim is the one of REQ-INP-124, and it is recomputed only when one of
    /// its inputs has changed — the pointer, the target size, the aim plane, the scene scale or
    /// the camera. REQ-INP-061 calls this once per displayed frame; a frame where the player
    /// moved neither the mouse nor the camera pays one comparison.
    pub fn update(&mut self, active: bool, aim: &ZapperInput<'_>) {
        if !active {
            return;
        }
        self.trigger_held = aim.right_button;

        if aim.bend.is_identity() {
            // REQ-INP-063, exactly as before AMD-043: one ray-plane intersection, no memo.
            self.last_curved_aim = None;
            if let Some(aimed) =
                try_aim(aim.view, aim.pointer, aim.size, aim.offset, aim.scene_scale)
            {
                self.position = aimed;
                self.has_position = true;
            }
            return;
        }

        let inputs = AimInputs {
            pointer: aim.pointer,
            size: aim.size,
            offset: aim.offset,
            scene_scale: aim.scene_scale,
            bend: aim.bend,
            view: *aim.view,
        };
        if self.last_curved_aim == Some(inputs) {
            // Nothing the aim depends on moved: the position stands (REQ-INP-064 keeps the last
            // valid one anyway).
            return;
        }
        self.last_curved_aim = Some(inputs);
        if let Some(aimed) = try_aim_bent(
            aim.view,
            aim.pointer,
            aim.size,
            aim.offset,
            aim.scene_scale,
            aim.bend,
        ) {
            self.position = aimed;
            self.has_position = true;
        }
    }

    /// Releases the trigger and points the gun away.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
