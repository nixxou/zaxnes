//! The pad as a menu pointer while the game is paused (SPEC-08 REQ-INP-105).

use super::input_action::InputAction;
use super::press_tracker::EdgeDetector;
use crate::flags::flag_set;

flag_set! {
    /// What the pad asks of the menu on one frame.
    pub struct MenuNavigationEvents(u32) {
        const PREVIOUS = 1 << 0;
        const NEXT = 1 << 1;
        const PREVIOUS_VERTICAL = 1 << 2;
        const NEXT_VERTICAL = 1 << 3;
        const SECONDARY = 1 << 4;
    }
}

/// Turns the pad's directions and Select into menu events on their rising edges.
#[derive(Debug, Clone, Copy, Default)]
pub struct MenuNavigation {
    left: EdgeDetector,
    right: EdgeDetector,
    up: EdgeDetector,
    down: EdgeDetector,
    select: EdgeDetector,
}

impl MenuNavigation {
    /// Creates a navigator with nothing held.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds the held actions and reports the menu events of this frame. Outside a paused,
    /// unsuspended game the edges are still consumed, so a key already held when the menu opens
    /// does not count as a press.
    pub fn update(&mut self, mask: InputAction, paused: bool, suspended: bool) -> MenuNavigationEvents {
        let left = self.left.rising(mask.contains(InputAction::LEFT));
        let right = self.right.rising(mask.contains(InputAction::RIGHT));
        let up = self.up.rising(mask.contains(InputAction::UP));
        let down = self.down.rising(mask.contains(InputAction::DOWN));
        let select = self.select.rising(mask.contains(InputAction::SELECT));
        if !paused || suspended {
            return MenuNavigationEvents::NONE;
        }
        let mut events = MenuNavigationEvents::NONE;
        for (happened, event) in [
            (left, MenuNavigationEvents::PREVIOUS),
            (right, MenuNavigationEvents::NEXT),
            (up, MenuNavigationEvents::PREVIOUS_VERTICAL),
            (down, MenuNavigationEvents::NEXT_VERTICAL),
            (select, MenuNavigationEvents::SECONDARY),
        ] {
            if happened {
                events |= event;
            }
        }
        events
    }
}
