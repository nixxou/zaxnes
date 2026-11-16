//! iNES mapper 168, the RacerMate Challenge II board (lot MAPPERS-N): 64 KiB of PRG-ROM, 64 KiB of
//! CHR-RAM in 4 KiB banks, a free-running IRQ counter, and a RAM protection latch.
//!
//! From nesdev ("INES Mapper 168"), read alongside puNES `mapper_168.c`, which agrees with it
//! (both read, not copied); Mesen2's `Unlicensed/Racermate.h` is an older, simpler reading
//! (FIDELITY-LEADS J22):
//!
//! * `$8000-$BFFF`: "Select 16 KB PRG ROM bank for CPU $8000-$BFFF" from bits 7-6, `$C000` fixed
//!   to the last bank; "Select 4 KB CHR RAM bank for PPU $1000-$1FFF" from the low four bits.
//!   The page says the CHR banks are "the 2's complement of those specified" and that emulators
//!   "could use either convention"; this board takes puNES's, `(reg & $0F) ^ 8`, so the banks a
//!   game names first land in U2, the battery-backed chip.
//! * `$C000-$FFFF`: one control bit, A7 on the original board and D2 after the rework of "all
//!   delivered boards" — the firmware writes `$FF` to `$F080` and `$00` to `$F000` to cover both,
//!   so either sets it here. "When this bit is high, the IRQ is acknowledged and the IRQ counter
//!   is frozen at 0"; low, it "counts up every M2 cycle" and the line follows bit 10, low for 1024
//!   cycles of every 2048.
//! * the protection: "While RAM is protected, battery-backed CHR RAMs can neither be read nor
//!   written"; it is cleared "When this bit changes from 1 to 0" and set at power-on. Only U2,
//!   banks 8-15, is battery-backed in the factory configuration. A protected read returns 0
//!   here, where the hardware leaves the PPU's open bus.
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// Mapper 168.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Racermate {
    banks: BankWindows,
    reg: u8,
    /// The control bit: the IRQ acknowledged and its counter frozen.
    frozen: bool,
    protected: bool,
    counter: u16,
}

impl Racermate {
    /// nesdev: "64 KiB of RAM in 4 KiB banks".
    pub const CHR_RAM_SIZE: usize = 0x10000;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            reg: 0,
            frozen: false,
            protected: true,
            counter: 0,
        }
    }

    fn refresh(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.reg >> 6));
        self.banks.map_prg_16k(2, -1);
        self.banks.map_chr_4k(0, 0);
        self.banks
            .map_chr_4k(4, i32::from((self.reg & 0x0F) ^ 0x08));
    }

    /// The 4 KiB CHR bank `address` shows.
    const fn chr_bank(&self, address: u16) -> u8 {
        if address < 0x1000 {
            0
        } else {
            (self.reg & 0x0F) ^ 0x08
        }
    }

    /// Whether `address` reaches a protected battery-backed bank (U2, banks 8-15).
    const fn locked(&self, address: u16) -> bool {
        self.protected && self.chr_bank(address) >= 8
    }
}

impl Mapper for Racermate {
    banked!();

    fn mapper_number(&self) -> i32 {
        168
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.reg);
        writer.write_bool(self.frozen);
        writer.write_bool(self.protected);
        writer.write_u16(self.counter);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.reg = reader.read_u8()?;
        self.frozen = reader.read_bool()?;
        self.protected = reader.read_bool()?;
        self.counter = reader.read_u16()?;
        self.refresh();
        Ok(())
    }

    /// "Removing power from the cartridge" clears the IRQ disable bit and sets the protection.
    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.reg = 0;
        self.frozen = false;
        self.protected = true;
        self.counter = 0;
        self.refresh();
    }

    fn irq_asserted(&self) -> bool {
        !self.frozen && self.counter & 0x400 != 0
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.frozen {
            self.counter = self.counter.wrapping_add(1) & 0x07FF;
        }
    }

    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0xC000 {
            self.reg = value;
            self.refresh();
            return;
        }
        let bit = address & 0x0080 != 0 || value & 0x04 != 0;
        if self.frozen && !bit {
            self.protected = false;
        }
        self.frozen = bit;
        if bit {
            self.counter = 0;
        }
    }

    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        (!self.protected).then(|| ChrView::new(memory.chr_ram(), &self.banks))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        if self.locked(address) {
            return 0;
        }
        self.banks.read_chr(address, memory.chr_ram())
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if !self.locked(address) {
            self.banks.write_chr(address, value, memory.chr_ram_mut());
        }
    }
}
