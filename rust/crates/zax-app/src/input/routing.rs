//! How the two players' actions reach the core and the camera (SPEC-08 §4.1).

use zax_nes::ControllerPortType;

use super::input_action::{controller_byte, InputAction};
use super::zapper::RELEASED_BYTE;

/// The 16-bit word `NesCore::run_frame` takes: port 1 in the low byte, port 2 in the high one.
/// A Zapper port carries the Zapper byte and no controller button at all (REQ-INP-004).
#[must_use]
pub fn core_word(
    player_one: InputAction,
    player_two: InputAction,
    port_one: ControllerPortType,
    port_two: ControllerPortType,
    zapper_byte: u8,
) -> u16 {
    let low = if port_one == ControllerPortType::Zapper {
        zapper_byte
    } else {
        controller_byte(player_one)
    };
    let high = if port_two == ControllerPortType::Zapper {
        zapper_byte
    } else {
        controller_byte(player_two)
    };
    u16::from(low) | (u16::from(high) << 8)
}

/// [`core_word`] with two controllers and the Zapper released.
#[must_use]
pub fn controller_word(player_one: InputAction, player_two: InputAction) -> u16 {
    core_word(
        player_one,
        player_two,
        ControllerPortType::Controller,
        ControllerPortType::Controller,
        RELEASED_BYTE,
    )
}

/// The camera actions of the players whose port is not a Zapper (REQ-INP-004).
#[must_use]
pub fn camera_sources(
    player_one: InputAction,
    player_two: InputAction,
    port_one: ControllerPortType,
    port_two: ControllerPortType,
) -> InputAction {
    let mut mask = InputAction::NONE;
    if port_one != ControllerPortType::Zapper {
        mask |= player_one;
    }
    if port_two != ControllerPortType::Zapper {
        mask |= player_two;
    }
    mask & InputAction::CAMERA_MASK
}

/// Whether the pointer drives a Zapper: a game runs and one of the ports holds one.
#[must_use]
pub fn zapper_active(
    game_running: bool,
    port_one: ControllerPortType,
    port_two: ControllerPortType,
) -> bool {
    game_running
        && (port_one == ControllerPortType::Zapper || port_two == ControllerPortType::Zapper)
}

/// Adds the script pad's mask to the player it is attached to, never masking a physical input
/// (REQ-INP-071, REQ-INP-072).
#[must_use]
pub fn merge_script_pad(
    player_one: InputAction,
    player_two: InputAction,
    pad_mask: InputAction,
    pad_index: i32,
) -> (InputAction, InputAction) {
    if pad_index == 0 {
        (player_one | pad_mask, player_two)
    } else {
        (player_one, player_two | pad_mask)
    }
}
