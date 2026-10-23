#![forbid(unsafe_code)]

//! zax-harness: headless tools of the port (port of `tools/Zax.Harness`).

mod error;
pub mod golden_core;

pub use error::HarnessError;
