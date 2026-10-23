//! How the libretro joypad maps onto the actions (SPEC-08 §6): a frontend feeds the buttons it
//! has, and the hotkeys stay with the host.

use super::input_action::InputAction;

/// A button of the libretro joypad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetroPadButton {
    /// `RETRO_DEVICE_ID_JOYPAD_B`.
    B = 0,
    /// `RETRO_DEVICE_ID_JOYPAD_Y`.
    Y = 1,
    /// `RETRO_DEVICE_ID_JOYPAD_SELECT`.
    Select = 2,
    /// `RETRO_DEVICE_ID_JOYPAD_START`.
    Start = 3,
    /// `RETRO_DEVICE_ID_JOYPAD_UP`.
    Up = 4,
    /// `RETRO_DEVICE_ID_JOYPAD_DOWN`.
    Down = 5,
    /// `RETRO_DEVICE_ID_JOYPAD_LEFT`.
    Left = 6,
    /// `RETRO_DEVICE_ID_JOYPAD_RIGHT`.
    Right = 7,
    /// `RETRO_DEVICE_ID_JOYPAD_A`.
    A = 8,
    /// `RETRO_DEVICE_ID_JOYPAD_X`.
    X = 9,
    /// `RETRO_DEVICE_ID_JOYPAD_L`.
    L = 10,
    /// `RETRO_DEVICE_ID_JOYPAD_R`.
    R = 11,
    /// `RETRO_DEVICE_ID_JOYPAD_L2`.
    L2 = 12,
    /// `RETRO_DEVICE_ID_JOYPAD_R2`.
    R2 = 13,
    /// `RETRO_DEVICE_ID_JOYPAD_L3`.
    L3 = 14,
    /// `RETRO_DEVICE_ID_JOYPAD_R3`.
    R3 = 15,
}

/// A direction of the libretro right analog stick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetroAnalogAxis {
    /// Right stick pushed left.
    RightStickLeft,
    /// Right stick pushed right.
    RightStickRight,
    /// Right stick pushed up.
    RightStickUp,
    /// Right stick pushed down.
    RightStickDown,
}

/// The joypad buttons that map to an action.
pub const BUTTONS: [(RetroPadButton, InputAction); 10] = [
    (RetroPadButton::Left, InputAction::LEFT),
    (RetroPadButton::Right, InputAction::RIGHT),
    (RetroPadButton::Up, InputAction::UP),
    (RetroPadButton::Down, InputAction::DOWN),
    (RetroPadButton::A, InputAction::A),
    (RetroPadButton::B, InputAction::B),
    (RetroPadButton::Select, InputAction::SELECT),
    (RetroPadButton::Start, InputAction::START),
    (RetroPadButton::L2, InputAction::ZOOM_IN),
    (RetroPadButton::R2, InputAction::ZOOM_OUT),
];

/// The analog directions that map to an action.
pub const ANALOG: [(RetroAnalogAxis, InputAction); 4] = [
    (RetroAnalogAxis::RightStickLeft, InputAction::ROT_LEFT),
    (RetroAnalogAxis::RightStickRight, InputAction::ROT_RIGHT),
    (RetroAnalogAxis::RightStickUp, InputAction::ROT_UP),
    (RetroAnalogAxis::RightStickDown, InputAction::ROT_DOWN),
];

/// The actions a libretro frontend never feeds: the hotkeys belong to the host.
pub const HOST_RESERVED_ACTIONS: InputAction = InputAction::HOTKEY_MASK;

/// The action of a joypad button, or the empty set when it has none.
#[must_use]
pub fn action_of_button(button: RetroPadButton) -> InputAction {
    BUTTONS
        .iter()
        .find(|(candidate, _)| *candidate == button)
        .map_or(InputAction::NONE, |(_, action)| *action)
}

/// The action of an analog direction.
#[must_use]
pub fn action_of_axis(axis: RetroAnalogAxis) -> InputAction {
    ANALOG
        .iter()
        .find(|(candidate, _)| *candidate == axis)
        .map_or(InputAction::NONE, |(_, action)| *action)
}
