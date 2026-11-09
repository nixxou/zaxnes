//! The iNES / NES 2.0 loader (port of `src/Zax.Nes/Rom`).

mod ines_header;
mod region_exceptions;
mod rom_image;

pub use ines_header::InesHeader;
pub use region_exceptions::region_exception;
pub use rom_image::{RomImage, RomLoadError};
// The outcome of a load is on the `Engine` trait, so it lives in `zax-core-api` since lot
// FCEUX-23 step 2; re-exported here at the path it was at.
pub use zax_core_api::{RomLoadFailure, RomLoadResult};
