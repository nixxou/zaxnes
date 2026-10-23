//! The game library and its persistence (SPEC-03 REQ-PROF-064, REQ-PROF-065).

mod game_library;
mod store;

pub use game_library::{GameLibrary, GameLibraryEntry, GameProfileSource, LibrarySortOption};
pub use store::{
    parse_rom_paths, GameLibraryFile, GameLibraryRecord, GameLibraryStore, RomPathRecord,
};
