//! The Select + Start combination: held for [`HOLD_SECONDS`] it quits the application (the
//! `PortMaster` convention); pressed and let go before that it is a tap, which opens the menu when
//! the setting "Menu = Select + Start" is on.

/// Seconds the two buttons are held before the application quits.
pub const HOLD_SECONDS: f32 = 1.0;

/// What a frame of the combination decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComboEvent {
    /// Nothing yet, or nothing at all.
    Idle,
    /// Both buttons were let go before the hold completed.
    Tap,
    /// The hold completed: quit.
    Quit,
}

/// The timer of the combination.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ExitCombo {
    held_for: f32,
    quit_sent: bool,
}

impl ExitCombo {
    /// A combination nobody holds.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            held_for: 0.0,
            quit_sent: false,
        }
    }

    /// How far the hold has come, from 0 to 1, for the bar the screens draw.
    #[must_use]
    pub fn progress(&self) -> f32 {
        (self.held_for / HOLD_SECONDS).clamp(0.0, 1.0)
    }

    /// One frame of `seconds`, with the two buttons held or not.
    pub fn update(&mut self, seconds: f32, both_held: bool) -> ComboEvent {
        if both_held {
            self.held_for += seconds;
            if self.held_for >= HOLD_SECONDS && !self.quit_sent {
                self.quit_sent = true;
                return ComboEvent::Quit;
            }
            return ComboEvent::Idle;
        }
        let tapped = self.held_for > 0.0 && !self.quit_sent;
        *self = Self::new();
        if tapped {
            ComboEvent::Tap
        } else {
            ComboEvent::Idle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hold_of_one_second_quits_once() {
        let mut combo = ExitCombo::new();
        for _ in 0..9 {
            assert_eq!(ComboEvent::Idle, combo.update(0.1, true));
        }
        assert!(combo.progress() > 0.85 && combo.progress() < 1.0);
        assert_eq!(ComboEvent::Quit, combo.update(0.15, true));
        assert_eq!(
            ComboEvent::Idle,
            combo.update(0.5, true),
            "quit is sent once"
        );
        assert_eq!(
            ComboEvent::Idle,
            combo.update(0.1, false),
            "and releasing is no tap"
        );
        assert_eq!(0.0, combo.progress());
    }

    #[test]
    fn a_short_press_is_a_tap() {
        let mut combo = ExitCombo::new();
        assert_eq!(ComboEvent::Idle, combo.update(0.3, true));
        assert_eq!(ComboEvent::Tap, combo.update(0.016, false));
        assert_eq!(ComboEvent::Idle, combo.update(0.016, false));
    }

    #[test]
    fn nothing_held_does_nothing() {
        let mut combo = ExitCombo::new();
        assert_eq!(ComboEvent::Idle, combo.update(5.0, false));
        assert_eq!(0.0, combo.progress());
    }
}
