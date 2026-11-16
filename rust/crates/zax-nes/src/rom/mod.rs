//! The iNES / NES 2.0 loader (port of `src/Zax.Nes/Rom`).

pub mod cartdb;
mod ines_header;
mod region_exceptions;
mod rom_image;
mod unif;

/// Lot MAPPERS-P: a `.studybox` tape and its BIOS as the image the core runs.
#[cfg(feature = "board-186")]
pub use crate::cartridges::mappers::study_box::package as package_study_box;
/// Lot FDS: a `.fds`/`.qd` disk image and `disksys.rom` as the image the core runs.
#[cfg(feature = "board-fds")]
pub use crate::cartridges::mappers::fds::package as package_fds;
pub use ines_header::InesHeader;
pub use region_exceptions::region_exception;
pub use rom_image::{RomImage, RomLoadError};
// The outcome of a load is on the `Engine` trait, so it lives in `zax-core-api` since lot
// FCEUX-23 step 2; re-exported here at the path it was at.
pub use zax_core_api::{RomLoadFailure, RomLoadResult};
