use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers 21, 22, 23, 25 and 27, Konami VRC2 and VRC4 (REQ-CORE-100): the same silicon
/// with the register address lines wired differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vrc2Vrc4 {
    banks: BankWindows,
    mapper_number: i32,
    chr: [i32; 8],
    has_irq: bool,
    chr_shift: bool,
    chr_high_mask: i32,
    prg0: u8,
    prg1: u8,
    swap_prg: bool,
    irq_latch: u8,
    irq_counter: i32,
    prescaler: i32,
    irq_enabled: bool,
    irq_enable_after_ack: bool,
    irq_cycle_mode: bool,
    irq_pending: bool,
}

impl Vrc2Vrc4 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        Self {
            banks: windows_for(memory),
            mapper_number,
            chr: [0; 8],
            // VRC2a (mapper 22) has neither the IRQ nor the 1 KiB CHR granularity.
            has_irq: mapper_number != 22,
            chr_shift: mapper_number == 22,
            // The pirate board of mapper 27 widens the high half of each CHR register from
            // five bits to eight.
            chr_high_mask: if mapper_number == 27 { 0xFF } else { 0x1F },
            prg0: 0,
            prg1: 1,
            swap_prg: false,
            irq_latch: 0,
            irq_counter: 0,
            prescaler: 341,
            irq_enabled: false,
            irq_enable_after_ack: false,
            irq_cycle_mode: false,
            irq_pending: false,
        }
    }

    fn register_index(&self, address: u16) -> i32 {
        let a = i32::from(address);
        match self.mapper_number {
            21 => (((a >> 1) | (a >> 6)) & 1) | ((((a >> 2) | (a >> 7)) & 1) << 1),
            22 | 25 => (((a >> 1) | (a >> 3)) & 1) | (((a | (a >> 2)) & 1) << 1),
            _ => ((a | (a >> 2)) & 1) | ((((a >> 1) | (a >> 3)) & 1) << 1),
        }
    }

    fn write_irq_register(&mut self, index: i32, value: u8) {
        match index {
            0 => self.irq_latch = (self.irq_latch & 0xF0) | (value & 0x0F),
            1 => self.irq_latch = (self.irq_latch & 0x0F) | ((value & 0x0F) << 4),
            2 => {
                self.irq_enable_after_ack = value & 0x01 != 0;
                self.irq_enabled = value & 0x02 != 0;
                self.irq_cycle_mode = value & 0x04 != 0;
                if self.irq_enabled {
                    self.irq_counter = i32::from(self.irq_latch);
                    self.prescaler = 341;
                }

                self.irq_pending = false;
            }
            _ => {
                self.irq_pending = false;
                self.irq_enabled = self.irq_enable_after_ack;
            }
        }
    }

    fn update_banks(&mut self) {
        if self.swap_prg {
            self.banks.map_prg_8k(0, -2);
            self.banks.map_prg_8k(2, i32::from(self.prg0));
        } else {
            self.banks.map_prg_8k(0, i32::from(self.prg0));
            self.banks.map_prg_8k(2, -2);
        }

        self.banks.map_prg_8k(1, i32::from(self.prg1));
        self.banks.map_prg_8k(3, -1);
        for slot in 0..BankWindows::CHR_WINDOWS {
            let bank = if self.chr_shift {
                self.chr[slot] >> 1
            } else {
                self.chr[slot]
            };
            self.banks.map_chr_1k(slot, bank);
        }
    }

    fn clock_irq_counter(&mut self) {
        if self.irq_counter == 0xFF {
            self.irq_counter = i32::from(self.irq_latch);
            self.irq_pending = true;
            return;
        }

        self.irq_counter += 1;
    }
}

impl Mapper for Vrc2Vrc4 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        for value in self.chr {
            writer.write_i32(value);
        }
        writer.write_u8(self.prg0);
        writer.write_u8(self.prg1);
        writer.write_bool(self.swap_prg);
        writer.write_u8(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_i32(self.prescaler);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_enable_after_ack);
        writer.write_bool(self.irq_cycle_mode);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for value in &mut self.chr {
            *value = reader.read_i32()?;
        }
        self.prg0 = reader.read_u8()?;
        self.prg1 = reader.read_u8()?;
        self.swap_prg = reader.read_bool()?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_i32()?;
        self.prescaler = reader.read_i32()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_enable_after_ack = reader.read_bool()?;
        self.irq_cycle_mode = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 8];
        self.prg0 = 0;
        self.prg1 = 1;
        self.swap_prg = false;
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.prescaler = 341;
        self.irq_enabled = false;
        self.irq_enable_after_ack = false;
        self.irq_cycle_mode = false;
        self.irq_pending = false;
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let index = self.register_index(address);
        match address & 0xF000 {
            0x8000 => {
                self.prg0 = value & 0x1F;
                self.update_banks();
            }
            0x9000 => {
                if index < 2 {
                    memory.set_mirroring(if self.has_irq {
                        match value & 0x03 {
                            0 => NametableMirroring::Vertical,
                            1 => NametableMirroring::Horizontal,
                            2 => NametableMirroring::SingleScreenA,
                            _ => NametableMirroring::SingleScreenB,
                        }
                    } else if value & 0x01 != 0 {
                        NametableMirroring::Horizontal
                    } else {
                        NametableMirroring::Vertical
                    });
                } else {
                    self.swap_prg = value & 0x02 != 0;
                    self.update_banks();
                }
            }
            0xA000 => {
                self.prg1 = value & 0x1F;
                self.update_banks();
            }
            0xB000..=0xE000 => {
                let group = i32::from((address & 0xF000) - 0xB000) >> 12;
                let slot = usize::try_from(group * 2 + (index >> 1)).unwrap_or(0);
                if index & 1 == 0 {
                    self.chr[slot] = (self.chr[slot] & !0x0F) | i32::from(value & 0x0F);
                } else {
                    self.chr[slot] =
                        (self.chr[slot] & 0x0F) | ((i32::from(value) & self.chr_high_mask) << 4);
                }

                self.update_banks();
            }
            _ => {
                if self.has_irq {
                    self.write_irq_register(index, value);
                }
            }
        }
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.has_irq || !self.irq_enabled {
            return;
        }

        if self.irq_cycle_mode {
            self.clock_irq_counter();
            return;
        }

        self.prescaler -= 3;
        if self.prescaler > 0 {
            return;
        }

        self.prescaler += 341;
        self.clock_irq_counter();
    }
}
