//! The resolved tiles of a "3dh" profile, kept between runs (AMD-062 REQ-PROF-082): one file per
//! profile identifier in the user data folder, read when the game is launched and written when it
//! stops.
//!
//! The cache is a convenience, never a source of truth: every pair it holds is verifiable (the
//! manifest string recomputes from the sixteen bytes), an unreadable or arbitrary file gives
//! nothing, and the profile file is never touched. `--no-tile-cache` turns it off, which is what
//! the determinism of REQ-PROF-080 is stated at — an empty resolved set to start from.

use std::fs;

use zax_profiles::hash::tile_cache;

use crate::user_data_paths::{write_atomic, UserDataPaths};

/// Reads and writes the resolved-tile cache files.
#[derive(Debug, Clone, Copy)]
pub struct TileCacheStore {
    enabled: bool,
}

impl TileCacheStore {
    /// A store that keeps the resolved tiles (`enabled`) or one that keeps nothing.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Whether anything is read or written at all.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// The text of the cache file of the profile `id`, empty when the store is off, the
    /// identifier is empty, or there is no readable file — all of which simply mean "nothing
    /// kept" (REQ-PROF-082).
    #[must_use]
    pub fn read(&self, paths: &UserDataPaths, id: &str) -> String {
        if !self.enabled || id.is_empty() {
            return String::new();
        }

        fs::read_to_string(paths.tile_cache_file(id)).unwrap_or_default()
    }

    /// Writes `contents` as the cache of the profile `id`. Returns whether a file was written; a
    /// store that is off, an unwritable root, an empty identifier and a cache with no entry
    /// write nothing.
    #[must_use]
    pub fn write(&self, paths: &UserDataPaths, id: &str, contents: &str) -> bool {
        if !self.enabled || id.is_empty() || !paths.is_writable || !tile_cache::has_entry(contents)
        {
            return false;
        }

        let file = paths.tile_cache_file(id);
        if let Some(folder) = file.parent() {
            if fs::create_dir_all(folder).is_err() {
                return false;
            }
        }

        // A cache that could not be written is no error: the tiles are found again from the ROM
        // and from the game at the next launch.
        write_atomic(&file, contents.as_bytes()).is_ok()
    }
}
