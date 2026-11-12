#![deny(unsafe_code)]

//! zax-libretro: Zaxnes as a libretro core, for `RetroArch` and the other libretro frontends.
//!
//! Step 1 (owner request, 2026-11-12): the NES in 2D, its widescreen hacks and their options,
//! the frontend's states, rewind and run-ahead, the battery RAM and the work RAM that
//! RetroAchievements reads. Step 2: the 3D of a game with a profile, with the frontend's OpenGL.
//!
//! [`core`], [`options`], [`profiles`] and [`three_d`] are safe Rust; [`ffi`], the C interface, is the crate's only
//! `unsafe` code.

pub mod core;
#[allow(unsafe_code)]
pub mod ffi;
pub mod options;
pub mod profiles;
pub mod scene_settings;
pub mod three_d;
