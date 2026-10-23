//! Settings of Zaxnes: the catalogue of SPEC-09 §4.1, the layered value model, the libretro core
//! options and the persistent JSON files (REQ-PERS-001 to REQ-PERS-024, REQ-PERS-080 to REQ-PERS-086).
//!
//! The crate never touches the file system: the host hands blobs in and out through a
//! [`store::SettingsStorage`] (REQ-PERS-082).
#![forbid(unsafe_code)]

pub mod catalogue;
pub mod core_options;
pub mod domain;
pub mod games;
pub mod meta;
pub mod model;
pub mod persistence;
pub mod store;
pub mod values;

mod json;
mod num;
mod table;
