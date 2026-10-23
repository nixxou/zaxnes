//! Keyboard, mouse and gamepad input (SPEC-08), device-neutral, and its SDL implementation.

pub mod calibration;
pub mod control;
pub mod devices;
pub mod keys;
pub mod labels;
pub mod sdl;

pub use control::{standard_gamepad, PhysicalControl, PhysicalControlKind};
pub use devices::{GamepadDevice, GamepadHub, KeyboardDevice, PointerDevice};
pub use keys::Key;
