use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 156, Daou Infosys DIS23C01 (Daou 306): a 16 KiB PRG bank at $8000 with the last
/// one fixed at $C000, eight 1 KiB CHR banks of 16 bits (low bytes at $C000-$C003 and
/// $C008-$C00B, high bytes four addresses above), the PRG bank at $C010 and the mirroring at
/// $C014. One-screen at power-on, as on Mesen (`DaouInfosys.h`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Daou306 {
    banks: BankWindows,
    chr_low: [u8; 8],
    chr_high: [u8; 8],
    prg: u8,
}

impl Daou306 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            chr_low: [0; 8],
            chr_high: [0; 8],
            prg: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.prg));
        self.banks.map_prg_16k(2, -1);
        for slot in 0..8 {
            let bank = u16::from(self.chr_high[slot]) << 8 | u16::from(self.chr_low[slot]);
            self.banks.map_chr_1k(slot, i32::from(bank));
        }
    }
}

impl Mapper for Daou306 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr_low);
        writer.write_raw(&self.chr_high);
        writer.write_u8(self.prg);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr_low)?;
        reader.read_into(&mut self.chr_high)?;
        self.prg = reader.read_u8()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        156
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.chr_low = [0; 8];
        self.chr_high = [0; 8];
        self.prg = 0;
        memory.set_mirroring(NametableMirroring::SingleScreenA);
        self.update_banks();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let slot = usize::from(address & 0x03);
        match address {
            0xC000..=0xC003 => self.chr_low[slot] = value,
            0xC004..=0xC007 => self.chr_high[slot] = value,
            0xC008..=0xC00B => self.chr_low[4 + slot] = value,
            0xC00C..=0xC00F => self.chr_high[4 + slot] = value,
            0xC010 => self.prg = value,
            0xC014 => memory.set_mirroring(if value & 0x01 != 0 {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            }),
            _ => return,
        }

        self.update_banks();
    }
}
