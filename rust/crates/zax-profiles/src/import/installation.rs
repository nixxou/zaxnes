//! Finding the profile bundle of a 3dSen installation (AMD-030 §3).

use std::path::{Path, PathBuf};

use super::UnityAssetFormatError;
use crate::catalogue::ProfileFileSystem;

/// Start of the file name of the bundle that holds the profiles.
pub const BUNDLE_PREFIX: &str = "defaultprofiles_assets_all_";

/// File extension of a bundle.
pub const BUNDLE_EXTENSION: &str = ".bundle";

const STREAMING_PATH: [&str; 2] = ["StreamingAssets", "aa"];
const DATA_SUFFIX: &str = "_Data";

/// The platform folders under `StreamingAssets/aa`, in the order they are tried (AMD-030 §3.1).
pub const PLATFORMS: [&str; 4] = [
    "StandaloneWindows",
    "StandaloneWindows64",
    "StandaloneLinux64",
    "StandaloneOSX",
];

/// The layout of an installation, as the message of a failed search states it.
#[must_use]
pub fn expected_layout() -> String {
    format!(
        "<installation>/<nom>{DATA_SUFFIX}/{}/<{}>/{BUNDLE_PREFIX}*{BUNDLE_EXTENSION}",
        STREAMING_PATH.join("/"),
        PLATFORMS.join("|")
    )
}

/// The bundle of `path`, which is the bundle itself or an installation folder (AMD-030 §3, §3.1,
/// §3.2); none when nothing is found.
#[must_use]
pub fn find_bundle(files: &dyn ProfileFileSystem, path: &Path) -> Option<PathBuf> {
    if files.file_exists(path) {
        return Some(path.to_path_buf());
    }
    if !files.directory_exists(path) {
        return None;
    }

    // 1. <folder>/<anything>_Data/StreamingAssets/aa/<platform>/
    if let Some(bundle) = find_under(files, &data_folders(files, path)) {
        return Some(bundle);
    }

    // 2. macOS: <folder>/*.app/Contents/Resources/Data/StreamingAssets/aa/<platform>/
    find_under(files, &mac_data_folders(files, path))
}

/// Like [`find_bundle`], with the reason when nothing is found.
///
/// # Errors
///
/// `path` is neither a file nor a folder, or no bundle is under it; the message names the
/// expected layout.
pub fn locate_bundle(
    files: &dyn ProfileFileSystem,
    path: &Path,
) -> Result<PathBuf, UnityAssetFormatError> {
    if let Some(bundle) = find_bundle(files, path) {
        return Ok(bundle);
    }

    if !files.directory_exists(path) && !files.file_exists(path) {
        return Err(UnityAssetFormatError::new(format!(
            "\"{}\" is neither a file nor a folder.",
            path.display()
        )));
    }

    Err(UnityAssetFormatError::new(format!(
        "no 3dSen profile bundle was found under \"{}\". \
         Name the folder holding 3dSen.exe, or the bundle itself; the expected path is {}.",
        path.display(),
        expected_layout()
    )))
}

/// True when `file_name` is the name of a profile bundle.
#[must_use]
pub fn is_bundle_name(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    lower.starts_with(BUNDLE_PREFIX) && lower.ends_with(BUNDLE_EXTENSION)
}

fn find_under(files: &dyn ProfileFileSystem, data_folders: &[PathBuf]) -> Option<PathBuf> {
    PLATFORMS.iter().find_map(|platform| {
        data_folders.iter().find_map(|data| {
            let folder = STREAMING_PATH
                .iter()
                .fold(data.clone(), |folder, part| folder.join(part))
                .join(platform);
            first_bundle_in(files, &folder)
        })
    })
}

fn first_bundle_in(files: &dyn ProfileFileSystem, folder: &Path) -> Option<PathBuf> {
    if !files.directory_exists(folder) {
        return None;
    }
    let mut bundles: Vec<PathBuf> = listed(files.list_files(folder, BUNDLE_EXTENSION))
        .into_iter()
        .filter(|file| is_bundle_name(&file_name(file)))
        .collect();
    // Several fingerprints in one folder is not expected; sorting keeps the choice stable.
    bundles.sort_by_key(|bundle| {
        bundle
            .to_string_lossy()
            .encode_utf16()
            .collect::<Vec<u16>>()
    });
    bundles.into_iter().next()
}

fn data_folders(files: &dyn ProfileFileSystem, root: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = listed(files.list_directories(root))
        .into_iter()
        .filter(|directory| {
            file_name(directory)
                .to_lowercase()
                .ends_with(&DATA_SUFFIX.to_lowercase())
        })
        .collect();
    folders.sort_by_key(|folder| {
        folder
            .to_string_lossy()
            .encode_utf16()
            .collect::<Vec<u16>>()
    });
    folders
}

fn mac_data_folders(files: &dyn ProfileFileSystem, root: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = listed(files.list_directories(root))
        .into_iter()
        .filter(|directory| file_name(directory).to_lowercase().ends_with(".app"))
        .map(|directory| directory.join("Contents").join("Resources").join("Data"))
        .collect();
    folders.sort_by_key(|folder| {
        folder
            .to_string_lossy()
            .encode_utf16()
            .collect::<Vec<u16>>()
    });
    folders
}

/// A folder that cannot be listed counts as empty, as in the reference: the search moves on to
/// the next candidate instead of failing.
fn listed(result: std::io::Result<Vec<PathBuf>>) -> Vec<PathBuf> {
    result.unwrap_or_default()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}
