//! The mix the audio device plays (SPEC-11): the console, the menu music and the interface
//! sounds, with the fades of REQ-AUD-020 to REQ-AUD-023.

use zax_platform::audio::BLOCK_FRAMES;
use zax_platform::WaveAudio;

use super::fade_gain::FadeGain;
use super::ui_sound::{UiAudioLibrary, UiSound};

/// Frames of one block the device asks for.
pub const BLOCK: usize = BLOCK_FRAMES;
/// Gain of the menu music relative to the volume.
pub const MUSIC_GAIN_FACTOR: f32 = 0.02;
/// Gain of the interface sounds relative to the volume.
pub const SOUND_GAIN_FACTOR: f32 = 0.07;

/// Where the console's samples come from: fills the mono buffer entirely and returns its length,
/// or returns less (REQ-AUD-003: all or nothing).
pub type AudioSampleSource = Box<dyn FnMut(&mut [f32]) -> usize>;

/// A small deterministic generator for the choice of the menu music track.
#[derive(Debug, Clone, Copy)]
pub struct TrackRandom {
    state: u64,
}

impl TrackRandom {
    /// Creates a generator from `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// A number below `bound` (`SplitMix64`).
    pub fn next_below(&mut self, bound: usize) -> usize {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        // The remainder is below `bound`, which is a `usize`.
        usize::try_from(z % (bound.max(1) as u64)).unwrap_or(0)
    }
}

/// The mixer. It is driven from the host's thread: the audio device pulls blocks from
/// [`fill`](Self::fill), the loop sets the state around it.
#[allow(clippy::struct_excessive_bools)] // independent state flags of the mix, as the C# class
pub struct AudioMixer {
    sample_rate: u32,
    emulation_gain: FadeGain,
    music_gain: FadeGain,
    assets: Option<Box<dyn UiAudioLibrary>>,
    random: TrackRandom,
    mono: Vec<f32>,
    volume: f32,
    menu_music_enabled: bool,
    emulation_audible: bool,
    ever_ran: bool,
    track: Option<WaveAudio>,
    track_index: Option<usize>,
    track_position: f64,
    track_suspended: bool,
    sound: Option<WaveAudio>,
    sound_position: f64,
    emulation_source: Option<AudioSampleSource>,
    /// Blocks the console could not fill while it was audible.
    pub underrun_blocks: u64,
    /// Blocks mixed.
    pub blocks: u64,
    /// How many times a menu track was started.
    pub track_changes: u32,
    /// How many interface sounds were started.
    pub sounds_played: u64,
}

impl AudioMixer {
    /// Creates a mixer at `sample_rate`, drawing the menu tracks from `assets`.
    #[must_use]
    pub fn new(sample_rate: u32, assets: Option<Box<dyn UiAudioLibrary>>, random: TrackRandom) -> Self {
        let volume = 1.0_f32;
        let mut mixer = Self {
            sample_rate: sample_rate.max(1),
            // REQ-AUD-020: the first launch of a game starts at gain V, since no previous fade
            // has lowered it, so the console gain starts settled at V and only fades once a game
            // has actually run. REQ-AUD-022: the music starts at 0 and rises with the same slope.
            emulation_gain: FadeGain::new(f64::from(volume)),
            music_gain: FadeGain::new(0.0),
            assets,
            random,
            mono: vec![0.0; BLOCK],
            volume,
            menu_music_enabled: true,
            emulation_audible: false,
            ever_ran: false,
            track: None,
            track_index: None,
            track_position: 0.0,
            track_suspended: false,
            sound: None,
            sound_position: 0.0,
            emulation_source: None,
            underrun_blocks: 0,
            blocks: 0,
            track_changes: 0,
            sounds_played: 0,
        };
        mixer.fade_emulation();
        mixer.fade_music();
        mixer
    }

    /// The sample rate of the mix.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Sets where the console's samples come from, or removes the source.
    pub fn set_emulation_source(&mut self, source: Option<AudioSampleSource>) {
        self.emulation_source = source;
    }

    /// The volume, 0 to 1.
    #[must_use]
    pub const fn volume(&self) -> f32 {
        self.volume
    }

    /// Sets the volume. The change is immediate when the gain had settled, and does not cut a
    /// fade that is still running (REQ-AUD-006, REQ-AUD-023).
    pub fn set_volume(&mut self, value: f32) {
        let clamped = value.clamp(0.0, 1.0);
        if (clamped - self.volume).abs() < 1e-7 {
            return;
        }
        self.volume = clamped;
        let emulation = self.emulation_target();
        self.emulation_gain.retarget(emulation, f64::from(self.volume));
        let (music, range) = (self.music_target(), self.music_range());
        self.music_gain.retarget(music, range);
    }

    /// Whether the menu music is on.
    #[must_use]
    pub const fn menu_music_enabled(&self) -> bool {
        self.menu_music_enabled
    }

    /// Turns the menu music on or off, bringing its gain to the new target at once, with no
    /// fade (REQ-AUD-023).
    pub fn set_menu_music_enabled(&mut self, value: bool) {
        if self.menu_music_enabled == value {
            return;
        }
        self.menu_music_enabled = value;
        let (target, range) = (self.music_target(), self.music_range());
        self.music_gain.snap(target, range);
        if value {
            self.track_suspended = false;
        }
    }

    /// Whether the console is audible: the emulation is running.
    #[must_use]
    pub const fn emulation_audible(&self) -> bool {
        self.emulation_audible
    }

    /// Makes the console audible or not; the change starts a fade (REQ-AUD-020, REQ-AUD-022).
    pub fn set_emulation_audible(&mut self, value: bool) {
        if self.emulation_audible == value {
            return;
        }
        self.emulation_audible = value;
        self.ever_ran |= value;
        // REQ-AUD-022: leaving Running makes the menu music active again, so the track has to
        // come off suspension, otherwise the menu stays silent for the rest of the session even
        // though the music gain climbs back to its target.
        if !value {
            self.track_suspended = false;
        }
        self.fade_emulation();
        self.fade_music();
    }

    /// The gain of the console.
    #[must_use]
    pub const fn emulation_gain(&self) -> f32 {
        self.emulation_gain.value()
    }

    /// The gain of the menu music.
    #[must_use]
    pub const fn music_gain(&self) -> f32 {
        self.music_gain.value()
    }

    /// The gain of the interface sounds.
    #[must_use]
    pub fn sound_gain(&self) -> f32 {
        SOUND_GAIN_FACTOR * self.volume
    }

    /// The index of the menu track playing, if any.
    #[must_use]
    pub const fn current_track(&self) -> Option<usize> {
        self.track_index
    }

    /// Whether the menu track is held at its position while the console is audible.
    #[must_use]
    pub const fn music_suspended(&self) -> bool {
        self.track_suspended
    }

    /// Whether an interface sound is playing.
    #[must_use]
    pub const fn sound_playing(&self) -> bool {
        self.sound.is_some()
    }

    /// Plays an interface sound, replacing the one that is playing.
    pub fn play_sound(&mut self, sound: UiSound) {
        let clip = self
            .assets
            .as_ref()
            .and_then(|assets| assets.sound(sound))
            .cloned();
        self.play_clip(clip);
    }

    /// Plays `clip`, replacing the sound that is playing; an empty or missing clip is ignored.
    pub fn play_clip(&mut self, clip: Option<WaveAudio>) {
        let Some(clip) = clip.filter(|clip| clip.frame_count() > 0) else {
            return;
        };
        self.sound = Some(clip);
        self.sound_position = 0.0;
        self.sounds_played += 1;
    }

    /// Stops the interface sound.
    pub fn stop_sound(&mut self) {
        self.sound = None;
        self.sound_position = 0.0;
    }

    /// Mixes one block of interleaved stereo samples.
    pub fn fill(&mut self, stereo: &mut [f32]) {
        if stereo.len() < 2 {
            return;
        }
        let frames = stereo.len() / 2;
        stereo.fill(0.0);
        self.mix_emulation(stereo, frames);
        self.mix_music(stereo, frames);
        self.mix_sound(stereo, frames);
        for value in stereo.iter_mut() {
            *value = value.clamp(-1.0, 1.0);
        }
        #[allow(clippy::cast_precision_loss)] // a block is a few hundred frames
        self.advance(frames as f64 / f64::from(self.sample_rate));
        self.blocks += 1;
    }

    /// Advances the fades by `real_seconds`.
    pub fn advance(&mut self, real_seconds: f64) {
        self.emulation_gain.advance(real_seconds);
        self.music_gain.advance(real_seconds);
        // REQ-AUD-022: once the fade-out has reached 0 the track is suspended, and DEC-11-Q1
        // says it resumes at that position.
        if !self.music_active() && self.music_gain.value() <= 0.0 {
            self.track_suspended = true;
        }
    }

    const fn music_active(&self) -> bool {
        !self.emulation_audible
    }

    fn music_target(&self) -> f64 {
        if self.music_active() && self.menu_music_enabled {
            f64::from(MUSIC_GAIN_FACTOR * self.volume)
        } else {
            0.0
        }
    }

    fn music_range(&self) -> f64 {
        f64::from(MUSIC_GAIN_FACTOR * self.volume)
    }

    fn emulation_target(&self) -> f64 {
        if self.emulation_audible || !self.ever_ran {
            f64::from(self.volume)
        } else {
            0.0
        }
    }

    fn fade_emulation(&mut self) {
        let target = self.emulation_target();
        self.emulation_gain.fade_to(target, f64::from(self.volume));
    }

    fn fade_music(&mut self) {
        let (target, range) = (self.music_target(), self.music_range());
        self.music_gain.fade_to(target, range);
    }

    fn mix_emulation(&mut self, stereo: &mut [f32], frames: usize) {
        let Some(source) = self.emulation_source.as_mut() else {
            return;
        };
        if self.mono.len() < frames {
            self.mono.resize(frames, 0.0);
        }
        let mono = &mut self.mono[..frames];
        let written = source(mono);
        if written < frames {
            // REQ-AUD-003: all or nothing. The samples the core did have stay in its buffer for
            // the next call, so nothing is lost and no block is half filled. Outside Running no
            // frame produces samples, so an empty buffer is the expected silence of REQ-AUD-010,
            // not an under-run.
            if self.emulation_audible {
                self.underrun_blocks += 1;
            }
            return;
        }
        let gain = self.emulation_gain.value();
        for (pair, value) in stereo.chunks_exact_mut(2).zip(mono.iter()) {
            let scaled = value * gain;
            pair[0] += scaled;
            pair[1] += scaled;
        }
    }

    fn mix_music(&mut self, stereo: &mut [f32], frames: usize) {
        if self.track_suspended {
            return;
        }
        let track_count = self.assets.as_ref().map_or(0, |assets| assets.track_count());
        if self.track.is_none() && self.music_active() && track_count > 0 {
            self.select_track();
        }
        let gain = self.music_gain.value();
        for pair in stereo.chunks_exact_mut(2).take(frames) {
            let Some(track) = self.track.as_ref() else {
                return;
            };
            if self.track_position >= frames_f64(track.frame_count()) - 1.0 {
                // REQ-AUD-030: a track does not loop; while the music is active a different one
                // starts straight away.
                if self.music_active() && track_count > 1 {
                    self.select_track();
                } else {
                    self.track = None;
                    self.track_index = None;
                    return;
                }
            }
            let Some(track) = self.track.as_ref() else {
                return;
            };
            let ratio = f64::from(track.sample_rate) / f64::from(self.sample_rate);
            let (left, right) = sample(&track.samples, usize::from(track.channels), track.frame_count(), self.track_position);
            pair[0] += left * gain;
            pair[1] += right * gain;
            self.track_position += ratio;
        }
    }

    fn mix_sound(&mut self, stereo: &mut [f32], frames: usize) {
        let gain = self.sound_gain();
        for pair in stereo.chunks_exact_mut(2).take(frames) {
            let Some(clip) = self.sound.as_ref() else {
                return;
            };
            if self.sound_position >= frames_f64(clip.frame_count()) - 1.0 {
                self.stop_sound();
                return;
            }
            let ratio = f64::from(clip.sample_rate) / f64::from(self.sample_rate);
            let (left, right) = sample(&clip.samples, usize::from(clip.channels), clip.frame_count(), self.sound_position);
            pair[0] += left * gain;
            pair[1] += right * gain;
            self.sound_position += ratio;
        }
    }

    /// Chooses the next menu track: a random one at start-up, then a random one among the
    /// others, so that the same track never plays twice in a row (REQ-AUD-030).
    fn select_track(&mut self) {
        let Some(assets) = self.assets.as_ref() else {
            return;
        };
        let total = assets.track_count();
        if total == 0 {
            return;
        }
        let previous = self.track_index;
        let mut chosen = None;
        for _ in 0..16 {
            let candidate = self.random.next_below(total);
            if Some(candidate) == previous && total > 1 {
                continue;
            }
            if let Some(track) = assets.load_track(candidate).filter(|track| track.frame_count() > 0) {
                chosen = Some((candidate, track));
                break;
            }
        }
        // Every draw failed: fall back to the first readable track, whichever it is.
        if chosen.is_none() {
            chosen = (0..total).find_map(|candidate| {
                assets
                    .load_track(candidate)
                    .filter(|track| track.frame_count() > 0)
                    .map(|track| (candidate, track))
            });
        }
        if let Some((index, track)) = chosen {
            self.track = Some(track);
            self.track_index = Some(index);
            self.track_position = 0.0;
            self.track_changes += 1;
        } else {
            self.track = None;
            self.track_index = None;
        }
    }
}

#[allow(clippy::cast_precision_loss)] // clip lengths are far below 2^52 frames
const fn frames_f64(frames: usize) -> f64 {
    frames as f64
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
// the C# `(int)position` of a positive position and `(float)` blend, as the mixer reads clips
fn sample(samples: &[f32], channels: usize, count: usize, position: f64) -> (f32, f32) {
    if position < 0.0 {
        return (0.0, 0.0);
    }
    let index = position as usize;
    let next = (index + 1).min(count.saturating_sub(1));
    let blend = (position - index as f64) as f32;
    if channels == 1 {
        let mono = lerp(samples[index], samples[next], blend);
        return (mono, mono);
    }
    let left = lerp(samples[index * 2], samples[next * 2], blend);
    let right = lerp(samples[index * 2 + 1], samples[next * 2 + 1], blend);
    (left, right)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + ((b - a) * t)
}
