#![forbid(unsafe_code)]

//! zax-harness: headless tools of the port (port of `tools/Zax.Harness`).

pub mod bench_core;
// `.bk2` (BizHawk) movies, read into `movie_file`'s own type (lot EVAL-SYNC step 4).
pub mod bk2;
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
// The movie instrument of lot EVAL-SYNC. **Not** behind `fceux-core`: `--engine zaxnes` is a
// measurement the default build can make, and the FCEUX half of the module is `cfg`'d inside it.
pub mod movie_file;
pub mod movie_sync;
pub mod packs;
pub mod profile_tool;
// The work-RAM trace of lot EVAL-SYNC-2, in the shape the owner's `BizHawk` Lua script writes it,
// so that a Zaxnes run can be held against a third machine's. Not behind `fceux-core` either.
pub mod ram_trace;
pub mod render;
pub mod test_roms;
#[cfg(feature = "vs-system")]
pub mod vs;

pub use error::HarnessError;
