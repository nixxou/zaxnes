//! The mouse in the console mode of the desktop (Rust-only, owner request): the drawing records
//! where its rows and hints lie, and a click, the wheel or the right button become the presses
//! of the pad the screens already answer. A handheld has no pointer: nothing is recorded there
//! that a click could reach.

use super::pad::PadButton;

/// What the pointer did this frame, read from the interface's input.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PointerInput {
    /// Where it is, in display pixels.
    pub position: [f32; 2],
    /// Whether the left button was clicked.
    pub clicked: bool,
    /// Whether the right button was clicked: back (B).
    pub right_clicked: bool,
    /// The wheel's notches, positive up.
    pub wheel: f32,
}

/// What a click on a zone does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneAction {
    /// A row of a list: shown in slot `slot` of the screen, focused or not; `value` is the span
    /// of its "< value >", whose halves step it down and up.
    Row {
        slot: usize,
        selected: bool,
        value: Option<(f32, f32)>,
    },
    /// A hint of the bottom bar: its button.
    Button(PadButton),
    /// A hint of two buttons ("L/R", "</>"): the left half the first, the right half the other.
    Pair(PadButton, PadButton),
}

/// A place of the screen a click reaches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub action: ZoneAction,
}

impl Zone {
    fn contains(&self, [x, y]: [f32; 2]) -> bool {
        (self.min[0]..self.max[0]).contains(&x) && (self.min[1]..self.max[1]).contains(&y)
    }
}

/// The button of a hint, as the hint bar prints it.
#[must_use]
pub fn hint_action(button: &str) -> Option<ZoneAction> {
    Some(match button {
        "A" => ZoneAction::Button(PadButton::A),
        "B" => ZoneAction::Button(PadButton::B),
        "X" => ZoneAction::Button(PadButton::X),
        "Y" => ZoneAction::Button(PadButton::Y),
        "L1" => ZoneAction::Button(PadButton::L1),
        "R1" => ZoneAction::Button(PadButton::R1),
        "L2" => ZoneAction::Button(PadButton::L2),
        "R2" => ZoneAction::Button(PadButton::R2),
        "Select" => ZoneAction::Button(PadButton::Select),
        "Start" => ZoneAction::Button(PadButton::Start),
        "Menu" => ZoneAction::Button(PadButton::Menu),
        "L/R" => ZoneAction::Pair(PadButton::L1, PadButton::R1),
        "</>" => ZoneAction::Pair(PadButton::Left, PadButton::Right),
        _ => return None,
    })
}

/// The presses `pointer` makes over `zones` (the last one drawn wins where they overlap): the
/// wheel moves the selection, a click on a row selects it and a click on the selected row
/// accepts it (A), or steps its value; the right button goes back.
#[must_use]
pub fn presses(zones: &[Zone], pointer: &PointerInput) -> Vec<PadButton> {
    let mut out = Vec::new();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a few notches
    let notches = pointer.wheel.abs().round() as usize;
    let wheel = if pointer.wheel > 0.0 {
        PadButton::Up
    } else {
        PadButton::Down
    };
    out.extend(std::iter::repeat_n(wheel, notches));
    if pointer.right_clicked {
        out.push(PadButton::B);
    }
    if !pointer.clicked {
        return out;
    }
    let Some(zone) = zones
        .iter()
        .rev()
        .find(|zone| zone.contains(pointer.position))
    else {
        return out;
    };
    let x = pointer.position[0];
    match zone.action {
        ZoneAction::Button(button) => out.push(button),
        ZoneAction::Pair(first, second) => {
            let middle = f32::midpoint(zone.min[0], zone.max[0]);
            out.push(if x < middle { first } else { second });
        }
        ZoneAction::Row {
            selected: true,
            value: Some((from, to)),
            ..
        } if x >= from => {
            out.push(if x < f32::midpoint(from, to) {
                PadButton::Left
            } else {
                PadButton::Right
            });
        }
        ZoneAction::Row { selected: true, .. } => out.push(PadButton::A),
        ZoneAction::Row { slot, .. } => {
            // The rows of a screen are one after the other: the selection moves as many steps.
            let focused = zones.iter().find_map(|zone| match zone.action {
                ZoneAction::Row {
                    slot,
                    selected: true,
                    ..
                } => Some(slot),
                _ => None,
            });
            if let Some(focused) = focused {
                let step = if slot > focused {
                    PadButton::Down
                } else {
                    PadButton::Up
                };
                out.extend(std::iter::repeat_n(step, slot.abs_diff(focused)));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(slot: usize, selected: bool) -> Zone {
        let top = 100.0 + 40.0 * f32::from(u16::try_from(slot).unwrap());
        Zone {
            min: [0.0, top],
            max: [600.0, top + 40.0],
            action: ZoneAction::Row {
                slot,
                selected,
                value: Some((400.0, 600.0)),
            },
        }
    }

    fn click(x: f32, y: f32) -> PointerInput {
        PointerInput {
            position: [x, y],
            clicked: true,
            ..PointerInput::default()
        }
    }

    #[test]
    fn a_click_selects_a_row_then_accepts_it() {
        let zones = [row(0, false), row(1, true), row(2, false), row(3, false)];
        assert_eq!(
            vec![PadButton::Down, PadButton::Down],
            presses(&zones, &click(10.0, 225.0))
        );
        assert_eq!(vec![PadButton::Up], presses(&zones, &click(10.0, 110.0)));
        assert_eq!(vec![PadButton::A], presses(&zones, &click(10.0, 150.0)));
    }

    #[test]
    fn a_click_on_the_value_of_the_selected_row_steps_it() {
        let zones = [row(0, true)];
        assert_eq!(vec![PadButton::Left], presses(&zones, &click(450.0, 110.0)));
        assert_eq!(
            vec![PadButton::Right],
            presses(&zones, &click(550.0, 110.0))
        );
    }

    #[test]
    fn hints_press_their_button_and_the_wheel_moves() {
        let zones = [
            Zone {
                min: [0.0, 400.0],
                max: [100.0, 440.0],
                action: hint_action("X").unwrap(),
            },
            Zone {
                min: [100.0, 400.0],
                max: [200.0, 440.0],
                action: hint_action("L/R").unwrap(),
            },
        ];
        assert_eq!(vec![PadButton::X], presses(&zones, &click(50.0, 420.0)));
        assert_eq!(vec![PadButton::L1], presses(&zones, &click(120.0, 420.0)));
        assert_eq!(vec![PadButton::R1], presses(&zones, &click(180.0, 420.0)));
        assert_eq!(
            Vec::<PadButton>::new(),
            presses(&zones, &click(300.0, 420.0))
        );
        assert!(hint_action("?").is_none());
        let wheel = PointerInput {
            wheel: -2.0,
            right_clicked: true,
            ..PointerInput::default()
        };
        assert_eq!(
            vec![PadButton::Down, PadButton::Down, PadButton::B],
            presses(&zones, &wheel)
        );
    }
}
