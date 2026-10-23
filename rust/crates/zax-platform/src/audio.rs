//! The audio device of SPEC-11 §3.1, through an SDL audio queue.
//!
//! 48 000 Hz and blocks of 512 frames (REQ-AUD-001, REQ-AUD-003). The core produces its samples
//! directly at the output rate, so no resampling happens here (SPEC-02 REQ-CORE-070).
//!
//! The device is deliberately dumb: it asks a filler — the mixer of SPEC-11, which owns the
//! gains, the fades, the menu music, the interface sounds and the all-or-nothing rule of
//! REQ-AUD-003 — for interleaved stereo blocks and queues them. The emulation is never clocked
//! by the audio (REQ-AUD-004). There is no audio thread of ours: SDL drains the queue on its
//! own, and the two buffers below are allocated once.

use sdl2::audio::{AudioQueue, AudioSpecDesired};

use crate::PlatformError;

/// Preferred output rate, 48 000 Hz (REQ-AUD-001).
pub const PREFERRED_SAMPLE_RATE: i32 = 48_000;
/// Frames per output block, about 10.7 ms at 48 kHz (REQ-AUD-003).
pub const BLOCK_FRAMES: usize = 512;
/// Channels of the output: stereo, the console signal duplicated (REQ-AUD-002).
pub const CHANNELS: usize = 2;
/// Blocks kept queued; 4 × 512 frames is about 43 ms of latency.
pub const BLOCK_COUNT: usize = 4;

const BLOCK_VALUES: usize = BLOCK_FRAMES * CHANNELS;
const BLOCK_BYTES: usize = BLOCK_VALUES * std::mem::size_of::<i16>();

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

/// How many blocks to queue when `queued_bytes` are still waiting in the device: enough to
/// keep [`BLOCK_COUNT`] blocks pending, counting a partly played block as pending.
#[must_use]
pub fn blocks_to_queue(queued_bytes: usize) -> usize {
    BLOCK_COUNT.saturating_sub(queued_bytes.div_ceil(BLOCK_BYTES))
}

/// The audio output.
pub struct AudioOutput {
    queue: AudioQueue<i16>,
    samples: Vec<f32>,
    block: Vec<i16>,
    sample_rate: i32,
    blocks: u64,
}

impl AudioOutput {
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

    #[test]
    fn the_queue_is_kept_at_four_blocks() {
        assert_eq!(blocks_to_queue(0), BLOCK_COUNT);
        assert_eq!(blocks_to_queue(1), BLOCK_COUNT - 1);
        assert_eq!(blocks_to_queue(BLOCK_BYTES), BLOCK_COUNT - 1);
        assert_eq!(blocks_to_queue(BLOCK_BYTES * BLOCK_COUNT), 0);
        assert_eq!(blocks_to_queue(BLOCK_BYTES * 10), 0);
    }
}
