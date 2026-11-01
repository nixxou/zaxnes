//! zax-android: the Android entry of Zaxnes. SDL's Java activity (`org.libsdl.app.SDLActivity`)
//! loads `libSDL2.so`, then this library, and calls its `SDL_main` on a thread of its own.
//!
//! The application runs as on a handheld (`--handheld`, the console interface driven by a
//! gamepad), its user data in the app's own folder on the shared storage
//! (`Android/data/<package>/files`): the app writes there without any permission, and the ROMs,
//! profiles, battery saves, save states and movies are copied there from a computer.

#[cfg(target_os = "android")]
mod entry {
    use std::ffi::{c_char, c_int, CStr};

    extern "C" {
        /// SDL's path of the app's folder on the shared storage (`getExternalFilesDir`), or null.
        fn SDL_AndroidGetExternalStoragePath() -> *const c_char;
    }

    /// The app's folder on the shared storage, if Android gives one.
    fn external_storage() -> Option<String> {
        // SAFETY: SDL returns null or a NUL-terminated string it keeps for the process' life.
        let path = unsafe { SDL_AndroidGetExternalStoragePath() };
        if path.is_null() {
            return None;
        }
        // SAFETY: checked non-null above; SDL's string is NUL-terminated.
        let path = unsafe { CStr::from_ptr(path) };
        Some(path.to_string_lossy().into_owned())
    }

    /// Called by SDL once its activity is up. The arguments SDL passes are not used.
    #[no_mangle]
    pub extern "C" fn SDL_main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
        let mut arguments = vec!["--handheld".to_owned()];
        if let Some(root) = external_storage() {
            arguments.push("--userdata".to_owned());
            arguments.push(root);
        }
        // A panic must not unwind into SDL's C code.
        std::panic::catch_unwind(|| zax_app::program::run(&arguments)).unwrap_or(1)
    }
}
