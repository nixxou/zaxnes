#![forbid(unsafe_code)]

//! zax-nes: the NES core (port of `src/Zax.Nes`).

pub mod apu;
pub mod cartridges;
pub mod cpu;
pub mod input;
mod nes_bus;
pub mod ppu;

pub use nes_bus::NesBus;
