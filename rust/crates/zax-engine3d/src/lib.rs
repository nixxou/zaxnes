#![forbid(unsafe_code)]

//! 2D-to-3D conversion engine of Zaxnes (SPEC-04, SPEC-04B): intake of the core's tile
//! records, frame model, pattern recognition, shapes and frame instances, palettes, the
//! tracker, the Lua hook call points, and the geometry and meshes the renderer consumes.
//!
//! The crate produces abstract mesh data and never touches a graphics API, a window or the
//! wall clock (REQ-ARCH-002).

pub mod draw_list;
pub mod engine;
pub mod error;
pub mod frame;
pub mod geometry;
pub mod grid;
pub mod image;
pub mod instance;
pub mod palette;
pub mod pattern;
pub mod registry;
pub mod script;
mod segmenter;
pub mod settings;
pub mod state_codec;
pub mod tracker;
pub mod vectors;

/// Stable identifier of this module, used by the headless harness and tests.
pub const MODULE_NAME: &str = "Zax.Engine3D";

/// Short human-readable role of this module.
pub const MODULE_ROLE: &str = "2D-to-3D conversion (abstract meshes)";
