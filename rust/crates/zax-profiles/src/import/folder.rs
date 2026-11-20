//! The import folder (owner request, 2026-11-20): what is put in it is imported by itself, on
//! every platform (the application, its Android and web builds, the libretro core). A 3dSen
//! profile bundle, a `.zip` or `.7z` holding one (or a whole 3dSen folder), and single 3D
//! profiles (`.3dn`, `.3dh`). Each file imported gets a `<file>.done` beside it, which says how
//! it went, so that it is not imported again; deleting the `.done` imports it once more.

use std::path::{Path, PathBuf};

/// The name of the folder, in the user data (or in `<system>/Zaxnes` for the libretro core).
pub const FOLDER: &str = "import";
/// The file that explains the folder.
pub const README: &str = "README.txt";
/// The marker of an imported file.
pub const DONE_SUFFIX: &str = ".done";

/// The extensions imported as a 3dSen source.
pub const SOURCE_EXTENSIONS: [&str; 4] = ["bundle", "zip", "7z", "assets"];
/// The extensions imported as single profiles.
pub const PROFILE_EXTENSIONS: [&str; 2] = ["3dn", "3dh"];

/// What the folder's README says, in English then in French.
pub const README_TEXT: &str = "\
Zaxnes - import folder
======================

Put here what you want imported; Zaxnes imports it by itself (when it starts, and while the
list of games shows), then writes a file of the same name ending in .done beside it, saying how
it went. Delete the .done file to import it again.

What can go here:
- the profiles of 3dSen: the file defaultprofiles_assets_all_*.bundle of your 3dSen
  (3dSen_Data/StreamingAssets/aa/StandaloneWindows/), under its name or another;
- a .zip or .7z archive holding that file, or a whole 3dSen folder (even a very old version,
  whose profiles are in its sharedassets files);
- 3D profiles: .3dn and .3dh files.

The imported profiles join yours; one with the same identifier is replaced.

-----------------------------------------------------------------------------------------------

Zaxnes - dossier d'import
=========================

Mettez ici ce que vous voulez importer : Zaxnes l'importe tout seul (au démarrage, et tant que
la liste des jeux s'affiche), puis écrit à côté un fichier du même nom finissant par .done, qui
dit comment ça s'est passé. Supprimez le fichier .done pour l'importer à nouveau.

Ce qu'on peut y mettre :
- les profils de 3dSen : le fichier defaultprofiles_assets_all_*.bundle de votre 3dSen
  (3dSen_Data/StreamingAssets/aa/StandaloneWindows/), sous son nom ou un autre ;
- une archive .zip ou .7z qui contient ce fichier, ou tout un dossier de 3dSen (même une très
  vieille version, dont les profils sont dans ses fichiers sharedassets) ;
- des profils 3D : des fichiers .3dn et .3dh.

Les profils importés rejoignent les vôtres ; celui d'un même identifiant est remplacé.
";

/// What a waiting file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingKind {
    /// A bundle, an archive or a data file: the profiles of a 3dSen.
    ThreeDSen,
    /// A single profile.
    Profile,
}

/// Creates the folder `folder` and its README when they are missing.
pub fn prepare(folder: &Path) {
    if std::fs::create_dir_all(folder).is_ok() {
        let readme = folder.join(README);
        if !readme.exists() {
            let _ = std::fs::write(readme, README_TEXT);
        }
    }
}

/// The files of `folder` waiting to be imported, by name: those of a kind it imports with no
/// `.done` beside them.
#[must_use]
pub fn waiting(folder: &Path) -> Vec<(PathBuf, WaitingKind)> {
    let Ok(listing) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<(PathBuf, WaitingKind)> = listing
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && !done_marker(path).exists())
        .filter_map(|path| {
            let extension = path.extension()?.to_string_lossy().to_lowercase();
            let kind = if SOURCE_EXTENSIONS.contains(&extension.as_str()) {
                WaitingKind::ThreeDSen
            } else if PROFILE_EXTENSIONS.contains(&extension.as_str()) {
                WaitingKind::Profile
            } else {
                return None;
            };
            Some((path, kind))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// The `.done` marker of `path`.
#[must_use]
pub fn done_marker(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(DONE_SUFFIX);
    path.with_file_name(name)
}

/// Marks `path` imported, `report` saying how it went.
pub fn mark_done(path: &Path, report: &str) {
    let _ = std::fs::write(done_marker(path), format!("{report}\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_lists_what_waits_and_forgets_what_is_done() {
        let folder = std::env::temp_dir().join(format!("zax-import-folder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        prepare(&folder);
        assert!(folder.join(README).is_file());
        for name in ["a.bundle", "b.zip", "c.7z", "d.3dn", "e.txt", "f.nes"] {
            std::fs::write(folder.join(name), b"x").unwrap();
        }
        let names = |folder: &Path| -> Vec<String> {
            waiting(folder)
                .iter()
                .map(|(path, _)| path.file_name().unwrap().to_string_lossy().into_owned())
                .collect()
        };
        assert_eq!(vec!["a.bundle", "b.zip", "c.7z", "d.3dn"], names(&folder));
        assert_eq!(WaitingKind::Profile, waiting(&folder)[3].1);
        mark_done(&folder.join("b.zip"), "3dSen: 1 profile(s) imported");
        assert_eq!(vec!["a.bundle", "c.7z", "d.3dn"], names(&folder));
        assert!(std::fs::read_to_string(folder.join("b.zip.done"))
            .unwrap()
            .contains("imported"));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
