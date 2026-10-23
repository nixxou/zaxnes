//! The confirmation box of REQ-UI-110: the question the host asks before it deletes a game or a
//! slot, with its two buttons. The host holds the question; this is how the box reads it, with
//! the texts translated where they are shown (AMD-036 §1).

use super::notifications;
use crate::emulation::host::ConfirmationRequest;

/// The box over the question the host is asking.
#[derive(Debug, Clone, Copy)]
pub struct ConfirmationView<'a> {
    request: &'a ConfirmationRequest,
}

impl<'a> ConfirmationView<'a> {
    /// The box for `request`.
    #[must_use]
    pub const fn new(request: &'a ConfirmationRequest) -> Self {
        Self { request }
    }

    /// The title: the game.
    #[must_use]
    pub fn title(&self) -> &'a str {
        &self.request.title
    }

    /// The question, in the language of the interface.
    #[must_use]
    pub fn message(&self) -> String {
        notifications::translate(&self.request.message)
    }

    /// The label of the confirming button, in the language of the interface.
    #[must_use]
    pub fn confirm_label(&self) -> String {
        notifications::translate(&self.request.confirm_label)
    }

    /// The label of the cancelling button, in the language of the interface.
    #[must_use]
    pub fn cancel_label(&self) -> String {
        notifications::translate(&self.request.cancel_label)
    }
}
