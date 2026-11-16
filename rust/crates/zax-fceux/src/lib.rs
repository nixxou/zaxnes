//! zax-fceux: FCEUX's emulation core as a second engine, behind the `fceux-core` feature.
//!
//! **Without the feature this crate is empty.** No `unsafe`, no link directive, no C compiled, no
//! dependency: `cargo build` of the workspace produces exactly the binaries it produced before
//! the crate existed. Everything below is `#[cfg(feature = "fceux-core")]`.
//!
//! With the feature on, the crate links the static library built by the fork
//! `cleanroom/fceux-zax` (branch `zax`) and wraps its C API ([`sys`]) in a safe interface
//! ([`Fceux`]). FCEUX's own sources are never copied into this repository: this crate talks to
//! the C API and nothing else.
//!
//! # Why `unsafe` is allowed here and nowhere else
//!
//! The workspace is `#![forbid(unsafe_code)]` crate by crate. This one cannot be: an FFI call is
//! `unsafe` by construction. The owner granted the exception for this crate alone, so every
//! `unsafe` block below carries a `// SAFETY:` comment, and all of it is confined to the two
//! modules of this crate — [`sys`] for the declarations and this file for the calls.
//!
//! # One instance per process
//!
//! FCEUX keeps the whole machine in file-scope globals, so there is exactly one emulator per
//! process. [`Fceux::new`] therefore fails with [`Error::AlreadyLive`] while another handle is
//! alive, and the handle is deliberately **not `Sync`**: it may *move* between threads (which is
//! what `zax-app` does with its core) but two threads must never be inside the library at once.

#![cfg_attr(not(feature = "fceux-core"), forbid(unsafe_code))]

#[cfg(feature = "fceux-core")]
pub mod sys;

#[cfg(feature = "fceux-core")]
mod core;

#[cfg(feature = "fceux-core")]
mod engine;

#[cfg(feature = "fceux-core")]
pub use core::{
    judge_movie, Error, Fceux, MovieHeader, MovieMode, RamInit, Region, Tile, TileCapture,
    FRAME_HEIGHT, FRAME_LEN, FRAME_WIDTH,
};
#[cfg(feature = "fceux-core")]
pub use engine::{FceuxEngine, BLOCK_VERSION};
