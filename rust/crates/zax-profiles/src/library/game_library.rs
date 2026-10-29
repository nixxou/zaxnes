//! The game library (SPEC-03 REQ-PROF-064, REQ-PROF-065, SPEC-09 REQ-PERS-030 to REQ-PERS-035):
//! every built-in and user profile as a game, with the ROM chosen for it.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use super::store::{GameLibraryFile, GameLibraryRecord, RomPathRecord};
use crate::catalogue::{
    shown_title, ProfileCatalogue, ProfileOrigin, UserProfile, DEFAULT_THREE_DIMENSIONAL_TITLE,
    TWO_DIMENSIONAL_MODE_TITLE,
};
use crate::format::{ProfileLog, NULL_PROFILE_LOG};
use crate::text;

/// Where a game's profile comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameProfileSource {
    /// The built-in catalogue.
    BuiltIn,
    /// A `.3dn` file.
    User,
}

impl std::fmt::Display for GameProfileSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::BuiltIn => "BuiltIn",
            Self::User => "User",
        })
    }
}

/// How the library is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySortOption {
    /// By title.
    Alphabetical,
    /// By last time played, most recent first.
    Recent,
}

/// One game of the library.
#[derive(Debug, Clone)]
pub struct GameLibraryEntry {
    id: String,
    /// Title; the user's own for a game with a custom title.
    pub title: String,
    source: GameProfileSource,
    profile_path: PathBuf,
    origin: ProfileOrigin,
    is_2d_mode: bool,
    is_default_three_dimensional: bool,
    /// The ROM chosen for this game; empty when none.
    pub rom_path: String,
    /// Last time played, in Unix seconds.
    pub last_played: i64,
    /// Cover art file, when there is one.
    pub cover: Option<String>,
}

impl GameLibraryEntry {
    /// The title kept for the game, `title` as the library file has it: a template's title
    /// the profile's file name does not say gives that name (`shown_title`), as it does when
    /// the profile is read.
    fn kept_title(&self, title: &str) -> String {
        if self.source == GameProfileSource::User {
            shown_title(title, &self.profile_path)
        } else {
            title.to_owned()
        }
    }

    fn new(
        id: &str,
        title: &str,
        source: GameProfileSource,
        profile_path: PathBuf,
        origin: ProfileOrigin,
    ) -> Self {
        Self {
            id: id.to_owned(),
            title: title.to_owned(),
            source,
            profile_path,
            origin,
            is_2d_mode: false,
            is_default_three_dimensional: false,
            rom_path: String::new(),
            last_played: 0,
            cover: None,
        }
    }

    /// Identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Where the profile comes from.
    #[must_use]
    pub fn source(&self) -> GameProfileSource {
        self.source
    }

    /// The profile file; empty for a generic entry.
    #[must_use]
    pub fn profile_path(&self) -> &Path {
        &self.profile_path
    }

    /// The folder a user profile was found in (AMD-033).
    #[must_use]
    pub fn origin(&self) -> ProfileOrigin {
        self.origin
    }

    /// True for a user profile found next to the executable.
    #[must_use]
    pub fn is_from_program_folder(&self) -> bool {
        self.source == GameProfileSource::User && self.origin == ProfileOrigin::ProgramFolder
    }

    /// True for the "2D Mode" entry.
    #[must_use]
    pub fn is_2d_mode(&self) -> bool {
        self.is_2d_mode
    }

    /// Q-065: true for the "3D by default" entry.
    #[must_use]
    pub fn is_default_three_dimensional(&self) -> bool {
        self.is_default_three_dimensional
    }

    /// True for an entry without a profile file of its own.
    #[must_use]
    pub fn is_generic(&self) -> bool {
        self.is_2d_mode || self.is_default_three_dimensional
    }

    /// The title shown: the fixed name of a generic entry whatever the custom title.
    #[must_use]
    pub fn display_title(&self) -> &str {
        if self.is_2d_mode {
            TWO_DIMENSIONAL_MODE_TITLE
        } else if self.is_default_three_dimensional {
            DEFAULT_THREE_DIMENSIONAL_TITLE
        } else {
            &self.title
        }
    }

    /// True when a ROM is chosen.
    #[must_use]
    pub fn has_rom(&self) -> bool {
        !self.rom_path.is_empty()
    }

    /// Q-074: only a profile of the user's own folder can be deleted.
    #[must_use]
    pub fn can_delete(&self) -> bool {
        self.source == GameProfileSource::User && self.origin == ProfileOrigin::User
    }
}

impl std::fmt::Display for GameLibraryEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} {}", self.id, self.display_title())
    }
}

/// The library: one entry per available profile.
#[derive(Debug, Clone, Default)]
pub struct GameLibrary {
    entries: Vec<GameLibraryEntry>,
}

impl GameLibrary {
    /// Builds the library from the built-in catalogue, the user profiles and the persisted
    /// library file. A built-in profile always wins over a user profile of the same identifier,
    /// and between user profiles the first one wins (SPEC-09 REQ-PERS-030); persisted entries
    /// that match no available profile are dropped.
    #[must_use]
    pub fn build(
        catalogue: &ProfileCatalogue,
        user_profiles: &[UserProfile],
        persisted: Option<&GameLibraryFile>,
        log: Option<&dyn ProfileLog>,
    ) -> Self {
        let log = log.unwrap_or(&NULL_PROFILE_LOG);
        let mut library = Self::default();

        for entry in catalogue.entries() {
            let mut game = GameLibraryEntry::new(
                &entry.id,
                &entry.title,
                GameProfileSource::BuiltIn,
                catalogue.profile_path(entry),
                ProfileOrigin::User,
            );
            game.is_2d_mode = entry.is_2d_mode();
            game.is_default_three_dimensional = entry.is_default_three_dimensional();
            library.add(game, log);
        }

        for profile in user_profiles {
            library.add(user_entry(profile), log);
        }

        if let Some(persisted) = persisted {
            library.apply(persisted, log);
        }
        library
    }

    /// The games, in the order they were added.
    #[must_use]
    pub fn entries(&self) -> &[GameLibraryEntry] {
        &self.entries
    }

    /// The game of `id`.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&GameLibraryEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// The game of `id`, to change its title, ROM, last time played or cover.
    pub fn find_mut(&mut self, id: &str) -> Option<&mut GameLibraryEntry> {
        self.entries.iter_mut().find(|entry| entry.id == id)
    }

    /// Adds a user profile as a game. A profile of an identifier a built-in profile holds is
    /// refused (none); one of an identifier another user profile holds takes over that entry
    /// (AMD-033 §2: the user folder wins).
    pub fn add_user_profile(
        &mut self,
        profile: &UserProfile,
        log: Option<&dyn ProfileLog>,
    ) -> Option<&GameLibraryEntry> {
        let position = self.entries.iter().position(|entry| entry.id == profile.id);
        if let Some(position) = position {
            let existing = &mut self.entries[position];
            if existing.source != GameProfileSource::User {
                return None;
            }
            // A profile imported into the user folder takes over the entry of a profile of the
            // same identifier found next to the executable.
            existing.profile_path.clone_from(&profile.file_path);
            existing.origin = profile.origin;
            return Some(&self.entries[position]);
        }

        self.add(user_entry(profile), log.unwrap_or(&NULL_PROFILE_LOG));
        self.entries.last()
    }

    /// Sets the ROM of game `id`; false when there is no such game.
    pub fn set_rom_path(&mut self, id: &str, rom_path: &str) -> bool {
        self.find_mut(id).is_some_and(|entry| {
            rom_path.clone_into(&mut entry.rom_path);
            true
        })
    }

    /// Records that game `id` was played at `unix_seconds`; false when there is no such game.
    pub fn mark_played(&mut self, id: &str, unix_seconds: i64) -> bool {
        self.find_mut(id).is_some_and(|entry| {
            entry.last_played = unix_seconds;
            true
        })
    }

    /// Removes game `id`; false when there is no such game or it cannot be deleted.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(position) = self
            .entries
            .iter()
            .position(|entry| entry.id == id && entry.can_delete())
        else {
            return false;
        };
        self.entries.remove(position);
        true
    }

    /// The games in the order of `option`.
    #[must_use]
    pub fn sorted(&self, option: LibrarySortOption) -> Vec<&GameLibraryEntry> {
        let mut sorted: Vec<&GameLibraryEntry> = self.entries.iter().collect();
        sorted.sort_by(|left, right| match option {
            LibrarySortOption::Recent => compare_by_recent(left, right),
            LibrarySortOption::Alphabetical => compare_by_title(left, right),
        });
        sorted
    }

    /// Applies the ROM paths of the original's `rom.json` (REQ-PERS-025); returns how many
    /// matched a game.
    pub fn apply_rom_paths(&mut self, records: &[RomPathRecord]) -> usize {
        let mut applied = 0;
        for record in records {
            let Some(entry) = self.find_mut(&record.id) else {
                continue;
            };
            entry.rom_path.clone_from(&record.rom_path);
            entry.last_played = record.last_time;
            if !record.title.is_empty() {
                entry.title = entry.kept_title(&record.title);
            }
            applied += 1;
        }
        applied
    }

    /// The library file to persist.
    #[must_use]
    pub fn to_file(&self) -> GameLibraryFile {
        GameLibraryFile {
            games: self
                .entries
                .iter()
                .map(|entry| GameLibraryRecord {
                    id: Some(entry.id.clone()),
                    title: Some(entry.title.clone()),
                    rom_path: Some(entry.rom_path.clone()),
                    last_played: entry.last_played,
                    cover: entry.cover.clone(),
                })
                .collect(),
            ..GameLibraryFile::default()
        }
    }

    fn add(&mut self, entry: GameLibraryEntry, log: &dyn ProfileLog) {
        if let Some(existing) = self.find(&entry.id) {
            // A built-in profile always wins; between user profiles the first one wins
            // (SPEC-09 REQ-PERS-030).
            log.warning(&format!(
                "duplicated profile identifier \"{}\": keeping the {} profile \"{}\" and \
                 ignoring \"{}\"",
                entry.id, existing.source, existing.title, entry.title
            ));
            return;
        }
        self.entries.push(entry);
    }

    fn apply(&mut self, persisted: &GameLibraryFile, log: &dyn ProfileLog) {
        for record in &persisted.games {
            let Some(id) = &record.id else {
                continue;
            };
            let Some(entry) = self.find_mut(id) else {
                log.information(&format!(
                    "library entry \"{id}\" matches no available profile and has been dropped"
                ));
                continue;
            };
            if let Some(title) = record.title.as_deref().filter(|title| !title.is_empty()) {
                entry.title = entry.kept_title(title);
            }
            entry.rom_path = record.rom_path.clone().unwrap_or_default();
            entry.last_played = record.last_played;
            entry.cover.clone_from(&record.cover);
        }
    }
}

fn user_entry(profile: &UserProfile) -> GameLibraryEntry {
    GameLibraryEntry::new(
        &profile.id,
        &profile.title,
        GameProfileSource::User,
        profile.file_path.clone(),
        profile.origin,
    )
}

fn compare_by_title(left: &GameLibraryEntry, right: &GameLibraryEntry) -> Ordering {
    text::ordinal_ignore_case(left.display_title(), right.display_title())
        .then_with(|| right.last_played.cmp(&left.last_played))
        .then_with(|| text::ordinal(&left.id, &right.id))
}

fn compare_by_recent(left: &GameLibraryEntry, right: &GameLibraryEntry) -> Ordering {
    right
        .last_played
        .cmp(&left.last_played)
        .then_with(|| compare_by_title(left, right))
}
