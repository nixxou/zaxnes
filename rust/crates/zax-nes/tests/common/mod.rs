//! Test helpers shared by the CPU and bus tests.

#![allow(dead_code)] // Each test binary uses a different subset.

use std::ops::{Index, IndexMut};

use zax_nes::apu::Apu2A03;
use zax_nes::cartridges::Cartridge;
use zax_nes::cpu::{Cpu6502, CpuBus};
use zax_nes::input::InputPorts;
use zax_nes::ppu::Ppu2C02;
use zax_nes::NesBus;

/// A flat 64 KiB memory with no device on it, used to measure instruction behaviour and
/// cycle counts in isolation. Every access costs exactly one cycle, like the real bus.
pub struct TestBus {
    memory: Vec<u8>,
    pub cycle_count: i64,
    pub nmi_asserted: bool,
    pub irq_asserted: bool,
    /// Every address read or written, in order, for dummy-cycle assertions.
    pub access_log: Vec<u16>,
}

impl TestBus {
    pub fn new() -> Self {
        Self {
            memory: vec![0; 0x10000],
            cycle_count: 0,
            nmi_asserted: false,
            irq_asserted: false,
            access_log: Vec::new(),
        }
    }

    /// Writes `bytes` starting at `address`.
    pub fn load(&mut self, address: usize, bytes: &[u8]) {
        self.memory[address..address + bytes.len()].copy_from_slice(bytes);
    }

    /// Builds a CPU whose PC is at `origin`, past the reset sequence, with the cycle counter
    /// back at zero so that a single `step` can be measured.
    pub fn create_cpu(&mut self, origin: u16, stack_pointer: u8) -> Cpu6502 {
        let mut cpu = Cpu6502::new();
        cpu.pc = origin;
        cpu.s = stack_pointer;
        self.cycle_count = 0;
        self.access_log.clear();
        cpu
    }

    /// `create_cpu` with the defaults of the C# helper: $1000 and SP $FD.
    pub fn cpu(&mut self) -> Cpu6502 {
        self.create_cpu(0x1000, 0xFD)
    }
}

impl Index<usize> for TestBus {
    type Output = u8;
    fn index(&self, address: usize) -> &u8 {
        &self.memory[address]
    }
}

impl IndexMut<usize> for TestBus {
    fn index_mut(&mut self, address: usize) -> &mut u8 {
        &mut self.memory[address]
    }
}

impl CpuBus for TestBus {
    fn read(&mut self, address: u16) -> u8 {
        self.cycle_count += 1;
        self.access_log.push(address);
        self.memory[usize::from(address)]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.cycle_count += 1;
        self.access_log.push(address);
        self.memory[usize::from(address)] = value;
    }

    fn peek(&self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    fn nmi_asserted(&self) -> bool {
        self.nmi_asserted
    }

    fn irq_asserted(&self) -> bool {
        self.irq_asserted
    }

    fn cycle_count(&self) -> i64 {
        self.cycle_count
    }
}

/// The 16 KiB PRG-ROM of the C# `TestRoms.Minimal`: `program` at $8000 (default: an
/// endless `JMP $8000`) and every vector pointing at $8000.
pub fn minimal_prg(program: Option<&[u8]>) -> Vec<u8> {
    let mut prg = vec![0u8; 0x4000];
    let program = program.unwrap_or(&[0x4C, 0x00, 0x80]);
    prg[..program.len()].copy_from_slice(program);
    for vector in [0x3FFA, 0x3FFC, 0x3FFE] {
        prg[vector] = 0x00;
        prg[vector + 1] = 0x80;
    }
    prg
}

/// Stands in for `NesCore` with the minimal ROM loaded: an NROM bus, past the reset.
pub fn loaded_core(program: Option<&[u8]>) -> (Cpu6502, NesBus) {
    let mut bus = NesBus::new(Ppu2C02::new(), Apu2A03::new(), InputPorts::new());
    bus.attach(Some(Cartridge::nrom(minimal_prg(program))));
    let mut cpu = Cpu6502::new();
    cpu.reset(&mut bus);
    (cpu, bus)
}

/// Finds a public test ROM in `rust/test-roms/`; `None` (with a message) when missing.
pub fn test_rom(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test-roms")
        .join(name);
    let bytes = std::fs::read(&path).ok();
    if bytes.is_none() {
        eprintln!("skipped: {} not found", path.display());
    }
    bytes
}
