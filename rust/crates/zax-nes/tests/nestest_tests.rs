//! Compares a run of the public `nestest.nes` in automation mode with the public
//! `nestest.log`, line by line (REQ-CORE-110).
//!
//! Automation mode means starting at $C000 instead of through the reset vector; the ROM
//! then runs the official opcodes and, after them, the unofficial ones, without touching
//! the PPU. Compared fields: PC, the instruction bytes, A, X, Y, P, SP and the CPU cycle
//! count. The disassembly text is reported when a line differs, to make a failure
//! readable, but it is not part of the comparison: the reference log shows the byte each
//! addressing mode points at, and some of those addresses are PPU registers, which the
//! side-effect-free host accessor of REQ-CORE-081 deliberately reads back as 0.
//!
//! The ROM and its log are public but are not redistributed; the test skips with a
//! message when `rust/test-roms/nestest.{nes,log}` is missing.

mod common;

use std::fmt::Write as _;

use common::{loaded_core, test_rom, TestBus};
use zax_nes::apu::Apu2A03;
use zax_nes::cartridges::Cartridge;
use zax_nes::cpu::{Cpu6502, CpuBus, CpuTraceLine, CpuTracer};
use zax_nes::input::InputPorts;
use zax_nes::ppu::Ppu2C02;
use zax_nes::NesBus;

/// Entry point of nestest's automation mode (public nestest documentation).
const AUTOMATION_ENTRY: u16 = 0xC000;

/// Addresses nestest writes its two result codes to; 0 means every test passed.
const RESULT_LOW: u16 = 0x0002;
const RESULT_HIGH: u16 = 0x0003;

/// The fields of one reference line.
struct LogLine {
    pc: u16,
    bytes: Vec<u8>,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    sp: u8,
    cycles: i64,
}

fn hex8(text: &str) -> Option<u8> {
    u8::from_str_radix(text, 16).ok()
}

fn field<'a>(line: &'a str, key: &str, width: usize) -> Option<&'a str> {
    let start = line.find(key)? + key.len();
    line.get(start..start + width)
}

/// Parses the fixed-width reference log. The PC occupies columns 1 to 4, the one to three
/// instruction bytes columns 7 to 14, and the mnemonic starts at column 17 — preceded, at
/// column 16, by an asterisk for the unofficial opcodes. The register fields start at
/// column 49.
fn parse(line: &str) -> Option<LogLine> {
    let pc = u16::from_str_radix(line.get(0..4)?, 16).ok()?;
    let bytes = line
        .get(6..15)?
        .split_whitespace()
        .map(hex8)
        .collect::<Option<Vec<u8>>>()?;
    if bytes.is_empty() {
        return None;
    }

    let registers = line.get(48..)?;
    let cycles_at = registers.find("CYC:")? + 4;
    Some(LogLine {
        pc,
        bytes,
        a: hex8(field(registers, "A:", 2)?)?,
        x: hex8(field(registers, " X:", 2)?)?,
        y: hex8(field(registers, " Y:", 2)?)?,
        p: hex8(field(registers, " P:", 2)?)?,
        sp: hex8(field(registers, " SP:", 2)?)?,
        cycles: registers.get(cycles_at..)?.trim().parse().ok()?,
    })
}

fn compare(expected: &LogLine, actual: &CpuTraceLine) -> Option<String> {
    if expected.pc != actual.pc {
        return Some(format!("PC ${:04X} vs ${:04X}", expected.pc, actual.pc));
    }
    if expected.bytes.as_slice() != actual.instruction_bytes() {
        return Some(format!(
            "instruction bytes {:02X?} vs {:02X?}",
            expected.bytes,
            actual.instruction_bytes()
        ));
    }
    if expected.a != actual.a {
        return Some(format!("A ${:02X} vs ${:02X}", expected.a, actual.a));
    }
    if expected.x != actual.x {
        return Some(format!("X ${:02X} vs ${:02X}", expected.x, actual.x));
    }
    if expected.y != actual.y {
        return Some(format!("Y ${:02X} vs ${:02X}", expected.y, actual.y));
    }
    if expected.p != actual.p {
        return Some(format!("P ${:02X} vs ${:02X}", expected.p, actual.p));
    }
    if expected.sp != actual.s {
        return Some(format!("SP ${:02X} vs ${:02X}", expected.sp, actual.s));
    }
    (expected.cycles != actual.cycles)
        .then(|| format!("CYC {} vs {}", expected.cycles, actual.cycles))
}

/// Builds the NROM cartridge of an iNES image (stands in for `NesCore.LoadRom`).
fn nrom_from_ines(image: &[u8]) -> Cartridge {
    assert_eq!(b"NES\x1A", &image[..4], "not an iNES image");
    let prg_size = usize::from(image[4]) * 0x4000;
    let offset = 16 + if image[6] & 0x04 != 0 { 512 } else { 0 };
    Cartridge::nrom(image[offset..offset + prg_size].to_vec())
}

#[test]
fn req_core_110_nestest_trace_matches_the_public_reference_log() {
    let (Some(image), Some(log)) = (test_rom("nestest.nes"), test_rom("nestest.log")) else {
        return;
    };
    let log = String::from_utf8_lossy(&log);

    let mut bus = NesBus::new(Ppu2C02::new(), Apu2A03::new(), InputPorts::new());
    bus.attach(Some(nrom_from_ines(&image)));
    let mut cpu = Cpu6502::new();
    cpu.reset(&mut bus);

    // Automation mode: enter at $C000 instead of the reset vector. The reset itself has
    // already consumed its seven cycles, which is where the log's CYC:7 comes from.
    cpu.pc = AUTOMATION_ENTRY;

    let mut compared = 0;
    let mut total = 0;
    let mut failures = String::new();

    for raw in log.lines() {
        if raw.trim().is_empty() {
            continue;
        }
        total += 1;

        let expected = parse(raw).unwrap_or_else(|| {
            panic!(
                "Reference log line {} could not be parsed: \"{raw}\"",
                compared + 1
            )
        });

        let actual = CpuTracer::capture(&cpu, &bus);
        if let Some(difference) = compare(&expected, &actual) {
            let _ = writeln!(failures, "line {}: {difference}", compared + 1);
            let _ = writeln!(failures, "  expected: {}", raw.trim_end());
            let _ = writeln!(failures, "  actual:   {}", actual.format());

            if failures.len() > 4000 {
                break;
            }
        }

        compared += 1;
        cpu.step(&mut bus);
    }

    assert!(
        failures.is_empty(),
        "{compared} of {total} reference lines compared.\n{failures}"
    );
    assert!(
        compared > 8000,
        "Only {compared} lines were compared; the log looks truncated."
    );

    // nestest reports its verdict in $0002 and $0003: zero means everything passed.
    assert_eq!(0x00, bus.peek(RESULT_LOW));
    assert_eq!(0x00, bus.peek(RESULT_HIGH));
}

#[test]
fn req_core_110_the_tracer_renders_the_nestest_line_format() {
    // Pinned without the ROM so that the format itself stays covered: the first line of
    // the public log is
    // "C000  4C F5 C5  JMP $C5F5   ... A:00 X:00 Y:00 P:24 SP:FD ... CYC:7".
    let mut bus = TestBus::new();
    bus.load(0xC000, &[0x4C, 0xF5, 0xC5]);
    bus[0xFFFC] = 0x00;
    bus[0xFFFD] = 0xC0;

    let mut cpu = Cpu6502::new();
    cpu.reset(&mut bus);

    let line = CpuTracer::capture(&cpu, &bus);

    assert_eq!(0xC000, line.pc);
    assert_eq!(&[0x4C, 0xF5, 0xC5], line.instruction_bytes());
    assert_eq!(0x00, line.a);
    assert_eq!(0x00, line.x);
    assert_eq!(0x00, line.y);
    assert_eq!(0x24, line.p);
    assert_eq!(0xFD, line.s);
    assert_eq!(7, line.cycles);

    let formatted = line.format();
    assert!(formatted.starts_with("C000  4C F5 C5"), "{formatted}");
    assert!(formatted.contains("JMP $C5F5"), "{formatted}");
    assert!(
        formatted.contains("A:00 X:00 Y:00 P:24 SP:FD CYC:7"),
        "{formatted}"
    );

    // And the parser of the comparison accepts the real reference layout.
    let reference = "C000  4C F5 C5  JMP $C5F5                       A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7";
    let parsed = parse(reference).expect("reference line parses");
    assert_eq!(None, compare(&parsed, &line));
}

#[test]
fn req_core_110_the_tracer_has_no_effect_on_the_emulation() {
    // Tracing must not change a single cycle or byte, otherwise the comparison above
    // would be meaningless (REQ-CORE-081).
    let program = [0xE6, 0x20, 0xA5, 0x20, 0x8D, 0x00, 0x03, 0x4C, 0x00, 0x80];

    let (mut untraced_cpu, mut untraced) = loaded_core(Some(&program));
    let (mut traced_cpu, mut traced) = loaded_core(Some(&program));

    for _ in 0..20_000 {
        let _ = CpuTracer::capture(&traced_cpu, &traced);
        traced_cpu.step(&mut traced);
        untraced_cpu.step(&mut untraced);
    }

    assert_eq!(untraced.cycle_count(), traced.cycle_count());
    assert_eq!(untraced_cpu.pc, traced_cpu.pc);

    for address in 0..0x800u16 {
        assert_eq!(untraced.peek(address), traced.peek(address));
    }
}
