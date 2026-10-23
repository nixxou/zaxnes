//! Cycle counts of every opcode, and the page-cross and branch penalties
//! (REQ-CORE-110, REQ-CORE-111).
//!
//! The expected counts are the published 6502 cycle table (nesdev "6502 instruction
//! reference" and "CPU unofficial opcodes"), transcribed here as an independent reference:
//! the core derives its counts from its bus accesses, not from a table, so a mistake in
//! either one shows up.

mod common;

use common::TestBus;
use zax_nes::cpu::{AddressingMode, Cpu6502, CpuOperation, OpcodeTable};

/// Base cycle count per opcode, indexed by opcode. 0 marks a `JAM`, which halts the chip
/// and has no documented count. Branches carry their *not taken* count and are checked
/// separately.
#[rustfmt::skip]
const BASE_CYCLES: [u32; 256] = [
    /*       0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F */
    /* 00 */ 7, 6, 0, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6,
    /* 10 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /* 20 */ 6, 6, 0, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6,
    /* 30 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /* 40 */ 6, 6, 0, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6,
    /* 50 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /* 60 */ 6, 6, 0, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6,
    /* 70 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /* 80 */ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
    /* 90 */ 2, 6, 0, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,
    /* A0 */ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
    /* B0 */ 2, 5, 0, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,
    /* C0 */ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    /* D0 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /* E0 */ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    /* F0 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
];

#[test]
fn req_core_110_every_opcode_takes_the_documented_number_of_cycles() {
    let mut failures = Vec::new();

    for opcode in 0..=255u8 {
        let info = OpcodeTable::get(opcode);

        if info.operation == CpuOperation::Jam || info.mode == AddressingMode::Relative {
            // JAM freezes the chip; branches depend on the flags and are checked below.
            continue;
        }

        let mut bus = TestBus::new();

        // Operands chosen so that no index crosses a page: $10 in page zero, $2010 for
        // absolute, and both index registers at 0.
        bus.load(0x1000, &[opcode, 0x10, 0x20]);
        let mut cpu = bus.cpu();

        let actual = cpu.step(&mut bus);
        let expected = BASE_CYCLES[usize::from(opcode)];

        if actual != expected {
            failures.push(format!(
                "${opcode:02X} {}{} {:?}: expected {expected}, got {actual}",
                if info.official { ' ' } else { '*' },
                info.mnemonic(),
                info.mode
            ));
        }
    }

    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn req_core_110_jam_freezes_the_cpu_without_hanging_the_host() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x02]);
    let mut cpu = bus.cpu();

    cpu.step(&mut bus);
    assert!(cpu.jammed());
    assert_eq!(0x1001, cpu.pc);

    // Further steps keep burning cycles without advancing PC.
    let before = bus.cycle_count;
    cpu.step(&mut bus);
    assert!(bus.cycle_count > before);
    assert_eq!(0x1001, cpu.pc);
}

#[test]
fn req_core_110_absolute_indexed_page_cross_penalty() {
    let cases = [
        (0xBD, 1, 4),    // LDA abs,X  — read, no page cross.
        (0xBD, 0xFF, 5), // LDA abs,X  — read, page crossed: +1.
        (0xB9, 0xFF, 5), // LDA abs,Y  — read, page crossed: +1.
        (0x9D, 1, 5),    // STA abs,X  — write always pays the extra cycle.
        (0x9D, 0xFF, 5), // STA abs,X  — and nothing more on a cross.
        (0xFE, 1, 7),    // INC abs,X  — read-modify-write, always 7.
        (0xFE, 0xFF, 7),
    ];
    for (opcode, index, expected) in cases {
        let mut bus = TestBus::new();

        // Base address $2010: index $FF crosses into page $21.
        bus.load(0x1000, &[opcode, 0x10, 0x20]);
        let mut cpu = bus.cpu();
        cpu.x = index;
        cpu.y = index;

        assert_eq!(
            expected,
            cpu.step(&mut bus),
            "${opcode:02X} index {index:02X}"
        );
    }
}

#[test]
fn req_core_110_indirect_indexed_page_cross_penalty() {
    let cases = [
        (0xB1, 0x01, 5), // LDA (ind),Y — no page cross.
        (0xB1, 0xFF, 6), // LDA (ind),Y — page crossed: +1.
        (0x91, 0x01, 6), // STA (ind),Y — write always pays it.
        (0x91, 0xFF, 6),
    ];
    for (opcode, y, expected) in cases {
        let mut bus = TestBus::new();
        bus.load(0x1000, &[opcode, 0x10]);
        bus[0x10] = 0x10;
        bus[0x11] = 0x20; // Pointer → $2010.

        let mut cpu = bus.cpu();
        cpu.y = y;

        assert_eq!(expected, cpu.step(&mut bus), "${opcode:02X} y {y:02X}");
    }
}

#[test]
fn req_core_111_branch_costs_two_three_or_four_cycles() {
    let cases = [
        (0xD0, true, 0x10, 2),  // BNE with Z set: not taken.
        (0xD0, false, 0x10, 3), // BNE with Z clear: taken, same page.
        (0xF0, false, 0x10, 2), // BEQ with Z clear: not taken.
        (0xF0, true, 0x10, 3),  // BEQ with Z set: taken, same page.
    ];
    for (opcode, zero_flag, offset, expected) in cases {
        let mut bus = TestBus::new();
        bus.load(0x1000, &[opcode, offset]);

        let mut cpu = bus.cpu();
        cpu.set_flag(Cpu6502::FLAG_Z, zero_flag);

        assert_eq!(expected, cpu.step(&mut bus), "${opcode:02X} Z={zero_flag}");
    }
}

#[test]
fn req_core_111_branch_across_a_page_costs_four_cycles() {
    let mut bus = TestBus::new();

    // At $10F0, a +$20 branch lands at $1112: the high byte of PC changes.
    bus.load(0x10F0, &[0xD0, 0x20]);
    let mut cpu = bus.create_cpu(0x10F0, 0xFD);

    assert_eq!(4, cpu.step(&mut bus));
    assert_eq!(0x1112, cpu.pc);
}

#[test]
fn req_core_111_backward_branch_across_a_page_costs_four_cycles() {
    let mut bus = TestBus::new();

    // At $1001, a -$10 branch lands at $0FF3.
    bus.load(0x1001, &[0xD0, 0xF0]);
    let mut cpu = bus.create_cpu(0x1001, 0xFD);

    assert_eq!(4, cpu.step(&mut bus));
    assert_eq!(0x0FF3, cpu.pc);
}

#[test]
fn req_core_110_read_modify_write_writes_the_old_value_back_first() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xE6, 0x10]); // INC $10
    bus[0x10] = 0x41;

    let mut cpu = bus.cpu();
    assert_eq!(5, cpu.step(&mut bus));
    assert_eq!(0x42, bus[0x10]);

    // Opcode, operand, read, dummy write, write: five accesses, the last two at $10.
    assert_eq!(vec![0x1000, 0x1001, 0x0010, 0x0010, 0x0010], bus.access_log);
}

#[test]
fn req_core_110_zero_page_indexed_wraps_inside_page_zero() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xB5, 0xFF]); // LDA $FF,X
    bus[0x7F] = 0x5A;

    let mut cpu = bus.cpu();
    cpu.x = 0x80; // $FF + $80 = $7F, not $017F.

    assert_eq!(4, cpu.step(&mut bus));
    assert_eq!(0x5A, cpu.a);
}

#[test]
fn req_core_110_indexed_indirect_pointer_wraps_inside_page_zero() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xA1, 0xFF]); // LDA ($FF,X)
    bus[0xFF] = 0x34;
    bus[0x00] = 0x12; // High byte comes from $00, not $0100.
    bus[0x1234] = 0x7E;

    let mut cpu = bus.cpu();
    cpu.x = 0x00;

    assert_eq!(6, cpu.step(&mut bus));
    assert_eq!(0x7E, cpu.a);
}

#[test]
fn req_core_110_indirect_jmp_does_not_carry_into_the_high_byte() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x6C, 0xFF, 0x20]); // JMP ($20FF)
    bus[0x20FF] = 0x34;
    bus[0x2000] = 0x12; // The NMOS bug: the high byte is read from $2000.
    bus[0x2100] = 0xFF;

    let mut cpu = bus.cpu();

    assert_eq!(5, cpu.step(&mut bus));
    assert_eq!(0x1234, cpu.pc);
}
