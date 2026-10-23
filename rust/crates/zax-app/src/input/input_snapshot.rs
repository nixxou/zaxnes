//! What the input devices reported for one displayed frame.

use super::input_action::InputAction;

/// The actions of both players and the pointer, sampled once per displayed frame.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // a plain data record of the sample, as the C# one
pub struct InputSnapshot {
    /// Actions of player one.
    pub player_one: InputAction,
    /// Actions of player two.
    pub player_two: InputAction,
    /// Pointer position in the render target, in pixels, origin top left.
    pub pointer: [f32; 2],
    /// Size of the render target, in pixels.
    pub pointer_size: [f32; 2],
    /// Whether the left button is held (camera orbit).
    pub left_button: bool,
    /// Whether the right button is held (Zapper trigger).
    pub right_button: bool,
    /// Wheel notches since the previous frame, positive forward.
    pub wheel: f32,
    /// Whether the pointer is over the interface.
    pub pointer_over_ui: bool,
    /// Whether the pointer moved since the previous frame.
    pub pointer_moved: bool,
}

impl InputSnapshot {
    /// A snapshot with nothing pressed and the pointer at the origin of a 1 x 1 target.
    pub const EMPTY: Self = Self {
        player_one: InputAction::NONE,
        player_two: InputAction::NONE,
        pointer: [0.0, 0.0],
        pointer_size: [1.0, 1.0],
        left_button: false,
        right_button: false,
        wheel: 0.0,
        pointer_over_ui: false,
        pointer_moved: false,
    };

    /// The actions of player `index`: player one for 0, player two otherwise.
    #[must_use]
    pub const fn player(&self, index: usize) -> InputAction {
        if index == 0 {
            self.player_one
        } else {
            self.player_two
        }
    }

    /// The union of both players' actions.
    #[must_use]
    pub fn both(&self) -> InputAction {
        self.player_one | self.player_two
    }

    /// A snapshot carrying only the two players' actions.
    #[must_use]
    pub const fn of(player_one: InputAction, player_two: InputAction) -> Self {
        Self {
            player_one,
            player_two,
            ..Self::EMPTY
        }
    }
}
