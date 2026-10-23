//! The WGL backend (Windows), where the desktop drivers ship no libEGL: a context on the device
//! context of a hidden window, which only provides the pixel format (the renderer draws into the
//! framebuffer object of [`super::OffscreenContext`]). A legacy context is made current first to
//! reach `wglCreateContextAttribsARB`, which then creates the requested one: desktop OpenGL 3.3
//! core, or OpenGL ES 3.0 through `WGL_EXT_create_context_es2_profile` when the driver offers it.
//!
//! The window and the context belong to the creating thread, as the offscreen context's
//! contract already requires. `opengl32.dll` is loaded once per context and never freed: the
//! GL 1.1 entry points `glow` resolves through it must outlive every context.

use std::ffi::{c_char, c_void, CStr, CString};
use std::iter;
use std::ptr;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_CLASS_ALREADY_EXISTS, HMODULE, HWND};
use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC, HDC};
use windows_sys::Win32::Graphics::OpenGL::{
    wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent, ChoosePixelFormat,
    SetPixelFormat, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA,
    PIXELFORMATDESCRIPTOR,
};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassExW, CS_OWNDC, WNDCLASSEXW,
    WS_OVERLAPPEDWINDOW,
};

use super::{ContextApi, OffscreenError};

const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;
const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;
const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: i32 = 0x0001;
const WGL_CONTEXT_ES2_PROFILE_BIT_EXT: i32 = 0x0004;

/// The extension that lets `wglCreateContextAttribsARB` create an OpenGL ES context.
const ES_PROFILE_EXTENSION: &str = "WGL_EXT_create_context_es2_profile";

const CLASS_NAME: &str = "ZaxRenderOffscreenGL";

type CreateContextAttribsArb =
    unsafe extern "system" fn(hdc: HDC, share: HGLRC, attributes: *const i32) -> HGLRC;
type GetExtensionsStringArb = unsafe extern "system" fn(hdc: HDC) -> *const c_char;
type Proc = unsafe extern "system" fn() -> isize;

/// The error of the failed Win32 call `call`, from the thread's last error.
fn last_error(call: &'static str) -> OffscreenError {
    // SAFETY: GetLastError only reads the calling thread's last-error value.
    let code = unsafe { GetLastError() };
    OffscreenError::Call {
        call,
        message: format!("Windows error {code}"),
    }
}

/// A NUL-terminated UTF-16 copy of `text`.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(iter::once(0)).collect()
}

/// A current WGL context; dropping it releases the context, then the window.
pub(super) struct NativeContext {
    // Declared first: fields drop in order, so the context goes before its window.
    context: Context,
    window: Window,
}

impl NativeContext {
    /// Creates a context of `api`, makes it current on the calling thread and loads GL from it.
    pub(super) fn new(api: ContextApi) -> Result<(Self, glow::Context), OffscreenError> {
        let opengl32 = load_opengl32()?;
        let window = Window::new()?;

        let dummy = Context::legacy(window.dc)?;
        dummy.make_current(window.dc)?;
        let context = create_context(window.dc, api);
        // The legacy context is only the way to the extension functions.
        // SAFETY: releasing the calling thread's current context takes no object.
        unsafe { wglMakeCurrent(ptr::null_mut(), ptr::null_mut()) };
        drop(dummy);
        let native = Self {
            context: context?,
            window,
        };
        native.context.make_current(native.window.dc)?;

        // SAFETY: the context is current on this thread and the loader returns the address of
        // the named GL function for it, or null, which glow treats as missing.
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                CString::new(name).map_or(ptr::null(), |name| proc_address(opengl32, &name))
            })
        };
        Ok((native, gl))
    }
}

impl Drop for NativeContext {
    fn drop(&mut self) {
        // SAFETY: releasing the calling thread's current context takes no object; `Context`
        // and `Window` then free their own handles.
        if unsafe { wglMakeCurrent(ptr::null_mut(), ptr::null_mut()) } == 0 {
            eprintln!(
                "zax-render: releasing the offscreen WGL context failed: {}",
                last_error("wglMakeCurrent")
            );
        }
    }
}

/// A WGL rendering context, deleted on drop.
struct Context(HGLRC);

impl Context {
    /// A legacy (compatibility) context on `dc`, whose pixel format is set.
    fn legacy(dc: HDC) -> Result<Self, OffscreenError> {
        // SAFETY: `dc` is the live device context of a window with a pixel format.
        let context = unsafe { wglCreateContext(dc) };
        if context.is_null() {
            return Err(last_error("wglCreateContext"));
        }
        Ok(Self(context))
    }

    fn make_current(&self, dc: HDC) -> Result<(), OffscreenError> {
        // SAFETY: `dc` and the context are live and share the window's pixel format.
        if unsafe { wglMakeCurrent(dc, self.0) } == 0 {
            return Err(last_error("wglMakeCurrent"));
        }
        Ok(())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: the context is live and owned by this value; a current one is made
        // non-current first by WGL itself.
        if unsafe { wglDeleteContext(self.0) } == 0 {
            eprintln!(
                "zax-render: deleting a WGL context failed: {}",
                last_error("wglDeleteContext")
            );
        }
    }
}

/// A hidden 1×1 window and its device context, with an RGBA8 + depth pixel format set.
struct Window {
    hwnd: HWND,
    dc: HDC,
}

impl Window {
    fn new() -> Result<Self, OffscreenError> {
        let class_name = wide(CLASS_NAME);
        // SAFETY: a null module name asks for the executable's own handle.
        let instance = unsafe { GetModuleHandleW(ptr::null()) };
        let class = WNDCLASSEXW {
            cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(u32::MAX),
            style: CS_OWNDC,
            lpfnWndProc: Some(DefWindowProcW),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..WNDCLASSEXW::default()
        };
        // SAFETY: the class description and its name outlive the call. A class registered by
        // an earlier context of the process is reported as already existing and is reused.
        if unsafe { RegisterClassExW(&raw const class) } == 0
            // SAFETY: GetLastError only reads the calling thread's last-error value.
            && unsafe { GetLastError() } != ERROR_CLASS_ALREADY_EXISTS
        {
            return Err(last_error("RegisterClassExW"));
        }

        // SAFETY: the class is registered for `instance`, the strings outlive the call and the
        // window is never shown (no WS_VISIBLE), so it needs no message loop.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                1,
                1,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null(),
            )
        };
        if hwnd.is_null() {
            return Err(last_error("CreateWindowExW"));
        }
        // SAFETY: `hwnd` is the live window just created.
        let dc = unsafe { GetDC(hwnd) };
        let window = Self { hwnd, dc };
        if dc.is_null() {
            return Err(last_error("GetDC"));
        }
        window.set_pixel_format()?;
        Ok(window)
    }

    fn set_pixel_format(&self) -> Result<(), OffscreenError> {
        let descriptor = PIXELFORMATDESCRIPTOR {
            nSize: u16::try_from(size_of::<PIXELFORMATDESCRIPTOR>()).unwrap_or(u16::MAX),
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cAlphaBits: 8,
            cDepthBits: 24,
            cStencilBits: 8,
            // The zero layer type is PFD_MAIN_PLANE.
            ..PIXELFORMATDESCRIPTOR::default()
        };
        // SAFETY: the device context is live and the descriptor outlives the call.
        let format = unsafe { ChoosePixelFormat(self.dc, &raw const descriptor) };
        if format == 0 {
            return Err(last_error("ChoosePixelFormat"));
        }
        // SAFETY: as above; `format` was just chosen for this device context.
        if unsafe { SetPixelFormat(self.dc, format, &raw const descriptor) } == 0 {
            return Err(last_error("SetPixelFormat"));
        }
        Ok(())
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: the window and its device context are live and owned by this value, and this
        // runs on the thread that created them.
        unsafe {
            if !self.dc.is_null() {
                ReleaseDC(self.hwnd, self.dc);
            }
            DestroyWindow(self.hwnd);
        }
    }
}

/// Loads `opengl32.dll`, which exports the GL 1.1 entry points `wglGetProcAddress` omits.
fn load_opengl32() -> Result<HMODULE, OffscreenError> {
    let name = wide("opengl32.dll");
    // SAFETY: the name is NUL-terminated and outlives the call; opengl32 is a system library.
    let module = unsafe { LoadLibraryW(name.as_ptr()) };
    if module.is_null() {
        return Err(OffscreenError::Load {
            library: "opengl32.dll",
            message: last_error("LoadLibraryW").to_string(),
        });
    }
    Ok(module)
}

/// Creates the context of `api` on `dc`; a legacy context must be current.
fn create_context(dc: HDC, api: ContextApi) -> Result<Context, OffscreenError> {
    let create = wgl_function(c"wglCreateContextAttribsARB").ok_or(OffscreenError::Unavailable(
        "the WGL driver has no WGL_ARB_create_context: no OpenGL 3.3 core context",
    ))?;
    // SAFETY: the function was resolved under its registered name, whose signature this is.
    let create = unsafe { std::mem::transmute::<Proc, CreateContextAttribsArb>(create) };

    let attributes = match api {
        ContextApi::DesktopCore => [
            WGL_CONTEXT_MAJOR_VERSION_ARB,
            3,
            WGL_CONTEXT_MINOR_VERSION_ARB,
            3,
            WGL_CONTEXT_PROFILE_MASK_ARB,
            WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
            0,
        ],
        ContextApi::Gles3 => {
            if !has_extension(dc, ES_PROFILE_EXTENSION) {
                return Err(OffscreenError::Unavailable(
                    "the WGL driver has no WGL_EXT_create_context_es2_profile: the OpenGL ES 3.0 \
                     variant is unsupported on this machine",
                ));
            }
            [
                WGL_CONTEXT_MAJOR_VERSION_ARB,
                3,
                WGL_CONTEXT_MINOR_VERSION_ARB,
                0,
                WGL_CONTEXT_PROFILE_MASK_ARB,
                WGL_CONTEXT_ES2_PROFILE_BIT_EXT,
                0,
            ]
        }
    };
    // SAFETY: `dc` is live with a pixel format, the attribute list is zero-terminated and
    // outlives the call, and no context is shared.
    let context = unsafe { create(dc, ptr::null_mut(), attributes.as_ptr()) };
    if context.is_null() {
        return Err(last_error("wglCreateContextAttribsARB"));
    }
    Ok(Context(context))
}

/// Whether the WGL extension string of `dc` lists `extension`.
fn has_extension(dc: HDC, extension: &str) -> bool {
    let Some(get) = wgl_function(c"wglGetExtensionsStringARB") else {
        return false;
    };
    // SAFETY: the function was resolved under its registered name, whose signature this is.
    let get = unsafe { std::mem::transmute::<Proc, GetExtensionsStringArb>(get) };
    // SAFETY: `dc` is live; the driver returns a NUL-terminated string it owns, or null.
    let list = unsafe { get(dc) };
    if list.is_null() {
        return false;
    }
    // SAFETY: the string is NUL-terminated and stays valid while the context is current.
    unsafe { CStr::from_ptr(list) }
        .to_string_lossy()
        .split_ascii_whitespace()
        .any(|name| name == extension)
}

/// A WGL or GL extension function of the current context, without the invalid addresses some
/// drivers return (1, 2, 3 and -1) instead of null.
fn wgl_function(name: &CStr) -> Option<Proc> {
    // SAFETY: the name is NUL-terminated and a context is current on this thread.
    let address = unsafe { wglGetProcAddress(name.as_ptr().cast()) }?;
    match address as usize {
        1..=3 | usize::MAX => None,
        _ => Some(address),
    }
}

/// The address of the GL function `name`: from WGL, else from `opengl32.dll` (GL 1.1).
fn proc_address(opengl32: HMODULE, name: &CStr) -> *const c_void {
    wgl_function(name)
        .or_else(|| {
            // SAFETY: `opengl32` stays loaded (never freed) and the name is NUL-terminated.
            unsafe { GetProcAddress(opengl32, name.as_ptr().cast()) }
        })
        .map_or(ptr::null(), |address| address as *const c_void)
}
