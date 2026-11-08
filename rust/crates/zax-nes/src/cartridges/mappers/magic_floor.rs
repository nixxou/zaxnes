use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, Cartridge, CartridgeMemory, ChrFetch, ChrView};
use crate::NametableMirroring;

/// iNES mapper **218**, the single-chip homebrew board of *Magic Floor* and *Starfight*
/// (REQ-CORE-102, MF-UNL-7 step 5).
///
/// > There is no CHR-ROM or CHR-RAM on the cartridge. Instead, the console's internal 2KiB
/// > nametable RAM is mapped as CHR-RAM.
///
/// "The cartridge board only contains a single PRG-ROM chip (up to 32KiB)" and a CIC, and that is
/// the whole board: **no register, no latch, no bank line and no CHR memory**. It is the only board
/// in this build with nothing a game can write.
///
/// # The four configurations, and the one formula that is all four
///
/// > [It] can be used in four configurations by wiring CIRAM A10 to one of the PPU A10..A13
/// > address lines.
///
/// | iNES byte 6 | line | the page's name for it |
/// | --- | --- | --- |
/// | `$A1` | PPU A10 | Two-screen, Horizontal arrangement ("Vertical mirroring") |
/// | `$A0` | PPU A11 | Two-screen, Vertical arrangement ("Horizontal mirroring") |
/// | `$A8` | PPU A12 | Single-screen arrangement BLK0 |
/// | `$A9` | PPU A13 | Single-screen arrangement BLK1 |
///
/// so bit 3 of byte 6 — the four-screen flag — picks the pair and bit 0 picks within it, and the
/// two two-screen rows are the ordinary iNES reading of bit 0 after all (`0` is CIRAM A10 from PPU
/// A11, `1` from PPU A10). Byte 6 is read whole here, which is the shape MR-30-1 gave the UNROM
/// 512 and the reason `InesHeader::byte6` is exposed at all.
///
/// Every access, pattern table or nametable, then lands at
///
/// ```text
/// CIRAM offset = ((PPU A<line>) << 10) | (PPU A9..A0)
/// ```
///
/// — one expression, because that *is* the wiring: A10 of the RAM comes from one PPU line and its
/// nine low address lines come from the PPU's. Read through it, the page's own descriptions of what
/// each configuration gives fall out:
///
/// * **A10 / A11** — the nametables alternate across that line (vertical / horizontal mirroring)
///   and the pattern tables show "2KiB (128 tiles) CHR-RAM, also mapped to nametable data", four
///   times over the 8 KiB;
/// * **A12** — PPU A12 is 0 throughout $2000-$2FFF, so all four nametables are block 0, and the
///   two pattern tables are blocks 0 and 1: "1KiB (64 tiles)" each, the one the background shows
///   "swappable via PPUCTRL" because that register's pattern-table select *is* A12;
/// * **A13** — PPU A13 is 1 throughout $2000-$3FFF and 0 throughout $0000-$1FFF, so the
///   nametables are block 1 and both pattern tables are block 0.
///
/// # Why the cartridge serves the nametables, and why that is not a liberty
///
/// CIRAM is in the console, and a board that maps it into the pattern tables is asking for one
/// 2 KiB memory to answer both windows — which this architecture cannot do with the PPU's own
/// VRAM, because [`Mapper::ppu_peek`] has no way to reach it. So the 2 KiB lives in the
/// cartridge's CHR-RAM (`registry::board_memory_sizes` gives the board exactly that much) and
/// [`Mapper::nametable_page`] answers `SERVED_BY_CARTRIDGE`, which is the shape MF-6c step 9 gave
/// the GTROM and MR-30-6 the UNROM 512's four-screen mode.
///
/// **It is the same chip either way.** Which side of the connector an emulator keeps CIRAM on is
/// invisible to a game; what is visible is that the pattern tables and the nametables are one
/// memory, and that is exactly what this arrangement gives. A board that served patterns from the
/// cartridge and nametables from console VRAM would be two memories and *would* be wrong —
/// *Magic Floor* writes its tiles through $2007 into the nametable window and reads them back as
/// patterns, which is what the board exists for.
///
/// This is the **fifth** distinct body of the `Mapper::read_nametable` dispatch Q-123 once
/// measured a cap of two on, and the second inside [`super::DecodeBoard`] — so `Board`'s own jump
/// table does not grow by an entry, which is what MF-6c step 7 established for a board joining
/// that enum.
///
/// # PRG, and the one thing that is not emulated
///
/// "A single PRG-ROM chip (up to 32KiB)", unbanked: $8000-$FFFF is bank 0 of 32 KiB, which a
/// 16 KiB image shows twice exactly as [`super::Nrom`] does. The page gives the board no PRG-RAM
/// and no register, so `board_memory_sizes` clears $6000-$7FFF and
/// [`Mapper::write_register`] is `Mapper`'s own empty default: **nothing a game writes below
/// $8000 reaches this board**, and nothing it writes above $8000 does either.
///
/// The CIC is not emulated, which is true of every board in this build and is not a property of
/// this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicFloor {
    banks: BankWindows,
    /// Which PPU address line drives CIRAM A10: 10, 11, 12 or 13. From the header's byte 6 at
    /// load, so no savestate byte carries it — the board has no register at all (REQ-CORE-090).
    ciram_a10_from: u32,
}

impl MagicFloor {
    /// The console's nametable RAM, which on this board is also the pattern tables.
    pub(crate) const CHR_RAM_SIZE: usize = 2 * 1024;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let byte6 = memory.rom().header().byte6;
        let mut board = Self {
            banks: windows_for(memory),
            // Bit 3 of byte 6 picks the pair and bit 0 picks within it; see the struct's table.
            ciram_a10_from: match (byte6 & 0x08 != 0, byte6 & 0x01 != 0) {
                (false, true) => 10,
                (false, false) => 11,
                (true, false) => 12,
                (true, true) => 13,
            },
        };
        board.banks.map_prg_32k(0);
        board
    }

    /// `((PPU A<line>) << 10) | (PPU A9..A0)`, the whole of this board's address decode.
    const fn ciram_offset(&self, address: u16) -> usize {
        let block = ((address >> self.ciram_a10_from) & 0x0001) as usize;
        (block << 10) | (address as usize & 0x03FF)
    }

    /// The nametable arrangement the configuration produces, for anything that asks the cartridge
    /// what its layout is.
    ///
    /// Nothing in the rendering path consults it — [`Mapper::nametable_page`] answers
    /// `SERVED_BY_CARTRIDGE` for all four tables — but it is the honest answer and not a
    /// placeholder: with CIRAM A10 on PPU A12 the whole of $2000-$2FFF is block 0, which is
    /// one screen, and with it on PPU A13 the whole of $2000-$3FFF is block 1.
    const fn arrangement(&self) -> NametableMirroring {
        match self.ciram_a10_from {
            10 => NametableMirroring::Vertical,
            11 => NametableMirroring::Horizontal,
            12 => NametableMirroring::SingleScreenA,
            _ => NametableMirroring::SingleScreenB,
        }
    }

    /// The byte at `offset` of the 2 KiB, or 0 when a dump gave the board none.
    fn read_ciram(memory: &CartridgeMemory, offset: usize) -> u8 {
        let ram = memory.chr_ram();
        if ram.is_empty() {
            0
        } else {
            let len = ram.len();
            ram[super::super::wrap_offset(offset, len)]
        }
    }

    /// The counterpart, which **ignores whatever CHR-ROM a dump declared**: this memory is the
    /// console's nametable RAM and it is RAM, where `CartridgeMemory::write_chr_direct` would
    /// refuse the write on an image that carries a CHR-ROM chip the board has not got.
    fn write_ciram(memory: &mut CartridgeMemory, offset: usize, value: u8) {
        let ram = memory.chr_ram_mut();
        if !ram.is_empty() {
            let len = ram.len();
            ram[super::super::wrap_offset(offset, len)] = value;
        }
    }
}

impl Mapper for MagicFloor {
    banked!();

    fn mapper_number(&self) -> i32 {
        218
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        // No register to clear. The arrangement is the board's wiring and is set here so that a
        // soft reset leaves the cartridge saying the same thing it said at power-on (AMD-061 §5).
        memory.set_mirroring(self.arrangement());
        self.banks.map_prg_32k(0);
    }

    /// `SERVED_BY_CARTRIDGE`, for all four tables and always — see the struct's own note on why
    /// the 2 KiB lives on this side of the connector.
    ///
    /// A constant and not a field: the four configurations differ in *where* a table lands inside
    /// the 2 KiB, which [`MagicFloor::ciram_offset`] answers, and not in whether the cartridge
    /// serves it.
    fn nametable_page(&self, _table: i32) -> i32 {
        Cartridge::SERVED_BY_CARTRIDGE
    }

    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        Self::read_ciram(memory, self.ciram_offset(address))
    }

    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        Self::write_ciram(memory, self.ciram_offset(address), value);
    }

    /// A pattern-table fetch returns CIRAM, which is the whole point of the board.
    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        Self::read_ciram(memory, self.ciram_offset(address))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        Self::write_ciram(memory, self.ciram_offset(address), value);
    }

    /// **`None`, and not the default body.** `Mapper::chr_view`'s own default would hand the PPU
    /// this board's [`BankWindows`] over its 2 KiB, and those windows describe eight 1 KiB slots
    /// of a chip that has two — the board's addressing is [`MagicFloor::ciram_offset`] and not a
    /// window table at all. [`super::DecodeBoard::chr_view`] answers `None` for the whole variant
    /// already; this is the same answer for [`Mapper::chr_peek_view`], which that enum
    /// **forwards**.
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }
}
