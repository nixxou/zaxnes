//! One instance per user data folder (SPEC-07 REQ-LOOP-054): a lock file named after the
//! folder, held for the life of the process.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;

/// What the second instance prints before it closes.
pub const ALREADY_RUNNING_MESSAGE: &str =
    "Zaxnes: another instance is already running on this data folder; this one closes (REQ-LOOP-054).";

/// The guard. Dropping it releases the lock.
#[derive(Debug)]
pub struct SingleInstance {
    lock: Option<File>,
    held: bool,
}

/// A 64-bit FNV-1a digest in hexadecimal: the name only has to be stable and short.
fn fnv_hex(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016X}")
}

impl SingleInstance {
    /// Whether this instance owns the folder.
    #[must_use]
    pub const fn is_owner(&self) -> bool {
        self.held
    }

    /// Takes the lock of `user_data_root` (the default folder when `None`). When the platform
    /// will not give the lock at all, the guard lets the program run.
    #[must_use]
    #[allow(clippy::incompatible_msrv)] // `File::try_lock` (1.89): the toolchain is pinned to 1.99.1
    pub fn acquire(user_data_root: Option<&Path>) -> Self {
        let path = std::env::temp_dir().join(Self::name_for(user_data_root));
        let Ok(file) = OpenOptions::new().create(true).truncate(false).write(true).open(path) else {
            return Self { lock: None, held: true };
        };
        match file.try_lock() {
            Ok(()) => Self {
                lock: Some(file),
                held: true,
            },
            Err(TryLockError::WouldBlock) => Self {
                lock: None,
                held: false,
            },
            Err(TryLockError::Error(_)) => Self { lock: None, held: true },
        }
    }

    /// The name of the lock for a data folder: stable, and the same for two spellings of the
    /// same folder that differ only in case or a trailing separator.
    #[must_use]
    pub fn name_for(user_data_root: Option<&Path>) -> String {
        let root = user_data_root
            .filter(|root| !root.as_os_str().is_empty())
            .map_or_else(
                || "<default>".to_owned(),
                |root| {
                    std::path::absolute(root)
                        .unwrap_or_else(|_| root.to_path_buf())
                        .to_string_lossy()
                        .trim_end_matches(['/', '\\'])
                        .to_owned()
                },
            );
        format!("Zax.Emulator.{}", fnv_hex(&root.to_lowercase()))
    }
}

impl Drop for SingleInstance {
    #[allow(clippy::incompatible_msrv)] // as above
    fn drop(&mut self) {
        if let Some(file) = self.lock.take() {
            if file.unlock().is_err() {
                // Closing the file releases the lock anyway.
            }
        }
    }
}
