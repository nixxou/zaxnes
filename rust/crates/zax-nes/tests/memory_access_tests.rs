//! Host memory access and open bus (REQ-CORE-080 to REQ-CORE-082), at bus level.
//!
//! `NesCore.ReadMemory`/`WriteMemory` are `NesBus::peek`/`poke` until T05 ports the core.

// Test indices and bytes are small: the narrowing casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

mod common;

use common::loaded_core;
use zax_nes::cpu::CpuBus;

#[test]
fn req_core_080_work_ram_is_mirrored_every_2_kib() {
    let (_, mut bus) = loaded_core(None);

    bus.poke(0x0012, 0x42);

    assert_eq!(0x42, bus.peek(0x0012));
    assert_eq!(0x42, bus.peek(0x0812));
    assert_eq!(0x42, bus.peek(0x1012));
    assert_eq!(0x42, bus.peek(0x1812));
}

#[test]
fn req_core_080_reading_fffc_returns_the_reset_vector_of_the_current_bank() {
    let (_, bus) = loaded_core(None);

    // The minimal ROM points every vector at $8000.
    assert_eq!(0x00, bus.peek(0xFFFC));
    assert_eq!(0x80, bus.peek(0xFFFD));
}

#[test]
fn req_core_080_prg_ram_is_readable_and_writable() {
    let (_, mut bus) = loaded_core(None);

    bus.poke(0x6000, 0x11);
    bus.poke(0x7FFF, 0x22);

    assert_eq!(0x11, bus.peek(0x6000));
    assert_eq!(0x22, bus.peek(0x7FFF));
}

#[test]
fn req_core_081_registers_and_the_expansion_area_peek_as_zero() {
    let (_, bus) = loaded_core(None);

    for address in 0x2000..=0x401Fu16 {
        assert_eq!(0, bus.peek(address));
    }

    assert_eq!(0, bus.peek(0x4020));
    assert_eq!(0, bus.peek(0x5FFF));
}

#[test]
fn req_core_082_writing_work_ram_has_the_same_effect_as_a_cpu_write() {
    // The program copies $0200 to $0201 forever, so a host write at $0200 must show up
    // at $0201 after a frame's worth of cycles (stands in for RunFrame until T05).
    let program = [0xAD, 0x00, 0x02, 0x8D, 0x01, 0x02, 0x4C, 0x00, 0x80];
    let (mut cpu, mut bus) = loaded_core(Some(&program));

    bus.poke(0x0200, 0x5E);
    while bus.cycle_count() < 29_781 {
        cpu.step(&mut bus);
    }

    assert_eq!(0x5E, bus.peek(0x0201));
}

#[test]
fn req_core_081_unmapped_reads_return_the_open_bus() {
    // $4018-$401F is disabled, so a CPU read there sees the last value on the bus.
    let (_, mut bus) = loaded_core(None);
    bus.poke(0x0000, 0x00);

    let first = bus.read(0x0200); // Work RAM: 0.
    assert_eq!(0x00, first);

    bus.poke(0x0201, 0x5A);
    assert_eq!(0x5A, bus.read(0x0201));
    assert_eq!(0x5A, bus.read(0x4018));
}

// ---------------------------------------------------------------- PPU side (T02)

/// The minimal ROM loaded in a machine (C# `LoadedCore`).
fn loaded_machine() -> zax_nes::NesCore {
    common::machine_with(&common::minimal(), "Minimal.nes")
}

fn write_palette(core: &mut zax_nes::NesCore, address: u16, value: u8) {
    core.write_memory(0x2006, (address >> 8) as u8);
    core.write_memory(0x2006, address as u8);
    core.write_memory(0x2007, value);
}

fn capture_palette(core: &mut zax_nes::NesCore) -> [u8; 32] {
    core.run_frame();
    core.frame_batch().palette
}

#[test]
fn req_core_081_peeking_2002_does_not_clear_the_vblank_flag() {
    let mut core = loaded_machine();

    // A frame ends at the start of vblank, so the flag is set here.
    core.run_frame();
    assert!(core.ppu().vblank_flag());

    // The host accessor returns 0 for $2000-$401F and leaves the PPU alone.
    assert_eq!(0x00, core.read_memory(0x2002));
    assert!(core.ppu().vblank_flag());

    // The PPU's own non-destructive view agrees and is equally inert.
    assert_eq!(0x80, core.ppu().peek(2) & 0x80);
    assert!(core.ppu().vblank_flag());

    // A real CPU read, on the other hand, still clears it.
    assert_eq!(0x80, core.ppu_read(2) & 0x80);
    assert!(!core.ppu().vblank_flag());
    assert_eq!(0x00, core.ppu().peek(2) & 0x80);
}

#[test]
fn req_core_082_writing_the_palette_through_2006_and_2007_shows_up_in_the_batch() {
    let mut core = loaded_machine();

    // Three frames first, as SPEC-02 §5.1 asks, then program the palette.
    for _ in 0..3 {
        core.run_frame();
    }

    core.write_memory(0x2006, 0x3F);
    core.write_memory(0x2006, 0x00);
    core.write_memory(0x2007, 0x30);

    assert_eq!(0x30, capture_palette(&mut core)[0]);
}

#[test]
fn req_core_045_palette_vector_v7() {
    let mut core = loaded_machine();
    for _ in 0..3 {
        core.run_frame();
    }

    write_palette(&mut core, 0x3F00, 0x0F);
    write_palette(&mut core, 0x3F01, 0x30);
    write_palette(&mut core, 0x3F11, 0x16);

    let palette = capture_palette(&mut core);
    assert_eq!(0x0F, palette[0]);
    assert_eq!(0x30, palette[1]);
    assert_eq!(0x16, palette[17]);

    // Writing $3F10 lands on entry 0, and the mirror at 16 follows (REQ-CORE-045).
    write_palette(&mut core, 0x3F10, 0x21);
    let palette = capture_palette(&mut core);
    assert_eq!(0x21, palette[0]);
    assert_eq!(0x21, palette[16]);
}

#[test]
fn req_core_045_palette_entries_are_six_bits_wide() {
    let mut core = loaded_machine();
    for _ in 0..3 {
        core.run_frame();
    }

    write_palette(&mut core, 0x3F03, 0xFF);

    assert_eq!(0x3F, capture_palette(&mut core)[3]);
}
