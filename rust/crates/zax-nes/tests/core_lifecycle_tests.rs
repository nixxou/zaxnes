//! Life cycle and the operation set of REQ-CORE-002 (REQ-CORE-001 to REQ-CORE-006,
//! REQ-CORE-031).

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use common::{machine_with, minimal, minimal_rom};
use zax_nes::{ControllerPortType, NesCore, VideoInfo};

fn counting_callback(core: &mut NesCore) -> Arc<AtomicUsize> {
    let callbacks = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&callbacks);
    core.set_video_callback(Some(Box::new(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    })));
    callbacks
}

#[test]
fn req_core_031_video_info_is_33_by_31_with_128_sprites() {
    let info = NesCore::VIDEO_INFO;

    assert_eq!(33, info.width);
    assert_eq!(31, info.height);
    assert_eq!(128, info.sprite_count);
    assert_eq!(1023, VideoInfo::MAX_BACKGROUND_RECORDS);
    assert_eq!(128, VideoInfo::MAX_SPRITE_RECORDS);
    assert_eq!(1151, VideoInfo::MAX_RECORDS);
}

#[test]
fn req_core_002_core_name_is_a_stable_non_empty_string() {
    assert_ne!(NesCore::CORE_NAME.trim(), "");
    assert_eq!("Zaxnes", NesCore::CORE_NAME);
}

#[test]
fn req_core_003_power_on_state_has_zeroed_ram_and_frame_counter() {
    let core = machine_with(&minimal(), "Minimal.nes");

    for address in 0x0000..=0x07FFu16 {
        assert_eq!(0, core.read_memory(address));
    }

    assert_eq!(0, core.frame_count());
    assert!(core.is_running());
}

#[test]
fn req_core_004_reset_is_a_power_cycle_and_clears_ram_and_prg_ram() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    core.write_memory(0x0300, 0x55);
    core.write_memory(0x6000, 0x55);
    assert_eq!(0x55, core.read_memory(0x0300));
    assert_eq!(0x55, core.read_memory(0x6000));

    core.run_frame(0);
    assert_eq!(1, core.frame_count());

    core.reset();

    // The reference clears PRG-RAM too (REQ-CORE-004, notes; DEC-02-Q3).
    assert_eq!(0x00, core.read_memory(0x0300));
    assert_eq!(0x00, core.read_memory(0x6000));
    assert_eq!(0, core.frame_count());
    assert_eq!(Some("Minimal.nes"), core.rom_name());
}

#[test]
fn req_core_004_reset_keeps_the_port_types() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    core.set_port_type(1, ControllerPortType::Zapper).unwrap();
    core.reset();

    assert_eq!(Ok(ControllerPortType::Zapper), core.port_type(1));
}

#[test]
fn req_core_005_running_a_frame_with_no_rom_does_nothing() {
    let mut core = NesCore::new();
    let callbacks = counting_callback(&mut core);

    for _ in 0..10 {
        assert_eq!(0, core.run_frame(0));
    }

    assert_eq!(0, callbacks.load(Ordering::Relaxed));
    assert_eq!(0, core.frame_count());
    assert_eq!(0, core.read_audio(&mut [0f32; 512], false).unwrap());
}

#[test]
fn req_core_005_closing_the_rom_stops_the_emulation() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    let callbacks = counting_callback(&mut core);
    core.run_frame(0);
    assert_eq!(1, callbacks.load(Ordering::Relaxed));

    core.close_rom();
    assert!(!core.is_running());

    let frames = core.frame_count();
    for _ in 0..10 {
        core.run_frame(0);
    }

    assert_eq!(1, callbacks.load(Ordering::Relaxed));
    assert_eq!(frames, core.frame_count());
}

#[test]
fn req_core_006_frame_counter_counts_frames_and_is_host_writable() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    for _ in 0..100 {
        core.run_frame(0);
    }

    assert_eq!(100, core.frame_count());

    core.set_frame_count(4242);
    assert_eq!(4242, core.frame_count());
}

#[test]
fn req_core_006_frame_counter_survives_a_savestate_round_trip() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    for _ in 0..100 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();

    for _ in 0..50 {
        core.run_frame(0);
    }

    assert_eq!(150, core.frame_count());

    core.try_load_state(&state, 0).unwrap();
    assert_eq!(100, core.frame_count());
}

fn run_alone(rom: &[u8], frames: u16) -> Vec<i64> {
    let mut core = machine_with(rom, "Alone.nes");
    (0..frames).map(|frame| core.run_frame(frame)).collect()
}

#[test]
fn req_core_001_two_instances_are_independent() {
    // Two cores, two different ROMs, run alternately: each must produce exactly what it
    // produces alone (REQ-ARCH-011).
    let rom_a = minimal_rom(0x01, None, None);
    let rom_b = minimal_rom(
        0x00,
        Some(&[0xA9, 0x37, 0x8D, 0x10, 0x03, 0x4C, 0x00, 0x80]),
        None,
    );

    let alone_a = run_alone(&rom_a, 600);
    let alone_b = run_alone(&rom_b, 600);

    let mut core_a = machine_with(&rom_a, "A.nes");
    let mut core_b = machine_with(&rom_b, "B.nes");

    let mut interleaved_a = Vec::new();
    let mut interleaved_b = Vec::new();
    for frame in 0..600u16 {
        interleaved_a.push(core_a.run_frame(frame));
        interleaved_b.push(core_b.run_frame(frame ^ 0xFFFF));
    }

    assert_eq!(alone_a, interleaved_a);
    assert_eq!(alone_b, interleaved_b);
    assert_eq!(0x37, core_b.read_memory(0x0310));
    assert_eq!(0x00, core_a.read_memory(0x0310));
}

#[test]
fn req_arch_002_the_crate_has_no_platform_dependency() {
    // The Rust counterpart of the C# check on referenced assemblies: the manifest of the
    // core names no windowing, GL or audio crate.
    let manifest = include_str!("../Cargo.toml");
    for forbidden in ["sdl2", "glow", "imgui", "winit", "glutin", "windows"] {
        assert!(!manifest.contains(forbidden), "{forbidden}");
    }
}
