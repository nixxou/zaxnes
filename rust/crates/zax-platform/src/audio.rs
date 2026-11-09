//! The audio device of SPEC-11 §3.1, through an SDL playback callback.
//!
//! 48 000 Hz and blocks of 512 frames (REQ-AUD-001, REQ-AUD-003). The core produces its samples
//! directly at the output rate, so no resampling happens here (SPEC-02 REQ-CORE-070).
//!
//! The device is deliberately dumb, and it *pulls*: SDL calls [`AudioRender::render`] on its own
//! audio thread whenever it needs a block, and the renderer — the stream of `zax-app`, which
//! owns the gains, the fades, the menu music and the interface sounds — reads the core's sample
//! ring itself. The emulation is never clocked by the audio (REQ-AUD-004), and a long rendered
//! frame on the main thread can no longer starve the device: nothing on the main thread stands
//! between the core's samples and the driver (Q-153).
//!
//! This replaces the push model of v0.4.2, where the main thread topped an `AudioQueue` up once
//! per rendered frame and a block of zeros went out whenever the core's ring held less than one
//! whole block. That model is still here, as [`ClassicAudioOutput`]: AUDIO-2 keeps it as the
//! fallback `audio.classic_output` asks for, for a device the callback path misbehaves on. The
//! two are never open at the same time, and which one opens is decided once, by the setting, when
//! the device opens.
//!
//! The callback allocates nothing and locks nothing of its own: its scratch buffer is sized when
//! the device is opened, from the block size SDL negotiated. There is no `unsafe` here — the
//! `AudioCallback` trait of the `sdl2` crate is safe Rust.

use sdl2::audio::{AudioCallback, AudioDevice, AudioQueue, AudioSpecDesired};

use crate::PlatformError;

/// Preferred output rate, 48 000 Hz (REQ-AUD-001).
pub const PREFERRED_SAMPLE_RATE: i32 = 48_000;
/// Frames per output block, about 10.7 ms at 48 kHz (REQ-AUD-003).
pub const BLOCK_FRAMES: usize = 512;
/// Channels of the output: stereo, the console signal duplicated (REQ-AUD-002).
pub const CHANNELS: usize = 2;

/// Converts float samples to 16-bit PCM: clamped to `[-1 ; 1]`, scaled by 32767 and rounded to
/// even, as the C# `MathF.Round` does.
pub fn to_pcm16(samples: &[f32], pcm: &mut [i16]) {
    for (out, &value) in pcm.iter_mut().zip(samples) {
        // The clamp bounds the product to ±32767, so the cast cannot truncate.
        #[allow(clippy::cast_possible_truncation)]
        let converted = (value.clamp(-1.0, 1.0) * 32767.0).round_ties_even() as i16;
        *out = converted;
    }
}

/// What the device pulls from, on SDL's audio thread.
///
/// `render` is handed an interleaved stereo slice and must fill **all** of it. It runs on a
/// real-time thread: it must not allocate, must not read a file and must not block on anything
/// the main thread can hold for long.
pub trait AudioRender: Send {
    /// Fills `stereo` — pairs of left and right samples — with the next frames of the mix.
    fn render(&mut self, stereo: &mut [f32]);
}

impl AudioRender for Box<dyn AudioRender> {
    fn render(&mut self, stereo: &mut [f32]) {
        (**self).render(stereo);
    }
}

/// The SDL side of the pull: converts what the renderer produced to the device's 16-bit frames.
struct Callback {
    render: Box<dyn AudioRender>,
    scratch: Vec<f32>,
}

impl AudioCallback for Callback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        // The scratch buffer is one negotiated block; SDL asks for exactly that, but a driver
        // that asks for more is served in several passes rather than by a reallocation here.
        for chunk in out.chunks_mut(self.scratch.len().max(2)) {
            let samples = &mut self.scratch[..chunk.len()];
            self.render.render(samples);
            to_pcm16(samples, chunk);
        }
    }
}

/// The audio output: an open SDL playback device driven by an [`AudioRender`].
pub struct AudioOutput {
    device: AudioDevice<Callback>,
    sample_rate: i32,
    block_frames: usize,
}

impl AudioOutput {
    /// Opens the default device and starts pulling from the renderer `make` builds. A machine
    /// without a sound device gets an error rather than a panic, so that it can still run the
    /// renderer of the picture.
    ///
    /// The device is opened at [`PREFERRED_SAMPLE_RATE`] and [`BLOCK_FRAMES`] if it accepts
    /// them, otherwise at whatever it offers (REQ-AUD-001). `make` is called once, on this
    /// thread, with the rate and the block size that were actually negotiated — the renderer
    /// needs both to size its buffers and its latency — and whatever it returns beside the
    /// renderer is handed back to the caller, which is how the main thread keeps its handles on
    /// a stream the device now owns.
    ///
    /// # Errors
    /// [`PlatformError`] if SDL cannot open an audio device.
    pub fn open<T>(
        sdl: &sdl2::Sdl,
        make: impl FnOnce(i32, usize) -> (Box<dyn AudioRender>, T),
    ) -> Result<(Self, T), PlatformError> {
        let audio = sdl.audio()?;
        let desired = AudioSpecDesired {
            freq: Some(PREFERRED_SAMPLE_RATE),
            // `CHANNELS` is 2.
            channels: Some(2),
            // `BLOCK_FRAMES` is 512.
            samples: Some(512),
        };
        let mut negotiated = (PREFERRED_SAMPLE_RATE, BLOCK_FRAMES);
        let mut kept = None;
        let device = audio.open_playback(None, &desired, |spec| {
            let block_frames = usize::from(spec.samples).max(1);
            let channels = usize::from(spec.channels).max(1);
            negotiated = (spec.freq, block_frames);
            let (render, extra) = make(spec.freq, block_frames);
            kept = Some(extra);
            Callback {
                render,
                scratch: vec![0.0; block_frames * channels],
            }
        })?;
        let Some(kept) = kept else {
            return Err(PlatformError::new(
                "the audio device opened without asking for a renderer",
            ));
        };
        // SDL opens a device paused. The stream's own prefill gate decides when sound starts,
        // so the device can run from here on (Q-153).
        device.resume();
        Ok((
            Self {
                device,
                sample_rate: negotiated.0,
                block_frames: negotiated.1,
            },
            kept,
        ))
    }

    /// The rate the device was opened at.
    #[must_use]
    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    /// Frames the device asks for per callback.
    #[must_use]
    pub fn block_frames(&self) -> usize {
        self.block_frames
    }

    /// Stops the callbacks; the device goes silent until [`resume`](Self::resume).
    pub fn pause(&self) {
        self.device.pause();
    }

    /// Starts the callbacks again.
    pub fn resume(&self) {
        self.device.resume();
    }
}

// ------------------------------------------------------------------ the classic output of v0.4.2

/// Blocks kept queued by [`ClassicAudioOutput`]; 4 × 512 frames is about 43 ms of latency.
pub const BLOCK_COUNT: usize = 4;

const BLOCK_VALUES: usize = BLOCK_FRAMES * CHANNELS;
const BLOCK_BYTES: usize = BLOCK_VALUES * std::mem::size_of::<i16>();

/// How many blocks [`ClassicAudioOutput::pump`] queues when `queued_bytes` are still waiting in
/// the device: enough to keep [`BLOCK_COUNT`] blocks pending, counting a partly played block as
/// pending.
#[must_use]
pub fn blocks_to_queue(queued_bytes: usize) -> usize {
    BLOCK_COUNT.saturating_sub(queued_bytes.div_ceil(BLOCK_BYTES))
}

/// The output `v0.4.2` had, kept as the fallback of `audio.classic_output` (AUDIO-2).
///
/// It *pushes*: the main thread tops an SDL queue up to [`BLOCK_COUNT`] blocks once per rendered
/// frame, asking a filler — the mixer of SPEC-11, which owns the gains, the fades, the menu music,
/// the interface sounds and the all-or-nothing rule of REQ-AUD-003 — for a whole interleaved
/// stereo block each time. The emulation is never clocked by the audio (REQ-AUD-004). There is no
/// audio thread of ours: SDL drains the queue on its own, and the two buffers below are allocated
/// once.
///
/// Its defects are the ones Q-153 records and AUDIO-1 fixed, and they are the reason it is not the
/// default: a block of zeros goes out whenever the core's ring holds less than one whole block,
/// and four blocks of 512 frames are at or past what a 3D main thread needs. Nothing here is a new
/// design — it is `v0.4.2`'s file, so that a device the callback path misbehaves on has the exact
/// behaviour it had before AUDIO-1.
pub struct ClassicAudioOutput {
    queue: AudioQueue<i16>,
    samples: Vec<f32>,
    block: Vec<i16>,
    sample_rate: i32,
    blocks: u64,
}

impl ClassicAudioOutput {
    /// Opens the default device. A machine without a sound device gets an error rather than a
    /// panic, so that it can still run the renderer.
    ///
    /// # Errors
    /// [`PlatformError`] if SDL cannot open an audio device.
    pub fn open(sdl: &sdl2::Sdl) -> Result<Self, PlatformError> {
        let audio = sdl.audio()?;
        let desired = AudioSpecDesired {
            freq: Some(PREFERRED_SAMPLE_RATE),
            // `CHANNELS` is 2.
            channels: Some(2),
            // `BLOCK_FRAMES` is 512.
            samples: Some(512),
        };
        let queue = audio.open_queue::<i16, _>(None, &desired)?;
        let sample_rate = queue.spec().freq;
        queue.resume();
        Ok(Self {
            queue,
            samples: vec![0.0; BLOCK_VALUES],
            block: vec![0; BLOCK_VALUES],
            sample_rate,
            blocks: 0,
        })
    }

    /// The rate the device was opened at.
    #[must_use]
    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    /// Number of blocks handed to the device since it was opened.
    #[must_use]
    pub fn blocks(&self) -> u64 {
        self.blocks
    }

    /// Keeps the device fed: as long as fewer than [`BLOCK_COUNT`] blocks are pending, asks
    /// `fill` for one interleaved stereo block and queues it. Called once per rendered frame.
    ///
    /// `fill` fills the whole slice — `512 × 2` values — with interleaved stereo audio. The
    /// mixer always produces a complete block, using silence where the core has nothing
    /// (REQ-AUD-003).
    ///
    /// # Errors
    /// [`PlatformError`] if SDL refuses the block.
    ///
    /// Returns how many blocks were queued.
    pub fn pump(&mut self, mut fill: impl FnMut(&mut [f32])) -> Result<usize, PlatformError> {
        let wanted = blocks_to_queue(self.queue.size() as usize);
        for _ in 0..wanted {
            fill(&mut self.samples);
            to_pcm16(&self.samples, &mut self.block);
            self.queue.queue_audio(&self.block)?;
            self.blocks += 1;
        }
        Ok(wanted)
    }

    /// Drops everything queued: what a game load, a state load or a reset does (REQ-AUD-010).
    pub fn flush(&mut self) {
        self.queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_clamped_and_rounded_to_even() {
        let mut pcm = [0i16; 6];
        to_pcm16(&[0.0, 1.0, -1.0, 2.0, -3.0, 0.5], &mut pcm);
        assert_eq!(pcm, [0, 32767, -32767, 32767, -32767, 16384]);
    }

    /// The classic output's rule, unchanged from `v0.4.2`: top the queue up to four blocks, a
    /// partly played block counting as pending.
    #[test]
    fn the_classic_queue_is_kept_at_four_blocks() {
        assert_eq!(blocks_to_queue(0), BLOCK_COUNT);
        assert_eq!(blocks_to_queue(1), BLOCK_COUNT - 1);
        assert_eq!(blocks_to_queue(BLOCK_BYTES), BLOCK_COUNT - 1);
        assert_eq!(blocks_to_queue(BLOCK_BYTES * BLOCK_COUNT), 0);
        assert_eq!(blocks_to_queue(BLOCK_BYTES * 10), 0);
    }
}
