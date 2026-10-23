use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::NametableMirroring;

/// iNES mapper 32, Irem G-101: two 8 KiB PRG banks, eight 1 KiB CHR banks, PRG swap mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IremG101 {
    banks: BankWindows,
    chr: [u8; 8],
    prg0: u8,
    prg1: u8,
    swap_prg: bool,
}

impl IremG101 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            chr: [0; 8],
            prg0: 0,
            prg1: 1,
            swap_prg: false,
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
        for slot in 0..8 {
            self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
        }
    }
}

impl Mapper for IremG101 {
    banked!();

    fn mapper_number(&self) -> i32 {
        32
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 8];
        self.prg0 = 0;
        self.prg1 = 1;
        self.swap_prg = false;
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xF000 {
            0x8000 => self.prg0 = value,
            0x9000 => {
                memory.set_mirroring(if value & 0x01 != 0 {
                    NametableMirroring::Horizontal
                } else {
                    NametableMirroring::Vertical
                });
                self.swap_prg = value & 0x02 != 0;
            }
            0xA000 => self.prg1 = value,
            0xB000 => self.chr[usize::from(address & 0x07)] = value,
            _ => return,
        }

        self.update_banks();
    }
}
