//! The single notification area of SPEC-10 §3.10: one notification at a time, no stacking, and
//! a timer measured in real time (REQ-UI-080, REQ-UI-081).

/// Severity of a notification (SPEC-10 REQ-UI-080).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationSeverity {
    /// White text.
    Normal,
    /// Yellow text.
    Warning,
    /// Red text.
    Error,
}

/// One notification: a text, a duration in real seconds and a severity.
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    /// Exact text to show.
    pub text: String,
    /// Severity, which decides the colour and the replacement rule.
    pub severity: NotificationSeverity,
    /// Duration in real seconds, unaffected by pause or game speed.
    pub seconds: f64,
}

/// Default duration, 4 s (REQ-UI-080).
pub const DEFAULT_SECONDS: f64 = 4.0;
/// Duration of the rollback notification (SPEC-07 §4).
pub const SHORT_SECONDS: f64 = 1.0;

/// The notification area. A new notification replaces the visible one, restarting its timer,
/// when nothing is visible or when its severity is at least that of the visible one; otherwise
/// it is dropped.
#[derive(Debug, Clone, Default)]
pub struct NotificationCenter {
    log: Vec<Notification>,
    current: Option<Notification>,
    remaining: f64,
}

impl NotificationCenter {
    /// Creates an empty area.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The visible notification, if any.
    #[must_use]
    pub fn current(&self) -> Option<&Notification> {
        if self.remaining > 0.0 {
            self.current.as_ref()
        } else {
            None
        }
    }

    /// Real seconds left before the visible notification disappears.
    #[must_use]
    pub const fn remaining(&self) -> f64 {
        self.remaining
    }

    /// Every notification that was accepted, oldest first. For tests and logs.
    #[must_use]
    pub fn log(&self) -> &[Notification] {
        &self.log
    }

    /// Shows `text`, applying the replacement rule of REQ-UI-081. Returns whether it was
    /// accepted.
    pub fn show(&mut self, text: &str, seconds: f64, severity: NotificationSeverity) -> bool {
        if self
            .current()
            .is_some_and(|visible| severity < visible.severity)
        {
            return false;
        }
        let notification = Notification {
            text: text.to_owned(),
            severity,
            seconds,
        };
        self.current = Some(notification.clone());
        self.remaining = seconds;
        self.log.push(notification);
        true
    }

    /// Shows `text` for the default duration with normal severity.
    pub fn show_normal(&mut self, text: &str) -> bool {
        self.show(text, DEFAULT_SECONDS, NotificationSeverity::Normal)
    }

    /// Shows an error notification of the default duration (REQ-UI-083).
    pub fn show_error(&mut self, text: &str) -> bool {
        self.show(text, DEFAULT_SECONDS, NotificationSeverity::Error)
    }

    /// Advances the timer by `real_seconds` of real time.
    pub fn advance(&mut self, real_seconds: f64) {
        if self.remaining > 0.0 {
            self.remaining = (self.remaining - real_seconds).max(0.0);
        }
    }

    /// Hides the visible notification immediately.
    pub const fn clear(&mut self) {
        self.remaining = 0.0;
    }
}
