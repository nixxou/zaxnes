//! One physical command: a key, a gamepad button, half of a stick axis or a trigger
//! (SPEC-08 §2), and the numbering of the standard gamepad layout.

use std::fmt;

/// What kind of physical command a binding points at (SPEC-08 §2, "binding").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PhysicalControlKind {
    /// No command: the action is unbound.
    #[default]
    None,
    /// A keyboard key, identified by its physical position (DEC-08-5).
    Key,
    /// A gamepad button, including the four hat directions.
    Button,
    /// One half of a gamepad stick axis.
    Axis,
    /// An analog trigger.
    Trigger,
}

/// One physical command (SPEC-08 §2).
///
/// The descriptor carries no windowing or gamepad library type, which is what lets the binding
/// model of SPEC-08 §3.5 be serialised (SPEC-09 REQ-PERS-065) and lets a test drive a simulated
/// device. The numeric codes are those of [`standard_gamepad`] for a recognised pad and the raw
/// indices of the device for anything else (REQ-INP-021). Key codes are the GLFW key codes
/// ([`super::Key`]), the layout-independent numbering DEC-08-5 asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PhysicalControl {
    /// Which family the command belongs to.
    pub kind: PhysicalControlKind,
    /// Key code, button index, axis index or trigger index.
    pub code: i32,
    /// Which half of an axis: +1 for the positive half, −1 for the negative half. Zero for the
    /// other kinds.
    pub direction: i32,
}

impl PhysicalControl {
    /// The unbound command (REQ-INP-045: an action left empty).
    pub const NONE: PhysicalControl = PhysicalControl {
        kind: PhysicalControlKind::None,
        code: 0,
        direction: 0,
    };

    /// Whether the descriptor points at a real command.
    #[must_use]
    pub fn is_bound(self) -> bool {
        self.kind != PhysicalControlKind::None
    }

    /// Whether the command belongs to a keyboard.
    #[must_use]
    pub fn is_keyboard(self) -> bool {
        self.kind == PhysicalControlKind::Key
    }

    /// Whether the command belongs to a gamepad.
    #[must_use]
    pub fn is_gamepad(self) -> bool {
        matches!(
            self.kind,
            PhysicalControlKind::Button | PhysicalControlKind::Axis | PhysicalControlKind::Trigger
        )
    }

    /// The key at physical position `code`.
    #[must_use]
    pub fn key(code: i32) -> Self {
        Self {
            kind: PhysicalControlKind::Key,
            code,
            direction: 0,
        }
    }

    /// Gamepad button `code`.
    #[must_use]
    pub fn button(code: i32) -> Self {
        Self {
            kind: PhysicalControlKind::Button,
            code,
            direction: 0,
        }
    }

    /// The positive half of axis `index`.
    #[must_use]
    pub fn axis_positive(index: i32) -> Self {
        Self::axis(index, 1)
    }

    /// The negative half of axis `index`.
    #[must_use]
    pub fn axis_negative(index: i32) -> Self {
        Self::axis(index, -1)
    }

    /// Half `direction` of axis `index`; a zero direction means the positive half.
    #[must_use]
    pub fn axis(index: i32, direction: i32) -> Self {
        Self {
            kind: PhysicalControlKind::Axis,
            code: index,
            direction: if direction >= 0 { 1 } else { -1 },
        }
    }

    /// Trigger `index` (0 left, 1 right).
    #[must_use]
    pub fn trigger(index: i32) -> Self {
        Self {
            kind: PhysicalControlKind::Trigger,
            code: index,
            direction: 0,
        }
    }

    /// Reads back what `Display` wrote. An unknown or malformed text yields [`Self::NONE`],
    /// which is how a corrupted entry degrades to "unbound" instead of failing the whole file
    /// (REQ-INP-052).
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        let Some((kind, value)) = text.split_once(':') else {
            return Self::NONE;
        };
        if kind.is_empty() || value.is_empty() {
            return Self::NONE;
        }

        let (value, direction) = match value.as_bytes().last() {
            Some(b'-') => (&value[..value.len() - 1], -1),
            Some(b'+') => (&value[..value.len() - 1], 1),
            _ => (value, 0),
        };

        let Ok(code) = value.trim().parse::<i32>() else {
            return Self::NONE;
        };
        if code < 0 {
            return Self::NONE;
        }

        if kind.eq_ignore_ascii_case("key") {
            Self::key(code)
        } else if kind.eq_ignore_ascii_case("button") {
            Self::button(code)
        } else if kind.eq_ignore_ascii_case("axis") {
            // A half-axis without a sign is meaningless; treat it as unbound rather than
            // guessing a direction.
            if direction == 0 {
                Self::NONE
            } else {
                Self::axis(code, direction)
            }
        } else if kind.eq_ignore_ascii_case("trigger") {
            Self::trigger(code)
        } else {
            Self::NONE
        }
    }
}

impl fmt::Display for PhysicalControl {
    /// The persisted form: `key:256`, `button:0`, `axis:2+`, `trigger:1`, or the empty string
    /// when unbound (SPEC-09 REQ-PERS-065).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            PhysicalControlKind::Key => write!(f, "key:{}", self.code),
            PhysicalControlKind::Button => write!(f, "button:{}", self.code),
            PhysicalControlKind::Axis => {
                write!(
                    f,
                    "axis:{}{}",
                    self.code,
                    if self.direction < 0 { '-' } else { '+' }
                )
            }
            PhysicalControlKind::Trigger => write!(f, "trigger:{}", self.code),
            PhysicalControlKind::None => Ok(()),
        }
    }
}

/// The control numbering of the standard Xbox-style layout SPEC-08 §4.3 is written against
/// (REQ-INP-020).
///
/// A device the platform recognises reports its controls under these numbers, whatever the
/// underlying driver calls them, so the default gamepad table is one fixed set of descriptors.
/// A pad with no standard mapping reports raw indices instead and therefore has no useful
/// defaults (REQ-INP-021).
pub mod standard_gamepad {
    /// South face button (A on Xbox, Cross on `PlayStation`).
    pub const SOUTH: i32 = 0;
    /// East face button (B on Xbox, Circle).
    pub const EAST: i32 = 1;
    /// West face button (X on Xbox, Square).
    pub const WEST: i32 = 2;
    /// North face button (Y on Xbox, Triangle).
    pub const NORTH: i32 = 3;
    /// Left bumper (LB / L1).
    pub const LEFT_BUMPER: i32 = 4;
    /// Right bumper (RB / R1).
    pub const RIGHT_BUMPER: i32 = 5;
    /// Back / View / Select / Share.
    pub const BACK: i32 = 6;
    /// Start / Menu / Options.
    pub const START: i32 = 7;
    /// Guide / Home / PS button.
    pub const GUIDE: i32 = 8;
    /// Left stick click (L3).
    pub const LEFT_STICK: i32 = 9;
    /// Right stick click (R3).
    pub const RIGHT_STICK: i32 = 10;
    /// Hat up.
    pub const D_PAD_UP: i32 = 11;
    /// Hat right.
    pub const D_PAD_RIGHT: i32 = 12;
    /// Hat down.
    pub const D_PAD_DOWN: i32 = 13;
    /// Hat left.
    pub const D_PAD_LEFT: i32 = 14;
    /// How many buttons the standard layout defines.
    pub const BUTTON_COUNT: i32 = 15;

    /// Left stick, horizontal axis; positive to the right.
    pub const LEFT_STICK_X: i32 = 0;
    /// Left stick, vertical axis; positive upwards.
    pub const LEFT_STICK_Y: i32 = 1;
    /// Right stick, horizontal axis; positive to the right.
    pub const RIGHT_STICK_X: i32 = 2;
    /// Right stick, vertical axis; positive upwards.
    pub const RIGHT_STICK_Y: i32 = 3;
    /// How many stick axes the standard layout defines.
    pub const AXIS_COUNT: i32 = 4;

    /// Left trigger (LT / L2).
    pub const LEFT_TRIGGER: i32 = 0;
    /// Right trigger (RT / R2).
    pub const RIGHT_TRIGGER: i32 = 1;
    /// How many triggers the standard layout defines.
    pub const TRIGGER_COUNT: i32 = 2;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC-09 REQ-PERS-065: the persisted form of a physical command is stable and
    /// round-trips.
    #[test]
    fn req_pers_065_a_control_round_trips_through_its_text() {
        for text in ["key:88", "button:3", "axis:2+", "axis:3-", "trigger:1"] {
            let control = PhysicalControl::parse(text);
            assert!(control.is_bound(), "{text}");
            assert_eq!(control.to_string(), text);
        }
    }

    /// A malformed entry degrades to "unbound".
    #[test]
    fn req_inp_052_a_malformed_control_is_unbound() {
        for text in [
            "", "axis:2", "mouse:0", "key:", "key:abc", "nonsense", "key:-3", ":4",
        ] {
            assert!(!PhysicalControl::parse(text).is_bound(), "{text:?}");
        }
    }

    #[test]
    fn descriptors_classify_themselves() {
        assert!(PhysicalControl::key(1).is_keyboard());
        assert!(PhysicalControl::axis_negative(0).is_gamepad());
        assert_eq!(PhysicalControl::axis(1, 0).direction, 1);
        assert_eq!(PhysicalControl::NONE, PhysicalControl::default());
        assert_eq!(PhysicalControl::NONE.to_string(), "");
    }
}
