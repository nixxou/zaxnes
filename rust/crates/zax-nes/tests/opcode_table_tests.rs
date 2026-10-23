//! The opcode matrix: completeness, official/unofficial split and instruction lengths
//! (REQ-CORE-110).

use zax_nes::cpu::{AddressingMode, CpuOperation, OpcodeTable, OperandAccess};

#[test]
fn req_core_110_all_256_opcodes_are_described() {
    // Every slot holds an enum value by construction; check the length range.
    for opcode in 0..=255u8 {
        let info = OpcodeTable::get(opcode);
        assert!(
            (1..=3).contains(&info.length()),
            "Opcode ${opcode:02X} has a bad length."
        );
        assert_eq!(
            3,
            info.mnemonic().len(),
            "Opcode ${opcode:02X} has no mnemonic."
        );
    }
    assert_eq!(256, OpcodeTable::all().len());
}

#[test]
fn req_core_110_there_are_151_official_opcodes() {
    // The documented NMOS 6502 has 151 official opcodes; the other 105 are the
    // unofficial ones nestest prefixes with '*'.
    let official = (0..=255u8)
        .filter(|&opcode| OpcodeTable::get(opcode).official)
        .count();
    assert_eq!(151, official);
}

#[test]
fn req_core_110_the_twelve_jam_opcodes_are_in_place() {
    let expected = [
        0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xB2, 0xD2, 0xF2,
    ];

    let found: Vec<u8> = (0..=255u8)
        .filter(|&opcode| OpcodeTable::get(opcode).operation == CpuOperation::Jam)
        .collect();

    assert_eq!(expected.to_vec(), found);
}

#[test]
fn req_core_110_nestest_unofficial_opcodes_are_mapped() {
    // The ten unofficial mnemonics that appear in the public nestest.log.
    let cases = [
        (0x07, CpuOperation::Slo),
        (0x27, CpuOperation::Rla),
        (0x47, CpuOperation::Sre),
        (0x67, CpuOperation::Rra),
        (0x87, CpuOperation::Sax),
        (0xA7, CpuOperation::Lax),
        (0xC7, CpuOperation::Dcp),
        (0xE7, CpuOperation::Isb),
        (0xEB, CpuOperation::Sbc),
        (0x1A, CpuOperation::Nop),
    ];
    for (opcode, expected) in cases {
        let info = OpcodeTable::get(opcode);

        assert_eq!(expected, info.operation);
        assert!(!info.official);
    }
}

#[test]
fn req_core_110_addressing_modes_and_lengths() {
    let cases = [
        (0xEA, AddressingMode::Implied, 1),
        (0x0A, AddressingMode::Accumulator, 1),
        (0xA9, AddressingMode::Immediate, 2),
        (0xA5, AddressingMode::ZeroPage, 2),
        (0xB5, AddressingMode::ZeroPageX, 2),
        (0xB6, AddressingMode::ZeroPageY, 2),
        (0xAD, AddressingMode::Absolute, 3),
        (0xBD, AddressingMode::AbsoluteX, 3),
        (0xB9, AddressingMode::AbsoluteY, 3),
        (0x6C, AddressingMode::Indirect, 3),
        (0xA1, AddressingMode::IndirectX, 2),
        (0xB1, AddressingMode::IndirectY, 2),
        (0xD0, AddressingMode::Relative, 2),
    ];
    for (opcode, mode, length) in cases {
        let info = OpcodeTable::get(opcode);

        assert_eq!(mode, info.mode);
        assert_eq!(length, info.length());
    }
}

#[test]
fn req_core_110_read_modify_write_opcodes_are_tagged_as_such() {
    // Every ASL, LSR, ROL, ROR, INC, DEC and their unofficial combinations that have a
    // memory operand must be read-modify-write, which is what gives them their dummy
    // write and their unconditional indexing penalty.
    let rmw = [
        CpuOperation::Asl,
        CpuOperation::Lsr,
        CpuOperation::Rol,
        CpuOperation::Ror,
        CpuOperation::Inc,
        CpuOperation::Dec,
        CpuOperation::Slo,
        CpuOperation::Rla,
        CpuOperation::Sre,
        CpuOperation::Rra,
        CpuOperation::Dcp,
        CpuOperation::Isb,
    ];

    for opcode in 0..=255u8 {
        let info = OpcodeTable::get(opcode);
        if !rmw.contains(&info.operation) {
            continue;
        }

        let has_memory_operand = !matches!(
            info.mode,
            AddressingMode::Accumulator | AddressingMode::Implied
        );
        let expected = if has_memory_operand {
            OperandAccess::ReadModifyWrite
        } else {
            OperandAccess::None
        };
        assert_eq!(expected, info.access);
    }
}
