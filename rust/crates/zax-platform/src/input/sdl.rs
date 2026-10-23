//! The SDL implementation of the input devices.
//!
//! The window loop feeds every SDL event to [`SdlInput::handle_event`]; the keyboard and the
//! pointer are snapshots of what those events said, and the gamepad hub reopens its devices
//! when SDL reports a plug or an unplug (REQ-INP-024).

use std::fmt;

use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::joystick::Joystick;
use sdl2::mouse::{MouseButton, MouseUtil};
use sdl2::{GameControllerSubsystem, JoystickSubsystem};

use super::calibration;
use super::control::{standard_gamepad as pad, PhysicalControl, PhysicalControlKind};
use super::devices::{
    is_virtual_reality, GamepadDevice, GamepadHub, KeyboardDevice, PointerDevice,
};
use super::keys::Key;
use crate::PlatformError;

/// Highest GLFW key code plus one.
const KEY_SLOTS: usize = 349;

/// The keyboard: physical key positions (DEC-08-5), as reported by SDL scancodes.
pub struct SdlKeyboard {
    held: [bool; KEY_SLOTS],
}

impl SdlKeyboard {
    fn new() -> Self {
        Self {
            held: [false; KEY_SLOTS],
        }
    }

    fn set(&mut self, key: Key, down: bool) {
        // Key codes are positive and below `KEY_SLOTS` by construction of the key table.
        if let Some(slot) = usize::try_from(key.code())
            .ok()
            .and_then(|i| self.held.get_mut(i))
        {
            *slot = down;
        }
    }
}

impl KeyboardDevice for SdlKeyboard {
    fn is_down(&self, key_code: i32) -> bool {
        // A code outside the table simply is not pressed; a binding file written on another
        // backend must not fail (REQ-INP-052).
        key_code > 0
            && usize::try_from(key_code)
                .ok()
                .and_then(|i| self.held.get(i))
                .copied()
                .unwrap_or(false)
    }

    fn collect_held_keys(&self, keys: &mut Vec<i32>) {
        keys.extend(
            Key::ALL
                .iter()
                .filter(|key| self.is_down(key.code()))
                .map(|key| key.code()),
        );
    }
}

/// The mouse: position, the two buttons the game reads, the wheel and the cursor.
pub struct SdlPointer {
    mouse: MouseUtil,
    position: [f32; 2],
    left: bool,
    right: bool,
    wheel: f32,
}

impl SdlPointer {
    fn new(mouse: MouseUtil) -> Self {
        mouse.show_cursor(true);
        Self {
            mouse,
            position: [0.0; 2],
            left: false,
            right: false,
            wheel: 0.0,
        }
    }
}

impl PointerDevice for SdlPointer {
    fn position(&self) -> [f32; 2] {
        self.position
    }

    fn left_button(&self) -> bool {
        self.left
    }

    fn right_button(&self) -> bool {
        self.right
    }

    fn take_wheel_notches(&mut self) -> f32 {
        std::mem::take(&mut self.wheel)
    }

    fn cursor_visible(&self) -> bool {
        self.mouse.is_cursor_showing()
    }

    fn set_cursor_visible(&mut self, visible: bool) {
        self.mouse.show_cursor(visible);
    }
}

impl Drop for SdlPointer {
    /// Shows the cursor again (REQ-INP-111).
    fn drop(&mut self) {
        self.mouse.show_cursor(true);
    }
}

/// SDL reports axes as `i16`; the pads of Silk.NET reported −1 to 1.
fn normalised(raw: i16) -> f32 {
    (f32::from(raw) / 32767.0).max(-1.0)
}

/// The other axis of the same stick, for the radial dead zone of REQ-INP-031.
fn paired_axis(axis: i32) -> Option<i32> {
    match axis {
        pad::LEFT_STICK_X => Some(pad::LEFT_STICK_Y),
        pad::LEFT_STICK_Y => Some(pad::LEFT_STICK_X),
        pad::RIGHT_STICK_X => Some(pad::RIGHT_STICK_Y),
        pad::RIGHT_STICK_Y => Some(pad::RIGHT_STICK_X),
        _ => None,
    }
}

fn button_of(code: i32) -> Option<Button> {
    Some(match code {
        pad::SOUTH => Button::A,
        pad::EAST => Button::B,
        pad::WEST => Button::X,
        pad::NORTH => Button::Y,
        pad::LEFT_BUMPER => Button::LeftShoulder,
        pad::RIGHT_BUMPER => Button::RightShoulder,
        pad::BACK => Button::Back,
        pad::START => Button::Start,
        pad::GUIDE => Button::Guide,
        pad::LEFT_STICK => Button::LeftStick,
        pad::RIGHT_STICK => Button::RightStick,
        pad::D_PAD_UP => Button::DPadUp,
        pad::D_PAD_RIGHT => Button::DPadRight,
        pad::D_PAD_DOWN => Button::DPadDown,
        pad::D_PAD_LEFT => Button::DPadLeft,
        _ => return None,
    })
}

/// A gamepad with the standard Xbox-style mapping of REQ-INP-020.
pub struct SdlGamepad {
    controller: GameController,
    name: String,
}

impl SdlGamepad {
    fn new(controller: GameController) -> Self {
        let name = controller.name();
        // The identity is the persistence key of the pad's binding set (REQ-INP-025). SDL
        // exposes the readable name here, so two identical pads share one set, which is what
        // the original does as well ("a model or an instance", SPEC-08 §2).
        let name = if name.trim().is_empty() {
            "Gamepad".to_owned()
        } else {
            name
        };
        Self { controller, name }
    }

    fn is_button_down(&self, code: i32) -> bool {
        button_of(code).is_some_and(|button| self.controller.button(button))
    }

    fn raw_axis(&self, index: i32) -> f32 {
        // The scene frame and SPEC-08 §4.3 both count the vertical axis upwards; SDL counts
        // it downwards.
        let (axis, up) = match index {
            pad::LEFT_STICK_X => (Axis::LeftX, false),
            pad::LEFT_STICK_Y => (Axis::LeftY, true),
            pad::RIGHT_STICK_X => (Axis::RightX, false),
            pad::RIGHT_STICK_Y => (Axis::RightY, true),
            _ => return 0.0,
        };
        let value = normalised(self.controller.axis(axis));
        if up {
            -value
        } else {
            value
        }
    }

    fn raw_trigger(&self, index: i32) -> f32 {
        match index {
            pad::LEFT_TRIGGER => normalised(self.controller.axis(Axis::TriggerLeft)),
            pad::RIGHT_TRIGGER => normalised(self.controller.axis(Axis::TriggerRight)),
            _ => 0.0,
        }
    }
}

impl GamepadDevice for SdlGamepad {
    fn identity(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        &self.name
    }

    fn is_standard(&self) -> bool {
        true
    }

    fn value(&self, control: PhysicalControl) -> f32 {
        match control.kind {
            PhysicalControlKind::Button => f32::from(u8::from(self.is_button_down(control.code))),
            PhysicalControlKind::Axis => {
                let other = paired_axis(control.code).map_or(0.0, |axis| self.raw_axis(axis));
                calibration::half_axis(self.raw_axis(control.code), other, control.direction)
            }
            PhysicalControlKind::Trigger => calibration::trigger(self.raw_trigger(control.code)),
            PhysicalControlKind::Key | PhysicalControlKind::None => 0.0,
        }
    }

    fn collect_active_controls(&self, controls: &mut Vec<PhysicalControl>) {
        for button in 0..pad::BUTTON_COUNT {
            if self.is_button_down(button) {
                controls.push(PhysicalControl::button(button));
            }
        }
        for axis in 0..pad::AXIS_COUNT {
            for direction in [1, -1] {
                let half = PhysicalControl::axis(axis, direction);
                if calibration::is_pressed(self.value(half)) {
                    controls.push(half);
                }
            }
        }
        for trigger in 0..pad::TRIGGER_COUNT {
            let control = PhysicalControl::trigger(trigger);
            if calibration::is_pressed(self.value(control)) {
                controls.push(control);
            }
        }
    }
}

/// A joystick with no standard mapping, whose raw buttons and axes can still be bound by
/// remapping (REQ-INP-021).
pub struct SdlJoystick {
    joystick: Joystick,
    name: String,
}

impl SdlJoystick {
    fn new(joystick: Joystick) -> Self {
        let name = joystick.name();
        let name = if name.trim().is_empty() {
            "Joystick".to_owned()
        } else {
            name
        };
        Self { joystick, name }
    }

    fn is_button_down(&self, code: i32) -> bool {
        // A device that vanished mid-frame reads as released; the hub drops it at the next
        // plug or unplug event.
        u32::try_from(code).is_ok_and(|index| self.joystick.button(index).unwrap_or(false))
    }

    fn raw_axis(&self, index: i32) -> f32 {
        // Same as above: a vanished device reads as centred.
        u32::try_from(index)
            .ok()
            .and_then(|index| self.joystick.axis(index).ok())
            .map_or(0.0, normalised)
    }
}

impl GamepadDevice for SdlJoystick {
    fn identity(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        &self.name
    }

    fn is_standard(&self) -> bool {
        false
    }

    fn value(&self, control: PhysicalControl) -> f32 {
        match control.kind {
            PhysicalControlKind::Button => f32::from(u8::from(self.is_button_down(control.code))),
            // A generic axis has no known pair, so the dead zone is applied to that axis alone.
            PhysicalControlKind::Axis => {
                calibration::half_axis(self.raw_axis(control.code), 0.0, control.direction)
            }
            _ => 0.0,
        }
    }

    fn collect_active_controls(&self, controls: &mut Vec<PhysicalControl>) {
        for button in 0..self.joystick.num_buttons() {
            if let Ok(code) = i32::try_from(button) {
                if self.is_button_down(code) {
                    controls.push(PhysicalControl::button(code));
                }
            }
        }
        for axis in 0..self.joystick.num_axes() {
            let Ok(index) = i32::try_from(axis) else {
                continue;
            };
            for direction in [1, -1] {
                let half = PhysicalControl::axis(index, direction);
                if calibration::is_pressed(self.value(half)) {
                    controls.push(half);
                }
            }
        }
    }
}

/// The gamepad hub over SDL: the connected non-VR pads, refreshed when SDL reports a plug or
/// an unplug so that it is visible at the next frame (REQ-INP-024, REQ-INP-041).
pub struct SdlGamepadHub {
    controllers: GameControllerSubsystem,
    joysticks: JoystickSubsystem,
    devices: Vec<Box<dyn GamepadDevice>>,
    dirty: bool,
}

impl SdlGamepadHub {
    fn new(controllers: GameControllerSubsystem, joysticks: JoystickSubsystem) -> Self {
        let mut hub = Self {
            controllers,
            joysticks,
            devices: Vec::new(),
            dirty: true,
        };
        hub.poll();
        hub
    }

    fn reopen(&mut self) -> Result<(), String> {
        self.devices.clear();
        for index in 0..self.joysticks.num_joysticks()? {
            if self.controllers.is_game_controller(index) {
                let name = self
                    .controllers
                    .name_for_index(index)
                    .map_err(|e| e.to_string())?;
                if is_virtual_reality(&name) {
                    continue;
                }
                let controller = self.controllers.open(index).map_err(|e| e.to_string())?;
                self.devices.push(Box::new(SdlGamepad::new(controller)));
            } else {
                let name = self
                    .joysticks
                    .name_for_index(index)
                    .map_err(|e| e.to_string())?;
                if is_virtual_reality(&name) {
                    continue;
                }
                let joystick = self.joysticks.open(index).map_err(|e| e.to_string())?;
                self.devices.push(Box::new(SdlJoystick::new(joystick)));
            }
        }
        Ok(())
    }
}

impl GamepadHub for SdlGamepadHub {
    fn devices(&self) -> &[Box<dyn GamepadDevice>] {
        &self.devices
    }

    fn poll(&mut self) {
        if !self.dirty {
            return;
        }
        // A device that cannot be opened right now (it was unplugged between the event and
        // this call) is transient: the list stays as far as it got and the next frame retries.
        self.dirty = self.reopen().is_err();
    }
}

/// Everything the input layer reads from SDL: keyboard, pointer and gamepads.
pub struct SdlInput {
    /// The keyboard, by physical position.
    pub keyboard: SdlKeyboard,
    /// The mouse.
    pub pointer: SdlPointer,
    /// The gamepads.
    pub gamepads: SdlGamepadHub,
}

impl SdlInput {
    /// Opens the input subsystems of `sdl`.
    ///
    /// # Errors
    /// [`PlatformError`] if SDL cannot initialise the controller or joystick subsystem.
    pub fn new(sdl: &sdl2::Sdl) -> Result<Self, PlatformError> {
        Ok(Self {
            keyboard: SdlKeyboard::new(),
            pointer: SdlPointer::new(sdl.mouse()),
            gamepads: SdlGamepadHub::new(sdl.game_controller()?, sdl.joystick()?),
        })
    }

    /// Updates the snapshots with one SDL event.
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::KeyDown {
                scancode: Some(scancode),
                ..
            } => {
                if let Some(key) = Key::from_scancode(*scancode) {
                    self.keyboard.set(key, true);
                }
            }
            Event::KeyUp {
                scancode: Some(scancode),
                ..
            } => {
                if let Some(key) = Key::from_scancode(*scancode) {
                    self.keyboard.set(key, false);
                }
            }
            #[allow(clippy::cast_precision_loss)] // pixel coordinates are far below 2^24
            Event::MouseMotion { x, y, .. } => self.pointer.position = [*x as f32, *y as f32],
            Event::MouseButtonDown { mouse_btn, .. } => self.set_button(*mouse_btn, true),
            Event::MouseButtonUp { mouse_btn, .. } => self.set_button(*mouse_btn, false),
            Event::MouseWheel { precise_y, .. } => self.pointer.wheel += precise_y,
            Event::ControllerDeviceAdded { .. }
            | Event::ControllerDeviceRemoved { .. }
            | Event::JoyDeviceAdded { .. }
            | Event::JoyDeviceRemoved { .. } => self.gamepads.dirty = true,
            _ => {}
        }
    }

    fn set_button(&mut self, button: MouseButton, down: bool) {
        match button {
            MouseButton::Left => self.pointer.left = down,
            MouseButton::Right => self.pointer.right = down,
            _ => {}
        }
    }
}

impl fmt::Debug for SdlInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SdlInput")
            .field("gamepads", &self.gamepads.devices.len())
            .finish_non_exhaustive()
    }
}
