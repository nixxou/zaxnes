//! The keyboard as a handheld pad (Rust-only): a handheld with a keyboard attached (or a desktop
//! run with `--handheld`) navigates the lists with the keys, through the same [`PadState`] the
//! gamepad feeds. The directions and the Select / Start keys are the configured keyboard
//! bindings of player one, so a rebinding is followed; the rest are fixed.

use zax_platform::input::{Key, KeyboardDevice};

use super::pad::{PadButton, PadState};
use crate::input::{InputAction, InputBindings};

/// The fixed keys of each button.
const FIXED: [(PadButton, &[Key]); 9] = [
    (PadButton::Up, &[Key::Up]),
    (PadButton::Down, &[Key::Down]),
    (PadButton::Left, &[Key::Left]),
    (PadButton::Right, &[Key::Right]),
    (PadButton::A, &[Key::Enter, Key::KeypadEnter, Key::Space]),
    (PadButton::B, &[Key::Escape, Key::Backspace]),
    (PadButton::X, &[Key::X]),
    (PadButton::Y, &[Key::Y]),
    (PadButton::Menu, &[Key::Tab, Key::F1]),
];

/// The buttons that also follow a configured keyboard binding.
const CONFIGURED: [(PadButton, InputAction); 6] = [
    (PadButton::Up, InputAction::UP),
    (PadButton::Down, InputAction::DOWN),
    (PadButton::Left, InputAction::LEFT),
    (PadButton::Right, InputAction::RIGHT),
    (PadButton::Select, InputAction::SELECT),
    (PadButton::Start, InputAction::START),
];

/// Which buttons the keyboard holds, so that only its changes reach the pad: a gamepad
/// releasing a button must not be undone by a key still held.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardPad {
    held: [bool; PadButton::ALL.len()],
}

impl KeyboardPad {
    /// A keyboard with nothing held.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            held: [false; PadButton::ALL.len()],
        }
    }

    /// Passes the changes of the keyboard on to `pad`.
    pub fn update(
        &mut self,
        pad: &mut PadState,
        bindings: &InputBindings,
        keyboard: &dyn KeyboardDevice,
    ) {
        for (index, button) in PadButton::ALL.into_iter().enumerate() {
            let down = Self::is_down(button, bindings, keyboard);
            if down != self.held[index] {
                self.held[index] = down;
                pad.set(button, down);
            }
        }
    }

    /// Lets every button the keyboard holds go, when the keyboard belongs to something else
    /// (a binding capture).
    pub fn release(&mut self, pad: &mut PadState) {
        for (index, button) in PadButton::ALL.into_iter().enumerate() {
            if self.held[index] {
                self.held[index] = false;
                pad.set(button, false);
            }
        }
    }

    fn is_down(button: PadButton, bindings: &InputBindings, keyboard: &dyn KeyboardDevice) -> bool {
        let fixed = FIXED
            .iter()
            .filter(|(target, _)| *target == button)
            .flat_map(|(_, keys)| keys.iter())
            .any(|key| keyboard.is_down(*key as i32));
        fixed
            || CONFIGURED
                .iter()
                .filter(|(target, _)| *target == button)
                .map(|(_, action)| bindings.keyboard(0).get_action(*action))
                .any(|control| control.is_keyboard() && keyboard.is_down(control.code))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    struct Keys(RefCell<Vec<i32>>);

    impl Keys {
        fn hold(&self, keys: &[Key]) {
            *self.0.borrow_mut() = keys.iter().map(|key| *key as i32).collect();
        }
    }

    impl KeyboardDevice for Keys {
        fn is_down(&self, key_code: i32) -> bool {
            self.0.borrow().contains(&key_code)
        }

        fn collect_held_keys(&self, keys: &mut Vec<i32>) {
            keys.extend(self.0.borrow().iter());
        }
    }

    #[test]
    fn the_keys_press_the_pad_buttons_once() {
        let (mut keyboard_pad, mut pad) = (KeyboardPad::new(), PadState::new());
        let (bindings, keys) = (InputBindings::new(), Keys(RefCell::default()));
        keys.hold(&[Key::Down, Key::Enter]);
        keyboard_pad.update(&mut pad, &bindings, &keys);
        assert!(pad.was_pressed(PadButton::Down) && pad.was_pressed(PadButton::A));
        pad.end_frame();
        keyboard_pad.update(&mut pad, &bindings, &keys);
        assert!(pad.is_held(PadButton::A) && !pad.was_pressed(PadButton::A));
    }

    #[test]
    fn select_and_start_follow_the_configured_keys() {
        let (mut keyboard_pad, mut pad) = (KeyboardPad::new(), PadState::new());
        let (bindings, keys) = (InputBindings::new(), Keys(RefCell::default()));
        keys.hold(&[Key::Space, Key::Enter]);
        keyboard_pad.update(&mut pad, &bindings, &keys);
        assert!(pad.is_held(PadButton::Select) && pad.is_held(PadButton::Start));
        keys.hold(&[]);
        keyboard_pad.update(&mut pad, &bindings, &keys);
        assert!(!pad.is_held(PadButton::Select) && !pad.is_held(PadButton::Start));
    }

    #[test]
    fn escape_is_back_and_tab_is_the_menu() {
        let (mut keyboard_pad, mut pad) = (KeyboardPad::new(), PadState::new());
        let (bindings, keys) = (InputBindings::new(), Keys(RefCell::default()));
        keys.hold(&[Key::Escape, Key::Tab]);
        keyboard_pad.update(&mut pad, &bindings, &keys);
        assert!(pad.is_held(PadButton::B) && pad.is_held(PadButton::Menu));
        keyboard_pad.release(&mut pad);
        assert!(!pad.is_held(PadButton::B));
    }
}
