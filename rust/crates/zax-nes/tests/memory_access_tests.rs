//! Host memory access and open bus (REQ-CORE-080 to REQ-CORE-082), at bus level.
//!
//! `NesCore.ReadMemory`/`WriteMemory` are `NesBus::peek`/`poke` until T05 ports the core.

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
