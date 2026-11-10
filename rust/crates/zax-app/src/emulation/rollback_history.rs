//! The rollback history of SPEC-07 §3.4: a snapshot every 60 emulated frames, at most 30 of
//! them, and a restore that goes back to the second-to-last one.
//!
//! A snapshot holds the core, the scripts' 64 `PData` values and the camera state (AMD-042
//! REQ-LOOP-069, which replaces the last sentence of REQ-LOOP-030's notes). It still holds
//! neither the learned conversion state, nor the converted frames, nor the meshes. The depth is
//! therefore about 30 s in NTSC and 36 s in PAL/Dendy. The history is never persisted (SPEC-09
//! REQ-PERS-058). It is cleared when a game is launched, when a savestate is loaded, on every
//! reset and at the **end** of a game (AMD-042 REQ-LOOP-071), and not by a pause, a quick save or
//! a rollback (REQ-LOOP-031).

use std::collections::VecDeque;

use zax_render::camera::CameraStateData;
use zax_render::numerics::Vector3;
use zax_scripting::persistent::BLOCK_SIZE;

/// Highest number of snapshots kept (REQ-LOOP-030).
pub const CAPACITY: usize = 30;
/// A snapshot is taken on every frame counter that is a multiple of 60.
pub const INTERVAL_FRAMES: i64 = 60;

/// What a snapshot carries besides the core state (AMD-042 REQ-LOOP-069, with the fourth element
/// AMD-063 REQ-LUA-119 adds): the 64 `PData` values as the block of SPEC-05 §4.6, the camera state
/// a savestate holds — REQ-SCN-024 being its single reference — and the forced camera rotation.
///
/// Both are fixed-size and `Copy`, so the steady state of the history still allocates nothing:
/// only the core state is a buffer, and that buffer is reused ([`RollbackHistory::capture_with`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapshotExtras {
    /// The 64 `PData` values, read the way `GetPData(i)` reads them, or `None` when the profile
    /// has no Lua runtime to read them from.
    pub persistent_data: Option<[u8; BLOCK_SIZE]>,
    /// The camera state, or `None` when the snapshot was taken with no camera at hand.
    pub camera: Option<CameraStateData>,
    /// The value of `CS.Nes3D.CameraControl.RotOverride` (AMD-063 REQ-LUA-119). It is a session
    /// value, so it is **not** part of `camera` — REQ-SCN-024 is unchanged — and travels here on
    /// its own; a rollback puts it back before the camera state it was taken with.
    pub rot_override: Option<Vector3>,
}

impl SnapshotExtras {
    /// Neither the persistent data nor the camera: what a snapshot of the core alone carries.
    pub const NONE: Self = Self {
        persistent_data: None,
        camera: None,
        rot_override: None,
    };
}

/// One entry of the history: the core state and the two things AMD-042 adds to it.
#[derive(Debug, Clone)]
pub struct RollbackSnapshot {
    /// The complete core state; an in-memory one is enough (REQ-CORE-121), so the ROM image is
    /// left out.
    pub core: Vec<u8>,
    /// The `PData` values and the camera state, taken at the same instant as the core.
    pub extras: SnapshotExtras,
}

/// The snapshots, oldest first.
#[derive(Debug, Clone, Default)]
pub struct RollbackHistory {
    snapshots: VecDeque<RollbackSnapshot>,
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
    pub fn capture(&mut self, core_state: Vec<u8>, extras: SnapshotExtras) {
        if self.snapshots.len() == CAPACITY {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(RollbackSnapshot {
            core: core_state,
            extras,
        });
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
        extras: SnapshotExtras,
        write: impl FnOnce(&mut Vec<u8>) -> Result<(), E>,
    ) -> Result<(), E> {
        let full = self.snapshots.len() == CAPACITY;
        let mut snapshot = if full {
            self.snapshots.pop_front().unwrap_or(RollbackSnapshot {
                core: Vec::new(),
                extras: SnapshotExtras::NONE,
            })
        } else {
            RollbackSnapshot {
                core: Vec::new(),
                extras: SnapshotExtras::NONE,
            }
        };
        match write(&mut snapshot.core) {
            Ok(()) => {
                snapshot.extras = extras;
                self.snapshots.push_back(snapshot);
                Ok(())
            }
            Err(error) => {
                if full {
                    self.snapshots.push_front(snapshot);
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
    pub fn take_rollback_target(&mut self) -> Option<RollbackSnapshot> {
        if self.snapshots.len() <= 1 {
            return self.snapshots.pop_back();
        }
        self.snapshots.pop_back();
        self.snapshots.pop_back()
    }

    /// The optional history panel of REQ-LOOP-033: restoring entry `index` (0 = oldest) keeps
    /// only the entries before it. Returns `None` when the index is out of range.
    pub fn take_at(&mut self, index: usize) -> Option<RollbackSnapshot> {
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
