//! The bound AMD-052 REQ-LOOP-077 puts on the rendered frames the GPU may keep waiting for
//! display: at `video.max_queued_frames` = 1 the program waits, before it starts drawing a
//! frame, for the previous one to be finished by the GPU (lowest latency); at 2 it may run one
//! frame ahead (smoother).

use std::rc::Rc;
use std::time::Instant;

use crate::gl::api::Gl;

/// The smallest depth the setting offers, and the lowest latency.
pub const MIN_DEPTH: i32 = 1;

/// The largest depth the setting offers, which is the default.
pub const MAX_DEPTH: i32 = 2;

/// How long a single `glClientWaitSync` call waits, in nanoseconds. A frame that takes longer
/// than a tenth of a second is waited for again; the bound only exists so that a driver that
/// loses a fence cannot hang the program for ever.
const WAIT_SLICE_NANOSECONDS: i32 = 100 * 1000 * 1000;

/// How many times a lost fence is waited for before it is given up on.
const WAIT_SLICES: u32 = 20;

/// Bounds the rendered frames the GPU has not finished yet (REQ-LOOP-077).
///
/// The instrument is the synchronisation object of GL 3.2 core and GLES 3.0, which REQ-SCN-056
/// allows without any extension: a fence placed after the last command of frame *n*
/// (`glFenceSync`) and queried with `glClientWaitSync`. The queue keeps the fences of the frames
/// the GPU has not confirmed yet, and [`Self::begin_frame`] blocks until no more than
/// `depth - 1` of them are left — so at depth 1 the fence of frame *n* is signalled before the
/// first command of frame *n + 1*, which is the acceptance criterion word for word, and at depth
/// 2 nothing is required of it.
///
/// This is presentation, hence the desktop frontend: the setting is **moot in libretro**, where
/// the swap chain and the presentation belong to the host (REQ-PERS-086) and the engine draws
/// into a target the host supplies (REQ-SCN-057).
///
/// The wait belongs **after** the emulated frames of the tick and before the first drawing
/// command, so that it cannot shift the cadence of the emulated frames (REQ-LOOP-010): the frame
/// clock is absolute — frame *n* is due at `t0 + n / f` — so a wait inside a rendered frame is
/// not a delay in the sense of REQ-LOOP-074 either, it only consumes some of the slack before
/// the next frame is due.
pub struct GpuFrameQueue {
    gl: Gl,
    /// At most [`MAX_DEPTH`] fences are ever outstanding, oldest first: the frame the GPU is
    /// drawing and, at depth 2, the one it has already been handed.
    fences: Vec<(glow::Fence, Instant)>,
    depth: i32,
    frame_started: Instant,
    waits: u64,
    last_latency: f64,
}

impl std::fmt::Debug for GpuFrameQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuFrameQueue")
            .field("depth", &self.depth)
            .field("outstanding", &self.fences.len())
            .field("waits", &self.waits)
            .finish_non_exhaustive()
    }
}

impl GpuFrameQueue {
    /// Creates a queue on `context`, which must be the current context of the calling thread for
    /// as long as the queue lives (the contract of the renderer's GL module).
    #[must_use]
    pub fn new(context: Rc<glow::Context>) -> Self {
        Self {
            gl: Gl::new(context),
            fences: Vec::with_capacity(2),
            depth: MAX_DEPTH,
            frame_started: Instant::now(),
            waits: 0,
            last_latency: 0.0,
        }
    }

    /// The depth in force, that is `video.max_queued_frames` clamped to the domain of
    /// REQ-LOOP-077.
    #[must_use]
    pub const fn depth(&self) -> i32 {
        self.depth
    }

    /// How many frames the GPU has not confirmed finished yet.
    #[must_use]
    pub fn outstanding(&self) -> usize {
        self.fences.len()
    }

    /// How many times [`Self::begin_frame`] has blocked on a fence.
    #[must_use]
    pub const fn waits(&self) -> u64 {
        self.waits
    }

    /// The latency of the last frame the GPU confirmed, in seconds: the real time between the
    /// moment the emulation of that frame was over — the start of its rendering — and the moment
    /// the program knew the frame was drawn and could be presented. This is the quantity the
    /// third acceptance criterion of REQ-LOOP-077 compares between the two depths; it is 0 until
    /// a frame has been confirmed.
    #[must_use]
    pub const fn last_latency_seconds(&self) -> f64 {
        self.last_latency
    }

    /// Called once per rendered frame, after the emulated frames of the tick and before the
    /// first drawing command. Blocks until at most `depth - 1` frames are still in the GPU's
    /// hands.
    pub fn begin_frame(&mut self, depth: i32) {
        self.depth = depth.clamp(MIN_DEPTH, MAX_DEPTH);
        self.frame_started = Instant::now();

        // The clamp above keeps the depth in 1..=2, so the conversion cannot fail.
        let depth = usize::try_from(self.depth).unwrap_or(MAX_DEPTH as usize);
        while self.fences.len() >= depth {
            self.retire_oldest();
        }
    }

    /// Called once per rendered frame, after its last drawing command. Places the fence the next
    /// [`Self::begin_frame`] waits on.
    pub fn end_frame(&mut self) {
        if let Some(fence) = self.gl.fence_sync() {
            self.fences.push((fence, self.frame_started));
        }
        // A driver that refuses a fence leaves the queue unbounded, which is the behaviour of
        // depth 2 on every frame: nothing else degrades.
    }

    /// Waits for every frame still in flight and drops their fences — what closing the window
    /// does, and what a test does before it reads a measurement back.
    pub fn flush(&mut self) {
        while !self.fences.is_empty() {
            self.retire_oldest();
        }
    }

    fn retire_oldest(&mut self) {
        if self.fences.is_empty() {
            return;
        }
        let (fence, started) = self.fences.remove(0);
        self.waits += 1;

        for _ in 0..WAIT_SLICES {
            if self.gl.wait_for_fence(fence, WAIT_SLICE_NANOSECONDS) {
                break;
            }
        }

        self.last_latency = started.elapsed().as_secs_f64();
        self.gl.delete_fence(fence);
    }
}

impl Drop for GpuFrameQueue {
    fn drop(&mut self) {
        self.flush();
    }
}
