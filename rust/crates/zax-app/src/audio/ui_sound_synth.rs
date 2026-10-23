//! The interface sounds synthesised in code, for a program installed without its audio assets.

use zax_platform::WaveAudio;

use super::chip_voices::{self, NoiseSource};
use super::ui_sound::UiSound;

/// Length of `sound` in seconds.
#[must_use]
pub const fn duration(sound: UiSound) -> f64 {
    match sound {
        UiSound::TabChange => 0.076,
        UiSound::PageChange => 0.060,
        UiSound::Move => 0.039,
        UiSound::Confirm => 0.209,
    }
}

/// Renders `sound` as a mono clip at `sample_rate` (at least 8000).
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // the C# casts of a positive duration times the rate; frames are far below 2^52
pub fn render(sound: UiSound, sample_rate: u32) -> WaveAudio {
    let rate = f64::from(sample_rate.max(8000));
    let seconds = duration(sound);
    let frames = ((seconds * rate).round() as usize).max(1);
    let mut samples = Vec::with_capacity(frames);
    let mut noise = NoiseSource::new(1 + sound as u16);
    let mut phase = 0.0_f64;
    let mut noise_phase = 0.0_f64;
    let mut noise_value = noise.next_sample();
    for frame in 0..frames {
        let time = frame as f64 / rate;
        let t = time / seconds;
        let value = match sound {
            // Selection: a bare triangle blip, high and dry. The quietest of the four, since it
            // fires on every movement of the cursor.
            UiSound::Move => {
                0.45 * chip_voices::triangle(phase)
                    * chip_voices::envelope(time, seconds, 0.001, 0.012)
            }
            // Page: a pulse falling a fifth, as "further back" in the list.
            UiSound::PageChange => {
                0.5 * chip_voices::pulse(phase, 0.5)
                    * chip_voices::envelope(time, seconds, 0.002, 0.022)
            }
            // Tab: a pulse rising a fifth, with a noise tick on the attack so that it reads as a
            // wider move than a page change.
            UiSound::TabChange => {
                (0.42
                    * chip_voices::pulse(phase, 0.25)
                    * chip_voices::envelope(time, seconds, 0.002, 0.030))
                    + (0.20 * noise_value * chip_voices::envelope(time, seconds, 0.0005, 0.006))
            }
            // Validation: two notes a fourth apart, the second louder: a coin, not a beep.
            UiSound::Confirm => {
                0.55 * chip_voices::pulse(phase, 0.5)
                    * chip_voices::envelope(time, seconds, 0.002, 0.090)
            }
        };
        samples.push(value.clamp(-1.0, 1.0));
        phase += pitch(sound, t) / rate;
        // The noise source is clocked well below the sample rate, which is what gives it a
        // grain rather than a hiss.
        noise_phase += 5500.0 / rate;
        if noise_phase >= 1.0 {
            noise_phase -= noise_phase.floor();
            noise_value = noise.next_sample();
        }
    }
    WaveAudio {
        samples,
        channels: 1,
        sample_rate,
    }
}

fn pitch(sound: UiSound, t: f64) -> f64 {
    match sound {
        UiSound::Move => chip_voices::frequency(93.0),
        UiSound::PageChange => chip_voices::frequency(86.0 - (7.0 * t)),
        UiSound::TabChange => chip_voices::frequency(79.0 + (7.0 * t)),
        UiSound::Confirm => chip_voices::frequency(if t < 0.33 { 83.0 } else { 88.0 }),
    }
}
