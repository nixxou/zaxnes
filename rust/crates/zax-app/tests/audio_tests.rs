//! Port of `AudioTests.cs`: the audio of SPEC-11 — the output model, the gains, the fades, the
//! menu music and the interface sounds.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code
#![allow(clippy::float_cmp)] // exact values by construction

mod common;

use common::{LoopHarness, TemporaryRoot};
use zax_app::audio::audio_assets::{AudioAssets, TRACK_FILE_NAMES};
use zax_app::audio::audio_mixer::{BLOCK, MUSIC_GAIN_FACTOR, SOUND_GAIN_FACTOR};
use zax_app::audio::fade_gain::FULL_FADE_SECONDS;
use zax_app::audio::{AudioMixer, TrackRandom, UiAudioLibrary, UiSound};
use zax_app::emulation::commands::Command;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_platform::WaveAudio;

const RATE: u32 = 48_000;

fn mixer() -> AudioMixer {
    AudioMixer::new(RATE, None, TrackRandom::new(1))
}

fn near(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} is not within {tolerance} of {expected}"
    );
}

/// A mono tone, as a decoded clip.
fn tone(seconds: f64) -> WaveAudio {
    // The clip length is a small positive count of frames.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let frames = (seconds * f64::from(RATE)) as usize;
    let samples = (0..frames)
        .map(|frame| {
            // Frame counts stay far below the 24 bits an f32 holds exactly.
            #[allow(clippy::cast_precision_loss)]
            let phase = 2.0 * std::f32::consts::PI * 440.0 * frame as f32 / RATE as f32;
            phase.sin()
        })
        .collect();
    WaveAudio {
        samples,
        channels: 1,
        sample_rate: RATE,
    }
}

/// Encodes a clip as a 16-bit PCM WAVE file, to exercise the decoder.
fn write_wave(audio: &WaveAudio) -> Vec<u8> {
    let data_bytes = u32::try_from(audio.samples.len() * 2).expect("clip size");
    let mut file = Vec::new();
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    file.extend_from_slice(b"WAVEfmt ");
    file.extend_from_slice(&16_u32.to_le_bytes());
    file.extend_from_slice(&1_u16.to_le_bytes());
    file.extend_from_slice(&audio.channels.to_le_bytes());
    file.extend_from_slice(&audio.sample_rate.to_le_bytes());
    file.extend_from_slice(&(audio.sample_rate * u32::from(audio.channels) * 2).to_le_bytes());
    file.extend_from_slice(&(audio.channels * 2).to_le_bytes());
    file.extend_from_slice(&16_u16.to_le_bytes());
    file.extend_from_slice(b"data");
    file.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in &audio.samples {
        // Clamped to the 16-bit range first, so the cast is exact.
        #[allow(clippy::cast_possible_truncation)]
        let value = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        file.extend_from_slice(&value.to_le_bytes());
    }
    file
}

/// REQ-AUD-002: the console signal is monophonic and goes to both channels unchanged.
#[test]
fn req_aud_002_the_console_signal_is_duplicated_on_both_channels() {
    let mut mixer = mixer();
    mixer.set_emulation_audible(true);
    mixer.set_emulation_source(Some(Box::new(|destination| {
        for (index, value) in destination.iter_mut().enumerate() {
            *value = if index % 2 == 0 { 0.5 } else { -0.5 };
        }
        destination.len()
    })));

    let mut block = vec![0.0_f32; BLOCK * 2];
    mixer.fill(&mut block);

    for frame in block.chunks(2) {
        near(frame[0], frame[1], 1e-6);
    }
    assert_ne!(block[0], 0.0);
}

/// REQ-AUD-003: a block is taken all or nothing. When the core has less than a whole block the
/// console contribution is digital silence.
#[test]
fn req_aud_003_an_incomplete_block_becomes_silence() {
    let mut mixer = mixer();
    mixer.set_emulation_audible(true);
    let available = BLOCK / 2;
    mixer.set_emulation_source(Some(Box::new(move |destination| {
        if destination.len() > available {
            // All or nothing, exactly as NesCore::read_audio behaves (REQ-CORE-071).
            return 0;
        }
        destination.fill(0.75);
        destination.len()
    })));

    let mut block = vec![1.0_f32; BLOCK * 2];
    mixer.fill(&mut block);

    assert!(block.iter().all(|value| *value == 0.0));
    assert_eq!(mixer.underrun_blocks, 1);
}

/// REQ-AUD-006 and vector A2: the console gain is `V`, so a quarter volume gives a quarter of the
/// peak.
#[test]
fn req_aud_006_the_console_gain_is_the_volume() {
    let mut mixer = mixer();
    mixer.set_emulation_audible(true);
    mixer.set_volume(0.25);
    mixer.set_emulation_source(Some(Box::new(|destination| {
        destination.fill(1.0);
        destination.len()
    })));

    let mut block = vec![0.0_f32; BLOCK * 2];
    mixer.fill(&mut block);

    near(block[0], 0.25, 1e-5);
    near(mixer.emulation_gain(), 0.25, 1e-5);
}

/// REQ-AUD-020 acceptance: the first launch after start-up begins at gain `V`, and a complete
/// fade then lasts 200/60 s whatever the render rate.
#[test]
fn req_aud_020_the_console_fade_lasts_three_and_a_third_seconds() {
    let mut mixer = mixer();
    near(mixer.emulation_gain(), 1.0, 1e-6);

    mixer.set_emulation_audible(true);
    near(mixer.emulation_gain(), 1.0, 1e-6);

    // Pausing starts the fade-out; it reaches 0 in 200/60 s of real time.
    mixer.set_emulation_audible(false);
    mixer.advance(FULL_FADE_SECONDS / 2.0);
    near(mixer.emulation_gain(), 0.5, 0.02);

    mixer.advance(FULL_FADE_SECONDS / 2.0);
    near(mixer.emulation_gain(), 0.0, 1e-6);

    // And the fade-in takes the same time, in as many small steps as one likes.
    mixer.set_emulation_audible(true);
    for _ in 0..200 {
        mixer.advance(FULL_FADE_SECONDS / 200.0);
    }
    near(mixer.emulation_gain(), 1.0, 1e-5);
}

/// REQ-AUD-021 and REQ-AUD-022: the menu music rises to `0.02 x V` while the state is not
/// Running and fades to 0 and suspends as soon as it is.
#[test]
fn req_aud_022_the_menu_music_fades_with_the_state() {
    let mut mixer = mixer();
    near(mixer.music_gain(), 0.0, 1e-9);

    mixer.advance(FULL_FADE_SECONDS);
    near(mixer.music_gain(), MUSIC_GAIN_FACTOR, 1e-6);

    mixer.set_emulation_audible(true);
    mixer.advance(FULL_FADE_SECONDS / 2.0);
    assert!((0.004..=0.012).contains(&mixer.music_gain()));

    mixer.advance(FULL_FADE_SECONDS);
    near(mixer.music_gain(), 0.0, 1e-9);
    assert!(mixer.music_suspended());
}

/// REQ-AUD-023: switching BG Music off takes the music gain to 0 at once, with no fade, and
/// switching it back on restores the target immediately.
#[test]
fn req_aud_023_toggling_the_menu_music_is_immediate() {
    let mut mixer = mixer();
    mixer.advance(FULL_FADE_SECONDS);
    near(mixer.music_gain(), MUSIC_GAIN_FACTOR, 1e-6);

    mixer.set_menu_music_enabled(false);
    near(mixer.music_gain(), 0.0, 1e-9);

    mixer.set_menu_music_enabled(true);
    near(mixer.music_gain(), MUSIC_GAIN_FACTOR, 1e-6);
}

/// REQ-AUD-021: the interface sounds are played at `0.07 x V` and the volume reaches them with
/// no fade (REQ-AUD-023).
#[test]
fn req_aud_021_the_interface_sound_gain_follows_the_volume() {
    let mut mixer = mixer();
    near(mixer.sound_gain(), SOUND_GAIN_FACTOR, 1e-6);

    mixer.set_volume(0.5);
    near(mixer.sound_gain(), SOUND_GAIN_FACTOR * 0.5, 1e-6);
}

/// REQ-AUD-040: the interface sounds share one voice — a new one cuts the one playing — and they
/// are heard whatever the emulation does.
#[test]
fn req_aud_040_interface_sounds_share_a_single_voice() {
    let mut mixer = mixer();
    mixer.set_emulation_audible(true);
    let clip = tone(1.0);

    mixer.play_clip(Some(clip.clone()));
    assert!(mixer.sound_playing());

    let mut block = vec![0.0_f32; BLOCK * 2];
    mixer.fill(&mut block);

    // Audible even while the game runs, at the interface gain.
    assert!(block.iter().any(|value| value.abs() > 1e-4));

    mixer.play_clip(Some(clip));
    assert!(mixer.sound_playing());

    mixer.stop_sound();
    assert!(!mixer.sound_playing());
}

/// REQ-AUD-010: in Paused and Stopped no emulated frame produces samples, so the console output
/// is exact silence and an empty buffer is not counted as an under-run.
#[test]
fn req_aud_010_pausing_gives_digital_silence() {
    let mut mixer = mixer();
    mixer.set_emulation_audible(false);
    mixer.set_emulation_source(Some(Box::new(|_| 0)));

    let mut block = vec![1.0_f32; BLOCK * 2];
    mixer.fill(&mut block);

    assert!(block.iter().all(|value| *value == 0.0));
    assert_eq!(mixer.underrun_blocks, 0);
}

/// REQ-AUD-030: a menu track does not loop, and when one ends while the music is active a
/// different one is drawn — never the same twice in a row.
#[test]
fn req_aud_030_a_new_track_is_drawn_among_the_others() {
    let root = TemporaryRoot::new("zax-audio");
    let directory = root.path().join("audio");
    std::fs::create_dir_all(&directory).unwrap();
    // Three very short tracks, so the playlist moves on inside a handful of blocks.
    for name in TRACK_FILE_NAMES {
        std::fs::write(directory.join(name), write_wave(&tone(0.02))).unwrap();
    }

    let assets = AudioAssets::try_open(Some(root.path()), |_| {}).unwrap();
    assert_eq!(assets.track_count(), 3);

    let mut mixer = AudioMixer::new(RATE, Some(Box::new(assets)), TrackRandom::new(7));
    let mut block = vec![0.0_f32; BLOCK * 2];

    let mut order: Vec<usize> = Vec::new();
    for _ in 0..40 {
        mixer.fill(&mut block);
        let current = mixer.current_track().expect("a track plays");
        if order.last() != Some(&current) {
            order.push(current);
        }
    }

    assert!(order.len() >= 3, "only {} tracks played", order.len());
    for pair in order.windows(2) {
        assert_ne!(pair[0], pair[1]);
    }
}

/// REQ-AUD-041: each interface event maps to its clip, and the four files of SPEC-11 section 2
/// are the ones read.
#[test]
fn req_aud_041_every_interface_event_has_its_clip() {
    let root = TemporaryRoot::new("zax-audio");
    let directory = root.path().join("audio");
    std::fs::create_dir_all(&directory).unwrap();
    for name in [
        "sfx_menu_move3.wav",
        "sfx_menu_move2.wav",
        "sfx_menu_move1.wav",
        "sfx_coin_double1.wav",
    ] {
        std::fs::write(directory.join(name), write_wave(&tone(0.05))).unwrap();
    }

    let assets = AudioAssets::try_open(Some(root.path()), |_| {}).unwrap();

    for sound in [
        UiSound::TabChange,
        UiSound::PageChange,
        UiSound::Move,
        UiSound::Confirm,
    ] {
        assert!(assets.sound(sound).is_some(), "{sound:?}");
    }

    let mut mixer = AudioMixer::new(RATE, Some(Box::new(assets)), TrackRandom::new(1));
    mixer.play_sound(UiSound::Confirm);
    assert!(mixer.sound_playing());
}

/// REQ-AUD-011: the audio follows the factor `k` by retiming — the core is told the factor,
/// which changes the pitch proportionally.
#[test]
fn req_aud_011_the_speed_factor_reaches_the_core() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");

    harness
        .host
        .update_settings(|model| model.set_game_speed(4));
    harness.tick(1.0 / 60.0);
    assert!((harness.host.session().unwrap().core().speed() - 2.0).abs() < 1e-9);

    harness
        .host
        .update_settings(|model| model.set_game_speed(-1));
    harness.tick(1.0 / 60.0);
    assert!((harness.host.session().unwrap().core().speed() - 0.8).abs() < 1e-9);
}

/// REQ-AUD-010: launching another game empties the core's buffer, so no fragment of the previous
/// session can be heard.
#[test]
fn req_aud_010_a_new_game_drops_the_pending_audio() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(60);

    harness.launch("g2");

    let session = harness.host.session().unwrap();
    assert_eq!(session.core().apu().available_samples(), 0);
}

/// REQ-AUD-006: `V = 0` silences everything, which is also what the Mute toggle of AMD-010 does
/// without touching the stored volume.
#[test]
fn req_aud_006_mute_silences_everything_without_losing_the_volume() {
    let mut harness = LoopHarness::create();
    harness.host.update_settings(|model| model.set_volume(0.8));
    harness.run(Command::ToggleMute);

    let settings = harness.host.settings();
    assert!(settings.mute());
    near(settings.effective_volume(), 0.0, 0.0);
    near(settings.volume(), 0.8, 1e-6);
    near(harness.host.mixer().unwrap().volume(), 0.0, 1e-6);

    harness.run(Command::ToggleMute);
    near(harness.host.mixer().unwrap().volume(), 0.8, 1e-6);
}
