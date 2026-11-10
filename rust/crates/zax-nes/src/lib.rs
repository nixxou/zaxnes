#![forbid(unsafe_code)]

//! zax-nes: the NES core (port of `src/Zax.Nes`).

pub mod apu;
pub mod builtin_patches;
pub mod cartridges;
pub mod cpu;
mod engine_impl;
pub mod hack_pack;
pub mod input;
pub mod movie;
mod nametable_mirroring;
mod nes_bus;
pub mod nes_colors;
mod nes_core;
pub mod ppu;
mod region_timing;
pub mod rom;
pub mod state;
mod timing_profile;
mod video_info;
#[cfg(feature = "board-vs-system")]
pub mod vs;
pub mod wide;
pub mod wide_decode;

pub use input::{ControllerPortType, ZapperPosition};
pub use nametable_mirroring::NametableMirroring;
pub use nes_bus::{NesBus, NesBusOf};
pub use nes_core::{NesCore, NesCoreOf, VideoCallback, ZapperPositionSource};
pub use region_timing::RegionTiming;
pub use timing_profile::{NesHawkCompat, Standard, TimingProfile};
pub use video_info::VideoInfo;

// The types both engines share moved to `zax-core-api` in lot FCEUX-23 step 2. They are
// re-exported here at the paths they were at — including inside `rom`, `state`, `apu` and `input`
// — so that nothing outside has to change on account of the move; a consumer that only needs a
// type should nonetheless name `zax_core_api` directly, which is what step 2 did to the
// application, the renderer and the harness.
pub use zax_core_api::{
    FrameBatch, NesRegion, NoRomLoaded, PaletteSnapshot, StateLoaded, TileRecord, VisibleRows,
    STANDARD_ROWS,
};
