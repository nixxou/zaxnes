//! The program's own audio: the console's samples, the menu music and the interface sounds
//! (SPEC-11).
//!
//! The console is pulled from the APU's ring by the audio thread ([`stream`], Q-153); the menu
//! music and the interface sounds are produced ahead of time on the main thread by
//! [`audio_mixer`], which also owns the fades of SPEC-11 §3.3.

pub mod audio_assets;
pub mod audio_mixer;
pub mod chip_voices;
pub mod fade_gain;
pub mod stream;
pub mod synth_audio_library;
pub mod ui_sound;
pub mod ui_sound_synth;

pub use audio_mixer::{AudioMixer, TrackRandom};
pub use stream::{
    ApuSamples, AudioControl, AudioLink, AudioStream, ConsoleSamples, ConsoleSlot, ProgramWriter,
};
pub use ui_sound::{UiAudioLibrary, UiSound};
