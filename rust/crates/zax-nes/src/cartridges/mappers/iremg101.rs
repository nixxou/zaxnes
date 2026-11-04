use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 32, Irem G-101: two 8 KiB PRG banks, eight 1 KiB CHR banks, PRG swap mode.
///
/// NES 2.0 submapper 1 is *Major League*, whose board ties CIRAM A10 high and leaves $9000
/// undecoded (MR-32-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IremG101 {
    banks: BankWindows,
    chr: [u8; 8],
    prg0: u8,
    prg1: u8,
    swap_prg: bool,
    /// Submapper 1, decided at load.
    major_league: bool,
}

impl IremG101 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        Self {
            banks: windows_for(memory),
            chr: [0; 8],
            prg0: 0,
            prg1: 1,
            swap_prg: false,
            major_league: header.is_nes20 && header.submapper == 1,
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

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_u8(self.prg0);
        writer.write_u8(self.prg1);
        writer.write_bool(self.swap_prg);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        self.prg0 = reader.read_u8()?;
        self.prg1 = reader.read_u8()?;
        self.swap_prg = reader.read_bool()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        32
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.chr = [0; 8];
        self.prg0 = 0;
        self.prg1 = 1;
        self.swap_prg = false;
        if self.major_league {
            memory.set_mirroring(NametableMirroring::SingleScreenA);
        }

        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xF000 {
            0x8000 => self.prg0 = value,
            0x9000 => {
                // MR-32-1, nesdev ("INES Mapper 032"): "Major League wants hardwired 1-screen
                // mirroring. (CIRAM A10 is tied to +5V on this game). Additionally, the
                // register at $9000 is entirely disabled: the game can only request 'PRG mode
                // 0'." One condition covers both halves, since PRG mode 0 is `swap_prg` false.
                if self.major_league {
                    return;
                }

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
