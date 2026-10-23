//! The Zapper: the pointer aimed through the camera at the picture plane (SPEC-08
//! REQ-INP-060 to REQ-INP-064).

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
}

/// The Zapper as the loop tracks it: the trigger and the last aimed position.
#[derive(Debug, Clone, Copy)]
pub struct ZapperState {
    trigger_held: bool,
    position: ZapperPosition,
    has_position: bool,
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
    pub fn update(&mut self, active: bool, aim: &ZapperInput<'_>) {
        if !active {
            return;
        }
        self.trigger_held = aim.right_button;
        if let Some(aimed) = try_aim(aim.view, aim.pointer, aim.size, aim.offset, aim.scene_scale) {
            self.position = aimed;
            self.has_position = true;
        }
    }

    /// Releases the trigger and points the gun away.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
