//! Frame boundaries, region timings and the video callback (REQ-CORE-015, REQ-CORE-016,
//! REQ-CORE-020, REQ-CORE-021).

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use common::{machine_with, minimal, minimal_rom};
use zax_nes::{NesRegion, RegionTiming};

fn assert_in_range(value: f64, low: f64, high: f64) {
    assert!(
        (low..=high).contains(&value),
        "{value} not in {low}..={high}"
    );
}

#[test]
fn req_core_015_average_cpu_cycles_per_frame() {
    for (region, expected, tolerance) in [
        (NesRegion::Ntsc, 29780.5, 0.5),
        (NesRegion::Pal, 33247.5, 0.5),
        (NesRegion::Dendy, 35464.0, 1.0),
        (NesRegion::Multi, 29780.5, 0.5),
    ] {
        let mut core = machine_with(&minimal(), "Minimal.nes");
        core.set_region(region);

        // A region change takes effect on the next frame and does not move the PPU, so the
        // first frame after it can be short: it runs from wherever the scan was to the
        // vblank line of the new region (REQ-CORE-016). The steady-state length is what
        // REQ-CORE-015 is about.
        core.run_frame(0);

        let frames = 1000;
        let total: i64 = (0..frames).map(|_| core.run_frame(0)).sum();

        #[allow(clippy::cast_precision_loss)] // Cycle counts are far below 2^52.
        let average = total as f64 / f64::from(frames);
        assert_in_range(average, expected - tolerance, expected + tolerance);
    }
}

#[test]
fn req_core_015_timing_table() {
    for (region, clock, scanlines, vblank_line, fps, skips_dot) in [
        (NesRegion::Ntsc, 1_789_773, 262, 241, 60, true),
        (NesRegion::Pal, 1_662_607, 312, 241, 50, false),
        (NesRegion::Dendy, 1_773_448, 312, 291, 50, false),
        (NesRegion::Multi, 1_789_773, 262, 241, 60, true),
    ] {
        let timing = RegionTiming::for_region(region);

        assert_eq!(clock, timing.cpu_clock_hz, "{region:?}");
        assert_eq!(scanlines, timing.scanlines_per_frame, "{region:?}");
        assert_eq!(vblank_line, timing.vblank_scanline, "{region:?}");
        assert_eq!(fps, timing.nominal_frames_per_second, "{region:?}");
        assert_eq!(skips_dot, timing.skips_dot_on_odd_frames, "{region:?}");
    }

    assert_eq!(341, RegionTiming::DOTS_PER_SCANLINE);
    assert_eq!(240, RegionTiming::VISIBLE_SCANLINES);
}

#[test]
fn req_core_015_noise_and_dmc_period_tables_are_pal_only_for_pal() {
    assert!(RegionTiming::for_region(NesRegion::Pal).uses_pal_noise_and_dmc_periods);
    assert!(!RegionTiming::for_region(NesRegion::Ntsc).uses_pal_noise_and_dmc_periods);
    assert!(!RegionTiming::for_region(NesRegion::Multi).uses_pal_noise_and_dmc_periods);
    assert!(!RegionTiming::for_region(NesRegion::Dendy).uses_pal_noise_and_dmc_periods);
}

#[test]
fn req_core_016_region_can_change_between_frames_without_resetting() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    core.write_memory(0x0200, 0x5A);
    for _ in 0..10 {
        core.run_frame(0);
    }

    core.set_region(NesRegion::Pal);

    let frames = 200;
    let total: i64 = (0..frames).map(|_| core.run_frame(0)).sum();

    #[allow(clippy::cast_precision_loss)] // Cycle counts are far below 2^52.
    let average = total as f64 / f64::from(frames);
    assert_in_range(average, 33247.0, 33248.0);

    // RAM is untouched and the frame counter kept counting.
    assert_eq!(0x5A, core.read_memory(0x0200));
    assert_eq!(210, core.frame_count());
}

#[test]
fn req_core_020_the_video_callback_runs_exactly_once_per_frame() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    let callbacks = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&callbacks);
    core.set_video_callback(Some(Box::new(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    })));

    for _ in 0..60 {
        core.run_frame(0);
    }

    assert_eq!(60, callbacks.load(Ordering::Relaxed));
}

#[test]
fn req_core_021_the_callback_runs_even_when_the_batch_is_empty() {
    // With PPUMASK = $00 nothing is rendered, so the batch holds no record, and the callback
    // must still be invoked once per frame.
    let mut core = machine_with(&minimal(), "Minimal.nes");

    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    core.set_video_callback(Some(Box::new(move |batch| {
        sink.lock()
            .unwrap()
            .push((batch.records.len(), batch.palette.len()));
    })));

    core.write_memory(0x2001, 0x00);
    core.run_frame(0);

    // REQ-CORE-030: always 32 palette bytes.
    assert_eq!(vec![(0, 32)], *seen.lock().unwrap());
}

#[test]
fn req_core_020_a_frame_ends_at_the_start_of_vblank() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    for _ in 0..10 {
        core.run_frame(0);

        // The frame stops on the dot the VBlank flag rises, so the PPU is parked at the very
        // beginning of vblank.
        assert_eq!(core.timing().vblank_scanline, core.ppu().scanline());
        assert!(core.ppu().vblank_flag());
        assert!((1..=16).contains(&core.ppu().dot()));
    }
}

#[test]
fn req_core_020_the_nmi_handler_has_barely_started_when_a_frame_returns() {
    // Main loop keeps NMI enabled; the handler at $8010 increments $0010.
    let mut program = [0u8; 0x13];

    // $8000: LDA #$80 / STA $2000 / JMP $8000
    program[..8].copy_from_slice(&[0xA9, 0x80, 0x8D, 0x00, 0x20, 0x4C, 0x00, 0x80]);

    // $8010: INC $0010 / RTI
    program[0x10..].copy_from_slice(&[0xE6, 0x10, 0x40]);

    let mut core = machine_with(&minimal_rom(0x01, Some(&program), Some(0x8010)), "Nmi.nes");

    for frame in 1..=50u8 {
        core.run_frame(0);

        let counter = core.read_memory(0x0010);
        assert!(
            (frame - 1..=frame).contains(&counter),
            "frame {frame}: {counter}"
        );
    }
}

#[test]
fn req_core_010_region_timing_rejects_an_unknown_region() {
    // A Rust enum cannot hold an unknown region; the only way in is the numeric value of a
    // savestate or of the settings, and that is refused.
    assert_eq!(None, NesRegion::from_u8(42));
}
