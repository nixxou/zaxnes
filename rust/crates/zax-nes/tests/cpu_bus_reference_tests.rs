//! Lot FIDELITY plan step 4 (lead F1 of `docs/review/FIDELITY-LEADS.md`): every opcode's bus
//! activity against `BeesNES`'s per-half-cycle table of the 2A03 (`Research/Instructions/Ricoh
//! 2A03 Instructions NES.xlsx`, MIT), turned into `tests/data/cpu_bus_reference.txt` by
//! `rust/tools/cpu_bus_reference.py`.
//!
//! One instruction runs on a flat 64 KiB bus that records every access; each access is compared
//! by direction and by the class of its address (`PC`, the stack page `S`, the vectors `V`, data
//! `D`), in the two scenarios of the generator: `short` (no page crossed, branches not taken) and
//! `long` (every indexed access across a page, branches taken across a page).

use zax_nes::cpu::{Cpu6502, CpuBus};

const CODE_SHORT: u16 = 0x0600;
const CODE_LONG: u16 = 0x06F0;

struct RecordingBus {
    memory: Vec<u8>,
    log: Vec<(bool, u16)>,
    cycles: i64,
}

impl RecordingBus {
    fn new() -> Self {
        Self {
            memory: vec![0; 0x1_0000],
            log: Vec::new(),
            cycles: 0,
        }
    }
}

impl CpuBus for RecordingBus {
    fn read(&mut self, address: u16) -> u8 {
        self.log.push((false, address));
        self.cycles += 1;
        self.memory[usize::from(address)]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.log.push((true, address));
        self.cycles += 1;
        self.memory[usize::from(address)] = value;
    }

    fn peek(&self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    fn nmi_asserted(&self) -> bool {
        false
    }

    fn irq_asserted(&self) -> bool {
        false
    }

    fn cycle_count(&self) -> i64 {
        self.cycles
    }
}

/// The flags that make a branch not taken (`short`) or taken (`long`): bit 7 for BPL/BMI, 6 for
/// BVC/BVS, 0 for BCC/BCS, 1 for BNE/BEQ; the odd column of each pair branches on a set flag.
fn branch_flags(opcode: u8, taken: bool) -> u8 {
    let flag = match opcode >> 6 {
        0 => 0x80,
        1 => 0x40,
        2 => 0x01,
        _ => 0x02,
    };
    let on_set = (opcode & 0x20) != 0;
    if on_set == taken {
        flag
    } else {
        0
    }
}

fn class(address: u16) -> &'static str {
    match address {
        0x0100..=0x01FF => "S",
        0xFFFA..=0xFFFF => "V",
        0x0600..=0x07FF => "PC",
        _ => "D",
    }
}

fn run(opcode: u8, long: bool) -> Vec<String> {
    let mut bus = RecordingBus::new();
    // Every pointer and every absolute operand reads $0202; with X or Y at $FF, $0202 + $FF
    // crosses into $03xx. The zero page, $0200-$03FF and everything but the code, the stack
    // and the vectors are data.
    bus.memory[0x0000..0x0100].fill(0x02);
    bus.memory[0x0200..0x0400].fill(0x02);
    // What RTS and RTI pull returns into the code: $0606.
    bus.memory[0x0100..0x0200].fill(0x06);
    let code = if long { CODE_LONG } else { CODE_SHORT };
    let start = usize::from(code);
    bus.memory[start] = opcode;
    bus.memory[start + 1] = 0x02;
    bus.memory[start + 2] = 0x02;
    if (opcode & 0x1F) == 0x10 && long {
        // A taken branch from $06F2 by +$20 lands on $0712, across a page.
        bus.memory[start + 1] = 0x20;
    }
    bus.memory[0xFFFE] = 0x00;
    bus.memory[0xFFFF] = 0x08;

    let mut cpu = Cpu6502::new();
    cpu.pc = code;
    cpu.s = 0xFD;
    let index = if long { 0xFF } else { 0x00 };
    cpu.x = index;
    cpu.y = index;
    let flags = if (opcode & 0x1F) == 0x10 {
        branch_flags(opcode, long)
    } else {
        0
    };
    cpu.set_p(0x20 | flags);
    cpu.step(&mut bus);
    bus.log
        .iter()
        .map(|&(write, address)| {
            format!(
                "{}:{}",
                if write { "W" } else { "R" },
                class(address)
            )
        })
        .collect()
}

#[test]
fn every_opcode_matches_the_2a03_bus_table() {
    let table = include_str!("data/cpu_bus_reference.txt");
    let mut mismatches = Vec::new();
    let mut checked = 0;
    for line in table.lines().filter(|line| !line.starts_with('#')) {
        let mut parts = line.split('|');
        let head: Vec<&str> = parts.next().unwrap_or_default().split_whitespace().collect();
        let Some(opcode) = head.first().and_then(|hex| u8::from_str_radix(hex, 16).ok()) else {
            continue;
        };
        if head.get(2) == Some(&"jam") {
            continue;
        }
        for (scenario, long) in [("short", false), ("long", true)] {
            let expected: Vec<String> = parts
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_owned)
                .collect();
            let actual = run(opcode, long);
            checked += 1;
            if actual != expected {
                mismatches.push(format!(
                    "{opcode:02X} {} {scenario}: expected {} got {}",
                    head.get(1).copied().unwrap_or("?"),
                    expected.join(" "),
                    actual.join(" ")
                ));
            }
        }
    }
    assert_eq!(488, checked, "244 opcodes x 2 scenarios");
    assert!(
        mismatches.is_empty(),
        "{} mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
