//! The platform's own file dialog through `rfd`: Windows Explorer, the XDG desktop portal on
//! Linux. The blocking call runs on a thread of its own and the frame loop polls the answer, so
//! rendering never stops.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use super::file_picker::{FilePicker, PickPoll, PickRequest};

/// The variable that turns the native dialog off (`0`) or on (`1`) whatever the platform default.
pub const ENVIRONMENT_VARIABLE: &str = "ZAXNES_NATIVE_DIALOG";

/// A dialog that closes with no answer sooner than this did not get on screen (no portal and no
/// fallback dialog tool): nobody cancels within the first moments.
const FAILURE_WINDOW: Duration = Duration::from_millis(400);

/// What the operating system can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Desktop {
    /// Whether this is a Linux handheld build (aarch64), which is driven by a gamepad.
    pub handheld: bool,
    /// Whether a windowing session and a message bus are there for the portal (Linux only).
    pub session: bool,
}

/// Whether the native dialog is wanted: the variable decides when it says `0` or `1`, the
/// platform otherwise (never on a handheld or without a desktop session).
#[must_use]
pub fn is_enabled(variable: Option<&str>, desktop: Desktop) -> bool {
    match variable.map(str::trim) {
        Some("0") => false,
        Some("1") => true,
        _ => !desktop.handheld && desktop.session,
    }
}

impl Desktop {
    /// Looks at the running system.
    #[must_use]
    pub fn detect() -> Self {
        let linux = cfg!(target_os = "linux");
        let present = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
        let session = if linux {
            let display = present("WAYLAND_DISPLAY") || present("DISPLAY");
            let bus = present("DBUS_SESSION_BUS_ADDRESS")
                || std::env::var_os("XDG_RUNTIME_DIR")
                    .is_some_and(|runtime| Path::new(&runtime).join("bus").exists());
            display && bus
        } else {
            true
        };
        Self {
            handheld: linux && cfg!(target_arch = "aarch64"),
            session,
        }
    }
}

struct Running {
    answer: Receiver<Option<PathBuf>>,
    since: Instant,
}

/// The `rfd` dialog.
pub struct NativePicker {
    enabled: bool,
    running: Option<Running>,
}

impl NativePicker {
    /// A picker that follows the environment variable and the system.
    #[must_use]
    pub fn detect() -> Self {
        let variable = std::env::var(ENVIRONMENT_VARIABLE).ok();
        Self {
            enabled: is_enabled(variable.as_deref(), Desktop::detect()),
            running: None,
        }
    }
}

fn build(request: &PickRequest) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new().set_title(&request.title);
    if !request.folder {
        for filter in &request.filters {
            // `*` is rfd's spelling of "every file".
            let extensions: &[&str] = if filter.extensions.is_empty() {
                &["*"]
            } else {
                &filter.extensions
            };
            dialog = dialog.add_filter(&filter.label, extensions);
        }
    }
    if let Some(directory) = request
        .start_directory
        .as_deref()
        .filter(|directory| Path::new(directory).is_dir())
    {
        dialog = dialog.set_directory(directory);
    }
    dialog
}

impl FilePicker for NativePicker {
    fn start(&mut self, request: PickRequest) -> bool {
        if !self.enabled || self.running.is_some() {
            return false;
        }
        let (sender, answer) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("native-file-dialog".to_owned())
            .spawn(move || {
                let dialog = build(&request);
                let picked = if request.folder {
                    dialog.pick_folder()
                } else {
                    dialog.pick_file()
                };
                // The receiver is gone only when the application is closing.
                sender.send(picked).ok();
            });
        if spawned.is_err() {
            return false;
        }
        self.running = Some(Running {
            answer,
            since: Instant::now(),
        });
        true
    }

    fn poll(&mut self) -> PickPoll {
        let Some(running) = &self.running else {
            return PickPoll::Pending;
        };
        let outcome = match running.answer.try_recv() {
            Ok(Some(path)) => PickPoll::Chosen(path.to_string_lossy().into_owned()),
            Ok(None) if running.since.elapsed() < FAILURE_WINDOW => PickPoll::Failed,
            Ok(None) => PickPoll::Cancelled,
            // The thread ended without an answer: the dialog panicked.
            Err(TryRecvError::Disconnected) => PickPoll::Failed,
            Err(TryRecvError::Empty) => return PickPoll::Pending,
        };
        self.running = None;
        outcome
    }

    fn needs_windowed(&self) -> bool {
        cfg!(windows)
    }
}
