//! The library of a program without audio assets: the interface sounds synthesised, no music.

use zax_platform::audio::PREFERRED_SAMPLE_RATE;
use zax_platform::WaveAudio;

use super::ui_sound::{UiAudioLibrary, UiSound};
use super::ui_sound_synth;

/// The four sounds, rendered once.
#[derive(Debug, Clone)]
pub struct SynthAudioLibrary {
    sounds: Vec<(UiSound, WaveAudio)>,
}

impl SynthAudioLibrary {
    /// Renders the sounds at `sample_rate`.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sounds: UiSound::ALL
                .iter()
                .map(|sound| (*sound, ui_sound_synth::render(*sound, sample_rate)))
                .collect(),
        }
    }
}

impl Default for SynthAudioLibrary {
    fn default() -> Self {
        Self::new(u32::try_from(PREFERRED_SAMPLE_RATE).unwrap_or(48_000))
    }
}

impl UiAudioLibrary for SynthAudioLibrary {
    fn track_count(&self) -> usize {
        0
    }

    fn sound(&self, sound: UiSound) -> Option<&WaveAudio> {
        self.sounds
            .iter()
            .find(|(candidate, _)| *candidate == sound)
            .map(|(_, clip)| clip)
    }

    fn load_track(&self, _index: usize) -> Option<WaveAudio> {
        None
    }
}
