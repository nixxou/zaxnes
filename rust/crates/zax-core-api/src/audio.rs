//! A rejected audio setting or read, and the three bounds its messages name.
//!
//! Moved here from `zax_nes::apu` by lot FCEUX-23 step 2, because
//! [`Engine::read_audio`](crate::Engine::read_audio) is on the trait. The bounds came with it:
//! the `Display` of [`AudioError`] prints them, so they have to be where the error is.
//! `zax_nes::apu::Apu2A03` keeps its three associated constants, defined as these, so every
//! existing use site and every message is unchanged to the character.

/// Lowest sample rate the APU accepts, in Hz (`Apu2A03::MIN_SAMPLE_RATE`).
pub const MIN_SAMPLE_RATE: i32 = 22_050;

/// Highest sample rate the APU accepts, in Hz (`Apu2A03::MAX_SAMPLE_RATE`).
pub const MAX_SAMPLE_RATE: i32 = 96_000;

/// Largest fine ratio adjustment the APU accepts (`Apu2A03::MAX_RATIO_ADJUSTMENT`).
pub const MAX_RATIO_ADJUSTMENT: f64 = 0.005;

/// A rejected audio setting or read (the C# throws `ArgumentException` for these).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioError {
    /// The sample rate is outside [`MIN_SAMPLE_RATE`, `MAX_SAMPLE_RATE`].
    SampleRateOutOfRange(i32),
    /// The speed factor is zero or negative.
    SpeedNotPositive(f64),
    /// The fine ratio adjustment exceeds [`MAX_RATIO_ADJUSTMENT`].
    RatioAdjustmentOutOfRange(f64),
    /// A stereo buffer must hold an even number of values.
    OddStereoBuffer(usize),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SampleRateOutOfRange(rate) => write!(
                f,
                "sample rate {rate} is outside {MIN_SAMPLE_RATE}..={MAX_SAMPLE_RATE}"
            ),
            Self::SpeedNotPositive(speed) => write!(f, "speed {speed} must be positive"),
            Self::RatioAdjustmentOutOfRange(value) => write!(
                f,
                "ratio adjustment {value} exceeds ±{MAX_RATIO_ADJUSTMENT}"
            ),
            Self::OddStereoBuffer(len) => {
                write!(
                    f,
                    "a stereo buffer must hold an even number of values, not {len}"
                )
            }
        }
    }
}

impl std::error::Error for AudioError {}
