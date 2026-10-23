//! The program's own audio: the mix of the console, the menu music and the interface sounds
//! (SPEC-11).

pub mod audio_assets;
pub mod audio_mixer;
pub mod chip_voices;
pub mod fade_gain;
pub mod synth_audio_library;
pub mod ui_sound;
pub mod ui_sound_synth;

pub use audio_mixer::{AudioMixer, TrackRandom};
pub use ui_sound::{UiAudioLibrary, UiSound};
