//! The iNES / NES 2.0 loader (port of `src/Zax.Nes/Rom`).

mod ines_header;
mod rom_image;
mod rom_load_failure;
mod rom_load_result;

pub use ines_header::InesHeader;
pub use rom_image::{RomImage, RomLoadError};
pub use rom_load_failure::RomLoadFailure;
pub use rom_load_result::RomLoadResult;
