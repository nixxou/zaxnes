//! The cover art of the library entries (SPEC-09 REQ-PERS-036, Q-050). Lot REWRITE-UI (owner,
//! 2026-11-17): covers are no longer made; what is left removes the cover a game had when the
//! game is deleted, and names it.

use std::path::Path;

use crate::user_data_paths::{remove_file_if_present, UserDataPaths};

/// Text between the title and the identifier in a cover's file name.
pub const NAME_INFIX: &str = "_3dsen_";

const RESERVED_EVERYWHERE: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Replaces every character a file name cannot hold, on any platform, with an underscore.
#[must_use]
pub fn sanitise_file_name(name: &str) -> String {
    name.chars()
        .map(|value| {
            if value < ' ' || RESERVED_EVERYWHERE.contains(&value) {
                '_'
            } else {
                value
            }
        })
        .collect()
}

/// The file name of the cover of the entry `id` titled `title`.
#[must_use]
pub fn cover_file_name(title: &str, id: &str) -> String {
    format!(
        "{}.png",
        sanitise_file_name(&format!("{title}{NAME_INFIX}{id}"))
    )
}

/// The cover file of an entry.
#[must_use]
pub fn cover_file(paths: &UserDataPaths, title: &str, id: &str) -> std::path::PathBuf {
    paths.covers_directory.join(cover_file_name(title, id))
}

/// Deletes the cover of an entry: both the derived name and whatever the entry recorded, so a
/// cover imported by an earlier version, or renamed by hand, goes too. A cover that cannot be
/// removed must not stop the game from being deleted.
pub fn delete_cover(paths: &UserDataPaths, title: &str, id: &str, stored_name: Option<&str>) {
    let derived = cover_file_name(title, id);
    for name in [stored_name, Some(derived.as_str())].into_iter().flatten() {
        if name.is_empty() {
            continue;
        }
        let Some(file_name) = Path::new(name).file_name() else {
            continue;
        };
        remove_file_if_present(&paths.covers_directory.join(file_name));
    }
}
