//! The index of the save slots (SPEC-09 REQ-PERS-040 to 047): 36 numbered slots and the quick
//! one, each either empty or naming the game and the instant it holds.

use std::fs;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::persistence::save_state_file;
use crate::user_data_paths::{remove_file_if_present, set_aside_corrupt_file, write_atomic, UserDataPaths};

/// Schema version of the index file.
pub const SCHEMA_VERSION: i32 = 1;
/// Number of numbered slots.
pub const NUMBERED_SLOT_COUNT: usize = 36;
/// Number of numbered slots the menus expose.
pub const EXPOSED_SLOT_COUNT: usize = 10;
/// Identifier of the quick slot.
pub const QUICK_SLOT_ID: &str = "quick";

/// One slot of the index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSlotRecord {
    /// `"1"` to `"36"` or `"quick"`.
    #[serde(rename = "id", default)]
    pub id: String,
    /// The game saved, empty for an empty slot.
    #[serde(rename = "gameId", default)]
    pub game_id: String,
    /// Unix time of the save, 0 for an empty slot.
    #[serde(rename = "savedAt", default)]
    pub saved_at: i64,
    /// File name of the state.
    #[serde(rename = "file", default)]
    pub file: Option<String>,
    /// File name of the thumbnail.
    #[serde(rename = "thumbnail", default)]
    pub thumbnail: Option<String>,
}

impl SaveSlotRecord {
    /// Whether the slot holds a state.
    #[must_use]
    pub fn is_occupied(&self) -> bool {
        !self.game_id.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SaveSlotIndexFile {
    #[serde(rename = "schemaVersion", default)]
    schema_version: i32,
    #[serde(rename = "slots", default)]
    slots: Vec<SaveSlotRecord>,
}

/// The slots, in menu order: the numbered ones, then the quick one.
#[derive(Debug, Clone)]
pub struct SaveSlotIndex {
    slots: Vec<SaveSlotRecord>,
    /// Whether the index file was unreadable and has been rebuilt from the state files.
    pub was_recovered: bool,
}

impl Default for SaveSlotIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveSlotIndex {
    /// Creates an index of empty slots.
    #[must_use]
    pub fn new() -> Self {
        let mut slots: Vec<SaveSlotRecord> = (1..=NUMBERED_SLOT_COUNT)
            .map(|number| SaveSlotRecord {
                id: number.to_string(),
                ..SaveSlotRecord::default()
            })
            .collect();
        slots.push(SaveSlotRecord {
            id: QUICK_SLOT_ID.to_owned(),
            ..SaveSlotRecord::default()
        });
        Self {
            slots,
            was_recovered: false,
        }
    }

    /// Every slot, in menu order.
    #[must_use]
    pub fn slots(&self) -> &[SaveSlotRecord] {
        &self.slots
    }

    /// The slot named `slot_id`.
    #[must_use]
    pub fn find(&self, slot_id: &str) -> Option<&SaveSlotRecord> {
        self.slots.iter().find(|slot| slot.id == slot_id)
    }

    fn find_mut(&mut self, slot_id: &str) -> Option<&mut SaveSlotRecord> {
        self.slots.iter_mut().find(|slot| slot.id == slot_id)
    }

    /// Whether the slot named `slot_id` exists and holds a state.
    #[must_use]
    pub fn is_occupied(&self, slot_id: &str) -> bool {
        self.find(slot_id).is_some_and(SaveSlotRecord::is_occupied)
    }

    /// Records that `slot_id` now holds a state of `game_id`.
    ///
    /// Returns false when no slot has that identifier.
    pub fn occupy(&mut self, slot_id: &str, game_id: &str, saved_at: i64, has_thumbnail: bool) -> bool {
        let Some(slot) = self.find_mut(slot_id) else {
            return false;
        };
        game_id.clone_into(&mut slot.game_id);
        slot.saved_at = saved_at;
        slot.file = Some(format!("{slot_id}.state"));
        slot.thumbnail = has_thumbnail.then(|| format!("{slot_id}.png"));
        true
    }

    /// Empties `slot_id`; an unknown slot is ignored.
    pub fn vacate(&mut self, slot_id: &str) {
        if let Some(slot) = self.find_mut(slot_id) {
            slot.game_id.clear();
            slot.saved_at = 0;
            slot.file = None;
            slot.thumbnail = None;
        }
    }

    /// The occupied slots of `game_id`, most recent first.
    #[must_use]
    pub fn occupied_for(&self, game_id: &str) -> Vec<SaveSlotRecord> {
        let mut found: Vec<SaveSlotRecord> = self
            .slots
            .iter()
            .filter(|slot| slot.is_occupied() && slot.game_id == game_id)
            .cloned()
            .collect();
        found.sort_by_key(|slot| std::cmp::Reverse(slot.saved_at));
        found
    }

    /// Reads the index. A missing file gives empty slots; an unreadable one is set aside and the
    /// index is rebuilt from the state files (REQ-PERS-047).
    #[must_use]
    pub fn load(paths: &UserDataPaths, now: OffsetDateTime) -> Self {
        let mut index = Self::new();
        if !paths.save_index_file.is_file() {
            return index;
        }
        let parsed = fs::read(&paths.save_index_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SaveSlotIndexFile>(&bytes).ok());
        let Some(file) = parsed else {
            set_aside_corrupt_file(&paths.save_index_file, now);
            let mut recovered = Self::new();
            recovered.was_recovered = true;
            recovered.rebuild_from_files(paths);
            return recovered;
        };
        for record in file.slots {
            if !record.is_occupied() {
                continue;
            }
            if let Some(slot) = index.find_mut(&record.id) {
                slot.game_id = record.game_id;
                slot.saved_at = record.saved_at;
                slot.file = Some(record.file.unwrap_or_else(|| format!("{}.state", record.id)));
                slot.thumbnail = record.thumbnail;
            }
        }
        index
    }

    /// Reads the header of every state file and fills the slots from them. A slot whose file
    /// cannot be read stays empty.
    pub fn rebuild_from_files(&mut self, paths: &UserDataPaths) {
        for slot in &mut self.slots {
            let Ok(bytes) = fs::read(paths.slot_state_file(&slot.id)) else {
                continue;
            };
            let Some(content) = save_state_file::try_read(&bytes) else {
                continue;
            };
            slot.game_id = content.header.game_id;
            slot.saved_at = content.header.saved_at;
            slot.file = Some(format!("{}.state", slot.id));
            slot.thumbnail = paths
                .slot_thumbnail_file(&slot.id)
                .is_file()
                .then(|| format!("{}.png", slot.id));
        }
    }

    /// Empties the slots whose game is gone or whose file is missing, deleting the files of the
    /// former. Returns how many slots were cleared (REQ-PERS-046).
    pub fn reconcile(&mut self, paths: &UserDataPaths, game_exists: impl Fn(&str) -> bool) -> usize {
        let mut cleared = 0;
        for slot in &mut self.slots {
            if !slot.is_occupied() {
                continue;
            }
            if !game_exists(&slot.game_id) {
                delete_files(paths, &slot.id);
            } else if paths.slot_state_file(&slot.id).is_file() {
                continue;
            }
            slot.game_id.clear();
            slot.saved_at = 0;
            slot.file = None;
            slot.thumbnail = None;
            cleared += 1;
        }
        cleared
    }

    /// Writes the index, unless the root is read-only.
    ///
    /// # Errors
    /// The file could not be written.
    pub fn save(&self, paths: &UserDataPaths) -> std::io::Result<()> {
        if !paths.is_writable {
            return Ok(());
        }
        let file = SaveSlotIndexFile {
            schema_version: SCHEMA_VERSION,
            slots: self.slots.clone(),
        };
        let json = serde_json::to_vec_pretty(&file).map_err(std::io::Error::other)?;
        write_atomic(&paths.save_index_file, &json)
    }
}

/// The identifier of numbered slot `number` (1 to 36).
#[must_use]
pub fn numbered_slot_id(number: i32) -> String {
    number.to_string()
}

/// Whether `slot_id` names the quick slot or a numbered one.
#[must_use]
pub fn is_valid_slot_id(slot_id: &str) -> bool {
    slot_id == QUICK_SLOT_ID
        || slot_id
            .parse::<usize>()
            .is_ok_and(|number| (1..=NUMBERED_SLOT_COUNT).contains(&number))
}

/// Deletes the state and the thumbnail of `slot_id`. A file that cannot be removed is left: the
/// slot is empty in the index either way.
pub fn delete_files(paths: &UserDataPaths, slot_id: &str) {
    remove_file_if_present(&paths.slot_state_file(slot_id));
    remove_file_if_present(&paths.slot_thumbnail_file(slot_id));
}
