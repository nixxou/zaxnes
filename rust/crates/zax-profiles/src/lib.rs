#![forbid(unsafe_code)]

//! Profile format and catalogue of Zaxnes (SPEC-03): the binary profile format (`.bytes` and
//! `.3dn`) with its reader and canonical writer, the domain model the conversion engine and the
//! Lua runtime work with, the catalogue of the built-in profiles, the user profiles and the game
//! library with its ROM associations, and the 3dSen import. It depends on no windowing,
//! rendering, audio or input API (REQ-ARCH-002) and reaches files only through
//! [`catalogue::ProfileFileSystem`].

pub mod catalogue;
pub mod enums;
pub mod export;
pub mod format;
pub mod import;
pub mod layout;
pub mod library;
pub mod pattern;
pub mod profile;
pub mod script;
pub mod script_order;
pub mod setting_application;
pub mod settings;
pub mod tags;
mod text;
pub mod tile;
pub mod values;
pub mod variant;
pub mod wire;

/// Stable identifier of this module, used by the headless harness and tests.
pub const MODULE_NAME: &str = "Zax.Profiles";

/// Short human-readable role of this module.
pub const MODULE_ROLE: &str = "Profile format and catalogue";
