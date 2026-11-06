use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers **80** and **207**, the Taito X1-005 (MF-6c step 2).
///
/// nesdev ("Taito X1-005", "INES Mapper 080", "INES Mapper 207"). The chip's registers sit
/// **below** $8000, which is what makes this board unlike every other Taito one in this build:
///
/// | register | what it does |
/// | --- | --- |
/// | `$7EF0` | 2 KiB CHR bank at $0000; on mapper 207, bit 7 is CIRAM A10 for tables 0 and 1 |
/// | `$7EF1` | 2 KiB CHR bank at $0800; on mapper 207, bit 7 is CIRAM A10 for tables 2 and 3 |
/// | `$7EF2`-`$7EF5` | 1 KiB CHR banks at $1000, $1400, $1800 and $1C00 |
/// | `$7EF6` | nametable arrangement, bit 0: "0: vertical (A11); 1: horizontal (A10)" |
/// | `$7EF8` | RAM permission: **"write $A3 to enable"** the chip's 128 bytes |
/// | `$7EFA`, `$7EFC`, `$7EFE` | 8 KiB PRG banks at $8000, $A000 and $C000 |
///
/// $E000-$FFFF is "8 KiB PRG-ROM bank, fixed to the last bank".
///
/// Three readings of the page this build follows and **Mesen 2 does not**, all three settled for
/// the page by AMD-061 §1:
///
/// 1. **"Bit 0 of written bank number is ignored when switching 2K banks ($7EF0, $7EF1)."** So
///    the register holds a 2 KiB bank number, and an odd value selects the same pair as the even
///    one below it. Mesen takes the value as a 1 KiB bank and maps `value` and `value + 1`
///    (`TaitoX1005.h:54-55`), which differs for every odd write.
/// 2. **"$7EF6 … is not mirrored at $7EF7."** Mesen answers both (`TaitoX1005.h:75`).
/// 3. **"All registers exist at $7EFx and mirrored at $7E7x because the CPU A7 line is
///    ignored."** Mesen decodes $7EF0-$7EFF only (`RegisterStartAddress`), so its board does not
///    answer the $7E7x image at all. [`TaitoX1005::register_of`] folds the two.
///
/// # The 128 bytes, and where they live
///
/// "CPU $7F00-$7F7F: 128 B internal RAM", mirrored at $7F80-$7FFF, and **nothing else in
/// $6000-$7FFF is decoded**. So the board owns that whole window in both directions — through
/// [`Mapper::cpu_peek_override`] for reads and [`Mapper::intercept_cpu_write`] for writes — which
/// is the shape the MMC6 already established here for its own 1 KiB (`Mmc3Board::cpu_peek_override`),
/// and it is what lets the three `prg_ram_*` methods stay unforwarded in [`super::SmallBoard`].
///
/// The bytes themselves are `CartridgeMemory`'s PRG-RAM, sized to exactly 128 by
/// `registry::board_memory_sizes`, so REQ-CORE-095's single battery slice *is* the chip's RAM —
/// §4.6 of `docs/review/MF-6-inventory.md` asks for the Namco 163's layout and this is that
/// layout with no header RAM in front of it. **One divergence from Mesen's `.sav`**: Mesen's is
/// 256 bytes, because `GetSaveRamPageSize` is 256 and its own comment says why — "The current
/// BaseMapper code doesn't support mapping blocks smaller than 256 bytes" — so its file is the
/// 128 bytes written twice. `NesCore::import_battery_ram` copies the shorter of the two lengths,
/// so such a file still loads into this build's 128.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaitoX1005 {
    banks: BankWindows,
    /// The six CHR registers of $7EF0-$7EF5, exactly as written. Bit 7 of the first two is
    /// mapper 207's CIRAM A10, which is why the raw byte is kept and not just the bank.
    chr: [u8; 6],
    /// The three PRG registers of $7EFA, $7EFC and $7EFE.
    prg: [u8; 3],
    /// $7EF8's latch. The RAM answers only while it holds [`TaitoX1005::RAM_MAGIC`].
    ram_permission: u8,
    /// Mapper 207, decided at load: "modifies Taito's X1-005 (80) boards in the exact same way
    /// that TLSROM (118) differs from MMC3 (4)", by wiring CHR A17 to CIRAM A10. $7EF6 is then
    /// "was old mirroring control register, now ignored".
    alternate_mirroring: bool,
    /// What [`Mapper::nametable_page`] answers, recomputed whenever $7EF0 or $7EF1 moves.
    ///
    /// A field and not a `match`, for the reason `Mmc3Board::nametable_pages` gives: that
    /// dispatch has to stay one load per arm. Derived — `load_registers` recomputes it and no
    /// savestate byte is spent on it (REQ-CORE-091).
    nametable_pages: [i32; 4],
}

impl TaitoX1005 {
    /// The chip's RAM: "CPU $7F00-$7F7F: 128 B internal RAM".
    pub(crate) const RAM_SIZE: usize = 0x80;

    /// "Write $A3 to enable 128 B internal RAM; other values disable it."
    const RAM_MAGIC: u8 = 0xA3;

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        Self {
            banks: windows_for(memory),
            chr: [0; 6],
            prg: [0; 3],
            ram_permission: 0,
            alternate_mirroring: mapper == 207,
            nametable_pages: [super::super::Cartridge::FOLLOW_MIRRORING; 4],
        }
    }

    /// The register `address` selects, with the $7E7x image folded onto $7EFx; `None` when the
    /// address is not a register at all.
    ///
    /// "All registers exist at $7EFx and mirrored at $7E7x because the CPU A7 line is ignored",
    /// so setting A7 rather than masking it is what makes one decode of two ranges.
    const fn register_of(address: u16) -> Option<u16> {
        let register = address | 0x0080;
        if matches!(register, 0x7EF0..=0x7EFF) {
            Some(register)
        } else {
            None
        }
    }

    /// True when `address` is one of the chip's 128 bytes: "$7F00-$7F7F: 128 B internal RAM",
    /// mirrored at $7F80-$7FFF, so A7 is ignored here too.
    const fn is_internal_ram(address: u16) -> bool {
        address >= 0x7F00
    }

    /// Index in the 128 bytes of a CPU address in $7F00-$7FFF.
    fn ram_offset(address: u16) -> usize {
        usize::from(address) & 0x7F
    }

    fn update_banks(&mut self) {
        for (slot, bank) in self.prg.iter().enumerate() {
            self.banks.map_prg_8k(slot, i32::from(*bank));
        }
        self.banks.map_prg_8k(3, -1);

        // "Bit 0 of written bank number is ignored when switching 2K banks": the register is a
        // 2 KiB bank number, so `map_chr_2k` takes it as it stands.
        self.banks.map_chr_2k(0, i32::from(self.chr[0] >> 1));
        self.banks.map_chr_2k(2, i32::from(self.chr[1] >> 1));
        for window in 0..4 {
            self.banks
                .map_chr_1k(4 + window, i32::from(self.chr[2 + window]));
        }

        self.refresh_nametable_pages();
    }

    /// Mapper 207's two nametable selects: CHR A17 — bit 7 of a 2 KiB register, the same line
    /// bit 7 of a 1 KiB TxSROM register is — drives CIRAM A10 for the pair of tables its register
    /// covers.
    fn refresh_nametable_pages(&mut self) {
        if !self.alternate_mirroring {
            return;
        }

        let page = |register: u8| i32::from(register >> 7);
        self.nametable_pages = [
            page(self.chr[0]),
            page(self.chr[0]),
            page(self.chr[1]),
            page(self.chr[1]),
        ];
    }
}

impl Mapper for TaitoX1005 {
    banked!();

    fn mapper_number(&self) -> i32 {
        if self.alternate_mirroring {
            207
        } else {
            80
        }
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_raw(&self.prg);
        writer.write_u8(self.ram_permission);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        reader.read_into(&mut self.prg)?;
        self.ram_permission = reader.read_u8()?;
        // Derived, and therefore not in the block: MF-5a step 3's `level` lesson.
        self.refresh_nametable_pages();
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 6];
        self.prg = [0; 3];
        self.ram_permission = 0;
        self.nametable_pages = [super::super::Cartridge::FOLLOW_MIRRORING; 4];
        self.update_banks();
    }

    fn nametable_page(&self, table: i32) -> i32 {
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 3]
    }

    /// **`false`**, so that `Cartridge::default_cpu_read`'s generic $6000-$7FFF path stands
    /// aside and [`TaitoX1005::try_read_prg`] below answers instead: only $7F00-$7FFF of that
    /// window is decoded, and only while $7EF8 holds $A3, which one address-independent `bool`
    /// cannot say. It also makes `Mapper::prg_ram_writable`'s default `false`, which is right for
    /// the same reason — the board's own [`Mapper::intercept_cpu_write`] has already consumed
    /// every write in the window.
    fn prg_ram_enabled(&self) -> bool {
        false
    }

    /// PRG-ROM above $8000, and the chip's 128 bytes at $7F00-$7FFF while $7EF8 holds $A3.
    ///
    /// Everything else in $6000-$7FFF answers `None`, which is open bus: "$7F00-$7F7F: 128 B
    /// internal RAM" is the only memory the chip decodes there, and the registers are write-only.
    /// See [`super::SmallBoard::try_read_prg`] on why this hook and not
    /// [`Mapper::cpu_peek_override`] — it is the whole performance story of MF-6c step 2.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        if !Self::is_internal_ram(address) || self.ram_permission != Self::RAM_MAGIC {
            return None;
        }

        memory.prg_ram().get(Self::ram_offset(address)).copied()
    }

    /// `#[inline(never)]`, as every `SmallBoard` register decode is since MF-6a step 1: the
    /// measurement there is what keeps a board's decode out of `NesBus::write_internal`'s layout.
    #[inline(never)]
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if !(0x6000..=0x7FFF).contains(&address) {
            return false;
        }

        if let Some(register) = Self::register_of(address) {
            self.write_x1_register(memory, register, value);
            return true;
        }

        if Self::is_internal_ram(address) && self.ram_permission == Self::RAM_MAGIC {
            let offset = Self::ram_offset(address);
            if let Some(byte) = memory.prg_ram_mut().get_mut(offset) {
                *byte = value;
            }
        }

        // Handled either way: nothing else in this window exists, so the generic PRG-RAM path
        // must not see it.
        true
    }
}

impl TaitoX1005 {
    /// One register write, `register` already folded onto $7EF0-$7EFF by
    /// [`TaitoX1005::register_of`].
    fn write_x1_register(&mut self, memory: &mut CartridgeMemory, register: u16, value: u8) {
        match register {
            0x7EF0..=0x7EF5 => {
                self.chr[usize::from(register - 0x7EF0)] = value;
                self.update_banks();
            }
            // "$7EF6 … is not mirrored at $7EF7", and on mapper 207 it "was old mirroring
            // control register, now ignored".
            0x7EF6 if !self.alternate_mirroring => memory.set_mirroring(if value & 0x01 == 0 {
                NametableMirroring::Horizontal
            } else {
                NametableMirroring::Vertical
            }),
            0x7EF8 | 0x7EF9 => self.ram_permission = value,
            0x7EFA..=0x7EFF => {
                self.prg[usize::from((register - 0x7EFA) >> 1)] = value;
                self.update_banks();
            }
            _ => {}
        }
    }
}
