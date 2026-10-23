use std::fmt;

use zax_engine3d::error::EngineError;
use zax_nes::apu::AudioError;
use zax_profiles::format::ProfileLoadError;
use zax_render::assets::png_image::PngError;
use zax_render::offscreen::OffscreenError;
use zax_render::RenderError;

/// Why a harness command failed.
#[derive(Debug)]
pub enum HarnessError {
    /// A file or directory could not be read or written.
    Io(std::io::Error),
    /// The core refused an audio read.
    Audio(AudioError),
    /// The command line is wrong; the message says how.
    Usage(String),
    /// A profile was not found or was refused by the loader.
    Profile(String),
    /// A frame dump does not parse; the message names the line.
    Format(String),
    /// The conversion engine refused the profile.
    Engine(EngineError),
    /// A text export could not be formatted.
    Text(std::fmt::Error),
    /// No offscreen GL context could be created.
    Offscreen(OffscreenError),
    /// The renderer failed.
    Render(RenderError),
    /// A capture could not be encoded.
    Png(PngError),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Audio(error) => write!(f, "{error}"),
            Self::Usage(message) | Self::Profile(message) | Self::Format(message) => {
                f.write_str(message)
            }
            Self::Engine(error) => write!(f, "{error}"),
            Self::Text(error) => write!(f, "{error}"),
            Self::Offscreen(error) => write!(f, "{error}"),
            Self::Render(error) => write!(f, "{error}"),
            Self::Png(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<std::io::Error> for HarnessError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<AudioError> for HarnessError {
    fn from(error: AudioError) -> Self {
        Self::Audio(error)
    }
}

impl From<ProfileLoadError> for HarnessError {
    fn from(error: ProfileLoadError) -> Self {
        Self::Profile(error.message)
    }
}

impl From<EngineError> for HarnessError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

impl From<std::fmt::Error> for HarnessError {
    fn from(error: std::fmt::Error) -> Self {
        Self::Text(error)
    }
}

impl From<OffscreenError> for HarnessError {
    fn from(error: OffscreenError) -> Self {
        Self::Offscreen(error)
    }
}

impl From<RenderError> for HarnessError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

impl From<PngError> for HarnessError {
    fn from(error: PngError) -> Self {
        Self::Png(error)
    }
}
