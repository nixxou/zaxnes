use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::NametableMirroring;

/// iNES mapper 18, Jaleco SS88006: nibble-wide bank registers and a CPU-cycle IRQ counter of
/// selectable width (REQ-CORE-100).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JalecoSs88006 {
    banks: BankWindows,
    prg: [u8; 3],
    chr: [u8; 8],
    irq_latch: i32,
    irq_counter: i32,
    irq_width: i32,
    irq_enabled: bool,
    irq_pending: bool,
}

fn set_nibble(register: &mut u8, half: u16, nibble: u8) {
    *register = if half == 0 {
        (*register & 0xF0) | nibble
    } else {
        (*register & 0x0F) | (nibble << 4)
    };
}

impl JalecoSs88006 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            prg: [0, 1, 2],
            chr: [0; 8],
            irq_latch: 0,
            irq_counter: 0,
            irq_width: 16,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    fn write_irq_control(&mut self, memory: &mut CartridgeMemory, index: u16, value: u8) {
        match index {
            0 => {
                self.irq_counter = self.irq_latch;
                self.irq_pending = false;
            }
            1 => {
                self.irq_enabled = value & 0x01 != 0;
                self.irq_pending = false;
                self.irq_width = if value & 0x08 != 0 {
                    4
                } else if value & 0x04 != 0 {
                    8
                } else if value & 0x02 != 0 {
                    12
                } else {
                    16
                };
            }
            2 => memory.set_mirroring(match value & 0x03 {
                0 => NametableMirroring::Horizontal,
                1 => NametableMirroring::Vertical,
                2 => NametableMirroring::SingleScreenA,
                _ => NametableMirroring::SingleScreenB,
            }),
            _ => {}
        }
    }

    fn update_banks(&mut self) {
        for slot in 0..3 {
            self.banks.map_prg_8k(slot, i32::from(self.prg[slot]));
        }

        self.banks.map_prg_8k(3, -1);
        for slot in 0..BankWindows::CHR_WINDOWS {
            self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
        }
    }
}

impl Mapper for JalecoSs88006 {
    banked!();

    fn mapper_number(&self) -> i32 {
        18
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg = [0, 1, 2];
        self.chr = [0; 8];
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_width = 16;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let group = (address >> 12) & 0x07;
        let index = address & 0x03;
        let nibble = value & 0x0F;
        match group {
            // $8000-$9003: the three PRG banks.
            0 => {
                set_nibble(&mut self.prg[usize::from(index >> 1)], index & 1, nibble);
                self.update_banks();
            }
            1 => {
                if index < 2 {
                    set_nibble(&mut self.prg[2], index, nibble);
                    self.update_banks();
                }
            }
            // $A000-$D003: the eight CHR banks.
            2..=5 => {
                set_nibble(
                    &mut self.chr[usize::from((group - 2) * 2 + (index >> 1))],
                    index & 1,
                    nibble,
                );
                self.update_banks();
            }
            // $E000-$E003: the four nibbles of the IRQ latch.
            6 => {
                let shift = u32::from(index) * 4;
                self.irq_latch = (self.irq_latch & !(0x0F << shift)) | (i32::from(nibble) << shift);
            }
            _ => self.write_irq_control(memory, index, value),
        }
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.irq_enabled {
            return;
        }

        let mask = if self.irq_width == 16 {
            0xFFFF
        } else {
            (1 << self.irq_width) - 1
        };
        let counted = self.irq_counter & mask;
        if counted == 0 {
            self.irq_pending = true;
            self.irq_counter = (self.irq_counter & !mask) | mask;
            return;
        }

        self.irq_counter = (self.irq_counter & !mask) | (counted - 1);
    }
}
