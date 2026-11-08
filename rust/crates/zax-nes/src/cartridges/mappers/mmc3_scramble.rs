use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mappers **114, 115, 121, 123, 182 and 187** — six MMC3-clone boards from SuperGame,
/// Hosenkan and 卡聖 (Kǎshèng), each of which adds some of the same three things to the chip
/// (MF-UNL-2 step 3, REQ-CORE-102).
///
/// `docs/review/UNL-census.md` §7.2 names the three: "(a) a `const` scramble table for the
/// register address and index, (b) an outer CHR-ROM bank register, (c) an NROM-like PRG override
/// register". No number has all of them and none has anything else, so this is **one board with
/// six wirings** — the shape MF-UNL-1 step 1 gave the JV001's six numbers — and it **holds** an
/// [`Mmc3`] rather than reimplementing one, which is [`super::Somari`]'s and
/// [`super::NesEvent`]'s shape.
///
/// | # | board | publisher | what it adds |
/// | ---: | --- | --- | --- |
/// | 114 | SuperGame / Hosenkan | SuperGame, Hosenkan | scrambled register **addresses and indices**, two patterns by submapper; outer CHR bank at `$6001`; NROM override at `$6000`; **MMC3A IRQ** |
/// | 182 | the same board | Hosenkan | nothing of its own |
/// | 115 | Kǎshèng SFC-02B/-03/-004 | Kǎshèng | the same pair under mask `$E003`, no scrambling, PRG A18 in the override, **MMC3C IRQ** |
/// | 123 | UNL-H2288 | Kǎshèng, Hummer Team | index scrambling only; the override at `$5800` with its bits reordered |
/// | 187 | Kǎshèng A98402 | Kǎshèng, Hummer Team | the override at `$5000`, a protection read, **CHR A18 from PPU A12** |
/// | 121 | Kǎshèng A9711 / A9713 | Kǎshèng | a protection array at `$5000-$5FFF`, an outer bank at `$5180`, extended PRG banking from bit-reversed `$8001` values, and the same CHR A18 trick |
///
/// # 182 is 114, and the page says so
///
/// NESdev **redirects `INES Mapper 182` to mapper 114's page**, which states: "iNES Mapper 182 is
/// a duplicate of Mapper 114. In fact, FCEUX explicitly reassigns Hosenkan's Mapper 182 ROMs to
/// Mapper 114. Because Hosenkan's games make no use of the `$6000-$6001` registers, other than
/// writing zero to both of them on startup, some implementations (such as the EverDrive N8's) do
/// not emulate the `$600x` registers." Hosenkan is 114's **submapper 0**, so 182 is that wiring
/// exactly, and the page's observation is a *behavioural* note rather than a second board — which
/// `req_core_100_mapper_182_tolerates_the_zero_its_games_write_at_startup` pins.
///
/// **Mesen 2 builds two different classes for the two numbers and this build follows neither of
/// them.** `MMC3_182` implements the reading 114's page explicitly refutes — "Previous
/// descriptions of Mapper 114/182 claim that a write to address `$C001` is equivalent to writing
/// both to an MMC3's `$C000` and `$C001` registers, and that writing to `$A001` has no effect. A
/// comparison of *Aladdin* (Mapper 114) and *Super Aladdin* (Mapper 4, by the same developer)
/// however indicates a **one-to-one mapping**, provided that MMC3A-like IRQ behavior is
/// enforced" — and `MMC3_114` gates its `$C000` on a flag the page does not describe. The page's
/// one-to-one table is what [`Mmc3Scramble::ADDRESS_SCRAMBLE`] holds.
///
/// # Where this board sits in `Board`
///
/// In [`super::IrqBoard`], boxed: every one of the six *is* the MMC3's board, counter included,
/// so every one answers **yes** to [`Mapper::irq_asserted`], which is that enum's entry
/// condition. It is boxed because it is an [`Mmc3`] plus its own registers and would otherwise
/// set the enum's size — MF-6b step 1 priced one byte of displacement in that payload at
/// +0.009 % to +0.027 % on eleven of lot PR-1's thirteen ROMs.
///
/// It is the first board to make `IrqBoard` forward [`Mapper::try_read_prg`], and 121 and 187 are
/// why: both answer a protection read below `$6000`, and *The King of Fighters '96* depends on
/// the second one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Scramble {
    /// The chip itself. Every register this board does not decode reaches it unchanged.
    mmc3: Mmc3,
    /// 114, 115, 121, 123, 182 or 187, for [`Mapper::mapper_number`].
    mapper_number: i32,
    /// Which row of [`Mmc3Scramble::ADDRESS_SCRAMBLE`] this board uses; 0 is the identity.
    address_scramble: usize,
    /// Which row of [`Mmc3Scramble::INDEX_SCRAMBLE`] this board uses; 0 is the identity.
    ///
    /// **The two are not the same choice**, and mapper 123 is why: its page gives an index table
    /// and no address table, and 114's own *Similar Mappers* says 123 "drops … register address
    /// scrambling while **retaining index register scrambling** (using the same pattern)".
    index_scramble: usize,
    /// Which port the NROM-like PRG override register is on, and how its bits are read.
    port: OverridePort,
    /// The NROM-like PRG override register, as written.
    override_register: u8,
    /// The outer CHR-ROM bank register of 114, 115 and 182 (`$6001`), and of 121 (`$5180`): one
    /// bit, CHR A18. Held as a `bool` because that is all any of the three pages gives it.
    chr_a18: bool,
    /// Whether CHR A18 is **also** driven by PPU A12, on the A9711 and the A98402.
    chr_a18_from_ppu_a12: bool,
    /// Mapper 121's `$5180`: the outer 256 KiB **PRG** bank of the A9713.
    prg_a18: bool,
    /// Mapper 121's protection index, the low two bits of the last `$5000-$5FFF` write, which
    /// the next read of that range answers from [`Mmc3Scramble::PROTECTION_ARRAY`].
    protection_index: u8,
    /// Mapper 121's `$8001` value with its **low six bits reversed**, which is what the
    /// protection table applies.
    protection_latch: u8,
    /// Mapper 121's `$8003`, six bits: which protection index is selected. **Zero means the
    /// MMC3's own PRG banking is in force**, which is the page's "All other indices: Restore
    /// normal MMC3 PRG-ROM banking" written as a state rather than an action.
    protection_mode: u8,
    /// Mapper 121: whether a *subsequent* `$8001` write re-applies the selected index. The
    /// page gives three indices — `$26`, `$28` and `$2A` — that "apply subsequent writes to
    /// `$8001` to this index as well", and every other valid one that does not.
    protection_sticky: bool,
    /// Mapper 121's three extended PRG banks, in window order from the top: `$E000`, `$C000`,
    /// `$A000`, which the page's indices `$26`, `$28` and `$2A` select.
    protection_banks: [u8; 3],
}

/// Which port a board's NROM-like PRG override register is on, and how its bits are read. Each
/// variant is one page's own diagram; see [`Mmc3Scramble::apply_override`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverridePort {
    /// 114 and 182: `$6000` under mask `$E001`, `D~[M.S. BBBb]`.
    Port6000Mask1,
    /// 115: `$6000` under mask `$E003`, `D~[MPN. PPPp]`, with PRG A18 in bit 6.
    Port6000Mask3,
    /// 123: `$5800` under mask `$F800`, `D~[.M31 .2S0]` — "notice the bit order".
    Port5800,
    /// 187: `$5000` under mask `$F001`, `D~[M.NB BBb.]`.
    Port5000,
    /// 121: **no** NROM override. Its `$8003` protection table banks `$A000`, `$C000` and
    /// `$E000` instead, and its `$5000-$5FFF` is the protection array.
    Protection,
}

impl Mmc3Scramble {
    /// The eight addresses of an MMC3's register window, indexed by
    /// [`Mmc3Scramble::port_index`].
    const MMC3_PORTS: [u16; 8] = [
        0x8000, 0x8001, 0xA000, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001,
    ];

    /// Which MMC3 register each of the eight written addresses means, as
    /// [`Mmc3Scramble::MMC3_PORTS`] indices — nesdev ("INES Mapper 114"), the two submappers'
    /// `Address written` / `MMC3 register meant` tables, read one-to-one as that page's own
    /// *Notes* require.
    ///
    /// | written | submapper 0 | submapper 1 |
    /// | --- | --- | --- |
    /// | `$8000` | `$A001` | `$A001` |
    /// | `$8001` | `$A000` | `$8001` |
    /// | `$A000` | `$8000` | `$8000` |
    /// | `$A001` | `$C000` | `$C001` |
    /// | `$C000` | `$8001` | `$A000` |
    /// | `$C001` | `$C001` | `$C000` |
    /// | `$E000` | `$E000` | `$E000` |
    /// | `$E001` | `$E001` | `$E001` |
    ///
    /// Row 0 is the identity, which is 115, 123, 187 and 121: mapper 123's page gives an index
    /// table and **no** address table, and 115's own *Similar Mappers* says it "does not scramble
    /// MMC3 register addresses and indices".
    const ADDRESS_SCRAMBLE: [[usize; 8]; 3] = [
        [0, 1, 2, 3, 4, 5, 6, 7],
        [3, 2, 0, 4, 1, 5, 6, 7],
        [3, 1, 0, 5, 2, 4, 6, 7],
    ];

    /// Which MMC3 bank register the low three bits of a `$8000` value mean — nesdev's
    /// `$8000 index written` / `MMC3 register meant` tables, which 114's two submappers and
    /// mapper 123 each carry. 123's is submapper 0's, which its own page says in so many words:
    /// mapper 123 "retain[s] index register scrambling (using the same pattern)".
    ///
    /// Row 0 is the identity, for 115, 187 and 121.
    const INDEX_SCRAMBLE: [[u8; 8]; 3] = [
        [0, 1, 2, 3, 4, 5, 6, 7],
        [0, 3, 1, 5, 6, 7, 2, 4],
        [0, 2, 5, 3, 6, 1, 7, 4],
    ];

    /// Mapper 121's protection array: "Returns the value from the four-byte protection array
    /// (`$83, $83, $42, $00`) indexed by the previous write to `$5000-$5FFF`" (nesdev, "INES
    /// Mapper 121").
    ///
    /// **FCEUX's second array is not implemented.** The page records it as a claim about another
    /// emulator's source rather than as its own finding — "FCEUX' source code claims that address
    /// bit 8 switches between two protection arrays, the second one being `$00, $02, $02, $03`"
    /// — and no game in its list is said to need it. Recorded here rather than guessed at.
    const PROTECTION_ARRAY: [u8; 4] = [0x83, 0x83, 0x42, 0x00];

    /// What mapper 187's protection read answers.
    ///
    /// nesdev ("INES Mapper 187"): "The actual values that are returned are unknown; *The King of
    /// Fighters '96* reads from here and only expects **bit 7** of the value being returned to be
    /// set." So the page states a constraint and not a value, and the value chosen is `$83` —
    /// mapper 121's array's own first entry, which satisfies the constraint and is what Mesen 2
    /// returns for that board's index 0 and 1. A recorded gap: any byte with bit 7 set would do.
    const PROTECTION_187: u8 = 0x83;

    /// The 8 KiB PRG banks mapper 121's and 115's outer bank line is above: 256 KiB.
    const OUTER_PRG_BANKS: i32 = 0x20;

    /// The 1 KiB CHR banks CHR A18 is above: 256 KiB.
    const OUTER_CHR_BANKS: i32 = 0x100;

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Option<Self> {
        let header = memory.rom().header();
        // 114's is the one submapper in this family: "Submapper 1 (different scrambling
        // pattern): Boogerman (SuperGame)". 182 is Hosenkan's board, which is submapper 0.
        let submapper114 = usize::from(mapper == 114 && header.is_nes20 && header.submapper == 1);
        // (address scramble, index scramble, override port)
        let (address_scramble, index_scramble, port) = match mapper {
            114 => (
                1 + submapper114,
                1 + submapper114,
                OverridePort::Port6000Mask1,
            ),
            182 => (1, 1, OverridePort::Port6000Mask1),
            115 => (0, 0, OverridePort::Port6000Mask3),
            123 => (0, 1, OverridePort::Port5800),
            187 => (0, 0, OverridePort::Port5000),
            121 => (0, 0, OverridePort::Protection),
            _ => return None,
        };

        let mut board = Self {
            // The number itself and not 4, which is the tightening MF-6a3 step 1 made for the
            // TQROM: `Mmc3Board::new` reads a submapper only for mapper 4, so no image of this
            // family can be taken for an MMC6, an MMC3A or a hard-wired-mirroring MMC3C. 114's
            // own submapper 1 is read above, where it belongs.
            mmc3: Mmc3::new(memory, mapper),
            mapper_number: mapper,
            address_scramble,
            index_scramble,
            port,
            override_register: 0,
            chr_a18: false,
            // nesdev ("INES Mapper 121" and "INES Mapper 187"), the same sentence on both pages:
            // "CHR A18 (the second half of CHR-ROM) is directly selected by PPU A12, regardless
            // of what the MMC3 is set to do."
            chr_a18_from_ppu_a12: mapper == 121 || mapper == 187,
            prg_a18: false,
            protection_index: 0,
            protection_latch: 0,
            protection_mode: 0,
            protection_sticky: false,
            protection_banks: [0; 3],
        };

        // 114's and 182's MMC3A counter; see `Mmc3Board::force_irq_revision_a`, whose note has
        // 115's counterpart and why it needs no call.
        if matches!(mapper, 114 | 182) {
            board.mmc3.force_irq_revision_a();
        }

        board.refresh();
        Some(board)
    }

    /// Where in [`Mmc3Scramble::MMC3_PORTS`] the address `address` sits, which is `$E001`'s
    /// three live bits packed: A13, A14 and A0.
    fn port_index(address: u16) -> usize {
        usize::from(((address >> 12) & 0x06) | (address & 0x01))
    }

    /// Recomputes the MMC3's windows from its own registers and lays this board's own banking
    /// over them.
    ///
    /// Called after **every** write the board takes, on [`super::Somari::refresh`]'s own
    /// argument: the chip keeps its registers and is asked to recompute from them, so the
    /// overlay is never counted twice and the board does not have to remember what it last
    /// applied.
    fn refresh(&mut self) {
        self.mmc3.refresh_banks();
        self.apply_override();
        self.apply_chr_a18();
    }

    /// The NROM-like PRG override, and mapper 121's protection banking in its place.
    ///
    /// Every one of the five pages that has an override says the same two things about it — bit 7
    /// is "0: Use PRG bank from MMC3 … 1: Ignore PRG bank from MMC3", and the field selects a
    /// "16 KiB PRG-ROM bank at CPU `$8000-$BFFF` **and** `$C000-$FFFF`" with one bit optionally
    /// "replace[d] … with CPU A14". So one body serves all five, and what differs is where the
    /// bank's four bits and the NROM-128/256 bit sit in the byte:
    ///
    /// | # | diagram | bank | the replaced bit |
    /// | ---: | --- | --- | --- |
    /// | 114, 182 | `M.S. BBBb` | bits 3-0 | bit 5 = `S` replaces the bank's bit 0 |
    /// | 115 | `MPN. PPPp` | bits 3-0, with bit 6 = PRG A18 above them | bit 5 = `N` |
    /// | 123 | `.M31 .2S0` | bits **5, 3, 2, 0** → bank bits 3, 1, 2, 0 | bit 1 = `S` |
    /// | 187 | `M.NB BBb.` | bits 4-1 | bit 5 = `N` |
    ///
    /// **123's bank bit 1 is register bit 3, which is the page's own prose and all four
    /// emulators' reading against the page's diagram** — Q-144 §1 holds the difference and
    /// MF-FIX applied it. nesdev's mapper-123 page contradicts itself: the register's diagram
    /// `[.M31 .2S0]` puts the bank's bit 1 at register bit **4** and leaves bit 3 unused, while
    /// the page's prose says to **ignore bits 0-3/5** twice, which only makes sense if bit 3 is
    /// one of the bank's. All four cross-checked projects read bit 3 — Mesen 2
    /// (`MMC3_123.h`, `(_exReg[0] & 0x08) >> 2`), FCEUX, Nestopia UE and puNES — so the diagram
    /// is the error. Under the old reading half of the `$5800` values loaded the wrong 16 KiB
    /// bank in *Mortal Kombat 3* (UNL-H2288) and *Earthworm Jim 2*. Register bit 4 is now the
    /// don't-care, which `q_144_mapper_123_takes_the_banks_bit_1_from_register_bit_3` pins from
    /// both sides.
    ///
    /// **115's PRG A18 is the page's and not Mesen 2's either.** Its bit 6 is annotated `PRG
    /// A18` with no "If M=1" qualifier, where every other field of that register has one, so the
    /// line is driven in MMC3 mode too; `MMC3_115.h` does not read bit 6 at all.
    fn apply_override(&mut self) {
        if self.port == OverridePort::Protection {
            self.apply_protection();
            return;
        }

        let register = self.override_register;
        // 115's `PRG A18`, which is above the MMC3's own bits and so applies in both modes.
        if self.port == OverridePort::Port6000Mask3 {
            self.prg_a18 = register & 0x40 != 0;
            let bits = if self.prg_a18 {
                Self::OUTER_PRG_BANKS
            } else {
                0
            };
            if let Some(banks) = self.mmc3.banks_mut() {
                banks.remap_prg_banks(Self::OUTER_PRG_BANKS - 1, bits);
            }
        }

        // "0: Use PRG bank from MMC3 … 1: Ignore PRG bank from MMC3", which is bit 7 on three
        // of the four diagrams and **bit 6 on 123's** -- `[.M31 .2S0]` puts `M` one place
        // right, bit 7 being a don't-care there.
        let enable = if self.port == OverridePort::Port5800 {
            0x40
        } else {
            0x80
        };
        if register & enable == 0 {
            return;
        }

        let (bank, nrom_256) = match self.port {
            OverridePort::Port6000Mask1 | OverridePort::Port6000Mask3 => {
                (register & 0x0F, register & 0x20 != 0)
            }
            // "notice the bit order": the bank's bits 3, 2, 1 and 0 are the register's 5, 2, 3
            // and 0, and register bit 4 is the don't-care. Q-144 §1 / MF-FIX: the page's diagram
            // labels bit 4, but the page's own prose says "ignore bits 0-3/5" twice and all four
            // emulators read bit 3.
            OverridePort::Port5800 => (
                ((register >> 2) & 0x08)
                    | (register & 0x04)
                    | ((register >> 2) & 0x02)
                    | (register & 0x01),
                register & 0x02 != 0,
            ),
            OverridePort::Port5000 => ((register >> 1) & 0x0F, register & 0x20 != 0),
            OverridePort::Protection => return,
        };

        let outer = if self.prg_a18 {
            Self::OUTER_PRG_BANKS / 2
        } else {
            0
        };
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };
        if nrom_256 {
            // "Replace bit 0 with CPU A14" (NROM-256): a 32 KiB view, the two halves of one
            // even/odd 16 KiB pair.
            let base = i32::from(bank & 0x0E) | outer;
            banks.map_prg_16k(0, base);
            banks.map_prg_16k(2, base | 1);
        } else {
            // "Do not replace bit 0" (NROM-128): the same 16 KiB at both windows.
            let base = i32::from(bank) | outer;
            banks.map_prg_16k(0, base);
            banks.map_prg_16k(2, base);
        }
    }

    /// Mapper 121's extended PRG banking: the three banks its `$8003` protection indices select,
    /// at `$A000`, `$C000` and `$E000`, with the A9713's outer 256 KiB bank over all four
    /// windows.
    ///
    /// nesdev ("INES Mapper 121"): "If an invalid index has been selected, the `$A000`/`$C000`/
    /// `$E000` banks selected via `$8003` **expire**, and normal MMC3 PRG-ROM banking is
    /// restored", which [`Mmc3Scramble::protection_mode`] of zero is. `$8000-$9FFF` keeps the
    /// MMC3's own bank in either case.
    ///
    /// The MMC3's bank is masked to the selected 256 KiB half before the outer bank is ORed in,
    /// which is Mesen 2's `page & 0x1F` and the only reading under which `$5180` is an *outer*
    /// bank at all.
    ///
    /// **Mesen 2 powers `$5180` on at `$80` and this build powers it on at zero.** That
    /// `Reset()` is labelled "Hack for Super 3-in-1" in its own source; the page gives the
    /// register no power-on value, and zero is the only one that does not silently start an A9711
    /// image — which has no `$5180` register at all — in the second half of its PRG-ROM.
    fn apply_protection(&mut self) {
        let outer = if self.prg_a18 {
            Self::OUTER_PRG_BANKS
        } else {
            0
        };
        let banks_mask = Self::OUTER_PRG_BANKS - 1;
        let protection = self.protection_mode != 0;
        let protection_banks = self.protection_banks;
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };

        banks.remap_prg_banks(banks_mask, outer);
        if !protection {
            return;
        }

        // `protection_banks[0]` is $E000, `[1]` $C000, `[2]` $A000 -- the page's indices $26,
        // $28 and $2A, which are the windows counted from the top.
        for (slot, bank) in [(3usize, 0usize), (2, 1), (1, 2)] {
            banks.map_prg_8k(
                slot,
                (i32::from(protection_banks[bank]) & banks_mask) | outer,
            );
        }
    }

    /// CHR A18: the outer bank register of 114, 115, 182 and 121, and the **PPU A12** wiring of
    /// the A9711 and the A98402.
    ///
    /// nesdev ("INES Mapper 121", "INES Mapper 187"), the same sentence on both: "CHR A18 (the
    /// second half of CHR-ROM) is directly selected by PPU A12, regardless of what the MMC3 is
    /// set to do. This means that the 'left' pattern table always uses the first and the 'right'
    /// pattern table always uses the second 256 KiB half of CHR-ROM."
    ///
    /// **Mesen 2 drives it from the MMC3's CHR mode instead, and this build does not follow
    /// it.** `MMC3_121.h` and `MMC3_187.h` both test `(slot < 4 && _chrMode == 0) || (slot >= 4
    /// && _chrMode == 1)`, which puts A18 on whichever windows the two 2 KiB registers drive —
    /// so flipping `$8000` bit 7 would move the two halves of CHR-ROM between the pattern tables.
    /// The page says "regardless of what the MMC3 is set to do", and names the pattern tables
    /// and not the registers. AMD-061 §1's rule applies: the page wins.
    ///
    /// Both lines are a wrap away from doing nothing on an image whose CHR-ROM is 256 KiB or
    /// less — [`BankWindows::map_chr_1k`] wraps the bank modulo the chip's banks — which is
    /// exactly what both pages say: "Only *Street Fighter Zero 2 '97* has 512 KiB of CHR-ROM",
    /// "Only *King of Fighters '96* has 512 KiB of CHR-ROM".
    fn apply_chr_a18(&mut self) {
        let outer = self.chr_a18;
        let from_a12 = self.chr_a18_from_ppu_a12;
        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };

        if outer {
            banks.or_chr_banks(0, Self::OUTER_CHR_BANKS);
        }
        if from_a12 {
            banks.or_chr_banks(BankWindows::CHR_WINDOWS / 2, Self::OUTER_CHR_BANKS);
        }
    }

    /// A write below `$8000`: the NROM override, the outer CHR bank, mapper 121's protection
    /// index and the A9713's outer bank. `true` when this board decoded the address.
    ///
    /// `#[inline(never)]`, which is the shape MF-6a2 step 3 measured and [`super::Somari`]'s
    /// `write_supervisor` already carries: this is one arm of a `Board::write_register` dispatch
    /// `NesBus::write_internal` reaches, and a cartridge of this family pays a `callq` per
    /// register write where nobody else pays anything.
    #[inline(never)]
    fn write_low(&mut self, address: u16, value: u8) -> bool {
        match self.port {
            // "Mask: Probably $E001" -- $6000 and $6001 repeating through $6000-$7FFF.
            OverridePort::Port6000Mask1 => {
                if address & 0xE000 != 0x6000 {
                    return false;
                }
                if address & 0x01 == 0 {
                    self.override_register = value;
                } else {
                    // "$6001: [.... ...+] Select 256 KiB CHR-ROM bank at PPU $0000-$1FFF."
                    self.chr_a18 = value & 0x01 != 0;
                }
            }
            // "Mask: $E003" -- $6000, $6001 and the read-only $6002 repeating every four bytes.
            OverridePort::Port6000Mask3 => {
                if address & 0xE000 != 0x6000 {
                    return false;
                }
                match address & 0x03 {
                    0 => self.override_register = value,
                    // "$6001: [.... ...C] CHR A18."
                    1 => self.chr_a18 = value & 0x01 != 0,
                    // $6002 is the solder-pad register and it is **read-only**; $6003 is not
                    // decoded. See `Mmc3Scramble::try_read_prg` on the pads themselves.
                    _ => return true,
                }
            }
            // "Mask: $F800" -- $5800-$5FFF.
            OverridePort::Port5800 => {
                if address & 0xF800 != 0x5800 {
                    return false;
                }
                self.override_register = value;
            }
            // "Mask: $F001" -- the even addresses of $5000-$5FFF. The odd ones are not decoded;
            // the whole range answers the protection read.
            OverridePort::Port5000 => {
                if address & 0xF001 != 0x5000 {
                    return false;
                }
                self.override_register = value;
            }
            // 121: "Protection Array Index ($5000-$5FFF), Mask: probably $F000, D~[.... ..II]",
            // and the A9713's "Outer Bank Register ($5180), Mask: $F180, D~[M... ....]".
            OverridePort::Protection => {
                if address & 0xF000 != 0x5000 {
                    return false;
                }
                self.protection_index = value & 0x03;
                if address & 0xF180 == 0x5180 {
                    self.prg_a18 = value & 0x80 != 0;
                    self.chr_a18 = value & 0x80 != 0;
                }
            }
        }

        self.refresh();
        true
    }

    /// Mapper 121's `$8001` and `$8003`, which are its protection latch and its protection
    /// index — nesdev ("INES Mapper 121"), both under mask `$E003`.
    ///
    /// > **Protection Latch (`$8001`).** If the previous write to the Protection Index register
    /// > (`$8003`) selected indices `$26`, `$28` or `$2A`, the value written to `$8001` — with
    /// > the lower six bits reversed — is immediately applied. Otherwise, it will only be applied
    /// > upon the next write to the `$8003`. **Regardless of the `$8003` setting, the MMC3 clone
    /// > will always receive the write as well.**
    /// >
    /// > **Protection Index (`$8003`).** Bits 0-5 select a protection index register. … Writing
    /// > to `$8003` also has the side effect of writing the same value to the MMC clone's
    /// > `$8000` register.
    ///
    /// So `$8003` is *both* the index and an MMC3 `$8000` write, and `$8001` is *both* the latch
    /// and an MMC3 `$8001` write. This returns whether the index table has to be re-run, and the
    /// caller does the MMC3 write either way.
    #[inline(never)]
    fn write_protection_port(&mut self, address: u16, value: u8) -> bool {
        match address & 0x03 {
            // The latch. "with the lower six bits reversed": D0-D5 end up as D5-D0.
            0x01 => {
                self.protection_latch = value.reverse_bits() >> 2;
                !self.protection_sticky
            }
            0x03 => {
                self.protection_mode = value & 0x3F;
                true
            }
            _ => false,
        }
    }

    /// Mapper 121's `$8003` table, run when the index is selected and again on a `$8001` write
    /// while the selected index "appl[ies] subsequent writes to `$8001` to this index as well".
    ///
    /// The page's own list, and `protection_banks` indexed from the top window:
    ///
    /// | index | what it does |
    /// | --- | --- |
    /// | `$26` | `$E000` from the latch, **and** subsequent `$8001` writes |
    /// | `$28` | `$C000` from the latch, and subsequent writes |
    /// | `$2A` | `$A000` from the latch, and subsequent writes |
    /// | `$2C` | `$E000` from the latch **if it is not zero**, no subsequent writes |
    /// | `$2F` | nothing, but the banks previously selected stay |
    /// | `$20`, `$29`, `$2B`, `$3C`, `$3F` | `$E000` from the latch, no subsequent writes |
    /// | anything else | "Restore normal MMC3 PRG-ROM banking" |
    fn run_protection_table(&mut self) {
        let latch = self.protection_latch;
        match self.protection_mode {
            0x26 => {
                self.protection_sticky = false;
                self.protection_banks[0] = latch;
            }
            0x28 => {
                self.protection_sticky = false;
                self.protection_banks[1] = latch;
            }
            0x2A => {
                self.protection_sticky = false;
                self.protection_banks[2] = latch;
            }
            0x2C => {
                self.protection_sticky = true;
                if latch != 0 {
                    self.protection_banks[0] = latch;
                }
            }
            0x20 | 0x29 | 0x2B | 0x3C | 0x3F => {
                self.protection_sticky = true;
                self.protection_banks[0] = latch;
            }
            // "Do nothing, but keep the $A000/$C000/$E000 banks previously selected via $8003."
            0x2F => {}
            // "All other indices: Restore normal MMC3 PRG-ROM banking."
            _ => self.protection_mode = 0,
        }
    }
}

impl Mapper for Mmc3Scramble {
    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    /// The chip's whole block first, then this board's own registers behind it. Nothing of the
    /// MMC3's own layout moves, and no earlier state can name any of these six numbers
    /// (REQ-CORE-090).
    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        writer.write_u8(self.override_register);
        writer.write_bool(self.chr_a18);
        writer.write_bool(self.prg_a18);
        writer.write_u8(self.protection_index);
        writer.write_u8(self.protection_latch);
        writer.write_u8(self.protection_mode);
        writer.write_bool(self.protection_sticky);
        writer.write_raw(&self.protection_banks);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        // The chip's own `load_registers` recomputes its windows from its registers, which is
        // what `refresh` below wants to find: the overlay is never applied twice.
        self.mmc3.load_registers(reader)?;
        self.override_register = reader.read_u8()?;
        self.chr_a18 = reader.read_bool()?;
        self.prg_a18 = reader.read_bool()?;
        self.protection_index = reader.read_u8()?;
        self.protection_latch = reader.read_u8()?;
        self.protection_mode = reader.read_u8()?;
        self.protection_sticky = reader.read_bool()?;
        reader.read_into(&mut self.protection_banks)?;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        // `Mmc3Board::reset` does not touch the revision, which is wiring and not state -- see
        // its own `dots_per_clock` note -- so 114's and 182's MMC3A counter survives a reset.
        self.override_register = 0;
        self.chr_a18 = false;
        self.prg_a18 = false;
        self.protection_index = 0;
        self.protection_latch = 0;
        self.protection_mode = 0;
        self.protection_sticky = false;
        self.protection_banks = [0; 3];
        self.refresh();
    }

    fn irq_asserted(&self) -> bool {
        self.mmc3.irq_asserted()
    }

    fn prg_ram_enabled(&self) -> bool {
        self.mmc3.prg_ram_enabled()
    }

    fn prg_ram_writable(&self) -> bool {
        self.mmc3.prg_ram_writable()
    }

    /// The protection reads of mappers 121 and 187, both below `$6000`.
    ///
    /// * **121**: "Returns the value from the four-byte protection array (`$83, $83, $42, $00`)
    ///   indexed by the previous write to `$5000-$5FFF`", mask probably `$F000`.
    /// * **187**: "Protection Read (`$5000-$5FFF`) … *The King of Fighters '96* reads from here
    ///   and only expects bit 7 of the value being returned to be set." See
    ///   [`Mmc3Scramble::PROTECTION_187`].
    ///
    /// **Mapper 115's `$6002` solder-pad register is a recorded gap.** Its page gives
    /// "`D~[.... .VVV]` Solder Pad value" and never says what the pads are set to on any of the
    /// SFC-02B/-03/-004 boards, and no game in its list is said to read them, so the window is
    /// left as open bus rather than answered with a guess. (Mesen 2 answers a different register
    /// there — a byte written to `$5080`, which is on no page.)
    ///
    /// Every other address falls through to the default, which is the MMC3's own PRG read.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address & 0xF000 == 0x5000 {
            return match self.port {
                OverridePort::Protection => {
                    Some(Self::PROTECTION_ARRAY[usize::from(self.protection_index & 0x03)])
                }
                OverridePort::Port5000 => Some(Self::PROTECTION_187),
                _ => None,
            };
        }

        self.banks()
            .and_then(|banks| banks.read_prg(address, memory.rom().prg_rom(), memory.prg_ram()))
    }

    /// This board's own registers first, then the chip's — with mapper 121's two ports being
    /// **both** at once, which is what its page requires.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            self.write_low(address, value);
            return;
        }

        // 121's `$8001` and `$8003`: the latch and the index, and the MMC3 "will always receive
        // the write as well".
        if self.port == OverridePort::Protection && address < 0xA000 {
            if self.write_protection_port(address, value) {
                self.run_protection_table();
            }

            // "Writing to $8003 also has the side effect of writing the same value to the MMC
            // clone's $8000 register": $8003 & $E001 is $8001 on an MMC3, so the address has to
            // be rewritten rather than passed through.
            let port = if address & 0x03 == 0x03 {
                0x8000
            } else {
                address
            };
            self.mmc3.write_register(memory, port, value);
            self.refresh();
            return;
        }

        let index = Self::port_index(address);
        let port = Self::MMC3_PORTS[Self::ADDRESS_SCRAMBLE[self.address_scramble][index]];
        // "Bits 6 and 7 of the value written to $8000 are unchanged": the index scramble reaches
        // the low three bits of the register that *means* the MMC3's $8000 and nothing else. The
        // boards that do not scramble indices are left alone entirely rather than passed through
        // an identity row, so their `$8000` keeps bits 3-5 -- which an MMC3 ignores, but which
        // its savestate carries.
        let value = if port == 0x8000 && self.index_scramble != 0 {
            (value & 0xC0) | Self::INDEX_SCRAMBLE[self.index_scramble][usize::from(value & 0x07)]
        } else {
            value
        };

        self.mmc3.write_register(memory, port, value);
        self.refresh();
    }

    /// `None`, which is the body the MMC5 and the RAMBO-1 already give: these boards' CHR
    /// fetches clock the A12 counter, so the PPU takes the full path.
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        self.mmc3.chr_peek_view(memory)
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        self.mmc3.ppu_peek(memory, address, fetch)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.mmc3.ppu_write(memory, address, value);
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        self.mmc3.on_ppu_address_bus(memory, address, clock);
    }
}
