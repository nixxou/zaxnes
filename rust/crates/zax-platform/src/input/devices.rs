//! The input devices as the binding layer sees them: physical key positions, a pointer, and
//! gamepads reduced to the device-neutral controls of [`PhysicalControl`].

use super::control::PhysicalControl;

/// The keyboard as the binding layer sees it: physical key positions, nothing else
/// (DEC-08-5, REQ-INP-012).
pub trait KeyboardDevice {
    /// Whether the key at physical position `key_code` is held.
    fn is_down(&self, key_code: i32) -> bool;

    /// Appends the physical positions of every key currently held to `keys`, for the capture
    /// of REQ-INP-042.
    fn collect_held_keys(&self, keys: &mut Vec<i32>);
}

/// The mouse as the input layer sees it (REQ-INP-120): a position in the render target, two
/// buttons and a wheel. Mouse buttons are deliberately **not** bindable (REQ-INP-012).
pub trait PointerDevice {
    /// Pointer position in the render target, in pixels, origin top left.
    fn position(&self) -> [f32; 2];

    /// Whether the left button is held (camera orbit, REQ-INP-091).
    fn left_button(&self) -> bool;

    /// Whether the right button is held (Zapper trigger, REQ-INP-062).
    fn right_button(&self) -> bool;

    /// The wheel notches accumulated since the previous call, positive forward, and resets the
    /// accumulator (REQ-INP-090).
    fn take_wheel_notches(&mut self) -> f32;

    /// Whether the system cursor is shown (REQ-INP-110, REQ-INP-111).
    fn cursor_visible(&self) -> bool;

    /// Shows or hides the system cursor.
    fn set_cursor_visible(&mut self, visible: bool);
}

/// One gamepad, reduced to the device-neutral controls of [`PhysicalControl`] with the dead
/// zones and the threshold of SPEC-08 §3.4 already applied.
pub trait GamepadDevice {
    /// Stable identity used as the persistence key of this pad's binding set
    /// (SPEC-08 §2, REQ-INP-025).
    fn identity(&self) -> &str;

    /// Name shown to the user (REQ-INP-026).
    fn display_name(&self) -> &str;

    /// Whether the pad answers to the standard Xbox-style numbering of
    /// [`standard_gamepad`](super::standard_gamepad), and therefore gets the defaults of
    /// SPEC-08 §4.3 (REQ-INP-020). A pad without a mapping reports raw indices (REQ-INP-021).
    fn is_standard(&self) -> bool;

    /// The normalised 0-to-1 value of `control`: 0 or 1 for a button, the calibrated travel for
    /// a half-axis or a trigger. A control this pad does not have reads 0.
    fn value(&self, control: PhysicalControl) -> f32;

    /// Appends every control of this pad that is currently pressed — buttons down, half-axes
    /// and triggers at or above 0.34 — to `controls`. This is what the capture of REQ-INP-042
    /// listens to and what makes a pad "active" for the automatic assignment of REQ-INP-023.
    fn collect_active_controls(&self, controls: &mut Vec<PhysicalControl>);
}

/// The set of gamepads the host can see, refreshed on demand. Hot-plug is observable through
/// [`devices`](Self::devices) changing between two [`poll`](Self::poll) calls (REQ-INP-024).
pub trait GamepadHub {
    /// The connected, non-VR pads, in a stable order (REQ-INP-022 keeps VR devices out).
    fn devices(&self) -> &[Box<dyn GamepadDevice>];

    /// Refreshes the device list and every pad's state. Called once per frame.
    fn poll(&mut self);
}

/// The identity fragments REQ-INP-022 excludes: a VR controller is never offered and never
/// assigned automatically.
pub const VIRTUAL_REALITY_MARKERS: [&str; 4] = ["Oculus", "Vive", "Knuckles", "Spatial"];

/// Whether `identity` names a VR device (REQ-INP-022).
#[must_use]
pub fn is_virtual_reality(identity: &str) -> bool {
    let lower = identity.to_lowercase();
    VIRTUAL_REALITY_MARKERS
        .iter()
        .any(|marker| lower.contains(&marker.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REQ-INP-022: a VR controller is never offered and never assigned.
    #[test]
    fn req_inp_022_virtual_reality_devices_are_never_offered() {
        for identity in [
            "Oculus Touch Right",
            "HTC Vive Controller",
            "Valve Knuckles",
            "Spatial Controller",
        ] {
            assert!(is_virtual_reality(identity), "{identity}");
        }
        assert!(!is_virtual_reality("Xbox Wireless Controller"));
        assert!(!is_virtual_reality(""));
    }
}
