//! The system's keyboard (Rust-only): on a touch screen (Android), SDL's text input raises it,
//! and its text comes as `TextInput` events.

use sdl2::sys;

/// Raises or lowers the system's on-screen keyboard (SDL's text input).
pub fn show_system_keyboard(shown: bool) {
    // SAFETY: SDL's text input calls take no arguments; the video subsystem is up while the
    // interface runs.
    unsafe {
        if shown {
            sys::SDL_StartTextInput();
        } else {
            sys::SDL_StopTextInput();
        }
    }
}
