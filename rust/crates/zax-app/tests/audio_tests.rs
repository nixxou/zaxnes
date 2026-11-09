//! Port of `AudioTests.cs`: the audio of SPEC-11 — the output model, the gains, the fades, the
//! menu music and the interface sounds.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code
#![allow(clippy::float_cmp)] // exact values by construction

mod common;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use common::{LoopHarness, TemporaryRoot};
use zax_app::audio::audio_assets::{AudioAssets, TRACK_FILE_NAMES};
use zax_app::audio::audio_mixer::{AudioSampleSource, BLOCK, MUSIC_GAIN_FACTOR, SOUND_GAIN_FACTOR};
use zax_app::audio::fade_gain::FULL_FADE_SECONDS;
use zax_app::audio::stream::{frames_of_ms, AudioControl, AudioStream, ConsoleSamples, RAMP_MS};
use zax_app::audio::{AudioMixer, TrackRandom, UiAudioLibrary, UiSound};
use zax_app::emulation::commands::Command;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_platform::audio::{blocks_to_queue, BLOCK_COUNT};
use zax_platform::WaveAudio;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

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

/// Q-153 §4: a resume, a state load or a rewind empties the ring and asks the stream to prefill
/// again. Nothing stale is replayed (REQ-AUD-010), the mid-stream prefill is half the latency so
/// the hole stays inside the 50 ms REQ-AUD-012 allows, and the signal is ramped in, not switched
/// on.
#[test]
fn q153_a_resume_clears_the_ring_and_ramps_back_in() {
    let latency_ms = 60.0;
    let (mut stream, control, queue) = stream(latency_ms);
    control.set_console_active(true);
    control.set_console_gain(1.0);
    feed(&queue, frames_of_ms(RATE, 300.0), 0.75);
    let _ = block_of(&mut stream);
    assert!(control.playing(), "the stream did not start");

    // What the host does on a resume: empty the core's ring, then ask for a resynchronisation.
    queue.lock().unwrap().clear();
    control.request_resync();
    assert!(
        block_of(&mut stream).iter().all(|value| value.abs() < 0.76),
        "the held tail must not exceed the signal"
    );
    assert!(!control.playing());

    // Half the latency is enough to start again, and no more than that is waited for.
    let half = frames_of_ms(RATE, latency_ms / 2.0);
    feed(&queue, half - BLOCK, 0.5);
    assert!(
        block_of(&mut stream).iter().all(|value| *value == 0.0),
        "the gate opened before half the latency"
    );
    feed(&queue, frames_of_ms(RATE, 300.0), 0.5);
    let block = block_of(&mut stream);
    assert!(control.playing());
    near(block[0], 0.0, 0.01); // ramped in
    assert!(block[block.len() - 1].abs() > 0.1); // and up by the end of the block
    assert_eq!(
        control.underruns(),
        0,
        "a resynchronisation the host asked for is not an under-run"
    );
}

/// Plays `seconds` of audio: the device pulls blocks while a producer that runs at real time
/// tops the ring up, except during a stall of `stall_ms` in the middle. Returns the under-runs.
///
/// This is finding 3 in miniature: the v0.4.2 output held about 32 ms, so a rendered frame
/// longer than that put a hole in the sound whatever the core had produced.
fn play_through_a_stall(latency_ms: f32, stall_ms: f32, seconds: f32) -> u64 {
    let (mut stream, control, queue) = stream(latency_ms);
    control.set_console_active(true);
    control.set_console_gain(1.0);
    // The producer fills the ring to the target before the device starts, as a game does.
    feed(&queue, frames_of_ms(RATE, latency_ms), 0.5);
    let blocks = (frames_of_ms(RATE, seconds * 1000.0) / BLOCK).max(2);
    let stall_at = blocks / 2;
    #[allow(clippy::cast_precision_loss)] // a few hundred blocks
    let block_ms = BLOCK as f32 * 1000.0 / RATE as f32;
    let mut owed = 0.0_f32;
    for block in 0..blocks {
        // The producer owes one block's worth of samples per block consumed, and nothing at all
        // while the main thread is stalled.
        owed += block_ms;
        if block == stall_at {
            // The stall: the device keeps pulling, the producer adds nothing.
            let mut stalled = 0.0;
            while stalled < stall_ms {
                let _ = block_of(&mut stream);
                stalled += block_ms;
            }
            continue;
        }
        if owed >= block_ms {
            feed(&queue, frames_of_ms(RATE, owed), 0.5);
            owed = 0.0;
        }
        let _ = block_of(&mut stream);
    }
    control.underruns()
}

/// Finding 3 and Q-153 §2: at the default latency a 40 ms stall of the main thread — a shader
/// compile, a Lua collection, the 33,3 ms frames of `video.half_rate_3d` — no longer puts a hole
/// in the sound. A stall well past the latency still does, and then it is one event with a ramp
/// at each end rather than a block of zeros.
#[test]
fn q153_a_forty_millisecond_stall_at_sixty_of_latency_does_not_under_run() {
    assert_eq!(
        play_through_a_stall(60.0, 40.0, 2.0),
        0,
        "a 40 ms stall under-ran at 60 ms of latency"
    );
    // Twice the latency cannot be covered by any buffer of that size.
    assert!(
        play_through_a_stall(60.0, 120.0, 2.0) >= 1,
        "a 120 ms stall should have been noticed"
    );
    // And the buffer the owner had in v0.4.2, about 32 ms, could not take 40 ms.
    assert!(
        play_through_a_stall(30.0, 40.0, 2.0) >= 1,
        "the test harness is not measuring what it claims"
    );
}

/// REQ-AUD-004 and REQ-AUD-005, closed loop: the real stream, the real rate controller and a
/// device whose clock is `drift` off the emulation's, run for `minutes` of audio. Returns the
/// under-runs and the latency the controller ended on, in milliseconds.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // block and sample counts are small and positive by construction
fn play_against_a_drifting_clock(latency_ms: f32, drift: f64, minutes: f64) -> (u64, f32) {
    let (mut stream, control, queue) = stream(latency_ms);
    control.set_console_active(true);
    control.set_console_gain(1.0);
    feed(&queue, frames_of_ms(RATE, latency_ms), 0.5);

    let mut rate = zax_app::audio::rate_control::RateControl::new();
    let cap = 0.005; // Apu2A03::MAX_RATIO_ADJUSTMENT
    let block_seconds = f64::from(BLOCK as u32) / f64::from(RATE);
    let blocks = (minutes * 60.0 / block_seconds) as usize;
    let mut adjustment = 0.0;
    let mut owed = 0.0_f64;
    let mut block = vec![0.0_f32; BLOCK * 2];
    for _ in 0..blocks {
        // The core produces a block's worth of its own time, which is a device block scaled by
        // the correction in force and by the difference between the two clocks.
        owed += f64::from(BLOCK as u32) * (1.0 + adjustment) / (1.0 + drift);
        let whole = owed.floor() as usize;
        owed -= owed.floor();
        feed(&queue, whole, 0.5);
        // The device pulls, then the host corrects on what the audio thread measured.
        stream.render_block(&mut block);
        adjustment = rate
            .step(
                f64::from(control.average_fill()) * 1000.0 / f64::from(RATE)
                    - f64::from(latency_ms),
                block_seconds,
                control.playing(),
                cap,
            )
            .adjustment;
    }
    let settled = control.average_fill() * 1000.0 / RATE as f32;
    (control.underruns(), settled)
}

/// REQ-AUD-004 acceptance and REQ-AUD-005: with the sound card's clock a few hundredths of a
/// percent off the emulation's, the latency holds and **nothing** under-runs. v0.4.2 paid that
/// same offset in blocks of silence, because its proportional-only controller needed a standing
/// error to correct anything at all (finding 2).
#[test]
fn req_aud_005_a_drifting_device_clock_neither_under_runs_nor_drifts_away() {
    for drift in [0.000_3, -0.000_3] {
        let (underruns, settled) = play_against_a_drifting_clock(60.0, drift, 10.0);
        assert_eq!(underruns, 0, "drift {drift}: {underruns} under-runs");
        assert!(
            (settled - 60.0).abs() < 3.0,
            "drift {drift}: settled at {settled} ms"
        );
    }
}

/// The same over a full hour, which is what the work order asks for. Ignored by default: it
/// renders about two hundred thousand blocks, which is a few seconds in release and most of a
/// minute in debug.
#[test]
#[ignore = "an hour of simulated audio; run with --include-ignored"]
fn req_aud_005_an_hour_against_a_drifting_clock_under_runs_nothing() {
    let (underruns, settled) = play_against_a_drifting_clock(60.0, 0.000_3, 60.0);
    assert_eq!(underruns, 0, "{underruns} under-runs over the hour");
    assert!((settled - 60.0).abs() < 3.0, "settled at {settled} ms");
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

// ------------------------------------------------- AUDIO-2: the classic output of v0.4.2

/// A console source for the classic output: `available` samples are there to be read, and the
/// read is all-or-nothing, which is what `AudioReader::read_samples` does
/// (`zax-nes/src/apu/mod.rs`, `Ok(0)` without consuming when the ring is short).
fn classic_source(available: Arc<Mutex<usize>>, value: f32) -> AudioSampleSource {
    Box::new(move |out| {
        let mut left = available.lock().unwrap();
        if *left < out.len() {
            return 0;
        }
        *left -= out.len();
        out.fill(value);
        out.len()
    })
}

/// AUDIO-2: the output the program opens is the **new** one. `audio.classic_output` is a
/// fallback, so a fresh profile never gets `v0.4.2`'s behaviour by accident.
#[test]
fn audio_2_the_new_output_is_the_default() {
    let model = SettingsModel::new();
    assert!(!model.classic_audio_output());
    assert!(!model.get(SettingId::AudioClassicOutput).as_bool());
    // And the host a device-less run builds is on the default path, so nothing of the classic
    // one is armed.
    let harness = LoopHarness::create();
    assert!(!harness.host.classic_audio());
}

/// AUDIO-2: the queue rule of `v0.4.2`, which is what "four blocks, refilled once per rendered
/// frame" means — `pump` queues exactly `blocks_to_queue(size)` blocks and no more.
///
/// The SDL queue itself cannot be opened without a sound device, so the test drives the rule and
/// a replica of `pump`'s six-line loop over a queue it drains by hand. What it pins is the
/// arithmetic the real `pump` passes straight to SDL: never more than four blocks pending, a
/// partly played block counting as a whole one, and one frame's consumption refilled at the next
/// frame.
#[test]
fn audio_2_the_classic_output_keeps_four_blocks_and_refills_once_per_frame() {
    let block_bytes = BLOCK * 2 * std::mem::size_of::<i16>();

    // The first frame fills the whole buffer: four blocks, about 43 ms at 48 kHz.
    assert_eq!(BLOCK_COUNT, 4);
    assert_eq!(blocks_to_queue(0), 4);

    // A partly played block counts as pending, so a queue holding four blocks less one byte asks
    // for nothing: this is the `div_ceil` of `v0.4.2` and the reason the real floor of the buffer
    // is nearer 32 ms than 43 (finding 3 of Q-153).
    assert_eq!(blocks_to_queue(block_bytes * 4 - 1), 0);
    assert_eq!(blocks_to_queue(block_bytes * 4), 0);
    assert_eq!(blocks_to_queue(block_bytes * 3 + 1), 0);
    assert_eq!(blocks_to_queue(block_bytes * 3), 1);
    // And a queue the device over-drained is never asked for more than the four.
    assert_eq!(blocks_to_queue(block_bytes * 99), 0);

    // `pump`'s loop, over a device that plays one whole block per rendered frame.
    let mut mixer = mixer();
    let mut pending = 0_usize;
    let mut blocks = Vec::new();
    let mut buffer = vec![0.0_f32; BLOCK * 2];
    for frame in 0..10 {
        let wanted = blocks_to_queue(pending);
        for _ in 0..wanted {
            mixer.fill_with_console(&mut buffer);
            pending += block_bytes;
        }
        blocks.push(wanted);
        assert_eq!(pending, block_bytes * 4, "frame {frame} left four pending");
        // The device plays one block before the next frame.
        pending -= block_bytes;
    }
    // Four at the start, then exactly what was consumed.
    assert_eq!(blocks, vec![4, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
    assert_eq!(mixer.blocks, 13);
}

/// AUDIO-2: the classic output's block, as `v0.4.2` mixed it — the console on both channels at
/// the gain of the moment, the programme added, the sum clamped once (REQ-AUD-002, REQ-AUD-006).
#[test]
fn audio_2_the_classic_block_is_the_console_on_both_channels_clamped_once() {
    let mut mixer = mixer();
    mixer.set_volume(1.0);
    mixer.set_emulation_audible(true);
    let available = Arc::new(Mutex::new(BLOCK * 4));
    mixer.set_emulation_source(Some(classic_source(Arc::clone(&available), 0.25)));

    let mut block = vec![0.0_f32; BLOCK * 2];
    mixer.fill_with_console(&mut block);
    for pair in block.chunks_exact(2) {
        near(pair[0], 0.25, 1e-6);
        assert_eq!(pair[0], pair[1], "REQ-AUD-002: the same on both channels");
    }
    assert_eq!(mixer.underrun_blocks, 0);

    // Beyond full scale the block is clamped, once, on the sum (SPEC-11 §4).
    mixer.set_emulation_source(Some(classic_source(Arc::clone(&available), 1.75)));
    mixer.fill_with_console(&mut block);
    for value in &block {
        near(*value, 1.0, 1e-6);
    }
}

/// AUDIO-2: the defect the classic output is kept **with** — finding 1 of Q-153. One sample short
/// of a block, `v0.4.2` sent out a whole block of zeros and left the samples in the ring, which is
/// a 10.7 ms hole with a discontinuity at each edge.
///
/// The control is `req_aud_003_a_late_console_is_held_and_faded_rather_than_zeroed` above: the
/// default output, in the same situation, plays what there was and ramps.
#[test]
fn audio_2_a_late_console_costs_the_classic_output_a_whole_block_of_zeros() {
    let mut mixer = mixer();
    mixer.set_volume(1.0);
    mixer.set_emulation_audible(true);
    let available = Arc::new(Mutex::new(BLOCK - 1));
    mixer.set_emulation_source(Some(classic_source(Arc::clone(&available), 0.5)));

    let mut block = vec![0.0_f32; BLOCK * 2];
    mixer.fill_with_console(&mut block);
    assert!(
        block.iter().all(|value| *value == 0.0),
        "v0.4.2 wrote a whole block of zeros rather than the samples it had"
    );
    assert_eq!(mixer.underrun_blocks, 1);
    // Nothing was consumed: the short read is all-or-nothing, so the samples are still waiting.
    assert_eq!(*available.lock().unwrap(), BLOCK - 1);

    // The sample that completes the block ends the hole, with no ramp of any kind — the signal is
    // switched on at full gain, which is the discontinuity of the finding.
    *available.lock().unwrap() = BLOCK;
    mixer.fill_with_console(&mut block);
    near(block[0], 0.5, 1e-6);
    assert_eq!(mixer.underrun_blocks, 1);

    // Outside Running an empty ring is the silence of REQ-AUD-010 and not an under-run.
    mixer.set_emulation_audible(false);
    *available.lock().unwrap() = 0;
    mixer.fill_with_console(&mut block);
    assert_eq!(mixer.underrun_blocks, 1);
}

/// AUDIO-2: both outputs run the whole loop without a sound device, which is where the existing
/// tests of this file already drive the default one. The classic path takes `v0.4.2`'s rate
/// control — proportional on what the main thread sees of the core's ring — and the default path,
/// with no device open, asks the core for nothing at all.
#[test]
fn audio_2_both_outputs_pump_and_correct_without_a_device() {
    // The classic output: nobody drains the core's ring here, so it fills far above the two
    // blocks `v0.4.2` aimed at and the proportional law saturates at the cap, slowing the
    // production down.
    let mut classic = LoopHarness::create_with(|options| options.classic_audio = true);
    classic.launch("g1");
    classic.run_frames(30);
    assert!(classic.host.classic_audio());
    let core = classic.host.session().unwrap().core();
    assert!(core.apu().available_samples() > 2 * BLOCK);
    assert_eq!(
        core.audio_ratio_adjustment(),
        -zax_nes::apu::Apu2A03::MAX_RATIO_ADJUSTMENT
    );
    // Its report says which output is open and counts the blocks of zeros, the latency, the rate
    // and the under-runs of Q-153 §6 having no meaning for it.
    let report = classic.host.audio_report().expect("the classic report");
    assert!(report.starts_with("classic output (v0.4.2):"), "{report}");
    assert!(report.contains("silence blocks"), "{report}");

    // The default output with no device: no link, so no correction is asked of the core and no
    // report is drawn.
    let mut silent = LoopHarness::create();
    silent.launch("g1");
    silent.run_frames(30);
    assert!(!silent.host.classic_audio());
    assert_eq!(
        silent
            .host
            .session()
            .unwrap()
            .core()
            .audio_ratio_adjustment(),
        0.0
    );
    assert_eq!(silent.host.audio_report(), None);
}

/// AUDIO-2: `attach_console` is now the one place that hands a game's samples to the output, so
/// the test that it really does so belongs here. With a detached link — a stream nothing pulls
/// from — a launch fills the console slot, the slot reports the samples the core produced, and a
/// read gets them; and the host publishes the gain and the "producing" flag the audio thread needs.
///
/// Without this the default output would be **silent** and every other test would still pass: the
/// stream's own tests install their console by hand.
#[test]
fn audio_2_a_launch_hands_the_game_s_samples_to_the_default_output() {
    let link = zax_app::audio::AudioLink::detached(RATE, frames_of_ms(RATE, 60.0));
    let for_host = link.clone();
    let mut harness = LoopHarness::create_with(move |options| options.audio = Some(for_host));
    harness.launch("g1");
    harness.run_frames(30);

    assert!(!harness.host.classic_audio());
    assert!(link.control.console_active(), "the console is producing");
    near(link.control.console_gain(), 1.0, 1e-6);

    let mut slot = link.console.lock().unwrap();
    let console = slot.as_mut().expect("the launch installed the console");
    assert!(
        console.available() > BLOCK,
        "the slot sees the core's ring: {} samples",
        console.available()
    );
    let mut out = vec![0.0_f32; BLOCK];
    assert_eq!(console.read(&mut out), BLOCK);
}

/// AUDIO-2: the choice takes effect when the device is (re)opened, and the host asks the loop for
/// that reopening exactly once per change — not once per frame, and not when the setting is
/// written with the value already in force.
#[test]
fn audio_2_changing_the_setting_asks_for_one_reopening() {
    let mut harness = LoopHarness::create();
    assert!(!harness.host.take_audio_reopen());

    harness.host.update_settings(|model| {
        model.set(SettingId::AudioClassicOutput, SettingValue::Bool(true))
    });
    assert!(harness.host.take_audio_reopen(), "the change asks for one");
    assert!(
        !harness.host.take_audio_reopen(),
        "and only one: the loop drains the request"
    );

    // Written again with the value it already holds: no change at all, so nothing reaches the
    // host.
    harness.host.update_settings(|model| {
        model.set(SettingId::AudioClassicOutput, SettingValue::Bool(true))
    });
    assert!(
        !harness.host.take_audio_reopen(),
        "the same value twice is not a change"
    );

    // Back to `false` — a real change, and it agrees with the output that is open, since no
    // reopening happened (only the loop can reopen: it owns the SDL device). Nothing to do.
    harness.host.update_settings(|model| {
        model.set(SettingId::AudioClassicOutput, SettingValue::Bool(false));
    });
    assert!(!harness.host.classic_audio());
    assert!(
        !harness.host.take_audio_reopen(),
        "back to the output that is open: nothing to reopen"
    );
}

/// REQ-AUD-010 on the classic path: it has no prefill gate and no `resynchronise_audio` of its
/// own, so the ring must be emptied where `v0.4.2` emptied it — at the end of a launch. Without
/// that call the fragment of the previous session would be the first thing the mixer played.
#[test]
fn audio_2_a_new_game_drops_the_pending_audio_on_the_classic_output_too() {
    let mut harness = LoopHarness::create_with(|options| options.classic_audio = true);
    harness.launch("g1");
    harness.run_frames(60);
    assert!(
        harness
            .host
            .session()
            .unwrap()
            .core()
            .apu()
            .available_samples()
            > 0
    );

    harness.launch("g2");
    assert_eq!(
        harness
            .host
            .session()
            .unwrap()
            .core()
            .apu()
            .available_samples(),
        0
    );
}
