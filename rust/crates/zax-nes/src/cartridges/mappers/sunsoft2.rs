use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers 89 and 93, the Sunsoft-2 chip on its two boards (REQ-CORE-102).
///
/// One 16 KiB PRG window at $8000 with the last bank fixed at $C000, one register over the whole
/// of $8000-$FFFF, **bus conflicts** on both (`registry::has_bus_conflicts`), and no IRQ and no
/// PRG-RAM on either page. What the board the chip sits on decides is the rest:
///
/// * **89**, the Sunsoft-3 board of *Tenka no Goikenban: Mito Koumon* — nesdev
///   ("INES Mapper 089") gives `$8000-FFFF: [CPPP MCCC]`: `PPP` (bits 6-4) the 16 KiB PRG bank,
///   `CCC` (bits 2-0) with `C` (bit 7) above them the **8 KiB CHR-ROM bank**, and `M` (bit 3)
///   one-screen mirroring, `%0 = 1ScA`, `%1 = 1ScB`.
/// * **93**, the Sunsoft-3R board of *Shanghai* and *Fantasy Zone* — nesdev
///   ("INES Mapper 093") gives `$8000-FFFF: [.PPP ...E]`: the same `PPP` in bits 6-4, no CHR bank
///   and no mirroring bit at all, and `E` (bit 0) a **CHR-RAM enable** whose disabled state the
///   page spells out: "reads are open bus" and "writes are ignored".
///
/// So `chr_ram_enable` is the board-revision parameter, and it is a runtime `bool` for the reason
/// `Namcot108`'s note gives: it is read by the register decode and by the CHR view, never on a
/// per-cycle path.
///
/// **Open bus is modelled as zero**, which is what every other board in this build answers for a
/// fetch with nothing behind it (`BankWindows::read_chr` on an empty memory, `Mapper::ppu_peek`'s
/// own default). The PPU's own open-bus value is not modelled anywhere in this core, so a board
/// is not the place to introduce it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sunsoft2 {
    banks: BankWindows,
    register: u8,
    /// 89 or 93, for [`Mapper::mapper_number`].
    mapper_number: i32,
    /// Mapper 93: bit 0 is a CHR-RAM enable, and there is no CHR bank and no mirroring bit.
    chr_ram_enable: bool,
}

impl Sunsoft2 {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        Self {
            banks: windows_for(memory),
            register: 0,
            mapper_number,
            chr_ram_enable: mapper_number == 93,
        }
    }

    /// Mapper 93's `E`: with it clear the CHR memory is deselected entirely.
    const fn chr_enabled(&self) -> bool {
        !self.chr_ram_enable || self.register & 0x01 != 0
    }

    fn update_banks(&mut self) {
        self.banks
            .map_prg_16k(0, i32::from((self.register >> 4) & 0x07));
        self.banks.map_prg_16k(2, -1);
        if !self.chr_ram_enable {
            // `C` is bit 7 and sits above `CCC`, making a four-bit 8 KiB bank number.
            self.banks.map_chr_8k(i32::from(
                (self.register & 0x07) | ((self.register & 0x80) >> 4),
            ));
        }
    }
}

impl Mapper for Sunsoft2 {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.mapper_number
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
        if !self.chr_ram_enable {
            // `M` clear is one-screen A; mapper 93 leaves the header's layout alone.
            memory.set_mirroring(NametableMirroring::SingleScreenA);
        }

        self.update_banks();
    }

    /// `None` while `E` is clear, so a peek of mapper 93 reads nothing rather than the CHR-RAM.
    ///
    /// `SmallBoard::chr_view` answers `None` for the whole variant — its own note says why — so
    /// this is only what the tile emitter and the host accessors see, through
    /// [`Mapper::chr_peek_view`], and the PPU's fetches come through [`Mapper::ppu_peek`] below.
    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        if !self.chr_enabled() {
            // "Reads are open bus": a view of nothing, which reads as zero.
            return Some(ChrView::linear(&[]));
        }

        let chr_rom = memory.rom().chr_rom();
        Some(ChrView::new(
            if chr_rom.is_empty() {
                memory.chr_ram()
            } else {
                chr_rom
            },
            &self.banks,
        ))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        if !self.chr_enabled() {
            return 0;
        }

        let chr_rom = memory.rom().chr_rom();
        self.banks.read_chr(
            address,
            if chr_rom.is_empty() {
                memory.chr_ram()
            } else {
                chr_rom
            },
        )
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        // "Writes are ignored" while `E` is clear; a CHR-ROM board ignores them anyway.
        if self.chr_enabled() && memory.rom().chr_rom().is_empty() {
            self.banks.write_chr(address, value, memory.chr_ram_mut());
        }
    }

    /// `#[inline(never)]`, as the other boards of this lot: see `Namcot108::write_register`.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        self.register = value;
        if !self.chr_ram_enable {
            memory.set_mirroring(if value & 0x08 == 0 {
                NametableMirroring::SingleScreenA
            } else {
                NametableMirroring::SingleScreenB
            });
        }

        self.update_banks();
    }
}
