//! The confirmation box of REQ-UI-110 as data: the question the host asks before it deletes a
//! game or a slot, with its two buttons. The texts are translated where they are read
//! (AMD-036 §1).

use super::notifications;
use crate::emulation::host::{ConfirmationRequest, ConfirmedAction};

/// The question being asked, if any.
#[derive(Debug, Clone, Default)]
pub struct ConfirmDialogModel {
    request: Option<ConfirmationRequest>,
}

impl ConfirmDialogModel {
    /// A closed box.
    #[must_use]
    pub const fn new() -> Self {
        Self { request: None }
    }

    /// Whether the box is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.request.is_some()
    }

    /// The title: the game.
    #[must_use]
    pub fn title(&self) -> &str {
        self.request
            .as_ref()
            .map_or("", |request| request.title.as_str())
    }

    /// The question, in the language of the interface.
    #[must_use]
    pub fn message(&self) -> String {
        self.request.as_ref().map_or_else(String::new, |request| {
            notifications::translate(&request.message)
        })
    }

    /// The label of the confirming button, in the language of the interface.
    #[must_use]
    pub fn confirm_label(&self) -> String {
        self.request.as_ref().map_or_else(String::new, |request| {
            notifications::translate(&request.confirm_label)
        })
    }

    /// The label of the cancelling button, in the language of the interface.
    #[must_use]
    pub fn cancel_label(&self) -> String {
        self.request.as_ref().map_or_else(String::new, |request| {
            notifications::translate(&request.cancel_label)
        })
    }

    /// Opens the box on `request`.
    pub fn open(&mut self, request: ConfirmationRequest) {
        self.request = Some(request);
    }

    /// The user agreed: closes the box and returns what to do.
    pub fn confirm(&mut self) -> Option<ConfirmedAction> {
        self.request.take().map(|request| request.action)
    }

    /// The box closes without doing anything.
    pub fn close(&mut self) {
        self.request = None;
    }
}
