use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 16, Bandai FCG: registers at $6000-$FFFF decoded on the low nibble, a 16-bit
/// CPU-cycle IRQ counter (REQ-CORE-100).
///
/// NES 2.0 submapper 4 is the FCG-1/2, which has no IRQ latch at all: $x00B and $x00C write
/// the counter itself. Submapper 5 and an undeclared dump are the LZ93D50, where they write a
/// latch that $x00A copies across (MR-16-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandaiFcg {
    banks: BankWindows,
    chr: [u8; 8],
    prg: u8,
    /// Submapper 4, decided at load.
    is_fcg12: bool,
    irq_latch: i32,
    irq_counter: i32,
    irq_enabled: bool,
    irq_pending: bool,
}

impl BandaiFcg {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        Self {
            banks: windows_for(memory),
            chr: [0; 8],
            prg: 0,
            is_fcg12: header.is_nes20 && header.submapper == 4,
            irq_latch: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    /// The counter on an FCG-1/2, the latch on an LZ93D50: `nibble` replaces the low byte when
    /// it is false and the high byte when it is true (MR-16-1).
    fn write_irq_value(&mut self, high: bool, value: u8) {
        let target = if self.is_fcg12 {
            &mut self.irq_counter
        } else {
            &mut self.irq_latch
        };
        *target = if high {
            (*target & 0x00FF) | (i32::from(value) << 8)
        } else {
            (*target & 0xFF00) | i32::from(value)
        };
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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_u8(self.prg);
        writer.write_i32(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        self.prg = reader.read_u8()?;
        self.irq_latch = reader.read_i32()?;
        self.irq_counter = reader.read_i32()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        Ok(())
    }

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
                // The LZ93D50's $x00A copies the latch into the counter. The FCG-1/2 has no
                // latch to copy: "writing to these two registers directly modifies the counter
                // itself" (MR-16-1).
                if !self.is_fcg12 {
                    self.irq_counter = self.irq_latch;
                }
            }
            0xB => self.write_irq_value(false, value),
            0xC => self.write_irq_value(true, value),
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
