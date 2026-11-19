//! The web build's main loop: the browser calls the program once per display refresh
//! (`emscripten_set_main_loop`, driven by `requestAnimationFrame`) instead of the program looping
//! for ever and lending the browser a turn now and then. A loop that only lends turns depends on
//! a timer the browser may slow down to one call a second; this one cannot stall that way.
//!
//! The foreign calls of the web build live here, in the platform crate (the application crate
//! forbids `unsafe`).

use std::ffi::{c_int, c_void};

extern "C" {
    fn emscripten_set_main_loop_arg(
        func: extern "C" fn(*mut c_void),
        arg: *mut c_void,
        fps: c_int,
        simulate_infinite_loop: c_int,
    );
    fn emscripten_cancel_main_loop();
}

/// Hands `frame` to the browser, which calls it once per display refresh until it returns
/// `false`. Never returns: the browser owns the loop from now on, and `frame` (with everything it
/// holds) lives until the page is closed.
pub fn run_main_loop<F: FnMut() -> bool + 'static>(frame: F) -> ! {
    extern "C" fn trampoline<F: FnMut() -> bool>(arg: *mut c_void) {
        // SAFETY: `arg` is the leaked box of `run_main_loop`, used by this loop alone, on the
        // browser's one thread.
        let frame = unsafe { &mut *arg.cast::<F>() };
        if !frame() {
            // SAFETY: a plain call of the runtime, from inside the loop it stops.
            unsafe { emscripten_cancel_main_loop() };
        }
    }
    let state = Box::into_raw(Box::new(frame));
    // SAFETY: the state is leaked, so it outlives the loop; 0 frames a second means one call per
    // `requestAnimationFrame`; 1 makes this call never return (the runtime unwinds out of it), so
    // nothing on the stack below is used again.
    unsafe {
        emscripten_set_main_loop_arg(trampoline::<F>, state.cast::<c_void>(), 0, 1);
    }
    unreachable!("emscripten_set_main_loop_arg with simulate_infinite_loop does not return")
}
