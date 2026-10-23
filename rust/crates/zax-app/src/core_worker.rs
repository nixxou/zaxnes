//! The emulation thread of the "emulation on a separate core" setting (Rust-only, handhelds).
//!
//! The session hands its core to the thread for one emulated frame and takes it back before
//! anything else reads it: the core is moved, never shared, so the emulation and the scripts
//! still see each other in the order of the single-threaded loop. Only the time at which the
//! frame runs changes: it runs while the main thread draws the previous one.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use zax_nes::NesCore;

/// One frame to run: the core and the input word of the frame.
struct Job {
    core: Box<NesCore>,
    word: u16,
}

/// A frame the thread ran: the core back, and the time the frame took.
pub struct FinishedFrame {
    /// The core, which the session owns again.
    pub core: Box<NesCore>,
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
                while let Ok(Job { mut core, word }) = inbox.recv() {
                    let started = Instant::now();
                    core.run_frame(word);
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

    /// Hands `core` to the thread to run one frame with `word`. Gives the core back when the
    /// thread is gone.
    ///
    /// # Errors
    ///
    /// The thread has stopped; the core is returned untouched.
    pub fn start(&self, core: Box<NesCore>, word: u16) -> Result<(), Box<NesCore>> {
        match &self.jobs {
            Some(jobs) => jobs.send(Job { core, word }).map_err(|error| error.0.core),
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
