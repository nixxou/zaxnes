//! Cartridges (port of `src/Zax.Nes/Cartridges`).
//!
//! Stub for T01: a plain NROM board, just enough for the CPU bus and nestest. T04
//! replaces it with the real cartridge and mapper set, keeping the CPU-side methods.

/// Default PRG-RAM size, 8 KiB at $6000-$7FFF.
const DEFAULT_PRG_RAM_SIZE: usize = 0x2000;

/// A cartridge plugged into the CPU and PPU buses.
#[derive(Debug, Clone)]
pub struct Cartridge {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    cpu_cycle: i64,
}

impl Cartridge {
    /// Builds an NROM board (mapper 0) from its PRG-ROM, 16 KiB (mirrored) or 32 KiB.
    #[must_use]
    pub fn nrom(prg_rom: Vec<u8>) -> Self {
        Self {
            prg_rom,
            prg_ram: vec![0; DEFAULT_PRG_RAM_SIZE],
            cpu_cycle: 0,
        }
    }

    /// iNES mapper number of this cartridge.
    #[must_use]
    #[allow(clippy::unused_self)] // Stub: NROM only until T04.
    pub const fn mapper_number(&self) -> i32 {
        0
    }

    /// Whether the cartridge is asserting the CPU IRQ line.
    #[must_use]
    #[allow(clippy::unused_self)] // Stub: NROM has no IRQ.
    pub const fn irq_asserted(&self) -> bool {
        false
    }

    /// CPU cycles elapsed since power-on, as the cartridge sees them.
    #[must_use]
    pub const fn cpu_cycle(&self) -> i64 {
        self.cpu_cycle
    }

    /// Reads a byte the cartridge drives on the CPU bus, in $4020-$FFFF; `None` when it does
    /// not drive the bus there and the caller keeps the open-bus value (REQ-CORE-081).
    pub fn try_cpu_read(&mut self, address: u16) -> Option<u8> {
        self.try_cpu_peek(address)
    }

    /// Side-effect-free counterpart of [`Cartridge::try_cpu_read`], used by the host memory
    /// accessor (REQ-CORE-080, REQ-CORE-081).
    #[must_use]
    pub fn try_cpu_peek(&self, address: u16) -> Option<u8> {
        if (0x6000..=0x7FFF).contains(&address) {
            return Some(self.prg_ram[usize::from(address - 0x6000) % self.prg_ram.len()]);
        }

        if address < 0x8000 || self.prg_rom.is_empty() {
            return None;
        }

        Some(self.prg_rom[usize::from(address - 0x8000) % self.prg_rom.len()])
    }

    /// Writes a byte from the CPU bus, in $4020-$FFFF.
    pub fn cpu_write(&mut self, address: u16, value: u8) {
        if (0x6000..=0x7FFF).contains(&address) {
            let len = self.prg_ram.len();
            self.prg_ram[usize::from(address - 0x6000) % len] = value;
        }
    }

    /// Advances the cartridge by one CPU cycle.
    pub fn step_cpu_cycle(&mut self) {
        self.cpu_cycle += 1;
    }
}
