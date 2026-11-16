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
        Some("studybox") => read_study_box(path),
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

/// The BIOS a Study Box tape runs with (lot MAPPERS-P, as Mesen 2 names it).
pub const STUDY_BOX_BIOS: &str = "StudyBox.bin";

/// Text of a Study Box tape without its BIOS.
pub const STUDY_BOX_BIOS_MISSING_MESSAGE: &str =
    "StudyBox.bin (256 KiB) is needed: put it beside the tape or beside the program";

/// A `.studybox` tape packaged with `StudyBox.bin`, found beside the tape, then beside the
/// executable or in its `firmware` folder, into the image the core runs.
fn read_study_box(path: &Path) -> Result<RomData, String> {
    let tape = fs::read(path).map_err(|error| error.to_string())?;
    let executable = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let candidates = [
        path.parent().map(|folder| folder.join(STUDY_BOX_BIOS)),
        executable
            .as_ref()
            .map(|folder| folder.join(STUDY_BOX_BIOS)),
        executable
            .as_ref()
            .map(|folder| folder.join("firmware").join(STUDY_BOX_BIOS)),
    ];
    let bios = candidates
        .iter()
        .flatten()
        .find_map(|candidate| fs::read(candidate).ok())
        .ok_or_else(|| STUDY_BOX_BIOS_MISSING_MESSAGE.to_owned())?;
    let image = zax_nes::rom::package_study_box(&tape, &bios)?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(RomData { image, name })
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
