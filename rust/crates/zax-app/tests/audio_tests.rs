//! Port of `AudioTests.cs`: the audio of SPEC-11 — the output model, the gains, the fades, the
//! menu music and the interface sounds.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code
#![allow(clippy::float_cmp)] // exact values by construction

mod common;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use common::{LoopHarness, TemporaryRoot};
use zax_app::audio::audio_assets::{AudioAssets, TRACK_FILE_NAMES};
use zax_app::audio::audio_mixer::{BLOCK, MUSIC_GAIN_FACTOR, SOUND_GAIN_FACTOR};
use zax_app::audio::fade_gain::FULL_FADE_SECONDS;
use zax_app::audio::stream::{frames_of_ms, AudioControl, AudioStream, ConsoleSamples, RAMP_MS};
use zax_app::audio::{AudioMixer, TrackRandom, UiAudioLibrary, UiSound};
use zax_app::emulation::commands::Command;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_platform::WaveAudio;

const RATE: u32 = 48_000;

fn mixer() -> AudioMixer {
    AudioMixer::new(RATE, None, TrackRandom::new(1))
}

/// A console the test feeds by hand, standing in for the APU's ring.
struct FakeConsole {
    queue: Arc<Mutex<VecDeque<f32>>>,
}

impl ConsoleSamples for FakeConsole {
    fn available(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    fn read(&mut self, out: &mut [f32]) -> usize {
        let mut queue = self.queue.lock().unwrap();
        let wanted = out.len().min(queue.len());
        for slot in out.iter_mut().take(wanted) {
            *slot = queue.pop_front().unwrap();
        }
        wanted
    }
}

/// A stream at the configured latency, with a console the test fills, and no programme.
fn stream(latency_ms: f32) -> (AudioStream, Arc<AudioControl>, Arc<Mutex<VecDeque<f32>>>) {
    let target = frames_of_ms(RATE, latency_ms);
    let (stream, control, slot, _program) = AudioStream::open(RATE, BLOCK, target);
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    *slot.lock().unwrap() = Some(Box::new(FakeConsole {
        queue: Arc::clone(&queue),
    }));
    (stream, control, queue)
}

/// Pushes `count` samples of `value` into the console's queue.
fn feed(queue: &Arc<Mutex<VecDeque<f32>>>, count: usize, value: f32) {
    let mut queue = queue.lock().unwrap();
    for _ in 0..count {
        queue.push_back(value);
    }
}

/// Renders one block and returns it.
fn block_of(stream: &mut AudioStream) -> Vec<f32> {
    let mut block = vec![0.0_f32; BLOCK * 2];
    stream.render_block(&mut block);
    block
}

/// The frames of the left channel.
fn left(block: &[f32]) -> Vec<f32> {
    block.chunks_exact(2).map(|pair| pair[0]).collect()
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
    let (mut stream, control, queue) = stream(20.0);
    control.set_console_active(true);
    control.set_console_gain(1.0);
    feed(&queue, frames_of_ms(RATE, 200.0), 0.5);

    // One block opens the prefill gate and ramps in; the next is the signal proper.
    let _ = block_of(&mut stream);
    let block = block_of(&mut stream);

    for pair in block.chunks_exact(2) {
        near(pair[0], pair[1], 1e-6);
    }
    assert_ne!(block[0], 0.0);
}

/// REQ-AUD-003 as amended by Q-153 §1: a console that runs out mid-block no longer turns the
/// whole block into zeros. The samples it did have are played, the last one is held and ramped
/// to zero, the block holds no discontinuity, and one under-run is counted.
#[test]
fn req_aud_003_a_late_console_is_held_and_faded_rather_than_zeroed() {
    let (mut stream, control, queue) = stream(20.0);
    control.set_console_active(true);
    control.set_console_gain(1.0);
    // Enough for the gate plus two whole blocks and a third of a block.
    let prefill = frames_of_ms(RATE, 20.0).max(BLOCK);
    feed(&queue, prefill + (BLOCK / 3), 0.75);

    let mut last = Vec::new();
    for _ in 0..4 {
        let block = block_of(&mut stream);
        if control.underruns() > 0 {
            last = left(&block);
            break;
        }
    }

    assert_eq!(control.underruns(), 1, "the drought is counted once");
    assert!(!last.is_empty(), "the under-run block was not reached");
    // Not the digital silence REQ-AUD-003 used to demand: the held signal is still there.
    assert!(
        last.iter().any(|value| value.abs() > 0.1),
        "the block is silent: {:?}",
        &last[..8]
    );
    // And no discontinuity: the ramp of RAMP_MS cannot move faster than one full scale over
    // that time, so neighbouring samples stay close.
    #[allow(clippy::cast_precision_loss)] // the rate is a small integer
    let per_sample = 1000.0 / (RATE as f32 * RAMP_MS);
    for pair in last.windows(2) {
        assert!(
            (pair[1] - pair[0]).abs() <= per_sample + 1e-3,
            "a jump of {} between {} and {}",
            (pair[1] - pair[0]).abs(),
            pair[0],
            pair[1]
        );
    }
    // The tail has reached zero by the end of the block.
    near(last[last.len() - 1], 0.0, 1e-6);
}

/// Q-153 §2: playback starts only once the ring holds the configured latency. Until then the
/// output is silence — there is nothing to hold yet — and nothing is counted as an under-run.
#[test]
fn q153_the_prefill_gates_playback() {
    let latency_ms = 60.0;
    let (mut stream, control, queue) = stream(latency_ms);
    control.set_console_active(true);
    control.set_console_gain(1.0);
    let target = frames_of_ms(RATE, latency_ms);

    // One block short of the target: still silent, and the samples are untouched.
    feed(&queue, target - 1, 0.75);
    let block = block_of(&mut stream);
    assert!(
        block.iter().all(|value| *value == 0.0),
        "the gate let the signal through early"
    );
    assert!(!control.playing());
    assert_eq!(
        queue.lock().unwrap().len(),
        target - 1,
        "samples were eaten"
    );

    // The last sample arrives: the gate opens and the signal is ramped in.
    feed(&queue, 1, 0.75);
    let block = block_of(&mut stream);
    assert!(control.playing());
    assert!(block.iter().any(|value| value.abs() > 0.1));
    // Ramped in, not switched on: the first sample is still near zero.
    near(block[0], 0.0, 0.01);
    assert_eq!(control.underruns(), 0, "waiting is not an under-run");
}

/// REQ-AUD-006 and vector A2: the console gain is `V`, so a quarter volume gives a quarter of the
/// peak. The mixer computes the gain, the audio thread applies it.
#[test]
fn req_aud_006_the_console_gain_is_the_volume() {
    let mut mixer = mixer();
    mixer.set_emulation_audible(true);
    mixer.set_volume(0.25);
    near(mixer.emulation_gain(), 0.25, 1e-5);

    let (mut stream, control, queue) = stream(20.0);
    control.set_console_active(true);
    control.set_console_gain(mixer.emulation_gain());
    feed(&queue, frames_of_ms(RATE, 200.0), 1.0);

    // Two blocks: the first opens the gate and ramps the gain and the envelope in.
    let _ = block_of(&mut stream);
    let block = block_of(&mut stream);

    near(block[0], 0.25, 1e-5);
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

/// REQ-AUD-010 and vector A6: in Paused and Stopped no emulated frame produces samples, so the
/// console output becomes exact silence — well inside the 100 ms the vector allows — and a dry
/// ring is not counted as an under-run.
#[test]
fn req_aud_010_pausing_gives_digital_silence() {
    let (mut stream, control, queue) = stream(20.0);
    control.set_console_active(true);
    control.set_console_gain(1.0);
    feed(&queue, frames_of_ms(RATE, 200.0), 0.75);
    let _ = block_of(&mut stream);
    assert_ne!(block_of(&mut stream)[0], 0.0, "the console is playing");

    // The game stops producing: the ramp of Q-153 §1 takes the signal down, and what is left in
    // the ring is not played.
    control.set_console_active(false);
    let mut blocks = 0;
    loop {
        let block = block_of(&mut stream);
        blocks += 1;
        if block.iter().all(|value| *value == 0.0) {
            break;
        }
        assert!(blocks < 5, "the output never reached digital silence");
    }

    #[allow(clippy::cast_precision_loss)] // a handful of blocks
    let elapsed_ms = (blocks * BLOCK) as f32 * 1000.0 / RATE as f32;
    assert!(elapsed_ms < 100.0, "silence took {elapsed_ms} ms");
    assert_eq!(
        control.underruns(),
        0,
        "a paused console is not an under-run"
    );
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
