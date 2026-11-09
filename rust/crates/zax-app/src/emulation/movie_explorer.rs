//! The explorer of a movie being played (EDITOR-1): a thread of its own that runs the whole movie
//! with a core of its own, the core alone (no conversion, no script, no sound), as fast as it
//! goes, and hands a checkpoint every [`CHECKPOINT_INTERVAL`] frames to the timeline. A seek far
//! ahead then starts from a checkpoint close to its target instead of running there.
//!
//! The explorer's core is built from the movie's start state (with its ROM image) and the
//! session's port types and widescreen margin; what the core keeps outside its state could still
//! differ, so the timeline trusts the explored checkpoints only once one of them has been found
//! to be the same machine as the session's own checkpoint of the same frame (frame and cycle
//! counts, work RAM, picture: [`super::movie_timeline::machine_digest`]).
//!
//! Only for a profile whose scripts leave the core alone, since the explorer runs none of them.
//! Dropping the explorer stops the thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread::JoinHandle;

use zax_core_api::ControllerPortType;
use zax_nes::NesCore;

use super::movie_timeline::{machine_digest, CHECKPOINT_INTERVAL};

/// What the explorer's core is made of.
#[derive(Debug, Clone)]
pub struct ExplorerSetup {
    /// The movie's start state, with its ROM image.
    pub start_state: Vec<u8>,
    /// The movie's input words.
    pub inputs: Vec<u16>,
    /// The session's controller ports.
    pub ports: [ControllerPortType; 2],
    /// The session's widescreen margin (a hack may read it).
    pub widescreen_margin: i32,
    /// The session's audio output rate, speed and drift ratio: the resampler's position is part
    /// of the core state (REQ-CORE-090), though nothing the emulation reads depends on it.
    pub audio: (i32, f64, f64),
}

/// The thread exploring a movie, and what it found so far.
pub struct MovieExplorer {
    found: Receiver<(i64, Vec<u8>, u64)>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MovieExplorer {
    /// Starts exploring.
    ///
    /// # Errors
    /// The thread could not be started.
    pub fn start(setup: ExplorerSetup) -> std::io::Result<Self> {
        let (sender, found) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("zaxnes-movie-explorer".to_owned())
            .spawn(move || {
                let Some(mut core) = explorer_core(&setup) else {
                    crate::app_log::line(
                        "Zaxnes: movie explorer: the movie's start state does not load.",
                    );
                    return;
                };
                let started = std::time::Instant::now();
                for &word in &setup.inputs {
                    if stopped.load(Ordering::Relaxed) {
                        return;
                    }
                    core.run_frame(word);
                    let frame_count = core.frame_count();
                    if frame_count % CHECKPOINT_INTERVAL == 0 {
                        let Ok(state) = core.save_state(false) else {
                            return;
                        };
                        let digest = machine_digest(&core);
                        if sender.send((frame_count, state, digest)).is_err() {
                            return;
                        }
                    }
                }
                crate::app_log::line(&format!(
                    "Zaxnes: movie explorer: {} frames explored in {:.1} s.",
                    setup.inputs.len(),
                    started.elapsed().as_secs_f64()
                ));
            })?;
        Ok(Self {
            found,
            stop,
            thread: Some(thread),
        })
    }

    /// The checkpoints found since the last call, with their machine digests.
    #[must_use]
    pub fn take_found(&self) -> Vec<(i64, Vec<u8>, u64)> {
        self.found.try_iter().collect()
    }
}

impl Drop for MovieExplorer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The explorer's core: the start state, the session's ports and margin, no sound.
fn explorer_core(setup: &ExplorerSetup) -> Option<NesCore> {
    let mut core = NesCore::new();
    core.try_load_state(&setup.start_state, 0).ok()?;
    for (port, kind) in setup.ports.iter().enumerate() {
        core.set_port_type(port, *kind).ok()?;
    }
    core.set_widescreen_margin(setup.widescreen_margin);
    // The sound is never heard here: a value the core refuses is left at its default.
    let (rate, speed, ratio) = setup.audio;
    let _ = core.set_sample_rate(rate);
    let _ = core.set_speed(speed);
    let _ = core.set_audio_ratio_adjustment(ratio);
    core.bus_mut().apu_mut().set_output_enabled(false);
    Some(core)
}
