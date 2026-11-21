//! Lot HD-PACK (owner request, 2026-11-21): the Mesen HD pack of the game in play.
//!
//! Where a pack is looked for, first found first:
//! 1. inside the ROM's own archive (a `hires.txt` in the `.zip` / `.7z` of the game);
//! 2. in `HdPacks/` beside the ROM, then in `<user data>/HdPacks/`: a folder holding a
//!    `hires.txt`, or a `.zip` / `.7z` of one, named like the ROM with the tolerance of the 3D
//!    profiles ([`crate::ui::handheld::rom_list::normalise`]: tags cut, letters and digits only,
//!    case ignored), the exact name first, else a single name that one starts with.
//!
//! The pack is read on a thread of its own (a pack can weigh hundreds of megabytes) and drawn by
//! [`zax_hdpacks::compose`] each frame it is in force.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;

use zax_core_api::TileRecord;
use zax_hdpacks::compose::ChrIndex;
use zax_hdpacks::{FolderFiles, HdPack, MemoryFiles};

use crate::emulation::rom_file;
use crate::ui::handheld::rom_list::normalise;

/// The folder of HD packs, beside the ROMs and in the user data.
pub const FOLDER: &str = "HdPacks";

/// Where a pack may be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackPlace {
    /// A folder with a `hires.txt`.
    Folder(PathBuf),
    /// An archive holding one (the ROM's own, or a pack's).
    Archive(PathBuf),
}

impl PackPlace {
    fn path(&self) -> &Path {
        match self {
            Self::Folder(path) | Self::Archive(path) => path,
        }
    }
}

/// A name without the extension of a ROM or of an archive (`Game.nes` and `Game.zip` are both
/// named `Game`).
fn bare(name: &str) -> &str {
    let path = Path::new(name);
    match path.extension().and_then(|e| e.to_str()) {
        Some(extension)
            if ["nes", "unf", "unif", "fds", "zip", "7z", "qd", "studybox"]
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known)) =>
        {
            path.file_stem().and_then(|s| s.to_str()).unwrap_or(name)
        }
        _ => name,
    }
}

/// The packs of `folder` that suit a ROM named `rom` (its file name), the closest first: the
/// same name, then the same name once normalised, then the one name one of them starts with.
#[must_use]
pub fn candidates_in(folder: &Path, rom: &str) -> Vec<PackPlace> {
    let Ok(listing) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let wanted = bare(rom).to_lowercase();
    let wanted_normal = normalise(bare(rom));
    let mut found: Vec<(u8, String, PackPlace)> = Vec::new();
    for entry in listing.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let place = if path.is_dir() {
            if !zax_hdpacks::files::is_pack_folder(&path) {
                continue;
            }
            PackPlace::Folder(path)
        } else if rom_file::is_archive(&path) {
            PackPlace::Archive(path)
        } else {
            continue;
        };
        let own = bare(&name);
        let normal = normalise(own);
        let rank = if own.to_lowercase() == wanted {
            0
        } else if !normal.is_empty() && normal == wanted_normal {
            1
        } else if !normal.is_empty()
            && !wanted_normal.is_empty()
            && (normal.starts_with(&wanted_normal) || wanted_normal.starts_with(&normal))
        {
            2
        } else {
            continue;
        };
        found.push((rank, name, place));
    }
    found.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    // A prefix is a guess: only without a closer name, and only when it is the only one.
    let guesses = found.iter().filter(|(rank, _, _)| *rank == 2).count();
    if guesses < found.len() || guesses > 1 {
        found.retain(|(rank, _, _)| *rank < 2);
    }
    found.into_iter().map(|(_, _, place)| place).collect()
}

/// Every place a pack for the ROM at `rom_path` may be, in the order they are tried.
#[must_use]
pub fn candidates(rom_path: &Path, user_root: Option<&Path>) -> Vec<PackPlace> {
    let mut places = Vec::new();
    if rom_file::is_archive(rom_path) {
        places.push(PackPlace::Archive(rom_path.to_path_buf()));
    }
    let name = rom_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if let Some(folder) = rom_path.parent() {
        places.extend(candidates_in(&folder.join(FOLDER), &name));
    }
    if let Some(root) = user_root {
        places.extend(candidates_in(&root.join(FOLDER), &name));
    }
    places
}

/// A pack ready to draw, with the CHR ROM index its tile numbers need.
#[derive(Debug)]
pub struct LoadedHdPack {
    pub pack: HdPack,
    pub chr: ChrIndex,
    /// Where it was read from, for the log.
    pub source: String,
}

/// The CHR ROM of an iNES image (empty for a CHR-RAM game or another format).
#[must_use]
pub fn chr_rom(image: &[u8]) -> &[u8] {
    if image.len() < 16 || &image[..4] != b"NES\x1a" {
        return &[];
    }
    let nes2 = image[7] & 0x0C == 0x08;
    let prg_units = usize::from(image[4])
        | if nes2 {
            usize::from(image[9] & 0x0F) << 8
        } else {
            0
        };
    let chr_units = usize::from(image[5])
        | if nes2 {
            usize::from(image[9] >> 4) << 8
        } else {
            0
        };
    let trainer = if image[6] & 0x04 != 0 { 512 } else { 0 };
    let start = 16 + trainer + prg_units * 16 * 1024;
    let end = start + chr_units * 8 * 1024;
    image.get(start..end).unwrap_or(&[])
}

/// Reads the first pack found for the ROM at `rom_path`.
///
/// # Errors
/// The text to log: a pack was found but could not be read.
pub fn load(rom_path: &Path, user_root: Option<&Path>) -> Result<Option<LoadedHdPack>, String> {
    for place in candidates(rom_path, user_root) {
        let pack = match &place {
            PackPlace::Folder(folder) => zax_hdpacks::pack::load(&FolderFiles(folder.clone())),
            PackPlace::Archive(archive) => {
                let Some(entries) = rom_file::read_hd_pack_files(archive)? else {
                    continue;
                };
                let Some(files) = MemoryFiles::from_archive(entries) else {
                    continue;
                };
                zax_hdpacks::pack::load(&files)
            }
        }
        .map_err(|error| format!("{}: {error}", place.path().display()))?;
        let chr = if pack.uses_indices {
            rom_file::read(rom_path)
                .map(|rom| ChrIndex::new(chr_rom(&rom.image)))
                .unwrap_or_default()
        } else {
            ChrIndex::default()
        };
        return Ok(Some(LoadedHdPack {
            pack,
            chr,
            source: place.path().display().to_string(),
        }));
    }
    Ok(None)
}

type Loading = Receiver<Result<Option<LoadedHdPack>, String>>;

/// The pack of the game in play, read in the background, and the HD picture of the last frame.
#[derive(Default)]
pub struct HdPackState {
    rom: Option<String>,
    loading: Option<Loading>,
    pack: Option<Arc<LoadedHdPack>>,
    /// The last HD picture, 4 bytes a pixel.
    pub picture: Vec<u8>,
    /// The tile records of the frame drawn (a copy, the session lending them briefly).
    pub records: Vec<TileRecord>,
}

impl HdPackState {
    /// Follows the game in play: a new ROM starts reading its pack (none: forgets it). `log` gets
    /// what was found.
    pub fn follow(&mut self, rom: Option<&str>, user_root: Option<&Path>, log: &dyn Fn(&str)) {
        if self.rom.as_deref() == rom {
            self.poll(log);
            return;
        }
        self.rom = rom.map(str::to_owned);
        self.pack = None;
        self.loading = None;
        let Some(rom) = rom else {
            return;
        };
        let rom = PathBuf::from(rom);
        let root = user_root.map(Path::to_path_buf);
        let (send, receive) = std::sync::mpsc::channel();
        let work = move || {
            let _ = send.send(load(&rom, root.as_deref()));
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            if std::thread::Builder::new()
                .name("hd-pack".into())
                .spawn(work)
                .is_err()
            {
                log("Zaxnes: the HD pack could not be read (no thread).");
                return;
            }
        }
        #[cfg(target_arch = "wasm32")]
        work();
        self.loading = Some(receive);
        self.poll(log);
    }

    fn poll(&mut self, log: &dyn Fn(&str)) {
        let Some(loading) = &self.loading else {
            return;
        };
        match loading.try_recv() {
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.loading = None,
            Ok(result) => {
                self.loading = None;
                match result {
                    Ok(Some(loaded)) => {
                        log(&format!(
                            "Zaxnes: HD pack {} (x{}, {} pictures, {} tiles).",
                            loaded.source,
                            loaded.pack.scale,
                            loaded.pack.images.len(),
                            loaded.pack.rules.len()
                        ));
                        self.pack = Some(Arc::new(loaded));
                    }
                    Ok(None) => {}
                    Err(error) => log(&format!("Zaxnes: HD pack not read: {error}")),
                }
            }
        }
    }

    /// The pack in force, once read.
    #[must_use]
    pub fn pack(&self) -> Option<Arc<LoadedHdPack>> {
        self.pack.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_is_found_by_name_with_the_profiles_tolerance() {
        let root = std::env::temp_dir().join(format!("zax-hdpack-find-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let packs = root.join(FOLDER);
        for folder in ["Super Mario Bros (World)", "Zelda", "Zelda II"] {
            let _ = std::fs::create_dir_all(packs.join(folder));
            let _ = std::fs::write(packs.join(folder).join("hires.txt"), "<ver>105");
        }
        let _ = std::fs::create_dir_all(packs.join("Metroid"));
        let _ = std::fs::write(packs.join("Castlevania (USA).zip"), b"PK\x03\x04");
        let found = |rom: &str| candidates_in(&packs, rom);
        assert_eq!(
            vec![PackPlace::Folder(packs.join("Super Mario Bros (World)"))],
            found("Super_Mario_Bros. (U) [!].nes")
        );
        assert_eq!(
            vec![PackPlace::Archive(packs.join("Castlevania (USA).zip"))],
            found("Castlevania [USA][!!].nes")
        );
        // A folder without hires.txt is no pack; two prefixes are no guess.
        assert!(found("Metroid (U).nes").is_empty());
        assert_eq!(
            vec![PackPlace::Folder(packs.join("Zelda"))],
            found("Zelda.nes")
        );
        assert!(found("Zeld.nes").is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_chr_rom_of_an_ines_image_is_found() {
        let mut image = b"NES\x1a\x01\x01\x00\x00".to_vec();
        image.resize(16 + 16 * 1024, 0);
        image.extend(std::iter::repeat_n(7u8, 8 * 1024));
        assert_eq!(8 * 1024, chr_rom(&image).len());
        assert_eq!(7, chr_rom(&image)[0]);
        assert!(chr_rom(b"junk").is_empty());
    }
}
