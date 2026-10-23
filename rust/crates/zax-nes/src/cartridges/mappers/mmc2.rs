use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers 9 (MMC2) and 10 (MMC4): two 4 KiB CHR windows, each with a latch that the
/// PPU flips by fetching tiles $FD or $FE (REQ-CORE-100, REQ-CORE-039).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc2 {
    banks: BankWindows,
    mapper_number: i32,
    is_mmc4: bool,
    chr: [u8; 4],
    prg: u8,
    latch_low: u8,
    latch_high: u8,
}

impl Mmc2 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        Self {
            banks: windows_for(memory),
            mapper_number,
            is_mmc4: mapper_number == 10,
            chr: [0; 4],
            prg: 0,
            latch_low: 1,
            latch_high: 1,
        }
    }

    fn update_banks(&mut self) {
        if self.is_mmc4 {
            self.banks.map_prg_16k(0, i32::from(self.prg));
            self.banks.map_prg_16k(2, -1);
        } else {
            self.banks.map_prg_8k(0, i32::from(self.prg));
            self.banks.map_prg_8k(1, -3);
            self.banks.map_prg_8k(2, -2);
            self.banks.map_prg_8k(3, -1);
        }

        self.banks
            .map_chr_4k(0, i32::from(self.chr[usize::from(self.latch_low)]));
        self.banks
            .map_chr_4k(4, i32::from(self.chr[2 + usize::from(self.latch_high)]));
    }

    fn move_latches(&mut self, address: u16) {
        if address < 0x1000 {
            let (fd, fe) = if self.is_mmc4 {
                (
                    (0x0FD8..=0x0FDF).contains(&address),
                    (0x0FE8..=0x0FEF).contains(&address),
                )
            } else {
                (address == 0x0FD8, address == 0x0FE8)
            };
            if fd {
                self.set_latch(0, 0);
            } else if fe {
                self.set_latch(0, 1);
            }

            return;
        }

        if (0x1FD8..=0x1FDF).contains(&address) {
            self.set_latch(1, 0);
        } else if (0x1FE8..=0x1FEF).contains(&address) {
            self.set_latch(1, 1);
        }
    }

    fn set_latch(&mut self, which: usize, value: u8) {
        let latch = if which == 0 {
            &mut self.latch_low
        } else {
            &mut self.latch_high
        };
        if *latch == value {
            return;
        }

        *latch = value;
        self.update_banks();
    }
}

impl Mapper for Mmc2 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_u8(self.prg);
        writer.write_i32(i32::from(self.latch_low));
        writer.write_i32(i32::from(self.latch_high));
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        self.prg = reader.read_u8()?;
        self.latch_low = u8::from(reader.read_index(2)? == 1);
        self.latch_high = u8::from(reader.read_index(2)? == 1);
        Ok(())
    }

    fn chr_view<'m>(
        &self,
        _memory: &'m crate::cartridges::CartridgeMemory,
    ) -> Option<crate::cartridges::ChrView<'m>> {
        // Its fetches have side effects or another source: the PPU takes the full path.
        None
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn chr_latch_state(&self) -> i32 {
        i32::from(self.latch_low | (self.latch_high << 1))
    }

    fn set_chr_latch_state(&mut self, state: i32) {
        self.latch_low = u8::from(state & 0x01 != 0);
        self.latch_high = u8::from((state >> 1) & 0x01 != 0);
        self.update_banks();
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 4];
        self.prg = 0;
        // Power-on: both latches select the $FE register (nesdev).
        self.latch_low = 1;
        self.latch_high = 1;
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xF000 {
            0xA000 => self.prg = value & 0x0F,
            0xB000 => self.chr[0] = value & 0x1F,
            0xC000 => self.chr[1] = value & 0x1F,
            0xD000 => self.chr[2] = value & 0x1F,
            0xE000 => self.chr[3] = value & 0x1F,
            0xF000 => {
                memory.set_mirroring(if value & 0x01 != 0 {
                    NametableMirroring::Horizontal
                } else {
                    NametableMirroring::Vertical
                });
                return;
            }
            _ => return,
        }

        self.update_banks();
    }

    fn ppu_read(&mut self, memory: &mut CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        let value = self.ppu_peek(memory, address, fetch);
        self.move_latches(address & 0x1FFF);
        value
    }

    fn advance_chr_latch(&mut self, pattern_table: i32, tile: i32) {
        let which = usize::from(pattern_table & 0x1000 != 0);
        if tile == 0xFD {
            self.set_latch(which, 0);
        } else if tile == 0xFE {
            self.set_latch(which, 1);
        }
    }
}
