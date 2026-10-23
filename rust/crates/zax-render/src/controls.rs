//! The user controls of SPEC-06 §3.5 applied to the camera setpoints: the extended rotation and
//! zoom actions, the mouse wheel and the left-button drag.
//!
//! Everything here is driven by elapsed real time (REQ-SCN-014, REQ-SCN-015, REQ-SCN-059) and
//! only acts while the game is running (REQ-SCN-018); the caller decides that.

use std::ops::BitOr;

use crate::camera::SceneCamera;
use crate::geometry;

/// The camera actions of SPEC-08 §4.1 the renderer-side controls consume, as a set of flags
/// with the bit values of that table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CameraAction(u32);

impl CameraAction {
    /// No action.
    pub const NONE: Self = Self(0);
    /// Rotate left: decreases `ry` (REQ-SCN-014).
    pub const ROTATE_LEFT: Self = Self(0x400);
    /// Rotate right: increases `ry`.
    pub const ROTATE_RIGHT: Self = Self(0x800);
    /// Rotate up: decreases `rx`.
    pub const ROTATE_UP: Self = Self(0x1000);
    /// Rotate down: increases `rx`.
    pub const ROTATE_DOWN: Self = Self(0x2000);
    /// Zoom in: decreases the distance (REQ-SCN-015).
    pub const ZOOM_IN: Self = Self(0x100);
    /// Zoom out: increases the distance.
    pub const ZOOM_OUT: Self = Self(0x200);

    /// The raw bits.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Whether every flag of `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for CameraAction {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Applies the held rotation and zoom actions over `real_seconds` (REQ-SCN-014, REQ-SCN-015).
/// Opposite actions do not cancel out: RotLeft wins over RotRight, RotDown over RotUp and
/// ZoomIn over ZoomOut.
pub fn apply(camera: &mut SceneCamera, held: CameraAction, real_seconds: f32) {
    if real_seconds <= 0.0 || held == CameraAction::NONE {
        return;
    }

    let setpoint = camera.current_mut();
    let rotation = geometry::ROTATION_DEGREES_PER_SECOND * real_seconds;

    if held.contains(CameraAction::ROTATE_LEFT) {
        setpoint.angles.y -= rotation;
    } else if held.contains(CameraAction::ROTATE_RIGHT) {
        setpoint.angles.y += rotation;
    }

    if held.contains(CameraAction::ROTATE_DOWN) {
        setpoint.angles.x += rotation;
    } else if held.contains(CameraAction::ROTATE_UP) {
        setpoint.angles.x -= rotation;
    }

    let zoom = geometry::ZOOM_UNITS_PER_SECOND * real_seconds;
    if held.contains(CameraAction::ZOOM_IN) {
        setpoint.set_distance(setpoint.distance() - zoom);
    } else if held.contains(CameraAction::ZOOM_OUT) {
        setpoint.set_distance(setpoint.distance() + zoom);
    }
}

/// Applies `notches` of mouse wheel, 5 units of distance each, positive forward (REQ-SCN-016,
/// DEC-08-3). It only acts when no zoom action is held.
pub fn apply_wheel(camera: &mut SceneCamera, notches: f32, held: CameraAction) {
    if notches == 0.0 || held.contains(CameraAction::ZOOM_IN) || held.contains(CameraAction::ZOOM_OUT)
    {
        return;
    }

    let setpoint = camera.current_mut();
    setpoint.set_distance(setpoint.distance() - (notches * geometry::WHEEL_UNITS_PER_NOTCH));
}

/// Applies a left-button drag of `pixels_right` and `pixels_up` pixels, 0.1° per pixel
/// (REQ-SCN-017).
pub fn apply_drag(camera: &mut SceneCamera, pixels_right: f32, pixels_up: f32) {
    if pixels_right == 0.0 && pixels_up == 0.0 {
        return;
    }

    let setpoint = camera.current_mut();
    setpoint.angles.y += pixels_right * geometry::DRAG_DEGREES_PER_PIXEL;
    setpoint.angles.x -= pixels_up * geometry::DRAG_DEGREES_PER_PIXEL;
}
