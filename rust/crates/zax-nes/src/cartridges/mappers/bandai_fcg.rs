use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::NametableMirroring;

/// iNES mapper 16, Bandai FCG: registers at $6000-$FFFF decoded on the low nibble, a 16-bit
/// CPU-cycle IRQ counter (REQ-CORE-100).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandaiFcg {
    banks: BankWindows,
    chr: [u8; 8],
    prg: u8,
    irq_latch: i32,
    irq_counter: i32,
    irq_enabled: bool,
    irq_pending: bool,
}

impl BandaiFcg {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            chr: [0; 8],
            prg: 0,
            irq_latch: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.prg));
        self.banks.map_prg_16k(2, -1);
        for slot in 0..BankWindows::CHR_WINDOWS {
            self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
        }
    }
}

impl Mapper for BandaiFcg {
    banked!();

    fn mapper_number(&self) -> i32 {
        16
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    fn prg_ram_enabled(&self) -> bool {
        false
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 8];
        self.prg = 0;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.update_banks();
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if address >= 0x6000 {
            self.write_register(memory, address, value);
            return true;
        }

        false
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x6000 {
            return;
        }

        match address & 0x000F {
            0x0..=0x7 => {
                self.chr[usize::from(address & 0x07)] = value;
                self.update_banks();
            }
            0x8 => {
                self.prg = value;
                self.update_banks();
            }
            0x9 => memory.set_mirroring(match value & 0x03 {
                0 => NametableMirroring::Vertical,
                1 => NametableMirroring::Horizontal,
                2 => NametableMirroring::SingleScreenA,
                _ => NametableMirroring::SingleScreenB,
            }),
            0xA => {
                self.irq_pending = false;
                self.irq_enabled = value & 0x01 != 0;
                self.irq_counter = self.irq_latch;
            }
            0xB => self.irq_latch = (self.irq_latch & 0xFF00) | i32::from(value),
            0xC => self.irq_latch = (self.irq_latch & 0x00FF) | (i32::from(value) << 8),
            // $x00D is the EEPROM port (REQ-CORE-100: P3).
            _ => {}
        }
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.irq_enabled {
            return;
        }

        self.irq_counter -= 1;
        if self.irq_counter < 0 {
            self.irq_counter = 0xFFFF;
            self.irq_pending = true;
        }
    }
}
