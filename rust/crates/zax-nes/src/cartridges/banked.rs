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

    /// Adds `banks` to every 1 KiB CHR window's bank number, each re-wrapped as
    /// [`Self::map_chr_1k`] wraps it.
    ///
    /// For a board whose outer CHR bank cannot be expressed through the sub-board that computed
    /// the windows: mapper 116 holds an MMC3, an MMC1 and a VRC2 whose `update_banks` write
    /// **absolute** bank numbers, and its supervisor register adds CHR A18 above all three. The
    /// bank is recovered from the offset rather than remembered, so the call is idempotent and
    /// the caller may re-apply it after every register write.
    ///
    /// Additive: no board but mapper 116 calls it, and it is behind `board-116`.
    #[cfg(feature = "board-116")]
    pub fn offset_chr_banks(&mut self, banks: i32) {
        for slot in 0..Self::CHR_WINDOWS {
            let bank = i32::try_from(self.chr_offset[slot] / Self::CHR_WINDOW_SIZE).unwrap_or(0);
            self.map_chr_1k(slot, bank.wrapping_add(banks));
        }
    }

    /// ORs a **per-window** bit pattern into each 1 KiB CHR window's bank number, each
    /// re-wrapped as [`Self::map_chr_1k`] wraps it.
    ///
    /// [`Self::offset_chr_banks`]'s counterpart for a board whose outer CHR line is **not the same
    /// line over all eight windows**: the Huang-1 on mappers 12 and 14 drives CHR A18 from an
    /// outer register, one bit per pattern table on mapper 12 and three bits over the unequal
    /// groups `$0000-$0FFF`, `$1000-$17FF` and `$1800-$1FFF` on mapper 14. MF-6b step 6 named this
    /// method before it existed — "`BankWindows` has no way to express a 1 KiB window table whose
    /// A18 changes at `$1000` and again at `$1800` other than by computing it per window".
    ///
    /// `OR` and not `+`, because a line is what both pages describe, and it is what makes the call
    /// **idempotent**: the bank a window already carries may already have the bit, so a board may
    /// re-apply it after every register write without remembering what it last laid down. The same
    /// reading, and the same reason, as [`Self::or_chr_banks`] above.
    ///
    /// Additive: behind `board-huang1`, its only caller.
    #[cfg(feature = "board-huang1")]
    pub fn or_chr_banks_each(&mut self, bits: [i32; Self::CHR_WINDOWS]) {
        for (slot, line) in bits.into_iter().enumerate() {
            let bank = i32::try_from(self.chr_offset[slot] / Self::CHR_WINDOW_SIZE).unwrap_or(0);
            self.map_chr_1k(slot, bank | line);
        }
    }

    /// Rewrites every 8 KiB PRG window's bank number as `(bank & mask) | bits`, each re-wrapped
    /// as [`Self::map_prg_8k`] wraps it.
    ///
    /// The counterpart of [`Self::offset_chr_banks`] for a board whose **outer PRG bank is an
    /// address line** rather than an addend: mapper 115's `$6000` bit 6 is "PRG A18" and mapper
    /// 121's `$5180` bit 7 selects an outer 256 KiB bank, and both pages describe the MMC3's own
    /// bits as sitting *below* that line. `mask` is what the MMC3 still drives, so the call is
    /// **idempotent** whenever `bits & mask == 0`, which is the only way these boards use it.
    ///
    /// **MF-UNL-11's three boards are the second, third and fourth callers**, and they are the
    /// reason [`Self::remap_chr_banks`] below exists: the UNL-8237's `$5000` bit 6 "replace[s] PRG
    /// A17 **and CHR A17** from the MMC3", and the Kǎshèng A9461 masks "PRG-ROM and CHR-ROM banks
    /// … to 128 KiB" under its own two outer bits, so both numbers need the same `(bank & mask) |
    /// bits` on *both* bank tables.
    #[cfg(any(feature = "board-mmc3-scramble", feature = "board-215"))]
    pub fn remap_prg_banks(&mut self, mask: i32, bits: i32) {
        for slot in 0..Self::PRG_WINDOWS {
            if self.prg_is_ram[slot] {
                continue;
            }

            let bank = i32::try_from(self.prg_offset[slot] / Self::PRG_WINDOW_SIZE).unwrap_or(0);
            self.map_prg_8k(slot, (bank & mask) | bits);
        }
    }

    /// ORs `bits` into the bank number of the 1 KiB CHR windows from `from_slot` on, each
    /// re-wrapped as [`Self::map_chr_1k`] wraps it.
    ///
    /// `from_slot` is what makes this the **CHR A18 driven by PPU A12** of the Kǎshèng A9711 and
    /// A98402 (mappers 121 and 187): nesdev gives both boards "CHR A18 (the second half of
    /// CHR-ROM) is directly selected by PPU A12, regardless of what the MMC3 is set to do. This
    /// means that the 'left' pattern table always uses the first and the 'right' pattern table
    /// always uses the second 256 KiB half of CHR-ROM" — so the four windows of PPU $1000-$1FFF
    /// carry the line and the four of $0000-$0FFF do not. `from_slot` = 0 is the same boards'
    /// outer CHR bank, which is a line over all eight.
    ///
    /// `OR` and not `+`, because a line is what both pages describe, and it is what makes the
    /// call idempotent: the bank a window already carries may already have the bit.
    ///
    /// Additive: behind `board-mmc3-scramble`, its only caller.
    #[cfg(feature = "board-mmc3-scramble")]
    pub fn or_chr_banks(&mut self, from_slot: usize, bits: i32) {
        for slot in from_slot..Self::CHR_WINDOWS {
            let bank = i32::try_from(self.chr_offset[slot] / Self::CHR_WINDOW_SIZE).unwrap_or(0);
            self.map_chr_1k(slot, bank | bits);
        }
    }

    /// Rewrites every 1 KiB CHR window's bank number as `(bank & mask) | bits`, each re-wrapped
    /// as [`Self::map_chr_1k`] wraps it — [`Self::remap_prg_banks`]'s counterpart on the CHR
    /// table (MF-UNL-11 step 1).
    ///
    /// [`Self::or_chr_banks`] above cannot serve these boards, because their outer line has to
    /// **replace** a line the MMC3 drives rather than sit above it: nesdev ("INES Mapper 215")
    /// gives `$5000` bit 6 as "1: Replace PRG A17 and CHR A17 with `$5001` bits 4 and 5", which
    /// means clearing bit 7 of the 1 KiB bank number before setting it, and nesdev ("INES Mapper
    /// 219") masks the chip's own banks "to 128 KiB" under two outer bits that are above them.
    ///
    /// `mask` is what the chip still drives, so the call is **idempotent** whenever
    /// `bits & mask == 0` — the only way these boards use it, and the same contract
    /// `remap_prg_banks` states.
    ///
    /// Additive: behind the three features of MF-UNL-11, its only callers.
    #[cfg(feature = "board-215")]
    pub fn remap_chr_banks(&mut self, mask: i32, bits: i32) {
        for slot in 0..Self::CHR_WINDOWS {
            let bank = i32::try_from(self.chr_offset[slot] / Self::CHR_WINDOW_SIZE).unwrap_or(0);
            self.map_chr_1k(slot, (bank & mask) | bits);
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

    /// The windows [`Self::read_prg`] reads from a PRG-ROM of `rom_len` bytes without
    /// wrapping (see `Mapper::rom_windows`).
    #[must_use]
    pub fn rom_windows(&self, rom_len: usize) -> [usize; Self::PRG_WINDOWS] {
        std::array::from_fn(|slot| {
            let start = self.prg_offset[slot];
            if self.prg_is_ram[slot] || start + Self::PRG_WINDOW_SIZE > rom_len {
                crate::cartridges::mappers::NO_ROM_WINDOW
            } else {
                start
            }
        })
    }

    /// Offset into PRG-ROM that `address` ($8000-$FFFF) shows through the window table, or
    /// `None` below $8000 and when the window shows PRG-RAM.
    ///
    /// For a board that writes its *own* ROM: the UNROM 512 carries flash rather than mask ROM
    /// and a game can erase and program it (MR-30-4).
    #[must_use]
    pub fn prg_rom_offset(&self, address: u16) -> Option<usize> {
        if address < 0x8000 {
            return None;
        }

        let slot = usize::from(address - 0x8000) / Self::PRG_WINDOW_SIZE;
        if self.prg_is_ram[slot] {
            return None;
        }

        Some(self.prg_offset[slot] + (usize::from(address) & (Self::PRG_WINDOW_SIZE - 1)))
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
