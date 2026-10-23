//! What the loop needs to know about the games (SPEC-09 §3): which exist, where their ROM is,
//! their profile, and the bookkeeping a launch leaves behind.

use std::sync::Arc;

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
}

impl ResolvedGame {
    /// Whether the entry is one of the two generic ones, which play any ROM.
    #[must_use]
    pub const fn is_generic(&self) -> bool {
        self.is_2d_mode || self.is_default_three_dimensional
    }
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
}
