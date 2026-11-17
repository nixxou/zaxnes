//! Deleting games and slots behind a confirmation, and the cover art (SPEC-09 REQ-PERS-036,
//! REQ-PERS-042, SPEC-10 REQ-UI-037, REQ-UI-110).

use super::{EmulationHost, GameView};
use crate::emulation::game_catalog::GameCatalog;
use crate::persistence::app_settings::AppSettings;
use crate::persistence::save_slots;

/// The question asked before a game is deleted.
pub const DELETE_GAME_QUESTION: &str = "Do you want to delete the game and its save states ?";
/// The label of the confirming button.
pub const YES_LABEL: &str = "Yes";
/// The label of the cancelling button.
pub const NO_LABEL: &str = "No";

/// The question asked before a slot is deleted.
#[must_use]
pub fn delete_slot_question(slot_id: &str) -> String {
    format!("Do you want to delete the slot {slot_id} ?")
}

/// What a confirmation does once the user agrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmedAction {
    /// Delete the slot with this identifier.
    DeleteSlot(String),
    /// Load the slot with the components of the built-in hack set now.
    LoadSlotAsSet(String),
    /// Set the components of the built-in hack as the slot's state had them, then load it.
    LoadSlotAligned(String),
    /// Nothing: a message the user only acknowledges (no cancelling button).
    Acknowledge,
    /// Force the built-in patch called `patch` on the ROM in `rom` (Rust-only).
    ForcePatch {
        /// The ROM file.
        rom: String,
        /// The patch's name.
        patch: String,
    },
}

/// A question the interface puts to the user before a deletion (REQ-UI-110).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationRequest {
    /// The title of the dialogue: the game.
    pub title: String,
    /// The question.
    pub message: String,
    /// The label of the confirming button.
    pub confirm_label: String,
    /// The label of the cancelling button.
    pub cancel_label: String,
    /// What agreeing does.
    pub action: ConfirmedAction,
    /// A second answer, with its button's label, besides agreeing and cancelling.
    pub alternative: Option<(String, ConfirmedAction)>,
}

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// The message shown once a game is deleted, in the language of the interface.
    #[must_use]
    pub fn deleted_message(&self, title: &str) -> String {
        if self.settings.language() == "fr" {
            format!("« {title} » supprimé.")
        } else {
            format!("“{title}” deleted.")
        }
    }

    /// The question waiting for the user's answer, if the interface is to ask it.
    #[must_use]
    pub const fn pending_confirmation(&self) -> Option<&ConfirmationRequest> {
        self.confirmation.as_ref()
    }

    /// The user agreed: does what the question was about. Returns whether there was one.
    pub fn confirm_pending(&mut self) -> bool {
        match self.confirmation.take() {
            Some(request) => {
                self.perform(request.action);
                true
            }
            None => false,
        }
    }

    /// The user chose the question's second answer, if it has one: does it. Returns whether
    /// there was one.
    pub fn alternative_pending(&mut self) -> bool {
        match self.confirmation.take() {
            Some(ConfirmationRequest {
                alternative: Some((_, action)),
                ..
            }) => {
                self.perform(action);
                true
            }
            other => {
                self.confirmation = other;
                false
            }
        }
    }

    /// The user declined: forgets the question.
    pub fn cancel_pending(&mut self) {
        self.confirmation = None;
    }

    pub(super) fn confirm(&mut self, request: ConfirmationRequest) -> bool {
        if self.confirm_with_interface {
            self.confirmation = Some(request);
        } else {
            self.perform(request.action);
        }
        true
    }

    fn perform(&mut self, action: ConfirmedAction) {
        match action {
            ConfirmedAction::DeleteSlot(slot_id) => self.delete_slot(&slot_id),
            ConfirmedAction::LoadSlotAsSet(slot_id) => self.load_slot_checked(&slot_id, false),
            ConfirmedAction::LoadSlotAligned(slot_id) => self.load_slot_aligned(&slot_id),
            ConfirmedAction::Acknowledge => {}
            ConfirmedAction::ForcePatch { rom, patch } => self.force_patch(&rom, &patch),
        }
    }

    /// Asks whether to force `patch` on the ROM in `rom`, `message` saying why it is not made
    /// for it; `force` and `cancel` are the labels of the buttons.
    pub fn ask_to_force_patch(
        &mut self,
        (title, message): (&str, &str),
        (force, cancel): (&str, &str),
        rom: &str,
        patch: &str,
    ) {
        self.log(&format!("{title}: {message}"));
        self.confirm(ConfirmationRequest {
            title: title.to_owned(),
            message: message.to_owned(),
            confirm_label: force.to_owned(),
            cancel_label: cancel.to_owned(),
            alternative: None,
            action: ConfirmedAction::ForcePatch {
                rom: rom.to_owned(),
                patch: patch.to_owned(),
            },
        });
    }

    /// Shows `message` under `title` until the user acknowledges it (Rust-only): a question with
    /// one button.
    pub fn inform(&mut self, title: &str, message: &str) {
        self.log(&format!("{title}: {message}"));
        self.confirm(ConfirmationRequest {
            title: title.to_owned(),
            message: message.to_owned(),
            confirm_label: "OK".to_owned(),
            cancel_label: String::new(),
            alternative: None,
            action: ConfirmedAction::Acknowledge,
        });
    }

    /// Deletes the files of the slot and empties it in the index.
    pub fn delete_slot(&mut self, slot_id: &str) {
        if !save_slots::is_valid_slot_id(slot_id) {
            return;
        }
        self.slots.delete_files(&self.paths, slot_id);
        self.slots.vacate(slot_id);
        self.save_slot_index();
    }

    /// Asks before deleting the slot; returns false when it holds nothing.
    pub fn request_delete_slot(&mut self, slot_id: &str) -> bool {
        if !save_slots::is_valid_slot_id(slot_id) {
            return false;
        }
        let Some(slot) = self.slots.find(slot_id).filter(|slot| slot.is_occupied()) else {
            return false;
        };
        let game_id = slot.game_id.clone();
        let title = self.games.title(&game_id).unwrap_or(game_id);
        self.confirm(ConfirmationRequest {
            title,
            message: delete_slot_question(slot_id),
            confirm_label: YES_LABEL.to_owned(),
            cancel_label: NO_LABEL.to_owned(),
            alternative: None,
            action: ConfirmedAction::DeleteSlot(slot_id.to_owned()),
        })
    }

    pub(super) fn save_slot_index(&self) {
        if self.legacy_slots.save(&self.paths).is_err() {
            self.log("the save slot index could not be written.");
        }
    }
}
