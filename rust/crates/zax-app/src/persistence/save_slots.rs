//! The save slots of a ROM (SPEC-09 REQ-PERS-040 to 047): 36 numbered slots and the quick one,
//! each either empty or naming the game and the instant it holds. Owner request: a ROM's states
//! are files named after it (`<rom>.ss<n>`, the quick slot's `<rom>.ss0`), read from the files
//! when the game is chosen; only its slots are shown. The slots shared by every game before
//! (`index.json`, `<slot>.state`) are read once more to move a game's states to its files.

use std::fs;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::persistence::save_state_file;
use crate::user_data_paths::{
    remove_file_if_present, set_aside_corrupt_file, write_atomic, UserDataPaths,
};

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
    /// The ROM whose states the slots hold (its file name without extension); `None` for the
    /// shared slots of before, or for none.
    rom: Option<String>,
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
            rom: None,
            was_recovered: false,
        }
    }

    /// The slots of the ROM `rom`, read from its state files.
    #[must_use]
    pub fn for_rom(paths: &UserDataPaths, rom: &str) -> Self {
        let mut index = Self::new();
        index.rom = Some(rom.to_owned());
        index.rebuild_from_files(paths);
        index
    }

    /// The ROM whose slots these are, if any.
    #[must_use]
    pub fn rom(&self) -> Option<&str> {
        self.rom.as_deref()
    }

    /// The state file of `slot_id`: the ROM's, or the shared slots' of before.
    #[must_use]
    pub fn state_file(&self, paths: &UserDataPaths, slot_id: &str) -> std::path::PathBuf {
        self.rom.as_deref().map_or_else(
            || paths.legacy_slot_state_file(slot_id),
            |rom| paths.slot_state_file(rom, slot_id),
        )
    }

    /// The thumbnail of `slot_id`: the ROM's, or the shared slots' of before.
    #[must_use]
    pub fn thumbnail_file(&self, paths: &UserDataPaths, slot_id: &str) -> std::path::PathBuf {
        self.rom.as_deref().map_or_else(
            || paths.legacy_slot_thumbnail_file(slot_id),
            |rom| paths.slot_thumbnail_file(rom, slot_id),
        )
    }

    /// Deletes the state and the thumbnail of `slot_id`. A file that cannot be removed is left:
    /// the slot is empty either way.
    pub fn delete_files(&self, paths: &UserDataPaths, slot_id: &str) {
        remove_file_if_present(&self.state_file(paths, slot_id));
        remove_file_if_present(&self.thumbnail_file(paths, slot_id));
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
    pub fn occupy(
        &mut self,
        slot_id: &str,
        game_id: &str,
        saved_at: i64,
        has_thumbnail: bool,
    ) -> bool {
        let rom = self.rom.clone();
        let Some(slot) = self.find_mut(slot_id) else {
            return false;
        };
        game_id.clone_into(&mut slot.game_id);
        slot.saved_at = saved_at;
        let extension = crate::user_data_paths::slot_extension(slot_id);
        if let Some(rom) = rom.as_deref() {
            slot.file = Some(format!("{rom}.{extension}"));
            slot.thumbnail = has_thumbnail.then(|| format!("{rom}.{extension}.png"));
        } else {
            slot.file = Some(format!("{slot_id}.state"));
            slot.thumbnail = has_thumbnail.then(|| format!("{slot_id}.png"));
        }
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

    /// Reads the index of the slots shared by every game before the files were named after the
    /// ROM (owner request), whose states are moved to the files of their game when it is
    /// chosen. A missing file gives empty slots; an unreadable one is set aside and the index is
    /// rebuilt from the state files (REQ-PERS-047).
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
                slot.file = Some(
                    record
                        .file
                        .unwrap_or_else(|| format!("{}.state", record.id)),
                );
                slot.thumbnail = record.thumbnail;
            }
        }
        index
    }

    /// Reads the header of every state file and fills the slots from them. A slot whose file
    /// cannot be read stays empty.
    pub fn rebuild_from_files(&mut self, paths: &UserDataPaths) {
        let rom = self.rom.clone();
        for slot in &mut self.slots {
            let (state, thumbnail) = match rom.as_deref() {
                Some(rom) => (
                    paths.slot_state_file(rom, &slot.id),
                    paths.slot_thumbnail_file(rom, &slot.id),
                ),
                None => (
                    paths.legacy_slot_state_file(&slot.id),
                    paths.legacy_slot_thumbnail_file(&slot.id),
                ),
            };
            let Ok(bytes) = fs::read(&state) else {
                continue;
            };
            let Some(content) = save_state_file::try_read(&bytes) else {
                continue;
            };
            slot.game_id = content.header.game_id;
            slot.saved_at = content.header.saved_at;
            slot.file = state
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            slot.thumbnail = thumbnail
                .is_file()
                .then(|| thumbnail.file_name())
                .flatten()
                .map(|name| name.to_string_lossy().into_owned());
        }
    }

    /// Empties the slots whose game is gone or whose file is missing, deleting the files of the
    /// former. Returns how many slots were cleared (REQ-PERS-046).
    pub fn reconcile(
        &mut self,
        paths: &UserDataPaths,
        game_exists: impl Fn(&str) -> bool,
    ) -> usize {
        let mut cleared = 0;
        for slot in &mut self.slots {
            if !slot.is_occupied() {
                continue;
            }
            if !game_exists(&slot.game_id) {
                remove_file_if_present(&paths.legacy_slot_state_file(&slot.id));
                remove_file_if_present(&paths.legacy_slot_thumbnail_file(&slot.id));
            } else if paths.legacy_slot_state_file(&slot.id).is_file() {
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

    /// Writes the index of the shared slots of before, unless the root is read-only; once every
    /// state has gone to its game's files, the index file goes. The slots of a ROM have no index:
    /// their files are their index.
    ///
    /// # Errors
    /// The file could not be written.
    pub fn save(&self, paths: &UserDataPaths) -> std::io::Result<()> {
        if !paths.is_writable || self.rom.is_some() {
            return Ok(());
        }
        if !self.slots.iter().any(SaveSlotRecord::is_occupied) {
            remove_file_if_present(&paths.save_index_file);
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

/// Moves the states of `game_id` from the shared slots of before (`legacy`) to the files of its
/// ROM `rom`, keeping their slot, unless the ROM has a state there already; the slots moved are
/// emptied in `legacy`. Returns how many were moved.
pub fn move_legacy_states(
    paths: &UserDataPaths,
    legacy: &mut SaveSlotIndex,
    game_id: &str,
    rom: &str,
) -> usize {
    let mut moved = 0;
    let ids: Vec<String> = legacy
        .occupied_for(game_id)
        .into_iter()
        .map(|slot| slot.id)
        .collect();
    for id in ids {
        let target = paths.slot_state_file(rom, &id);
        if !target.exists() && fs::rename(paths.legacy_slot_state_file(&id), &target).is_ok() {
            let thumbnail = paths.legacy_slot_thumbnail_file(&id);
            if thumbnail.is_file() {
                let _ = fs::rename(&thumbnail, paths.slot_thumbnail_file(rom, &id));
            }
            legacy.vacate(&id);
            moved += 1;
        }
    }
    moved
}
