#![deny(unsafe_code)]

//! zax-libretro: Zaxnes as a libretro core, for `RetroArch` and the other libretro frontends.
//!
//! Step 1 (owner request, 2026-11-12): the NES in 2D, its widescreen hacks and their options,
//! the frontend's states, rewind and run-ahead, the battery RAM and the work RAM that
//! RetroAchievements reads. The 3D comes in step 2.
//!
//! [`core`] and [`options`] are safe Rust; [`ffi`], the C interface, is the crate's only
//! `unsafe` code.

pub mod core;
#[allow(unsafe_code)]
pub mod ffi;
pub mod options;
