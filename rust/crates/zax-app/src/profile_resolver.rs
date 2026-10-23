//! Finds the profile named on the command line: a file, or a built-in entry by title,
//! identifier or unique prefix.

use std::path::Path;

use zax_profiles::catalogue::{built_in, CatalogueEntry, ProfileCatalogue};
use zax_profiles::format;
use zax_profiles::profile::GameProfile;

use crate::neutral_assets;
use crate::physical_file_system::PhysicalProfileFileSystem;

/// A profile with where it came from.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    /// The profile.
    pub profile: GameProfile,
    /// The title to show.
    pub title: String,
    /// The file it was read from, or `"built-in"`.
    pub source: String,
}

fn catalogue_of(asset_root: Option<&Path>) -> Result<ProfileCatalogue, String> {
    match asset_root {
        None => Ok(built_in::create()),
        Some(root) => neutral_assets::load_catalogue(root).map_err(|error| error.to_string()),
    }
}

/// Resolves `request`.
///
/// # Errors
/// The text to print: the file is not a profile, no built-in entry matches, or the catalogue
/// cannot be read.
pub fn resolve(
    request: Option<&str>,
    asset_root: Option<&Path>,
    log: &dyn Fn(&str),
) -> Result<ResolvedProfile, String> {
    let request = request.filter(|request| !request.trim().is_empty());
    if let Some(file) = request.filter(|request| Path::new(request).is_file()) {
        let bytes = std::fs::read(file).map_err(|error| error.to_string())?;
        let document = format::load(&bytes, None, Some(file))
            .map_err(|error| format!("{file}: {}", error.message))?;
        let profile = document.profile;
        let title = profile.title.clone().unwrap_or_else(|| {
            Path::new(file)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        return Ok(ResolvedProfile {
            profile,
            title,
            source: file.to_owned(),
        });
    }
    // AMD-024 REQ-PUB-001 (Q-065): with no asset folder the catalogue is the built-in one,
    // which holds "2D Mode" and "3D by default (no profile)" and no file.
    let catalogue = catalogue_of(asset_root)?;
    let entry = match request {
        None => catalogue.two_dimensional_mode(),
        Some(request) => find_by_title_or_id(&catalogue, request),
    };
    let Some(entry) = entry else {
        return Err(format!(
            "no built-in profile matches '{}'. Use --list-profiles to see the titles.",
            request.unwrap_or_default()
        ));
    };
    // A built-in entry with no file is the empty profile, built in code (Q-065).
    if entry.file.is_empty() {
        return Ok(ResolvedProfile {
            profile: GameProfile::create_empty(),
            title: entry.title.clone(),
            source: "built-in".to_owned(),
        });
    }
    match catalogue.load_profile(&PhysicalProfileFileSystem, entry, None) {
        Ok(document) => Ok(ResolvedProfile {
            profile: document.profile,
            title: entry.title.clone(),
            source: entry.file.clone(),
        }),
        Err(error) => {
            // REQ-PROF-033: a refused profile falls back to the empty profile.
            log(&format!(
                "{}: {}; falling back to the empty profile.",
                entry.title, error.message
            ));
            Ok(ResolvedProfile {
                profile: GameProfile::create_empty(),
                title: entry.title.clone(),
                source: entry.file.clone(),
            })
        }
    }
}

/// The catalogue entries, sorted by title.
///
/// # Errors
/// The catalogue cannot be read.
pub fn list(asset_root: Option<&Path>) -> Result<Vec<CatalogueEntry>, String> {
    let mut entries = match asset_root {
        None => built_in::entries(),
        Some(root) => neutral_assets::load_catalogue(root)
            .map_err(|error| error.to_string())?
            .entries()
            .to_vec(),
    };
    entries.sort_by_key(|entry| entry.title.to_lowercase());
    Ok(entries)
}

fn find_by_title_or_id<'a>(
    catalogue: &'a ProfileCatalogue,
    request: &str,
) -> Option<&'a CatalogueEntry> {
    let wanted = request.to_lowercase();
    if let Some(entry) = catalogue.entries().iter().find(|entry| {
        entry.title.to_lowercase() == wanted
            || entry.id.to_lowercase() == wanted
            || entry.file.to_lowercase() == wanted
    }) {
        return Some(entry);
    }
    // Then a unique prefix, so that "Super Mario" is enough on the command line.
    let mut matches = catalogue
        .entries()
        .iter()
        .filter(|entry| entry.title.to_lowercase().starts_with(&wanted));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}
