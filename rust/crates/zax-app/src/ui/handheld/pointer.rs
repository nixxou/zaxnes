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
    /// The left button (or a finger): what grabs and drags the scrollbar.
    pub button: Hold,
}

/// The left button or a finger, for the scrollbar of a list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Hold {
    /// Up.
    #[default]
    Up,
    /// Gone down this frame: it grabs the scrollbar under it.
    Pressed,
    /// Down since an earlier frame: the scrollbar follows it.
    Down,
}

impl Hold {
    /// The state of a button down `now` and down or not the frame `before`.
    #[must_use]
    pub const fn of(now: bool, before: bool) -> Self {
        match (now, before) {
            (false, _) => Self::Up,
            (true, false) => Self::Pressed,
            (true, true) => Self::Down,
        }
    }
}

/// What a click on a zone does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneAction {
    /// A row of a list: the `rank`-th row a selection can rest on (group titles are not
    /// counted), focused or not; `value` is the span of its "< value >", whose halves step it
    /// down and up.
    Row {
        rank: usize,
        selected: bool,
        value: Option<(f32, f32)>,
    },
    /// The scrollbar of a list longer than the screen (owner request): `rows` of `total` shown
    /// from `first`. A click or a drag on it scrolls the list without selecting a row.
    Scrollbar {
        first: usize,
        rows: usize,
        total: usize,
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
    /// Whether the point is in the zone.
    #[must_use]
    pub fn contains(&self, [x, y]: [f32; 2]) -> bool {
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

/// The first row a scrollbar `zone` shows with the pointer at height `y`: the thumb centred on
/// it, as far as the list goes.
#[must_use]
pub fn scrolled_first(zone: &Zone, y: f32) -> Option<usize> {
    let ZoneAction::Scrollbar { rows, total, .. } = zone.action else {
        return None;
    };
    let hidden = total.saturating_sub(rows);
    if hidden == 0 {
        return Some(0);
    }
    let room = zone.max[1] - zone.min[1];
    #[allow(clippy::cast_precision_loss)] // A list of a few thousand rows at most.
    let thumb = (room * rows as f32 / total as f32).max(room * 0.05);
    let travel = (room - thumb).max(1.0);
    let fraction = ((y - zone.min[1] - thumb * 0.5) / travel).clamp(0.0, 1.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let first = (fraction * hidden as f32).round() as usize;
    Some(first.min(hidden))
}

/// The presses `pointer` makes over `zones` (the last one drawn wins where they overlap): the
/// wheel moves the selection, a click on a row selects it and a click on the selected row
/// accepts it (A), or steps its value; the right button goes back. `focus` is the rank of the
/// selected row, which a click moves from even when the list is scrolled away from it.
#[must_use]
pub fn presses(zones: &[Zone], pointer: &PointerInput, focus: Option<usize>) -> Vec<PadButton> {
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
        // One press per row the selection has to cross: the ranks count the rows a press moves
        // through, the group titles of RO-3 (which up and down skip) left out.
        ZoneAction::Row { rank, .. } => {
            let focus = focus.or_else(|| {
                zones.iter().find_map(|zone| match zone.action {
                    ZoneAction::Row {
                        rank,
                        selected: true,
                        ..
                    } => Some(rank),
                    _ => None,
                })
            });
            if let Some(focus) = focus {
                let (step, count) = if rank > focus {
                    (PadButton::Down, rank - focus)
                } else {
                    (PadButton::Up, focus - rank)
                };
                out.extend(std::iter::repeat_n(step, count));
            }
        }
        // The scrollbar scrolls (`HandheldUi::update_pointer`), it presses nothing.
        ZoneAction::Scrollbar { .. } => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row drawn in `slot`, the `rank`-th row a selection rests on.
    fn ranked(slot: usize, rank: usize, selected: bool) -> Zone {
        let top = 100.0 + 40.0 * f32::from(u16::try_from(slot).unwrap());
        Zone {
            min: [0.0, top],
            max: [600.0, top + 40.0],
            action: ZoneAction::Row {
                rank,
                selected,
                value: Some((400.0, 600.0)),
            },
        }
    }

    fn row(slot: usize, selected: bool) -> Zone {
        ranked(slot, slot, selected)
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
            presses(&zones, &click(10.0, 225.0), Some(1))
        );
        assert_eq!(
            vec![PadButton::Up],
            presses(&zones, &click(10.0, 110.0), Some(1))
        );
        assert_eq!(
            vec![PadButton::A],
            presses(&zones, &click(10.0, 150.0), Some(1))
        );
    }

    /// RO-3: a group title holds a slot and records no zone, so the selection crossing it moves
    /// by one row and not by two. Here slot 2 is a title: a click on slot 3 from slot 1 is one
    /// Down, and a click on slot 0 from slot 3 is two Ups.
    #[test]
    fn ro_3_a_click_past_a_group_title_moves_by_one_row() {
        let zones = [row(0, false), row(1, true), ranked(3, 2, false)];
        assert_eq!(
            vec![PadButton::Down],
            presses(&zones, &click(10.0, 225.0), Some(1))
        );
        let zones = [row(0, false), row(1, false), ranked(3, 2, true)];
        assert_eq!(
            vec![PadButton::Up, PadButton::Up],
            presses(&zones, &click(10.0, 110.0), Some(2))
        );
        // A click on the title itself reaches nothing: it records no zone of its own, and the row
        // zones do not cover it.
        assert_eq!(
            Vec::<PadButton>::new(),
            presses(&zones, &click(10.0, 190.0), Some(2))
        );
    }

    /// Owner request: a list scrolled away from its selection still moves it to the row
    /// clicked, and the scrollbar presses nothing.
    #[test]
    fn a_click_in_a_scrolled_list_moves_from_the_selection_out_of_sight() {
        // Rows of ranks 10 to 12 shown, the selection on rank 2.
        let zones = [
            ranked(0, 10, false),
            ranked(1, 11, false),
            ranked(2, 12, false),
        ];
        assert_eq!(
            vec![PadButton::Down; 9],
            presses(&zones, &click(10.0, 145.0), Some(2))
        );
        let bar = Zone {
            min: [0.0, 100.0],
            max: [8.0, 500.0],
            action: ZoneAction::Scrollbar {
                first: 0,
                rows: 10,
                total: 50,
            },
        };
        assert_eq!(
            Vec::<PadButton>::new(),
            presses(&[bar], &click(4.0, 300.0), Some(2))
        );
        // The thumb at the top shows the first rows, at the bottom the last.
        assert_eq!(Some(0), scrolled_first(&bar, 100.0));
        assert_eq!(Some(40), scrolled_first(&bar, 500.0));
        assert_eq!(Some(20), scrolled_first(&bar, 300.0));
    }

    #[test]
    fn a_click_on_the_value_of_the_selected_row_steps_it() {
        let zones = [row(0, true)];
        assert_eq!(
            vec![PadButton::Left],
            presses(&zones, &click(450.0, 110.0), Some(0))
        );
        assert_eq!(
            vec![PadButton::Right],
            presses(&zones, &click(550.0, 110.0), Some(0))
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
        assert_eq!(
            vec![PadButton::X],
            presses(&zones, &click(50.0, 420.0), None)
        );
        assert_eq!(
            vec![PadButton::L1],
            presses(&zones, &click(120.0, 420.0), None)
        );
        assert_eq!(
            vec![PadButton::R1],
            presses(&zones, &click(180.0, 420.0), None)
        );
        assert_eq!(
            Vec::<PadButton>::new(),
            presses(&zones, &click(300.0, 420.0), None)
        );
        assert!(hint_action("?").is_none());
        let wheel = PointerInput {
            wheel: -2.0,
            right_clicked: true,
            ..PointerInput::default()
        };
        assert_eq!(
            vec![PadButton::Down, PadButton::Down, PadButton::B],
            presses(&zones, &wheel, None)
        );
    }
}
