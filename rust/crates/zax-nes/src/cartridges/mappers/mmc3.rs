// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// `clock_irq_counter`'s revision A condition is Mesen's `(count > 0 || _irqReload)`
// (`Core/NES/Mappers/Nintendo/MMC3.h`, `NotifyVramAddressChange`); see THIRD-PARTY-NOTICES.md.

use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers 4 (MMC3 and MMC6) and 118 (TxSROM) (REQ-CORE-100, REQ-CORE-114,
/// REQ-CORE-102).
///
/// Eight bank registers behind $8000/$8001, an A12-clocked scanline counter behind
/// $C000-$E001 (nesdev, "MMC3"), PRG-RAM protection at $A001. MMC6 (NES 2.0 submapper 1)
/// has 1 KiB of RAM in two halves with their own enable bits. **TxSROM** (TKSROM and TLSROM,
/// mapper 118) is the same chip with CHR A17 wired to CIRAM A10, so bit 7 of the CHR bank
/// registers picks the nametable; see [`Mmc3Board::refresh_nametable_pages`].
///
/// **`MC_ACC` is Acclaim's clone, and it is a type parameter and not a field** (MR-4-3). nesdev
/// ("MMC3"): "Acclaim's MC-ACC chip is their own variant of the MMC3 ... The only known
/// difference is that the scanline counter uses a ÷8 prescaler instead of filtering A12, clocked
/// by falling edges", and *NES 2.0 submappers* gives mapper 4 "Submapper 3: MC-ACC chip variant".
/// Mesen 2 builds a separate board for it (`MapperFactory.cpp:285-290` → `McAcc`), and so does
/// this — but a separate *monomorphisation* of the same code and not a second copy of it:
/// [`Mmc3`] is `Mmc3Board<false>`, [`McAcc`] is `Mmc3Board<true>`, they share every method body,
/// and the four places the clone differs are `if MC_ACC` branches a `const` parameter folds away.
/// The plain MMC3 therefore comes out of the compiler **instruction for instruction as it was**,
/// which a field could not have given it: `on_ppu_address_bus` runs on every PPU fetch, where
/// comparing `address & $1000` against a field instead of against zero would have cost two
/// instructions, and MR-4-3's own conclusion was that "no MMC3 game pays a branch".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Board<const MC_ACC: bool> {
    banks: BankWindows,
    a12: A12Watcher,
    bank: [u8; 8],
    /// 4 or 118, for [`Mapper::mapper_number`].
    mapper_number: i32,
    is_mmc6: bool,
    /// TxSROM: the board drives CIRAM A10 itself, so bit 7 of the CHR registers is read for
    /// the nametables (mapper 118).
    is_txsrom: bool,
    /// What [`Mapper::nametable_page`] answers for the four logical tables.
    ///
    /// A plain MMC3 holds `FOLLOW_MIRRORING` here from one end of a run to the other, which is
    /// exactly what the trait's default answers; a TxSROM holds the pages bit 7 of its CHR
    /// registers names. A field and not a `match`, for the reason `UnRom512::nametable_answer`
    /// gives: `Board::nametable_page` has to stay one load per arm. Derived state —
    /// `load_registers` recomputes it and no savestate byte moved (REQ-CORE-091).
    nametable_pages: [i32; 4],
    /// $A000 is ignored: a four-screen header, NES 2.0 submapper 2 ("MMC3C with hard-wired
    /// mirroring", MR-4-9), or a TxSROM, whose mirroring register "is bypassed by the
    /// configuration described above, so writing here has no effect" (nesdev, "INES Mapper 118").
    ignore_a000: bool,
    /// NES 2.0 submapper 4, the NEC / Sharp MMC3A, whose IRQ counter is **revision A**
    /// (MR-4-9, AMD-061 §8 closing Q-114). Chosen at load; every other mapper-4 image, iNES
    /// 1.0 included, keeps revision B and with it REQ-CORE-114's `mmc3_test_2` test 6.
    irq_revision_a: bool,
    /// MR-4-3: A12 falls left before the next clock of the scanline counter, counting down from
    /// 7. Only `MC_ACC` reads it, and only `MC_ACC` writes it to the savestate.
    irq_prescaler: u8,
    select: u8,
    protect: u8,
    irq_latch: u8,
    irq_counter: i32,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
}

/// The MMC3 and the MMC6, mapper 4 and mapper 118 (`Mmc3Board<false>`).
pub type Mmc3 = Mmc3Board<false>;

/// The Acclaim MC-ACC, mapper 4 submapper 3 (`Mmc3Board<true>`).
pub type McAcc = Mmc3Board<true>;

impl<const MC_ACC: bool> Mmc3Board<MC_ACC> {
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let header = memory.rom().header();
        let is_txsrom = mapper_number == 118;
        let is_mmc6 = !is_txsrom && header.is_nes20 && header.submapper == 1;
        // The MC-ACC filters nothing and counts the other edge, so it gets a watcher that only
        // tracks A12's level; `on_ppu_address_bus` calls `fell` on it instead of `rose`.
        let a12 = if MC_ACC {
            A12Watcher::new(0)
        } else {
            A12Watcher::new(A12Watcher::MMC3_FILTER_DOTS)
        };
        Self {
            banks: windows_for(memory),
            a12,
            bank: [0; 8],
            mapper_number,
            is_mmc6,
            is_txsrom,
            nametable_pages: [super::super::Cartridge::FOLLOW_MIRRORING; 4],
            // MR-4-9, nesdev ("NES 2.0 submappers"), mapper 4: "Submapper 2: MMC3C with
            // hard-wired mirroring" — the same condition as a four-screen header, so the
            // $A000 writes are ignored on both. A TxSROM ignores it too, for its own reason.
            ignore_a000: is_txsrom
                || header.mirroring == NametableMirroring::FourScreen
                || (header.is_nes20 && header.submapper == 2),
            // AMD-061 §8: "le sous-mappeur NES 2.0 d'une image l'emporte sur la révision par
            // défaut". Submapper 4 is "NEC MMC3 (Loading latch with 0 disables IRQ)" (nesdev,
            // "NES 2.0 submappers"), which is the revision A counter.
            irq_revision_a: !is_txsrom && header.is_nes20 && header.submapper == 4,
            irq_prescaler: 0,
            select: 0,
            protect: Self::power_on_protect(is_mmc6),
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    /// The power-on $A001.
    ///
    /// MR-4-5: the MMC3's is not documented, and enabled-and-writable is the only value that
    /// lets a game, or blargg's MMC3 test ROMs which never write $A001, use the RAM out of a
    /// cold start. The MMC6's *is* documented: $8000 powers on at 0, so the RAM is disabled,
    /// so the chip holds $A001 at $00 (MR-4-7) and no half is readable or writable yet.
    const fn power_on_protect(is_mmc6: bool) -> u8 {
        if is_mmc6 {
            0x00
        } else {
            0x80
        }
    }

    /// MR-4-6: the MMC6's own PRG-RAM, "1 KB PRG RAM, mirrored" over $7000-$7FFF. The size is
    /// settled at load by `registry::board_memory_sizes`, so the address mask alone is the
    /// offset and no division is left on the access path.
    pub(crate) const MMC6_PRG_RAM_SIZE: usize = 0x0400;

    fn mmc6_offset(address: u16) -> usize {
        usize::from(address & 0x03FF)
    }

    /// $A001 of an MMC6 is `HhLl xxxx` (nesdev, "MMC6"): bit 4 enables writing and bit 5
    /// reading the $7000-$71FF half, bit 6 writing and bit 7 reading the $7200-$73FF half
    /// (MR-4-1 — we had each pair the other way round).
    fn mmc6_half_readable(&self, address: u16) -> bool {
        self.protect & if address & 0x0200 != 0 { 0x80 } else { 0x20 } != 0
    }

    fn mmc6_half_writable(&self, address: u16) -> bool {
        self.protect & if address & 0x0200 != 0 { 0x40 } else { 0x10 } != 0
    }

    /// Whether either half answers a read, which is what decides between a value and open bus
    /// on this window (MR-4-8).
    const fn mmc6_any_half_readable(&self) -> bool {
        self.protect & 0xA0 != 0
    }

    /// The TxSROM's nametables (mapper 118).
    ///
    /// nesdev ("INES Mapper 118"): the board wires CHR A17 — bit 7 of the bank number — to CIRAM
    /// A10, so "the CHR bankswitching directly affects the mirroring mapping the nametable RAM".
    /// It is the registers mapped to **the first pattern table** that drive it, which bit 7 of
    /// $8000 chooses between: with it clear the two 2 KiB registers R0 and R1 are at PPU
    /// $0000-$0FFF and take two nametables each, and with it set the four 1 KiB registers R2-R5
    /// are there and take one each. Mesen 2 reads the register the same way
    /// (`TxSRom.h:16-39`), and a TxSROM write to $A000 does nothing at all.
    ///
    /// A plain MMC3 never comes here: its four pages stay `FOLLOW_MIRRORING`, so
    /// `Mapper::nametable_page` answers what the trait's default answers and `$A000` keeps
    /// driving `memory.mirroring()` as before.
    fn refresh_nametable_pages(&mut self) {
        if !self.is_txsrom {
            return;
        }

        let page = |register: usize| i32::from(self.bank[register] >> 7);
        self.nametable_pages = if self.select & 0x80 == 0 {
            [page(0), page(0), page(1), page(1)]
        } else {
            [page(2), page(3), page(4), page(5)]
        };
    }

    fn update_banks(&mut self) {
        self.refresh_nametable_pages();

        if self.select & 0x40 != 0 {
            self.banks.map_prg_8k(0, -2);
            self.banks.map_prg_8k(2, i32::from(self.bank[6]));
        } else {
            self.banks.map_prg_8k(0, i32::from(self.bank[6]));
            self.banks.map_prg_8k(2, -2);
        }

        self.banks.map_prg_8k(1, i32::from(self.bank[7]));
        self.banks.map_prg_8k(3, -1);

        let high = if self.select & 0x80 != 0 { 4 } else { 0 };
        let low = 4 - high;
        self.banks.map_chr_2k(high, i32::from(self.bank[0] >> 1));
        self.banks
            .map_chr_2k(high + 2, i32::from(self.bank[1] >> 1));
        for i in 0..4 {
            self.banks.map_chr_1k(low + i, i32::from(self.bank[2 + i]));
        }
    }

    /// Clocks the scanline counter on an A12 rise, and asserts the line when it lands on zero.
    ///
    /// **The two revisions (AMD-061 §8, Q-114).** Revision B — every mapper-4 image but a
    /// declared submapper 4 — asserts whenever the counter is zero after the clock, so a latch
    /// of $00 gives an IRQ on *every* rise; that is what REQ-CORE-114 and `mmc3_test_2` test 6
    /// require and it is unchanged. Revision A also demands that the counter was **moving**:
    /// non-zero before the clock, or a `$C001` reload pending. A latch of $00 then gives exactly
    /// one IRQ — the pending reload lets the first clock through and nothing lets the next one —
    /// which is §8's criterion.
    fn clock_irq_counter(&mut self) {
        let before = self.irq_counter;
        let reloading = self.irq_reload;
        if before == 0 || reloading {
            self.irq_counter = i32::from(self.irq_latch);
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }

        let moving = !self.irq_revision_a || before > 0 || reloading;
        if moving && self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }
}

impl<const MC_ACC: bool> Mapper for Mmc3Board<MC_ACC> {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.bank);
        writer.write_u8(self.select);
        writer.write_u8(self.protect);
        writer.write_u8(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_bool(self.irq_reload);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        self.a12.save_state(writer);
        if MC_ACC {
            writer.write_u8(self.irq_prescaler);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.bank)?;
        self.select = reader.read_u8()?;
        self.protect = reader.read_u8()?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_i32()?;
        self.irq_reload = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.a12.load_state(reader)?;
        if MC_ACC {
            self.irq_prescaler = reader.read_u8()?;
        }

        // The windows again from the registers, over the PRG-ROM loaded: a state of the same
        // game made with a PRG-ROM of another size (a widescreen hack that added banks, its
        // fixed ones moved to the end) maps the fixed banks of this one. It brings the TxSROM's
        // nametable pages back with them, which are derived from the same registers.
        self.update_banks();
        Ok(())
    }

    fn chr_view<'m>(
        &self,
        _memory: &'m crate::cartridges::CartridgeMemory,
    ) -> Option<crate::cartridges::ChrView<'m>> {
        // Its fetches have side effects or another source: the PPU takes the full path.
        None
    }

    fn chr_peek_view<'m>(
        &self,
        memory: &'m crate::cartridges::CartridgeMemory,
    ) -> Option<crate::cartridges::ChrView<'m>> {
        // Its peek is the default one, through its bank windows: only the fetches clock the
        // IRQ counter.
        let chr_rom = memory.rom().chr_rom();
        let chr = if chr_rom.is_empty() {
            memory.chr_ram()
        } else {
            chr_rom
        };
        self.banks()
            .map(|banks| crate::cartridges::ChrView::new(chr, banks))
    }

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    /// `FOLLOW_MIRRORING` on a plain MMC3, and the TxSROM's own pages on mapper 118
    /// ([`Mmc3::refresh_nametable_pages`]). `Cartridge` caches the four answers, and a
    /// mapper-118 image is the second one — after the MMC5 — whose pages a register write can
    /// move, so `Cartridge::nametable_pages_vary` names it too.
    fn nametable_page(&self, table: i32) -> i32 {
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 3]
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// The line only rises when the counter reaches 0 with IRQs on, and the counter only
    /// moves on an A12 rise, which come at least `min_rise_interval` dots apart.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        if self.irq_pending || !self.irq_enabled {
            return Some(i64::MAX);
        }

        let clocks_to_zero = if self.irq_counter == 0 || self.irq_reload {
            1 + i64::from(self.irq_latch)
        } else {
            i64::from(self.irq_counter)
        };
        // MR-4-3's warning: eight falls per clock is not `A12Watcher::min_rise_interval`, and a
        // horizon copied from the MMC3's would make the PPU late. A fall needs two dots — one
        // high and one low — so eight dots per clock is a safe floor, which is what a horizon
        // has to be.
        let dots_per_clock = if MC_ACC {
            8
        } else {
            self.a12.min_rise_interval()
        };
        Some((clocks_to_zero - 1) * dots_per_clock)
    }

    fn prg_ram_enabled(&self) -> bool {
        if self.is_mmc6 {
            self.select & 0x20 != 0
        } else {
            self.protect & 0x80 != 0
        }
    }

    /// The MMC3's reading of $A001: bit 7 enables the RAM, bit 6 write-protects it. An MMC6
    /// never gets here — it decodes the whole of $6000-$7FFF itself in `intercept_cpu_write`,
    /// half by half, so the generic PRG-RAM path is unreachable on that chip — which is why
    /// bit 6 keeps its MMC3 meaning here and not the MMC6's "enable writing the high half".
    fn prg_ram_writable(&self) -> bool {
        self.prg_ram_enabled() && self.protect & 0x40 == 0
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = [0; 8];
        self.bank[7] = 1;
        self.select = 0;
        self.protect = Self::power_on_protect(self.is_mmc6);
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_reload = false;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.irq_prescaler = 0;
        self.a12.reset();
        self.update_banks();
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if !self.is_mmc6 || !(0x6000..=0x7FFF).contains(&address) {
            return None;
        }

        // $6000-$6FFF is not decoded on this board, and MR-4-8, nesdev ("MMC6"): "If neither
        // bank is enabled for reading, the $7000-$7FFF area is open bus." The RAM being
        // disabled through $8000 holds $A001 at zero (MR-4-7), so it is the same case.
        if address < 0x7000 || !self.prg_ram_enabled() || !self.mmc6_any_half_readable() {
            return Some(None);
        }

        // "If only one bank is enabled for reading, the other reads back as zero."
        Some(Some(if self.mmc6_half_readable(address) {
            memory.prg_ram()[Self::mmc6_offset(address)]
        } else {
            0
        }))
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if !self.is_mmc6 || !(0x6000..=0x7FFF).contains(&address) {
            return false;
        }

        if address >= 0x7000 && self.prg_ram_enabled() && self.mmc6_half_writable(address) {
            let offset = Self::mmc6_offset(address);
            memory.prg_ram_mut()[offset] = value;
        }

        true
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        match address & 0xE001 {
            0x8000 => {
                self.select = value;
                self.update_banks();
            }
            0x8001 => {
                self.bank[usize::from(self.select & 0x07)] = value;
                self.update_banks();
            }
            0xA000 => {
                if !self.ignore_a000 {
                    memory.set_mirroring(if value & 0x01 != 0 {
                        NametableMirroring::Horizontal
                    } else {
                        NametableMirroring::Vertical
                    });
                }
            }
            // MR-4-7, nesdev ("MMC6"): "When PRG RAM is disabled via $8000, the mapper
            // continuously sets $A001 to $00, and so all writes to $A001 are ignored." The
            // MMC3 proper has no such gate.
            0xA001 => {
                if !self.is_mmc6 || self.select & 0x20 != 0 {
                    self.protect = value;
                }
            }
            0xC000 => self.irq_latch = value,
            0xC001 => {
                self.irq_counter = 0;
                self.irq_reload = true;
                // MR-4-3: on an MC-ACC "writing to $C001 resets pulse counter", which is how a
                // game puts the ÷8 prescaler back in phase.
                if MC_ACC {
                    self.irq_prescaler = 0;
                }
            }
            0xE000 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            _ => self.irq_enabled = true,
        }
    }

    /// Clocks the scanline counter on the edge this board counts: an A12 rise past the filter on
    /// an MMC3, and every eighth A12 fall on an MC-ACC (MR-4-3). Both arms of each `if MC_ACC`
    /// are compile-time dead on one of the two boards, so the MMC3 keeps exactly the two lines it
    /// had.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        let edge = if MC_ACC {
            self.a12.fell(address)
        } else {
            self.a12.rose(address, clock)
        };
        if !edge {
            return;
        }

        if MC_ACC {
            // "Counter clocking happens once per 8 A12 cycles at first cycle": the first fall of
            // each group of eight, which is the phase a `$C001` write restarts from.
            if self.irq_prescaler == 0 {
                self.irq_prescaler = 7;
                self.clock_irq_counter();
            } else {
                self.irq_prescaler -= 1;
            }
        } else {
            self.clock_irq_counter();
        }
    }
}
