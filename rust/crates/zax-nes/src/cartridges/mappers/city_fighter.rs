//! NES 2.0 mapper 266, the *City Fighter IV* board (lot MAPPERS-N): a VRC4 clone with CPU A13 and
//! A14 crossed, an outer 32 KiB PRG register and a 4-bit DAC.
//!
//! nesdev ("NES 2.0 Mapper 266"): "CPU A13 connects to VRC4 A14, and CPU A14 connects to VRC4
//! A13", CPU A3/A2 on the chip's two register lines (the VRC4e wiring once A13/A14 are
//! uncrossed, as puNES `mapper_266.c` initialises it, read and not copied); `$900C` (mask `$F80C`)
//! "PRG higher address select: bits 1-0 (PP) give PRG A16-A15", while "PRG A13 and A14 stay
//! connected to VRC4, so VRC4 can still select 8 KiB inner banks"; `$980C` the DAC; the rest is
//! the VRC4's, its IRQ included.
//!
//! puNES takes the inner bank from the window number instead of the VRC4's register, and Mesen2
//! (`CityFighter.h`) has a register file of its own with a 9-bit CPU counter; this board follows
//! the page (FIDELITY-LEADS J31). **The DAC is not emulated**: its writes are dropped (J31).
use super::{Mapper, Vrc2Vrc4};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// Mapper 266.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityFighter {
    vrc: Vrc2Vrc4,
    /// `$900C`'s PRG A16-A15.
    outer: u8,
}

impl CityFighter {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            vrc: Vrc2Vrc4::new_vrc4e(memory, 266),
            outer: 0,
        }
    }

    /// The VRC4's 8 KiB banks keep their low two bits; `PP` gives the rest.
    fn narrow(&mut self) {
        let bits = i32::from(self.outer & 0x03) << 2;
        if let Some(banks) = self.vrc.banks_mut() {
            banks.remap_prg_banks(0x03, bits);
        }
    }
}

impl Mapper for CityFighter {
    fn banks(&self) -> Option<&BankWindows> {
        self.vrc.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.vrc.banks_mut()
    }

    fn mapper_number(&self) -> i32 {
        266
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.vrc.save_registers(writer);
        writer.write_u8(self.outer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.vrc.load_registers(reader)?;
        self.outer = reader.read_u8()?;
        self.narrow();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.outer = 0;
        self.vrc.reset(memory);
        self.narrow();
    }

    fn irq_asserted(&self) -> bool {
        self.vrc.irq_asserted()
    }

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        self.vrc.on_cpu_cycle(memory);
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        match address & 0xF80C {
            0x900C => self.outer = value & 0x03,
            // The DAC (not emulated).
            0x980C => {}
            _ => {
                let chip =
                    (address & 0x9FFF) | ((address & 0x2000) << 1) | ((address & 0x4000) >> 1);
                self.vrc.write_register(memory, chip, value);
            }
        }
        self.narrow();
    }
}
