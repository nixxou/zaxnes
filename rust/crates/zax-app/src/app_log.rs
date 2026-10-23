//! The application's log: every line goes to the console and, once the user folder is known,
//! to `<user data>/logs/zaxnes.log` as well (Rust-only). A handheld launcher may lose the
//! console (no terminal, a redirection the firmware's shell refuses), so the start-up report
//! (SDL, GL, driver) and the frame profiler's lines must reach a file the program writes itself.
//!
//! The file is written line by line (a `LineWriter`), so a crash or a forced exit keeps every
//! line already logged. It is replaced at each start.

use std::fs::{self, File};
use std::io::{LineWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The name of the log file in the logs folder.
pub const FILE_NAME: &str = "zaxnes.log";

/// The file sink, set once by [`open`].
static FILE: OnceLock<Mutex<LineWriter<File>>> = OnceLock::new();

/// Opens `<directory>/zaxnes.log` (the folder is created) as the file copy of the log, and
/// returns its path.
///
/// # Errors
///
/// Why the folder or the file could not be created, or that a log file is already open.
pub fn open(directory: &Path) -> Result<PathBuf, String> {
    let path = directory.join(FILE_NAME);
    let failed = |error: std::io::Error| format!("'{}': {error}", path.display());
    fs::create_dir_all(directory).map_err(failed)?;
    let file = File::create(&path).map_err(failed)?;
    FILE.set(Mutex::new(LineWriter::new(file)))
        .map_err(|_| "a log file is already open".to_owned())?;
    Ok(path)
}

/// Opens the log file in the first of `directories` that takes it. Returns its path, and the
/// failures met on the way, for the log.
#[must_use]
pub fn open_first(directories: &[PathBuf]) -> (Option<PathBuf>, Vec<String>) {
    let mut failures = Vec::new();
    for directory in directories {
        match open(directory) {
            Ok(path) => return (Some(path), failures),
            Err(failure) => failures.push(failure),
        }
    }
    (None, failures)
}

/// Writes `line` to the console's standard output and to the log file.
pub fn line(line: &str) {
    println!("{line}");
    to_file(line);
}

/// Writes `line` to the console's error output and to the log file.
pub fn error(line: &str) {
    eprintln!("{line}");
    to_file(line);
}

fn to_file(line: &str) {
    if let Some(file) = FILE.get() {
        if let Ok(mut file) = file.lock() {
            // A full disk must not stop the program: the console still has the line.
            let _ = writeln!(file, "{line}");
        }
    }
}
