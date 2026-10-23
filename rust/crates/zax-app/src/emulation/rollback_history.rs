//! The rollback history of SPEC-07 §3.4: a snapshot of the core alone every 60 emulated frames,
//! at most 30 of them, and a restore that goes back to the second-to-last one.
//!
//! A snapshot holds the core and nothing else: not the scripts' persistent data, not the camera,
//! not the 3D scene (REQ-LOOP-030, DEC-07-Q3). The depth is therefore about 30 s in NTSC and 36 s
//! in PAL/Dendy. The history is never persisted (SPEC-09 REQ-PERS-058). It is cleared when a game
//! is launched, when a savestate is loaded and on every reset, and not by a pause, a quick save or
//! a rollback (REQ-LOOP-031).

use std::collections::VecDeque;

/// Highest number of snapshots kept (REQ-LOOP-030).
pub const CAPACITY: usize = 30;
/// A snapshot is taken on every frame counter that is a multiple of 60.
pub const INTERVAL_FRAMES: i64 = 60;

/// The snapshots, oldest first.
#[derive(Debug, Clone, Default)]
pub struct RollbackHistory {
    snapshots: VecDeque<Vec<u8>>,
}

impl RollbackHistory {
    /// Creates an empty history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshots currently held.
    #[must_use]
    pub fn count(&self) -> usize {
        self.snapshots.len()
    }

    /// Whether a rollback would restore nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Whether the frame about to run deserves a snapshot: its core frame counter is a multiple
    /// of 60, so the snapshot is taken before the frame executes.
    #[must_use]
    pub const fn is_snapshot_frame(core_frame_count: i64) -> bool {
        core_frame_count >= 0 && core_frame_count % INTERVAL_FRAMES == 0
    }

    /// Appends a snapshot, dropping the oldest one when the history is full.
    pub fn capture(&mut self, core_state: Vec<u8>) {
        if self.snapshots.len() == CAPACITY {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(core_state);
    }

    /// Appends the snapshot `write` puts in its buffer, as [`RollbackHistory::capture`]
    /// does. When the history is full, `write` gets the oldest snapshot's buffer to overwrite,
    /// so the steady state allocates nothing. `write` must leave the buffer untouched when it
    /// fails: the oldest snapshot then stays where it was and the history is unchanged.
    ///
    /// # Errors
    ///
    /// What `write` returns.
    pub fn capture_with<E>(
        &mut self,
        write: impl FnOnce(&mut Vec<u8>) -> Result<(), E>,
    ) -> Result<(), E> {
        let full = self.snapshots.len() == CAPACITY;
        let mut buffer = if full {
            self.snapshots.pop_front().unwrap_or_default()
        } else {
            Vec::new()
        };
        match write(&mut buffer) {
            Ok(()) => {
                self.snapshots.push_back(buffer);
                Ok(())
            }
            Err(error) => {
                if full {
                    self.snapshots.push_front(buffer);
                }
                Err(error)
            }
        }
    }

    /// Takes the snapshot a rollback restores and removes it from the history together with the
    /// last one (REQ-LOOP-032 step 2).
    ///
    /// With a single snapshot, that snapshot is restored and removed; with none, nothing is
    /// restored, but the caller still applies step 3 (go to Running, reset the clock). The
    /// original misbehaves in both cases and is deliberately not reproduced.
    pub fn take_rollback_target(&mut self) -> Option<Vec<u8>> {
        if self.snapshots.len() <= 1 {
            return self.snapshots.pop_back();
        }
        self.snapshots.pop_back();
        self.snapshots.pop_back()
    }

    /// The optional history panel of REQ-LOOP-033: restoring entry `index` (0 = oldest) keeps
    /// only the entries before it. Returns `None` when the index is out of range.
    pub fn take_at(&mut self, index: usize) -> Option<Vec<u8>> {
        if index >= self.snapshots.len() {
            return None;
        }
        self.snapshots.drain(index + 1..);
        self.snapshots.pop_back()
    }

    /// Drops every snapshot.
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
}
