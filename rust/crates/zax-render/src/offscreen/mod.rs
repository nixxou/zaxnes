//! An offscreen OpenGL context with a colour-and-depth framebuffer, so that the renderer can be
//! driven and read back without a window (SPEC-01 REQ-ARCH-031, SPEC-06 REQ-SCN-057): the
//! harness `render` command and the GPU tests use it.
//!
//! The context comes from the platform's native API: EGL elsewhere (`egl.rs`, Mesa surfaceless
//! or the default display) and WGL on Windows (`wgl.rs`, a hidden window), where the desktop
//! drivers ship no libEGL. The renderer draws into a framebuffer object of this module, the same
//! path the application takes: the target comes from the outside.
//!
//! This module and its backends hold the only `unsafe` code outside [`crate::gl::api`]: loading
//! the native library, creating the context and building `glow` from its procedure addresses.

use std::rc::Rc;

use glow::HasContext;
use thiserror::Error;

use crate::target::RenderTarget;

#[cfg(not(windows))]
mod egl;
#[cfg(windows)]
mod wgl;

#[cfg(not(windows))]
use egl::NativeContext;
#[cfg(windows)]
use wgl::NativeContext;

/// The API of the context to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextApi {
    /// Desktop OpenGL 3.3 core, the reference target (REQ-SCN-056).
    DesktopCore,
    /// OpenGL ES 3.0 (REQ-ARCH-030).
    Gles3,
}

/// Why no offscreen context could be created.
#[derive(Debug, Error)]
pub enum OffscreenError {
    /// The native GL library could not be loaded.
    #[error("cannot load {library}: {message}")]
    Load {
        /// The library.
        library: &'static str,
        /// What the loader reported.
        message: String,
    },
    /// A call of the native context API (EGL or WGL) failed.
    #[error("{call} failed: {message}")]
    Call {
        /// The failing call.
        call: &'static str,
        /// What the API reported.
        message: String,
    },
    /// The platform offers no context of the requested kind.
    #[error("{0}")]
    Unavailable(&'static str),
    /// The framebuffer could not be built.
    #[error("the {width}x{height} offscreen target is unusable: {message}")]
    Target {
        /// Width in pixels.
        width: i32,
        /// Height in pixels.
        height: i32,
        /// What went wrong.
        message: String,
    },
}

/// A current GL context and its offscreen target; dropping it releases both.
pub struct OffscreenContext {
    gl: Rc<glow::Context>,
    framebuffer: glow::Framebuffer,
    color: glow::Texture,
    depth: glow::Renderbuffer,
    width: i32,
    height: i32,
    /// Declared last: fields drop in order, so the context outlives `gl`.
    #[expect(dead_code, reason = "held for its `Drop`, which releases the context")]
    native: NativeContext,
}

impl std::fmt::Debug for OffscreenContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OffscreenContext")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl OffscreenContext {
    /// Creates a context of `api`, makes it current on the calling thread and gives it a
    /// `width × height` RGBA8 + depth target.
    ///
    /// # Errors
    ///
    /// [`OffscreenError`] when the native context API is missing or offers no matching
    /// context, or the target cannot be built.
    pub fn new(width: i32, height: i32, api: ContextApi) -> Result<Self, OffscreenError> {
        let (native, gl) = NativeContext::new(api)?;
        // On failure the context is released by dropping `native`.
        let (framebuffer, color, depth) = allocate(&gl, width, height)?;

        Ok(Self {
            gl: Rc::new(gl),
            framebuffer,
            color,
            depth,
            width,
            height,
            native,
        })
    }

    /// The GL context, to hand to the renderer.
    #[must_use]
    pub fn gl(&self) -> Rc<glow::Context> {
        Rc::clone(&self.gl)
    }

    /// Width of the target in pixels.
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    /// Height of the target in pixels.
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }

    /// The render target to hand to the renderer.
    #[must_use]
    pub const fn target(&self) -> RenderTarget {
        RenderTarget {
            framebuffer: Some(self.framebuffer.0),
            width: self.width,
            height: self.height,
        }
    }

    /// Waits until every GL command issued so far has been executed, so that a timing taken
    /// around a render covers the work of the GPU and not only the submission.
    pub fn finish(&self) {
        // SAFETY: the context is current on this thread and `glFinish` takes no arguments.
        unsafe { self.gl.finish() };
    }

    /// Reads the target back as rows of RGBA bytes, the top row first.
    #[must_use]
    #[allow(clippy::cast_sign_loss)] // The size is positive (checked at creation).
    pub fn read_pixels(&self) -> Vec<u8> {
        let stride = self.width as usize * 4;
        let mut bottom_up = vec![0; stride * self.height as usize];
        // SAFETY: the context is current, the framebuffer is live and the buffer holds exactly
        // width × height RGBA pixels at a pack alignment of 1.
        unsafe {
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.framebuffer));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                self.width,
                self.height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut bottom_up)),
            );
        }

        bottom_up
            .chunks_exact(stride)
            .rev()
            .flatten()
            .copied()
            .collect()
    }
}

impl Drop for OffscreenContext {
    fn drop(&mut self) {
        // SAFETY: the context is still current (the renderer, which shares it, is dropped
        // first) and the three objects are live.
        unsafe {
            self.gl.delete_framebuffer(self.framebuffer);
            self.gl.delete_texture(self.color);
            self.gl.delete_renderbuffer(self.depth);
        }
        // `native` releases the context when the fields drop, after this body.
    }
}

/// Builds the RGBA8 + depth framebuffer of the target.
#[allow(clippy::cast_possible_wrap)] // GL enums are small positive values.
fn allocate(
    gl: &glow::Context,
    width: i32,
    height: i32,
) -> Result<(glow::Framebuffer, glow::Texture, glow::Renderbuffer), OffscreenError> {
    let error = |message: String| OffscreenError::Target {
        width,
        height,
        message,
    };
    if width <= 0 || height <= 0 {
        return Err(error("the size must be positive".to_owned()));
    }

    // SAFETY: the context is current on this thread; every object is created here and bound
    // only while it is live.
    unsafe {
        let framebuffer = gl.create_framebuffer().map_err(error)?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

        let color = gl.create_texture().map_err(error)?;
        gl.bind_texture(glow::TEXTURE_2D, Some(color));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            width,
            height,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(color),
            0,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);

        let depth = gl.create_renderbuffer().map_err(error)?;
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
        gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, width, height);
        gl.framebuffer_renderbuffer(
            glow::FRAMEBUFFER,
            glow::DEPTH_ATTACHMENT,
            glow::RENDERBUFFER,
            Some(depth),
        );
        gl.bind_renderbuffer(glow::RENDERBUFFER, None);

        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        if status != glow::FRAMEBUFFER_COMPLETE {
            gl.delete_renderbuffer(depth);
            gl.delete_texture(color);
            gl.delete_framebuffer(framebuffer);
            return Err(error(format!(
                "incomplete framebuffer (status 0x{status:X})"
            )));
        }

        Ok((framebuffer, color, depth))
    }
}
