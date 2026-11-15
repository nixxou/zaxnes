#[cfg(feature = "board-394")]
use super::Mmc3;
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers **35, 90, 209 and 211**: 晶太 (Jīngtài) / J.Y. Company's proprietary ASIC
/// (MF-UNL-4).
///
/// nesdev, "J.Y. Company ASIC" — **one page for all four numbers**, to which `INES Mapper 035`,
/// `INES Mapper 090`, `INES Mapper 209` and `INES Mapper 211` all redirect. The chip is the one
/// behind thirteen of Hummer Team's unlicensed ports (*Super Mario World*, *Aladdin*,
/// *Final Fight 3*, *Tekken 2*, *Donkey Kong Country 4*, *Contra Spirits*, …), which
/// `docs/review/UNL-census.md` §1.6 calls "by a wide margin, the most-played unlicensed NES
/// software there is".
///
/// # The register map
///
/// | window | mask | what it is |
/// | --- | --- | --- |
/// | `$5000`, `$5400`, `$5C00` (read) | `$FFFF` | board solder pad, `JJ.. ....` |
/// | `$5800`-`$5801` | `$F803` | unsigned 8×8→16 hardware multiplier |
/// | `$5802`-`$5803` | `$F803` | an accumulator and a test register |
/// | `$8000`-`$8003` | `$F803` | the four PRG bank registers |
/// | `$9000`-`$9007` / `$A000`-`$A007` | `$F807` | the eight CHR bank registers, LSB / MSB |
/// | `$B000`-`$B003` / `$B004`-`$B007` | `$F807` | the four nametable registers, LSB / MSB |
/// | `$C000`-`$C007` | `$F007` | the IRQ |
/// | `$D000`-`$D003` | `$F803` | the banking modes, the mirroring, the PPU configuration and the outer bank |
///
/// The `$F803`/`$F807` masks mean a write is **ignored when address bit 11 is set**, so each
/// window is only the low 2 KiB of its 4 KiB block; `$C00x` alone accepts bit 11, its mask being
/// `$F007`. The page states each one separately and this board tests each one separately.
///
/// # The four numbers, and the two `bool`s between them
///
/// | number | the page's words |
/// | ---: | --- |
/// | **209** | "the **standard implementation**, used for most single-game cartridges" |
/// | **90** | "inhibits, **via jumper**, ROM nametables and Extended Mirroring that would otherwise be enabled via register" |
/// | **211** | "a mere duplicate of INES Mapper 209 **with correct emulation**" — "no such PCB exists in real hardware" |
/// | **35** | "like Mapper 209 but explicitly specifies 8 KiB of WRAM … an exact duplicate" |
///
/// So 209, 211 and 35 are the **same** board here and 90 is that board with
/// [`JyCompany::jumper_inhibits_nametables`] set; 35's difference is in
/// `registry::board_memory_sizes` and not in this file at all. Q-148 and Q-149 record the two
/// places that leaves Mesen 2 — which builds a wholly different and much smaller board for 35,
/// and special-cases 211 as "N always set" against its own page's sentence.
///
/// # Where the hooks go
///
/// The board is an [`IrqBoard`](super::IrqBoard) variant, which is what makes the census's "four
/// hooks at once" affordable: that enum **already forwards** `on_cpu_cycle`,
/// `on_ppu_address_bus`, `irq_asserted`, `nametable_page`, `read_nametable`, `prg_ram_enabled`
/// and `try_read_prg`, so this board adds **no arm to any `Board` dispatch**. MF-UNL-4 step 1
/// measured the one that was supposed to be dear — `Mapper::read_nametable`, Q-123's ceiling —
/// at `Ppu2C02::read_nametable_inner` **80 instructions in all three binaries, identical to the
/// instruction**; `rust/docs/PERFORMANCE-HISTORY.md` §5.18 has it.
///
/// The MMC4-like CHR latch is on `on_ppu_address_bus` and **not** on `Mapper::ppu_read`, which
/// `IrqBoard` does not override: the page says the latch moves when "the PPU **reads** from
/// $0FD8-$0FDF", every such read already reaches `Cartridge::on_ppu_address_bus`, and overriding
/// `ppu_read` would add a distinct body to a dispatch §4.10 of the inventory prices at "one arm's
/// worth, +16 measured" for every board of the enum. Mesen 2 puts the same latch on the same hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JyCompany {
    banks: BankWindows,
    mapper_number: i32,
    /// Mapper 90's jumper: "inhibits … ROM nametables and Extended Mirroring that would
    /// otherwise be enabled via register". The one per-number parameter of the board itself.
    jumper_inhibits_nametables: bool,

    /// `$8000`-`$8003`, seven bits each. Their unit is the PRG banking mode's: an 8 KiB bank in
    /// modes 2 and 3, a 16 KiB bank in mode 1, a 32 KiB bank in mode 0.
    prg: [u8; 4],
    /// `$9000`-`$9007`, the CHR bank registers' LSBs.
    chr_low: [u8; 8],
    /// `$A000`-`$A007`, their MSBs.
    chr_high: [u8; 8],
    /// `$B000`-`$B003`: a CIRAM bank in bit 0, the ROM/CIRAM select in bit 7, and the LSB of a
    /// 1 KiB CHR bank in all eight bits at once — the page gives bit 7 all three jobs.
    nametable_low: [u8; 4],
    /// `$B004`-`$B007`, the MSBs of those CHR banks.
    nametable_high: [u8; 4],

    /// `$D000` bits 1-0: 0 = 32 KiB, 1 = 16 KiB, 2 = 8 KiB, 3 = 8 KiB with the bank numbers'
    /// bits 0-6 reversed.
    prg_mode: u8,
    /// `$D000` bit 2: the `$E000`/`$C000`/`$8000`-`$FFFF` bank is switchable via `$8003` rather
    /// than hard-wired to the last bank of the outer bank.
    prg_last_switchable: bool,
    /// `$D000` bits 4-3: 0 = 8 KiB, 1 = 4 KiB, 2 = 2 KiB, 3 = 1 KiB.
    chr_mode: u8,
    /// `$D000` bit 5: ROM nametables enabled.
    rom_nametables: bool,
    /// `$D000` bit 6: ROM nametables for **all** four tables, rather than for the ones
    /// `$B00x` bit 7 selects against `$D002` bit 7.
    rom_nametables_all: bool,
    /// `$D000` bit 7: `$6000`-`$7FFF` shows an 8 KiB PRG-ROM bank instead of WRAM.
    prg_rom_at_6000: bool,

    /// `$D001` bits 1-0, the nametable arrangement. The page's names are *arrangements* and this
    /// core's are *mirrorings*, which are each other's opposites: "0: Horizontal (A10)" is
    /// [`NametableMirroring::Vertical`].
    arrangement: u8,
    /// `$D001` bit 3: Extended Mirroring — the CIRAM bank of each table from `$B00x` bit 0.
    extended_mirroring: bool,

    /// `$D002` bit 6: CHR write-enable. "Bit 6 only matters when using CHR-RAM."
    chr_write_enabled: bool,
    /// `$D002` bit 7: which value of `$B00x` bit 7 means ROM. Clear → bit 7 set selects ROM.
    nametable_select_invert: bool,

    /// `$D003` bit 7: the MMC4-like automatic CHR bankswitching mode, "only meaningful in 4 KiB
    /// CHR-ROM banking mode".
    mmc4_mode: bool,
    /// `$D003` bit 5, `L`: mask `$900x`/`$A00x` to 512 KiB and ignore `C`, rather than to
    /// 256 KiB and use it.
    outer_chr_512k: bool,
    /// `$D003` bits 4-3, `cc`: the 512 KiB outer CHR bank, CHR A19-A20.
    outer_chr_bank: u8,
    /// `$D003` bits 2-1, `PP`: the 512 KiB outer PRG bank, PRG A19-A20.
    outer_prg_bank: u8,
    /// `$D003` bit 0, `C`: the 256 KiB outer CHR bank, CHR A18. Ignored when `L` is set.
    outer_chr_a18: bool,
    /// Lot MAPPERS-P: the multicart that reuses `$D003` as a wider outer bank (281, 282 or 295),
    /// or `None` for the four single-game numbers. puNES (`mapper_281.c`, `mapper_282.c`,
    /// `mapper_295.c`) keeps the ASIC as it is and replaces only the two bank functions that
    /// place an 8 KiB PRG bank and a 1 KiB CHR bank in the outer bank; so does this board.
    multicart: Option<JyMulticart>,
    /// `$D003` as written, for [`JyCompany::multicart`]'s decodes, which use all eight bits.
    outer_raw: u8,
    /// Lot MAPPERS-P: mapper 394's board around the ASIC, `None` for every other number.
    #[cfg(feature = "board-394")]
    ext_394: Option<Box<Jy394>>,

    /// The two MMC4-like latches, one per pattern table, each 0 or 1.
    chr_latch: [u8; 2],

    /// `$C000` bit 0, `$C002` and `$C003`. The page gives it three jobs at once: "'Disabling'
    /// acknowledges a pending IRQ, inhibits counting, and resets the prescaler to zero."
    irq_enabled: bool,
    /// `$C001` bits 1-0: 0 = CPU M2 rise, 1 = PPU A12 rise, 2 = PPU reads, 3 = CPU writes.
    irq_source: u8,
    /// `$C001` bits 7-6: 0 and 3 disable counting, 1 increases, 2 decreases.
    irq_direction: u8,
    /// `$C001` bit 2: the prescaler mask is `$07` rather than `$FF`.
    irq_small_prescaler: bool,
    /// `$C001` bit 3: enables the `$C007` mode, "Its functionality is not known, and no known
    /// game uses it."
    irq_c007_mode: bool,
    /// `$C007`, kept so that the save state carries what a game wrote.
    irq_c007: u8,
    irq_prescaler: u8,
    irq_counter: u8,
    /// `$C006`: XORed into what `$C004` and `$C005` write.
    irq_xor: u8,
    irq_pending: bool,
    /// The previous PPU address, for the unfiltered A12 rise of IRQ source 1.
    last_ppu_address: u16,

    /// `$5800` write, the multiplicand.
    multiplicand: u8,
    /// `$5801` write, the multiplier.
    multiplier: u8,
    /// `$5802`: a write adds to it, `$5803` resets it to zero.
    accumulator: u8,
    /// `$5803`, the test register.
    test_register: u8,

    /// What [`Mapper::nametable_page`] answers for the four logical tables. A field and not a
    /// `match`, for the reason `Mmc3Board::nametable_pages` gives: that dispatch is consulted
    /// once per nametable fetch. Derived — `load_registers` recomputes it.
    nametable_pages: [i32; 4],
}

impl JyCompany {
    /// 1 KiB, the unit of a CHR window and of a ROM nametable.
    const CHR_BANK_SIZE: usize = 0x0400;
    /// 8 KiB, the unit of a PRG window.
    const PRG_BANK_SIZE: usize = 0x2000;

    /// The outer PRG bank size of all four numbers, in 8 KiB banks: "The outer PRG bank size is
    /// 512 KiB" (the page, on mapper 209, and 35, 90 and 211 are the same PCB family).
    const OUTER_PRG_BANKS: i32 = 512 / 8;
    /// The same in 1 KiB CHR banks, in the two sizes `$D003` bit 5 chooses between.
    const OUTER_CHR_BANKS_256K: i32 = 256;
    const OUTER_CHR_BANKS_512K: i32 = 512;

    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            mapper_number,
            jumper_inhibits_nametables: mapper_number == 90 || mapper_number == 388,
            prg: [0; 4],
            chr_low: [0; 8],
            chr_high: [0; 8],
            nametable_low: [0; 4],
            nametable_high: [0; 4],
            prg_mode: 0,
            prg_last_switchable: false,
            chr_mode: 0,
            rom_nametables: false,
            rom_nametables_all: false,
            prg_rom_at_6000: false,
            arrangement: 0,
            extended_mirroring: false,
            chr_write_enabled: false,
            nametable_select_invert: false,
            mmc4_mode: false,
            outer_chr_512k: false,
            outer_chr_bank: 0,
            outer_prg_bank: 0,
            outer_chr_a18: false,
            multicart: JyMulticart::of(mapper_number),
            outer_raw: 0,
            #[cfg(feature = "board-394")]
            ext_394: (mapper_number == 394).then(|| Box::new(Jy394::new(memory))),
            chr_latch: [0; 2],
            irq_enabled: false,
            irq_source: 0,
            irq_direction: 0,
            irq_small_prescaler: false,
            irq_c007_mode: false,
            irq_c007: 0,
            irq_prescaler: 0,
            irq_counter: 0,
            irq_xor: 0,
            irq_pending: false,
            last_ppu_address: 0,
            multiplicand: 0,
            multiplier: 0,
            accumulator: 0,
            test_register: 0,
            nametable_pages: [super::super::Cartridge::FOLLOW_MIRRORING; 4],
        };
        board.update_banks();
        #[cfg(feature = "board-394")]
        board.apply_394();
        board
    }

    /// The bank number of one of the four PRG registers, in the **mode's own unit**, with
    /// `$D000` bits 1-0's mode-3 bit reversal applied.
    ///
    /// "3: 8 KiB banking, but with bank numbers **bits 0-6 reversed**" — bit 0 ↔ bit 6,
    /// bit 1 ↔ bit 5, bit 2 ↔ bit 4, and bit 3 to itself. Mesen 2's `InvertPrgBits` drops bit 3:
    /// **Q-150** item 1. The page's "bits 0-6" includes it, and a seven-bit reversal that left
    /// the middle bit out would not be a reversal.
    fn prg_register(&self, index: usize) -> i32 {
        let value = self.prg[index];
        if self.prg_mode == 3 {
            i32::from(
                (value & 0x01) << 6
                    | (value & 0x02) << 4
                    | (value & 0x04) << 2
                    | (value & 0x08)
                    | (value & 0x10) >> 2
                    | (value & 0x20) >> 4
                    | (value & 0x40) >> 6,
            )
        } else {
            i32::from(value)
        }
    }

    /// An 8 KiB PRG bank, masked to the 512 KiB outer bank and placed in it by `$D003` bits 2-1.
    ///
    /// "The PRG-ROM bank selected via $8000-$8003 is always masked to the outer bank size …
    /// Accessing more than that requires manipulating the Outer Bank register at $D003."
    fn prg_bank(&self, bank_8k: i32) -> i32 {
        if let Some((prg_base, _)) = self.base_394() {
            return prg_base | (bank_8k & 0x1F);
        }
        if let Some(multicart) = self.multicart {
            return multicart.prg_bank(self.outer_raw, bank_8k);
        }
        (bank_8k & (Self::OUTER_PRG_BANKS - 1))
            | (i32::from(self.outer_prg_bank) * Self::OUTER_PRG_BANKS)
    }

    /// The 8 KiB bank the `$E000`/`$C000`/`$8000` window shows: `$8003` when `$D000` bit 2 says
    /// switchable, and otherwise "hard-wired to the last bank **within the outer bank**", which
    /// is why it is the outer bank's last and not the image's.
    fn prg_last_bank(&self, scale: i32) -> i32 {
        if self.prg_last_switchable {
            self.prg_bank(self.prg_register(3) * scale)
        } else {
            self.prg_bank(Self::OUTER_PRG_BANKS - scale)
        }
    }

    /// The 1 KiB CHR bank one of the eight registers selects, in the **mode's own unit**,
    /// already scaled to 1 KiB, masked to the outer bank size and placed in the outer bank.
    ///
    /// `$D003` bit 5 chooses the mask: "0: Mask $900x/$A00x to 256 KiB, use C / 1: Mask
    /// $900x/$A00x to 512 KiB, ignore C", with `cc` (bits 4-3) always the 512 KiB outer bank and
    /// `C` (bit 0) the 256 KiB one inside it. Mesen 2 drops the mask and the outer bank entirely
    /// in the `L=1` case and uses the raw 16-bit register instead: **Q-150** item 3.
    fn chr_bank(&self, index: usize) -> i32 {
        let register = i32::from(self.chr_low[index]) | (i32::from(self.chr_high[index]) << 8);
        let bank_1k = register * Self::CHR_UNIT_1K[usize::from(self.chr_mode) & 3];
        if let Some((_, chr_base)) = self.base_394() {
            return chr_base | (bank_1k & 0xFF);
        }
        if let Some(multicart) = self.multicart {
            return multicart.chr_bank(self.outer_raw, bank_1k);
        }
        let outer = i32::from(self.outer_chr_bank) * Self::OUTER_CHR_BANKS_512K;
        if self.outer_chr_512k {
            (bank_1k & (Self::OUTER_CHR_BANKS_512K - 1)) | outer
        } else {
            (bank_1k & (Self::OUTER_CHR_BANKS_256K - 1))
                | (i32::from(self.outer_chr_a18) * Self::OUTER_CHR_BANKS_256K)
                | outer
        }
    }

    /// How many 1 KiB banks one unit of each CHR mode is: 8 KiB, 4 KiB, 2 KiB, 1 KiB.
    const CHR_UNIT_1K: [i32; 4] = [8, 4, 2, 1];

    /// Which of the eight CHR registers serves the 4 KiB window `window` (0 or 1) under the
    /// MMC4-like mode.
    ///
    /// "$9000/$A000 … when latch =0 / $9002/$A002 … when latch =1" for `$0000`-`$0FFF`, and
    /// `$9004`/`$9006` the same way for `$1000`-`$1FFF`. With the mode off the page's plain
    /// 4 KiB layout is `$9000` and `$9004`, which is the same table read with both latches at 0.
    fn chr_4k_register(&self, window: usize) -> usize {
        window * 4
            + if self.mmc4_mode {
                usize::from(self.chr_latch[window]) * 2
            } else {
                0
            }
    }

    /// `$D001`'s arrangement as this core's mirroring. The page's arrangements are this core's
    /// mirrorings reversed: "0: Horizontal (A10)" puts $2000 and $2400 on different pages, which
    /// is `Vertical` here.
    const fn arrangement_mirroring(&self) -> NametableMirroring {
        match self.arrangement {
            0 => NametableMirroring::Vertical,
            1 => NametableMirroring::Horizontal,
            2 => NametableMirroring::SingleScreenA,
            _ => NametableMirroring::SingleScreenB,
        }
    }

    /// `prg_base()`/`chr_base()` of `mapper_394.c`, `None` for every other number.
    #[allow(clippy::unused_self)]
    fn base_394(&self) -> Option<(i32, i32)> {
        #[cfg(feature = "board-394")]
        if let Some(ext) = &self.ext_394 {
            return Some(ext.bases());
        }
        None
    }

    /// 394's MMC3 while `$5001` D4 is clear (`m394.reg[1] & $10`), `None` while the ASIC runs.
    #[cfg(feature = "board-394")]
    fn mmc3_394(&self) -> Option<&Mmc3> {
        self.ext_394
            .as_deref()
            .filter(|ext| ext.regs[1] & 0x10 == 0)
            .map(|ext| &ext.mmc3)
    }

    #[cfg(feature = "board-394")]
    fn mmc3_394_mut(&mut self) -> Option<&mut Mmc3> {
        self.ext_394
            .as_deref_mut()
            .filter(|ext| ext.regs[1] & 0x10 == 0)
            .map(|ext| &mut ext.mmc3)
    }

    /// `prg_swap_mmc3_394`/`chr_swap_mmc3_394`: the MMC3's windows through the outer bank;
    /// `$5001` D3 clear is the NROM mode, a 32 KiB bank from `$5003` D0-D2.
    #[cfg(feature = "board-394")]
    fn apply_394(&mut self) {
        let Some(ext) = self.ext_394.as_deref_mut() else {
            return;
        };
        let (prg_base, chr_base) = ext.bases();
        let r = ext.regs.map(i32::from);
        ext.mmc3.refresh_banks();
        let Some(b) = ext.mmc3.banks_mut() else {
            return;
        };
        if r[1] & 0x08 == 0 {
            let base = (prg_base | ((r[3] & 0x07) << 1)) & 0xFC;
            for slot in 0..4 {
                b.map_prg_8k(slot, base | i32::try_from(slot).unwrap_or(0));
            }
        } else {
            let mask = 0x1F >> i32::from(r[3] & 0x10 == 0);
            b.remap_prg_banks(mask, prg_base & !mask);
        }
        b.remap_chr_banks(0xFF >> i32::from(r[3] & 0x80 == 0), chr_base);
    }

    /// `extcl_cpu_wr_mem_394`: `$5000-$5FFF` the board's four registers (A0-A1), after which the
    /// chip in use re-fixes its windows and mirroring (`extcl_after_mapper_init_394`); every write
    /// then goes to that chip.
    #[cfg(feature = "board-394")]
    fn write_394(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if (0x5000..=0x5FFF).contains(&address) {
            if let Some(ext) = self.ext_394.as_deref_mut() {
                ext.regs[usize::from(address & 0x03)] = value;
            }
            self.update_banks();
            self.apply_394();
            match self.ext_394.as_deref_mut() {
                Some(ext) if ext.regs[1] & 0x10 == 0 => {
                    ext.mmc3.write_register(memory, 0xA000, ext.a000);
                    self.apply_394();
                }
                _ => memory.set_mirroring(self.arrangement_mirroring()),
            }
        }
        if let Some(ext) = self.ext_394.as_deref_mut() {
            if ext.regs[1] & 0x10 == 0 {
                if address & 0xE001 == 0xA000 {
                    ext.a000 = value;
                }
                ext.mmc3.write_register(memory, address, value);
                self.apply_394();
                return;
            }
        }
        self.write_jy(memory, address, value);
    }

    /// What [`Mapper::nametable_page`] answers for logical table `table`.
    ///
    /// The page gives the four `$B00x` registers two jobs and a jumper that cancels both:
    ///
    /// * **Extended Mirroring** (`$D001` bit 3): "Enabled, ignore MM bits, use $B000-$B003
    ///   bit 0" — one CIRAM bank per table, which is §4.5 of the inventory's *cheap* level.
    /// * **ROM nametables** (`$D000` bit 5): the table comes from a 1 KiB CHR bank instead, for
    ///   all four tables when `$D000` bit 6 is set and otherwise for the ones whose `$B00x`
    ///   bit 7 **differs** from `$D002` bit 7. That is `SERVED_BY_CARTRIDGE`, the dear level, and
    ///   [`JyCompany::read_nametable`] is the body. **Q-149**: Mesen 2 decodes neither `$D001`
    ///   bit 3 nor the two mechanisms separately, and special-cases mapper 211 instead.
    /// * **mapper 90's jumper**, which "inhibits … ROM nametables and Extended Mirroring that
    ///   would otherwise be enabled via register": both, so the four registers "have no
    ///   function" and the layout is `$D001`'s MM bits alone.
    ///
    /// **ROM nametables are tested before Extended Mirroring**, and the page's own last line of
    /// the `$B00x` section is why: "enabling ROM nametables via $D005 bit 5 and all $B00x
    /// registers having bit 7 match $D002 bit 7 effectively duplicates the functionality of
    /// Extended Mirroring". With bit 5 set and bit 6 clear the per-table decode *already*
    /// contains Extended Mirroring's own answer — `$B00x` bit 0 as the CIRAM bank — so the two
    /// orders agree wherever both bits are set and bit 6 is clear, and where bit 6 is set the
    /// page's "enabled globally for all nametables" leaves nothing to choose. This is a
    /// derivation and not a guess, which is why it is not a Q ticket.
    fn nametable_page_of(&self, table: usize) -> i32 {
        if self.jumper_inhibits_nametables {
            return super::super::Cartridge::FOLLOW_MIRRORING;
        }

        if self.rom_nametables {
            if self.rom_nametables_all {
                return super::super::Cartridge::SERVED_BY_CARTRIDGE;
            }

            // "$B000 bit 0: Select 1 KiB CIRAM bank … if $B000 bit 7 **matches** $D002 bit 7"
            // and the CHR bank "if $B000 bit 7 **differs** from $D002 bit 7".
            if (self.nametable_low[table] & 0x80 != 0) != self.nametable_select_invert {
                return super::super::Cartridge::SERVED_BY_CARTRIDGE;
            }

            return i32::from(self.nametable_low[table] & 0x01);
        }

        if self.extended_mirroring {
            return i32::from(self.nametable_low[table] & 0x01);
        }

        super::super::Cartridge::FOLLOW_MIRRORING
    }

    /// Recomputes the four-entry cache [`Mapper::nametable_page`] reads.
    ///
    /// A field and not a `match`, for the reason `Mmc3Board::nametable_pages` gives: that dispatch
    /// is consulted once per nametable fetch. Derived, so it is **not** in the save-state block
    /// and `load_registers` recomputes it — MF-5a step 3's `level` lesson.
    fn refresh_nametable_pages(&mut self) {
        self.nametable_pages = std::array::from_fn(|table| self.nametable_page_of(table));
    }

    fn update_banks(&mut self) {
        match self.prg_mode & 0x03 {
            // 32 KiB: "$8003: Select 32 KiB PRG-ROM bank at CPU $8000-$FFFF". The register's
            // unit is a 32 KiB bank, which is four of `BankWindows`'s, and the hard-wired case
            // is the outer bank's last 32 KiB. Q-150 item 2: Mesen 2 reads the register as an
            // 8 KiB bank here and as a 32 KiB one for the $6000 window, in the same build.
            0 => {
                let base = self.prg_last_bank(4);
                for slot in 0..4 {
                    self.banks
                        .map_prg_8k(slot, base + i32::try_from(slot).unwrap_or(0));
                }
            }
            // 16 KiB: "$8001: … at CPU $8000-$BFFF" and "$8003: … at CPU $C000-$FFFF".
            1 => {
                let low = self.prg_bank(self.prg_register(1) * 2);
                self.banks.map_prg_8k(0, low);
                self.banks.map_prg_8k(1, low + 1);
                let high = self.prg_last_bank(2);
                self.banks.map_prg_8k(2, high);
                self.banks.map_prg_8k(3, high + 1);
            }
            // 8 KiB, with and without mode 3's bit reversal.
            _ => {
                for slot in 0..3 {
                    let bank = self.prg_bank(self.prg_register(slot));
                    self.banks.map_prg_8k(slot, bank);
                }
                self.banks.map_prg_8k(3, self.prg_last_bank(1));
            }
        }

        match self.chr_mode & 0x03 {
            0 => {
                let bank = self.chr_bank(0);
                for slot in 0..BankWindows::CHR_WINDOWS {
                    self.banks
                        .map_chr_1k(slot, bank + i32::try_from(slot).unwrap_or(0));
                }
            }
            1 => {
                for window in 0..2 {
                    let bank = self.chr_bank(self.chr_4k_register(window));
                    for page in 0..4 {
                        self.banks
                            .map_chr_1k(window * 4 + page, bank + i32::try_from(page).unwrap_or(0));
                    }
                }
            }
            2 => {
                for window in 0..4 {
                    let bank = self.chr_bank(window * 2);
                    self.banks.map_chr_1k(window * 2, bank);
                    self.banks.map_chr_1k(window * 2 + 1, bank + 1);
                }
            }
            _ => {
                for slot in 0..BankWindows::CHR_WINDOWS {
                    let bank = self.chr_bank(slot);
                    self.banks.map_chr_1k(slot, bank);
                }
            }
        }

        self.refresh_nametable_pages();
    }

    /// `$5800`-`$5803`: the multiplier's two operands, the accumulator and the test register.
    /// Mask `$F803`, and the solder pad register is read-only.
    fn write_arithmetic_register(&mut self, address: u16, value: u8) {
        match address & 0xF803 {
            0x5800 => self.multiplicand = value,
            0x5801 => self.multiplier = value,
            // "$5802 write: Increase accumulator by written value."
            0x5802 => self.accumulator = self.accumulator.wrapping_add(value),
            // "$5803 write: Reset accumulator to 0 and set test register to written value."
            0x5803 => {
                self.accumulator = 0;
                self.test_register = value;
            }
            _ => {}
        }
    }

    /// `$C000`-`$C007`, mask `$F007`: the only window of the chip whose writes are "accepted even
    /// if address bit 11 is set", which is why there is no `0x0800` guard here.
    fn write_irq_register(&mut self, address: u16, value: u8) {
        match address & 0x07 {
            // "`.... ...E` — Disable (0)/Enable (1) IRQ generation."
            0 => {
                if value & 0x01 == 0 {
                    self.disable_irq();
                } else {
                    self.irq_enabled = true;
                }
            }
            // "`DD.. EPSS`".
            1 => {
                self.irq_direction = (value >> 6) & 0x03;
                self.irq_c007_mode = value & 0x08 != 0;
                self.irq_small_prescaler = value & 0x04 != 0;
                self.irq_source = value & 0x03;
            }
            2 => self.disable_irq(),
            3 => self.irq_enabled = true,
            // "(*) The written value is XORed with the content of register $C006 before being
            // stored." Both of these, and not `$C006` itself.
            4 => self.irq_prescaler = value ^ self.irq_xor,
            5 => self.irq_counter = value ^ self.irq_xor,
            6 => self.irq_xor = value,
            // "$C007: Configure unknown mode … Its functionality is not known, and no known game
            // uses it." Kept so that a save state carries what a game wrote.
            _ => self.irq_c007 = value,
        }
    }

    /// `$D000`-`$D003`, the four configuration registers, with A11 already tested by the caller.
    fn write_config_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0x03 {
            0 => {
                self.prg_mode = value & 0x03;
                self.prg_last_switchable = value & 0x04 != 0;
                self.chr_mode = (value >> 3) & 0x03;
                self.rom_nametables = value & 0x20 != 0;
                self.rom_nametables_all = value & 0x40 != 0;
                self.prg_rom_at_6000 = value & 0x80 != 0;
            }
            1 => {
                self.arrangement = value & 0x03;
                self.extended_mirroring = value & 0x08 != 0;
                // The page's arrangements are this core's mirrorings reversed: "0:
                // Horizontal (A10)" puts $2000 and $2400 on different pages, which is
                // `Vertical` here.
                memory.set_mirroring(self.arrangement_mirroring());
            }
            2 => {
                self.chr_write_enabled = value & 0x40 != 0;
                self.nametable_select_invert = value & 0x80 != 0;
            }
            _ => {
                // `4.Lc cPPC`.
                self.mmc4_mode = value & 0x80 != 0;
                self.outer_chr_512k = value & 0x20 != 0;
                self.outer_chr_bank = (value >> 3) & 0x03;
                self.outer_prg_bank = (value >> 1) & 0x03;
                self.outer_chr_a18 = value & 0x01 != 0;
                self.outer_raw = value;
            }
        }
    }

    /// The 16-bit product of the two multiplier operands. "After writing the two operands to
    /// $5800 and $5801, the result can be read 8 M2 cycles later; if the registers are read from
    /// earlier than that, intermediate results are obtained." The intermediate results are not
    /// modelled — the product is whole as soon as both operands are in — which is Mesen 2's
    /// reading too, and no documented game reads the registers early.
    fn product(&self) -> u16 {
        u16::from(self.multiplicand) * u16::from(self.multiplier)
    }

    /// Sets latch `window` to `value`, recomputing the windows only when it moved.
    ///
    /// The test matters: this runs on the PPU's address bus, 57 600-odd times a frame on a
    /// cartridge of this board, and all but a handful of those addresses leave both latches where
    /// they are. It is `Mmc2::set_latch`'s own shape, for the same reason.
    fn set_latch(&mut self, window: usize, value: u8) {
        if self.chr_latch[window] == value {
            return;
        }

        self.chr_latch[window] = value;
        self.update_banks();
    }

    /// The MMC4-like latch on a PPU read of `address`.
    ///
    /// "If MMC4-like bankswitching mode is enabled via $D003 bit 7, the latch is set to 0 when
    /// the PPU reads from $0FD8-$0FDF/$1FD8-$1FDF and to 1 when the PPU reads from
    /// $0FE8-$0FEF/$1FE8-$1FEF."
    ///
    /// **Both latches move whatever the CHR banking mode is**, and only mode 1 reads them. The
    /// page's "only meaningful in 4 KiB CHR-ROM banking mode" says where the latch *matters*, not
    /// where it moves, and a latch frozen by a mode change would be a second piece of state the
    /// page does not describe. [`JyCompany::chr_4k_register`] is the one place the value is used.
    ///
    /// Mesen 2 moves the same latches on the same hook, but gates them on **`MapperID == 209`**
    /// rather than on `$D003` bit 7 — so mapper 90's *Mighty Morphin' Power Rangers III*, which
    /// the page says "uses MMC4-like mode for displaying title screen", would not get it there.
    /// The page gates the mode on the register for every number of the chip: **Q-150** item 5.
    fn move_chr_latch(&mut self, address: u16) {
        if !self.mmc4_mode || address >= 0x2000 {
            return;
        }

        let window = usize::from(address & 0x1000 != 0);
        match address & 0x0FF8 {
            0x0FD8 => self.set_latch(window, 0),
            0x0FE8 => self.set_latch(window, 1),
            _ => {}
        }
    }

    /// `$C001` bits 1-0: "0: CPU M2 rise / 1: PPU A12 rise (unfiltered, eight per scanline) /
    /// 2: PPU reads (170 per scanline) / 3: CPU writes".
    const SOURCE_CPU_M2: u8 = 0;
    const SOURCE_PPU_A12: u8 = 1;
    const SOURCE_PPU_READ: u8 = 2;

    /// "'Disabling' acknowledges a pending IRQ, inhibits counting, and resets the prescaler to
    /// zero" — all three, from `$C000` with bit 0 clear and from `$C002`.
    ///
    /// Mesen 2 acknowledges and inhibits but leaves the prescaler where it was: **Q-151** item 2,
    /// and "inhibits counting" is item 1.
    fn disable_irq(&mut self) {
        self.irq_enabled = false;
        self.irq_pending = false;
        self.irq_prescaler = 0;
    }

    /// One clock of the selected source, through the prescaler and into the counter.
    ///
    /// The page's three numbered steps, in order:
    ///
    /// 1. "When counting is enabled and the counter is clocked from the source selected by $C001
    ///    bits 0-1, the prescaler is increased/decreased (depending on $C001 bits 6-7)."
    /// 2. "If the AND-masked prescaler (according to $C001 bit 2) wraps, meaning it is zero after
    ///    increasing or **matches the prescaler mask after decreasing**, the IRQ counter is
    ///    increased/decreased."
    /// 3. "If the IRQ counter wraps, meaning it is zero after increasing or $FF after decreasing,
    ///    an IRQ is generated."
    ///
    /// Two of the three leave Mesen 2, and **Q-151** items 1 and 3 record both. Step 1's "when
    /// counting is enabled" is the `irq_enabled` test: the page's own gloss on disabling says it "inhibits
    /// counting", where Mesen clocks the prescaler whatever the enable says and tests it only
    /// before asserting. Step 2's decreasing case is `prescaler == mask`, the value a decrement
    /// from zero leaves; Mesen's `if(--prescaler == 0)` wraps one count earlier, off a prescaler
    /// of **one**.
    ///
    /// The bits above the mask are left alone in both directions, which is Mesen's reading and
    /// the only one that makes `$C001` bit 2 a *mask* rather than a width.
    fn tick_irq(&mut self) {
        if !self.irq_enabled {
            return;
        }

        let mask = if self.irq_small_prescaler { 0x07 } else { 0xFF };
        let wrapped = match self.irq_direction {
            1 => {
                let next = (self.irq_prescaler & mask).wrapping_add(1) & mask;
                self.irq_prescaler = (self.irq_prescaler & !mask) | next;
                next == 0
            }
            2 => {
                let next = (self.irq_prescaler & mask).wrapping_sub(1) & mask;
                self.irq_prescaler = (self.irq_prescaler & !mask) | next;
                next == mask
            }
            // "0: Counting disabled" and "3: Counting disabled".
            _ => return,
        };
        if !wrapped {
            return;
        }

        if self.irq_direction == 1 {
            self.irq_counter = self.irq_counter.wrapping_add(1);
            self.irq_pending |= self.irq_counter == 0;
        } else {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
            self.irq_pending |= self.irq_counter == 0xFF;
        }
    }
}

impl Mapper for JyCompany {
    fn banks(&self) -> Option<&BankWindows> {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394() {
            return mmc3.banks();
        }
        Some(&self.banks)
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        #[cfg(feature = "board-394")]
        if self.mmc3_394().is_some() {
            return self.mmc3_394_mut().and_then(Mmc3::banks_mut);
        }
        Some(&mut self.banks)
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr_low);
        writer.write_raw(&self.chr_high);
        writer.write_raw(&self.nametable_low);
        writer.write_raw(&self.nametable_high);
        writer.write_u8(self.prg_mode);
        writer.write_bool(self.prg_last_switchable);
        writer.write_u8(self.chr_mode);
        writer.write_bool(self.rom_nametables);
        writer.write_bool(self.rom_nametables_all);
        writer.write_bool(self.prg_rom_at_6000);
        writer.write_u8(self.arrangement);
        writer.write_bool(self.extended_mirroring);
        writer.write_bool(self.chr_write_enabled);
        writer.write_bool(self.nametable_select_invert);
        writer.write_bool(self.mmc4_mode);
        writer.write_bool(self.outer_chr_512k);
        writer.write_u8(self.outer_chr_bank);
        writer.write_u8(self.outer_prg_bank);
        writer.write_bool(self.outer_chr_a18);
        // Only the multicarts write it, so a single-game state keeps its old layout.
        if self.multicart.is_some() {
            writer.write_u8(self.outer_raw);
        }
        writer.write_raw(&self.chr_latch);
        writer.write_bool(self.irq_enabled);
        writer.write_u8(self.irq_source);
        writer.write_u8(self.irq_direction);
        writer.write_bool(self.irq_small_prescaler);
        writer.write_bool(self.irq_c007_mode);
        writer.write_u8(self.irq_c007);
        writer.write_u8(self.irq_prescaler);
        writer.write_u8(self.irq_counter);
        writer.write_u8(self.irq_xor);
        writer.write_bool(self.irq_pending);
        writer.write_i32(i32::from(self.last_ppu_address));
        writer.write_u8(self.multiplicand);
        writer.write_u8(self.multiplier);
        writer.write_u8(self.accumulator);
        writer.write_u8(self.test_register);
        #[cfg(feature = "board-394")]
        if let Some(ext) = &self.ext_394 {
            writer.write_raw(&ext.regs);
            writer.write_u8(ext.a000);
            ext.mmc3.save_registers(writer);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr_low)?;
        reader.read_into(&mut self.chr_high)?;
        reader.read_into(&mut self.nametable_low)?;
        reader.read_into(&mut self.nametable_high)?;
        self.prg_mode = reader.read_u8()?;
        self.prg_last_switchable = reader.read_bool()?;
        self.chr_mode = reader.read_u8()?;
        self.rom_nametables = reader.read_bool()?;
        self.rom_nametables_all = reader.read_bool()?;
        self.prg_rom_at_6000 = reader.read_bool()?;
        self.arrangement = reader.read_u8()?;
        self.extended_mirroring = reader.read_bool()?;
        self.chr_write_enabled = reader.read_bool()?;
        self.nametable_select_invert = reader.read_bool()?;
        self.mmc4_mode = reader.read_bool()?;
        self.outer_chr_512k = reader.read_bool()?;
        self.outer_chr_bank = reader.read_u8()?;
        self.outer_prg_bank = reader.read_u8()?;
        self.outer_chr_a18 = reader.read_bool()?;
        if self.multicart.is_some() {
            self.outer_raw = reader.read_u8()?;
        }
        reader.read_into(&mut self.chr_latch)?;
        // Validated and not masked: a latch of 2 or more would index `chr_low` out of its eight
        // registers through `chr_4k_register`, so a corrupt block has to be refused rather than
        // quietly repaired. `Mmc2` gets this for free from `StateReader::read_index`; this board
        // keeps the two latches in the block as bytes, so the check is here.
        if self.chr_latch.iter().any(|latch| *latch > 1) {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: J.Y. ASIC CHR latches {:?} are not 0 or 1.",
                self.chr_latch
            )));
        }
        self.irq_enabled = reader.read_bool()?;
        self.irq_source = reader.read_u8()?;
        self.irq_direction = reader.read_u8()?;
        self.irq_small_prescaler = reader.read_bool()?;
        self.irq_c007_mode = reader.read_bool()?;
        self.irq_c007 = reader.read_u8()?;
        self.irq_prescaler = reader.read_u8()?;
        self.irq_counter = reader.read_u8()?;
        self.irq_xor = reader.read_u8()?;
        self.irq_pending = reader.read_bool()?;
        self.last_ppu_address = u16::try_from(reader.read_i32()? & 0xFFFF).unwrap_or(0);
        self.multiplicand = reader.read_u8()?;
        self.multiplier = reader.read_u8()?;
        self.accumulator = reader.read_u8()?;
        self.test_register = reader.read_u8()?;
        // 394: both chips' windows are rebuilt from their registers, whichever the block above
        // the registers was read into.
        #[cfg(feature = "board-394")]
        if let Some(ext) = self.ext_394.as_deref_mut() {
            reader.read_into(&mut ext.regs)?;
            ext.a000 = reader.read_u8()?;
            ext.mmc3.load_registers(reader)?;
            self.update_banks();
            self.apply_394();
        }
        // Derived, and therefore not in the block: MF-5a step 3's `level` lesson.
        self.refresh_nametable_pages();
        Ok(())
    }

    /// Every register is zero at power-on, so `$D001`'s arrangement 0 is the mirroring the board
    /// drives — the page's "Horizontal (A10)", which is [`NametableMirroring::Vertical`] here.
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        *self = Self::new(memory, self.mapper_number);
        memory.set_mirroring(NametableMirroring::Vertical);
    }

    /// `$D000` bit 7 clear: "Map WRAM, if present, otherwise open bus". Set: the window is an
    /// 8 KiB PRG-ROM bank, so the RAM is not readable there and [`Mapper::try_read_prg`] answers
    /// instead.
    fn prg_ram_enabled(&self) -> bool {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394() {
            return mmc3.prg_ram_enabled();
        }
        !self.prg_rom_at_6000
    }

    fn prg_ram_writable(&self) -> bool {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394() {
            return mmc3.prg_ram_writable();
        }
        self.prg_ram_enabled()
    }

    /// `$8000`-`$FFFF` through the window table, `$5000`-`$5FFF`'s readable registers, and
    /// `$6000`-`$7FFF` when `$D000` bit 7 maps PRG-ROM there.
    ///
    /// **This and not [`Mapper::cpu_read_override`]**, which `IrqBoard` does not forward: that
    /// dispatch is `#[inline(always)]` and on every CPU read of an MMC5 game, and a forward
    /// would give it a new body for the sake of a board that has no read side effect at all.
    /// `Cartridge::default_cpu_read` reaches this method for every address of `$4020`-`$FFFF`
    /// that is not readable PRG-RAM, which is exactly the three windows above.
    ///
    /// The order of the two `$5xxx` tests is the page's own: the solder pad register is
    /// "Mask: $FFFF" at exactly `$5000`, `$5400` and `$5C00` — "Apparently, the address of this
    /// register is fully decoded, as the SC-128 version of *Final Fight 3*, which otherwise
    /// happily randomizes the masked-off address bits, keeps A0-A11 constantly at zero to read
    /// from this register" — while the multiplier's mask is `$F803`, under which `$5C00` would
    /// also be `$5800`. Fully decoded wins for those three addresses.
    ///
    /// The solder pad answers **0**: `JJ.. ....` selects "different title screens or game counts
    /// on multicarts", a dump carries no record of how the pads were cut, and 0 is the setting
    /// Mesen 2 reports too.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394() {
            return mmc3.try_read_prg(memory, address);
        }
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        match address {
            0x5000 | 0x5400 | 0x5C00 => Some(0),
            0x5001..=0x5FFF => match address & 0xF803 {
                0x5800 => Some(u8::try_from(self.product() & 0x00FF).unwrap_or(0)),
                0x5801 => Some(u8::try_from(self.product() >> 8).unwrap_or(0)),
                0x5802 => Some(self.accumulator),
                0x5803 => Some(self.test_register),
                // Inside $5000-$5FFF but not a register of this chip: open bus.
                _ => None,
            },
            0x6000..=0x7FFF if self.prg_rom_at_6000 => {
                let rom = memory.rom().prg_rom();
                if rom.is_empty() {
                    return None;
                }

                // "if register $D000 bit 7 is set, $8003 also selects an 8 KiB PRG-ROM bank at
                // CPU $6000-$7FFF, which is $8003 in 8 KiB banking mode; $8003 SHL 1 OR 1 in
                // 16 KiB banking mode; $8003 SHL 2 OR 3 in 32 KiB banking mode."
                let bank = self.prg_bank(match self.prg_mode & 0x03 {
                    0 => self.prg_register(3) * 4 + 3,
                    1 => self.prg_register(3) * 2 + 1,
                    _ => self.prg_register(3),
                });
                let count = i32::try_from(rom.len() / Self::PRG_BANK_SIZE)
                    .unwrap_or(1)
                    .max(1);
                let base =
                    usize::try_from(bank.rem_euclid(count)).unwrap_or(0) * Self::PRG_BANK_SIZE;
                Some(rom[(base + usize::from(address - 0x6000)) % rom.len()])
            }
            _ => None,
        }
    }

    fn irq_asserted(&self) -> bool {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394() {
            return mmc3.irq_asserted();
        }
        self.irq_pending
    }

    /// `None`, and it is not a choice this board gets to make: [`super::IrqBoard`] answers `None`
    /// for the whole variant and does not forward this method, because MF-6b step 4 measured any
    /// forward at `NesBus::catch_up_ppu` 188 → 190 instructions and `mmc5` +9 214 379 over 300
    /// frames.
    ///
    /// It is also the honest answer for three of this board's four IRQ sources. `Some(i64::MAX)`
    /// means "nothing the PPU does can change the IRQ line", which holds only for source 0 (CPU
    /// M2) and source 3 (CPU writes); sources 1 and 2 are the PPU's own bus, so a board that
    /// could answer per-source would still have to answer `None` whenever a game selects one of
    /// them, and no bound at all is always correct. `jy_asic_ppu_irq_horizon_is_none_for_every_source`
    /// pins the property the census asks for.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        None
    }

    /// IRQ source 0, "CPU M2 rise" — the power-on setting of `$C001`, and the one the census
    /// calls "a CPU-cycle IRQ".
    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394_mut() {
            mmc3.on_cpu_cycle(memory);
            return;
        }
        let _ = memory;
        if self.irq_source == Self::SOURCE_CPU_M2 {
            self.tick_irq();
        }
    }

    /// IRQ sources 1 and 2 — "PPU A12 rise (unfiltered, eight per scanline)" and "PPU reads
    /// (170 per scanline)".
    ///
    /// **Unfiltered**, which is the one word that keeps this board away from
    /// [`A12Watcher`](crate::cartridges::A12Watcher): the MMC3's counter needs the filter to tell
    /// a real rise from the short dips of the fetch pipeline, and this one counts every rise the
    /// bus shows, eight a scanline instead of one.
    ///
    /// `last_ppu_address` is kept whatever the source is, so that selecting source 1 mid-frame
    /// does not read a stale edge — which is why all four numbers appear in
    /// `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`'s expected list.
    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394_mut() {
            mmc3.on_ppu_address_bus(memory, address, clock);
            return;
        }
        let _ = (memory, clock);
        let clock = match self.irq_source {
            // "2: PPU reads (170 per scanline)" — every fetch, whatever the address.
            Self::SOURCE_PPU_READ => true,
            // "1: PPU A12 rise (unfiltered, eight per scanline)".
            Self::SOURCE_PPU_A12 => address & 0x1000 != 0 && self.last_ppu_address & 0x1000 == 0,
            _ => false,
        };
        if clock {
            self.tick_irq();
        }

        self.last_ppu_address = address;
        self.move_chr_latch(address);
    }

    /// The MMC4-like latches, as an opaque pair: bit 0 is the low window's and bit 1 the high
    /// window's.
    ///
    /// The tile emitter reads this, walks the line forward with
    /// [`Mapper::advance_chr_latch`] so that each record carries the pattern the hardware would
    /// show at that column, then writes the saved value back (REQ-CORE-039) — the same contract
    /// the MMC2's own three methods keep.
    fn chr_latch_state(&self) -> i32 {
        i32::from(self.chr_latch[0] | (self.chr_latch[1] << 1))
    }

    fn set_chr_latch_state(&mut self, state: i32) {
        self.chr_latch = [
            u8::from(state & 0x01 != 0),
            u8::from((state >> 1) & 0x01 != 0),
        ];
        self.update_banks();
    }

    /// Moves the latch of `pattern_table` as the fetch of `tile` would.
    ///
    /// `$0FD8`-`$0FDF` is tile `$FD`'s second bit plane, all eight rows, and `$0FE8`-`$0FEF` is
    /// tile `$FE`'s — so the page's two address ranges *are* the MMC4's two tile numbers, which is
    /// why this body is `Mmc2::advance_chr_latch`'s.
    fn advance_chr_latch(&mut self, pattern_table: i32, tile: i32) {
        let window = usize::from(pattern_table & 0x1000 != 0);
        if tile == 0xFD {
            self.set_latch(window, 0);
        } else if tile == 0xFE {
            self.set_latch(window, 1);
        }
    }

    fn nametable_page(&self, table: i32) -> i32 {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394() {
            return mmc3.nametable_page(table);
        }
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 3]
    }

    /// A nametable byte out of CHR-ROM: "$B000/$B004 (LSB/MSB): Select 1 KiB CHR bank at PPU
    /// $2000-$23FF", and the three others the same way.
    ///
    /// The bank is the **whole sixteen bits** of the register pair, bit 7 of the LSB included —
    /// the page gives that bit the ROM/CIRAM select *and* leaves it in the bank number, and says
    /// nothing about masking the pair to the outer bank the way it does for `$900x`/`$A00x`.
    /// Mesen 2 reads it the same way.
    ///
    /// **This is the body Q-123 is about, and it costs nothing here**: it sits inside the
    /// `IrqBoard::read_nametable` forward mapper 68 already pays for, so `Board::read_nametable`
    /// does not grow an arm. MF-UNL-4 step 1 measured `Ppu2C02::read_nametable_inner` at **80
    /// instructions with and without a sixth body, identical to the instruction**;
    /// `rust/docs/PERFORMANCE-HISTORY.md` §5.18 has the figures and the rule for the next lot.
    ///
    /// Reached only behind a `SERVED_BY_CARTRIDGE` answer from [`Mapper::nametable_page`], so a
    /// cartridge of this board with its nametable registers idle never calls it.
    ///
    /// **There is no `write_nametable` beside it, and that is Q-152.** Mesen 2's own comment on
    /// the mechanism is "This behavior only affects reads, not writes" — the CIRAM byte behind the
    /// overlay is still written and still there — and this core cannot express that: one
    /// `nametable_page` answer routes both directions, and the board has no access to console
    /// VRAM. The write is dropped, which is right for the write and loses the byte underneath.
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        let table = usize::from(address >> 10) & 3;
        let mut bank =
            usize::from(self.nametable_low[table]) | (usize::from(self.nametable_high[table]) << 8);
        // puNES's `mirroring_swap_jyasic_28x` passes ROM nametables through the same outer bank.
        if let Some(multicart) = self.multicart {
            let placed = multicart.chr_bank(self.outer_raw, i32::try_from(bank).unwrap_or(0));
            bank = usize::try_from(placed).unwrap_or(0);
        }
        // `mirroring_swap_jyasic_394`: 394's outer CHR bank too.
        if let Some((_, chr_base)) = self.base_394() {
            bank = usize::try_from(chr_base).unwrap_or(0) | (bank & 0xFF);
        }
        memory.read_chr_direct(bank * Self::CHR_BANK_SIZE + (usize::from(address) & 0x03FF))
    }

    /// Writes CHR-RAM, gated by `$D002` bit 6.
    ///
    /// "CHR write-enable — 0: disabled, CHR memory is write-protected / 1: enabled", and
    /// "Bit 6 only matters when using CHR-RAM", which is why the test is only reached on a board
    /// with no CHR-ROM. **Power-on is write-protected**, the register being zero: a CHR-RAM image
    /// of one of these numbers has to ask for the RAM before it can fill it, which is what the
    /// register means. Mesen 2 does not decode this bit at all.
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        #[cfg(feature = "board-394")]
        if let Some(mmc3) = self.mmc3_394_mut() {
            mmc3.ppu_write(memory, address, value);
            return;
        }
        if memory.rom().chr_rom().is_empty() && self.chr_write_enabled {
            self.banks.write_chr(address, value, memory.chr_ram_mut());
        }
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        #[cfg(feature = "board-394")]
        if self.ext_394.is_some() {
            self.write_394(memory, address, value);
            return;
        }
        self.write_jy(memory, address, value);
    }
}

impl JyCompany {
    /// The ASIC's own register decode.
    fn write_jy(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            self.write_arithmetic_register(address, value);
            return;
        }

        match address & 0xF000 {
            // Mask $F803: "writes are ignored if address bit 11 is set".
            0x8000 => {
                if address & 0x0800 == 0 {
                    self.prg[usize::from(address) & 0x03] = value & 0x7F;
                }
            }
            // Mask $F807.
            0x9000 => {
                if address & 0x0800 == 0 {
                    self.chr_low[usize::from(address) & 0x07] = value;
                }
            }
            0xA000 => {
                if address & 0x0800 == 0 {
                    self.chr_high[usize::from(address) & 0x07] = value;
                }
            }
            0xB000 => {
                if address & 0x0800 == 0 {
                    let index = usize::from(address) & 0x03;
                    if address & 0x04 == 0 {
                        self.nametable_low[index] = value;
                    } else {
                        self.nametable_high[index] = value;
                    }
                }
            }
            // "Mask: $F007. Writes are **accepted** even if address bit 11 is set" — the one
            // window of this chip that does not test A11. No window moves either: the IRQ
            // registers are not bank registers.
            0xC000 => {
                self.write_irq_register(address, value);
                return;
            }
            // Mask $F803 on all four of $D000-$D003.
            0xD000 => {
                if address & 0x0800 != 0 {
                    return;
                }

                self.write_config_register(memory, address, value);
            }
            _ => return,
        }

        self.update_banks();
    }
}

/// The three J.Y. multicarts of lot MAPPERS-P: the ASIC of mappers 209/211 with `$D003` read as
/// a wider outer bank. Each decode is puNES's `prg_bank`/`chr_bank` of its file; nesdev has a
/// page for none of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JyMulticart {
    /// `mapper_281.c`: 256 KiB outer PRG banks (`$D003` from A18 up), 256 KiB outer CHR banks.
    M281,
    /// `mapper_282.c`: PRG `$D003` bits 1-7 from A18 up; CHR bit 0 at A18 and bits 3-4 at
    /// A19-A20, widened to 512 KiB (bits 3-4 only) when bit 5 is set.
    M282,
    /// `mapper_295.c`: 128 KiB outer PRG and CHR banks.
    M295,
    /// `mapper_358.c`: 282's PRG decode; CHR bit 0 at A18 and bits 2-3 at A19-A20, widened to
    /// 512 KiB when bit 5 is set.
    M358,
    /// `mapper_386.c`: PRG A18 from `$D003` bit 1 and A19 from bit 3; 358's CHR decode.
    M386,
    /// `mapper_387.c`: 386 with 128 KiB inner PRG banks.
    M387,
    /// `mapper_388.c`: PRG A18-A19 from `$D003` bits 2-3; CHR A18-A19 from bits 0-1, A18 alone
    /// in the 512 KiB mode. The board's jumper inhibits ROM nametables and Extended Mirroring
    /// (`init_JYASIC(FALSE, ...)`), as on mapper 90.
    M388,
    /// `mapper_397.c`: 282's PRG decode and 295's CHR decode.
    M397,
    /// `mapper_421.c`: PRG from `$D003` A18 up with a 512 KiB inner mask when bit 6 is set; CHR
    /// A18-A19 from bits 0-1 over a 512 KiB inner bank.
    M421,
}

impl JyMulticart {
    fn of(mapper: i32) -> Option<Self> {
        match mapper {
            281 => Some(Self::M281),
            282 => Some(Self::M282),
            295 => Some(Self::M295),
            358 => Some(Self::M358),
            386 => Some(Self::M386),
            387 => Some(Self::M387),
            388 => Some(Self::M388),
            397 => Some(Self::M397),
            421 => Some(Self::M421),
            _ => None,
        }
    }

    /// An 8 KiB PRG bank placed in the outer bank `outer` (`$D003`).
    fn prg_bank(self, outer: u8, bank_8k: i32) -> i32 {
        let outer = i32::from(outer);
        match self {
            Self::M281 => (outer << 5) | (bank_8k & 0x1F),
            Self::M282 | Self::M358 | Self::M397 => ((outer << 4) & !0x1F) | (bank_8k & 0x1F),
            Self::M295 => (outer << 4) | (bank_8k & 0x0F),
            Self::M386 => ((outer & 0x02) << 4) | ((outer & 0x08) << 3) | (bank_8k & 0x1F),
            Self::M387 => ((outer & 0x02) << 3) | ((outer & 0x08) << 2) | (bank_8k & 0x0F),
            Self::M388 => ((outer & 0x0C) << 3) | (bank_8k & 0x1F),
            Self::M421 => {
                let mask = if outer & 0x40 != 0 { 0x3F } else { 0x1F };
                ((outer << 4) & !mask) | (bank_8k & mask)
            }
        }
    }

    /// A 1 KiB CHR bank placed in the outer bank `outer` (`$D003`).
    fn chr_bank(self, outer: u8, bank_1k: i32) -> i32 {
        let outer = i32::from(outer);
        match self {
            Self::M281 => (outer << 8) | (bank_1k & 0xFF),
            Self::M388 => {
                let base = ((outer & 0x01) << 8) | ((outer & 0x02) << 8);
                if outer & 0x20 == 0 {
                    base | (bank_1k & 0xFF)
                } else {
                    (base & 0x0200) | (bank_1k & 0x1FF)
                }
            }
            Self::M282 | Self::M358 | Self::M386 | Self::M387 => {
                let base = if self == Self::M282 {
                    ((outer & 0x01) << 8) | ((outer & 0x18) << 6)
                } else {
                    ((outer & 0x01) << 8) | ((outer & 0x0C) << 7)
                };
                if outer & 0x20 == 0 {
                    base | (bank_1k & 0xFF)
                } else {
                    (base & 0x0600) | (bank_1k & 0x1FF)
                }
            }
            Self::M295 | Self::M397 => (outer << 7) | (bank_1k & 0x7F),
            Self::M421 => ((outer & 0x03) << 8) | (bank_1k & 0x1FF),
        }
    }
}

/// Lot MAPPERS-P: mapper 394's board (`mapper_394.c`), which wires the J.Y. ASIC and an MMC3
/// to the same ROMs and picks one with `$5001` D4; both see the outer bank of its four
/// registers.
#[cfg(feature = "board-394")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Jy394 {
    /// `m394.reg`, `$5000-$5003` by A0-A1; power-on `$5001` = `$0F`, `$5003` = `$90`.
    regs: [u8; 4],
    /// The last value written to the MMC3's `$A000`, which a switch back to the MMC3 re-applies
    /// (the chip keeps no copy here).
    a000: u8,
    submapper: u8,
    mmc3: Mmc3,
}

#[cfg(feature = "board-394")]
impl Jy394 {
    fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        Self {
            regs: [0, 0x0F, 0, 0x90],
            a000: 0,
            submapper: if header.is_nes20 {
                u8::try_from(header.submapper & 0x0F).unwrap_or(0)
            } else {
                0
            },
            mmc3: Mmc3::new(memory, 394),
        }
    }

    /// `prg_base()` (8 KiB banks) and `chr_base()` (1 KiB banks): submapper 1 takes CHR A17-A18
    /// from `$5001` D1-D2.
    fn bases(&self) -> (i32, i32) {
        let r = self.regs.map(i32::from);
        let prg = ((r[3] & 0x08) << 1) | ((r[1] & 0x03) << 5);
        let chr = if self.submapper == 1 {
            ((r[3] & 0x40) << 1) | ((r[1] & 0x02) << 8) | ((r[1] & 0x04) << 6)
        } else {
            ((r[3] & 0x40) << 1) | ((r[1] & 0x01) << 8)
        };
        (prg, chr)
    }
}
