//! The cartridge RAM file of one ROM (SPEC-09 REQ-PERS-060): read when a game starts, written
//! when it stops and at most every 30 seconds while it runs, and only when the content changed.
//!
//! Lot MAPPERS-P: the same store keeps the flash save of a board whose save medium is its flash
//! chip (`<rom>.flash`, [`BatteryRamStore::for_flash`]), on the same schedule.

use std::fs;

use crate::user_data_paths::{
    remove_file_if_present, write_atomic, UserDataPaths, BATTERY_EXTENSION, FLASH_EXTENSION,
};

/// How often the RAM is written while a game runs, in real seconds.
pub const AUTO_SAVE_SECONDS: f64 = 30.0;

/// Reads and writes the battery RAM of the ROM that is attached.
#[derive(Debug)]
pub struct BatteryRamStore {
    /// The file's extension: `srm` for battery RAM, `flash` for a flash save.
    extension: &'static str,
    fingerprint: Option<String>,
    last_written: Vec<u8>,
    since_write: f64,
    /// How many times the RAM has been written, which the tests watch.
    pub writes: u32,
}

impl Default for BatteryRamStore {
    fn default() -> Self {
        Self::with_extension(BATTERY_EXTENSION)
    }
}

impl BatteryRamStore {
    /// Creates a store with no ROM attached.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store of flash saves (`<rom>.flash`) rather than battery RAM.
    #[must_use]
    pub const fn for_flash() -> Self {
        Self::with_extension(FLASH_EXTENSION)
    }

    const fn with_extension(extension: &'static str) -> Self {
        Self {
            extension,
            fingerprint: None,
            last_written: Vec::new(),
            since_write: 0.0,
            writes: 0,
        }
    }

    /// The fingerprint of the attached ROM.
    #[must_use]
    pub fn fingerprint(&self) -> Option<&str> {
        self.fingerprint.as_deref()
    }

    /// Attaches the ROM whose fingerprint is `rom_sha1` and returns the RAM saved for it, if any.
    /// A ROM whose save file cannot be read simply starts with empty RAM.
    pub fn attach(&mut self, paths: &UserDataPaths, rom_sha1: &str) -> Option<Vec<u8>> {
        self.fingerprint = Some(rom_sha1.to_owned());
        self.since_write = 0.0;
        self.last_written.clear();
        let data = fs::read(paths.save_file(rom_sha1, self.extension)).ok()?;
        self.last_written.clone_from(&data);
        Some(data)
    }

    /// Forgets the attached ROM.
    pub fn detach(&mut self) {
        self.fingerprint = None;
        self.last_written.clear();
        self.since_write = 0.0;
    }

    /// Writes `ram` when a ROM is attached, the root is writable and the content changed since
    /// the last write. Returns whether a file was written.
    pub fn flush(&mut self, paths: &UserDataPaths, ram: &[u8]) -> bool {
        let Some(fingerprint) = &self.fingerprint else {
            return false;
        };
        if ram.is_empty() || !paths.is_writable || ram == self.last_written.as_slice() {
            return false;
        }
        if write_atomic(&paths.save_file(fingerprint, self.extension), ram).is_err() {
            // The RAM is written again at the next flush (as C#, which ignores the failure).
            return false;
        }
        self.last_written.clear();
        self.last_written.extend_from_slice(ram);
        self.since_write = 0.0;
        self.writes += 1;
        true
    }

    /// Accounts for `real_seconds` and flushes once 30 s have passed since the last write.
    pub fn advance(&mut self, paths: &UserDataPaths, real_seconds: f64, ram: &[u8]) -> bool {
        if self.fingerprint.is_none() || ram.is_empty() {
            return false;
        }
        self.since_write += real_seconds;
        self.since_write >= AUTO_SAVE_SECONDS && self.flush(paths, ram)
    }

    /// [`Self::advance`] for a content that costs something to build (a flash save is assembled
    /// from the chip's changed sectors): `content` runs only when a write is due.
    pub fn advance_with(
        &mut self,
        paths: &UserDataPaths,
        real_seconds: f64,
        content: impl FnOnce() -> Vec<u8>,
    ) -> bool {
        if self.fingerprint.is_none() {
            return false;
        }
        self.since_write += real_seconds;
        if self.since_write < AUTO_SAVE_SECONDS {
            return false;
        }
        let data = content();
        if data.is_empty() {
            // Nothing to keep yet: wait for the next period.
            self.since_write = 0.0;
            return false;
        }
        self.flush(paths, &data)
    }

    /// Deletes the RAM file of `rom_sha1`. A file that cannot be removed stays (as C#).
    pub fn delete(paths: &UserDataPaths, rom_sha1: &str) {
        remove_file_if_present(&paths.battery_file(rom_sha1));
    }
}
