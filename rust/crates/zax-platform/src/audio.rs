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
//! whole block.
//!
//! The callback allocates nothing and locks nothing of its own: its scratch buffer is sized when
//! the device is opened, from the block size SDL negotiated. There is no `unsafe` here — the
//! `AudioCallback` trait of the `sdl2` crate is safe Rust.

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_clamped_and_rounded_to_even() {
        let mut pcm = [0i16; 6];
        to_pcm16(&[0.0, 1.0, -1.0, 2.0, -3.0, 0.5], &mut pcm);
        assert_eq!(pcm, [0, 32767, -32767, 32767, -32767, 16384]);
    }
}
