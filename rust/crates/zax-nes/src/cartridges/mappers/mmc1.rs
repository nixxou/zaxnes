use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 1, MMC1 (SxROM), SUROM and SOROM included (REQ-CORE-100).
///
/// Each write to $8000-$FFFF shifts one bit into a five-bit serial register and the fifth
/// write commits it to the register the address selects. Two writes on consecutive CPU
/// cycles only count once (read-modify-write instructions). On 512 KiB boards bit 4 of CHR
/// bank 0 supplies the top PRG address line, and its `S` bits bank the PRG-RAM of SOROM and
/// SXROM when the header announces more than 8 KiB.
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
    /// NES 2.0 submapper 5 (MR-1-4), decided at load.
    fixed_prg: bool,
    /// How to read the `S` bits of CHR bank 0 for this board's PRG-RAM (MR-1-3), decided at
    /// load so that `prg_ram_offset` keeps exactly one shift and one mask.
    ram_bank_shift: u32,
    ram_bank_mask: u8,
}

impl Mmc1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        // MR-1-3, nesdev ("MMC1"): "For SXROM, the upper `S` (bit 3) selects the SRAM's A14,
        // and the lower `S` (bit 2) selects A13", and "The SOROM board only implements the
        // upper `S` bit". SOROM is the 16 KiB board, SXROM the 32 KiB one; our old
        // `(chr0 >> 2) & 0x03` reduced to bit 2 over a 16 KiB RAM, one bit off, which put the
        // SOROM titles' saves in the half that is not battery-backed.
        let (ram_bank_shift, ram_bank_mask) =
            match memory.prg_ram().len() / BankWindows::PRG_WINDOW_SIZE {
                0 | 1 => (0, 0x00),
                2 => (3, 0x01),
                _ => (2, 0x03),
            };
        Self {
            banks: windows_for(memory),
            shift: 0,
            shift_count: 0,
            control: 0x0C,
            chr0: 0,
            chr1: 0,
            prg: 0,
            last_write_cycle: -4,
            fixed_prg: header.is_nes20 && header.submapper == 5,
            ram_bank_shift,
            ram_bank_mask,
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

        // MR-1-4. NES 2.0 submapper 5 is SEROM, SHROM and SH1ROM, which "use a fixed 32k PRG
        // ROM with no banking support" (nesdev, "NES 2.0 submappers"); Dr. Mario is the dump
        // that names the board. The PRG register does nothing there.
        if self.fixed_prg {
            self.banks.map_prg_32k(0);
        } else {
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

    /// The `S` bits of CHR bank 0 pick the 8 KiB PRG-RAM chip; which bits, and whether there
    /// are any, is `Mmc1::new`'s decision (MR-1-3).
    fn prg_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        let bank = usize::from((self.chr0 >> self.ram_bank_shift) & self.ram_bank_mask);
        (bank * BankWindows::PRG_WINDOW_SIZE + usize::from(address - 0x6000))
            % memory.prg_ram().len()
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let ignored = memory.cpu_cycle() - self.last_write_cycle < 2;
        self.last_write_cycle = memory.cpu_cycle();

        // MR-1-1, nesdev ("MMC1"): the consecutive-write restriction "only applies to the data
        // being written on bit 0; the bit 7 reset is never ignored" — the sentence that names
        // Shinsenden, whose reset arrives as the second half of a read-modify-write. The test
        // used to sit above this write, so the reset was swallowed with the data.
        if value & 0x80 != 0 {
            self.shift = 0;
            self.shift_count = 0;
            self.control |= 0x0C;
            self.update_banks(memory);
            return;
        }

        if ignored {
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
