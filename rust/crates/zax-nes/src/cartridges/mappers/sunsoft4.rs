use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::NametableMirroring;

use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper **68**, the Sunsoft-4 (MF-6c step 4).
///
/// nesdev ("INES Mapper 068"). Six registers on the high nibble of $8000-$FFFF:
///
/// | register | what it does |
/// | --- | --- |
/// | `$8000`-`$B000` | four **2 KiB** CHR banks at $0000, $0800, $1000 and $1800 |
/// | `$C000` | the **lower** nametable's 1 KiB CHR-ROM bank; "7 bits used; D7 ignored" |
/// | `$D000` | the **upper** nametable's, the same way |
/// | `$E000` | `...C ..MM` — bits 1-0 the mirroring, **bit 4 the CIRAM / CHR-ROM select for nametables** |
/// | `$F000` | `...E BBBB` — bits 3-0 the 16 KiB PRG bank at $8000, **bit 4 the PRG-RAM enable** |
///
/// $C000-$FFFF is the last 16 KiB bank, wired.
///
/// # The two halves, and why they are two Cargo features
///
/// §4.5 of `docs/review/MF-6-inventory.md` calls this board "the hook the inventory should make a
/// sub-lot most afraid of", and it is right about which half:
///
/// * **the CIRAM half is free.** The four mirroring modes go through
///   [`CartridgeMemory::set_mirroring`] like every other board's mirroring register — not through
///   [`Mapper::nametable_page`] — and the PRG, the PRG-RAM enable and the four CHR windows are a
///   register decode and nothing more. That is `board-68`.
/// * **the CHR-ROM nametable mode is Q-123's ceiling.** `$E000` bit 4 puts CHR-ROM banks over
///   $2000-$2FFF, which is a cartridge-served nametable in the exact sense Q-123 is about, and it
///   is the *same* feature the VRC6 has and this build deliberately does not implement: MF-5a
///   built **five** shapes for a third `Mapper::read_nametable` arm body and every one of them
///   cost every other board, the measured conclusion being "the trigger is the third distinct
///   body, not its size". That is `board-68-chr-nametables`, which implies `board-68`.
///
/// **The owner's rule of 2026-11-06 is why the second one exists at all.** §5 MF-6c step 4 of the
/// inventory offered the lot a choice between finding a shape MF-5a did not try and decoding the
/// bit and ignoring it, "the second [being] the expected outcome". The owner's rule 1 replaces
/// that: "No board, and no feature of a board, is ever dropped for performance … A costly
/// *mechanism* of a board (Sunsoft-4's CHR-ROM nametables through `read_nametable` …) is
/// implemented too, behind its own feature if it costs more than the board itself." So the mode is
/// **built**, and the measurement decides only whether its feature is in `default`.
/// `docs/progress/MF-6c.md` step 4 has the figure and
/// `docs/review/BOARD-FEATURES.md` the row.
///
/// # What is left out, and recorded: *Nantettatte!! Baseball*
///
/// That one cartridge has a second PRG-ROM on a subboard, selected by a bit of `$F000` and gated
/// by a licensing timer the board starts when $6000-$7FFF is written. **A second ROM image has no
/// path through `Cartridge::create` and no spec text asks for one** — §5 MF-6c step 4's own words
/// — so the register is a 4-bit bank here, which is what the page's `...E BBBB` diagram gives it,
/// and the external ROM is a gap of the same kind as MR-30-4's UNROM 512 flash and MF-6c step 3's
/// Datach barcode scanner. Mesen 2 reads bit 3 of that register as an internal/external chip
/// select and holds a 105 000-cycle timer for it (`Sunsoft4.h:120-139`); on a single-chip dump
/// its reading and this one agree, because bank 8 and above wrap into the same eight banks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sunsoft4 {
    banks: BankWindows,
    /// The four 2 KiB CHR registers of $8000-$B000.
    chr: [u8; 4],
    /// $C000 and $D000: the two nametable banks. Kept with bit 7 **set**, which is how the board
    /// wires them — "7 bits used; D7 ignored", and Mesen 2's `value | 0x80` (`Sunsoft4.h:112`).
    nametable_bank: [u8; 2],
    /// $E000 bits 1-0, kept so that the CHR-ROM nametable mapping can be recomputed when bit 4
    /// moves without re-reading `CartridgeMemory`.
    mirroring: u8,
    /// $E000 bit 4: the nametables come from CHR-ROM instead of CIRAM.
    chr_nametables: bool,
    /// $F000 bits 3-0, the 16 KiB bank at $8000.
    prg: u8,
    /// $F000 bit 4.
    prg_ram_on: bool,
    /// What [`Mapper::nametable_page`] answers for the four logical tables: `SERVED_BY_CARTRIDGE`
    /// in CHR-ROM mode and `FOLLOW_MIRRORING` otherwise.
    ///
    /// A field and not a `match`, for the reason `Mmc3Board::nametable_pages` gives: that dispatch
    /// is consulted once per nametable fetch. Derived — `load_registers` recomputes it.
    nametable_pages: [i32; 4],
}

impl Sunsoft4 {
    /// 1 KiB, the unit the two nametable registers count in.
    #[cfg(feature = "board-68-chr-nametables")]
    const NAMETABLE_BANK_SIZE: usize = 0x0400;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            chr: [0; 4],
            nametable_bank: [0x80; 2],
            mirroring: 0,
            chr_nametables: false,
            prg: 0,
            prg_ram_on: false,
            nametable_pages: [super::super::Cartridge::FOLLOW_MIRRORING; 4],
        }
    }

    /// Whether the CHR-ROM nametable mode is both decoded and served.
    ///
    /// `cfg!` and not `#[cfg]` on the field: the byte stays so that the struct and its save-state
    /// block are the same in both configurations — the bit is **decoded and kept either way**,
    /// which is what makes `board-68-chr-nametables` a switch on the *serving* and not on the
    /// register decode.
    const fn serves_chr_nametables(&self) -> bool {
        cfg!(feature = "board-68-chr-nametables") && self.chr_nametables
    }

    fn update_banks(&mut self) {
        for (window, bank) in self.chr.iter().enumerate() {
            self.banks.map_chr_2k(window * 2, i32::from(*bank));
        }
        // "Bank 0's initial state is undefined, but some roms expect it to be the first page"
        // (Mesen 2, `Sunsoft4.h:53`), which is what a register of zero names anyway.
        self.banks.map_prg_16k(0, i32::from(self.prg));
        self.banks.map_prg_16k(2, -1);
        self.refresh_nametable_pages();
    }

    fn refresh_nametable_pages(&mut self) {
        let page = if self.serves_chr_nametables() {
            super::super::Cartridge::SERVED_BY_CARTRIDGE
        } else {
            super::super::Cartridge::FOLLOW_MIRRORING
        };
        self.nametable_pages = [page; 4];
    }

    /// Which of the two nametable registers serves logical table `table`, under the mirroring
    /// mode $E000 bits 1-0 select.
    ///
    /// The mapping is the mirroring itself: with vertical mirroring the two tables alternate, with
    /// horizontal they pair up, and a single-screen mode puts one register everywhere. Mesen 2
    /// computes it the same way (`Sunsoft4.h:22-29`).
    #[cfg(feature = "board-68-chr-nametables")]
    const fn nametable_register(&self, table: usize) -> usize {
        match self.mirroring {
            0 => table & 1,
            1 => (table >> 1) & 1,
            2 => 0,
            _ => 1,
        }
    }
}

impl Mapper for Sunsoft4 {
    banked!();

    fn mapper_number(&self) -> i32 {
        68
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_raw(&self.nametable_bank);
        writer.write_u8(self.mirroring);
        writer.write_bool(self.chr_nametables);
        writer.write_u8(self.prg);
        writer.write_bool(self.prg_ram_on);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        reader.read_into(&mut self.nametable_bank)?;
        self.mirroring = reader.read_u8()?;
        self.chr_nametables = reader.read_bool()?;
        self.prg = reader.read_u8()?;
        self.prg_ram_on = reader.read_bool()?;
        // Derived, and therefore not in the block: MF-5a step 3's `level` lesson.
        self.refresh_nametable_pages();
        Ok(())
    }

    /// `$F000` bit 4. **Power-on is disabled**, which is Mesen 2's state too
    /// (`Sunsoft4.h:47`, `_prgRamEnabled = false`): the page gives the bit an enable meaning, so
    /// the RAM is off until a game asks for it, and *Nantettatte!! Baseball*'s licensing timer —
    /// the one thing that would read the window before a write — is out of scope.
    fn prg_ram_enabled(&self) -> bool {
        self.prg_ram_on
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.chr = [0; 4];
        self.nametable_bank = [0x80; 2];
        self.mirroring = 0;
        self.chr_nametables = false;
        self.prg = 0;
        self.prg_ram_on = false;
        memory.set_mirroring(NametableMirroring::Vertical);
        self.update_banks();
    }

    fn nametable_page(&self, table: i32) -> i32 {
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 3]
    }

    /// **The third distinct body of `Mapper::read_nametable`, and the reason
    /// `board-68-chr-nametables` exists.**
    ///
    /// `$E000` bit 4 set maps the 1 KiB CHR-ROM banks of `$C000` and `$D000` over $2000-$2FFF,
    /// which register serving which logical table being decided by the mirroring mode — see
    /// [`Sunsoft4::nametable_register`]. Q-123 priced a third arm body in this dispatch at
    /// `Ppu2C02::read_nametable_inner` **132 → 109 for every game in the library**, measured on a
    /// tree seven lots older; `docs/progress/MF-6c.md` step 4 re-measures it rather than quoting
    /// that figure, because the owner's rule says the mode ships whatever it costs and only its
    /// feature's placement is in question.
    #[cfg(feature = "board-68-chr-nametables")]
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        let table = usize::from(address >> 10) & 3;
        let bank = usize::from(self.nametable_bank[self.nametable_register(table)]);
        let offset = bank * Self::NAMETABLE_BANK_SIZE + (usize::from(address) & 0x03FF);
        memory.read_chr_direct(offset)
    }

    /// `#[inline(never)]`: MF-6a step 1 measured what an inlined register decode does to
    /// `NesBus::write_internal`'s layout, and every `SmallBoard` decode has carried the attribute
    /// since.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xF000 {
            0x8000 | 0x9000 | 0xA000 | 0xB000 => {
                self.chr[usize::from((address >> 12) & 0x03)] = value;
            }
            // "7 bits used; D7 ignored": the board wires CHR A17 high for a nametable fetch, so
            // the bank is in the top half of CHR-ROM whatever bit 7 of the write says.
            0xC000 => self.nametable_bank[0] = value | 0x80,
            0xD000 => self.nametable_bank[1] = value | 0x80,
            0xE000 => {
                self.mirroring = value & 0x03;
                memory.set_mirroring(match self.mirroring {
                    0 => NametableMirroring::Vertical,
                    1 => NametableMirroring::Horizontal,
                    2 => NametableMirroring::SingleScreenA,
                    _ => NametableMirroring::SingleScreenB,
                });
                // Decoded and kept whatever the feature says; see
                // [`Sunsoft4::serves_chr_nametables`].
                self.chr_nametables = value & 0x10 != 0;
            }
            _ => {
                // "...E BBBB": four bits of bank and the PRG-RAM enable above them. Bit 3 is part
                // of the bank here and not *Nantettatte!! Baseball*'s chip select; see the
                // struct's own note on that gap.
                self.prg = value & 0x0F;
                self.prg_ram_on = value & 0x10 != 0;
            }
        }

        self.update_banks();
    }
}
