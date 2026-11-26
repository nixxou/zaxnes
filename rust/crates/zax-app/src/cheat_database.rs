//! A cheat database the player points at (lot CHEATS): a folder or a zip of sheets, one per ROM
//! image, in Mesen's JSON (BeesNES's `Cheats.zip`, Mesen's own) or libretro's `.cht`
//! (libretro-database `cht/`). Nothing is shipped until the owner chooses (`docs/progress/CHEATS.md`).
//!
//! A ROM finds its sheet through the game database: its data CRC gives its No-Intro name, the name
//! the sheets carry; a ROM the database does not know (a hack, a homebrew) is looked for by its
//! file name. `zax_cheats::sheet::choose` says how sure the match is.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use zax_cheats::import::{self, Imported};
use zax_cheats::sheet::{self, RomNames, SheetMatch};

/// The format of one sheet, from its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SheetFormat {
    Mesen,
    Libretro,
}

impl SheetFormat {
    fn of(name: &str) -> Option<(&str, Self)> {
        let (stem, extension) = name.rsplit_once('.')?;
        if extension.eq_ignore_ascii_case("json") {
            Some((stem, Self::Mesen))
        } else if extension.eq_ignore_ascii_case("cht") {
            Some((stem, Self::Libretro))
        } else {
            None
        }
    }
}

/// Where the sheets are.
#[derive(Debug)]
enum Source {
    Folder(PathBuf),
    Zip(PathBuf),
}

/// One sheet: where it is and its format.
#[derive(Debug, Clone)]
struct Entry {
    /// The file name in the folder, or the entry name in the zip.
    file: String,
    format: SheetFormat,
}

/// A database's sheets, by name (the file name without its extension).
#[derive(Debug)]
pub struct CheatDatabase {
    source: Source,
    sheets: BTreeMap<String, Entry>,
}

/// The sheet found for a ROM and its cheats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundSheet {
    /// The sheet's name.
    pub name: String,
    /// How sure the match is.
    pub matched: SheetMatch,
    /// What it holds.
    pub imported: Imported,
}

impl CheatDatabase {
    /// Lists the sheets of a folder (its files, not its subfolders) or of a zip.
    ///
    /// # Errors
    ///
    /// When the folder or the zip does not read.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let mut sheets = BTreeMap::new();
        let mut add = |file: String| {
            if let Some((name, format)) = SheetFormat::of(&file) {
                let name = name.rsplit('/').next().unwrap_or(name).to_owned();
                sheets.entry(name).or_insert(Entry { file, format });
            }
        };
        let source = if path.is_dir() {
            for entry in fs::read_dir(path)?.flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_file()) {
                    add(entry.file_name().to_string_lossy().into_owned());
                }
            }
            Source::Folder(path.to_owned())
        } else {
            let archive =
                zip::ZipArchive::new(fs::File::open(path)?).map_err(std::io::Error::other)?;
            for name in archive.file_names() {
                if !name.ends_with('/') {
                    add(name.to_owned());
                }
            }
            Source::Zip(path.to_owned())
        };
        Ok(Self { source, sheets })
    }

    /// How many sheets it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sheets.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
    }

    /// The name of the sheet for a ROM and how sure the match is, from its data CRC (the game
    /// database's No-Intro name) and its file name without the extension.
    #[must_use]
    pub fn sheet_name_for(
        &self,
        data_crc: Option<u32>,
        file_stem: &str,
    ) -> Option<(String, SheetMatch)> {
        let no_intro_name = data_crc
            .and_then(crate::gamedb::rom)
            .map_or("", |rom| rom.no_intro_name);
        let names: Vec<&str> = self.sheets.keys().map(String::as_str).collect();
        sheet::choose(
            &names,
            RomNames {
                no_intro_name,
                file_stem,
            },
        )
        .map(|(name, matched)| (name.to_owned(), matched))
    }

    /// The sheet for a ROM, read (see [`Self::sheet_name_for`]); `None` when there is none.
    ///
    /// # Errors
    ///
    /// When the sheet found does not read.
    pub fn sheet_for(
        &self,
        data_crc: Option<u32>,
        file_stem: &str,
    ) -> std::io::Result<Option<FoundSheet>> {
        let Some((name, matched)) = self.sheet_name_for(data_crc, file_stem) else {
            return Ok(None);
        };
        let imported = self.read(&name)?;
        Ok(Some(FoundSheet {
            name,
            matched,
            imported,
        }))
    }

    /// Reads one sheet by name.
    ///
    /// # Errors
    ///
    /// When it is not in the database or does not read.
    pub fn read(&self, name: &str) -> std::io::Result<Imported> {
        let entry = self
            .sheets
            .get(name)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, name.to_owned()))?;
        let bytes = match &self.source {
            Source::Folder(folder) => fs::read(folder.join(&entry.file))?,
            Source::Zip(path) => {
                let mut archive =
                    zip::ZipArchive::new(fs::File::open(path)?).map_err(std::io::Error::other)?;
                let mut file = archive
                    .by_name(&entry.file)
                    .map_err(std::io::Error::other)?;
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                bytes
            }
        };
        let text = String::from_utf8_lossy(&bytes);
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        match entry.format {
            SheetFormat::Mesen => import::mesen_json(text).map_err(std::io::Error::other),
            SheetFormat::Libretro => Ok(import::libretro_cht(text)),
        }
    }
}
