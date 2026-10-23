//! The folder of neutral assets that may ship beside the program (AMD-024 REQ-PUB-001).

use std::path::{Path, PathBuf};

use zax_profiles::catalogue::{ProfileCatalogue, ProfileCatalogueError, FULL_EDITION};

use crate::physical_file_system::PhysicalProfileFileSystem;

/// Default name of the folder.
pub const FOLDER_NAME: &str = "assets-neutral";
/// Name of the catalogue file inside it.
pub const CATALOGUE_FILE_NAME: &str = "profiles.json";

/// The absolute path of the asset folder named by `hint`, if it exists.
#[must_use]
pub fn locate(hint: Option<&Path>) -> Option<PathBuf> {
    let hint = hint.filter(|hint| !hint.as_os_str().is_empty() && hint.is_dir())?;
    Some(std::path::absolute(hint).unwrap_or_else(|_| hint.to_path_buf()))
}

/// Reads the profile catalogue of the asset folder `root`.
///
/// # Errors
/// The catalogue is missing, unreadable or malformed.
pub fn load_catalogue(root: &Path) -> Result<ProfileCatalogue, ProfileCatalogueError> {
    ProfileCatalogue::load(
        &PhysicalProfileFileSystem,
        &root.join(CATALOGUE_FILE_NAME),
        FULL_EDITION,
    )
}
