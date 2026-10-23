use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 1, MMC1 (SxROM), SUROM and SOROM included (REQ-CORE-100).
///
/// Each write to $8000-$FFFF shifts one bit into a five-bit serial register and the fifth
/// write commits it to the register the address selects. Two writes on consecutive CPU
/// cycles only count once (read-modify-write instructions). On 512 KiB boards bit 4 of CHR
/// bank 0 supplies the top PRG address line, and bits 2-3 of it bank the PRG-RAM of SOROM
/// and SXROM when the header announces more than 8 KiB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc1 {
    banks: BankWindows,
    shift: i32,
    shift_count: i32,
    control: u8,
    chr0: u8,
    chr1: u8,
    prg: u8,
    last_write_cycle: i64,
}

impl Mmc1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            shift: 0,
            shift_count: 0,
            control: 0x0C,
            chr0: 0,
            chr1: 0,
            prg: 0,
            last_write_cycle: -4,
        }
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        // A18 of a 512 KiB board, in 16 KiB units.
        let outer = if memory.rom().prg_rom().len() > 256 * 1024 {
            i32::from((self.chr0 >> 4) & 0x01) << 4
        } else {
            0
        };
        let bank = i32::from(self.prg & 0x0F) | outer;

        match (self.control >> 2) & 0x03 {
            // 32 KiB at $8000, the low bit of the bank number ignored.
            0 | 1 => self.banks.map_prg_32k(bank >> 1),
            // First bank of the block fixed at $8000, switchable at $C000.
            2 => {
                self.banks.map_prg_16k(0, outer);
                self.banks.map_prg_16k(2, bank);
            }
            // Switchable at $8000, last bank of the block fixed at $C000.
            _ => {
                self.banks.map_prg_16k(0, bank);
                self.banks.map_prg_16k(2, outer | 0x0F);
            }
        }

        if self.control & 0x10 != 0 {
            self.banks.map_chr_4k(0, i32::from(self.chr0));
            self.banks.map_chr_4k(4, i32::from(self.chr1));
        } else {
            self.banks.map_chr_8k(i32::from(self.chr0 >> 1));
        }

        memory.set_mirroring(match self.control & 0x03 {
            0 => NametableMirroring::SingleScreenA,
            1 => NametableMirroring::SingleScreenB,
            2 => NametableMirroring::Vertical,
            _ => NametableMirroring::Horizontal,
        });
    }
}

impl Mapper for Mmc1 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_i32(self.shift);
        writer.write_i32(self.shift_count);
        writer.write_u8(self.control);
        writer.write_u8(self.chr0);
        writer.write_u8(self.chr1);
        writer.write_u8(self.prg);
        writer.write_i64(self.last_write_cycle);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.shift = reader.read_i32()?;
        self.shift_count = reader.read_i32()?;
        self.control = reader.read_u8()?;
        self.chr0 = reader.read_u8()?;
        self.chr1 = reader.read_u8()?;
        self.prg = reader.read_u8()?;
        self.last_write_cycle = reader.read_i64()?;
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        1
    }

    /// Bit 4 of the PRG register disables the PRG-RAM chip (nesdev, "MMC1").
    fn prg_ram_enabled(&self) -> bool {
        self.prg & 0x10 == 0
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.shift = 0;
        self.shift_count = 0;
        // Power-on: the last bank is fixed at $C000, the only state the chip is known to come
        // up in (nesdev); everything else starts at 0.
        self.control = 0x0C;
        self.chr0 = 0;
        self.chr1 = 0;
        self.prg = 0;
        // Far enough back that the very first write is never taken for the second half of a
        // read-modify-write pair.
        self.last_write_cycle = -4;
        self.update_banks(memory);
    }

    fn prg_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        let len = memory.prg_ram().len();
        let bank = if len > BankWindows::PRG_WINDOW_SIZE {
            usize::from((self.chr0 >> 2) & 0x03)
        } else {
            0
        };
        (bank * BankWindows::PRG_WINDOW_SIZE + usize::from(address - 0x6000)) % len
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let ignored = memory.cpu_cycle() - self.last_write_cycle < 2;
        self.last_write_cycle = memory.cpu_cycle();
        if ignored {
            return;
        }

        if value & 0x80 != 0 {
            self.shift = 0;
            self.shift_count = 0;
            self.control |= 0x0C;
            self.update_banks(memory);
            return;
        }

        self.shift |= i32::from(value & 0x01) << self.shift_count;
        self.shift_count += 1;
        if self.shift_count < 5 {
            return;
        }

        let data = u8::try_from(self.shift & 0x1F).unwrap_or(0);
        self.shift = 0;
        self.shift_count = 0;
        match address & 0x6000 {
            0x0000 => self.control = data,
            0x2000 => self.chr0 = data,
            0x4000 => self.chr1 = data,
            _ => self.prg = data,
        }

        self.update_banks(memory);
    }
}
