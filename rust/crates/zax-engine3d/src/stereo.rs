//! Lots F3D and SBS: which eye a Famicom 3D System game draws for.
//!
//! Those games draw one field for the left eye and the next for the right eye, and say which on
//! OUT1 of `$4016` (nesdev, "Famicom 3D System"), for the shutter glasses. The tracker only notes
//! that bit at the end of each field and counts how many fields in a row alternated: a three-bit
//! test and a counter, nothing copied, for every game. The application then keeps the picture of
//! each eye and shows the two side by side (the faithful side by side of lot SBS), or the left
//! one alone.
//!
//! Lot F3D first measured the disparity of the two pictures to give these games a relief in the
//! default 3D; the owner found the result worse than the 2D and had it removed (2026-11-25). The
//! measure lives in the history of `feat/famicom-3d-system` (`docs/progress/F3D.md`).

/// How many alternating fields in a row make a game a 3D System one (a tenth of a second).
const ACTIVE_AFTER: u32 = 6;

/// The eye of the fields of the game in play and whether it is a 3D System one.
#[derive(Debug, Clone, Copy, Default)]
pub struct StereoTracker {
    /// Fields in a row whose eye differs from the field before.
    alternations: u32,
    /// The eye of the newest field (`Some(true)` for the right one).
    last_eye: Option<bool>,
}

impl StereoTracker {
    /// Forgets everything (a new game, a reset, a state loaded).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Records which eye the field the core just ran is for: `outputs` the OUT0-OUT2 lines at its
    /// end, `None` on an engine that does not report them.
    pub fn note_eye(&mut self, outputs: Option<u8>) {
        let Some(outputs) = outputs else {
            self.clear();
            return;
        };
        let right = outputs & 0x02 != 0;
        self.alternations = if self.last_eye.is_some_and(|last| last != right) {
            self.alternations.saturating_add(1)
        } else {
            0
        };
        self.last_eye = Some(right);
    }

    /// Whether the game alternates the eyes field after field.
    #[must_use]
    pub const fn active(&self) -> bool {
        self.alternations >= ACTIVE_AFTER
    }

    /// Which eye the newest field is for, while active: `Some(true)` for the right one.
    #[must_use]
    pub const fn newest_eye(&self) -> Option<bool> {
        if self.active() {
            self.last_eye
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_until_the_eyes_alternate() {
        let mut tracker = StereoTracker::default();
        for _ in 0..10 {
            tracker.note_eye(Some(0));
        }
        assert!(!tracker.active());
        assert_eq!(tracker.newest_eye(), None);
    }

    #[test]
    fn active_after_six_alternations_and_says_the_eye() {
        let mut tracker = StereoTracker::default();
        for field in 0..7u8 {
            tracker.note_eye(Some((field % 2) * 2));
        }
        assert!(tracker.active());
        assert_eq!(tracker.newest_eye(), Some(false));
        tracker.note_eye(Some(2));
        assert_eq!(tracker.newest_eye(), Some(true));
        // Two fields for the same eye: no longer a 3D System picture.
        tracker.note_eye(Some(2));
        assert!(!tracker.active());
    }

    #[test]
    fn an_engine_without_the_lines_forgets_everything() {
        let mut tracker = StereoTracker::default();
        for field in 0..8u8 {
            tracker.note_eye(Some((field % 2) * 2));
        }
        tracker.note_eye(None);
        assert!(!tracker.active());
        assert_eq!(tracker.newest_eye(), None);
    }
}
