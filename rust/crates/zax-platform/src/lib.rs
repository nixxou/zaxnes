//! zax-platform: the only crate that owns a window, a GL context, physical input devices or an
//! audio device (port of `Zax.Platform`, over SDL2). Everything above it talks to abstractions
//! so that a libretro frontend can replace this layer wholesale.

pub mod audio;
pub mod console;
pub mod framebuffer;
mod gl_loader;
pub mod icon;
pub mod imgui_glyphs;
pub mod imgui_renderer;
pub mod input;
pub mod pacing;
pub mod wave;
#[cfg(feature = "wgpu")]
pub mod wgpu_display;
pub mod window;

pub use audio::AudioOutput;
pub use icon::{WindowIconImage, WindowIconState, WindowIconTarget};
pub use pacing::{DisplayPacingState, DisplaySurface};
pub use wave::{WaveAudio, WaveError};
pub use window::{GlFlavor, HostWindow, HostWindowSettings, WindowGraphics};

/// Stable identifier of this module.
pub const MODULE_NAME: &str = "Zax.Platform";
/// Short human-readable role of this module.
pub const MODULE_ROLE: &str = "Window, GL context, input, audio device";

/// An SDL call failed; the message is SDL's own.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct PlatformError(String);

impl PlatformError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl From<String> for PlatformError {
    fn from(message: String) -> Self {
        Self(message)
    }
}

impl From<sdl2::video::WindowBuildError> for PlatformError {
    fn from(error: sdl2::video::WindowBuildError) -> Self {
        Self(error.to_string())
    }
}

impl From<sdl2::IntegerOrSdlError> for PlatformError {
    fn from(error: sdl2::IntegerOrSdlError) -> Self {
        Self(error.to_string())
    }
}
