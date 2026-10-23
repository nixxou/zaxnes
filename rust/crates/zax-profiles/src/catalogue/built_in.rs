//! The two generic entries of the catalogue that need no profile file (Q-065).

use std::path::Path;

use super::profile_catalogue::{
    CatalogueEntry, ProfileCatalogue, DEFAULT_THREE_DIMENSIONAL_TITLE, TWO_DIMENSIONAL_MODE_TITLE,
};

/// Identifier of the "2D Mode" entry.
pub const TWO_DIMENSIONAL_MODE_ID: &str = "47";

/// Identifier of the "3D by default" entry.
pub const DEFAULT_THREE_DIMENSIONAL_ID: &str = "3d";

/// The generic entries: play any ROM without conversion, or in 3D with the default geometry.
#[must_use]
pub fn entries() -> Vec<CatalogueEntry> {
    vec![
        CatalogueEntry {
            id: TWO_DIMENSIONAL_MODE_ID.to_owned(),
            title: TWO_DIMENSIONAL_MODE_TITLE.to_owned(),
            publisher: String::new(),
            description: "Joue n'importe quelle ROM sans conversion 3D.".to_owned(),
            author: String::new(),
            file: String::new(),
        },
        CatalogueEntry {
            id: DEFAULT_THREE_DIMENSIONAL_ID.to_owned(),
            title: DEFAULT_THREE_DIMENSIONAL_TITLE.to_owned(),
            publisher: String::new(),
            description: "Joue n'importe quelle ROM en 3D avec la géométrie par défaut.".to_owned(),
            author: String::new(),
            file: String::new(),
        },
    ]
}

/// A catalogue of the generic entries only.
#[must_use]
pub fn create() -> ProfileCatalogue {
    ProfileCatalogue::from_entries(Path::new(""), entries())
}
