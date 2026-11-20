//! Where the profiles of a 3dSen come from (owner request, 2026-11-20): its profile bundle, the
//! folder of an installation, or a `.zip` or `.7z` archive holding either; and the oldest
//! versions, which kept their profiles in their data files (`sharedassets*.assets`) rather than
//! in a bundle (3dSenVR 0.8.2's demo).
//!
//! A source is accepted only when it holds at least one profile that decodes: a bundle of
//! another Unity game, or an archive without 3dSen in it, is refused with a message saying so.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use super::installation::{find_bundle, is_bundle_name, locate_bundle, BUNDLE_EXTENSION};
use super::profile_bundle::read_text_assets;
use super::serialized_file::UnitySerializedFile;
use super::text_asset::{UnityTextAsset, CLASS_ID};
use super::UnityAssetFormatError;
use crate::catalogue::ProfileFileSystem;

/// The first bytes of a Unity bundle.
const UNITY_FS: &[u8] = b"UnityFS";
/// The first bytes of a zip.
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
/// The first bytes of a 7z.
const SEVEN_ZIP_MAGIC: &[u8] = &[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C];
/// The largest file read out of an archive.
const MAXIMUM_ENTRY: u64 = 512 * 1024 * 1024;

/// The profiles of a 3dSen read from `path`: a bundle, an installation folder, an archive, or
/// a data file of an old version. Gives the file they were read from (for the messages; in an
/// archive, the archive followed by the entry) and the text assets, of which at least one is a
/// profile.
///
/// # Errors
///
/// Nothing at `path` holds a 3dSen profile, or what does is damaged.
pub fn read_profile_source(
    files: &dyn ProfileFileSystem,
    path: &Path,
) -> Result<(PathBuf, Vec<UnityTextAsset>), UnityAssetFormatError> {
    if files.directory_exists(path) {
        return read_folder(files, path);
    }
    if !files.file_exists(path) {
        return Err(search_failure(files, path));
    }
    let bytes = files
        .read_all_bytes(path)
        .map_err(|error| UnityAssetFormatError::new(format!("{}: {error}", path.display())))?;
    if bytes.starts_with(ZIP_MAGIC) || bytes.starts_with(SEVEN_ZIP_MAGIC) {
        return read_archive(path, bytes);
    }
    let assets = read_profile_file(&bytes).ok_or_else(|| not_three_d_sen(path))?;
    Ok((path.to_path_buf(), assets))
}

/// An installation folder: its bundle, else (an old version) its data files.
fn read_folder(
    files: &dyn ProfileFileSystem,
    folder: &Path,
) -> Result<(PathBuf, Vec<UnityTextAsset>), UnityAssetFormatError> {
    if let Some(bundle) = find_bundle(files, folder) {
        let bytes = files.read_all_bytes(&bundle).map_err(|error| {
            UnityAssetFormatError::new(format!("{}: {error}", bundle.display()))
        })?;
        let assets = read_profile_file(&bytes).ok_or_else(|| not_three_d_sen(&bundle))?;
        return Ok((bundle, assets));
    }
    for data in old_data_files(files, folder) {
        if let Some(assets) = files
            .read_all_bytes(&data)
            .ok()
            .and_then(|bytes| read_profile_file(&bytes))
        {
            return Ok((data, assets));
        }
    }
    Err(search_failure(files, folder))
}

/// The `sharedassets*.assets` of the `*_Data` folders of `folder`.
fn old_data_files(files: &dyn ProfileFileSystem, folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for data in files.list_directories(folder).unwrap_or_default() {
        let name = file_name(&data).to_lowercase();
        if !name.ends_with("_data") {
            continue;
        }
        let mut assets: Vec<PathBuf> = files
            .list_files(&data, ".assets")
            .unwrap_or_default()
            .into_iter()
            .filter(|file| file_name(file).to_lowercase().starts_with("sharedassets"))
            .collect();
        assets.sort();
        found.extend(assets);
    }
    found
}

/// The text assets of a bundle's or a data file's `bytes`, when one of them is a profile.
#[must_use]
pub fn read_profile_file(bytes: &[u8]) -> Option<Vec<UnityTextAsset>> {
    let assets = if bytes.starts_with(UNITY_FS) {
        read_text_assets(bytes).ok()?
    } else {
        read_serialized_text_assets(bytes)?
    };
    holds_a_profile(&assets).then_some(assets)
}

/// The text assets of a serialized file (an old version's `sharedassets*.assets`).
fn read_serialized_text_assets(bytes: &[u8]) -> Option<Vec<UnityTextAsset>> {
    let serialized = UnitySerializedFile::read(bytes).ok()?;
    serialized
        .objects
        .iter()
        .filter(|object| object.class_id == CLASS_ID)
        .map(|object| UnityTextAsset::read(object, serialized.is_big_endian).ok())
        .collect()
}

/// Whether one of `assets` decodes as a profile: what tells a 3dSen bundle from another one.
fn holds_a_profile(assets: &[UnityTextAsset]) -> bool {
    assets
        .iter()
        .any(|asset| crate::format::load(&asset.script, None, Some(&asset.name)).is_ok())
}

/// An archive: its entries are tried in this order, the first one holding profiles taken: the
/// profile bundles by their name, the other bundles, then an old version's data files.
fn read_archive(
    path: &Path,
    bytes: Vec<u8>,
) -> Result<(PathBuf, Vec<UnityTextAsset>), UnityAssetFormatError> {
    let mut archive = Archive::open(bytes).ok_or_else(|| {
        UnityAssetFormatError::new(format!("{}: the archive cannot be read.", path.display()))
    })?;
    let names = archive.names();
    let base = |name: &String| file_name(Path::new(name)).to_lowercase();
    let mut candidates: Vec<&String> = names
        .iter()
        .filter(|name| is_bundle_name(&base(name)))
        .collect();
    candidates.extend(
        names
            .iter()
            .filter(|name| base(name).ends_with(BUNDLE_EXTENSION) && !is_bundle_name(&base(name))),
    );
    candidates.extend(names.iter().filter(|name| {
        let base = base(name);
        base.starts_with("sharedassets") && base.ends_with(".assets")
    }));
    for name in candidates {
        let Some(bytes) = archive.read(name) else {
            continue;
        };
        if let Some(assets) = read_profile_file(&bytes) {
            return Ok((path.join(name), assets));
        }
    }
    Err(UnityAssetFormatError::new(format!(
        "no 3dSen profile was found in the archive \"{}\": it should hold 3dSen's \
         defaultprofiles_assets_all_*.bundle (or a whole 3dSen folder).",
        path.display()
    )))
}

fn not_three_d_sen(path: &Path) -> UnityAssetFormatError {
    UnityAssetFormatError::new(format!(
        "\"{}\" is not a 3dSen profile bundle: it holds no 3dSen profile. The file to choose is \
         defaultprofiles_assets_all_*.bundle, under 3dSen_Data/StreamingAssets/aa.",
        path.display()
    ))
}

/// The message of the usual search, which names the layout it looked for.
fn search_failure(files: &dyn ProfileFileSystem, path: &Path) -> UnityAssetFormatError {
    locate_bundle(files, path)
        .err()
        .unwrap_or_else(|| not_three_d_sen(path))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A zip or a 7z, read from its bytes.
enum Archive {
    Zip(zip::ZipArchive<Cursor<Vec<u8>>>),
    SevenZip(Box<sevenz_rust2::ArchiveReader<Cursor<Vec<u8>>>>),
}

impl Archive {
    fn open(bytes: Vec<u8>) -> Option<Self> {
        if bytes.starts_with(ZIP_MAGIC) {
            return zip::ZipArchive::new(Cursor::new(bytes)).ok().map(Self::Zip);
        }
        let mut reader =
            sevenz_rust2::ArchiveReader::new(Cursor::new(bytes), sevenz_rust2::Password::empty())
                .ok()?;
        reader.set_thread_count(1);
        Some(Self::SevenZip(Box::new(reader)))
    }

    /// The names of its files, in its order.
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

    /// The bytes of the file `name`.
    fn read(&mut self, name: &str) -> Option<Vec<u8>> {
        match self {
            Self::Zip(archive) => {
                let entry = archive.by_name(name).ok()?;
                read_limited(entry, MAXIMUM_ENTRY)
            }
            Self::SevenZip(reader) => {
                let mut found = None;
                reader
                    .for_each_entries_with_decision(|entry, data| {
                        if entry.name != name {
                            return Ok(sevenz_rust2::EntryDecision::Continue);
                        }
                        found = read_limited(data, MAXIMUM_ENTRY);
                        Ok(sevenz_rust2::EntryDecision::Stop)
                    })
                    .ok()?;
                found
            }
        }
    }
}

fn read_limited(reader: impl Read, limit: u64) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes).ok()?;
    (u64::try_from(bytes.len()).ok()? <= limit).then_some(bytes)
}
