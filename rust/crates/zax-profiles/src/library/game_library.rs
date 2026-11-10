//! The game library (SPEC-03 REQ-PROF-064, REQ-PROF-065, SPEC-09 REQ-PERS-030 to REQ-PERS-035):
//! every built-in and user profile as a game, with the ROM chosen for it.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use super::search;
use super::store::{GameLibraryFile, GameLibraryRecord, RomPathRecord};
use crate::catalogue::{
    shown_title, BuiltInProfiles, GameIdentity, ProfileCatalogue, ProfileGeneration, ProfileOrigin,
    UserProfile, DEFAULT_THREE_DIMENSIONAL_TITLE, TWO_DIMENSIONAL_MODE_TITLE,
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

/// How the library is ordered (SPEC-09 REQ-PERS-033, as AMD-053 §3 point 8 amends it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySortOption {
    /// By title.
    Alphabetical,
    /// By last time played, most recent first.
    Recent,
    /// Favourites first, then the others, each group by title (AMD-053 REQ-UI-210).
    FavoritesFirst,
}

impl LibrarySortOption {
    /// The order the value of `ui.sort_option` names: 0 A-Z, 1 Recent, 2 Favorites first;
    /// anything else is A-Z.
    #[must_use]
    pub const fn of_setting(value: i32) -> Self {
        match value {
            1 => Self::Recent,
            2 => Self::FavoritesFirst,
            _ => Self::Alphabetical,
        }
    }

    /// The order the Load and Save lists follow (REQ-PERS-057): « Favorites first » is without
    /// effect on them, so the value 2 falls back to A-Z.
    #[must_use]
    pub const fn of_setting_for_slots(value: i32) -> Self {
        match value {
            1 => Self::Recent,
            _ => Self::Alphabetical,
        }
    }
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
    /// Whether the user marked the game as a favourite (AMD-053 REQ-UI-210, §3 point 11). False
    /// by default, and false for every entry of a file migrated from schema version 1.
    pub favorite: bool,
    /// The identity of the profile whose content the entry shows (AMD-065 REQ-PROF-086) and its
    /// generation (REQ-PROF-085).
    identity: GameIdentity,
    /// The identities of the profiles this entry has replaced (REQ-PROF-087). A third profile of
    /// the same game must still find the entry by what any of them carried: a built-in paired by
    /// its `Id` with a 2.1 profile of another name stays reachable under that name.
    replaced: Vec<GameIdentity>,
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
        identity: GameIdentity,
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
            favorite: false,
            identity,
            replaced: Vec::new(),
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

    /// The identity of the profile the entry shows (AMD-065 REQ-PROF-086).
    #[must_use]
    pub const fn identity(&self) -> &GameIdentity {
        &self.identity
    }

    /// The generation of the profile the entry shows (REQ-PROF-085).
    #[must_use]
    pub const fn generation(&self) -> ProfileGeneration {
        self.identity.generation()
    }

    /// REQ-PROF-086: whether this entry is the game `identity` designates — by what the profile
    /// it shows carries, or by what any profile it has replaced carried.
    #[must_use]
    pub fn designates(&self, identity: &GameIdentity) -> bool {
        self.identity.same_game(identity)
            || self
                .replaced
                .iter()
                .any(|replaced| replaced.same_game(identity))
    }

    /// REQ-PROF-087: the 2.1 profile `incoming` takes this entry over. The **identifier** stays —
    /// `rom.json`, the saves and the cover are keyed on it — and everything that is content
    /// comes from the new profile, its title included. What the entry has replaced so far is
    /// kept, so that a later profile of the same game still finds it.
    fn take_over(&mut self, incoming: Self) {
        let replaced = std::mem::replace(&mut self.identity, incoming.identity);
        if !self.replaced.contains(&replaced) {
            self.replaced.push(replaced);
        }
        self.title = incoming.title;
        self.source = incoming.source;
        self.profile_path = incoming.profile_path;
        self.origin = incoming.origin;
        self.is_2d_mode = incoming.is_2d_mode;
        self.is_default_three_dimensional = incoming.is_default_three_dimensional;
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
    ///
    /// A profile of generation 2.1 replaces the entry of the profile of an earlier generation of
    /// the same game (AMD-065 REQ-PROF-087). The built-in profiles are known here by the file
    /// names the catalogue gives them, which decides condition 2 of REQ-PROF-086 but not
    /// condition 1: the `Id` a built-in **file** carries is only known once that file is read,
    /// which is what [`Self::build_with_built_in_profiles`] is for. Between two user profiles
    /// both conditions are decided, their headers travelling with them.
    #[must_use]
    pub fn build(
        catalogue: &ProfileCatalogue,
        user_profiles: &[UserProfile],
        persisted: Option<&GameLibraryFile>,
        log: Option<&dyn ProfileLog>,
    ) -> Self {
        Self::build_with_built_in_profiles(
            catalogue,
            &BuiltInProfiles::default(),
            user_profiles,
            persisted,
            log,
        )
    }

    /// [`Self::build`] knowing what the built-in profile **files** carry, as
    /// `ProfileCatalogue::read_built_in_profiles` reads them: the `Id` of each is what pairs the
    /// 56 games whose 3dn and 3dh agree on an identifier and disagree on the file name
    /// (AMD-065 REQ-PROF-086 point 1).
    #[must_use]
    pub fn build_with_built_in_profiles(
        catalogue: &ProfileCatalogue,
        built_in: &BuiltInProfiles,
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
                GameIdentity::of_catalogue(entry, built_in.header(&entry.id)),
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
    ///
    /// A profile of generation 2.1 also takes over the entry of a profile of an earlier
    /// generation of the same game, whatever its identifier, and an earlier one adds nothing to a
    /// game a 2.1 profile already holds (AMD-065 REQ-PROF-087). This is the path the interface
    /// import (AMD-056 REQ-UI-214) and the import of a whole installation take, so that neither
    /// has to wait for a restart to show one entry per game.
    pub fn add_user_profile(
        &mut self,
        profile: &UserProfile,
        log: Option<&dyn ProfileLog>,
    ) -> Option<&GameLibraryEntry> {
        let log = log.unwrap_or(&NULL_PROFILE_LOG);
        let incoming = user_entry(profile);
        let same_id = self
            .entries
            .iter()
            .position(|entry| entry.id == profile.id && entry.source == GameProfileSource::User);
        if let Some(position) = same_id {
            let existing = &self.entries[position];
            // REQ-PROF-087: a 3dn does not take the entry of the 3dh of the same game over, even
            // when the two files agree on their `Id`, which is the case for 56 of the 101.
            if existing.generation().is_two_point_one() && !incoming.generation().is_two_point_one()
            {
                log.information(&replacement_message(&existing.id, existing, &incoming));
                return None;
            }
            if incoming.generation().is_two_point_one() && !existing.generation().is_two_point_one()
            {
                self.entries[position].take_over(incoming);
                return Some(&self.entries[position]);
            }
            // A profile imported into the user folder takes over the entry of a profile of the
            // same identifier found next to the executable.
            let existing = &mut self.entries[position];
            existing.profile_path.clone_from(&profile.file_path);
            existing.origin = profile.origin;
            existing.identity = incoming.identity;
            return Some(&self.entries[position]);
        }

        // Anything else — a new identifier, or one a built-in profile holds — follows the rules
        // of the build: REQ-PROF-087 first, then REQ-PERS-030's refusal.
        let position = self.add(incoming, log)?;
        Some(&self.entries[position])
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

    /// The games in the order of `option` (SPEC-09 REQ-PERS-033).
    #[must_use]
    pub fn sorted(&self, option: LibrarySortOption) -> Vec<&GameLibraryEntry> {
        let mut sorted: Vec<&GameLibraryEntry> = self.entries.iter().collect();
        sorted.sort_by(|left, right| match option {
            LibrarySortOption::Recent => compare_by_recent(left, right),
            LibrarySortOption::FavoritesFirst => compare_by_favorite(left, right),
            LibrarySortOption::Alphabetical => compare_by_title(left, right),
        });
        sorted
    }

    /// The enumeration of the games the interface shows: the order of `option`, the favourites
    /// filter of REQ-UI-210 and the title search of REQ-UI-211, an entry being kept when it
    /// satisfies **both**.
    ///
    /// `search` is the text typed; a blank text excludes nothing. `favorites_only` is whether
    /// `library.filter_favorites` is ticked.
    #[must_use]
    pub fn enumerate(
        &self,
        option: LibrarySortOption,
        search: &str,
        favorites_only: bool,
    ) -> Vec<&GameLibraryEntry> {
        self.sorted(option)
            .into_iter()
            .filter(|entry| {
                (!favorites_only || entry.favorite)
                    && search::matches(entry.display_title(), search)
            })
            .collect()
    }

    /// Marks or unmarks an entry as a favourite (REQ-UI-210, command `favorite.toggle`). Returns
    /// false when the identifier is unknown.
    pub fn set_favorite(&mut self, id: &str, favorite: bool) -> bool {
        self.find_mut(id).is_some_and(|entry| {
            entry.favorite = favorite;
            true
        })
    }

    /// Flips the favourite mark of `id` and returns its new value; none when the identifier is
    /// unknown.
    pub fn toggle_favorite(&mut self, id: &str) -> Option<bool> {
        let entry = self.find_mut(id)?;
        entry.favorite = !entry.favorite;
        Some(entry.favorite)
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
                    favorite: entry.favorite,
                })
                .collect(),
            ..GameLibraryFile::default()
        }
    }

    /// Adds `entry` and returns where it landed; none when it added nothing.
    fn add(&mut self, entry: GameLibraryEntry, log: &dyn ProfileLog) -> Option<usize> {
        // REQ-PROF-087 is asked first: a profile of generation 2.1 takes the entry of the same
        // game over whatever the identifiers, which AMD-065 puts before REQ-PERS-030. The test
        // below only fires when one of the two profiles is a 2.1, so nothing of the rule between
        // two profiles of an earlier generation moves.
        if let Some(position) = self.same_game_position(&entry) {
            return self.replace_same_game(position, entry, log);
        }
        if let Some(existing) = self.find(&entry.id) {
            // A built-in profile always wins; between user profiles the first one wins
            // (SPEC-09 REQ-PERS-030).
            log.warning(&format!(
                "duplicated profile identifier \"{}\": keeping the {} profile \"{}\" and \
                 ignoring \"{}\"",
                entry.id, existing.source, existing.title, entry.title
            ));
            return None;
        }
        self.entries.push(entry);
        Some(self.entries.len() - 1)
    }

    /// The entry `incoming` replaces, or is dropped for, under REQ-PROF-087: one that designates
    /// the same game (REQ-PROF-086) when **at least one of the two** is of generation 2.1.
    ///
    /// Between two profiles of an earlier generation nothing changes, which is what keeps the
    /// four profiles of AMD-039 §5 — a 2.0 whose Lua scripts differ from the built-in of the same
    /// name — each with their own entry (AMD-037, AMD-065 §2 last paragraph).
    fn same_game_position(&self, incoming: &GameLibraryEntry) -> Option<usize> {
        let one_is_2_1 = |existing: &GameLibraryEntry| {
            existing.generation().is_two_point_one() || incoming.generation().is_two_point_one()
        };
        self.entries
            .iter()
            .position(|existing| one_is_2_1(existing) && existing.designates(&incoming.identity))
    }

    /// REQ-PROF-087 on the entry at `position`, which `incoming` designates the same game as;
    /// `position` again when the incoming profile took the entry over, none when it added nothing.
    fn replace_same_game(
        &mut self,
        position: usize,
        incoming: GameLibraryEntry,
        log: &dyn ProfileLog,
    ) -> Option<usize> {
        let existing = &self.entries[position];
        // Several profiles of generation 2.1: the existing rules decide — the user profile wins
        // over the built-in, then the first read.
        let wins = if existing.generation().is_two_point_one() {
            incoming.generation().is_two_point_one()
                && existing.source == GameProfileSource::BuiltIn
                && incoming.source == GameProfileSource::User
        } else {
            incoming.generation().is_two_point_one()
        };
        if !wins {
            log.information(&replacement_message(&existing.id, existing, &incoming));
            return None;
        }
        log.information(&replacement_message(&existing.id, &incoming, existing));
        self.entries[position].take_over(incoming);
        Some(position)
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
            entry.favorite = record.favorite;
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
        GameIdentity::of_user(profile),
    )
}

/// What the log says of a replacement (AMD-065 REQ-PROF-087): the identifier the entry keeps,
/// which profile it is shown from, and which one adds none. Nothing is deleted or rewritten, and
/// the line says so.
fn replacement_message(
    entry_id: &str,
    kept: &GameLibraryEntry,
    dropped: &GameLibraryEntry,
) -> String {
    format!(
        "\"{}\" and \"{}\" are the same game: the entry \"{entry_id}\" shows the {} profile {} \
         and the other adds none; its file stays on the disk",
        kept.display_title(),
        dropped.display_title(),
        if kept.generation().is_two_point_one() {
            "2.1"
        } else {
            "earlier"
        },
        kept.profile_path.display()
    )
}

fn compare_by_title(left: &GameLibraryEntry, right: &GameLibraryEntry) -> Ordering {
    text::ordinal_ignore_case(left.display_title(), right.display_title())
        .then_with(|| right.last_played.cmp(&left.last_played))
        .then_with(|| text::ordinal(&left.id, &right.id))
}

/// « Favorites first »: the favourites, then the others, each group by title (AMD-053 §3 point 8).
fn compare_by_favorite(left: &GameLibraryEntry, right: &GameLibraryEntry) -> Ordering {
    right
        .favorite
        .cmp(&left.favorite)
        .then_with(|| compare_by_title(left, right))
}

fn compare_by_recent(left: &GameLibraryEntry, right: &GameLibraryEntry) -> Ordering {
    right
        .last_played
        .cmp(&left.last_played)
        .then_with(|| compare_by_title(left, right))
}
