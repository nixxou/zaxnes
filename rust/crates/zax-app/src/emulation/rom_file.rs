//! Reading a ROM from disk, bare or zipped (SPEC-07 REQ-LOOP-023).

use std::fs;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

/// Notification text of a damaged zip.
pub const CORRUPTED_ZIP_MESSAGE: &str = "Corrupted zip file";
/// Notification text of an archive that cannot be extracted.
pub const UNCOMPRESSION_FAILED_MESSAGE: &str = "Uncompression is not successful";
/// Notification text of a file that is not a ROM.
pub const INVALID_ROM_MESSAGE: &str = "Invalid rom file.";

/// A ROM read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomData {
    /// The bytes of the image.
    pub image: Vec<u8>,
    /// The name the core loads it under.
    pub name: String,
}

/// Reads the ROM at `path`: a zip gives its first entry under the archive's name with the
/// extension `.nes`; a `.7z` is refused (Q-028: no 7-Zip decoder is available).
///
/// # Errors
/// The text to show the user: the file system's own message for a missing or unreadable file
/// (REQ-LOOP-023), or one of the fixed messages of this module.
pub fn read(path: &Path) -> Result<RomData, String> {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase());
    match extension.as_deref() {
        Some("zip") => read_zip(path),
        Some("7z") => Err(UNCOMPRESSION_FAILED_MESSAGE.to_owned()),
        _ => {
            let image = fs::read(path).map_err(|error| error.to_string())?;
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(RomData { image, name })
        }
    }
}

fn read_zip(path: &Path) -> Result<RomData, String> {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|_| CORRUPTED_ZIP_MESSAGE.to_owned())?;
    if archive.is_empty() {
        return Err(UNCOMPRESSION_FAILED_MESSAGE.to_owned());
    }
    let mut entry = archive
        .by_index(0)
        .map_err(|_| UNCOMPRESSION_FAILED_MESSAGE.to_owned())?;
    let mut image = Vec::new();
    entry
        .read_to_end(&mut image)
        .map_err(|_| UNCOMPRESSION_FAILED_MESSAGE.to_owned())?;
    if image.is_empty() {
        return Err(UNCOMPRESSION_FAILED_MESSAGE.to_owned());
    }
    Ok(RomData {
        image,
        name: format!("{stem}.nes"),
    })
}
