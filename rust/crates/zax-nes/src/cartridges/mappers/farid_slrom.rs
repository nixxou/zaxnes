//! NES 2.0 mapper 323, Farid's SLROM 8-in-1 homebrew multicart (lot MAPPERS-N): an MMC1 whose
//! banks are confined to a 128 KiB PRG / 128 KiB CHR game by an outer register written at
//! `$6000-$7FFF` while the work RAM is enabled, until its lock bit is set.
//!
//! Mesen2 `Homebrew/FaridSlrom.h` and puNES `mapper_323.c` (read, not copied): PRG
//! `outer | (page & 7)` in 16 KiB pages, CHR `outer << 2 | (page & $1F)` in 4 KiB pages, the lock
//! in bit 3, everything cleared on reset. Mesen2 takes the outer bank from bits 4-6, puNES from
//! bits 4-7; this board takes puNES's four bits, which agree with Mesen2 on any 1 MiB image
//! (FIDELITY-LEADS J27). Mesen2 makes `$6000-$7FFF` a register range so the write does not reach
//! the RAM; puNES lets the RAM take it too, as this board does.
//!
//! A wrapper of [`Mmc1`], whose banks it narrows after every change with
//! [`BankWindows::remap_prg_banks`]/[`BankWindows::remap_chr_banks`]: the MMC1 itself stays on
//! lot CORE-2's measured path, untouched.
use super::{Mapper, Mmc1};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// Mapper 323.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaridSlrom {
    mmc1: Mmc1,
    outer: u8,
}

impl FaridSlrom {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            mmc1: Mmc1::new(memory),
            outer: 0,
        }
    }

    /// The outer bank over the MMC1's: 16 KiB pages `(outer & $F0) >> 1 | (page & 7)`, 4 KiB CHR
    /// pages `(outer & $F0) << 1 | (page & $1F)`, in the 8 KiB and 1 KiB units of the windows.
    fn narrow(&mut self) {
        let base = i32::from(self.outer & 0xF0);
        if let Some(banks) = self.mmc1.banks_mut() {
            banks.remap_prg_banks(0x0F, base);
            banks.remap_chr_banks(0x7F, base << 3);
        }
    }

    const fn locked(&self) -> bool {
        self.outer & 0x08 != 0
    }
}

impl Mapper for FaridSlrom {
    fn banks(&self) -> Option<&BankWindows> {
        self.mmc1.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc1.banks_mut()
    }

    fn mapper_number(&self) -> i32 {
        323
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc1.save_registers(writer);
        writer.write_u8(self.outer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc1.load_registers(reader)?;
        self.outer = reader.read_u8()?;
        self.narrow();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.outer = 0;
        self.mmc1.reset(memory);
        self.narrow();
    }

    fn prg_ram_enabled(&self) -> bool {
        self.mmc1.prg_ram_enabled()
    }

    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        self.mmc1.prg_ram_window(memory)
    }

    /// `$6000-$7FFF` with the RAM enabled and the lock clear: the outer register, and the RAM
    /// takes the byte as well (puNES). Written here, answering `true`, so that the cartridge
    /// refreshes its cached ROM windows after the outer bank moves them.
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if !(0x6000..0x8000).contains(&address) || !self.mmc1.prg_ram_enabled() || self.locked() {
            return false;
        }
        self.outer = value;
        self.narrow();
        let (base, wrap) = self.mmc1.prg_ram_window(memory);
        if wrap != 0 {
            let offset = (base + usize::from(address - 0x6000)) % wrap;
            if let Some(byte) = memory.prg_ram_mut().get_mut(offset) {
                *byte = value;
            }
        }
        true
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        self.mmc1.write_register(memory, address, value);
        self.narrow();
    }
}
