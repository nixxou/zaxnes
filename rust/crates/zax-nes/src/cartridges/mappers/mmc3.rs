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
/// **The `CHR_RAM_*` parameters are the CHR-RAM family and `MULTICART` is mappers 47 and 37**,
/// each a `const` parameter for the same reason and each behind a Cargo feature (`board-119`,
/// `board-mmc3-chrram`, `board-mmc3-multicart`). MF-6a2 step 3 built all three, measured **six**
/// shapes and reverted them on that lot's stop-the-lot condition; the owner's tolerance of
/// 2026-11-06 accepts their cost and MF-6a3 rebuilt them from its write-up. The shape here is
/// that step's **C**, which was the best of the six: `const` parameters (so mapper 4 and 118 keep
/// their own monomorphisation), the CHR-RAM board's `ppu_peek`/`ppu_write` bodies
/// `#[inline(never)]`, one `update_banks` body whose outer-bank arithmetic is compile-time dead
/// off the multicart, and the outer register's decode out of line. Shape **A** — the same
/// parameters with everything inlined — took `Board::ppu_peek` from 271 instructions to **583**
/// and cost `m4` **3.4 M over 300 frames**, and `#[inline(never)]` on the one body recovered
/// 8.0 M of set total; shape **D**, one parameter and one `Board` variant for all three numbers,
/// was *worse* on `m4` (+750 676 against +258 640).
///
/// # The CHR-RAM family, and the four columns of its page (MF-UNL-2 step 1)
///
/// nesdev's *MMC3 with CHR ROM and CHR RAM* lists every MMC3-like number that carries a CHR-RAM
/// chip beside its CHR-ROM, and the table has **four** columns: the number, the **first RAM
/// bank**, the **size of RAM** in 1 KiB banks, and **"RAM mirrored?"**. So the generalisation
/// `docs/review/UNL-census.md` §7.2 asks for — `Mmc3ChrRam<FIRST, LAST, SIZE>`, "the 1 KiB CHR
/// banks in `FIRST..=LAST` come from the CHR-RAM chip, the rest from CHR-ROM" — carries three of
/// them, and the fourth is [`Mmc3Board::CHR_RAM_MIRRORED`]. Its own note says why it cannot be
/// dropped.
///
/// [`Mmc3ChrRam`] is the census's three-parameter alias and the four numbers of
/// `board-mmc3-chrram` are written through it; [`Tqrom`] is mapper 119, the one member of the
/// family that was already here, and the only one whose `MIRRORED` is set.
///
/// Monomorphisations are instantiated for mappers 4/118, 4 submapper 3, the two multicarts and
/// one per CHR-RAM number, and no other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Board<
    const MC_ACC: bool,
    const CHR_RAM_FIRST: u8,
    const CHR_RAM_LAST: u8,
    const CHR_RAM_KIB: u8,
    const CHR_RAM_MIRRORED: bool,
    const CHR_RAM_GAL: bool,
    const MULTICART: bool,
> {
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
    /// The CHR-RAM family: one bit per 1 KiB CHR window, **set when that window reads the
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
    /// Mapper 195, the Waixing FS303: the GAL's `CHRB` register, which moves the CHR-RAM window
    /// at run time (MF-UNL-2 step 2). Latched by a PPU write to a CHR-**ROM** window; see
    /// [`Mmc3Board::gal_window`] and [`Mmc3Board::chr_ram_write`].
    ///
    /// **Not** derived: no other register implies it, so it is the one byte this lot appends to
    /// the save-state block — behind the `const` that is the only reason the field exists, so no
    /// byte of any other board's block moves (REQ-CORE-090).
    #[cfg(feature = "board-mmc3-chrram")]
    chr_ram_gal: u8,
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

/// The MMC3 and the MMC6, mapper 4 and mapper 118.
pub type Mmc3 = Mmc3Board<false, 0, 0, 0, false, false, false>;

/// The Acclaim MC-ACC, mapper 4 submapper 3.
pub type McAcc = Mmc3Board<true, 0, 0, 0, false, false, false>;

/// `docs/review/UNL-census.md` §7.2's own alias for the CHR-RAM family: "the 1 KiB CHR banks in
/// `FIRST..=LAST` come from the CHR-RAM chip, the rest from CHR-ROM", with `SIZE` the chip's own
/// size in 1 KiB banks.
///
/// The four numbers of `board-mmc3-chrram` are written through it, each with the row nesdev's
/// *MMC3 with CHR ROM and CHR RAM* gives it. Mapper 119 is **not**: its row has the fourth
/// column set, so it is spelled out as [`Tqrom`] below.
#[cfg(feature = "board-mmc3-chrram")]
pub type Mmc3ChrRam<const FIRST: u8, const LAST: u8, const SIZE: u8> =
    Mmc3Board<false, FIRST, LAST, SIZE, false, false, false>;

/// Mapper 74, the Waixing **43-393 / 43-406 / 860908C** PCB: "an MMC3 clone together with a
/// 74LS138 and a 74LS139 to redirect 1 KiB CHR-ROM banks #8 and #9 to 2 KiB of CHR-RAM"
/// (nesdev, "INES Mapper 074").
///
/// *機甲戰士* (the *Metal Max* translation), *甲A — China Soccer League*,
/// *第四次: 机器人大战* and the original 1997 *風雲 — Traitor Legend*.
#[cfg(feature = "board-mmc3-chrram")]
pub type Mmc3ChrRam74 = Mmc3ChrRam<0x08, 0x09, 2>;

/// Mapper 192, the Waixing **FS308**: 74's board with twice the RAM — "CHR Pages $08-$0B are
/// CHR-RAM, other pages are CHR-ROM" (nesdev, "INES Mapper 192"), which that page calls
/// "effectively the same as INES Mapper 074 but with twice as much CHR RAM".
///
/// *Ying Lie Qun Xia Zhuan*.
#[cfg(feature = "board-mmc3-chrram")]
pub type Mmc3ChrRam192 = Mmc3ChrRam<0x08, 0x0B, 4>;

/// Mapper 194, whose page names no board: "an additional 2k of CHR-RAM which is selectable. CHR
/// Pages $00-$01 are CHR-RAM, other pages are CHR-ROM. Apart from that, this mapper behaves
/// exactly like your typical MMC3" (nesdev, "INES Mapper 194", quoting Disch).
///
/// *Dai-2-Ji — Super Robot Taisen*, the port.
#[cfg(feature = "board-mmc3-chrram")]
pub type Mmc3ChrRam194 = Mmc3ChrRam<0x00, 0x01, 2>;

/// Mapper 195, the Waixing **FS303**: an MMC3 clone "with a GAL that can dynamically change
/// which CHR banks are mapped to RAM" (nesdev, "INES Mapper 195").
///
/// **`GAL` is set, so `FIRST` and `LAST` are not read at all** — the window comes from the
/// board's own `CHRB` register, [`Mmc3Board::gal_window`]. `SIZE` is **8** and not the census's
/// 4: the page's "The PCB mounts 32 KiB of CHR-RAM, although address lines for only 8 KiB are
/// connected" is what the chip can address, and three of its lines is exactly what makes the
/// page's own worked example true — "setting $80 ($28-$2B) will select different 4 KiB of
/// CHR-RAM from setting $88 ($4C-$4F), but the same 4 KiB as setting $82 ($00-$03), because CHR
/// A13 is not connected to CHR-RAM". A 4 KiB chip could not tell `$28-$2B` from `$4C-$4F`.
///
/// *哥伦布传 — 黄金中文版* (the *Columbus* translation) and *天神之剑* (the *Crystalis* port);
/// *天使之翼 2* is a hack and *赌神* a re-release. **Q-142** holds the power-on question.
#[cfg(feature = "board-mmc3-chrram")]
pub type WaixingFs303 = Mmc3Board<false, 0, 0, 8, false, true, false>;

/// The Nintendo TQROM, mapper 119: the MMC3 with a CHR-RAM chip beside its CHR-ROM, **CHR A16**
/// choosing between them.
///
/// nesdev ("INES Mapper 119"): "Bit 6 of the CHR bank number is used to switch between the CHR
/// ROM chip and the CHR RAM chip", the board doing it with "a 74HC32 IC" whose output "controls
/// the CHR ROM's enable line. PPU A13 is ORed with CHR A16 in order to create a /CE signal for
/// the CHR ROM", and "Both TQROM games used 8 KiB CHR RAM". The two titles are *High Speed* and
/// *Pin\*Bot*.
///
/// **`MIRRORED` is set, and that is the whole of this number's difference from the other four**
/// (MF-UNL-2 step 1): *MMC3 with CHR ROM and CHR RAM* gives 119 first RAM bank "**64, 192**",
/// `RAM mirrored? Yes`, and the one-line behaviour "CHR A16 switches between CHR ROM and CHR
/// RAM". See [`Mmc3Board::CHR_RAM_MIRRORED`].
#[cfg(feature = "board-119")]
pub type Tqrom = Mmc3Board<false, 0x40, 0x7F, 8, true, false, false>;

/// Nintendo's two MMC3 multicarts, mappers 47 and 37: an MMC3 whose PRG and CHR bank numbers
/// carry one or two more address lines from an outer register at $6000-$7FFF.
///
/// 47 is *Super Spike V'Ball + Nintendo World Cup*, 37 *Super Mario Bros. + Tetris + Nintendo
/// World Cup* (PAL). See [`Mmc3Board::multicart_prg`] and [`Mmc3Board::multicart_chr`] for the
/// two pages' own wiring, and [`Mmc3Board::multicart_outer_write`] for the condition both of
/// them put on the register.
#[cfg(feature = "board-mmc3-multicart")]
pub type Mmc3Multicart = Mmc3Board<false, 0, 0, 0, false, false, true>;

impl<
        const MC_ACC: bool,
        const CHR_RAM_FIRST: u8,
        const CHR_RAM_LAST: u8,
        const CHR_RAM_KIB: u8,
        const CHR_RAM_MIRRORED: bool,
        const CHR_RAM_GAL: bool,
        const MULTICART: bool,
    >
    Mmc3Board<
        MC_ACC,
        CHR_RAM_FIRST,
        CHR_RAM_LAST,
        CHR_RAM_KIB,
        CHR_RAM_MIRRORED,
        CHR_RAM_GAL,
        MULTICART,
    >
{
    /// Whether this monomorphisation carries a CHR-RAM chip beside its CHR-ROM: the `SIZE`
    /// column of the family's page, zero on every board that does not.
    ///
    /// It replaces MF-6a3 step 1's `TQROM` parameter and is used in exactly the places that
    /// `if TQROM` was, so mappers 4, 118, the MC-ACC and the two multicarts keep bodies in which
    /// the whole mechanism is compile-time dead.
    #[cfg(feature = "board-119")]
    const HAS_CHR_RAM: bool = CHR_RAM_KIB != 0;

    /// The chip's own bank lines, as a mask of the 1 KiB bank number.
    ///
    /// The chip has as many address lines above the kilobyte as its size needs — three on the
    /// TQROM's 8 KiB — so the low `log2(SIZE)` bits of the bank number are its whole address
    /// above the kilobyte and every bank above `SIZE` aliases down onto it. That is the page's
    /// `RAM mirrored?` **inside** the chip, which holds for all five numbers: `FIRST` is a
    /// multiple of `SIZE` in every row of the table, so `bank & mask` and
    /// `(bank - FIRST) & mask` are the same number.
    ///
    /// `saturating_sub` and not `- 1`, so that the boards with no chip (`SIZE` = 0) have a
    /// well-defined mask of zero rather than a const-evaluation underflow.
    #[cfg(feature = "board-119")]
    const CHR_RAM_BANK_MASK: u8 = CHR_RAM_KIB.saturating_sub(1);

    /// Whether the 1 KiB CHR bank `bank` comes from the CHR-RAM chip rather than the CHR-ROM.
    ///
    /// The census's own sentence is "the 1 KiB CHR banks in `FIRST..=LAST` come from the CHR-RAM
    /// chip, the rest from CHR-ROM", and that is the body — **behind `MIRRORED`**, which is the
    /// fourth column of nesdev's *MMC3 with CHR ROM and CHR RAM* and cannot be folded into the
    /// pair.
    ///
    /// **Mapper 119 is why.** That page's table gives 119 first RAM bank "**64, 192**" — two
    /// numbers — and its *Behaviour* section the reason: "119: CHR A16 switches between CHR ROM
    /// and CHR RAM". The select is **one bank line** and not a comparison, so CHR A17 (bit 7) is
    /// a don't-care and banks `$C0-$FF` read the RAM exactly as `$40-$7F` do. The same holds of
    /// 191, whose line is CHR A17. The numbers of `board-mmc3-chrram` are the page's `No` rows:
    /// their select is a genuine range, decoded on the board by a 74LS138/74LS139 pair (74's
    /// page) or a GAL (195's).
    ///
    /// With `MIRRORED` set the bank number is reduced by `& LAST` before the range test, which is
    /// exactly "the lines above the window are not connected": `$C0 & $7F` is `$40`, and
    /// `bank & $7F >= $40` is bit 6 of `bank` — the expression MF-6a3 step 1 wrote by hand.
    ///
    /// **Mesen 2 reads it the other way and this build does not follow it.** Its registration is
    /// `MMC3_ChrRam(0x40, 0x7F, 8)` and `MMC3_ChrRam::SelectChrPage` tests
    /// `page >= _firstRamBank && page <= _lastRamBank`, under which a bank of `$C0-$FF` reads the
    /// CHR-ROM. `docs/review/UNL-census.md` §7.2 quotes that registration as the form to
    /// generalise into; the page is what `board-119` has implemented since MF-6a3 step 1, and
    /// this lot's own rule is that no existing mapper's emulation moves.
    /// `req_core_100_tqrom_chr_ram_select_is_chr_a16_over_all_256_bank_numbers` is the gate.
    /// On mapper 195 the window is not a `const` pair at all: see [`Mmc3Board::gal_window`].
    #[cfg(feature = "board-119")]
    fn chr_ram_selected(&self, bank: u8) -> bool {
        #[cfg(feature = "board-mmc3-chrram")]
        if CHR_RAM_GAL {
            return match Self::gal_window(self.chr_ram_gal) {
                Some((first, last)) => bank >= first && bank <= last,
                None => false,
            };
        }

        let bank = if CHR_RAM_MIRRORED {
            bank & CHR_RAM_LAST
        } else {
            bank
        };
        bank >= CHR_RAM_FIRST && bank <= CHR_RAM_LAST
    }

    /// The power-on `CHRB` of mapper 195's GAL, which its page states: **`$80`**. See
    /// [`Mmc3Board::gal_window`].
    #[cfg(feature = "board-mmc3-chrram")]
    const GAL_POWER_ON: u8 = 0x80;

    /// Mapper 195's GAL: which 1 KiB CHR banks the `CHRB` register puts on the CHR-RAM chip,
    /// as `(first, last)`, or `None` for "all of CHR-ROM" (MF-UNL-2 step 2).
    ///
    /// nesdev ("INES Mapper 195"), the Waixing **FS303**, whose MMC3 clone comes "with a GAL that
    /// can dynamically change which CHR banks are mapped to RAM". The register is written "via
    /// the PPU", and the page's diagram is
    ///
    /// ```text
    /// CHRB~[1Z.D L.L.]
    ///       || | | |
    ///       |+-|-+-+---------------- Select first bank and size of CHR RAM:
    ///       || |                      $80 = $28-$2B      $C0 = $46-$47
    ///       || |                      $82 = $00-$03      $C2 = $7C-$7D
    ///       || |                      $88 = $4C-$4F      $C8 = $0A-$0B
    ///       || |                      $8A = $64-$67      $CA = only CHR ROM
    ///       || +-------------------- If 1, ignore above and always enable CHR ROM / disable CHR RAM
    ///       |+---------------------- Number of banks of CHR RAM, 0=4KiB, 1=2KiB
    ///       +----------------------- Must be 1
    /// Power-on value: $80
    /// ```
    ///
    /// **The eight rows are one three-bit field**, which is what the diagram's own arrow says:
    /// the "Select first bank and size" line reaches `Z` (bit 6) *and* both `L` bits (bits 3 and
    /// 1), and `(Z, L, L)` counted as a number walks the page's list in the order the page writes
    /// it — `$80, $82, $88, $8A, $C0, $C2, $C8, $CA`. So this is one eight-entry table and not
    /// two four-entry ones, and `Z`'s own annotation ("0=4KiB, 1=2KiB") falls out of it: the
    /// first four rows are four banks wide and the last four are two.
    ///
    /// Bits 5, 2 and 0 are don't-cares, and **bit 4 (`D`) wins over the field**.
    ///
    /// Mesen 2 does **not** implement the GAL: it registers the number as the static
    /// `MMC3_ChrRam(0x00, 0x03, 4)`, which is the `$82` row. `docs/review/UNL-census.md` §7.2
    /// quotes that registration *and* asks for "the page's run-time GAL" in the same sentence,
    /// and the two disagree about the power-on state — the page's is `$80`, `$28-$2B`. **Q-142**
    /// holds both readings and the choice; the page's is implemented.
    #[cfg(feature = "board-mmc3-chrram")]
    const fn gal_window(register: u8) -> Option<(u8, u8)> {
        // "If 1, ignore above and always enable CHR ROM / disable CHR RAM."
        if register & 0x10 != 0 {
            return None;
        }

        // `(Z, L, L)` = bits 6, 3 and 1, in that order.
        let row = ((register >> 4) & 0x04) | ((register >> 2) & 0x02) | ((register >> 1) & 0x01);
        let (first, banks) = match row {
            0 => (0x28, 4),
            1 => (0x00, 4),
            2 => (0x4C, 4),
            3 => (0x64, 4),
            4 => (0x46, 2),
            5 => (0x7C, 2),
            6 => (0x0A, 2),
            // `$CA = only CHR ROM`.
            _ => return None,
        };
        Some((first, first + banks - 1))
    }

    /// The 1 KiB CHR bank number window `window` shows, re-derived from the bank registers.
    ///
    /// Which registers drive which windows is bit 7 of `$8000`, the same swap `update_banks`
    /// makes: four windows take the two 2 KiB registers R0 and R1, two windows each, and the
    /// other four take R2-R5 one each. A 2 KiB register "ignores bit 0", so its two windows show
    /// `v & $FE` and `v | $01`.
    ///
    /// The number is re-derived from the register here rather than read off [`BankWindows`],
    /// because the window table holds the bank wrapped modulo the *CHR-ROM's* size and that wrap
    /// need not preserve the chip-select line or the chip's own low bits.
    ///
    /// **`pub(crate)` and widened for mapper 262** (MF-UNL-12 step 1), whose 8 KiB CHR-RAM chip is
    /// selected by a register rather than by a bank number and so needs exactly this answer: the
    /// bank the chip drives at that window, before the window table's wrap.
    #[cfg(any(feature = "board-119", feature = "board-262"))]
    pub(crate) fn chr_bank_of_window(&self, window: usize) -> u8 {
        let high = usize::from(self.select & 0x80 != 0) * 4;
        if window >= high && window < high + 4 {
            // The 2 KiB half: R0 over the first two windows of it, R1 over the next two.
            let register = (window - high) >> 1;
            (self.bank[register] & 0xFE) | u8::from((window - high) & 1 != 0)
        } else {
            // The 1 KiB half: R2-R5, one window each, starting at `low` = 4 - high, so the
            // register is `2 + (window - low)`. `window + high` is at least 4 here.
            self.bank[window + high - 2]
        }
    }

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
            #[cfg(feature = "board-mmc3-chrram")]
            chr_ram_gal: Self::GAL_POWER_ON,
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

    /// This board's own CHR-RAM chip, in bytes: the `SIZE` column of nesdev's *MMC3 with CHR ROM
    /// and CHR RAM*, which for the TQROM is "Both TQROM games used 8 KiB CHR RAM". Settled at
    /// load by `registry::board_memory_sizes`, because no header field can state it — see that
    /// arm.
    #[cfg(feature = "board-119")]
    pub(crate) const CHR_RAM_SIZE: usize = CHR_RAM_KIB as usize * 1024;

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

    /// The chip select: which of the eight 1 KiB CHR windows read the CHR-RAM chip. See
    /// [`Mmc3Board::chr_ram_windows`] and [`Mmc3Board::chr_ram_selected`].
    ///
    /// One pass over the eight windows, each asked for its own bank number
    /// ([`Mmc3Board::chr_bank_of_window`]) and that number put to the select. MF-6a3 step 1 wrote
    /// the TQROM's eight bits out by hand — the select being a single line, a 2 KiB register's two
    /// halves were known to land on the same chip — and the generalisation cannot assume that of
    /// a `FIRST..=LAST` window, so each window is decided on its own bank.
    #[cfg(feature = "board-119")]
    fn refresh_chr_source(&mut self) {
        let mut mask = 0u8;
        for window in 0..BankWindows::CHR_WINDOWS {
            if self.chr_ram_selected(self.chr_bank_of_window(window)) {
                mask |= 1u8 << window;
            }
        }
        self.chr_ram_windows = mask;
    }

    /// Recomputes the windows from the registers, for a board that holds this chip and writes
    /// its own outer bank or PRG override over them: mapper 116 (MF-6a3 step 4) and the six
    /// scrambled boards of `board-mmc3-scramble` and the TXC 189 (MF-UNL-2 steps 3 and 5).
    /// Behind those three features, its only callers.
    ///
    /// Recomputing before the overlay is what makes such a board's refresh **idempotent**: the
    /// windows that go into it never already carry one, so no write can count a bank twice.
    ///
    /// **The fourth and fifth callers are mappers 12 and 14** (MF-UNL-8 steps 1 and 2), whose
    /// outer CHR line is laid over these windows per window rather than as one addend; see
    /// [`Huang1`](super::Huang1).
    #[cfg(any(
        feature = "board-116",
        feature = "board-huang1",
        feature = "board-mmc3-scramble",
        feature = "board-189",
        feature = "board-215",
        feature = "board-208",
        feature = "board-219",
        feature = "board-262"
    ))]
    pub(crate) fn refresh_banks(&mut self) {
        self.update_banks();
    }

    /// `$8000` itself, for a board that holds this chip and has to know which windows the chip's
    /// own bank registers drive: bit 6 is the PRG mode and bit 7 the CHR swap (MF-UNL-6 step 1).
    ///
    /// Behind `board-268`, its only caller. [`Mmc3Outer`](super::Mmc3Outer) recomputes all twelve
    /// windows itself rather than laying an overlay over the chip's, because mapper 268's outer
    /// register replaces *address lines* of both bank numbers — see that type's `refresh`.
    #[cfg(feature = "board-268")]
    pub(crate) const fn select(&self) -> u8 {
        self.select
    }

    /// One of the eight bank registers, for the board above. `index` is masked to the eight the
    /// chip has, which is what `$8000`'s own three bits do.
    ///
    /// **The second caller is mapper 208's submapper 1** (MF-UNL-11 step 2), whose page makes its
    /// one 32 KiB PRG page "the bank register value SHR 2" of the chip's own `R6`.
    #[cfg(any(feature = "board-268", feature = "board-208"))]
    pub(crate) const fn bank_register(&self, index: usize) -> u8 {
        self.bank[index & 0x07]
    }

    /// Forces the **revision A** scanline counter, for a board that holds this chip and whose
    /// own page states the rule (MF-UNL-2 step 3).
    ///
    /// nesdev ("INES Mapper 114"): "IRQ behavior resembles that of the MMC3A, i.e. **a latch
    /// value of zero disables the IRQ**, on which *Aladdin* depends", and 115's page says the
    /// same of 114 in the other vocabulary — "Mapper 114's IRQ behavior resembles 'NEC'
    /// behavior". That is exactly [`Mmc3Board::irq_revision_a`], which mapper 4 reaches through
    /// NES 2.0 submapper 4; these boards are not mapper 4 and carry no submapper, so the board
    /// that holds the chip sets it at load.
    ///
    /// The counterpart needs no call: 115's "Sharp" rule — "a latch value of zero causes an IRQ
    /// on every scanline, on which the Chinese version of *幽☆遊☆白書 Final* depends" — is this
    /// build's **default** revision B, so 115 takes the chip as it comes.
    ///
    /// **Mapper 268 is the third caller** (MF-UNL-6 step 1): its page's *Notes* say "AA6023's IRQ
    /// operator follows MMC3 'NEC' behavior, i.e. a latch value of zero disables IRQ", which is
    /// 114's sentence in the other vocabulary.
    ///
    /// **Mapper 12 is the fourth** (MF-UNL-8 step 1): "The Huang-1 ASIC functions exclusively as an
    /// MMC3A clone on this PCB" and "MMC3A-specific IRQ behavior is necessary for running *Dragon
    /// Ball Z 5*" (nesdev, "INES Mapper 012"). Mapper 14's page says nothing of the kind, so that
    /// number takes the chip as it comes.
    #[cfg(any(
        feature = "board-huang1",
        feature = "board-mmc3-scramble",
        feature = "board-268"
    ))]
    pub(crate) fn force_irq_revision_a(&mut self) {
        self.irq_revision_a = true;
    }

    /// Makes `$A000` inert, for a board that holds this chip and whose own page takes the
    /// mirroring away from it (MF-UNL-8 step 2, **Q-165**).
    ///
    /// nesdev ("INES Mapper 014"): "According to Nestopia's mapper source code, **mirroring can
    /// only be controlled in VRC2 mode**, and the VRC2 PRG and mirroring registers do not respond
    /// to address mirrors. These behaviors contradict other emulators and have not been verified
    /// on original hardware." The Gouder SL-1632's PAL routes the mirroring from the VRC2 half of
    /// the Huang-1 and from nowhere else, so the MMC3 half's one mirroring register — `$A000`, and
    /// it has no other function — drives nothing on that board.
    ///
    /// [`Mmc3Board::ignore_a000`] is already exactly that condition: mapper 4's NES 2.0 submapper
    /// 2 ("MMC3C with hard-wired mirroring"), a four-screen header and the TxSROM reach it at load.
    /// This board is none of the three and carries no such submapper, so the board that holds the
    /// chip sets it — the shape [`Mmc3Board::force_irq_revision_a`] above already has.
    ///
    /// **Mapper 208 is the second caller** (MF-UNL-11 step 2): its page's register list has no
    /// `$A000` section and puts "mirroring control" among the board's own additional registers, so
    /// the chip's one mirroring register drives nothing there either. Its submapper 1 says the
    /// opposite in its own words and so does not call this (Q-171).
    ///
    /// **Mapper 262 is the third caller** (MF-UNL-12 step 1): that board carries the 4 KiB of
    /// nametable four-screen mirroring needs, so the cartridge drives CIRAM A10 **and** A11 and the
    /// chip's one mirroring register reaches nothing -- which is the same condition a four-screen
    /// *header* already reaches at load. Its page says "$A000: As normal MMC3" all the same, so the
    /// choice is recorded in Q-172 and pinned by a test.
    #[cfg(any(feature = "board-huang1", feature = "board-208", feature = "board-262"))]
    pub(crate) fn force_ignore_a000(&mut self) {
        self.ignore_a000 = true;
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
        if Self::HAS_CHR_RAM {
            self.refresh_chr_source();
        }
    }

    /// Where in the CHR-RAM chip window `window` starts, when the chip select puts it there.
    ///
    /// The chip's own bank lines are its whole address above the kilobyte — three of them on the
    /// TQROM's 8 KiB — which is [`Mmc3Board::CHR_RAM_BANK_MASK`].
    #[cfg(feature = "board-119")]
    fn chr_ram_offset(&self, window: usize) -> usize {
        usize::from(self.chr_bank_of_window(window) & Self::CHR_RAM_BANK_MASK)
            * BankWindows::CHR_WINDOW_SIZE
    }

    /// A CHR fetch on a board of the CHR-RAM family.
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
    fn chr_ram_read(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        let index = usize::from(address) & 0x1FFF;
        let window = index / BankWindows::CHR_WINDOW_SIZE;
        if self.chr_ram_windows & (1u8 << window) == 0 {
            return self.banks.read_chr(address, memory.rom().chr_rom());
        }

        let ram = memory.chr_ram();
        if ram.is_empty() {
            return 0;
        }

        let offset = self.chr_ram_offset(window) + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
        ram[super::super::wrap_offset(offset, ram.len())]
    }

    /// A PPU write on a board of the CHR-RAM family.
    ///
    /// `Mapper::ppu_write`'s default drops every write on a board that has CHR-ROM, and these
    /// boards have both chips — so without this the CHR-RAM would be unreachable.
    /// `req_core_100_tqrom_a_write_to_a_ram_selected_window_is_not_swallowed` is §5 of the
    /// inventory's own wording for it. `#[inline(never)]` for the reason
    /// [`Mmc3Board::chr_ram_read`] carries: `Board::ppu_write`'s arms are inlined into
    /// `NesBus::write_internal`, which a `$2007` write arrives through.
    #[cfg(feature = "board-119")]
    #[inline(never)]
    fn chr_ram_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let index = usize::from(address) & 0x1FFF;
        let window = index / BankWindows::CHR_WINDOW_SIZE;
        if self.chr_ram_windows & (1u8 << window) == 0 {
            // **Mapper 195's GAL latches here**, and the page is what puts it on this branch:
            // "The games write, via the PPU, to specific CHR banks in order to select both which
            // and how many CHR banks are mapped to RAM instead of ROM. **Writes to CHR ROM** in
            // multiples of 1 KiB." A write to a RAM window is a RAM write and a write to a ROM
            // window is the register — which is the only way a board with one PPU write line
            // could tell the two apart.
            //
            // Bit 7 "Must be 1", so a byte without it is not a `CHRB` value and the window is
            // left where it was. On every other board of the family this branch is what it was:
            // the ROM chip drives the window and the write goes nowhere.
            #[cfg(feature = "board-mmc3-chrram")]
            if CHR_RAM_GAL && value & 0x80 != 0 {
                self.chr_ram_gal = value;
                self.update_banks();
            }

            return;
        }

        let offset = self.chr_ram_offset(window) + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
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

impl<
        const MC_ACC: bool,
        const CHR_RAM_FIRST: u8,
        const CHR_RAM_LAST: u8,
        const CHR_RAM_KIB: u8,
        const CHR_RAM_MIRRORED: bool,
        const CHR_RAM_GAL: bool,
        const MULTICART: bool,
    > Mapper
    for Mmc3Board<
        MC_ACC,
        CHR_RAM_FIRST,
        CHR_RAM_LAST,
        CHR_RAM_KIB,
        CHR_RAM_MIRRORED,
        CHR_RAM_GAL,
        MULTICART,
    >
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
        // (REQ-CORE-090). The CHR-RAM family has nothing to append: `chr_ram_windows` is derived and
        // `load_registers` brings it back through `update_banks`.
        #[cfg(feature = "board-mmc3-multicart")]
        if MULTICART {
            writer.write_u8(self.outer);
        }

        // Mapper 195's GAL register, on the same terms: behind the `const`, so the family's
        // other four numbers and every board above append nothing (MF-UNL-2 step 2).
        #[cfg(feature = "board-mmc3-chrram")]
        if CHR_RAM_GAL {
            writer.write_u8(self.chr_ram_gal);
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

        #[cfg(feature = "board-mmc3-chrram")]
        if CHR_RAM_GAL {
            self.chr_ram_gal = reader.read_u8()?;
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
        // A board with two chips cannot answer: a `ChrView` is one slice plus eight offsets and
        // has no way to
        // say "this window is the other chip". `None` is the body the MMC5 and the RAMBO-1
        // already give, so it adds nothing to that dispatch (MF-6a step 2's finding, reused).
        #[cfg(feature = "board-119")]
        if Self::HAS_CHR_RAM {
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

    /// The CHR-RAM family's two chips, and `Mapper::ppu_peek`'s own default for every other
    /// monomorphisation — written out rather than inherited so that the ones that are **not** of
    /// the family tail-merge with the shared default arm of `Board::ppu_peek`, which MF-6a2
    /// step 3 measured at 272 instructions against the baseline's 271.
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
        if Self::HAS_CHR_RAM {
            return self.chr_ram_read(memory, address);
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

    /// The family's CHR-RAM, and `Mapper::ppu_write`'s own default otherwise; see
    /// [`Mmc3Board::chr_ram_write`] and the note on `ppu_peek` above.
    #[cfg(feature = "board-119")]
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if Self::HAS_CHR_RAM {
            return self.chr_ram_write(memory, address, value);
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
        // "Power-on value: $80" (nesdev, "INES Mapper 195").
        #[cfg(feature = "board-mmc3-chrram")]
        {
            self.chr_ram_gal = Self::GAL_POWER_ON;
        }
        self.a12.reset();
        // `chr_ram_windows` comes back here too: `update_banks` refreshes it on a CHR-RAM board.
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
