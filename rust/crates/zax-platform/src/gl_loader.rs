//! Builds the `glow` context the renderer draws with from the window's SDL GL context.

use std::ffi::c_void;
use std::rc::Rc;

use sdl2::VideoSubsystem;

/// Wraps the GL context current on this thread.
///
/// The context created with the window must be current, which `HostWindow` guarantees.
pub(crate) fn glow_context(video: &VideoSubsystem) -> Rc<glow::Context> {
    // SAFETY: the loader returns the addresses SDL resolves for the GL context that is current
    // on the calling thread, which is the contract `from_loader_function` requires; the
    // returned `glow::Context` is only used on that thread because `Rc` is not `Send`.
    let context = unsafe {
        glow::Context::from_loader_function(|name| video.gl_get_proc_address(name).cast::<c_void>())
    };
    Rc::new(context)
}
