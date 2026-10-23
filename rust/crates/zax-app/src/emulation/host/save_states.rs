//! Save states and rollback (SPEC-07 §3.4, SPEC-09 REQ-PERS-050 to REQ-PERS-058).

use zax_engine3d::state_codec;
use zax_nes::NesCore;
use zax_render::assets::png_image;

use super::{EmulationHost, THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH};
use crate::audio::UiSound;
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::notification_center::{NotificationSeverity, DEFAULT_SECONDS, SHORT_SECONDS};
use crate::persistence::save_slots::{is_valid_slot_id, QUICK_SLOT_ID};
use crate::persistence::save_state_file::{self, SaveStateContent, SaveStateHeader};
use crate::user_data_paths::{remove_file_if_present, write_atomic};

impl<C: GameCatalog> EmulationHost<C> {
    /// Captures the state of the game being played for `slot_id`: the core, the scripts' data,
    /// the camera and the learned conversion state (REQ-PERS-055).
    pub fn capture_save_state(&mut self, slot_id: &str) -> Option<SaveStateContent> {
        let game_id = self.current_game_id()?.to_owned();
        let fingerprint = self.current_rom_fingerprint().unwrap_or_default();
        let saved_at = (self.now)().unix_timestamp();
        let session = self.session.as_mut().filter(|session| session.has_rom())?;
        let lua = match session.capture_script_state(&mut self.settings, &mut self.scene) {
            Ok(lua) => lua,
            Err(error) => {
                self.log(&format!("the scripts' state could not be read: {error}"));
                return None;
            }
        };
        let core = session.core().save_state(true).ok()?;
        let camera = save_state_file::write_camera(&self.scene.camera.capture());
        let conversion = state_codec::write(session.engine());
        Some(SaveStateContent {
            header: SaveStateHeader {
                core_id: NesCore::CORE_NAME.to_owned(),
                core_state_version: NesCore::STATE_VERSION,
                game_id,
                rom_sha1: fingerprint,
                saved_at,
                slot_id: slot_id.to_owned(),
            },
            core,
            lua,
            camera,
            conversion,
        })
    }

    /// Saves to `slot_id` (REQ-LOOP-041 to REQ-LOOP-043). Returns whether the file was written.
    pub fn save_to_slot(&mut self, slot_id: &str, with_thumbnail: bool) -> bool {
        // REQ-LOOP-041: a save only happens outside Stopped and with a current game.
        if self.state == EmulationState::Stopped
            || self.loading
            || !self
                .session
                .as_ref()
                .is_some_and(crate::game_session::GameSession::has_rom)
        {
            return false;
        }
        if !is_valid_slot_id(slot_id) || self.slots.find(slot_id).is_none() {
            return false;
        }
        if !self.paths.is_writable {
            // REQ-PERS-073: a write action on a read-only root shows the same notification.
            let message = self
                .paths
                .unwritable_message
                .clone()
                .unwrap_or_else(|| "User data folder is not writable.".to_owned());
            self.notifications.show_error(&message);
            return false;
        }
        if let Some(mixer) = &mut self.mixer {
            mixer.play_sound(UiSound::Confirm);
        }
        let Some(content) = self.capture_save_state(slot_id) else {
            return false;
        };
        let picture = if with_thumbnail {
            self.thumbnail_for_slot()
        } else {
            None
        };
        match self.write_slot(slot_id, &content, picture.as_deref()) {
            Ok(()) => true,
            Err(error) => {
                self.slots.vacate(slot_id);
                self.save_slot_index();
                self.notifications.show_error(&error.to_string());
                false
            }
        }
    }

    fn write_slot(
        &mut self,
        slot_id: &str,
        content: &SaveStateContent,
        picture: Option<&[u8]>,
    ) -> std::io::Result<()> {
        self.slots.occupy(
            slot_id,
            &content.header.game_id,
            content.header.saved_at,
            picture.is_some(),
        );
        self.slots.save(&self.paths)?;
        let file = save_state_file::write(content);
        let bound = self.session.as_ref().map_or(0, |session| {
            save_state_file::max_file_size(session.core().max_state_size())
        });
        if file.len() > bound {
            // REQ-ARCH-013: the announced bound must hold for the whole session. Going past it
            // is a bug in the bound, not a reason to lose the user's state, so it is logged and
            // the state is still written.
            self.log(&format!(
                "savestate of slot {slot_id} is larger than the announced bound."
            ));
        }
        write_atomic(&self.paths.slot_state_file(slot_id), &file)?;
        let thumbnail_file = self.paths.slot_thumbnail_file(slot_id);
        if let Some(picture) = picture {
            let png = png_image::encode_rgb(
                u32::try_from(THUMBNAIL_WIDTH).unwrap_or(0),
                u32::try_from(THUMBNAIL_HEIGHT).unwrap_or(0),
                picture,
            )
            .map_err(std::io::Error::other)?;
            std::fs::write(thumbnail_file, png)?;
        } else {
            // A quick save has no thumbnail (REQ-LOOP-043): the previous one must not stay
            // behind and be shown for a different state.
            remove_file_if_present(&thumbnail_file);
        }
        Ok(())
    }

    /// The quick save (REQ-LOOP-043): the quick slot, no thumbnail, and its notification.
    pub fn quick_save(&mut self) {
        if self.current_game.is_none() || self.state == EmulationState::Stopped {
            return;
        }
        if self.save_to_slot(QUICK_SLOT_ID, false) {
            self.notifications
                .show("Quick Save", DEFAULT_SECONDS, NotificationSeverity::Normal);
        }
    }

    /// The quick load (REQ-LOOP-045): loads the quick slot, with its notification.
    pub fn quick_load(&mut self) {
        if !self.slots.is_occupied(QUICK_SLOT_ID) {
            return;
        }
        self.load_slot(QUICK_SLOT_ID);
        if self.loading {
            self.notifications
                .show("Quick Load", DEFAULT_SECONDS, NotificationSeverity::Normal);
        }
    }

    /// The picture a thumbnail is made of (Q-027): the capture taken at the pause while
    /// paused, a fresh capture otherwise.
    pub(super) fn thumbnail_for_slot(&mut self) -> Option<Vec<u8>> {
        if self.state == EmulationState::Paused {
            if let Some(paused) = &self.pause_thumbnail {
                return Some(paused.clone());
            }
        }
        self.capture_thumbnail
            .as_mut()
            .and_then(|capture| capture())
            .or_else(|| self.pause_thumbnail.clone())
    }

    /// Goes back to the second-to-last snapshot (REQ-LOOP-032).
    pub(super) fn perform_rollback(&mut self) {
        if self.state == EmulationState::Stopped
            || !self
                .session
                .as_ref()
                .is_some_and(crate::game_session::GameSession::has_rom)
        {
            return;
        }
        self.notifications
            .show("Roll Back", SHORT_SECONDS, NotificationSeverity::Normal);
        if let Some(snapshot) = self.rollback.take_rollback_target() {
            // DEC-07-Q3: the core alone. Neither the camera nor the scripts' persistent data
            // are restored, and no Save or Load hook runs (REQ-LUA-074).
            if let Some(session) = self.session.as_mut() {
                if let Err(error) = session.restore_core(&snapshot) {
                    self.log(&format!("rollback snapshot refused: {error}"));
                }
            }
        }
        // Step 3 applies even when nothing was restored (REQ-LOOP-032, notes).
        self.set_state(EmulationState::Running);
        self.pacer.reset();
    }
}
