//! The catalogue of built-in profiles, the user profile folders and the file-system boundary
//! (SPEC-03 REQ-PROF-060 to REQ-PROF-065, AMD-033).

pub mod built_in;
mod filesystem;
mod folders;
mod profile_catalogue;
mod user_folder;

pub use filesystem::{ProfileFileSystem, UserDataLayout};
pub use folders::ProfileFolders;
pub use profile_catalogue::{
    CatalogueEntry, ProfileCatalogue, ProfileCatalogueError, DEFAULT_THREE_DIMENSIONAL_TITLE,
    DEMO_EDITION, FULL_EDITION, TWO_DIMENSIONAL_MODE_TITLE,
};
pub use user_folder::{
    derived_id, import_messages, shown_title, ProfileImportOutcome, ProfileImportResult,
    ProfileOrigin, UserProfile, UserProfileFolder,
};
