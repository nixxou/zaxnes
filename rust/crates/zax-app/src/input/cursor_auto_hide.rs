//! The system cursor hides itself after two seconds of stillness while a game runs (SPEC-08
//! REQ-INP-110, REQ-INP-111).

/// Decides when the cursor is shown.
#[derive(Debug, Clone, Copy)]
pub struct CursorAutoHide {
    since_check: f32,
    moved_since_check: bool,
    still_checks: u32,
    cursor_visible: bool,
}

impl Default for CursorAutoHide {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorAutoHide {
    /// Interval between two stillness checks, in real seconds.
    pub const CHECK_SECONDS: f32 = 0.1;
    /// Still checks in a row after which the cursor hides.
    pub const CHECKS_BEFORE_HIDING: u32 = 20;

    /// Creates the state with the cursor visible.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            since_check: 0.0,
            moved_since_check: false,
            still_checks: 0,
            cursor_visible: true,
        }
    }

    /// How many checks in a row found the pointer still.
    #[must_use]
    pub const fn still_checks(&self) -> u32 {
        self.still_checks
    }

    /// Whether the cursor is shown.
    #[must_use]
    pub const fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Feeds one displayed frame and returns whether the cursor is shown.
    pub fn update(&mut self, game_running: bool, pointer_moved: bool, real_seconds: f32) -> bool {
        if !game_running {
            // REQ-INP-111: visible at all times in the menu, in pause, in the library and while
            // loading, and the counter starts again from zero when the game resumes.
            self.show();
            return self.cursor_visible;
        }
        if pointer_moved {
            // REQ-INP-110: any movement shows it again immediately and resets the counter.
            self.show();
            self.moved_since_check = true;
        }
        self.since_check += real_seconds;
        while self.since_check >= Self::CHECK_SECONDS {
            self.since_check -= Self::CHECK_SECONDS;
            if self.moved_since_check {
                self.still_checks = 0;
                self.moved_since_check = false;
                continue;
            }
            self.still_checks += 1;
            if self.still_checks >= Self::CHECKS_BEFORE_HIDING {
                self.cursor_visible = false;
            }
        }
        self.cursor_visible
    }

    /// Shows the cursor and restarts the count.
    pub fn show(&mut self) {
        *self = Self::new();
    }
}
