//! The voices of the synthesised interface sounds: pulse, triangle and noise, in the manner of
//! the console's own channels.

/// The MIDI note of the concert A.
pub const CONCERT_A: i32 = 69;

/// Frequency in hertz of a MIDI note.
#[must_use]
pub fn frequency(midi_note: f64) -> f64 {
    440.0 * 2.0_f64.powf((midi_note - f64::from(CONCERT_A)) / 12.0)
}

/// A pulse wave of the given duty, -1 or 1.
#[must_use]
pub fn pulse(phase: f64, duty: f32) -> f32 {
    if fraction(phase) < f64::from(duty) {
        1.0
    } else {
        -1.0
    }
}

/// A triangle wave, -1 to 1.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // the C# `(float)` of the double sample
pub fn triangle(phase: f64) -> f32 {
    let f = fraction(phase);
    ((4.0 * (f - 0.5).abs()) - 1.0) as f32
}

/// The amplitude envelope of a note: a linear attack, an exponential decay and a 4 ms release
/// at the end of `length`.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // the C# `(float)` of the double levels
pub fn envelope(time: f64, length: f64, attack: f64, decay: f64) -> f32 {
    const RELEASE: f64 = 0.004;
    if time < 0.0 || time > length {
        return 0.0;
    }
    let mut level = if attack > 0.0 && time < attack {
        (time / attack) as f32
    } else {
        1.0
    };
    if decay > 0.0 {
        level *= (-(time - time.min(attack)) / decay).exp() as f32;
    }
    let remaining = length - time;
    if remaining < RELEASE {
        level *= (remaining / RELEASE) as f32;
    }
    level.clamp(0.0, 1.0)
}

/// The console's 15-bit noise register.
#[derive(Debug, Clone, Copy)]
pub struct NoiseSource {
    register: u16,
}

impl NoiseSource {
    /// Creates a source seeded with `seed` (0 reads as 1).
    #[must_use]
    pub const fn new(seed: u16) -> Self {
        Self {
            register: if seed == 0 { 1 } else { seed },
        }
    }

    /// The next sample, -1 or 1.
    pub fn next_sample(&mut self) -> f32 {
        let bit = (self.register ^ (self.register >> 1)) & 1;
        self.register = (self.register >> 1) | (bit << 14);
        if self.register & 1 == 0 {
            -1.0
        } else {
            1.0
        }
    }
}

fn fraction(value: f64) -> f64 {
    let f = value - value.floor();
    if f < 0.0 {
        f + 1.0
    } else {
        f
    }
}
