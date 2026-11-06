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
/// [`Mmc3`] is `Mmc3Board<false, _, _>`, [`McAcc`] is `Mmc3Board<true, _, _>`, they share every
/// method body, and the four places the clone differs are `if MC_ACC` branches a `const`
/// parameter folds away.
/// The plain MMC3 therefore comes out of the compiler **instruction for instruction as it was**,
/// which a field could not have given it: `on_ppu_address_bus` runs on every PPU fetch, where
/// comparing `address & $1000` against a field instead of against zero would have cost two
/// instructions, and MR-4-3's own conclusion was that "no MMC3 game pays a branch".
///
/// **`TQROM` is mapper 119 and `MULTICART` is mappers 47 and 37**, each a `const` parameter for
/// the same reason and each behind a Cargo feature (`board-119`, `board-mmc3-multicart`). MF-6a2
/// step 3 built all three, measured **six** shapes and reverted them on that lot's stop-the-lot
/// condition; the owner's tolerance of 2026-11-06 accepts their cost and MF-6a3 rebuilt them
/// from its write-up. The shape here is that step's **C**, which was the best of the six:
/// two `const` parameters (so mapper 4 and 118 keep their own monomorphisation), the TQROM's
/// `ppu_peek`/`ppu_write` bodies `#[inline(never)]`, one `update_banks` body whose outer-bank
/// arithmetic is compile-time dead off the multicart, and the outer register's decode out of
/// line. Shape **A** — the same two parameters with everything inlined — took
/// `Board::ppu_peek` from 271 instructions to **583** and cost `m4` **3.4 M over 300 frames**,
/// and `#[inline(never)]` on the one body recovered 8.0 M of set total; shape **D**, one
/// parameter and one `Board` variant for all three numbers, was *worse* on `m4` (+750 676
/// against +258 640).
///
/// Four monomorphisations are instantiated and no other: `Mmc3Board<false, false, false>`
/// ([`Mmc3`], mappers 4 and 118), `<true, false, false>` ([`McAcc`]), `<false, true, false>`
/// ([`Tqrom`]) and `<false, false, true>` ([`Mmc3Multicart`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Board<const MC_ACC: bool, const TQROM: bool, const MULTICART: bool> {
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
    /// TQROM (mapper 119): one bit per 1 KiB CHR window, **set when that window reads the
    /// CHR-RAM chip** rather than the CHR-ROM. Derived state — refreshed by `update_banks` when
    /// a register commits, and brought back by `load_registers` through the same call, so no
    /// savestate byte carries it (MF-5a step 3's `level` lesson, REQ-CORE-091).
    ///
    /// MF-6a2 step 3's design records "the eight windows' source bytes in a `[u8; 8]`", for the
    /// reason [`Mmc3Board::nametable_pages`] already gives: [`Mapper::ppu_peek`] is per fetch
    /// and must not re-derive the CHR mode there. **One `u8` of bit flags carries the same eight
    /// answers in one byte**, which is MF-6b step 1's finding applied to that design — "a field
    /// offset is a cost even when no code changes", measured at +9 886 794 on PR-1's set for one
    /// byte of displacement on `VrcIrq::pending`, and that step postdates MF-6a2's. A single
    /// byte lands in the padding this struct already has where eight could not.
    #[cfg(feature = "board-119")]
    chr_ram_windows: u8,
    /// Mappers 47 and 37: the outer register at $6000-$7FFF, three bits wide.
    ///
    /// 047 is `[.... ...B]` and 037 `[.... .QBB]`, so bits 3-7 are not latched by either — which
    /// `req_core_100_mapper_37_reproduces_its_outer_bank_table` pins.
    #[cfg(feature = "board-mmc3-multicart")]
    outer: u8,
    /// PPU dots one clock of the scanline counter needs, which is what
    /// [`Mapper::ppu_irq_horizon`] multiplies the clocks-to-zero by: the A12 watcher's
    /// `min_rise_interval` on an MMC3, and 8 on an MC-ACC, whose ÷8 prescaler needs two dots
    /// per fall (MR-4-3's warning).
    ///
    /// A **field** settled in `new`, and not the `if MC_ACC` it was, for a reason MF-6a2 step 2
    /// measured: with the `if`, the two monomorphisations' `ppu_irq_horizon` are two distinct
    /// arm bodies in the `Board::ppu_irq_horizon` dispatch that `NesBus::catch_up_ppu` inlines,
    /// and a body there is worth 0.005 % to 0.082 % to **every** board in the library. With the
    /// field they are one body, and the MC-ACC's `shlq $3` is gone from `catch_up_ppu`. Wiring
    /// and not state: `reset` leaves it and no savestate byte carries it.
    dots_per_clock: i64,
    select: u8,
    protect: u8,
    irq_latch: u8,
    irq_counter: i32,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
}

/// The MMC3 and the MMC6, mapper 4 and mapper 118 (`Mmc3Board<false, false, false>`).
pub type Mmc3 = Mmc3Board<false, false, false>;

/// The Acclaim MC-ACC, mapper 4 submapper 3 (`Mmc3Board<true, false, false>`).
pub type McAcc = Mmc3Board<true, false, false>;

/// The Nintendo TQROM, mapper 119: the MMC3 with a CHR-RAM chip beside its CHR-ROM, bit 6 of
/// the CHR bank number choosing between them (`Mmc3Board<false, true, false>`).
///
/// nesdev ("INES Mapper 119"): "Bit 6 of the CHR bank number is used to switch between the CHR
/// ROM chip and the CHR RAM chip", the board doing it with "a 74HC32 IC" whose output "controls
/// the CHR ROM's enable line. PPU A13 is ORed with CHR A16 in order to create a /CE signal for
/// the CHR ROM", and "Both TQROM games used 8 KiB CHR RAM". The two titles are *High Speed* and
/// *Pin\*Bot*.
#[cfg(feature = "board-119")]
pub type Tqrom = Mmc3Board<false, true, false>;

/// Nintendo's two MMC3 multicarts, mappers 47 and 37: an MMC3 whose PRG and CHR bank numbers
/// carry one or two more address lines from an outer register at $6000-$7FFF
/// (`Mmc3Board<false, false, true>`).
///
/// 47 is *Super Spike V'Ball + Nintendo World Cup*, 37 *Super Mario Bros. + Tetris + Nintendo
/// World Cup* (PAL). See [`Mmc3Board::multicart_prg`] and [`Mmc3Board::multicart_chr`] for the
/// two pages' own wiring, and [`Mmc3Board::multicart_outer_write`] for the condition both of
/// them put on the register.
#[cfg(feature = "board-mmc3-multicart")]
pub type Mmc3Multicart = Mmc3Board<false, false, true>;

impl<const MC_ACC: bool, const TQROM: bool, const MULTICART: bool>
    Mmc3Board<MC_ACC, TQROM, MULTICART>
{
    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let header = memory.rom().header();
        let is_txsrom = mapper_number == 118;
        // **`mapper_number == 4` and not `!is_txsrom`**, which is the same condition for mappers
        // 4 and 118 and a different one for 119, 47 and 37. nesdev's *NES 2.0 submappers*
        // defines submappers 1-4 for mapper 4 and **none** for 118, 119, 47 or 37, so a NES 2.0
        // dump of a TQROM carrying a submapper byte would otherwise have been read as an MMC6
        // or an MMC3A. The same tightening is on `ignore_a000` and `irq_revision_a` below.
        let is_mmc6 = mapper_number == 4 && header.is_nes20 && header.submapper == 1;
        // The MC-ACC filters nothing and counts the other edge, so it gets a watcher that only
        // tracks A12's level; `on_ppu_address_bus` calls `fell` on it instead of `rose`.
        let a12 = if MC_ACC {
            A12Watcher::new(0)
        } else {
            A12Watcher::new(A12Watcher::MMC3_FILTER_DOTS)
        };
        // MR-4-3's warning: eight falls per clock is not `A12Watcher::min_rise_interval`, and a
        // horizon copied from the MMC3's would make the PPU late. A fall needs two dots — one
        // high and one low — so eight dots per clock is a safe floor, which is what a horizon
        // has to be.
        let dots_per_clock = if MC_ACC { 8 } else { a12.min_rise_interval() };
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
                || (mapper_number == 4 && header.is_nes20 && header.submapper == 2),
            // AMD-061 §8: "le sous-mappeur NES 2.0 d'une image l'emporte sur la révision par
            // défaut". Submapper 4 is "NEC MMC3 (Loading latch with 0 disables IRQ)" (nesdev,
            // "NES 2.0 submappers"), which is the revision A counter.
            irq_revision_a: mapper_number == 4 && header.is_nes20 && header.submapper == 4,
            irq_prescaler: 0,
            #[cfg(feature = "board-119")]
            chr_ram_windows: 0,
            #[cfg(feature = "board-mmc3-multicart")]
            outer: 0,
            dots_per_clock,
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

    /// The TQROM's own CHR-RAM chip (mapper 119): nesdev, "Both TQROM games used 8 KiB CHR
    /// RAM". Settled at load by `registry::board_memory_sizes`, because no header field can
    /// state it — see that arm.
    #[cfg(feature = "board-119")]
    pub(crate) const TQROM_CHR_RAM_SIZE: usize = 0x2000;

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

    /// One 8 KiB PRG window, with the multicart's outer bits folded in.
    ///
    /// Off the multicart this is `self.banks.map_prg_8k(slot, bank)` and nothing else — `if
    /// MULTICART` is compile-time dead, and with `board-mmc3-multicart` off the statement is not
    /// even compiled — which is what keeps `Mmc3Board::update_banks` the body it was for mappers
    /// 4 and 118.
    #[inline]
    fn set_prg_8k(&mut self, slot: usize, bank: i32) {
        #[cfg(feature = "board-mmc3-multicart")]
        let bank = if MULTICART {
            self.multicart_prg(bank)
        } else {
            bank
        };
        self.banks.map_prg_8k(slot, bank);
    }

    /// One 2 KiB CHR window pair; see [`Mmc3Board::set_prg_8k`] on why the `if` is free.
    ///
    /// On the multicart the outer bit is a line of the **1 KiB** bank number, so the pair is
    /// expanded here rather than in [`BankWindows::map_chr_2k`].
    #[inline]
    fn set_chr_2k(&mut self, slot: usize, bank: i32) {
        #[cfg(feature = "board-mmc3-multicart")]
        if MULTICART {
            self.set_chr_1k(slot, bank.wrapping_mul(2));
            self.set_chr_1k(slot + 1, bank.wrapping_mul(2).wrapping_add(1));
            return;
        }

        self.banks.map_chr_2k(slot, bank);
    }

    /// One 1 KiB CHR window; see [`Mmc3Board::set_prg_8k`] on why the `if` is free.
    #[inline]
    fn set_chr_1k(&mut self, slot: usize, bank: i32) {
        #[cfg(feature = "board-mmc3-multicart")]
        let bank = if MULTICART {
            self.multicart_chr(bank)
        } else {
            bank
        };
        self.banks.map_chr_1k(slot, bank);
    }

    /// What the outer register at $6000-$7FFF does to an MMC3 8 KiB PRG bank number on the two
    /// multicarts.
    ///
    /// **Mapper 47.** nesdev ("INES Mapper 047"): $6000-$7FFF is `[.... ...B]`, "each block has
    /// 128k PRG and 128k CHR", and $8000-$FFFF is the "Same as MMC3 for selected block". So `B`
    /// is PRG A17 above the MMC3's four bits of 8 KiB bank, and the fixed `-1` and `-2` windows
    /// become the **block's** last two rather than the image's — which is what masking to the
    /// block's four bits before adding `B` does, `-2 & $0F` being 14 and `-1 & $0F` 15. Mesen 2
    /// reads it identically (`MMC3_47.h`).
    ///
    /// **Mapper 37, and the one place the page and Mesen 2 disagree.** nesdev ("INES Mapper
    /// 037") latches `[.... .QBB]` and gives the wiring rather than a mask: "PRG ROM A16 =
    /// NAND(NAND(Q0,Q1),NAND(Q2,M16))" with "PRG ROM A17 ... driven by Q2", `M16` being the
    /// MMC3's own A16 — bit 3 of an 8 KiB bank number. That NAND of two NANDs is
    ///
    /// ```text
    /// A16 = (Q0 AND Q1) OR (Q2 AND M16),   A17 = Q2
    /// ```
    ///
    /// and **one expression reproduces the page's whole four-row table**, the two 64 KiB windows
    /// and the two 128 KiB ones:
    ///
    /// | written | A17 | A16 | PRG window | the page's "PRG bank size" |
    /// | ---: | --- | --- | --- | --- |
    /// | 0, 1, 2 | 0 | 0 | $00000-$0FFFF | 64 KiB |
    /// | 3 | 0 | **1**, forced | $10000-$1FFFF | 64 KiB |
    /// | 4, 5, 6 | 1 | the MMC3's | $20000-$3FFFF | 128 KiB |
    /// | 7 | 1 | **1**, forced | $30000-$3FFFF | 64 KiB |
    ///
    /// **Mesen 2 differs on the last row and AMD-061 §1 settles it for the page**: `MMC3_37.h`
    /// gives block 7 `page &= 0x07; page |= 0x20`, which sets A18 and leaves A16 clear —
    /// $20000-$27FFF — where the page's table says $30000-$3FFFF and its own formula says
    /// A16 = 1 because `Q0 AND Q1` is set.
    #[cfg(feature = "board-mmc3-multicart")]
    fn multicart_prg(&self, bank: i32) -> i32 {
        // The MMC3's own four bits: one 128 KiB block of 8 KiB banks, a negative "last bank"
        // counted inside the block rather than inside the image.
        let inner = bank & 0x0F;
        if self.mapper_number == 47 {
            return inner | (i32::from(self.outer & 0x01) << 4);
        }

        let q = i32::from(self.outer);
        let q2 = (q >> 2) & 1;
        let m16 = (inner >> 3) & 1;
        let a16 = ((q & 1) & ((q >> 1) & 1)) | (q2 & m16);
        (inner & 0x07) | (a16 << 3) | (q2 << 4)
    }

    /// What the outer register does to an MMC3 1 KiB CHR bank number on the two multicarts.
    ///
    /// On **47** `B` is CHR A17 above the MMC3's seven bits, exactly as it is PRG A17 above its
    /// four ([`Mmc3Board::multicart_prg`]). On **37** the page has CHR A17 *driven* by Q2, i.e.
    /// **the MMC3's own bit 7 never reaches the CHR-ROM**, which is the second place Mesen 2
    /// differs — `MMC3_37.h` masks the CHR bank with nothing and only ORs $80 in for blocks 4-7.
    /// AMD-061 §1 settles that one for the page too.
    #[cfg(feature = "board-mmc3-multicart")]
    fn multicart_chr(&self, bank: i32) -> i32 {
        let inner = bank & 0x7F;
        if self.mapper_number == 47 {
            inner | (i32::from(self.outer & 0x01) << 7)
        } else {
            inner | (((i32::from(self.outer) >> 2) & 1) << 7)
        }
    }

    /// The TQROM's chip select: which of the eight 1 KiB CHR windows read the CHR-RAM chip
    /// (mapper 119). See [`Mmc3Board::chr_ram_windows`].
    ///
    /// The select is a line of the **bank number**, so a 2 KiB register — which "ignores bit 0"
    /// — puts **both** of its 1 KiB windows on the same chip: `(v >> 1) * 2` is `v & $FE`, so
    /// bit 6 of the written byte is bit 6 of both 1 KiB banks. Which registers drive which
    /// windows is bit 7 of $8000, the same swap `update_banks` makes.
    #[cfg(feature = "board-119")]
    fn refresh_chr_source(&mut self) {
        let high = usize::from(self.select & 0x80 != 0) * 4;
        let low = 4 - high;
        let bit = |register: usize, slot: usize| -> u8 {
            if self.bank[register] & 0x40 == 0 {
                0
            } else {
                1u8 << slot
            }
        };
        let mask = bit(0, high)
            | bit(0, high + 1)
            | bit(1, high + 2)
            | bit(1, high + 3)
            | bit(2, low)
            | bit(3, low + 1)
            | bit(4, low + 2)
            | bit(5, low + 3);
        self.chr_ram_windows = mask;
    }

    fn update_banks(&mut self) {
        self.refresh_nametable_pages();

        if self.select & 0x40 != 0 {
            self.set_prg_8k(0, -2);
            self.set_prg_8k(2, i32::from(self.bank[6]));
        } else {
            self.set_prg_8k(0, i32::from(self.bank[6]));
            self.set_prg_8k(2, -2);
        }

        self.set_prg_8k(1, i32::from(self.bank[7]));
        self.set_prg_8k(3, -1);

        let high = if self.select & 0x80 != 0 { 4 } else { 0 };
        let low = 4 - high;
        self.set_chr_2k(high, i32::from(self.bank[0] >> 1));
        self.set_chr_2k(high + 2, i32::from(self.bank[1] >> 1));
        for i in 0..4 {
            self.set_chr_1k(low + i, i32::from(self.bank[2 + i]));
        }

        #[cfg(feature = "board-119")]
        if TQROM {
            self.refresh_chr_source();
        }
    }

    /// Where in the 8 KiB CHR-RAM chip window `window` starts, when the chip select puts it
    /// there (mapper 119).
    ///
    /// The chip has **three** bank lines, so the low three bits of the bank number are its whole
    /// address above the kilobyte. The bank number is re-derived from the register here rather
    /// than read off [`BankWindows`], because the window table holds the bank wrapped modulo the
    /// *CHR-ROM's* size and that wrap need not preserve bit 2.
    #[cfg(feature = "board-119")]
    fn tqrom_ram_offset(&self, window: usize) -> usize {
        let high = usize::from(self.select & 0x80 != 0) * 4;
        let bank = if window >= high && window < high + 4 {
            // The 2 KiB half: R0 over the first two windows of it, R1 over the next two.
            let register = (window - high) >> 1;
            (self.bank[register] & 0xFE) | u8::from((window - high) & 1 != 0)
        } else {
            // The 1 KiB half: R2-R5, one window each, starting at `low` = 4 - high, so the
            // register is `2 + (window - low)`. `window + high` is at least 4 here.
            self.bank[window + high - 2]
        };
        usize::from(bank & 0x07) * BankWindows::CHR_WINDOW_SIZE
    }

    /// A TQROM CHR fetch (mapper 119).
    ///
    /// **`#[inline(never)]`, and MF-6a2 step 3 measured why.** `Board::ppu_peek`'s arms are
    /// inlined into `Ppu2C02::background_fetch`, 7 % to 11 % of every ROM of lot PR-1's set;
    /// inlined, this body took that function from **271 instructions to 583** and cost `m4`
    /// **3.4 M over 300 frames**, `metroid` 3.5 M and `mm2` 5.4 M — three games on boards mapper
    /// 119 does not touch. `#[inline(never)]` on it alone recovered **8.0 M of set total** and
    /// put `Board::ppu_peek` back to 272. That is the fourth hook of this project with a
    /// measured body cap (PERFORMANCE-HISTORY §5.12), and the price is a `callq` per fetch for
    /// the one board that needs it.
    ///
    /// A window left on the CHR-ROM needs **no new arithmetic at all**: [`BankWindows`] already
    /// holds the raw register wrapped modulo the CHR-ROM's banks, which is that window's offset.
    #[cfg(feature = "board-119")]
    #[inline(never)]
    fn tqrom_read(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        let index = usize::from(address) & 0x1FFF;
        let window = index / BankWindows::CHR_WINDOW_SIZE;
        if self.chr_ram_windows & (1u8 << window) == 0 {
            return self.banks.read_chr(address, memory.rom().chr_rom());
        }

        let ram = memory.chr_ram();
        if ram.is_empty() {
            return 0;
        }

        let offset = self.tqrom_ram_offset(window) + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
        ram[super::super::wrap_offset(offset, ram.len())]
    }

    /// A PPU write on a TQROM (mapper 119).
    ///
    /// `Mapper::ppu_write`'s default drops every write on a board that has CHR-ROM, and a TQROM
    /// has both chips — so without this the CHR-RAM would be unreachable.
    /// `req_core_100_tqrom_a_write_to_a_ram_selected_window_is_not_swallowed` is §5 of the
    /// inventory's own wording for it. `#[inline(never)]` for the reason
    /// [`Mmc3Board::tqrom_read`] carries: `Board::ppu_write`'s arms are inlined into
    /// `NesBus::write_internal`, which a `$2007` write arrives through.
    #[cfg(feature = "board-119")]
    #[inline(never)]
    fn tqrom_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let index = usize::from(address) & 0x1FFF;
        let window = index / BankWindows::CHR_WINDOW_SIZE;
        if self.chr_ram_windows & (1u8 << window) == 0 {
            // The ROM chip drives this window: the write goes nowhere.
            return;
        }

        let offset = self.tqrom_ram_offset(window) + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
        let ram = memory.chr_ram_mut();
        if !ram.is_empty() {
            let len = ram.len();
            ram[super::super::wrap_offset(offset, len)] = value;
        }
    }

    /// The outer register of mappers 47 and 37, at $6000-$7FFF.
    ///
    /// **The condition both pages put on it**, and §5 of `docs/review/MF-6-inventory.md`'s own
    /// "easy thing to miss": it is "only writable when MMC3 PRG-RAM is enabled and writable (see
    /// $A001)" (047) and "you will need to enable writes to PRG-RAM" (037). That is
    /// [`Mapper::prg_ram_writable`] — $A001 bit 7 set and bit 6 clear — and **the range is
    /// consumed either way**, because the register *is* the board's decode of $6000-$7FFF.
    ///
    /// 037's page adds that the register "is not readable and attempting to read it will return
    /// open bus", which falls out of giving both numbers `prg_ram: Some(0)` in
    /// `registry::board_memory_sizes`: with no RAM behind the window,
    /// `Cartridge::default_cpu_read` falls through to [`Mapper::try_read_prg`] and that answers
    /// `None` below $8000.
    ///
    /// Out of line, which is shape C of MF-6a2 step 3's six: this is a CPU-write decode and
    /// `Board::intercept_cpu_write`'s arms reach `NesBus::write_internal`.
    #[cfg(feature = "board-mmc3-multicart")]
    #[inline(never)]
    fn multicart_outer_write(&mut self, address: u16, value: u8) -> bool {
        if !(0x6000..=0x7FFF).contains(&address) {
            return false;
        }

        if self.prg_ram_writable() {
            self.outer = value & 0x07;
            self.update_banks();
        }

        true
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

impl<const MC_ACC: bool, const TQROM: bool, const MULTICART: bool> Mapper
    for Mmc3Board<MC_ACC, TQROM, MULTICART>
{
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

        // Appended **behind** the existing versioning and behind the `const` that is the only
        // reason the field exists, so no byte of mapper 4's, 118's or the MC-ACC's block moves
        // (REQ-CORE-090). The TQROM has nothing to append: `chr_ram_windows` is derived and
        // `load_registers` brings it back through `update_banks`.
        #[cfg(feature = "board-mmc3-multicart")]
        if MULTICART {
            writer.write_u8(self.outer);
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

        #[cfg(feature = "board-mmc3-multicart")]
        if MULTICART {
            self.outer = reader.read_u8()?;
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
        // A TQROM cannot answer: a `ChrView` is one slice plus eight offsets and has no way to
        // say "this window is the other chip". `None` is the body the MMC5 and the RAMBO-1
        // already give, so it adds nothing to that dispatch (MF-6a step 2's finding, reused).
        #[cfg(feature = "board-119")]
        if TQROM {
            return None;
        }

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

    /// The TQROM's two chips (mapper 119), and `Mapper::ppu_peek`'s own default for every other
    /// monomorphisation — written out rather than inherited so that the three that are **not** a
    /// TQROM tail-merge with the shared default arm of `Board::ppu_peek`, which MF-6a2 step 3
    /// measured at 272 instructions against the baseline's 271.
    ///
    /// The whole override is behind `board-119`: with that feature off, `Mmc3Board` has no
    /// `ppu_peek` of its own and mapper 4 takes the trait default exactly as it did before.
    #[cfg(feature = "board-119")]
    fn ppu_peek(
        &self,
        memory: &CartridgeMemory,
        address: u16,
        _fetch: crate::cartridges::ChrFetch,
    ) -> u8 {
        if TQROM {
            return self.tqrom_read(memory, address);
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

    /// The TQROM's CHR-RAM (mapper 119), and `Mapper::ppu_write`'s own default otherwise; see
    /// [`Mmc3Board::tqrom_write`] and the note on `ppu_peek` above.
    #[cfg(feature = "board-119")]
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if TQROM {
            return self.tqrom_write(memory, address, value);
        }

        if memory.rom().chr_rom().is_empty() {
            self.banks.write_chr(address, value, memory.chr_ram_mut());
        }
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
        // [`Mmc3Board::dots_per_clock`] settled this at load, which is what makes the two
        // monomorphisations one arm body here instead of two.
        Some((clocks_to_zero - 1) * self.dots_per_clock)
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
        #[cfg(feature = "board-mmc3-multicart")]
        {
            self.outer = 0;
        }
        self.a12.reset();
        // `chr_ram_windows` comes back here too: `update_banks` refreshes it on a TQROM.
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
        // Mappers 47 and 37 decode $6000-$7FFF themselves; see
        // [`Mmc3Board::multicart_outer_write`].
        #[cfg(feature = "board-mmc3-multicart")]
        if MULTICART {
            return self.multicart_outer_write(address, value);
        }

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
