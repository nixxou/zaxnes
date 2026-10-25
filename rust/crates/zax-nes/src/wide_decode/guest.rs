//! A sandbox where a decoder runs a game's own routines: the CPU over a copy of the console RAM
//! and the PRG-ROM, nothing else. Writes to the PPU, the APU or the cartridge go nowhere and reads
//! of them return 0; the console is not touched. For a game whose level format is easier to run
//! than to rewrite (Super Mario Bros.' area parser).
//!
//! Only the CPU RAM and a fixed 32 KiB PRG-ROM (NROM) are mapped: `$8000-$FFFF` reads the image,
//! wrapped to its size.

use crate::cpu::{Cpu6502, CpuBus};

/// Where a called routine returns to: the sandbox stops there. Nothing a game runs reaches it.
const RETURN: u16 = 0x0000;

/// The console RAM and the PRG-ROM a routine runs over.
pub struct Guest<'a> {
    /// CPU RAM, `$0000-$07FF`.
    pub ram: [u8; 0x800],
    prg_rom: &'a [u8],
    cycles: i64,
}

impl<'a> Guest<'a> {
    /// A sandbox over a copy of `ram` (the first 2 KiB) and `prg_rom`.
    #[must_use]
    pub fn new(ram: &[u8], prg_rom: &'a [u8]) -> Self {
        let mut copy = [0; 0x800];
        let length = ram.len().min(copy.len());
        copy[..length].copy_from_slice(&ram[..length]);
        Self {
            ram: copy,
            prg_rom,
            cycles: 0,
        }
    }

    /// Calls the routine at `address` (`JSR`) with an empty stack, and runs it until it returns.
    /// Returns `false` when it did not within `max_cycles` CPU cycles (or jammed): the RAM is
    /// then whatever it left.
    pub fn call(&mut self, address: u16, max_cycles: i64) -> bool {
        let mut cpu = Cpu6502::new();
        // RTS pulls the return address minus one.
        let [low, high] = RETURN.wrapping_sub(1).to_le_bytes();
        self.ram[0x1FF] = high;
        self.ram[0x1FE] = low;
        cpu.s = 0xFD;
        cpu.pc = address;
        let end = self.cycles + max_cycles;
        while self.cycles < end {
            cpu.step(self);
            if cpu.pc == RETURN {
                return true;
            }
        }
        false
    }

    /// RAM byte at `address`.
    #[must_use]
    pub fn peek_ram(&self, address: u16) -> u8 {
        self.ram[usize::from(address) & 0x7FF]
    }

    /// Writes RAM byte `address`.
    pub fn poke_ram(&mut self, address: u16, value: u8) {
        self.ram[usize::from(address) & 0x7FF] = value;
    }
}

impl CpuBus for Guest<'_> {
    fn read(&mut self, address: u16) -> u8 {
        self.cycles += 1;
        self.peek(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.cycles += 1;
        if address < 0x2000 {
            self.ram[usize::from(address) & 0x7FF] = value;
        }
    }

    fn peek(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => self.ram[usize::from(address) & 0x7FF],
            0x8000..=0xFFFF if !self.prg_rom.is_empty() => {
                self.prg_rom[usize::from(address - 0x8000) % self.prg_rom.len()]
            }
            _ => 0,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_routine_runs_over_the_copy_and_returns() {
        // $8000: LDA $10 / CLC / ADC #$05 / STA $11 / STA $2000 / RTS
        let mut prg = vec![0xEA; 0x8000];
        prg[..10].copy_from_slice(&[0xA5, 0x10, 0x18, 0x69, 0x05, 0x85, 0x11, 0x8D, 0x00, 0x20]);
        prg[10] = 0x60;
        let mut ram = [0u8; 0x800];
        ram[0x10] = 7;
        let mut guest = Guest::new(&ram, &prg);
        assert!(guest.call(0x8000, 1000));
        assert_eq!(guest.peek_ram(0x11), 12);
        assert_eq!(ram[0x11], 0, "the console's RAM is untouched");
        // An endless loop ($8000: JMP $8000) stops at the cap.
        prg[..3].copy_from_slice(&[0x4C, 0x00, 0x80]);
        let mut guest = Guest::new(&ram, &prg);
        assert!(!guest.call(0x8000, 1000));
    }
}
