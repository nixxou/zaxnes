//! The only way the profile library reaches files: the host implements
//! [`ProfileFileSystem`] and passes it in (the core never touches host paths itself).

use std::io;
use std::path::{Path, PathBuf};

/// File operations the catalogue, the library and the importer need.
pub trait ProfileFileSystem {
    /// True when `path` is an existing file.
    fn file_exists(&self, path: &Path) -> bool;

    /// True when `path` is an existing directory.
    fn directory_exists(&self, path: &Path) -> bool;

    /// Creates `path` and its missing parents; no error when it exists.
    ///
    /// # Errors
    ///
    /// The directory could not be created.
    fn create_directory(&self, path: &Path) -> io::Result<()>;

    /// The files directly inside `directory` whose name ends with `extension`, compared
    /// without regard to case, in no particular order; empty when the directory does not exist.
    ///
    /// # Errors
    ///
    /// The directory exists but could not be listed.
    fn list_files(&self, directory: &Path, extension: &str) -> io::Result<Vec<PathBuf>>;

    /// The directories directly inside `directory`, in no particular order; empty when the
    /// directory does not exist.
    ///
    /// # Errors
    ///
    /// The directory exists but could not be listed.
    fn list_directories(&self, directory: &Path) -> io::Result<Vec<PathBuf>>;

    /// The whole content of a file.
    ///
    /// # Errors
    ///
    /// The file does not exist or could not be read.
    fn read_all_bytes(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// The whole content of a file as UTF-8 text.
    ///
    /// # Errors
    ///
    /// The file does not exist, could not be read or is not text.
    fn read_all_text(&self, path: &Path) -> io::Result<String>;

    /// Writes a file, replacing it.
    ///
    /// # Errors
    ///
    /// The file could not be written.
    fn write_all_bytes(&self, path: &Path, contents: &[u8]) -> io::Result<()>;

    /// Writes a text file, replacing it.
    ///
    /// # Errors
    ///
    /// The file could not be written.
    fn write_all_text(&self, path: &Path, contents: &str) -> io::Result<()>;

    /// Deletes a file; no error when it does not exist.
    ///
    /// # Errors
    ///
    /// The file exists but could not be deleted.
    fn delete_file(&self, path: &Path) -> io::Result<()>;

    /// Copies a file.
    ///
    /// # Errors
    ///
    /// The source could not be read, or the destination exists and `overwrite` is false.
    fn copy_file(&self, source: &Path, destination: &Path, overwrite: bool) -> io::Result<()>;

    /// Moves a file.
    ///
    /// # Errors
    ///
    /// The source could not be moved, or the destination exists and `overwrite` is false.
    fn move_file(&self, source: &Path, destination: &Path, overwrite: bool) -> io::Result<()>;
}

/// Where the user's data lives (SPEC-09 REQ-PERS-072): profiles, the library file and the cover
/// art, under one injected root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDataLayout {
    root: PathBuf,
    profiles_directory: PathBuf,
    library_file: PathBuf,
    covers_directory: PathBuf,
}

impl UserDataLayout {
    /// Default name of the profiles folder.
    pub const DEFAULT_PROFILES_FOLDER_NAME: &'static str = "profiles";

    /// Name of the library file.
    pub const LIBRARY_FILE_NAME: &'static str = "library.json";

    /// Name of the covers folder.
    pub const COVERS_FOLDER_NAME: &'static str = "covers";

    /// The layout under `root`, with the profiles in `profiles_folder_name`.
    ///
    /// Q-008: SPEC-03 REQ-PROF-062 names `CustomProfiles`, SPEC-09 REQ-PERS-072 names
    /// `profiles`; the second is the normative layout of the reimplementation, and the name
    /// stays a parameter until R0 answers.
    #[must_use]
    pub fn new(root: &Path, profiles_folder_name: &str) -> Self {
        Self {
            root: root.to_path_buf(),
            profiles_directory: root.join(profiles_folder_name),
            library_file: root.join(Self::LIBRARY_FILE_NAME),
            covers_directory: root.join(Self::COVERS_FOLDER_NAME),
        }
    }

    /// The layout under `root` with the default profiles folder name.
    #[must_use]
    pub fn with_default_names(root: &Path) -> Self {
        Self::new(root, Self::DEFAULT_PROFILES_FOLDER_NAME)
    }

    /// The root folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The user profiles folder.
    #[must_use]
    pub fn profiles_directory(&self) -> &Path {
        &self.profiles_directory
    }

    /// The library file.
    #[must_use]
    pub fn library_file(&self) -> &Path {
        &self.library_file
    }

    /// The covers folder.
    #[must_use]
    pub fn covers_directory(&self) -> &Path {
        &self.covers_directory
    }

    /// Creates the root, the profiles folder and the covers folder.
    ///
    /// # Errors
    ///
    /// A folder could not be created.
    pub fn ensure_directories(&self, files: &dyn ProfileFileSystem) -> io::Result<()> {
        files.create_directory(&self.root)?;
        files.create_directory(&self.profiles_directory)?;
        files.create_directory(&self.covers_directory)
    }
}
