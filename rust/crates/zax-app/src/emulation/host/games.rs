//! Deleting games and slots behind a confirmation, and the cover art (SPEC-09 REQ-PERS-036,
//! REQ-PERS-042, SPEC-10 REQ-UI-037, REQ-UI-110).

use super::EmulationHost;
use crate::emulation::game_catalog::GameCatalog;
use crate::persistence::app_settings::AppSettings;
use crate::persistence::game_art;
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
    /// Delete the game with this identifier.
    DeleteGame(String),
    /// Delete the slot with this identifier.
    DeleteSlot(String),
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
}

impl<C: GameCatalog> EmulationHost<C> {
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

    /// The user declined: forgets the question.
    pub fn cancel_pending(&mut self) {
        self.confirmation = None;
    }

    fn confirm(&mut self, request: ConfirmationRequest) -> bool {
        if self.confirm_with_interface {
            self.confirmation = Some(request);
        } else {
            self.perform(request.action);
        }
        true
    }

    fn perform(&mut self, action: ConfirmedAction) {
        match action {
            ConfirmedAction::DeleteGame(game_id) => {
                self.delete_game(&game_id);
            }
            ConfirmedAction::DeleteSlot(slot_id) => self.delete_slot(&slot_id),
        }
    }

    /// Asks before deleting the game; returns false when it cannot be deleted now.
    pub fn request_delete_game(&mut self, game_id: &str) -> bool {
        if game_id.is_empty() || self.loading || !self.games.can_delete(game_id) {
            return false;
        }
        let title = self.games.title(game_id).unwrap_or_else(|| game_id.to_owned());
        self.confirm(ConfirmationRequest {
            title,
            message: DELETE_GAME_QUESTION.to_owned(),
            confirm_label: YES_LABEL.to_owned(),
            cancel_label: NO_LABEL.to_owned(),
            action: ConfirmedAction::DeleteGame(game_id.to_owned()),
        })
    }

    /// Deletes a user game: the emulation stops when it is the one playing, its slots go, then
    /// its profile file, its cover and its library entry (REQ-PERS-042).
    pub fn delete_game(&mut self, game_id: &str) -> bool {
        if game_id.is_empty() || !self.games.can_delete(game_id) {
            return false;
        }
        // The title is read before the entry goes, so the notification can name the game.
        let title = self.games.title(game_id).unwrap_or_else(|| game_id.to_owned());
        // 1. The emulation stops when the game being deleted is the one playing.
        if self.current_game_id() == Some(game_id) {
            self.stop_current_game(true, true);
        }
        // 2. Every slot of that game, the quick slot included.
        for slot in self.slots.occupied_for(game_id) {
            self.delete_slot(&slot.id);
        }
        // 3 to 5. The profile file, the cover art and the library entry, then library.json.
        if !self.games.delete_game(game_id) {
            return false;
        }
        let message = self.deleted_message(&title);
        self.notifications.show_normal(&message);
        true
    }

    /// Stores a capture of the game as its cover (REQ-PERS-036); returns whether it did.
    pub fn create_game_art(&mut self) -> bool {
        let Some((id, fallback_title)) = self
            .current_game
            .as_ref()
            .map(|game| (game.id.clone(), game.title.clone()))
        else {
            return false;
        };
        let Some(capture) = self.thumbnail_for_slot() else {
            return false;
        };
        let title = self.games.title(&id).unwrap_or(fallback_title);
        let stored = game_art::set_cover_from_capture(&self.paths, &title, &id, &capture);
        self.store_cover(&id, stored, false)
    }

    /// Fits the PNG at `path` as the cover of the current game; returns whether it did.
    pub fn import_cover(&mut self, path: &str) -> bool {
        let Some(game) = self.current_game.as_ref() else {
            return false;
        };
        if path.is_empty() {
            return false;
        }
        let (id, fallback_title) = (game.id.clone(), game.title.clone());
        let title = self.games.title(&id).unwrap_or(fallback_title);
        let stored = Some(game_art::import_cover(&self.paths, &title, &id, std::path::Path::new(path)));
        self.store_cover(&id, stored, false)
    }

    /// The cover of the "2D Mode" entry follows the pause capture every time the menu appears
    /// (REQ-PERS-036, third source).
    pub(super) fn refresh_two_dimensional_cover(&mut self) {
        let Some(game) = self.current_game.as_ref().filter(|game| game.is_2d_mode) else {
            return;
        };
        let Some(capture) = self.pause_thumbnail.as_deref() else {
            return;
        };
        let (id, fallback_title) = (game.id.clone(), game.title.clone());
        let title = self.games.title(&id).unwrap_or(fallback_title);
        let stored = game_art::set_cover_from_capture(&self.paths, &title, &id, capture);
        self.store_cover(&id, stored, true);
    }

    fn store_cover(&mut self, game_id: &str, stored: Option<Result<String, String>>, quiet: bool) -> bool {
        match stored {
            Some(Ok(file_name)) => {
                self.games.set_cover(game_id, Some(&file_name));
                true
            }
            Some(Err(failure)) => {
                if !quiet && !failure.is_empty() {
                    self.notifications.show_error(&failure);
                }
                false
            }
            None => false,
        }
    }

    /// Deletes the files of the slot and empties it in the index.
    pub fn delete_slot(&mut self, slot_id: &str) {
        if !save_slots::is_valid_slot_id(slot_id) {
            return;
        }
        save_slots::delete_files(&self.paths, slot_id);
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
            action: ConfirmedAction::DeleteSlot(slot_id.to_owned()),
        })
    }

    pub(super) fn save_slot_index(&self) {
        if self.slots.save(&self.paths).is_err() {
            self.log("the save slot index could not be written.");
        }
    }
}
