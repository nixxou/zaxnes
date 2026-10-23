#![forbid(unsafe_code)]

//! zax-harness: headless tools of the port (port of `tools/Zax.Harness`).

pub mod compare;
mod error;
pub mod golden_core;
pub mod lua_api;
pub mod meshes;
pub mod render;

pub use error::HarnessError;
