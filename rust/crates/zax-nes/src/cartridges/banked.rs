use super::wrap_offset;
use crate::state::{InvalidState, StateReader, StateWriter};

/// Bank windows of every switching board: four 8 KiB PRG windows at $8000-$FFFF and eight
/// 1 KiB CHR windows at $0000-$1FFF (port of `BankedCartridge`, REQ-CORE-100).
///
/// All the boards of REQ-CORE-100 switch PRG in multiples of 8 KiB and CHR in multiples of
/// 1 KiB, so one pair of window tables expresses every layout: a mapper only recomputes the
/// windows when one of its registers changes, and the bus path stays a table lookup. Bank
/// numbers are wrapped modulo the number of banks the ROM actually has, which is what lets a
/// board select "the last bank" with a negative index and keeps a short dump from reading
/// out of bounds.
///
/// A PRG window may also show PRG-RAM, which MMC5 needs ($5113-$5117 pick ROM or RAM per
/// window); CHR windows address CHR-ROM when the cartridge has any and CHR-RAM otherwise.
/// The memories themselves stay in the [`super::Cartridge`]; the board passes them in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankWindows {
    prg_offset: [usize; Self::PRG_WINDOWS],
    prg_is_ram: [bool; Self::PRG_WINDOWS],
    chr_offset: [usize; Self::CHR_WINDOWS],
    prg_bank_count_8k: i32,
    chr_bank_count_1k: i32,
    prg_ram_bank_count_8k: i32,
}

impl BankWindows {
    /// Size of a PRG window, in bytes.
    pub const PRG_WINDOW_SIZE: usize = 0x2000;

    /// Size of a CHR window, in bytes.
    pub const CHR_WINDOW_SIZE: usize = 0x0400;

    /// Number of PRG windows between $8000 and $FFFF.
    pub const PRG_WINDOWS: usize = 4;

    /// Number of CHR windows between $0000 and $1FFF.
    pub const CHR_WINDOWS: usize = 8;

    /// Initialises the window tables to a plain linear layout, for a cartridge with
    /// `prg_rom_len` bytes of PRG-ROM, `chr_len` bytes of CHR (ROM, or RAM when it has no
    /// ROM) and `work_ram_len` bytes of PRG-RAM.
    #[must_use]
    pub fn new(prg_rom_len: usize, chr_len: usize, work_ram_len: usize) -> Self {
        let count = |len: usize, size: usize| i32::try_from(len / size).unwrap_or(i32::MAX).max(1);
        let mut windows = Self {
            prg_offset: [0; Self::PRG_WINDOWS],
            prg_is_ram: [false; Self::PRG_WINDOWS],
            chr_offset: [0; Self::CHR_WINDOWS],
            prg_bank_count_8k: count(prg_rom_len, Self::PRG_WINDOW_SIZE),
            chr_bank_count_1k: count(chr_len, Self::CHR_WINDOW_SIZE),
            prg_ram_bank_count_8k: count(work_ram_len, Self::PRG_WINDOW_SIZE),
        };

        for slot in 0..Self::PRG_WINDOWS {
            windows.map_prg_8k(slot, slot_bank(slot));
        }

        for slot in 0..Self::CHR_WINDOWS {
            windows.map_chr_1k(slot, slot_bank(slot));
        }

        windows
    }

    /// Number of 8 KiB PRG banks the ROM holds, at least one.
    #[must_use]
    pub const fn prg_bank_count_8k(&self) -> i32 {
        self.prg_bank_count_8k
    }

    /// Number of 1 KiB CHR banks the cartridge holds, at least one.
    #[must_use]
    pub const fn chr_bank_count_1k(&self) -> i32 {
        self.chr_bank_count_1k
    }

    /// Number of 8 KiB PRG-RAM banks the cartridge holds, at least one.
    #[must_use]
    pub const fn prg_ram_bank_count_8k(&self) -> i32 {
        self.prg_ram_bank_count_8k
    }

    // ---------------------------------------------------------------- window setters

    /// Maps 8 KiB PRG bank `bank` at window `slot` (0 for $8000 to 3 for $E000); a negative
    /// bank counts back from the last bank.
    pub fn map_prg_8k(&mut self, slot: usize, bank: i32) {
        self.prg_offset[slot] = wrap(bank, self.prg_bank_count_8k) * Self::PRG_WINDOW_SIZE;
        self.prg_is_ram[slot] = false;
    }

    /// Maps a 16 KiB PRG bank over two windows starting at `slot`.
    pub fn map_prg_16k(&mut self, slot: usize, bank: i32) {
        self.map_prg_8k(slot, bank.wrapping_mul(2));
        self.map_prg_8k(slot + 1, bank.wrapping_mul(2).wrapping_add(1));
    }

    /// Maps a 32 KiB PRG bank over the whole $8000-$FFFF range.
    pub fn map_prg_32k(&mut self, bank: i32) {
        for slot in 0..Self::PRG_WINDOWS {
            self.map_prg_8k(slot, bank.wrapping_mul(4).wrapping_add(slot_bank(slot)));
        }
    }

    /// Shows 8 KiB of PRG-RAM at window `slot` (MMC5).
    pub fn map_prg_ram_8k(&mut self, slot: usize, bank: i32) {
        self.prg_offset[slot] = wrap(bank, self.prg_ram_bank_count_8k) * Self::PRG_WINDOW_SIZE;
        self.prg_is_ram[slot] = true;
    }

    /// Maps 1 KiB CHR bank `bank` at window `slot`.
    pub fn map_chr_1k(&mut self, slot: usize, bank: i32) {
        self.chr_offset[slot] = wrap(bank, self.chr_bank_count_1k) * Self::CHR_WINDOW_SIZE;
    }

    /// Maps a 2 KiB CHR bank over two windows starting at `slot`.
    pub fn map_chr_2k(&mut self, slot: usize, bank: i32) {
        self.map_chr_1k(slot, bank.wrapping_mul(2));
        self.map_chr_1k(slot + 1, bank.wrapping_mul(2).wrapping_add(1));
    }

    /// Maps a 4 KiB CHR bank over four windows starting at `slot`.
    pub fn map_chr_4k(&mut self, slot: usize, bank: i32) {
        for i in 0..4 {
            self.map_chr_1k(slot + i, bank.wrapping_mul(4).wrapping_add(slot_bank(i)));
        }
    }

    /// Maps an 8 KiB CHR bank over the whole $0000-$1FFF range.
    pub fn map_chr_8k(&mut self, bank: i32) {
        for slot in 0..Self::CHR_WINDOWS {
            self.map_chr_1k(slot, bank.wrapping_mul(8).wrapping_add(slot_bank(slot)));
        }
    }

    // ---------------------------------------------------------------- bus

    /// Reads PRG-ROM, or PRG-RAM when the window shows it, at `address`; `None` below
    /// $8000.
    #[must_use]
    pub fn read_prg(&self, address: u16, rom: &[u8], ram: &[u8]) -> Option<u8> {
        if address < 0x8000 {
            return None;
        }

        let slot = usize::from(address - 0x8000) / Self::PRG_WINDOW_SIZE;
        let offset = self.prg_offset[slot] + (usize::from(address) & (Self::PRG_WINDOW_SIZE - 1));
        if self.prg_is_ram[slot] {
            return Some(if ram.is_empty() {
                0
            } else {
                ram[wrap_offset(offset, ram.len())]
            });
        }

        Some(rom[wrap_offset(offset, rom.len())])
    }

    /// Writes a PRG window that currently shows PRG-RAM (MMC5); `false` when the window
    /// shows ROM, which ignores writes.
    pub fn try_write_prg_window(
        &self,
        address: u16,
        value: u8,
        prg_ram: &mut [u8],
        writable: bool,
    ) -> bool {
        if address < 0x8000 {
            return false;
        }

        let slot = usize::from(address - 0x8000) / Self::PRG_WINDOW_SIZE;
        if !self.prg_is_ram[slot] {
            return false;
        }

        if !prg_ram.is_empty() && writable {
            let len = prg_ram.len();
            prg_ram[wrap_offset(
                self.prg_offset[slot] + (usize::from(address) & (Self::PRG_WINDOW_SIZE - 1)),
                len,
            )] = value;
        }

        true
    }

    /// Reads CHR through the window table, without any side effect; `chr` is the CHR-ROM,
    /// or the CHR-RAM on a board without ROM.
    #[must_use]
    pub fn read_chr(&self, address: u16, chr: &[u8]) -> u8 {
        if chr.is_empty() {
            return 0;
        }

        chr[wrap_offset(self.chr_offset(address), chr.len())]
    }

    /// Writes CHR-RAM through the window table; the caller skips it on a CHR-ROM board.
    pub fn write_chr(&self, address: u16, value: u8, chr_ram: &mut [u8]) {
        if !chr_ram.is_empty() {
            let len = chr_ram.len();
            chr_ram[wrap_offset(self.chr_offset(address), len)] = value;
        }
    }

    fn chr_offset(&self, address: u16) -> usize {
        let index = usize::from(address) & 0x1FFF;
        self.chr_offset[index / Self::CHR_WINDOW_SIZE] + (index & (Self::CHR_WINDOW_SIZE - 1))
    }
}

/// A window index as a bank number (both are below 8).
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // slot < 8.
const fn slot_bank(slot: usize) -> i32 {
    slot as i32
}

/// Wraps `bank` into [0, `count`[; a negative bank counts back from the end, so -1 is the
/// last bank.
fn wrap(bank: i32, count: i32) -> usize {
    usize::try_from(bank.rem_euclid(count)).unwrap_or(0)
}

/// The CHR a board's pattern fetches read when reading has no side effect: the CHR memory
/// and the offset of each 1 KiB window. [`ChrView::read`] answers exactly what
/// [`BankWindows::read_chr`] answers for the same windows.
#[derive(Debug, Clone, Copy)]
pub struct ChrView<'m> {
    data: &'m [u8],
    offsets: [usize; BankWindows::CHR_WINDOWS],
}

impl<'m> ChrView<'m> {
    /// A view through `windows` of `chr` (the CHR-ROM, or the CHR-RAM on a board without ROM).
    #[must_use]
    pub const fn new(chr: &'m [u8], windows: &BankWindows) -> Self {
        Self {
            data: chr,
            offsets: windows.chr_offset,
        }
    }

    /// A view of `chr` laid out linearly, as a board without CHR banking shows it.
    #[must_use]
    pub const fn linear(chr: &'m [u8]) -> Self {
        let mut offsets = [0; BankWindows::CHR_WINDOWS];
        let mut window = 0;
        while window < BankWindows::CHR_WINDOWS {
            offsets[window] = window * BankWindows::CHR_WINDOW_SIZE;
            window += 1;
        }
        Self { data: chr, offsets }
    }

    /// The offset of each 1 KiB window: the banks the view shows, to be shown again later.
    #[must_use]
    pub const fn offsets(&self) -> [usize; BankWindows::CHR_WINDOWS] {
        self.offsets
    }

    /// The same CHR memory through the windows `offsets` (from [`Self::offsets`]): what an
    /// earlier bank selection showed.
    #[must_use]
    pub const fn with_offsets(self, offsets: [usize; BankWindows::CHR_WINDOWS]) -> Self {
        Self {
            data: self.data,
            offsets,
        }
    }

    /// The byte at `address` ($0000-$1FFF), 0 when there is no CHR at all.
    #[must_use]
    pub fn read(&self, address: u16) -> u8 {
        if self.data.is_empty() {
            return 0;
        }

        let index = usize::from(address) & 0x1FFF;
        let offset = self.offsets[index / BankWindows::CHR_WINDOW_SIZE]
            + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
        self.data[wrap_offset(offset, self.data.len())]
    }
}

impl BankWindows {
    /// Appends the window tables to `writer` (REQ-CORE-090); the bank counts follow from the
    /// ROM and are not stored.
    pub fn save_state(&self, writer: &mut StateWriter) {
        for slot in 0..Self::PRG_WINDOWS {
            writer.write_usize(self.prg_offset[slot]);
            writer.write_bool(self.prg_is_ram[slot]);
        }

        for offset in self.chr_offset {
            writer.write_usize(offset);
        }
    }

    /// Restores the window tables from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds a negative offset.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for slot in 0..Self::PRG_WINDOWS {
            self.prg_offset[slot] = reader.read_usize()?;
            self.prg_is_ram[slot] = reader.read_bool()?;
        }

        for offset in &mut self.chr_offset {
            *offset = reader.read_usize()?;
        }

        Ok(())
    }
}
