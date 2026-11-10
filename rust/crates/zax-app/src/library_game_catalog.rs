//! The game catalogue over the profile library (SPEC-09 §3): the built-in entries, the profiles
//! of the user's folder and of the program's folder, and what `library.json` remembers.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use time::OffsetDateTime;
use zax_profiles::catalogue::{
    built_in, import_messages, ProfileFolders, ProfileImportOutcome, ProfileImportPlan,
    ProfileImportResult, ProfileOrigin, UserProfile, UserProfileFolder,
};
use zax_profiles::format::{self, ProfileLog};
use zax_profiles::hash::raw_tile::RawTile;
use zax_profiles::hash::tile_cache;
use zax_profiles::hash::tiles::TileResolver;
use zax_profiles::hash::{self};
use zax_profiles::import::{ThreeDSenImportError, ThreeDSenImportSummary, ThreeDSenImporter};
use zax_profiles::library::{GameLibrary, GameLibraryEntry, GameLibraryStore, GameProfileSource};
use zax_profiles::profile::GameProfile;

use crate::emulation::game_catalog::{GameCatalog, HashTileCounts, ResolvedGame};
use crate::neutral_assets;
use crate::persistence::game_art;
use crate::physical_file_system::PhysicalProfileFileSystem;
use crate::user_data_paths::UserDataPaths;

/// The text sink the catalogue logs to.
pub type LogSink = Box<dyn Fn(&str)>;

/// What a 3dSen import reports: the summary when it ran, and the message to show.
#[derive(Debug, Clone)]
pub struct ThreeDSenImportReport {
    /// The summary of the profiles written, when the import ran.
    pub summary: Option<ThreeDSenImportSummary>,
    /// The message to show the user.
    pub message: String,
}

struct ProfileLogAdapter<'a>(&'a dyn Fn(&str));

impl ProfileLog for ProfileLogAdapter<'_> {
    fn information(&self, message: &str) {
        (self.0)(message);
    }

    fn warning(&self, message: &str) {
        (self.0)(message);
    }

    fn error(&self, message: &str) {
        (self.0)(message);
    }
}

/// A profile read from a file, with the tiles a "3dh" one is still waiting for.
///
/// The two travel together because a resolution changes both: the strings that have their bytes,
/// and the patterns whose cells hold them (AMD-062 REQ-PROF-080).
#[derive(Debug, Clone)]
struct LoadedProfile {
    profile: Arc<GameProfile>,
    tiles: Option<TileResolver>,
}

impl LoadedProfile {
    fn empty() -> Self {
        Self {
            profile: Arc::new(GameProfile::create_empty()),
            tiles: None,
        }
    }
}

/// The library, its file, and the profiles read so far.
pub struct LibraryGameCatalog {
    library: GameLibrary,
    paths: UserDataPaths,
    log: LogSink,
    profiles: HashMap<String, LoadedProfile>,
    /// The bytes of the built-in profile files: a user copy of one adds no game (AMD-037).
    built_in_contents: Vec<Vec<u8>>,
}

impl LibraryGameCatalog {
    /// Builds the library from the asset folder's catalogue (or the built-in one), the profile
    /// folders and `library.json`.
    pub fn open(asset_root: Option<&Path>, paths: &UserDataPaths, log: LogSink) -> Self {
        let catalogue = asset_root
            .and_then(|root| match neutral_assets::load_catalogue(root) {
                Ok(catalogue) => Some(catalogue),
                Err(error) => {
                    log(&format!("the profile catalogue could not be read: {error}"));
                    None
                }
            })
            // AMD-024 REQ-PUB-001 (Q-065): with no asset folder the catalogue is the one written
            // in code, "2D Mode" and "3D by default (no profile)", neither of which has a file.
            .unwrap_or_else(built_in::create);
        let adapter = ProfileLogAdapter(&*log);
        let files = PhysicalProfileFileSystem;
        let persisted = GameLibraryStore::new(&files, Some(&adapter))
            .load(&paths.library_file, OffsetDateTime::now_utc());
        // AMD-033 §1: the user folder and the read-only "profiles" folder next to the
        // executable, in every mode.
        // Read once: the bytes a user copy of a built-in profile is recognised by (AMD-037) and
        // the `Id` each built-in file carries, which pairs it with the 2.1 profile of the same
        // game (AMD-065 REQ-PROF-086 point 1).
        let built_in = catalogue.read_built_in_profiles(&files);
        let user_profiles = match profile_folders(&files, paths, &adapter)
            .scan_excluding(&catalogue.ids(), built_in.contents())
        {
            Ok(profiles) => profiles,
            Err(error) => {
                log(&format!(
                    "the user profile folder could not be read: {error}"
                ));
                Vec::new()
            }
        };
        let library = GameLibrary::build_with_built_in_profiles(
            &catalogue,
            &built_in,
            &user_profiles,
            persisted.as_ref(),
            Some(&adapter),
        );
        let mut catalog = Self::from_library(library, paths.clone(), log);
        catalog.built_in_contents = built_in.into_contents();
        catalog
    }

    /// A catalogue over an already built library.
    #[must_use]
    pub fn from_library(library: GameLibrary, paths: UserDataPaths, log: LogSink) -> Self {
        Self {
            library,
            paths,
            log,
            profiles: HashMap::new(),
            built_in_contents: Vec::new(),
        }
    }

    /// The library.
    #[must_use]
    pub const fn library(&self) -> &GameLibrary {
        &self.library
    }

    /// The library, for editing.
    pub fn library_mut(&mut self) -> &mut GameLibrary {
        &mut self.library
    }

    /// Imports the `.3dn` file at `path` into the user folder and the library: the reading of
    /// [`Self::inspect_profile_import`] followed at once by the writing of
    /// [`Self::apply_profile_import`].
    pub fn import_profile(&mut self, path: &Path) -> ProfileImportResult {
        let plan = self.inspect_profile_import(path);
        self.apply_profile_import(&plan)
    }

    /// Reads a chosen `.3dn` file and says what importing it would do, **without writing
    /// anything** (AMD-056 REQ-UI-214): the interface needs the outcome to choose its dialogue,
    /// and a replacement must not happen before the dialogue is answered.
    ///
    /// A file that cannot be **read** is the third cause of REQ-UI-084, as it was before; only a
    /// failure to **write**, which can happen after the confirmation, carries the message of its
    /// own error ([`Self::apply_profile_import`]).
    pub fn inspect_profile_import(&self, path: &Path) -> ProfileImportPlan {
        // Only the profiles of the user folder: the import replaces the file of the entry it
        // takes over, and nothing next to the executable is ever deleted (AMD-033 §1). A profile
        // of the program folder simply loses its entry to the imported one.
        //
        // AMD-056 REQ-UI-214 names the entry by its *displayed* title, the one a rename changed,
        // which is what `GameLibraryEntry::title` holds.
        let known: Vec<UserProfile> = self
            .library
            .entries()
            .iter()
            .filter(|entry| {
                entry.source() == GameProfileSource::User && entry.origin() == ProfileOrigin::User
            })
            .map(user_profile_of)
            .collect();
        let built_in: Vec<&str> = self
            .library
            .entries()
            .iter()
            .filter(|entry| entry.source() == GameProfileSource::BuiltIn)
            .map(GameLibraryEntry::id)
            .collect();
        let files = PhysicalProfileFileSystem;
        let adapter = ProfileLogAdapter(&*self.log);
        let folder = UserProfileFolder::new(
            &files,
            &self.paths.profiles_directory,
            Some(&adapter),
            ProfileOrigin::User,
        );
        folder
            .inspect(path, &built_in, &known)
            .unwrap_or_else(|_| ProfileImportPlan {
                outcome: ProfileImportOutcome::Rejected,
                source_path: path.to_path_buf(),
                message: import_messages::obsolete_format(path),
                id: String::new(),
                title: String::new(),
                header: format::ProfileHeader::default(),
                replaced: None,
                destination: std::path::PathBuf::new(),
            })
    }

    /// Carries out a plan of [`Self::inspect_profile_import`]: the file is copied into the user's
    /// profiles and the library gains — or updates — its entry, without a restart.
    ///
    /// A write that fails carries the message of its own error, which REQ-UI-214 shows in the
    /// refusal dialogue: the third text of REQ-UI-084 would say nothing true there, the file
    /// having been readable and well formed.
    pub fn apply_profile_import(&mut self, plan: &ProfileImportPlan) -> ProfileImportResult {
        let files = PhysicalProfileFileSystem;
        let adapter = ProfileLogAdapter(&*self.log);
        let folder = UserProfileFolder::new(
            &files,
            &self.paths.profiles_directory,
            Some(&adapter),
            ProfileOrigin::User,
        );
        let result = match folder.apply(plan) {
            Ok(result) => result,
            Err(error) => {
                (self.log)(&format!(
                    "the profile import of '{}' failed: {error}",
                    plan.source_path.display()
                ));
                return ProfileImportResult {
                    outcome: ProfileImportOutcome::Rejected,
                    profile: None,
                    message: error.to_string(),
                };
            }
        };
        if let (ProfileImportOutcome::Added | ProfileImportOutcome::Replaced, Some(imported)) =
            (result.outcome, &result.profile)
        {
            self.library.add_user_profile(imported, Some(&adapter));
            // A replaced profile must be read again rather than served from the cache.
            self.profiles.remove(&imported.id);
            self.save();
        }
        result
    }

    /// Imports the profiles of a 3dSen installation or bundle into the user folder and the
    /// library.
    pub fn import_from_three_d_sen(
        &mut self,
        installation_or_bundle_path: &Path,
    ) -> ThreeDSenImportReport {
        if !self.paths.is_writable {
            // SPEC-09 REQ-PERS-073: with no writable root there is nowhere to put the profiles.
            return ThreeDSenImportReport {
                summary: None,
                message: self
                    .paths
                    .unwritable_message
                    .clone()
                    .unwrap_or_else(|| "the user folder does not accept writes.".to_owned()),
            };
        }
        let files = PhysicalProfileFileSystem;
        let importer = ThreeDSenImporter::new(&files, &self.paths.profiles_directory, None);
        let summary = match importer.import_from(installation_or_bundle_path) {
            Ok(summary) => summary,
            Err(error) => {
                let message = match &error {
                    ThreeDSenImportError::Format(format) => format.message().to_owned(),
                    ThreeDSenImportError::Io(io) => io.to_string(),
                };
                (self.log)(&format!("the 3dSen import failed: {message}"));
                return ThreeDSenImportReport {
                    summary: None,
                    message,
                };
            }
        };
        self.add_imported_profiles();
        let message = summary.message();
        ThreeDSenImportReport {
            summary: Some(summary),
            message,
        }
    }

    fn add_imported_profiles(&mut self) {
        let files = PhysicalProfileFileSystem;
        let adapter = ProfileLogAdapter(&*self.log);
        let profiles = match profile_folders(&files, &self.paths, &adapter)
            .scan_excluding(&[], &self.built_in_contents)
        {
            Ok(profiles) => profiles,
            Err(error) => {
                (self.log)(&format!(
                    "the user profile folder could not be read: {error}"
                ));
                return;
            }
        };
        let mut added = false;
        for profile in &profiles {
            if self
                .library
                .add_user_profile(profile, Some(&adapter))
                .is_some()
            {
                self.profiles.remove(&profile.id);
                added = true;
            }
        }
        if added {
            self.save();
        }
    }

    /// Writes `library.json`, unless the root is read-only.
    pub fn save(&self) {
        if !self.paths.is_writable {
            return;
        }
        let files = PhysicalProfileFileSystem;
        let adapter = ProfileLogAdapter(&*self.log);
        let mut file = self.library.to_file();
        if let Err(error) =
            GameLibraryStore::new(&files, Some(&adapter)).save(&self.paths.library_file, &mut file)
        {
            (self.log)(&format!("library.json could not be written: {error}"));
        }
    }

    fn load_profile(&self, entry: &GameLibraryEntry) -> LoadedProfile {
        let path = entry.profile_path();
        if path.as_os_str().is_empty() || !path.is_file() {
            return LoadedProfile::empty();
        }
        match std::fs::read(path) {
            Ok(bytes) => match format::load(&bytes, None, Some(&path.to_string_lossy())) {
                Ok(document) => {
                    // AMD-062: a "3dh" profile carries the strings its tiles name; nothing is
                    // resolved yet, the cache and the ROM come at the launch (REQ-PROF-080).
                    let tiles = document.hash_state().map(|state| state.tiles.clone());
                    return LoadedProfile {
                        profile: Arc::new(document.profile),
                        tiles,
                    };
                }
                // REQ-PROF-033: a refused profile falls back to the empty profile rather than
                // stopping the launch.
                Err(error) => (self.log)(&format!(
                    "{}: {}; falling back to the empty profile.",
                    entry.display_title(),
                    error.message
                )),
            },
            Err(error) => (self.log)(&format!("{}: {error}", entry.display_title())),
        }
        LoadedProfile::empty()
    }

    /// The profile of `game_id` as it has been read, reading its file on first need.
    fn loaded(&mut self, game_id: &str) -> Option<&mut LoadedProfile> {
        if !self.profiles.contains_key(game_id) {
            let entry = self.library.find(game_id)?;
            let loaded = self.load_profile(entry);
            self.profiles.insert(game_id.to_owned(), loaded);
        }

        self.profiles.get_mut(game_id)
    }
}

fn profile_folders<'a>(
    files: &'a PhysicalProfileFileSystem,
    paths: &UserDataPaths,
    log: &'a dyn ProfileLog,
) -> ProfileFolders<'a> {
    ProfileFolders::new(
        files,
        &paths.profiles_directory,
        Some(&paths.program_profiles_directory),
        Some(log),
    )
}

fn user_profile_of(entry: &GameLibraryEntry) -> UserProfile {
    UserProfile {
        id: entry.id().to_owned(),
        title: entry.title.clone(),
        file_path: entry.profile_path().to_path_buf(),
        header: format::ProfileHeader::default(),
        origin: ProfileOrigin::User,
    }
}

impl GameCatalog for LibraryGameCatalog {
    fn resolve(&mut self, game_id: &str) -> Option<ResolvedGame> {
        let loaded = self.loaded(game_id)?.clone();
        let entry = self.library.find(game_id)?;
        Some(ResolvedGame {
            id: entry.id().to_owned(),
            title: entry.display_title().to_owned(),
            profile: loaded.profile,
            is_2d_mode: entry.is_2d_mode(),
            is_default_three_dimensional: entry.is_default_three_dimensional(),
            tiles: loaded.tiles,
        })
    }

    fn exists(&self, game_id: &str) -> bool {
        self.library.find(game_id).is_some()
    }

    fn prepare_hash_tiles(
        &mut self,
        game_id: &str,
        cache: &str,
        rom_image: &[u8],
    ) -> Option<HashTileCounts> {
        let loaded = self.loaded(game_id)?;
        let tiles = loaded.tiles.as_mut()?;
        if !tiles.is_complete() {
            // REQ-PROF-082 then REQ-PROF-078: what was kept, then what the ROM holds. An
            // unreadable, truncated or arbitrary cache gives nothing and is no error.
            let kept = tile_cache::read(cache);
            tiles.apply_cache(kept.iter().map(|(key, raw)| (key.as_str(), *raw)));
            tiles.resolve_from_rom(rom_image);
            hash::tiles::apply(Arc::make_mut(&mut loaded.profile), tiles);
        }

        Some(HashTileCounts {
            resolved: tiles.resolved_count(),
            total: tiles.resolved_count() + tiles.waiting_count(),
        })
    }

    fn absorb_hash_tiles(
        &mut self,
        game_id: &str,
        entries: &[(String, RawTile)],
    ) -> Option<String> {
        let loaded = self.loaded(game_id)?;
        let tiles = loaded.tiles.as_mut()?;
        // Every pair is verifiable, and `apply_cache` is what verifies it (REQ-PROF-082): what
        // the session resolved goes through the same gate as what a file holds.
        let added = tiles.apply_cache(entries.iter().map(|(key, raw)| (key.as_str(), *raw)));
        if added > 0 {
            hash::tiles::apply(Arc::make_mut(&mut loaded.profile), tiles);
        }

        Some(tile_cache::write(tiles.entries()))
    }

    fn forget_profile(&mut self, game_id: &str) {
        self.profiles.remove(game_id);
    }

    fn profile_file(&self, game_id: &str) -> Option<std::path::PathBuf> {
        let entry = self.library.find(game_id)?;
        let path = entry.profile_path();
        (!entry.is_generic() && path.is_file()).then(|| path.to_path_buf())
    }

    fn rebuild_from_project(&mut self, game_id: &str) -> Option<Result<(), String>> {
        let entry = self.library.find(game_id)?;
        if entry.source() != GameProfileSource::User {
            return None;
        }
        let folder = crate::editor_mode::folder(&self.paths.root);
        let project = crate::editor_mode::project_of(&folder, game_id)?;
        let file = entry.profile_path().to_path_buf();
        Some(
            crate::editor_mode::build_project(&project).and_then(|built| {
                std::fs::write(&file, &built.bytes)
                    .map_err(|error| format!("{}: {error}", file.display()))
            }),
        )
    }

    fn rom_path(&self, game_id: &str) -> Option<String> {
        self.library
            .find(game_id)
            .filter(|entry| entry.has_rom())
            .map(|entry| entry.rom_path.clone())
    }

    fn game_for_rom(&self, rom_path: &str) -> Option<String> {
        let wanted = full_path(rom_path);
        self.library
            .entries()
            .iter()
            .find(|entry| entry.has_rom() && full_path(&entry.rom_path) == wanted)
            .map(|entry| entry.id().to_owned())
    }

    fn two_dimensional_mode_id(&self) -> Option<String> {
        self.library
            .entries()
            .iter()
            .find(|entry| entry.is_2d_mode())
            .map(|entry| entry.id().to_owned())
    }

    fn default_three_dimensional_id(&self) -> Option<String> {
        self.library
            .entries()
            .iter()
            .find(|entry| entry.is_default_three_dimensional())
            .map(|entry| entry.id().to_owned())
    }

    fn set_rom_path(&mut self, game_id: &str, rom_path: &str) {
        if self.library.set_rom_path(game_id, rom_path) {
            self.save();
        }
    }

    fn mark_played(&mut self, game_id: &str, unix_seconds: i64) {
        if self.library.mark_played(game_id, unix_seconds) {
            self.save();
        }
    }

    fn delete_game(&mut self, game_id: &str) -> bool {
        // A built-in profile and the "2D Mode" entry must not be deletable (REQ-UI-037), and
        // neither is a profile of the folder next to the executable: deleting a game deletes its
        // file, and that folder is read-only for Zaxnes (AMD-033 §1, Q-074).
        let Some(entry) = self
            .library
            .find(game_id)
            .filter(|entry| entry.can_delete())
        else {
            return false;
        };
        let title = entry.title.clone();
        let cover = entry.cover.clone();
        let path = entry.profile_path().to_path_buf();
        let id = entry.id().to_owned();
        if !path.as_os_str().is_empty() {
            let files = PhysicalProfileFileSystem;
            let folder = UserProfileFolder::new(
                &files,
                &self.paths.profiles_directory,
                None,
                ProfileOrigin::User,
            );
            let profile = UserProfile {
                id: id.clone(),
                title: title.clone(),
                file_path: path,
                header: format::ProfileHeader::default(),
                origin: ProfileOrigin::User,
            };
            if let Err(error) = folder.delete(&profile) {
                (self.log)(&format!(
                    "the profile file of '{game_id}' could not be deleted: {error}"
                ));
            }
        }
        game_art::delete_cover(&self.paths, &title, &id, cover.as_deref());
        if !self.library.remove(game_id) {
            return false;
        }
        self.profiles.remove(game_id);
        self.save();
        (self.log)(&format!(
            "game '{game_id}' deleted with its profile file and its cover."
        ));
        true
    }

    fn title(&self, game_id: &str) -> Option<String> {
        self.library
            .find(game_id)
            .map(|entry| entry.display_title().to_owned())
    }

    fn can_delete(&self, game_id: &str) -> bool {
        self.library
            .find(game_id)
            .is_some_and(GameLibraryEntry::can_delete)
    }

    fn set_cover(&mut self, game_id: &str, file_name: Option<&str>) -> bool {
        let Some(entry) = self.library.find_mut(game_id) else {
            return false;
        };
        entry.cover = file_name.map(str::to_owned);
        self.save();
        true
    }

    fn cover(&self, game_id: &str) -> Option<String> {
        self.library
            .find(game_id)
            .and_then(|entry| entry.cover.clone())
    }

    fn is_favorite(&self, game_id: &str) -> bool {
        self.library
            .find(game_id)
            .is_some_and(|entry| entry.favorite)
    }

    fn toggle_favorite(&mut self, game_id: &str) -> Option<bool> {
        let favorite = self.library.toggle_favorite(game_id)?;
        self.save();
        Some(favorite)
    }
}

fn full_path(path: &str) -> String {
    std::path::absolute(path)
        .unwrap_or_else(|_| path.into())
        .to_string_lossy()
        .to_lowercase()
}
