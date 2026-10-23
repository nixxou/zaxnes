//! Readable labels for the physical commands a binding points at, which the key configuration
//! screen shows next to each action (DEC-08-5, SPEC-10; the screen itself is lot L9).
//!
//! Bindings are stored as physical positions, so the label is the one of the **US layout**
//! position unless the backend can tell us the character the active layout produces. That is the
//! "comprehensible label" DEC-08-5 asks SPEC-10 for, and it is why the default table of §4.2
//! documents `Z` as the AZERTY `W`.

use super::control::{standard_gamepad as pad, PhysicalControl, PhysicalControlKind};
use super::keys::Key;

/// The label of `control`, or "—" when it is unbound.
#[must_use]
pub fn of(control: PhysicalControl) -> String {
    match control.kind {
        PhysicalControlKind::Key => key_label(control.code),
        PhysicalControlKind::Button => button_label(control.code),
        PhysicalControlKind::Axis => axis_label(control.code, control.direction),
        PhysicalControlKind::Trigger => if control.code == pad::LEFT_TRIGGER {
            "Left trigger"
        } else {
            "Right trigger"
        }
        .to_owned(),
        PhysicalControlKind::None => "—".to_owned(),
    }
}

fn key_label(code: i32) -> String {
    let Some(key) = Key::from_code(code) else {
        return format!("Key {code}");
    };
    match key {
        Key::Left => "Left arrow",
        Key::Right => "Right arrow",
        Key::Up => "Up arrow",
        Key::Down => "Down arrow",
        Key::Enter => "Enter",
        Key::KeypadEnter => "Keypad Enter",
        Key::Space => "Space",
        Key::Escape => "Esc",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Minus => "-",
        Key::Equal => "=",
        Key::Semicolon => ";",
        Key::Apostrophe => "'",
        Key::Slash => "/",
        Key::BackSlash => "\\",
        Key::GraveAccent => "`",
        Key::LeftBracket => "[",
        Key::RightBracket => "]",
        Key::ShiftLeft => "Left Shift",
        Key::ShiftRight => "Right Shift",
        Key::ControlLeft => "Left Ctrl",
        Key::ControlRight => "Right Ctrl",
        Key::AltLeft => "Left Alt",
        Key::AltRight => "Right Alt",
        Key::SuperLeft => "Left Win",
        Key::SuperRight => "Right Win",
        Key::Backspace => "Backspace",
        Key::Tab => "Tab",
        Key::CapsLock => "Caps Lock",
        Key::PageUp => "Page Up",
        Key::PageDown => "Page Down",
        Key::Home => "Home",
        Key::End => "End",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        other => other.name(),
    }
    .to_owned()
}

fn button_label(code: i32) -> String {
    match code {
        pad::SOUTH => "South (A)",
        pad::EAST => "East (B)",
        pad::WEST => "West (X)",
        pad::NORTH => "North (Y)",
        pad::LEFT_BUMPER => "Left bumper",
        pad::RIGHT_BUMPER => "Right bumper",
        pad::BACK => "Back",
        pad::START => "Start",
        pad::GUIDE => "Guide",
        pad::LEFT_STICK => "Left stick click",
        pad::RIGHT_STICK => "Right stick click",
        pad::D_PAD_UP => "D-pad up",
        pad::D_PAD_RIGHT => "D-pad right",
        pad::D_PAD_DOWN => "D-pad down",
        pad::D_PAD_LEFT => "D-pad left",
        _ => return format!("Button {code}"),
    }
    .to_owned()
}

fn axis_label(axis: i32, direction: i32) -> String {
    let negative = direction < 0;
    match axis {
        pad::LEFT_STICK_X => {
            if negative {
                "Left stick left"
            } else {
                "Left stick right"
            }
        }
        pad::LEFT_STICK_Y => {
            if negative {
                "Left stick down"
            } else {
                "Left stick up"
            }
        }
        pad::RIGHT_STICK_X => {
            if negative {
                "Right stick left"
            } else {
                "Right stick right"
            }
        }
        pad::RIGHT_STICK_Y => {
            if negative {
                "Right stick down"
            } else {
                "Right stick up"
            }
        }
        _ => return format!("Axis {axis}{}", if negative { '-' } else { '+' }),
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_use_the_us_layout_position_names() {
        assert_eq!(of(PhysicalControl::key(Key::Z.code())), "Z");
        assert_eq!(of(PhysicalControl::key(Key::Left.code())), "Left arrow");
        assert_eq!(of(PhysicalControl::key(Key::Escape.code())), "Esc");
        assert_eq!(of(PhysicalControl::key(Key::BackSlash.code())), "\\");
        assert_eq!(of(PhysicalControl::key(Key::F5.code())), "F5");
        assert_eq!(of(PhysicalControl::key(Key::Number3.code())), "Number3");
        assert_eq!(of(PhysicalControl::key(9999)), "Key 9999");
    }

    #[test]
    fn gamepad_controls_have_readable_labels() {
        assert_eq!(of(PhysicalControl::button(pad::SOUTH)), "South (A)");
        assert_eq!(of(PhysicalControl::button(99)), "Button 99");
        assert_eq!(
            of(PhysicalControl::axis_positive(pad::LEFT_STICK_Y)),
            "Left stick up"
        );
        assert_eq!(
            of(PhysicalControl::axis_negative(pad::RIGHT_STICK_X)),
            "Right stick left"
        );
        assert_eq!(of(PhysicalControl::axis_negative(7)), "Axis 7-");
        assert_eq!(
            of(PhysicalControl::trigger(pad::LEFT_TRIGGER)),
            "Left trigger"
        );
        assert_eq!(
            of(PhysicalControl::trigger(pad::RIGHT_TRIGGER)),
            "Right trigger"
        );
    }

    #[test]
    fn an_unbound_control_reads_as_a_dash() {
        assert_eq!(of(PhysicalControl::NONE), "—");
    }

    #[test]
    fn key_codes_round_trip_and_scancodes_map_to_glfw_codes() {
        use sdl2::keyboard::Scancode;
        assert_eq!(Key::from_code(65), Some(Key::A));
        assert_eq!(Key::from_code(1), None);
        assert_eq!(Key::from_scancode(Scancode::Return), Some(Key::Enter));
        assert_eq!(Key::from_scancode(Scancode::W), Some(Key::W));
        assert!(Key::ALL
            .windows(2)
            .all(|pair| pair[0].code() < pair[1].code()));
    }
}
