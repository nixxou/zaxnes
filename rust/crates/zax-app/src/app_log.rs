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

/// Writes `line` to the log file **only**, never to the console. It is what `-emustats` asks for
/// (AMD-053 REQ-UI-213): one line per emulated frame, in the log file and not on the parent
/// console, which a normal launch does not borrow.
pub fn file_line(line: &str) {
    to_file(line);
}

/// The last lines of the log, kept in memory only once [`keep_recent`] was called (the MCP
/// server's `get_log`): before that, one atomic load per line.
static RECENT: OnceLock<Mutex<std::collections::VecDeque<String>>> = OnceLock::new();
/// How many lines [`RECENT`] keeps.
const RECENT_LINES: usize = 1000;

/// Starts keeping the last lines of the log in memory.
pub fn keep_recent() {
    let _ = RECENT.set(Mutex::new(std::collections::VecDeque::new()));
}

/// Whether the last lines are kept.
#[must_use]
pub fn keeping_recent() -> bool {
    RECENT.get().is_some()
}

/// Keeps `line` among the recent ones, when they are kept (the Lua log goes here too).
pub fn remember(line: &str) {
    if let Some(recent) = RECENT.get() {
        if let Ok(mut recent) = recent.lock() {
            if recent.len() == RECENT_LINES {
                recent.pop_front();
            }
            recent.push_back(line.to_owned());
        }
    }
}

/// The last `count` lines kept, oldest first.
#[must_use]
pub fn recent(count: usize) -> Vec<String> {
    RECENT
        .get()
        .and_then(|recent| recent.lock().ok())
        .map(|recent| recent.iter().rev().take(count).rev().cloned().collect())
        .unwrap_or_default()
}

fn to_file(line: &str) {
    remember(line);
    if let Some(file) = FILE.get() {
        if let Ok(mut file) = file.lock() {
            // A full disk must not stop the program: the console still has the line.
            let _ = writeln!(file, "{line}");
        }
    }
}
