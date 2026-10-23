//! Where the program keeps its user data (SPEC-09 §2): the per-user folder, the portable
//! `userdata` folder next to the executable, or an explicit root, and the files inside it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

/// Name of the per-user folder.
pub const PRODUCT_NAME: &str = "Zaxnes";
/// Name of the folder of the previous product, whose content is migrated once (AMD-029 §2).
pub const LEGACY_PRODUCT_NAME: &str = "Sen";
/// A file of this name next to the executable switches to the portable mode.
pub const PORTABLE_MARKER_NAME: &str = "portable.txt";
/// Folder of the portable mode, next to the executable.
pub const PORTABLE_FOLDER_NAME: &str = "userdata";
/// Folder of the profiles, in the user root and next to the executable.
pub const PROFILES_FOLDER_NAME: &str = "profiles";

const CORRUPT_STAMP: &[FormatItem<'static>] =
    format_description!("[year][month][day]-[hour][minute][second]");

/// Where the root of the user data was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserDataRootKind {
    /// The per-user application data folder.
    LocalAppData,
    /// The `userdata` folder next to the executable, chosen by a `portable.txt` marker.
    Portable,
    /// A root given on the command line (`--userdata`).
    Explicit,
}

/// The folders and files of the user data.
#[derive(Debug, Clone)]
pub struct UserDataPaths {
    /// Root folder.
    pub root: PathBuf,
    /// How the root was chosen.
    pub kind: UserDataRootKind,
    /// `settings.json`.
    pub settings_file: PathBuf,
    /// `input.json`.
    pub input_file: PathBuf,
    /// `library.json`.
    pub library_file: PathBuf,
    /// Folder of the save states.
    pub saves_directory: PathBuf,
    /// Index of the save slots.
    pub save_index_file: PathBuf,
    /// Folder of the battery-backed RAM files.
    pub battery_directory: PathBuf,
    /// Folder of the user's profiles.
    pub profiles_directory: PathBuf,
    /// Folder of the executable.
    pub executable_directory: PathBuf,
    /// Folder of the profiles shipped with the program.
    pub program_profiles_directory: PathBuf,
    /// Folder of the cover art.
    pub covers_directory: PathBuf,
    /// Folder of the logs.
    pub logs_directory: PathBuf,
    /// Whether the root accepts writes (REQ-PERS-073).
    pub is_writable: bool,
    /// The legacy root this root was copied from, when it was.
    pub migrated_from: Option<PathBuf>,
    /// The notification text shown when a write is attempted on a read-only root.
    pub unwritable_message: Option<String>,
}

impl UserDataPaths {
    fn new(root: PathBuf, kind: UserDataRootKind, executable_directory: PathBuf) -> Self {
        let saves_directory = root.join("saves");
        Self {
            settings_file: root.join("settings.json"),
            input_file: root.join("input.json"),
            library_file: root.join("library.json"),
            save_index_file: saves_directory.join("index.json"),
            saves_directory,
            battery_directory: root.join("battery"),
            profiles_directory: root.join(PROFILES_FOLDER_NAME),
            program_profiles_directory: executable_directory.join(PROFILES_FOLDER_NAME),
            covers_directory: root.join("covers"),
            logs_directory: root.join("logs"),
            root,
            kind,
            executable_directory,
            is_writable: false,
            migrated_from: None,
            unwritable_message: None,
        }
    }

    /// The save state file of `slot_id`.
    #[must_use]
    pub fn slot_state_file(&self, slot_id: &str) -> PathBuf {
        self.saves_directory.join(format!("{slot_id}.state"))
    }

    /// The thumbnail of `slot_id`.
    #[must_use]
    pub fn slot_thumbnail_file(&self, slot_id: &str) -> PathBuf {
        self.saves_directory.join(format!("{slot_id}.png"))
    }

    /// The battery RAM file of the ROM whose fingerprint is `rom_sha1`.
    #[must_use]
    pub fn battery_file(&self, rom_sha1: &str) -> PathBuf {
        self.battery_directory.join(format!("{rom_sha1}.srm"))
    }

    /// Resolves the root, in this order: the explicit root, the portable folder when the marker
    /// is next to the executable, the per-user folder (migrating the legacy one first). The
    /// folders are then created and probed for writing.
    ///
    /// `executable_directory` and `local_app_data_directory` default to the running executable's
    /// folder and the platform's per-user data folder; tests pass their own.
    #[must_use]
    pub fn resolve(
        explicit_root: Option<&Path>,
        executable_directory: Option<&Path>,
        local_app_data_directory: Option<&Path>,
    ) -> Self {
        let base = executable_directory.map_or_else(default_executable_directory, Path::to_path_buf);
        let explicit_root = explicit_root.filter(|root| !root.as_os_str().is_empty());
        let mut paths = match explicit_root {
            Some(root) => Self::new(absolute(root), UserDataRootKind::Explicit, base),
            None if base.join(PORTABLE_MARKER_NAME).is_file() => Self::new(
                base.join(PORTABLE_FOLDER_NAME),
                UserDataRootKind::Portable,
                base,
            ),
            None => {
                let local = local_app_data_directory
                    .map(Path::to_path_buf)
                    .or_else(default_local_app_data)
                    .unwrap_or_else(|| base.clone());
                let mut paths = Self::new(
                    local.join(PRODUCT_NAME),
                    UserDataRootKind::LocalAppData,
                    base,
                );
                // Before `ensure_directories`, which would create the new root and make it look
                // as if it had always been there (AMD-029 §2).
                let legacy = local.join(LEGACY_PRODUCT_NAME);
                if try_copy_legacy_root(&legacy, &paths.root) {
                    paths.migrated_from = Some(legacy);
                }
                paths
            }
        };
        paths.ensure_directories();
        paths
    }

    /// Creates the folders and probes the root for writing, which sets
    /// [`is_writable`](Self::is_writable) (REQ-PERS-073).
    pub fn ensure_directories(&mut self) {
        self.is_writable = self.create_and_probe().is_ok();
        self.unwritable_message = (!self.is_writable)
            .then(|| format!("User data folder is not writable: {}", self.root.display()));
    }

    fn create_and_probe(&self) -> io::Result<()> {
        for folder in [
            &self.root,
            &self.saves_directory,
            &self.battery_directory,
            &self.profiles_directory,
            &self.covers_directory,
            &self.logs_directory,
        ] {
            fs::create_dir_all(folder)?;
        }
        let probe = self.root.join(".write-probe");
        fs::write(&probe, [])?;
        fs::remove_file(probe)
    }
}

/// Copies `legacy_root` to `new_root` when the new one does not exist yet (AMD-029 §2). Returns
/// whether it did. A half-copied root is still usable (what is missing takes its defaults, and
/// the old root is untouched), so a failure only reports false.
#[must_use]
pub fn try_copy_legacy_root(legacy_root: &Path, new_root: &Path) -> bool {
    if new_root.exists() || !legacy_root.is_dir() {
        return false;
    }
    // The same folder under two spellings would copy a tree onto itself.
    if same_folder(legacy_root, new_root) {
        return false;
    }
    copy_tree(legacy_root, new_root).is_ok()
}

fn same_folder(left: &Path, right: &Path) -> bool {
    let normal = |path: &Path| absolute(path).to_string_lossy().trim_end_matches(['/', '\\']).to_lowercase();
    normal(left) == normal(right)
}

fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if !target.exists() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Writes `contents` to `path` through a temporary file beside it, then a replace
/// (REQ-PERS-021).
///
/// # Errors
/// The folder or the file could not be written.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let directory = path.parent().filter(|parent| !parent.as_os_str().is_empty());
    if let Some(directory) = directory {
        fs::create_dir_all(directory)?;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let temporary = path.with_file_name(name);
    fs::write(&temporary, contents)?;
    fs::rename(temporary, path)
}

/// Sets an unreadable file aside as `<path>.corrupt-<timestamp>` (REQ-PERS-022). Nothing else
/// can be done when that fails: the caller carries on with defaults.
pub fn set_aside_corrupt_file(path: &Path, now: OffsetDateTime) {
    if !path.is_file() {
        return;
    }
    let Ok(stamp) = now.format(CORRUPT_STAMP) else {
        return;
    };
    let mut target = path.as_os_str().to_os_string();
    target.push(format!(".corrupt-{stamp}"));
    if fs::rename(path, PathBuf::from(target)).is_err() {
        // The caller carries on with defaults whether or not the file could be moved.
    }
}

/// Deletes `path` when it is a file. A file that cannot be removed stays, which every caller
/// tolerates (as C#).
pub fn remove_file_if_present(path: &Path) {
    if path.is_file() && fs::remove_file(path).is_err() {
        // Left in place: the next write replaces it.
    }
}

fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

fn default_executable_directory() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_local_app_data() -> Option<PathBuf> {
    let from_env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from);
    from_env("LOCALAPPDATA")
        .or_else(|| from_env("XDG_DATA_HOME"))
        .or_else(|| from_env("HOME").map(|home| home.join(".local").join("share")))
}
