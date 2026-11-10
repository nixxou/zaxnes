//! What the loop needs to know about the games (SPEC-09 §3): which exist, where their ROM is,
//! their profile, and the bookkeeping a launch leaves behind.

use std::sync::Arc;

use zax_profiles::hash::raw_tile::RawTile;
use zax_profiles::hash::tiles::TileResolver;
use zax_profiles::profile::GameProfile;

/// A game resolved to what a launch needs.
#[derive(Debug, Clone)]
pub struct ResolvedGame {
    /// Identifier of the library entry.
    pub id: String,
    /// Title to show.
    pub title: String,
    /// The profile; the empty one when the entry has no file or it could not be read.
    pub profile: Arc<GameProfile>,
    /// Whether this is the "2D Mode" entry.
    pub is_2d_mode: bool,
    /// Whether this is the "3D by default (no profile)" entry.
    pub is_default_three_dimensional: bool,
    /// Which manifest strings of a "3dh" profile have their sixteen bytes; none for a 3dn
    /// profile (AMD-062 REQ-PROF-080). The catalogue keeps it with the profile, so that what a
    /// session resolved survives a relaunch of the same game.
    pub tiles: Option<TileResolver>,
}

impl ResolvedGame {
    /// Whether the entry is one of the two generic ones, which play any ROM.
    #[must_use]
    pub const fn is_generic(&self) -> bool {
        self.is_2d_mode || self.is_default_three_dimensional
    }

    /// Whether a "3dh" profile still has a cell waiting for its sixteen bytes (REQ-PROF-081).
    #[must_use]
    pub fn has_waiting_tiles(&self) -> bool {
        self.tiles
            .as_ref()
            .is_some_and(|tiles| !tiles.is_complete())
    }
}

/// How many manifest strings of a "3dh" profile have their sixteen bytes, out of how many
/// (AMD-062 REQ-PROF-078, logged at every launch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashTileCounts {
    /// Strings that have their bytes.
    pub resolved: usize,
    /// Strings of the manifest.
    pub total: usize,
}

/// The library as the loop sees it.
pub trait GameCatalog {
    /// The game `game_id`, or `None` when it is not in the library.
    fn resolve(&mut self, game_id: &str) -> Option<ResolvedGame>;

    /// Whether the library has the game.
    fn exists(&self, game_id: &str) -> bool;

    /// The ROM chosen for the game, if any.
    fn rom_path(&self, game_id: &str) -> Option<String>;

    /// The game whose ROM is `rom_path`.
    fn game_for_rom(&self, rom_path: &str) -> Option<String>;

    /// The "2D Mode" entry.
    fn two_dimensional_mode_id(&self) -> Option<String>;

    /// The "3D by default" entry.
    fn default_three_dimensional_id(&self) -> Option<String>;

    /// Records the ROM chosen for the game.
    fn set_rom_path(&mut self, game_id: &str, rom_path: &str);

    /// Records the instant the game was launched, which feeds the "Recent" sort.
    fn mark_played(&mut self, game_id: &str, unix_seconds: i64);

    /// Deletes the game with its profile file and its cover; false when it cannot be deleted.
    fn delete_game(&mut self, game_id: &str) -> bool;

    /// The title of the game.
    fn title(&self, game_id: &str) -> Option<String>;

    /// Whether the game may be deleted: not a built-in profile, not "2D Mode", not a profile of
    /// the program's own folder (REQ-UI-037).
    fn can_delete(&self, game_id: &str) -> bool;

    /// Sets the cover file of the game; false when the game does not exist.
    fn set_cover(&mut self, game_id: &str, file_name: Option<&str>) -> bool;

    /// The cover file of the game.
    fn cover(&self, game_id: &str) -> Option<String>;

    /// Forgets the profile of `game_id` read so far, so that the next [`GameCatalog::resolve`]
    /// reads its file again (the script reload of EDITOR-1). A catalogue that reads nothing
    /// has nothing to forget.
    fn forget_profile(&mut self, _game_id: &str) {}

    /// Builds again the profile file of `game_id` from its project of the editor mode
    /// (`editor_mode.rs`), before a script reload reads it: `None` when it has no project, else
    /// whether the build went through. A catalogue without files has no project.
    /// The profile file of `game_id`, when it has one (the generic entries have none).
    fn profile_file(&self, _game_id: &str) -> Option<std::path::PathBuf> {
        None
    }

    fn rebuild_from_project(&mut self, _game_id: &str) -> Option<Result<(), String>> {
        None
    }

    /// Whether the game is marked as a favourite (AMD-053 REQ-UI-210). False for a catalogue that
    /// has no library behind it.
    fn is_favorite(&self, _game_id: &str) -> bool {
        false
    }

    /// Flips the favourite mark of the game and persists it (`favorite.toggle`); returns its new
    /// value, or none when the game does not exist or the catalogue has no library.
    fn toggle_favorite(&mut self, _game_id: &str) -> Option<bool> {
        None
    }

    /// Gives the waiting tiles of a "3dh" profile what the cache of REQ-PROF-082 kept —
    /// `cache` is the text of its cache file, empty when there is none — and then what the ROM
    /// image holds (REQ-PROF-078), and rebuilds the patterns that gained a tile. Both happen
    /// **before** the patterns are registered, so that the first converted frame already has
    /// them.
    ///
    /// Returns how many manifest strings have their bytes out of how many, or none for a 3dn
    /// profile and for a catalogue that reads no profile file. Resolution is monotone, so
    /// calling it again on the same game only adds.
    fn prepare_hash_tiles(
        &mut self,
        _game_id: &str,
        _cache: &str,
        _rom_image: &[u8],
    ) -> Option<HashTileCounts> {
        None
    }

    /// Keeps what a session resolved while it ran (AMD-062 REQ-PROF-080): the pairs are applied
    /// to the profile the catalogue holds, so that a relaunch of the same game starts from them,
    /// and the text of the cache file of REQ-PROF-082 is returned for the host to write.
    ///
    /// None for a 3dn profile and for a catalogue that reads no profile file.
    fn absorb_hash_tiles(
        &mut self,
        _game_id: &str,
        _entries: &[(String, RawTile)],
    ) -> Option<String> {
        None
    }
}
