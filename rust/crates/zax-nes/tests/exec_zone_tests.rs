//! The widescreen extension's execution zone: the CPU's instruction fetches (opcodes, operands
//! and its dummy reads of the program counter) tell themselves from data reads, and the bus
//! shows a hack pack's code to the former only in `$2000`-`$3FFF`.

// Test addresses and bytes are small.
#![allow(clippy::cast_possible_truncation)]

mod common;

use common::{loaded_core, TestBus};
use zax_nes::cpu::CpuBus;

#[test]
fn opcodes_and_operands_are_code_fetches_and_data_is_not() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xAD, 0x00, 0x03]); // LDA $0300
    let mut cpu = bus.cpu();
    cpu.step(&mut bus);
    assert_eq!(vec![0x1000, 0x1001, 0x1002], bus.code_log);
    assert_eq!(vec![0x1000, 0x1001, 0x1002, 0x0300], bus.access_log);
}

#[test]
fn the_dummy_reads_of_the_program_counter_are_code_fetches() {
    // NOP: the opcode, then a dummy read of the next byte.
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xEA]);
    let mut cpu = bus.cpu();
    cpu.step(&mut bus);
    assert_eq!(vec![0x1000, 0x1001], bus.code_log);

    // A taken branch across a page: the operand, the dummy read, the page fix-up.
    let mut bus = TestBus::new();
    bus.load(0x10F0, &[0xD0, 0x20]); // BNE +$20 with Z clear
    let mut cpu = bus.create_cpu(0x10F0, 0xFD);
    cpu.step(&mut bus);
    assert_eq!(vec![0x10F0, 0x10F1, 0x10F2, 0x1012], bus.code_log);

    // RTS: the dummy read of the PC, then of the return address.
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x60]);
    bus.load(0x01FE, &[0x02, 0x30]); // return to $3003
    let mut cpu = bus.create_cpu(0x1000, 0xFD);
    cpu.step(&mut bus);
    assert_eq!(vec![0x1000, 0x1001, 0x3002], bus.code_log);
    assert_eq!(0x3003, cpu.pc);
}

#[test]
fn an_interrupt_entry_reads_the_program_counter_as_code() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xEA, 0xEA]);
    bus.load(0xFFFA, &[0x00, 0x40]);
    let mut cpu = bus.cpu();
    bus.nmi_asserted = true;
    cpu.step(&mut bus);
    bus.code_log.clear();
    cpu.step(&mut bus);
    assert_eq!(
        vec![0x1001, 0x1001],
        bus.code_log,
        "the two reads before the pushes"
    );
    assert_eq!(0x4000, cpu.pc);
}

/// The minimal ROM with `program` at $8000, the extension unlocked, `code` the pack's code and
/// the zone on, page `page`.
fn zone_machine(program: &[u8], code: &[u8], page: u8) -> (zax_nes::cpu::Cpu6502, zax_nes::NesBus) {
    let (cpu, mut bus) = loaded_core(Some(program));
    for byte in *b"WIDE" {
        bus.write(0x5000, byte);
    }
    bus.ppu_mut().wide_mut().set_code(code);
    bus.write(0x500B, 0x80 | page);
    (cpu, bus)
}

#[test]
fn code_in_the_zone_runs_and_its_data_reads_still_reach_the_ppu() {
    // At $2000: LDA $2002 / STA $0300 / LDA #$5A / STA $0301 / JMP $200A (an endless loop).
    let code = [
        0xAD, 0x02, 0x20, 0x8D, 0x00, 0x03, 0xA9, 0x5A, 0x8D, 0x01, 0x03, 0x4C, 0x0B, 0x20,
    ];
    let (mut cpu, mut bus) = zone_machine(&[0x4C, 0x00, 0x20], &code, 0);
    for _ in 0..8 {
        cpu.step(&mut bus);
    }
    assert_eq!(0x5A, bus.ram()[0x0301], "the code ran from the zone");
    assert_ne!(
        0xAD,
        bus.ram()[0x0300],
        "LDA $2002 read the PPU, not the code"
    );
    assert_eq!(0x200B, cpu.pc, "looping in the zone");
}

#[test]
fn the_paged_part_shows_the_page_asked_for() {
    let mut code = vec![0xFF; 0x800 + 2 * 0x1800];
    code[0] = 0x11;
    code[0x800 + 0x1800] = 0x22; // page 1 at $2800
    let (_, mut bus) = zone_machine(&[0x4C, 0x00, 0x80], &code, 1);
    assert_eq!(0x11, bus.read_code(0x2000));
    assert_eq!(0x22, bus.read_code(0x2800));
    assert_eq!(0xFF, bus.read_code(0x3FFF), "past the code");
}

#[test]
fn without_the_zone_a_fetch_there_is_a_plain_read() {
    let (_, mut bus) = zone_machine(&[0x4C, 0x00, 0x80], &[0x11; 16], 0);
    bus.write(0x500B, 0x00);
    let fetched = bus.read_code(0x2000);
    assert_ne!(0x11, fetched, "the zone off: the PPU answers");
    // Data reads never see the code, the zone on or off.
    bus.write(0x500B, 0x80);
    assert_ne!(0x11, bus.read(0x2000));
    assert_eq!(0x11, bus.read_code(0x2000));
}
