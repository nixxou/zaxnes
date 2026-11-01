#![forbid(unsafe_code)]
// Like the C# `WinExe`: no console window on Windows; the parent's console is attached on demand.
#![cfg_attr(windows, windows_subsystem = "windows")]

//! The `zaxnes` executable: its command line, run by [`zax_app::program::run`].

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(zax_app::program::run(&arguments));
}
