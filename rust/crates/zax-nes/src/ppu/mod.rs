//! The 2C02 PPU (port of `src/Zax.Nes/Ppu`).
//!
//! Stub for T01: no rendering and no timing, only OAM so that OAM DMA can be exercised.
//! T02 replaces it, keeping the bus-facing methods. The cartridge is passed in by the bus
//! because the bus owns it.

use crate::cartridges::Cartridge;

/// The picture processing unit.
#[derive(Debug, Clone)]
pub struct Ppu2C02 {
    oam: [u8; 256],
    oam_address: u8,
    nmi_line_sample: bool,
}

impl Default for Ppu2C02 {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu2C02 {
    /// Creates a PPU in its power-on state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            oam: [0; 256],
            oam_address: 0,
            nmi_line_sample: false,
        }
    }

    /// The /NMI line as the CPU samples it near the start of the cycle.
    #[must_use]
    pub const fn nmi_line_sample(&self) -> bool {
        self.nmi_line_sample
    }

    /// Primary OAM.
    #[must_use]
    pub const fn oam(&self) -> &[u8; 256] {
        &self.oam
    }

    /// Advances the PPU by the dots of one CPU cycle.
    pub fn step_cpu_cycle(&mut self, _cartridge: Option<&mut Cartridge>) {}

    /// Reads register `register` (0-7) with its side effects.
    #[allow(clippy::unused_self)] // Stub until T02.
    pub fn read(&mut self, _register: u16, _cartridge: Option<&mut Cartridge>) -> u8 {
        0
    }

    /// Reads register `register` (0-7) without side effects.
    #[must_use]
    #[allow(clippy::unused_self)] // Stub until T02.
    pub const fn peek(&self, _register: u16) -> u8 {
        0
    }

    /// Writes register `register` (0-7).
    pub fn write(&mut self, register: u16, value: u8, _cartridge: Option<&mut Cartridge>) {
        match register & 0x07 {
            3 => self.oam_address = value,
            4 => self.write_oam(value),
            _ => {}
        }
    }

    /// Writes one OAM byte, used by OAM DMA ($4014).
    pub fn write_oam_dma(&mut self, value: u8) {
        self.write_oam(value);
    }

    fn write_oam(&mut self, value: u8) {
        // Bits 2 to 4 of a sprite's attribute byte do not exist in the chip.
        self.oam[usize::from(self.oam_address)] = if (self.oam_address & 0x03) == 0x02 {
            value & 0xE3
        } else {
            value
        };
        self.oam_address = self.oam_address.wrapping_add(1);
    }
}
