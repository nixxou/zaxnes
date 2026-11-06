use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper **77**, the Irem LROG017 of *Napoleon Senki* (MF-6c step 6).
///
/// nesdev ("INES Mapper 077"), one register and a memory map:
///
/// ```text
/// $8000-$FFFF: [CCCC PPPP]
///   C = 2 KiB CHR bank at $0000
///   P = 32 KiB PRG bank at $8000
/// Bus conflicts: yes
///
/// PPU $0000-$07FF  swappable CHR-ROM
/// PPU $0800-$2FFF  RAM -- "just provide 10KiB from $0800-$2FFF"
/// PPU $3000-$3FFF  open bus
/// ```
///
/// Discrete logic: "Two 74161s latch the 8 bits", "A 74138 divides the PPU's 16KiB of address
/// space into eight 2KiB regions", and "A 74'21 (two 4-input AND gates) combines the other four
/// outputs to generate a chip enable for the 8KiB RAM on the PPU".
///
/// # The split CHR, which is why this board is here and not in [`super::SmallBoard`]
///
/// $0000-$07FF is the CHR-**ROM** chip through the bank register and $0800-$1FFF is the
/// CHR-**RAM** chip, so a [`ChrView`] cannot describe it: a view is one slice plus eight window
/// offsets and has no way to say "this window is the other chip". That is the TQROM's situation
/// exactly (`Mmc3Board::tqrom_read`, MF-6a3 step 1), and the answer is the same — [`ChrView`] is
/// `None` on both dispatches and [`Mapper::ppu_peek`] and [`Mapper::ppu_write`] serve the two
/// chips, each `#[inline(never)]` so that the arms they put in `Board::ppu_peek` and
/// `NesBus::write_internal` are a `callq` and not a body.
///
/// **`Ppu2C02::read_pattern_planes` is not touched by any of that**, which §5 MF-6c step 5 of
/// `docs/review/MF-6-inventory.md` made this board's own bar: that function reads the PPU's fast
/// path, and a board whose `chr_view` is `None` leaves it alone by construction — it is the
/// *span* fast path such a board gives up, one fetch at a time through `ppu_peek`, which is the
/// price the nine `SmallBoard` boards already pay (`SmallBoard::chr_view`'s own note).
///
/// # The 10 KiB of RAM, and where the four nametables come from
///
/// The page's implementation note settles the layout for an emulator: "When making an emulator,
/// you do not need to care about the specific order of the CHR-RAM banks: just provide 10KiB from
/// $0800-$2FFF." This build provides exactly that, in two pieces:
///
/// * **6 KiB of CHR-RAM** at $0800-$1FFF, from [`CartridgeMemory::chr_ram`], sized by
///   `registry::board_memory_sizes` because no header field can state it (the dump declares
///   CHR-*ROM*, so REQ-CORE-012's CHR-RAM default does not apply) — the TQROM's arm's reasoning,
///   for the same reason. Mesen 2 agrees, `GetChrRamSize` `0x1800` (`IremLrog017.h:10`).
/// * **4 KiB of nametable** at $2000-$2FFF, which is [`NametableMirroring::FourScreen`]: this
///   core's console VRAM is 4 KiB (`ppu::VRAM_SIZE`), so the four tables are four distinct
///   kilobytes and nothing has to be served by the cartridge at all.
///
/// On the board those 10 KiB are 8 KiB of cartridge RAM (the chip the 74'21 enables) plus the
/// console's 2 KiB of CIRAM, and which 2 KiB of $2000-$2FFF is which is invisible to a program:
/// both are RAM, and all five 2 KiB regions are distinct either way. That is what the page's own
/// note licenses, and it is why this board needs no [`Mapper::read_nametable`] body — Q-123's
/// dispatch is untouched (§4.5), unlike the UNROM 512's four-screen mode, which serves its
/// nametables out of CHR-RAM because that board has no CIRAM to spare.
///
/// The mirroring is **forced** at power-on and on reset rather than read from byte 6: the page's
/// memory map is the board's wiring and an iNES 1.0 dump of a singleton mapper cannot be relied on
/// to carry the four-screen bit. Mesen 2 does the same (`SetMirroringType(MirroringType::
/// FourScreens)` in `InitMapper`).
///
/// # What the page does not give, and is therefore left alone
///
/// **PRG-RAM.** The page has no PRG-RAM line and the hardware note lists one RAM chip, on the PPU
/// bus; the register is at $8000-$FFFF, so nothing of the board's decodes $6000-$7FFF. The
/// header's size therefore stands, which is Mesen 2's reading too (`IremLrog017.h` overrides no
/// work-RAM size), and it is the choice MF-6c step 1 argued for $800D bit 5: of the two possible
/// errors, taking RAM away from a game that never asks for it is the worse one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IremLrog017 {
    banks: BankWindows,
    /// The two 74161s: `[CCCC PPPP]`.
    register: u8,
}

impl IremLrog017 {
    /// The CHR-RAM chip's 6 KiB at $0800-$1FFF. See the struct's note on the 10 KiB.
    pub(crate) const CHR_RAM_SIZE: usize = 0x1800;

    /// First PPU address the CHR-RAM chip answers: "$0000-$07FF swappable CHR-ROM" above it, the
    /// 74138's first 2 KiB region.
    const CHR_RAM_BASE: usize = 0x0800;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_32k(i32::from(self.register & 0x0F));
        self.banks.map_chr_2k(0, i32::from(self.register >> 4));
    }

    /// Offset in the CHR-RAM chip of PPU `address`, which is $0800-$1FFF here.
    ///
    /// Linear, which the page's "you do not need to care about the specific order of the CHR-RAM
    /// banks" licenses and which is the order Mesen 2 selects (`IremLrog017.h:20-22`, RAM pages
    /// 0, 1 and 2 at $0800, $1000 and $1800).
    const fn ram_offset(address: u16) -> usize {
        ((address & 0x1FFF) as usize) - Self::CHR_RAM_BASE
    }
}

impl Mapper for IremLrog017 {
    banked!();

    fn mapper_number(&self) -> i32 {
        77
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.register = reader.read_u8()?;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.register = 0;
        // The board's wiring, not the header's byte 6; see the struct's own note.
        memory.set_mirroring(NametableMirroring::FourScreen);
        self.update_banks();
    }

    /// `#[inline(never)]`: MF-6a step 1 measured what an inlined register decode does to
    /// `NesBus::write_internal`'s layout, and every board of that shape has carried the attribute
    /// since.
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, _address: u16, value: u8) {
        self.register = value;
        self.update_banks();
    }

    /// `None`, as [`super::SmallBoard::chr_view`] answers it and for the measured reason that note
    /// gives: this dispatch's arms are inlined into `Ppu2C02::run_visible_span` and so into
    /// `run_dots`, and what that loop cannot afford is one more distinct body. `None` is the body
    /// the MMC2, the MMC3, the MMC5, the RAMBO-1 and the nine small boards already answer.
    ///
    /// It is also the only answer available: the two chips cannot be one [`ChrView`].
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    /// `None` for the same reason the TQROM answers `None` (`Mmc3Board::chr_peek_view`): a view is
    /// one slice plus eight offsets, and $0000-$07FF and $0800-$1FFF are two chips. The tile
    /// emitter and the host accessors fall back on [`Mapper::ppu_peek`] below.
    fn chr_peek_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    /// The two chips: the bank register's 2 KiB of CHR-ROM at $0000-$07FF, the RAM chip above it.
    ///
    /// `#[inline(never)]`, for the reason `Mmc3Board::tqrom_read` carries: `Board::ppu_peek`'s
    /// arms are inlined into `Ppu2C02::background_fetch`, 7 % to 11 % of every ROM of lot PR-1's
    /// set, and MF-6a2 step 3 measured one body too many there at 271 → 583 instructions and
    /// 3.4 M to 5.4 M over 300 frames on three games on boards it does not touch.
    #[inline(never)]
    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        if (usize::from(address) & 0x1FFF) < Self::CHR_RAM_BASE {
            return self.banks.read_chr(address, memory.rom().chr_rom());
        }

        let ram = memory.chr_ram();
        if ram.is_empty() {
            return 0;
        }

        ram[super::super::wrap_offset(Self::ram_offset(address), ram.len())]
    }

    /// The RAM chip's writes. [`Mapper::ppu_write`]'s default drops every write on a board that
    /// has CHR-ROM and this board has both chips, so without this the 6 KiB would be unreachable
    /// — the TQROM's case in the same words. `#[inline(never)]` because this dispatch's arms are
    /// inlined into `NesBus::write_internal`, which a `$2007` write arrives through.
    #[inline(never)]
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if (usize::from(address) & 0x1FFF) < Self::CHR_RAM_BASE {
            // The ROM chip drives this window: the write goes nowhere.
            return;
        }

        let offset = Self::ram_offset(address);
        let ram = memory.chr_ram_mut();
        if !ram.is_empty() {
            let len = ram.len();
            ram[super::super::wrap_offset(offset, len)] = value;
        }
    }
}
