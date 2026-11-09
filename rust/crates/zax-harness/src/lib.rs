#![forbid(unsafe_code)]

//! zax-harness: headless tools of the port (port of `tools/Zax.Harness`).

pub mod bench_core;
pub mod classify;
pub mod compare;
mod error;
// The FCEUX engine's two commands. Behind the feature on purpose: with `fceux-core` off neither
// the module nor its dependency on `zax-fceux` exists, and the binary is the one it always was.
#[cfg(feature = "fceux-core")]
pub mod fceux_bench;
#[cfg(feature = "fceux-core")]
pub mod fceux_compare;
#[cfg(feature = "fceux-core")]
pub mod fceux_run;
#[cfg(feature = "fceux-core")]
pub mod fceux_tiles;
pub mod golden_core;
mod json_text;
pub mod lua_api;
pub mod meshes;
pub mod packs;
pub mod profile_tool;
pub mod render;
pub mod test_roms;
#[cfg(feature = "vs-system")]
pub mod vs;

pub use error::HarnessError;
