//! An offscreen OpenGL context with a colour-and-depth framebuffer, so that the renderer can be
//! driven and read back without a window (SPEC-01 REQ-ARCH-031, SPEC-06 REQ-SCN-057): the
//! harness `render` command and the GPU tests use it.
//!
//! It goes through EGL, loaded at run time, on Mesa's surfaceless platform when there is one
//! (llvmpipe needs no display server) and on the default display otherwise. The context is made
//! current without a surface (`EGL_KHR_surfaceless_context`) and the renderer draws into a
//! framebuffer object of this module, the same path the application takes: the target comes
//! from the outside. The EGL display is initialised and never terminated: terminating it would
//! destroy the contexts other threads of the process may still be using.
//!
//! This module holds the only `unsafe` code outside [`crate::gl::api`]: loading the library and
//! building `glow` from its procedure addresses.

use std::rc::Rc;

use glow::HasContext;
use khronos_egl as egl;
use thiserror::Error;

use crate::target::RenderTarget;

/// `EGL_PLATFORM_SURFACELESS_MESA`.
const PLATFORM_SURFACELESS_MESA: egl::Enum = 0x31DD;

type Egl = egl::DynamicInstance<egl::EGL1_4>;

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
    /// `libEGL.so.1` could not be loaded.
    #[error("cannot load libEGL: {0}")]
    Load(String),
    /// An EGL call failed.
    #[error("EGL {call} failed: {error}")]
    Egl {
        /// The failing call.
        call: &'static str,
        /// What EGL reported.
        error: egl::Error,
    },
    /// No display or no matching configuration.
    #[error("EGL: {0}")]
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

fn egl_call<T>(call: &'static str, result: Result<T, egl::Error>) -> Result<T, OffscreenError> {
    result.map_err(|error| OffscreenError::Egl { call, error })
}

/// A current GL context and its offscreen target; dropping it releases both.
pub struct OffscreenContext {
    egl: Egl,
    display: egl::Display,
    context: egl::Context,
    gl: Rc<glow::Context>,
    framebuffer: glow::Framebuffer,
    color: glow::Texture,
    depth: glow::Renderbuffer,
    width: i32,
    height: i32,
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
    /// [`OffscreenError`] when EGL is missing or offers no matching context.
    pub fn new(width: i32, height: i32, api: ContextApi) -> Result<Self, OffscreenError> {
        // SAFETY: libEGL is the system's EGL loader; loading it runs no initialiser beyond the
        // library's own, and every entry point the instance binds has the EGL 1.4 signature.
        let egl = unsafe { Egl::load_required_from_filename("libEGL.so.1") }
            .map_err(|error| OffscreenError::Load(error.to_string()))?;
        let display = open_display(&egl)?;
        egl_call("eglInitialize", egl.initialize(display))?;

        let (api_enum, bit, attributes) = match api {
            ContextApi::DesktopCore => (
                egl::OPENGL_API,
                egl::OPENGL_BIT,
                [
                    egl::CONTEXT_MAJOR_VERSION,
                    3,
                    egl::CONTEXT_MINOR_VERSION,
                    3,
                    egl::CONTEXT_OPENGL_PROFILE_MASK,
                    egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                    egl::NONE,
                ],
            ),
            ContextApi::Gles3 => (
                egl::OPENGL_ES_API,
                egl::OPENGL_ES3_BIT,
                [
                    egl::CONTEXT_MAJOR_VERSION,
                    3,
                    egl::CONTEXT_MINOR_VERSION,
                    0,
                    egl::NONE,
                    egl::NONE,
                    egl::NONE,
                ],
            ),
        };
        egl_call("eglBindAPI", egl.bind_api(api_enum))?;
        let config = egl_call(
            "eglChooseConfig",
            egl.choose_first_config(
                display,
                &[
                    egl::SURFACE_TYPE,
                    egl::PBUFFER_BIT,
                    egl::RENDERABLE_TYPE,
                    bit,
                    egl::NONE,
                ],
            ),
        )?
        .ok_or(OffscreenError::Unavailable(
            "no configuration for the requested API",
        ))?;
        let context = egl_call(
            "eglCreateContext",
            egl.create_context(display, config, None, &attributes),
        )?;
        egl_call(
            "eglMakeCurrent",
            egl.make_current(display, None, None, Some(context)),
        )?;

        // SAFETY: the context is current on this thread and the loader returns the address of
        // the named GL function for it, or null, which glow treats as missing.
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                egl.get_proc_address(name)
                    .map_or(std::ptr::null(), |address| address as *const _)
            })
        };

        let target = allocate(&gl, width, height);
        let (framebuffer, color, depth) = match target {
            Ok(target) => target,
            Err(error) => {
                // The context is torn down before the error is reported; a failure to do so
                // would only add a second error to the first.
                drop(gl);
                release_context(&egl, display, context);
                return Err(error);
            }
        };

        Ok(Self {
            egl,
            display,
            context,
            gl: Rc::new(gl),
            framebuffer,
            color,
            depth,
            width,
            height,
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
        release_context(&self.egl, self.display, self.context);
    }
}

/// Releases the context; a failure leaves nothing to recover in a destructor, so it is
/// reported on the standard error stream, the only channel a drop has.
fn release_context(egl: &Egl, display: egl::Display, context: egl::Context) {
    let released = egl
        .make_current(display, None, None, None)
        .and_then(|()| egl.destroy_context(display, context));
    if let Err(error) = released {
        eprintln!("zax-render: releasing the offscreen EGL context failed: {error}");
    }
}

fn open_display(egl: &Egl) -> Result<egl::Display, OffscreenError> {
    if let Some(egl15) = egl.upcast::<egl::EGL1_5>() {
        // SAFETY: the surfaceless platform takes no native display, so the null default is
        // the documented argument.
        let surfaceless = unsafe {
            egl15.get_platform_display(
                PLATFORM_SURFACELESS_MESA,
                egl::DEFAULT_DISPLAY,
                &[egl::ATTRIB_NONE],
            )
        };
        if let Ok(display) = surfaceless {
            return Ok(display);
        }
    }

    // SAFETY: EGL_DEFAULT_DISPLAY is the documented null argument.
    unsafe { egl.get_display(egl::DEFAULT_DISPLAY) }
        .ok_or(OffscreenError::Unavailable("no EGL display"))
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
