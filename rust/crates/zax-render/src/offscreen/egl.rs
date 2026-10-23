//! The EGL backend (every platform but Windows): libEGL loaded at run time, on Mesa's
//! surfaceless platform when there is one (llvmpipe needs no display server) and on the default
//! display otherwise. The context is made current without a surface
//! (`EGL_KHR_surfaceless_context`). The EGL display is initialised and never terminated:
//! terminating it would destroy the contexts other threads of the process may still be using.

use khronos_egl as egl;

use super::{ContextApi, OffscreenError};

/// `EGL_PLATFORM_SURFACELESS_MESA`.
const PLATFORM_SURFACELESS_MESA: egl::Enum = 0x31DD;

const LIBRARY: &str = "libEGL.so.1";

type Egl = egl::DynamicInstance<egl::EGL1_4>;

fn egl_call<T>(call: &'static str, result: Result<T, egl::Error>) -> Result<T, OffscreenError> {
    result.map_err(|error| OffscreenError::Call {
        call,
        message: error.to_string(),
    })
}

/// A current EGL context; dropping it releases the context.
pub(super) struct NativeContext {
    egl: Egl,
    display: egl::Display,
    context: egl::Context,
}

impl NativeContext {
    /// Creates a context of `api`, makes it current on the calling thread and loads GL from it.
    pub(super) fn new(api: ContextApi) -> Result<(Self, glow::Context), OffscreenError> {
        // SAFETY: libEGL is the system's EGL loader; loading it runs no initialiser beyond the
        // library's own, and every entry point the instance binds has the EGL 1.4 signature.
        let egl =
            unsafe { Egl::load_required_from_filename(LIBRARY) }.map_err(|error| {
                OffscreenError::Load {
                    library: LIBRARY,
                    message: error.to_string(),
                }
            })?;
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
            "EGL offers no configuration for the requested API",
        ))?;
        let context = egl_call(
            "eglCreateContext",
            egl.create_context(display, config, None, &attributes),
        )?;
        let native = Self {
            egl,
            display,
            context,
        };
        egl_call(
            "eglMakeCurrent",
            native
                .egl
                .make_current(display, None, None, Some(context)),
        )?;

        // SAFETY: the context is current on this thread and the loader returns the address of
        // the named GL function for it, or null, which glow treats as missing.
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                native
                    .egl
                    .get_proc_address(name)
                    .map_or(std::ptr::null(), |address| address as *const _)
            })
        };
        Ok((native, gl))
    }
}

impl Drop for NativeContext {
    /// Releases the context; a failure leaves nothing to recover in a destructor, so it is
    /// reported on the standard error stream, the only channel a drop has.
    fn drop(&mut self) {
        let released = self
            .egl
            .make_current(self.display, None, None, None)
            .and_then(|()| self.egl.destroy_context(self.display, self.context));
        if let Err(error) = released {
            eprintln!("zax-render: releasing the offscreen EGL context failed: {error}");
        }
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
