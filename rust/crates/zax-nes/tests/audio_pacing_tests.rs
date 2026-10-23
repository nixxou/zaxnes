//! Audio pacing and the output buffer (REQ-CORE-070, REQ-CORE-071, REQ-CORE-072).

// Helpers unwrap like the tests do; casts are on small test values.
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use common::minimal;
use zax_nes::apu::{Apu2A03, AudioError};
use zax_nes::{NesCore, NesRegion};

fn loaded_core() -> NesCore {
    let mut core = NesCore::new();
    assert!(core.load_rom(&minimal(), "Audio.nes").success);
    core
}

/// Runs `frames` frames, draining in blocks of 4096, and counts every sample produced.
fn produced_over(core: &mut NesCore, frames: usize, reference: Option<&mut NesCore>) -> usize {
    let mut produced = 0;
    let mut sink = vec![0.0f32; 4096];
    let mut reference = reference;

    for _ in 0..frames {
        let cycles = core.run_frame();
        if let Some(reference) = reference.as_deref_mut() {
            assert_eq!(reference.run_frame(), cycles);
        }
        while core.apu().available_samples() >= sink.len() {
            produced += core.read_audio(&mut sink, false).unwrap();
        }
    }

    produced + core.apu().available_samples()
}

fn assert_in_range(value: usize, expected: usize, tolerance: usize) {
    assert!(
        (expected - tolerance..=expected + tolerance).contains(&value),
        "{value} is not within {tolerance} of {expected}"
    );
}

#[test]
fn req_core_070_six_hundred_frames_produce_the_expected_sample_count() {
    // A frame produces sampleRate / nominalFps samples: 800 per frame at 48 kHz in NTSC,
    // 960 in PAL.
    for (region, sample_rate, expected, tolerance) in [
        (NesRegion::Ntsc, 48_000, 480_000, 800),
        (NesRegion::Pal, 48_000, 576_000, 960),
        (NesRegion::Dendy, 48_000, 576_000, 960),
        (NesRegion::Multi, 48_000, 480_000, 800),
        (NesRegion::Ntsc, 44_100, 441_000, 800),
        (NesRegion::Ntsc, 22_050, 220_500, 800),
        (NesRegion::Ntsc, 96_000, 960_000, 800),
    ] {
        let mut core = loaded_core();
        core.set_region(region);
        core.set_sample_rate(sample_rate).unwrap();

        assert_in_range(produced_over(&mut core, 600, None), expected, tolerance);
    }
}

#[test]
fn req_core_072_the_speed_factor_only_retimes_the_audio() {
    for (speed, expected) in [(2.0, 240_000), (0.5, 960_000)] {
        let mut core = loaded_core();
        core.set_sample_rate(48_000).unwrap();
        core.set_speed(speed).unwrap();

        assert_in_range(produced_over(&mut core, 600, None), expected, 800);
    }
}

#[test]
fn req_core_072_the_speed_factor_does_not_change_the_emulation() {
    let mut fast = loaded_core();
    let mut normal = loaded_core();
    fast.set_speed(1.5).unwrap(); // Fast forward (DEC-07-Q2).

    for _ in 0..120 {
        assert_eq!(normal.run_frame(), fast.run_frame());
    }

    for address in 0..0x800u16 {
        assert_eq!(normal.read_memory(address), fast.read_memory(address));
    }
}

#[test]
fn req_core_071_a_read_is_all_or_nothing() {
    let mut core = loaded_core();
    core.set_sample_rate(48_000).unwrap();
    core.run_frame(); // About 800 samples.

    let available = core.apu().available_samples();
    assert!((780..=820).contains(&available), "{available}");

    // Asking for more than there is consumes nothing.
    assert_eq!(0, core.read_audio(&mut [0.0; 1024], false).unwrap());
    assert_eq!(available, core.apu().available_samples());

    // Asking for less fills the whole buffer.
    assert_eq!(512, core.read_audio(&mut [0.0; 512], false).unwrap());
    assert_eq!(available - 512, core.apu().available_samples());
}

#[test]
fn req_core_071_a_stereo_read_consumes_half_as_many_mono_samples() {
    let mut core = loaded_core();
    core.set_sample_rate(48_000).unwrap();
    core.run_frame();

    let available = core.apu().available_samples();
    let mut buffer = [0.0f32; 512];

    assert_eq!(512, core.read_audio(&mut buffer, true).unwrap());
    assert_eq!(available - 256, core.apu().available_samples());

    // Left equals right.
    for pair in buffer.chunks_exact(2) {
        assert_eq!(pair[0], pair[1]);
    }
}

#[test]
fn req_core_071_samples_stay_inside_minus_one_and_one() {
    let mut core = loaded_core();
    for _ in 0..5 {
        core.run_frame();
    }

    let mut buffer = [0.0f32; 2048];
    assert_eq!(buffer.len(), core.read_audio(&mut buffer, false).unwrap());

    assert!(buffer.iter().all(|s| (-1.0..=1.0).contains(s)));
}

#[test]
fn req_core_071_a_stereo_buffer_must_hold_an_even_number_of_values() {
    let core = loaded_core();
    assert_eq!(
        Err(AudioError::OddStereoBuffer(511)),
        core.read_audio(&mut [0.0; 511], true)
    );
}

#[test]
fn req_core_070_the_sample_rate_range_is_accepted() {
    for sample_rate in [22_050, 48_000, 96_000] {
        let mut core = loaded_core();
        core.set_sample_rate(sample_rate).unwrap();

        assert_eq!(sample_rate, core.sample_rate());
    }
}

#[test]
fn req_core_070_a_sample_rate_outside_the_range_is_rejected() {
    for sample_rate in [22_049, 96_001] {
        let mut core = loaded_core();
        assert_eq!(
            Err(AudioError::SampleRateOutOfRange(sample_rate)),
            core.set_sample_rate(sample_rate)
        );
    }
}

#[test]
fn req_core_070_the_fine_ratio_adjustment_shifts_the_sample_count_without_touching_the_emulation() {
    for (adjustment, expected) in [(0.005, 482_400), (-0.005, 477_600)] {
        let mut core = loaded_core();
        let mut reference = loaded_core();
        core.set_sample_rate(48_000).unwrap();
        core.set_audio_ratio_adjustment(adjustment).unwrap();

        assert_in_range(
            produced_over(&mut core, 600, Some(&mut reference)),
            expected,
            800,
        );

        for address in 0..0x800u16 {
            assert_eq!(reference.read_memory(address), core.read_memory(address));
        }
    }
}

#[test]
fn req_core_070_the_fine_ratio_adjustment_is_capped_at_half_a_percent() {
    let mut core = loaded_core();

    core.set_audio_ratio_adjustment(0.005).unwrap();
    assert_eq!(0.005, core.audio_ratio_adjustment());

    core.set_audio_ratio_adjustment(-0.005).unwrap();
    assert_eq!(-0.005, core.audio_ratio_adjustment());

    assert_eq!(
        Err(AudioError::RatioAdjustmentOutOfRange(0.01)),
        core.set_audio_ratio_adjustment(0.01)
    );
}

#[test]
fn req_core_072_a_speed_of_zero_or_less_is_rejected() {
    let mut core = loaded_core();

    assert_eq!(Err(AudioError::SpeedNotPositive(0.0)), core.set_speed(0.0));
    assert_eq!(
        Err(AudioError::SpeedNotPositive(-1.0)),
        core.set_speed(-1.0)
    );
}

#[test]
fn req_core_071_reading_from_another_thread_does_not_corrupt_the_buffer() {
    let mut core = loaded_core();
    core.set_sample_rate(48_000).unwrap();

    let read = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let reader = core.audio_reader();

    let consumer = {
        let read = Arc::clone(&read);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut buffer = [0.0f32; 256];
            while !stop.load(Ordering::Acquire) {
                let count = reader.read_samples(&mut buffer, false).unwrap();
                if count == 0 {
                    std::thread::yield_now();
                    continue;
                }

                read.fetch_add(count, Ordering::Relaxed);
                assert!(buffer.iter().all(|s| (-1.0..=1.0).contains(s)));
            }
        })
    };

    for _ in 0..300 {
        core.run_frame();
    }

    stop.store(true, Ordering::Release);
    consumer.join().unwrap();

    assert!(read.load(Ordering::Relaxed) > 0);
}

#[test]
fn req_core_071_the_buffer_holds_at_least_ten_thousand_samples() {
    // REQ-CORE-071 asks for at least 10 000 samples; SPEC-11 REQ-AUD-005 asks for a bounded
    // buffer, 100 to 200 ms being the recommended range.
    for sample_rate in [22_050, 48_000, 96_000] {
        let mut core = loaded_core();
        core.set_sample_rate(sample_rate).unwrap();

        let capacity = core.apu().buffer_capacity();
        assert!(capacity >= Apu2A03::MIN_BUFFER_SAMPLES);
        assert!(capacity <= Apu2A03::MIN_BUFFER_SAMPLES.max(sample_rate as usize / 5));
    }
}

#[test]
fn req_core_071_an_overflowing_buffer_drops_the_oldest_samples() {
    let mut core = loaded_core();
    core.set_sample_rate(48_000).unwrap();

    // Far more than the buffer can hold, with nothing read in between.
    for _ in 0..60 {
        core.run_frame();
    }

    assert_eq!(core.apu().buffer_capacity(), core.apu().available_samples());
}
