//! The emulation thread of the "emulation on a separate core" setting (Rust-only, handhelds).
//!
//! The session hands its core to the thread for one emulated frame and takes it back before
//! anything else reads it: the core is moved, never shared, so the emulation and the scripts
//! still see each other in the order of the single-threaded loop. Only the time at which the
//! frame runs changes: it runs while the main thread draws the previous one.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::emulation::engine_handle::EngineHandle;

/// One frame to run: the engine, the input word of the frame, and the run-ahead in force.
struct Job {
    core: EngineHandle,
    word: u16,
    /// Frames of run-ahead (AMD-041 REQ-LOOP-064), 0 for an ordinary frame. The whole sequence
    /// of REQ-LOOP-065 runs on this thread, which is the unit the thread already was: one real
    /// frame. The core comes back in the **displayed** frame's state, and the session puts the
    /// real one back once it has read that frame (`GameSession::capture_in_flight`).
    run_ahead: u8,
}

/// A frame the thread ran: the core back, and the time the frame took.
pub struct FinishedFrame {
    /// The core, which the session owns again.
    pub core: EngineHandle,
    /// How long `NesCore::run_frame` took on the emulation thread.
    pub elapsed: Duration,
}

/// The emulation thread.
pub struct CoreWorker {
    jobs: Option<Sender<Job>>,
    finished: Receiver<FinishedFrame>,
    thread: Option<JoinHandle<()>>,
}

impl CoreWorker {
    /// Starts the thread.
    ///
    /// # Errors
    ///
    /// The operating system refused to create the thread.
    pub fn spawn() -> std::io::Result<Self> {
        let (jobs, inbox) = channel::<Job>();
        let (outbox, finished) = channel();
        let thread = thread::Builder::new()
            .name("zaxnes-emulation".to_owned())
            .spawn(move || {
                while let Ok(Job {
                    mut core,
                    word,
                    run_ahead,
                }) = inbox.recv()
                {
                    let started = Instant::now();
                    core.run_frame_ahead(word, run_ahead);
                    let frame = FinishedFrame {
                        core,
                        elapsed: started.elapsed(),
                    };
                    if outbox.send(frame).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            jobs: Some(jobs),
            finished,
            thread: Some(thread),
        })
    }

    /// Hands `core` to the thread to run one real frame with `word` and `run_ahead` frames in
    /// advance (AMD-041 REQ-LOOP-065; 0 for an ordinary frame). Gives the engine back when the
    /// thread is gone.
    ///
    /// The engine *moves* to the thread, which is what makes `Engine: Send` enough and `Sync`
    /// unnecessary: FCEUX keeps one machine in process globals, so two threads must never be
    /// inside it, and this hands it over rather than sharing it (lot FCEUX-23 step 4).
    ///
    /// # Errors
    ///
    /// The thread has stopped; the core is returned untouched.
    pub fn start(&self, core: EngineHandle, word: u16, run_ahead: u8) -> Result<(), EngineHandle> {
        match &self.jobs {
            Some(jobs) => jobs
                .send(Job {
                    core,
                    word,
                    run_ahead,
                })
                .map_err(|error| error.0.core),
            None => Err(core),
        }
    }

    /// Waits for the frame started last and takes the core back; `None` when the thread died
    /// during the frame (the core is lost with it).
    #[must_use]
    pub fn finish(&self) -> Option<FinishedFrame> {
        self.finished.recv().ok()
    }
}

impl Drop for CoreWorker {
    fn drop(&mut self) {
        // Closing the job channel ends the thread's loop.
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                // The thread panicked; there is nothing left to clean up.
            }
        }
    }
}

impl std::fmt::Debug for CoreWorker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CoreWorker")
    }
}
