#![forbid(unsafe_code)]

//! zax-nes: the NES core (port of `src/Zax.Nes`).

pub mod apu;
pub mod cartridges;
pub mod cpu;
mod frame_batch;
pub mod input;
mod machine;
mod nametable_mirroring;
mod nes_bus;
pub mod nes_colors;
mod nes_region;
pub mod ppu;
mod region_timing;
pub mod rom;
mod tile_record;
mod video_info;

pub use frame_batch::FrameBatch;
pub use machine::Machine;
pub use nametable_mirroring::NametableMirroring;
pub use nes_bus::NesBus;
pub use nes_region::NesRegion;
pub use region_timing::RegionTiming;
pub use tile_record::TileRecord;
pub use video_info::VideoInfo;
