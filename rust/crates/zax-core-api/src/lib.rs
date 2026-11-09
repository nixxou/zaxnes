#![forbid(unsafe_code)]

//! zax-core-api: what the two emulation engines agree on.
//!
//! The application can run a game on Zaxnes's own core (`zax-nes`) or, in a build with the
//! `fceux-core` feature, on FCEUX's (`zax-fceux`). This crate holds the types that sit on the
//! boundary between them — the frame's tile records and palette, the input word, the region, the
//! errors a load or a state can fail with — and, from step 3 of lot FCEUX-23, the `Engine` trait
//! the application drives.
//!
//! It exists because of the open question §2 of `docs/review/FCEUX-ENGINE.md` left: `frame_batch`
//! returns a [`FrameBatch`], which used to live in `zax-nes`, and `zax-fceux` must not depend on
//! the core it is an alternative to. So the types moved down here, both engines depend on this
//! crate, and neither depends on the other.
//!
//! # What is **not** here
//!
//! Everything that is Zaxnes's machine rather than the boundary: the bus, the PPU, the
//! cartridges, the savestate *format* (`zax_nes::state`'s writer and reader), the widescreen
//! extension and its `visible_rows` producer, the hack packs, the margin decoders, the Zapper and
//! the VS. System. `FrameBatch`'s widescreen fields are here because the struct is one type and
//! the renderer reads all of it; FCEUX simply leaves them at their standard values, and
//! [`Capabilities`](crate::Capabilities) says so.
//!
//! # Nothing moved in behaviour
//!
//! Step 2 of the lot was a pure move: `zax-nes` re-exports every type below at the path it was
//! at, so a reader of `zax_nes::FrameBatch` or `zax_nes::rom::RomLoadResult` still finds it, and
//! golden-core came out byte-identical to the lot's baseline.

mod audio;
mod engine;
mod frame_batch;
mod input;
mod region;
mod rom_load_failure;
mod rom_load_result;
mod state;
mod tile_record;

pub use audio::{AudioError, MAX_RATIO_ADJUSTMENT, MAX_SAMPLE_RATE, MIN_SAMPLE_RATE};
pub use engine::{Capabilities, Capability, Engine, EngineKind};
pub use frame_batch::{FrameBatch, PaletteSnapshot, VisibleRows, STANDARD_ROWS};
pub use input::{button, ControllerPortType, InputWord, InvalidPort, PORT_2_SHIFT, PORT_COUNT};
pub use region::NesRegion;
pub use rom_load_failure::RomLoadFailure;
pub use rom_load_result::RomLoadResult;
pub use state::{
    engine_of_state, state_magic, InvalidState, NoRomLoaded, StateLoadFailure, StateLoaded,
    FOREIGN_CORE_MESSAGE,
};
pub use tile_record::TileRecord;
