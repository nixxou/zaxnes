//! The framebuffer owned by the host, into which the renderer draws.

use std::num::NonZeroU32;

use thiserror::Error;

/// A framebuffer owned by the host, into which [`SceneRenderer`](crate::SceneRenderer) draws.
///
/// PLAN.md, libretro portability section: the renderer draws into a target provided from the
/// outside (FBO, size and context), so that a libretro frontend can supply its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTarget {
    /// The OpenGL framebuffer object, complete for both colour and depth, or `None` when the
    /// host really did hand over its default framebuffer; the renderer never assumes it.
    pub framebuffer: Option<NonZeroU32>,
    /// Width of the target in pixels. Must be positive.
    pub width: i32,
    /// Height of the target in pixels. Must be positive.
    pub height: i32,
}

/// A target whose declared size cannot be rendered into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TargetError {
    /// The width is not positive.
    #[error("render target width must be positive, got {0}")]
    Width(i32),
    /// The height is not positive.
    #[error("render target height must be positive, got {0}")]
    Height(i32),
}

impl RenderTarget {
    /// True when the declared size can be rendered into.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Checks that the target size is usable.
    ///
    /// # Errors
    ///
    /// [`TargetError`] when the width or the height is not positive.
    pub const fn validate(&self) -> Result<(), TargetError> {
        if self.width <= 0 {
            return Err(TargetError::Width(self.width));
        }

        if self.height <= 0 {
            return Err(TargetError::Height(self.height));
        }

        Ok(())
    }
}
