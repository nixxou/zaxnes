#![forbid(unsafe_code)]

//! Mesen HD packs (feat/hd-pack).
//!
//! A pack is a `hires.txt` and its PNG pictures ([`pack::HdPack`]), read from a folder or from
//! files already in memory (an archive, [`files::PackFiles`]). Each frame, [`compose::compose`]
//! draws the HD picture from the core's picture and tile records: every tile the pack replaces is
//! drawn bigger, the rest is the original pixel enlarged.

pub mod compose;
pub mod files;
pub mod image;
pub mod pack;

pub use compose::{compose, FrameInput};
pub use files::{FolderFiles, MemoryFiles, PackFiles};
pub use pack::HdPack;

/// The 64 NES colours as RGB, index = NES colour (6 bits).
pub type NesRgb = [[u8; 3]; 64];

/// What can go wrong reading a pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HdPackError {
    /// A file the pack needs is missing.
    Missing(String),
    /// A PNG could not be decoded.
    Png(String),
    /// A line of `hires.txt` is malformed.
    Line { number: usize, text: String },
}

impl std::fmt::Display for HdPackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "{name} is missing from the pack"),
            Self::Png(text) => write!(f, "picture: {text}"),
            Self::Line { number, text } => write!(f, "hires.txt line {number}: {text}"),
        }
    }
}

impl std::error::Error for HdPackError {}

/// The result type of the crate.
pub type Result<T> = std::result::Result<T, HdPackError>;
