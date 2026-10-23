//! Parent-console attach for the Windows GUI-subsystem build (port of `AttachToParentConsole`
//! in `Zax.App/Program.cs`).
//!
//! `zaxnes` is a `windows` subsystem executable, like the C# `WinExe`, so a launch from Explorer
//! opens no console window. Run from a terminal, the process has no console of its own, and the
//! text of `--help`, `--list-profiles` and the usage errors would go nowhere; this borrows the
//! launcher's console.
//!
//! The standard output handle is checked first so that a redirected run is left exactly as it
//! is: `AttachConsole` replaces the standard handles, which would send the output to the
//! terminal instead of the pipe the caller set up (the publish script and the architecture
//! tests read this program through a pipe). A launch from Explorer has no parent console and the
//! call simply fails, which is not an error.
//!
//! The C# also switches the console encoding to UTF-8. Nothing is needed here: Rust writes to a
//! console through `WriteConsoleW` and to a pipe as UTF-8 bytes.

/// Borrows the console of the parent process when there is one and the standard output has not
/// been given a handle already. Does nothing on other platforms.
pub fn attach_to_parent_console() {
    #[cfg(windows)]
    attach_windows();
}

#[cfg(windows)]
fn attach_windows() {
    use windows_sys::Win32::System::Console::{
        AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
    };

    // SAFETY: both calls take no pointers. `GetStdHandle` only reads this process's handle
    // table; `AttachConsole` changes the process's console and standard handles, which nothing
    // else has captured at this point (the caller runs it first thing in `main`). Its failure
    // (no parent console) is deliberately ignored, as in the C#.
    unsafe {
        if GetStdHandle(STD_OUTPUT_HANDLE).is_null() {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}
