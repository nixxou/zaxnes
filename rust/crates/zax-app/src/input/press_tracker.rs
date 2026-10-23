//! Edge and press-length detection over the samples of the displayed frames.

/// How a press ended or progressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressKind {
    /// Nothing to report.
    None,
    /// The key was released before the long-press threshold.
    Short,
    /// The key crossed the long-press threshold.
    Long,
}

/// Tells a short press from a long one (SPEC-08 REQ-INP-101).
#[derive(Debug, Clone, Copy, Default)]
pub struct PressTracker {
    held: f32,
    down: bool,
    consumed: bool,
}

impl PressTracker {
    /// How long a press must last to be long, in real seconds.
    pub const LONG_PRESS_SECONDS: f32 = 0.3;

    /// Creates a tracker with nothing pressed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            held: 0.0,
            down: false,
            consumed: false,
        }
    }

    /// Whether the key is down.
    #[must_use]
    pub const fn is_down(&self) -> bool {
        self.down
    }

    /// How long the key has been held, in real seconds.
    #[must_use]
    pub const fn held_seconds(&self) -> f32 {
        self.held
    }

    /// Feeds one sample. A long press reports as soon as the threshold is crossed; a short one
    /// when the key is released.
    pub fn update(&mut self, down: bool, real_seconds: f32) -> PressKind {
        if down {
            self.held = if self.down { self.held + real_seconds } else { 0.0 };
            self.down = true;
            if !self.consumed && self.held >= Self::LONG_PRESS_SECONDS {
                self.consumed = true;
                return PressKind::Long;
            }
            return PressKind::None;
        }
        if !self.down {
            return PressKind::None;
        }
        self.down = false;
        let was_consumed = std::mem::take(&mut self.consumed);
        if was_consumed {
            PressKind::None
        } else {
            PressKind::Short
        }
    }

    /// Forgets any press in progress.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Reports the frames on which a key goes down or up.
#[derive(Debug, Clone, Copy, Default)]
pub struct EdgeDetector {
    previous: bool,
}

impl EdgeDetector {
    /// Whether the previous sample was down.
    #[must_use]
    pub const fn was_down(&self) -> bool {
        self.previous
    }

    /// Feeds a sample and reports a transition from up to down.
    pub fn rising(&mut self, current: bool) -> bool {
        let rising = current && !self.previous;
        self.previous = current;
        rising
    }

    /// Feeds a sample and reports a transition from down to up.
    pub fn falling(&mut self, current: bool) -> bool {
        let falling = !current && self.previous;
        self.previous = current;
        falling
    }

    /// Forgets the previous sample.
    pub fn reset(&mut self) {
        self.previous = false;
    }
}
