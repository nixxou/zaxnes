//! Reading a ROM from disk, bare or in a `.zip` or `.7z` archive (SPEC-07 REQ-LOOP-023).
//!
//! A bare file is read as it always was: one comparison of its extension, then the file. An
//! archive is opened only when a game is loaded or studied, never while one runs: its game ends
//! up in the same bytes in memory as a bare file's.

use std::fs;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use zip::ZipArchive;

/// Notification text of a damaged zip.
pub const CORRUPTED_ZIP_MESSAGE: &str = "Corrupted zip file";
/// Notification text of an archive that cannot be extracted.
pub const UNCOMPRESSION_FAILED_MESSAGE: &str = "Uncompression is not successful";
/// Notification text of a file that is not a ROM.
pub const INVALID_ROM_MESSAGE: &str = "Invalid rom file.";
/// Notification text of a RAR archive (owner decision: zip and 7z only).
pub const RAR_UNSUPPORTED_MESSAGE: &str = "RAR archives are not supported: use zip or 7z";

/// The extensions of the games Zaxnes loads, bare or in an archive.
pub const GAME_EXTENSIONS: [&str; 6] = ["nes", "unf", "unif", "studybox", "fds", "qd"];
/// The archives a game can come in.
pub const ARCHIVE_EXTENSIONS: [&str; 2] = ["zip", "7z"];
/// A 3D profile packed with the game: it wins over the profile chosen for the ROM.
pub const PROFILE_EXTENSION: &str = "3dn";
/// A hack pack packed with the game: it takes the place of the catalogue's pack for it.
pub const PACK_EXTENSION: &str = "zxpk";

/// The first bytes of a zip.
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
/// The first bytes of a 7z.
const SEVEN_ZIP_MAGIC: &[u8] = &[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C];

/// The largest file read out of an archive.
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;

/// A ROM read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomData {
    /// The bytes of the image.
    pub image: Vec<u8>,
    /// The name the core loads it under.
    pub name: String,
}

/// A file found beside the game in its archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedFile {
    /// Its name in the archive, without the folders.
    pub name: String,
    /// Its bytes.
    pub bytes: Vec<u8>,
}

/// What a ROM file holds: the game and, from an archive, a profile and a hack pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameContent {
    /// The game.
    pub rom: RomData,
    /// The first `.3dn` of the archive.
    pub profile: Option<PackedFile>,
    /// The first `.zxpk` of the archive.
    pub pack: Option<PackedFile>,
}

/// The lower-case extension of `name`.
fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn has_extension(path: &Path, known: &[&str]) -> bool {
    path.extension().is_some_and(|extension| {
        known
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
    })
}

/// Whether `path` is an archive Zaxnes opens (by its extension).
#[must_use]
pub fn is_archive(path: &Path) -> bool {
    has_extension(path, &ARCHIVE_EXTENSIONS)
}

/// Whether `path` names a game or an archive (by its extension): what the lists of ROMs show.
#[must_use]
pub fn is_rom_file(path: &Path) -> bool {
    has_extension(path, &GAME_EXTENSIONS) || is_archive(path)
}

/// Reads the ROM at `path`; from an archive, its game (see [`read_content`]).
///
/// # Errors
/// The text to show the user: the file system's own message for a missing or unreadable file
/// (REQ-LOOP-023), or one of the fixed messages of this module.
pub fn read(path: &Path) -> Result<RomData, String> {
    read_content(path).map(|content| content.rom)
}

/// Reads what the file at `path` holds: a bare game, or the game of a `.zip` or `.7z` (its first
/// entry with a game's extension, in the archive's order; its first other file when none has
/// one), with the archive's first `.3dn` and first `.zxpk`. A `.rar` is refused.
///
/// # Errors
/// As [`read`].
pub fn read_content(path: &Path) -> Result<GameContent, String> {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase());
    let rom = match extension.as_deref() {
        Some("zip") => return read_archive(path, Archive::zip(open(path)?)?),
        Some("7z") => return read_archive(path, Archive::seven_zip(open(path)?)?),
        Some("rar") => return Err(RAR_UNSUPPORTED_MESSAGE.to_owned()),
        Some(kind @ ("studybox" | "fds" | "qd")) => {
            let bytes = fs::read(path).map_err(|error| error.to_string())?;
            package(kind, bytes, file_name(path), path)?
        }
        _ => {
            let image = fs::read(path).map_err(|error| error.to_string())?;
            // An archive under another name (a frontend's file without its extension): known by
            // its first bytes, a comparison of four bytes for any other game.
            if image.starts_with(ZIP_MAGIC) {
                return read_archive(path, Archive::zip(Cursor::new(image))?);
            }
            if image.starts_with(SEVEN_ZIP_MAGIC) {
                return read_archive(path, Archive::seven_zip(Cursor::new(image))?);
            }
            RomData {
                image,
                name: file_name(path),
            }
        }
    };
    Ok(GameContent {
        rom,
        profile: None,
        pack: None,
    })
}

/// The game `bytes` of the kind `kind` (an extension), named `name`: a disk image or a tape is
/// packaged with its BIOS, looked for from `beside` (the file, or the archive it came out of).
fn package(kind: &str, bytes: Vec<u8>, name: String, beside: &Path) -> Result<RomData, String> {
    let image = match kind {
        "studybox" => {
            let bios = find_bios(beside, STUDY_BOX_BIOS)
                .ok_or_else(|| STUDY_BOX_BIOS_MISSING_MESSAGE.to_owned())?;
            zax_nes::rom::package_study_box(&bytes, &bios)?
        }
        "fds" | "qd" => {
            let bios =
                find_bios(beside, FDS_BIOS).ok_or_else(|| FDS_BIOS_MISSING_MESSAGE.to_owned())?;
            zax_nes::rom::package_fds(&bytes, &bios, kind == "qd")?
        }
        _ => bytes,
    };
    Ok(RomData { image, name })
}

/// The BIOS a Study Box tape runs with (lot MAPPERS-P, as Mesen 2 names it).
pub const STUDY_BOX_BIOS: &str = "StudyBox.bin";

/// Text of a Study Box tape without its BIOS.
pub const STUDY_BOX_BIOS_MISSING_MESSAGE: &str =
    "StudyBox.bin (256 KiB) is needed: put it beside the tape or beside the program";

/// The Famicom Disk System's BIOS, which the player supplies (lot FDS, as Mesen 2 names it).
pub const FDS_BIOS: &str = "disksys.rom";

/// Text of a disk image without its BIOS.
pub const FDS_BIOS_MISSING_MESSAGE: &str = "disksys.rom (8 KiB, the Disk System's BIOS) is \
     needed: put it beside the disk image or beside the program";

/// The BIOS file `bios`, read from beside the game `path`, then beside the executable, then in
/// the executable's `firmware` folder.
fn find_bios(path: &Path, bios: &str) -> Option<Vec<u8>> {
    let executable = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let candidates = [
        path.parent().map(|folder| folder.join(bios)),
        executable.as_ref().map(|folder| folder.join(bios)),
        executable
            .as_ref()
            .map(|folder| folder.join("firmware").join(bios)),
    ];
    candidates
        .iter()
        .flatten()
        .find_map(|candidate| fs::read(candidate).ok())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn open(path: &Path) -> Result<fs::File, String> {
    fs::File::open(path).map_err(|error| error.to_string())
}

/// An archive open for reading, from its file or from its bytes.
enum Archive<R: Read + Seek> {
    Zip(ZipArchive<R>),
    SevenZip(sevenz_rust2::ArchiveReader<R>),
}

impl<R: Read + Seek> Archive<R> {
    fn zip(file: R) -> Result<Self, String> {
        ZipArchive::new(file)
            .map(Self::Zip)
            .map_err(|_| CORRUPTED_ZIP_MESSAGE.to_owned())
    }

    fn seven_zip(file: R) -> Result<Self, String> {
        let mut reader = sevenz_rust2::ArchiveReader::new(file, sevenz_rust2::Password::empty())
            .map_err(|_| UNCOMPRESSION_FAILED_MESSAGE.to_owned())?;
        // One thread: a ROM is small, and the web build has none to spare.
        reader.set_thread_count(1);
        Ok(Self::SevenZip(reader))
    }

    /// The names of its files (not its folders), in its order.
    fn names(&self) -> Vec<String> {
        match self {
            Self::Zip(archive) => (0..archive.len())
                .filter_map(|index| archive.name_for_index(index))
                .filter(|name| !name.ends_with('/'))
                .map(str::to_owned)
                .collect(),
            Self::SevenZip(reader) => reader
                .archive()
                .files
                .iter()
                .filter(|entry| !entry.is_directory && entry.has_stream)
                .map(|entry| entry.name.clone())
                .collect(),
        }
    }

    /// The bytes of the files `wanted`, by name, in one pass (a solid 7z decodes in order: the
    /// pass stops once every wanted file is read).
    fn read(&mut self, wanted: &[&str]) -> Result<Vec<(String, Vec<u8>)>, String> {
        let mut found = Vec::new();
        match self {
            Self::Zip(archive) => {
                for name in wanted {
                    let entry = archive
                        .by_name(name)
                        .map_err(|_| UNCOMPRESSION_FAILED_MESSAGE.to_owned())?;
                    if entry.size() > MAX_ENTRY_BYTES {
                        return Err(UNCOMPRESSION_FAILED_MESSAGE.to_owned());
                    }
                    let mut bytes = Vec::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
                    entry
                        .take(MAX_ENTRY_BYTES)
                        .read_to_end(&mut bytes)
                        .map_err(|_| UNCOMPRESSION_FAILED_MESSAGE.to_owned())?;
                    found.push(((*name).to_owned(), bytes));
                }
            }
            Self::SevenZip(reader) => {
                reader
                    .for_each_entries_with_decision(|entry, data| {
                        if !wanted.contains(&entry.name.as_str()) {
                            return Ok(sevenz_rust2::EntryDecision::Continue);
                        }
                        if entry.size > MAX_ENTRY_BYTES {
                            return Err(std::io::Error::other("entry too large").into());
                        }
                        let mut bytes =
                            Vec::with_capacity(usize::try_from(entry.size).unwrap_or(0));
                        data.read_to_end(&mut bytes)?;
                        found.push((entry.name.clone(), bytes));
                        Ok(if found.len() == wanted.len() {
                            sevenz_rust2::EntryDecision::Stop
                        } else {
                            sevenz_rust2::EntryDecision::Continue
                        })
                    })
                    .map_err(|_| UNCOMPRESSION_FAILED_MESSAGE.to_owned())?;
            }
        }
        Ok(found)
    }
}

/// The base name of an archive entry (after its last folder).
fn base_name(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// The game of the archive at `path`, with its profile and its pack.
fn read_archive<R: Read + Seek>(
    path: &Path,
    mut archive: Archive<R>,
) -> Result<GameContent, String> {
    let names = archive.names();
    let first_of = |kind: &str| {
        names
            .iter()
            .find(|name| extension_of(name) == kind)
            .cloned()
    };
    let game = names
        .iter()
        .find(|name| GAME_EXTENSIONS.contains(&extension_of(name).as_str()))
        .or_else(|| {
            // No game's extension: the first file that is neither a profile nor a pack, read as a
            // `.nes` (what the zip reader always did with its first entry).
            names.iter().find(|name| {
                ![PROFILE_EXTENSION, PACK_EXTENSION].contains(&extension_of(name).as_str())
            })
        })
        .cloned()
        .ok_or_else(|| UNCOMPRESSION_FAILED_MESSAGE.to_owned())?;
    let profile = first_of(PROFILE_EXTENSION);
    let pack = first_of(PACK_EXTENSION);
    let wanted: Vec<&str> = [Some(&game), profile.as_ref(), pack.as_ref()]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
    let mut files = archive.read(&wanted)?;
    let mut take = |name: &str| {
        files
            .iter()
            .position(|(found, _)| found == name)
            .map(|at| files.swap_remove(at).1)
    };
    let bytes = take(&game).ok_or_else(|| UNCOMPRESSION_FAILED_MESSAGE.to_owned())?;
    if bytes.is_empty() {
        return Err(UNCOMPRESSION_FAILED_MESSAGE.to_owned());
    }
    let mut packed = |name: Option<String>| {
        name.and_then(|name| {
            take(&name).map(|bytes| PackedFile {
                name: base_name(&name).to_owned(),
                bytes,
            })
        })
    };
    let profile = packed(profile);
    let pack = packed(pack);
    // The game takes the archive's name with its own kind: the saves and states of an archived
    // game are named after the archive, as they were.
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let kind = extension_of(&game);
    let kind = if GAME_EXTENSIONS.contains(&kind.as_str()) {
        kind
    } else {
        "nes".to_owned()
    };
    let rom = package(&kind, bytes, format!("{stem}.{kind}"), path)?;
    Ok(GameContent { rom, profile, pack })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp(name: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("zaxnes-rom-file-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("temp folder");
        folder.join(name)
    }

    fn zip_of(path: &Path, files: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(fs::File::create(path).expect("zip"));
        for (name, bytes) in files {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .expect("entry");
            writer.write_all(bytes).expect("bytes");
        }
        writer.finish().expect("finish");
    }

    #[test]
    fn a_zip_gives_its_first_game_and_its_extras() {
        let path = temp("Game (USA).zip");
        zip_of(
            &path,
            &[
                ("readme.txt", b"hello"),
                ("folder/Game.3dn", b"profile"),
                ("Game.nes", b"NES\x1arom"),
                ("Other.nes", b"other"),
                ("Game.zxpk", b"ZXPKpack"),
            ],
        );
        let content = read_content(&path).expect("content");
        assert_eq!(content.rom.image, b"NES\x1arom");
        assert_eq!(content.rom.name, "Game (USA).nes");
        let profile = content.profile.expect("profile");
        assert_eq!(
            (profile.name.as_str(), profile.bytes.as_slice()),
            ("Game.3dn", &b"profile"[..])
        );
        assert_eq!(content.pack.expect("pack").bytes, b"ZXPKpack");
        assert_eq!(read(&path).expect("rom").image, b"NES\x1arom");
    }

    #[test]
    fn a_zip_without_a_game_extension_gives_its_first_file() {
        let path = temp("odd.zip");
        zip_of(&path, &[("a.3dn", b"p"), ("game.bin", b"data")]);
        let content = read_content(&path).expect("content");
        assert_eq!(content.rom.image, b"data");
        assert_eq!(content.rom.name, "odd.nes");
    }

    #[test]
    fn a_seven_zip_gives_its_first_game_and_its_profile() {
        let path = temp("Game (Japan).7z");
        let mut writer = sevenz_rust2::ArchiveWriter::create(&path).expect("7z");
        for (name, bytes) in [
            ("notes.txt", &b"x"[..]),
            ("Game.fds", b"FDS\x1adisk"),
            ("Game.3dn", b"p"),
        ] {
            writer
                .push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(bytes))
                .expect("entry");
        }
        writer.finish().expect("finish");
        // The disk needs its BIOS: without it, the FDS message tells which file is missing.
        assert_eq!(read(&path), Err(FDS_BIOS_MISSING_MESSAGE.to_owned()));
        let folder = path.parent().expect("folder").to_path_buf();
        let path = folder.join("Game2.7z");
        let mut writer = sevenz_rust2::ArchiveWriter::create(&path).expect("7z");
        for (name, bytes) in [("Game.3dn", &b"p"[..]), ("Game.nes", b"NES\x1arom")] {
            writer
                .push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(bytes))
                .expect("entry");
        }
        writer.finish().expect("finish");
        let content = read_content(&path).expect("content");
        assert_eq!(content.rom.image, b"NES\x1arom");
        assert_eq!(content.rom.name, "Game2.nes");
        assert_eq!(content.profile.expect("profile").bytes, b"p");
        assert!(content.pack.is_none());
    }

    #[test]
    fn an_archive_without_its_extension_is_known_by_its_bytes() {
        let zipped = temp("named.zip");
        zip_of(&zipped, &[("Game.nes", b"NES\x1arom")]);
        let path = temp("game");
        fs::copy(&zipped, &path).expect("copy");
        let content = read_content(&path).expect("content");
        assert_eq!(content.rom.image, b"NES\x1arom");
        assert_eq!(content.rom.name, "game.nes");
    }

    #[test]
    fn a_rar_is_refused_clearly() {
        let path = temp("game.rar");
        fs::write(&path, b"Rar!").expect("rar");
        assert_eq!(read(&path), Err(RAR_UNSUPPORTED_MESSAGE.to_owned()));
    }

    #[test]
    fn a_bare_rom_has_no_extras() {
        let path = temp("bare.nes");
        fs::write(&path, b"NES\x1a").expect("rom");
        let content = read_content(&path).expect("content");
        assert_eq!(content.rom.name, "bare.nes");
        assert!(content.profile.is_none() && content.pack.is_none());
    }

    #[test]
    fn the_lists_take_archives() {
        assert!(is_rom_file(Path::new("a.7Z")) && is_rom_file(Path::new("a.zip")));
        assert!(is_rom_file(Path::new("a.nes")) && !is_rom_file(Path::new("a.rar")));
    }
}
