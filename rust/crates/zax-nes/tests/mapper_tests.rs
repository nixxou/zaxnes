//! Mapper behaviour on numbered-bank images (REQ-CORE-100, REQ-CORE-102, REQ-CORE-039,
//! REQ-CORE-033, AMD-021), including the save states and the battery RAM.

// Test values are small: the narrowing casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::unwrap_used
)]

mod common;

use common::mapper_fixture::{load, MapperMachine, MapperRom};
use zax_nes::cartridges::{Cartridge, ChrFetch};
use zax_nes::NametableMirroring as Nm;
use zax_nes::NesCore;

/// The four pages `Cartridge::nametable_page` answers, which is what the PPU reads.
fn nametable_pages(core: &zax_nes::NesCore) -> [i32; 4] {
    let cartridge = core.cartridge().unwrap();
    std::array::from_fn(|table| cartridge.nametable_page(i32::try_from(table).unwrap()))
}

fn peek(core: &zax_nes::NesCore, address: u16) -> u8 {
    core.cartridge()
        .unwrap()
        .ppu_peek(address, ChrFetch::Background)
}

/// Writes CHR through the board's own path, which is the only way to reach a second chip:
/// `Cartridge::ppu_write` dispatches to the board, and on the TQROM (119) that is the chip-select
/// decode, on the Irem LROG017 (77) it is the $0800-$1FFF half of the split, and on the Oeka
/// Kids board (96) it is the 4 KiB CHR-RAM page the PPU address bus last latched, on the GTROM
/// (111) it is the 8 KiB CHR half of a 32 KiB chip whose other half is the nametables, and on the
/// two Chinese-RPG boards (162, 163) it is the 8 KiB chip whose A12 their switch overrides, and on
/// the FS305 (178) it is the 8 KiB chip no register of the board can move at all (Q-154), and on
/// the AA6023 (268) it is the CHR-RAM chip `$xxx4`'s mixed mode puts behind a 2 KiB pair of
/// windows, and on the Sachen 8259 (141) it is the unbanked 8 KiB of the 2M-RAM-COB board, whose
/// CHR registers drive nothing at all.
#[cfg(any(
    feature = "board-262",
    feature = "board-29",
    feature = "board-77",
    feature = "board-96",
    feature = "board-111",
    feature = "board-119",
    feature = "board-162",
    feature = "board-163",
    feature = "board-164",
    feature = "board-178",
    feature = "board-218",
    feature = "board-268",
    feature = "board-sachen8259"
))]
fn chr_write(core: &mut NesCore, address: u16, value: u8) {
    core.cartridge_mut().unwrap().ppu_write(address, value);
}

#[test]
// The function *is* the two tables REQ-CORE-100 and REQ-CORE-102 ask for, plus the page or
// lot each addition came from. MF-UNL-2 step 3's six numbers are the ones that crossed the
// threshold.
#[allow(clippy::too_many_lines)]
fn req_core_100_every_required_mapper_is_registered() {
    // REQ-CORE-100's own table.
    let required: [i32; 31] = [
        0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 13, 15, 16, 18, 21, 22, 23, 25, 27, 28, 30, 32, 34, 41, 64,
        66, 71, 87, 101, 225, 228,
    ];
    for mapper in required {
        assert!(
            Cartridge::SUPPORTED_MAPPERS.contains(&mapper),
            "mapper {mapper} of REQ-CORE-100 is not registered"
        );
    }

    // "Ajouter des mappers au-delà de la liste est permis" (REQ-CORE-102), and these are the
    // ones this build adds. Listed so that an accidental addition is caught as surely as an
    // accidental removal.
    //
    // * 156, Daou 306, which runs Metal Force (Korea), a widescreen target;
    // * 69, Sunsoft FME-7 / 5A / 5B (MF-3 step 3);
    // * 118, TxSROM, which shares the MMC3's board (MF-3 step 5);
    // * 180, the other 74-series wiring of the UxROM board (MF-3 step 6);
    // * 78, the Irem IF-12 and the Jaleco board of Uchuusen (MF-3 step 7);
    // * 24 and 26, Konami VRC6 and VRC6b (MF-5a step 1);
    // * 19 and 210, the Namco 163 and the Namcot 175 and 340 cut from it (MF-5a step 4);
    // * 85, Konami VRC7 (MF-5b step 1);
    // * 206, the Namco 108 / DxROM, with 76, 88, 95 and 154 as its four parameters
    //   (MF-6a step 1);
    // * 33 (Taito TC0190), 97 (Irem TAM-S1), 140 (Jaleco JF-11/14), 184 (Sunsoft-1) and
    //   89 and 93 (Sunsoft-2 on its two boards) (MF-6a step 2);
    // * 70 and 152 (Bandai 74161/7432 without and with mirroring control), 75 (Konami VRC1)
    //   and 232 (Camerica BF9096) (MF-6a step 3);
    // * 94 (HVC-UN1ROM, a wiring of the UxROM board), 185 (CNROM with the security diodes) and
    //   155 (the MMC1 with no PRG-RAM disable) (MF-6a step 4);
    // * 73 (Konami VRC3) (MF-6b step 1);
    // * 65 (Irem H3001) and 67 (Sunsoft-3), the two plain CPU-cycle down counters
    //   (MF-6b step 2);
    // * 105 (the Nintendo NES-EVENT board of Nintendo World Championships 1990) (MF-6b step 3);
    // * 48 (the Taito TC0690, the TC0190 of mapper 33 with the MMC3's scanline counter)
    //   (MF-6b step 4);
    // * 158 (Tengen's 800037, the RAMBO-1 with CHR A17 on CIRAM A10) (MF-6b step 5);
    // * 119 (the Nintendo TQROM) and 47 and 37 (Nintendo's two MMC3 multicarts), each behind
    //   its own Cargo feature (MF-6a3 step 1) -- see `docs/review/BOARD-FEATURES.md`;
    // * 116 (the Gouder SOMARI-P and its Huang ASIC), the one number REQ-CORE-101 leaves
    //   optional, behind `board-116` (MF-6a3 step 4, answering Q-130 question 1);
    // * 153 (the Bandai FCG board with an LZ93D50 and 8 KiB of battery-backed WRAM), behind
    //   `board-153` (MF-6c step 1);
    // * 80 and 207 (the Taito X1-005 and its single-screen wiring) and 82 and 552 (the Taito
    //   X1-017 with the two bank orders), behind `board-taito-x1` (MF-6c step 2);
    // * 157 (the Datach Joint ROM System) and 159, the two Bandai LZ93D50 boards with a serial
    //   EEPROM, behind `board-eeprom` (MF-6c step 3);
    // * 68 (the Sunsoft-4), behind `board-68`, with its CHR-ROM nametable mode behind the second
    //   feature `board-68-chr-nametables` (MF-6c step 4);
    // * 77 (the Irem LROG017 of Napoleon Senki), behind `board-77` -- the first member of
    //   `DecodeBoard`, this project's second shared `Board` variant (MF-6c step 6);
    // * 72 and 92 (Jaleco's JF-17 and JF-19) and 86 (the JF-13), the three boards with a
    //   uPD7756C speech chip this build does not sound, behind `board-jaleco-72-92-86`
    //   (MF-6c step 7);
    // * 96 (the Bandai Oeka Kids board, whose CHR page is latched off the PPU address bus),
    //   behind `board-96` (MF-6c step 8);
    // * 111 (the Membler Industries GTROM / Cheapocabra, seven homebrew titles), behind
    //   `board-111` (MF-6c step 9);
    // * 36, 132, 136, 147, 172 and 173, the six boards of the TXC 05-00002-010 / Sachen JV001
    //   protection ASIC -- one chip, one `DecodeBoard` variant -- behind `board-jv001`
    //   (MF-UNL-1 step 1);
    // * 150 and 243, Sachen's eight-register ASIC -- one class in Mesen 2, one `DecodeBoard`
    //   variant here -- behind `board-sachen-8reg` (MF-UNL-1 step 2);
    // * 133, the Sachen 72008's one latch, behind `board-133` (MF-UNL-1 step 3);
    // * 74, 192, 194 and 195, the other four members of mapper 119's CHR-RAM family -- four
    //   `const` monomorphisations of the MMC3's board behind one `IrqBoard` variant -- behind
    //   `board-mmc3-chrram` (MF-UNL-2 step 2);
    // * 114, 115, 121, 123, 182 and 187, the six MMC3 clones with scrambled registers, an outer
    //   CHR bank and an NROM-like PRG override -- one board with six wirings, one boxed
    //   `IrqBoard` variant -- behind `board-mmc3-scramble` (MF-UNL-2 step 3);
    // * 263, the Rex Soft UNL-KOF97, behind `board-263` (MF-UNL-2 step 4);
    // * 189, the TXC 01-22017/01-22018 and the Yoko FC-001EM, behind `board-189`
    //   (MF-UNL-2 step 5);
    // * 79 and 146, the AVE NINA-03/06 and the Sachen 3015 / SA-016 that nesdev's one page for
    //   both numbers calls "functionally identical" -- one `DecodeBoard` variant and one latch
    //   at $4100 -- behind `board-nina03` (MF-UNL-3 step 1);
    // * 38 (Bit Corp. UNL-PCI556), 107 (Magicseries), 145 (Sachen SA-72007) and 149 (Sachen
    //   SA-0036), four of lot MF-UNL-3's seven single-latch boards: **one** `DecodeBoard`
    //   variant for all seven with a runtime field saying which, one Cargo feature each
    //   (`board-38`, `board-107`, `board-145`, `board-149`) because the owner's rule is one
    //   feature per board that costs (MF-UNL-3 step 2);
    // * 143 (the Sachen TCA-01, an NROM board whose only register is a protection read) and 144
    //   (the AGCI 50282 of Death Race, mapper 11's board with a 300 ohm resistor on CPU D0), two
    //   more of the same variant, behind `board-143` and `board-144` (MF-UNL-3 step 3);
    // * 190 (the Zemina board of Magic Kid Googoo), the last of that variant's seven and the
    //   only one with more than one window, behind `board-190` (MF-UNL-3 step 4);
    // * 35, 90, 209 and 211, the J.Y. Company ASIC -- one chip, four PCB configurations, one
    //   boxed `IrqBoard` variant -- behind `board-jy-asic` (MF-UNL-4). 209 is the page's
    //   "standard implementation", 211 and 35 are the same board (Q-149 and Q-148) and 90 is the
    //   one with a jumper that inhibits its nametable registers.
    // * 163, the 南晶 (Nanjing) FC-001, the first of lot MF-UNL-5's three Chinese-RPG boards and
    //   the second board in this core whose CHR window is latched off the **PPU address bus** --
    //   CHR A12 from "PPU A9 latched on last rise of PPU A13" -- behind `board-163`
    //   (MF-UNL-5 step 1);
    // * 162, the 外星 (Waixing) FS304, the same four ports and the same CHR-RAM latch with a PRG
    //   composition of its own -- one `ChrWindowKind` and **no** second variant -- behind
    //   `board-162` (MF-UNL-5 step 2);
    // * 112, the board its page calls "quite similar to iNES Mapper 206 ... however, the registers
    //   appears to be scrambled a bit" -- three ports at $8000/$A000/$E000 and the 206's eight
    //   register indices rotated by six, which is **one `bool` on `Namcot108`** and not a board --
    //   behind `board-112` (MF-UNL-7 step 2, Q-161);
    // * 193, NTDEC's TC-112 -- five registers at $6000-$6004 under mask $E007, three CHR windows
    //   of **unequal** width and one switchable PRG window of four -- which shares `Namcot108`'s
    //   window table and savestate block and nothing else, behind `board-193` (MF-UNL-7 step 3);
    // * 298, the unlicensed *Lethal Weapon*: "a VRC4 clone in the VRC4b configuration" whose one
    //   documented difference is that "$F003 only acknowledges a pending IRQ, but it does not move
    //   the 'A' control bit to the 'E' control bit" -- one `bool` on `Vrc2Vrc4` and no variant --
    //   behind `board-298` (MF-UNL-7 step 4);
    // * 29, the Sealie Computing RET-CUFROM revD of *Glider*: one `[...P PPCC]` register over the
    //   whole of $8000-$FFFF, two bits of 8 KiB CHR-RAM bank and three of 16 KiB PRG bank, an
    //   **eighth** `LatchBoard` variant and the first of them with CHR-RAM -- behind `board-29`
    //   (MF-UNL-7 step 6);
    // * 218, the single-chip homebrew board of *Magic Floor* -- no register, no latch and **no
    //   CHR memory**, the console's own 2 KiB of nametable RAM answering the pattern tables as
    //   well, with the header's byte 6 saying which PPU address line drives CIRAM A10 -- behind
    //   `board-218` (MF-UNL-7 step 5);
    // * 137, 138, 139 and 141, the Sachen 8259 -- one chip with four wirings (the 8259D, B, C and
    //   A, in that order of number), an index/data pair at $4100/$4101 under mask $C101 and a
    //   per-wiring CHR shift -- one `DecodeBoard` variant, behind `board-sachen8259`
    //   (MF-UNL-7 step 1). The family page's own disclaimer governs all four and **Q-160** records
    //   the two places it and mapper 137's page contradict each other;
    // * 12, the Gouder SL-5020B: the same **Huang-1** ASIC `board-116` carries, on a PCB where it
    //   "functions exclusively as an MMC3A clone", with a GAL16V8 adding an outer CHR register at
    //   $4132 and a language-bit read at the same address -- behind `board-huang1` (MF-UNL-8
    //   step 1, Q-164). Submapper 1, the Front Fareast Magic Card 4M, is refused at load;
    // * 14, the Gouder SL-1632 (`UNL-SL1632`): the same ASIC with a PAL that "moves the supervisor
    //   register address to $A131, **overlapping the MMC3 and VRC2 registers there**" -- two modes
    //   and no MMC1, and **three** independent CHR A18 bits over the unequal window groups
    //   $0000-$0FFF, $1000-$17FF and $1800-$1FFF -- behind `board-huang1` (MF-UNL-8 step 2,
    //   Q-165). MF-6b step 6 decided the number out of mapper 116's board on those differences;
    // * 83 and 264, the Cony / Yoko chip-on-board ASIC -- "the same chip-on-board ASIC" on five
    //   PCBs, of which this build serves four: a PRG base, a mode register, a 16-bit counter whose
    //   clock is selectable between CPU M2 and PPU A12, four 8 KiB PRG registers, eight 1 KiB CHR
    //   registers (four 2 KiB ones on 83.1 and 264), a solder-pad read and four bytes of scratch
    //   RAM -- behind `board-cony` (MF-UNL-8 step 3). Submapper 2, the one PCB with PRG-NVRAM at
    //   $6000, is refused at load (Q-167);
    // * 91, the JY830623C / YY840238C and the EJ-006-1 -- two boards that differ by **one mask
    //   bit** ($F003 against $F007) and the four registers it makes reachable, with a PA12 counter
    //   and hard-wired mirroring on one and an M2 counter and two mirroring ports on the other,
    //   plus an outer bank register whose diagram is an **address** -- behind `board-91`
    //   (MF-UNL-8 step 4, Q-168);
    // * 178, the 外星 FS305 / 南晶 NJ0430, four registers at **$4800-$4803**, four PRG banking
    //   modes, mirroring from a register and a **banked** 8 KiB PRG-RAM window -- and **no**
    //   PPU-driven CHR window, which is what its own page says against the census's section
    //   heading (Q-154) -- behind `board-178` (MF-UNL-5 step 3);
    // * 164, the 东达 PEC-9588 / 燕城 cy2000-3: five ports at $5000/$5100/$5200/$5300/$5500, two
    //   PRG banking modes with two shapes of semi-fixed bank, mirroring from $5300 behind the mode
    //   bit, a **93C66 512-byte serial EEPROM over Microwire** -- a third EEPROM shape beside
    //   `board-eeprom`'s two I2C parts, sharing its persistence path and not its code -- and the
    //   **1 bpp video mode**, CHR A3 from PPU A0 and CHR A12 from PPU A9 "both latched on last
    //   rise of PPU A13" -- behind `board-164` (MF-UNL-10, Q-170);
    // * 215, the Sugar Softec **UNL-8237** (submapper 0) and **UNL-8237A** (submapper 1): an
    //   MMC3 clone whose register addresses and bank indices are permuted by one of **eight**
    //   patterns a register at $5007 selects at run time, with outer PRG and CHR banks at $5001
    //   and an NROM-like PRG override at $5000 -- behind `board-215` (MF-UNL-11 step 1, Q-171);
    // * 208, the unnamed Gouder board of the Street Fighter IV backport: an MMC3 clone banking
    //   PRG-ROM 32 KiB at a time from a register at $4800/$6800 that also drives the mirroring,
    //   with a **256-byte protection lookup table** -- an index at $5000-$57FF and four registers
    //   at $5800-$5FFF holding the CPU datum XORed with the indexed entry -- behind `board-208`
    //   (MF-UNL-11 step 2, Q-171);
    // * 219, the Kasheng A9461: an MMC3 clone with a second **extended** banking mode that $8002
    //   bit 5 opens -- four independent 8 KiB PRG windows with a bit-reversed bank field, a CHR
    //   bank-number latch, a "pseudo 2 KiB" 1 KiB switch whose quarter number is also bit 0 of
    //   its bank, and a plain 1 KiB switch -- over a two-bit 128 KiB outer PRG and CHR bank at
    //   $5002/$5003 -- behind `board-219` (MF-UNL-11 step 3, Q-171);
    //
    // **The expected list is built and not written**, because the board features decide which
    // numbers the registry advertises. The numbers of REQ-CORE-100's own table are in no
    // feature, so the loop above holds in every configuration; these are the additions, and a
    // featured one is expected exactly when its feature is on. An accidental addition is still
    // caught as surely as an accidental removal.
    let extra: Vec<i32> = Cartridge::SUPPORTED_MAPPERS
        .iter()
        .copied()
        .filter(|mapper| !required.contains(mapper))
        .collect();
    let small = cfg!(feature = "board-small-boards");
    let namcot = cfg!(feature = "board-namcot108");
    let multicart = cfg!(feature = "board-mmc3-multicart");
    let taito_x1 = cfg!(feature = "board-taito-x1");
    let eeprom = cfg!(feature = "board-eeprom");
    let jaleco = cfg!(feature = "board-jaleco-72-92-86");
    let jv001 = cfg!(feature = "board-jv001");
    let sachen = cfg!(feature = "board-sachen-8reg");
    let chrram = cfg!(feature = "board-mmc3-chrram");
    let scramble = cfg!(feature = "board-mmc3-scramble");
    let nina03 = cfg!(feature = "board-nina03");
    let jy = cfg!(feature = "board-jy-asic");
    let sachen8259 = cfg!(feature = "board-sachen8259");
    let huang1 = cfg!(feature = "board-huang1");
    let cony = cfg!(feature = "board-cony");
    let additions: [(i32, bool); 113] = [
        (12, huang1),
        (14, huang1),
        (19, true),
        (24, true),
        (26, true),
        (29, cfg!(feature = "board-29")),
        (33, small),
        (35, jy),
        (36, jv001),
        (37, multicart),
        (38, cfg!(feature = "board-38")),
        (47, multicart),
        (48, true),
        (65, true),
        (67, true),
        (68, cfg!(feature = "board-68")),
        (69, true),
        (70, small),
        (72, jaleco),
        (73, true),
        (74, chrram),
        (75, small),
        (76, namcot),
        (77, cfg!(feature = "board-77")),
        (78, true),
        (79, nina03),
        (80, taito_x1),
        (82, taito_x1),
        (83, cony),
        (85, true),
        (86, jaleco),
        (88, namcot),
        (89, small),
        (90, jy),
        (91, cfg!(feature = "board-91")),
        (92, jaleco),
        (93, small),
        (94, cfg!(feature = "board-94")),
        (95, namcot),
        (96, cfg!(feature = "board-96")),
        (97, small),
        // MF-VS step 4: mapper 99 is the VS. System board itself and 151 the VRC1 on a VS board,
        // two numbers that exist only on that machine.
        (99, cfg!(feature = "board-vs-system")),
        (105, cfg!(feature = "board-105")),
        (107, cfg!(feature = "board-107")),
        (111, cfg!(feature = "board-111")),
        (112, cfg!(feature = "board-112")),
        (114, scramble),
        (115, scramble),
        (116, cfg!(feature = "board-116")),
        (118, true),
        (119, cfg!(feature = "board-119")),
        (121, scramble),
        (123, scramble),
        (132, jv001),
        (133, cfg!(feature = "board-133")),
        (136, jv001),
        (137, sachen8259),
        (138, sachen8259),
        (139, sachen8259),
        (140, small),
        (141, sachen8259),
        (143, cfg!(feature = "board-143")),
        (144, cfg!(feature = "board-144")),
        (145, cfg!(feature = "board-145")),
        (146, nina03),
        (147, jv001),
        (149, cfg!(feature = "board-149")),
        (150, sachen),
        (151, cfg!(feature = "board-vs-system")),
        (152, small),
        (153, cfg!(feature = "board-153")),
        (154, namcot),
        (155, cfg!(feature = "board-155")),
        (156, true),
        (157, eeprom),
        (158, cfg!(feature = "board-158")),
        (159, eeprom),
        (162, cfg!(feature = "board-162")),
        (163, cfg!(feature = "board-163")),
        (164, cfg!(feature = "board-164")),
        (172, jv001),
        (173, jv001),
        (178, cfg!(feature = "board-178")),
        (180, true),
        (182, scramble),
        (184, small),
        (185, small),
        (187, scramble),
        (189, cfg!(feature = "board-189")),
        (190, cfg!(feature = "board-190")),
        (192, chrram),
        (193, cfg!(feature = "board-193")),
        (194, chrram),
        (195, chrram),
        (206, namcot),
        (207, taito_x1),
        (208, cfg!(feature = "board-208")),
        (209, jy),
        (210, true),
        (211, jy),
        (215, cfg!(feature = "board-215")),
        (218, cfg!(feature = "board-218")),
        (219, cfg!(feature = "board-219")),
        (232, small),
        (243, sachen),
        (246, cfg!(feature = "board-246")),
        (262, cfg!(feature = "board-262")),
        (263, cfg!(feature = "board-263")),
        (264, cony),
        (268, cfg!(feature = "board-268")),
        (292, cfg!(feature = "board-292")),
        (298, cfg!(feature = "board-298")),
        (552, taito_x1),
    ];
    let expected: Vec<i32> = additions
        .iter()
        .filter(|(_, on)| *on)
        .map(|(mapper, _)| *mapper)
        .collect();
    assert_eq!(expected, extra);

    // Ascending, which is what the registry's own comment promises.
    assert!(Cartridge::SUPPORTED_MAPPERS.windows(2).all(|p| p[0] < p[1]));
}

#[test]
fn req_core_102_any_other_mapper_is_refused() {
    // **116 is no longer here**, and that is Q-130 question 1 answered. REQ-CORE-101 leaves the
    // number optional and that ticket's provisional choice was "not implemented at all, so
    // REQ-CORE-011 refuses it"; MF-6a3 step 4 implements it instead, behind `board-116`, so the
    // sweep drops it exactly when that feature is on. REQ-CORE-102's own acceptance criterion
    // (mapper 200) is untouched either way, which is the other half of Q-130.
    let mut refused = vec![6usize, 177, 200];
    if !cfg!(feature = "board-116") {
        refused.insert(1, 116);
    }
    for mapper in refused {
        assert!(!Cartridge::is_supported(mapper as i32));
        let mut core = zax_nes::NesCore::new();
        let load = core.load_rom(&MapperRom::new(mapper).image(), "Other.nes");
        assert!(!load.success);
        assert!(load.error.unwrap().contains(&mapper.to_string()));
        assert!(!core.is_running());
    }
}

// ---------------------------------------------------------------- MMC1 (1)

#[test]
fn req_core_100_mmc1_powers_on_with_the_last_bank_at_c000() {
    let core = MapperRom::new(1).prg(16).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
}

#[test]
fn req_core_100_mmc1_switches_16_kib_at_8000_in_prg_mode_3() {
    let mut core = MapperRom::new(1).prg(16).load();
    core.mmc1_write(0x8000, 0x0F); // Control: PRG mode 3, 4 KiB CHR, horizontal.
    core.mmc1_write(0xE000, 0x03);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    assert_eq!(Nm::Horizontal, core.mirroring());
}

#[test]
fn req_core_100_mmc1_fixes_the_first_bank_at_8000_in_prg_mode_2() {
    let mut core = MapperRom::new(1).prg(16).load();
    core.mmc1_write(0x8000, 0x0A); // PRG mode 2, 8 KiB CHR, vertical.
    core.mmc1_write(0xE000, 0x05);
    assert_eq!([0, 1, 10, 11], core.prg_windows());
    assert_eq!(Nm::Vertical, core.mirroring());
}

#[test]
fn req_core_100_mmc1_switches_32_kib_in_prg_modes_0_and_1() {
    let mut core = MapperRom::new(1).prg(16).load();
    core.mmc1_write(0x8000, 0x02); // PRG mode 0, 8 KiB CHR, vertical.
    core.mmc1_write(0xE000, 0x07); // The low bit is ignored: 16 KiB bank 6, not 7.
    assert_eq!([12, 13, 14, 15], core.prg_windows());
}

#[test]
fn req_core_100_mmc1_switches_chr_in_8_and_4_kib_units() {
    let mut core = MapperRom::new(1).prg(8).chr(32).load();
    core.mmc1_write(0x8000, 0x0C); // 8 KiB CHR.
    core.mmc1_write(0xA000, 0x03); // Bank 3 of 4 KiB units: the low bit is ignored.
    assert_eq!([8, 9, 10, 11, 12, 13, 14, 15], core.chr_windows());
    core.mmc1_write(0x8000, 0x1C); // 4 KiB CHR.
    core.mmc1_write(0xA000, 0x03);
    core.mmc1_write(0xC000, 0x06);
    assert_eq!([12, 13, 14, 15, 24, 25, 26, 27], core.chr_windows());
}

#[test]
fn req_core_100_mmc1_takes_the_512_kib_bank_bit_from_the_chr_register() {
    // SUROM: the PRG register only reaches 256 KiB and bit 4 of the CHR register is A18,
    // with the fixed window following it into the same half.
    let mut core = MapperRom::new(1).prg(64).load();
    core.mmc1_write(0x8000, 0x0C);
    core.mmc1_write(0xE000, 0x02);
    assert_eq!([4, 5, 30, 31], core.prg_windows());
    core.mmc1_write(0xA000, 0x10);
    assert_eq!([36, 37, 62, 63], core.prg_windows());
}

#[test]
fn req_core_100_mmc1_ignores_a_write_on_the_cycle_after_another() {
    // A read-modify-write instruction writes twice in a row; the chip only takes the first
    // (nesdev, "MMC1").
    let mut core = MapperRom::new(1).prg(16).load();
    core.mmc1_write(0x8000, 0x0F);
    // Five accepted writes load the register; interleaving an immediate second write per
    // bit must not change the result.
    for bit in 0..5 {
        core.mapper_write(0xE000, (0x03 >> bit) & 0x01);
        // Same cycle as the write above, so its bit 0 is dropped. Bit 7 is clear: the reset
        // bit would be taken whatever the cycle (MR-1-1), which
        // `mr_1_1_mmc1_reset_bit_bypasses_the_consecutive_write_filter` pins.
        core.write_memory(0xE000, 0x7F);
    }

    assert_eq!([6, 7, 14, 15], core.prg_windows());
}

#[test]
fn req_core_100_mmc1_disables_prg_ram_with_bit_4_of_the_prg_register() {
    let mut core = MapperRom::new(1).prg(16).battery().load();
    core.write_memory(0x6000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6000));
    core.mmc1_write(0xE000, 0x10);
    core.write_memory(0x6000, 0xA5);
    assert_eq!(0, core.read_memory(0x6000));
    core.mmc1_write(0xE000, 0x00);
    assert_eq!(0x5A, core.read_memory(0x6000));
}

/// Holy Mapperel `M1_P128K` and `M1_P128K_CR8K`, detailed digits `4000` (REQ-CORE-100).
///
/// nesdev ("MMC1"), SNROM: the board "uses the upper CHR bank select line coming out of the
/// mapper (CHR A16; bit 4 of bank number) as an additional chip enable for the PRG-RAM".
#[test]
fn req_core_100_mmc1_snrom_disables_prg_ram_with_bit_4_of_the_chr_register() {
    // 128 KiB PRG, CHR-RAM, 8 KiB PRG-RAM: the SNROM row of the board table.
    let mut core = MapperRom::new(1).prg(16).chr(0).battery().load();
    core.write_memory(0x6000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6000));

    core.mmc1_write(0xA000, 0x10);
    core.write_memory(0x6000, 0xA5);
    assert_eq!(0, core.read_memory(0x6000));

    core.mmc1_write(0xA000, 0x00);
    assert_eq!(0x5A, core.read_memory(0x6000));

    // The $E000 half of the same chip enable is unchanged (digit `1xxx` of the same test).
    core.mmc1_write(0xE000, 0x10);
    assert_eq!(0, core.read_memory(0x6000));
}

/// The other half of the test ROM's `4xxx` digit: "$A000 bit 4 **disables** WRAM (all but
/// SNROM)" is just as much a failure. On a CHR-ROM board bit 4 is CHR A16 and on a 512 KiB
/// board it is the outer PRG bank, so neither may touch the RAM (REQ-CORE-100).
#[test]
fn req_core_100_mmc1_only_an_snrom_reads_bit_4_of_the_chr_register_as_a_ram_enable() {
    // SJROM / SKROM: 16 KiB of CHR-ROM, so bit 4 is a bank line.
    let mut chr_rom = MapperRom::new(1).prg(16).chr(16).load();
    // SUROM: 512 KiB of PRG, so bit 4 is A18 — pinned for the banking side by
    // `req_core_100_mmc1_takes_the_512_kib_bank_bit_from_the_chr_register`.
    let mut surom = MapperRom::new(1).prg(64).chr(0).load();
    // SXROM: 32 KiB of PRG-RAM, so bits 2-3 bank it and bit 4 is A18.
    let mut big_ram = MapperRom::new(1)
        .prg(16)
        .chr(0)
        .prg_ram(32 * 1024, 0)
        .load();

    for core in [&mut chr_rom, &mut surom, &mut big_ram] {
        core.write_memory(0x6000, 0x3C);
        core.mmc1_write(0xA000, 0x10);
        core.write_memory(0x6000, 0x3C);
        assert_eq!(0x3C, core.read_memory(0x6000));
    }
}

// ---------------------------------------------------------------- UxROM, CNROM, AxROM

#[test]
fn req_core_100_uxrom_switches_16_kib_and_fixes_the_last_bank() {
    let mut core = MapperRom::new(2).prg(16).chr(8).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.write_memory(0x8000, 0x05);
    assert_eq!([10, 11, 14, 15], core.prg_windows());
}

#[test]
fn req_core_100_cnrom_switches_8_kib_of_chr() {
    let mut core = MapperRom::new(3).prg(4).chr(32).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    core.write_memory(0x9234, 0x02);
    assert_eq!([16, 17, 18, 19, 20, 21, 22, 23], core.chr_windows());
}

#[test]
fn req_core_100_axrom_switches_32_kib_and_selects_the_single_nametable() {
    let mut core = MapperRom::new(7).prg(16).chr(0).load();
    core.write_memory(0x8000, 0x02);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(Nm::SingleScreenA, core.mirroring());
    core.write_memory(0x8000, 0x12);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(Nm::SingleScreenB, core.mirroring());
    assert_eq!(
        core.map_nametable(0x2000) + 1024,
        core.map_nametable(0x2400) + 1024
    );
}

/// nesdev ("INES Mapper 180"): the board is a UNROM that "uses AND logic (74HC08) instead of OR
/// logic (74HC32), producing a fixed first bank at $8000 rather than a fixed last bank at
/// $C000". *Crazy Climber* is the only game (REQ-CORE-102).
#[test]
fn req_core_100_mapper_180_fixes_the_first_bank_and_switches_at_c000() {
    // The board has bus conflicts, so the register is written through a bank of $FF: 8 KiB bank
    // 0 is the low half of the fixed 16 KiB window at $8000.
    let mut core = MapperRom::new(180)
        .prg(16)
        .chr(0)
        .prg_bank_all_ones(0)
        .load();
    assert_eq!(180, core.cartridge().unwrap().mapper_number());

    // Power-on: the first bank at $8000 and bank 0 at $C000, not the last one — so both
    // windows show the same two 8 KiB banks, the low one being the $FF-filled bank 0.
    assert_eq!([0xFF, 1, 0xFF, 1], core.prg_windows());

    core.write_memory(0x8000, 0x03);
    assert_eq!([0xFF, 1, 6, 7], core.prg_windows());
    core.write_memory(0x8000, 0x07);
    assert_eq!([0xFF, 1, 14, 15], core.prg_windows());
}

/// And mapper 2 keeps the other wiring, with no submapper and therefore no bus conflicts
/// (MR-BUS-1): the register switches $8000 and the **last** bank is fixed at $C000.
#[test]
fn req_core_100_mapper_2_keeps_the_fixed_last_bank() {
    let mut core = MapperRom::new(2).prg(16).chr(0).load();
    assert_eq!(2, core.cartridge().unwrap().mapper_number());
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.write_memory(0x8000, 0x03);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
}

// ---------------------------------------------------------------- MMC3 (4)

#[test]
fn req_core_100_mmc3_maps_two_switchable_prg_banks_and_swaps_them() {
    let mut core = MapperRom::new(4).prg(16).load();
    core.mmc3_bank(6, 3);
    core.mmc3_bank(7, 5);
    assert_eq!([3, 5, 14, 15], core.prg_windows());
    core.mapper_write(0x8000, 0x46); // Bit 6: the switchable window moves to $C000.
    assert_eq!([14, 5, 3, 15], core.prg_windows());
}

#[test]
fn req_core_100_mmc3_maps_two_2_kib_and_four_1_kib_chr_banks_and_inverts_them() {
    let mut core = MapperRom::new(4).chr(32).load();
    core.mmc3_bank(0, 2); // 2 KiB window: the low bit of the register is ignored.
    core.mmc3_bank(1, 8);
    core.mmc3_bank(2, 20);
    core.mmc3_bank(3, 21);
    core.mmc3_bank(4, 22);
    core.mmc3_bank(5, 23);
    assert_eq!([2, 3, 8, 9, 20, 21, 22, 23], core.chr_windows());
    core.mapper_write(0x8000, 0x80); // Bit 7 swaps the halves.
    assert_eq!([20, 21, 22, 23, 2, 3, 8, 9], core.chr_windows());
}

#[test]
fn req_core_100_mmc3_controls_mirroring_and_prg_ram_protection() {
    let mut core = MapperRom::new(4).battery().load();
    core.mapper_write(0xA000, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());
    core.mapper_write(0xA000, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.write_memory(0x6100, 0x11);
    assert_eq!(0x11, core.read_memory(0x6100));
    core.mapper_write(0xA001, 0xC0); // Enabled but write protected.
    core.write_memory(0x6100, 0x22);
    assert_eq!(0x11, core.read_memory(0x6100));
    core.mapper_write(0xA001, 0x00); // Chip disabled: reads stop too.
    assert_eq!(0, core.read_memory(0x6100));
}

#[test]
fn req_core_100_mmc3_leaves_a_four_screen_board_alone() {
    let mut core = MapperRom::new(4).mirroring(0x08).load();
    core.mapper_write(0xA000, 0x01);
    assert_eq!(Nm::FourScreen, core.mirroring());
}

#[test]
fn req_core_100_mmc6_puts_one_kib_of_ram_at_7000() {
    // NES 2.0 submapper 1 of mapper 4: the 1 KiB window is enabled by bit 5 of the bank
    // select and each half has its own read and write enable in $A001.
    let mut core = MapperRom::new(4).submapper(1).load();
    core.write_memory(0x7000, 0x33);
    assert_eq!(0, core.read_memory(0x7000));
    core.mapper_write(0x8000, 0x20); // RAM enabled.
    core.mapper_write(0xA001, 0x30); // First half readable and writable.
    core.write_memory(0x7000, 0x33);
    assert_eq!(0x33, core.read_memory(0x7000));
    // The second half is still disabled, and the 1 KiB is mirrored to the end of the window.
    core.write_memory(0x7200, 0x44);
    assert_eq!(0, core.read_memory(0x7200));
    assert_eq!(0x33, core.read_memory(0x7400));
}

// ---------------------------------------------------------------- MMC5 (5)

#[test]
fn req_core_100_mmc5_has_four_prg_modes() {
    let cases: [(u8, [u8; 4]); 4] = [
        (0, [12, 13, 14, 15]),
        (1, [4, 5, 14, 15]),
        (2, [4, 5, 9, 15]),
        (3, [2, 5, 9, 15]),
    ];
    for (mode, expected) in cases {
        let mut core = MapperRom::new(5).prg(16).load();
        core.write_memory(0x5114, 0x82); // $8000: ROM bank 2.
        core.write_memory(0x5115, 0x85); // ROM bank 5, or the 16 KiB pair 4-5.
        core.write_memory(0x5116, 0x89); // $C000: ROM bank 9.
        core.write_memory(0x5117, 0x0F); // $E000 is always ROM: bank 15.
        core.write_memory(0x5100, mode);
        assert_eq!(expected, core.prg_windows(), "mode {mode}");
    }
}

#[test]
fn req_core_100_mmc5_can_show_prg_ram_in_a_prg_window() {
    let mut core = MapperRom::new(5).prg(16).load();
    core.write_memory(0x5100, 0x03); // 8 KiB windows.
    core.write_memory(0x5102, 0x02); // Unlock the RAM.
    core.write_memory(0x5103, 0x01);
    core.write_memory(0x5114, 0x00); // $8000 shows PRG-RAM bank 0 (bit 7 clear).
    core.write_memory(0x8100, 0x77);
    assert_eq!(0x77, core.read_memory(0x8100));
    core.write_memory(0x5114, 0x80); // Back to ROM bank 0.
    assert_eq!(0, core.read_memory(0x8100));
}

#[test]
fn req_core_100_mmc5_banks_the_prg_ram_window_at_6000() {
    let mut core = MapperRom::new(5).battery().load();
    core.write_memory(0x5102, 0x02);
    core.write_memory(0x5103, 0x01);
    core.write_memory(0x5113, 0x00);
    core.write_memory(0x6100, 0xA1);
    core.write_memory(0x5113, 0x03);
    core.write_memory(0x6100, 0xB2);
    assert_eq!(0xB2, core.read_memory(0x6100));
    core.write_memory(0x5113, 0x00);
    assert_eq!(0xA1, core.read_memory(0x6100));
}

#[test]
fn req_core_100_mmc5_write_protects_prg_ram_until_both_values_are_written() {
    let mut core = load(5);
    core.write_memory(0x6100, 0x5A);
    assert_eq!(0, core.read_memory(0x6100));
    core.write_memory(0x5102, 0x02);
    core.write_memory(0x6100, 0x5A);
    assert_eq!(0, core.read_memory(0x6100));
    core.write_memory(0x5103, 0x01);
    core.write_memory(0x6100, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6100));
}

#[test]
fn req_core_100_mmc5_has_four_chr_modes() {
    let cases: [(u8, [u8; 8]); 4] = [
        (3, [0, 1, 2, 3, 4, 5, 6, 7]),
        (2, [2, 3, 6, 7, 10, 11, 14, 15]),
        (1, [12, 13, 14, 15, 28, 29, 30, 31]),
        (0, [56, 57, 58, 59, 60, 61, 62, 63]),
    ];
    for (mode, expected) in cases {
        let mut core = MapperRom::new(5).chr(64).load();
        for register in 0..8u16 {
            core.write_memory(0x5120 + register, register as u8);
        }

        core.write_memory(0x5101, mode);
        assert_eq!(expected, core.chr_windows(), "mode {mode}");
    }
}

#[test]
fn req_core_100_mmc5_serves_background_fetches_from_the_second_set_with_tall_sprites() {
    let mut core = MapperRom::new(5).chr(64).load();
    core.write_memory(0x5101, 0x03); // 1 KiB windows.
    for register in 0..8u16 {
        core.write_memory(0x5120 + register, register as u8);
    }

    for register in 0..4u16 {
        core.write_memory(0x5128 + register, 16 + register as u8);
    }

    let peek = |core: &zax_nes::NesCore, address, fetch| {
        core.cartridge().unwrap().ppu_peek(address, fetch)
    };
    // 8x8 sprites: "$5128-$512B are completely ignored", so set A serves everything whatever
    // was written last (MR-5-1).
    core.write_memory(0x2000, 0x00);
    assert_eq!(0, peek(&core, 0x0000, ChrFetch::Background));
    assert_eq!(0, peek(&core, 0x0000, ChrFetch::Sprite));
    // 8x16 sprites: background from $5128-$512B, sprites from $5120-$5127.
    core.write_memory(0x2000, 0x20);
    assert_eq!(16, peek(&core, 0x0000, ChrFetch::Background));
    assert_eq!(0, peek(&core, 0x0000, ChrFetch::Sprite));
    assert_eq!(4, peek(&core, 0x1000, ChrFetch::Sprite));
    // And inside a frame the background takes set B whichever set was written last, which is
    // the clause MR-5-1 restored: write set A last, then arm the in-frame flag.
    core.write_memory(0x5120, 0x00);
    common::mapper_fixture::arm_mmc5_in_frame(&mut core);
    assert_eq!(16, peek(&core, 0x0000, ChrFetch::Background));
    assert_eq!(0, peek(&core, 0x0000, ChrFetch::Sprite));
}

#[test]
fn req_core_100_mmc5_maps_each_nametable_to_vram_exram_or_fill_mode() {
    let mut core = load(5);
    core.write_memory(0x5104, 0x02); // ExRAM in RAM mode, so the CPU can fill it.
    core.write_memory(0x5C00, 0x6B);
    core.write_memory(0x5106, 0x5C); // Fill tile.
    core.write_memory(0x5107, 0x02); // Fill attribute.
                                     // $2000 -> VRAM page 0, $2400 -> page 1, $2800 -> ExRAM, $2C00 -> fill mode.
    core.write_memory(0x5105, 0xE4);
    core.write_memory(0x5104, 0x00); // ExRAM as a nametable.
    assert_eq!(0, core.map_nametable(0x2000));
    let ppu = core.ppu();
    let cartridge = core.cartridge();
    assert_eq!(0x6B, ppu.read_nametable_byte(0x2800, cartridge));
    assert_eq!(0x5C, ppu.read_nametable_byte(0x2C00, cartridge));
    assert_eq!(0x02 * 0x55, ppu.read_nametable_byte(0x2FC0, cartridge));
}

#[test]
fn req_core_100_mmc5_multiplies_two_bytes() {
    let mut core = load(5);
    core.write_memory(0x5205, 0x12);
    core.write_memory(0x5206, 0x34);
    assert_eq!((0x12u32 * 0x34) & 0xFF, u32::from(core.read_memory(0x5205)));
    assert_eq!((0x12u32 * 0x34) >> 8, u32::from(core.read_memory(0x5206)));
}

// ---------------------------------------------------------------- MMC2 and MMC4 (9, 10)

#[test]
fn req_core_100_mmc2_switches_8_kib_and_fixes_the_last_three_banks() {
    let mut core = MapperRom::new(9).prg(8).chr(32).load();
    core.mapper_write(0xA000, 0x02);
    assert_eq!([2, 5, 6, 7], core.prg_windows());
}

#[test]
fn req_core_100_mmc4_switches_16_kib_and_fixes_the_last_bank() {
    let mut core = MapperRom::new(10).prg(8).chr(32).load();
    core.mapper_write(0xA000, 0x02);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
}

#[test]
fn req_core_039_the_fd_and_fe_latches_pick_the_chr_bank() {
    for mapper in [9usize, 10] {
        let mut core = MapperRom::new(mapper).chr(64).load();
        core.mapper_write(0xB000, 0x01); // $0000 with latch $FD: 4 KiB bank 1.
        core.mapper_write(0xC000, 0x02); // $0000 with latch $FE: bank 2.
        core.mapper_write(0xD000, 0x03); // $1000 with latch $FD: bank 3.
        core.mapper_write(0xE000, 0x04); // $1000 with latch $FE: bank 4.
        let cartridge = core.cartridge_mut().unwrap();
        let mut read = |address| cartridge.ppu_read(address, ChrFetch::Background);
        // Power-on selects the $FE registers. Reading the pattern of tile $FD moves the low
        // latch, but the byte served is still the one of the bank that was in force.
        assert_eq!(2 * 4, read(0x0000));
        assert_eq!((2 * 4) + 3, read(0x0FD8));
        assert_eq!(4, read(0x0000));
        assert_eq!((4) + 3, read(0x0FE8));
        assert_eq!(2 * 4, read(0x0000));
        // The high window has its own latch and its own pair of registers.
        assert_eq!(4 * 4, read(0x1000));
        assert_eq!((4 * 4) + 3, read(0x1FD8));
        assert_eq!(3 * 4, read(0x1000));
    }
}

#[test]
fn req_core_033_a_peek_never_moves_an_mmc2_latch() {
    let mut core = MapperRom::new(9).chr(64).load();
    core.mapper_write(0xB000, 0x01);
    core.mapper_write(0xC000, 0x02);
    let cartridge = core.cartridge().unwrap();
    let before = cartridge.chr_latch_state();
    for address in 0..0x2000u16 {
        let _ = cartridge.ppu_peek(address, ChrFetch::Background);
    }

    assert_eq!(before, cartridge.chr_latch_state());
    assert_eq!(2 * 4, cartridge.ppu_peek(0x0000, ChrFetch::Background));
}

#[test]
fn req_core_100_mmc4_only_needs_the_row_of_the_tile_to_move_its_latch() {
    // MMC2 reacts to the single address $0FD8, MMC4 to the whole row $0FD8-$0FDF.
    let mut mmc2 = MapperRom::new(9).chr(64).load();
    mmc2.mapper_write(0xB000, 0x01);
    mmc2.mapper_write(0xC000, 0x02);
    mmc2.cartridge_mut()
        .unwrap()
        .ppu_read(0x0FD9, ChrFetch::Background);
    assert_eq!(2 * 4, peek(&mmc2, 0x0000));
    let mut mmc4 = MapperRom::new(10).chr(64).load();
    mmc4.mapper_write(0xB000, 0x01);
    mmc4.mapper_write(0xC000, 0x02);
    mmc4.cartridge_mut()
        .unwrap()
        .ppu_read(0x0FD9, ChrFetch::Background);
    assert_eq!(4, peek(&mmc4, 0x0000));
}

// ---------------------------------------------------------------- Sunsoft FME-7 (69)

/// Invokes FME-7 command `command` with `parameter`: the command register is at $8000-$9FFF and
/// "Writing here invokes the command with the specified parameter byte" at $A000-$BFFF (nesdev,
/// "Sunsoft FME-7").
fn fme7(core: &mut NesCore, command: u8, parameter: u8) {
    core.write_memory(0x8000, command);
    core.write_memory(0xA000, parameter);
}

#[test]
fn req_core_100_fme7_switches_three_prg_banks_and_wires_the_last() {
    let mut core = MapperRom::new(69).prg(16).chr(32).load();
    // Commands $9, $A, $B are $8000-$9FFF, $A000-$BFFF and $C000-$DFFF; $E000-$FFFF is absent
    // from the page's table because the last bank is wired there.
    fme7(&mut core, 0x09, 3);
    fme7(&mut core, 0x0A, 5);
    fme7(&mut core, 0x0B, 9);
    assert_eq!([3, 5, 9, 15], core.prg_windows());

    // Six bank bits, `..bB BBBB`: bits 6 and 7 are not part of the number.
    fme7(&mut core, 0x09, 0xC2);
    assert_eq!([2, 5, 9, 15], core.prg_windows());
}

#[test]
fn req_core_100_fme7_switches_eight_one_kib_chr_banks() {
    let mut core = MapperRom::new(69).prg(16).chr(32).load();
    for slot in 0..8u8 {
        fme7(&mut core, slot, 24 + slot);
    }

    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], core.chr_windows());
}

#[test]
fn req_core_100_fme7_command_c_picks_one_of_four_arrangements() {
    let mut core = MapperRom::new(69).prg(16).chr(32).load();
    for (value, expected) in [
        (0x00, Nm::Vertical),
        (0x01, Nm::Horizontal),
        (0x02, Nm::SingleScreenA),
        (0x03, Nm::SingleScreenB),
        // `.... ..MM`: the upper bits are not decoded.
        (0xFC, Nm::Vertical),
    ] {
        fme7(&mut core, 0x0C, value);
        assert_eq!(expected, core.mirroring(), "parameter {value:#04X}");
    }
}

/// Command $8 is `ERbB BBBB`: `E` is the RAM chip's enable, `R` picks "0=ROM, 1=RAM" and the
/// rest is the 8 KiB bank (nesdev, "Sunsoft FME-7"). All three states are observable.
#[test]
fn req_core_100_fme7_puts_ram_or_rom_or_nothing_at_6000() {
    let mut core = MapperRom::new(69).prg(16).chr(32).battery().load();

    // `R` and `E` set: the work RAM answers.
    fme7(&mut core, 0x08, 0xC0);
    core.write_memory(0x6000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6000));

    // `R` set, `E` clear: the chip is deselected and the window is open bus.
    fme7(&mut core, 0x08, 0x40);
    assert_eq!(0, core.read_memory(0x6000));
    core.write_memory(0x6000, 0xA5); // Must not reach the RAM either.

    // `R` clear: a plain 8 KiB PRG-ROM bank, which the fixture numbers.
    fme7(&mut core, 0x08, 0x05);
    assert_eq!(0x05, core.read_memory(0x6000));
    assert_eq!(0x05, core.read_memory(0x7FFF));
    fme7(&mut core, 0x08, 0x0B);
    assert_eq!(0x0B, core.read_memory(0x6000));
    // A write to the ROM window changes nothing.
    core.write_memory(0x6000, 0x00);
    assert_eq!(0x0B, core.read_memory(0x6000));

    // And the RAM kept what it held while the window was elsewhere.
    fme7(&mut core, 0x08, 0xC0);
    assert_eq!(0x5A, core.read_memory(0x6000));
}

/// `req_core_090_the_mapper_state_survives_a_save_and_restore` covers every board of
/// `SUPPORTED_MAPPERS`, this board included, through the two addresses every other board uses.
/// The FME-7 needs its own: writing $8000 and $A000 the way that test does only ever invokes
/// command $1, so the $6000 window, the arrangement and the IRQ counter would never move
/// (REQ-CORE-090, REQ-CORE-091).
#[test]
fn req_core_090_the_fme7_state_survives_a_save_and_restore() {
    let mut core = MapperRom::new(69).prg(16).chr(32).load();
    for slot in 0..8u8 {
        fme7(&mut core, slot, 24 + slot);
    }

    fme7(&mut core, 0x09, 3);
    fme7(&mut core, 0x0A, 5);
    fme7(&mut core, 0x0B, 9);
    fme7(&mut core, 0x08, 0x07); // ROM bank 7 at $6000.
    fme7(&mut core, 0x0C, 0x02); // One-screen A.
    fme7(&mut core, 0x0E, 0x34);
    fme7(&mut core, 0x0F, 0x12); // Counter $1234.
    fme7(&mut core, 0x0D, 0x81); // Counting, IRQ enabled.

    let state = core.save_state(true).unwrap();
    for _ in 0..2 {
        core.run_frame(0);
    }

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!([3, 5, 9, 15], core.prg_windows());
    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], core.chr_windows());
    assert_eq!(0x07, core.read_memory(0x6000));
    assert_eq!(Nm::SingleScreenA, core.mirroring());
    // Re-saving gives the same bytes, so the counter and the two enables came back too.
    assert_eq!(state, core.save_state(true).unwrap());
}

// ---------------------------------------------------------------- TxSROM (118)

/// nesdev ("INES Mapper 118"): the board wires CHR A17 to CIRAM A10, so "the CHR bankswitching
/// directly affects the mirroring mapping the nametable RAM". With bit 7 of $8000 **clear** the
/// two 2 KiB registers R0 and R1 are at PPU $0000-$0FFF, and bit 7 of each takes two nametables.
#[test]
fn req_core_100_txsrom_takes_two_nametables_from_each_2_kib_register() {
    let mut core = MapperRom::new(118).prg(16).chr(64).load();
    assert_eq!(118, core.cartridge().unwrap().mapper_number());

    for (r0, r1, expected) in [
        (0x00u8, 0x00u8, [0, 0, 0, 0]),
        (0x80, 0x00, [1, 1, 0, 0]),
        (0x00, 0x80, [0, 0, 1, 1]),
        (0x80, 0x80, [1, 1, 1, 1]),
    ] {
        core.mmc3_bank(0, r0);
        core.mmc3_bank(1, r1);
        assert_eq!(
            expected,
            nametable_pages(&core),
            "R0 {r0:#04X} R1 {r1:#04X}"
        );
    }
}

/// Loads bank register `register` while keeping bit 7 of `$8000` set, which is the MMC3's CHR
/// mode: the register index and the mode share that byte, so [`MapperMachine::mmc3_bank`] —
/// which writes the index alone — would put the board back in CHR mode 0 on every call.
fn txsrom_bank_in_chr_mode_1(core: &mut NesCore, register: u8, value: u8) {
    core.mapper_write(0x8000, 0x80 | register);
    core.mapper_write(0x8001, value);
}

/// And with bit 7 of $8000 **set** the four 1 KiB registers R2-R5 are at $0000-$0FFF and take one
/// nametable each — "mapping the 1KB CHR banks to the first pattern table by setting bit 7 of
/// $8000".
#[test]
fn req_core_100_txsrom_takes_one_nametable_from_each_1_kib_register() {
    let mut core = MapperRom::new(118).prg(16).chr(64).load();
    for register in 2..6u8 {
        txsrom_bank_in_chr_mode_1(&mut core, register, 0x00);
    }

    assert_eq!([0, 0, 0, 0], nametable_pages(&core));

    for (register, table) in [(2u8, 0usize), (3, 1), (4, 2), (5, 3)] {
        txsrom_bank_in_chr_mode_1(&mut core, register, 0x80);
        let mut expected = [0; 4];
        expected[..=table].fill(1);

        assert_eq!(expected, nametable_pages(&core), "register {register}");
    }
}

/// The two sets are read according to bit 7 of $8000 and nothing else: flipping the CHR mode with
/// both sets already loaded swaps which four bits are in charge.
#[test]
fn req_core_100_txsrom_switches_between_the_two_register_sets() {
    let mut core = MapperRom::new(118).prg(16).chr(64).load();
    core.mmc3_bank(0, 0x80); // R0: nametables 0 and 1 in CHR mode 0.
    core.mmc3_bank(1, 0x00);
    core.mmc3_bank(2, 0x00);
    core.mmc3_bank(3, 0x00);
    core.mmc3_bank(4, 0x80); // R4: nametable 2 in CHR mode 1.
    core.mmc3_bank(5, 0x80); // R5: nametable 3.

    assert_eq!([1, 1, 0, 0], nametable_pages(&core));
    core.mapper_write(0x8000, 0x80);
    assert_eq!([0, 0, 1, 1], nametable_pages(&core));
    core.mapper_write(0x8000, 0x00);
    assert_eq!([1, 1, 0, 0], nametable_pages(&core));
}

/// "This bit is bypassed by the configuration described above, so writing here has no effect":
/// `$A000` does nothing on a `TxSROM`, where it still drives the arrangement on a plain MMC3.
#[test]
fn req_core_100_txsrom_ignores_the_mirroring_register() {
    let mut txsrom = MapperRom::new(118).prg(16).chr(64).load();
    txsrom.mmc3_bank(0, 0x80);
    txsrom.mmc3_bank(1, 0x00);
    let before = txsrom.mirroring();
    for value in [0x00u8, 0x01] {
        txsrom.mapper_write(0xA000, value);
        assert_eq!(before, txsrom.mirroring(), "$A000 = {value:#04X}");
        assert_eq!([1, 1, 0, 0], nametable_pages(&txsrom));
    }

    // The plain MMC3 is untouched: $A000 still picks the arrangement, and the pages stay the
    // console's.
    let mut mmc3 = MapperRom::new(4).prg(16).chr(64).load();
    mmc3.mapper_write(0xA000, 0x01);
    assert_eq!(Nm::Horizontal, mmc3.mirroring());
    mmc3.mapper_write(0xA000, 0x00);
    assert_eq!(Nm::Vertical, mmc3.mirroring());
    assert_eq!(
        [Cartridge::FOLLOW_MIRRORING; 4],
        nametable_pages(&mmc3),
        "a plain MMC3 leaves its nametables to the console"
    );
}

/// The pages are derived from the bank registers, so a savestate written before this lot still
/// loads and a `TxSROM` state brings its nametables back with its banks (REQ-CORE-090,
/// REQ-CORE-091).
#[test]
fn req_core_090_the_txsrom_nametables_come_back_with_its_banks() {
    let mut core = MapperRom::new(118).prg(16).chr(64).load();
    for (register, value) in [(2u8, 0x80u8), (3, 0x00), (4, 0x80), (5, 0x00)] {
        txsrom_bank_in_chr_mode_1(&mut core, register, value);
    }

    assert_eq!([1, 0, 1, 0], nametable_pages(&core));

    let state = core.save_state(true).unwrap();
    core.mapper_write(0x8000, 0x00);
    core.mmc3_bank(0, 0x00);
    core.mmc3_bank(1, 0x00);
    assert_eq!([0, 0, 0, 0], nametable_pages(&core));

    core.try_load_state(&state, 0).unwrap();
    assert_eq!([1, 0, 1, 0], nametable_pages(&core));
    assert_eq!(state, core.save_state(true).unwrap());
}

// ---------------------------------------------------------------- mapper 78 (IF-12, Uchuusen)

/// A mapper-78 image on the submapper `submapper` says: the board has bus conflicts
/// unconditionally, so PRG bank 0 — the low half of the switchable 16 KiB window at power-on — is
/// filled with $FF for the register to be writable through it.
fn mapper_78(submapper: Option<u8>, byte6: u8) -> NesCore {
    let mut rom = MapperRom::new(78)
        .prg(16)
        .chr(32)
        .mirroring(byte6)
        .prg_bank_all_ones(0);
    if let Some(submapper) = submapper {
        rom = rom.submapper(submapper);
    }

    rom.load()
}

/// The register is `CCCC MPPP` over the whole of $8000-$FFFF: bits 0-2 the 16 KiB PRG bank at
/// $8000-$BFFF, bits 4-7 the 8 KiB CHR bank, and $C000-$FFFF fixed to the last 16 KiB (nesdev,
/// "INES Mapper 078").
#[test]
fn req_core_100_mapper_78_switches_prg_chr_and_mirroring_from_one_register() {
    let mut core = mapper_78(Some(3), 0x00);
    assert_eq!(78, core.cartridge().unwrap().mapper_number());
    assert_eq!([0xFF, 1, 14, 15], core.prg_windows());

    core.write_memory(0x8000, 0x35); // PRG bank 5, CHR bank 3, bit 3 clear.
    assert_eq!([10, 11, 14, 15], core.prg_windows());
    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], core.chr_windows());

    // Bits 0-2 only: bit 3 is the mirroring and does not reach the PRG bank. On a fresh board,
    // because the write above moved the $FF-filled bank out of the window the register is
    // written through (the board has conflicts).
    let mut fresh = mapper_78(Some(3), 0x00);
    fresh.write_memory(0x8000, 0x0F);
    assert_eq!([14, 15, 14, 15], fresh.prg_windows());
    assert_eq!(Nm::Vertical, fresh.mirroring());
}

/// Bit 3 means one thing on each board. nesdev: *Holy Diver* — NES 2.0 submapper 3 — is
/// "0 = H, 1 = V" through "a 74HC00, which serves as a 2:1 mux of PPU A10 and PPU A11 into CIRAM
/// A10", and *Cosmo Carrier* — submapper 1 — is "0 = 1scA, 1 = 1scB".
#[test]
fn req_core_100_mapper_78_reads_bit_3_according_to_its_board() {
    let mut holy_diver = mapper_78(Some(3), 0x00);
    holy_diver.write_memory(0x8000, 0x00);
    assert_eq!(Nm::Horizontal, holy_diver.mirroring());
    holy_diver.write_memory(0x8000, 0x08);
    assert_eq!(Nm::Vertical, holy_diver.mirroring());

    let mut uchuusen = mapper_78(Some(1), 0x00);
    uchuusen.write_memory(0x8000, 0x00);
    assert_eq!(Nm::SingleScreenA, uchuusen.mirroring());
    uchuusen.write_memory(0x8000, 0x08);
    assert_eq!(Nm::SingleScreenB, uchuusen.mirroring());
}

/// And for an image with no submapper, the page's own heuristic: "iNES1 ROM image headers often
/// set the 'alternative nametables' flag for Holy Diver and cleared it for Cosmo Carrier" — byte
/// 6 bit 3. Mesen 2 has no equivalent and gives such an image Cosmo Carrier's wiring whatever
/// byte 6 says; AMD-061 §1 makes the documentation prevail.
#[test]
fn req_core_100_mapper_78_without_a_submapper_follows_byte_6_bit_3() {
    let mut flagged = mapper_78(None, 0x08);
    flagged.write_memory(0x8000, 0x08);
    assert_eq!(
        Nm::Vertical,
        flagged.mirroring(),
        "byte 6 bit 3 set: Holy Diver"
    );

    let mut plain = mapper_78(None, 0x00);
    plain.write_memory(0x8000, 0x08);
    assert_eq!(
        Nm::SingleScreenB,
        plain.mirroring(),
        "byte 6 bit 3 clear: Cosmo Carrier"
    );

    // The submapper wins over the heuristic where there is one.
    let mut both = mapper_78(Some(1), 0x08);
    both.write_memory(0x8000, 0x08);
    assert_eq!(Nm::SingleScreenB, both.mirroring());
}

/// A four-screen header is never left in place: `update_banks` sets the arrangement from the
/// register on the first reset, so the byte-6 flag the heuristic reads has no other effect.
#[test]
fn req_core_100_mapper_78_never_keeps_the_four_screen_header() {
    let core = mapper_78(None, 0x08);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

// ---------------------------------------------------------------- the small boards

#[test]
fn req_core_100_color_dreams_splits_its_register_between_prg_and_chr() {
    // Mapper 11 has unconditional bus conflicts (MR-BUS-1, MR-11's board page), so the write
    // goes through a bank of $FF: the CHR field lives in bits 4-7 and a numbered bank could
    // never pass it. What is tested is the register decode, not the conflict.
    let mut core = MapperRom::new(11)
        .prg(16)
        .chr(64)
        .prg_bank_all_ones(0)
        .load();
    core.write_memory(0x8000, 0x32);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], core.chr_windows());
}

#[test]
fn req_core_100_cprom_fixes_the_first_4_kib_of_chr_ram() {
    let mut core = MapperRom::new(13).prg(4).chr(0).load();
    core.write_memory(0x8000, 0x02);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    // CHR-RAM starts empty, so the banks are told apart by writing through the PPU.
    core.cartridge_mut().unwrap().ppu_write(0x1000, 0xAA);
    core.write_memory(0x8000, 0x01);
    core.cartridge_mut().unwrap().ppu_write(0x1000, 0xBB);
    assert_eq!(0xBB, peek(&core, 0x1000));
    core.write_memory(0x8000, 0x02);
    assert_eq!(0xAA, peek(&core, 0x1000));
    assert_eq!(0, peek(&core, 0x0000));
}

#[test]
fn req_core_100_mapper_15_has_four_prg_layouts() {
    // AMD-061 §2: layout 1 fixes the last 16 KiB bank of the 128 KiB block B selects, i.e.
    // `B | 7`. This image is exactly one 128 KiB block, so its last bank is both, and the
    // vectors are the same as under AMD-021 R2-002; `amd_061_mapper_15_acceptance_vectors`
    // is the one that tells the two readings apart.
    let cases: [(u16, [u8; 4]); 4] = [
        (0, [4, 5, 6, 7]),
        (1, [4, 5, 14, 15]),
        (2, [4, 4, 4, 4]),
        (3, [4, 5, 4, 5]),
    ];
    for (mode, expected) in cases {
        let mut core = MapperRom::new(15).prg(16).chr(8).load();
        core.write_memory(0x8000 + mode, 0x02);
        assert_eq!(expected, core.prg_windows(), "mode {mode}");
    }
}

#[test]
fn req_core_100_mapper_15_carries_mirroring_in_bit_6() {
    // AMD-021 R2-002: bit 6 is the layout and bit 7 the half-bank bit of layout 2.
    let mut core = MapperRom::new(15).prg(16).chr(8).load();
    core.write_memory(0x8000, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());
    core.write_memory(0x8000, 0x40);
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.write_memory(0x8000, 0x80);
    assert_eq!(Nm::Vertical, core.mirroring());
}

#[test]
fn amd_061_mapper_15_acceptance_vectors() {
    // AMD-061 §2's criterion, which replaces AMD-021 R2-002's. A numbered 1 MiB image, 64
    // banks of 16 KiB, so 16 KiB bank n is the pair of 8 KiB banks (2n, 2n + 1).
    let mut core = MapperRom::new(15).prg(128).chr(8).load();
    // Power-on shows the first 32 KiB.
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    // "$8001 <- $05 -> 5 / 7": layout 1, B = 5, and the fixed window is `5 | 7` = 7 — the last
    // 16 KiB of B's own 128 KiB block, not bank 63 at the end of the image.
    core.write_memory(0x8001, 0x05);
    assert_eq!([10, 11, 14, 15], core.prg_windows());
    // "$8001 <- $0A -> 10 / 15": B = 10 is in the second block, so `10 | 7` = 15 stays there.
    core.write_memory(0x8001, 0x0A);
    assert_eq!([20, 21, 30, 31], core.prg_windows());
    // "$8002 <- $85 -> 8 KiB bank 11 everywhere": layout 2, B = 5, h = 1.
    core.write_memory(0x8002, 0x85);
    assert_eq!([11, 11, 11, 11], core.prg_windows());
    // "$8000 <- $40 -> horizontal mirroring": bit 6.
    core.write_memory(0x8000, 0x40);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

#[test]
fn req_core_100_bandai_fcg_banks_prg_chr_and_mirroring() {
    let mut core = MapperRom::new(16).prg(16).chr(32).load();
    for register in 0..8u16 {
        core.write_memory(0x8000 + register, 16 + register as u8);
    }

    core.write_memory(0x8008, 0x03);
    core.write_memory(0x8009, 0x02);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    assert_eq!([16, 17, 18, 19, 20, 21, 22, 23], core.chr_windows());
    assert_eq!(Nm::SingleScreenA, core.mirroring());
    // The FCG-1 and FCG-2 wiring puts the same registers at $6000-$7FFF.
    core.write_memory(0x6009, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

/// Mapper 153's three departures from mapper 16, each on the page's own words (MF-6c step 1):
/// the outer 256 KiB PRG bank on $8000-$8003, the fixed window staying inside that block, and
/// "No CHR banking is available".
#[cfg(feature = "board-153")]
#[test]
fn req_core_100_mapper_153_banks_a_256_kib_outer_block_and_no_chr() {
    // 512 KiB of PRG: 64 banks of 8 KiB, i.e. two 256 KiB outer blocks of sixteen 16 KiB banks.
    let mut core = MapperRom::new(153).prg(64).chr(32).battery().load();
    // Power-on: inner bank 0, outer block 0, and the fixed window is the block's last 16 KiB.
    assert_eq!([0, 1, 30, 31], core.prg_windows());
    // $8008 is the inner 16 KiB bank, four bits wide.
    core.write_memory(0x8008, 0x05);
    assert_eq!([10, 11, 30, 31], core.prg_windows());
    // Bit 0 of $8000-$8003 is the outer block, "The same value must be written to all four".
    for register in 0..4u16 {
        core.write_memory(0x8000 + register, 0x01);
    }
    // Both windows moved into the second block: bank 5 of it, and its last bank.
    assert_eq!([42, 43, 62, 63], core.prg_windows());
    // The inner field is four bits: $0F wraps inside the block rather than leaving it.
    core.write_memory(0x8008, 0x1F);
    assert_eq!([62, 63, 62, 63], core.prg_windows());
    // "No CHR banking is available": $8000-$8007 moved no CHR window, which is the linear 8 KiB
    // the board is wired to.
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
    for register in 0..8u16 {
        core.write_memory(0x8000 + register, 0x01);
    }
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
    // Back to block 0 when all four carry a clear bit 0.
    for register in 0..4u16 {
        core.write_memory(0x8000 + register, 0x00);
    }
    assert_eq!([30, 31, 30, 31], core.prg_windows());
    // $8009's mirroring and $800A-$800C's IRQ are submapper 5's, unchanged.
    core.write_memory(0x8009, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

/// Mapper 153's WRAM: 8 KiB of battery-backed RAM at $6000-$7FFF, gated by $800D bit 5, with the
/// register ports mirrored across $8000-$FFFF **only** so that the window is RAM and not a port
/// (MF-6c step 1, REQ-CORE-095).
#[cfg(feature = "board-153")]
#[test]
fn req_core_100_mapper_153_has_battery_wram_at_6000_gated_by_800d_bit_5() {
    let mut core = MapperRom::new(153).prg(64).chr(32).battery().load();
    // Power-on is enabled: see `BandaiFcg`'s note on `prg_ram_on`.
    core.write_memory(0x6123, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6123));
    // $800D bit 5 clear disables the chip: the window stops reading and stops accepting writes,
    // and the byte already in it is kept.
    core.write_memory(0x800D, 0x00);
    assert_ne!(0x5A, core.read_memory(0x6123));
    core.write_memory(0x6123, 0x11);
    core.write_memory(0x800D, 0x20);
    assert_eq!(0x5A, core.read_memory(0x6123));
    // The ports are mirrored across $8000-$FFFF only: a write of $6009 is RAM, not the
    // mirroring register. (Mapper 16 answers that address, which the FCG test above pins.)
    core.write_memory(0x8009, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());
    core.write_memory(0x6009, 0x01);
    assert_eq!(Nm::Vertical, core.mirroring());
    assert_eq!(0x01, core.read_memory(0x6009));
    // And REQ-CORE-095's single slice is the whole 8 KiB.
    let battery = core.cartridge().unwrap().battery_ram();
    assert_eq!(8 * 1024, battery.len());
    assert_eq!(0x5A, battery[0x123]);
}

/// Mapper 16's board is unaffected by mapper 153's parameter: the shape MF-6a3 step 1 used for
/// `req_core_100_an_ordinary_mapper_4_image_is_unaffected_by_119_47_and_37`, and the test that
/// says the three branches of `BandaiFcg` are a parameter and not a change (MF-6c step 1).
#[test]
fn mf_6c_an_ordinary_mapper_16_image_is_unaffected_by_153() {
    let mut core = MapperRom::new(16).prg(64).chr(32).load();
    // $8000-$8007 are the eight CHR registers, not an outer PRG bank: bit 0 of $8000 moves CHR
    // window 0 and no PRG window. (Mesen 2 would make this image's $8000 an outer bank, because
    // it has 512 KiB of PRG; the page documents that on mapper 153 alone. `BandaiFcg`'s note.)
    assert_eq!([0, 1, 62, 63], core.prg_windows());
    core.write_memory(0x8000, 0x11);
    assert_eq!([0, 1, 62, 63], core.prg_windows());
    assert_eq!(0x11, core.chr_bank(0x0000));
    // $8008 is a full byte on mapper 16, not four bits, and the fixed window is the image's last
    // 16 KiB and not a block's.
    core.write_memory(0x8008, 0x1F);
    assert_eq!([62, 63, 62, 63], core.prg_windows());
    // $6000-$7FFF is the register window, not RAM: $6009 is still the mirroring register and the
    // board has no readable PRG-RAM there.
    core.write_memory(0x6009, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.write_memory(0x6123, 0x5A);
    assert_ne!(0x5A, core.read_memory(0x6123));
    // And $800D is the EEPROM port, so it enables nothing.
    core.write_memory(0x800D, 0x20);
    assert_ne!(0x5A, core.read_memory(0x6123));
}

/// The Taito X1-005's register file: the $7E7x image, the 2 KiB registers whose LSB the page
/// says is ignored, $7EF6's mirroring and its **absence** at $7EF7, and the wired last bank
/// (MF-6c step 2).
#[cfg(feature = "board-taito-x1")]
#[test]
fn req_core_100_taito_x1_005_banks_prg_and_chr_with_the_2_kib_lsb_ignored() {
    let mut core = MapperRom::new(80).prg(32).chr(32).load();
    // "8 KiB PRG-ROM bank, fixed to the last bank" at $E000, and three switchable windows.
    core.write_memory(0x7EFA, 0x03);
    core.write_memory(0x7EFC, 0x05);
    core.write_memory(0x7EFE, 0x06);
    assert_eq!([3, 5, 6, 31], core.prg_windows());
    // The odd mirrors of the PRG registers answer too.
    core.write_memory(0x7EFB, 0x04);
    assert_eq!([4, 5, 6, 31], core.prg_windows());
    // "Bit 0 of written bank number is ignored when switching 2K banks": $05 and $04 both select
    // 2 KiB bank 2, i.e. 1 KiB banks 4 and 5. (Mesen would read $05 as 1 KiB banks 5 and 6.)
    core.write_memory(0x7EF0, 0x05);
    core.write_memory(0x7EF1, 0x06);
    for register in 2..6u16 {
        core.write_memory(0x7EF2 + register - 2, 0x08 + register as u8 - 2);
    }
    assert_eq!([4, 5, 6, 7, 8, 9, 10, 11], core.chr_windows());
    core.write_memory(0x7EF0, 0x04);
    assert_eq!([4, 5], [core.chr_bank(0x0000), core.chr_bank(0x0400)]);
    // "All registers exist at $7EFx and mirrored at $7E7x because the CPU A7 line is ignored."
    core.write_memory(0x7E72, 0x0A);
    assert_eq!(10, core.chr_bank(0x1000));
    // $7EF6 bit 0: "0: vertical (A11); 1: horizontal (A10)" — the arrangement, so clear is this
    // build's `Horizontal` and set is its `Vertical`.
    core.write_memory(0x7EF6, 0x01);
    assert_eq!(Nm::Vertical, core.mirroring());
    core.write_memory(0x7EF6, 0x00);
    assert_eq!(Nm::Horizontal, core.mirroring());
    // "$7EF6 … is not mirrored at $7EF7." (Mesen answers both.)
    core.write_memory(0x7EF7, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

/// The X1-005's 128 bytes: "$7F00-$7F7F: 128 B internal RAM" mirrored at $7F80-$7FFF, answering
/// only while $7EF8 holds $A3, with the rest of $6000-$7FFF open bus and the whole of it on
/// REQ-CORE-095's battery slice (MF-6c step 2).
#[cfg(feature = "board-taito-x1")]
#[test]
fn req_core_100_taito_x1_005_internal_ram_answers_only_after_a3_at_7ef8() {
    let mut core = MapperRom::new(80).prg(32).chr(32).battery().load();
    // Power-on: the permission latch is zero, so the RAM is off the bus and refuses writes.
    core.write_memory(0x7F10, 0x5A);
    assert_ne!(0x5A, core.read_memory(0x7F10));
    // "Write $A3 to enable 128 B internal RAM; other values disable it."
    core.write_memory(0x7EF8, 0xA3);
    core.write_memory(0x7F10, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x7F10));
    // Mirrored once: A7 is ignored inside the RAM too.
    assert_eq!(0x5A, core.read_memory(0x7F90));
    core.write_memory(0x7F90, 0x3C);
    assert_eq!(0x3C, core.read_memory(0x7F10));
    // Nothing else in the window is decoded: $6000-$7EFF is open bus, the registers included.
    assert_ne!(0x3C, core.read_memory(0x6000));
    assert_ne!(0x3C, core.read_memory(0x7000));
    assert_ne!(0x3C, core.read_memory(0x7EFA));
    // $7EF9 is the permission register's own mirror, and any other value disables.
    core.write_memory(0x7EF9, 0x00);
    assert_ne!(0x3C, core.read_memory(0x7F10));
    core.write_memory(0x7EF9, 0xA3);
    assert_eq!(0x3C, core.read_memory(0x7F10));
    // REQ-CORE-095: the chip's RAM *is* the battery slice, and it is 128 bytes and not the
    // header's 8 KiB.
    let battery = core.cartridge().unwrap().battery_ram();
    assert_eq!(128, battery.len());
    assert_eq!(0x3C, battery[0x10]);
}

/// Mapper 207 "modifies Taito's X1-005 (80) boards in the exact same way that TLSROM (118)
/// differs from MMC3 (4)": CHR A17 — bit 7 of each 2 KiB CHR register — drives CIRAM A10 for the
/// pair of tables that register covers, and $7EF6 "was old mirroring control register, now
/// ignored" (MF-6c step 2).
#[cfg(feature = "board-taito-x1")]
#[test]
fn req_core_100_mapper_207_puts_chr_a17_on_ciram_a10() {
    let mut core = MapperRom::new(207).prg(32).chr(32).load();
    // Power-on: both registers are zero, so both pairs show physical page 0.
    assert_eq!([0, 0, 0, 0], nametable_pages(&core));
    core.write_memory(0x7EF0, 0x80);
    assert_eq!([1, 1, 0, 0], nametable_pages(&core));
    core.write_memory(0x7EF1, 0x80);
    assert_eq!([1, 1, 1, 1], nametable_pages(&core));
    core.write_memory(0x7EF0, 0x00);
    assert_eq!([0, 0, 1, 1], nametable_pages(&core));
    // $7EF6 is ignored on this wiring: the pages stay where the CHR registers put them.
    core.write_memory(0x7EF6, 0x01);
    assert_eq!([0, 0, 1, 1], nametable_pages(&core));
    // And mapper 80 keeps $7EF6 and answers `FOLLOW_MIRRORING`, which is the other half of the
    // same claim.
    let mut eighty = MapperRom::new(80).prg(32).chr(32).load();
    eighty.write_memory(0x7EF0, 0x80);
    assert_eq!([-1, -1, -1, -1], nametable_pages(&eighty));
}

/// iNES 082 and NES 2.0 552 are the same silicon with the PRG register read in opposite bit
/// order — "NES 2.0 Mapper 552 represents the actual way the mask ROM is connected and is thus
/// the correct bank order, while iNES Mapper 082 represents the bank order as it was understood
/// before January 2020, when the mapper was reverse-engineered." **Both orders in one test**, so
/// the pair cannot be got backwards (MF-6c step 2).
#[cfg(feature = "board-taito-x1")]
#[test]
fn req_core_100_mapper_82_and_552_are_the_same_board_with_the_bank_order_reversed() {
    // $14 is `0001 0100`: bit 4 and bit 2 set.
    //   082 reads bit 2 as PRG A13 and bit 5 as A16, so the bank is `$14 >> 2` = 5;
    //   552 reads bit 5 as PRG A13 and bit 2 as A16, so bit 4 is A14 (2) and bit 2 is A16 (8),
    //   which is bank 10.
    let mut eighty_two = MapperRom::new(82).prg(32).chr(32).load();
    eighty_two.write_memory(0x7EFA, 0x14);
    // The other two registers are still at their power-on zero, which is bank 0: neither page
    // gives a power-on bank, and a register that reads zero names bank zero.
    assert_eq!([5, 0, 0, 31], eighty_two.prg_windows());
    let mut five_five_two = MapperRom::new(552).prg(32).chr(32).load();
    five_five_two.write_memory(0x7EFA, 0x14);
    assert_eq!([10, 0, 0, 31], five_five_two.prg_windows());
    // The lowest line each number reads: $04 is A13 on 082 (bank 1) and A16 on 552 (bank 8).
    eighty_two.write_memory(0x7EFA, 0x04);
    five_five_two.write_memory(0x7EFA, 0x04);
    assert_eq!(1, eighty_two.prg_windows()[0]);
    assert_eq!(8, five_five_two.prg_windows()[0]);
    // And the three windows are the same three on both: $7EFB is $A000, $7EFC is $C000, and
    // $E000 is wired to the last bank.
    for core in [&mut eighty_two, &mut five_five_two] {
        core.write_memory(0x7EFB, 0x0C);
        core.write_memory(0x7EFC, 0x10);
    }
    assert_eq!([1, 3, 4, 31], eighty_two.prg_windows());
    assert_eq!([8, 12, 2, 31], five_five_two.prg_windows());
    // Both report their own number, so a dump of either is never silently taken for the other.
    assert_eq!(82, eighty_two.cartridge().unwrap().mapper_number());
    assert_eq!(552, five_five_two.cartridge().unwrap().mapper_number());
}

/// The X1-017's CHR A12 inversion and its three independently gated RAM windows (MF-6c step 2).
#[cfg(feature = "board-taito-x1")]
#[test]
fn req_core_100_taito_x1_017_inverts_chr_a12_and_gates_three_ram_windows() {
    let mut core = MapperRom::new(82).prg(32).chr(32).battery().load();
    core.write_memory(0x7EF0, 0x04); // 2 KiB bank 2: 1 KiB banks 4 and 5.
    core.write_memory(0x7EF1, 0x06); // 2 KiB bank 3: 1 KiB banks 6 and 7.
    for register in 0..4u16 {
        core.write_memory(0x7EF2 + register, 0x08 + register as u8);
    }
    assert_eq!([4, 5, 6, 7, 8, 9, 10, 11], core.chr_windows());
    // $7EF6 bit 1 is "CHR A12 inversion (determines whether 2KB banks occupy $0000-$0FFF or
    // $1000-$1FFF)": the two groups swap halves, and bit 0 still sets the arrangement.
    core.write_memory(0x7EF6, 0x02);
    assert_eq!([8, 9, 10, 11, 4, 5, 6, 7], core.chr_windows());
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.write_memory(0x7EF6, 0x03);
    assert_eq!(Nm::Vertical, core.mirroring());
    assert_eq!([8, 9, 10, 11, 4, 5, 6, 7], core.chr_windows());
    core.write_memory(0x7EF6, 0x00);
    assert_eq!([4, 5, 6, 7, 8, 9, 10, 11], core.chr_windows());

    // The three windows, each behind its own magic value, and each one only its own window.
    for address in [0x6100u16, 0x6900, 0x7100] {
        core.write_memory(address, 0x5A);
        assert_ne!(0x5A, core.read_memory(address), "{address:04X} before");
    }
    core.write_memory(0x7EF7, 0xCA); // $6000-$67FF.
    core.write_memory(0x6100, 0x11);
    assert_eq!(0x11, core.read_memory(0x6100));
    assert_ne!(0x11, core.read_memory(0x6900));
    core.write_memory(0x7EF8, 0x69); // $6800-$6FFF.
    core.write_memory(0x6900, 0x22);
    assert_eq!(0x22, core.read_memory(0x6900));
    core.write_memory(0x7EF9, 0x84); // $7000-$73FF.
    core.write_memory(0x7100, 0x33);
    assert_eq!(0x33, core.read_memory(0x7100));
    // The wrong magic value is as good as none, and each latch gates only its own window.
    core.write_memory(0x7EF7, 0x69);
    assert_ne!(0x11, core.read_memory(0x6100));
    assert_eq!(0x22, core.read_memory(0x6900));
    // $7400-$7FFF is not decoded at all on this chip — no window covers it.
    core.write_memory(0x7500, 0x44);
    assert_ne!(0x44, core.read_memory(0x7500));
    // REQ-CORE-095: 5 KiB, in the window order $6000 / $6800 / $7000.
    let battery = core.cartridge().unwrap().battery_ram();
    assert_eq!(5 * 1024, battery.len());
    assert_eq!(0x11, battery[0x100]);
    assert_eq!(0x22, battery[0x900]);
    assert_eq!(0x33, battery[0x1100]);
}

/// Drives the two-wire bus of a Bandai LZ93D50's EEPROM from the CPU side, through $800D: bit 5
/// is the internal chip's SCL and bit 6 the SDA both chips see (nesdev's own diagram for
/// mapper 157, `RDC. ....`). MF-6c step 3.
#[cfg(feature = "board-eeprom")]
mod i2c {
    use super::NesCore;

    /// Puts `scl` and `sda` on the two lines. `direction` is $800D bit 7, which Q-134 records as
    /// decoded and inert — the tests pass it so that the claim is exercised and not assumed.
    pub fn lines(core: &mut NesCore, scl: u8, sda: u8, direction: u8) {
        core.write_memory(0x800D, (direction << 7) | (sda << 6) | (scl << 5));
    }

    /// One clock of `bit`: SCL low with the data set up, then SCL high to clock it in.
    pub fn clock(core: &mut NesCore, bit: u8, direction: u8) {
        lines(core, 0, bit, direction);
        lines(core, 1, bit, direction);
    }

    /// "START is identified by a high to low transition of the SDA line while the clock SCL is
    /// *stable* in the high state."
    pub fn start(core: &mut NesCore, direction: u8) {
        lines(core, 0, 1, direction);
        lines(core, 1, 1, direction);
        lines(core, 1, 0, direction);
    }

    /// "STOP is identified by a low to high transition of the SDA line while the clock SCL is
    /// *stable* in the high state."
    pub fn stop(core: &mut NesCore, direction: u8) {
        lines(core, 0, 0, direction);
        lines(core, 1, 0, direction);
        lines(core, 1, 1, direction);
    }

    /// The `width` bits of `value`, **most significant first** — the big-endian clocking the
    /// mapper-159 page says the hardware does.
    pub fn send(core: &mut NesCore, value: u8, width: u8, direction: u8) {
        for index in 0..width {
            clock(core, (value >> (width - 1 - index)) & 1, direction);
        }
    }

    /// The EEPROMs' data out: bit 4 of a read of $6000-$7FFF (the page's `xxxE Bxxx`).
    pub fn data_out(core: &NesCore) -> u8 {
        (core.read_memory(0x6000) >> 4) & 1
    }

    /// Clocks eight bits out of the chip, most significant first, and acknowledges so that a
    /// sequential read goes on.
    pub fn receive(core: &mut NesCore, direction: u8) -> u8 {
        let mut value = 0u8;
        for _ in 0..8 {
            lines(core, 0, 1, direction);
            lines(core, 1, 1, direction);
            value = (value << 1) | data_out(core);
        }
        // The master's acknowledge: SDA low for one clock.
        clock(core, 0, direction);
        value
    }

    /// A whole X24C01 transfer: seven address bits then the R/W bit in the same byte, the chip's
    /// acknowledge, then the data.
    pub fn x24c01_write(core: &mut NesCore, address: u8, bytes: &[u8]) {
        start(core, 0);
        send(core, address & 0x7F, 7, 0);
        clock(core, 0, 0); // R/W = write.
        clock(core, 1, 0); // the chip's acknowledge.
        for byte in bytes {
            send(core, *byte, 8, 0);
            clock(core, 1, 0);
        }
        stop(core, 0);
    }

    pub fn x24c01_read(core: &mut NesCore, address: u8, count: usize) -> Vec<u8> {
        start(core, 0);
        send(core, address & 0x7F, 7, 0);
        clock(core, 1, 0); // R/W = read.
        clock(core, 1, 0); // the chip's acknowledge.
        let out = (0..count).map(|_| receive(core, 1)).collect();
        stop(core, 0);
        out
    }

    /// An X24C02 transfer: the chip-address byte ($A0, R/W clear), the word address, then data.
    pub fn x24c02_write(core: &mut NesCore, address: u8, bytes: &[u8]) {
        start(core, 0);
        send(core, 0xA0, 8, 0);
        clock(core, 1, 0);
        send(core, address, 8, 0);
        clock(core, 1, 0);
        for byte in bytes {
            send(core, *byte, 8, 0);
            clock(core, 1, 0);
        }
        stop(core, 0);
    }

    /// A "Current Address Read": the chip address with R/W set, after the word address has been
    /// positioned by a write that was stopped before any data.
    pub fn x24c02_read(core: &mut NesCore, address: u8, count: usize) -> Vec<u8> {
        start(core, 0);
        send(core, 0xA0, 8, 0);
        clock(core, 1, 0);
        send(core, address, 8, 0);
        clock(core, 1, 0);
        start(core, 0);
        send(core, 0xA1, 8, 0);
        clock(core, 1, 0);
        let out = (0..count).map(|_| receive(core, 1)).collect();
        stop(core, 0);
        out
    }
}

/// Mapper 159's X24C01, and **the one behavioural claim of MF-6c step 3**, quoted from nesdev
/// ("INES Mapper 159"):
///
/// > Bandai's developers consistently thought the X24C01 was a little-endian device, and their
/// > code clocks in address and data bytes assuming this. However, it's actually a big-endian
/// > device, and a multi-byte read or write from EEPROM will increment the address accordingly.
///
/// So the chip is clocked most significant bit first, and a sequential transfer walks the
/// hardware's address. Mesen 2 clocks this part LSB first (`Eeprom24C01.h:15`); AMD-061 §1 makes
/// the page prevail.
#[cfg(feature = "board-eeprom")]
#[test]
fn req_core_100_the_x24c01_is_big_endian_and_its_address_increments_accordingly() {
    let mut core = MapperRom::new(159).prg(32).chr(32).battery().load();
    assert_eq!(159, core.cartridge().unwrap().mapper_number());

    // A single byte round-trips.
    i2c::x24c01_write(&mut core, 0x05, &[0x5A]);
    assert_eq!(vec![0x5A], i2c::x24c01_read(&mut core, 0x05, 1));

    // **Big-endian**: the byte landed where the most-significant-first address said, not where a
    // little-endian reading of the same seven bits would have put it. $05 is `000 0101`; read
    // LSB-first those bits are `101 0000` = $50, which must hold nothing.
    assert_eq!(vec![0x00], i2c::x24c01_read(&mut core, 0x50, 1));

    // **A multi-byte write increments the address**: three bytes from $10 land at $10, $11, $12.
    i2c::x24c01_write(&mut core, 0x10, &[0x11, 0x22, 0x33]);
    assert_eq!(vec![0x11, 0x22, 0x33], i2c::x24c01_read(&mut core, 0x10, 3));
    // And a multi-byte read walks the same way, so the two agree: reading from $11 gives the
    // second and third bytes.
    assert_eq!(vec![0x22, 0x33], i2c::x24c01_read(&mut core, 0x11, 2));

    // The chip is 128 bytes and the address wraps inside it: $7F then $00.
    i2c::x24c01_write(&mut core, 0x7F, &[0xAB, 0xCD]);
    assert_eq!(vec![0xAB], i2c::x24c01_read(&mut core, 0x7F, 1));
    assert_eq!(vec![0xCD], i2c::x24c01_read(&mut core, 0x00, 1));

    // REQ-CORE-095: the 128 bytes are the battery slice, and they are the EEPROM's.
    let battery = core.cartridge().unwrap().battery_ram();
    assert_eq!(128, battery.len());
    assert_eq!(0x5A, battery[0x05]);
    assert_eq!([0x11, 0x22, 0x33], battery[0x10..0x13]);
    assert_eq!(0xCD, battery[0x00]);

    // The EEPROM is not CPU-addressable memory: $6000-$7FFF is the data-out register, so a write
    // there cannot be read back as RAM.
    core.write_memory(0x6123, 0x77);
    assert_ne!(0x77, core.read_memory(0x6123));
}

/// Mapper 157, the Datach Joint ROM System: the internal X24C02, the external X24C01 whose clock
/// is on $8000-$8003, their outputs wire-`ANDed`, and "No CHR banking is available"
/// (MF-6c step 3).
#[cfg(feature = "board-eeprom")]
#[test]
fn req_core_100_the_datach_has_an_internal_24c02_and_an_external_24c01() {
    // Byte 10 = $10 is the 128 bytes the page uses to declare the second chip.
    let mut core = MapperRom::new(157)
        .prg(32)
        .chr(32)
        .battery()
        .prg_ram(0, 128)
        .load();
    assert_eq!(157, core.cartridge().unwrap().mapper_number());
    // 256 bytes of internal chip, then the 128 of the external one.
    assert_eq!(256 + 128, core.cartridge().unwrap().battery_ram().len());

    // The internal X24C02 answers the chip address $A0 and an 8-bit word address, big-endian, and
    // writes sequentially.
    i2c::x24c02_write(&mut core, 0x80, &[0xDE, 0xAD]);
    assert_eq!(
        vec![0xDE, 0xAD],
        i2c::x24c02_read(&mut core, 0x80, 2),
        "the internal 24C02"
    );
    assert_eq!(0xDE, core.cartridge().unwrap().battery_ram()[0x80]);

    // A chip address that is not $A0 selects nobody: "This chip wasn't selected, go back to idle
    // mode", so the byte at $80 is untouched.
    i2c::start(&mut core, 0);
    i2c::send(&mut core, 0x50, 8, 0);
    i2c::clock(&mut core, 1, 0);
    i2c::send(&mut core, 0x80, 8, 0);
    i2c::clock(&mut core, 1, 0);
    i2c::send(&mut core, 0x00, 8, 0);
    i2c::clock(&mut core, 1, 0);
    i2c::stop(&mut core, 0);
    assert_eq!(0xDE, core.cartridge().unwrap().battery_ram()[0x80]);

    // "No CHR banking is available": the eight registers move no CHR window, and the four lowest
    // are the external chip's clock instead.
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
    for register in 0..8u16 {
        core.write_memory(0x8000 + register, 0x0F);
    }
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());

    // The PRG banking, the mirroring and the IRQ are submapper 5's, unchanged.
    core.write_memory(0x8008, 0x03);
    assert_eq!([6, 7, 30, 31], core.prg_windows());
    core.write_memory(0x8009, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

/// The external chip's clock is bit 3 of $8000-$8003 — "External EEPROM Clock ($8000-$8003
/// write)", the diagram `xxxx Cxxx` — and the two chips' outputs are wire-`ANDed`: "When both
/// internal and external EEPROMs exist, their outputs are `ANDed` together via open-drain circuit"
/// (MF-6c step 3).
#[cfg(feature = "board-eeprom")]
#[test]
fn req_core_100_the_datach_external_eeprom_is_clocked_from_8000_and_wire_anded() {
    let mut core = MapperRom::new(157)
        .prg(32)
        .chr(32)
        .battery()
        .prg_ram(0, 128)
        .load();

    // Driving the external chip: SDA comes from $800D bit 6 as always, SCL from bit 3 of a write
    // to one of $8000-$8003.
    //
    // **The two lines are in two registers, so they cannot move together** — which is why the
    // master has to lower SCL *before* it changes SDA. Changing SDA while SCL is high is a START
    // or a STOP by definition, and the page's own warning about this register ("PPU rendering
    // must be disabled during external EEPROM access") is about the same hazard from the other
    // side. The internal chip sees SCL stuck at 0 throughout and so never leaves idle.
    let scl = |core: &mut NesCore, value: u8| core.write_memory(0x8000, value << 3);
    let sda = |core: &mut NesCore, value: u8| core.write_memory(0x800D, value << 6);
    let bit = |core: &mut NesCore, value: u8| {
        scl(core, 0);
        sda(core, value);
        scl(core, 1);
    };

    // START: SDA falls while SCL is stable high.
    scl(&mut core, 0);
    sda(&mut core, 1);
    scl(&mut core, 1);
    sda(&mut core, 0);
    // Address $01, seven bits most significant first, then the write bit and the chip's ack.
    for value in [0u8, 0, 0, 0, 0, 0, 1, 0, 1] {
        bit(&mut core, value);
    }
    // $3C, eight bits, then the chip's ack.
    for value in [0u8, 0, 1, 1, 1, 1, 0, 0, 1] {
        bit(&mut core, value);
    }

    // The byte landed in the **external** chip's 128 bytes, which are behind the internal 256.
    assert_eq!(0x3C, core.cartridge().unwrap().battery_ram()[256 + 0x01]);
    // And not in the internal chip, whose own clock never moved.
    assert_eq!(0x00, core.cartridge().unwrap().battery_ram()[0x01]);

    // **The wire-AND.** The last clock left the external chip mid-acknowledge, holding SDA low,
    // where the internal chip has never left idle and releases it — and the register reads **0**.
    // "When both internal and external EEPROMs exist, their outputs are ANDed together via
    // open-drain circuit": the only chip that can be pulling it down is the external one.
    assert_eq!(0, i2c::data_out(&core), "the two outputs are ANDed");
    // One clock fall later the acknowledge is over and both chips release the line again.
    scl(&mut core, 0);
    assert_eq!(1, i2c::data_out(&core));
}

/// Q-134: $800D bit 7 is "Both EEPROM's Direction bit (1=Enable Read)" on the page and has **no
/// effect** here — the state machine knows which way a byte is going because the protocol told
/// it, and Mesen 2 ignores the bit for the same reason. The provisional choice is pinned so that
/// it is a choice and not a bug, the way `q_123_vrc6_keeps_its_nametables_on_ciram_when_n_asks_for_chr_rom`
/// pins the VRC6's ignored bit.
#[cfg(feature = "board-eeprom")]
#[test]
fn q_134_the_eeprom_direction_bit_is_decoded_and_does_not_gate_sda() {
    // The same transfer twice, once with the direction bit clear throughout and once with it set
    // throughout, into two identical cartridges.
    let image = MapperRom::new(159).prg(32).chr(32).battery();
    let mut clear = image.load();
    let mut set = image.load();

    for (core, direction) in [(&mut clear, 0u8), (&mut set, 1u8)] {
        i2c::start(core, direction);
        i2c::send(core, 0x05, 7, direction);
        i2c::clock(core, 0, direction);
        i2c::clock(core, 1, direction);
        i2c::send(core, 0x5A, 8, direction);
        i2c::clock(core, 1, direction);
        i2c::stop(core, direction);
    }

    assert_eq!(0x5A, clear.cartridge().unwrap().battery_ram()[0x05]);
    assert_eq!(
        0x5A,
        set.cartridge().unwrap().battery_ram()[0x05],
        "Q-134: the direction bit does not gate SDA"
    );
}

/// The Sunsoft-4's CIRAM half: four 2 KiB CHR banks, the 16 KiB window with the last bank wired,
/// the four mirroring modes of $E000 bits 1-0 and the PRG-RAM enable of $F000 bit 4
/// (MF-6c step 4).
#[cfg(feature = "board-68")]
#[test]
fn req_core_100_sunsoft_4_banks_four_2_kib_chr_windows_and_one_prg_window() {
    let mut core = MapperRom::new(68).prg(32).chr(32).battery().load();
    assert_eq!(68, core.cartridge().unwrap().mapper_number());
    // Power-on: bank 0 at $8000, the image's last 16 KiB wired at $C000.
    assert_eq!([0, 1, 30, 31], core.prg_windows());
    // "...E BBBB": four bits of bank.
    core.write_memory(0xF000, 0x05);
    assert_eq!([10, 11, 30, 31], core.prg_windows());
    core.write_memory(0xF000, 0x0F);
    assert_eq!([30, 31, 30, 31], core.prg_windows());
    // The four 2 KiB CHR registers, on the high nibble of their own quarter of $8000-$BFFF.
    core.write_memory(0x8000, 0x01);
    core.write_memory(0x9123, 0x03);
    core.write_memory(0xA456, 0x05);
    core.write_memory(0xB789, 0x07);
    assert_eq!([2, 3, 6, 7, 10, 11, 14, 15], core.chr_windows());
    // $E000 bits 1-0: the four modes.
    for (bits, expected) in [
        (0x00u8, Nm::Vertical),
        (0x01, Nm::Horizontal),
        (0x02, Nm::SingleScreenA),
        (0x03, Nm::SingleScreenB),
    ] {
        core.write_memory(0xE000, bits);
        assert_eq!(expected, core.mirroring(), "$E000 = {bits:#04X}");
    }
    // With bit 4 clear the nametables stay on CIRAM, which is `FOLLOW_MIRRORING`.
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
    // $F000 bit 4 is the PRG-RAM enable, and it is **off** at power-on.
    let fresh = MapperRom::new(68).prg(32).chr(32).battery().load();
    assert_ne!(0x5A, fresh.read_memory(0x6100));
    core.write_memory(0xF000, 0x10);
    core.write_memory(0x6100, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6100));
    core.write_memory(0xF000, 0x00);
    assert_ne!(0x5A, core.read_memory(0x6100));
    core.write_memory(0xF000, 0x10);
    assert_eq!(0x5A, core.read_memory(0x6100));
    assert_eq!(8 * 1024, core.cartridge().unwrap().battery_ram().len());
}

/// **Q-123's third `Mapper::read_nametable` body**, which the owner's rule of 2026-11-06 has this
/// lot build rather than decode and ignore: $E000 bit 4 maps the 1 KiB CHR-ROM banks of $C000 and
/// $D000 over $2000-$2FFF, which register serving which logical table being decided by the
/// mirroring mode. Behind `board-68-chr-nametables`, whose own measurement decides whether it is
/// in `default` (MF-6c step 4).
#[cfg(feature = "board-68-chr-nametables")]
#[test]
fn req_core_100_sunsoft_4_serves_its_nametables_from_chr_rom_on_e000_bit_4() {
    // 256 KiB of CHR so that the 1 KiB banks $80 and up exist: "7 bits used; D7 ignored", and the
    // board wires CHR A17 high for a nametable fetch, so bank $80 is the first one it can reach.
    let mut core = MapperRom::new(68).prg(32).chr(256).load();
    let cartridge =
        |core: &NesCore, address: u16| core.cartridge().unwrap().read_nametable(address);

    // Bit 4 clear: CIRAM, and `nametable_page` says so.
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
    // $C000 and $D000 take a 1 KiB bank with bit 7 forced set: $00 is bank $80 and $05 is $85.
    core.write_memory(0xC000, 0x00);
    core.write_memory(0xD000, 0x05);
    // Bit 4 set, with vertical mirroring: the two registers alternate across the four tables.
    core.write_memory(0xE000, 0x10);
    assert_eq!(
        [-2, -2, -2, -2],
        nametable_pages(&core),
        "SERVED_BY_CARTRIDGE"
    );
    assert_eq!(0x80, cartridge(&core, 0x2000));
    assert_eq!(0x85, cartridge(&core, 0x2400));
    assert_eq!(0x80, cartridge(&core, 0x2800));
    assert_eq!(0x85, cartridge(&core, 0x2C00));
    // Horizontal: they pair up.
    core.write_memory(0xE000, 0x11);
    assert_eq!(0x80, cartridge(&core, 0x2000));
    assert_eq!(0x80, cartridge(&core, 0x2400));
    assert_eq!(0x85, cartridge(&core, 0x2800));
    assert_eq!(0x85, cartridge(&core, 0x2C00));
    // The two single-screen modes put one register everywhere.
    core.write_memory(0xE000, 0x12);
    assert_eq!(
        [0x80, 0x80, 0x80, 0x80],
        [0x2000u16, 0x2400, 0x2800, 0x2C00].map(|a| cartridge(&core, a))
    );
    core.write_memory(0xE000, 0x13);
    assert_eq!(
        [0x85, 0x85, 0x85, 0x85],
        [0x2000u16, 0x2400, 0x2800, 0x2C00].map(|a| cartridge(&core, a))
    );
    // The offset inside the 1 KiB bank is the low ten bits of the address.
    assert_eq!(0x85, cartridge(&core, 0x23FF));
    // And bit 4 clear puts the nametables back on CIRAM without the registers moving.
    core.write_memory(0xE000, 0x03);
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
}

/// The bit is **decoded and kept in the savestate whatever `board-68-chr-nametables` says**, so
/// that the feature switches the serving and not the register decode — which is what lets a state
/// written by one configuration load into the other (REQ-CORE-090, REQ-CORE-091). MF-6c step 4.
#[cfg(feature = "board-68")]
#[test]
fn mf_6c_sunsoft_4_decodes_e000_bit_4_in_both_configurations() {
    let mut core = MapperRom::new(68).prg(32).chr(256).load();
    core.write_memory(0xC000, 0x02);
    core.write_memory(0xE000, 0x10);
    let state = core.save_state(true).unwrap();
    let mut other = MapperRom::new(68).prg(32).chr(256).load();
    other.try_load_state(&state, 0).unwrap();
    // The pages come back derived, and they are whatever this build serves.
    assert_eq!(nametable_pages(&core), nametable_pages(&other));
    let served = cfg!(feature = "board-68-chr-nametables");
    assert_eq!(
        if served { -2 } else { -1 },
        nametable_pages(&other)[0],
        "the feature decides the serving, not the decode"
    );
}

// ------------------------------------------- Irem LROG017 (77)

/// nesdev ("INES Mapper 077"): `$8000-$FFFF: [CCCC PPPP]`, `C` a "2k CHR reg @ $0000" and `P` a
/// "32k PRG reg @ $8000", with "Bus conflicts". The four-screen nametable is the page's own memory
/// map, which gives $2000-$27FF and $2800-$2FFF as two distinct 2 KiB regions of RAM
/// (MF-6c step 6).
#[cfg(feature = "board-77")]
#[test]
fn req_core_100_mapper_77_banks_32_kib_of_prg_and_2_kib_of_chr_rom() {
    // 8 KiB bank 0 is filled with $FF, so a write through the power-on window survives the bus
    // conflict whole; every other bank reads its own number and masks the value.
    let mut core = MapperRom::new(77)
        .prg(32)
        .chr(32)
        .prg_bank_all_ones(0)
        .load();
    assert_eq!(77, core.cartridge().unwrap().mapper_number());
    // Power-on: 32 KiB PRG bank 0 and 2 KiB CHR bank 0. $0800-$1FFF is the RAM chip, which has
    // powered on at zero and is not the bank register's business at all.
    assert_eq!([255, 1, 2, 3], core.prg_windows());
    assert_eq!([0, 1, 0, 0, 0, 0, 0, 0], core.chr_windows());
    // The board's own wiring, whatever byte 6 of the dump says.
    assert_eq!(Nm::FourScreen, core.mirroring());

    // One write of `[CCCC PPPP]`: C = 3 is 1 KiB banks 6 and 7, P = 5 is 8 KiB banks 20 to 23.
    core.mapper_write(0x8000, 0x35);
    assert_eq!([20, 21, 22, 23], core.prg_windows());
    assert_eq!([6, 7, 0, 0, 0, 0, 0, 0], core.chr_windows());

    // **Bus conflicts**: the same write again, with 8 KiB bank 20 now under $8000, latches
    // $35 & 20 = $14 — C = 1 (1 KiB banks 2 and 3) and P = 4 (8 KiB banks 16 to 19).
    core.mapper_write(0x8000, 0x35);
    assert_eq!(
        [16, 17, 18, 19],
        core.prg_windows(),
        "the ROM went on driving the bus"
    );
    assert_eq!([2, 3, 0, 0, 0, 0, 0, 0], core.chr_windows());

    // Four bits of PRG, wrapping into the image's eight 32 KiB banks: $0F is bank 15, i.e. 7.
    let mut prg = MapperRom::new(77)
        .prg(32)
        .chr(32)
        .prg_bank_all_ones(0)
        .load();
    prg.mapper_write(0x8000, 0x0F);
    assert_eq!([28, 29, 30, 31], prg.prg_windows());

    // Four bits of CHR: bank 15 is the last 2 KiB of a 32 KiB CHR-ROM, and the PRG field is
    // independent of it.
    let mut chr = MapperRom::new(77)
        .prg(32)
        .chr(32)
        .prg_bank_all_ones(0)
        .load();
    chr.mapper_write(0x8000, 0xF0);
    assert_eq!([30, 31, 0, 0, 0, 0, 0, 0], chr.chr_windows());
    assert_eq!([255, 1, 2, 3], chr.prg_windows());

    // The register is the whole of $8000-$FFFF: a write at $FFFF decodes the same, through the
    // byte of 8 KiB bank 3 that is there — $03 & 3 is $03, so P = 3 is banks 12 to 15.
    let mut top = MapperRom::new(77)
        .prg(32)
        .chr(32)
        .prg_bank_all_ones(0)
        .load();
    top.mapper_write(0xFFFF, 0x03);
    assert_eq!([12, 13, 14, 15], top.prg_windows());
}

/// The split CHR, which is this board's reason to exist: "PPU $0000-$07FF: swappable CHR-ROM" and
/// "$0800-$1FFF: fixed CHR-RAM", the page's implementation note sizing the rest — "you do not need
/// to care about the specific order of the CHR-RAM banks: just provide 10KiB from $0800-$2FFF".
///
/// §5 MF-6c step 5 of `docs/review/MF-6-inventory.md` asks for "a pattern read from CHR-ROM at
/// $0000 and from CHR-RAM at $0800 in the same test", and this is it; the 10 KiB are checked as
/// the two pieces they are, 6 KiB of CHR-RAM and the four distinct nametables of the four-screen
/// mode (MF-6c step 6).
#[cfg(feature = "board-77")]
#[test]
fn req_core_100_mapper_77_reads_chr_rom_at_0000_and_chr_ram_at_0800() {
    let mut core = MapperRom::new(77)
        .prg(32)
        .chr(32)
        .prg_bank_all_ones(0)
        .load();

    // $0000-$07FF is the ROM chip through the bank register.
    assert_eq!((0, 1), (peek(&core, 0x0000), peek(&core, 0x0400)));
    core.mapper_write(0x8000, 0x30); // C = 3: 1 KiB banks 6 and 7. P = 0 keeps bank 0 at $8000.
    assert_eq!((6, 7), (peek(&core, 0x0000), peek(&core, 0x0400)));
    chr_write(&mut core, 0x0000, 0xA5);
    assert_eq!(6, peek(&core, 0x0000), "the ROM window swallowed the write");

    // $0800-$1FFF is the RAM chip, all 6 KiB of it, and the three 2 KiB regions are distinct.
    let ram = [
        (0x0800u16, 0x11u8),
        (0x1000, 0x22),
        (0x1800, 0x33),
        (0x1FFF, 0x44),
    ];
    for (address, value) in ram {
        chr_write(&mut core, address, value);
    }
    assert_eq!(
        [0x11, 0x22, 0x33, 0x44],
        ram.map(|(address, _)| peek(&core, address))
    );

    // The bank register does not move it: C = 5 is ROM banks 10 and 11 and the RAM is where it
    // was, which is the page's "fixed CHR-RAM".
    core.mapper_write(0x8000, 0x50);
    assert_eq!((10, 11), (peek(&core, 0x0000), peek(&core, 0x0400)));
    assert_eq!(
        [0x11, 0x22, 0x33, 0x44],
        ram.map(|(address, _)| peek(&core, address))
    );

    // Neither CHR dispatch can describe two chips in one view, so both answer `None` and the PPU
    // takes `Mapper::ppu_peek` one fetch at a time — the TQROM's answer, in `DecodeBoard`'s note.
    assert!(core.cartridge().unwrap().chr_view().is_none());
    assert!(core.cartridge().unwrap().chr_peek_view().is_none());

    // 6 KiB, and not REQ-CORE-012's 32 KiB default: that default only applies to an image with no
    // CHR-ROM, and byte 5 of this one declares 32 KiB of it.
    assert_eq!(
        0x1800,
        core.cartridge().unwrap().memory().chr_ram().len(),
        "the board's own RAM chip"
    );

    // The other 4 KiB of the page's ten: $2000-$2FFF is four distinct nametables, out of this
    // core's own 4 KiB of console VRAM, so the cartridge serves no nametable at all.
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
    for (table, value) in [
        (0x2000u16, 0x50u8),
        (0x2400, 0x51),
        (0x2800, 0x52),
        (0x2C00, 0x53),
    ] {
        core.write_memory(0x2006, (table >> 8) as u8);
        core.write_memory(0x2006, (table & 0xFF) as u8);
        core.write_memory(0x2007, value);
    }
    let nametable = |core: &mut NesCore, address: u16| {
        core.write_memory(0x2006, (address >> 8) as u8);
        core.write_memory(0x2006, (address & 0xFF) as u8);
        core.ppu_read(7); // The read buffer is one fetch behind.
        core.ppu_read(7)
    };
    assert_eq!(
        [0x50, 0x51, 0x52, 0x53],
        [0x2000u16, 0x2400, 0x2800, 0x2C00].map(|address| nametable(&mut core, address))
    );
}

// ------------------------------------------- Jaleco JF-17 / JF-19 / JF-13 (72, 92, 86)

/// nesdev ("INES Mapper 072"): the port at $8000-$FFFF is two latches, each loading on the
/// **rising edge** of its own bit — "When a 1 is written after a 0 was previously written, load
/// the bank specified by D" for bit 7 (PRG) and bit 6 (CHR). $C000-$FFFF is the page's `{ -1}`,
/// wired (MF-6c step 7).
#[cfg(feature = "board-jaleco-72-92-86")]
#[test]
fn req_core_100_the_jf17_latches_load_on_a_rising_edge_of_their_own_bit() {
    // The writes go to $C000, inside the **wired** window, whose 8 KiB bank 14 is filled with
    // $FF: this board has bus conflicts, so a write through a numbered bank would be masked, and
    // that window is the only one that cannot move under the test's feet. $E000-$FFFF is the
    // other half of the same window and is **not** usable, because that is where the fixture's
    // stub program and the vectors live.
    let mut core = MapperRom::new(72)
        .prg(16)
        .chr(128)
        .prg_bank_all_ones(14)
        .load();
    assert_eq!(72, core.cartridge().unwrap().mapper_number());
    assert_eq!([0, 1, 255, 15], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));

    // Bit 7 clear: no load, whatever the field says.
    core.mapper_write(0xC000, 0x03);
    assert_eq!([0, 1, 255, 15], core.prg_windows(), "no edge, no load");
    // 0 -> 1: the 16 KiB bank of bits 2-0, at $8000-$BFFF.
    core.mapper_write(0xC000, 0x83);
    assert_eq!([6, 7, 255, 15], core.prg_windows());
    // 1 -> 1 is not an edge: the field is ignored.
    core.mapper_write(0xC000, 0x85);
    assert_eq!([6, 7, 255, 15], core.prg_windows(), "still no edge");
    // 1 -> 0 is not one either, and then 0 -> 1 loads the new field.
    core.mapper_write(0xC000, 0x05);
    assert_eq!([6, 7, 255, 15], core.prg_windows());
    core.mapper_write(0xC000, 0x85);
    assert_eq!([10, 11, 255, 15], core.prg_windows());
    // Bit 3 is "Required by mapper 92 games only": on this board the field is three bits, so $8F
    // is bank 7 -- the image's last, which is the one $C000 is wired to.
    core.mapper_write(0xC000, 0x0F);
    core.mapper_write(0xC000, 0x8F);
    assert_eq!([255, 15, 255, 15], core.prg_windows());

    // The CHR latch is bit 6, independent of bit 7 and of the PRG field.
    core.mapper_write(0xC000, 0x47); // Bit 6 rising: 8 KiB CHR bank 7.
    assert_eq!(56, core.chr_bank(0x0000));
    assert_eq!(
        63,
        core.chr_bank(0x1C00),
        "one 8 KiB bank over the whole range"
    );
    core.mapper_write(0xC000, 0x45); // 1 -> 1: ignored.
    assert_eq!(56, core.chr_bank(0x0000), "no edge on bit 6 either");
    core.mapper_write(0xC000, 0x05);
    core.mapper_write(0xC000, 0x45);
    assert_eq!(40, core.chr_bank(0x0000));
    // And the PRG windows did not move while the CHR latch was being driven.
    assert_eq!([255, 15, 255, 15], core.prg_windows());

    // Mirroring is a "Cuttable trace", so it is the header's and no write changes it.
    let before = core.mirroring();
    core.mapper_write(0xC000, 0xFF);
    assert_eq!(before, core.mirroring());
}

/// nesdev ("INES Mapper 092"): "This mapper is identical to mapper 072 except for the different
/// PRG Setup" — the page's two diagrams put the register at $8000-$BFFF with `{ -1}` fixed above
/// it on mapper 72, and `{ 0 }` fixed below it with the register at $C000-$FFFF on mapper 92. The
/// fourth bank bit is the one marked "Required by mapper 92 games only" (MF-6c step 7).
#[cfg(feature = "board-jaleco-72-92-86")]
#[test]
fn req_core_100_mapper_92_drives_the_other_window_with_a_fourth_bank_bit() {
    // One write of $8B on each number, through 8 KiB bank 0 filled with $FF. On mapper 92 that
    // window is the fixed `{ 0 }` and cannot move; on mapper 72 it is the register's own, and one
    // write is all this test needs from it.
    let image = |mapper: usize| MapperRom::new(mapper).prg(32).chr(128).prg_bank_all_ones(0);

    let mut jf17 = image(72).load();
    assert_eq!(
        [255, 1, 30, 31],
        jf17.prg_windows(),
        "72: the fixed window is the image's last"
    );
    jf17.mapper_write(0x8000, 0x8B);
    assert_eq!(
        [6, 7, 30, 31],
        jf17.prg_windows(),
        "72: three bits, so $8B is 16 KiB bank 3"
    );

    let mut jf19 = image(92).load();
    assert_eq!(92, jf19.cartridge().unwrap().mapper_number());
    assert_eq!(
        [255, 1, 30, 31],
        jf19.prg_windows(),
        "92: the fixed window is bank 0"
    );
    jf19.mapper_write(0x8000, 0x8B);
    assert_eq!(
        [255, 1, 22, 23],
        jf19.prg_windows(),
        "92: four bits, so $8B is 16 KiB bank 11, and at $C000"
    );

    // The CHR half is the same board on both numbers: bit 6's edge, four bits of 8 KiB bank.
    jf19.mapper_write(0x8000, 0x4C);
    assert_eq!(96, jf19.chr_bank(0x0000));
}

/// nesdev ("INES Mapper 086"): `$6000-$6FFF: [.CPP ..CC]`, the CHR field's third bit sitting up at
/// bit 6 above a GNROM-like pair; `$7000-$7FFF: [..RP TTTT]`, the `µPD7756C`'s track, start and
/// reset. And the page's design flaw: "accidental copies of the registers are present at
/// $E000-$FFFF because /ROMSEL is high when M2 is low, not just when A15 is low" (MF-6c step 7).
#[cfg(feature = "board-jaleco-72-92-86")]
#[test]
fn req_core_100_the_jf13_registers_have_accidental_copies_at_e000() {
    // 128 KiB of PRG is four 32 KiB banks for the two-bit field and 64 KiB of CHR is eight 8 KiB
    // banks for the three-bit one, which is the board the page describes.
    let mut core = MapperRom::new(86).prg(16).chr(64).load();
    assert_eq!(86, core.cartridge().unwrap().mapper_number());
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));

    // "..PP" at bits 5-4 and "..CC" at bits 1-0.
    core.mapper_write(0x6000, 0x23);
    assert_eq!([8, 9, 10, 11], core.prg_windows(), "32 KiB PRG bank 2");
    assert_eq!(24, core.chr_bank(0x0000), "8 KiB CHR bank 3");
    // **Bit 6 is the CHR field's third bit**: $61 is PRG bank 2 again and CHR bank 5, not 1.
    core.mapper_write(0x6000, 0x61);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(40, core.chr_bank(0x0000), "bit 6 is CHR A15");

    // The accidental copy: $E000-$EFFF *is* $6000-$6FFF, because `address & $7000` cannot tell
    // them apart — which is what a /ROMSEL driven by M2 rather than by A15 amounts to.
    core.mapper_write(0xE123, 0x10);
    assert_eq!([4, 5, 6, 7], core.prg_windows(), "$E000 is the $6000 port");
    assert_eq!(0, core.chr_bank(0x0000));
    // And $F000-$FFFF is the **audio** port's copy, so it moves no bank at all.
    core.mapper_write(0xF000, 0xFF);
    assert_eq!([4, 5, 6, 7], core.prg_windows(), "$F000 is the $7000 port");
    assert_eq!(0, core.chr_bank(0x0000));
    // $8000-$DFFF decodes nothing: `address & $7000` is neither $6000 nor $7000 anywhere in it.
    for address in [0x8000u16, 0x9000, 0xA000, 0xB000, 0xC000, 0xD000] {
        core.mapper_write(address, 0x30);
        assert_eq!([4, 5, 6, 7], core.prg_windows(), "{address:#06X}");
    }

    // "No PRG RAM (mapper ports occupy $6000-$7FFF instead)": the window is the register.
    assert_eq!(0, core.cartridge().unwrap().memory().prg_ram().len());
}

/// §4.8 of `docs/review/MF-6-inventory.md`, and the decision it takes rather than defers: the
/// three Jaleco boards get their boards and **not their `µPD7756C` audio**. REQ-CORE-075's list of
/// expansion-audio chips — "VRC6, VRC7, MMC5, Namco 163, Sunsoft 5B, FDS" — does not include it,
/// and its samples are an ADPCM mask ROM inside the ASIC, in no source this project may read.
///
/// The silence is pinned so that it is a choice and not a bug, the way
/// `q_123_vrc6_keeps_its_nametables_on_ciram_when_n_asks_for_chr_rom` pins the VRC6's ignored bit
/// (MF-6c step 7).
#[cfg(feature = "board-jaleco-72-92-86")]
#[test]
fn mf_6c_the_three_jaleco_upd7756c_boards_are_silent() {
    for mapper in [72usize, 92, 86] {
        let mut core = MapperRom::new(mapper).prg(16).chr(64).load();
        assert!(
            !core.cartridge().unwrap().has_expansion_audio(),
            "mapper {mapper} declares no expansion audio"
        );
        // Every write the audio ports take, on both of this family's two decodes: the track
        // number, the start bit low and the reset bit low.
        for (address, value) in [
            (0x7000u16, 0x0Au8),
            (0x7000, 0x1A),
            (0x7000, 0x2A),
            (0xF000, 0x0A),
            (0xE000, 0x00),
            (0x8000, 0x30),
            (0x8000, 0x10),
            (0x8000, 0x20),
        ] {
            core.mapper_write(address, value);
            assert_eq!(
                None,
                core.cartridge().unwrap().expansion_audio_output(),
                "mapper {mapper}, {address:#06X} = {value:#04X}"
            );
        }
    }
}

// ------------------------------------------- Oeka Kids (96)

/// Drives the PPU's address bus across the edge that latches the Oeka Kids board's CHR bank: out
/// of $2000-$2FFF and then into `address`. The board latches on the entry and on nothing else.
#[cfg(feature = "board-96")]
fn latch_chr_page(core: &mut NesCore, address: u16) {
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x0000, 1);
    cartridge.on_ppu_address_bus(address, 2);
}

/// nesdev ("INES Mapper 096"): `CPU $8000-$FFFF: [xxxx xCPP]`, two bits of 32 KiB PRG bank with
/// the 16 KiB outer CHR bank above them, "PRG ROM capacity: 128 KiB", "PRG RAM capacity: No" and
/// "Hardwired Vertical mirroring" (MF-6c step 8).
#[cfg(feature = "board-96")]
#[test]
fn req_core_100_mapper_96_banks_32_kib_of_prg_and_has_no_prg_ram() {
    // The register is at $8000-$FFFF and the board has bus conflicts (Q-135), so the write goes
    // through 8 KiB bank 0 while the power-on bank is still there.
    let image = || MapperRom::new(96).prg(16).chr(0).prg_bank_all_ones(0);
    let mut core = image().load();
    assert_eq!(96, core.cartridge().unwrap().mapper_number());
    assert_eq!([255, 1, 2, 3], core.prg_windows());
    assert_eq!(Nm::Vertical, core.mirroring(), "hardwired");

    // "..PP": two bits, four 32 KiB banks in 128 KiB.
    core.mapper_write(0x8000, 0x02);
    assert_eq!([8, 9, 10, 11], core.prg_windows());

    // Bit 2 is the outer CHR bank and moves no PRG window: $05 is PRG bank 1 either way.
    let mut outer = image().load();
    outer.mapper_write(0x8000, 0x05);
    assert_eq!([4, 5, 6, 7], outer.prg_windows());
    let mut plain = image().load();
    plain.mapper_write(0x8000, 0x01);
    assert_eq!([4, 5, 6, 7], plain.prg_windows());

    // "PRG RAM capacity: No".
    assert_eq!(0, core.cartridge().unwrap().memory().prg_ram().len());
    // "CHR capacity: 32 KiB RAM", which is eight of the 4 KiB pages the board banks in.
    assert_eq!(
        32 * 1024,
        core.cartridge().unwrap().memory().chr_ram().len()
    );
}

/// **The board's whole mechanism**, and §4.4 of `docs/review/MF-6-inventory.md` is why it is the
/// last board of the lot: the 4 KiB CHR-RAM page at PPU $0000-$0FFF is latched off the **PPU
/// address bus**, on the entry into $2000-$2FFF and from bits 9-8 of the address —
///
/// ```text
/// 13  PPU address  0
///  DD xxCC xxxx xxxx
///  ||   ||
///  ||   ++---------- Select 4 KiB inner CHR RAM bank for PPU $0000-$0FFF
///  ++--------------- Latch CC bits when 'DD' changes from anything else to '10'
/// ```
///
/// — and the page is explicit that it happens with no fetch behind it: "This latching occurs
/// regardless of whether the PPU or game code initiated the address change, enabling software to
/// manually select CHR pages by writing specific values to $2006." **That is the claim §5 MF-6c
/// asks this test for**, and the one MR-1's open MMC2 question bears on (MF-6c step 8).
#[cfg(feature = "board-96")]
#[test]
fn req_core_100_mapper_96_latches_its_chr_page_from_the_ppu_address_bus() {
    let mut core = MapperRom::new(96)
        .prg(16)
        .chr(0)
        .prg_bank_all_ones(0)
        .load();

    // Put a different byte in each of the eight 4 KiB CHR-RAM pages, through the board's own two
    // selects: bit 2 of the register for the outer 16 KiB bank, the PPU address for the inner one.
    // The register writes are $00 and $04, which both leave PRG bank 0 (and its $FF bytes) at
    // $8000 so that the next one survives the bus conflict.
    for outer in [0x00u8, 0x04] {
        core.mapper_write(0x8000, outer);
        for inner in 0..4u16 {
            latch_chr_page(&mut core, 0x2000 | (inner << 8));
            chr_write(&mut core, 0x0000, 0xA0 | (outer << 2) | inner as u8);
        }
    }

    // And read them back the same way. Bits 9-8 of the address are the page, so $2000, $2100,
    // $2200 and $2300 are pages 0 to 3 of the selected outer bank.
    for outer in [0x00u8, 0x04] {
        core.mapper_write(0x8000, outer);
        for inner in 0..4u16 {
            latch_chr_page(&mut core, 0x2000 | (inner << 8));
            assert_eq!(
                0xA0 | (outer << 2) | inner as u8,
                peek(&core, 0x0000),
                "outer {outer:#04X}, inner {inner}"
            );
            // $1000-$1FFF is the outer bank's **last** page, wired, whatever the latch holds.
            assert_eq!(
                0xA0 | (outer << 2) | 0x03,
                peek(&core, 0x1000),
                "the wired window, outer {outer:#04X}"
            );
        }
    }

    // **The latch is an edge and not a decode**: once inside $2000-$2FFF, moving the address
    // within it changes nothing. Page 1 is selected, and walking to $2200 and $2300 leaves it.
    core.mapper_write(0x8000, 0x00);
    latch_chr_page(&mut core, 0x2100);
    assert_eq!(0xA1, peek(&core, 0x0000));
    for address in [0x2200u16, 0x2300, 0x2000, 0x2FFF] {
        core.cartridge_mut().unwrap().on_ppu_address_bus(address, 3);
        assert_eq!(
            0xA1,
            peek(&core, 0x0000),
            "no edge into $2000-$2FFF, no latch: {address:#06X}"
        );
    }
    // An address outside the range does not latch either, and then the next entry does.
    core.cartridge_mut().unwrap().on_ppu_address_bus(0x1300, 4);
    assert_eq!(0xA1, peek(&core, 0x0000));
    core.cartridge_mut().unwrap().on_ppu_address_bus(0x2200, 5);
    assert_eq!(0xA2, peek(&core, 0x0000));

    // **A bare `$2006` write, with no fetch behind it**, which is the page's own claim and the
    // reason this board is in `Cartridge::watches_ppu_bus_without_read` as well as in
    // `watches_ppu_reads`. Only the *second* write of a $2006 pair loads `v` and reaches the
    // cartridge, so a game selects a page the way the hardware makes it: one pair that leaves
    // $2000-$2FFF, then one that enters it at the page it wants.
    let write_v = |core: &mut NesCore, address: u16| {
        let _ = core.read_memory(0x2002); // Clear the two-write latch, whatever it held.
        core.write_memory(0x2006, (address >> 8) as u8);
        core.write_memory(0x2006, (address & 0xFF) as u8);
    };
    write_v(&mut core, 0x0000);
    assert_eq!(
        0xA2,
        peek(&core, 0x0000),
        "leaving the range latches nothing"
    );
    write_v(&mut core, 0x2300);
    assert_eq!(
        0xA3,
        peek(&core, 0x0000),
        "a $2006 write of $2300 selects page 3 with no fetch at all"
    );
    write_v(&mut core, 0x1000);
    write_v(&mut core, 0x2000);
    assert_eq!(0xA0, peek(&core, 0x0000), "and $2000 selects page 0");
    // A second pair into the range is **no** edge, so it cannot change the page: the game has to
    // leave it first, which is the hardware's own rule and not an approximation of it.
    write_v(&mut core, 0x2300);
    assert_eq!(
        0xA0,
        peek(&core, 0x0000),
        "two entries in a row are one edge"
    );

    // The whole of it survives a save state, **the previous address included**: the edge needs
    // both of its sides and nothing derives the one that is in the past (REQ-CORE-090).
    write_v(&mut core, 0x1000);
    write_v(&mut core, 0x2100);
    let state = core.save_state(true).unwrap();
    let mut other = MapperRom::new(96)
        .prg(16)
        .chr(0)
        .prg_bank_all_ones(0)
        .load();
    other.try_load_state(&state, 0).unwrap();
    assert_eq!(0xA1, peek(&other, 0x0000), "the latched page came back");
    other.cartridge_mut().unwrap().on_ppu_address_bus(0x2300, 6);
    assert_eq!(
        0xA1,
        peek(&other, 0x0000),
        "and so did the previous address, so $2300 is no edge"
    );
}

// ------------------------------------------- GTROM / Cheapocabra (111)

/// Reads a nametable byte the way the PPU does, through the board when it serves its own.
#[cfg(any(feature = "board-111", feature = "board-jy-asic"))]
fn nametable(core: &NesCore, address: u16) -> u8 {
    core.cartridge().unwrap().read_nametable(address)
}

/// Writes one, the same way.
#[cfg(feature = "board-111")]
fn nametable_write(core: &mut NesCore, address: u16, value: u8) {
    core.cartridge_mut()
        .unwrap()
        .write_nametable(address, value);
}

/// nesdev ("GTROM"): one register at "$5000-$5FFF and $7000-$7FFF", `GRNC PPPP` — four bits of
/// 32 KiB PRG bank with "no portion of the bank is fixed", bit 4 the 8 KiB CHR-RAM bank, bit 5 the
/// 8 KiB nametable bank, and the two LED bits above them (MF-6c step 9).
#[cfg(feature = "board-111")]
#[test]
fn req_core_100_the_gtrom_banks_32_kib_of_prg_from_either_register_window() {
    // 512 KiB of PRG is the board's capacity: sixteen 32 KiB banks for the four-bit field.
    let mut core = MapperRom::new(111).prg(64).chr(0).load();
    assert_eq!(111, core.cartridge().unwrap().mapper_number());
    assert_eq!([0, 1, 2, 3], core.prg_windows(), "no window is fixed");

    // $5000-$5FFF, and the whole 32 KiB moves: a four-bit field over sixteen banks.
    core.mapper_write(0x5000, 0x0F);
    assert_eq!([60, 61, 62, 63], core.prg_windows());
    core.mapper_write(0x5ABC, 0x05);
    assert_eq!([20, 21, 22, 23], core.prg_windows());
    // $7000-$7FFF is the same register, which is also why the board has no PRG-RAM there.
    core.mapper_write(0x7FFF, 0x0A);
    assert_eq!([40, 41, 42, 43], core.prg_windows());
    assert_eq!(0, core.cartridge().unwrap().memory().prg_ram().len());
    // And nothing between or below the two windows decodes: $4FFF and $6000-$6FFF are not it.
    core.mapper_write(0x6000, 0x01);
    core.mapper_write(0x6FFF, 0x02);
    assert_eq!(
        [40, 41, 42, 43],
        core.prg_windows(),
        "$6000-$6FFF is not the register"
    );

    // Bit 4, the 8 KiB CHR-RAM bank, over the 32 KiB chip's first half. The two banks are
    // distinct memory, which is what one write to each proves.
    chr_write(&mut core, 0x0000, 0x11);
    core.mapper_write(0x5000, 0x1A); // Bit 4 set: the other CHR bank, same PRG bank.
    assert_eq!([40, 41, 42, 43], core.prg_windows());
    chr_write(&mut core, 0x0000, 0x22);
    assert_eq!(0x22, peek(&core, 0x0000));
    core.mapper_write(0x5000, 0x0A);
    assert_eq!(
        0x11,
        peek(&core, 0x0000),
        "the first CHR bank kept its byte"
    );

    // The two LED bits are decoded into the register and drive nothing; what they must not do is
    // disturb the banks they share a byte with.
    core.mapper_write(0x5000, 0xCA);
    assert_eq!([40, 41, 42, 43], core.prg_windows());
    assert_eq!(0x11, peek(&core, 0x0000));

    // "The board contains 32KB of RAM": two 8 KiB CHR banks and two 8 KiB nametable banks.
    assert_eq!(
        32 * 1024,
        core.cartridge().unwrap().memory().chr_ram().len()
    );
}

/// The nametables, which are the reason this board is the lot's last: **two** 8 KiB banks, each
/// holding four distinct tables and the bonus RAM above them.
///
/// > This provides unique memory in both banks for all 4 nametables. This means the mapper always
/// > uses 4-screen mirroring and can neither use nor benefit from a switch to control the
/// > nametable arrangement.
///
/// and, of $3000-$3EFF:
///
/// > On this board these addresses are not mirrored and instead are wired to real, independent
/// > addresses on the RAM chip.
///
/// So `nametable_page` answers `SERVED_BY_CARTRIDGE` always, and this is the **fourth** distinct
/// body of the `Mapper::read_nametable` dispatch Q-123 measured a cap of two on (MF-6c step 9).
#[cfg(feature = "board-111")]
#[test]
fn req_core_100_the_gtrom_serves_two_banks_of_four_distinct_nametables() {
    let mut core = MapperRom::new(111).prg(64).chr(0).load();
    assert_eq!(
        [-2, -2, -2, -2],
        nametable_pages(&core),
        "SERVED_BY_CARTRIDGE, for all four tables and always"
    );
    assert_eq!(Nm::FourScreen, core.mirroring());

    // Bank 0: four distinct tables, and the bonus RAM of $3000-$3EFF behind them.
    let cells = [
        0x2000u16, 0x2400, 0x2800, 0x2C00, 0x3000, 0x3400, 0x3800, 0x3C00,
    ];
    for (i, address) in cells.into_iter().enumerate() {
        nametable_write(&mut core, address, 0x40 | i as u8);
    }
    for (i, address) in cells.into_iter().enumerate() {
        assert_eq!(0x40 | i as u8, nametable(&core, address), "{address:#06X}");
    }

    // Bit 5: the other 8 KiB bank, eight more distinct kilobytes.
    core.mapper_write(0x5000, 0x20);
    for address in cells {
        assert_eq!(0, nametable(&core, address), "a fresh bank, {address:#06X}");
    }
    for (i, address) in cells.into_iter().enumerate() {
        nametable_write(&mut core, address, 0x80 | i as u8);
    }
    for (i, address) in cells.into_iter().enumerate() {
        assert_eq!(0x80 | i as u8, nametable(&core, address), "{address:#06X}");
    }
    // And bank 0 is where it was left.
    core.mapper_write(0x5000, 0x00);
    for (i, address) in cells.into_iter().enumerate() {
        assert_eq!(0x40 | i as u8, nametable(&core, address), "{address:#06X}");
    }

    // The nametable half is behind the CHR half in the same chip, so a CHR write cannot reach it:
    // 16 KiB of CHR and 16 KiB of nametable in the 32 KiB the page gives the board.
    chr_write(&mut core, 0x0000, 0xFF);
    core.mapper_write(0x5000, 0x10);
    chr_write(&mut core, 0x0000, 0xFF);
    core.mapper_write(0x5000, 0x00);
    assert_eq!(
        0x40,
        nametable(&core, 0x2000),
        "the CHR banks and the nametable banks are different halves of the chip"
    );

    // All of it survives a save state, the selected bank included (REQ-CORE-090).
    core.mapper_write(0x5000, 0x20);
    let state = core.save_state(true).unwrap();
    let mut other = MapperRom::new(111).prg(64).chr(0).load();
    other.try_load_state(&state, 0).unwrap();
    assert_eq!(0x80, nametable(&other, 0x2000));
    assert_eq!(0x87, nametable(&other, 0x3C00));
    other.mapper_write(0x5000, 0x00);
    assert_eq!(0x40, nametable(&other, 0x2000));
}

/// The SST39SF040 the game saves into, on this board's chip addressing: "Like UNROM 512, GTROM
/// uses a Microchip SST39SF040 flash chip" and "Programs must execute from RAM while erasing and
/// writing". The sequences are the chip's, on **chip** addresses $5555 and $2AAA, which is R0's
/// answer to Q-117; persistence and the software-ID commands are the gaps MR-30-4 records for the
/// UNROM 512 (MF-6c step 9).
#[cfg(feature = "board-111")]
#[test]
fn req_core_100_the_gtrom_programs_and_erases_its_own_prg_rom() {
    let mut core = MapperRom::new(111).prg(64).chr(0).load();
    // Bank 0 is at $8000-$FFFF, so chip $5555 is CPU $D555 and chip $2AAA is CPU $AAAA.
    let unlock = |core: &mut NesCore| {
        core.mapper_write(0xD555, 0xAA);
        core.mapper_write(0xAAAA, 0x55);
    };

    // Program a byte: the sequence, then the address and the data. 8 KiB bank 1 reads 1, and
    // flash can only clear bits, so $01 & $FE is $00.
    assert_eq!(1, core.read_memory(0xA123));
    unlock(&mut core);
    core.mapper_write(0xD555, 0xA0);
    core.mapper_write(0xA123, 0xFE);
    assert_eq!(0, core.read_memory(0xA123), "the byte was programmed");
    // A byte already at 0 cannot be raised: that is the chip, not an approximation of it.
    unlock(&mut core);
    core.mapper_write(0xD555, 0xA0);
    core.mapper_write(0xA123, 0xFF);
    assert_eq!(0, core.read_memory(0xA123), "flash only clears bits");

    // Erase the 4 KiB sector it is in, which takes the whole sector to $FF and leaves the next
    // one alone. $A123 is in the sector $A000-$AFFF; $B000 is in the one after it.
    assert_eq!(1, core.read_memory(0xB000));
    unlock(&mut core);
    core.mapper_write(0xD555, 0x80);
    unlock(&mut core);
    core.mapper_write(0xA123, 0x30);
    assert_eq!(0xFF, core.read_memory(0xA000));
    assert_eq!(0xFF, core.read_memory(0xAFFF));
    assert_eq!(1, core.read_memory(0xB000), "the next sector is untouched");

    // The bank is part of the chip address: the same CPU address in another PRG bank is another
    // sector of the chip, and the one just erased stays erased.
    core.mapper_write(0x5000, 0x01);
    assert_eq!(5, core.read_memory(0xA123), "8 KiB bank 5, another sector");
    core.mapper_write(0x5000, 0x00);
    assert_eq!(0xFF, core.read_memory(0xA123));

    // A sequence that goes wrong writes nothing: the wrong unlock value aborts it, and the data
    // write that follows is just a write to ROM.
    core.mapper_write(0xD555, 0xAA);
    core.mapper_write(0xAAAA, 0x54); // Not $55.
    core.mapper_write(0xD555, 0xA0);
    core.mapper_write(0xB123, 0x00);
    assert_eq!(
        1,
        core.read_memory(0xB123),
        "an aborted sequence programs nothing"
    );
}

/// The chip's two unlock writes. They are on **chip** addresses $5555 and $2AAA, so they are only
/// reachable while the register holds **PRG bank 0**, where they are CPU $D555 and $AAAA — which
/// is the pair nesdev names in those terms (MF-UNL-9).
#[cfg(feature = "board-111")]
fn gtrom_unlock(core: &mut NesCore) {
    core.mapper_write(0xD555, 0xAA);
    core.mapper_write(0xAAAA, 0x55);
}

/// `$5555:$AA`, `$2AAA:$55`, `$5555:$80`, `$5555:$AA`, `$2AAA:$55`, `ADDR:$30` — the sector erase.
#[cfg(feature = "board-111")]
fn gtrom_erase(core: &mut NesCore, address: u16) {
    gtrom_unlock(core);
    core.mapper_write(0xD555, 0x80);
    gtrom_unlock(core);
    core.mapper_write(address, 0x30);
}

/// `$5555:$AA`, `$2AAA:$55`, `$5555:$A0`, `ADDR:DATA` — the byte program.
#[cfg(feature = "board-111")]
fn gtrom_program(core: &mut NesCore, address: u16, value: u8) {
    gtrom_unlock(core);
    core.mapper_write(0xD555, 0xA0);
    core.mapper_write(address, value);
}

/// How many sectors of the chip a game has written.
#[cfg(feature = "board-111")]
fn gtrom_dirty_sectors(core: &NesCore) -> usize {
    core.cartridge()
        .unwrap()
        .memory()
        .flash()
        .unwrap() // Mapper 111's PRG-ROM is a flash chip; no other board's is.
        .dirty_sectors()
}

/// A GTROM with **three** sectors of its chip written: `$A123` = `$3C` and `$E010` = `$C3` in PRG
/// bank 0 (chip sectors 10 and 14), and bank 3's sector 26 erased whole. The register is left
/// holding bank 0.
///
/// The third one is the interesting one. The unlock pair is only reachable in bank 0 — chip
/// `$5555` is CPU `$D555` there and nowhere else — so a game that flashes another bank unlocks in
/// bank 0 and moves the register **before the last write of the sequence**, which is a CPU-bus
/// write the chip never sees. Chip address `$1A123`, four banks along from bank 0's `$A123`.
#[cfg(feature = "board-111")]
fn gtrom_flashed() -> NesCore {
    let mut core = MapperRom::new(111).prg(64).chr(0).load();
    // A game erases before it programs, because programming only clears bits; after the erase the
    // sector is $FF and any value fits.
    gtrom_erase(&mut core, 0xA123);
    gtrom_program(&mut core, 0xA123, 0x3C);
    gtrom_erase(&mut core, 0xE010);
    gtrom_program(&mut core, 0xE010, 0xC3);

    gtrom_unlock(&mut core);
    core.mapper_write(0xD555, 0x80);
    gtrom_unlock(&mut core);
    core.mapper_write(0x5000, 0x03);
    core.mapper_write(0xA123, 0x30);
    assert_eq!(0xFF, core.read_memory(0xA123), "bank 3's sector was erased");
    core.mapper_write(0x5000, 0x00);

    assert_eq!(0x3C, core.read_memory(0xA123), "bank 0's sector was not");
    assert_eq!(0xC3, core.read_memory(0xE010));
    assert_eq!(3, gtrom_dirty_sectors(&core));
    core
}

/// **The save file**, which MF-UNL-9 step 1 added and which is the whole reason the census grades
/// this one mapper number an L: nesdev ("GTROM") is categorical that the chip is the board's only
/// save medium —
///
/// > Both of these mappers lack PRG RAM for game saves but the configuration of both boards make
/// > it possible to erase and rewrite sections of the chip using NES software.
///
/// — and the census asks that the file be "the changed sectors rather than a separate RAM image"
/// (REQ-CORE-095).
#[cfg(feature = "board-111")]
#[test]
fn req_core_095_the_gtrom_flash_save_is_the_changed_sectors() {
    // A chip nothing has written has nothing to save, so a host writes no file at all for the
    // overwhelming majority of sessions.
    let fresh = MapperRom::new(111).prg(64).chr(0).load();
    assert_eq!(0, gtrom_dirty_sectors(&fresh));
    assert!(
        fresh.export_flash_save().is_empty(),
        "a fresh chip has nothing to save"
    );

    // **The changed sectors, and not an image of the chip**: three 4 KiB sectors and a short
    // header, against a 512 KiB chip.
    let core = gtrom_flashed();
    let save = core.export_flash_save();
    assert!(
        (3 * 4 * 1024..4 * 4 * 1024).contains(&save.len()),
        "three sectors and a header, not the 512 KiB chip: {} bytes",
        save.len()
    );
    // The two programmed bytes **appear in the file**, each inside the $FF of its erased sector.
    assert!(
        save.windows(3).any(|bytes| bytes == [0xFF, 0x3C, 0xFF]),
        "the byte programmed at $A123 is in the file"
    );
    assert!(
        save.windows(3).any(|bytes| bytes == [0xFF, 0xC3, 0xFF]),
        "and the one at $E010"
    );

    // The file read back into a fresh machine: the two bytes, the whole erased sector around the
    // first of them, the sector of another bank, and nothing else.
    let mut reloaded = MapperRom::new(111).prg(64).chr(0).load();
    assert_eq!(1, reloaded.read_memory(0xA123), "the dump's own byte");
    assert!(reloaded.import_flash_save(&save));
    assert_eq!(0x3C, reloaded.read_memory(0xA123));
    assert_eq!(
        0xFF,
        reloaded.read_memory(0xA000),
        "the erased sector came back whole"
    );
    assert_eq!(0xFF, reloaded.read_memory(0xAFFF));
    assert_eq!(
        1,
        reloaded.read_memory(0xB000),
        "and no sector the file did not name"
    );
    assert_eq!(0xC3, reloaded.read_memory(0xE010));
    reloaded.mapper_write(0x5000, 0x03);
    assert_eq!(0xFF, reloaded.read_memory(0xA123), "bank 3's sector too");

    // A reset keeps all of it: a flash chip is not volatile memory, and this is the one save
    // medium of this build that `Cartridge::clear_volatile_ram` cannot reach.
    reloaded.reset();
    assert_eq!(0x3C, reloaded.read_memory(0xA123));
    assert_eq!(3, gtrom_dirty_sectors(&reloaded));

    // A file this build does not read changes nothing, the way `import_battery_ram` refuses a
    // board with no battery.
    assert!(!reloaded.import_flash_save(&[]));
    assert!(!reloaded.import_flash_save(b"not a flash save file at all"));
    assert!(
        !reloaded.import_flash_save(&save[..save.len() - 1]),
        "a truncated file is refused"
    );
    assert_eq!(
        0x3C,
        reloaded.read_memory(0xA123),
        "and the chip is left as it was"
    );
}

/// The savestate half of the census's test: "a flashed byte survives a save-state round-trip"
/// (REQ-CORE-090, REQ-CORE-091).
///
/// Before MF-UNL-9 the cartridge's section of the block carried this board's three registers and
/// no flash at all, so a flashed byte was lost by a round trip; the changed sectors are now
/// written at the end of that section behind `NesCore::state_version_has_flash`.
#[cfg(feature = "board-111")]
#[test]
fn req_core_090_the_gtrom_flashed_sectors_come_back_with_its_registers() {
    let core = gtrom_flashed();
    let state = core.save_state(true).unwrap();

    let mut other = MapperRom::new(111).prg(64).chr(0).load();
    other.try_load_state(&state, 0).unwrap();
    assert_eq!(0x3C, other.read_memory(0xA123));
    assert_eq!(0xC3, other.read_memory(0xE010));
    assert_eq!(3, gtrom_dirty_sectors(&other));
    other.mapper_write(0x5000, 0x03);
    assert_eq!(0xFF, other.read_memory(0xA123), "bank 3's sector too");

    // And a state loaded into a machine that flashed something **else** leaves the chip holding
    // what the state holds and no more: the sector this third machine erased is the dump's again.
    // That is what the per-sector pristine copy of `FlashRom` is for, and it is the one thing a
    // save file alone would not have needed.
    let mut third = MapperRom::new(111).prg(64).chr(0).load();
    gtrom_erase(&mut third, 0xC000);
    assert_eq!(0xFF, third.read_memory(0xC000));
    third.try_load_state(&state, 0).unwrap();
    assert_eq!(0x3C, third.read_memory(0xA123));
    assert_eq!(
        2,
        third.read_memory(0xC000),
        "the sector this machine erased and the state did not is the dump's again"
    );
    assert_eq!(
        3,
        gtrom_dirty_sectors(&third),
        "and its dirty set is the state's three sectors"
    );
}

/// The counterpart of `req_core_095_a_cartridge_without_a_battery_exposes_nothing` for the second
/// save medium: a board whose PRG-ROM is mask ROM has no flash chip, exposes no file and refuses
/// one (REQ-CORE-095, MF-UNL-9 step 2).
#[test]
fn req_core_095_a_board_without_a_flash_chip_exposes_nothing() {
    let mut core = load(4);
    assert!(core.cartridge().unwrap().memory().flash().is_none());

    let empty: [u8; 0] = [];
    assert_eq!(core.export_flash_save(), empty);
    assert!(!core.import_flash_save(&[1, 2, 3]));
}

/// "This means the mapper always uses 4-screen mirroring and can neither use nor benefit from a
/// switch to control the nametable arrangement" — so neither the header's flag nor any value of
/// the register can make it anything else (MF-UNL-9).
#[cfg(feature = "board-111")]
#[test]
fn req_core_100_the_gtrom_is_four_screen_whatever_the_header_says() {
    // Byte 6 bit 0 is the vertical/horizontal flag and bit 3 the four-screen one: all four
    // combinations, including the two that ask for something else.
    for bits in [0x00u8, 0x01, 0x08, 0x09] {
        let mut core = MapperRom::new(111).prg(64).chr(0).mirroring(bits).load();
        assert_eq!(Nm::FourScreen, core.mirroring(), "header bits {bits:#04X}");
        assert_eq!(
            [-2, -2, -2, -2],
            nametable_pages(&core),
            "header bits {bits:#04X}: served by the cartridge, all four"
        );

        // Every value of the register — both nametable banks, both CHR banks, both LEDs — leaves
        // it four-screen, because there is no bit that could do anything else.
        for register in [0x00u8, 0x10, 0x20, 0x30, 0xC0, 0xFF] {
            core.mapper_write(0x5000, register);
            assert_eq!(
                Nm::FourScreen,
                core.mirroring(),
                "header {bits:#04X}, register {register:#04X}"
            );
        }

        core.reset();
        assert_eq!(Nm::FourScreen, core.mirroring(), "and after a reset");
    }
}

/// The two LEDs: "The LED lights on the board are universally unsupported at this time", and this
/// build has no host surface for an indicator on a cartridge either, so bits 6 and 7 are decoded
/// into the register byte — which puts them in the savestate for free — and drive nothing.
///
/// What they must not do is disturb anything they share that byte with, which is everything the
/// board has (MF-UNL-9).
#[cfg(feature = "board-111")]
#[test]
fn req_core_100_the_gtrom_led_bits_change_nothing_observable() {
    let mut core = MapperRom::new(111).prg(64).chr(0).load();

    // PRG bank 10, CHR bank 1, nametable bank 1, both LEDs on.
    core.mapper_write(0x5000, 0x3A);
    chr_write(&mut core, 0x0000, 0x11);
    nametable_write(&mut core, 0x2000, 0x22);
    let windows = core.prg_windows();
    for leds in [0x40u8, 0x80, 0xC0] {
        core.mapper_write(0x5000, 0x3A | leds);
        assert_eq!(windows, core.prg_windows(), "LEDs {leds:#04X}");
        assert_eq!(0x11, peek(&core, 0x0000), "LEDs {leds:#04X}: the CHR bank");
        assert_eq!(
            0x22,
            nametable(&core, 0x2000),
            "LEDs {leds:#04X}: the nametable bank"
        );
        assert_eq!(Nm::FourScreen, core.mirroring(), "LEDs {leds:#04X}");
    }

    // Nor the flash: the LEDs are written through the register window and the chip only sees
    // $8000-$FFFF, so an LED write in the middle of a command sequence does not abort it. Bank 0
    // throughout, because that is where the chip's unlock addresses are reachable.
    core.mapper_write(0x5000, 0x20);
    gtrom_erase(&mut core, 0xA123);
    gtrom_unlock(&mut core);
    core.mapper_write(0x5000, 0xE0); // Both LEDs off, same banks.
    core.mapper_write(0xD555, 0xA0);
    core.mapper_write(0xA123, 0x3C);
    assert_eq!(
        0x3C,
        core.read_memory(0xA123),
        "an LED write is not a chip write"
    );
}

#[test]
fn req_core_100_jaleco_ss88006_loads_its_banks_one_nibble_at_a_time() {
    let mut core = MapperRom::new(18).prg(32).chr(32).load();
    core.write_memory(0x8000, 0x05); // $8000 bank low nibble.
    core.write_memory(0x8001, 0x01); // high nibble: bank $15.
    core.write_memory(0x8002, 0x03); // $A000 bank 3.
    core.write_memory(0x8003, 0x00);
    core.write_memory(0x9000, 0x07); // $C000 bank 7.
    core.write_memory(0x9001, 0x00);
    assert_eq!([0x15, 3, 7, 31], core.prg_windows());
    core.write_memory(0xA000, 0x09); // CHR 0 low nibble.
    core.write_memory(0xA001, 0x01); // high nibble: bank $19.
    assert_eq!(0x19, core.chr_bank(0x0000));
    core.write_memory(0xF002, 0x01);
    assert_eq!(Nm::Vertical, core.mirroring());
}

/// The register offset of VRC register `index` under the wiring of `mapper`.
fn vrc_register(mapper: usize, index: u16) -> u16 {
    match mapper {
        21 => ((index & 1) << 1) | ((index & 2) << 1),
        22 | 25 => ((index & 1) << 1) | ((index & 2) >> 1),
        _ => index,
    }
}

#[test]
fn req_core_100_vrc_switches_two_prg_banks_and_fixes_the_last_two() {
    // Mapper 22 is left out of the swap half: it is a VRC2, which has no swap mode at all
    // (MR-21-2, `mr_21_2_vrc2a_has_no_prg_swap_mode`).
    for mapper in [21usize, 22, 23, 25, 27] {
        let mut core = MapperRom::new(mapper).prg(16).chr(32).load();
        core.write_memory(0x8000, 0x03);
        core.write_memory(0xA000, 0x05);
        assert_eq!([3, 5, 14, 15], core.prg_windows(), "mapper {mapper}");
        // $9002 bit 1 moves the first switchable bank to $C000.
        core.write_memory(0x9000 | vrc_register(mapper, 2), 0x02);
        let expected = if mapper == 22 {
            [3, 5, 14, 15]
        } else {
            [14, 5, 3, 15]
        };
        assert_eq!(expected, core.prg_windows(), "mapper {mapper}");
    }
}

#[test]
fn req_core_100_vrc2a_addresses_chr_in_2_kib_units() {
    // Mapper 22 swaps the two register-select lines and shifts the CHR bank right by one,
    // which is what the public m22chrbankingtest ROM displays.
    let mut core = MapperRom::new(22).chr(64).load();
    core.write_memory(0xB002, 0x04); // Register index 1 on this wiring: CHR 0 high.
    core.write_memory(0xB000, 0x06); // CHR 0 low.
    assert_eq!((0x46 >> 1) & 0x3F, core.chr_bank(0x0000));
}

#[test]
fn req_core_100_vrc4_builds_each_chr_bank_from_two_nibbles() {
    let mut core = MapperRom::new(23).chr(64).load();
    core.write_memory(0xB000, 0x05); // CHR 0 low nibble.
    core.write_memory(0xB001, 0x02); // CHR 0 high nibble: bank $25.
    assert_eq!(0x25, core.chr_bank(0x0000));
    core.write_memory(0xE002, 0x0F); // CHR 7 low nibble.
    core.write_memory(0xE003, 0x01); // bank $1F.
    assert_eq!(0x1F, core.chr_bank(0x1C00));
}

fn action53_select(core: &mut zax_nes::NesCore, register: u8, value: u8) {
    core.write_memory(0x5000, register);
    core.write_memory(0x8000, value);
}

#[test]
fn req_core_100_action53_combines_an_outer_and_an_inner_prg_bank() {
    // AMD-021 R2-001: the outer register counts 32 KiB banks (E = 2 x outer) and power-on is
    // outer = 63, inner = 15, mode = 0, which on a 256 KiB image wraps to the last 32 KiB.
    let mut core = MapperRom::new(28).prg(32).chr(0).load();
    assert_eq!([28, 29, 30, 31], core.prg_windows());
    // Mode $1B: horizontal (bits 0-1 = 3), UNROM layout (bits 2-3 = 2), 64 KiB game (bits 4-5
    // = 1, so m = 3 and the inner bank picks one of four 16 KiB banks).
    action53_select(&mut core, 0x80, 0x1B);
    action53_select(&mut core, 0x81, 0x04); // Outer bank 4, in 32 KiB units: E = 8.
    action53_select(&mut core, 0x01, 0x02); // Inner bank 2 inside the game.
    assert_eq!(Nm::Horizontal, core.mirroring());
    // Layout 2: $8000 shows E = 8 and $C000 shows (8 & ~3) | 2 = 10.
    assert_eq!([16, 17, 20, 21], core.prg_windows());
    // Layout 0 shows a 32 KiB game: (8 & ~3) | (2I & 3) = 8, then 9.
    action53_select(&mut core, 0x80, 0x10);
    assert_eq!([16, 17, 18, 19], core.prg_windows());
    assert_eq!(Nm::SingleScreenA, core.mirroring());
}

#[test]
fn amd_021_r2_001_action53_acceptance_vectors() {
    // A numbered 1 MiB image: 16 KiB bank n is the pair of 8 KiB banks (2n, 2n + 1).
    let mut core = MapperRom::new(28).prg(128).chr(0).load();
    // Power-on: outer = 63 -> E = 126, mode 0, inner = 15 -> banks 126 and 127, which a
    // 64-bank image wraps to 62 and 63.
    assert_eq!([124, 125, 126, 127], core.prg_windows());
    // (ext = 5, mode = 0) -> 10 / 11.
    action53_select(&mut core, 0x81, 0x05);
    assert_eq!([20, 21, 22, 23], core.prg_windows());
    // (ext = 2, mode = $18, int = 3) -> 4 / 7.
    action53_select(&mut core, 0x81, 0x02);
    action53_select(&mut core, 0x80, 0x18);
    action53_select(&mut core, 0x01, 0x03);
    assert_eq!([8, 9, 14, 15], core.prg_windows());
    // (ext = 2, mode = $1C, int = 3) -> 7 / 5.
    action53_select(&mut core, 0x80, 0x1C);
    assert_eq!([14, 15, 10, 11], core.prg_windows());
}

#[test]
fn amd_021_r2_001_action53_takes_the_single_screen_bit_from_a_chr_write() {
    // AMD-021 R2-001: while bit 1 of the mode is clear, a CHR or inner-PRG write replaces bit
    // 0 of the mode with bit 4 of the value.
    let mut core = MapperRom::new(28).prg(128).chr(0).load();
    assert_eq!(Nm::SingleScreenA, core.mirroring());
    core.write_memory(0x5000, 0x00);
    core.write_memory(0x8000, 0x10);
    assert_eq!(Nm::SingleScreenB, core.mirroring());
    // Back to screen A, and a write to the outer register never touches the mode.
    core.write_memory(0x5000, 0x01);
    core.write_memory(0x8000, 0x00);
    assert_eq!(Nm::SingleScreenA, core.mirroring());
    core.write_memory(0x5000, 0x81);
    core.write_memory(0x8000, 0x1F);
    assert_eq!(Nm::SingleScreenA, core.mirroring());
    // With bit 1 of the mode set (the two-screen layouts) the bit is left alone.
    core.write_memory(0x5000, 0x80);
    core.write_memory(0x8000, 0x02); // Vertical.
    core.write_memory(0x5000, 0x00);
    core.write_memory(0x8000, 0x10);
    assert_eq!(Nm::Vertical, core.mirroring());
}

#[test]
fn req_core_100_unrom512_switches_prg_chr_ram_and_mirroring() {
    // The battery bit is how a submapper-0 UNROM 512 declares that it does *not* have bus
    // conflicts (MR-BUS-1, nesdev "UNROM 512"); byte 6 keeps bits 3 and 0, so the arrangement
    // is the same `%....1..0` as before. That also makes it self-flashable, so the bank latch
    // is at $C000-$FFFF and $8000-$BFFF is the flash write window (Q-117).
    let mut core = MapperRom::new(30)
        .prg(32)
        .chr(0)
        .mirroring(0x08)
        .battery()
        .load();
    core.write_memory(0xC000, 0x45); // PRG bank 5, CHR-RAM bank 2, screen A.
    assert_eq!([10, 11, 30, 31], core.prg_windows());
    assert_eq!(Nm::SingleScreenA, core.mirroring());
    core.write_memory(0xC000, 0xC5);
    assert_eq!(Nm::SingleScreenB, core.mirroring());
}

/// Q-117 — and a configuration that is **not** self-flashable keeps the latch over the whole of
/// $8000-$FFFF, which is what the page's unqualified `A~[1... .... .... ....]` says. Submapper 0
/// without the battery bit is that case (it has bus conflicts), so the write goes through a
/// bank filled with $FF.
#[test]
fn q_117_unrom512_without_self_flashing_keeps_the_latch_over_8000_ffff() {
    let mut core = MapperRom::new(30)
        .prg(32)
        .chr(0)
        .mirroring(0x08)
        .prg_bank_all_ones(0)
        .load();
    core.write_memory(0x8000, 0x05);
    assert_eq!([10, 11, 30, 31], core.prg_windows());
}

#[test]
fn req_core_100_irem_g101_switches_two_prg_banks_eight_chr_banks_and_mirroring() {
    let mut core = MapperRom::new(32).prg(16).chr(32).load();
    core.write_memory(0x8000, 0x03);
    core.write_memory(0xA000, 0x05);
    assert_eq!([3, 5, 14, 15], core.prg_windows());
    core.write_memory(0x9000, 0x03); // Horizontal, PRG mode 1.
    assert_eq!(Nm::Horizontal, core.mirroring());
    assert_eq!([14, 5, 3, 15], core.prg_windows());
    for register in 0..8u16 {
        core.write_memory(0xB000 + register, 24 + register as u8);
    }

    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], core.chr_windows());
}

#[test]
fn req_core_100_bnrom_switches_32_kib_from_8000() {
    // A BNROM register write is ANDed with the PRG-ROM content (MR-34-2), so the write goes
    // through a bank of $FF; the decode is what this test is about.
    let mut core = MapperRom::new(34)
        .prg(16)
        .chr(0)
        .prg_bank_all_ones(0)
        .load();
    core.write_memory(0x8000, 0x02);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
}

#[test]
fn req_core_100_nina001_has_its_registers_at_the_top_of_the_prg_ram_window() {
    let mut core = MapperRom::new(34).prg(16).chr(32).load();
    core.write_memory(0x7FFD, 0x01);
    core.write_memory(0x7FFE, 0x03);
    core.write_memory(0x7FFF, 0x06);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!([12, 13, 14, 15, 24, 25, 26, 27], core.chr_windows());
}

#[test]
fn req_core_100_caltron_takes_the_prg_bank_from_the_address_and_the_chr_from_the_data() {
    let mut core = MapperRom::new(41).prg(16).chr(64).load();
    // `..MC CPPP` (MR-41-1/2): bits 0-2 the PRG bank, bits 3-4 the outer CHR bank, bit 5 the
    // mirroring. PRG 32 KiB bank 6, which wraps to 2 in a 128 KiB image; horizontal; CHR high
    // 1. Bank 6 also opens the inner CHR register, which bank 2 alone would not (MR-41-3).
    core.write_memory(0x6006 | 0x20 | 0x08, 0x00);
    core.write_memory(0x8000, 0x01);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(((1 << 2) | 1) * 8, core.chr_bank(0x0000));
    assert_eq!(Nm::Horizontal, core.mirroring());
}

#[test]
fn req_core_100_rambo1_maps_three_prg_banks_and_a_1_kib_chr_mode() {
    let mut core = MapperRom::new(64).prg(16).chr(32).load();
    core.mmc3_bank(6, 3);
    core.mmc3_bank(7, 5);
    core.mmc3_bank(15, 9);
    assert_eq!([3, 5, 9, 15], core.prg_windows());
    // MR-64-7: PRG mode 1 swaps R6 with R15 and leaves R7 at $A000.
    core.mapper_write(0x8000, 0x40);
    assert_eq!([9, 5, 3, 15], core.prg_windows());
    // In the 1 KiB CHR mode, registers 8 and 9 fill the odd kilobytes of the first two
    // windows. The bank registers go in first, since selecting one clears the mode bits.
    core.mmc3_bank(0, 16);
    core.mmc3_bank(8, 17);
    core.mmc3_bank(1, 18);
    core.mmc3_bank(9, 19);
    core.mmc3_bank(2, 20);
    core.mmc3_bank(3, 21);
    core.mmc3_bank(4, 22);
    core.mmc3_bank(5, 23);
    core.mapper_write(0x8000, 0x20);
    assert_eq!([16, 17, 18, 19, 20, 21, 22, 23], core.chr_windows());
}

#[test]
fn req_core_100_gxrom_splits_its_register_between_prg_and_chr() {
    // The GxROM board table gives mapper 66 bus conflicts (MR-66-1), and the PRG field is bits
    // 4-5, so the write goes through a bank of $FF.
    let mut core = MapperRom::new(66)
        .prg(16)
        .chr(32)
        .prg_bank_all_ones(0)
        .load();
    core.write_memory(0x8000, 0x23);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], core.chr_windows());
}

#[test]
fn req_core_100_codemasters_switches_16_kib_and_can_pick_one_screen() {
    let mut core = MapperRom::new(71).prg(16).chr(8).load();
    core.write_memory(0xC000, 0x05);
    assert_eq!([10, 11, 14, 15], core.prg_windows());
    core.write_memory(0x9000, 0x10);
    assert_eq!(Nm::SingleScreenB, core.mirroring());
}

#[test]
fn req_core_100_jaleco_j87_swaps_the_two_bits_of_its_chr_bank() {
    let mut core = MapperRom::new(87).prg(4).chr(32).load();
    core.write_memory(0x6000, 0x01); // Bank 2, not 1.
    assert_eq!(2 * 8, core.chr_bank(0x0000));
    core.write_memory(0x6000, 0x02); // Bank 1.
    assert_eq!(8, core.chr_bank(0x0000));
}

#[test]
fn amd_061_mapper_101_takes_eight_bits_from_6000_7fff() {
    // AMD-061 §4's criterion, which replaces AMD-021 R2-003's: a numbered 64 KiB CHR image,
    // eight 8 KiB banks, and 8 KiB CHR bank n is the run of 1 KiB banks starting at 8n.
    let mut core = MapperRom::new(101).prg(4).chr(64).load();
    // "$6000 <- $06 -> bank 6": the whole byte, where R2-003 took `value & 3` = 2.
    core.write_memory(0x6000, 0x06);
    assert_eq!(6 * 8, core.chr_bank(0x0000));
    // "$7FFF <- $0D -> bank 5": the top of the window still decodes, and 13 modulo 8 is 5.
    core.write_memory(0x7FFF, 0x0D);
    assert_eq!(5 * 8, core.chr_bank(0x0000));
    // "$E000 <- $03 -> no change": $8000-$FFFF is not decoded at all, where R2-003 took the
    // writes whose address carried bits 13 and 14.
    core.write_memory(0xE000, 0x03);
    assert_eq!(5 * 8, core.chr_bank(0x0000));
    // Nor is any other address above the register window.
    core.write_memory(0x8000, 0x01);
    core.write_memory(0xA000, 0x01);
    core.write_memory(0xC000, 0x02);
    assert_eq!(5 * 8, core.chr_bank(0x0000));
}

#[test]
fn req_core_100_multicart_225_takes_everything_from_the_address() {
    let mut core = MapperRom::new(225).prg(32).chr(64).load();
    // 32 KiB mode: PRG bank field 2 (16 KiB units, low bit dropped) and CHR bank 3.
    core.write_memory(0x8000 | (2 << 6) | 3, 0x00);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(3 * 8, core.chr_bank(0x0000));
    assert_eq!(Nm::Vertical, core.mirroring());
    // 16 KiB mode, mirrored over both windows, horizontal layout.
    core.write_memory(0x8000 | 0x2000 | 0x1000 | (5 << 6), 0x00);
    assert_eq!([10, 11, 10, 11], core.prg_windows());
    assert_eq!(Nm::Horizontal, core.mirroring());
}

#[test]
fn req_core_100_action52_splits_the_chr_bank_between_address_and_data() {
    let mut core = MapperRom::new(228).prg(32).chr(64).load();
    core.write_memory(0x8000 | (2 << 6) | 0x01, 0x02);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(((1 << 2) | 2) * 8, core.chr_bank(0x0000)); // Wraps to 64 KiB of CHR: 48 < 64.
    core.write_memory(0x8000 | 0x20 | (3 << 6), 0x00);
    assert_eq!([6, 7, 6, 7], core.prg_windows());
}

// ---------------------------------------------------------------- reset

#[test]
fn req_core_004_a_reset_puts_the_mapper_back_to_its_power_on_banks() {
    for mapper in [1usize, 4, 16, 64] {
        let mut core = MapperRom::new(mapper).prg(16).chr(32).load();
        let power_on = core.prg_windows();
        core.mmc1_write(0xE000, 0x05);
        core.mmc3_bank(6, 7);
        core.write_memory(0x8008, 0x03);
        core.reset();
        assert_eq!(power_on, core.prg_windows(), "mapper {mapper}");
        assert!(!core.cartridge().unwrap().irq_asserted());
    }
}

#[test]
fn amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state() {
    // AMD-061 §5 amends REQ-CORE-004's "mapper revenu dans son état initial" to exempt a board
    // whose public documentation says its state survives the RESET signal, and the list it
    // opens holds exactly one board: mapper 28. The MMC1 is named as *not* being on it. So
    // every other supported board must still come back, which is what this sweeps.
    for &mapper in NesCore::SUPPORTED_MAPPERS {
        if mapper == 28 {
            continue;
        }

        let mapper_number = usize::try_from(mapper).unwrap();
        let mut core = MapperRom::new(mapper_number).prg(16).chr(32).load();
        let power_on = (core.prg_windows(), core.chr_windows(), core.mirroring());
        for (address, value) in [
            (0x5000u16, 0x01u8),
            (0x6000, 0x02),
            (0x8000, 0x03),
            (0x9000, 0x04),
            (0xA000, 0x05),
            (0xC000, 0x06),
            (0xE000, 0x07),
        ] {
            core.write_memory(address, value);
        }

        core.reset();
        assert_eq!(
            power_on,
            (core.prg_windows(), core.chr_windows(), core.mirroring()),
            "mapper {mapper} did not come back to its power-on state"
        );
        assert!(!core.cartridge().unwrap().irq_asserted(), "mapper {mapper}");
    }
}

#[test]
fn amd_061_mapper_28_keeps_its_registers_across_a_reset() {
    // AMD-061 §3: "Réinitialisation : l'état de la carte (registres externe, interne, de mode
    // et de sélection) est conservé", from nesdev's "The mapper state is unchanged on reset"
    // (finding MR-28-3, ticket Q-112).
    let mut core = MapperRom::new(28).prg(128).chr(0).load();
    // Power-on is AMD-021 R2-001's outer 63 / inner 15 / mode 0, which §3 keeps: the last
    // 16 KiB bank of the image sits at $C000-$FFFF.
    assert_eq!([124, 125, 126, 127], core.prg_windows());

    // The same vector as `amd_021_r2_001_action53_acceptance_vectors`' third one.
    action53_select(&mut core, 0x81, 0x02);
    action53_select(&mut core, 0x80, 0x18);
    action53_select(&mut core, 0x01, 0x03);
    let running = (core.prg_windows(), core.mirroring());
    assert_eq!([8, 9, 14, 15], running.0);

    core.reset();
    assert_eq!(
        running,
        (core.prg_windows(), core.mirroring()),
        "the four registers of mapper 28 must survive a reset"
    );

    // And a reset still restores everything REQ-CORE-004 keeps requiring of it: §5 changes
    // only the mapper point.
    assert!(!core.cartridge().unwrap().irq_asserted());
}

// ---------------------------------------------------------------- savestates

#[test]
fn req_core_090_the_mapper_state_survives_a_save_and_restore() {
    for &mapper in NesCore::SUPPORTED_MAPPERS {
        let mapper_number = usize::try_from(mapper).unwrap();
        let mut core = MapperRom::new(mapper_number)
            .prg(16)
            .chr(32)
            .battery()
            .load();

        // Move whatever the board has: a few register writes, then some frames so that any
        // IRQ counter and latch has run.
        for (address, value) in [
            (0x8000, 0x01),
            (0x8001, 0x02),
            (0xA000, 0x03),
            (0xB000, 0x04),
            (0xC000, 0x05),
            (0xC001, 0x06),
            (0xE001, 0x07),
            (0x6000, 0x42),
        ] {
            core.write_memory(address, value);
        }

        for _ in 0..2 {
            core.run_frame(0);
        }

        let state = core.save_state(true).unwrap();
        let windows = core.prg_windows();
        let chr = core.chr_windows();

        for _ in 0..5 {
            core.run_frame(0);
        }

        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed, "mapper {mapper}");
        assert_eq!(windows, core.prg_windows(), "mapper {mapper}");
        assert_eq!(chr, core.chr_windows(), "mapper {mapper}");
        assert_eq!(state, core.save_state(true).unwrap(), "mapper {mapper}");
    }
}

/// §7.1 item 2 of `docs/review/UNL-census.md`: "A save-state round-trip per board with mapper
/// state dirtied first" — for the twelve numbers of lot MF-UNL-2, each through **its own**
/// registers.
///
/// `req_core_090_the_mapper_state_survives_a_save_and_restore` sweeps every number of
/// `SUPPORTED_MAPPERS` and dirties each one through `$8000-$E001` and `$6000`, which is where
/// every board of the earlier lots keeps its registers. **Not one of this lot's own additions is
/// at any of those addresses**: mapper 195's GAL register is written through the *PPU*, 123's
/// override is at `$5800`, 187's at `$5000`, 121's protection ports at `$5000`/`$5180`/`$8003`,
/// 189's bank at `$4100`, and 114's and 115's outer CHR bank at `$6001`. So the generic sweep
/// proves the chip's block round-trips and says nothing about the bytes this lot appends.
///
/// This test writes each board's own ports, saves, runs five frames, restores, and requires the
/// re-saved bytes to be identical — which is the sweep's own criterion and covers every field
/// including the derived ones (REQ-CORE-090, REQ-CORE-091).
#[cfg(all(
    feature = "board-mmc3-chrram",
    feature = "board-mmc3-scramble",
    feature = "board-263",
    feature = "board-189"
))]
#[test]
// The function *is* a twelve-row table -- one arm per board, with the page each register
// comes from -- written twice, once to dirty and once to move what was dirtied.
#[allow(clippy::too_many_lines)]
fn req_core_090_every_board_of_mf_unl_2_round_trips_with_its_own_registers_dirtied() {
    // What the board itself shows, and nothing of the machine around it: the four PRG windows
    // and the eight CHR window **offsets**. The offsets and not the bytes, because a 512-bank
    // CHR-ROM filled with bank numbers reads the same byte 256 banks apart -- and CHR A18 is
    // exactly a move of 256 banks, so bytes could not see 114's and 115's `$6001`.
    fn board_layout(core: &NesCore) -> ([u8; 4], Option<[usize; 8]>) {
        (
            core.prg_windows(),
            core.cartridge()
                .unwrap()
                .chr_peek_view()
                .map(|view| view.offsets()),
        )
    }

    for mapper in [
        74usize, 114, 115, 121, 123, 182, 187, 189, 192, 194, 195, 263,
    ] {
        let mut core = MapperRom::new(mapper).prg(32).chr(512).battery().load();

        // The MMC3 underneath, at whichever addresses this board sends $8000/$8001 to.
        let submapper1 = false;
        for register in 0..8u8 {
            match mapper {
                263 => {
                    // A12 drives A0 and the data is permuted, so the index has to be the byte
                    // that permutes to the register number. `reverse` is a search, which a test
                    // may do.
                    let permute = |v: u8| -> u8 {
                        (v & 0xD8)
                            | ((v & 0x20) >> 4)
                            | ((v & 0x04) << 3)
                            | ((v & 0x02) >> 1)
                            | ((v & 0x01) << 2)
                    };
                    let unpermute = |want: u8| -> u8 {
                        (0..=255u8)
                            .find(|candidate| permute(*candidate) == want)
                            .unwrap_or(want)
                    };
                    core.mapper_write(0x8000, unpermute(register));
                    core.mapper_write(0x9000, unpermute(register + 0x11));
                }
                114 | 115 | 123 | 182 | 187 => {
                    scrambled_mmc3_bank(&mut core, mapper, submapper1, register, register + 0x11);
                }
                _ => core.mmc3_bank(register, register + 0x11),
            }
        }

        // And then this board's own, one line per page.
        match mapper {
            // The GAL's `CHRB`, through a PPU write to a window the CHR-ROM drives: bank $3F is
            // in no row of the page's table. `$8A` selects $64-$67, and R2 is then parked *in*
            // that window so that the register is **observable** -- bank $64 reads the RAM chip
            // under `$8A` and the CHR-ROM under the `$C8` written after the capture.
            195 => {
                core.mmc3_bank(2, 0x3F);
                chr_write(&mut core, 0x1000, 0x8A);
                core.mmc3_bank(2, 0x64);
                chr_write(&mut core, 0x1000, 0x5E);
            }
            // The outer CHR bank at $6001, and the NROM override at $6000.
            114 | 115 | 182 => {
                core.mapper_write(0x6001, 0x01);
                core.mapper_write(0x6000, 0xA5);
            }
            // The override at $5800, with M at bit 6.
            123 => core.mapper_write(0x5800, 0x40 | 0x25),
            // The override at $5000, and a protection read for good measure.
            187 => {
                core.mapper_write(0x5000, 0xAA);
                let _ = core.read_memory(0x5000);
            }
            // The protection index, the A9713's outer bank, and the $8003 table with a
            // bit-reversed $8001 behind it.
            121 => {
                core.mapper_write(0x5000, 0x02);
                core.mapper_write(0x5180, 0x80);
                core.mapper_write(0x8003, 0x26);
                core.mapper_write(0x8001, 0x02);
            }
            // The 32 KiB bank.
            189 => core.mapper_write(0x4100, 0x30),
            _ => {}
        }

        // The counter too, at the addresses this board sends $C000-$E001 to, and two frames so
        // that it has run.
        if matches!(mapper, 114 | 182) {
            core.mapper_write(0xA001, 0x20); // means $C000
            core.mapper_write(0xC001, 0x00); // means $C001
            core.mapper_write(0xE001, 0x00); // means $E001
        } else if mapper == 263 {
            core.mapper_write(0xC000, 0x00); // $C000 permutes to 0, which is latch 0
            core.mapper_write(0xD000, 0x00);
            core.mapper_write(0xF000, 0x00);
        } else {
            core.mapper_write(0xC000, 0x20);
            core.mapper_write(0xC001, 0x00);
            core.mapper_write(0xE001, 0x00);
        }
        for _ in 0..2 {
            core.run_frame(0);
        }

        let state = core.save_state(true).unwrap();
        let layout = board_layout(&core);
        let chr_byte = peek(&core, 0x1000);

        for _ in 0..5 {
            core.run_frame(0);
        }

        // **And then the board's own registers again, differently** -- without this the test
        // would pass on a board whose appended byte is never written to the state at all,
        // because the live field would still hold what the capture held. Each line below must
        // move the layout, and the assertions after the load are what prove it came back.
        match mapper {
            // `$C8` selects $0A-$0B, where `$8A` selected $64-$67.
            195 => {
                core.mmc3_bank(2, 0x3F);
                chr_write(&mut core, 0x1000, 0xC8);
            }
            114 | 115 | 182 => {
                core.mapper_write(0x6001, 0x00);
                core.mapper_write(0x6000, 0x8C);
            }
            123 => core.mapper_write(0x5800, 0x40 | 0x10),
            187 => core.mapper_write(0x5000, 0x8C),
            121 => {
                core.mapper_write(0x5000, 0x01);
                core.mapper_write(0x5180, 0x00);
                core.mapper_write(0x8003, 0x2A);
                core.mapper_write(0x8001, 0x3E);
            }
            189 => core.mapper_write(0x4100, 0x10),
            _ => {}
        }
        // **74, 192, 194 and 263 append nothing**: their whole save block is the chip's, which
        // `req_core_090_the_mapper_state_survives_a_save_and_restore` already covers, so there
        // is no second write to make and nothing for this assertion to prove. Every other board
        // of the lot has at least one byte of its own, and for those the second write above
        // **must** move what the board shows — otherwise the assertions below would pass on a
        // board that never wrote that byte to the state at all, because the live field would
        // still hold what the capture held. (That is not hypothetical: the first version of this
        // test did exactly that, and dropping mapper 195's GAL byte from `save_registers` on
        // purpose did not fail it.)
        if !matches!(mapper, 74 | 192 | 194 | 263) {
            assert_ne!(
                (layout, chr_byte),
                (board_layout(&core), peek(&core, 0x1000)),
                "mapper {mapper}: the second set of register writes must move what the board \
                 shows, or the assertions below prove nothing about the bytes this lot appends"
            );
        }

        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed, "mapper {mapper}");
        assert_eq!(layout, board_layout(&core), "mapper {mapper}: the windows");
        assert_eq!(
            chr_byte,
            peek(&core, 0x1000),
            "mapper {mapper}: the byte PPU $1000 shows, which is the CHR *source* and so the \
             only observation that can see mapper 195's GAL register"
        );
        assert_eq!(
            state,
            core.save_state(true).unwrap(),
            "mapper {mapper}: the whole block, derived fields included"
        );
    }
}

/// AMD-061's own sweep, `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state`,
/// reaches every number of `SUPPORTED_MAPPERS` — and for the same reason as the save-state sweep
/// above it dirties none of this lot's own registers. This one does, and then asks for the
/// power-on layout back.
///
/// Mapper 195's power-on is the one that is not all zeroes: its `CHRB` powers on at `$80`
/// (Q-142), so after a reset banks `$28-$2B` are the RAM chip again whatever the GAL was left
/// holding.
#[cfg(all(
    feature = "board-mmc3-chrram",
    feature = "board-mmc3-scramble",
    feature = "board-263",
    feature = "board-189"
))]
#[test]
fn amd_061_a_reset_puts_every_board_of_mf_unl_2_back_to_its_power_on_state() {
    for mapper in [
        74usize, 114, 115, 121, 123, 182, 187, 189, 192, 194, 195, 263,
    ] {
        let core = MapperRom::new(mapper).prg(32).chr(64).load();
        let power_on = (core.prg_windows(), core.chr_windows(), core.mirroring());

        let mut core = core;
        for register in 0..8u8 {
            core.mapper_write(0x8000, register);
            core.mapper_write(0x8001, register + 0x11);
        }
        match mapper {
            195 => {
                core.mmc3_bank(2, 0x3F);
                chr_write(&mut core, 0x1000, 0x8A);
            }
            114 | 115 | 182 => {
                core.mapper_write(0x6001, 0x01);
                core.mapper_write(0x6000, 0xA5);
            }
            123 => core.mapper_write(0x5800, 0x40 | 0x25),
            187 => core.mapper_write(0x5000, 0xAA),
            121 => {
                core.mapper_write(0x5180, 0x80);
                core.mapper_write(0x8003, 0x26);
                core.mapper_write(0x8001, 0x02);
            }
            189 => core.mapper_write(0x4100, 0x30),
            _ => {}
        }
        core.mapper_write(0xA000, 0x01);

        core.reset();

        assert_eq!(
            power_on,
            (core.prg_windows(), core.chr_windows(), core.mirroring()),
            "mapper {mapper}: a reset must give the power-on layout back"
        );
        assert!(
            !core.cartridge().unwrap().irq_asserted(),
            "mapper {mapper}: and no pending IRQ"
        );
    }
}

#[test]
fn req_core_095_battery_backed_prg_ram_is_exposed_and_survives_a_reload() {
    for mapper in [1usize, 4, 5, 10] {
        let select_ram_bank = |core: &mut NesCore| {
            if mapper == 5 {
                core.write_memory(0x5102, 0x02);
                core.write_memory(0x5103, 0x01);
            }
        };

        let mut core = MapperRom::new(mapper).battery().load();
        select_ram_bank(&mut core);

        core.write_memory(0x6123, 0x7E);
        assert_eq!(0x7E, core.read_memory(0x6123), "mapper {mapper}");

        let saved = core.export_battery_ram().to_vec();
        assert!(!saved.is_empty(), "mapper {mapper}");

        let mut reloaded = MapperRom::new(mapper).battery().load();
        assert!(reloaded.import_battery_ram(&saved), "mapper {mapper}");

        select_ram_bank(&mut reloaded);
        assert_eq!(0x7E, reloaded.read_memory(0x6123), "mapper {mapper}");

        // A reset no longer clears it, since the host has taken charge of it.
        reloaded.reset();
        select_ram_bank(&mut reloaded);
        assert_eq!(0x7E, reloaded.read_memory(0x6123), "mapper {mapper}");
    }
}

#[test]
fn req_core_095_a_cartridge_without_a_battery_exposes_nothing() {
    let mut core = load(4);

    let empty: [u8; 0] = [];
    assert_eq!(core.export_battery_ram(), empty);
    assert!(!core.import_battery_ram(&[1, 2, 3]));
}

// ---------------------------------------------------------------- VRC6 (mappers 24 and 26)

/// The VRC6's register address for `index` (0-3) in the group at `group`, under the wiring of
/// `mapper`: "`VRC6b` (mapper 026) has A0 and A1 swapped".
fn vrc6_register(mapper: usize, group: u16, index: u16) -> u16 {
    let offset = if mapper == 26 {
        ((index & 0x01) << 1) | ((index & 0x02) >> 1)
    } else {
        index
    };
    group | offset
}

fn write_vrc6(core: &mut NesCore, mapper: usize, group: u16, index: u16, value: u8) {
    let address = vrc6_register(mapper, group, index);
    core.write_memory(address, value);
}

#[test]
fn req_core_100_vrc6_switches_a_16_kib_and_an_8_kib_prg_bank() {
    for mapper in [24usize, 26] {
        let mut core = MapperRom::new(mapper).prg(16).chr(32).load();
        // "$8000-$8003: Select 16 KB PRG ROM bank at $8000-$BFFF", four bits, so bank 3 is the
        // pair of 8 KiB banks 6 and 7.
        write_vrc6(&mut core, mapper, 0x8000, 0, 0x03);
        // "$C000-$C003: Select 8 KB PRG ROM bank at $C000-$DFFF", five bits.
        write_vrc6(&mut core, mapper, 0xC000, 0, 0x09);
        // $E000-$FFFF is absent from the page's PRG table: the last bank is wired there.
        assert_eq!([6, 7, 9, 15], core.prg_windows(), "mapper {mapper}");

        // All four mirrors of each register are the same register.
        for index in 1..4 {
            write_vrc6(&mut core, mapper, 0x8000, index, 0x02);
            assert_eq!(
                [4, 5, 9, 15],
                core.prg_windows(),
                "mapper {mapper} m{index}"
            );
            write_vrc6(&mut core, mapper, 0x8000, index, 0x03);
        }

        // The 16 KiB field is four bits and the 8 KiB field five: nothing above them latches.
        write_vrc6(&mut core, mapper, 0x8000, 0, 0xF3);
        write_vrc6(&mut core, mapper, 0xC000, 0, 0xE9);
        assert_eq!([6, 7, 9, 15], core.prg_windows(), "mapper {mapper}");
    }
}

#[test]
fn req_core_100_vrc6b_has_a0_and_a1_swapped() {
    // $D001 and $D002 are two different CHR registers, R1 and R2, so the swap is visible on
    // them; it is invisible on $C000-$C003, whose four mirrors are one register.
    let mut a = MapperRom::new(24).chr(32).load();
    let mut b = MapperRom::new(26).chr(32).load();
    a.write_memory(0xD001, 0x07);
    b.write_memory(0xD001, 0x07);

    // Mapper 24 latches R1, which is PPU $0400-$07FF.
    assert_eq!([0, 7, 0, 0, 0, 0, 0, 0], a.chr_windows());
    // Mapper 26 decodes the same address as $D002, so it latches R2 instead.
    assert_eq!([0, 0, 7, 0, 0, 0, 0, 0], b.chr_windows());
}

#[test]
fn req_core_100_vrc6_arranges_chr_in_the_four_banking_modes() {
    let mut core = MapperRom::new(24).chr(64).load();
    for register in 0..8u16 {
        let group = if register < 4 { 0xD000 } else { 0xE000 };
        core.write_memory(group | (register & 3), 0x10 | (register as u8));
    }

    // Mode 0 with P clear: eight 1 KiB banks, R0-R7 straight through.
    core.write_memory(0xB003, 0x00);
    assert_eq!(
        [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17],
        core.chr_windows()
    );

    // Mode 1 with P set: four 2 KiB banks from R0-R3, each half `R & $FE` then `R | 1` — PPU
    // A10 passes through.
    core.write_memory(0xB003, 0x21);
    assert_eq!(
        [0x10, 0x11, 0x10, 0x11, 0x12, 0x13, 0x12, 0x13],
        core.chr_windows()
    );

    // Mode 1 with P clear: "CHR A10 … according to the latched value", so both 1 KiB halves of
    // each 2 KiB window show the whole register.
    core.write_memory(0xB003, 0x01);
    assert_eq!(
        [0x10, 0x10, 0x11, 0x11, 0x12, 0x12, 0x13, 0x13],
        core.chr_windows()
    );

    // Mode 2 with P set: four 1 KiB banks from R0-R3, then two 2 KiB banks from R4 and R5.
    core.write_memory(0xB003, 0x22);
    assert_eq!(
        [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x14, 0x15],
        core.chr_windows()
    );

    // Mode 3 has the same CHR arrangement as mode 2; only its nametables differ.
    core.write_memory(0xB003, 0x23);
    assert_eq!(
        [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x14, 0x15],
        core.chr_windows()
    );
}

#[test]
fn req_core_100_vrc6_b003_fixes_four_arrangements_when_p_is_set() {
    let mut core = MapperRom::new(24).chr(32).load();
    // The four rows of the page's own tables, re-derived from its even/odd columns in
    // `Vrc6::derive_nametables`: $20/$27 vertical, $23/$24 horizontal, $28/$2F one screen A,
    // $2B/$2C one screen B. Each arrangement goes through the cartridge mirroring, so every
    // page follows it.
    for (mode, expected) in [
        (0x20, Nm::Vertical),
        (0x27, Nm::Vertical),
        (0x23, Nm::Horizontal),
        (0x24, Nm::Horizontal),
        (0x28, Nm::SingleScreenA),
        (0x2F, Nm::SingleScreenA),
        (0x2B, Nm::SingleScreenB),
        (0x2C, Nm::SingleScreenB),
    ] {
        core.write_memory(0xB003, mode);
        assert_eq!(expected, core.mirroring(), "mode {mode:#04X}");
        assert_eq!([-1, -1, -1, -1], nametable_pages(&core), "mode {mode:#04X}");
    }
}

#[test]
fn req_core_100_vrc6_picks_a_ciram_page_per_table_from_the_latched_bit() {
    let mut core = MapperRom::new(24).chr(32).load();
    core.write_memory(0xE000, 0x00); // R4 = 0.
    core.write_memory(0xE001, 0x01); // R5 = 1.
    core.write_memory(0xE002, 0x01); // R6 = 1.
    core.write_memory(0xE003, 0x00); // R7 = 0.

    // Mode 0 with P clear: each half of the screen takes one register, R6 then R7.
    core.write_memory(0xB003, 0x00);
    assert_eq!([1, 1, 0, 0], nametable_pages(&core));

    // Mode 1 is the page's "4-screen" row: one register per table, R4-R7.
    core.write_memory(0xB003, 0x01);
    assert_eq!([0, 1, 1, 0], nametable_pages(&core));

    // Mode 2 pairs the tables the other way: R6, R7, R6, R7.
    core.write_memory(0xB003, 0x02);
    assert_eq!([1, 0, 1, 0], nametable_pages(&core));

    // Only the low bit of the register is the page.
    core.write_memory(0xE002, 0xFE);
    assert_eq!([0, 0, 0, 0], nametable_pages(&core));
}

#[test]
fn q_123_vrc6_keeps_its_nametables_on_ciram_when_n_asks_for_chr_rom() {
    // Q-123: `$B003` bit 4 means "Nametables come from CHRROM", and this build does not serve
    // them — a third board overriding `Mapper::read_nametable` reshapes the dispatch that
    // `Ppu2C02::read_nametable_inner` folds in and costs every board two instructions and 8
    // bytes of stack per nametable fetch (measured; see `<Board as Mapper>::read_nametable`).
    // The provisional choice is that `N` changes nothing: the four tables stay on CIRAM, laid
    // out by the same registers, which is what this pins so that a lot implementing the mode
    // has to come here and say so.
    let mut core = MapperRom::new(24).chr(64).load();
    core.write_memory(0xE002, 0x01); // R6.
    core.write_memory(0xE003, 0x00); // R7.

    // Mode 0 with P clear and N clear: a page per half from the latched bit.
    core.write_memory(0xB003, 0x00);
    let without_n = nametable_pages(&core);
    assert_eq!([1, 1, 0, 0], without_n);

    // The same with N set: unchanged, and in particular never `SERVED_BY_CARTRIDGE`.
    core.write_memory(0xB003, 0x10);
    assert_eq!(without_n, nametable_pages(&core));
    assert!(!nametable_pages(&core).contains(&Cartridge::SERVED_BY_CARTRIDGE));

    // And the fixed arrangements still reach the cartridge mirroring with N set.
    core.write_memory(0xB003, 0x34);
    assert_eq!(Nm::Horizontal, core.mirroring());
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
}

#[test]
fn req_core_100_vrc6_enables_prg_ram_with_bit_7_of_b003() {
    let mut core = MapperRom::new(24).chr(32).load();
    // Power-on is $B003 = 0, so "PRG RAM enable" is clear and the window is open bus.
    core.write_memory(0x6000, 0x5A);
    assert_ne!(0x5A, core.read_memory(0x6000));

    core.write_memory(0xB003, 0x80);
    core.write_memory(0x6000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6000));

    // Disabling it again hides what was written without losing it.
    core.write_memory(0xB003, 0x00);
    assert_ne!(0x5A, core.read_memory(0x6000));
    core.write_memory(0xB003, 0x80);
    assert_eq!(0x5A, core.read_memory(0x6000));
}

// ------------------------------------------- the Namco 108 family (206, and 76, 88, 95, 154)

/// Loads bank register `register` with `value` through the two registers of the Namco 108: the
/// bank select at $8000-$9FFE (even) and the bank data at $8001-$9FFF (odd). The same pair the
/// MMC3 has, so [`MapperMachine::mmc3_bank`] drives it; named for what it is here.
#[cfg(feature = "board-namcot108")]
fn namcot108_bank(core: &mut NesCore, register: u8, value: u8) {
    core.mmc3_bank(register, value);
}

#[cfg(feature = "board-namcot108")]
/// nesdev ("INES Mapper 206"): `PRG Window: 8K + 8K + 16K fixed`, R6 at $8000-$9FFF and R7 at
/// $A000-$BFFF, with "the last two 8KiB banks fixed to the end".
#[test]
fn req_core_100_namcot_108_switches_two_prg_windows_and_fixes_the_last_two() {
    let mut core = MapperRom::new(206).prg(8).load();
    // Power-on, which `reset` settles: R6 = 0 and R7 = 1.
    assert_eq!([0, 1, 6, 7], core.prg_windows());

    namcot108_bank(&mut core, 6, 3);
    namcot108_bank(&mut core, 7, 5);
    assert_eq!([3, 5, 6, 7], core.prg_windows());

    // "bits 3-0" of the data register are the PRG bank, and a bank beyond the ROM wraps rather
    // than reading out of it.
    namcot108_bank(&mut core, 6, 0x0F);
    assert_eq!([7, 5, 6, 7], core.prg_windows());
}

#[cfg(feature = "board-namcot108")]
/// `CHR Window: 2Kx2 + 1Kx4` — R0 and R1 are 2 KiB banks at PPU $0000-$07FF and $0800-$0FFF,
/// R2-R5 are 1 KiB banks at $1000-$13FF through $1C00-$1FFF. "Only bits 5-1 exist for 2KB CHR
/// banks", so bit 0 of R0 and R1 is not part of the bank.
#[test]
fn req_core_100_namcot_108_banks_two_2_kib_and_four_1_kib_chr_windows() {
    let mut core = MapperRom::new(206).prg(8).chr(16).load();
    namcot108_bank(&mut core, 0, 4);
    namcot108_bank(&mut core, 1, 8);
    for register in 2..6u8 {
        namcot108_bank(&mut core, register, 10 + register);
    }

    assert_eq!([4, 5, 8, 9, 12, 13, 14, 15], core.chr_windows());

    // Bit 0 of a 2 KiB register does not exist: 5 selects the same pair as 4.
    namcot108_bank(&mut core, 0, 5);
    assert_eq!([4, 5], core.chr_windows()[..2]);
}

#[cfg(feature = "board-namcot108")]
/// `Mirroring: Fixed H/V, or 4` — hardwired. **No register of this board touches it**, which is
/// the claim that separates 206 from 154, and the one a port of the MMC3 gets wrong by leaving
/// $A000 alive.
#[test]
fn req_core_100_namcot_108_has_no_mirroring_register() {
    for (bits, expected) in [(0u8, Nm::Horizontal), (1, Nm::Vertical)] {
        let mut core = MapperRom::new(206).prg(8).mirroring(bits).load();
        assert_eq!(expected, core.mirroring());

        for address in [
            0x8000u16, 0x8001, 0x9FFE, 0x9FFF, 0xA000, 0xA001, 0xC000, 0xE000,
        ] {
            for value in [0x00u8, 0x01, 0x40, 0xFF] {
                core.mapper_write(address, value);
                assert_eq!(expected, core.mirroring(), "{address:#06X} = {value:#04X}");
            }
        }
    }
}

#[cfg(feature = "board-namcot108")]
/// `IRQ: None` and `PRG RAM: None`, and with them the register mask $E001: the three MMC3
/// register pairs above $9FFF do not exist on this chip, so writing the MMC3's IRQ latch,
/// reload, disable and enable addresses changes nothing and asserts nothing.
#[test]
fn req_core_100_namcot_108_has_no_irq_and_no_prg_ram() {
    let mut core = MapperRom::new(206).prg(8).chr(16).load();
    namcot108_bank(&mut core, 6, 3);
    let prg = core.prg_windows();
    let chr = core.chr_windows();

    for address in [0xA000u16, 0xA001, 0xC000, 0xC001, 0xE000, 0xE001] {
        core.mapper_write(address, 0x01);
    }
    for _ in 0..4 {
        core.run_frame(0);
    }

    assert!(!core.cartridge().unwrap().irq_asserted());
    assert_eq!(prg, core.prg_windows());
    assert_eq!(chr, core.chr_windows());

    // "PRG RAM: None": $6000-$7FFF is not RAM, so a write there is not read back.
    core.write_memory(0x6000, 0x5A);
    assert_ne!(0x5A, core.read_memory(0x6000));
}

#[cfg(feature = "board-namcot108")]
/// nesdev ("INES Mapper 076"), the NAMCOT-3446 of *Megami Tensei*: R2-R5 are **four 2 KiB CHR
/// windows** at $0000, $0800, $1000 and $1800, each a 2 KiB bank number six bits wide, so
/// 128 KiB of CHR is reachable. R0 and R1 are not wired to anything.
#[test]
fn req_core_100_namcot_3446_banks_chr_in_four_2_kib_windows() {
    let mut core = MapperRom::new(76).prg(8).chr(128).load();
    namcot108_bank(&mut core, 2, 0);
    namcot108_bank(&mut core, 3, 1);
    namcot108_bank(&mut core, 4, 2);
    // Bank 63 of 2 KiB is the last of 128 KiB: the page's "coarser CHR banking".
    namcot108_bank(&mut core, 5, 63);
    assert_eq!([0, 1, 2, 3, 4, 5, 126, 127], core.chr_windows());

    // The 108's own two 2 KiB registers have no effect on this board.
    namcot108_bank(&mut core, 0, 0x3F);
    namcot108_bank(&mut core, 1, 0x3F);
    assert_eq!([0, 1, 2, 3, 4, 5, 126, 127], core.chr_windows());
}

#[cfg(feature = "board-namcot108")]
/// nesdev ("INES Mapper 088"), the NAMCOT-3433/3443 of *Dragon Spirit* and *Quinty*: "CHR
/// support is increased to 128 KiB by connecting PPU's A12 line to the CHR ROM's A16 line", so
/// the left pattern table can only reach the first 64 KiB and the right one only the second —
/// the page's own implementation note, an AND with $3F on registers 0-1 and an OR with $40 on
/// registers 2-5. This is the one claim a careless port of the 108 gets wrong.
#[test]
fn req_core_100_namcot_3433_splits_128_kib_of_chr_by_pattern_table() {
    let mut core = MapperRom::new(88).prg(8).chr(128).load();
    // R0 asks for 1 KiB bank 80, which is in the second half: the AND brings it back to 16.
    namcot108_bank(&mut core, 0, 0x50);
    namcot108_bank(&mut core, 1, 0x7F);
    for register in 2..6u8 {
        namcot108_bank(&mut core, register, register - 2);
    }

    assert_eq!([16, 17, 62, 63, 64, 65, 66, 67], core.chr_windows());

    // The claim itself, over every bank value the registers can hold: $0000-$0FFF stays in the
    // first 64 KiB and $1000-$1FFF in the second.
    for value in 0..64u8 {
        for register in 0..6u8 {
            namcot108_bank(&mut core, register, value | 0x40);
        }
        let windows = core.chr_windows();
        assert!(windows[..4].iter().all(|bank| *bank < 64), "{windows:?}");
        assert!(windows[4..].iter().all(|bank| *bank >= 64), "{windows:?}");
    }
}

#[cfg(feature = "board-namcot108")]
/// nesdev ("INES Mapper 095"), the NAMCOT-3425 of *Dragon Buster*: the bank data register is
/// `..ND DDDD`, N being "Nametable select, based on last value written to Bank select register".
/// Register 0 drives the nametable at PPU $2000-$27FF and register 1 the one at $2800-$2FFF —
/// CHR A15 wired to CIRAM A10.
#[test]
fn req_core_100_namcot_3425_takes_its_nametable_from_chr_a15() {
    let mut core = MapperRom::new(95).prg(8).chr(32).load();
    assert_eq!([0, 0, 0, 0], nametable_pages(&core));

    for (r0, r1, expected) in [
        (0x00u8, 0x00u8, [0, 0, 0, 0]),
        (0x20, 0x00, [1, 1, 0, 0]),
        (0x00, 0x20, [0, 0, 1, 1]),
        (0x20, 0x20, [1, 1, 1, 1]),
    ] {
        namcot108_bank(&mut core, 0, r0);
        namcot108_bank(&mut core, 1, r1);
        assert_eq!(
            expected,
            nametable_pages(&core),
            "R0 {r0:#04X} R1 {r1:#04X}"
        );
        // Never served by the cartridge: these are CIRAM pages (Q-123's ceiling is untouched).
        assert!(!nametable_pages(&core).contains(&Cartridge::SERVED_BY_CARTRIDGE));
    }

    // And N is not part of the bank: with it set, R0 = $20 shows the same pair as R0 = $00.
    namcot108_bank(&mut core, 0, 0x20);
    assert_eq!([0, 1], core.chr_windows()[..2]);
    namcot108_bank(&mut core, 0, 0x0A);
    assert_eq!([10, 11], core.chr_windows()[..2]);
    assert_eq!([0, 0, 1, 1], nametable_pages(&core));

    // The other four numbers of the family answer the trait's default, which is the latch's
    // other half: 95 alone is in `Cartridge::nametable_pages_vary`.
    let plain = MapperRom::new(206).prg(8).chr(32).load();
    assert_eq!([Cartridge::FOLLOW_MIRRORING; 4], nametable_pages(&plain));
}

#[cfg(feature = "board-namcot108")]
/// nesdev ("INES Mapper 154"), the NAMCOT-3453 of *Devil Man*: `$8000-$FFFF: [.N.. ....]`, N
/// being one-screen select, 0 = 1ScA and 1 = 1ScB. The page's own statement of the difference:
/// "the nametable control bit is present over the entire 32kB range; unlike the associated
/// Namco 108 which is only present at $8000-$9FFF".
#[test]
fn req_core_100_namcot_3453_decodes_its_one_screen_bit_over_all_of_8000_ffff() {
    let mut core = MapperRom::new(154).prg(8).chr(128).load();

    // Every 8 KiB block of $8000-$FFFF, and both polarities, including the two blocks a 206
    // does not decode at all.
    for address in [
        0x8000u16, 0x8001, 0x9FFF, 0xA000, 0xBFFF, 0xC000, 0xDFFF, 0xE000, 0xFFFF,
    ] {
        core.mapper_write(address, 0x40);
        assert_eq!(Nm::SingleScreenB, core.mirroring(), "{address:#06X} N set");
        core.mapper_write(address, 0x00);
        assert_eq!(
            Nm::SingleScreenA,
            core.mirroring(),
            "{address:#06X} N clear"
        );
    }

    // And the board is an 88 otherwise: the same CHR split by pattern table.
    namcot108_bank(&mut core, 0, 0x50);
    namcot108_bank(&mut core, 2, 0x00);
    assert_eq!([16, 17], core.chr_windows()[..2]);
    assert_eq!(64, core.chr_windows()[4]);
}

// ------------------------------------------- Taito TC0190 (33), Irem TAM-S1 (97),
// ------------------------------------------- Jaleco JF-11 (140), Sunsoft-1 (184),
// ------------------------------------------- Sunsoft-2 (89, 93)

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 033"): PRG register 0 at `$8000` with the mirroring bit above it,
/// register 1 at `$8001`, and the last two 8 KiB banks fixed to the end.
#[test]
fn req_core_100_tc0190_switches_two_prg_windows_and_decodes_the_mirroring_bit() {
    let mut core = MapperRom::new(33).prg(16).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());

    core.mapper_write(0x8000, 0x05);
    core.mapper_write(0x8001, 0x07);
    assert_eq!([5, 7, 14, 15], core.prg_windows());
    // `M` clear is vertical; the page's `[.MPP PPPP]` puts it at bit 6.
    assert_eq!(Nm::Vertical, core.mirroring());

    core.mapper_write(0x8000, 0x45);
    assert_eq!(Nm::Horizontal, core.mirroring());
    assert_eq!([5, 7, 14, 15], core.prg_windows());
}

#[cfg(feature = "board-small-boards")]
/// The CHR half, and the claim that separates this board from an MMC3: the two 2 KiB registers
/// at `$8002`-`$8003` "use the full written value (not dropping the LSB like MMC3), allowing
/// 512 KiB addressability", where the four 1 KiB ones at `$A000`-`$A003` reach 256 KiB.
#[test]
fn req_core_100_tc0190_banks_2_kib_chr_from_the_whole_byte_unlike_an_mmc3() {
    let mut core = MapperRom::new(33).prg(16).chr(128).load();
    core.mapper_write(0x8002, 0x03);
    core.mapper_write(0x8003, 0x05);
    for register in 0..4u16 {
        core.mapper_write(0xA000 + register, 20 + register as u8);
    }

    // An MMC3 would read $8002 = 3 as 2 KiB bank 1 (banks 2 and 3 of 1 KiB); this board reads
    // it as 2 KiB bank 3, which is 1 KiB banks 6 and 7.
    assert_eq!([6, 7, 10, 11, 20, 21, 22, 23], core.chr_windows());
}

#[cfg(feature = "board-small-boards")]
/// Register mask `$A003`: the eight registers repeat over the whole of $8000-$FFFF, so a write
/// to `$C000` is a write to `$8000` and one to `$E003` is a write to `$A003`.
#[test]
fn req_core_100_tc0190_decodes_its_registers_through_the_a003_mask() {
    let mut core = MapperRom::new(33).prg(16).chr(128).load();
    core.mapper_write(0xC000, 0x45); // -> $8000.
    assert_eq!(Nm::Horizontal, core.mirroring());
    assert_eq!([5, 1, 14, 15], core.prg_windows());

    core.mapper_write(0xE003, 0x1F); // -> $A003.
    assert_eq!(0x1F, core.chr_windows()[7]);

    core.mapper_write(0x9002, 0x03); // -> $8002.
    assert_eq!([6, 7], core.chr_windows()[..2]);
}

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 097"): `$8000-BFFF: [M..P PPPP]`, the `P` field switching the 16 KiB
/// bank **at $C000** while $8000-$BFFF holds the last bank — the opposite way round from every
/// other 16 KiB board here.
#[test]
fn req_core_100_tam_s1_switches_c000_and_fixes_the_last_bank_at_8000() {
    let mut core = MapperRom::new(97).prg(16).load();
    assert_eq!([14, 15, 0, 1], core.prg_windows());

    core.mapper_write(0x8000, 0x03);
    assert_eq!([14, 15, 6, 7], core.prg_windows());
    // `M` is bit 7: "%0 = Horz (PPU A11)".
    assert_eq!(Nm::Horizontal, core.mirroring());

    core.mapper_write(0xBFFF, 0x85);
    assert_eq!([14, 15, 10, 11], core.prg_windows());
    assert_eq!(Nm::Vertical, core.mirroring());

    // The register is $8000-$BFFF and nothing above it.
    core.mapper_write(0xC000, 0x01);
    assert_eq!([14, 15, 10, 11], core.prg_windows());
    assert_eq!(Nm::Vertical, core.mirroring());
}

#[cfg(feature = "board-small-boards")]
/// "This mapper does not have bus conflicts", so a five-bit bank survives a write landing on a
/// zero-filled PRG bank — which on a board *with* conflicts the AND would take to 0.
#[test]
fn req_core_100_tam_s1_has_no_bus_conflicts() {
    let mut core = MapperRom::new(97).prg(16).load();
    // $8000 shows bank 14, whose bytes are all 14: an AND would leave $1F & 14 = 14 -> bank 14
    // of 16 KiB, which wraps to 8 KiB banks 12, 13. Without conflicts the register takes $1F.
    core.mapper_write(0x8000, 0x1F);
    assert_eq!([14, 15, 14, 15], core.prg_windows());
}

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 140"): `$6000-7FFF: [...PP CCCC]`, a GNROM whose register sits below
/// $8000 — "Regs lie at $6000-7FFF, so there's no SRAM".
#[test]
fn req_core_100_jf11_banks_32_kib_prg_and_8_kib_chr_from_6000() {
    let mut core = MapperRom::new(140).prg(16).chr(128).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    core.mapper_write(0x6000, 0x2A);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!([80, 81, 82, 83, 84, 85, 86, 87], core.chr_windows());

    // The whole window is the register.
    core.mapper_write(0x7FFF, 0x13);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(24, core.chr_windows()[0]);

    // And there is no RAM behind it: what was written is not read back.
    core.write_memory(0x6000, 0x5A);
    assert_ne!(0x5A, core.read_memory(0x6000));
}

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 184"), Sunsoft-1: two 4 KiB CHR banks from one register at $6000-$7FFF,
/// the upper one limited to "banks 4-7" because bit 6 is tied high in hardware.
#[test]
fn req_core_100_sunsoft_1_banks_two_4_kib_chr_windows_with_the_upper_one_above_four() {
    let mut core = MapperRom::new(184).prg(4).chr(32).load();
    core.mapper_write(0x6000, 0x52);
    // L = 2 -> 4 KiB bank 2, i.e. 1 KiB banks 8-11; H = %01 with bit 6 high -> bank 5.
    assert_eq!([8, 9, 10, 11, 20, 21, 22, 23], core.chr_windows());

    // Bit 6 is high whatever is written, so clearing it in software changes nothing: the upper
    // window can never show banks 0-3. The page: "existing software is well behaved and writes
    // to this with the $40s bit set".
    for value in 0..=0xFFu8 {
        core.mapper_write(0x6000, value);
        let windows = core.chr_windows();
        assert!(windows[4] >= 16, "{value:#04X} -> {windows:?}");
    }

    // No PRG banking at all, and no RAM at $6000-$7FFF.
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    core.write_memory(0x6000, 0x5A);
    assert_ne!(0x5A, core.read_memory(0x6000));
}

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 089"), the Sunsoft-2 on the Sunsoft-3 board: `$8000-FFFF: [CPPP MCCC]`
/// — a 16 KiB PRG bank from bits 6-4, an 8 KiB CHR bank from bits 2-0 with bit 7 above them,
/// one-screen mirroring from bit 3, and the last 16 KiB fixed.
///
/// The board has **bus conflicts**, so the test writes into a PRG bank filled with $FF: on the
/// numbered banks the AND would mask every bit above 3 away.
#[test]
fn req_core_100_sunsoft_2_banks_16_kib_prg_and_8_kib_chr_on_mapper_89() {
    let mut core = MapperRom::new(89)
        .prg(16)
        .chr(128)
        .prg_bank_all_ones(14)
        .load();
    // Bank 14 is $FF-filled for the bus conflict, so it reads back as 255 and not as 14.
    assert_eq!([0, 1, 255, 15], core.prg_windows());

    // $C000 is 8 KiB bank 14, the $FF-filled one, so the register latches what is written.
    core.mapper_write(0xC000, 0x35); // C=0 PPP=3 M=0 CCC=5 -> PRG 3, CHR 5, 1ScA.
    assert_eq!([6, 7, 255, 15], core.prg_windows());
    assert_eq!(40, core.chr_windows()[0]);
    assert_eq!(Nm::SingleScreenA, core.mirroring());

    // Bit 7 is the CHR bank's high bit: 8 | 5 = 13.
    core.mapper_write(0xC000, 0xB5);
    assert_eq!(104, core.chr_windows()[0]);

    // Bit 3 is the one-screen select, "%1 = 1ScB".
    core.mapper_write(0xC000, 0x3D);
    assert_eq!(Nm::SingleScreenB, core.mirroring());
}

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 093"), the Sunsoft-2 on the Sunsoft-3R board: `$8000-FFFF: [.PPP ...E]`
/// — the same PRG field, **no** CHR bank and **no** mirroring bit, and `E` a CHR-RAM enable
/// whose disabled state is "reads are open bus" and "writes are ignored".
#[test]
fn req_core_100_sunsoft_2_enables_chr_ram_with_bit_0_on_mapper_93() {
    let mut core = MapperRom::new(93)
        .prg(16)
        .chr(0)
        .prg_bank_all_ones(14)
        .load();
    assert_eq!([0, 1, 255, 15], core.prg_windows());

    // `E` set: the CHR-RAM answers, and a write to it lands.
    core.mapper_write(0xC000, 0x31);
    assert_eq!([6, 7, 255, 15], core.prg_windows());
    core.cartridge_mut().unwrap().ppu_write(0x0040, 0x5A);
    assert_eq!(0x5A, peek(&core, 0x0040));

    // `E` clear: reads are open bus, which this core answers as zero, and writes are ignored.
    core.mapper_write(0xC000, 0x30);
    assert_eq!(0, peek(&core, 0x0040));
    core.cartridge_mut().unwrap().ppu_write(0x0040, 0x3C);
    core.mapper_write(0xC000, 0x31);
    assert_eq!(0x5A, peek(&core, 0x0040));

    // And the board has no mirroring register: the header's layout stands whatever is written.
    let before = core.mirroring();
    for value in [0x00u8, 0x08, 0x40, 0xFF] {
        core.mapper_write(0xC000, value);
        assert_eq!(before, core.mirroring(), "{value:#04X}");
    }
}

#[cfg(feature = "board-small-boards")]
/// Both Sunsoft-2 boards carry the page's own "BUS CONFLICTS" line, which is one arm of
/// `registry::has_bus_conflicts` and is visible from outside: a write landing on a numbered
/// bank latches the AND of the value with the bank's bytes.
#[test]
fn req_core_100_sunsoft_2_has_bus_conflicts_on_both_boards() {
    for mapper in [89usize, 93] {
        let mut core = MapperRom::new(mapper).prg(16).chr(128).load();
        // $C000 shows 8 KiB bank 14, whose every byte is 14 = %0000_1110. A PRG field of %111
        // in bits 6-4 therefore cannot survive the write.
        core.mapper_write(0xC000, 0x70);
        assert_eq!(
            [0, 1, 14, 15],
            core.prg_windows(),
            "mapper {mapper}: the ROM did not drive the bus"
        );
    }
}

// ------------------------------------------- Bandai 74161/7432 (70, 152), VRC1 (75),
// ------------------------------------------- Camerica BF9096 (232)

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 070") gives the board `[PPPP CCCC]` and no mirroring control; nesdev
/// ("INES Mapper 152") gives the same board `[MPPP CCCC]`, `M` being one-screen select. One
/// write pins the pair: `$F0` is PRG bank 15 and no layout change on 70, and PRG bank 7 with
/// one-screen B on 152.
///
/// Both boards have bus conflicts, so the write goes to `$C000`, whose 8 KiB bank is filled
/// with $FF: through a numbered bank the AND would mask the field away.
#[test]
fn req_core_100_bandai_74161_reads_bit_7_as_a_bank_bit_on_70_and_a_nametable_on_152() {
    // 32 x 8 KiB is 16 x 16 KiB, so a four-bit PRG field is fully reachable.
    let mut on_70 = MapperRom::new(70)
        .prg(32)
        .chr(32)
        .prg_bank_all_ones(30)
        .load();
    let before = on_70.mirroring();
    on_70.mapper_write(0xC000, 0xF0);
    // Four PRG bits: %1111 is 16 KiB bank 15, i.e. 8 KiB banks 30 and 31; 30 reads back as 255.
    assert_eq!([255, 31, 255, 31], on_70.prg_windows());
    assert_eq!(
        before,
        on_70.mirroring(),
        "mapper 70 has no mirroring control"
    );

    let mut on_152 = MapperRom::new(152)
        .prg(32)
        .chr(32)
        .prg_bank_all_ones(30)
        .load();
    on_152.mapper_write(0xC000, 0xF0);
    // Three PRG bits: %111 is 16 KiB bank 7, i.e. 8 KiB banks 14 and 15.
    assert_eq!([14, 15, 255, 31], on_152.prg_windows());
    assert_eq!(Nm::SingleScreenB, on_152.mirroring());

    on_152.mapper_write(0xC000, 0x70);
    assert_eq!(Nm::SingleScreenA, on_152.mirroring());
    assert_eq!([14, 15, 255, 31], on_152.prg_windows());
}

#[cfg(feature = "board-small-boards")]
/// The half the two numbers share: a 16 KiB PRG window at $8000 with the last bank fixed at
/// $C000, and an 8 KiB CHR bank from bits 3-0 — and the bus conflict, visible because the write
/// below lands on 8 KiB bank 14, whose every byte is 14.
#[test]
fn req_core_100_bandai_74161_shares_its_prg_and_chr_split_between_70_and_152() {
    for mapper in [70usize, 152] {
        let mut core = MapperRom::new(mapper).prg(16).chr(128).load();
        assert_eq!([0, 1, 14, 15], core.prg_windows(), "mapper {mapper}");

        // $FF & 14 = 14: PRG field 0, CHR field 14.
        core.mapper_write(0xC000, 0xFF);
        assert_eq!([0, 1, 14, 15], core.prg_windows(), "mapper {mapper}");
        assert_eq!(112, core.chr_windows()[0], "mapper {mapper}");
    }
}

#[cfg(feature = "board-small-boards")]
/// nesdev ("VRC1"): three switchable 8 KiB PRG windows at $8000, $A000 and $C000, with
/// $E000-$FFFF wired to the last bank.
#[test]
fn req_core_100_vrc1_switches_three_prg_windows_and_wires_the_last() {
    let mut core = MapperRom::new(75).prg(16).chr(32).load();
    assert_eq!([0, 1, 2, 15], core.prg_windows());

    core.mapper_write(0x8000, 0x05);
    core.mapper_write(0xA000, 0x07);
    core.mapper_write(0xC000, 0x09);
    assert_eq!([5, 7, 9, 15], core.prg_windows());

    // Four bits: the register ignores anything above them.
    core.mapper_write(0x8000, 0xF3);
    assert_eq!([3, 7, 9, 15], core.prg_windows());
}

#[cfg(feature = "board-small-boards")]
/// The CHR half is two 4 KiB banks of **five** bits each: `$E000` and `$F000` carry the low four
/// and `$9000` bits 1 and 2 the high one.
#[test]
fn req_core_100_vrc1_takes_its_chr_high_bit_from_9000() {
    let mut core = MapperRom::new(75).prg(16).chr(128).load();
    core.mapper_write(0xE000, 0x03);
    core.mapper_write(0xF000, 0x05);
    // 4 KiB banks 3 and 5, i.e. 1 KiB banks 12-15 and 20-23.
    assert_eq!([12, 13, 14, 15, 20, 21, 22, 23], core.chr_windows());

    // $9000 bit 1 is the high bit of the first window, bit 2 of the second: bank 3 + 16 = 19,
    // i.e. 1 KiB banks 76-79, and bank 5 + 16 = 21, i.e. 84-87.
    core.mapper_write(0x9000, 0x02);
    assert_eq!([76, 77, 78, 79, 20, 21, 22, 23], core.chr_windows());
    core.mapper_write(0x9000, 0x06);
    assert_eq!([76, 77, 78, 79, 84, 85, 86, 87], core.chr_windows());
    core.mapper_write(0x9000, 0x00);
    assert_eq!([12, 13, 14, 15, 20, 21, 22, 23], core.chr_windows());
}

#[cfg(feature = "board-small-boards")]
/// `$9000` bit 0 is the mirroring bit, "0 = Vertical, 1 = Horizontal"; and the page's condition:
/// "the mirroring bit is ignored if the cartridge is wired for 4-screen VRAM".
#[test]
fn req_core_100_vrc1_has_a_mirroring_bit_that_a_four_screen_header_ignores() {
    let mut core = MapperRom::new(75).prg(16).chr(32).load();
    core.mapper_write(0x9000, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.mapper_write(0x9000, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());

    // Byte 6 bit 3 is the four-screen bit.
    let mut four = MapperRom::new(75).prg(16).chr(128).mirroring(0x08).load();
    assert_eq!(Nm::FourScreen, four.mirroring());
    for value in [0x00u8, 0x01, 0x07] {
        four.mapper_write(0x9000, value);
        assert_eq!(Nm::FourScreen, four.mirroring(), "{value:#04X}");
    }

    // And the CHR high bits still work on a four-screen board.
    four.mapper_write(0xE000, 0x01);
    four.mapper_write(0x9000, 0x02);
    assert_eq!(68, four.chr_windows()[0]);
}

#[cfg(feature = "board-small-boards")]
/// There is no IRQ on the VRC1: nesdev gives the chip none, and the three addresses a VRC2 or
/// VRC4 would use for one do nothing here.
#[test]
fn req_core_100_vrc1_has_no_irq() {
    let mut core = MapperRom::new(75).prg(16).chr(32).load();
    for address in [0x9000u16, 0xB000, 0xD000, 0xE000, 0xF000] {
        core.mapper_write(address, 0xFF);
    }
    for _ in 0..4 {
        core.run_frame(0);
    }

    assert!(!core.cartridge().unwrap().irq_asserted());
}

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 232"): `$8000-$BFFF: [...B B...]` selects a 64 KiB block and
/// `$C000-$FFFF: [.... ..PP]` a 16 KiB page inside it — "pages are taken from the 64k block
/// currently selected by $8000" — with the block's last page fixed at $C000.
#[test]
fn req_core_100_bf9096_takes_its_pages_from_the_selected_64_kib_block() {
    let mut core = MapperRom::new(232).prg(32).chr(16).load();
    // Power-on: block 0, page 0, so $8000 is page 0 and $C000 page 3 of block 0.
    assert_eq!([0, 1, 6, 7], core.prg_windows());

    core.mapper_write(0xC000, 0x02);
    assert_eq!([4, 5, 6, 7], core.prg_windows());

    // Block 1 is the second 64 KiB: its four pages are 4 to 7 of 16 KiB.
    core.mapper_write(0x8000, 0x08);
    assert_eq!([12, 13, 14, 15], core.prg_windows());

    core.mapper_write(0xC000, 0x00);
    assert_eq!([8, 9, 14, 15], core.prg_windows());

    // Bits outside the two fields are not part of either register: $EF keeps block 1 and $FC
    // keeps page 0.
    core.mapper_write(0x8000, 0xEF);
    assert_eq!([8, 9, 14, 15], core.prg_windows());
    core.mapper_write(0xC000, 0xFC);
    assert_eq!([8, 9, 14, 15], core.prg_windows());
}

#[cfg(feature = "board-small-boards")]
/// NES 2.0 submapper 1 is the Aladdin Deck Enhancer, whose two block bits are the other way
/// round; submapper 0 is the plain *Quattro* board. `$08` is bit 3 alone, so it is block %01
/// plain and block %10 swapped.
#[test]
fn req_core_100_bf9096_swaps_its_block_bits_on_submapper_1() {
    let mut plain = MapperRom::new(232).prg(32).chr(16).load();
    plain.mapper_write(0x8000, 0x08);
    assert_eq!([8, 9, 14, 15], plain.prg_windows());

    let mut aladdin = MapperRom::new(232).prg(32).chr(16).submapper(1).load();
    aladdin.mapper_write(0x8000, 0x08);
    assert_eq!([16, 17, 22, 23], aladdin.prg_windows());
}

// ------------------------------------------- the parameters of existing boards:
// ------------------------------------------- 94 on UxROM, 185 on CNROM, 155 on MMC1

#[cfg(feature = "board-94")]
/// nesdev ("INES Mapper 094"): "very similar to `UxROM`, but the register is shifted by two bits",
/// `xxxP PPxx`, with the same switchable window at $8000-$BFFF and the last bank fixed at $C000.
///
/// **Both numbers in one test**, so the parameter cannot be got backwards: `$14` is bank 5 on
/// mapper 94 (bits 4-2) and bank 20 — which wraps to 4 — on mapper 2.
#[test]
fn req_core_100_un1rom_shifts_the_uxrom_field_by_two_bits() {
    let mut on_94 = MapperRom::new(94).prg(16).load();
    assert_eq!([0, 1, 14, 15], on_94.prg_windows());
    on_94.mapper_write(0x8000, 0x14);
    assert_eq!([10, 11, 14, 15], on_94.prg_windows());

    let mut on_2 = MapperRom::new(2).prg(16).load();
    on_2.mapper_write(0x8000, 0x14);
    assert_eq!([8, 9, 14, 15], on_2.prg_windows());

    // Three bits on mapper 94: bits 7-5 and 1-0 are not part of the bank.
    on_94.mapper_write(0x8000, 0xE3);
    assert_eq!([0, 1, 14, 15], on_94.prg_windows());
    on_94.mapper_write(0x8000, 0x1F);
    assert_eq!([14, 15, 14, 15], on_94.prg_windows());
}

#[cfg(feature = "board-small-boards")]
/// nesdev ("INES Mapper 185"): submappers 4 to 7 say which value of the two chip-select bits
/// enables the CHR-ROM; "when another value is active, CHR-ROM is disabled, meaning that reading
/// from the pattern tables returns open bus behavior".
#[test]
fn req_core_100_cnrom_protect_enables_chr_rom_on_its_submappers_chip_select() {
    for (submapper, enabling) in [(4u8, 0u8), (5, 1), (6, 2), (7, 3)] {
        let mut core = MapperRom::new(185)
            .prg(4)
            .chr(8)
            .submapper(submapper)
            .prg_bank_all_ones(3)
            .load();
        for value in 0..4u8 {
            // $E100 is inside 8 KiB bank 3, filled with $FF, so the bus conflict leaves the
            // value written. $E000 itself would not: the fixture puts a `JMP` there for the
            // reset vector to point at, so the AND would be with $4C.
            core.mapper_write(0xE100, value);
            let read = peek(&core, 0x0400);
            if value == enabling {
                // CHR 1 KiB bank 1 is filled with 1.
                assert_eq!(1, read, "submapper {submapper} value {value}: enabled");
            } else {
                // Disabled: the chip is deselected and D0 floats high.
                assert_eq!(1 | 0x01, read, "submapper {submapper} value {value}");
                assert_eq!(0x03, peek(&core, 0x0800), "submapper {submapper}: D0 high");
            }
        }
    }
}

#[cfg(feature = "board-small-boards")]
/// Submapper 0, which is every iNES 1.0 image: the page's heuristic needs to count $2007 reads,
/// which this core cannot tell from a background fetch, so **Q-131** records the question and
/// the latch-value heuristic is what ships — "if C AND $0F is nonzero, and if C does not equal
/// $13: CHR is enabled".
#[test]
fn q_131_cnrom_protect_reads_its_submapper_0_heuristic_off_the_latch() {
    let mut core = MapperRom::new(185)
        .prg(4)
        .chr(8)
        .prg_bank_all_ones(3)
        .load();
    // Power-on latch 0: the low nibble is zero, so the CHR is disabled.
    assert_eq!(0x03, peek(&core, 0x0800));

    core.mapper_write(0xE100, 0x01);
    assert_eq!(2, peek(&core, 0x0800));

    // $13 is the one nonzero value the heuristic excludes.
    core.mapper_write(0xE100, 0x13);
    assert_eq!(0x03, peek(&core, 0x0800));

    core.mapper_write(0xE100, 0x0F);
    assert_eq!(2, peek(&core, 0x0800));
}

/// Mapper 3 is **not** touched by any of this: its register is still an 8 KiB CHR bank select,
/// and nothing about it is a chip select.
#[test]
fn req_core_100_plain_cnrom_is_unaffected_by_mapper_185() {
    let mut core = MapperRom::new(3).prg(4).chr(32).load();
    core.mapper_write(0x8000, 0x03);
    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], core.chr_windows());
    assert_eq!(3, core.cartridge().unwrap().mapper_number());
}

#[cfg(feature = "board-155")]
/// nesdev ("INES Mapper 155"): the MMC1 on a board that does not wire the PRG-RAM disable, so
/// bit 4 of the PRG register has no effect on it — **and an MMC1 in the same test still honours
/// it**, which is what says the parameter did not reach mapper 1.
#[test]
fn req_core_100_mapper_155_ignores_the_mmc1_prg_ram_disable() {
    for (mapper, still_readable) in [(155usize, true), (1, false)] {
        let mut core = MapperRom::new(mapper).prg(16).chr(16).load();
        assert_eq!(mapper, core.cartridge().unwrap().mapper_number() as usize);

        core.write_memory(0x6000, 0x5A);
        assert_eq!(0x5A, core.read_memory(0x6000), "mapper {mapper}: power-on");

        // $E000 bit 4 is `R`, "PRG RAM chip enable (0: enabled; 1: disabled)".
        core.mmc1_write(0xE000, 0x10);
        assert_eq!(
            still_readable,
            core.read_memory(0x6000) == 0x5A,
            "mapper {mapper}: after the disable bit"
        );

        // And clearing it again gives an MMC1 its RAM back, with what was written still there.
        core.mmc1_write(0xE000, 0x00);
        assert_eq!(
            0x5A,
            core.read_memory(0x6000),
            "mapper {mapper}: re-enabled"
        );
    }
}

// ------------------------------------------- Konami VRC3 (73)

/// nesdev ("VRC3"): "CPU $8000-$BFFF: 16 KiB switchable PRG ROM bank" and "CPU $C000-$FFFF:
/// 16 KiB PRG ROM bank, fixed to the last bank", the register being `$F000-$FFFF` with "three
/// bits" of bank. The mirroring is hard-wired and `$E000-$EFFF` is not decoded at all, which is
/// what makes `$A000` an IRQ latch on this board where every other Konami chip has an
/// arrangement register there.
#[test]
fn req_core_100_the_vrc3_banks_16_kib_at_8000_and_fixes_the_last_at_c000() {
    let mut core = MapperRom::new(73).prg(8).chr(32).load();
    assert_eq!([0, 1, 6, 7], core.prg_windows());

    core.write_memory(0xF000, 0x02);
    assert_eq!([4, 5, 6, 7], core.prg_windows());

    // Three bits, so bit 3 of the written value is not latched: $0B selects bank 3.
    core.write_memory(0xF000, 0x0B);
    assert_eq!([6, 7, 6, 7], core.prg_windows());

    // $E000-$EFFF is not a register on this board.
    core.write_memory(0xE000, 0x00);
    assert_eq!([6, 7, 6, 7], core.prg_windows());

    // And no register of the board moves CHR: the whole 8 KiB is fixed.
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
    for address in [0x8000u16, 0x9000, 0xA000, 0xB000, 0xC000, 0xD000, 0xF000] {
        core.write_memory(address, 0x0F);
        assert_eq!(
            [0, 1, 2, 3, 4, 5, 6, 7],
            core.chr_windows(),
            "a write to {address:04X} moved CHR"
        );
    }
}

/// nesdev ("VRC3"): "PPU $0000-$1FFF: 8 KiB CHR RAM". REQ-CORE-012 gives an iNES 1.0 image with
/// no CHR-ROM **32 KiB** of CHR-RAM, which this board has no bank line to reach, so the size is
/// the board's and `registry::board_memory_sizes` settles it at load.
#[test]
fn req_core_100_the_vrc3_carries_eight_kib_of_chr_ram_and_not_the_default_thirty_two() {
    let core = MapperRom::new(73).prg(8).chr(0).load();
    assert_eq!(
        0x2000,
        core.cartridge().unwrap().memory().chr_ram().len(),
        "the board's 8 KiB, not REQ-CORE-012's default"
    );
}

// ------------------------------------------- Irem H3001 (65)

/// nesdev ("INES Mapper 065"): "On powerup, it appears as though PRG regs are inited to specific
/// values: $8000 = $00, $A000 = $01. Games do rely on this and will crash otherwise." With the
/// layout bit clear that is PRG reg 0 at `$8000`, PRG reg 1 at `$A000`, `$3E` at `$C000` and
/// `$3F` at `$E000` — the ROM's last two banks, which is what the board's own bank mask makes of
/// those two numbers at any size.
///
/// The mask is `$F007`, so A3-A11 are not decoded and the one register answers over its whole
/// page; and **`$C000` is not a register at all** — the page opens on Disch's older
/// documentation "incorrectly believing in a changeable bank at $C000", which Mesen 2 still has.
#[test]
fn req_core_100_the_h3001_powers_on_with_prg_reg_0_at_zero_and_reg_1_at_one() {
    let mut core = MapperRom::new(65).prg(8).chr(32).load();
    assert_eq!([0, 1, 6, 7], core.prg_windows());

    core.write_memory(0x8000, 0x03);
    core.write_memory(0xA000, 0x04);
    assert_eq!([3, 4, 6, 7], core.prg_windows());

    // The $F007 mask: every address of the page is the same register.
    core.write_memory(0x8007, 0x02);
    core.write_memory(0xAFFF, 0x05);
    assert_eq!([2, 5, 6, 7], core.prg_windows());
    core.write_memory(0x8123, 0x01);
    assert_eq!([1, 5, 6, 7], core.prg_windows());

    // And the three pages above $9000 that decode to nothing move nothing.
    for address in [0xC000u16, 0xD000, 0xE000, 0xF000] {
        core.write_memory(address, 0x07);
        assert_eq!([1, 5, 6, 7], core.prg_windows(), "{address:04X} banked PRG");
    }
}

/// `$9000` is `[X... ....]`: "%0 = bank at $8000 set by writes to $8000, bank at $C000 always
/// $3E", "%1 = bank at $C000 set by writes to $8000, bank at $8000 always $3E". The `$A000`
/// window and the `$3F` at `$E000` do not move either way.
#[test]
fn req_core_100_the_h3001_layout_bit_moves_prg_reg_0_to_c000() {
    let mut core = MapperRom::new(65).prg(8).chr(32).load();
    core.write_memory(0x8000, 0x03);
    assert_eq!([3, 1, 6, 7], core.prg_windows());

    core.write_memory(0x9000, 0x80);
    assert_eq!([6, 1, 3, 7], core.prg_windows());

    // The same register, now feeding the other window.
    core.write_memory(0x8000, 0x02);
    assert_eq!([6, 1, 2, 7], core.prg_windows());

    // Only bit 7 is the layout, and clearing it puts the bank back at $8000.
    core.write_memory(0x9000, 0x7F);
    assert_eq!([2, 1, 6, 7], core.prg_windows());
}

/// The CHR table of the page: `$B000` to `$B007`, eight 1 KiB banks "mapped sequentially from
/// $0000-$1C00". The eight registers are told apart by A0-A2 alone, so this is also the one place
/// the `$F007` mask splits a page rather than collapsing it.
#[test]
fn req_core_100_the_h3001_banks_eight_1_kib_chr_regs_at_b000() {
    let mut core = MapperRom::new(65).prg(8).chr(32).load();
    // Every window shows CHR bank 0 at power-on, the registers being zero.
    assert_eq!([0; 8], core.chr_windows());

    for slot in 0..8u16 {
        core.write_memory(0xB000 + slot, (31 - slot) as u8);
    }

    assert_eq!([31, 30, 29, 28, 27, 26, 25, 24], core.chr_windows());

    // A3-A11 undecoded here too: $B80C is $B004.
    core.write_memory(0xB80C, 0x07);
    assert_eq!([31, 30, 29, 28, 7, 26, 25, 24], core.chr_windows());
}

/// `$9001` is `[MM.. ....]` with **three** outcomes — "%00 = Vert", "%10 = Horz", "%01,%11 =
/// 1scA" — so bit 6 decides the one-screen case first and bit 7 only chooses between the two
/// mirrorings below it. Mesen 2 tests bit 7 alone (`IremH3001.h:57`) and never reaches the
/// one-screen state; AMD-061 §1 puts the page first.
#[test]
fn req_core_100_the_h3001_mirroring_is_decided_by_bit_6_before_bit_7() {
    for (value, expected) in [
        (0x00u8, zax_nes::NametableMirroring::Vertical),
        (0x80, zax_nes::NametableMirroring::Horizontal),
        (0x40, zax_nes::NametableMirroring::SingleScreenA),
        (0xC0, zax_nes::NametableMirroring::SingleScreenA),
        // Bits 0-5 are not part of `MM`.
        (0x3F, zax_nes::NametableMirroring::Vertical),
    ] {
        let mut core = MapperRom::new(65).prg(8).chr(32).load();
        core.write_memory(0x9001, value);
        assert_eq!(expected, core.mirroring(), "$9001 = {value:02X}");
    }
}

// ------------------------------------------- Sunsoft-3 (67)

/// nesdev ("INES Mapper 067"): "CPU $8000-$BFFF: 16 KiB switchable PRG ROM bank", "CPU
/// $C000-$FFFF: 16 KiB PRG ROM bank, fixed to the last bank", and the four 2 KiB CHR banks at
/// `$8800`, `$9800`, `$A800` and `$B800`.
#[test]
fn req_core_100_the_sunsoft3_banks_16_kib_at_8000_and_four_2_kib_chr_windows() {
    let mut core = MapperRom::new(67).prg(8).chr(32).load();
    assert_eq!([0, 1, 6, 7], core.prg_windows());
    assert_eq!([0, 1, 0, 1, 0, 1, 0, 1], core.chr_windows());

    core.write_memory(0xF800, 0x02);
    assert_eq!([4, 5, 6, 7], core.prg_windows());

    for (address, bank) in [(0x8800u16, 0u8), (0x9800, 1), (0xA800, 2), (0xB800, 3)] {
        core.write_memory(address, bank);
    }

    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
}

/// The two fields of the page that are narrower than the byte written into them: `$F800` is
/// `...X PPPP`, whose `X` is "a 1 bit latch, present but unused", and the CHR registers reach
/// only six bits because "the hardware only has six pins for CHR banking, for a limit of 128KB
/// of CHR". Both are checked on a ROM whose bank count does **not** divide the dropped bit, so
/// that masking and not masking give different banks — Mesen 2 passes the whole byte through and
/// relies on its own wrap.
#[test]
fn req_core_100_the_sunsoft3_prg_latch_bit_and_the_two_spare_chr_bits_are_dropped() {
    let mut core = MapperRom::new(67).prg(12).chr(48).load();
    assert_eq!([0, 1, 10, 11], core.prg_windows());

    // $12: bank 2 with the unused latch set. Taken whole it would be bank 18, which on six
    // 16 KiB banks is bank 0.
    core.write_memory(0xF800, 0x12);
    assert_eq!([4, 5, 10, 11], core.prg_windows());

    // $45: 2 KiB CHR bank 5, which is 1 KiB banks 10 and 11. Taken whole it would be bank 69,
    // which on twenty-four 2 KiB banks is bank 21, i.e. 1 KiB banks 42 and 43.
    core.write_memory(0x8800, 0x45);
    assert_eq!([10, 11], core.chr_windows()[..2]);
}

/// `$E800` is `.... ..MM`: "Nametable mirroring (0=vertical, 1=horizontal, 2=1scA, 3=1scB)", the
/// four states "aka connect VRAM A10 to (0=PPU A10, 1=PPU A11, 2=Gnd, 3=Vcc)".
#[test]
fn req_core_100_the_sunsoft3_mirroring_register_has_four_states() {
    for (value, expected) in [
        (0x00u8, zax_nes::NametableMirroring::Vertical),
        (0x01, zax_nes::NametableMirroring::Horizontal),
        (0x02, zax_nes::NametableMirroring::SingleScreenA),
        (0x03, zax_nes::NametableMirroring::SingleScreenB),
        // Bits 2-7 are not part of `MM`.
        (0xFE, zax_nes::NametableMirroring::SingleScreenA),
    ] {
        let mut core = MapperRom::new(67).prg(8).chr(32).load();
        core.write_memory(0xE800, value);
        assert_eq!(expected, core.mirroring(), "$E800 = {value:02X}");
    }
}

// ------------------------------------------- Tengen 800037 (158)

/// Loads RAMBO-1 bank register `register` while keeping bit 7 of `$8000` at `swap`, that bit
/// being both the CHR half-swap and — on mapper 158 — what chooses which registers drive the
/// nametables. [`MapperMachine::mmc3_bank`] writes the index alone and would clear it.
#[cfg(feature = "board-158")]
fn rambo1_bank(core: &mut zax_nes::NesCore, swap: bool, register: u8, value: u8) {
    core.mapper_write(0x8000, if swap { 0x80 | register } else { register });
    core.mapper_write(0x8001, value);
}

#[cfg(feature = "board-158")]
/// nesdev ("INES Mapper 158"): the board "modifies RAMBO-1 (mapper 64) in the exact same way that
/// TLSROM (118) differs from MMC3 (4), by which we mean the RAMBO-1's CHR A17 output is connected
/// to CIRAM A10". With bit 7 of `$8000` clear the two 2 KiB registers R0 and R1 are at PPU
/// `$0000-$0FFF` and take **two** nametables each, which is
/// `req_core_100_txsrom_takes_two_nametables_from_each_2_kib_register`'s table on this chip.
#[test]
fn req_core_100_tengen_800037_takes_two_nametables_from_each_2_kib_register() {
    let mut core = MapperRom::new(158).prg(16).chr(64).load();
    assert_eq!(158, core.cartridge().unwrap().mapper_number());

    for (r0, r1, expected) in [
        (0x00u8, 0x00u8, [0, 0, 0, 0]),
        (0x80, 0x00, [1, 1, 0, 0]),
        (0x00, 0x80, [0, 0, 1, 1]),
        (0x80, 0x80, [1, 1, 1, 1]),
    ] {
        rambo1_bank(&mut core, false, 0, r0);
        rambo1_bank(&mut core, false, 1, r1);
        assert_eq!(
            expected,
            nametable_pages(&core),
            "R0 {r0:#04X} R1 {r1:#04X}"
        );
    }
}

#[cfg(feature = "board-158")]
/// And with bit 7 of `$8000` **set** the four 1 KiB registers R2-R5 are at the first pattern
/// table and take one nametable each — the same bit `update_banks` swaps the CHR halves with, so
/// the registers that drive CIRAM A10 are always the ones the PPU fetches `$0000-$0FFF` from.
#[test]
fn req_core_100_tengen_800037_takes_one_nametable_from_each_1_kib_register() {
    let mut core = MapperRom::new(158).prg(16).chr(64).load();
    for register in 2..6u8 {
        rambo1_bank(&mut core, true, register, 0x00);
    }

    assert_eq!([0, 0, 0, 0], nametable_pages(&core));

    for (register, expected) in [
        (2u8, [1, 0, 0, 0]),
        (3, [1, 1, 0, 0]),
        (4, [1, 1, 1, 0]),
        (5, [1, 1, 1, 1]),
    ] {
        rambo1_bank(&mut core, true, register, 0x80);
        assert_eq!(expected, nametable_pages(&core), "R{register}");
    }

    // Registers 8 and 9 fill the odd kilobytes of 1 KiB mode and reach no nametable: there are
    // four pages and the four 1 KiB registers already name them.
    rambo1_bank(&mut core, true, 8, 0x00);
    rambo1_bank(&mut core, true, 9, 0x00);
    assert_eq!([1, 1, 1, 1], nametable_pages(&core));
}

#[cfg(feature = "board-158")]
/// `$A000` "is bypassed by the configuration described above, so writing here has no effect" —
/// nesdev's words for the `TxSROM`, which this board copies. **And a RAMBO-1 in the same test
/// still honours it**, which is what says the `const` parameter did not reach mapper 64.
#[test]
fn req_core_100_tengen_800037_ignores_the_mirroring_register() {
    let mut board = MapperRom::new(158).prg(16).chr(64).load();
    rambo1_bank(&mut board, false, 0, 0x80);
    rambo1_bank(&mut board, false, 1, 0x00);
    let before = board.mirroring();
    for value in [0x00u8, 0x01] {
        board.mapper_write(0xA000, value);
        assert_eq!(before, board.mirroring(), "$A000 = {value:#04X}");
        assert_eq!([1, 1, 0, 0], nametable_pages(&board));
    }

    let mut rambo1 = MapperRom::new(64).prg(16).chr(64).load();
    assert_eq!(64, rambo1.cartridge().unwrap().mapper_number());
    rambo1.mapper_write(0xA000, 0x01);
    assert_eq!(Nm::Horizontal, rambo1.mirroring());
    rambo1.mapper_write(0xA000, 0x00);
    assert_eq!(Nm::Vertical, rambo1.mirroring());
    // And its pages stay the console's, bit 7 of its CHR registers or not.
    rambo1_bank(&mut rambo1, false, 0, 0x80);
    assert_eq!([-1, -1, -1, -1], nametable_pages(&rambo1));
}

#[cfg(feature = "board-158")]
/// The banking is the RAMBO-1's, unchanged: the one thing mapper 158 rewires is CIRAM A10, so a
/// mapper-64 and a mapper-158 image given the same registers must show the same windows.
#[test]
fn mf_6b_tengen_800037_banks_exactly_as_a_rambo1_does() {
    let mut rambo1 = MapperRom::new(64).prg(16).chr(64).load();
    let mut board = MapperRom::new(158).prg(16).chr(64).load();
    for core in [&mut rambo1, &mut board] {
        // The third PRG register, the 1 KiB CHR mode and registers 8 and 9 with it.
        core.mapper_write(0x8000, 0x20);
        for (register, value) in [
            (6u8, 3u8),
            (7, 5),
            (15, 9),
            (0, 10),
            (1, 12),
            (8, 11),
            (9, 13),
        ] {
            core.mapper_write(0x8000, 0x20 | register);
            core.mapper_write(0x8001, value);
        }

        for register in 2..6u8 {
            core.mapper_write(0x8000, 0x20 | register);
            core.mapper_write(0x8001, 20 + register);
        }
    }

    assert_eq!(rambo1.prg_windows(), board.prg_windows());
    assert_eq!(rambo1.chr_windows(), board.chr_windows());
    assert_eq!([10, 11, 12, 13, 22, 23, 24, 25], board.chr_windows());
}

// ------------------------------------------- Taito TC0690 (48)

/// nesdev ("INES Mapper 048"): the TC0690 is "a superset of TC0190+PAL16R4 abilities", so the
/// banking is mapper 33's — two switchable 8 KiB PRG banks with the last two fixed, two 2 KiB
/// CHR registers using the whole written value and four 1 KiB ones — with the register mask
/// widened from `$A003` to **`$E003`**. That one difference is what frees `$C000`-`$C003` for the
/// IRQ and `$E000` for a mirroring register: on a TC0190 a write to `$C000` masks down to
/// `$8000` and moves PRG, and here it does not.
#[test]
fn req_core_100_the_tc0690_banks_like_a_tc0190_through_the_wider_e003_mask() {
    let mut core = MapperRom::new(48).prg(16).chr(32).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());

    core.write_memory(0x8000, 0x03);
    core.write_memory(0x8001, 0x05);
    assert_eq!([3, 5, 14, 15], core.prg_windows());

    // Six bits of bank, as on the TC0190: $43 is bank 3.
    core.write_memory(0x8000, 0x43);
    assert_eq!([3, 5, 14, 15], core.prg_windows());

    // Two 2 KiB registers that take the whole byte, then four 1 KiB ones.
    core.write_memory(0x8002, 0x01);
    core.write_memory(0x8003, 0x03);
    for register in 0..4u16 {
        core.write_memory(0xA000 + register, 20 + register as u8);
    }

    assert_eq!([2, 3, 6, 7, 20, 21, 22, 23], core.chr_windows());

    // A3-A12 are not decoded, so $8004 is $8000...
    core.write_memory(0x8004, 0x07);
    assert_eq!([7, 5, 14, 15], core.prg_windows());

    // ...and the IRQ registers bank nothing, which is the whole of the mask's difference.
    core.write_memory(0xC000, 0x02);
    core.write_memory(0xC002, 0x02);
    assert_eq!([7, 5, 14, 15], core.prg_windows());
}

/// `$E000` is `[.M.. ....]`, which is **bit 6** — the page's own register diagram and Mesen 2
/// (`value & 0x40`) agree, and the page's prose says bit 4, so the whole byte is walked here
/// rather than the one bit, and bit 4 is among the values that must change nothing.
///
/// The board controls the layout, so the header's own mirroring bit never decides it: the
/// power-on state is the register's zero, vertical, where the fixture's header says horizontal.
#[test]
fn req_core_100_the_tc0690_mirroring_is_bit_6_of_e000() {
    for (value, expected) in [
        (0x00u8, zax_nes::NametableMirroring::Vertical),
        (0x40, zax_nes::NametableMirroring::Horizontal),
        (0x10, zax_nes::NametableMirroring::Vertical),
        (0xBF, zax_nes::NametableMirroring::Vertical),
        (0xFF, zax_nes::NametableMirroring::Horizontal),
    ] {
        let mut core = MapperRom::new(48).prg(16).chr(32).load();
        assert_eq!(
            zax_nes::NametableMirroring::Vertical,
            core.mirroring(),
            "power-on"
        );
        core.write_memory(0xE000, value);
        assert_eq!(expected, core.mirroring(), "$E000 = {value:02X}");
    }

    // $E001-$E003 are not registers on this board.
    let mut core = MapperRom::new(48).prg(16).chr(32).load();
    for address in [0xE001u16, 0xE002, 0xE003] {
        core.write_memory(address, 0x40);
        assert_eq!(
            zax_nes::NametableMirroring::Vertical,
            core.mirroring(),
            "{address:04X} set the mirroring"
        );
    }
}

// ------------------------------------------- NES-EVENT (105)

#[cfg(feature = "board-105")]
/// A mapper-105 image whose timer has been unlocked by the documented sequence, leaving `$A000`
/// at `$10`: `I` set, `O` clear, `AA` zero.
fn unlocked_nes_event() -> zax_nes::NesCore {
    let mut core = MapperRom::new(105).prg(32).chr(0).load();
    core.mmc1_write(0xA000, 0x00);
    core.mmc1_write(0xA000, 0x10);
    core
}

#[cfg(feature = "board-105")]
/// nesdev ("NES-EVENT"): "the first 32 KiB is hardwired until the timer is started (write 0 then
/// 1 to bit 4) for the first time", and Disch "PRG cannot be swapped until the mapper has been
/// 'initialized' by setting the `I` bit to 0, then to `1`. [...] In addition, if the mapper has
/// not been 'unlocked', the first 32k of the first chip is always selected regardless (as if
/// $A000 contained $00)."
///
/// The half-done sequence is the point of the middle of this test: one write of `I=0` is not the
/// unlock, so a board that counted a single edge would swap PRG a write too early.
#[test]
fn req_core_100_the_nes_event_hardwires_the_first_32_kib_until_the_timer_is_unlocked() {
    let mut core = MapperRom::new(105).prg(32).chr(0).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // `O` set and a bank in `$E000`, with only the first half of the unlock written: nothing
    // moves, "as if $A000 contained $00".
    core.mmc1_write(0xA000, 0x08);
    core.mmc1_write(0xE000, 0x03);
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // `I` back to 1 completes it, and the registers that were ignored take effect at once: `O`
    // is still set, so this is the second chip with the MMC1's power-on mode — `B` switchable at
    // $8000 and the chip's last bank fixed at $C000.
    core.mmc1_write(0xA000, 0x18);
    assert_eq!([22, 23, 30, 31], core.prg_windows());
}

/// The `$A000` register's own diagram: bit 0 "not used (ordinarily selects bottom or top half of
/// CHR RAM)", bits 3-1 "0-3: select 32 KiB bank in $8000-$FFFF from lower 128KB ROM" and "4-7:
/// normal MMC1 behavior from upper 128KB ROM" — Disch's `O` bit above his two-bit `A`, which
/// "selects between the chips, and also determines which PRG Reg is used to select the page".
#[cfg(feature = "board-105")]
#[test]
fn req_core_100_the_nes_event_chip_select_picks_both_the_rom_half_and_the_prg_register() {
    let mut core = unlocked_nes_event();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // `O=0`: "use first PRG chip (first 128k), use `A` PRG Reg, 32k swap".
    core.mmc1_write(0xA000, 0x14); // AA = %10.
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    core.mmc1_write(0xA000, 0x16); // AA = %11.
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    // Bit 0 is not used: $17 is the same bank as $16.
    core.mmc1_write(0xA000, 0x17);
    assert_eq!([12, 13, 14, 15], core.prg_windows());

    // `O=1`: "use second PRG chip (second 128k), use `B` PRG Reg, MMC1 style swap" — all three
    // of the MMC1's modes, over the second 128 KiB and nothing else.
    core.mmc1_write(0xA000, 0x18);
    core.mmc1_write(0xE000, 0x03);
    assert_eq!([22, 23, 30, 31], core.prg_windows(), "P=1, S=1");
    core.mmc1_write(0x8000, 0x08);
    assert_eq!([16, 17, 22, 23], core.prg_windows(), "P=1, S=0");
    core.mmc1_write(0x8000, 0x00);
    assert_eq!([20, 21, 22, 23], core.prg_windows(), "P=0, 32 KiB");

    // `B` is three bits: the chip is 128 KiB and bit 3 of $E000 cannot reach past it.
    core.mmc1_write(0x8000, 0x0C);
    core.mmc1_write(0xE000, 0x0B);
    assert_eq!(
        [22, 23, 30, 31],
        core.prg_windows(),
        "$0B is bank 3 of the chip"
    );
}

#[cfg(feature = "board-105")]
/// Disch: "this mapper has 8k CHR-RAM, and it is not swappable", and the page's parts list "8K of
/// CHR RAM". The MMC1 inside still has its CHR mode bit and both CHR registers — "PPU A12 is
/// still connected to the MMC1" — and **none of them may move a pattern fetch**.
///
/// The image carries CHR-ROM so that the banks have numbers to read; a real *Nintendo World
/// Championships* dump has none, which the next test covers.
#[test]
fn req_core_100_no_nes_event_register_moves_its_unbanked_chr() {
    let mut core = MapperRom::new(105).prg(32).chr(16).load();
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());

    // The MMC1's 4 KiB CHR mode, and a bank in each of its two CHR registers.
    core.mmc1_write(0x8000, 0x1C);
    core.mmc1_write(0xA000, 0x02);
    core.mmc1_write(0xC000, 0x03);
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
}

#[cfg(feature = "board-105")]
/// "8K of CHR RAM" against REQ-CORE-012's 32 KiB default for an image with no CHR-ROM, the same
/// reading `req_core_100_the_vrc3_carries_eight_kib_of_chr_ram_*` pins for mapper 73.
#[test]
fn req_core_100_the_nes_event_carries_eight_kib_of_chr_ram() {
    let core = MapperRom::new(105).prg(32).chr(0).load();
    assert_eq!(
        0x2000,
        core.cartridge().unwrap().memory().chr_ram().len(),
        "the board's 8 KiB, not REQ-CORE-012's default"
    );
}

#[cfg(feature = "board-105")]
/// **The trap of this board**, and the reason it does not delegate `Mapper::prg_ram_enabled` to
/// the MMC1 it holds. Disch gives `$E000` as `[...W BBBB]` with "`W` = WRAM disable (same as
/// MMC1)" — so `W` is the only wire to the 8 KiB of PRG-RAM. But a *Nintendo World
/// Championships* dump is 256 KiB of PRG with no CHR-ROM and 8 KiB of PRG-RAM, which is exactly
/// the signature `Mmc1::is_snrom` reads as an SNROM, and on an SNROM bit 4 of `$A000` is a second
/// chip enable for that RAM. Bit 4 of `$A000` here is the **timer**: starting the tournament
/// clock would switch the save RAM off, and it powers on set, so the RAM would never be readable
/// at all.
#[test]
fn req_core_100_the_nes_event_wram_disable_is_e000_and_never_the_timer_bit() {
    let mut core = MapperRom::new(105).prg(32).chr(0).load();
    core.write_memory(0x6000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6000), "at power-on, with `I` set");

    // Both edges of the timer bit, neither of which is a RAM disable.
    core.mmc1_write(0xA000, 0x00);
    assert_eq!(0x5A, core.read_memory(0x6000), "the timer started");
    core.mmc1_write(0xA000, 0x10);
    assert_eq!(0x5A, core.read_memory(0x6000), "the timer reset");

    // `W` of $E000 is, as on every other MMC1 board.
    core.mmc1_write(0xE000, 0x10);
    assert_ne!(
        0x5A,
        core.read_memory(0x6000),
        "`W` did not disable the RAM"
    );
    core.mmc1_write(0xE000, 0x00);
    assert_eq!(0x5A, core.read_memory(0x6000), "and it came back");
}

#[cfg(feature = "board-105")]
/// `$8000` is "`[.... PSMM]`, same as MMC1 (but CHR mode bit isn't used)", so the mirroring is
/// the chip's own and this board adds nothing to it.
#[test]
fn req_core_100_the_nes_event_takes_its_mirroring_from_the_mmc1() {
    for (bits, expected) in [
        (0x00u8, zax_nes::NametableMirroring::SingleScreenA),
        (0x01, zax_nes::NametableMirroring::SingleScreenB),
        (0x02, zax_nes::NametableMirroring::Vertical),
        (0x03, zax_nes::NametableMirroring::Horizontal),
    ] {
        let mut core = MapperRom::new(105).prg(32).chr(0).load();
        core.mmc1_write(0x8000, 0x0C | bits);
        assert_eq!(expected, core.mirroring(), "$8000 = {bits:02X}");
    }
}

// ------------------------------------------- the TQROM (119) and the two MMC3 multicarts
// ------------------------------------------- (47, 37), MF-6a3 step 1

/// nesdev ("INES Mapper 119"): "Bit 6 of the CHR bank number is used to switch between the CHR
/// ROM chip and the CHR RAM chip."
///
/// The select is a line of the **bank number**, so it is read the same way on a 1 KiB register
/// and on a 2 KiB one -- and a 2 KiB register, which "ignores bit 0", puts **both** of its 1 KiB
/// windows on the same chip. Bit 7 of $8000 swaps which registers drive which windows, so the
/// claim is made in both CHR modes.
#[cfg(feature = "board-119")]
#[test]
fn req_core_100_tqrom_bit_6_of_a_chr_bank_selects_the_chr_ram_chip() {
    let mut core = MapperRom::new(119).prg(16).chr(64).load();

    // A 1 KiB register, CHR mode 0: R2 drives PPU $1000-$13FF, the 1 KiB half being the high one.
    core.mmc3_bank(2, 0x05);
    assert_eq!(
        5,
        peek(&core, 0x1000),
        "bit 6 clear: the CHR-ROM chip, bank 5"
    );
    core.mmc3_bank(2, 0x45); // Bit 6 set, low three bits still 5.
    chr_write(&mut core, 0x1000, 0xA5);
    assert_eq!(0xA5, peek(&core, 0x1000), "bit 6 set: the CHR-RAM chip");
    core.mmc3_bank(2, 0x05);
    assert_eq!(5, peek(&core, 0x1000), "and back to the ROM, unchanged");
    core.mmc3_bank(2, 0x45);
    assert_eq!(0xA5, peek(&core, 0x1000), "the RAM kept it");

    // The same register in CHR mode 1, where bit 7 of $8000 has moved the 1 KiB half to
    // $0000-$03FF: the select is read off the same bit of the same register.
    //
    // The writes go through `txsrom_bank_in_chr_mode_1`, which exists for the trap its own note
    // describes: the register index and the CHR mode share $8000, so `mmc3_bank` -- which
    // writes the index alone -- would put the board back in CHR mode 0 on every call.
    txsrom_bank_in_chr_mode_1(&mut core, 2, 0x45);
    assert_eq!(0xA5, peek(&core, 0x0000), "CHR mode 1: still the RAM chip");
    txsrom_bank_in_chr_mode_1(&mut core, 2, 0x05);
    assert_eq!(5, peek(&core, 0x0000), "CHR mode 1: still the ROM chip");

    // A 2 KiB register: R0 is at $1000-$17FF in CHR mode 1, and **both** of its 1 KiB windows
    // follow bit 6 of the one register.
    txsrom_bank_in_chr_mode_1(&mut core, 0, 0x40); // 1 KiB banks $40 and $41, RAM banks 0 and 1.
    chr_write(&mut core, 0x1000, 0xB0);
    chr_write(&mut core, 0x1400, 0xB1);
    assert_eq!(
        (0xB0, 0xB1),
        (peek(&core, 0x1000), peek(&core, 0x1400)),
        "both halves of the 2 KiB register are on the RAM chip"
    );
    txsrom_bank_in_chr_mode_1(&mut core, 0, 0x0A); // 1 KiB banks 10 and 11 of the ROM.
    assert_eq!(
        (10, 11),
        (peek(&core, 0x1000), peek(&core, 0x1400)),
        "and both halves move back to the ROM together"
    );

    // A `ChrView` is one slice plus eight offsets and cannot say "this window is the other
    // chip", so this board answers `None` -- the body the MMC5 and the RAMBO-1 already give.
    assert!(
        core.cartridge().unwrap().chr_peek_view().is_none(),
        "a TQROM cannot be expressed as one ChrView"
    );
}

/// Section 5 of `docs/review/MF-6-inventory.md`'s own wording: "a write to a CHR-RAM-selected
/// window landing in RAM and not being swallowed".
///
/// `Mapper::ppu_write`'s default drops every write on a board that has CHR-ROM, and a TQROM has
/// both chips -- so without the board's own override the 8 KiB would be unreachable. The write
/// must also be dropped on a ROM window, be visible through a *second* window that shows the
/// same RAM bank and not through one that shows another, and leave the CHR-ROM alone.
#[cfg(feature = "board-119")]
#[test]
fn req_core_100_tqrom_a_write_to_a_ram_selected_window_is_not_swallowed() {
    let mut core = MapperRom::new(119).prg(16).chr(64).load();

    // R2 on RAM bank 3, R3 on RAM bank 4, R4 on ROM bank 7, R5 on RAM bank 3 again.
    core.mmc3_bank(2, 0x43);
    core.mmc3_bank(3, 0x44);
    core.mmc3_bank(4, 0x07);
    core.mmc3_bank(5, 0x43);

    chr_write(&mut core, 0x1000, 0x11); // RAM bank 3.
    chr_write(&mut core, 0x1800, 0x22); // ROM bank 7: dropped.

    assert_eq!(0x11, peek(&core, 0x1000), "the write landed in the RAM");
    assert_eq!(7, peek(&core, 0x1800), "the ROM window swallowed the write");
    assert_eq!(
        0x11,
        peek(&core, 0x1C00),
        "a second window on the same RAM bank shows it"
    );
    // RAM bank 4 is still the zero it powered on with.
    assert_eq!(0, peek(&core, 0x1400), "another RAM bank does not");

    // And the CHR-ROM itself is untouched: the same bank, selected again, reads its own number.
    core.mmc3_bank(2, 0x03);
    assert_eq!(3, peek(&core, 0x1000), "the CHR-ROM bank 3 is still bank 3");

    // The board's 8 KiB, and not REQ-CORE-012's 32 KiB default: that default only applies to an
    // image with no CHR-ROM, and byte 5 of this one declares 64 KiB of it.
    assert_eq!(
        0x2000,
        core.cartridge().unwrap().memory().chr_ram().len(),
        "the TQROM's own CHR-RAM chip"
    );
}

/// Fills the TQROM's eight 1 KiB CHR-RAM banks with `$40 | bank`, through a RAM-selected window.
///
/// `$40 | bank` is **outside** the byte space of a 64-bank CHR-ROM: a numbered-bank image fills
/// ROM bank `b` with `b`, and [`BankWindows::map_chr_1k`] wraps the bank number modulo the ROM's
/// 64, so every ROM window reads a byte in `0..=63`. A read of `$40 | k` therefore says "the RAM
/// chip, bank `k`" and nothing else can — which is what makes the exhaustive test below able to
/// name the chip *and* the offset from one byte.
#[cfg(feature = "board-119")]
fn fill_tqrom_chr_ram(core: &mut NesCore) {
    for bank in 0..8u8 {
        core.mmc3_bank(2, 0x40 | bank);
        chr_write(core, 0x1000, 0x40 | bank);
    }
}

/// **The regression gate of lot MF-UNL-2 step 1**, and the one place this lot can test against
/// something that already works: mapper 119's chip select and RAM offset over **all 256 values**
/// of a bank register, in both CHR modes and through both register widths.
///
/// The claim is the family page's, not a range: *MMC3 with CHR ROM and CHR RAM* gives mapper 119
/// first RAM bank "**64, 192**", `RAM mirrored? **Yes**`, and in its *Behaviour* section "119:
/// **CHR A16** switches between CHR ROM and CHR RAM". One bank line, so
///
/// * the chip is the **RAM** exactly when bit 6 of the bank number is set — `$40-$7F` *and*
///   `$C0-$FF`, which is why the page names two first banks;
/// * the offset inside the 8 KiB chip is the low three bits of the bank number, the chip having
///   three address lines above the kilobyte, so banks `$40`, `$48`, … `$78`, `$C0`, … all alias.
///
/// Mesen 2 registers the number as `MMC3_ChrRam(0x40, 0x7F, 8)` and reads that pair as a closed
/// range, under which `$C0-$FF` would read the CHR-**ROM**. `docs/review/UNL-census.md` §7.2
/// quotes the registration; the page is what this build implements, and what it implemented
/// before the generalisation. This test is the proof that nothing moved.
#[cfg(feature = "board-119")]
#[test]
fn req_core_100_tqrom_chr_ram_select_is_chr_a16_over_all_256_bank_numbers() {
    let mut core = MapperRom::new(119).prg(16).chr(64).load();
    fill_tqrom_chr_ram(&mut core);

    // What a window showing bank `value` must read: the RAM marker when CHR A16 is set, and the
    // ROM's own bank number — wrapped modulo its 64 banks — when it is not.
    let expected = |value: u8| -> u8 {
        if value & 0x40 == 0 {
            value % 64
        } else {
            0x40 | (value & 0x07)
        }
    };

    for value in 0..=255u8 {
        // A 1 KiB register (R2) in CHR mode 0, at PPU $1000-$13FF.
        core.mmc3_bank(2, value);
        assert_eq!(
            expected(value),
            peek(&core, 0x1000),
            "CHR mode 0, R2 = {value:#04X}"
        );

        // The same register in CHR mode 1, where bit 7 of $8000 has moved the 1 KiB half to
        // $0000-$03FF. The select is a line of the bank number, so the window it lands in
        // changes nothing.
        txsrom_bank_in_chr_mode_1(&mut core, 2, value);
        assert_eq!(
            expected(value),
            peek(&core, 0x0000),
            "CHR mode 1, R2 = {value:#04X}"
        );

        // A 2 KiB register (R0) in CHR mode 0, at PPU $0000-$07FF. It "ignores bit 0", so its
        // two 1 KiB windows show banks `value & $FE` and `value | $01` — both on whichever chip
        // bit 6 of the one register names, and the RAM offsets differ in bit 0 only.
        core.mmc3_bank(0, value);
        assert_eq!(
            (expected(value & 0xFE), expected(value | 0x01)),
            (peek(&core, 0x0000), peek(&core, 0x0400)),
            "CHR mode 0, R0 = {value:#04X}"
        );
    }
}

// ------------------------------------------- the UNL-KOF97 (263) and the TXC 189,
// ------------------------------------------- MF-UNL-2 steps 4 and 5

/// nesdev ("NES 2.0 Mapper 263"): "The data written to the MMC3 clone has the following bit order
/// compared to a regular MMC3:
///
/// ```text
/// Written bit 76543210
///             --------
/// Actual bit  76243051
/// ```
///
/// Which means that to get from the written data to the actual data, the following operation must
/// be performed:
///
/// ```text
/// Val = (Val & 0xD8) | ((Val & 0x20) >> 4) | ((Val & 4) << 3) | ((Val & 2) >> 1) | ((Val & 1) << 2);
/// ```"
///
/// The page states the permutation **twice**, and the board implements the expression — so this
/// test builds it from the **bit map** instead, over all 256 inputs, and the two statements prove
/// each other. The board is read through its own CHR register, which takes the permuted byte as
/// a 1 KiB bank number: the image has 256 numbered banks, so the byte that comes out is the byte
/// that landed.
#[cfg(feature = "board-263")]
#[test]
fn req_core_100_mapper_263_permutes_every_one_of_the_256_written_bytes() {
    // `Actual bit 76243051` under `Written bit 76543210`: the actual value's bit at position
    // `i` is the written value's bit at `SOURCE[i]`, positions counted from 7 down.
    const SOURCE: [u32; 8] = [7, 6, 2, 4, 3, 0, 5, 1];
    let expected = |written: u8| -> u8 {
        let mut actual = 0u8;
        for (offset, source) in SOURCE.iter().enumerate() {
            let bit = 7 - u32::try_from(offset).unwrap_or(0);
            if written & (1 << source) != 0 {
                actual |= 1 << bit;
            }
        }
        actual
    };

    let mut core = MapperRom::new(263).prg(16).chr(256).load();

    // The **index** is permuted too -- it is a byte written to the same clone -- so selecting
    // register 2 means writing the byte that permutes to 2, which is the first thing a test of
    // this board has to notice.
    let select_r2 = (0..=255u8)
        .find(|candidate| expected(*candidate) == 0x02)
        .expect("the permutation is a bijection on the byte");

    // R2 is at PPU $1000-$13FF in CHR mode 0. The MMC3's $8000 is written at $8000 on this
    // board -- A12 clear -- and its $8001 at $9000, A12 set.
    for written in 0..=255u8 {
        core.mapper_write(0x8000, select_r2);
        core.mapper_write(0x9000, written);
        assert_eq!(
            expected(written),
            peek(&core, 0x1000),
            "263: the byte written as {written:#04X}"
        );
    }
}

/// nesdev ("NES 2.0 Mapper 263"): "**A12 is used for the MMC3's A0**", with the page's own
/// eight-row table.
///
/// | written | `$8000` | `$9000` | `$A000` | `$B000` | `$C000` | `$D000` | `$E000` | `$F000` |
/// | --- | --- | --- | --- | --- | --- | --- | --- | --- |
/// | MMC3 | `$8000` | `$8001` | `$A000` | `$A001` | `$C000` | `$C001` | `$E000` | `$E001` |
///
/// Checked the way the census asks the 114 scramble to be checked: the whole bank layout against
/// a plain mapper-4 image of the same ROM, written through the scrambled pair — and then the
/// three registers an address table alone cannot reach, `$A001`'s mirroring, `$C000`'s latch and
/// `$E001`'s enable.
#[cfg(feature = "board-263")]
#[test]
fn req_core_100_mapper_263_drives_the_mmc3s_a0_from_a12() {
    let banks = [0x02u8, 0x04, 0x11, 0x12, 0x13, 0x14, 0x05, 0x06];

    let mut reference = MapperRom::new(4).prg(16).chr(64).load();
    for (register, value) in banks.iter().copied().enumerate() {
        reference.mmc3_bank(u8::try_from(register).unwrap(), value);
    }

    // The permutation is on the data, so the *value* written has to be the one that permutes to
    // what the reference wrote. `permute` is an involution on no bit in particular, so the
    // inverse is built by search over the 256 bytes.
    let permute = |v: u8| -> u8 {
        (v & 0xD8) | ((v & 0x20) >> 4) | ((v & 0x04) << 3) | ((v & 0x02) >> 1) | ((v & 0x01) << 2)
    };
    let unpermute = |want: u8| -> u8 {
        (0..=255u8)
            .find(|candidate| permute(*candidate) == want)
            .expect("the permutation is a bijection on the byte")
    };

    let mut core = MapperRom::new(263).prg(16).chr(64).load();
    for (register, value) in banks.iter().copied().enumerate() {
        // $8000 is at $8000 (A12 clear) and $8001 at $9000 (A12 set).
        core.mapper_write(0x8000, unpermute(u8::try_from(register).unwrap()));
        core.mapper_write(0x9000, unpermute(value));
    }

    assert_eq!(reference.prg_windows(), core.prg_windows(), "263: PRG");
    assert_eq!(reference.chr_windows(), core.chr_windows(), "263: CHR");

    // $A001 means the MMC3's mirroring register, and $B000 means its $A001 -- the PRG-RAM
    // protection. So a write of 1 at $A000 is horizontal and one at $B000 is not.
    core.mapper_write(0xA000, unpermute(0x01));
    assert_eq!(Nm::Horizontal, core.mirroring(), "263: $A000");
    core.mapper_write(0xA000, unpermute(0x00));
    assert_eq!(Nm::Vertical, core.mirroring(), "263: $A000 = 0");

    // $C000 means the latch, $D000 the reload and $F000 the enable: three A12 rises with a latch
    // of 2 give one IRQ, and the same writes made at the *unscrambled* addresses give none.
    core.write_memory(0x2001, 0x00);
    core.mapper_write(0xC000, unpermute(0x02));
    core.mapper_write(0xD000, unpermute(0x00));
    core.mapper_write(0xF000, unpermute(0x00));
    for i in 0..3i64 {
        let cart = core.cartridge_mut().unwrap();
        cart.on_ppu_address_bus(0x0000, 10_000 + i * 100);
        cart.on_ppu_address_bus(0x1000, 10_000 + i * 100 + 50);
    }
    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "263: $C000/$D000/$F000 are the latch, the reload and the enable"
    );
}

/// nesdev ("INES Mapper 189"): "`D~[..PP ....]` `A~[010. ...1 .... ....]`
/// `(Address & $E100)==$4100` … The original board variants switch four 32 KiB PRG-ROM banks
/// based on bits 4 and 5 on writes to CPU $4100-$5FFF (address bit 9 must be set)", and for the
/// oversize board `D~[.PPP ....]`, "PRG A17..A15".
///
/// The census's own wording is "a `$4100` write with A9 set switches the 32 KiB bank; one with A9
/// clear does not" — and **"address bit 9" is A8, the ninth line**, which the page says three
/// ways: its mask `$E100` reads A8 and not A9, its address diagram sets the bit in the nibble
/// that is bits 11-8, and the oversize game "still writes to the original `$4132` address", which
/// has A8 set and A9 clear. So the test is `$4100` against `$4000`.
#[cfg(feature = "board-189")]
#[test]
fn req_core_100_mapper_189_banks_32_kib_of_prg_from_4100() {
    let mut core = MapperRom::new(189).prg(32).chr(64).load();

    // Power-on is bank 0, the whole of $8000-$FFFF, and the MMC3's own R6/R7 reach nothing.
    assert_eq!([0, 1, 2, 3], core.prg_windows(), "189: the first 32 KiB");
    core.mmc3_bank(6, 0x09);
    core.mmc3_bank(7, 0x0A);
    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "189: the MMC3's PRG registers reach nothing on this board"
    );

    // Bits 4-5 of the written byte, over all four banks of a 256 KiB image.
    for bank in 0..4u8 {
        core.mapper_write(0x4100, bank << 4);
        let base = bank * 4;
        assert_eq!(
            [base, base + 1, base + 2, base + 3],
            core.prg_windows(),
            "189: $4100 = {:#04X}",
            bank << 4
        );
    }

    // The oversize board's third bit, PRG A17, which this 256 KiB image can address: banks 4-7
    // are the second 128 KiB and no original-board image reaches them.
    for bank in 4..8u8 {
        core.mapper_write(0x4100, bank << 4);
        let base = bank * 4;
        assert_eq!(
            [base, base + 1, base + 2, base + 3],
            core.prg_windows(),
            "189: the oversize board's PRG A17, $4100 = {:#04X}",
            bank << 4
        );
    }

    // The mask is $E100: A8 set and A15-A13 = 010. $4132 is the address the oversize game
    // actually writes, and $4000 -- A8 clear -- is not decoded at all.
    core.mapper_write(0x4132, 0x30);
    assert_eq!(
        [12, 13, 14, 15],
        core.prg_windows(),
        "189: $4132 is the address Mario Fighter III writes"
    );
    core.mapper_write(0x4000, 0x00);
    assert_eq!(
        [12, 13, 14, 15],
        core.prg_windows(),
        "189: A8 clear is not the register"
    );
    // And the low nibble of the byte is not the bank: that is the generic pirate boards' wiring,
    // which this build deliberately does not decode.
    core.mapper_write(0x4100, 0x01);
    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "189: only the upper nibble is the bank"
    );

    // **One decode runs the original board too**, which is why the third bit is taken rather
    // than masked away: on the 128 KiB of a *Thunder Warrior* image PRG A17 wraps, so the four
    // banks its two bits name are the only four there are.
    let mut small = MapperRom::new(189).prg(16).chr(64).load();
    small.mapper_write(0x4100, 0x40);
    assert_eq!(
        [0, 1, 2, 3],
        small.prg_windows(),
        "189: PRG A17 wraps on the original board's 128 KiB"
    );
    small.mapper_write(0x4100, 0x30);
    assert_eq!(
        [12, 13, 14, 15],
        small.prg_windows(),
        "189: and its own two bits reach all four banks"
    );

    // Everything else is a stock MMC3: its CHR banking, its mirroring and its counter.
    core.mmc3_bank(2, 0x07);
    assert_eq!(
        7,
        core.chr_bank(0x1000),
        "189: the MMC3's CHR banking is intact"
    );
    core.mapper_write(0xA000, 0x01);
    assert_eq!(
        Nm::Horizontal,
        core.mirroring(),
        "189: and its mirroring register"
    );

    // The counter is built although *Thunder Warrior*'s 28-pin PT8154 has none: nothing in an
    // image distinguishes the two boards, and a dump that never writes $C000-$E001 never sees it.
    core.write_memory(0x2001, 0x00);
    core.mapper_write(0xC000, 0x02);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for i in 0..3i64 {
        let cart = core.cartridge_mut().unwrap();
        cart.on_ppu_address_bus(0x0000, 10_000 + i * 100);
        cart.on_ppu_address_bus(0x1000, 10_000 + i * 100 + 50);
    }
    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "189: the fully functional MMC3 clone Street Fighter II needs"
    );
}

// ------------------------------------------- the six scrambled MMC3 clones (114, 115, 121,
// ------------------------------------------- 123, 182, 187), MF-UNL-2 step 3

/// Loads bank register `register` of an MMC3 the long way round, through whichever addresses
/// `mapper`'s board scrambles `$8000` and `$8001` to.
///
/// nesdev ("INES Mapper 114"), submapper 0: a write meant for the MMC3's `$8000` is made at
/// `$A000` and one meant for its `$8001` at `$C000`; submapper 1 puts the second at `$8001`.
/// The index itself is scrambled too, so the register number is passed through the page's
/// `$8000 index written` table on the way in.
#[cfg(feature = "board-mmc3-scramble")]
fn scrambled_mmc3_bank(
    core: &mut NesCore,
    mapper: usize,
    submapper1: bool,
    register: u8,
    value: u8,
) {
    const INDEX_A: [u8; 8] = [0, 3, 1, 5, 6, 7, 2, 4];
    const INDEX_B: [u8; 8] = [0, 2, 5, 3, 6, 1, 7, 4];

    let (select, data, table): (u16, u16, Option<[u8; 8]>) = match (mapper, submapper1) {
        // 114 submapper 0 and 182: $8000 is written at $A000, $8001 at $C000.
        (114, false) | (182, _) => (0xA000, 0xC000, Some(INDEX_A)),
        // 114 submapper 1: $8000 at $A000, $8001 at $8001.
        (114, true) => (0xA000, 0x8001, Some(INDEX_B)),
        // 123: no address scrambling, index scrambling with submapper 0's pattern.
        (123, _) => (0x8000, 0x8001, Some(INDEX_A)),
        // 115, 187, 121: neither.
        _ => (0x8000, 0x8001, None),
    };

    // The table maps *written* index to *meant* register, so a test that wants register `r`
    // has to write the index the table sends there.
    let written = table.map_or(register, |table| {
        // The tables are permutations of 0-7, so the search always lands.
        let index = table.iter().position(|meant| *meant == register);
        index.and_then(|i| u8::try_from(i).ok()).unwrap_or(register)
    });
    core.mapper_write(select, written);
    core.mapper_write(data, value);
}

/// nesdev ("INES Mapper 114"), both submappers' `Address written` / `MMC3 register meant` and
/// `$8000 index written` / `MMC3 register meant` tables, read **one-to-one** as that page's own
/// *Notes* require.
///
/// The claim: a write made at the *scrambled* address with the *scrambled* index produces the
/// same bank layout as the plain MMC3 of the same image, and a write made at the unscrambled
/// address does not. 182 is 114 submapper 0 and 123 scrambles the index alone, so both are swept
/// here beside it; 115, 187 and 121 scramble nothing and stand as the control.
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn req_core_100_the_scrambled_boards_reproduce_an_unscrambled_mmc3() {
    // The layout a plain MMC3 reaches after loading R0-R5 and R6-R7.
    let banks = [0x02u8, 0x04, 0x11, 0x12, 0x13, 0x14, 0x05, 0x06];
    let mut reference = MapperRom::new(4).prg(16).chr(64).load();
    for (register, value) in banks.iter().copied().enumerate() {
        reference.mmc3_bank(u8::try_from(register).unwrap(), value);
    }
    let expected_prg = reference.prg_windows();
    let expected_chr = reference.chr_windows();

    for (mapper, submapper1) in [
        (114usize, false),
        (114, true),
        (115, false),
        (121, false),
        (123, false),
        (182, false),
        (187, false),
    ] {
        let mut rom = MapperRom::new(mapper).prg(16).chr(64);
        if submapper1 {
            rom = rom.submapper(1);
        }
        let mut core = rom.load();

        for (register, value) in banks.iter().copied().enumerate() {
            scrambled_mmc3_bank(
                &mut core,
                mapper,
                submapper1,
                u8::try_from(register).unwrap(),
                value,
            );
        }

        assert_eq!(
            expected_prg,
            core.prg_windows(),
            "mapper {mapper} submapper {}: PRG",
            u8::from(submapper1)
        );
        assert_eq!(
            expected_chr,
            core.chr_windows(),
            "mapper {mapper} submapper {}: CHR",
            u8::from(submapper1)
        );
    }

    // And the other half of the claim: on a board that scrambles addresses, writing at the
    // *unscrambled* pair does **not** reach the same registers. $8000 on a 114 means the MMC3's
    // $A001 -- the PRG-RAM protection -- so a bank written that way never lands.
    for (mapper, submapper1) in [(114usize, false), (114, true), (182, false)] {
        let mut rom = MapperRom::new(mapper).prg(16).chr(64);
        if submapper1 {
            rom = rom.submapper(1);
        }
        let mut core = rom.load();
        for (register, value) in banks.iter().copied().enumerate() {
            core.mmc3_bank(u8::try_from(register).unwrap(), value);
        }

        assert_ne!(
            expected_prg,
            core.prg_windows(),
            "mapper {mapper}: an unscrambled write must not reach the MMC3's own registers"
        );
    }
}

/// The NROM-like PRG override of 114, 115, 123 and 187, which every one of their pages describes
/// in the same two sentences: bit 7 is "0: Use PRG bank from MMC3 … 1: Ignore PRG bank from
/// MMC3", and the field selects a "16 KiB PRG-ROM bank at CPU $8000-$BFFF **and** $C000-$FFFF"
/// with one bit optionally "replace[d] … with CPU A14".
///
/// So with the override set the MMC3's own PRG registers stop reaching $8000-$FFFF, and clearing
/// it restores them — which is the census's own wording for this test. The per-number part is
/// where the bank's four bits sit in the byte, and **123's bit order is the page's prose**: the
/// bank's bits 3, 2, 1 and 0 are register bits 5, 2, 3 and 0, and register bit 4 is a
/// don't-care. The page's diagram `[.M31 .2S0]` says bit 4 instead and contradicts the page's own
/// twice-stated "ignore bits 0-3/5"; Q-144 §1 records that all four cross-checked emulators read
/// bit 3, and MF-FIX applied it. This case uses bank 5, whose bits are 2 and 0, so it is blind to
/// the difference — `q_144_mapper_123_takes_the_banks_bit_1_from_register_bit_3` is the test that
/// sees it.
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn req_core_100_the_nrom_override_takes_prg_away_from_the_mmc3() {
    // (mapper, port, the byte that selects 16 KiB bank 5 in NROM-128 mode, the NROM-256 bit)
    let cases: [(usize, u16, u8, u8); 4] = [
        // 114: `M.S. BBBb`, bank in bits 3-0, S in bit 5.
        (114, 0x6000, 0x80 | 0x05, 0x20),
        // 115: `MPN. PPPp`, the same field, N in bit 5.
        (115, 0x6000, 0x80 | 0x05, 0x20),
        // 123: `.M31 .2S0`, M in bit 6 -- bank 5 is bits 2 and 0 of the bank, so register bits
        // 2 and 0.
        (123, 0x5800, 0x40 | 0x04 | 0x01, 0x02),
        // 187: `M.NB BBb.`, bank in bits 4-1, N in bit 5.
        (187, 0x5000, 0x80 | (0x05 << 1), 0x20),
    ];

    for (mapper, port, nrom128, nrom256_bit) in cases {
        let mut core = MapperRom::new(mapper).prg(32).chr(64).load();

        // The MMC3's own banking first, so that the override has something to take away.
        scrambled_mmc3_bank(&mut core, mapper, false, 6, 0x09);
        scrambled_mmc3_bank(&mut core, mapper, false, 7, 0x0A);
        assert_eq!(
            [9, 10, 30, 31],
            core.prg_windows(),
            "mapper {mapper}: the MMC3's own layout"
        );

        // NROM-128: the same 16 KiB bank at both halves, so banks 10 and 11 twice.
        core.mapper_write(port, nrom128);
        assert_eq!(
            [10, 11, 10, 11],
            core.prg_windows(),
            "mapper {mapper}: NROM-128 bank 5 at both halves"
        );

        // The MMC3's registers go on latching and go on not reaching the bus.
        scrambled_mmc3_bank(&mut core, mapper, false, 6, 0x03);
        assert_eq!(
            [10, 11, 10, 11],
            core.prg_windows(),
            "mapper {mapper}: the MMC3's PRG registers are overridden"
        );

        // NROM-256: "Replace bit 0 with CPU A14", so the even/odd pair of 16 KiB banks -- bank 5
        // with its low bit cleared is 4, which is 8 KiB banks 8..11.
        core.mapper_write(port, nrom128 | nrom256_bit);
        assert_eq!(
            [8, 9, 10, 11],
            core.prg_windows(),
            "mapper {mapper}: NROM-256 is a 32 KiB window"
        );

        // And clearing bit 7 (bit 6 on 123) restores the MMC3's own, which has kept latching.
        core.mapper_write(port, 0x00);
        assert_eq!(
            [3, 10, 30, 31],
            core.prg_windows(),
            "mapper {mapper}: the MMC3's banking comes back"
        );
    }
}

/// Q-144 §1, settled by `docs/review/Q-XCHECK.md` and applied by MF-FIX: **mapper 123's `$5800`
/// bank field takes its bit 1 from register bit 3, and register bit 4 is a don't-care.**
///
/// nesdev's mapper-123 page contradicts itself. Its register diagram is `D~[.M31 .2S0]`, which
/// labels register bit **4** as the bank's bit 1 and leaves bit 3 unused; but the page's prose
/// says, twice, to **ignore `$5800` bits 0-3 and 5**, and that is only true of a bank field whose
/// bits are 5, 3, 2 and 0. All four cross-checked projects read bit 3 — Mesen 2's `MMC3_123.h`
/// computes `(_exReg[0] & 0x08) >> 2`, and FCEUX, Nestopia UE and puNES agree — so the diagram is
/// the error and the prose is the board.
///
/// The test walks the two bits one at a time with everything else clear, which is the only
/// observation that separates the two readings: under the diagram's reading the `$08` write would
/// move the bank and the `$10` write would not, and both assertions below would fail the other
/// way round. Keeping the bit-4 case is the point — it is what stops the old reading from being
/// restored silently.
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn q_144_mapper_123_takes_the_banks_bit_1_from_register_bit_3() {
    // 32 banks of 8 KiB = sixteen 16 KiB banks, so the field's bits 3, 2, 1 and 0 are all
    // reachable without a wrap and bank 2 is distinguishable from bank 0.
    let mut core = MapperRom::new(123usize).prg(32).chr(64).load();

    // Bit 6 is `M` on this number ("Ignore PRG bank from MMC3"), bit 1 is `S` (NROM-128 here), so
    // `$40` alone is bank 0 in NROM-128 mode: 16 KiB bank 0 at both halves.
    core.mapper_write(0x5800, 0x40);
    assert_eq!(
        [0, 1, 0, 1],
        core.prg_windows(),
        "$5800 = $40: the override is on and the bank field is zero"
    );

    // Register bit 3 is the bank's bit 1, so bank 2 -- 8 KiB banks 4 and 5 at both halves.
    core.mapper_write(0x5800, 0x40 | 0x08);
    assert_eq!(
        [4, 5, 4, 5],
        core.prg_windows(),
        "$5800 = $48: register bit 3 is the bank's bit 1 (the page's prose, all four emulators)"
    );

    // Register bit 4 is the don't-care, so the bank stays zero. Under the page's diagram this
    // would be bank 2 and the two assertions would be the other way round.
    core.mapper_write(0x5800, 0x40 | 0x10);
    assert_eq!(
        [0, 1, 0, 1],
        core.prg_windows(),
        "$5800 = $50: register bit 4 is inert (the page's diagram says it is the bank's bit 1)"
    );

    // And the two together are still bank 2, which pins bit 4 as inert rather than merely
    // weaker: an implementation that ORed both bits into the bank's bit 1 would also pass the
    // two assertions above.
    core.mapper_write(0x5800, 0x40 | 0x18);
    assert_eq!(
        [4, 5, 4, 5],
        core.prg_windows(),
        "$5800 = $58: bit 4 adds nothing to what bit 3 already selected"
    );

    // The rest of the field is unchanged by this fix: bit 5 is the bank's bit 3, bit 2 its bit 2
    // and bit 0 its bit 0, so $40|$25 is bank 13 -- 8 KiB banks 26 and 27.
    core.mapper_write(0x5800, 0x40 | 0x25);
    assert_eq!(
        [26, 27, 26, 27],
        core.prg_windows(),
        "$5800 = $65: bank bits 3, 2 and 0 are register bits 5, 2 and 0, as before"
    );
}

/// nesdev ("INES Mapper 114"): "`$6001`: `[.... ...+]` Select 256 KiB CHR-ROM bank at PPU
/// $0000-$1FFF", and 115's "`$6001`: `[.... ...C]` CHR A18".
///
/// One bit above the MMC3's seven, so the same CHR register reads two different 256 KiB halves
/// of a 512 KiB CHR-ROM depending on it.
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn req_core_100_the_outer_chr_bank_of_114_and_115_is_chr_a18() {
    for mapper in [114usize, 115, 182] {
        // 512 banks of 1 KiB: CHR A18 reaches the second half.
        let mut core = MapperRom::new(mapper).prg(16).chr(512).load();
        scrambled_mmc3_bank(&mut core, mapper, false, 2, 0x05);

        // The numbered banks only carry eight bits, so bank 5 and bank 261 both read 5; the
        // offset is what the test reads instead.
        let offset = |core: &NesCore| {
            core.cartridge()
                .unwrap()
                .chr_peek_view()
                .map(|view| view.offsets()[4])
        };
        let low = offset(&core);
        core.mapper_write(0x6001, 0x01);
        let high = offset(&core);

        assert_ne!(
            low, high,
            "mapper {mapper}: $6001 bit 0 must move the window by 256 KiB"
        );
        assert_eq!(
            Some(low.unwrap() + 256 * 1024),
            high,
            "mapper {mapper}: and by exactly 256 KiB"
        );

        core.mapper_write(0x6001, 0x00);
        assert_eq!(low, offset(&core), "mapper {mapper}: and back");
    }
}

/// nesdev ("INES Mapper 121" and "INES Mapper 187"), the same sentence on both pages: "CHR A18
/// (the second half of CHR-ROM) is directly selected by **PPU A12**, regardless of what the MMC3
/// is set to do. This means that the 'left' pattern table always uses the first and the 'right'
/// pattern table always uses the second 256 KiB half of CHR-ROM."
///
/// So a fetch in $0000-$0FFF and one in $1000-$1FFF land in different 256 KiB halves **with
/// identical MMC3 CHR registers**, which is the census's own wording — and "regardless of what
/// the MMC3 is set to do" is checked by flipping `$8000` bit 7, the CHR mode, which is where
/// Mesen 2 reads the line from instead.
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn req_core_100_mappers_121_and_187_drive_chr_a18_from_ppu_a12() {
    /// The second 256 KiB half of a 512 KiB CHR-ROM, in bytes.
    const HALF: usize = 256 * 1024;
    /// The four 1 KiB windows of PPU $0000-$0FFF, the "left" pattern table.
    const HALF_SLOTS: usize = 4;

    for mapper in [121usize, 187] {
        let mut core = MapperRom::new(mapper).prg(16).chr(512).load();

        // Every CHR register on the same bank, so only the window's own address can separate
        // the two pattern tables.
        for register in 0..6u8 {
            scrambled_mmc3_bank(&mut core, mapper, false, register, 0x00);
        }

        for chr_mode in [0x00u8, 0x80] {
            core.mapper_write(0x8000, chr_mode);
            let offsets = core
                .cartridge()
                .unwrap()
                .chr_peek_view()
                .expect("an MMC3 clone answers a peek view")
                .offsets();

            for (slot, offset) in offsets.iter().enumerate().take(HALF_SLOTS) {
                assert!(
                    *offset < HALF,
                    "mapper {mapper} CHR mode {chr_mode:#04X} slot {slot}: the left pattern                      table must be in the first 256 KiB half, not at {offset:#X}"
                );
            }
            for (slot, offset) in offsets.iter().enumerate().skip(HALF_SLOTS) {
                assert!(
                    (HALF..2 * HALF).contains(offset),
                    "mapper {mapper} CHR mode {chr_mode:#04X} slot {slot}: the right pattern                      table must be in the second 256 KiB half, not at {offset:#X}"
                );
            }
            // And the two tables show the *same* bank 256 KiB apart, which is what "regardless
            // of what the MMC3 is set to do" means with every CHR register on bank 0.
            assert_eq!(
                offsets[0] + HALF,
                offsets[4],
                "mapper {mapper} CHR mode {chr_mode:#04X}: exactly one 256 KiB half apart"
            );
        }
    }
}

/// The two IRQ rules this family carries, which are each other's negation and are each stated by
/// name:
///
/// * **114** (and 182) — "IRQ behavior resembles that of the MMC3A, i.e. a latch value of zero
///   **disables** the IRQ, on which *Aladdin* depends" (nesdev, "INES Mapper 114");
/// * **115** — "IRQ behavior resembles 'Sharp' behavior, i.e. a latch value of zero causes an
///   IRQ **on every scanline**, on which the Chinese version of *幽☆遊☆白書 Final* depends"
///   (nesdev, "INES Mapper 115").
///
/// The census's own wording for this test is "115's sharp IRQ: latch = 0 asserts on every
/// scanline, where the MMC3 proper does not" — and in *this* build the MMC3 proper is revision B
/// and **does**, which is REQ-CORE-114's own `mmc3_test_2` test 6. So the contrast that tests the
/// parameter is 115 against **114**, which is what is checked here; 123, 187 and 121 take the
/// chip as it comes, their pages stating no rule, and are swept with 115.
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn req_core_114_mapper_114_disables_the_irq_on_a_zero_latch_and_115_does_not() {
    for (mapper, expected) in [
        (114usize, 1),
        (182, 1),
        (115, 3),
        (121, 3),
        (123, 3),
        (187, 3),
    ] {
        let mut core = MapperRom::new(mapper).prg(16).chr(64).load();
        core.write_memory(0x2001, 0x00); // No rendering: only the rises below clock it.
        core.mapper_write(0xC000, 0x00); // Latch $00 -- the MMC3's own address on every board
        core.mapper_write(0xC001, 0x00); // of the six, because no scramble row moves $C00x
        core.mapper_write(0xE001, 0x00); // except 114's, and 114's sends $C000 to $8001.

        // 114 and 182 scramble $C000/$C001/$E001 away, so the three writes above have to be made
        // at the addresses that *mean* them.
        if matches!(mapper, 114 | 182) {
            core.mapper_write(0xA001, 0x00); // means $C000: the latch.
            core.mapper_write(0xC001, 0x00); // means $C001: the reload.
            core.mapper_write(0xE001, 0x00); // means $E001: enable.
        }

        let mut irqs = 0;
        for i in 0..3i64 {
            {
                let cart = core.cartridge_mut().unwrap();
                cart.on_ppu_address_bus(0x0000, 10_000 + i * 100);
                cart.on_ppu_address_bus(0x1000, 10_000 + i * 100 + 50);
            }

            if core.cartridge().unwrap().irq_asserted() {
                irqs += 1;
                // Acknowledge and re-enable, at the addresses that mean $E000 and $E001 -- which
                // no scramble row moves.
                core.mapper_write(0xE000, 0x00);
                core.mapper_write(0xE001, 0x00);
            }
        }

        assert_eq!(
            expected, irqs,
            "mapper {mapper}: a latch of $00 must give {expected} IRQ(s) over three rises"
        );
    }
}

/// nesdev ("INES Mapper 121"), the A9711's protection: "Returns the value from the four-byte
/// protection array (`$83, $83, $42, $00`) indexed by the previous write to `$5000-$5FFF`", and
/// 187's: "*The King of Fighters '96* reads from here and only expects bit 7 of the value being
/// returned to be set."
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn req_core_100_mappers_121_and_187_answer_their_protection_reads() {
    let mut core = MapperRom::new(121).prg(16).chr(64).load();
    for (index, expected) in [(0u8, 0x83u8), (1, 0x83), (2, 0x42), (3, 0x00)] {
        core.mapper_write(0x5000, index);
        assert_eq!(
            expected,
            core.read_memory(0x5000),
            "121: index {index} of the protection array"
        );
        // "Mask: probably $F000": the whole of $5000-$5FFF answers.
        assert_eq!(expected, core.read_memory(0x5ABC), "121: and at $5ABC");
    }

    // The index is two bits wide (`[.... ..II]`), so bits 2-7 of the written byte are not it.
    core.mapper_write(0x5000, 0xFE);
    assert_eq!(
        0x42,
        core.read_memory(0x5000),
        "121: only bits 0-1 are the index"
    );

    let core = MapperRom::new(187).prg(16).chr(64).load();
    let value = core.read_memory(0x5123);
    assert_ne!(
        0,
        value & 0x80,
        "187: KOF '96 only expects bit 7 of the protection read to be set, and it must be"
    );
}

/// nesdev ("INES Mapper 121"), the A9711's `$8003` protection table and its `$8001` latch: the
/// value written to `$8001` "with the lower six bits reversed" banks `$E000`, `$C000` or `$A000`
/// according to the index written to `$8003`, and "If an invalid index has been selected, the
/// `$A000`/`$C000`/`$E000` banks selected via `$8003` expire, and normal MMC3 PRG-ROM banking is
/// restored."
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn req_core_100_mapper_121_banks_prg_from_bit_reversed_8001_values() {
    let mut core = MapperRom::new(121).prg(32).chr(64).load();

    // The MMC3's own layout first, so that the expiry at the end has something to restore.
    core.mapper_write(0x8000, 0x06);
    core.mapper_write(0x8001, 0x09);
    core.mapper_write(0x8000, 0x07);
    core.mapper_write(0x8001, 0x0A);
    assert_eq!([9, 10, 30, 31], core.prg_windows(), "the MMC3's own layout");

    // `$8001` = $0C is `001100` in six bits, reversed `001100` -> $0C again, which is a poor
    // witness; $02 is `000010`, reversed `010000` = $10 = bank 16.
    // Index $26 banks $E000 (window 3).
    core.mapper_write(0x8003, 0x26);
    core.mapper_write(0x8001, 0x02);
    assert_eq!(
        16,
        core.prg_windows()[3],
        "index $26: $E000 takes the bit-reversed $8001 value"
    );

    // "apply subsequent writes to $8001 to this index as well": another $8001 write moves it
    // without touching $8003.
    core.mapper_write(0x8001, 0x04); // `000100` -> `001000` = $08.
    assert_eq!(8, core.prg_windows()[3], "index $26 is sticky on $8001");

    // Index $28 banks $C000 (window 2) and $2A banks $A000 (window 1).
    core.mapper_write(0x8003, 0x28);
    core.mapper_write(0x8001, 0x02);
    assert_eq!(16, core.prg_windows()[2], "index $28: $C000");
    core.mapper_write(0x8003, 0x2A);
    core.mapper_write(0x8001, 0x02);
    assert_eq!(16, core.prg_windows()[1], "index $2A: $A000");

    // "$2F: Do nothing, but keep the $A000/$C000/$E000 banks previously selected via $8003."
    let kept = core.prg_windows();
    core.mapper_write(0x8003, 0x2F);
    core.mapper_write(0x8001, 0x3F);
    assert_eq!(
        kept,
        core.prg_windows(),
        "index $2F keeps what was selected"
    );

    // "All other indices: Restore normal MMC3 PRG-ROM banking." The MMC3's own registers have
    // gone on latching underneath -- $8003 is also a $8000 write, so the index it last wrote is
    // the MMC3's select too, and the $8001 writes above landed in whichever register that named.
    core.mapper_write(0x8003, 0x00);
    let restored = core.prg_windows();
    assert_ne!(
        kept, restored,
        "an invalid index must let the MMC3's own banking back"
    );
    assert_eq!(
        [30, 31],
        [restored[2], restored[3]],
        "and the MMC3's two fixed windows are the image's last two again"
    );
}

/// nesdev ("INES Mapper 114"), the page's own observation about 182's games: "Because Hosenkan's
/// games make no use of the `$6000-$6001` registers, other than **writing zero to both of them
/// on startup**, some implementations (such as the `EverDrive` N8's) do not emulate the `$600x`
/// registers."
///
/// So the board has to tolerate that pair of writes and nothing else: a zero at `$6000` leaves
/// bit 7 clear, which is "Use PRG bank from MMC3", and a zero at `$6001` leaves CHR A18 clear.
/// The census lists this as a test in so many words.
#[cfg(feature = "board-mmc3-scramble")]
#[test]
fn req_core_100_mapper_182_tolerates_the_zero_its_games_write_at_startup() {
    let mut core = MapperRom::new(182).prg(16).chr(64).load();
    for register in 0..8u8 {
        scrambled_mmc3_bank(&mut core, 182, false, register, register + 1);
    }
    let before = (core.prg_windows(), core.chr_windows());

    core.mapper_write(0x6000, 0x00);
    core.mapper_write(0x6001, 0x00);

    assert_eq!(
        before,
        (core.prg_windows(), core.chr_windows()),
        "the zero Hosenkan's games write at startup must change nothing"
    );
}

// ------------------------------------------- the other four of mapper 119's CHR-RAM family
// ------------------------------------------- (74, 192, 194, 195), MF-UNL-2 step 2

/// The `First RAM bank` and `Size of RAM` rows of nesdev's *MMC3 with CHR ROM and CHR RAM*,
/// which is the whole of what 74, 192 and 194 add to the MMC3: 74 "redirect[s] 1 KiB CHR-ROM
/// banks #8 and #9 to 2 KiB of CHR-RAM", 192 is "effectively the same as INES Mapper 074 but
/// with twice as much CHR RAM" so "CHR Pages $08-$0B are CHR-RAM", and 194's "CHR Pages
/// $00-$01 are CHR-RAM, other pages are CHR-ROM".
///
/// The claim is made at **both edges** — the bank one below `FIRST` and the bank one above
/// `LAST` read the CHR-ROM and swallow a write, every bank in between reads a writable RAM —
/// and then on the `RAM mirrored? **No**` column, which is what separates these three from the
/// TQROM: the bank `FIRST + SIZE` is past the window and so is **ROM**, where a single-line
/// select would have caught it.
#[cfg(feature = "board-mmc3-chrram")]
#[test]
fn req_core_100_the_chr_ram_family_puts_its_own_window_on_the_ram_chip() {
    for (mapper, first, last, banks) in [
        (74usize, 0x08u8, 0x09u8, 2u8),
        (192, 0x08, 0x0B, 4),
        (194, 0x00, 0x01, 2),
    ] {
        let mut core = MapperRom::new(mapper).prg(16).chr(64).load();

        // Every bank of the window is RAM and keeps what is written to it. R2 is at PPU
        // $1000-$13FF in CHR mode 0, and `$C0 | bank` is outside the byte space of a 64-bank
        // numbered CHR-ROM, so a read can name the chip.
        for bank in first..=last {
            core.mmc3_bank(2, bank);
            chr_write(&mut core, 0x1000, 0xC0 | bank);
            assert_eq!(
                0xC0 | bank,
                peek(&core, 0x1000),
                "mapper {mapper}: bank {bank:#04X} is the RAM chip"
            );
        }

        // One below the window and one above it are the CHR-ROM, whose numbered banks read
        // their own number, and a write to either goes nowhere.
        for bank in [first.wrapping_sub(1), last + 1] {
            core.mmc3_bank(2, bank);
            chr_write(&mut core, 0x1000, 0x5A);
            assert_eq!(
                bank % 64,
                peek(&core, 0x1000),
                "mapper {mapper}: bank {bank:#04X} is the CHR-ROM and swallows the write"
            );
        }

        // `RAM mirrored? No`: `FIRST + SIZE` has the same low bits as `FIRST` -- the chip's own
        // lines -- and is still the ROM, because the select is a range and not a line.
        let outside = first + banks;
        core.mmc3_bank(2, outside);
        assert_eq!(
            outside % 64,
            peek(&core, 0x1000),
            "mapper {mapper}: bank {outside:#04X} is past the window, so the ROM"
        );

        // The board's own chip, and not REQ-CORE-012's 32 KiB default: byte 5 declares 64 KiB
        // of CHR-ROM, so that default does not apply, and no header field can state this.
        assert_eq!(
            usize::from(banks) * 1024,
            core.cartridge().unwrap().memory().chr_ram().len(),
            "mapper {mapper}: the chip of its own row"
        );
    }
}

/// Writes `value` to mapper 195's `CHRB` register, which is a PPU write to a window the
/// CHR-**ROM** drives: "The games write, via the PPU, to specific CHR banks in order to select
/// both which and how many CHR banks are mapped to RAM instead of ROM. **Writes to CHR ROM** in
/// multiples of 1 KiB" (nesdev, "INES Mapper 195").
///
/// Bank `$3F` is in no row of the page's table, so it is a ROM window whatever the register
/// holds.
#[cfg(feature = "board-mmc3-chrram")]
fn fs303_chrb(core: &mut NesCore, value: u8) {
    core.mmc3_bank(2, 0x3F);
    chr_write(core, 0x1000, value);
}

/// Reads the byte 1 KiB CHR bank `bank` shows with `CHRB` set to `register`.
#[cfg(feature = "board-mmc3-chrram")]
fn fs303_read(core: &mut NesCore, register: u8, bank: u8) -> u8 {
    fs303_chrb(core, register);
    core.mmc3_bank(2, bank);
    peek(core, 0x1000)
}

/// nesdev ("INES Mapper 195"), the Waixing FS303's GAL — the register that moves the CHR-RAM
/// window at run time, over **all 256 values** of it and all 256 bank numbers.
///
/// The decode is one three-bit field, `(Z, L, L)` = bits 6, 3 and 1, which is what the page's
/// own diagram draws and what makes its list `$80, $82, $88, $8A, $C0, $C2, $C8, $CA`
/// consecutive. So bits 5, 2 and 0 are don't-cares and every row is reached by eight byte
/// values; bit 4 (`D`) is "ignore above and always enable CHR ROM"; and bit 7 "Must be 1", so a
/// byte without it is not a register value at all.
///
/// The eight banks of the chip are filled with `$E0 | k` first, through the two rows that cover
/// them — which is also the page's worked example — so one read names the chip *and* the offset:
/// a 64-bank CHR-ROM reads `bank % 64`, below `$40`, and nothing else can read `$E0 | k`.
#[cfg(feature = "board-mmc3-chrram")]
#[test]
fn req_core_100_mapper_195_gal_moves_the_chr_ram_window_at_run_time() {
    let mut core = MapperRom::new(195).prg(16).chr(64).load();

    // The chip is 8 KiB -- "The PCB mounts 32 KiB of CHR-RAM, although address lines for only
    // 8 KiB are connected" -- and not the 4 KiB of the census's `(0x00, 0x03, 4)`.
    assert_eq!(
        0x2000,
        core.cartridge().unwrap().memory().chr_ram().len(),
        "the 8 KiB the FS303's address lines reach"
    );

    // Power-on is `$80`, so banks $28-$2B are already the RAM chip before any register write
    // (Q-142: the census's `(0x00, 0x03, 4)` is the `$82` row, not this one).
    core.mmc3_bank(2, 0x28);
    chr_write(&mut core, 0x1000, 0xE0);
    assert_eq!(
        0xE0,
        peek(&core, 0x1000),
        "power-on $80: bank $28 is the RAM chip"
    );

    // Fill the chip: `$80` reaches its banks 0-3 through $28-$2B and `$88` its banks 4-7
    // through $4C-$4F, the offset being the low three bits of the bank number.
    for (register, first) in [(0x80u8, 0x28u8), (0x88, 0x4C)] {
        fs303_chrb(&mut core, register);
        for i in 0..4u8 {
            core.mmc3_bank(2, first + i);
            chr_write(&mut core, 0x1000, 0xE0 | ((first + i) & 0x07));
        }
    }

    // The page's table, by `(Z, L, L)`.
    let row_window = |row: u8| -> Option<(u8, u8)> {
        match row {
            0 => Some((0x28, 0x2B)),
            1 => Some((0x00, 0x03)),
            2 => Some((0x4C, 0x4F)),
            3 => Some((0x64, 0x67)),
            4 => Some((0x46, 0x47)),
            5 => Some((0x7C, 0x7D)),
            6 => Some((0x0A, 0x0B)),
            _ => None,
        }
    };

    for value in 0x80..=0xFFu8 {
        let window = if value & 0x10 == 0 {
            row_window(((value >> 4) & 0x04) | ((value >> 2) & 0x02) | ((value >> 1) & 0x01))
        } else {
            None
        };

        fs303_chrb(&mut core, value);
        for bank in 0..=0xFFu8 {
            core.mmc3_bank(2, bank);
            let read = peek(&core, 0x1000);
            if window.is_some_and(|(first, last)| bank >= first && bank <= last) {
                assert_eq!(
                    0xE0 | (bank & 0x07),
                    read,
                    "195 CHRB={value:#04X} bank {bank:#04X}: the RAM chip"
                );
            } else {
                assert_eq!(
                    bank % 64,
                    read,
                    "195 CHRB={value:#04X} bank {bank:#04X}: the CHR-ROM"
                );
            }
        }
    }

    // "Must be 1": a byte without bit 7 is not a `CHRB` value, so the window stays where the
    // last real write left it.
    fs303_chrb(&mut core, 0x82);
    fs303_chrb(&mut core, 0x00);
    core.mmc3_bank(2, 0x00);
    assert_eq!(
        0xE0,
        peek(&core, 0x1000),
        "a write with bit 7 clear leaves the $82 window in place"
    );

    // The page's own worked example, which is why `SIZE` is 8 and not 4: "setting $80 ($28-$2B)
    // will select different 4 KiB of CHR-RAM from setting $88 ($4C-$4F), but the same 4 KiB as
    // setting $82 ($00-$03), because CHR A13 is not connected to CHR-RAM."
    assert_eq!(
        fs303_read(&mut core, 0x80, 0x28),
        fs303_read(&mut core, 0x82, 0x00),
        "$82 ($00-$03) is the same 4 KiB of the chip as $80 ($28-$2B)"
    );
    assert_ne!(
        fs303_read(&mut core, 0x80, 0x28),
        fs303_read(&mut core, 0x88, 0x4C),
        "$88 ($4C-$4F) is a different 4 KiB"
    );
    assert_eq!(
        fs303_read(&mut core, 0x88, 0x4C),
        fs303_read(&mut core, 0x8A, 0x64),
        "$8A ($64-$67) aliases onto $88's 4 KiB, three address lines being all there are"
    );
}

/// The four numbers are an MMC3 in everything but the CHR source, so each one's A12 scanline
/// counter and `$A001` PRG-RAM protection are the MMC3's — and `IrqBoard`'s entry condition is
/// that they answer *yes* to [`Mapper::irq_asserted`].
///
/// 195's own page names one more thing: "A compatible circuit board (with unknown PCB code) adds
/// 4 KiB of non-battery-backed PRG-RAM (recognizable as such in the NES 2.0 header) mapped to CPU
/// $5000-$5FFF" — *天神之剑*. That window **was** the one thing of the page this build did not
/// carry; MF-FIX added it behind `board-195-prg-ram-5000` once the cross-check found FCEUX and
/// puNES both mapping it unconditionally (CHANGE 2, Q-173), and
/// `q_173_mapper_195_has_4_kib_of_prg_ram_at_5000` is its test. This one pins the rest of the
/// board, `$A001` included — and the `$6000` assertions below are what prove that putting the new
/// chip at the head of the allocation left the WRAM window where it was.
#[cfg(feature = "board-mmc3-chrram")]
#[test]
fn req_core_114_the_chr_ram_family_keeps_the_mmc3s_scanline_counter() {
    for mapper in [74usize, 192, 194, 195] {
        let mut core = MapperRom::new(mapper).prg(16).chr(64).load();

        // Latch 4, reload, enable: the counter is the MMC3's, so it asserts on the fifth A12
        // rise and not before. The rises are driven by hand with rendering off and from a clock
        // well past the register writes', which is the idiom `mr1_review_tests` already uses --
        // the A12 watcher filters a rise that is too close to the last one.
        core.write_memory(0x2001, 0x00);
        core.mapper_write(0xC000, 4);
        core.mapper_write(0xC001, 0);
        core.mapper_write(0xE001, 0);

        let cartridge = core.cartridge_mut().unwrap();
        for rise in 1..=5i64 {
            cartridge.on_ppu_address_bus(0x0000, 10_000 + rise * 100);
            cartridge.on_ppu_address_bus(0x1000, 10_000 + rise * 100 + 50);
            assert_eq!(
                rise == 5,
                cartridge.irq_asserted(),
                "mapper {mapper}: rise {rise}"
            );
        }

        // $A001: bit 7 enables the RAM, bit 6 write-protects it, exactly as on mapper 4 --
        // read off the RAM itself, which is the header's 8 KiB at $6000-$7FFF because no page
        // of the four states a size of its own.
        core.mapper_write(0xA001, 0x80);
        core.mapper_write(0x6000, 0x5A);
        assert_eq!(
            0x5A,
            core.read_memory(0x6000),
            "mapper {mapper}: $A001 = $80"
        );
        core.mapper_write(0xA001, 0xC0);
        core.mapper_write(0x6000, 0x33);
        assert_eq!(
            0x5A,
            core.read_memory(0x6000),
            "mapper {mapper}: $A001 = $C0 write-protects"
        );
    }
}

/// **Q-173**, MF-FIX / CHANGE 2 of `docs/review/Q-XCHECK.md`: nesdev ("INES Mapper 195") gives the
/// FS303 a compatible board that "**adds** 4 KiB of non-battery-backed PRG-RAM (recognizable as
/// such in the NES 2.0 header) mapped to CPU `$5000-$5FFF`", used by *天神之剑* — Waixing's
/// translation of *Crystalis*. This build did not carry the window; the cross-check found FCEUX
/// (`mmc3.cpp:1036-1041`) and puNES (`mapper_195.c:134-137`) both mapping it on **every**
/// mapper-195 image, and both putting it *past* the 8 KiB that `$6000-$7FFF` shows.
///
/// The four things this pins, in order:
///
/// 1. the window reads and writes, over the whole 4 KiB, with CPU A0-A11 straight through;
/// 2. it is **a chip of its own** — the page's "adds" — so no byte of it is visible through
///    `$6000-$7FFF` and no byte of the WRAM is visible through `$5000-$5FFF`;
/// 3. the MMC3's `$A001` does **not** gate it, which is what makes it usable from reset and what
///    both implementations do;
/// 4. the other three numbers of the family have no such window, which is the discriminant for
///    the per-variant test inside the board.
#[cfg(feature = "board-195-prg-ram-5000")]
#[test]
fn q_173_mapper_195_has_4_kib_of_prg_ram_at_5000() {
    let mut core = MapperRom::new(195usize).prg(16).chr(64).load();

    // (1) The whole 4 KiB takes writes and gives them back, with A0-A11 straight through: the
    // first, the last and one in the middle, and $5000 and $5FFF must not be the same byte.
    for (address, value) in [(0x5000u16, 0x11u8), (0x5ABC, 0x22), (0x5FFF, 0x33)] {
        core.write_memory(address, value);
    }
    for (address, value) in [(0x5000u16, 0x11u8), (0x5ABC, 0x22), (0x5FFF, 0x33)] {
        assert_eq!(
            value,
            core.read_memory(address),
            "$5000-$5FFF is 4 KiB of RAM, A0-A11 straight through ({address:#06X})"
        );
    }

    // (2) A chip of its own. $A001 enables the WRAM, and then neither window shows the other's
    // bytes: the extra chip is the head of the allocation and $6000-$7FFF starts behind it.
    core.mapper_write(0xA001, 0x80);
    core.write_memory(0x6000, 0x44);
    core.write_memory(0x6ABC, 0x55);
    core.write_memory(0x7FFF, 0x66);
    assert_eq!(
        [0x11, 0x22, 0x33],
        [
            core.read_memory(0x5000),
            core.read_memory(0x5ABC),
            core.read_memory(0x5FFF)
        ],
        "the WRAM writes must not have reached the $5000 chip"
    );
    assert_eq!(
        [0x44, 0x55, 0x66],
        [
            core.read_memory(0x6000),
            core.read_memory(0x6ABC),
            core.read_memory(0x7FFF)
        ],
        "and the WRAM window is unmoved: 8 KiB, its own bytes"
    );

    // (3) $A001 does not gate the extra chip. Bit 7 clear disables the WRAM and bit 6 write-
    // protects it; the $5000 window goes on reading and writing through both.
    core.mapper_write(0xA001, 0xC0); // Enabled, write-protected.
    core.write_memory(0x5ABC, 0x77);
    assert_eq!(
        0x77,
        core.read_memory(0x5ABC),
        "$A001 bit 6 write-protects the WRAM, not the compatible board's chip"
    );
    core.mapper_write(0xA001, 0x00); // Disabled.
    core.write_memory(0x5ABC, 0x88);
    assert_eq!(
        0x88,
        core.read_memory(0x5ABC),
        "$A001 bit 7 disables the WRAM, not the compatible board's chip"
    );

    // (4) The family's other three numbers have no window there: a write is a register write
    // their boards do not decode, and the read is not RAM.
    for mapper in [74usize, 192, 194] {
        let mut core = MapperRom::new(mapper).prg(16).chr(64).load();
        core.mapper_write(0xA001, 0x80);
        core.write_memory(0x5ABC, 0x99);
        assert_ne!(
            0x99,
            core.read_memory(0x5ABC),
            "mapper {mapper}: only 195's compatible board has the $5000 window"
        );
    }
}

/// nesdev ("INES Mapper 047"): $6000-$7FFF is `[.... ...B]`, "each block has 128k PRG and 128k
/// CHR", and $8000-$FFFF is the "Same as MMC3 for selected block".
///
/// So `B` is PRG A17 above the MMC3's four bits of 8 KiB bank and CHR A17 above its seven bits
/// of 1 KiB bank, the fixed `-1` and `-2` windows become the **block's** last two rather than
/// the image's, and a bank number wraps inside the block.
#[cfg(feature = "board-mmc3-multicart")]
#[test]
fn req_core_100_mapper_47_switches_128_kib_blocks_of_prg_and_chr() {
    let mut core = MapperRom::new(47).prg(32).chr(256).load();

    // Block 0. Power-on is R6 = 0 and R7 = 1, and the two fixed windows are the block's last.
    assert_eq!([0, 1, 14, 15], core.prg_windows(), "block 0 at power-on");
    core.write_memory(0x6000, 0x01);
    assert_eq!(
        [16, 17, 30, 31],
        core.prg_windows(),
        "block 1: every window, the two fixed ones included, moves by 128 KiB"
    );

    // A bank number wraps **inside** the block: $13 is bank 3 of whichever block is selected,
    // never bank 19 of the image.
    core.mmc3_bank(6, 0x13);
    assert_eq!(
        [19, 17, 30, 31],
        core.prg_windows(),
        "block 1, inner bank 3"
    );
    core.write_memory(0x6000, 0x00);
    assert_eq!([3, 1, 14, 15], core.prg_windows(), "block 0, inner bank 3");

    // CHR: the same bit is A17 above the MMC3's seven, on a 2 KiB register and a 1 KiB one.
    core.mmc3_bank(0, 0x02);
    core.mmc3_bank(1, 0x08);
    core.mmc3_bank(2, 0x14);
    core.mmc3_bank(3, 0x15);
    core.mmc3_bank(4, 0x16);
    core.mmc3_bank(5, 0xA0); // Bit 7 is the MMC3's own, so it wraps inside the block too.
    assert_eq!([2, 3, 8, 9, 20, 21, 22, 32], core.chr_windows(), "block 0");
    core.write_memory(0x6000, 0x01);
    assert_eq!(
        [130, 131, 136, 137, 148, 149, 150, 160],
        core.chr_windows(),
        "block 1: CHR A17 comes from the outer register"
    );
}

/// nesdev ("INES Mapper 037") gives the wiring rather than a mask: "PRG ROM A16 =
/// NAND(NAND(Q0,Q1),NAND(Q2,M16))" with "PRG ROM A17 ... driven by Q2", `M16` being the MMC3's
/// own A16. That NAND of two NANDs is `A16 = (Q0 AND Q1) OR (Q2 AND M16)`, and **one expression
/// reproduces the page's whole four-row table** -- the two 64 KiB windows and the two 128 KiB
/// ones. This test is that table, all eight values against the MMC3's A16 clear and set.
///
/// The two places the page and Mesen 2 disagree are both here, and AMD-061 section 1 settles
/// them for the page: block 7 is $30000-$3FFFF where `MMC3_37.h` puts it at $20000-$27FFF, and
/// the MMC3's own CHR bit 7 never reaches the CHR-ROM where Mesen masks nothing.
#[cfg(feature = "board-mmc3-multicart")]
#[test]
fn req_core_100_mapper_37_reproduces_its_outer_bank_table() {
    // The page's table, as the first PRG window's 8 KiB bank number: `M16` is bit 3 of R6.
    for (m16, banks) in [
        (0x00u8, [0u8, 0, 0, 8, 16, 16, 16, 24]),
        (0x08, [0, 0, 0, 8, 24, 24, 24, 24]),
    ] {
        let mut core = MapperRom::new(37).prg(32).chr(256).load();
        core.mmc3_bank(6, m16);
        for (written, expected) in banks.iter().copied().enumerate() {
            core.write_memory(0x6000, u8::try_from(written).unwrap());
            assert_eq!(
                expected,
                core.prg_windows()[0],
                "R6 = {m16:#04X}, $6000 = {written}"
            );
        }
    }

    // Bits 3-7 are not latched: $FB is block 3 and nothing else.
    let mut core = MapperRom::new(37).prg(32).chr(256).load();
    core.write_memory(0x6000, 0xFB);
    assert_eq!(8, core.prg_windows()[0], "bits 3-7 are not latched");

    // CHR A17 is **driven** by Q2, so the MMC3's own bit 7 never reaches the CHR-ROM.
    core.write_memory(0x6000, 0x00);
    core.mmc3_bank(2, 0x20);
    assert_eq!(32, core.chr_windows()[4], "Q2 clear: CHR A17 clear");
    core.mmc3_bank(2, 0xA0);
    assert_eq!(
        32,
        core.chr_windows()[4],
        "the MMC3's bit 7 is dropped, not ORed in"
    );
    core.write_memory(0x6000, 0x04);
    assert_eq!(160, core.chr_windows()[4], "Q2 set: CHR A17 set");
}

/// The condition both multicart pages put on the outer register, and section 5 of the
/// inventory's own "easy thing to miss": it is "only writable when MMC3 PRG-RAM is enabled and
/// writable (see $A001)" (047) and "you will need to enable writes to PRG-RAM" (037).
///
/// 037's page adds that the register "is not readable and attempting to read it will return open
/// bus", which is what `prg_ram: Some(0)` gives: with no RAM behind the window,
/// `Cartridge::default_cpu_read` falls through to `Mapper::try_read_prg`, which answers `None`
/// below $8000.
#[cfg(feature = "board-mmc3-multicart")]
#[test]
fn req_core_100_the_mmc3_multicart_outer_register_needs_prg_ram_enabled_and_writable() {
    for mapper in [47usize, 37] {
        let mut core = MapperRom::new(mapper).prg(32).chr(256).load();
        // The value that selects the second 128 KiB of PRG on each number: `B` on 47, `Q` on 37.
        let outer = if mapper == 47 { 0x01 } else { 0x04 };

        // $A001 powers on at $80 -- enabled and writable (MR-4-5) -- so the register takes.
        core.write_memory(0x6000, outer);
        assert_eq!(
            16,
            core.prg_windows()[0],
            "mapper {mapper}: $A001 = $80 at power-on, the register takes"
        );

        // $C0: enabled but write-protected.
        core.mapper_write(0xA001, 0xC0);
        core.write_memory(0x6000, 0x00);
        assert_eq!(
            16,
            core.prg_windows()[0],
            "mapper {mapper}: $A001 = $C0 is write-protected"
        );

        // $00: the chip is disabled altogether.
        core.mapper_write(0xA001, 0x00);
        core.write_memory(0x6000, 0x00);
        assert_eq!(
            16,
            core.prg_windows()[0],
            "mapper {mapper}: $A001 = $00 is disabled"
        );

        // And $80 again lets it through.
        core.mapper_write(0xA001, 0x80);
        core.write_memory(0x6000, 0x00);
        assert_eq!(
            0,
            core.prg_windows()[0],
            "mapper {mapper}: $A001 = $80 again, and the register takes"
        );

        // The window is not RAM and not readable: the register *is* the board's decode of it.
        core.write_memory(0x6000, 0x5A);
        assert_ne!(
            0x5A,
            core.read_memory(0x6000),
            "mapper {mapper}: $6000-$7FFF reads open bus, not the register"
        );
    }
}

/// The non-regression the three boards exist to not break: an ordinary mapper-4 image behaves
/// exactly as it did before they were added.
///
/// A write to $6000 is PRG-RAM and not an outer register; CHR bank $A0 is bank $A0 and not bank
/// $20 of a block; the four PRG windows span the whole image rather than one 128 KiB block of
/// it; and mapper 4 still offers a `chr_peek_view`, which the TQROM cannot.
#[test]
fn req_core_100_an_ordinary_mapper_4_image_is_unaffected_by_119_47_and_37() {
    let mut core = MapperRom::new(4).prg(32).chr(256).battery().load();

    // $6000-$7FFF is the MMC3's own PRG-RAM, and writing there moves no bank.
    let before = core.prg_windows();
    core.write_memory(0x6000, 0x01);
    assert_eq!(before, core.prg_windows(), "no outer register on mapper 4");
    assert_eq!(0x01, core.read_memory(0x6000), "the PRG-RAM is still there");

    // The whole image, not a block of it: the two fixed windows are the image's last two.
    assert_eq!([0, 1, 30, 31], core.prg_windows());
    core.mmc3_bank(6, 0x13);
    assert_eq!(
        [19, 1, 30, 31],
        core.prg_windows(),
        "bank $13 is bank 19 of the image, not bank 3 of a block"
    );

    // Seven bits of CHR bank, all of them the MMC3's own.
    core.mmc3_bank(2, 0xA0);
    assert_eq!(
        160,
        core.chr_windows()[4],
        "CHR bank $A0 is bank $A0, not bank $20 of a block"
    );

    // And the MMC3's peek is still a view: only its fetches clock the IRQ counter, so the
    // pattern tables can be read as one slice plus eight offsets.
    assert!(
        core.cartridge().unwrap().chr_peek_view().is_some(),
        "mapper 4 still offers a chr_peek_view"
    );
}

// ------------------------------------------- the Gouder SOMARI-P (116), MF-6a3 step 4

/// Writes the supervisor register at `$4100`.
#[cfg(feature = "board-116")]
fn somari_mode(core: &mut NesCore, value: u8) {
    core.mapper_write(0x4100, value);
}

/// The eight 1 KiB CHR window offsets the active chip shows.
///
/// Read through the view and not through [`MapperMachine::chr_windows`], because the fixture
/// fills CHR bank N with `N as u8` and the outer bank of mapper 116 moves a window by **256**
/// banks: the two would read the same byte. The offsets distinguish them.
///
/// Mappers 12 and 14 use it for the same reason: their CHR A18 is the same 256-bank move, applied
/// per window group. So do mapper 83.3, whose `CC` bits are CHR A19-A18, and mapper 91's submapper
/// 0, whose outer register drives CHR A19 (MF-UNL-8).
///
/// MF-UNL-11's three boards use it too: the UNL-8237's outer CHR bank is 256 banks and its CHR A17
/// 128, and the A9461's outer bank is 128 (step 4).
#[cfg(any(
    feature = "board-116",
    feature = "board-91",
    feature = "board-cony",
    feature = "board-huang1",
    feature = "board-208",
    feature = "board-215",
    feature = "board-219",
    feature = "board-246",
    feature = "board-262",
    feature = "board-292"
))]
fn chr_offsets(core: &NesCore) -> [usize; 8] {
    core.cartridge().unwrap().chr_peek_view().unwrap().offsets()
}

/// nesdev ("INES Mapper 116"): the supervisor register is at `$4100` with "Mask: $E100", holding
/// `[.... .CMM]`, and the board powers on at **`$01`, MMC3 mode**.
///
/// The register is below `$8000`, which `Cartridge::cpu_write` reaches for all of `$4020-$FFFF`,
/// so `Mapper::write_register`'s usual `address < 0x8000` guard must not be this board's first
/// line — a board that returned early there could never be switched out of MMC3 mode at all,
/// which is what every mirror below would show.
#[cfg(feature = "board-116")]
#[test]
fn req_core_100_the_somari_supervisor_register_is_at_4100_under_an_e100_mask() {
    // Power-on: MMC3 mode. R6 = 0, R7 = 1 with the last two banks fixed is the MMC3's own
    // power-on layout, and nothing else on this board gives it.
    let mut core = MapperRom::new(116).prg(32).chr(128).load();
    assert_eq!([0, 1, 30, 31], core.prg_windows(), "power-on is MMC3 mode");

    // Every mirror the $E100 mask admits reaches the register: A8 set and A13-A15 = %010, with
    // A0-A7 and A9-A12 undecoded.
    for address in [0x4100u16, 0x4101, 0x41FF, 0x4300, 0x5100, 0x5300, 0x5FFF] {
        let mut probe = MapperRom::new(116).prg(32).chr(128).load();
        probe.mapper_write(address, 0x00); // VRC2 mode.
                                           // $8000 is the VRC2's first 8 KiB PRG register; on the MMC3 it is the bank select and
                                           // moves nothing at all.
        probe.mapper_write(0x8000, 0x05);
        assert_eq!(
            [5, 1, 30, 31],
            probe.prg_windows(),
            "{address:#06X} is the supervisor register"
        );
    }

    // And an address the mask excludes is not it: A8 clear.
    core.mapper_write(0x4000, 0x00);
    core.mapper_write(0x8000, 0x05);
    assert_eq!(
        [0, 1, 30, 31],
        core.prg_windows(),
        "$4000 is not the supervisor register, so this is still an MMC3"
    );
}

/// "Three chips each keeping its own registers, so that when switching between modes using the
/// supervisor register, the entire mapper state changes, including PRG/CHR-ROM switchable and
/// fixed banks as well as IRQ state."
///
/// So each chip's registers must survive a round trip through the other two: set a layout only
/// that chip could produce, walk away through both others, come back and find it unchanged.
#[cfg(feature = "board-116")]
#[test]
fn req_core_100_the_somari_switches_the_whole_mapper_state_between_its_three_chips() {
    let mut core = MapperRom::new(116).prg(32).chr(128).load();

    // The MMC3: $8000/$8001 as a select-and-value pair.
    core.mmc3_bank(6, 3);
    core.mmc3_bank(7, 5);
    assert_eq!([3, 5, 30, 31], core.prg_windows(), "the MMC3's own layout");

    // The VRC2: two 8 KiB registers at $8000 and $A000, nothing like the MMC3's pair.
    somari_mode(&mut core, 0x00);
    core.mapper_write(0x8000, 0x07);
    core.mapper_write(0xA000, 0x09);
    assert_eq!([7, 9, 30, 31], core.prg_windows(), "the VRC2's own layout");

    // The MMC1: 16 KiB banking through the five-bit serial shift register.
    somari_mode(&mut core, 0x02);
    core.mmc1_write(0x8000, 0x0C); // PRG mode 3, 8 KiB CHR.
    core.mmc1_write(0xE000, 0x05);
    assert_eq!(
        [10, 11, 30, 31],
        core.prg_windows(),
        "the MMC1's own layout"
    );

    // And all three are still there, mode 3 being mode 2.
    somari_mode(&mut core, 0x01);
    assert_eq!(
        [3, 5, 30, 31],
        core.prg_windows(),
        "the MMC3 kept its registers"
    );
    somari_mode(&mut core, 0x00);
    assert_eq!(
        [7, 9, 30, 31],
        core.prg_windows(),
        "the VRC2 kept its registers"
    );
    somari_mode(&mut core, 0x03);
    assert_eq!(
        [10, 11, 30, 31],
        core.prg_windows(),
        "the MMC1 kept its registers, and mode 3 is mode 2"
    );
}

/// `C` is **bit 2** of the supervisor register and selects the outer 256 KiB CHR-ROM bank, CHR
/// A18: "outer 256 KiB CHR-ROM bank". The page's own diagram puts it at bit 2 and Mesen agrees
/// (`(_mode & 0x04) << 6`); a summary of that page which says bit 3 is wrong, which is what the
/// bit-3 probe pins.
///
/// **The MMC1 mode ignores it** (Mesen's `UpdateChr` case 2/3 does not apply it), and the bank
/// must not be counted twice when a further register write arrives.
#[cfg(feature = "board-116")]
#[test]
fn req_core_100_the_somari_outer_chr_bank_is_bit_2_and_the_mmc1_mode_ignores_it() {
    const KIB: usize = 1024;
    let mut core = MapperRom::new(116).prg(32).chr(512).load();

    // MMC3 mode: R2 and R3 drive PPU $1000-$17FF with the CHR mode bit clear.
    core.mmc3_bank(2, 0x10);
    core.mmc3_bank(3, 0x11);
    let plain = chr_offsets(&core);
    assert_eq!((16 * KIB, 17 * KIB), (plain[4], plain[5]), "`C` clear");

    somari_mode(&mut core, 0x05); // MMC3 mode with `C` set.
    let outer = chr_offsets(&core);
    assert_eq!(
        (272 * KIB, 273 * KIB),
        (outer[4], outer[5]),
        "`C` is bit 2, and it is 256 KiB of CHR"
    );

    // A further register write must not add the bank a second time.
    core.mmc3_bank(2, 0x10);
    assert_eq!(
        272 * KIB,
        chr_offsets(&core)[4],
        "the outer bank is not counted twice"
    );

    // Bit 3 is not the chip select.
    somari_mode(&mut core, 0x09);
    assert_eq!(16 * KIB, chr_offsets(&core)[4], "bit 3 is not `C`");

    // The MMC1 mode ignores the bank: the same registers give the same windows either way.
    somari_mode(&mut core, 0x06); // MMC1 mode, `C` set.
    core.mmc1_write(0x8000, 0x1C); // 4 KiB CHR mode, PRG mode 3.
    core.mmc1_write(0xA000, 0x04);
    let with_c = chr_offsets(&core);
    somari_mode(&mut core, 0x02); // MMC1 mode, `C` clear.
    assert_eq!(with_c, chr_offsets(&core), "the MMC1 mode ignores `C`");
    // The MMC1's own 4 KiB CHR mode: register $04 is 4 KiB bank 4, which is 1 KiB bank 16.
    assert_eq!(
        16 * KIB,
        with_c[0],
        "and shows the bank its own register names"
    );
}

/// nesdev ("INES Mapper 116"): the VRC2 mode is "A0/A1, i.e. a `VRC2b`", and its **CHR registers
/// power up to `$FF`** — "one version of *Somari the Adventurer* relies on this", which is why
/// the power-on value is forced rather than left at the chip's own zero.
#[cfg(feature = "board-116")]
#[test]
fn req_core_100_the_somari_vrc2_mode_is_a_vrc2b_with_its_chr_registers_at_ff() {
    let mut core = MapperRom::new(116).prg(32).chr(512).load();
    somari_mode(&mut core, 0x00);

    // $FF in every one of the eight 1 KiB registers: bank 255 of a 512-bank CHR-ROM.
    assert_eq!([255u8; 8], core.chr_windows(), "the power-on $FF");

    // A VRC2b takes the register index from A1 (which half of the register) and A0 (which
    // nibble), which is what this build's mapper-23 arm computes with `index = addr & 3`. So
    // $B000 is CHR register 0's low nibble and $B001 its high nibble: $3 then $1 gives bank $13.
    core.mapper_write(0xB000, 0x03);
    core.mapper_write(0xB001, 0x01);
    assert_eq!(
        0x13,
        core.chr_bank(0x0000),
        "A0 picks the nibble and A1 the half-register: a VRC2b"
    );

    // And the VRC2 has no IRQ at all — that is the VRC4's — so $F000-$FFFF moves nothing.
    let before = core.prg_windows();
    for address in [0xF000u16, 0xF001, 0xF002, 0xF003] {
        core.mapper_write(address, 0xFF);
    }
    assert_eq!(
        before,
        core.prg_windows(),
        "no VRC4 IRQ registers on a VRC2"
    );
}

/// Submapper 1, the **SOMARI-W** PAL: it "resets the MMC1's serial shift register when switching
/// *to* MMC1 mode, equivalent to writing to $8000-$FFFF with bit 7 set", and submapper 0, the
/// **SOMARI-P**, does not.
///
/// The test leaves the shift register three bits full on purpose, walks away to the VRC2 and
/// comes back: on submapper 1 the five bits that follow are the whole of what commits, and on
/// submapper 0 the three stale bits are still in front of them, so the same writes commit
/// something else. The reading that puts the reset on submapper 1 is the one recorded in
/// [`Somari`]'s own note; no game in the sources can show the difference, because the only
/// cartridge of the other PAL "does not seem to rely on it".
#[cfg(feature = "board-116")]
#[test]
fn req_core_100_the_somari_w_pal_resets_the_mmc1_shift_register_on_the_switch() {
    let mut windows = Vec::new();
    for submapper in [0u8, 1] {
        let mut core = MapperRom::new(116)
            .prg(32)
            .chr(128)
            .submapper(submapper)
            .load();
        somari_mode(&mut core, 0x02); // MMC1 mode.
        core.mmc1_write(0x8000, 0x0C); // PRG mode 3, a clean start.

        // Three bits of a $E000 write and no more.
        for _ in 0..3 {
            core.mapper_write(0xE000, 0x01);
        }

        somari_mode(&mut core, 0x00); // away to the VRC2...
        somari_mode(&mut core, 0x02); // ...and back *to* MMC1 mode.
        core.mmc1_write(0xE000, 0x05);
        windows.push(core.prg_windows());
    }

    assert_eq!(
        [10, 11, 30, 31],
        windows[1],
        "submapper 1 reset the shift register, so $05 is the whole of what committed"
    );
    assert_ne!(
        windows[1], windows[0],
        "submapper 0 does not reset it, so the three stale bits are still in front"
    );
}

/// The non-regression: the three chips mapper 116 holds are untouched on their own numbers.
#[cfg(feature = "board-116")]
#[test]
fn req_core_100_the_somari_does_not_disturb_the_three_chips_it_holds() {
    // A mapper-4 image has no supervisor register, so $4100 moves nothing.
    let mut mmc3 = MapperRom::new(4).prg(32).chr(128).load();
    mmc3.mmc3_bank(6, 3);
    let before = mmc3.prg_windows();
    mmc3.mapper_write(0x4100, 0x00);
    assert_eq!(
        before,
        mmc3.prg_windows(),
        "no supervisor register on mapper 4"
    );

    // A mapper-23 image is still a VRC2/VRC4 resolved from its own header, not a forced VRC2b,
    // and its CHR registers still power on at zero rather than at $FF.
    let vrc = MapperRom::new(23).prg(16).chr(32).load();
    assert_eq!(23, vrc.cartridge().unwrap().mapper_number());
    assert_eq!([0u8; 8], vrc.chr_windows(), "the VRC2's own power-on CHR");

    // And a mapper-1 image is still an MMC1, whose $4100 is nothing either.
    let mut mmc1 = MapperRom::new(1).prg(16).chr(32).load();
    mmc1.mmc1_write(0x8000, 0x0C);
    mmc1.mmc1_write(0xE000, 0x03);
    let before = mmc1.prg_windows();
    mmc1.mapper_write(0x4100, 0x00);
    assert_eq!(
        before,
        mmc1.prg_windows(),
        "no supervisor register on mapper 1"
    );
}

// ------------------------------------------- the Huang-1's other two PCBs (12, 14), MF-UNL-8

/// nesdev ("INES Mapper 012"): the GAL16V8's outer CHR register is at `$4132` with "Mask: $E100",
/// holding `...B ...A` — **bit 0** is "CHR A18 when PPU A12=0" and **bit 4** "CHR A18 when PPU
/// A12=1", so one line per pattern table.
///
/// Read through [`chr_offsets`] and not through [`MapperMachine::chr_windows`], for the reason that
/// helper gives: the fixture fills CHR bank N with `N as u8` and CHR A18 moves a window by 256
/// banks, so the two layouts would read the same byte.
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_102_mapper_12_drives_chr_a18_per_pattern_table_from_4132() {
    // 512 KiB of CHR, which is what the register exists for: "The GAL16V8 provides an outer CHR
    // bank register at $4132 to support 512 KiB of CHR-ROM".
    let mut core = MapperRom::new(12).prg(16).chr(512).load();
    // The MMC3's own power-on layout: R0/R1 as two 2 KiB banks over $0000-$0FFF and R2-R5 as four
    // 1 KiB banks over $1000-$1FFF, every register zero.
    let base: [usize; 8] = [0, 0x400, 0, 0x400, 0, 0, 0, 0];
    assert_eq!(base, chr_offsets(&core), "an MMC3 and nothing else yet");

    let a18 = 256 * 0x400;
    core.mapper_write(0x4132, 0x01);
    assert_eq!(
        [
            base[0] + a18,
            base[1] + a18,
            base[2] + a18,
            base[3] + a18,
            base[4],
            base[5],
            base[6],
            base[7]
        ],
        chr_offsets(&core),
        "bit 0 is CHR A18 for PPU $0000-$0FFF alone"
    );

    core.mapper_write(0x4132, 0x10);
    assert_eq!(
        [
            base[0],
            base[1],
            base[2],
            base[3],
            base[4] + a18,
            base[5] + a18,
            base[6] + a18,
            base[7] + a18
        ],
        chr_offsets(&core),
        "bit 4 is CHR A18 for PPU $1000-$1FFF alone"
    );

    core.mapper_write(0x4132, 0x11);
    assert_eq!(
        base.map(|offset| offset + a18),
        chr_offsets(&core),
        "both lines at once reach the second 256 KiB throughout"
    );

    // And with the register clear the second half is unreachable, which is the other half of the
    // census's test: "with $4132 clear it is not".
    core.mapper_write(0x4132, 0x00);
    assert_eq!(base, chr_offsets(&core));
    assert!(
        chr_offsets(&core).iter().all(|offset| *offset < a18),
        "no window reaches above 256 KiB with the register clear"
    );
}

/// The register's decode, and the two things the page says about *when* it acts.
///
/// "Mask: $E100" is A15, A14, A13 and A8 and nothing else, so `$4132` repeats through the whole of
/// `$4100`-`$5FFF` wherever A8 is set — and `$6100 & $E100` is `$6100`, so no address in
/// $6000-$7FFF reaches it and the board keeps the PRG-RAM window an MMC3 has.
///
/// "Because this register is implemented outside the ASIC, it is **not affected by MMC3 $8000 bit
/// 7**, and **writes take effect immediately**." Bit 7 of `$8000` swaps which of the chip's bank
/// registers drive which windows; the line is laid over the windows afterwards, so the swap moves
/// the banks and not which windows carry A18. **FCEUX's reading is deliberately not implemented**
/// — it makes the register "a temporary latch … that takes effect on the next write to an MMC3 CHR
/// bank register", and the page's own next sentence is that the homebrew *Ultimate Mortal Kombat
/// III* "relies on FCEUX' behavior and would therefore not work on the original PCB" (Q-164).
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_102_mapper_12_decodes_4132_under_e100_and_acts_at_once() {
    let a18 = 256 * 0x400;
    for address in [0x4132u16, 0x4100, 0x41FF, 0x4300, 0x5100, 0x5F32, 0x5FFF] {
        let mut core = MapperRom::new(12).prg(16).chr(512).load();
        core.mapper_write(address, 0x11);
        assert!(
            chr_offsets(&core).iter().all(|offset| *offset >= a18),
            "{address:#06X} is the outer CHR register"
        );
    }

    // A8 clear is not the register, and neither is anything in the PRG-RAM window.
    for address in [0x4032u16, 0x4232, 0x6132, 0x7132] {
        let mut core = MapperRom::new(12).prg(16).chr(512).load();
        core.mapper_write(address, 0x11);
        assert!(
            chr_offsets(&core).iter().all(|offset| *offset < a18),
            "{address:#06X} is not the outer CHR register"
        );
    }

    // Immediately, and not on the next CHR write: one write to $4132 and the windows have moved,
    // with no MMC3 register touched between.
    let mut core = MapperRom::new(12).prg(16).chr(512).load();
    core.mmc3_bank(2, 9); // R2: the 1 KiB window at $1000.
    assert_eq!(9 * 0x400, chr_offsets(&core)[4]);
    core.mapper_write(0x4132, 0x10);
    assert_eq!(
        (256 + 9) * 0x400,
        chr_offsets(&core)[4],
        "the write took effect with no CHR register written after it"
    );

    // $8000 bit 7 moves the chip's registers between the halves and leaves the lines where they
    // are: with only bit 0 set, PPU $0000-$0FFF carries A18 before and after the swap.
    core.mapper_write(0x4132, 0x01);
    let before = chr_offsets(&core);
    assert!(before[..4].iter().all(|offset| *offset >= a18));
    assert!(before[4..].iter().all(|offset| *offset < a18));
    core.mapper_write(0x8000, 0x80);
    let after = chr_offsets(&core);
    assert_ne!(before, after, "the swap moved the chip's own banks");
    assert!(
        after[..4].iter().all(|offset| *offset >= a18),
        "$0000-$0FFF still carries CHR A18 after the swap"
    );
    assert!(
        after[4..].iter().all(|offset| *offset < a18),
        "$1000-$1FFF still does not"
    );
}

/// "*Dragon Ball Z 5* reads the same address to select between Chinese and English text",
/// `.... ...A`, under the same `$E100` mask as the write side.
///
/// The page does not say which value is Chinese — "the GAL16V8 has the value hard-wired (to
/// Chinese text in all known copies)" — so the figure is a choice. **Q-164 §3** made it zero;
/// `docs/review/Q-XCHECK.md` §CHANGE 6 overturned that and MF-FIX made it **`$01`**, because
/// FCEUX initialises its read register to 1 with the comment "chinese is default"
/// (`mmc3.cpp:394`) and Nestopia UE answers a hard-coded `0x01` (`NstBoardRexSoftDb5z.cpp:83`).
/// puNES makes it a user dip switch and Mesen 2 has no read side, so two projects name the value
/// and none contradicts it; under `$00` the game selects **English**.
///
/// The second half of the test is the shape the page does give, and this fix does not touch it:
/// the board answers inside the `$E100` mask and nothing outside it, so an address the mask
/// excludes is open bus and not a byte the board drove.
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_102_mapper_12_answers_a_read_of_4132_with_the_language_bit() {
    let core = MapperRom::new(12).prg(16).chr(64).load();
    let cartridge = core.cartridge().unwrap();
    for address in [0x4132u16, 0x4100, 0x41FF, 0x4300, 0x5100, 0x5FFF] {
        assert_eq!(
            Some(0x01),
            cartridge.try_cpu_peek(address),
            "{address:#06X} is the language bit, and it is 1 -- Chinese (CHANGE 6)"
        );
    }

    for address in [0x4032u16, 0x4232, 0x5032] {
        assert_eq!(
            None,
            cartridge.try_cpu_peek(address),
            "{address:#06X} is outside the $E100 mask, so the board drives nothing"
        );
    }
}

/// Submapper 1 is the Front Fareast **Magic Card 4M**: "ROM images that have been extracted from
/// disk images for the Front Fareast Magic Card 4M RAM cartridge", which need the Super Magic
/// Card's own register file and a 512-byte trainer "initialized by `JSR`ing to $7003 before
/// `JMP`ing to the game's reset vector".
///
/// That is copier hardware, which is §1.8 of `docs/review/UNL-census.md`'s reason for leaving
/// mappers 6, 8 and 17 out, and census §7.2 leaves this submapper out by name. A dump of it is
/// refused with REQ-CORE-011's message rather than run as an MMC3A clone it is not (Q-164).
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_011_mapper_12_submapper_1_is_the_magic_card_4m_and_is_refused() {
    let mut core = NesCore::new();
    let load = core.load_rom(
        &MapperRom::new(12).prg(16).chr(64).submapper(1).image(),
        "MagicCard4M.nes",
    );
    assert!(!load.success);
    assert!(load.error.unwrap().contains("12"));
    assert!(!core.is_running());

    // Submapper 0 and an iNES 1.0 image are both the SL-5020B.
    for rom in [
        MapperRom::new(12).prg(16).chr(64),
        MapperRom::new(12).prg(16).chr(64).submapper(0),
    ] {
        assert_eq!(12, rom.load().cartridge().unwrap().mapper_number());
    }
}

/// Nothing of this board reaches mapper 4, 118 or 116: `$4132` is the outer register here and
/// nothing at all on any of the three, and a mapper-12 image is not read as an MMC6 or an MMC3C
/// however its submapper byte is set — `Mmc3::new` reads a submapper only for mapper 4, which is
/// MF-6a3 step 1's tightening.
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_102_mapper_12_leaves_the_mmc3_numbers_alone() {
    let mut mmc3 = MapperRom::new(4).prg(16).chr(512).load();
    let before = chr_offsets(&mmc3);
    mmc3.mapper_write(0x4132, 0x11);
    assert_eq!(before, chr_offsets(&mmc3), "no $4132 on mapper 4");
    assert_eq!(None, mmc3.cartridge().unwrap().try_cpu_peek(0x4132));

    // Submapper 1 of mapper 4 is the MMC6, whose PRG-RAM is 1 KiB; the same byte on a mapper-12
    // image is the Magic Card 4M and refused, so the only mapper-12 board there is has the 8 KiB
    // an MMC3 has.
    let huang = MapperRom::new(12).prg(16).chr(64).submapper(2).load();
    assert_eq!(
        8 * 1024,
        huang.cartridge().unwrap().memory().prg_ram().len(),
        "a mapper-12 submapper byte is not an MMC6's"
    );
}

/// nesdev ("INES Mapper 014"), the supervisor register at `$A131`:
///
/// ```text
/// D~7654 3210
///   ---------
///   A.B. C.M.
///   | |  | +-- Mapper mode (0: VRC2, 1: MMC3)
///   | |  +---- CHR A18 for PPU $0000-$0FFF
///   | +------- CHR A18 for PPU $1000-$17FF
///   +--------- CHR A18 for PPU $1800-$1FFF
/// ```
///
/// **Bit 1** is the mode and **bits 3, 5 and 7** the three lines. The board powers on in MMC3
/// mode (Q-165), and the two chips' layouts are unmistakable: an MMC3's `$8000`/`$8001`
/// select-and-value pair against a VRC2's two independent 8 KiB registers at `$8000` and `$A000`.
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_102_mapper_14_switches_between_two_modes_from_a131() {
    let mut core = MapperRom::new(14).prg(32).chr(128).load();
    // Power-on: MMC3 mode, so R6 = 0, R7 = 1 and the last two 8 KiB banks fixed.
    assert_eq!([0, 1, 30, 31], core.prg_windows(), "power-on is MMC3 mode");
    core.mmc3_bank(6, 3);
    core.mmc3_bank(7, 5);
    assert_eq!([3, 5, 30, 31], core.prg_windows(), "the MMC3's own layout");

    // VRC2 mode: bit 1 clear. Its two PRG registers are nothing like the MMC3's pair.
    core.mapper_write(0xA131, 0x00);
    core.mapper_write(0x8000, 0x07);
    core.mapper_write(0xA000, 0x09);
    assert_eq!([7, 9, 30, 31], core.prg_windows(), "the VRC2's own layout");

    // And back: each chip kept its own registers, which is what the ASIC's page requires of it.
    core.mapper_write(0xA131, 0x02);
    assert_eq!(
        [3, 5, 30, 31],
        core.prg_windows(),
        "the MMC3's registers held"
    );
    core.mapper_write(0xA131, 0x00);
    assert_eq!([7, 9, 30, 31], core.prg_windows(), "and the VRC2's did too");

    // **There is no MMC1 mode**: the page gives the register one mode bit, so bits 0 and 2 are a
    // CHR line and nothing, and no value of it can reach a third chip. Bit 0 set with bit 1 clear
    // is still the VRC2.
    core.mapper_write(0xA131, 0x01);
    assert_eq!(
        [7, 9, 30, 31],
        core.prg_windows(),
        "no MMC1 mode on this PCB"
    );
}

/// The three CHR A18 bits, one case per window group — the test census §7.2 asks for by name:
/// "with the three bits set differently, windows 0-3, 4-5 and 6-7 land in different 256 KiB
/// halves."
///
/// MF-6b step 6 called this "a different CHR path" and it is why mapper 14 is a board and not a
/// parameter of mapper 116: that number's one `C` bit is CHR A18 for the whole of CHR, and no
/// addend over eight equal windows can say a table whose A18 changes at `$1000` and again at
/// `$1800`.
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_102_mapper_14_drives_three_chr_a18_bits_over_unequal_window_groups() {
    let mut core = MapperRom::new(14).prg(16).chr(512).load();
    let a18 = 256 * 0x400;
    let base = chr_offsets(&core);
    let carries = |core: &NesCore| chr_offsets(core).map(|offset| offset >= a18);

    core.mapper_write(0xA131, 0x02); // MMC3 mode, no lines.
    assert_eq!([false; 8], carries(&core));

    core.mapper_write(0xA131, 0x02 | 0x08); // Bit 3: PPU $0000-$0FFF.
    assert_eq!(
        [true, true, true, true, false, false, false, false],
        carries(&core),
        "bit 3 is the four 1 KiB windows of $0000-$0FFF"
    );

    core.mapper_write(0xA131, 0x02 | 0x20); // Bit 5: PPU $1000-$17FF.
    assert_eq!(
        [false, false, false, false, true, true, false, false],
        carries(&core),
        "bit 5 is the two of $1000-$17FF and not the four of $1000-$1FFF"
    );

    core.mapper_write(0xA131, 0x02 | 0x80); // Bit 7: PPU $1800-$1FFF.
    assert_eq!(
        [false, false, false, false, false, false, true, true],
        carries(&core),
        "bit 7 is the two of $1800-$1FFF"
    );

    core.mapper_write(0xA131, 0x02 | 0xA8); // All three.
    assert_eq!([true; 8], carries(&core));
    core.mapper_write(0xA131, 0x02);
    assert_eq!(base, chr_offsets(&core), "and clearing them puts it back");

    // The lines are the board's and not the chip's, so they hold in VRC2 mode too.
    core.mapper_write(0xA131, 0x20);
    assert_eq!(
        [false, false, false, false, true, true, false, false],
        carries(&core),
        "bit 5 in VRC2 mode"
    );
}

/// The overlap, and the order census §7.2 states for it: "a write to $A131 switches mode *and* is
/// seen by the sub-board's register at the same address — the ordering test that the overlap makes
/// necessary", with "the board's decode runs *before* the sub-board's".
///
/// `$A131 & $E001` is `$A001`, the MMC3's PRG-RAM protect register, so in MMC3 mode a supervisor
/// write is also a `$A001` write — and `$A001`'s bit 7 is the RAM enable, so writing `$02` there
/// (MMC3 mode, no CHR lines) **disables the PRG-RAM** as a side effect. That is the overlap
/// showing, and it is observable without inventing a probe.
///
/// In VRC2 mode nothing reaches the chip, and that is the page's own second rule rather than a
/// choice: "the VRC2 PRG and mirroring registers do not respond to address mirrors", and `$A131`
/// is a mirror of the VRC2's `$A000` PRG register (Q-165).
#[cfg(feature = "board-huang1")]
#[test]
fn q_165_mapper_14_supervisor_writes_are_seen_by_the_sub_board_at_the_same_address() {
    // MMC3 mode: $A131 is $A001, and the MMC3 powers on with the RAM enabled and writable.
    let mut core = MapperRom::new(14).prg(16).chr(64).load();
    core.write_memory(0x6000, 0x5A);
    assert_eq!(
        Some(0x5A),
        core.cartridge().unwrap().try_cpu_peek(0x6000),
        "the MMC3's RAM is enabled"
    );

    // $02 is MMC3 mode with bit 7 clear, which is $A001's "PRG-RAM disabled".
    core.mapper_write(0xA131, 0x02);
    assert_eq!(
        [0, 1, 14, 15],
        core.prg_windows(),
        "still MMC3 mode, so the supervisor half of the write landed"
    );
    assert_eq!(
        None,
        core.cartridge().unwrap().try_cpu_peek(0x6000),
        "and the $A001 half landed too: the overlap is real"
    );

    // $82 puts bit 7 back, which re-enables it — the same write doing both jobs again.
    core.mapper_write(0xA131, 0x82);
    assert_eq!(Some(0x5A), core.cartridge().unwrap().try_cpu_peek(0x6000));
    assert_eq!([0, 1, 14, 15], core.prg_windows(), "and still MMC3 mode");

    // VRC2 mode: $A131 is a mirror of $A000 and the chip does not answer mirrors, so the VRC2's
    // second PRG register keeps its power-on value of 1 through any number of supervisor writes.
    let mut core = MapperRom::new(14).prg(16).chr(64).load();
    core.mapper_write(0xA131, 0x00); // VRC2 mode.
    assert_eq!([0, 1, 14, 15], core.prg_windows(), "the VRC2's power-on");
    // Every value here keeps bit 1 clear, or the board would be in MMC3 mode and $A000 would be
    // the MMC3's inert mirroring register instead of the VRC2's second PRG register.
    for value in [0x00u8, 0x08, 0x20, 0x80, 0xA9] {
        core.mapper_write(0xA131, value);
        assert_eq!(
            1,
            core.prg_windows()[1],
            "{value:#04X}: $A131 is a mirror of $A000 and the VRC2 ignores it"
        );
    }

    // And $A000 itself is not a mirror, so that one does land.
    core.mapper_write(0xA000, 0x09);
    assert_eq!(9, core.prg_windows()[1]);
}

/// The other half of the page's Nestopia paragraph, on both registers it covers.
///
/// > According to Nestopia's mapper source code, **mirroring can only be controlled in VRC2
/// > mode**, and **the VRC2 PRG and mirroring registers do not respond to address mirrors**. These
/// > behaviors contradict other emulators and have not been verified on original hardware.
///
/// Both are implemented, the page's own caveat is in the code comment, and Q-165 records that the
/// page says plainly it is unverified. The first rule is the MMC3's `$A000` made inert; the second
/// covers `$9000` as well as the two PRG registers.
#[cfg(feature = "board-huang1")]
#[test]
fn q_165_mapper_14_takes_its_mirroring_from_the_vrc2_alone_and_from_no_mirror() {
    let mut core = MapperRom::new(14).prg(16).chr(64).load();

    // VRC2 mode: $9000 bit 0 is the arrangement, and it is the only thing on this board that
    // moves the mirroring.
    core.mapper_write(0xA131, 0x00);
    core.mapper_write(0x9000, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.mapper_write(0x9000, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());

    // A mirror of $9000 does nothing, which is the second rule.
    for address in [0x9001u16, 0x9002, 0x9003, 0x90FF, 0x9FFF] {
        core.mapper_write(address, 0x01);
        assert_eq!(
            Nm::Vertical,
            core.mirroring(),
            "{address:#06X} is a mirror of $9000"
        );
    }

    // MMC3 mode: $A000 is the chip's mirroring register and it is inert here, so the arrangement
    // the VRC2 last set survives the mode switch and every $A000 write in it.
    core.mapper_write(0xA131, 0x02);
    for value in [0x00u8, 0x01] {
        core.mapper_write(0xA000, value);
        assert_eq!(
            Nm::Vertical,
            core.mirroring(),
            "mirroring can only be controlled in VRC2 mode"
        );
    }

    // And the mode switch itself does not move it either way round.
    core.mapper_write(0xA131, 0x00);
    core.mapper_write(0x9000, 0x01);
    core.mapper_write(0xA131, 0x02);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

/// "Mask: $FFFF?" — the question mark is the page's, so the decode is the one the page states and
/// Q-165 records that the page is unsure of it: `$A131` and no other address.
///
/// It matters because every near miss is a real register of the sub-board: `$A130` and `$A132` are
/// `$A000`/`$A001` mirrors on the MMC3, `$A131`'s own block is the VRC2's PRG register, and a
/// board that decoded the supervisor loosely would switch mode on an ordinary bank write.
#[cfg(feature = "board-huang1")]
#[test]
fn q_165_mapper_14_decodes_the_supervisor_register_at_a131_exactly() {
    for address in [0xA130u16, 0xA132, 0xA111, 0xA031, 0xB131, 0x2131, 0xA133] {
        let mut core = MapperRom::new(14).prg(32).chr(128).load();
        core.mmc3_bank(6, 3);
        core.mmc3_bank(7, 5);
        core.mapper_write(address, 0x00); // Would be VRC2 mode if it were the register.
        assert_eq!(
            [3, 5, 30, 31],
            core.prg_windows(),
            "{address:#06X} is not the supervisor register, so this is still the MMC3"
        );
    }

    let mut core = MapperRom::new(14).prg(32).chr(128).load();
    core.mmc3_bank(6, 3);
    core.mapper_write(0xA131, 0x00);
    core.mapper_write(0x8000, 0x07);
    assert_eq!(
        7,
        core.prg_windows()[0],
        "$A131 exactly is the supervisor register"
    );
}

/// Nothing of mapper 14 reaches mapper 116 or the five Konami numbers, which is the whole point of
/// the feature's shape: the VRC2 rule is a load-time field set by this board's constructor alone.
#[cfg(all(feature = "board-huang1", feature = "board-116"))]
#[test]
fn req_core_102_mapper_14_leaves_mapper_116_and_the_vrc_numbers_alone() {
    // Mapper 116's supervisor register is at $4100 and $A131 is nothing on it.
    let mut somari = MapperRom::new(116).prg(32).chr(128).load();
    somari.mmc3_bank(6, 3);
    somari.mapper_write(0xA131, 0x00);
    assert_eq!(
        [3, 1, 30, 31],
        somari.prg_windows(),
        "no $A131 on mapper 116"
    );

    // And a VRC2b of mapper 23 submapper 3 still answers its PRG and mirroring registers at every
    // mirror, because the rule is mapper 14's field and not the variant's.
    let mut vrc = MapperRom::new(23).submapper(3).prg(16).chr(32).load();
    vrc.mapper_write(0xA003, 0x09);
    assert_eq!(9, vrc.prg_windows()[1], "a real VRC2b answers $A003");
    vrc.mapper_write(0x9003, 0x01);
    assert_eq!(Nm::Horizontal, vrc.mirroring(), "and $9003");
}

// ------------------------------------------- the JV001 / TXC 05-00002-010 protection ASIC
// ------------------------------------------- (36, 132, 136, 147, 172, 173), MF-UNL-1 step 1

/// The chip's "load" sequence: `$4102` takes the Input, `$4100` copies it into the accumulator
/// (complemented while Invert is set), and the caller then latches Output with `$8000`.
#[cfg(feature = "board-jv001")]
fn jv001_load(core: &mut NesCore, input: u8) {
    core.mapper_write(0x4102, input);
    core.mapper_write(0x4100, 0x00);
}

/// nesdev ("INES Mapper 036"): the chip's six data pins are CPU **D4-D5** on TXC's 01-22000-400,
/// `read $4100: [xxRR xxxx]` and `write $4102: [..PP ....]`; the 32 KiB PRG bank comes from
/// Output and the 8 KiB CHR bank from a latch of the board's own — "Mask: $E200, write $4200:
/// [.... CCCC]" (MF-UNL-1 step 1).
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_mapper_36_puts_the_chip_on_d4_d5_and_its_chr_latch_at_4200() {
    let mut core = MapperRom::new(36).prg(16).chr(128).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // $4102 = $30 is `PP` = 3 on D4-D5; Invert is clear at power-on on this number, so $4100
    // copies it straight through and $8000 clocks it onto the PRG pins: 32 KiB bank 3.
    jv001_load(&mut core, 0x30);
    core.mapper_write(0x8000, 0x00);
    assert_eq!([12, 13, 14, 15], core.prg_windows());

    // And the protection read answers the same bits back on D4-D5 and drives nothing else.
    assert_eq!(0x30, core.read_memory(0x4100));

    // The CHR latch is the board's, not the chip's: four bits at $4200 under mask $E200.
    core.mapper_write(0x4200, 0x05);
    assert_eq!(40, core.chr_bank(0x0000));
    assert_eq!(47, core.chr_bank(0x1C00), "one 8 KiB bank over the range");
    // $6200 is not it -- `$6200 & $E200` is `$6200` -- so the window stays inside $4000-$5FFF.
    core.mapper_write(0x6200, 0x02);
    assert_eq!(40, core.chr_bank(0x0000), "no register in $6000-$7FFF");
}

/// nesdev ("INES Mapper 132"): the chip on CPU D0-D3, and "write $8000: Copy R register to PRG
/// A15, CHR A14, CHR A13 banking pins sequentially" — Output bit 2 the 32 KiB PRG bank and bits
/// 1-0 the 8 KiB CHR bank (MF-UNL-1 step 1).
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_mapper_132_drives_prg_a15_and_chr_a14_a13_from_one_output() {
    let mut core = MapperRom::new(132).prg(16).chr(32).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));

    jv001_load(&mut core, 0x07);
    // The bank pins do not move until $8000 clocks Output: that is what makes the board look
    // like a CNROM to the game.
    assert_eq!([0, 1, 2, 3], core.prg_windows(), "Output not latched yet");
    core.mapper_write(0x8000, 0x00);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(24, core.chr_bank(0x0000));

    assert_eq!(0x07, core.read_memory(0x4100));
}

/// The protection: "Games will also check the lower three or four bits of $4100 for the correct
/// value after several increment and inversion operations as a copy-protection measure" (nesdev,
/// "INES Mapper 132"), over the three widths the family has — three bits on 132, four on 136 and
/// six on 172 (MF-UNL-1 step 1).
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_the_jv001_answers_4100_incremented_and_inverted_over_its_own_width() {
    // ---- three bits, mapper 132: the accumulator is `S`+`RRR` masked to $07.
    let mut m132 = MapperRom::new(132).prg(16).chr(32).load();
    jv001_load(&mut m132, 0x05);
    assert_eq!(0x05, m132.read_memory(0x4100), "copied through");

    // "write $4101: [.... ...V] - Invert Mode", and then $4100 copies `~PP`.
    m132.mapper_write(0x4101, 0x01);
    m132.mapper_write(0x4100, 0x00);
    assert_eq!(
        0x05 ^ 0x07,
        m132.read_memory(0x4100) & 0x07,
        "the low three bits come back complemented"
    );

    // "write $4103: [.... ...C] - Increment Mode", and then $4100 counts.
    m132.mapper_write(0x4103, 0x01);
    m132.mapper_write(0x4100, 0x00);
    assert_eq!(
        (0x05 ^ 0x07) + 1,
        m132.read_memory(0x4100) & 0x07,
        "incremented over three bits"
    );

    // ---- four bits, mapper 136: the same chip with the mask at $0F, and **Invert set at
    // power-on**, which is the one piece of power-on state either account of the chip gives.
    let mut m136 = MapperRom::new(136).prg(8).chr(64).load();
    jv001_load(&mut m136, 0x03);
    assert_eq!(
        0x03 ^ 0x0F,
        m136.read_memory(0x4100) & 0x0F,
        "Invert is set at power-on on this wiring"
    );
    m136.mapper_write(0x4103, 0x01);
    m136.mapper_write(0x4100, 0x00);
    assert_eq!(
        (0x03 ^ 0x0F) + 1,
        m136.read_memory(0x4100) & 0x0F,
        "incremented over four bits"
    );

    // ---- six bits, mapper 172: "Games will check the lower six bits of $4100". The two bits
    // above the accumulator are the Input latch's own, "inverted if Invert==1".
    let mut m172 = MapperRom::new(172).prg(4).chr(32).load();
    jv001_load(&mut m172, 0x00);
    assert_eq!(
        0x3F,
        m172.read_memory(0x4100) & 0x3F,
        "all six bits complemented from a zero Input"
    );
}

/// nesdev ("INES Mapper 172"): "the bit order D0-D5 is *reversed*". The reversal is a bit
/// permutation **in the board's wiring**, so it shows on what the chip's Output selects and not
/// on a value that goes in through it and comes back out through it (MF-UNL-1 step 1).
///
/// This is the test that proves the parameter is wired, and the second half is the reason it is
/// written against the *bank* and not against the read: a complement commutes with a bit
/// permutation and `reverse6` is an involution, so `reverse(!reverse(v))` is `!v` — the read
/// path cancels the reversal exactly. The banking pins are inside the chip and do not.
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_mapper_172_reverses_d0_d5_where_it_shows_on_the_bank() {
    // The same CPU byte into the same sequence on both boards. On 136 `$01` is Input bit 0, so
    // the accumulator takes it and Output's low nibble is `!1 & $0F` = $0E; on 172 the byte
    // arrives at the chip as `$20`, which is **above** the four-bit accumulator, so the
    // accumulator loads zero and Output is `$0F` with `$20` above it.
    let mut m136 = MapperRom::new(136).prg(8).chr(64).load();
    jv001_load(&mut m136, 0x01);
    m136.mapper_write(0x8000, 0x00);
    // Output $0E, "CHR ROM bank = (Output & 7)" = 6.
    assert_eq!(48, m136.chr_bank(0x0000));

    let mut m172 = MapperRom::new(172).prg(4).chr(32).load();
    jv001_load(&mut m172, 0x01);
    m172.mapper_write(0x8000, 0x00);
    // Output $2F, "8 KiB CHR-ROM bank number := (Output &3)" = 3, where 136's own Output would
    // have given bank 2 under the same formula.
    assert_eq!(24, m172.chr_bank(0x0000));
    assert_eq!(2, 0x0E & 0x03);

    // The read path, where the reversal cancels: both boards answer `!v` over six bits for the
    // same written byte, and that is an identity and not an accident.
    let mut a = MapperRom::new(136).prg(8).chr(64).load();
    let mut b = MapperRom::new(172).prg(4).chr(32).load();
    for input in [0x00u8, 0x13, 0x15, 0x2A, 0x3F] {
        jv001_load(&mut a, input);
        jv001_load(&mut b, input);
        assert_eq!(
            (!input) & 0x3F,
            a.read_memory(0x4100) & 0x3F,
            "136, input {input:#04X}"
        );
        assert_eq!(
            a.read_memory(0x4100) & 0x3F,
            b.read_memory(0x4100) & 0x3F,
            "the reversal is its own inverse on a round trip, input {input:#04X}"
        );
    }
}

/// nesdev ("INES Mapper 172"): "$8000-$FFFF: Output ← Register; nametable mirroring changes
/// (Horizontal if Invert==0, Vertical if Invert==1)". The one number of the six whose board has
/// a mirroring wire at all (MF-UNL-1 step 1).
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_mapper_172_takes_its_mirroring_from_invert_on_an_8000_write() {
    let mut core = MapperRom::new(172).prg(4).chr(32).load();
    // Invert is set at power-on on this wiring, and `reset` is where the board says so.
    assert_eq!(Nm::Vertical, core.mirroring());

    // **Invert arrives on CPU bit 5 here and not on bit 0**, which is the reversed bus seen
    // from the other side: the page says so in as many words, "$4101: Invert ← bit 5 of written
    // value", where 132's and 173's say `[.... ...V]`. $20 is the bit, $01 is nothing.
    core.mapper_write(0x4101, 0x20);
    core.mapper_write(0x8000, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring(), "Invert still set");

    core.mapper_write(0x4101, 0x00); // Invert clear.
    assert_eq!(
        Nm::Vertical,
        core.mirroring(),
        "the wire is clocked by the $8000 write and not by $4101"
    );
    core.mapper_write(0x8000, 0x00);
    assert_eq!(Nm::Horizontal, core.mirroring());

    core.mapper_write(0x4101, 0x01); // Bit 0 is not the Invert pin on this board.
    core.mapper_write(0x8000, 0x00);
    assert_eq!(Nm::Horizontal, core.mirroring(), "bit 0 is not the wire");

    core.mapper_write(0x4101, 0x20); // Invert set.
    core.mapper_write(0x8000, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());

    // And no other number of the family has the wire: 136's board hardwires it.
    let mut m136 = MapperRom::new(136).prg(8).chr(64).load();
    let before = m136.mirroring();
    m136.mapper_write(0x4101, 0x01);
    m136.mapper_write(0x8000, 0x00);
    assert_eq!(before, m136.mirroring(), "mapper 136 has no mirroring wire");
}

/// nesdev ("INES Mapper 136"): "CHR ROM bank = (Output & 7)" and "PRG ROM bank = (Output >> 4) &
/// 1", the latter for *Mahjong Companion* alone — a line Mesen 2's `Sachen_136` does not
/// implement and this build does, the page being the source of truth (MF-UNL-1 step 1).
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_mapper_136_banks_chr_from_output_and_prg_from_bit_4() {
    let mut core = MapperRom::new(136).prg(8).chr(64).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // Input $13: `$3` into the accumulator (complemented, Invert being set) and `$10` above it.
    // Output is then `$1C`: CHR bank 4, PRG bank 1.
    jv001_load(&mut core, 0x13);
    core.mapper_write(0x8000, 0x00);
    assert_eq!(32, core.chr_bank(0x0000));
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(0x2C, core.read_memory(0x4100) & 0x3F);
}

/// nesdev ("INES Mapper 147"), the Sachen 3018's 60-pin release: the chip's pins are CPU
/// **D2-D7**, so every byte is the chip's value rotated by two, and the Output diagram puts
/// "PRG-ROM bank select (A15-A16)" on register bits 5 and 0 with "CHR-ROM bank select (A13-A16)"
/// on bits 4-1 (MF-UNL-1 step 1).
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_mapper_147_puts_the_chip_on_d2_d7() {
    let mut core = MapperRom::new(147).prg(16).chr(128).load();

    // $94 rotated right two is $25 at the chip: Input $5 into the accumulator (complemented,
    // Invert being set, so the accumulator is $A) and $20 above it. Output is $2A -- PRG bank
    // 2 from bits 5 and 0, CHR bank 5 from bits 4-1.
    jv001_load(&mut core, 0x94);
    core.mapper_write(0x8000, 0x00);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(40, core.chr_bank(0x0000));

    // And the read is the same rotation the other way: "Returns Register bits 0-5 on CPU data
    // bus bits 2-7".
    assert_eq!(0x6B, core.read_memory(0x4100));
}

/// nesdev ("INES Mapper 173"): "copy bottom two bits of internal register 'RRR' to CHR A15 and
/// CHR A13 banking pins", with $4101's "value … immediately inverted and used as CHR A14". The
/// page's shorter formula line, `(Output &1) | (~Invert <<1)`, is the same wiring with CHR A15
/// left out; this build takes all three bits, which is Mesen 2's reading too (MF-UNL-1 step 1).
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_mapper_173_drives_chr_a14_from_the_inverse_of_invert() {
    let mut core = MapperRom::new(173).prg(4).chr(64).load();
    // Invert is clear at power-on on this wiring, so CHR A14 is high before the game writes
    // anything at all: bank 2.
    assert_eq!(16, core.chr_bank(0x0000));

    jv001_load(&mut core, 0x03);
    core.mapper_write(0x8000, 0x00);
    // Output $3: A13 from bit 0, A15 from bit 1, A14 still high -- bank 7.
    assert_eq!(56, core.chr_bank(0x0000));

    // "This value is immediately inverted and used as CHR A14": the bank moves on the $4101
    // write, with no $8000 behind it.
    core.mapper_write(0x4101, 0x01);
    assert_eq!(40, core.chr_bank(0x0000), "A14 low, bank 5");
    core.mapper_write(0x4101, 0x00);
    assert_eq!(56, core.chr_bank(0x0000), "A14 high again, bank 7");
}

/// nesdev ("INES Mapper 173"), the 小瑪琍 board: it "uses the Mapper 173 circuit board but
/// replaces the 32 KiB CHR-ROM chip with a SGS M2764A 8 KiB UVEPROM. Its pin 27 function is
/// /PGM instead of the normal A14. Setting Invert on this board just disables CHR-ROM."
///
/// So on an image with 8 KiB of CHR-ROM the Invert bit is a chip-enable and not a bank bit, and
/// the board shows nothing while it is set (MF-UNL-1 step 1).
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_102_mapper_173_with_an_8_kib_chr_rom_disables_it_on_invert() {
    let mut core = MapperRom::new(173).prg(4).chr(8).load();
    assert_eq!(0, core.chr_bank(0x0000));
    assert_eq!(7, core.chr_bank(0x1C00), "the one 8 KiB chip, unbanked");

    core.mapper_write(0x4101, 0x01);
    assert_eq!(0, core.chr_bank(0x0000), "CHR-ROM disabled");
    assert_eq!(0, core.chr_bank(0x1C00), "CHR-ROM disabled");

    core.mapper_write(0x4101, 0x00);
    assert_eq!(7, core.chr_bank(0x1C00), "and back");

    // An image with the number's usual 32 KiB keeps all three bank bits: Invert is CHR A14
    // there and not a chip enable.
    let mut big = MapperRom::new(173).prg(4).chr(32).load();
    big.mapper_write(0x4101, 0x01);
    assert_eq!(0, big.chr_bank(0x0000), "A14 low, bank 0");
    big.mapper_write(0x4101, 0x00);
    assert_eq!(16, big.chr_bank(0x0000), "A14 high, bank 2");
}

/// Q-140, and the one place this lot leaves `docs/review/UNL-census.md` for the page: **mapper
/// 173's board has no bus conflicts**, and that is what lets *撲克精靈* run on it.
///
/// The census's §6.2 row and its test list give the number `bus`. Its page gives the board no
/// bus-conflict line at all, and the paragraph the census is reading says the opposite of what
/// the row does: *撲克精靈* "writes the actual 8 KiB bank number to $8000 to copy Register into
/// Output, making it potentially CNROM-compatible. It will not run as Mapper 3 on emulators that
/// emulate bus conflicts (such as FCEUX) though, nor will it run on an actual Nintendo CNROM
/// board, for the same reason." Giving the JV001 board conflicts would reproduce, on the one
/// title the number exists for, the failure that sentence describes.
///
/// Both halves are pinned: the write reaches the latch whole on 173, and the same write on a
/// CNROM **with** conflicts loses its bank — which is the page's sentence, reproduced.
#[cfg(feature = "board-jv001")]
#[test]
fn q_140_mapper_173_has_no_bus_conflicts_and_a_cnrom_with_them_loses_the_bank() {
    // The game's own move: load the bank into the chip, then store the bank number at $8000.
    // The 32 KiB PRG window's first bank is numbered 0, so a board with conflicts would AND
    // every written byte to zero.
    let mut core = MapperRom::new(173).prg(4).chr(64).load();
    jv001_load(&mut core, 0x03);
    assert_eq!(
        0u8,
        core.read_memory(0x8000),
        "the ROM byte under the write"
    );
    core.mapper_write(0x8000, 0x07);
    assert_eq!(56, core.chr_bank(0x0000), "bank 7, the chip's own Output");

    // The byte written is not used at all on this board -- the write only clocks Output -- so
    // the bank is the same whatever the game stores and whatever the ROM holds under it.
    for value in [0x00u8, 0x07, 0xFF, 0x5A] {
        core.mapper_write(0x8000, value);
        assert_eq!(56, core.chr_bank(0x0000), "stored {value:#04X}");
    }

    // The other half of the page's sentence: the same sequence on a CNROM board that does have
    // conflicts (submapper 2, MR-BUS-1) latches `value & rom`, which is zero here.
    let mut cnrom = MapperRom::new(3).prg(4).chr(64).submapper(2).load();
    assert_eq!(0u8, cnrom.read_memory(0x8000));
    cnrom.mapper_write(0x8000, 0x07);
    assert_eq!(
        0,
        cnrom.chr_bank(0x0000),
        "a CNROM with bus conflicts loses the bank, which is why the number exists"
    );
}

/// REQ-CORE-090/091 for the family: every byte of the chip survives a save and a restore, with
/// the state dirtied first — including the two flags and the bits above the accumulator, which
/// no bank window can show.
#[cfg(feature = "board-jv001")]
#[test]
fn req_core_090_the_jv001_chip_survives_a_save_and_restore() {
    for mapper in [36usize, 132, 136, 147, 172, 173] {
        let mut core = MapperRom::new(mapper).prg(16).chr(128).load();
        // Dirty every register the chip has: Input and its high bits, Invert, Increment, the
        // accumulator through a load and an increment, Output through the latch, and mapper
        // 36's own CHR latch.
        core.mapper_write(0x4101, 0x01);
        core.mapper_write(0x4102, 0x35);
        core.mapper_write(0x4100, 0x00);
        core.mapper_write(0x4103, 0x01);
        core.mapper_write(0x4100, 0x00);
        core.mapper_write(0x8000, 0x00);
        core.mapper_write(0x4200, 0x09);

        let state = core.save_state(true).unwrap();
        let windows = (core.prg_windows(), core.chr_windows(), core.mirroring());
        let protection = core.read_memory(0x4100);

        // Move it somewhere else, then come back.
        core.mapper_write(0x4101, 0x00);
        core.mapper_write(0x4102, 0x00);
        core.mapper_write(0x4100, 0x00);
        core.mapper_write(0x8000, 0x00);
        core.mapper_write(0x4200, 0x00);

        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed, "mapper {mapper}");
        assert_eq!(
            windows,
            (core.prg_windows(), core.chr_windows(), core.mirroring()),
            "mapper {mapper}"
        );
        assert_eq!(protection, core.read_memory(0x4100), "mapper {mapper}");
        assert_eq!(state, core.save_state(true).unwrap(), "mapper {mapper}");
    }
}

// ------------------------------------------- Sachen's eight-register ASIC (150, 243),
// ------------------------------------------- MF-UNL-1 step 2

/// The board's one idiom: select a register at the index port, then write it at the data port.
#[cfg(feature = "board-sachen-8reg")]
fn sachen_write(core: &mut NesCore, register: u8, value: u8) {
    core.mapper_write(0x4100, register);
    core.mapper_write(0x4101, value);
}

/// nesdev ("INES Mapper 150"), the Sachen SA-015 / SA-630: "R4: .... ...C (CHR A15)", "R5:
/// .... ..PP (PRG A16..A15)", "R6: .... ..BA (CHR A14..A13)" — and "Registers 0-3 are completely
/// unused", which this build encodes as a **no-op and not an `unreachable!`** because "All three
/// bits in all eight registers are fully readable and writable" (MF-UNL-1 step 2).
#[cfg(feature = "board-sachen-8reg")]
#[test]
fn req_core_102_mapper_150_banks_chr_from_r4_and_r6_and_prg_from_r5() {
    let mut core = MapperRom::new(150).prg(16).chr(64).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));

    // R4 is CHR A15, two bits above R6's pair.
    sachen_write(&mut core, 4, 0x01);
    assert_eq!(32, core.chr_bank(0x0000));
    // R6 is CHR A14-A13, under it.
    sachen_write(&mut core, 6, 0x03);
    assert_eq!(56, core.chr_bank(0x0000));
    assert_eq!(63, core.chr_bank(0x1C00), "one 8 KiB bank over the range");
    // R5 is the 32 KiB PRG bank.
    sachen_write(&mut core, 5, 0x02);
    assert_eq!([8, 9, 10, 11], core.prg_windows());

    // The four registers the page calls unused: written, stored, read back, and **nothing
    // moves**. Mapper 150 has nothing on R2 either; 243 is the number that does.
    let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
    for register in [0u8, 1, 2, 3] {
        sachen_write(&mut core, register, 0x07);
        assert_eq!(
            banks,
            (core.prg_windows(), core.chr_windows(), core.mirroring()),
            "R{register} has no external effect"
        );
        assert_eq!(
            0x07,
            core.read_memory(0x4101),
            "R{register} is readable all the same"
        );
    }
}

/// nesdev ("INES Mapper 243"), the Sachen SA-020A of *美女拳 - Honey Peach*: the same ASIC with
/// "R2: CHR A13", "R4: CHR A14" and "R6: CHR A16..A15", so the CHR field is four bits where 150's
/// is three, and R0, R1 and R3 have "no external effect" (MF-UNL-1 step 2).
#[cfg(feature = "board-sachen-8reg")]
#[test]
fn req_core_102_mapper_243_adds_chr_a13_on_r2() {
    let mut core = MapperRom::new(243).prg(16).chr(128).load();
    assert_eq!(0, core.chr_bank(0x0000));

    // R2 is the bottom CHR bit on this number and nothing at all on 150.
    sachen_write(&mut core, 2, 0x01);
    assert_eq!(8, core.chr_bank(0x0000));
    sachen_write(&mut core, 4, 0x01);
    assert_eq!(24, core.chr_bank(0x0000));
    sachen_write(&mut core, 6, 0x03);
    assert_eq!(120, core.chr_bank(0x0000));
    sachen_write(&mut core, 5, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows());

    let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
    for register in [0u8, 1, 3] {
        sachen_write(&mut core, register, 0x07);
        assert_eq!(
            banks,
            (core.prg_windows(), core.chr_windows(), core.mirroring()),
            "R{register} has no external effect"
        );
        assert_eq!(0x07, core.read_memory(0x4101));
    }

    // And the same byte on R2 does move the bank here, which is the whole difference between
    // the two numbers.
    sachen_write(&mut core, 2, 0x00);
    assert_eq!(112, core.chr_bank(0x0000), "R2 is CHR A13 on mapper 243");
}

/// R7's four layouts, on both numbers (nesdev, "INES Mapper 150" and "INES Mapper 243"):
///
/// ```text
/// R7: ...  .MM.
///      0: S0-S0-S0-S1 (lower right unique, or vertically-flipped L)
///      1: Horizontal
///      2: Vertical
///      3: Single-screen, page 1
/// ```
///
/// Mode 0 is the one this core cannot say with a `NametableMirroring`, so it comes out of
/// `Mapper::nametable_page`; mode 3 is **Q-141**, "page 1" being `S1` in the page's own notation
/// where Mesen 2 reads it as screen A (MF-UNL-1 step 2).
#[cfg(feature = "board-sachen-8reg")]
#[test]
fn q_141_the_sachen_eight_register_asic_has_four_nametable_layouts() {
    for mapper in [150usize, 243] {
        let mut core = MapperRom::new(mapper).prg(16).chr(64).load();

        // Mode 0 at power-on, R7 being zero: three tables on page 0 and the fourth on page 1.
        assert_eq!(
            [0, 0, 0, 1],
            nametable_pages(&core),
            "mapper {mapper}, the vertically-flipped L"
        );

        // `MM` is bits 2-1 of the three-bit register, so the four modes are 0, 2, 4 and 6.
        sachen_write(&mut core, 7, 0x02);
        assert_eq!(Nm::Horizontal, core.mirroring(), "mapper {mapper}");
        assert_eq!(
            [-1, -1, -1, -1],
            nametable_pages(&core),
            "and the pages follow the mirroring again"
        );

        sachen_write(&mut core, 7, 0x04);
        assert_eq!(Nm::Vertical, core.mirroring(), "mapper {mapper}");

        // Q-141: "Single-screen, page 1" is `S1`, the page naming its two screens `S0` and `S1`
        // one row above. Mesen 2 reads the same line as screen A.
        sachen_write(&mut core, 7, 0x06);
        assert_eq!(Nm::SingleScreenB, core.mirroring(), "mapper {mapper}");

        // And back to the layout no mirroring can name.
        sachen_write(&mut core, 7, 0x00);
        assert_eq!([0, 0, 0, 1], nametable_pages(&core), "mapper {mapper}");

        // Bit 0 of R7 is not part of `MM`.
        sachen_write(&mut core, 7, 0x03);
        assert_eq!(Nm::Horizontal, core.mirroring(), "mapper {mapper}");
    }
}

/// "Mask: $C101" on both ports (nesdev, "INES Mapper 150"), which decodes A15, A14, A8 and A0
/// and nothing else — so the index and the data registers repeat through the whole of
/// $4100-$7FFF, and `registry::board_memory_sizes` has to give the board no PRG-RAM or a write
/// at $6101 would land in work RAM instead of in the ASIC (MF-UNL-1 step 2).
#[cfg(feature = "board-sachen-8reg")]
#[test]
fn req_core_102_the_sachen_eight_register_ports_repeat_up_to_7fff() {
    let mut core = MapperRom::new(150).prg(16).chr(64).load();

    // $6100 / $6101 are the same two ports as $4100 / $4101.
    core.mapper_write(0x6100, 0x04);
    core.mapper_write(0x6101, 0x01);
    assert_eq!(32, core.chr_bank(0x0000), "CHR A15 set from $6101");
    assert_eq!(0x01, core.read_memory(0x6101), "and readable there too");

    // So are $5300 / $5301 and $7F00 / $7F01: only A15, A14, A8 and A0 are decoded.
    core.mapper_write(0x7F00, 0x05);
    core.mapper_write(0x7F01, 0x03);
    assert_eq!([12, 13, 14, 15], core.prg_windows());

    // An address with A8 clear is not a port at all, whatever else it holds.
    core.mapper_write(0x4000, 0x06);
    core.mapper_write(0x4001, 0x02);
    assert_eq!(
        [12, 13, 14, 15],
        core.prg_windows(),
        "A8 clear is no register"
    );
    assert_eq!(
        32,
        core.chr_bank(0x0000),
        "and R6 was not written through $4001 either"
    );

    // The index port is write-only -- the page gives "read/write" to the data port alone -- so
    // a read of it is open bus, which this core answers as 0.
    assert_eq!(0x00, core.read_memory(0x4100));

    // $8000-$FFFF cannot be a port under this mask: `$8100 & $C101` is `$8100`.
    core.mapper_write(0x8100, 0x04);
    core.mapper_write(0x8101, 0x00);
    assert_eq!([12, 13, 14, 15], core.prg_windows(), "no port above $8000");
}

/// REQ-CORE-090/091: the eight registers and the index survive a save and a restore, with the
/// state dirtied first — including the three the page calls unused, which no bank window can
/// show and only the data port can prove (MF-UNL-1 step 2).
#[cfg(feature = "board-sachen-8reg")]
#[test]
fn req_core_090_the_sachen_eight_register_asic_survives_a_save_and_restore() {
    for mapper in [150usize, 243] {
        let mut core = MapperRom::new(mapper).prg(16).chr(128).load();
        for register in 0u8..8 {
            sachen_write(&mut core, register, (register + 1) & 0x07);
        }
        sachen_write(&mut core, 3, 0x05);
        core.mapper_write(0x4100, 0x03); // Leave the index somewhere non-zero.

        let state = core.save_state(true).unwrap();
        let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
        let pages = nametable_pages(&core);
        assert_eq!(0x05, core.read_memory(0x4101), "mapper {mapper}");

        for register in 0u8..8 {
            sachen_write(&mut core, register, 0x00);
        }

        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed, "mapper {mapper}");
        assert_eq!(
            banks,
            (core.prg_windows(), core.chr_windows(), core.mirroring()),
            "mapper {mapper}"
        );
        assert_eq!(pages, nametable_pages(&core), "mapper {mapper}");
        assert_eq!(
            0x05,
            core.read_memory(0x4101),
            "mapper {mapper}: R3 came back, and the index with it"
        );
        assert_eq!(state, core.save_state(true).unwrap(), "mapper {mapper}");
    }
}

// ------------------------------------------- the Sachen 72008 (133), MF-UNL-1 step 3

/// nesdev ("INES Mapper 133"), the 72-pin half of the page: "Mask: $E100, $4100: [.... .PCC] -
/// 32 KiB PRG bank and 8 KiB CHR bank". The 60-pin Sachen 3009's Namco-108 decode is
/// deliberately not built — the page's own closing sentence is that "only the 72-pin variant
/// requires emulation" — and `Sachen72008`'s note records the choice (MF-UNL-1 step 3).
#[cfg(feature = "board-133")]
#[test]
fn req_core_102_mapper_133_banks_prg_and_chr_from_one_latch_at_4100() {
    let mut core = MapperRom::new(133).prg(8).chr(32).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));

    // `CC` is the 8 KiB CHR bank.
    core.mapper_write(0x4100, 0x03);
    assert_eq!(24, core.chr_bank(0x0000));
    assert_eq!(31, core.chr_bank(0x1C00), "one 8 KiB bank over the range");
    // `P` is the 32 KiB PRG bank, above it.
    core.mapper_write(0x4100, 0x05);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(8, core.chr_bank(0x0000), "and `CC` went back to 1");
    core.mapper_write(0x4100, 0x00);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));
}

/// The latch's mask, "$E100", which decodes A15-A13 and A8 and nothing else: the register
/// repeats every other page through $4000-$5FFF and is **not** reachable from $6000-$7FFF or
/// from $8000-$FFFF.
///
/// Mesen 2 decodes `(addr & $6100) == $4100` over a $4100-$FFFF window, one address line
/// narrower, which also answers at $C100-$DFFF; this build takes the page's mask (MF-UNL-1
/// step 3).
#[cfg(feature = "board-133")]
#[test]
fn req_core_102_the_sachen_72008_latch_answers_only_under_mask_e100() {
    let mut core = MapperRom::new(133).prg(8).chr(32).load();

    // $4300 and $5F00 are the same latch as $4100.
    core.mapper_write(0x4300, 0x02);
    assert_eq!(16, core.chr_bank(0x0000));
    core.mapper_write(0x5F00, 0x01);
    assert_eq!(8, core.chr_bank(0x0000));

    // A8 clear is not the latch.
    core.mapper_write(0x4000, 0x03);
    assert_eq!(8, core.chr_bank(0x0000), "A8 clear is no register");

    // Neither is $6100, which is why the board needs no `intercept_cpu_write` and keeps the
    // header's PRG-RAM.
    core.mapper_write(0x6100, 0x03);
    assert_eq!(8, core.chr_bank(0x0000), "no register in $6000-$7FFF");

    // Nor $C100, where Mesen's narrower mask would answer.
    core.mapper_write(0xC100, 0x03);
    assert_eq!(8, core.chr_bank(0x0000), "no register above $8000");
    assert_eq!([0, 1, 2, 3], core.prg_windows());
}

/// REQ-CORE-090/091 and REQ-CORE-004 for the board: one byte, saved, restored and reset.
#[cfg(feature = "board-133")]
#[test]
fn req_core_090_the_sachen_72008_latch_survives_a_save_and_restore() {
    let mut core = MapperRom::new(133).prg(8).chr(32).load();
    core.mapper_write(0x4100, 0x06);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(16, core.chr_bank(0x0000));

    let state = core.save_state(true).unwrap();
    core.mapper_write(0x4100, 0x01);
    assert_eq!(8, core.chr_bank(0x0000));

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(16, core.chr_bank(0x0000));
    assert_eq!(state, core.save_state(true).unwrap());

    core.reset();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));
}

// ------------------------------------- the AVE NINA-03/06 (79, 146), MF-UNL-3 step 1

/// nesdev ("NINA-003-006"), the one page of both numbers: `A~[010x xxx1 xxxx xxxx]`,
/// `D~[.... PCCC]` — bits 2-0 the 8 KiB CHR bank and bit 3 the 32 KiB PRG bank. The address
/// pattern reads A15, A14, A13 and A8, which is mask $E100 against $4100 (MF-UNL-3 step 1).
///
/// 146 "denotes the 60-pin Sachen 3015 and SA-016 (chip-on-board) circuit boards, which are
/// functionally identical", so the two numbers are one board and the loop is the assertion.
#[cfg(feature = "board-nina03")]
#[test]
fn req_core_102_the_nina_03_banks_prg_and_chr_from_one_latch_at_4100() {
    for mapper in [79usize, 146] {
        let mut core = MapperRom::new(mapper).prg(8).chr(64).load();
        assert_eq!([0, 1, 2, 3], core.prg_windows(), "mapper {mapper}");
        assert_eq!(0, core.chr_bank(0x0000), "mapper {mapper}");

        // `CCC` is three bits wide: eight 8 KiB banks of the page's 64 KiB of CHR-ROM.
        core.mapper_write(0x4100, 0x07);
        assert_eq!(56, core.chr_bank(0x0000), "mapper {mapper}: CHR bank 7");
        assert_eq!(
            63,
            core.chr_bank(0x1C00),
            "mapper {mapper}: one 8 KiB bank over the whole range"
        );
        assert_eq!(
            [0, 1, 2, 3],
            core.prg_windows(),
            "mapper {mapper}: `CCC` is not the PRG bank"
        );

        // `P` is bit 3, above it: two 32 KiB banks of the page's 64 KiB of PRG-ROM.
        core.mapper_write(0x4100, 0x08);
        assert_eq!(
            [4, 5, 6, 7],
            core.prg_windows(),
            "mapper {mapper}: PRG bank 1"
        );
        assert_eq!(
            0,
            core.chr_bank(0x0000),
            "mapper {mapper}: and `CCC` went back to 0"
        );

        core.mapper_write(0x4100, 0x0C);
        assert_eq!([4, 5, 6, 7], core.prg_windows(), "mapper {mapper}");
        assert_eq!(32, core.chr_bank(0x0000), "mapper {mapper}: both fields");
    }
}

/// The window the page enumerates — "$4100-$41FF, $4300-$43FF, $45xx, $47xx, …, $5Dxx, and
/// $5Fxx" — which is mask $E100: A8 set, A15-A13 `010`, and nothing else decoded.
///
/// **The write at $8000 is the anti-test the census asks for.** FCEUX gives this board a CNROM
/// switch there; this build does not, because the page's own last line attributes a latch at
/// $8000-$FFFF to mapper **148** and gives this board "Bus conflicts: No" (see `Nina03`'s note).
#[cfg(feature = "board-nina03")]
#[test]
fn req_core_102_mapper_79_ignores_a_write_at_8000() {
    let mut core = MapperRom::new(79).prg(8).chr(64).load();

    // The mirrors the page lists are the same latch.
    core.mapper_write(0x4300, 0x02);
    assert_eq!(16, core.chr_bank(0x0000));
    core.mapper_write(0x5F00, 0x03);
    assert_eq!(24, core.chr_bank(0x0000));
    core.mapper_write(0x41FF, 0x01);
    assert_eq!(8, core.chr_bank(0x0000));

    // A8 clear is not the latch: $4000, $4200 and $5E00 are all outside the pattern.
    for address in [0x4000u16, 0x4200, 0x5E00] {
        core.mapper_write(address, 0x05);
        assert_eq!(8, core.chr_bank(0x0000), "{address:#06X} is no register");
    }

    // Nor is $6100: `$6100 & $E100` is `$6100`, which is why the board needs no
    // `intercept_cpu_write` body.
    core.mapper_write(0x6100, 0x05);
    assert_eq!(8, core.chr_bank(0x0000), "no register in $6000-$7FFF");

    // And the deliberate omission: no CNROM switch at $8000-$FFFF, on any of its addresses.
    for address in [0x8000u16, 0x8100, 0xC100, 0xFFFF] {
        core.mapper_write(address, 0x05);
        assert_eq!(
            8,
            core.chr_bank(0x0000),
            "{address:#06X}: FCEUX's CNROM switch is mapper 148's board, not this one"
        );
        assert_eq!([0, 1, 2, 3], core.prg_windows(), "{address:#06X}");
    }
}

/// REQ-CORE-090/091 and REQ-CORE-004 for the board: one byte, saved, restored and reset.
///
/// The sweeps `req_core_090_the_mapper_state_survives_a_save_and_restore` and
/// `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` write
/// $5000-$E001 and reach **no** register of this board, so §7.1's "save-state round-trip per
/// board with mapper state dirtied first" needs this test of its own.
#[cfg(feature = "board-nina03")]
#[test]
fn req_core_090_the_nina_03_latch_survives_a_save_and_restore() {
    for mapper in [79usize, 146] {
        let mut core = MapperRom::new(mapper).prg(8).chr(64).load();
        core.mapper_write(0x4100, 0x0A);
        assert_eq!([4, 5, 6, 7], core.prg_windows(), "mapper {mapper}");
        assert_eq!(16, core.chr_bank(0x0000), "mapper {mapper}");

        let state = core.save_state(true).unwrap();
        core.mapper_write(0x4100, 0x01);
        assert_eq!(8, core.chr_bank(0x0000), "mapper {mapper}");

        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed, "mapper {mapper}");
        assert_eq!([4, 5, 6, 7], core.prg_windows(), "mapper {mapper}");
        assert_eq!(16, core.chr_bank(0x0000), "mapper {mapper}");
        assert_eq!(state, core.save_state(true).unwrap(), "mapper {mapper}");

        core.reset();
        assert_eq!([0, 1, 2, 3], core.prg_windows(), "mapper {mapper}");
        assert_eq!(0, core.chr_bank(0x0000), "mapper {mapper}");
    }
}

// ------------------------- the seven single-latch boards of MF-UNL-3, steps 2 to 4

/// nesdev ("INES Mapper 038"), *Bank select ($7000-$7FFF)*: `[xxxx CCPP]`, bits 1-0 the 32 KiB
/// PRG bank and bits 3-2 the 8 KiB CHR bank — four of each, which is the page's 128 KiB of PRG
/// and 32 KiB of CHR (MF-UNL-3 step 2).
///
/// The $F000 half is the page's *Hardware* note — "writes to $F000-$FFFF are **very likely** to
/// also trigger a bankswitch" — implemented as the same decode and recorded in **Q-147**.
#[cfg(feature = "board-38")]
#[test]
fn req_core_102_mapper_38_banks_from_7000_and_from_f000() {
    let mut core = MapperRom::new(38).prg(16).chr(32).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));

    // `PP` is the 32 KiB PRG bank.
    core.mapper_write(0x7000, 0x03);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000), "`PP` is not the CHR bank");

    // `CC` is the 8 KiB CHR bank, two bits above it.
    core.mapper_write(0x7000, 0x0C);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(24, core.chr_bank(0x0000));
    assert_eq!(31, core.chr_bank(0x1C00), "one 8 KiB bank over the range");

    // A $F000 write is the same latch: A15 is not decoded.
    core.mapper_write(0xF000, 0x06);
    assert_eq!([8, 9, 10, 11], core.prg_windows(), "$F000 switches too");
    assert_eq!(8, core.chr_bank(0x0000));
    core.mapper_write(0xFFFF, 0x09);
    assert_eq!([4, 5, 6, 7], core.prg_windows(), "and so does $FFFF");
    assert_eq!(16, core.chr_bank(0x0000));

    // $6FFF is not: A12 is clear. Nor is $6000, which is why `board_memory_sizes` taking the
    // window's RAM away does not put a register there.
    for address in [0x6FFFu16, 0x6000, 0x8000, 0xEFFF] {
        core.mapper_write(address, 0x03);
        assert_eq!(
            [4, 5, 6, 7],
            core.prg_windows(),
            "{address:#06X} is no register"
        );
        assert_eq!(16, core.chr_bank(0x0000), "{address:#06X}");
    }
}

/// nesdev ("INES Mapper 107"): `[BBBB BBBb]`, the 8 KiB CHR bank from **all eight** bits and the
/// 32 KiB PRG bank from the upper **seven** — "unusually, bits 1-7 is used to switch both CHR and
/// PRG banks simultaneously" (MF-UNL-3 step 2).
///
/// **The eight bits are the census's own test**: "a bank number above 7 is reachable", where
/// "FCEUX implements only the bottom 3 bits of the unified register, however `MesenCE` supports
/// the full 8 bits".
#[cfg(feature = "board-107")]
#[test]
fn req_core_102_mapper_107_honours_all_eight_bits_of_its_one_register() {
    let mut core = MapperRom::new(107).prg(32).chr(128).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));

    // Bit 0 reaches the CHR bank and not the PRG bank: the two fields are one place apart.
    core.mapper_write(0x8000, 0x01);
    assert_eq!([0, 1, 2, 3], core.prg_windows(), "bit 0 is not PRG A15");
    assert_eq!(8, core.chr_bank(0x0000), "bit 0 is CHR A13");

    // Bank 2 of 32 KiB is register value 4 or 5.
    core.mapper_write(0x8000, 0x05);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(40, core.chr_bank(0x0000));

    // **Above bank 7**, which is what the three-bit reading cannot reach: CHR bank 8 of the
    // image's sixteen, and PRG bank 4 of its eight.
    core.mapper_write(0x8000, 0x08);
    assert_eq!([16, 17, 18, 19], core.prg_windows(), "PRG bank 4");
    assert_eq!(64, core.chr_bank(0x0000), "CHR bank 8");
    assert_eq!(71, core.chr_bank(0x1C00));

    // The whole of $8000-$FFFF is the register; nothing below it is.
    core.mapper_write(0xFFFF, 0x00);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    for address in [0x4100u16, 0x6000, 0x7FFF] {
        core.mapper_write(address, 0x05);
        assert_eq!(
            [0, 1, 2, 3],
            core.prg_windows(),
            "{address:#06X} is no register"
        );
    }

    // **Bit 7 needs an image whose bank count does not swallow it.** $88 and $08 differ by 128
    // in the CHR field and by 64 in the PRG field, and a power-of-two image wraps both away:
    // on the sixteen 8 KiB CHR banks above, $88 and $08 are the same bank. 192 KiB of PRG is
    // **24** 8 KiB banks and 24 KiB of CHR is 24 1 KiB banks, neither of which divides either
    // distance, so the top bit is visible in both fields at once.
    let mut wide = MapperRom::new(107).prg(24).chr(24).load();
    wide.mapper_write(0x8000, 0x08);
    assert_eq!(
        [16, 17, 18, 19],
        wide.prg_windows(),
        "PRG bank 4 of 24 banks"
    );
    assert_eq!(16, wide.chr_bank(0x0000), "CHR bank 8 of 24 banks");
    wide.mapper_write(0x8000, 0x88);
    assert_eq!(
        [8, 9, 10, 11],
        wide.prg_windows(),
        "bit 7 reached the PRG field: $88 >> 1 is bank 68"
    );
    assert_eq!(
        8,
        wide.chr_bank(0x0000),
        "and the CHR field: $88 is 8 KiB bank 136"
    );
}

/// nesdev ("INES Mapper 145") and ("INES Mapper 149"): `[C... ....]` — **bit 7** — selects the
/// 8 KiB CHR bank, at `$4100` under mask $E100 on the SA-72007 and at `$8000` under mask
/// **$8000** on the SA-0036, which "moves the register to overlap with PRG ROM and so **has bus
/// conflicts**" (MF-UNL-3 step 2).
///
/// 149's bank 1 is only reachable over a PRG bank whose bytes are `$FF`: the generic conflict
/// path of `Cartridge::cpu_write` ANDs the written value with the ROM byte, and a numbered bank
/// carries bits 0-3 alone — which is what `MapperRom::prg_bank_all_ones` exists for.
#[cfg(any(feature = "board-145", feature = "board-149"))]
#[test]
fn req_core_102_mappers_145_and_149_switch_chr_from_bit_7_alone() {
    #[cfg(feature = "board-145")]
    {
        let mut core = MapperRom::new(145).prg(4).chr(16).load();
        assert_eq!(
            [0, 1, 2, 3],
            core.prg_windows(),
            "an NROM-256 image, unbanked"
        );
        assert_eq!(0, core.chr_bank(0x0000));

        // Bit 7 and nothing else: $7F changes nothing, $80 switches.
        core.mapper_write(0x4100, 0x7F);
        assert_eq!(0, core.chr_bank(0x0000), "bits 6-0 are not the bank");
        core.mapper_write(0x4100, 0x80);
        assert_eq!(8, core.chr_bank(0x0000), "bit 7 is the bank");
        assert_eq!(15, core.chr_bank(0x1C00), "one 8 KiB bank over the range");
        assert_eq!([0, 1, 2, 3], core.prg_windows(), "no PRG bank line");

        // Mask $E100: $4300 and $5F00 are the same latch, $4000 and $6100 are not, and
        // $8000-$FFFF is not — this board's register is below $6000 and it has no conflicts.
        core.mapper_write(0x4300, 0x00);
        assert_eq!(0, core.chr_bank(0x0000));
        core.mapper_write(0x5F00, 0x80);
        assert_eq!(8, core.chr_bank(0x0000));
        for address in [0x4000u16, 0x6100, 0x8000, 0xFFFF] {
            core.mapper_write(address, 0x00);
            assert_eq!(8, core.chr_bank(0x0000), "{address:#06X} is no register");
        }
    }

    #[cfg(feature = "board-149")]
    {
        // PRG bank 1 is all `$FF`, so a write landing in $A000-$BFFF keeps its bit 7.
        let mut core = MapperRom::new(149)
            .prg(4)
            .chr(16)
            .prg_bank_all_ones(1)
            .load();
        assert_eq!(0, core.chr_bank(0x0000));

        // Mask $8000, so every address above $7FFF is the register — here $A000, over the
        // `$FF` bank, which is the only way a conflicting board latches bit 7 at all.
        core.mapper_write(0xA000, 0x80);
        assert_eq!(8, core.chr_bank(0x0000), "bit 7 over an $FF ROM byte");
        assert_eq!(15, core.chr_bank(0x1C00));
        core.mapper_write(0xBFFF, 0x00);
        assert_eq!(0, core.chr_bank(0x0000), "and back");

        // **The conflict is the board's own page**: the same write over a *numbered* bank, whose
        // bytes are 0, latches nothing.
        core.mapper_write(0xA000, 0x80);
        assert_eq!(8, core.chr_bank(0x0000));
        core.mapper_write(0x8000, 0x7F);
        assert_eq!(
            0,
            core.chr_bank(0x0000),
            "$8000 is bank 0, whose bytes are 0: the AND clears the bank"
        );

        // Nothing below $8000 is the register: A15 is the whole mask.
        core.mapper_write(0xA000, 0x80);
        assert_eq!(8, core.chr_bank(0x0000));
        for address in [0x4100u16, 0x6000, 0x7FFF] {
            core.mapper_write(address, 0x00);
            assert_eq!(8, core.chr_bank(0x0000), "{address:#06X} is no register");
        }
    }
}

/// REQ-CORE-090/091 and REQ-CORE-004 for the boards of `SingleLatch`: the latch, saved, restored
/// and reset, with the state dirtied first.
///
/// §7.1 item 2 asks for this per board, and the two generic sweeps cannot do it for three of
/// them: `req_core_090_the_mapper_state_survives_a_save_and_restore` and
/// `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` write
/// $5000-$E001, which reaches mapper 107's, 144's and 149's register but not mapper 38's $7000
/// or mapper 145's $4100.
///
/// **Mapper 143 is deliberately not in the table**: its board has no write side at all, so there
/// is no state to dirty and the `assert_ne!` below could not hold. What it has instead is
/// `req_core_102_mapper_143_answers_the_inversion_of_its_own_address_bits`, which asserts that
/// no write anywhere moves a window — and the generic sweep still round-trips its block.
#[cfg(any(
    feature = "board-38",
    feature = "board-107",
    feature = "board-144",
    feature = "board-145",
    feature = "board-149"
))]
#[test]
fn req_core_090_every_single_latch_board_round_trips_with_its_latch_dirtied() {
    // Per board: the register's address, the value to latch, and the value to dirty it with
    // afterwards — all three from the board's own page.
    let boards: &[(usize, u16, u8, u8)] = &[
        #[cfg(feature = "board-38")]
        (38, 0x7000, 0x0D, 0x02),
        #[cfg(feature = "board-107")]
        (107, 0x8000, 0x05, 0x02),
        // 144 writes into $A000-$BFFF, where the image below has an `$FF` bank: the board's own
        // conflict makes `$FF & ($32 | 1)` the latched `$33`.
        #[cfg(feature = "board-144")]
        (144, 0xA000, 0x32, 0x00),
        #[cfg(feature = "board-145")]
        (145, 0x4100, 0x80, 0x00),
        // 149 writes over PRG bank 1, which the image below fills with `$FF` so that the
        // conflicting write keeps bit 7.
        #[cfg(feature = "board-149")]
        (149, 0xA000, 0x80, 0x00),
    ];

    for &(mapper, port, dirty, later) in boards {
        let mut core = MapperRom::new(mapper)
            .prg(16)
            .chr(32)
            .prg_bank_all_ones(1)
            .load();
        core.mapper_write(port, dirty);
        let banks = (core.prg_windows(), core.chr_windows());

        let state = core.save_state(true).unwrap();
        core.mapper_write(port, later);
        assert_ne!(
            banks,
            (core.prg_windows(), core.chr_windows()),
            "mapper {mapper}: the second write has to move something, or the test is vacuous"
        );

        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed, "mapper {mapper}");
        assert_eq!(
            banks,
            (core.prg_windows(), core.chr_windows()),
            "mapper {mapper}"
        );
        assert_eq!(state, core.save_state(true).unwrap(), "mapper {mapper}");

        core.reset();
        assert_eq!(
            0,
            core.chr_bank(0x0000),
            "mapper {mapper}: a reset puts the latch back to zero"
        );
    }
}

/// nesdev ("INES Mapper 143"), the whole of the board: `A:[010. ...1 ..DD DDDD] -> [xxii iiii]`,
/// "reads provide the logical inversion of DDDDDD in the bits iiiiii", bits 7-6 open bus, and
/// **no bankswitching register at all** (MF-UNL-3 step 3).
///
/// `DDDDDD` is **the address**, bits A5-A0. The census's own test asks for "the logical
/// inversion of the **written** bits", which a board whose only chips are "a 74138 and a 74368"
/// cannot do and which this page's diagram does not say; the page wins and **Q-145** records it.
/// The test therefore walks all 64 values of A5-A0 and asserts the inversion of each, and then
/// asserts that a *write* changes nothing.
#[cfg(feature = "board-143")]
#[test]
fn req_core_102_mapper_143_answers_the_inversion_of_its_own_address_bits() {
    let mut core = MapperRom::new(143).prg(4).chr(8).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows(), "an NROM board, unbanked");

    // All 64 values of the address's low six bits, at $4100 and at one of the page's mirrors.
    for low in 0u16..64 {
        for base in [0x4100u16, 0x4140, 0x5F00] {
            let address = base | low;
            // The two bits the page calls open bus read as 0, for `Jv001`'s own reason.
            let expected = (!low as u8) & 0x3F;
            assert_eq!(
                expected,
                core.read_memory(address),
                "{address:#06X}: the inversion of A5-A0"
            );
        }
    }

    // A8 clear is not the port: `$4000 & $E100` is `$4000`, so the read is open bus — which this
    // core answers with the last value on the bus, not with the board.
    assert_ne!(
        0x3F,
        core.read_memory(0x4000),
        "A8 clear is not the protection port"
    );

    // And there is no write side: nothing a game writes anywhere moves a window.
    for address in [0x4100u16, 0x4140, 0x5F00, 0x6000, 0x7000, 0x8000, 0xFFFF] {
        core.mapper_write(address, 0x0F);
        assert_eq!(
            [0, 1, 2, 3],
            core.prg_windows(),
            "{address:#06X}: this board has no bank register"
        );
        assert_eq!(0, core.chr_bank(0x0000), "{address:#06X}");
    }

    // The protection read still answers after all of that: it is wiring and has no state.
    assert_eq!(0x3F, core.read_memory(0x4100));
    assert_eq!(0x3E, core.read_memory(0x4101));
}

/// nesdev ("INES Mapper 144"), the AGCI 50282 of *Death Race*: mapper 11's `[CCCC PPPP]`
/// register, latched through a bus conflict in which **the ROM's least significant bit always
/// wins** — "a 300 Ω resistor was added between CPU D0 and the combination of mapper hardware
/// and ROM" (MF-UNL-3 step 3).
///
/// This build takes the page's third candidate, which it calls "most succinct",
/// `EffectiveData = (ROM[address] & (WrittenData | 1))` — the same function as its first,
/// Nestopia 1.4.0's `(WrittenData & ROM[address]) | (ROM[address] & 1)`. The second, FCEUX
/// 2.1.5's "ignores writes to 0x8000", is a workaround for one address and is not taken.
///
/// **The census's own figure is not produced by any of the three** — it asks that "$FF over a
/// ROM byte of $00 yields bank $FE", where all three give `$00` — so the test pins the rule the
/// page states instead: with the ROM byte's bit 0 set and the written bit 0 clear, the effective
/// bit 0 is the **ROM's**. Q-146 records it.
#[cfg(feature = "board-144")]
#[test]
fn req_core_102_mapper_144_lets_the_roms_bit_0_win_its_bus_conflict() {
    // PRG bank 3 is all `$FF`, so a write into $E000-$FFFF conflicts against `$FF`; every other
    // bank carries its own number, whose bit 0 is the bank's.
    let mut core = MapperRom::new(144)
        .prg(16)
        .chr(32)
        .prg_bank_all_ones(3)
        .load();
    // Window 3 is that bank at power-on, so `$E000` is where a write meets `$FF`.
    assert_eq!([0, 1, 2, 255], core.prg_windows());
    assert_eq!(0, core.chr_bank(0x0000));

    // Over an `$FF` ROM byte the conflict changes nothing at all: `$FF & ($32 | 1)` is `$33`,
    // which is CHR bank 3 and PRG bank 3.
    //
    // **$E100 and not $E000**, which the first version of this test got wrong: the fixture puts
    // an endless loop and the three vectors at the start and end of the *last* PRG bank, so the
    // byte at $E000 of that bank is the `JMP`'s `$4C` and not the bank's own number.
    core.mapper_write(0xE100, 0x32);
    assert_eq!([12, 13, 14, 15], core.prg_windows(), "PRG bank 3 of four");
    assert_eq!(24, core.chr_bank(0x0000), "CHR bank 3 of four");

    // **The rule.** $E100 now shows numbered bank 15, whose bytes are `$0F`, and the written
    // value's bit 0 is clear: a plain Color Dreams AND would give `$0F & $30` = `$00` and PRG
    // bank 0, and the resistor gives `$0F & ($30 | 1)` = `$01` and PRG bank **1**.
    core.mapper_write(0xE100, 0x30);
    assert_eq!(
        [4, 5, 6, 7],
        core.prg_windows(),
        "the ROM's bit 0 won: bank 1, where an unmodified mapper 11 would show bank 0"
    );
    assert_eq!(
        0,
        core.chr_bank(0x0000),
        "the ROM byte masked the CHR field"
    );

    // Bits 7-1 are the ordinary AND, and the numbered banks show it: $8100 is bank 4, whose
    // bytes are `$04`, so a written `$FF` latches `$04 & $FF` = `$04` — PRG bank 0, not 3.
    core.mapper_write(0x8100, 0xFF);
    assert_eq!(
        [0, 1, 2, 255],
        core.prg_windows(),
        "$FF became the ROM's own $04"
    );
    assert_eq!(0, core.chr_bank(0x0000));

    // Nothing below $8000 is the register.
    for address in [0x4100u16, 0x6000, 0x7FFF] {
        core.mapper_write(address, 0x32);
        assert_eq!(
            [0, 1, 2, 255],
            core.prg_windows(),
            "{address:#06X} is no register"
        );
    }
}

/// nesdev ("INES Mapper 190"), the Zemina board of *Magic Kid Googoo* — the one board of this
/// lot's seven with more than one window (MF-UNL-3 step 4):
///
/// ```text
/// CPU $6000-$7FFF:  8 KiB RAM
/// CPU $8000-$BFFF:  16 KiB switchable PRG ROM bank
/// CPU $C000-$FFFF:  16 KiB PRG ROM, fixed to the FIRST bank
/// PPU $0000-$1FFF:  four 2 KiB switchable CHR ROM banks
///
/// CHR bank select ($A000-$BFFF):  A~[101. .... .... ..bb]  D~[CCCC CCCC]
/// PRG bank select ($8000-$9FFF, $C000-$DFFF):  A~[1P0. .... .... ....]  D~[.... .PPP]
/// ```
///
/// Three things this test exists to pin, each of which an implementation could plausibly get
/// wrong: the fixed window is the **first** bank and not the last; `P` is an **address** line
/// (A14), so the same data byte selects two different banks from the two ports; and `bb` is the
/// address's A1-A0, so one byte reaches one 2 KiB window.
#[cfg(feature = "board-190")]
#[test]
fn req_core_102_mapper_190_banks_three_windows_from_two_ports() {
    let mut core = MapperRom::new(190).prg(32).chr(64).load();

    // Power-on: PRG bank 0 at $8000-$BFFF and the **first** bank fixed at $C000-$FFFF, so the
    // four 8 KiB windows are 0, 1, 0, 1 — which is what separates this board from every
    // UxROM-like one in this build, where they would be 0, 1, 30, 31.
    assert_eq!([0, 1, 0, 1], core.prg_windows());
    // All four CHR registers power on at 0, so each 2 KiB window shows 2 KiB bank 0 — the eight
    // 1 KiB windows are four copies of banks 0 and 1 and not a linear layout.
    assert_eq!([0, 1, 0, 1, 0, 1, 0, 1], core.chr_windows());

    // `[.... .PPP]` at $8000-$9FFF: three bits, the low eight 16 KiB banks.
    core.mapper_write(0x8000, 0x05);
    assert_eq!([10, 11, 0, 1], core.prg_windows(), "16 KiB bank 5");
    core.mapper_write(0x9FFF, 0x07);
    assert_eq!([14, 15, 0, 1], core.prg_windows(), "the whole port, bank 7");

    // **`P` is A14.** The same data byte written to $C000-$DFFF selects bank 8 + 7 = 15, not 7.
    core.mapper_write(0xC000, 0x07);
    assert_eq!(
        [30, 31, 0, 1],
        core.prg_windows(),
        "bank 15: `P` came from A14"
    );
    core.mapper_write(0xD000, 0x00);
    assert_eq!([16, 17, 0, 1], core.prg_windows(), "bank 8, not bank 0");

    // A13 set is neither port, so $E000-$FFFF and $A000's PRG field reach nothing.
    core.mapper_write(0xE000, 0x03);
    assert_eq!([16, 17, 0, 1], core.prg_windows(), "$E000 is no PRG port");
    core.mapper_write(0xFFFF, 0x03);
    assert_eq!([16, 17, 0, 1], core.prg_windows(), "nor is $FFFF");

    // `bb` is A1-A0 of the address, and the byte is a whole 2 KiB bank number: four ports, four
    // windows, each of two 1 KiB halves.
    core.mapper_write(0xA000, 0x04);
    assert_eq!([8, 9, 0, 1, 0, 1, 0, 1], core.chr_windows(), "window 0");
    core.mapper_write(0xA001, 0x05);
    assert_eq!([8, 9, 10, 11, 0, 1, 0, 1], core.chr_windows(), "window 1");
    core.mapper_write(0xA002, 0x06);
    assert_eq!([8, 9, 10, 11, 12, 13, 0, 1], core.chr_windows(), "window 2");
    core.mapper_write(0xBFFF, 0x07);
    assert_eq!(
        [8, 9, 10, 11, 12, 13, 14, 15],
        core.chr_windows(),
        "window 3, from $BFFF — the port is the whole of $A000-$BFFF"
    );

    // The CHR port is not the PRG port: A13 set is what tells them apart.
    assert_eq!(
        [16, 17, 0, 1],
        core.prg_windows(),
        "the PRG bank did not move"
    );

    // Nothing below $8000 is a register: the board's $6000-$7FFF is RAM, which
    // `board_memory_sizes` gives it on the page's own "CPU $6000-$7FFF: 8 KiB RAM".
    core.write_memory(0x6000, 0x42);
    assert_eq!(0x42, core.read_memory(0x6000), "8 KiB of RAM at $6000");
    for address in [0x4100u16, 0x6000, 0x7FFF] {
        core.mapper_write(address, 0x03);
        assert_eq!(
            [16, 17, 0, 1],
            core.prg_windows(),
            "{address:#06X} is no register"
        );
    }
}

/// REQ-CORE-090/091 and REQ-CORE-004 for mapper 190, the one board of `SingleLatch` whose state
/// is five bytes and not one: the 16 KiB PRG bank **and** the four 2 KiB CHR registers
/// (MF-UNL-3 step 4).
///
/// It is not in `req_core_090_every_single_latch_board_round_trips_with_its_latch_dirtied`'s
/// table because that table is one port and one byte per board. The four CHR registers are what
/// this test exists for: it dirties all five, saves, dirties all five differently, loads, and
/// requires every window back — which is the shape MF-UNL-2 step 6 found necessary when a
/// round-trip that only saves and loads passed on a board whose appended byte never reached the
/// state at all.
#[cfg(feature = "board-190")]
#[test]
fn req_core_090_mapper_190_round_trips_its_prg_bank_and_its_four_chr_registers() {
    let mut core = MapperRom::new(190).prg(32).chr(64).load();
    core.mapper_write(0xC000, 0x03); // PRG bank 11, through the high port.
    for slot in 0u16..4 {
        core.mapper_write(0xA000 | slot, 0x0C + slot as u8);
    }
    let banks = (core.prg_windows(), core.chr_windows());
    assert_eq!([22, 23, 0, 1], banks.0);
    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], banks.1);

    let state = core.save_state(true).unwrap();

    // Dirty all five differently, so that a field missing from the block cannot pass.
    core.mapper_write(0x8000, 0x01);
    for slot in 0u16..4 {
        core.mapper_write(0xA000 | slot, slot as u8);
    }
    assert_ne!(banks, (core.prg_windows(), core.chr_windows()));

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(banks, (core.prg_windows(), core.chr_windows()));
    assert_eq!(state, core.save_state(true).unwrap());

    core.reset();
    assert_eq!(
        [0, 1, 0, 1],
        core.prg_windows(),
        "bank 0, and the first bank fixed"
    );
    assert_eq!([0, 1, 0, 1, 0, 1, 0, 1], core.chr_windows());
}

// ------------------------------------- the J.Y. Company ASIC (35, 90, 209, 211), MF-UNL-4

/// The 1 KiB CHR bank each of the eight windows shows, straight off the window table.
///
/// `chr_windows()` reads the *first byte* of each window, and the numbered-bank fixture fills
/// 1 KiB bank `b` with `b as u8` — so banks 0 and 256 are indistinguishable through it, while
/// this board's outer CHR bank register reaches bank 512. `Cartridge::chr_peek_view` carries the
/// raw window offsets, which is the only way to see them.
#[cfg(feature = "board-jy-asic")]
fn chr_banks(core: &NesCore) -> [usize; 8] {
    core.cartridge()
        .unwrap()
        .chr_peek_view()
        .unwrap()
        .offsets()
        .map(|offset| offset / 0x0400)
}

/// The 8×8→16 hardware multiplier of `$5800`-`$5801`, table-driven as the census asks.
///
/// "$5800 write: 8-bit Unsigned Multiplicand / $5801 write: 8-bit Unsigned Multiplier (and begin
/// multiplying) / $5800 read: Unsigned 16-bit Product (low byte) / $5801 read: … (high byte)".
/// Unsigned: $FF × $FF is $FE01 and not $0001.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_multiplies_two_bytes_into_a_sixteen_bit_product() {
    let mut core = MapperRom::new(209).prg(64).load();
    for (a, b) in [
        (0x00u8, 0x00u8),
        (0x01, 0x01),
        (0x12, 0x34),
        (0x10, 0x10),
        (0xFF, 0x01),
        (0xFF, 0xFF),
        (0x80, 0x02),
        (0x7F, 0x7F),
    ] {
        core.mapper_write(0x5800, a);
        core.mapper_write(0x5801, b);
        let product = u16::from(a) * u16::from(b);
        assert_eq!(
            product & 0x00FF,
            u16::from(core.read_memory(0x5800)),
            "{a:#04X} * {b:#04X} low"
        );
        assert_eq!(
            product >> 8,
            u16::from(core.read_memory(0x5801)),
            "{a:#04X} * {b:#04X} high"
        );
    }

    // Mask $F803: A2-A10 are don't-care, so $5804 and $5BFD are $5800 and $5801 again.
    core.mapper_write(0x5804, 0x03);
    core.mapper_write(0x5BFD, 0x05);
    assert_eq!(15, core.read_memory(0x5FFC), "mask $F803 on the product");
}

/// `$5802`-`$5803`, the accumulator and the test register. Mesen 2 implements neither.
///
/// "$5802 write: Increase accumulator by written value / $5803 write: Reset accumulator to 0 and
/// set test register to written value / $5802 read: Read accumulator value / $5803 read: Read
/// test register".
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_accumulates_and_keeps_a_test_register() {
    let mut core = MapperRom::new(209).prg(64).load();
    assert_eq!(
        0,
        core.read_memory(0x5802),
        "the accumulator powers on at 0"
    );
    assert_eq!(0, core.read_memory(0x5803));

    core.mapper_write(0x5802, 0x10);
    core.mapper_write(0x5802, 0x05);
    assert_eq!(0x15, core.read_memory(0x5802), "increased, not replaced");

    // Eight bits wide, so it wraps rather than saturating.
    core.mapper_write(0x5802, 0xFF);
    assert_eq!(0x14, core.read_memory(0x5802));

    core.mapper_write(0x5803, 0x7E);
    assert_eq!(0, core.read_memory(0x5802), "$5803 resets the accumulator");
    assert_eq!(0x7E, core.read_memory(0x5803), "and sets the test register");
}

/// The board solder pad register, and the one place the page's two masks overlap.
///
/// It is "Mask: $FFFF" at exactly `$5000`, `$5400` and `$5C00` — "Apparently, the address of this
/// register is fully decoded, as the SC-128 version of *Final Fight 3*, which otherwise happily
/// randomizes the masked-off address bits, keeps A0-A11 constantly at zero to read from this
/// register" — while the multiplier's is `$F803`, under which `$5C00` is also `$5800`. The fully
/// decoded register wins for those three addresses, which is what this test pins: Mesen 2 decodes
/// `$5C00` as the product's low byte.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_solder_pad_register_is_fully_decoded() {
    let mut core = MapperRom::new(209).prg(64).load();
    core.mapper_write(0x5800, 0x10);
    core.mapper_write(0x5801, 0x10);
    assert_eq!(0x00, core.read_memory(0x5800), "$0100 low");
    assert_eq!(0x01, core.read_memory(0x5801), "$0100 high");

    // The three fully decoded addresses answer the pads, which a dump does not record: 0.
    for address in [0x5000u16, 0x5400, 0x5C00] {
        assert_eq!(0, core.read_memory(address), "{address:#06X} is the pads");
    }

    // One address along, the $F803 mask is back in charge of the $5800 block.
    assert_eq!(0x00, core.read_memory(0x5C04), "$5C04 is $5800 again");
    assert_eq!(0x01, core.read_memory(0x5C05), "$5C05 is $5801");
}

/// The four PRG banking modes of `$D000` bits 1-0, with the last window switchable.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_has_four_prg_banking_modes() {
    let mut core = MapperRom::new(209).prg(64).load();

    // Power-on: mode 0, 32 KiB, and bit 2 clear so the window is "hard-wired to the last bank
    // within the outer bank" — the last 32 KiB of 512 KiB.
    assert_eq!([60, 61, 62, 63], core.prg_windows());

    // Mode 0, switchable: "$8003: Select 32 KiB PRG-ROM bank at CPU $8000-$FFFF".
    core.mapper_write(0xD000, 0x04);
    core.mapper_write(0x8003, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows(), "32 KiB bank 1");

    // Mode 1, 16 KiB: "$8001: … at CPU $8000-$BFFF" and "$8003: … at CPU $C000-$FFFF".
    core.mapper_write(0xD000, 0x05);
    core.mapper_write(0x8001, 0x02);
    core.mapper_write(0x8003, 0x03);
    assert_eq!([4, 5, 6, 7], core.prg_windows(), "16 KiB banks 2 and 3");

    // Mode 2, 8 KiB: one register per window.
    core.mapper_write(0xD000, 0x06);
    for slot in 0u16..4 {
        core.mapper_write(0x8000 | slot, 0x11 + slot as u8);
    }
    assert_eq!(
        [17, 18, 19, 20],
        core.prg_windows(),
        "8 KiB, four registers"
    );

    // Mode 3 is mode 2 with the bank numbers' bits 0-6 reversed; see the Q-150 test. $11 is
    // 001 0001, which reversed is 100 0100 = $44, masked to the outer bank's six bits = 4; $12
    // gives 36, $13 gives 100 -> 36 again, and $14 gives 20.
    core.mapper_write(0xD000, 0x07);
    assert_eq!(
        [4, 36, 36, 20],
        core.prg_windows(),
        "the same four registers, reversed"
    );
}

/// `$D000` bit 2 clear: "Hard-wired to the last bank", and the page's "within the outer bank" is
/// what makes it the 512 KiB window's last and not the image's.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_fixes_the_last_window_to_the_outer_banks_last() {
    let mut core = MapperRom::new(209).prg(128).load();

    // A 1 MiB image, so the outer bank matters: with $D003 bits 2-1 clear the hard-wired bank is
    // 63 and not 127 — the last of the **first** 512 KiB.
    core.mapper_write(0xD000, 0x02); // mode 2, hard-wired.
    core.mapper_write(0x8000, 0x05);
    core.mapper_write(0x8001, 0x06);
    core.mapper_write(0x8002, 0x07);
    core.mapper_write(0x8003, 0x08); // ignored while the window is hard-wired.
    assert_eq!([5, 6, 7, 63], core.prg_windows());

    core.mapper_write(0xD000, 0x01); // mode 1, hard-wired: the last 16 KiB.
    assert_eq!([12, 13, 62, 63], core.prg_windows());

    core.mapper_write(0xD000, 0x00); // mode 0, hard-wired: the last 32 KiB.
    assert_eq!([60, 61, 62, 63], core.prg_windows());

    // $D003 bits 2-1 move all of it into the second 512 KiB, the hard-wired bank included.
    core.mapper_write(0xD003, 0x02);
    assert_eq!([124, 125, 126, 127], core.prg_windows(), "outer PRG bank 1");
    // And with bit 2 set the last window follows $8003 into that outer bank too.
    core.mapper_write(0xD000, 0x06);
    assert_eq!([69, 70, 71, 72], core.prg_windows());
}

/// **Q-150**: "3: 8 KiB banking, but with bank numbers **bits 0-6 reversed**", and bit 3 is one of
/// those seven bits.
///
/// Mesen 2's `InvertPrgBits` maps bits 0, 1 and 2 to 6, 5 and 4 and bits 4, 5 and 6 to 2, 1 and 0,
/// and **drops bit 3 entirely** — a value of `$08` selects bank 0 there and bank 8 here. A
/// seven-bit reversal that left the middle bit out would not be a reversal, so the page wins.
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_150_jy_asic_prg_mode_3_reverses_bit_3_onto_itself() {
    let mut core = MapperRom::new(209).prg(64).load();
    core.mapper_write(0xD000, 0x07); // mode 3, switchable last window.

    // One bit at a time, through the window at $8000: bit n becomes bit 6-n.
    let cases: [(u8, usize); 7] = [
        (0x40, 1),
        (0x20, 2),
        (0x10, 4),
        (0x08, 8), // the bit Mesen 2 drops.
        (0x04, 16),
        (0x02, 32),
        // Bit 0 becomes bit 6, which is 64 — outside the 64-bank outer window, so the page's
        // "always masked to the outer bank size" makes it bank 0.
        (0x01, 0),
    ];
    for (written, expected) in cases {
        core.mapper_write(0x8000, written);
        assert_eq!(
            expected as u8,
            core.prg_windows()[0],
            "mode 3, {written:#04X} reversed"
        );
    }

    // And a palindrome in seven bits comes back as itself: $2A is 010 1010 either way round.
    core.mapper_write(0x8000, 0x2A);
    assert_eq!(0x2A & 0x3F, core.prg_windows()[0]);
}

/// "The PRG-ROM bank selected via $8000-$8003 is always masked to the outer bank size (512/256/128
/// KiB, depending on PCB/mapper number). Accessing more than that requires manipulating the Outer
/// Bank register at $D003."
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_masks_the_prg_bank_to_the_outer_bank() {
    let mut core = MapperRom::new(209).prg(128).load();
    core.mapper_write(0xD000, 0x06); // 8 KiB, switchable.

    // Sixty-four 8 KiB banks make the 512 KiB outer bank, so bit 6 of the register is masked off
    // rather than reaching PRG A19.
    core.mapper_write(0x8000, 0x40);
    assert_eq!(0, core.prg_windows()[0], "bank 64 is masked to bank 0");
    core.mapper_write(0x8000, 0x41);
    assert_eq!(1, core.prg_windows()[0]);

    // $D003 bits 2-1 are what reaches A19-A20.
    core.mapper_write(0xD003, 0x02);
    assert_eq!(65, core.prg_windows()[0], "outer bank 1 plus bank 1");
}

/// The four CHR banking modes of `$D000` bits 4-3.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_has_four_chr_banking_modes() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();

    // Power-on: mode 0, one 8 KiB bank, register 0.
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
    core.mapper_write(0x9000, 0x01);
    assert_eq!([8, 9, 10, 11, 12, 13, 14, 15], core.chr_windows(), "8 KiB");

    // Mode 1, two 4 KiB banks: "$9000/$A000 … at PPU $0000-$0FFF" and "$9004/$A004 … at PPU
    // $1000-$1FFF" with the MMC4-like mode off.
    core.mapper_write(0xD000, 0x08);
    core.mapper_write(0x9000, 0x01);
    core.mapper_write(0x9004, 0x03);
    assert_eq!([4, 5, 6, 7, 12, 13, 14, 15], core.chr_windows(), "4 KiB");

    // Mode 2, four 2 KiB banks, from the even registers.
    core.mapper_write(0xD000, 0x10);
    for window in 0u16..4 {
        core.mapper_write(0x9000 | (window * 2), 0x04 + window as u8);
    }
    assert_eq!([8, 9, 10, 11, 12, 13, 14, 15], core.chr_windows(), "2 KiB");

    // Mode 3, eight 1 KiB banks, from all eight.
    core.mapper_write(0xD000, 0x18);
    for window in 0u16..8 {
        core.mapper_write(0x9000 | window, 0x20 + window as u8);
    }
    assert_eq!(
        [32, 33, 34, 35, 36, 37, 38, 39],
        core.chr_windows(),
        "1 KiB"
    );
}

/// `$D003`'s outer CHR bank: `L` picks the mask, `cc` is CHR A19-A20 and `C` is CHR A18.
///
/// "0: Mask $900x/$A00x to 256 KiB, use C / 1: Mask $900x/$A00x to 512 KiB, ignore C", with the
/// `$A00x` register the MSB of the bank number. Mesen 2 drops both the mask and the outer bank in
/// the `L=1` case and uses the raw sixteen-bit register instead (Q-150).
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_outer_chr_bank_picks_its_own_mask() {
    let mut core = MapperRom::new(209).prg(64).chr(640).load();
    core.mapper_write(0xD000, 0x18); // 1 KiB banking, so a register is a 1 KiB bank.

    // L clear: the mask is 256 banks, so the MSB register cannot reach past it…
    core.mapper_write(0x9000, 0x05);
    core.mapper_write(0xA000, 0x01);
    assert_eq!(5, chr_banks(&core)[0], "masked to 256 KiB");

    // …and `C` is what reaches CHR A18 instead.
    core.mapper_write(0xD003, 0x01);
    assert_eq!(256 + 5, chr_banks(&core)[0], "C is CHR A18");

    // L set: the mask is 512 banks, the MSB reaches bank bit 8, and `C` is ignored.
    core.mapper_write(0xD003, 0x21);
    assert_eq!(256 + 5, chr_banks(&core)[0], "the MSB, and C ignored");
    core.mapper_write(0xA000, 0x00);
    assert_eq!(5, chr_banks(&core)[0]);

    // `cc` is CHR A19-A20 in both halves.
    core.mapper_write(0xD003, 0x28); // L set, cc = 1.
    assert_eq!(512 + 5, chr_banks(&core)[0], "cc with L set");
    core.mapper_write(0xD003, 0x08); // L clear, cc = 1, C clear.
    assert_eq!(512 + 5, chr_banks(&core)[0], "cc with L clear");
}

/// The `$F803` and `$F807` masks: "writes are ignored if address bit 11 is set".
///
/// The page states it for `$8000`-`$8003`, `$9000`-`$A007`, `$B000`-`$B007` and `$D000`-`$D003`
/// separately and says the opposite for `$C000`-`$C007` — "Mask: $F007. Writes are accepted even
/// if address bit 11 is set" — so each window is tested on its own.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_ignores_a_write_with_address_bit_11_set() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    core.mapper_write(0xD000, 0x1E); // 8 KiB PRG switchable, 1 KiB CHR.
    core.mapper_write(0x8000, 0x11);
    core.mapper_write(0x9000, 0x22);
    let banks = (core.prg_windows(), core.chr_windows());

    for address in [0x8800u16, 0x8FFF, 0x9800, 0xA800, 0xB800, 0xD800, 0xDFFF] {
        core.mapper_write(address, 0x7F);
        assert_eq!(
            banks,
            (core.prg_windows(), core.chr_windows()),
            "{address:#06X} has A11 set and must be ignored"
        );
    }

    // And the low half of each block is the register, A2-A10 being don't-care on $8000-$8003.
    core.mapper_write(0x87FC, 0x05);
    assert_eq!(5, core.prg_windows()[0], "$87FC is $8000");
    core.mapper_write(0x97F8, 0x06);
    assert_eq!(6, core.chr_windows()[0], "$97F8 is $9000");
}

/// `$D000` bit 7: "1: Map 8 KiB PRG-ROM bank selected via $8003 (appropriately shifted and
/// right-filled with binary 1 in 16/32 KiB PRG modes)".
///
/// The page's three formulas, one per mode: "$8003 in 8 KiB banking mode; $8003 SHL 1 OR 1 in
/// 16 KiB banking mode; $8003 SHL 2 OR 3 in 32 KiB banking mode". With the bit clear the window
/// is WRAM, which `board_memory_sizes` gives every one of the four numbers.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_maps_a_prg_rom_bank_at_6000_on_demand() {
    let mut core = MapperRom::new(209).prg(64).load();

    // Bit 7 clear: 8 KiB of WRAM, readable and writable.
    core.write_memory(0x6000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6000), "WRAM while bit 7 is clear");

    core.mapper_write(0x8003, 0x09);
    core.mapper_write(0xD000, 0x86); // 8 KiB mode, switchable, ROM at $6000.
    assert_eq!(9, core.read_memory(0x6000), "$8003 in 8 KiB mode");
    core.write_memory(0x6000, 0xFF);
    assert_eq!(9, core.read_memory(0x6000), "and ROM ignores a write");

    core.mapper_write(0xD000, 0x85); // 16 KiB mode: SHL 1 OR 1.
    assert_eq!(9 * 2 + 1, core.read_memory(0x6000));

    core.mapper_write(0xD000, 0x84); // 32 KiB mode: SHL 2 OR 3.
    assert_eq!((9 * 4 + 3) & 0x3F, core.read_memory(0x6000));

    // Bit 7 clear again, and the WRAM is where it was: the window moved, not the chip.
    core.mapper_write(0xD000, 0x06);
    assert_eq!(0x5A, core.read_memory(0x6000));
}

/// **Q-148**: mapper 35 is mapper 209 "but explicitly specifies 8 KiB of WRAM", and that sentence
/// is the whole of the number.
///
/// Mesen 2 builds a wholly different and much smaller board for 35 — four PRG registers, eight
/// CHR registers, an MMC3-style *filtered* A12 counter and a mirroring bit, with no multiplier, no
/// banking modes and no nametable control. The page says the two numbers are "an exact duplicate",
/// so this build gives 35 the same board and the difference lands in `board_memory_sizes`.
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_148_jy_asic_mapper_35_is_mapper_209_with_8_kib_of_wram() {
    let mut m35 = MapperRom::new(35).prg(64).load();
    let mut m209 = MapperRom::new(209).prg(64).load();

    // The same board: the multiplier the page gives the chip and Mesen 2's mapper 35 has not.
    for core in [&mut m35, &mut m209] {
        core.mapper_write(0x5800, 0x0F);
        core.mapper_write(0x5801, 0x11);
        assert_eq!(0xFF, core.read_memory(0x5800));
        assert_eq!(0x00, core.read_memory(0x5801));
    }

    // And the same banking modes, which Mesen 2's mapper 35 also has not.
    for core in [&mut m35, &mut m209] {
        core.mapper_write(0xD000, 0x05);
        core.mapper_write(0x8001, 0x02);
        core.mapper_write(0x8003, 0x03);
        assert_eq!([4, 5, 6, 7], core.prg_windows());
    }

    // 8 KiB of WRAM, stated by the number rather than defaulted by the header: the whole window
    // holds distinct bytes, so it is 8 KiB of chip and not a smaller one mirrored.
    for (address, value) in [(0x6000u16, 0x3C), (0x6FFF, 0x5A), (0x7FFF, 0xA5)] {
        m35.write_memory(address, value);
    }
    assert_eq!(
        [0x3C, 0x5A, 0xA5],
        [0x6000u16, 0x6FFF, 0x7FFF].map(|address| m35.read_memory(address))
    );
}

/// REQ-CORE-090/091 and REQ-CORE-004 for the J.Y. Company ASIC, whose state is the largest of any
/// board in `IrqBoard`: twenty-eight register bytes, some thirty mode bits, the two CHR latches,
/// the IRQ and the three arithmetic registers.
///
/// Dirtied first, as `docs/review/BOARD-FEATURES.md` requires, and dirtied **differently** after
/// the save, which is the shape MF-UNL-2 step 6 found necessary: a round-trip that only saves and
/// loads passes on a board whose appended byte never reached the state at all.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_090_jy_asic_round_trips_its_whole_register_set() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();

    core.mapper_write(0xD000, 0x1E); // 8 KiB PRG switchable, 1 KiB CHR.
    for slot in 0u16..4 {
        core.mapper_write(0x8000 | slot, 0x05 + slot as u8);
    }
    for slot in 0u16..8 {
        core.mapper_write(0x9000 | slot, 0x10 + slot as u8);
        core.mapper_write(0xA000 | slot, 0x00);
    }
    for slot in 0u16..4 {
        core.mapper_write(0xB000 | slot, 0x80 | slot as u8);
        core.mapper_write(0xB004 | slot, 0x02);
    }
    core.mapper_write(0xD001, 0x09); // Extended Mirroring, arrangement 1.
    core.mapper_write(0xD002, 0xC0); // CHR write-enable and the ROM select inverted.
    core.mapper_write(0xD003, 0x29); // L, cc, PP and C — but not the MMC4 mode.
    core.mapper_write(0x5800, 0x12);
    core.mapper_write(0x5801, 0x34);
    core.mapper_write(0x5802, 0x56);
    core.mapper_write(0xC006, 0x0F); // the XOR register.
    core.mapper_write(0xC004, 0xF0);
    core.mapper_write(0xC005, 0x3C);
    core.mapper_write(0xC001, 0x4F);
    core.mapper_write(0xC003, 0x00);

    let before = (
        core.prg_windows(),
        chr_banks(&core),
        nametable_pages(&core),
        core.read_memory(0x5800),
        core.read_memory(0x5801),
        core.read_memory(0x5802),
    );
    let state = core.save_state(true).unwrap();

    // Dirty every group differently.
    core.mapper_write(0xD000, 0x00);
    core.mapper_write(0xD001, 0x00);
    core.mapper_write(0xD002, 0x00);
    core.mapper_write(0xD003, 0x00);
    for slot in 0u16..4 {
        core.mapper_write(0x8000 | slot, 0x3F);
        core.mapper_write(0xB000 | slot, 0x00);
    }
    for slot in 0u16..8 {
        core.mapper_write(0x9000 | slot, 0x00);
    }
    core.mapper_write(0x5800, 0x00);
    core.mapper_write(0x5801, 0x00);
    core.mapper_write(0x5803, 0x00);
    core.mapper_write(0xC002, 0x00);
    assert_ne!(before.0, core.prg_windows());

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(
        before,
        (
            core.prg_windows(),
            chr_banks(&core),
            nametable_pages(&core),
            core.read_memory(0x5800),
            core.read_memory(0x5801),
            core.read_memory(0x5802),
        )
    );
    assert_eq!(state, core.save_state(true).unwrap());

    // REQ-CORE-004: every register back to zero, which is 32 KiB PRG mode with the window
    // hard-wired to the outer bank's last 32 KiB and one 8 KiB CHR bank.
    core.reset();
    assert_eq!([60, 61, 62, 63], core.prg_windows());
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
    assert_eq!(Nm::Vertical, core.cartridge().unwrap().mirroring());
    assert_eq!(0, core.read_memory(0x5802));
}

/// `$D001` bits 1-0, and the one place the page's vocabulary is this core's reversed.
///
/// The page names *arrangements* — "0: Horizontal (A10), 1: Vertical (A11), 2: One-screen, page 0
/// (GND), 3: One-screen, page 1 (VCC)" — where [`Nm`] names *mirrorings*, which are each other's
/// opposites: a horizontal arrangement driven from A10 puts `$2000` and `$2400` on different
/// pages, which is [`Nm::Vertical`]. Mesen 2 reads the two bits the same way round.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_arrangement_bits_are_this_cores_mirroring_reversed() {
    let mut core = MapperRom::new(209).prg(64).load();
    for (bits, expected) in [
        (0x00u8, Nm::Vertical),
        (0x01, Nm::Horizontal),
        (0x02, Nm::SingleScreenA),
        (0x03, Nm::SingleScreenB),
    ] {
        core.mapper_write(0xD001, bits);
        assert_eq!(
            expected,
            core.cartridge().unwrap().mirroring(),
            "arrangement {bits}"
        );
    }
}

/// `$D002` bit 6, the CHR write-enable: "0: disabled, CHR memory is write-protected / 1: enabled,
/// CHR memory is write-enabled", and "Bit 6 only matters when using CHR-RAM".
///
/// **Power-on is write-protected**, every register being zero, so a CHR-RAM image of one of these
/// numbers has to ask for the RAM before it can fill it. Mesen 2 does not decode the bit at all.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_chr_write_enable_gates_chr_ram() {
    let mut core = MapperRom::new(209).prg(64).chr(0).load();
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.ppu_write(0x0000, 0x5A);
    assert_eq!(
        0,
        cartridge.ppu_peek(0x0000, ChrFetch::Background),
        "write-protected at power-on"
    );

    core.mapper_write(0xD002, 0x40);
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.ppu_write(0x0000, 0x5A);
    assert_eq!(
        0x5A,
        cartridge.ppu_peek(0x0000, ChrFetch::Background),
        "write-enabled"
    );

    core.mapper_write(0xD002, 0x00);
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.ppu_write(0x0000, 0x00);
    assert_eq!(
        0x5A,
        cartridge.ppu_peek(0x0000, ChrFetch::Background),
        "write-protected again"
    );
}

/// Drives the board's MMC4-like latch the way a pattern fetch does: through the PPU address bus,
/// which is where the page puts it and where `IrqBoard` already forwards the hook.
#[cfg(feature = "board-jy-asic")]
fn jy_fetch(core: &mut NesCore, address: u16) {
    core.cartridge_mut().unwrap().on_ppu_address_bus(address, 0);
}

/// The census's own test: "a fetch of the latch tile switches the bank for subsequent fetches, and
/// the latch is per pattern table".
///
/// "In 4 KiB CHR banking mode with MMC4 mode on: $9000/$A000 … when latch =0, $9002/$A002 … when
/// latch =1" for PPU `$0000`-`$0FFF`, and `$9004`/`$9006` the same way for `$1000`-`$1FFF`; "the
/// latch is set to 0 when the PPU reads from $0FD8-$0FDF/$1FD8-$1FDF and to 1 when the PPU reads
/// from $0FE8-$0FEF/$1FE8-$1FEF".
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_039_jy_asic_mmc4_latch_picks_one_of_two_banks_per_pattern_table() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    core.mapper_write(0xD000, 0x08); // 4 KiB CHR banking.
    core.mapper_write(0xD003, 0x80); // MMC4-like mode on.
    core.mapper_write(0x9000, 0x01); // $0000 with latch 0: 4 KiB bank 1.
    core.mapper_write(0x9002, 0x02); // $0000 with latch 1: bank 2.
    core.mapper_write(0x9004, 0x03); // $1000 with latch 0: bank 3.
    core.mapper_write(0x9006, 0x04); // $1000 with latch 1: bank 4.

    // Power-on selects the latch-0 registers, which is also the layout the mode-off case gives.
    assert_eq!([4, 12], [peek(&core, 0x0000), peek(&core, 0x1000)]);

    // The low window's latch, and only it.
    jy_fetch(&mut core, 0x0FE8);
    assert_eq!([8, 12], [peek(&core, 0x0000), peek(&core, 0x1000)], "low 1");
    jy_fetch(&mut core, 0x0FD8);
    assert_eq!([4, 12], [peek(&core, 0x0000), peek(&core, 0x1000)], "low 0");

    // The high window's, and only it.
    jy_fetch(&mut core, 0x1FE8);
    assert_eq!(
        [4, 16],
        [peek(&core, 0x0000), peek(&core, 0x1000)],
        "high 1"
    );
    jy_fetch(&mut core, 0x1FD8);
    assert_eq!(
        [4, 12],
        [peek(&core, 0x0000), peek(&core, 0x1000)],
        "high 0"
    );
}

/// The latch ranges are eight addresses wide, not one: "$0FD8-$0FDF" and "$0FE8-$0FEF", which is
/// tile `$FD`'s and tile `$FE`'s second bit plane, all eight rows.
///
/// That is the MMC4's rule and not the MMC2's — mapper 9 reacts to the single address `$0FD8` —
/// and it is why [`JyCompany`]'s `advance_chr_latch` is the MMC4's body.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_039_jy_asic_latch_range_is_the_whole_row_of_the_tile() {
    for low in 0u16..8 {
        let mut core = MapperRom::new(209).prg(64).chr(64).load();
        core.mapper_write(0xD000, 0x08);
        core.mapper_write(0xD003, 0x80);
        core.mapper_write(0x9002, 0x02);
        jy_fetch(&mut core, 0x0FE8 | low);
        assert_eq!(8, peek(&core, 0x0000), "$0FE8 + {low} must latch");
    }

    // And an address outside the two ranges does not: $0FF8 is tile $FF's second plane.
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    core.mapper_write(0xD000, 0x08);
    core.mapper_write(0xD003, 0x80);
    core.mapper_write(0x9002, 0x02);
    for address in [0x0FC8u16, 0x0FD0, 0x0FE0, 0x0FF8, 0x0FD8 - 0x0100] {
        jy_fetch(&mut core, address);
        assert_eq!(0, peek(&core, 0x0000), "{address:#06X} must not latch");
    }
}

/// `$D003` bit 7 is what enables the mode, for **every** number of the chip.
///
/// With the bit clear the page's plain 4 KiB layout applies — "$9000/$A000: … at PPU $0000-$0FFF"
/// and "$9004/$A004: … at PPU $1000-$1FFF" — which is the same register table read with both
/// latches at 0, so a fetch of the latch tile changes nothing at all.
///
/// Mesen 2 gates its latch on `MapperID == 209` instead, so mapper 90's *Mighty Morphin' Power
/// Rangers III* — the title the page names as the one that "uses MMC4-like mode for displaying
/// title screen" — would not get it there. This test runs all four numbers (Q-150).
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_150_jy_asic_mmc4_mode_is_d003_bit_7_on_every_number() {
    for mapper in [35usize, 90, 209, 211] {
        let mut core = MapperRom::new(mapper).prg(64).chr(64).load();
        core.mapper_write(0xD000, 0x08); // 4 KiB CHR banking, MMC4 mode still off.
        core.mapper_write(0x9002, 0x02);
        jy_fetch(&mut core, 0x0FE8);
        assert_eq!(0, peek(&core, 0x0000), "{mapper}: the mode is off");

        core.mapper_write(0xD003, 0x80);
        jy_fetch(&mut core, 0x0FE8);
        assert_eq!(8, peek(&core, 0x0000), "{mapper}: the mode is on");
    }
}

/// A peek never moves a latch (REQ-CORE-033), and neither does a fetch above `$1FFF`.
///
/// The second half is this board's own: its latch sits on `Mapper::on_ppu_address_bus`, which sees
/// the nametable and attribute fetches of every rendered line as well as the pattern fetches, so
/// the `address >= 0x2000` test is what keeps `$2FD8` out of it. Both halves are read through
/// `ppu_peek` and not through `chr_latch_state`, so that the test means the same thing with
/// `board-jy-asic-tile-latch` off — that feature is what makes the latch state *visible*, not
/// what makes it move.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_033_jy_asic_a_peek_never_moves_its_latch() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    core.mapper_write(0xD000, 0x08);
    core.mapper_write(0xD003, 0x80);
    core.mapper_write(0x9002, 0x02);
    core.mapper_write(0x9004, 0x03);
    core.mapper_write(0x9006, 0x04);
    let before = [peek(&core, 0x0000), peek(&core, 0x1000)];
    assert_eq!([0, 12], before, "both latches power on at 0");

    let cartridge = core.cartridge().unwrap();
    for address in 0..0x2000u16 {
        let _ = cartridge.ppu_peek(address, ChrFetch::Background);
    }
    assert_eq!(
        before,
        [peek(&core, 0x0000), peek(&core, 0x1000)],
        "a peek is side-effect free"
    );

    // $2FD8 and $3FE8 are nametable and palette space: the latch must not see them.
    for address in [0x2FD8u16, 0x2FE8, 0x3FD8, 0x3FE8] {
        jy_fetch(&mut core, address);
    }
    assert_eq!(
        before,
        [peek(&core, 0x0000), peek(&core, 0x1000)],
        "nothing above $1FFF is a pattern fetch"
    );
}

/// **The feature `board-jy-asic-tile-latch` off is REQ-CORE-033 exactly**: the tile emitter saves
/// 0, moves nothing and restores 0, so it cannot disturb the latch the emulation uses.
///
/// `Mapper`'s three defaults are `0`, nothing and nothing, and with the feature off `IrqBoard`
/// forwards none of the three — so this is what the whole contract reduces to. MF-UNL-4 step 7
/// measured the forwards at `mmc5` +0.374 % and the best shape of them at `metroid` +0.030 %,
/// against a tolerance of +0.02 % per ROM, which is why there is a feature to be off.
#[cfg(all(feature = "board-jy-asic", not(feature = "board-jy-asic-tile-latch")))]
#[test]
fn req_core_033_jy_asic_without_the_tile_latch_feature_the_emitter_cannot_move_the_latch() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    core.mapper_write(0xD000, 0x08);
    core.mapper_write(0xD003, 0x80);
    core.mapper_write(0x9002, 0x02);

    // The board's own latch still moves on the bus, which is the emulation.
    jy_fetch(&mut core, 0x0FE8);
    assert_eq!(8, peek(&core, 0x0000), "the latch itself is in `default`");

    // And the emitter's three methods are the inert defaults.
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(0, cartridge.chr_latch_state());
    cartridge.advance_chr_latch(0x0000, 0xFD);
    assert_eq!(8, cartridge.ppu_peek(0x0000, ChrFetch::Background));
    cartridge.set_chr_latch_state(0);
    assert_eq!(8, cartridge.ppu_peek(0x0000, ChrFetch::Background));
}

/// REQ-CORE-039's own contract: the tile emitter reads the latch state, walks the line forward
/// with `advance_chr_latch`, then puts the saved value back — and the emulation is unchanged
/// either way.
///
/// `$0FD8`-`$0FDF` is tile `$FD`'s second bit plane and `$0FE8`-`$0FEF` is tile `$FE`'s, so the
/// page's two address ranges are the MMC4's two tile numbers and this board's
/// `advance_chr_latch` is `Mmc2`'s body.
#[cfg(feature = "board-jy-asic-tile-latch")]
#[test]
fn req_core_039_jy_asic_advance_chr_latch_walks_the_line_without_moving_the_board() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    core.mapper_write(0xD000, 0x08);
    core.mapper_write(0xD003, 0x80);
    core.mapper_write(0x9000, 0x01);
    core.mapper_write(0x9002, 0x02);
    core.mapper_write(0x9004, 0x03);
    core.mapper_write(0x9006, 0x04);

    let cartridge = core.cartridge_mut().unwrap();
    let saved = cartridge.chr_latch_state();
    assert_eq!(0, saved, "both latches power on at 0");

    // Tile $FE of the low table, then tile $FD of the high one: one latch each, and the view the
    // emitter would record moves with them.
    cartridge.advance_chr_latch(0x0000, 0xFE);
    assert_eq!(8, cartridge.ppu_peek(0x0000, ChrFetch::Background));
    cartridge.advance_chr_latch(0x1000, 0xFE);
    assert_eq!(16, cartridge.ppu_peek(0x1000, ChrFetch::Background));
    assert_eq!(0b11, cartridge.chr_latch_state());

    // A tile that is neither moves nothing.
    cartridge.advance_chr_latch(0x0000, 0x42);
    assert_eq!(0b11, cartridge.chr_latch_state());

    // Put it back, and the board is where the emulation left it.
    cartridge.set_chr_latch_state(saved);
    assert_eq!(saved, cartridge.chr_latch_state());
    assert_eq!(4, cartridge.ppu_peek(0x0000, ChrFetch::Background));
    assert_eq!(12, cartridge.ppu_peek(0x1000, ChrFetch::Background));
}

/// Extended Mirroring, `$D001` bit 3: "Enabled, ignore MM bits, use $B000-$B003 bit 0".
///
/// One CIRAM bank per logical table, which is §4.5 of the inventory's *cheap* level — a
/// `nametable_page` answer and not a `read_nametable` body. Mesen 2 has no decode of this bit at
/// all and reads `$D000` bit 5 for the same job (Q-149).
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_149_jy_asic_extended_mirroring_is_d001_bit_3() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();

    // With the bit clear the four tables follow `$D001`'s MM bits, whatever `$B00x` says.
    for slot in 0u16..4 {
        core.mapper_write(0xB000 | slot, 0x01);
    }
    assert_eq!(
        [-1, -1, -1, -1],
        nametable_pages(&core),
        "MM bits in charge"
    );

    // With it set, bit 0 of each register is that table's CIRAM bank.
    core.mapper_write(0xD001, 0x08);
    assert_eq!([1, 1, 1, 1], nametable_pages(&core));
    for (slot, bit) in [(0u16, 0u8), (1, 1), (2, 1), (3, 0)] {
        core.mapper_write(0xB000 | slot, bit);
    }
    assert_eq!([0, 1, 1, 0], nametable_pages(&core), "a bank per table");

    // Only bit 0: the other seven are the ROM/CIRAM select and the CHR bank's LSB.
    core.mapper_write(0xB000, 0xFE);
    assert_eq!([0, 1, 1, 0], nametable_pages(&core));

    // And back: clearing the bit puts the MM bits back in charge.
    core.mapper_write(0xD001, 0x00);
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
}

/// ROM nametables for **all** four tables: `$D000` bit 5 set and bit 6 set.
///
/// "$B000/$B004 (LSB/MSB): Select 1 KiB CHR bank at PPU $2000-$23FF", and the three others the
/// same way — so a `$2000`-`$2FFF` read returns CHR-ROM and not CIRAM, which is the census's own
/// test.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_serves_all_four_nametables_from_chr_rom() {
    // 256 KiB of CHR, so that bank $85 exists rather than wrapping: the page gives bit 7 of the
    // LSB the ROM/CIRAM select *and* leaves it in the bank number, which the last case needs.
    let mut core = MapperRom::new(209).prg(64).chr(256).load();
    core.mapper_write(0xD000, 0x60); // ROM nametables, for all.
    assert_eq!(
        [-2, -2, -2, -2],
        nametable_pages(&core),
        "SERVED_BY_CARTRIDGE for all four"
    );

    // Each register pair is a 1 KiB CHR bank, and the fixture fills bank `b` with `b`.
    for (slot, bank) in [(0u16, 0x05u8), (1, 0x11), (2, 0x20), (3, 0x3F)] {
        core.mapper_write(0xB000 | slot, bank);
        core.mapper_write(0xB004 | slot, 0x00);
    }
    assert_eq!(
        [0x05, 0x11, 0x20, 0x3F],
        [0x2000u16, 0x2400, 0x2800, 0x2C00].map(|address| nametable(&core, address)),
        "one CHR bank per table"
    );

    // Within a table, the low ten bits are the offset in that 1 KiB bank.
    assert_eq!(0x05, nametable(&core, 0x23FF));
    assert_eq!(0x11, nametable(&core, 0x2401));

    // Bit 7 of the LSB is in the bank number as well as being the select: with bit 6 set the
    // select is not consulted, so $85 is bank $85 and the fixture's byte for it is $85.
    core.mapper_write(0xB000, 0x85);
    assert_eq!(0x85, nametable(&core, 0x2000), "bit 7 is part of the bank");
}

/// ROM nametables for **selected** tables: `$D000` bit 5 set, bit 6 clear, and `$D002` bit 7
/// deciding which value of `$B00x` bit 7 means ROM.
///
/// "0: $B00x bit 7=0 selects CIRAM, $B00x bit 7=1 selects ROM nametable / 1: $B00x bit 7=1 selects
/// CIRAM, $B00x bit 7=0 selects ROM nametable" — so the table is ROM exactly when the two bits
/// **differ**.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_selects_rom_nametables_table_by_table() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    core.mapper_write(0xD000, 0x20); // ROM nametables, selected individually.

    // $D002 bit 7 clear: bit 7 set selects ROM, bit 7 clear selects a CIRAM bank from bit 0.
    for (slot, value) in [(0u16, 0x80u8), (1, 0x00), (2, 0x81), (3, 0x01)] {
        core.mapper_write(0xB000 | slot, value);
    }
    assert_eq!(
        [-2, 0, -2, 1],
        nametable_pages(&core),
        "bit 7 differs = ROM"
    );

    // $D002 bit 7 set inverts the test, and only that: the CIRAM banks are still bit 0.
    core.mapper_write(0xD002, 0x80);
    assert_eq!([0, -2, 1, -2], nametable_pages(&core), "inverted");

    // The two tables that are ROM read their own CHR bank; $B001 is $00 and $B003 is $01.
    core.mapper_write(0xB005, 0x00);
    core.mapper_write(0xB007, 0x00);
    assert_eq!(
        [0x00, 0x01],
        [nametable(&core, 0x2400), nametable(&core, 0x2C00)]
    );

    // "These registers have no function if neither ROM nametables ($D000 bit 5) nor Extended
    // Mirroring ($D001 bit 3) are enabled."
    core.mapper_write(0xD000, 0x00);
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
}

/// The `$B004`-`$B007` MSBs reach the bank number, which is what makes the ROM nametables able to
/// address the whole 1 MiB of CHR the chip allows.
///
/// Unlike `$900x`/`$A00x`, the page states **no** outer-bank mask for the nametable pair — its
/// "always masked to the outer bank size" sentence is about the CHR bank registers — so the pair
/// is the whole sixteen bits. Mesen 2 reads it the same way.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_nametable_bank_is_the_whole_register_pair() {
    let mut core = MapperRom::new(209).prg(64).chr(640).load();
    core.mapper_write(0xD000, 0x60);

    // Bank $0105 is past the 256 KiB a `$900x` register could reach on its own.
    core.mapper_write(0xB000, 0x05);
    core.mapper_write(0xB004, 0x01);
    // The fixture fills 1 KiB bank `b` with `b as u8`, so bank $0105 carries $05 — the same byte
    // as bank $05. What distinguishes them is the *second* half of the bank: $0106 against $06.
    core.mapper_write(0xB001, 0x06);
    core.mapper_write(0xB005, 0x01);
    assert_eq!(
        [0x05, 0x06],
        [nametable(&core, 0x2000), nametable(&core, 0x2400)]
    );

    // With the MSB back to zero the same LSBs name banks $05 and $06, which read the same two
    // bytes — so this test proves the MSB is *decoded*, and the outer-CHR-bank test proves the
    // window table can tell the two apart.
    core.mapper_write(0xB004, 0x00);
    core.mapper_write(0xB005, 0x00);
    assert_eq!(
        [0x05, 0x06],
        [nametable(&core, 0x2000), nametable(&core, 0x2400)]
    );
}

/// Mapper 90's jumper: "inhibits, via jumper, ROM nametables and Extended Mirroring that would
/// otherwise be enabled via register", and "their function is supressed via a jumper on some PCB
/// variants that are denoted by iNES Mapper 090".
///
/// **Both** mechanisms, so the four `$B00x` registers "have no function" and the layout is
/// `$D001`'s MM bits whatever any of the three registers says. The other three numbers are the
/// same board with the jumper uncut.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_100_jy_asic_mapper_90_inhibits_both_nametable_mechanisms() {
    let mut m90 = MapperRom::new(90).prg(64).chr(64).load();
    let mut m209 = MapperRom::new(209).prg(64).chr(64).load();

    for core in [&mut m90, &mut m209] {
        // Everything on at once: ROM nametables for all, Extended Mirroring, and four registers.
        core.mapper_write(0xD000, 0x60);
        core.mapper_write(0xD001, 0x08);
        for slot in 0u16..4 {
            core.mapper_write(0xB000 | slot, 0x81);
        }
    }

    assert_eq!(
        [-1, -1, -1, -1],
        nametable_pages(&m90),
        "90's jumper inhibits both"
    );
    assert_eq!(
        [-2, -2, -2, -2],
        nametable_pages(&m209),
        "and 209's does not"
    );

    // 90 still has every other register of the chip: the arrangement bits, the banking and the
    // multiplier. Only the nametable registers are cut.
    m90.mapper_write(0xD001, 0x01);
    assert_eq!(Nm::Horizontal, m90.cartridge().unwrap().mirroring());
    m90.mapper_write(0x5800, 0x0F);
    m90.mapper_write(0x5801, 0x11);
    assert_eq!(0xFF, m90.read_memory(0x5800));
}

/// **Q-149**: 211 is "a mere duplicate of INES Mapper 209 **with correct emulation**", so it is
/// 209's board bit for bit — and 35 is too.
///
/// Mesen 2 special-cases 211 as `(_advancedNtControl || MapperID == 211) && MapperID != 90`, which
/// is the historical definition the page itself calls a misunderstanding: "the mapper was defined
/// when the function of register $D001 bit 3 was not known". With the register decoded there is
/// nothing left for the special case to do, and the page says so.
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_149_jy_asic_mapper_211_is_mapper_209_and_not_n_always_set() {
    let mut m211 = MapperRom::new(211).prg(64).chr(64).load();
    let mut m35 = MapperRom::new(35).prg(64).chr(64).load();
    let mut m209 = MapperRom::new(209).prg(64).chr(64).load();

    // Power-on: the nametable registers are idle on all three, where Mesen 2's 211 would already
    // be answering a CIRAM bank per table from four registers nobody has written.
    for (number, core) in [(211, &m211), (35, &m35), (209, &m209)] {
        assert_eq!(
            [-1, -1, -1, -1],
            nametable_pages(core),
            "{number} powers on with its nametable registers idle"
        );
    }

    // And the three answer identically once a game asks: Extended Mirroring from $D001 bit 3.
    for core in [&mut m211, &mut m35, &mut m209] {
        core.mapper_write(0xD001, 0x08);
        for (slot, bit) in [(0u16, 1u8), (1, 0), (2, 0), (3, 1)] {
            core.mapper_write(0xB000 | slot, bit);
        }
    }
    let expected = [1, 0, 0, 1];
    for (number, core) in [(211, &m211), (35, &m35), (209, &m209)] {
        assert_eq!(expected, nametable_pages(core), "{number}");
    }
}

/// **Q-152**: a write to a nametable the board serves from CHR-ROM is dropped, where the hardware
/// leaves the CIRAM byte behind it intact.
///
/// Mesen 2's own comment on the mechanism is "This behavior only affects reads, not writes", and
/// the forum thread it cites says the CIRAM is still written and still there. This core cannot
/// express "read from ROM, write to CIRAM" at one address: `Mapper::nametable_page` answers once
/// for both directions, and `SERVED_BY_CARTRIDGE` sends the write to the board, which has no
/// access to console VRAM. So the write is dropped and the ticket records it.
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_152_jy_asic_a_write_to_a_rom_nametable_is_dropped() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    core.mapper_write(0xD000, 0x60); // ROM nametables, for all four.
    core.mapper_write(0xB000, 0x05);
    core.mapper_write(0xB004, 0x00);
    assert_eq!(0x05, nametable(&core, 0x2000));

    core.cartridge_mut().unwrap().write_nametable(0x2000, 0x5A);
    assert_eq!(0x05, nametable(&core, 0x2000), "CHR-ROM ignores the write");

    // And with the mechanism off, the console's own VRAM is back in charge: the page is
    // FOLLOW_MIRRORING and the PPU never asks the board at all.
    core.mapper_write(0xD000, 0x00);
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
}

// ------------------------------------------- Nanjing FC-001 (163)

/// Drives the PPU's address bus across the edge the Nánjīng FC-001 latches on: out of
/// $2000-$3FFF, then into `address`. The board latches PPU A9 on the **rise of PPU A13** and on
/// nothing else, so both halves of the walk matter.
///
/// Mapper 164 latches on the same edge and takes a second line off it (MF-UNL-10 step 2), so the
/// helper is shared with `board-164`.
#[cfg(any(feature = "board-162", feature = "board-163", feature = "board-164"))]
fn latch_a13_rise(core: &mut NesCore, address: u16) {
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x0000, 1);
    cartridge.on_ppu_address_bus(address, 2);
}

/// The four 8 KiB PRG windows as bank numbers, for a board whose banks are 32 KiB: the fixture
/// fills each 8 KiB bank with its own number, so a 32 KiB bank `b` reads `[4b, 4b+1, 4b+2, 4b+3]`.
#[cfg(any(feature = "board-162", feature = "board-163"))]
fn prg_32k_bank(bank: usize) -> [u8; 4] {
    std::array::from_fn(|slot| (bank * 4 + slot) as u8)
}

/// nesdev ("INES Mapper 163"): the PRG bank is `A15 | A16<<1 | A17<<2 | A18<<3 | A19<<4 | A20<<5`
/// with `PPPP` of `$5000` giving A18..A15 and `..PP` of `$5200` giving A20..A19 — and
/// "**because reset clears the A bit, games will boot in 32 KiB PRG-ROM bank #3**", the mode
/// register's `A` ($5300 bit 2) forcing `PRG A15/A16=11b` while it is clear (MF-UNL-5 step 1).
#[cfg(feature = "board-163")]
#[test]
fn req_core_102_mapper_163_boots_in_bank_3_and_composes_six_bank_bits() {
    // 128 banks of 8 KiB is 1 MiB, i.e. 32 banks of 32 KiB: five of the six bank bits, which is
    // as far as the fixture's numbered banks reach.
    let mut core = MapperRom::new(163).prg(128).chr(0).load();
    assert_eq!(163, core.cartridge().unwrap().mapper_number());

    // "All registers are initialized to $00 on reset", and bank #3 is what that composes.
    assert_eq!(prg_32k_bank(3), core.prg_windows());

    // With `A` still clear, `$5000`'s two low bits reach nothing: A15 and A16 stay `11b` and only
    // A18..A17 move. $00 to $03 are all bank 3; $04 is A17, so bank 7; $08 is A18, so bank 11.
    for value in [0x00u8, 0x01, 0x02, 0x03] {
        core.mapper_write(0x5000, value);
        assert_eq!(
            prg_32k_bank(3),
            core.prg_windows(),
            "$5000={value:#04X} with A clear"
        );
    }
    core.mapper_write(0x5000, 0x04);
    assert_eq!(prg_32k_bank(7), core.prg_windows(), "A17");
    core.mapper_write(0x5000, 0x08);
    assert_eq!(
        prg_32k_bank(11),
        core.prg_windows(),
        "A18, with A15/A16 still 11b"
    );

    // "$5300=$04" is the `A` bit alone, which the page's *Notes* call the dumping configuration:
    // "Mapper 163 cartridges should be dumped with $5300=$04 to obtain the correct PRG bank
    // order". With it set, all four bits of `$5000` are the bank.
    core.mapper_write(0x5300, 0x04);
    for value in 0..16u8 {
        core.mapper_write(0x5000, value);
        assert_eq!(
            prg_32k_bank(usize::from(value)),
            core.prg_windows(),
            "$5000={value:#04X} with A set"
        );
    }

    // `.... ..PP` of `$5200` is A20..A19, bits 5-4 of the bank number. A 1 MiB image has 32 banks
    // of 32 KiB, so A19 is the top bit it can show and A20 wraps.
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5200, 0x01);
    assert_eq!(prg_32k_bank(16), core.prg_windows(), "A19");
    core.mapper_write(0x5200, 0x02);
    assert_eq!(
        prg_32k_bank(0),
        core.prg_windows(),
        "A20 wraps on a 1 MiB image"
    );
    core.mapper_write(0x5200, 0x03);
    assert_eq!(
        prg_32k_bank(16),
        core.prg_windows(),
        "A20|A19 wraps onto A19"
    );

    // "CPU $6000-$7FFF: 8 KiB unbanked PRG-RAM" and "PPU $0000-$1FFF: 8 KiB ... CHR-RAM bank".
    assert_eq!(
        (8 * 1024, 8 * 1024),
        (
            core.cartridge().unwrap().memory().prg_ram().len(),
            core.cartridge().unwrap().memory().chr_ram().len()
        )
    );
}

/// **The mechanism that names the lot**: "Enable automatic 4 KiB CHR-RAM switch … 1: CHR A12=PPU
/// A9 latched on last rise of PPU A13", which the page describes as "the left pattern table is
/// used for the the top half of any nametable, and the right pattern table for the bottom half of
/// any nametable, **regardless of the scroll position**".
///
/// The census asks for exactly this test: "a PPU fetch at an address in one quarter of the window
/// selects one 4 KiB page and a fetch in another selects another, **with no register write between
/// them** — the test that distinguishes this hook from register-driven banking" (MF-UNL-5 step 1).
#[cfg(feature = "board-163")]
#[test]
fn req_core_102_mapper_163_latches_its_chr_page_from_the_rise_of_ppu_a13() {
    let mut core = MapperRom::new(163).prg(16).chr(0).load();

    // With the switch off the 8 KiB chip is flat: CHR A12 is the fetch's own A12, so $0000 and
    // $1000 are two different pages and no PPU address changes that.
    chr_write(&mut core, 0x0000, 0xC0);
    chr_write(&mut core, 0x1000, 0xC1);
    for address in [0x2000u16, 0x2200, 0x23FF] {
        latch_a13_rise(&mut core, address);
        assert_eq!(
            (0xC0, 0xC1),
            (peek(&core, 0x0000), peek(&core, 0x1000)),
            "the switch is off"
        );
    }

    // Bit 7 of `$5000` turns the switch on. Now CHR A12 is the latch, so **both** halves of the
    // pattern-table space show the same 4 KiB page: $0000 and $1000 read the same byte.
    core.mapper_write(0x5000, 0x80);
    latch_a13_rise(&mut core, 0x2000); // A9 clear: the top half of a nametable, page 0.
    assert_eq!((0xC0, 0xC0), (peek(&core, 0x0000), peek(&core, 0x1000)));
    latch_a13_rise(&mut core, 0x2200); // A9 set: the bottom half, page 1.
    assert_eq!((0xC1, 0xC1), (peek(&core, 0x0000), peek(&core, 0x1000)));

    // **No register write between them**, which is the census's own criterion: four fetches, four
    // answers, and the only thing that moved is the PPU address.
    for (address, byte) in [
        (0x2000u16, 0xC0u8),
        (0x2200, 0xC1),
        (0x2100, 0xC0),
        (0x23C0, 0xC1),
    ] {
        latch_a13_rise(&mut core, address);
        assert_eq!(byte, peek(&core, 0x0000), "a fetch at {address:#06X}");
    }

    // And it is the **rise** and not a decode: once the address is inside $2000-$3FFF, moving it
    // within the range latches nothing. Page 0 is selected, and walking to $2200 leaves it.
    latch_a13_rise(&mut core, 0x2000);
    for address in [0x2200u16, 0x2600, 0x3E00, 0x2FFF] {
        core.cartridge_mut().unwrap().on_ppu_address_bus(address, 3);
        assert_eq!(
            0xC0,
            peek(&core, 0x0000),
            "no second rise at {address:#06X}"
        );
    }

    // A bare `$2006` write is such a rise too, the page's latch being on the address lines and
    // not on /RD: this core cannot tell it apart from a fetch's, and neither can the board.
    // `Cartridge::on_ppu_address_bus_without_read` is the entry point behind the second of §4.3's
    // two flags, and it answers nothing unless the number is in that list — so this is the
    // observation that proves the board is in **both**.
    {
        let cartridge = core.cartridge_mut().unwrap();
        cartridge.on_ppu_address_bus_without_read(0x0000, 4);
        cartridge.on_ppu_address_bus_without_read(0x2200, 5);
    }
    assert_eq!(0xC1, peek(&core, 0x0000), "a bare $2006 write latches too");
}

/// The feedback latch: a write to `$5100` latches `.... .F.E`, a write to `$5101` "flip[s] the
/// latched F bit when E=1" and ignores the written `F`, and a read of `$5500-$5501` under mask
/// `$F300` answers the "inverted value of latched F bit" in bit 2 (MF-UNL-5 step 1).
#[cfg(feature = "board-163")]
#[test]
fn req_core_102_mapper_163_answers_its_feedback_register_inverted() {
    let mut core = MapperRom::new(163).prg(16).chr(0).load();

    // Power-on: F is clear, so the read is its inverse, bit 2 set.
    assert_eq!(0x04, core.read_memory(0x5500));

    // "A=0: Value latched" — $5100 latches F (bit 2) and E (bit 0) both.
    core.mapper_write(0x5100, 0x04);
    assert_eq!(0x00, core.read_memory(0x5500), "F set reads back inverted");
    core.mapper_write(0x5100, 0x00);
    assert_eq!(0x04, core.read_memory(0x5500));

    // "A=1: Flip the latched F bit when E=1" — and only when E is set.
    core.mapper_write(0x5101, 0x00);
    assert_eq!(0x04, core.read_memory(0x5500), "E clear flips nothing");
    core.mapper_write(0x5101, 0x01);
    assert_eq!(0x00, core.read_memory(0x5500), "E set flips F");
    core.mapper_write(0x5101, 0x01);
    assert_eq!(0x04, core.read_memory(0x5500), "and flips it back");
    // "A=1: Ignored" for the written F: $05 is E set and F set, and only the flip happens.
    core.mapper_write(0x5101, 0x05);
    assert_eq!(0x00, core.read_memory(0x5500));

    // Mask $F300 reads A15-A12 and A9-A8: `$5500 & $F300` is `$5100`, so the port answers at every
    // $5xxx address with A9 clear and A8 set, and at no other. The four windows the mask leaves
    // are $51xx, $55xx, $59xx and $5Dxx.
    for address in [
        0x5100u16, 0x5101, 0x51FF, 0x5500, 0x5501, 0x55AA, 0x5900, 0x5DFF,
    ] {
        assert_eq!(
            0x00,
            core.read_memory(address),
            "{address:#06X} is the port"
        );
    }
}

/// `$5300` bit 0: "Swap D0,D1 on writes to $5000-$5200", which the page gives as the way "to obtain
/// the correct PRG bank order".
///
/// **Q-155** is the range: the page's own words contain `$5100`, whose two data bits are the
/// feedback latch's and not bank lines. This build swaps the two **bank** ports and leaves the
/// feedback port alone, and this test pins both halves of that choice.
#[cfg(feature = "board-163")]
#[test]
fn q_155_mapper_163_swaps_d0_and_d1_on_the_two_bank_ports_only() {
    let mut core = MapperRom::new(163).prg(128).chr(0).load();

    // `A` set, so all four bits of `$5000` are the bank; then the swap bit beside it. $05 is
    // `A`|`B`, which is "PRG A15/A16 from $5000" and "swap D0,D1".
    core.mapper_write(0x5300, 0x05);
    for (written, bank) in [
        (0x01u8, 2usize),
        (0x02, 1),
        (0x03, 3),
        (0x09, 10),
        (0x06, 5),
    ] {
        core.mapper_write(0x5000, written);
        assert_eq!(
            prg_32k_bank(bank),
            core.prg_windows(),
            "$5000={written:#04X} swapped"
        );
    }

    // The swap is on the **write** path: clearing `B` afterwards does not re-swap a bank already
    // latched. $01 was written swapped and is bank 2; it stays bank 2.
    core.mapper_write(0x5000, 0x01);
    core.mapper_write(0x5300, 0x04);
    assert_eq!(
        prg_32k_bank(2),
        core.prg_windows(),
        "the latch holds the swapped value"
    );

    // And the feedback port is **not** swapped. The observation that separates the two readings is
    // the flip: with the swap applied to $5101 as well, $01 would arrive as $02, whose E is clear,
    // and nothing would flip.
    core.mapper_write(0x5300, 0x05);
    core.mapper_write(0x5100, 0x00);
    assert_eq!(0x04, core.read_memory(0x5500));
    core.mapper_write(0x5101, 0x01);
    assert_eq!(0x00, core.read_memory(0x5500), "E=1 flips F, unswapped");
}

/// §7.1 item 2 of `docs/review/UNL-census.md`, for this lot's first board: a save-state round-trip
/// with the board's **own** registers dirtied first, and the battery PRG-RAM with it.
///
/// `req_core_090_the_mapper_state_survives_a_save_and_restore` sweeps every number of
/// `SUPPORTED_MAPPERS` and dirties each through `$8000-$E001` and `$6000`. **Not one of this
/// board's registers is at any of those addresses** — all four write ports are in $5000-$53FF and
/// the CHR page comes off the PPU bus — so the generic sweep says nothing about the bytes this
/// board appends.
#[cfg(feature = "board-163")]
#[test]
fn req_core_090_mapper_163_round_trips_its_four_ports_its_latch_and_its_battery_ram() {
    let mut core = MapperRom::new(163).prg(128).chr(0).battery().load();

    // Every port, and the PPU-bus latch with the switch on so that it reaches the windows.
    core.mapper_write(0x5300, 0x05);
    core.mapper_write(0x5000, 0x8A);
    core.mapper_write(0x5200, 0x02);
    core.mapper_write(0x5100, 0x04);
    core.mapper_write(0x5101, 0x01);
    latch_a13_rise(&mut core, 0x2200);
    core.write_memory(0x6123, 0x7E);

    let state = core.save_state(true).unwrap();
    let windows = core.prg_windows();
    let chr = core.chr_windows();
    let feedback = core.read_memory(0x5500);
    assert_eq!(0x04, feedback, "F was set and then flipped back");

    // Move everything the board has, then come back.
    core.mapper_write(0x5300, 0x00);
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5101, 0x01);
    latch_a13_rise(&mut core, 0x2000);
    for _ in 0..5 {
        core.run_frame(0);
    }

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(windows, core.prg_windows(), "the PRG windows");
    assert_eq!(chr, core.chr_windows(), "the latched CHR page");
    assert_eq!(feedback, core.read_memory(0x5500), "the feedback latch");
    assert_eq!(state, core.save_state(true).unwrap(), "the whole block");

    // "8 KiB unbanked PRG-RAM, battery-backed": the byte is in what REQ-CORE-095 exposes.
    assert_eq!(0x7E, core.read_memory(0x6123));
    let saved = core.export_battery_ram().to_vec();
    assert_eq!(8 * 1024, saved.len());
    assert_eq!(0x7E, saved[0x0123]);
}

/// The board has no IRQ and no expansion audio, which §7.2 step 4 asks be pinned so that the
/// deferral of submapper 1's ADPCM chip is visible as a deliberate gap and not an omission.
#[cfg(feature = "board-163")]
#[test]
fn req_core_075_mapper_163_has_no_expansion_audio_and_no_irq_horizon() {
    let mut core = MapperRom::new(163).prg(16).chr(0).submapper(1).load();
    let cartridge = core.cartridge().unwrap();
    assert_eq!(None, cartridge.expansion_audio_output());
    assert_eq!(Some(i64::MAX), cartridge.ppu_irq_horizon());
    assert!(!cartridge.irq_asserted());

    // Ten frames of a submapper-1 image: the board runs, the chip is simply not there.
    for _ in 0..10 {
        core.run_frame(0);
    }
    assert!(!core.cartridge().unwrap().irq_asserted());
}

// ------------------------------------------- Waixing FS304 (162)

/// nesdev ("INES Mapper 162"): the two mode bits of `$5300` pick where PRG A15 and PRG A16 come
/// from —
///
/// > `B` ($5300.0) — PRG A15 mode: 0: PRG A15=$5100.1; 1: PRG A15=1 if `A`=0, PRG A15=$5000.0 if
/// > `A`=1
/// >
/// > `A` ($5300.2) — PRG A16 mode: 0: PRG A16=1; 1: PRG A16=$5000.1
///
/// — and the page's own check of the arithmetic is the boot bank: "Games boot in PRG bank #2 since
/// reset initializes all registers to $00" (MF-UNL-5 step 2).
#[cfg(feature = "board-162")]
#[test]
fn req_core_102_mapper_162_boots_in_bank_2_and_takes_a15_from_5100() {
    let mut core = MapperRom::new(162).prg(128).chr(0).load();
    assert_eq!(162, core.cartridge().unwrap().mapper_number());

    // `B`=0 gives A15 = `$5100.1` = 0, `A`=0 gives A16 = 1: bank 2.
    assert_eq!(prg_32k_bank(2), core.prg_windows());

    // With both mode bits still clear, `$5100` bit 1 **is** PRG A15 — the one line that tells this
    // board from the FC-001 — and `$5000` bits 3-2 are A18..A17 whatever the mode is.
    core.mapper_write(0x5100, 0x02);
    assert_eq!(prg_32k_bank(3), core.prg_windows(), "A15 from $5100.1");
    core.mapper_write(0x5100, 0x00);
    core.mapper_write(0x5000, 0x04);
    assert_eq!(prg_32k_bank(6), core.prg_windows(), "A17, A16 still forced");
    core.mapper_write(0x5000, 0x08);
    assert_eq!(
        prg_32k_bank(10),
        core.prg_windows(),
        "A18, A16 still forced"
    );

    // `A` alone ($04): "PRG A16=$5000.1", and A15 is still `$5100.1` because `B` is clear.
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5300, 0x04);
    assert_eq!(prg_32k_bank(0), core.prg_windows(), "A16 released");
    core.mapper_write(0x5000, 0x02);
    assert_eq!(prg_32k_bank(2), core.prg_windows(), "A16 from $5000.1");
    core.mapper_write(0x5100, 0x02);
    assert_eq!(
        prg_32k_bank(3),
        core.prg_windows(),
        "A15 still from $5100.1"
    );

    // `B` alone ($01): "PRG A15=1 if A=0", so `$5100` stops reaching the bank and A15 is forced.
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5300, 0x01);
    assert_eq!(prg_32k_bank(3), core.prg_windows(), "A15=1 and A16=1");
    core.mapper_write(0x5100, 0x00);
    assert_eq!(prg_32k_bank(3), core.prg_windows(), "$5100 reaches nothing");

    // `A`|`B` ($05): both lines come from `$5000`, which is the configuration in which this board
    // and the FC-001 read their low nibble the same way.
    core.mapper_write(0x5300, 0x05);
    for value in 0..16u8 {
        core.mapper_write(0x5000, value);
        assert_eq!(
            prg_32k_bank(usize::from(value)),
            core.prg_windows(),
            "$5000={value:#04X} with A and B set"
        );
    }

    // `.... ..PP` of `$5200` is A20..A19, as on the FC-001; a 1 MiB image shows A19 and wraps A20.
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5200, 0x01);
    assert_eq!(prg_32k_bank(16), core.prg_windows(), "A19");
    core.mapper_write(0x5200, 0x02);
    assert_eq!(prg_32k_bank(0), core.prg_windows(), "A20 wraps");

    // The same two chips as mapper 163, from the same two lines of the page's *Banks* section.
    assert_eq!(
        (8 * 1024, 8 * 1024),
        (
            core.cartridge().unwrap().memory().prg_ram().len(),
            core.cartridge().unwrap().memory().chr_ram().len()
        )
    );
}

/// The same automatic CHR-RAM switch as mapper 163, from the same three lines of the page: bit 7
/// of `$5000`, CHR A12 = "PPU A9 latched on last rise of PPU A13".
///
/// The census's criterion again, on the second board: two fetches in two quarters of the window
/// select two pages **with no register write between them** (MF-UNL-5 step 2).
#[cfg(feature = "board-162")]
#[test]
fn req_core_102_mapper_162_latches_the_same_chr_page_from_the_ppu_address_bus() {
    let mut core = MapperRom::new(162).prg(16).chr(0).load();

    chr_write(&mut core, 0x0000, 0xB0);
    chr_write(&mut core, 0x1000, 0xB1);

    // Switch off: the chip is flat and the PPU address changes nothing.
    latch_a13_rise(&mut core, 0x2200);
    assert_eq!((0xB0, 0xB1), (peek(&core, 0x0000), peek(&core, 0x1000)));

    // Switch on: CHR A12 is the latch, so both halves show the latched page.
    core.mapper_write(0x5000, 0x80);
    for (address, byte) in [
        (0x2000u16, 0xB0u8),
        (0x2200, 0xB1),
        (0x2100, 0xB0),
        (0x23C0, 0xB1),
    ] {
        latch_a13_rise(&mut core, address);
        assert_eq!(
            (byte, byte),
            (peek(&core, 0x0000), peek(&core, 0x1000)),
            "a fetch at {address:#06X}"
        );
    }

    // Turning the switch on must not disturb the PRG bank: bit 7 is not a bank line, and the
    // board boots in bank 2 whether or not the CHR switch is set.
    assert_eq!(prg_32k_bank(2), core.prg_windows());

    // And the bare-$2006 path, the second of §4.3's two latches.
    {
        let cartridge = core.cartridge_mut().unwrap();
        cartridge.on_ppu_address_bus_without_read(0x0000, 4);
        cartridge.on_ppu_address_bus_without_read(0x2200, 5);
    }
    assert_eq!(0xB1, peek(&core, 0x0000));
}

/// **The one-register test each page's *Notes* give for telling a dump of one number from a dump
/// of the other**, written as one test because the two sentences are each other's negation:
///
/// > (162) To ensure that a cartridge to be dumped is really INES Mapper 162, write $04 to $5300
/// > and verify that $5100.1 **changes** the PRG bank.
/// >
/// > (163) To verify that the cartridge is really mapper 163, you can rule out INES Mapper 162 by
/// > writing $04 to $5300 and verifying that $5100.1 does **not** change the PRG bank.
///
/// `$04` sets `A` and clears `B`, which on 162 is "PRG A15=$5100.1" and on 163 leaves `$5100` as
/// the feedback latch with no bank line at all (MF-UNL-5 step 2).
#[cfg(all(feature = "board-162", feature = "board-163"))]
#[test]
fn req_core_102_the_two_chr_window_boards_differ_by_the_dumpers_own_test() {
    let mut waixing = MapperRom::new(162).prg(128).chr(0).load();
    let mut nanjing = MapperRom::new(163).prg(128).chr(0).load();

    for core in [&mut waixing, &mut nanjing] {
        core.mapper_write(0x5300, 0x04);
    }
    let before = (waixing.prg_windows(), nanjing.prg_windows());
    for core in [&mut waixing, &mut nanjing] {
        core.mapper_write(0x5100, 0x02);
    }

    assert_ne!(
        before.0,
        waixing.prg_windows(),
        "162: $5100.1 moves the bank"
    );
    assert_eq!(
        before.1,
        nanjing.prg_windows(),
        "163: $5100 is the feedback latch and moves nothing"
    );

    // The other half of the difference, from the same pair of writes: only the FC-001 answers a
    // read of the feedback window. On the FS304 the board drives nothing there.
    assert_eq!(0x04, nanjing.read_memory(0x5500));
    assert_ne!(0x04, waixing.read_memory(0x5500), "162 has no read port");
}

/// §7.1 item 2 for this lot's second board: a save-state round-trip with its own registers dirtied,
/// the PPU-bus latch included, and the battery PRG-RAM with it.
#[cfg(feature = "board-162")]
#[test]
fn req_core_090_mapper_162_round_trips_its_four_ports_its_latch_and_its_battery_ram() {
    let mut core = MapperRom::new(162).prg(128).chr(0).battery().load();

    core.mapper_write(0x5300, 0x05);
    core.mapper_write(0x5000, 0x89);
    core.mapper_write(0x5100, 0x02);
    core.mapper_write(0x5200, 0x01);
    latch_a13_rise(&mut core, 0x2200);
    core.write_memory(0x6456, 0x3C);

    let state = core.save_state(true).unwrap();
    let windows = core.prg_windows();
    let chr = core.chr_windows();

    core.mapper_write(0x5300, 0x00);
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5200, 0x00);
    latch_a13_rise(&mut core, 0x2000);
    for _ in 0..5 {
        core.run_frame(0);
    }

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(windows, core.prg_windows(), "the PRG windows");
    assert_eq!(chr, core.chr_windows(), "the latched CHR page");
    assert_eq!(state, core.save_state(true).unwrap(), "the whole block");

    assert_eq!(0x3C, core.read_memory(0x6456));
    let saved = core.export_battery_ram().to_vec();
    assert_eq!(8 * 1024, saved.len());
    assert_eq!(0x3C, saved[0x0456]);
}

/// The FS304 has no IRQ and no expansion audio either, and `ppu_irq_horizon` stays `Some(i64::MAX)`
/// even though the board is in both PPU watch lists — its /IRQ line cannot change through anything
/// the PPU does, because it has none (§7.2 step 4).
#[cfg(feature = "board-162")]
#[test]
fn req_core_075_mapper_162_has_no_expansion_audio_and_no_irq_horizon() {
    let mut core = MapperRom::new(162).prg(16).chr(0).load();
    core.mapper_write(0x5000, 0x80);
    for _ in 0..10 {
        core.run_frame(0);
    }

    let cartridge = core.cartridge().unwrap();
    assert_eq!(None, cartridge.expansion_audio_output());
    assert_eq!(Some(i64::MAX), cartridge.ppu_irq_horizon());
    assert!(!cartridge.irq_asserted());
}

// ------------------------------------------- Waixing FS305 / Nanjing NJ0430 (178)

/// nesdev ("INES Mapper 178"): the four registers are at **$4800-$4803**, and `$4800`'s two mode
/// bits pick one of four PRG arrangements out of an inner bank (`$4801`, PRG A16..A14) and an
/// eight-bit outer bank (`$4802`, PRG A24?..A17) — so a 16 KiB bank number is `L | H<<3`
/// (MF-UNL-5 step 3).
#[cfg(feature = "board-178")]
#[test]
fn req_core_102_mapper_178_has_four_prg_modes_from_an_inner_and_an_outer_bank() {
    let mut core = MapperRom::new(178).prg(32).chr(0).load();
    assert_eq!(178, core.cartridge().unwrap().mapper_number());

    // Power-on: every register clear, so mode 0 (BNROM) with the first 32 KiB at $8000-$FFFF.
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(Nm::Vertical, core.mirroring(), "$4800 bit 0 clear");

    // Mode 0, "NROM-256/BNROM (PRG A14=CPU A14)": A14 is the CPU's line, so bit 0 of the inner
    // bank reaches nothing and the two halves are one 32 KiB bank. $01 and $00 are the same.
    for value in [0x00u8, 0x01] {
        core.mapper_write(0x4801, value);
        assert_eq!(
            [0, 1, 2, 3],
            core.prg_windows(),
            "mode 0, $4801={value:#04X}"
        );
    }
    core.mapper_write(0x4801, 0x02);
    assert_eq!([4, 5, 6, 7], core.prg_windows(), "mode 0, A15");
    core.mapper_write(0x4801, 0x04);
    assert_eq!([8, 9, 10, 11], core.prg_windows(), "mode 0, A16");

    // Mode 1 ($4800 = $02), "UNROM (PRG A14..16=111b if CPU A14=1)": the inner bank drives
    // $8000-$BFFF with all three bits, and $C000-$FFFF is fixed to 16 KiB bank 7 of the outer
    // bank, which on a 256 KiB image with outer 0 is the eighth 16 KiB bank.
    core.mapper_write(0x4800, 0x02);
    for inner in 0..8u8 {
        core.mapper_write(0x4801, inner);
        let low = inner * 2;
        assert_eq!(
            [low, low + 1, 14, 15],
            core.prg_windows(),
            "mode 1, $4801={inner:#04X}"
        );
    }

    // Mode 2 ($4800 = $04), "NROM-128": one 16 KiB bank in both halves.
    core.mapper_write(0x4800, 0x04);
    for inner in 0..8u8 {
        core.mapper_write(0x4801, inner);
        let low = inner * 2;
        assert_eq!(
            [low, low + 1, low, low + 1],
            core.prg_windows(),
            "mode 2, $4801={inner:#04X}"
        );
    }

    // Mode 3 ($4800 = $06), "UNROM but with bit 0 of \"fixed\" bank selectable (PRG A15..16=11b if
    // CPU A14=1)": the fixed half is 16 KiB bank 6 or 7 of the outer bank, picked by $4801 bit 0.
    core.mapper_write(0x4800, 0x06);
    for inner in 0..8u8 {
        core.mapper_write(0x4801, inner);
        let low = inner * 2;
        let fixed = ((inner & 0x01) | 0x06) * 2;
        assert_eq!(
            [low, low + 1, fixed, fixed + 1],
            core.prg_windows(),
            "mode 3, $4801={inner:#04X}"
        );
    }

    // The outer bank is three bits up from the window, i.e. 128 KiB a step. On a 256 KiB image
    // outer 1 is the second half, and in mode 1 both windows move with it.
    core.mapper_write(0x4800, 0x02);
    core.mapper_write(0x4801, 0x00);
    core.mapper_write(0x4802, 0x01);
    assert_eq!([16, 17, 30, 31], core.prg_windows(), "the outer bank");
    core.mapper_write(0x4802, 0x02);
    assert_eq!([0, 1, 14, 15], core.prg_windows(), "and it wraps");
}

/// `$4800` bit 0: "Nametable mirroring — 0: Vertical, 1: Horizontal", which is the one register of
/// this board the two other boards of the lot do not have at all (theirs is hard-wired).
#[cfg(feature = "board-178")]
#[test]
fn req_core_102_mapper_178_takes_both_mirroring_arrangements_from_4800() {
    // The header says horizontal; the register must win in both directions.
    let mut core = MapperRom::new(178).prg(32).chr(0).mirroring(1).load();
    assert_eq!(
        Nm::Vertical,
        core.mirroring(),
        "the register, not the header"
    );

    for (value, expected) in [
        (0x01u8, Nm::Horizontal),
        (0x00, Nm::Vertical),
        (0x07, Nm::Horizontal),
        (0x06, Nm::Vertical),
    ] {
        core.mapper_write(0x4800, value);
        assert_eq!(expected, core.mirroring(), "$4800={value:#04X}");
    }

    // And a reset puts it back to the power-on arrangement, which is the cleared register's.
    core.mapper_write(0x4800, 0x01);
    core.reset();
    assert_eq!(Nm::Vertical, core.mirroring());
}

/// "PRG-RAM is mapped to CPU $6000-$7FFF and is banked in 8 KiB amounts" from `$4803`, over a chip
/// the page sizes as "PRG-RAM A20?..A13" — the one mechanism this board brings that neither of the
/// lot's other two has, and the first **banked** PRG-RAM window in `DecodeBoard` (MF-UNL-5 step 3).
#[cfg(feature = "board-178")]
#[test]
fn req_core_102_mapper_178_banks_its_prg_ram_from_4803() {
    // 32 KiB of battery-backed PRG-RAM: four of the board's 8 KiB banks, which is what an NES 2.0
    // header can state and an iNES 1.0 one cannot.
    let mut core = MapperRom::new(178)
        .prg(32)
        .chr(0)
        .battery()
        .prg_ram(0, 32 * 1024)
        .load();
    assert_eq!(
        32 * 1024,
        core.cartridge().unwrap().memory().prg_ram().len()
    );

    // A different byte in each bank, written through the window the register selects.
    for bank in 0..4u8 {
        core.mapper_write(0x4803, bank);
        core.write_memory(0x6000, 0x50 | bank);
        core.write_memory(0x7FFF, 0x60 | bank);
    }
    for bank in 0..4u8 {
        core.mapper_write(0x4803, bank);
        assert_eq!(
            0x50 | bank,
            core.read_memory(0x6000),
            "bank {bank} at $6000"
        );
        assert_eq!(
            0x60 | bank,
            core.read_memory(0x7FFF),
            "bank {bank} at $7FFF"
        );
    }

    // All eight bits are the register, and the wrap is the chip's: bank 4 is bank 0 again on a
    // 32 KiB chip, which is what the address lines a smaller chip does not have would do.
    core.mapper_write(0x4803, 0x04);
    assert_eq!(0x50, core.read_memory(0x6000), "bank 4 wraps onto bank 0");
    core.mapper_write(0x4803, 0xFF);
    assert_eq!(0x53, core.read_memory(0x6000), "bank 255 wraps onto bank 3");

    // The whole chip is what a host persists, and the bytes of every bank are in it.
    core.mapper_write(0x4803, 0x00);
    let saved = core.export_battery_ram().to_vec();
    assert_eq!(32 * 1024, saved.len());
    for bank in 0..4usize {
        assert_eq!(0x50 | bank as u8, saved[bank * 8 * 1024]);
    }

    // And an image that declares nothing still gets its one window, `board_memory_sizes` giving
    // the board a floor of 8 KiB (Q-154): every bank is then the same chip.
    let mut plain = MapperRom::new(178).prg(32).chr(0).load();
    assert_eq!(
        8 * 1024,
        plain.cartridge().unwrap().memory().prg_ram().len()
    );
    plain.write_memory(0x6010, 0x9A);
    for bank in [0x01u8, 0x7F, 0xFF] {
        plain.mapper_write(0x4803, bank);
        assert_eq!(
            0x9A,
            plain.read_memory(0x6010),
            "one chip, bank {bank:#04X}"
        );
    }
}

/// **Q-154**: this board's own page gives it no PPU-driven CHR window, where the census's §7.2
/// heading puts it in a lot named after one.
///
/// *Banks* says "PPU $0000-$1FFF: **8 KiB unbanked CHR-RAM**", the overview says the chipset "is
/// designed for using 8 KiB of chip-internal unbanked CHR-RAM", and no sentence of the page
/// mentions PPU A9, PPU A13, a 4 KiB CHR window or CHR switching. So the board is in **neither**
/// PPU watch list, and this test is the anti-test that says so: the PPU address bus moves and the
/// CHR window does not.
#[cfg(feature = "board-178")]
#[test]
fn q_154_mapper_178_has_no_ppu_driven_chr_window() {
    let mut core = MapperRom::new(178).prg(32).chr(0).load();

    // Two different bytes in the two 4 KiB halves of the unbanked chip.
    chr_write(&mut core, 0x0000, 0x71);
    chr_write(&mut core, 0x1000, 0x72);

    // Drive the bus across every edge the lot's other two boards latch on — the rise of A13 with
    // A9 clear and with A9 set, by both entry points — and require the window not to move.
    for address in [0x2000u16, 0x2200, 0x23C0, 0x2FFF, 0x3F00] {
        {
            let cartridge = core.cartridge_mut().unwrap();
            cartridge.on_ppu_address_bus(0x0000, 1);
            cartridge.on_ppu_address_bus(address, 2);
            cartridge.on_ppu_address_bus_without_read(0x0000, 3);
            cartridge.on_ppu_address_bus_without_read(address, 4);
        }
        assert_eq!(
            (0x71, 0x72),
            (peek(&core, 0x0000), peek(&core, 0x1000)),
            "the chip is unbanked, whatever the PPU address does ({address:#06X})"
        );
    }

    // Nor does any register of the board move it: the four ports are the two PRG banks, the mode
    // and the PRG-RAM bank, and none of them is a CHR line.
    for (address, value) in [
        (0x4800u16, 0x07u8),
        (0x4801, 0x07),
        (0x4802, 0xFF),
        (0x4803, 0xFF),
    ] {
        core.mapper_write(address, value);
        assert_eq!(
            (0x71, 0x72),
            (peek(&core, 0x0000), peek(&core, 0x1000)),
            "{address:#06X}={value:#04X} is not a CHR line"
        );
    }

    // The registers are at $4800-$4803 and **not** at $5000-$5300, which is the other half of
    // Q-154: a write where the lot's other two boards keep their ports changes nothing here.
    let before = core.prg_windows();
    for address in [0x5000u16, 0x5100, 0x5200, 0x5300] {
        core.mapper_write(address, 0x05);
        assert_eq!(before, core.prg_windows(), "{address:#06X} is not a port");
    }
}

/// §7.1 item 2 for the lot's third board: a save-state round-trip with its own four registers
/// dirtied, the banked PRG-RAM window included.
#[cfg(feature = "board-178")]
#[test]
fn req_core_090_mapper_178_round_trips_its_four_registers_and_its_ram_bank() {
    let mut core = MapperRom::new(178)
        .prg(32)
        .chr(0)
        .battery()
        .prg_ram(0, 32 * 1024)
        .load();

    core.mapper_write(0x4800, 0x03); // mode 1, horizontal.
    core.mapper_write(0x4801, 0x05);
    core.mapper_write(0x4802, 0x01);
    core.mapper_write(0x4803, 0x02);
    core.write_memory(0x6789, 0x4B);

    let state = core.save_state(true).unwrap();
    let windows = core.prg_windows();
    let mirroring = core.mirroring();
    let ram_byte = core.read_memory(0x6789);

    // Move every register, and the RAM window with them.
    core.mapper_write(0x4800, 0x04);
    core.mapper_write(0x4801, 0x00);
    core.mapper_write(0x4802, 0x00);
    core.mapper_write(0x4803, 0x00);
    for _ in 0..5 {
        core.run_frame(0);
    }
    assert_ne!(windows, core.prg_windows(), "the move must be visible");

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(windows, core.prg_windows(), "the PRG windows");
    assert_eq!(mirroring, core.mirroring(), "the mirroring bit");
    assert_eq!(ram_byte, core.read_memory(0x6789), "the PRG-RAM window");
    assert_eq!(state, core.save_state(true).unwrap(), "the whole block");
}

/// The board has no IRQ and no expansion audio, which is what makes the deferral of submappers 1
/// (the infrared sensor's IRQ), 2 (the TMS5220 speech chip) and 3 (the solder-pad read-out)
/// invisible to every other board (§7.2 step 4).
#[cfg(feature = "board-178")]
#[test]
fn req_core_075_mapper_178_has_no_expansion_audio_and_no_irq_on_any_submapper() {
    for submapper in [1u8, 2, 3] {
        let mut core = MapperRom::new(178)
            .prg(32)
            .chr(0)
            .submapper(submapper)
            .load();
        for _ in 0..10 {
            core.run_frame(0);
        }

        let cartridge = core.cartridge().unwrap();
        assert_eq!(None, cartridge.expansion_audio_output(), "{submapper}");
        assert_eq!(Some(i64::MAX), cartridge.ppu_irq_horizon(), "{submapper}");
        assert!(!cartridge.irq_asserted(), "{submapper}");
    }
}

// ------------------------------------------------- the AA6023 of the COOLBOY boards (268)

/// A mapper-268 image of `prg_bytes` bytes of PRG-ROM and 8 KiB of CHR-RAM, every 8 KiB PRG bank
/// marked with its own **sixteen-bit** number in its first two bytes (MF-UNL-6).
///
/// `MapperRom` cannot serve this board, twice over: byte 4 of an iNES 1.0 header holds 16 KiB
/// units in **one** byte, and a single fill byte per bank cannot name a bank above 255 where this
/// board has twelve bank bits. So the size goes in NES 2.0's **exponent-multiplier** form — "if
/// the MSB nibble is $F, the size is `(low & 3) * 2 + 1` shifted left by `low >> 2`" — and the
/// bank number in two bytes, which [`coolboy_banks`] reads back.
#[cfg(feature = "board-268")]
fn coolboy_image(prg_bytes: usize, submapper: u8) -> Vec<u8> {
    assert!(prg_bytes.is_power_of_two() && prg_bytes >= 0x4000);
    let exponent = prg_bytes.trailing_zeros() as u8;
    let mut image = vec![0u8; 16 + prg_bytes];
    image[..4].copy_from_slice(&[0x4E, 0x45, 0x53, 0x1A]);
    // The exponent form with multiplier 1, so the size is exactly `1 << exponent`.
    image[4] = exponent << 2;
    image[6] = 0xC0; // Mapper bits 0-3 of 268 = $C.
    image[7] = 0x08; // NES 2.0; mapper bits 4-7 are zero.
    image[8] = (submapper << 4) | 0x01; // The submapper, and mapper bits 8-11 = 1.
    image[9] = 0x0F; // The PRG size's MSB nibble: the exponent form.
    image[11] = 0x07; // 8 KiB of CHR-RAM: the header's `64 << shift`.

    for bank in 0..prg_bytes / 0x2000 {
        let start = 16 + bank * 0x2000;
        image[start] = (bank >> 8) as u8;
        image[start + 1] = bank as u8;
    }

    image
}

/// The four 8 KiB PRG bank numbers the windows show, read off the markers [`coolboy_image`] wrote.
#[cfg(feature = "board-268")]
fn coolboy_banks(core: &NesCore) -> [u16; 4] {
    [0x8000u16, 0xA000, 0xC000, 0xE000].map(|address| {
        (u16::from(core.read_memory(address)) << 8) | u16::from(core.read_memory(address + 1))
    })
}

/// Loads a mapper-268 image and checks the number came through the twelve-bit mapper field.
#[cfg(feature = "board-268")]
fn coolboy(prg_bytes: usize, submapper: u8) -> NesCore {
    let mut core = NesCore::new();
    let load = core.load_rom(&coolboy_image(prg_bytes, submapper), "Coolboy.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(268, core.cartridge().unwrap().mapper_number());
    core
}

/// nesdev ("NES 2.0 Mapper 268"): **the board powers on as a 512 KiB MMC3**, and that is not a
/// quirk but the whole reason the chipset works in a multicart (MF-UNL-6 step 1).
///
/// The four PRG mask bits do not all read the same way round — A17 and A18 come "from 0: MMC3;
/// 1: offset" and A19 and A20 "from 0: offset; 1: MMC3" — so with every register clear the MMC3
/// drives A13 to A18 and an offset of zero drives A19 and A20. A plain MMC3 write that selects a
/// bank above 512 KiB is therefore **truncated**, which is why the page says that "a great number
/// of Chinese cartridges use the ASIC as a simple MMC3 clone and thus are usually found set to
/// INES Mapper 004", and why the games that are not have to open the lines first.
#[cfg(feature = "board-268")]
#[test]
fn req_core_102_mapper_268_powers_on_as_a_512_kib_mmc3() {
    // 2 MiB, which is the MMC3's own ceiling: the whole of this test is inside it, so the single
    // fill byte of `MapperRom` names every bank.
    let mut core = MapperRom::new(268).prg(256).chr(64).load();

    // The MMC3's power-on layout, clipped to the first 512 KiB: $FE and $FF become $3E and $3F.
    assert_eq!([0, 1, 62, 63], core.prg_windows());

    // A bank above 512 KiB asked for through the chip alone: bits 6 and 7 of the register are
    // masked away and bank $45 lands on bank $05.
    core.mmc3_bank(6, 0x45);
    assert_eq!(
        [0x05, 1, 62, 63],
        core.prg_windows(),
        "A19 and A20 are masked off"
    );

    // `$6001` bits 6 and 5 are "PRG A19/A20 from 1: MMC3", so with them set the chip reaches the
    // whole 2 MiB and the fixed banks are the image's own last two again.
    core.mapper_write(0x6001, 0x60);
    assert_eq!(
        [0x45, 1, 0xFE, 0xFF],
        core.prg_windows(),
        "the MMC3's own 2 MiB"
    );

    // The other route to the same bank: with the masks back to zero, A19 is the **offset's** own
    // bit 2 of `EEE`, so `$6000 = $04` puts bank $45 back where the chip could not.
    core.mapper_write(0x6001, 0x00);
    core.mapper_write(0x6000, 0x04);
    assert_eq!(
        [0x45, 0x41, 0x7E, 0x7F],
        core.prg_windows(),
        "A19 from the offset"
    );

    // The offset is an address **line** and not an addend: it replaces the bit rather than adding
    // to it, so the same write with the mask open leaves the chip's own A19 in charge.
    core.mapper_write(0x6001, 0x60);
    assert_eq!([0x45, 1, 0xFE, 0xFF], core.prg_windows(), "the mask wins");
}

/// The census's own test for this lot — "an outer-bank write selects a bank above 512 KiB" —
/// walked one address line at a time over an 8 MiB image (MF-UNL-6 step 1).
///
/// With all four mask bits handed to the offset the MMC3 drives A13 to A16 only, so the granule
/// is 128 KiB and the two fixed windows are the **selected** granule's last two rather than the
/// image's — which is what makes `$FE` and `$FF` the right fixed banks on this board, and what
/// the page's own banking mode `$00` means by "CCKKJEEE … a way to select one of 256 different
/// 128 KiB PRG MMC3 games".
#[cfg(feature = "board-268")]
#[test]
fn req_core_102_mapper_268_selects_a_bank_above_512_kib_from_its_outer_registers() {
    let mut core = coolboy(8 * 1024 * 1024, 0);
    // The power-on MMC3, clipped to its first 512 KiB: $FE and $FF become $3E and $3F.
    assert_eq!(
        [0, 1, 0x3E, 0x3F],
        coolboy_banks(&core),
        "the power-on MMC3"
    );

    // `$6000` bit 6 = B (A17 from the offset), `$6001` bit 7 = G (A18 from the offset), and bits
    // 6 and 5 clear leave A19 and A20 with the offset too.
    for (register, value, line, bank) in [
        (0x6000u16, 0x40u8, "no line", 0x000u16),
        (0x6000, 0x41, "A17", 0x010),
        (0x6000, 0x42, "A18", 0x020),
        (0x6000, 0x44, "A19", 0x040),
        (0x6001, 0x90, "A20", 0x080),
        (0x6001, 0x84, "A21", 0x100),
        (0x6001, 0x88, "A22", 0x200),
    ] {
        // The register this row does not write goes back to "every mask bit to the offset".
        if register == 0x6000 {
            core.mapper_write(0x6001, 0x80);
        } else {
            core.mapper_write(0x6000, 0x40);
        }
        core.mapper_write(register, value);
        assert_eq!(
            [bank, bank | 0x01, bank | 0x0E, bank | 0x0F],
            coolboy_banks(&core),
            "{line}: {register:#06X} = {value:#04X}"
        );
    }

    // And the MMC3 underneath still drives the four bits it is left: the granule's own banks.
    core.mapper_write(0x6000, 0x40);
    core.mapper_write(0x6001, 0x88);
    core.mmc3_bank(6, 0x0A);
    assert_eq!(
        [0x20A, 0x201, 0x20E, 0x20F],
        coolboy_banks(&core),
        "the MMC3 inside the selected 128 KiB"
    );
}

/// The page's own address table: "right → `$6000-$6FFF` … Submapper bit 0 = 0 … UNIF MAPR
/// **COOLBOY**", "left → `$5000-$5FFF` … bit 0 = 1 … **MINDKIDS**" — and **mask `$F007`**, which
/// makes the whole 4 KiB window the register file (MF-UNL-6 step 1).
#[cfg(feature = "board-268")]
#[test]
fn req_core_102_mapper_268_answers_at_6000_on_a_coolboy_and_at_5000_on_a_mindkids() {
    for (submapper, port, other) in [(0u8, 0x6000u16, 0x5000u16), (1, 0x5000, 0x6000)] {
        let mut core = coolboy(8 * 1024 * 1024, submapper);
        assert_eq!(
            [0, 1, 0x3E, 0x3F],
            coolboy_banks(&core),
            "submapper {submapper}"
        );

        // The other board's port is not decoded at all.
        core.mapper_write(other + 1, 0x80);
        core.mapper_write(other, 0x44);
        assert_eq!(
            [0, 1, 0x3E, 0x3F],
            coolboy_banks(&core),
            "submapper {submapper}: {other:#06X} is the other board's port"
        );

        // Its own is, and so is every copy of it the mask leaves: `$F007` reads A12-A15 and
        // A0-A2 and nothing between, so the six registers repeat through the whole page.
        core.mapper_write(port + 0x0A01, 0x80);
        core.mapper_write(port + 0x0FF8, 0x44);
        assert_eq!(
            [0x40, 0x41, 0x4E, 0x4F],
            coolboy_banks(&core),
            "submapper {submapper}: the mask is $F007"
        );

        // `$xxx6` and `$xxx7` are inside the window and are not registers: the ASIC provides six.
        core.mapper_write(port + 6, 0xFF);
        core.mapper_write(port + 7, 0xFF);
        assert_eq!(
            [0x40, 0x41, 0x4E, 0x4F],
            coolboy_banks(&core),
            "submapper {submapper}: six of the eight addresses are registers"
        );
    }
}

/// **Q-156**: §7.2 step 3 asks for the lockout "implemented as a no-op with the reason in a
/// comment" and the page refutes that, so MF-UNL-2's rule applies and the page wins.
///
/// nesdev gives `$xxx3` bit 7 as "Lockout (prevent further writes to all registers but the one at
/// `$xxx2`, only works in MMC3 mode)", and its *WRAM usage* section is why it cannot be a no-op:
/// on a COOLBOY the register window **is** the lower half of the WRAM window, so a game sets the
/// registers up with the MMC3's WRAM disabled, sets the lockout, and only then enables the WRAM.
/// With a no-op lockout the first byte a save routine wrote to `$6003` would re-bank the
/// cartridge.
#[cfg(feature = "board-268")]
#[test]
fn q_156_mapper_268_locks_its_registers_out_and_keeps_the_wram_behind_them() {
    let mut core = coolboy(8 * 1024 * 1024, 0);
    core.mapper_write(0x6001, 0x80);
    core.mapper_write(0x6000, 0x44);
    assert_eq!([0x40, 0x41, 0x4E, 0x4F], coolboy_banks(&core));

    // The page's own sequence, in its own order.
    core.mapper_write(0xA001, 0x00); // Disable WRAM.
    core.mapper_write(0x6003, 0x80); // Enable Lockout.
    core.mapper_write(0xA001, 0x80); // Enable WRAM.

    // Every register but `$xxx2` is held, so a WRAM write anywhere in $6000-$6FFF no longer banks
    // — and every one of those bytes reaches the WRAM, which is the point of the sequence.
    let writes = [
        (0x6000u16, 0x11u8),
        (0x6001, 0x22),
        (0x6003, 0x33),
        (0x6004, 0x44),
        (0x6123, 0x5A),
    ];
    for (address, value) in writes {
        core.mapper_write(address, value);
        assert_eq!(
            [0x40, 0x41, 0x4E, 0x4F],
            coolboy_banks(&core),
            "{address:#06X} must be locked out"
        );
    }
    for (address, value) in writes {
        assert_eq!(
            value,
            core.read_memory(address),
            "the WRAM behind {address:#06X}"
        );
    }

    // The lockout is not a latch of the write itself: it is bit 7 of `$xxx3` read every time, and
    // the page says it "only works in MMC3 mode" — so a board left in GNROM mode takes writes
    // again. `$xxx3` cannot be written while the lockout holds, so this is shown from the other
    // side, on a second cartridge: with the GNROM bit set beside the lockout bit, `$xxx0` moves.
    let mut core = coolboy(8 * 1024 * 1024, 0);
    core.mapper_write(0x6001, 0x80);
    core.mapper_write(0x6003, 0x80 | 0x10);
    core.mapper_write(0x6000, 0x44);
    assert_ne!(
        [0, 1, 0x3E, 0x3F],
        coolboy_banks(&core),
        "lockout does not work in GNROM mode"
    );
}

/// §7.2's "leave the rest refusing", read as the page's own index: it groups the thirteen PCB
/// codes by **submapper**, and the groups do not share a register layout — `$xxx0` has four
/// distinct bit diagrams on that page and `$xxx1` three. So this board is submappers **0 and 1**,
/// which is six of the thirteen codes and the six that carry all twelve titles, and submappers 2
/// to 11 are refused with REQ-CORE-011's typed cause (**Q-157**, MF-UNL-6 step 1).
#[cfg(feature = "board-268")]
#[test]
fn q_157_mapper_268_serves_two_submappers_and_refuses_the_nine_whose_layout_differs() {
    assert!(
        Cartridge::is_supported(268),
        "the number itself is supported"
    );

    for submapper in 0..=1u8 {
        let mut core = NesCore::new();
        let load = core.load_rom(&coolboy_image(512 * 1024, submapper), "Coolboy.nes");
        assert!(load.success, "submapper {submapper}: {:?}", load.error);
    }

    for submapper in 2..=11u8 {
        let mut core = NesCore::new();
        let load = core.load_rom(&coolboy_image(512 * 1024, submapper), "Coolboy.nes");
        assert!(!load.success, "submapper {submapper} must be refused");
        assert_eq!(
            zax_nes::rom::RomLoadFailure::UnsupportedMapper,
            load.failure,
            "submapper {submapper}"
        );
        assert!(
            load.error.unwrap().contains("268"),
            "submapper {submapper}: the message names the number"
        );
        assert!(!core.is_running(), "submapper {submapper}");
    }
}

/// nesdev's *Notes*: "AA6023's IRQ operator follows MMC3 **'NEC' behavior**, i.e. a latch value
/// of zero disables IRQ" — which is `Mmc3Board::irq_revision_a`, the counter mapper 4 reaches
/// through NES 2.0 submapper 4 and mappers 114 and 182 through their own page's wording.
///
/// The contrast is the one
/// `req_core_114_mapper_114_disables_the_irq_on_a_zero_latch_and_115_does_not` draws: in this
/// build the MMC3 proper is revision B and gives **three** IRQs over three rises with a latch of
/// `$00` (REQ-CORE-114's own `mmc3_test_2` test 6), where this board gives one.
#[cfg(feature = "board-268")]
#[test]
fn req_core_114_mapper_268_disables_the_irq_on_a_zero_latch() {
    for (mapper, expected) in [(4usize, 3), (268, 1)] {
        let mut core = MapperRom::new(mapper).prg(16).chr(64).load();
        core.write_memory(0x2001, 0x00); // No rendering: only the rises below clock it.
        core.mapper_write(0xC000, 0x00);
        core.mapper_write(0xC001, 0x00);
        core.mapper_write(0xE001, 0x00);

        let mut irqs = 0;
        for i in 0..3i64 {
            {
                let cartridge = core.cartridge_mut().unwrap();
                cartridge.on_ppu_address_bus(0x0000, 10_000 + i * 100);
                cartridge.on_ppu_address_bus(0x1000, 10_000 + i * 100 + 50);
            }

            if core.cartridge().unwrap().irq_asserted() {
                irqs += 1;
                core.mapper_write(0xE000, 0x00);
                core.mapper_write(0xE001, 0x00);
            }
        }

        assert_eq!(
            expected, irqs,
            "mapper {mapper}: a latch of $00 must give {expected} IRQ(s) over three rises"
        );
    }
}

/// A mapper-268 image with **both** a CHR-ROM and a CHR-RAM chip, which is the "KT-???" row of
/// the page's PCB table and the only configuration `$xxx4` is meaningful on (MF-UNL-6 step 2).
///
/// 256 KiB of PRG-ROM with the markers [`coolboy_banks`] reads, `chr_rom_1k` KiB of CHR-ROM with
/// every 1 KiB bank filled with its own number, and 8 KiB of CHR-RAM — which only an NES 2.0
/// header can declare beside a CHR-ROM, byte 11 being the CHR-RAM's own field.
#[cfg(feature = "board-268")]
fn coolboy_chr_image(chr_rom_1k: usize) -> Vec<u8> {
    let prg_bytes = 256 * 1024;
    let chr_bytes = chr_rom_1k * 1024;
    let mut image = vec![0u8; 16 + prg_bytes + chr_bytes];
    image[..4].copy_from_slice(&[0x4E, 0x45, 0x53, 0x1A]);
    image[4] = (prg_bytes / (16 * 1024)) as u8;
    image[5] = (chr_bytes / (8 * 1024)) as u8;
    image[6] = 0xC0;
    image[7] = 0x08; // NES 2.0.
    image[8] = 0x01; // Submapper 0, mapper bits 8-11 = 1.
    image[11] = 0x07; // 8 KiB of CHR-RAM beside the CHR-ROM.

    for bank in 0..prg_bytes / 0x2000 {
        let start = 16 + bank * 0x2000;
        image[start] = (bank >> 8) as u8;
        image[start + 1] = bank as u8;
    }
    for bank in 0..chr_rom_1k {
        let start = 16 + prg_bytes + bank * 1024;
        image[start..start + 1024].fill(bank as u8);
    }

    image
}

/// Banking mode **`$10`**, "normal" GNROM mode (MF-UNL-6 step 2).
///
/// nesdev: "PRG A16 and A15 come from the `QQ` controls. When in 16 KiB PRG mode (`L` control),
/// PRG A14 comes from the `R` control. Otherwise, it comes from CPU A14. CHR A16, A15, A14, and
/// A13 come from `MMMM` controls. CHR A12, A11, and A10 come from PPU A12, A11, and A10."
///
/// The two mask registers are opened to the offset first, so that the MMC3's own A17 and A18 are
/// out of the way and what is left is the GNROM page alone — which is also the page's own note on
/// this mode: "The CHR and PRG mask registers **STILL AFFECT THIS MODE**, allowing weird splicing
/// of MMC3 banking (with 128 KiB granularity) overlaying GNROM-style banking." The last block
/// below is that sentence.
#[cfg(feature = "board-268")]
#[test]
fn req_core_102_mapper_268_banks_a_gnrom_page_in_banking_mode_10() {
    let mut core = MapperRom::new(268).prg(256).chr(128).load();
    core.mapper_write(0x6000, 0x40); // B: PRG A17 from the offset.
    core.mapper_write(0x6001, 0x80); // G: PRG A18 from the offset; H and I leave A19/A20 there.
    core.mapper_write(0x6003, 0x10); // GNROM mode, `L` clear: 16 KiB banks.

    // `L` = 0, `R` = 0: one 16 KiB bank at both halves, which is an NROM-128.
    assert_eq!([0, 1, 0, 1], core.prg_windows(), "16 KiB, R = 0");
    core.mapper_write(0x6003, 0x10 | 0x02); // `R` = 1.
    assert_eq!([2, 3, 2, 3], core.prg_windows(), "16 KiB, R = 1");

    // `L` = 1: a 32 KiB bank with PRG A14 back on CPU A14, which is an NROM-256.
    core.mapper_write(0x6001, 0x82);
    core.mapper_write(0x6003, 0x10);
    assert_eq!([0, 1, 2, 3], core.prg_windows(), "32 KiB");

    // `QQ` is PRG A16 and A15 above it, so the page walks in 32 KiB steps.
    for (qq, bank) in [(0x00u8, 0u8), (0x04, 4), (0x08, 8), (0x0C, 12)] {
        core.mapper_write(0x6003, 0x10 | qq);
        assert_eq!(
            [bank, bank + 1, bank + 2, bank + 3],
            core.prg_windows(),
            "QQ = {qq:#04X}"
        );
    }

    // CHR: `MMMM` is A16-A13 and the window index is PPU A12-A10, so the eight windows are one
    // 8 KiB page of the CHR-ROM selected in 8 KiB steps.
    core.mapper_write(0x6002, 0x70 | 0x02); // TTT = 7 (nothing masked), MMMM = 2.
    assert_eq!(
        [16, 17, 18, 19, 20, 21, 22, 23],
        core.chr_windows(),
        "MMMM = 2"
    );

    // `TTT` is "ANDed with CHR A16, A15, A14 in MMMM", so clearing it collapses the offset to its
    // own A13 — and A13 is `MMMM` bit 0, which the mask never covers.
    core.mapper_write(0x6002, 0x0E);
    assert_eq!(
        [0, 1, 2, 3, 4, 5, 6, 7],
        core.chr_windows(),
        "TTT = 0 masks A16-A14 away"
    );
    core.mapper_write(0x6002, 0x01);
    assert_eq!(
        [8, 9, 10, 11, 12, 13, 14, 15],
        core.chr_windows(),
        "and A13 comes through it"
    );

    // "GNROM CHR bitmask lock (1: prevents subsequent updates to S or T bits)": with bit 7 set a
    // later write still moves `MMMM` and no longer `TTT`.
    core.mapper_write(0x6002, 0x80 | 0x70 | 0x0E);
    assert_eq!(
        [112, 113, 114, 115, 116, 117, 118, 119],
        core.chr_windows(),
        "MMMM = $E with nothing masked"
    );
    core.mapper_write(0x6002, 0x02);
    assert_eq!(
        [16, 17, 18, 19, 20, 21, 22, 23],
        core.chr_windows(),
        "the lock keeps TTT = 7 and takes the new MMMM"
    );

    // And the mask registers still affect this mode: `EEE` bit 0 is PRG A17, so the whole GNROM
    // page moves by 128 KiB.
    core.mapper_write(0x6003, 0x10);
    core.mapper_write(0x6000, 0x41);
    assert_eq!([16, 17, 18, 19], core.prg_windows(), "A17 from the offset");
}

/// Banking mode **`$40`**, the "weird" MMC3 mode — which the page's own *Notes* say "no cart has
/// been discovered so far that makes use of", and which is implemented because §7.2 step 1 asks
/// for "the GNROM-style and 'weird' MMC3 variants it documents" (MF-UNL-6 step 2).
///
/// nesdev: "If MMC3 PRG ROM bank mode is 'normal' (8+8+16F), then the PRG banks at `0xC000` and
/// `0xE000` are fixed to 8 KiB **bank 0**, instead of `0xFE` and `0xFF`. Supposedly this effect
/// is **completely ignored** when MMC3 PRG ROM bank mode is inverted (8F+8+8+8F). The second half
/// of each of the 2 KiB CHR banks is replaced with bank 0, and the LSB of MMC3 registers 0 and 1
/// work (instead of being replaced by PPU A10)."
#[cfg(feature = "board-268")]
#[test]
fn req_core_102_mapper_268_fixes_the_top_windows_to_bank_zero_in_banking_mode_40() {
    let mut core = MapperRom::new(268).prg(256).chr(128).load();
    core.mapper_write(0x6001, 0x60); // H and I: the chip drives A19 and A20 too.
    core.mmc3_bank(6, 0x05);
    core.mmc3_bank(7, 0x09);
    assert_eq!([5, 9, 0xFE, 0xFF], core.prg_windows(), "a plain 2 MiB MMC3");

    // The weird mode, with the MMC3's own PRG mode "normal".
    core.mapper_write(0x6003, 0x40);
    assert_eq!(
        [5, 9, 0, 0],
        core.prg_windows(),
        "the two fixed banks are 0"
    );

    // "Completely ignored when MMC3 PRG ROM bank mode is inverted": $8000 bit 6 swaps the
    // switchable window to $C000, and the rule stops applying.
    core.mapper_write(0x8000, 0x40);
    assert_eq!([0xFE, 9, 5, 0xFF], core.prg_windows(), "inverted: ignored");
    core.mapper_write(0x6003, 0x00);
    assert_eq!(
        [0xFE, 9, 5, 0xFF],
        core.prg_windows(),
        "inverted: and the same without the mode"
    );

    // The CHR half, with the chip's PRG mode back to normal and its CHR swap clear, so that the
    // two 2 KiB registers drive the first pattern table.
    core.mapper_write(0x8000, 0x00);
    core.mmc3_bank(0, 0x11);
    core.mmc3_bank(1, 0x23);
    for register in 2..6u8 {
        core.mmc3_bank(register, 0x2E + register);
    }
    assert_eq!(
        [0x10, 0x11, 0x22, 0x23, 0x30, 0x31, 0x32, 0x33],
        core.chr_windows(),
        "the MMC3's own CHR layout, bit 0 of R0 and R1 ignored"
    );

    core.mapper_write(0x6003, 0x40);
    assert_eq!(
        [0x11, 0, 0x23, 0, 0x30, 0x31, 0x32, 0x33],
        core.chr_windows(),
        "the LSB of R0 and R1 works and the second half of each pair is bank 0"
    );
}

/// Banking mode **`$50`**, the "weird GNROM-ish" mode: "Combine the rules mentioned above for
/// 'Normal' GNROM mode with 'Weird' MMC3 mode. (These descriptions should not be contradictory.)"
/// (MF-UNL-6 step 2).
///
/// They are not contradictory, and this test is what that costs: in GNROM mode the MMC3's CHR
/// registers reach nothing, so the weird rule about their LSB has nothing to act on, and what is
/// left of "weird" is the two fixed **PRG** windows becoming bank 0 — visible here because the
/// mask leaves the chip A17 and A18, so `$FE` and `$FF` carry bits the offset does not.
#[cfg(feature = "board-268")]
#[test]
fn req_core_102_mapper_268_combines_both_rules_in_banking_mode_50() {
    let mut core = MapperRom::new(268).prg(256).chr(128).load();
    core.mapper_write(0x6001, 0x02); // `L` = 1 (32 KiB GNROM pages); the masks stay at power-on.
    core.mmc3_bank(6, 0x05);

    core.mapper_write(0x6003, 0x10);
    assert_eq!(
        [0, 1, 0x32, 0x33],
        core.prg_windows(),
        "mode $10 splices the fixed banks' A17 and A18 over the GNROM page"
    );

    core.mapper_write(0x6003, 0x50);
    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "mode $50 makes those two bank 0 first, so the page is whole"
    );

    // And the weird half is "completely ignored when MMC3 PRG ROM bank mode is inverted" here
    // too, so with $8000 bit 6 set the two modes are the same layout.
    core.mapper_write(0x8000, 0x40);
    core.mapper_write(0x6003, 0x10);
    let inverted = core.prg_windows();
    core.mapper_write(0x6003, 0x50);
    assert_eq!(
        inverted,
        core.prg_windows(),
        "inverted: $10 and $50 agree, which is what 'completely ignored' means"
    );
    assert_eq!([48, 1, 2, 51], inverted);
}

/// `$xxx4`, the mixed CHR-ROM/CHR-RAM window — "only meaningful on cartridges that mount both
/// CHR-ROM and CHR-RAM", and "by writing the appropriate value to D1-D7, the functionality of
/// INES Mapper 074 and INES Mapper 194 is replicated" (MF-UNL-6 step 2).
///
/// That last sentence is what pins the comparison: mapper 74's CHR-RAM is banks `$08-$09` and
/// mapper 194's `$00-$01`, each a 2 KiB pair whose two banks differ in D0 alone — so the register
/// names a **pair** through bits 7-1, and `$xxx4 = $09` *is* mapper 74 on this chipset.
#[cfg(feature = "board-268")]
#[test]
fn req_core_102_mapper_268_mixes_chr_ram_into_its_chr_rom_windows() {
    let mut core = NesCore::new();
    let load = core.load_rom(&coolboy_chr_image(64), "CoolboyChr.nes");
    assert!(load.success, "{:?}", load.error);

    // The MMC3's own CHR layout: R0 = $08 puts CHR-ROM banks $08 and $09 at the first 2 KiB.
    core.mmc3_bank(0, 0x08);
    core.mmc3_bank(1, 0x0C);
    for register in 2..6u8 {
        core.mmc3_bank(register, 0x10 + register);
    }
    assert_eq!(
        [8, 9, 12, 13, 18, 19, 20, 21],
        core.chr_windows(),
        "CHR-ROM everywhere"
    );

    // `$xxx4 = $09`: mixed mode on, and D1-D7 = $04 — the register value $08, which is R0's.
    core.mapper_write(0x6004, 0x09);
    assert_eq!(
        [0, 0, 12, 13, 18, 19, 20, 21],
        core.chr_windows(),
        "R0's 2 KiB pair reads the CHR-RAM chip, which is blank"
    );

    // And it is RAM: a PPU write to a selected window sticks, where one to a CHR-ROM window does
    // not. `Mapper::ppu_write`'s default drops every write on a board that has CHR-ROM, so
    // without the override the chip would be unreachable.
    chr_write(&mut core, 0x0000, 0x5A);
    chr_write(&mut core, 0x0400, 0x6B);
    chr_write(&mut core, 0x1000, 0xFF);
    assert_eq!(
        [0x5A, 0x6B, 12, 13, 18, 19, 20, 21],
        core.chr_windows(),
        "the CHR-RAM takes the write and the CHR-ROM does not"
    );

    // The select follows the register value, not the window: pointing it at R1's byte moves the
    // RAM pair to the second 2 KiB without touching the first.
    core.mapper_write(0x6004, ((0x0C >> 1) << 1) | 0x01);
    assert_eq!(
        [8, 9, 0, 0, 18, 19, 20, 21],
        core.chr_windows(),
        "R1's pair instead"
    );

    // Bit 0 clear is "Select CHR-ROM only", and the whole mechanism goes away.
    core.mapper_write(0x6004, 0x08);
    assert_eq!(
        [8, 9, 12, 13, 18, 19, 20, 21],
        core.chr_windows(),
        "CHR-ROM only again"
    );

    // Mapper 194's own row, on the same chipset: `$xxx4 = $01` is D1-D7 = 0, the pair $00-$01.
    core.mmc3_bank(0, 0x00);
    core.mapper_write(0x6004, 0x01);
    assert_eq!(
        [0x5A, 0x6B, 12, 13, 18, 19, 20, 21],
        core.chr_windows(),
        "the bytes written above are still in the RAM chip"
    );
}

/// **Q-159**: `$xxx3` bit 5 is "1: Also enable PRG RAM in `$5000-$5FFF`", and that one clause is
/// the whole of what the page says about it — no mapping for a 4 KiB window over an 8 KiB chip,
/// and no game named that uses it (MF-UNL-6 step 2).
///
/// Two readings, both recorded in the ticket. **A COOLBOY only**: on a MINDKIDS the register file
/// is at `$5000-$5FFF` under mask `$F007`, which decodes the whole page, so the bit cannot mean
/// what it says there — and the last block below is that, from the other side. **CPU A0-A12
/// straight through**, so the window shows the chip's second 4 KiB, the same bytes as
/// `$7000-$7FFF`; that is what an ASIC asserting a chip select over an unchanged address bus
/// does.
#[cfg(feature = "board-268")]
#[test]
fn q_159_mapper_268_opens_a_second_prg_ram_window_at_5000_from_xxx3_bit_5() {
    let mut core = coolboy(512 * 1024, 0);
    core.mapper_write(0x7400, 0x5A);
    core.mapper_write(0x7123, 0xA5);

    // Closed, a write to $5000-$5FFF reaches nothing.
    core.mapper_write(0x5400, 0xFF);
    assert_eq!(0x5A, core.read_memory(0x7400), "the window is closed");

    // Open, it is the chip's second 4 KiB.
    core.mapper_write(0x6003, 0x20);
    assert_eq!(0x5A, core.read_memory(0x5400), "$5400 is $7400");
    assert_eq!(0xA5, core.read_memory(0x5123), "and $5123 is $7123");
    core.mapper_write(0x5456, 0x3C);
    assert_eq!(0x3C, core.read_memory(0x7456), "and it takes writes");

    // "Also": the MMC3's own $A001 still gates it, the bit adding a window and not an enable.
    core.mapper_write(0xA001, 0xC0); // Enabled (bit 7) and write-protected (bit 6).
    core.mapper_write(0x5456, 0x00);
    assert_eq!(
        0x3C,
        core.read_memory(0x5456),
        "write-protected through $A001"
    );

    // Closed again, and the byte stays where it was.
    core.mapper_write(0xA001, 0x80);
    core.mapper_write(0x6003, 0x00);
    core.mapper_write(0x5456, 0x00);
    assert_eq!(0x3C, core.read_memory(0x7456), "closed again");

    // On a MINDKIDS the same bit opens nothing, because that range is the register file: a write
    // at $5400 is `$xxx0` through the mask and banks the cartridge.
    let mut core = coolboy(512 * 1024, 1);
    core.mapper_write(0x5001, 0x80);
    core.mapper_write(0x5003, 0x20);
    let before = coolboy_banks(&core);
    core.mapper_write(0x5400, 0x44);
    assert_ne!(
        before,
        coolboy_banks(&core),
        "on a MINDKIDS $5400 is still $xxx0"
    );
}

/// A 16-byte mapper-268 header and nothing behind it, declaring `prg_low`/`prg_msb` bytes of
/// PRG-ROM and `chr_low`/`chr_msb` of CHR-ROM in NES 2.0's own two fields (MF-UNL-6 step 3).
///
/// For the refusal half of the size pair: an image whose header announces more than it carries
/// must come back with REQ-CORE-011's typed cause rather than panicking, whatever arithmetic the
/// announcement needs.
#[cfg(feature = "board-268")]
fn coolboy_header(prg_low: u8, prg_msb: u8, chr_low: u8, chr_msb: u8) -> Vec<u8> {
    let mut image = vec![0u8; 16];
    image[..4].copy_from_slice(&[0x4E, 0x45, 0x53, 0x1A]);
    image[4] = prg_low;
    image[5] = chr_low;
    image[6] = 0xC0;
    image[7] = 0x08;
    image[8] = 0x01;
    image[9] = (chr_msb << 4) | (prg_msb & 0x0F);
    image
}

/// §7.2 step 2's own request — "Lift the PRG/CHR size ceiling in the loader and in
/// `board_memory_sizes` to what a 64 MiB image needs — a check this build has never had to make,
/// and the lot's real risk" — and step 4's "header-size regression test: an image declaring
/// 64 MiB of PRG loads, and one declaring more is refused with REQ-CORE-011's message rather than
/// panicking" (MF-UNL-6 step 3).
///
/// **There was no ceiling in the loader to lift**, which is the step's finding and **Q-158**
/// records it: `InesHeader::try_parse` reads NES 2.0's twelve-bit size field and its
/// exponent-multiplier form and imposes no maximum of its own, `BankWindows` counts banks in an
/// `i32` (8 192 of them at 64 MiB), and every bank number is wrapped modulo that count. What
/// refuses an over-size *declaration* is the truncation check — the header announces more bytes
/// than the file carries — and what makes a pathological announcement safe rather than fatal is
/// `nes20_rom_size`'s `wrapping_shl` plus `usize::try_from`, which turn an exponent of 63 into
/// `usize::MAX` and so into the same refusal.
#[cfg(feature = "board-268")]
#[test]
fn req_core_011_a_64_mib_image_loads_and_a_larger_declaration_is_refused() {
    // The YH2018A's own size, in the only form a header can carry it: multiplier 1, exponent 26.
    let mut core = coolboy(64 * 1024 * 1024, 0);
    assert_eq!(
        64 * 1024 * 1024,
        core.cartridge().unwrap().memory().rom().prg_rom().len(),
        "the whole 64 MiB came through the header's exponent form"
    );
    assert_eq!(
        [0, 1, 0x3E, 0x3F],
        coolboy_banks(&core),
        "and it is an MMC3"
    );

    // The two outer lines no image below 32 MiB can show: `CC` of `$xxx0` is PRG A24 and A23.
    core.mapper_write(0x6000, 0x40); // Every mask bit to the offset.
    core.mapper_write(0x6001, 0x80);
    for (value, line, bank) in [
        (0x40u8, "no line", 0x000u16),
        (0x50, "A23", 0x400),
        (0x60, "A24", 0x800),
        (0x70, "A24 and A23", 0xC00),
    ] {
        core.mapper_write(0x6000, value);
        assert_eq!(
            [bank, bank | 0x01, bank | 0x0E, bank | 0x0F],
            coolboy_banks(&core),
            "{line}: $6000 = {value:#04X}"
        );
    }

    // And a declaration this build refuses: 128 MiB announced by a 16-byte file.
    for (prg_low, prg_msb, chr_low, chr_msb, what) in [
        (27u8 << 2, 0x0Fu8, 0x00u8, 0x00u8, "128 MiB of PRG-ROM"),
        // Exponent 63 with multiplier 7, which overflows the shift: `wrapping_shl` plus
        // `usize::try_from` make it `usize::MAX`, so the refusal is the same one.
        (0xFF, 0x0F, 0x00, 0x00, "an impossible PRG exponent"),
        (0x20, 0x00, 29 << 2, 0x0F, "512 MiB of CHR-ROM"),
    ] {
        let mut core = NesCore::new();
        let load = core.load_rom(
            &coolboy_header(prg_low, prg_msb, chr_low, chr_msb),
            "Huge.nes",
        );
        assert!(!load.success, "{what} must be refused");
        assert_eq!(
            zax_nes::rom::RomLoadFailure::TruncatedFile,
            load.failure,
            "{what}"
        );
        assert!(!core.is_running(), "{what}");
    }
}

/// **Q-158**: nesdev's *64 MiB mapping*, which is the one row of this board's own table that a
/// bank-window core cannot express (MF-UNL-6 step 3).
///
/// > The **YH2018A** PCB uses the SC0 (`$6001` bit 0) and SC1 (`$6003` bit 0) bits to address two
/// > halves of 32 MiB for a total of 64 MiB. … `0`/`0`: PRG A25=PRG A0, i.e. bytes at even
/// > addresses come from the first 32 MiB and bytes at odd addresses come from the second 32 MiB
/// > half. `0`/`1`: PRG A25=1, i.e. second 32 MiB half. `1`/`x`: PRG A25=0, i.e. first 32 MiB
/// > half.
///
/// The first row interleaves two chips byte by byte, and a [`zax_nes::cartridges::BankWindows`]
/// window is a byte offset plus a length. It is also the **reset** state — and the page says what
/// the hardware's users do about that: "either the code at the reset vector must be carefully
/// placed with bytes alternating between the two 32 MiB halves, or (as the existing multicarts
/// do) **must be duplicated in both 32 MiB halves**." So this board answers the *first* half for
/// that row, the half every known dump duplicates its boot code into, and the ticket records what
/// would close it.
#[cfg(feature = "board-268")]
#[test]
fn q_158_mapper_268_reads_the_second_32_mib_half_only_when_sc1_is_set_alone() {
    let mut core = coolboy(64 * 1024 * 1024, 0);
    core.mapper_write(0x6000, 0x40); // Every mask bit to the offset, every offset line clear.

    for (sc0, sc1, half, why) in [
        (
            0u8,
            0u8,
            0x0000u16,
            "PRG A25 = PRG A0, answered as the first half",
        ),
        (0, 1, 0x1000, "PRG A25 = 1, the second 32 MiB half"),
        (1, 0, 0x0000, "PRG A25 = 0"),
        (1, 1, 0x0000, "PRG A25 = 0 whatever SC1 says"),
    ] {
        core.mapper_write(0x6001, 0x80 | sc0);
        core.mapper_write(0x6003, sc1);
        assert_eq!(
            [half, half | 0x01, half | 0x0E, half | 0x0F],
            coolboy_banks(&core),
            "SC0 = {sc0}, SC1 = {sc1}: {why}"
        );
    }

    // The line is driven on every image and not only on a declared YH2018A, because no header
    // field distinguishes one — and on a smaller image it wraps away, which is what the missing
    // address line of the real chip does.
    let mut core = coolboy(512 * 1024, 0);
    core.mapper_write(0x6000, 0x40);
    core.mapper_write(0x6001, 0x80);
    let first = coolboy_banks(&core);
    core.mapper_write(0x6003, 0x01);
    assert_eq!(
        first,
        coolboy_banks(&core),
        "on a 512 KiB image PRG A25 reaches nothing"
    );
}

/// `registry::board_memory_sizes`'s row for this number: the page's "8 KiB of WRAM" as a **floor**,
/// and the CHR left to the header because the page gives a maximum per PCB rather than a figure
/// (MF-UNL-6 step 3).
#[cfg(feature = "board-268")]
#[test]
fn req_core_012_mapper_268_keeps_its_8_kib_of_wram_and_takes_its_chr_from_the_header() {
    // The CHR-RAM-only row of the PCB table: 8 KiB declared by byte 11, which is what
    // `coolboy_image` writes.
    let core = coolboy(512 * 1024, 0);
    let cartridge = core.cartridge().unwrap();
    assert_eq!(
        0x2000,
        cartridge.memory().chr_ram().len(),
        "the declared 8 KiB"
    );
    assert_eq!(0x2000, cartridge.memory().prg_ram().len(), "and the WRAM");

    // The mixed row, where the same board carries both chips: the CHR-ROM is the image's and the
    // CHR-RAM is byte 11's, side by side.
    let mut core = NesCore::new();
    let load = core.load_rom(&coolboy_chr_image(64), "CoolboyChr.nes");
    assert!(load.success, "{:?}", load.error);
    let cartridge = core.cartridge().unwrap();
    assert_eq!(64 * 1024, cartridge.memory().rom().chr_rom().len());
    assert_eq!(0x2000, cartridge.memory().chr_ram().len());
}

/// §7.1 item 2 for this lot's one board: a save-state round-trip with **all six** outer registers
/// dirtied, the CHR-RAM chip of `$xxx4`'s mixed mode included (MF-UNL-6 step 4).
///
/// The generic sweep `req_core_090_the_mapper_state_survives_a_save_and_restore` reaches this
/// number by being registered, but it writes `$8000-$E001` and `$6000` — and of this board's six
/// registers only `$xxx0` is at any of those addresses. So the six are written here, the state is
/// taken, every one of them is moved, and the restored block is required to be identical byte for
/// byte.
#[cfg(feature = "board-268")]
#[test]
fn req_core_090_mapper_268_round_trips_its_six_outer_registers() {
    let mut core = NesCore::new();
    let load = core.load_rom(&coolboy_chr_image(64), "CoolboyChr.nes");
    assert!(load.success, "{:?}", load.error);

    // The layout the board powers on with, for the reset check at the end.
    let power_on = (coolboy_banks(&core), core.chr_windows());

    // The MMC3 underneath, then the board's own six.
    for register in 0..8u8 {
        core.mmc3_bank(register, register + 0x11);
    }
    core.mapper_write(0x6000, 0x44); // Masks and the A19 offset.
    core.mapper_write(0x6001, 0x82); // G, L and the SC0 bit.
    core.mapper_write(0x6002, 0x75); // The GNROM CHR offset and its bitmask.
    core.mapper_write(0x6003, 0x02); // `R`, with the banking mode left in MMC3 mode.
    core.mapper_write(0x6004, 0x11); // Mixed CHR-RAM/CHR-ROM: R0's pair, whose value is $11.
    core.mapper_write(0x6005, 0x10); // `$xxx5`, which this build latches and does not act on.
    chr_write(&mut core, 0x0000, 0x7E); // Into the CHR-RAM the line above selected.
    core.write_memory(0x7456, 0x4B); // And the WRAM behind the register window.

    let state = core.save_state(true).unwrap();
    let prg = coolboy_banks(&core);
    let chr = core.chr_windows();
    let ram_byte = core.read_memory(0x7456);

    // Move every one of the six, and the chip select with them.
    for register in 0..6u16 {
        core.mapper_write(0x6000 + register, 0x00);
    }
    for _ in 0..5 {
        core.run_frame(0);
    }
    assert_ne!(prg, coolboy_banks(&core), "the move must be visible in PRG");
    assert_ne!(chr, core.chr_windows(), "and in CHR");

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(prg, coolboy_banks(&core), "the PRG windows");
    assert_eq!(
        chr,
        core.chr_windows(),
        "the CHR windows and the chip select"
    );
    assert_eq!(0x7E, core.chr_bank(0x0000), "the CHR-RAM byte");
    assert_eq!(ram_byte, core.read_memory(0x7456), "the WRAM");
    assert_eq!(state, core.save_state(true).unwrap(), "the whole block");

    // AMD-061 §5: and a reset puts all six back to zero beside the MMC3's own, which is the
    // layout the board powered on with — a 512 KiB MMC3 with the CHR-ROM in every window.
    core.reset();
    assert_eq!(
        power_on,
        (coolboy_banks(&core), core.chr_windows()),
        "a reset clears all six outer registers"
    );
}

// ------------------------------------------- the Sachen 8259 (137, 138, 139, 141),
// ------------------------------------------- MF-UNL-7 step 1

/// The board's one idiom, which is the eight-register ASIC's: select a register at $4100, then
/// write it at $4101. A helper of its own because the two boards are behind different features.
#[cfg(feature = "board-sachen8259")]
fn s8259_write(core: &mut NesCore, register: u8, value: u8) {
    core.mapper_write(0x4100, register);
    core.mapper_write(0x4101, value);
}

/// Every register the two pages document is reachable through the index/data pair, and each drives
/// what its row says (nesdev, "Sachen 8259": "Chr Select (Internal 0-3)", "Chr Outer Bank
/// (Internal 4)", "Prg Bank (Internal 5)", "Mode and Mirroring Select (Internal 7)";
/// "INES Mapper 137" adds "CHR 3 Bit #3 (Internal 6)").
///
/// Measured on the 8259B, mapper 138, because it is the one variant whose bank unit *is* its
/// window — "Passes PPU A[10:0] through to CHR ROM A[10:0]" and "6 bits of chr reg to CHR ROM
/// A[16:11]" — so a register value and a 2 KiB bank number are the same number here, and the
/// arithmetic of the other three is the next test's business (MF-UNL-7 step 1).
#[cfg(feature = "board-sachen8259")]
#[test]
fn req_core_102_the_sachen_8259_reaches_registers_0_to_5_and_7() {
    let mut core = MapperRom::new(138).prg(16).chr(128).load();

    // Power-on: every register zero, so block 0 of CHR in all four windows and PRG bank 0. On the
    // 8259B each window is 2 KiB and each takes its own register, so the first 2 KiB shows four
    // times over.
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!([0, 1, 0, 1, 0, 1, 0, 1], core.chr_windows());

    // R0-R3, "Select lower 3 bits of 2KiB CHR block": one register per block, three bits each.
    for block in 0u8..4 {
        s8259_write(&mut core, block, block + 1);
    }
    assert_eq!([2, 3, 4, 5, 6, 7, 8, 9], core.chr_windows());

    // R4, "Select upper 3 bits of all four 2KiB CHR blocks": the same three bits above every one
    // of them, so all four windows move by the same eight 2 KiB banks.
    s8259_write(&mut core, 4, 0x01);
    assert_eq!([18, 19, 20, 21, 22, 23, 24, 25], core.chr_windows());
    s8259_write(&mut core, 4, 0x07);
    assert_eq!([114, 115, 116, 117, 118, 119, 120, 121], core.chr_windows());

    // R5, "Select 32KiB PRG bank for CPU $8000": three bits, over this image's four banks.
    s8259_write(&mut core, 5, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    s8259_write(&mut core, 5, 0x02);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    s8259_write(&mut core, 5, 0x07);
    assert_eq!([12, 13, 14, 15], core.prg_windows(), "and bank 7 wraps");

    // R7, `.... .mms`: the mirroring mode. Mode 1 is `H` on the family page.
    s8259_write(&mut core, 7, 0x02);
    assert_eq!(Nm::Horizontal, core.mirroring());

    // R6 is **not** on the family page, and on these three numbers it drives nothing. The write
    // still lands — the index is three bits wide — and the next test but one shows the single
    // number where the bit is wired.
    let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
    s8259_write(&mut core, 6, 0x07);
    assert_eq!(
        banks,
        (core.prg_windows(), core.chr_windows(), core.mirroring()),
        "R6 drives nothing on the 8259A, B and C"
    );

    // And there is **no read port**: the family page says nothing like mapper 150's "All three
    // bits in all eight registers are fully readable and writable", so the data port is
    // write-only here and a read of it is open bus, which this core answers as 0.
    assert_eq!(0x00, core.read_memory(0x4101));
    assert_eq!(0x00, core.read_memory(0x4100));
}

/// The *CHR Rewiring* table as arithmetic: the four wirings differ by how far the six register
/// bits are shifted above the 2 KiB window the PPU's own A12-A11 chose (nesdev, "Sachen 8259").
///
/// With every register zero the shift alone is visible, and the three signatures are three
/// pictures of the same silicon:
///
/// * the **8259B** (138) passes PPU A10-A0, so each window is its own 2 KiB bank and the first
///   2 KiB shows four times;
/// * the **8259A** (141) passes A11-A0, so A11 is an address line of a 4 KiB bank *and* the
///   window selector, and the first 4 KiB shows twice;
/// * the **8259C** (139) passes A12-A0, so the four windows are one contiguous 8 KiB bank.
///
/// The page's capacities are the same statement: 128, 256 and 512 KiB for six bits of register
/// times a 2, 4 or 8 KiB unit.
#[cfg(feature = "board-sachen8259")]
#[test]
fn req_core_102_the_sachen_8259_chr_shift_tells_its_four_wirings_apart() {
    let windows = |mapper: usize| MapperRom::new(mapper).prg(16).chr(128).load().chr_windows();
    assert_eq!([0, 1, 0, 1, 0, 1, 0, 1], windows(138), "the 8259B, shift 0");
    assert_eq!([0, 1, 2, 3, 0, 1, 2, 3], windows(141), "the 8259A, shift 1");
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], windows(139), "the 8259C, shift 2");

    // And one register value lands at a different bank on each: the same `R0 = 1` is 2 KiB bank
    // 1, 2 and 4 respectively, which is CHR A11, A12 and A13 in the chip's own numbering.
    for (mapper, expected) in [(138usize, 2u8), (141, 4), (139, 8)] {
        let mut core = MapperRom::new(mapper).prg(16).chr(128).load();
        s8259_write(&mut core, 0, 0x01);
        assert_eq!(
            expected,
            core.chr_bank(0x0000),
            "mapper {mapper}: R0 = 1 reaches 1 KiB bank {expected}"
        );
    }

    // The 8259D is the fourth, and it is not a shift at all: four **1 KiB** windows over
    // $0000-$0FFF and "PPU $1000-$1FFF: fixed to last 4KiB of CHR ROM".
    let mut core = MapperRom::new(137).prg(16).chr(128).load();
    assert_eq!(
        [0, 0, 0, 0, 124, 125, 126, 127],
        core.chr_windows(),
        "the 8259D, and its fixed upper half"
    );
    s8259_write(&mut core, 0, 0x01);
    assert_eq!(1, core.chr_bank(0x0000), "one 1 KiB bank per register");
    assert_eq!(0, core.chr_bank(0x0400), "and only its own window moves");
}

/// Mapper 137's own page gives the 8259D an R4 that is **one bit per block** where the family
/// page's is three common upper bits, and an R6 the family page does not mention at all:
///
/// ```text
/// ...0 0ccc   CHR register 0
/// ...B 0ccc   CHR register 1
/// ...C 0ccc   CHR register 2
/// ...D Eccc   CHR register 3
/// ```
///
/// with R4 `.... .DCB` and R6 `.... ...E`. So window 0 reaches eight 1 KiB banks, windows 1 and 2
/// sixteen with a hole in the middle, and **only window 3 reaches the whole 32 KiB** the page
/// gives the chip — which is what its two extra bits are for (MF-UNL-7 step 1).
#[cfg(feature = "board-sachen8259")]
#[test]
fn req_core_102_mapper_137_puts_one_bit_of_r4_on_each_of_its_blocks() {
    let mut core = MapperRom::new(137).prg(16).chr(32).load();

    // `.... .DCB`: bit 0 is window 1's bit 4, bit 1 window 2's and bit 2 window 3's. Window 0 has
    // no bit of R4 at all — its diagram is `...0 0ccc`.
    s8259_write(&mut core, 4, 0x07);
    assert_eq!([0, 16, 16, 16], core.chr_windows()[..4]);
    s8259_write(&mut core, 4, 0x01);
    assert_eq!([0, 16, 0, 0], core.chr_windows()[..4], "B alone");
    s8259_write(&mut core, 4, 0x02);
    assert_eq!([0, 0, 16, 0], core.chr_windows()[..4], "C alone");
    s8259_write(&mut core, 4, 0x04);
    assert_eq!([0, 0, 0, 16], core.chr_windows()[..4], "D alone");

    // R6's `E` is bit **3** of CHR register 3 and of nothing else, so window 3 is the only one
    // that can reach a bank with bit 3 set.
    s8259_write(&mut core, 6, 0x01);
    assert_eq!([0, 0, 0, 24], core.chr_windows()[..4], "D and E together");
    s8259_write(&mut core, 4, 0x00);
    assert_eq!([0, 0, 0, 8], core.chr_windows()[..4], "E alone");

    // Which is what "only register 3 accesses the full ROM" means: 31 is the last 1 KiB bank of
    // the 32 KiB the page gives the chip, and `D`, `E` and `ccc` together are the only field that
    // reaches it.
    s8259_write(&mut core, 3, 0x07);
    s8259_write(&mut core, 4, 0x04);
    assert_eq!(31, core.chr_bank(0x0C00), "the last bank of 32 KiB");
}

/// R7's four layouts, and **the one place the two pages contradict each other** (Q-160): the
/// family page gives `0 = V, 1 = H, 2 = (0,1,1,1), 3 = 1scA` and mapper 137's page gives
/// `0 = H, 1 = V, 2 = (0,1,1,1), 3 = 1scA`, so modes 0 and 1 are swapped between them. Each
/// number takes its own page, which is MF-UNL-2's rule.
///
/// Mode 2 is the one this core cannot say with a `NametableMirroring` — one table on CIRAM page 0
/// and the other three on page 1 — so it comes out of `Mapper::nametable_page`, which is the
/// eight-register ASIC's mode 0 read the other way round (MF-UNL-7 step 1).
#[cfg(feature = "board-sachen8259")]
#[test]
fn q_160_the_sachen_8259_swaps_mirroring_modes_0_and_1_between_its_two_pages() {
    for (mapper, mode_0, mode_1) in [
        (138usize, Nm::Vertical, Nm::Horizontal),
        (139, Nm::Vertical, Nm::Horizontal),
        (141, Nm::Vertical, Nm::Horizontal),
        // The 8259D, whose own page is the second of the two.
        (137, Nm::Horizontal, Nm::Vertical),
    ] {
        let mut core = MapperRom::new(mapper).prg(16).chr(32).load();

        // Mode 0 at power-on, R7 being zero.
        assert_eq!(mode_0, core.mirroring(), "mapper {mapper}, mode 0");
        assert_eq!([-1, -1, -1, -1], nametable_pages(&core), "mapper {mapper}");

        // `mm` is bits 2-1, so the four modes are 0, 2, 4 and 6.
        s8259_write(&mut core, 7, 0x02);
        assert_eq!(mode_1, core.mirroring(), "mapper {mapper}, mode 1");

        // `(0,1,1,1)`: a per-table layout, so `nametable_page` answers and the mirroring field is
        // left where mode 1 put it.
        s8259_write(&mut core, 7, 0x04);
        assert_eq!(
            [0, 1, 1, 1],
            nametable_pages(&core),
            "mapper {mapper}, mode 2"
        );

        s8259_write(&mut core, 7, 0x06);
        assert_eq!(
            Nm::SingleScreenA,
            core.mirroring(),
            "mapper {mapper}, mode 3 is 1scA on both pages"
        );
        assert_eq!(
            [-1, -1, -1, -1],
            nametable_pages(&core),
            "and the pages follow the mirroring again"
        );
    }
}

/// Simple mode, R7 bit 0: "When simple mode is enabled, mirroring is fixed to V, ignoring the m
/// bits, and all 4 CHR banks use the CHR bank 0 register" (nesdev, "Sachen 8259").
///
/// Mapper 137's own page says only "mirroring is fixed to _H_, and banks become weird". So the
/// mirroring follows each number's own page — which is, on both pages, **that page's own mode 0**
/// — and the banks follow the family page's concrete statement, there being no other; **Q-160**
/// records that "weird" is all the 8259D's page says (MF-UNL-7 step 1).
#[cfg(feature = "board-sachen8259")]
#[test]
fn q_160_simple_mode_puts_chr_block_0_in_every_window() {
    let mut core = MapperRom::new(138).prg(16).chr(128).load();
    for block in 0u8..4 {
        s8259_write(&mut core, block, block + 1);
    }
    assert_eq!([2, 3, 4, 5, 6, 7, 8, 9], core.chr_windows());

    // `s` set: every window reads R0, which is 1 — and R4's outer bits still apply, the page's
    // sentence being about the bank registers.
    s8259_write(&mut core, 7, 0x01);
    assert_eq!([2, 3, 2, 3, 2, 3, 2, 3], core.chr_windows());
    assert_eq!(
        Nm::Vertical,
        core.mirroring(),
        "fixed to V on the family page"
    );
    s8259_write(&mut core, 4, 0x01);
    assert_eq!([18, 19, 18, 19, 18, 19, 18, 19], core.chr_windows());

    // "ignoring the m bits": mode 2's per-table layout does not come back while `s` is set.
    s8259_write(&mut core, 7, 0x05);
    assert_eq!([-1, -1, -1, -1], nametable_pages(&core));
    assert_eq!(Nm::Vertical, core.mirroring());

    // And clearing `s` restores all four blocks and the `mm` field at once.
    s8259_write(&mut core, 7, 0x04);
    assert_eq!([18, 19, 20, 21, 22, 23, 24, 25], core.chr_windows());
    assert_eq!([0, 1, 1, 1], nametable_pages(&core));

    // Mapper 137's own page fixes it to `H` instead, and that is the whole of the difference.
    let mut core = MapperRom::new(137).prg(16).chr(32).load();
    s8259_write(&mut core, 7, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring(), "fixed to H on mapper 137");
}

/// The 2M-RAM-COB board of *Q-Boy*, which the family page gives as the one CHR-RAM cartridge of
/// the family: it "replaces CHR-ROM with 8 KiB of unbanked CHR-RAM, and is usually set to mapper
/// 141".
///
/// So an image with no CHR-ROM is a board with **no CHR bank lines at all**: the four CHR
/// registers and the outer one are latched and drive nothing, the 8 KiB window stays where
/// `BankWindows::new` put it, and the bytes the PPU writes come back where they were written
/// (MF-UNL-7 step 1).
#[cfg(feature = "board-sachen8259")]
#[test]
fn req_core_102_the_2m_ram_cob_board_has_no_chr_bank_lines() {
    let mut core = MapperRom::new(141).prg(16).chr(0).load();

    // 8 KiB of CHR-RAM, unbanked and zero-filled.
    assert_eq!([0, 0, 0, 0, 0, 0, 0, 0], core.chr_windows());
    chr_write(&mut core, 0x0000, 0x5A);
    chr_write(&mut core, 0x1C00, 0xA5);
    assert_eq!(0x5A, core.chr_bank(0x0000));
    assert_eq!(0xA5, core.chr_bank(0x1C00));

    // Every CHR register written, and nothing moves: on a CHR-ROM board `R0 = 7` with `R4 = 7`
    // would be 2 KiB bank 126 on this wiring.
    for register in [0u8, 1, 2, 3, 4, 6] {
        s8259_write(&mut core, register, 0x07);
    }
    assert_eq!(0x5A, core.chr_bank(0x0000), "the window has not moved");
    assert_eq!(0xA5, core.chr_bank(0x1C00));

    // The PRG bank and the mirroring are the chip's and not the CHR chip's, so they still work.
    s8259_write(&mut core, 5, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    s8259_write(&mut core, 7, 0x02);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

/// REQ-CORE-090/091: the eight registers and the index survive a save and a restore, with the
/// state dirtied first — including R6 on the three numbers whose page does not mention it, which
/// no bank window of those numbers can show (MF-UNL-7 step 1).
#[cfg(feature = "board-sachen8259")]
#[test]
fn req_core_090_the_sachen_8259_survives_a_save_and_restore() {
    for mapper in [137usize, 138, 139, 141] {
        let mut core = MapperRom::new(mapper).prg(16).chr(32).load();
        for register in 0u8..8 {
            s8259_write(&mut core, register, (register + 3) & 0x07);
        }
        // Leave the index somewhere that is not the last register written, so that the byte has to
        // be in the block rather than implied by it.
        core.mapper_write(0x4100, 0x02);

        let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
        let pages = nametable_pages(&core);
        let state = core.save_state(true).unwrap();

        core.reset();
        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed, "mapper {mapper}");
        assert_eq!(
            banks,
            (core.prg_windows(), core.chr_windows(), core.mirroring()),
            "mapper {mapper}"
        );
        assert_eq!(pages, nametable_pages(&core), "mapper {mapper}");
        assert_eq!(state, core.save_state(true).unwrap(), "mapper {mapper}");
    }
}

// ------------------------------------------- mapper 112, the 206 with its ports split,
// ------------------------------------------- MF-UNL-7 step 2

/// Mapper 112's idiom: an index at **$8000** and the byte at **$A000**, where the 206 has an
/// even/odd pair at $8000/$8001 (nesdev, "INES Mapper 112").
#[cfg(feature = "board-112")]
fn m112_write(core: &mut NesCore, register: u8, value: u8) {
    core.mapper_write(0x8000, register);
    core.mapper_write(0xA000, value);
}

/// "Quite similar to iNES Mapper 206 … however, the registers appears to be scrambled a bit", and
/// the scramble is a **rotation by six**: `RRR` names the 206's `(RRR + 6) mod 8`, so 0 and 1 are
/// the two PRG windows where the 206 has them last, and 2-7 are the CHR registers where the 206
/// has them first.
///
/// The test is the census's: the scrambled order produces a 206's bank layout, and the
/// unscrambled order does not (MF-UNL-7 step 2).
#[cfg(all(feature = "board-112", feature = "board-namcot108"))]
#[test]
fn req_core_102_mapper_112_rotates_the_206s_register_order_by_six() {
    // `RRR` => the value this test writes through it. Read down the page's own list: two 8 KiB
    // PRG banks, two 2 KiB CHR banks, four 1 KiB CHR banks.
    let values: [(u8, u8); 8] = [
        (0, 5),
        (1, 9),
        (2, 4),
        (3, 6),
        (4, 20),
        (5, 21),
        (6, 22),
        (7, 23),
    ];

    let mut core = MapperRom::new(112).prg(16).chr(32).load();
    // Power-on is the 206's: R6 and R7 at banks 0 and 1, the last two 8 KiB banks fixed to the
    // end, and the whole CHR register file zero.
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    assert_eq!([0, 1, 0, 1, 0, 0, 0, 0], core.chr_windows());

    for (register, value) in values {
        m112_write(&mut core, register, value);
    }
    let scrambled = (core.prg_windows(), core.chr_windows());
    assert_eq!(
        ([5, 9, 14, 15], [4, 5, 6, 7, 20, 21, 22, 23]),
        scrambled,
        "the page's own eight selects, in the windows the page gives them"
    );

    // The same eight bytes through a **206**, each at the register the rotation names. Same
    // silicon below the register file, so the same windows to the slot.
    let mut namco = MapperRom::new(206).prg(16).chr(32).load();
    for (register, value) in values {
        namco.mmc3_bank((register.wrapping_add(6)) & 0x07, value);
    }
    assert_eq!(
        scrambled,
        (namco.prg_windows(), namco.chr_windows()),
        "mapper 112's layout is the 206's"
    );

    // And the unscrambled order is not: the same eight bytes written through mapper 112's ports
    // at the 206's own indices put the PRG banks where the CHR registers belong.
    let mut unscrambled = MapperRom::new(112).prg(16).chr(32).load();
    for (register, value) in values {
        m112_write(&mut unscrambled, (register.wrapping_add(6)) & 0x07, value);
    }
    assert_ne!(
        scrambled,
        (unscrambled.prg_windows(), unscrambled.chr_windows()),
        "the rotation is not a no-op"
    );
    assert_eq!(
        ([4, 6, 14, 15], [20, 21, 20, 21, 22, 23, 5, 9]),
        (unscrambled.prg_windows(), unscrambled.chr_windows()),
        "and this is where the unrotated bytes land"
    );
}

/// The three ports the page names and the five addresses inside its own mask that it does not:
/// select at $8000, data at $A000, nametable at $E000, "Mask: $E001" (MF-UNL-7 step 2).
///
/// The nametable row is worth its own assertion because of how the page writes it —
/// "0: horizontal (A10); 1: vertical (A11)". The parentheses settle it: `horizontal (A10)` is
/// CIRAM A10 driven from PPU A10, which is this core's `Vertical`, so the numbering is the MMC3's
/// own "0: vertical; 1: horizontal" after all.
#[cfg(feature = "board-112")]
#[test]
fn req_core_102_mapper_112_decodes_three_ports_under_mask_e001() {
    let mut core = MapperRom::new(112).prg(16).chr(32).load();

    // The mask reads A15, A14, A13 and A0, so $9FFE is the select port and $BFFE the data one.
    core.mapper_write(0x9FFE, 0x00);
    core.mapper_write(0xBFFE, 0x07);
    assert_eq!([7, 1, 14, 15], core.prg_windows(), "the ports repeat");

    // $8001, $A001 and $E001 are inside the mask and are **not** registers: the page names three
    // addresses and this decode answers three.
    let before = (core.prg_windows(), core.chr_windows());
    core.mapper_write(0x8001, 0x01);
    core.mapper_write(0xA001, 0x0B);
    assert_eq!(
        before,
        (core.prg_windows(), core.chr_windows()),
        "the odd halves are not the register pair"
    );

    // Nor is $C000-$DFFF, which the page does not mention at all. The window there is the 206's
    // "fixed to the end" and nothing can move it (Q-161).
    core.mapper_write(0xC000, 0x00);
    core.mapper_write(0xC001, 0x0B);
    assert_eq!(
        before,
        (core.prg_windows(), core.chr_windows()),
        "$C000-$DFFF is not a port"
    );
    assert_eq!([14, 15], core.prg_windows()[2..], "and the top is fixed");

    // $E000: `xxxx xxxN`.
    core.mapper_write(0xE000, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring(), "0: horizontal (A10)");
    core.mapper_write(0xE000, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring(), "1: vertical (A11)");
    core.mapper_write(0xE000, 0xFE);
    assert_eq!(Nm::Vertical, core.mirroring(), "bit 0 alone");
}

/// Q-161's second half: the page gives the data port a whole byte, `DDDD DDDD`, and uses the
/// MMC3's own wording — "Select 2 KB CHR bank" — for selects 2 and 3, without restating the 206's
/// "only bits 5-1 exist for 2KB CHR banks".
///
/// The 206's rule is taken, because that is the board this one is "quite similar to" and because
/// it is a property of the silicon: the register drives CHR A11 upwards and the window's own A10
/// comes from the PPU. So the two 2 KiB selects ignore bit 0 and the four 1 KiB ones do not
/// (MF-UNL-7 step 2).
#[cfg(feature = "board-112")]
#[test]
fn q_161_mapper_112_ignores_the_low_bit_of_its_two_2_kib_chr_selects() {
    let mut core = MapperRom::new(112).prg(16).chr(32).load();

    // Selects 2 and 3: an odd byte and the even one below it are the same 2 KiB bank.
    for (register, slot) in [(2u8, 0usize), (3, 2)] {
        m112_write(&mut core, register, 0x04);
        let even = core.chr_windows();
        m112_write(&mut core, register, 0x05);
        assert_eq!(even, core.chr_windows(), "register {register}, bit 0");
        assert_eq!(
            [4, 5],
            core.chr_windows()[slot..slot + 2],
            "register {register} shows 1 KiB banks 4 and 5"
        );
    }

    // Selects 4-7 are 1 KiB registers and every bit of them counts.
    m112_write(&mut core, 4, 0x11);
    assert_eq!(17, core.chr_bank(0x1000));
    m112_write(&mut core, 4, 0x10);
    assert_eq!(16, core.chr_bank(0x1000), "bit 0 is a bank bit here");
}

/// REQ-CORE-090/091: the eight registers and the index survive a save and a restore, with the
/// state dirtied first. The block is `Namcot108`'s own and **unchanged** — eight bank bytes and
/// the select — which is what lets this number share the board (MF-UNL-7 step 2).
#[cfg(feature = "board-112")]
#[test]
fn req_core_090_mapper_112_survives_a_save_and_restore() {
    let mut core = MapperRom::new(112).prg(16).chr(32).load();
    for register in 0u8..8 {
        m112_write(&mut core, register, register * 3 + 1);
    }
    core.mapper_write(0xE000, 0x01);
    // An index that is not the last register written, so the byte has to be in the block.
    core.mapper_write(0x8000, 0x03);

    let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
    let state = core.save_state(true).unwrap();

    core.reset();
    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(
        banks,
        (core.prg_windows(), core.chr_windows(), core.mirroring())
    );
    assert_eq!(state, core.save_state(true).unwrap());

    // And the restored index is the one that was saved: the next data write lands at register 3,
    // which the rotation makes the 206's R1 — the 2 KiB CHR window at $0800.
    core.mapper_write(0xA000, 0x0C);
    assert_eq!([12, 13], core.chr_windows()[2..4]);
}

// ------------------------------------------- mapper 193, NTDEC's TC-112, MF-UNL-7 step 3

/// The three CHR windows are **unequal** and each comes from its own register (nesdev, "INES
/// Mapper 193"):
///
/// ```text
///   $0000                           $1000           $1800
/// +-------------------------------+---------------+---------------+
/// |           <<$6000>>           |    <$6001>    |    <$6002>    |
/// +-------------------------------+---------------+---------------+
/// ```
///
/// All three registers hold a **1 KiB** bank number, which is what makes their diagrams differ:
/// `CCCC CC..` loses two bits because its window is 4 KiB and `CCCC CCC.` one because its windows
/// are 2 KiB. So the bank is the byte shifted and not masked, exactly as the 206's own R0 and R1
/// are shifted one place for their 2 KiB windows (MF-UNL-7 step 3).
#[cfg(feature = "board-193")]
#[test]
fn req_core_102_mapper_193_banks_three_unequal_chr_windows() {
    let mut core = MapperRom::new(193).prg(16).chr(64).load();

    // Power-on: every register zero, so 1 KiB bank 0 at the head of all three windows.
    assert_eq!([0, 1, 2, 3, 0, 1, 0, 1], core.chr_windows());

    // $6000 drives the **4 KiB** window at $0000-$0FFF, and its low two bits do not exist.
    core.mapper_write(0x6000, 0x08);
    assert_eq!([8, 9, 10, 11], core.chr_windows()[..4]);
    core.mapper_write(0x6000, 0x0B);
    assert_eq!(
        [8, 9, 10, 11],
        core.chr_windows()[..4],
        "`CCCC CC..`: the low two bits are not bank bits"
    );

    // $6001 drives the **2 KiB** window at $1000-$17FF and $6002 the one at $1800-$1FFF, each
    // losing one bit.
    core.mapper_write(0x6001, 0x10);
    core.mapper_write(0x6002, 0x20);
    assert_eq!([8, 9, 10, 11, 16, 17, 32, 33], core.chr_windows());
    core.mapper_write(0x6001, 0x11);
    core.mapper_write(0x6002, 0x21);
    assert_eq!(
        [8, 9, 10, 11, 16, 17, 32, 33],
        core.chr_windows(),
        "`CCCC CCC.`: the low bit is not a bank bit"
    );

    // And the three windows are genuinely different widths: one register moves 4 KiB of pattern
    // space and the other two 2 KiB each.
    core.mapper_write(0x6000, 0x00);
    assert_eq!([0, 1, 2, 3, 16, 17, 32, 33], core.chr_windows());
}

/// One switchable 8 KiB PRG window of four, the other three "fixed to the last three banks"
/// (nesdev, "INES Mapper 193", its *PRG Setup* row `| $6003 | { -3} | { -2} | { -1} |`), and
/// `[.... PPPP]` is four bits over this image's sixteen banks (MF-UNL-7 step 3).
#[cfg(feature = "board-193")]
#[test]
fn req_core_102_mapper_193_switches_one_prg_window_and_fixes_the_last_three() {
    let mut core = MapperRom::new(193).prg(16).chr(64).load();
    assert_eq!([0, 13, 14, 15], core.prg_windows());

    core.mapper_write(0x6003, 0x05);
    assert_eq!([5, 13, 14, 15], core.prg_windows());
    core.mapper_write(0x6003, 0x0F);
    assert_eq!([15, 13, 14, 15], core.prg_windows());

    // Four bits, so bits 7-4 of the byte are not bank bits.
    core.mapper_write(0x6003, 0xF5);
    assert_eq!([5, 13, 14, 15], core.prg_windows());
}

/// "$6004: [.... ...M] Mirroring", with "Range,Mask: $6000-7FFF, $E007" — which reads A15, A14,
/// A13 and A2-A0, so the five registers repeat every eight bytes through the whole of $6000-$7FFF
/// and `registry::board_memory_sizes` has to give the number no PRG-RAM.
///
/// The page's "No games are known to rely on changing mirroring at run-time. At least some games
/// have hard-wired mirroring" is a note about the **catalogue** and not about the register, so the
/// register is implemented and this is the test the census asks for by name (Q-161, MF-UNL-7
/// step 3).
#[cfg(feature = "board-193")]
#[test]
fn q_161_mapper_193_changes_mirroring_from_6004() {
    let mut core = MapperRom::new(193).prg(16).chr(64).load();

    core.mapper_write(0x6004, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.mapper_write(0x6004, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());
    core.mapper_write(0x6004, 0xFE);
    assert_eq!(Nm::Vertical, core.mirroring(), "bit 0 alone");

    // The mask: $7FFC is $6004 and $7FFB is $6003, so the five repeat to the top of the window.
    core.mapper_write(0x7FFC, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring(), "the ports repeat");
    core.mapper_write(0x7FFB, 0x07);
    assert_eq!([7, 13, 14, 15], core.prg_windows());

    // $6005-$6007 are inside the window and are **not** registers.
    let before = (core.prg_windows(), core.chr_windows(), core.mirroring());
    for address in [0x6005u16, 0x6006, 0x6007] {
        core.mapper_write(address, 0x00);
    }
    assert_eq!(
        before,
        (core.prg_windows(), core.chr_windows(), core.mirroring()),
        "$6005-$6007 reach nothing"
    );

    // And no address above $8000 can be a register under this mask: A15 is clear in it.
    for address in [0x8000u16, 0xE000, 0xE004, 0xFFFC] {
        core.mapper_write(address, 0x00);
    }
    assert_eq!(
        before,
        (core.prg_windows(), core.chr_windows(), core.mirroring()),
        "the board has no port above $8000"
    );
}

/// REQ-CORE-090/091: the four bank registers survive a save and a restore, with the state dirtied
/// first. The block is `Namcot108`'s own and **unchanged**, which is what lets this number share
/// the board (MF-UNL-7 step 3).
#[cfg(feature = "board-193")]
#[test]
fn req_core_090_mapper_193_survives_a_save_and_restore() {
    let mut core = MapperRom::new(193).prg(16).chr(64).load();
    core.mapper_write(0x6000, 0x14);
    core.mapper_write(0x6001, 0x18);
    core.mapper_write(0x6002, 0x1C);
    core.mapper_write(0x6003, 0x09);
    core.mapper_write(0x6004, 0x01);

    let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
    assert_eq!(
        (
            [9, 13, 14, 15],
            [20, 21, 22, 23, 24, 25, 28, 29],
            Nm::Horizontal
        ),
        banks
    );
    let state = core.save_state(true).unwrap();

    core.reset();
    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(
        banks,
        (core.prg_windows(), core.chr_windows(), core.mirroring())
    );
    assert_eq!(state, core.save_state(true).unwrap());
}

// ------------------------------------------- NES 2.0 mapper 298, MF-UNL-7 step 4

/// "The board uses a VRC4 clone in the `VRC4b` configuration (CPU 0x02s to VRC4 A0, CPU 0x01s to
/// VRC4 A1)" (nesdev, "NES 2.0 Mapper 298"), and that sentence is the whole of the board's
/// register documentation — so the test is that the number *is* a `VRC4b`, bank for bank, against
/// the `VRC4b` this build already had: mapper 25 submapper 1.
///
/// The wiring is what the assertions turn on. On a `VRC4b`, bit 0 of the register index comes from
/// **A1** and bit 1 from **A0**, so $B002 is the *high* nibble of CHR register 0 where a `VRC4f`
/// would put the high nibble at $B001 (MF-UNL-7 step 4).
#[cfg(feature = "board-298")]
#[test]
fn req_core_102_mapper_298_is_a_vrc4b_bank_for_bank() {
    let banks = |mapper: usize| {
        let mut core = if mapper == 25 {
            MapperRom::new(25).submapper(1).prg(16).chr(32).load()
        } else {
            MapperRom::new(298).prg(16).chr(32).load()
        };

        // $8000 and $A000 are the two 8 KiB PRG registers on every variant.
        core.write_memory(0x8000, 0x05);
        core.write_memory(0xA000, 0x09);

        // CHR register 0: the low nibble at $B000 and the high one at **$B002**, which is the
        // VRC4b's A0/A1 swap and not a VRC4f's.
        core.write_memory(0xB000, 0x04);
        let low_only = core.chr_bank(0x0000);
        core.write_memory(0xB002, 0x01);

        // And the mirroring register, $9000 with the index below 2.
        core.write_memory(0x9000, 0x01);

        (
            core.prg_windows(),
            low_only,
            core.chr_bank(0x0000),
            core.mirroring(),
        )
    };

    assert_eq!(
        ([5, 9, 14, 15], 4, 20, Nm::Horizontal),
        banks(25),
        "the VRC4b this build already had"
    );
    assert_eq!(banks(25), banks(298), "and mapper 298 is the same board");
}

/// REQ-CORE-090/091: mapper 298 round-trips through `Vrc2Vrc4`'s **own, unchanged** savestate
/// block — the eight CHR registers, the two PRG bytes, the swap flag and `VrcIrq`'s seven fields,
/// and **no byte of its own**, because the one thing that makes this number different comes from
/// the mapper number at load (MF-UNL-7 step 4).
#[cfg(feature = "board-298")]
#[test]
fn req_core_090_mapper_298_keeps_the_vrc4s_save_block() {
    let dirty = |mapper: usize| {
        let mut core = if mapper == 25 {
            MapperRom::new(25).submapper(1).prg(16).chr(32).load()
        } else {
            MapperRom::new(298).prg(16).chr(32).load()
        };
        core.write_memory(0x8000, 0x05);
        core.write_memory(0xA000, 0x09);
        core.write_memory(0x9001, 0x02); // Swap mode.
        for slot in 0u16..8 {
            let group = 0xB000 + (slot >> 1) * 0x1000;
            // Bit 0 of the register index is CPU A1 on a `VRC4b`, so the low nibble of a CHR
            // register is at $xy00 or $xy01 and its high nibble two bytes above.
            let half = slot & 1;
            core.write_memory(group + half, u8::try_from(slot).unwrap() + 1);
            core.write_memory(group + half + 0x0002, 0x01);
        }
        core.write_memory(0xF000, 0x0A);
        core.write_memory(0xF002, 0x0C);
        core.write_memory(0xF001, 0x06);
        core
    };

    // The same banks the VRC4b reaches from the same writes, which is what says the block being
    // round-tripped is that board's and not a new one.
    assert_eq!(
        {
            let other = dirty(25);
            (other.prg_windows(), other.chr_windows(), other.mirroring())
        },
        {
            let mine = dirty(298);
            (mine.prg_windows(), mine.chr_windows(), mine.mirroring())
        }
    );

    let mut core = dirty(298);
    let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
    let state = core.save_state(true).unwrap();

    core.reset();
    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(
        banks,
        (core.prg_windows(), core.chr_windows(), core.mirroring())
    );
    assert_eq!(state, core.save_state(true).unwrap());
}

// ------------------------------------------- mapper 218, Magic Floor, MF-UNL-7 step 5

/// Loads a mapper-218 image in one of its four configurations. `byte6` is the header's own,
/// which is how the board is wired: bit 3 picks the pair of lines and bit 0 picks within it.
///
/// The image declares **no CHR** — "There is no CHR-ROM or CHR-RAM on the cartridge" — so the
/// 2 KiB the board gets is `registry::board_memory_sizes`'s and not a header field's.
#[cfg(feature = "board-218")]
fn magic_floor(byte6: u8) -> NesCore {
    MapperRom::new(218).prg(4).chr(0).mirroring(byte6).load()
}

/// "There is no CHR-ROM or CHR-RAM on the cartridge. Instead, the console's internal 2KiB
/// nametable RAM is mapped as CHR-RAM" (nesdev, "INES Mapper 218").
///
/// So a pattern-table fetch returns CIRAM, and **the nametable window and the pattern tables are
/// the same 2 KiB**: a byte written through $2007 at $2000 comes back out of the pattern table the
/// configuration puts it under. That is what the board exists for — *Magic Floor* draws its tiles
/// through the nametable window — and it is the half of this board a test can state without naming
/// a register, because **there is no CHR bank register to test**, which is the point
/// (MF-UNL-7 step 5).
#[cfg(feature = "board-218")]
#[test]
fn req_core_102_mapper_218_serves_its_pattern_tables_from_ciram() {
    // CIRAM A10 from PPU A10: the two-screen configuration with vertical mirroring.
    let mut core = magic_floor(0x01);

    // Nothing banked, nothing readable yet: 2 KiB of RAM, zero-filled.
    assert_eq!([0, 0, 0, 0, 0, 0, 0, 0], core.chr_windows());

    // A write into the **nametable** window comes back out of the **pattern** window, because
    // they are one memory. $2000 is block 0 offset 0; $2400 is block 1 offset 0 under this wiring.
    core.cartridge_mut().unwrap().write_nametable(0x2000, 0x5A);
    core.cartridge_mut().unwrap().write_nametable(0x2400, 0xA5);
    assert_eq!(0x5A, core.chr_bank(0x0000), "block 0 under the patterns");
    assert_eq!(0xA5, core.chr_bank(0x0400), "block 1 beside it");

    // And the other way round: a pattern-table write is visible in the nametables.
    chr_write(&mut core, 0x0010, 0x3C);
    assert_eq!(
        0x3C,
        core.cartridge().unwrap().read_nametable(0x2010),
        "one memory, both windows"
    );

    // The board serves all four tables itself, which is what lets one memory answer both.
    assert_eq!(
        [
            Cartridge::SERVED_BY_CARTRIDGE,
            Cartridge::SERVED_BY_CARTRIDGE,
            Cartridge::SERVED_BY_CARTRIDGE,
            Cartridge::SERVED_BY_CARTRIDGE
        ],
        nametable_pages(&core)
    );

    // "A single PRG-ROM chip (up to 32KiB)", unbanked — and a 32 KiB image shows its two 16 KiB
    // halves in order. There is no register, so nothing a game writes can move it.
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    for address in [0x6000u16, 0x7FFF, 0x8000, 0xA000, 0xC000, 0xE000, 0xFFFF] {
        core.mapper_write(address, 0xFF);
    }
    assert_eq!(
        (
            [0, 1, 2, 3],
            [0x5A, 0xA5, 0x5A, 0xA5, 0x5A, 0xA5, 0x5A, 0xA5]
        ),
        (core.prg_windows(), core.chr_windows()),
        "there is no register on this board at all"
    );
}

/// The four configurations, which the board "can be used in … by wiring CIRAM A10 to one of the
/// PPU A10..A13 address lines" and which the header's byte 6 declares (nesdev, "INES Mapper 218"):
///
/// | byte 6 | line | the page's name |
/// | --- | --- | --- |
/// | `$A1` | PPU A10 | Two-screen, Horizontal arrangement ("Vertical mirroring") |
/// | `$A0` | PPU A11 | Two-screen, Vertical arrangement ("Horizontal mirroring") |
/// | `$A8` | PPU A12 | Single-screen arrangement BLK0 |
/// | `$A9` | PPU A13 | Single-screen arrangement BLK1 |
///
/// The test writes one distinguishable byte into each of the two 1 KiB blocks and then reads the
/// four nametables and the two pattern tables, so each configuration shows up as a pattern of
/// blocks rather than as a claim about it (MF-UNL-7 step 5).
#[cfg(feature = "board-218")]
#[test]
fn req_core_102_mapper_218_wires_ciram_a10_to_one_of_four_ppu_lines() {
    // Byte 6, the PPU line it names, the four nametables and the eight 1 KiB pattern windows.
    // The marker bytes are $B0 in block 0 of the 2 KiB and $B1 in block 1.
    for (byte6, line, tables, patterns) in [
        // Two-screen: the nametables alternate across the line that drives CIRAM A10, and **both
        // pattern tables show the whole 2 KiB** -- the page's "2KiB (128 tiles) CHR-RAM, also
        // mapped to nametable data".
        (
            0x01u8,
            10u32,
            [0xB0u8, 0xB1, 0xB0, 0xB1],
            [0xB0u8, 0xB1, 0xB0, 0xB1, 0xB0, 0xB1, 0xB0, 0xB1],
        ),
        (
            0x00,
            11,
            [0xB0, 0xB0, 0xB1, 0xB1],
            [0xB0, 0xB0, 0xB1, 0xB1, 0xB0, 0xB0, 0xB1, 0xB1],
        ),
        // Single-screen: PPU A12 is 0 throughout $2000-$2FFF and PPU A13 is 1 throughout it, so
        // the nametables are all block 0 or all block 1 -- and each pattern table shows **1 KiB**,
        // the page's "1KiB (64 tiles)". On the A12 wiring the block the background shows is
        // "swappable via PPUCTRL", because that register's pattern-table select *is* A12; on the
        // A13 wiring the patterns never see block 1 at all.
        (
            0x08,
            12,
            [0xB0, 0xB0, 0xB0, 0xB0],
            [0xB0, 0xB0, 0xB0, 0xB0, 0xB1, 0xB1, 0xB1, 0xB1],
        ),
        (
            0x09,
            13,
            [0xB1, 0xB1, 0xB1, 0xB1],
            [0xB0, 0xB0, 0xB0, 0xB0, 0xB0, 0xB0, 0xB0, 0xB0],
        ),
    ] {
        let mut core = magic_floor(byte6);

        // Block 0 through the pattern window, then block 1 through the nametable window: on each
        // wiring, `$2000 | (1 << line)` is an address whose CIRAM A10 is high.
        chr_write(&mut core, 0x0000, 0xB0);
        core.cartridge_mut()
            .unwrap()
            .write_nametable(0x2000 | (1 << line), 0xB1);

        let nametables: [u8; 4] = std::array::from_fn(|table| {
            let address = 0x2000 | u16::try_from(table).unwrap() << 10;
            core.cartridge().unwrap().read_nametable(address)
        });
        assert_eq!(tables, nametables, "byte 6 ${byte6:02X}, PPU A{line}");
        assert_eq!(
            patterns,
            core.chr_windows(),
            "byte 6 ${byte6:02X}, PPU A{line}: the eight 1 KiB pattern windows"
        );

        // The page's two capacities, as a count rather than a claim: a two-screen wiring shows
        // both blocks inside one pattern table and a single-screen one shows a single block.
        let blocks_in_one_table =
            usize::from(patterns[..4].contains(&0xB0)) + usize::from(patterns[..4].contains(&0xB1));
        assert_eq!(
            usize::from(line == 10 || line == 11) + 1,
            blocks_in_one_table,
            "byte 6 ${byte6:02X}: 2 KiB on a two-screen wiring, 1 KiB on a single-screen one"
        );
    }
}

/// The arrangement the cartridge reports, which is **not** what the rendering reads — all four
/// tables are `SERVED_BY_CARTRIDGE` — but is the honest answer for anything that asks, and
/// survives a soft reset (AMD-061 §5, MF-UNL-7 step 5).
#[cfg(feature = "board-218")]
#[test]
fn amd_061_mapper_218_reports_the_arrangement_its_wiring_gives() {
    for (byte6, arrangement) in [
        (0x01u8, Nm::Vertical),
        (0x00, Nm::Horizontal),
        (0x08, Nm::SingleScreenA),
        (0x09, Nm::SingleScreenB),
    ] {
        let mut core = magic_floor(byte6);
        assert_eq!(arrangement, core.mirroring(), "byte 6 ${byte6:02X}");
        core.reset();
        assert_eq!(
            arrangement,
            core.mirroring(),
            "byte 6 ${byte6:02X} after a reset"
        );
    }
}

/// REQ-CORE-090/091: the board has **no register**, so what a savestate carries for it is the
/// 2 KiB itself — which `Cartridge` writes as the CHR-RAM blob — and nothing else. The round trip
/// is still worth pinning, because that blob is the nametables as well on this board
/// (MF-UNL-7 step 5).
#[cfg(feature = "board-218")]
#[test]
fn req_core_090_mapper_218_round_trips_its_two_kib_of_ciram() {
    let mut core = magic_floor(0x01);
    chr_write(&mut core, 0x0000, 0x11);
    core.cartridge_mut().unwrap().write_nametable(0x2400, 0x22);

    let state = core.save_state(true).unwrap();
    chr_write(&mut core, 0x0000, 0x99);
    assert_eq!(0x99, core.chr_bank(0x0000));

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(0x11, core.chr_bank(0x0000), "the pattern side");
    assert_eq!(
        0x22,
        core.cartridge().unwrap().read_nametable(0x2400),
        "and the nametable side of the same 2 KiB"
    );
    assert_eq!(state, core.save_state(true).unwrap());
}

// ------------------------------------------- mapper 29, the RET-CUFROM, MF-UNL-7 step 6

/// The Sealie Computing RET-CUFROM revD, "allocated on 15-nov-2013 to implement some homebrew
/// games, including *Glider*" (nesdev, "INES Mapper 029"):
///
/// ```text
/// Range,Mask:   $8000-FFFF, $8000
///
/// $8000:  [...P PPCC]
/// ```
///
/// `CC` is the 8 KiB CHR-RAM bank over "4x banks worth of full 8KB CHR ram" and `PPP` the 16 KiB
/// PRG bank over "8x 16KB PRG banks", with $C000-$FFFF fixed to the last. Four CHR banks, eight
/// PRG banks, and writes to CHR-RAM visible on read-back — which is the census's own test list for
/// this number (MF-UNL-7 step 6).
#[cfg(feature = "board-29")]
#[test]
fn req_core_102_mapper_29_banks_four_chr_ram_banks_and_eight_prg_banks() {
    // Eight 16 KiB PRG banks, and no CHR-ROM: the 32 KiB of CHR-RAM is the board's own, from
    // `registry::board_memory_sizes`.
    let mut core = MapperRom::new(29).prg(16).chr(0).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    assert_eq!(Nm::Vertical, core.mirroring(), "hard-wired for vertical");

    // `PPP`, bits 4-2: eight 16 KiB banks, the last two 8 KiB windows fixed to the end.
    for bank in 0u8..8 {
        core.mapper_write(0x8000, bank << 2);
        assert_eq!(
            [bank * 2, bank * 2 + 1, 14, 15],
            core.prg_windows(),
            "PRG bank {bank}"
        );
    }

    // `CC`, bits 1-0: four 8 KiB CHR-RAM banks. Each is written through the PPU and read back,
    // which is the only way to tell four banks of RAM apart.
    for bank in 0u8..4 {
        core.mapper_write(0x8000, bank);
        chr_write(&mut core, 0x0000, 0xC0 | bank);
        chr_write(&mut core, 0x1FFF, 0xD0 | bank);
    }
    for bank in 0u8..4 {
        core.mapper_write(0x8000, bank);
        assert_eq!(0xC0 | bank, core.chr_bank(0x0000), "CHR-RAM bank {bank}");
        assert_eq!(
            0xD0 | bank,
            core.cartridge()
                .unwrap()
                .ppu_peek(0x1FFF, ChrFetch::Background),
            "the top of CHR-RAM bank {bank}"
        );
    }

    // The two fields are independent, which the `...` of the diagram's top three bits is: a
    // register of $1F is PRG bank 7 and CHR bank 3 at once.
    core.mapper_write(0x8000, 0x1F);
    assert_eq!([14, 15, 14, 15], core.prg_windows());
    assert_eq!(0xC3, core.chr_bank(0x0000));
    core.mapper_write(0x8000, 0xFF);
    assert_eq!(
        ([14, 15, 14, 15], 0xC3),
        (core.prg_windows(), core.chr_bank(0x0000)),
        "bits 7-5 are not register bits"
    );
}

/// "Range,Mask: $8000-FFFF, $8000" — A15 alone, so **every** address in $8000-$FFFF is the
/// register — and "8KB of WRAM mounted in the usual place", which is $6000-$7FFF and is therefore
/// not the register (MF-UNL-7 step 6).
#[cfg(feature = "board-29")]
#[test]
fn req_core_102_the_ret_cufrom_register_is_the_whole_of_8000_ffff() {
    let mut core = MapperRom::new(29).prg(16).chr(0).load();

    for address in [0x8000u16, 0x9ABC, 0xC000, 0xE001, 0xFFFF] {
        core.mapper_write(address, 0x08);
        assert_eq!([4, 5, 14, 15], core.prg_windows(), "${address:04X}");
        core.mapper_write(address, 0x00);
        assert_eq!([0, 1, 14, 15], core.prg_windows(), "${address:04X}");
    }

    // $6000-$7FFF is the WRAM the page gives the board, and a write there is RAM and not a
    // register: the bank stays where it was and the byte comes back out.
    core.mapper_write(0x8000, 0x0C);
    core.write_memory(0x6789, 0x42);
    assert_eq!(
        [6, 7, 14, 15],
        core.prg_windows(),
        "the register is untouched"
    );
    assert_eq!(0x42, core.read_memory(0x6789), "and the WRAM answers");

    // And below it nothing answers at all: the board has no port under $6000.
    core.mapper_write(0x4100, 0x00);
    core.mapper_write(0x5000, 0x00);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
}

/// REQ-CORE-090/091: the latch and the 32 KiB of CHR-RAM survive a save and a restore, with the
/// state dirtied first. The savestate block is `SingleLatch`'s own and unchanged — one register
/// byte and the four bytes mapper 190 uses — which is what an eighth board of that variant costs
/// the other seven (MF-UNL-7 step 6).
#[cfg(feature = "board-29")]
#[test]
fn req_core_090_mapper_29_survives_a_save_and_restore() {
    let mut core = MapperRom::new(29).prg(16).chr(0).load();
    core.mapper_write(0x8000, 0x16); // PRG bank 5, CHR bank 2.
    chr_write(&mut core, 0x0400, 0x77);
    core.write_memory(0x6000, 0x88);

    // The CHR window reads are the **bytes of CHR-RAM** and not bank numbers on this board, so
    // the $77 written at $0400 is the second of the eight.
    let banks = (core.prg_windows(), core.chr_windows(), core.mirroring());
    assert_eq!(
        ([10, 11, 14, 15], [0, 0x77, 0, 0, 0, 0, 0, 0], Nm::Vertical),
        banks
    );
    let state = core.save_state(true).unwrap();

    core.reset();
    assert_eq!(
        [0, 1, 14, 15],
        core.prg_windows(),
        "a reset clears the latch"
    );

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(
        banks,
        (core.prg_windows(), core.chr_windows(), core.mirroring())
    );
    assert_eq!(0x77, core.chr_bank(0x0400), "the CHR-RAM byte");
    assert_eq!(0x88, core.read_memory(0x6000), "the WRAM byte");
    assert_eq!(state, core.save_state(true).unwrap());
}

// ------------------------------------ the Cony / Yoko ASIC (83, 264), MF-UNL-8 step 3

/// Mapper 264's address rewiring, as the page's own five-row table: it "connects CPU A11.A10 to
/// the ASIC's A9..A8 inputs, so all addresses change accordingly".
///
/// | register | mapper 83 | mapper 264 |
/// | --- | --- | --- |
/// | the mode register | `$8100` | `$8400` |
/// | the IRQ counter | `$8200` | `$8800` |
/// | the 8 KiB PRG banks | `$8300` | `$8C00` |
/// | the IRQ clock source | `$8318` | `$8C18` |
/// | the scratch RAM | `$5100` | `$5400` |
#[cfg(feature = "board-cony")]
fn cony_register(mapper: usize, address: u16) -> u16 {
    if mapper == 264 {
        // The inverse of the board's own `asic_address`: ASIC A9-A8 come back out on CPU A11-A10.
        (address & 0xF0FF) | ((address & 0x0300) << 2)
    } else {
        address
    }
}

#[cfg(feature = "board-cony")]
fn cony_write(core: &mut NesCore, mapper: usize, address: u16, value: u8) {
    core.mapper_write(cony_register(mapper, address), value);
}

/// The PRG base at `$8000` (`PPPP PPPP`, "PRG A21..A14") and the four banking modes of `$8100`
/// bits 4-3, in the page's own words:
///
/// * **0**: "PRG A17/A16..A14=1 if CPU A14=1 (`UxROM`)";
/// * **1**: "None (NROM-128)";
/// * **2**: "PRG A17/A16..A13 from 4x8 KiB PRG registers";
/// * **3**: "Same as 2".
///
/// With "the $E000-$FFFF window ... hard-wired to the last (inner) bank" in modes 2 and 3, and the
/// inner bank 256 KiB on this PCB.
#[cfg(feature = "board-cony")]
#[test]
fn req_core_102_cony_banks_prg_in_the_four_modes_of_its_mode_register() {
    let mut core = MapperRom::new(83).prg(32).chr(64).load();
    // Power-on is mode 0 with a base of zero: UxROM, so the last 16 KiB of the inner bank is
    // fixed at $C000 and the reset vector is reachable out of a cold start.
    assert_eq!([0, 1, 30, 31], core.prg_windows(), "mode 0, base 0");

    // The base is a 16 KiB unit: $04 is PRG A14-A21 = 4, which is 8 KiB bank 8.
    core.mapper_write(0x8000, 0x04);
    assert_eq!(
        [8, 9, 30, 31],
        core.prg_windows(),
        "mode 0 switches the low 16 KiB and fixes the high one"
    );

    // Mode 1, NROM-128: CPU A14 reaches no PRG line, so the one 16 KiB bank appears twice.
    core.mapper_write(0x8100, 0x08);
    assert_eq!([8, 9, 8, 9], core.prg_windows(), "mode 1 is NROM-128");

    // Modes 2 and 3: the four 8 KiB registers, with $E000 hard-wired to the last inner bank.
    for mode in [0x10u8, 0x18] {
        core.mapper_write(0x8100, mode);
        core.mapper_write(0x8300, 3);
        core.mapper_write(0x8301, 5);
        core.mapper_write(0x8302, 7);
        assert_eq!(
            [3, 5, 7, 31],
            core.prg_windows(),
            "mode {mode:#04X}: three registers and the last inner bank"
        );
    }

    // The register is five bits, `...p PPPP`, so bits 7-5 of a write are not latched.
    core.mapper_write(0x8300, 0xE3);
    assert_eq!(3, core.prg_windows()[0], "`...p PPPP` is five bits");

    // $8300-$830F is the register file under the page's own mask, `(Address AND $8310)=$8300`, so
    // A3 is undecoded and $8308 is $8300 again.
    core.mapper_write(0x8308, 9);
    assert_eq!(9, core.prg_windows()[0], "A3 is not in the $8310 mask");
}

/// `$8310`-`$8317`, "CHR-ROM A17..A10" per 1 KiB window on mapper 83.0 -- and the rewiring that
/// makes 83.1 and 264 bank in **2 KiB** amounts instead.
///
/// > Mappers 83.1 and 264 increase the CHR-ROM size from 256 to 512 KiB by connecting CHR-ROM A10
/// > to PPU A10 and CHR-ROM A11..A18 to the mapper ASIC's A10..A17 outputs ... As a result,
/// > CHR-ROM is banked in 2 KiB rather than 1 KiB amounts, using on the the four CHR bank
/// > registers 0, 1, 6 and 7.
///
/// The page's own address table is what picks those four: the ASIC sees `$0000`/`$0400` as reg 0,
/// `$0800`/`$0C00` as reg 1, `$1000`/`$1400` as **reg 6** and `$1800`/`$1C00` as **reg 7** -- so
/// registers 2 to 5 drive nothing at all on those two PCBs, which is the half of this a reader
/// would get wrong.
#[cfg(feature = "board-cony")]
#[test]
fn req_core_102_cony_banks_chr_in_1_kib_on_83_0_and_in_2_kib_on_83_1_and_264() {
    // 83.0: eight 1 KiB windows from eight registers.
    let mut core = MapperRom::new(83).prg(16).chr(64).load();
    for register in 0..8u16 {
        core.mapper_write(0x8310 + register, u8::try_from(register).unwrap() + 1);
    }
    assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], core.chr_windows());

    // 83.1 and 264: four 2 KiB windows from registers 0, 1, 6 and 7.
    for mapper in [83usize, 264] {
        let mut core = if mapper == 83 {
            MapperRom::new(83).submapper(1).prg(16).chr(256).load()
        } else {
            MapperRom::new(264).prg(16).chr(256).load()
        };

        for (register, bank) in [(0u16, 1u8), (1, 2), (6, 3), (7, 4)] {
            cony_write(&mut core, mapper, 0x8310 + register, bank);
        }
        assert_eq!(
            [2, 3, 4, 5, 6, 7, 8, 9],
            core.chr_windows(),
            "mapper {mapper}: four 2 KiB banks"
        );

        // And the four registers the rewiring disconnects drive nothing.
        for register in 2..6u16 {
            cony_write(&mut core, mapper, 0x8310 + register, 0x7F);
        }
        assert_eq!(
            [2, 3, 4, 5, 6, 7, 8, 9],
            core.chr_windows(),
            "mapper {mapper}: registers 2-5 are not connected"
        );
    }
}

/// The rows of the page's mapper-264 table, each tested by writing the **mapper-83** address on a
/// mapper-264 image and finding that it did something else -- which is the only way to show a
/// rewiring rather than assert it.
#[cfg(feature = "board-cony")]
#[test]
fn req_core_102_mapper_264_moves_every_register_by_the_a11_a10_rewiring() {
    // $8400 is the mode register and $8C00 the first 8 KiB PRG register.
    let mut core = MapperRom::new(264).prg(32).chr(256).load();
    core.mapper_write(0x8400, 0x10); // Mode 2.
    core.mapper_write(0x8C00, 3);
    assert_eq!(
        [3, 0, 0, 15],
        core.prg_windows(),
        "$8400 and $8C00 are the ASIC's, with a 128 KiB inner bank"
    );

    // So $8100 is the PRG **base** on this PCB (ASIC $8000) and not the mode register. The inner
    // bank is 128 KiB here, so only the base's A17 and above show: $08 is PRG A14-A21 = 8, which
    // is 8 KiB bank 16, and the four registers drive A13-A16 inside it.
    core.mapper_write(0x8100, 0x08);
    assert_eq!(
        [19, 16, 16, 31],
        core.prg_windows(),
        "$8100 is the PRG base on this PCB, not the mode register"
    );

    // $8C18 is the IRQ clock source and $8318 is not, so writing $FF at $8318 must leave the
    // counter on CPU M2.
    let mut core = MapperRom::new(264).prg(16).chr(256).load();
    core.mapper_write(0x8400, 0x80); // Arm the counter, counting up.
    core.mapper_write(0x8318, 0xFF); // Not the source register here.
    core.mapper_write(0x8800, 0xFE);
    core.mapper_write(0x8801, 0xFF);
    for _ in 0..2 {
        core.bus_mut().tick();
    }
    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "$8318 is not the source register on mapper 264, so M2 still counts"
    );

    // And $5400-$5403 is the scratch RAM where mapper 83 has $5100-$5103.
    let mut core = MapperRom::new(264).prg(16).chr(256).load();
    core.mapper_write(0x5400, 0x5A);
    assert_eq!(Some(0x5A), core.cartridge().unwrap().try_cpu_peek(0x5400));
    core.mapper_write(0x5100, 0xA5);
    assert_eq!(
        Some(0x5A),
        core.cartridge().unwrap().try_cpu_peek(0x5400),
        "$5100 is not the scratch RAM on mapper 264"
    );
}

/// `$8100` bit 5, "1: Map 8 KiB PRG-ROM bank to CPU $6000", with "register $8303 appl[ying] to the
/// $6000-$7FFF window, if enabled in the Mode register".
///
/// The window is PRG-**ROM** on all four PCBs this build serves -- the page's *Banks* section gives
/// it "into 32 KiB PRG-NVRAM (Mapper 83.2)/into PRG-ROM (all others)" and 83.2 is refused -- so
/// `board_memory_sizes` gives both numbers no PRG-RAM and the board answers the window itself.
/// With bit 5 clear there is nothing there at all, which `try_cpu_peek` shows as `None` rather than
/// as a zero the board drove.
#[cfg(feature = "board-cony")]
#[test]
fn req_core_102_cony_maps_a_prg_rom_bank_at_6000_when_the_mode_register_says_so() {
    let mut core = MapperRom::new(83).prg(32).chr(64).load();
    assert_eq!(
        None,
        core.cartridge().unwrap().try_cpu_peek(0x6000),
        "power-on: no window and no RAM at $6000"
    );

    core.mapper_write(0x8100, 0x10 | 0x20); // Mode 2, $6000 enabled.
    core.mapper_write(0x8303, 9);
    assert_eq!(
        Some(9),
        core.cartridge().unwrap().try_cpu_peek(0x6000),
        "$8303 is the $6000-$7FFF window"
    );
    assert_eq!(
        Some(9),
        core.cartridge().unwrap().try_cpu_peek(0x7FFF),
        "and the whole 8 KiB of it"
    );

    // The other three registers are the $8000/$A000/$C000 windows and not this one.
    core.mapper_write(0x8300, 3);
    core.mapper_write(0x8301, 5);
    core.mapper_write(0x8302, 7);
    assert_eq!([3, 5, 7, 31], core.prg_windows());
    assert_eq!(Some(9), core.cartridge().unwrap().try_cpu_peek(0x6000));

    // Bit 5 clear takes the window away again; a write there is not RAM either.
    core.mapper_write(0x8100, 0x10);
    core.write_memory(0x6000, 0x5A);
    assert_eq!(None, core.cartridge().unwrap().try_cpu_peek(0x6000));
}

/// `$8100` bits 1-0, `NN`, the nametable arrangement -- and the page gives both namings, which are
/// each other's opposites: "0: Horizontal (**Vertical mirroring**)", "1: Vertical (**Horizontal
/// mirroring**)", "2: Single screen 0", "3: Single screen 1". This core's `NametableMirroring` is
/// the one in the page's own parentheses.
#[cfg(feature = "board-cony")]
#[test]
fn req_core_102_cony_sets_its_mirroring_from_the_mode_register() {
    let mut core = MapperRom::new(83).prg(16).chr(64).load();
    for (bits, expected) in [
        (0x00u8, Nm::Vertical),
        (0x01, Nm::Horizontal),
        (0x02, Nm::SingleScreenA),
        (0x03, Nm::SingleScreenB),
    ] {
        core.mapper_write(0x8100, bits);
        assert_eq!(expected, core.mirroring(), "NN = {bits}");
    }

    // The mode register is one byte: the arrangement rides along with the banking mode and the
    // other four bits, so a write that sets mode 2 also sets the arrangement.
    core.mapper_write(0x8100, 0x10 | 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
}

/// Submapper 3, the **N-C44/82-08-24J** 4-in-1: a PRG base of `CCPP PPPP` where the other PCBs
/// have `PPPP PPPP`, so "PRG A19..A14" in the low six bits and "CHR-ROM A19..A18" in the top two --
/// and a **128 KiB** inner PRG bank, because "PRG Base A17 is only modified on mappers 83.0, 83.1
/// and 83.2, and not on 83.3 and 264".
///
/// The inner bank's width is the whole of what that sentence changes, and it is visible in mode 2:
/// with a 256 KiB inner bank the base's low five bits are inside it and contribute nothing, where
/// with a 128 KiB one the base's A17 sits above the register's four bits and selects the half.
#[cfg(feature = "board-cony")]
#[test]
fn req_core_102_mapper_83_submapper_3_has_a_128_kib_inner_prg_bank_and_an_outer_chr_bank() {
    let mut core = MapperRom::new(83).submapper(3).prg(32).chr(512).load();
    core.mapper_write(0x8100, 0x10); // Mode 2.
    core.mapper_write(0x8000, 0x08); // `CCPP PPPP`: PRG A14-A19 = 8, so 8 KiB bank 16.
    core.mapper_write(0x8300, 3);
    core.mapper_write(0x8301, 5);
    core.mapper_write(0x8302, 7);
    assert_eq!(
        [19, 21, 23, 31],
        core.prg_windows(),
        "the base's A17 is above the register's A13-A16, and $E000 is the last inner bank"
    );

    // The `p` bit of `...p PPPP` is PRG A17, and on this PCB it "has no effect".
    core.mapper_write(0x8300, 0x10 | 3);
    assert_eq!(19, core.prg_windows()[0], "the p bit has no effect on 83.3");

    // And the top two bits of the base are CHR A19-A18, an outer 256 KiB CHR bank.
    let a18 = 256 * 0x400;
    core.mapper_write(0x8310, 1);
    assert_eq!(0x400, chr_offsets(&core)[0], "no outer bank yet");
    core.mapper_write(0x8000, 0x40 | 0x08);
    assert_eq!(0x400 + a18, chr_offsets(&core)[0], "`CC` = 1 is CHR A18");
    assert_eq!(
        [19, 21, 23, 31],
        core.prg_windows(),
        "and the PRG half of the same register is unchanged"
    );
}

/// The solder pad at `$5000` (`(Address AND $F100)=$5000`) and the four bytes of scratch RAM at
/// `$5100`-`$5103`, which is the test census §7.2 asks for by name.
///
/// The pad answers **0** and that is a recorded gap (Q-166): "the 'good' value is not always 0 or
/// 3", and **no field of any header records how a pad was cut**, so §7.2's "returns the configured
/// value" has no configuration to read. What the test pins is the decode -- the board answers
/// inside each mask and drives nothing outside it -- and that the scratch RAM is RAM.
///
/// The scratch RAM's printed mask, "(Address AND $8310)=$8300", is a copy of the PRG bank
/// register's and cannot be right: `$5100 AND $8310` is `$0100`. The address pattern
/// `A~[0101 ..01 .... 00AA]` is used instead (Q-166).
#[cfg(feature = "board-cony")]
#[test]
fn q_166_cony_reads_its_solder_pad_at_5000_and_its_four_scratch_bytes_at_5100() {
    let mut core = MapperRom::new(83).prg(16).chr(64).load();

    // The pad: every mirror the $F100 mask admits, which is A8 clear in $5000-$5FFF.
    for address in [0x5000u16, 0x50FF, 0x5200, 0x5A00, 0x5EFF] {
        assert_eq!(
            Some(0x00),
            core.cartridge().unwrap().try_cpu_peek(address),
            "{address:#06X} is the solder pad"
        );
    }

    // The scratch RAM reads back what was written, one byte per address.
    for (index, value) in [0x11u8, 0x22, 0x33, 0x44].into_iter().enumerate() {
        core.mapper_write(0x5100 + u16::try_from(index).unwrap(), value);
    }
    for (index, value) in [0x11u8, 0x22, 0x33, 0x44].into_iter().enumerate() {
        assert_eq!(
            Some(value),
            core.cartridge()
                .unwrap()
                .try_cpu_peek(0x5100 + u16::try_from(index).unwrap()),
            "scratch byte {index}"
        );
    }

    // A4 is undecoded in the pattern, so $5110 is $5100 again; A2 is decoded, so $5104 is not.
    core.mapper_write(0x5110, 0x99);
    assert_eq!(Some(0x99), core.cartridge().unwrap().try_cpu_peek(0x5100));
    assert_eq!(
        None,
        core.cartridge().unwrap().try_cpu_peek(0x5104),
        "A2 is decoded, so $5104 is not the scratch RAM"
    );

    // And nothing of either register exists outside $5000-$5FFF.
    for address in [0x4100u16, 0x4F00, 0x6100] {
        assert_eq!(
            None,
            core.cartridge().unwrap().try_cpu_peek(address),
            "{address:#06X} is neither register"
        );
    }
}

/// Submapper 2, the **N-C88-1603/83-08-19J** 4-in-1 of *Dragon Ball Party*: the one PCB of the five
/// whose `$6000`-`$7FFF` is 32 KiB of **PRG-NVRAM** rather than PRG-ROM, with its bank in the PRG
/// base register's top two bits and the `6` bit of the mode register overridden -- "the circuit
/// board hardware around the ASIC forces CPU $6000-$7FFF to PRG-RAM instead".
///
/// That window is [`Mapper::prg_ram_window`], a method `IrqBoard` does not forward, so the PCB
/// would add a body to a `Board` dispatch for a multicart that carries no original title. It is
/// refused at load with REQ-CORE-011's message; **Q-167** has the argument and MF-UNL-6's Q-157 is
/// the precedent.
#[cfg(feature = "board-cony")]
#[test]
fn req_core_011_mapper_83_submapper_2_wants_prg_nvram_at_6000_and_is_refused() {
    let mut core = NesCore::new();
    let load = core.load_rom(
        &MapperRom::new(83).submapper(2).prg(32).chr(256).image(),
        "DragonBallParty.nes",
    );
    assert!(!load.success);
    assert!(load.error.unwrap().contains("83"));
    assert!(!core.is_running());

    // The other three submappers of mapper 83, and mapper 264, all load.
    for submapper in [0u8, 1, 3] {
        let core = MapperRom::new(83)
            .submapper(submapper)
            .prg(32)
            .chr(256)
            .load();
        assert_eq!(83, core.cartridge().unwrap().mapper_number());
    }
    assert_eq!(
        264,
        MapperRom::new(264)
            .prg(32)
            .chr(256)
            .load()
            .cartridge()
            .unwrap()
            .mapper_number()
    );
    // An iNES 1.0 image has no submapper byte, so it is the standard PCB and not the refused one.
    assert_eq!(
        83,
        MapperRom::new(83)
            .prg(32)
            .chr(256)
            .load()
            .cartridge()
            .unwrap()
            .mapper_number()
    );
}

// ------------------------------------------- mapper 91, the JY830623C and the EJ-006-1,
// ------------------------------------------- MF-UNL-8 step 4

/// The four register groups the page lists, on both boards: four 2 KiB CHR banks at
/// `$6000`-`$6003`, two 8 KiB PRG banks at `$7000`-`$7001`, and `$C000`-`$FFFF` "hard-wired to
/// last bank".
///
/// On submapper 0 the outer register is "PRG A17-A18" above a **128 KiB inner bank**, so the two
/// bank registers drive A13-A16 and the fixed 16 KiB is the last of that inner bank — the same
/// reading of the same words the Cony board's `$E000` window takes. Submapper 1 has no outer
/// register at all, the page gives its two bank registers no diagram, and its fixed 16 KiB is the
/// ROM's last.
#[cfg(feature = "board-91")]
#[test]
fn req_core_102_mapper_91_banks_two_prg_windows_and_four_2_kib_chr_windows() {
    // Submapper 0, with a 128 KiB inner bank over a 256 KiB ROM.
    let mut core = MapperRom::new(91).prg(32).chr(64).load();
    assert_eq!(
        [0, 0, 14, 15],
        core.prg_windows(),
        "power-on: bank 0 twice and the last of the inner bank fixed"
    );
    core.mapper_write(0x7000, 3);
    core.mapper_write(0x7001, 5);
    assert_eq!([3, 5, 14, 15], core.prg_windows());

    // The registers drive A13-A16 alone on this board, so bits above the inner bank are not
    // latched -- that is what the outer register is for.
    core.mapper_write(0x7000, 0x13);
    assert_eq!(
        3,
        core.prg_windows()[0],
        "A17 and above are the outer bank's"
    );

    // Four 2 KiB CHR windows, one register each.
    for register in 0..4u16 {
        core.mapper_write(0x6000 + register, u8::try_from(register).unwrap() + 1);
    }
    assert_eq!([2, 3, 4, 5, 6, 7, 8, 9], core.chr_windows());

    // Submapper 1: no inner bank, so all eight bits of each register reach the ROM and the fixed
    // 16 KiB is the ROM's own last.
    let mut core = MapperRom::new(91).submapper(1).prg(32).chr(64).load();
    core.mapper_write(0x7000, 0x13);
    core.mapper_write(0x7001, 0x15);
    assert_eq!([19, 21, 30, 31], core.prg_windows());
}

/// "Mask: `$F003`/`$F007` (submapper 0/1)" — **one bit, and it is the whole submapper**.
///
/// With the mask at `$03`, `$6004` and `$6005` are CHR registers 0 and 1 again and `$6006`/`$6007`
/// are registers 2 and 3, which is exactly a board with "hard-wired mirroring" and no counter
/// register; `$7006` and `$7007` are then indices 2 and 3, which is where that board's IRQ ports
/// are. The test writes each address on each board and shows the two decodes apart.
#[cfg(feature = "board-91")]
#[test]
fn req_core_102_mapper_91_decodes_its_registers_under_f003_or_f007_by_submapper() {
    // Submapper 0: $6004 is CHR register 0 and $6006 is CHR register 2.
    let mut core = MapperRom::new(91).prg(16).chr(64).load();
    core.mapper_write(0x6004, 7);
    core.mapper_write(0x6006, 9);
    assert_eq!(
        [14, 15, 0, 1, 18, 19, 0, 1],
        core.chr_windows(),
        "the $F003 mask folds $6004-$6007 back onto the four CHR registers"
    );
    assert_eq!(
        MapperRom::new(91).prg(16).chr(64).load().mirroring(),
        core.mirroring(),
        "hard-wired mirroring: the header's, and no register moves it"
    );

    // And every mirror of a CHR register the mask admits is that register: A11-A2 are undecoded.
    core.mapper_write(0x6FFC, 3);
    assert_eq!(6, core.chr_windows()[0], "$6FFC is CHR register 0");

    // Submapper 1: $6004 and $6005 are the mirroring ports and $6006/$6007 the counter, so none of
    // the four touches CHR at all.
    let mut core = MapperRom::new(91).submapper(1).prg(16).chr(64).load();
    for register in 0..4u16 {
        core.mapper_write(0x6000 + register, u8::try_from(register).unwrap() + 1);
    }
    let banks = core.chr_windows();
    for address in [0x6004u16, 0x6005, 0x6006, 0x6007] {
        core.mapper_write(address, 0x7F);
    }
    assert_eq!(banks, core.chr_windows(), "$6004-$6007 are not CHR here");
}

/// "$6004: Select **horizontal mirroring**, value ignored" and "$6005: Select **vertical
/// mirroring**, value ignored" — submapper 1 only.
///
/// The page uses the *mirroring* vocabulary here where mapper 83's page gives both namings, so this
/// takes the word the page uses rather than inverting it; **Q-168** records that the two pages of
/// this lot are not consistent with each other about it.
#[cfg(feature = "board-91")]
#[test]
fn q_168_mapper_91_submapper_1_selects_mirroring_from_6004_and_6005() {
    let mut core = MapperRom::new(91).submapper(1).prg(16).chr(64).load();
    core.mapper_write(0x6004, 0x00);
    assert_eq!(Nm::Horizontal, core.mirroring(), "the value is ignored");
    core.mapper_write(0x6005, 0xFF);
    assert_eq!(Nm::Vertical, core.mirroring(), "and ignored here too");
    core.mapper_write(0x6004, 0xA5);
    assert_eq!(Nm::Horizontal, core.mirroring());

    // Every mirror the $F007 mask admits reaches them: A11-A3 are undecoded.
    core.mapper_write(0x6FFD, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring(), "$6FFD is $6005");

    // And neither port exists on submapper 0, whose mirroring is hard-wired: whatever the header's
    // byte 6 said at load is what the board shows, through both ports and whatever the value.
    for bits in [0x00u8, 0x01] {
        let mut core = MapperRom::new(91).prg(16).chr(64).mirroring(bits).load();
        let wired = core.mirroring();
        for address in [0x6004u16, 0x6005] {
            core.mapper_write(address, 0x00);
            assert_eq!(
                wired,
                core.mirroring(),
                "{address:#06X} does not move a hard-wired board"
            );
        }
    }
}

/// The outer bank register of submapper 0, whose diagram is an **`A~`** and not a `D~`:
///
/// ```text
/// A~FEDC BA98 7654 3210
///   -------------------
///   .... .... .... .PPC
///                   ||+- Select outer 512 KiB CHR-ROM bank (CHR A19)
///                   ++-- Select outer 128 KiB PRG-ROM bank (PRG A17-A18)
/// ```
///
/// So which address of `$8000`-`$9FFF` a game writes is the register and the byte it writes is not
/// read at all — which this test shows by writing the **same value** to four addresses and getting
/// four layouts. `C` is A0 and `PP` is A2-A1, taken MSB-first as every other nesdev field diagram
/// is (**Q-168**: the page's own label writes the pair ascending, which is the one place it could
/// be read the other way, and the difference is the two middle PRG banks).
#[cfg(feature = "board-91")]
#[test]
fn q_168_mapper_91_submapper_0_takes_its_outer_bank_from_the_address_written() {
    // A19 is the 512 KiB boundary — "select outer **512 KiB** CHR-ROM bank" — so a megabyte of
    // CHR is what it takes to show the line at all.
    let mut core = MapperRom::new(91).prg(64).chr(1024).load();
    core.mapper_write(0x7000, 3);
    core.mapper_write(0x7001, 5);
    core.mapper_write(0x6000, 1);
    let a19 = 512 * 0x400;

    // `PP` = 0: the first 128 KiB.
    assert_eq!([3, 5, 14, 15], core.prg_windows());
    assert!(chr_offsets(&core)[0] < a19, "no CHR A19 yet");

    // A0 alone is CHR A19 and moves no PRG bank at all.
    core.mapper_write(0x8001, 0x00);
    assert_eq!([3, 5, 14, 15], core.prg_windows(), "A0 is not a PRG line");
    assert!(
        chr_offsets(&core)[0] >= a19,
        "A0 is CHR A19, from the address and not the datum"
    );

    // A1 is PRG A17 and A2 is PRG A18, so $8002 is the second 128 KiB and $8004 the third.
    core.mapper_write(0x8002, 0x00);
    assert_eq!([19, 21, 30, 31], core.prg_windows(), "A1 is PRG A17");
    assert!(chr_offsets(&core)[0] < a19, "and A0 is clear again");

    core.mapper_write(0x8004, 0x00);
    assert_eq!([35, 37, 46, 47], core.prg_windows(), "A2 is PRG A18");

    core.mapper_write(0x8007, 0x00);
    assert_eq!([51, 53, 62, 63], core.prg_windows(), "both lines at once");
    assert!(chr_offsets(&core)[0] >= a19, "and CHR A19 with them");

    // The datum really is ignored: the same address with every value gives the same layout.
    for value in [0x00u8, 0xFF, 0x5A] {
        core.mapper_write(0x8002, value);
        assert_eq!(
            [19, 21, 30, 31],
            core.prg_windows(),
            "{value:#04X}: the value written is not the register"
        );
    }

    // The page's own range is $8000-$9FFF. Submapper 1 has no such register, so the same writes
    // move nothing there.
    let mut core = MapperRom::new(91).submapper(1).prg(64).chr(1024).load();
    core.mapper_write(0x7000, 3);
    let before = core.prg_windows();
    for address in [0x8001u16, 0x8002, 0x8004, 0x9007] {
        core.mapper_write(address, 0x00);
    }
    assert_eq!(
        before,
        core.prg_windows(),
        "submapper 1 has no outer bank register"
    );
}

// -------------------- MF-UNL-8's four save-state round trips, §7.1's second common thing

/// What a board of this lot shows, and nothing of the machine around it: the four PRG windows, the
/// eight CHR window **offsets** and the mirroring.
///
/// The offsets and not the bytes, for `chr_offsets`'s own reason: three of the four boards move a
/// CHR window by 256 or 512 banks and the fixture's fill byte reads the same 256 banks apart.
#[cfg(any(feature = "board-91", feature = "board-cony", feature = "board-huang1"))]
fn unl8_layout(core: &NesCore) -> ([u8; 4], [usize; 8], Nm) {
    (core.prg_windows(), chr_offsets(core), core.mirroring())
}

/// REQ-CORE-090/091: mapper 12's GAL register is at **`$4132`**, which
/// `req_core_090_the_mapper_state_survives_a_save_and_restore` never writes — that sweep dirties
/// `$8000`-`$E001` and `$6000` — so the round trip is pinned here with the board's own register
/// moved first (MF-UNL-8 step 1).
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_090_mapper_12_round_trips_its_outer_chr_register() {
    let mut core = MapperRom::new(12).prg(32).chr(512).battery().load();
    core.mmc3_bank(0, 0x12);
    core.mmc3_bank(6, 3);
    core.mapper_write(0x4132, 0x11); // Both CHR A18 lines.
    core.mapper_write(0xC000, 0x20); // And the counter, so that it has run.
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    let layout = unl8_layout(&core);

    // Move it somewhere the chip's own registers could not reach: the lines off, which is a
    // 256-bank move on every window.
    core.mapper_write(0x4132, 0x00);
    assert_ne!(layout, unl8_layout(&core), "the register moved the layout");

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(layout, unl8_layout(&core), "and the state brought it back");
    assert_eq!(
        state,
        core.save_state(true).unwrap(),
        "the whole block, derived fields included"
    );
}

/// REQ-CORE-090/091: mapper 14 keeps **two chips' registers at once** — the page requires it, which
/// is why the block carries both whatever the mode is — and its supervisor register is at `$A131`,
/// an address the generic sweep does not write (MF-UNL-8 step 2).
///
/// The round trip is taken with each chip's registers set to a layout only that chip could produce,
/// so that a state restored into a board sitting in the *other* mode would be caught.
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_090_mapper_14_round_trips_both_chips_and_the_supervisor() {
    let mut core = MapperRom::new(14).prg(32).chr(512).battery().load();

    // The MMC3's own select-and-value pair, in MMC3 mode.
    core.mmc3_bank(6, 3);
    core.mmc3_bank(7, 5);
    // The VRC2's two independent PRG registers and its mirroring, in VRC2 mode.
    core.mapper_write(0xA131, 0x00);
    core.mapper_write(0x8000, 0x07);
    core.mapper_write(0xA000, 0x09);
    core.mapper_write(0x9000, 0x01);
    // Back to MMC3 mode with two of the three CHR A18 lines set, and let the counter run.
    core.mapper_write(0xA131, 0x02 | 0x08 | 0x80);
    core.mapper_write(0xC000, 0x20);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    let layout = unl8_layout(&core);

    // Walk away through the other mode and change its registers too, so that nothing the capture
    // held is still live.
    core.mapper_write(0xA131, 0x00);
    core.mapper_write(0x8000, 0x02);
    core.mapper_write(0xA000, 0x04);
    assert_ne!(layout, unl8_layout(&core));

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(layout, unl8_layout(&core), "the MMC3 mode's own layout");
    assert_eq!(state, core.save_state(true).unwrap());

    // And the VRC2's registers came back with it: switching to its mode shows what was set before
    // the capture and not what was written after it.
    core.mapper_write(0xA131, 0x00);
    assert_eq!(
        [7, 9, 30, 31],
        core.prg_windows(),
        "the VRC2 kept its own registers across the round trip"
    );
    assert_eq!(Nm::Horizontal, core.mirroring(), "and its mirroring");

    // Mapper 12's board has no VRC2, so its block is **shorter** than mapper 14's: a block carries
    // what the board has, which is `Mmc3Board`'s own shape for mapper 195's GAL byte.
    let twelve = MapperRom::new(12)
        .prg(32)
        .chr(512)
        .battery()
        .load()
        .save_state(true)
        .unwrap();
    let fourteen = MapperRom::new(14)
        .prg(32)
        .chr(512)
        .battery()
        .load()
        .save_state(true)
        .unwrap();
    assert!(
        twelve.len() < fourteen.len(),
        "mapper 12 writes no VRC2 block: {} against {}",
        twelve.len(),
        fourteen.len()
    );
}

/// REQ-CORE-090/091: the Cony board's register file is at `$8000`-`$831F` and its four scratch
/// bytes at `$5100`-`$5103`, and the generic sweep reaches only the first of those — `$8000` is the
/// PRG base under this board's mask and nothing else it writes is a register here
/// (MF-UNL-8 step 3).
#[cfg(feature = "board-cony")]
#[test]
fn req_core_090_cony_round_trips_its_register_file_and_its_scratch_ram() {
    for mapper in [83usize, 264] {
        let mut core = if mapper == 83 {
            MapperRom::new(83)
                .submapper(1)
                .prg(32)
                .chr(256)
                .battery()
                .load()
        } else {
            MapperRom::new(264).prg(32).chr(256).battery().load()
        };

        cony_write(&mut core, mapper, 0x8100, 0x80 | 0x20 | 0x10 | 0x02); // Arm, $6000 on, mode 2, single-screen 0.
        cony_write(&mut core, mapper, 0x8000, 0x04);
        for (register, value) in [(0u16, 3u8), (1, 5), (2, 7), (3, 9)] {
            cony_write(&mut core, mapper, 0x8300 + register, value);
        }
        for (register, value) in [(0u16, 1u8), (1, 2), (6, 3), (7, 4)] {
            cony_write(&mut core, mapper, 0x8310 + register, value);
        }
        cony_write(&mut core, mapper, 0x8318, 0xFF); // The A12 clock source.
        cony_write(&mut core, mapper, 0x8200, 0x34); // The counter, LSB then MSB.
        cony_write(&mut core, mapper, 0x8201, 0x12);
        for (index, value) in [0x11u8, 0x22, 0x33, 0x44].into_iter().enumerate() {
            cony_write(
                &mut core,
                mapper,
                0x5100 + u16::try_from(index).unwrap(),
                value,
            );
        }

        let state = core.save_state(true).unwrap();
        let layout = unl8_layout(&core);
        let window_6000 = core.cartridge().unwrap().try_cpu_peek(0x6000);
        assert_eq!(Some(9), window_6000, "mapper {mapper}: $8303's window");

        // Move every one of them.
        cony_write(&mut core, mapper, 0x8100, 0x01);
        cony_write(&mut core, mapper, 0x8000, 0x0A);
        for register in 0..4u16 {
            cony_write(&mut core, mapper, 0x8300 + register, 0x1F);
        }
        for register in 0..8u16 {
            cony_write(&mut core, mapper, 0x8310 + register, 0x7E);
        }
        cony_write(&mut core, mapper, 0x8318, 0x00);
        for index in 0..4u16 {
            cony_write(&mut core, mapper, 0x5100 + index, 0x00);
        }
        assert_ne!(layout, unl8_layout(&core), "mapper {mapper}");

        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed);
        assert_eq!(layout, unl8_layout(&core), "mapper {mapper}: the windows");
        assert_eq!(
            window_6000,
            core.cartridge().unwrap().try_cpu_peek(0x6000),
            "mapper {mapper}: the $6000 window"
        );
        for (index, value) in [0x11u8, 0x22, 0x33, 0x44].into_iter().enumerate() {
            assert_eq!(
                Some(value),
                core.cartridge().unwrap().try_cpu_peek(cony_register(
                    mapper,
                    0x5100 + u16::try_from(index).unwrap()
                )),
                "mapper {mapper}: scratch byte {index}"
            );
        }
        assert_eq!(
            state,
            core.save_state(true).unwrap(),
            "mapper {mapper}: the whole block, the clock source included"
        );
    }
}

/// REQ-CORE-090/091: mapper 91's register file is at `$6000`-`$7001` and its outer bank register is
/// the **address** of a write to `$8000`-`$9FFF`, so the generic sweep's `$6000` write reaches one
/// CHR register and its `$8000`-`$E001` writes reach the outer register by accident and nothing
/// else. Both boards are round-tripped here with every register of each moved first
/// (MF-UNL-8 step 4).
#[cfg(feature = "board-91")]
#[test]
fn req_core_090_mapper_91_round_trips_both_of_its_boards() {
    for submapper in [0u8, 1] {
        let mut core = MapperRom::new(91)
            .submapper(submapper)
            .prg(64)
            .chr(1024)
            .battery()
            .load();

        for register in 0..4u16 {
            core.mapper_write(0x6000 + register, u8::try_from(register).unwrap() + 1);
        }
        core.mapper_write(0x7000, 3);
        core.mapper_write(0x7001, 5);
        if submapper == 0 {
            core.mapper_write(0x8003, 0x00); // The outer bank: CHR A19 and PRG A17.
            core.mapper_write(0x7007, 0x00); // Start the A12 counter.
            let cartridge = core.cartridge_mut().unwrap();
            for rise in 0..10 {
                cartridge.on_ppu_address_bus(0x2000, i64::from(rise) * 8);
                cartridge.on_ppu_address_bus(0x1000, i64::from(rise) * 8 + 2);
            }
        } else {
            core.mapper_write(0x6004, 0x00); // Horizontal mirroring.
            core.mapper_write(0x6006, 0x34); // The counter, LSB then MSB.
            core.mapper_write(0x6007, 0x12);
            core.mapper_write(0x7007, 0x00); // Start the M2 counter.
            for _ in 0..10 {
                core.bus_mut().tick();
            }
        }

        let state = core.save_state(true).unwrap();
        let layout = unl8_layout(&core);

        // Move every register the board has.
        for register in 0..4u16 {
            core.mapper_write(0x6000 + register, 0x7E);
        }
        core.mapper_write(0x7000, 0x0A);
        core.mapper_write(0x7001, 0x0C);
        if submapper == 0 {
            core.mapper_write(0x8004, 0x00);
        } else {
            core.mapper_write(0x6005, 0x00);
        }
        assert_ne!(layout, unl8_layout(&core), "submapper {submapper}");

        let loaded = core.try_load_state(&state, 0).unwrap();
        assert_eq!(state.len(), loaded.consumed);
        assert_eq!(
            layout,
            unl8_layout(&core),
            "submapper {submapper}: the windows and the mirroring"
        );
        assert_eq!(
            state,
            core.save_state(true).unwrap(),
            "submapper {submapper}: the whole block, the counter included"
        );
    }
}

/// AMD-061 §5's rule for this lot's four boards: a reset "puts the mapper back to its initial
/// state", and the generic sweep
/// `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` dirties none of the
/// registers these four keep outside `$8000`-`$E001` and `$6000`. This one does.
///
/// Mapper 14's power-on is the one that is not all zeroes: its supervisor register is `$02`, MMC3
/// mode (Q-165), so after a reset the board is the MMC3 again whatever mode it was left in.
#[cfg(all(feature = "board-91", feature = "board-cony", feature = "board-huang1"))]
#[test]
fn amd_061_a_reset_puts_every_board_of_mf_unl_8_back_to_its_power_on_state() {
    for mapper in [12usize, 14, 83, 91, 264] {
        let mut core = MapperRom::new(mapper).prg(32).chr(256).load();
        let power_on = unl8_layout(&core);

        match mapper {
            12 => {
                core.mapper_write(0x4132, 0x11);
                core.mmc3_bank(6, 3);
            }
            14 => {
                core.mapper_write(0xA131, 0x00); // VRC2 mode, no lines.
                core.mapper_write(0x8000, 0x07);
                core.mapper_write(0x9000, 0x01);
            }
            83 | 264 => {
                cony_write(&mut core, mapper, 0x8100, 0x10 | 0x01);
                cony_write(&mut core, mapper, 0x8000, 0x04);
                cony_write(&mut core, mapper, 0x8300, 3);
                cony_write(&mut core, mapper, 0x8310, 7);
            }
            _ => {
                core.mapper_write(0x6000, 7);
                core.mapper_write(0x7000, 3);
                core.mapper_write(0x8003, 0x00);
            }
        }
        assert_ne!(power_on, unl8_layout(&core), "mapper {mapper}: dirtied");

        core.reset();
        assert_eq!(power_on, unl8_layout(&core), "mapper {mapper}");
        assert!(!core.cartridge().unwrap().irq_asserted());
    }
}

// ------------------------------------------- Dongda PEC-9588 (164), lot MF-UNL-10

/// Drives the three Microwire lines of mapper 164's 93C66 from the CPU side, through `$5200`:
/// "`.T.S .C.D`" — DI on bit 0, CLK on bit 2, the populated chip's CS on bit 4 and the
/// unpopulated second chip's on bit 6 (nesdev, "INES Mapper 164"; Microchip DS20001795F
/// table 1-4). MF-UNL-10 step 2.
#[cfg(feature = "board-164")]
mod microwire {
    use super::{MapperMachine, NesCore};

    /// `10`, `01`, `11` and `00` of table 1-4.
    pub const READ: u8 = 0b10;
    pub const WRITE: u8 = 0b01;
    pub const ERASE: u8 = 0b11;
    pub const EXTENDED: u8 = 0b00;

    /// The sub-opcode of the extended instructions, which lives in the top two bits of the
    /// nine-bit address field: EWDS `00`, WRAL `01`, ERAL `10`, EWEN `11`.
    pub const EWDS: u16 = 0b0_0000_0000;
    pub const WRAL: u16 = 0b0_1000_0000;
    pub const ERAL: u16 = 0b1_0000_0000;
    pub const EWEN: u16 = 0b1_1000_0000;

    pub fn lines(core: &mut NesCore, cs: u8, clk: u8, di: u8) {
        core.mapper_write(0x5200, (cs << 4) | (clk << 2) | di);
    }

    /// One clock of `bit`: CLK low with DI set up, then CLK high to clock it in.
    pub fn clock(core: &mut NesCore, bit: u8) {
        lines(core, 1, 0, bit);
        lines(core, 1, 1, bit);
    }

    /// "When preparing to transmit an instruction, either the CLK or DI signal levels must be at
    /// a logic low as CS is toggled active high" (DS20001795F section 2.1).
    pub fn select(core: &mut NesCore) {
        lines(core, 0, 0, 0);
        lines(core, 1, 0, 0);
    }

    /// "DO will enter the High-Z state on the falling edge of CS" (section 2.0).
    pub fn deselect(core: &mut NesCore) {
        lines(core, 1, 0, 0);
        lines(core, 0, 0, 0);
    }

    /// The `width` bits of `value`, most significant first — `A8 A7 ... A0` and `D7 ... D0`.
    pub fn send(core: &mut NesCore, value: u16, width: u8) {
        for index in 0..width {
            clock(core, ((value >> (width - 1 - index)) & 1) as u8);
        }
    }

    /// The chip's DO, which the board presents **inverted** in bit 2 of `$5500`.
    pub fn data_out(core: &NesCore) -> u8 {
        (!(core.read_memory(0x5500) >> 2)) & 1
    }

    /// One clock rise with DI low, and the bit the chip drives after it.
    pub fn receive_bit(core: &mut NesCore) -> u8 {
        lines(core, 1, 0, 0);
        lines(core, 1, 1, 0);
        data_out(core)
    }

    /// A start bit, a two-bit opcode and nine address bits. CS stays high afterwards, so a
    /// caller may go on to clock data.
    pub fn command(core: &mut NesCore, opcode: u8, address: u16) {
        select(core);
        clock(core, 1); // the start bit.
        send(core, u16::from(opcode), 2);
        send(core, address, 9);
    }

    /// The whole of one extended instruction with no data behind it.
    pub fn extended(core: &mut NesCore, sub_opcode: u16) {
        command(core, EXTENDED, sub_opcode);
        deselect(core);
    }

    pub fn write_word(core: &mut NesCore, address: u16, value: u8) {
        command(core, WRITE, address);
        send(core, u16::from(value), 8);
        deselect(core);
    }

    pub fn erase_word(core: &mut NesCore, address: u16) {
        command(core, ERASE, address);
        deselect(core);
    }

    pub fn write_all(core: &mut NesCore, value: u8) {
        command(core, EXTENDED, WRAL);
        send(core, u16::from(value), 8);
        deselect(core);
    }

    /// Eight bits out of the chip, most significant first.
    pub fn receive_word(core: &mut NesCore) -> u8 {
        (0..8).fold(0u8, |word, _| (word << 1) | receive_bit(core))
    }

    /// A read burst: the dummy zero the address's last clock puts on DO, then `count` words
    /// clocked out sequentially with CS held high.
    pub fn read_burst(core: &mut NesCore, address: u16, count: usize) -> (u8, Vec<u8>) {
        command(core, READ, address);
        let dummy = data_out(core);
        let words = (0..count).map(|_| receive_word(core)).collect();
        deselect(core);
        (dummy, words)
    }

    /// One word, for the many assertions that want exactly that.
    pub fn read_word(core: &mut NesCore, address: u16) -> u8 {
        read_burst(core, address, 1).1[0]
    }
}

/// The 93C66's seven instructions, bit by bit over the serial line, against table 1-4 of
/// Microchip DS20001795F — the **x8** organisation, which is the one the page selects ("The
/// EEPROM's 'ORG' pin is tied to GND, selecting a word size of 8 bits") and which section 7.2 of
/// `docs/review/UNL-census.md` gets wrong (Q-170 section 1.1).
///
/// The census asks for exactly this: "the EEPROM's read and write command sequences, bit by bit
/// over the serial line, against the 93C66's documented protocol".
#[cfg(feature = "board-164")]
#[test]
fn req_core_102_the_93c66_clocks_the_datasheets_seven_instructions() {
    let mut core = MapperRom::new(164).prg(16).chr(0).battery().load();

    // The two memories of this board, and which of them REQ-CORE-095 exposes: 2 KiB of volatile
    // RAM the CPU addresses, then the EEPROM's 512 bytes, which are the whole battery slice.
    assert_eq!(
        2 * 1024 + 512,
        core.cartridge().unwrap().memory().prg_ram().len()
    );
    assert_eq!(512, core.cartridge().unwrap().battery_ram().len());

    // "After power-up, the device is automatically in the EWDS mode. Therefore, an EWEN
    // instruction must be performed before the initial ERASE or WRITE instruction can be
    // executed" (section 2.6): this write changes nothing.
    microwire::write_word(&mut core, 0x000, 0x5A);
    assert_eq!(
        (0, vec![0x00]),
        microwire::read_burst(&mut core, 0x000, 1),
        "a WRITE before an EWEN, and the dummy zero of a READ"
    );

    // EWEN, and the same write lands. "Execution of a READ instruction is independent of both the
    // EWEN and EWDS instructions", which the read above already relied on.
    microwire::extended(&mut core, microwire::EWEN);
    microwire::write_word(&mut core, 0x000, 0x5A);
    assert_eq!(0x5A, microwire::read_word(&mut core, 0x000));

    // **Nine address bits**, which is the whole of Q-170 section 1.1: $1FF and $0FF differ by A8
    // alone, so an eight-bit address field would alias them.
    microwire::write_word(&mut core, 0x1FF, 0xA5);
    assert_eq!(0xA5, microwire::read_word(&mut core, 0x1FF));
    assert_eq!(
        0x00,
        microwire::read_word(&mut core, 0x0FF),
        "A8 is a ninth address bit and not a don't-care"
    );

    // Data is clocked **most significant bit first** (`D7 ... D0`): $B2 reversed is $4D, so the
    // byte that comes back tells the two orders apart.
    microwire::write_word(&mut core, 0x011, 0xB2);
    assert_eq!(0xB2, microwire::read_word(&mut core, 0x011));

    // "Sequential read is possible when CS is held high. The memory data will automatically cycle
    // to the next register and output sequentially" (section 2.7) — and figure 2-7 shows **one**
    // dummy zero for the whole burst, not one per word.
    for (address, value) in [(0x020u16, 0x11u8), (0x021, 0x22), (0x022, 0x33)] {
        microwire::write_word(&mut core, address, value);
    }
    assert_eq!(
        (0, vec![0x11, 0x22, 0x33]),
        microwire::read_burst(&mut core, 0x020, 3)
    );
    // The address the chip increments wraps inside the 512 words it has.
    microwire::write_word(&mut core, 0x000, 0x7B);
    assert_eq!(
        (0, vec![0xA5, 0x7B]),
        microwire::read_burst(&mut core, 0x1FF, 2),
        "the sequential read wraps $1FF to $000"
    );

    // "The ERASE instruction forces all data bits of the specified address to the logical 1
    // state" (section 2.4) — one word, and its neighbours untouched.
    microwire::erase_word(&mut core, 0x021);
    assert_eq!(
        (0, vec![0x11, 0xFF, 0x33]),
        microwire::read_burst(&mut core, 0x020, 3)
    );

    // "The Write All (WRAL) instruction will write the entire memory array with the data
    // specified in the command" (section 2.9).
    microwire::write_all(&mut core, 0x7E);
    assert!(
        core.cartridge()
            .unwrap()
            .battery_ram()
            .iter()
            .all(|byte| *byte == 0x7E),
        "WRAL writes all 512 words"
    );

    // "The Erase All (ERAL) instruction will erase the entire memory array to the logical 1
    // state" (section 2.5).
    microwire::extended(&mut core, microwire::ERAL);
    assert!(
        core.cartridge()
            .unwrap()
            .battery_ram()
            .iter()
            .all(|byte| *byte == 0xFF),
        "ERAL erases all 512 words"
    );

    // EWDS, and "programming remains enabled until an EWDS instruction is executed" the other way
    // round: nothing the chip is asked to program afterwards moves.
    microwire::extended(&mut core, microwire::EWDS);
    microwire::write_word(&mut core, 0x030, 0x01);
    microwire::erase_word(&mut core, 0x031);
    microwire::write_all(&mut core, 0x00);
    assert!(
        core.cartridge()
            .unwrap()
            .battery_ram()
            .iter()
            .all(|byte| *byte == 0xFF),
        "every programming instruction is inhibited after EWDS"
    );
}

/// `$5500`: "93C66 EEPROM DAT input (**inverted**)" in bit 2, under the same `$FF00` mask the four
/// write ports use, with nothing else driven (MF-UNL-10 step 2; Q-170 section 2.2 for the idle
/// level).
#[cfg(feature = "board-164")]
#[test]
fn req_core_102_mapper_164_answers_the_microwire_line_inverted() {
    let mut core = MapperRom::new(164).prg(16).chr(0).battery().load();

    // Idle: DO is undriven, which this build reads as 1 (Q-170 section 2.2), so the inverted bit
    // is 0 and no other bit of the register is driven either.
    assert_eq!(0x00, core.read_memory(0x5500));
    // Mask $FF00: the port answers at every address of $5500-$55FF.
    assert_eq!(0x00, core.read_memory(0x55FF));

    // A read burst drives DO, and the register shows its inverse. The dummy zero is the first
    // thing out, so bit 2 is **set** at that moment.
    microwire::extended(&mut core, microwire::EWEN);
    microwire::write_word(&mut core, 0x040, 0xFF);
    microwire::command(&mut core, microwire::READ, 0x040);
    assert_eq!(0x04, core.read_memory(0x5500), "the dummy zero, inverted");
    for _ in 0..8 {
        assert_eq!(1, microwire::receive_bit(&mut core), "$FF, bit by bit");
        assert_eq!(0x00, core.read_memory(0x5500));
    }
    microwire::deselect(&mut core);
    assert_eq!(0x00, core.read_memory(0x5500), "High-Z again");
}

/// The two PRG banking modes of `$5000` bit 4, with the two shapes of semi-fixed bank `S` selects
/// and the outer bits of `$5100` above them (nesdev, "INES Mapper 164").
///
/// The page's mode strings are written **A18 first**, which it proves twice: `111p0` is "fixed
/// bank=1C or 1E", which only holds read that way, and `11111` is bank `$1F`, which its own note
/// confirms — "the game's reset vectors will be at the end of 16 KiB bank #31".
#[cfg(feature = "board-164")]
#[test]
fn req_core_102_mapper_164_banks_prg_in_both_modes_with_two_fixed_banks() {
    // 2 MiB, which is what seven bank bits address: 256 banks of 8 KiB, each filled with its own
    // number, so the outer bits of `$5100` are visible instead of wrapping.
    let mut core = MapperRom::new(164).prg(256).chr(0).load();

    // Power-on. "Because this register is initialized to $00 on reset, thus clearing bit 4 which
    // causes UxROM mode, the game's reset vectors will be at the end of 16 KiB bank #31":
    // 16 KiB bank 0 at $8000 and bank $1F -- 8 KiB banks 62 and 63 -- at $C000.
    assert_eq!([0, 1, 62, 63], core.prg_windows());

    // `PPPp` is A17..A14, so the nibble is the low four bits of the 16 KiB bank number.
    core.mapper_write(0x5000, 0x05);
    assert_eq!([10, 11, 62, 63], core.prg_windows());

    // `Q` is A18, bit 4 of the bank number: $20 of the port is $10 of the bank.
    core.mapper_write(0x5000, 0x25);
    assert_eq!([42, 43, 62, 63], core.prg_windows());

    // `S` set: "PRG A14..A18=111p0 when CPU A14=1 and S=1 (fixed bank=1C or 1E)", `p` being bit 0
    // of the port. With `p` clear the fixed bank is $1C = 28, i.e. 8 KiB banks 56 and 57.
    core.mapper_write(0x5000, 0x40);
    assert_eq!([0, 1, 56, 57], core.prg_windows());
    // With `p` set it is $1E = 30, and the switchable bank moves to 1 with it.
    core.mapper_write(0x5000, 0x41);
    assert_eq!([2, 3, 60, 61], core.prg_windows());

    // "$5100: .... ..PP, PRG A20..A19" -- two bits above A18, i.e. +32 and +64 on a 16 KiB bank,
    // and they reach the fixed window too.
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5100, 0x01);
    assert_eq!([64, 65, 126, 127], core.prg_windows());
    core.mapper_write(0x5100, 0x02);
    assert_eq!([128, 129, 190, 191], core.prg_windows());
    core.mapper_write(0x5100, 0x03);
    assert_eq!([192, 193, 254, 255], core.prg_windows());

    // Mode 1: "PRG A14=CPU A14, PRG A15..A18=PPPp (BxROM, 32 KiB switchable bank)" -- the same
    // nibble is now the 32 KiB bank number, and `$5100` sits two bits above it.
    core.mapper_write(0x5100, 0x00);
    core.mapper_write(0x5000, 0x10);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    core.mapper_write(0x5000, 0x1F);
    assert_eq!([60, 61, 62, 63], core.prg_windows());
    core.mapper_write(0x5100, 0x03);
    assert_eq!([252, 253, 254, 255], core.prg_windows());

    // `Q` and `S` are UxROM-mode lines and neither reaches a BxROM bank: the page gives mode 1
    // four lines and no fixed bank at all.
    core.mapper_write(0x5000, 0x7F);
    assert_eq!([252, 253, 254, 255], core.prg_windows());

    // And back: clearing the mode bit restores the pair of windows.
    core.mapper_write(0x5000, 0x00);
    assert_eq!([192, 193, 254, 255], core.prg_windows());
}

/// "`$5000` bit 4 ... Also selects nametable mirroring: 0: Forced vertical mirroring; 1: Mirroring
/// selected by `$5300`", whose bit 7 is "0=Horizontal, 1=Vertical" — the census's "mirroring from
/// $5300" (MF-UNL-10 step 2).
#[cfg(feature = "board-164")]
#[test]
fn req_core_102_mapper_164_forces_vertical_mirroring_until_5300_takes_over() {
    // A horizontal header, so that "forced vertical" is visibly the board's doing.
    let mut core = MapperRom::new(164).prg(16).chr(0).load();
    assert_eq!(Nm::Vertical, core.mirroring());

    // With the mode bit clear, `$5300` is not consulted at all.
    for value in [0x80u8, 0x00, 0xFF] {
        core.mapper_write(0x5300, value);
        assert_eq!(
            Nm::Vertical,
            core.mirroring(),
            "$5300={value:#04X} while $5000.4 is clear"
        );
    }

    // With it set, `$5300` bit 7 decides.
    core.mapper_write(0x5300, 0x00);
    core.mapper_write(0x5000, 0x10);
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.mapper_write(0x5300, 0x80);
    assert_eq!(Nm::Vertical, core.mirroring());
    core.mapper_write(0x5300, 0x7F);
    assert_eq!(
        Nm::Horizontal,
        core.mirroring(),
        "only bit 7 of $5300 is a mirroring line"
    );

    // Clearing the mode bit forces vertical again, whatever `$5300` still holds.
    core.mapper_write(0x5000, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());
}

/// The **1 bpp video mode**: "when PPU A13=0 (pattern table) ... 1: CHR A3=PPU A0, CHR A12=PPU A9,
/// both latched on last rise of PPU A13".
///
/// The census asks that "the 1 bpp mode changes which CHR bits a pattern fetch consumes", which is
/// what the four groups of fetches below do with no register write between them: the only thing
/// that moves is the nametable address the latch last saw.
#[cfg(feature = "board-164")]
#[test]
fn req_core_102_mapper_164_takes_chr_a3_and_a12_from_the_latch_in_1_bpp_mode() {
    let mut core = MapperRom::new(164).prg(16).chr(0).load();

    // Four marker bytes at the four addresses the two substituted lines can reach, written with
    // the mode **off** so that the write path is the identity (Q-170 section 2.4).
    for (address, value) in [
        (0x0000u16, 0xC0u8),
        (0x0008, 0xC1),
        (0x1000, 0xC2),
        (0x1008, 0xC3),
    ] {
        chr_write(&mut core, address, value);
    }

    // With the mode off the chip is flat: A3 and A12 are the fetch's own, and no PPU address
    // changes that.
    for nametable in [0x2000u16, 0x2001, 0x2200, 0x2201] {
        latch_a13_rise(&mut core, nametable);
        assert_eq!(
            [0xC0, 0xC1, 0xC2, 0xC3],
            [
                peek(&core, 0x0000),
                peek(&core, 0x0008),
                peek(&core, 0x1000),
                peek(&core, 0x1008)
            ],
            "the mode is off"
        );
    }

    // Bit 7 of `$5000` turns it on. Now **all four fetches read one byte** — the one the two
    // latched bits name — because neither A3 nor A12 follows the fetch any more. PPU A0 is the
    // nametable address's coarse-X bit 0 and PPU A9 its coarse-Y bit 4.
    core.mapper_write(0x5000, 0x80);
    for (nametable, byte) in [
        (0x2000u16, 0xC0u8), // A0=0, A9=0.
        (0x2001, 0xC1),      // A0=1, A9=0.
        (0x2200, 0xC2),      // A0=0, A9=1.
        (0x2201, 0xC3),      // A0=1, A9=1.
    ] {
        latch_a13_rise(&mut core, nametable);
        assert_eq!(
            [byte; 4],
            [
                peek(&core, 0x0000),
                peek(&core, 0x0008),
                peek(&core, 0x1000),
                peek(&core, 0x1008)
            ],
            "1 bpp, latched from {nametable:#06X}"
        );
    }

    // **Both bitplanes of a tile read the same eight bytes**, which is what makes the mode 1 bpp:
    // $0008 is the high plane of tile 0 and it answers the low plane's byte.
    latch_a13_rise(&mut core, 0x2000);
    assert_eq!(peek(&core, 0x0000), peek(&core, 0x0008));

    // Everything else of the fetch is untouched: fine Y (A2-A0) and the tile index (A11-A4) still
    // address the chip, so a fetch of $0234 or $1234 with both latches set reads $123C.
    core.mapper_write(0x5000, 0x00);
    chr_write(&mut core, 0x123C, 0x5E);
    core.mapper_write(0x5000, 0x80);
    latch_a13_rise(&mut core, 0x2201);
    assert_eq!(0x5E, peek(&core, 0x0234), "fine Y and the tile index pass");
    assert_eq!(0x5E, peek(&core, 0x1234));

    // It is the **rise** of A13 and not a decode of it: once the address is inside $2000-$3FFF,
    // moving it within the range latches nothing.
    latch_a13_rise(&mut core, 0x2000);
    for address in [0x2201u16, 0x2601, 0x3E01, 0x2FFF] {
        core.cartridge_mut().unwrap().on_ppu_address_bus(address, 3);
        assert_eq!(
            0xC0,
            peek(&core, 0x0000),
            "no second rise at {address:#06X}"
        );
    }

    // A bare `$2006` write is such a rise too, the latch being on the address lines and not on
    // /RD — which is the observation that proves the number is in **both** of section 4.3's
    // lists.
    {
        let cartridge = core.cartridge_mut().unwrap();
        cartridge.on_ppu_address_bus_without_read(0x0000, 4);
        cartridge.on_ppu_address_bus_without_read(0x2201, 5);
    }
    assert_eq!(0xC3, peek(&core, 0x0000), "a bare $2006 write latches too");
}

/// **Q-170 section 2.4**: the page describes the 1 bpp substitution as a property of a fetch and
/// says nothing about a `$2007` write. CHR A3 and CHR A12 are pins on the RAM chip, so a write
/// lands where a fetch of the same address would read — the reading this build takes, pinned here.
#[cfg(feature = "board-164")]
#[test]
fn q_170_the_1_bpp_substitution_is_on_the_write_path_as_well() {
    let mut core = MapperRom::new(164).prg(16).chr(0).load();

    core.mapper_write(0x5000, 0x80);
    latch_a13_rise(&mut core, 0x2201); // A0=1, A9=1: CHR A3=1, CHR A12=1.
    chr_write(&mut core, 0x0000, 0x9D);

    // The byte is at $1008 and not at $0000, which the flat chip shows once the mode is off.
    core.mapper_write(0x5000, 0x00);
    assert_eq!(0x9D, peek(&core, 0x1008));
    assert_eq!(0x00, peek(&core, 0x0000));

    // And the read side agrees with the write side, which is the point: with the same latch, a
    // fetch of $0000 reads back what the write put there.
    core.mapper_write(0x5000, 0x80);
    latch_a13_rise(&mut core, 0x2201);
    assert_eq!(0x9D, peek(&core, 0x0000));
}

/// Section 7.1 item 2 of `docs/review/UNL-census.md` for this lot's one board: a save-state
/// round-trip with the board's **own** registers dirtied first — which the generic sweep
/// `req_core_090_the_mapper_state_survives_a_save_and_restore` cannot do, its writes being at
/// `$8000-$E001` and `$6000` where this board has no register at all.
///
/// The census also asks that "a written word survives a save-state round-trip and reaches the save
/// file", and the state is taken **mid-command** so that the EEPROM's own bus is covered too.
#[cfg(feature = "board-164")]
#[test]
fn req_core_090_mapper_164_round_trips_its_ports_its_two_latches_and_its_eeprom() {
    let mut core = MapperRom::new(164).prg(256).chr(0).battery().load();

    // A word in the EEPROM, which is the save.
    microwire::extended(&mut core, microwire::EWEN);
    microwire::write_word(&mut core, 0x123, 0x6C);

    // Every port, the two PPU-bus latches with 1 bpp on so that they reach a fetch, and a byte in
    // the volatile RAM the CPU addresses.
    core.mapper_write(0x5100, 0x03);
    core.mapper_write(0x5300, 0x80);
    core.mapper_write(0x5000, 0xD5);
    latch_a13_rise(&mut core, 0x2201);
    core.write_memory(0x6123, 0x7E);
    chr_write(&mut core, 0x0000, 0x4B);

    // And the chip left **mid-command**: a start bit, a READ opcode and four of the nine address
    // bits, so that the mode, the opcode, the partial address and the counter are all non-zero.
    microwire::select(&mut core);
    microwire::clock(&mut core, 1);
    microwire::send(&mut core, u16::from(microwire::READ), 2);
    microwire::send(&mut core, 0b1010, 4);

    let state = core.save_state(true).unwrap();
    let windows = core.prg_windows();
    let chr = core.chr_windows();
    let mirroring = core.mirroring();

    // Move everything the board has, the EEPROM included, then come back.
    microwire::send(&mut core, 0b10001, 5);
    microwire::deselect(&mut core);
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5100, 0x00);
    core.mapper_write(0x5300, 0x00);
    microwire::write_all(&mut core, 0x00);
    latch_a13_rise(&mut core, 0x2000);
    for _ in 0..5 {
        core.run_frame(0);
    }

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(windows, core.prg_windows(), "the PRG windows");
    assert_eq!(chr, core.chr_windows(), "the latched 1 bpp addressing");
    assert_eq!(mirroring, core.mirroring(), "the mirroring");
    assert_eq!(state, core.save_state(true).unwrap(), "the whole block");

    // The restored chip finishes the command it was in the middle of: the five remaining address
    // bits make $153, and the dummy zero and that word follow.
    microwire::send(&mut core, 0b10011, 5);
    assert_eq!(0, microwire::data_out(&core), "the dummy zero");
    let word = microwire::receive_word(&mut core);
    microwire::deselect(&mut core);
    assert_eq!(0x00, word, "the word the restored bus was addressing");

    // REQ-CORE-095: the written word is in what a host persists, and the volatile RAM is not.
    assert_eq!(0x6C, microwire::read_word(&mut core, 0x123));
    let saved = core.export_battery_ram().to_vec();
    assert_eq!(512, saved.len());
    assert_eq!(0x6C, saved[0x123]);
    assert_eq!(0x7E, core.read_memory(0x6123));
}

/// The volatile RAM at `$6000-$7FFF`: "0-8 KiB unbanked PRG-RAM, **not battery-backed**", which
/// this build gives as 2 KiB "mirrored three times across the respective address range" (Q-170
/// section 2.3) — and which the EEPROM behind it is no part of.
#[cfg(feature = "board-164")]
#[test]
fn q_170_mapper_164_mirrors_2_kib_of_volatile_ram_and_keeps_the_eeprom_out_of_the_window() {
    let mut core = MapperRom::new(164).prg(16).chr(0).battery().load();

    // 2 KiB mirrored four times over the 8 KiB window: the same byte at all four aliases.
    core.write_memory(0x6001, 0x3A);
    for address in [0x6001u16, 0x6801, 0x7001, 0x7801] {
        assert_eq!(0x3A, core.read_memory(address), "{address:#06X}");
    }
    core.write_memory(0x7801, 0x4B);
    assert_eq!(0x4B, core.read_memory(0x6001));

    // The EEPROM is not CPU-addressable: it is behind the volatile chip in the same vector, which
    // is where REQ-CORE-095's battery slice is, and no address of $6000-$7FFF reaches it.
    microwire::extended(&mut core, microwire::EWEN);
    microwire::write_all(&mut core, 0xE7);
    for address in [0x6001u16, 0x6801, 0x7001, 0x7801] {
        assert_eq!(0x4B, core.read_memory(address), "{address:#06X}");
    }

    // And the save is the EEPROM's 512 words and not the RAM, which is the page's own division:
    // "PRG-RAM is never battery-backed".
    let saved = core.export_battery_ram().to_vec();
    assert_eq!(512, saved.len());
    assert!(saved.iter().all(|byte| *byte == 0xE7));
}

/// The board has no IRQ, no expansion audio and — unlike the three boards before it in the
/// PPU-address-bus latch — **moves no window** when that bus changes, which is what keeps its
/// `on_ppu_address_bus` to two stored bits (MF-UNL-10 step 2).
#[cfg(feature = "board-164")]
#[test]
fn req_core_075_mapper_164_has_no_irq_and_moves_no_window_on_the_ppu_bus() {
    let mut core = MapperRom::new(164).prg(16).chr(0).load();
    {
        let cartridge = core.cartridge().unwrap();
        assert_eq!(None, cartridge.expansion_audio_output());
        assert_eq!(Some(i64::MAX), cartridge.ppu_irq_horizon());
        assert!(!cartridge.irq_asserted());
    }

    // Eight distinct bytes, one per 1 KiB CHR window, so that a window that did move would show
    // it. 1 bpp is **off**, which is the configuration in which `chr_windows` reads the windows
    // and not the substitution.
    for slot in 0u16..8 {
        chr_write(&mut core, slot * 0x0400, 0xA0 | slot as u8);
    }
    let windows = (core.prg_windows(), core.chr_windows());

    // A whole rendered line's worth of addresses across the latch: 32 nametable fetches, each
    // with its pattern fetches behind it.
    for tile in 0u16..32 {
        let cartridge = core.cartridge_mut().unwrap();
        cartridge.on_ppu_address_bus(0x2000 | tile, i64::from(tile) * 8);
        cartridge.on_ppu_address_bus(0x1000 | (tile << 4), i64::from(tile) * 8 + 4);
    }
    assert_eq!(
        windows,
        (core.prg_windows(), core.chr_windows()),
        "neither the PRG nor the CHR windows follow the PPU address bus"
    );
    assert!(!core.cartridge().unwrap().irq_asserted());

    for _ in 0..10 {
        core.run_frame(0);
    }
    assert!(!core.cartridge().unwrap().irq_asserted());
}

// ------------------------------------------- the Sugar Softec UNL-8237 / UNL-8237A (215),
// ------------------------------------------- lot MF-UNL-11 step 1

/// nesdev ("INES Mapper 215"), *MMC3-compatible registers*: the whole of `$8000-$FFFF`'s
/// behaviour is two tables indexed by `$5007`, an **address** permutation and a permutation of
/// the low three **data** bits of the register that means the MMC3's `$8000`.
///
/// §7.2 of `docs/review/UNL-census.md` asks for "each of the eight `$5007` patterns produces its
/// documented bank layout — eight cases, one per pattern, which is the only way to prove the table
/// is transcribed right", and this is that test: per mode it programs **all eight** MMC3 bank
/// registers through the scrambled index/data pair and asks for the whole twelve-window layout.
/// So each case pins two columns of the address table and all eight entries of the data row, and a
/// third column with the mirroring register.
///
/// The index values below are the **inverse** of the page's data row — the page gives
/// written → meant and a test has to go the other way — and two of them are a cross-check on the
/// transcription: mode 1's inverse is `[0, 3, 1, 5, 6, 7, 2, 4]`, which is mapper 114 submapper
/// 0's own `INDEX_SCRAMBLE` row, and mode 4's row is mapper 114 submapper 1's. The same chip
/// family and the same two permutations, arrived at from two different pages.
#[cfg(feature = "board-215")]
#[test]
fn req_core_102_mapper_215_runs_each_of_the_eight_5007_patterns_over_the_whole_bank_table() {
    // (mode, the written address that means the MMC3's $8000, the one that means its $8001,
    //  the one that means its $A000, the eight values that select bank registers 0-7)
    let patterns: [(u8, u16, u16, u16, [u8; 8]); 8] = [
        (0, 0x8000, 0x8001, 0xA000, [0, 1, 2, 3, 4, 5, 6, 7]),
        (1, 0xA000, 0xC000, 0x8001, [0, 3, 1, 5, 6, 7, 2, 4]),
        (2, 0x8000, 0x8001, 0xA000, [0, 3, 5, 7, 2, 1, 6, 4]),
        (3, 0x8001, 0xA000, 0xA001, [0, 7, 5, 2, 6, 4, 1, 3]),
        (4, 0xA000, 0x8001, 0xC000, [0, 5, 1, 3, 7, 2, 4, 6]),
        (5, 0x8000, 0x8001, 0xA000, [0, 1, 2, 3, 4, 5, 6, 7]),
        (6, 0x8000, 0x8001, 0xA000, [0, 1, 2, 3, 4, 5, 6, 7]),
        (7, 0x8000, 0x8001, 0xA000, [0, 1, 2, 3, 4, 5, 6, 7]),
    ];
    // R0-R7, picked so that every one of the twelve windows shows a distinct bank: R0 and R1 are
    // the 2 KiB registers, whose low bit the chip ignores.
    let values: [u8; 8] = [0x10, 0x12, 0x04, 0x05, 0x06, 0x07, 0x09, 0x0B];

    for (mode, port_index, port_data, port_mirroring, index_values) in patterns {
        let mut core = MapperRom::new(215).prg(64).chr(256).load();
        // $5001 powers up at $xF, which on a 512 KiB image is an outer bank that is not there;
        // the outer lines have their own tests and this one wants the chip's own numbers.
        core.mapper_write(0x5001, 0x00);
        core.mapper_write(0x5007, mode);

        for register in 0..8usize {
            core.mapper_write(port_index, index_values[register]);
            core.mapper_write(port_data, values[register]);
        }

        assert_eq!(
            [0x09, 0x0B, 30, 31],
            core.prg_windows(),
            "$5007 = {mode}: R6 at $8000, R7 at $A000, the region's last two fixed"
        );
        assert_eq!(
            [0x10, 0x11, 0x12, 0x13, 0x04, 0x05, 0x06, 0x07],
            core.chr_windows(),
            "$5007 = {mode}: R0 and R1 over the 2 KiB halves, R2-R5 over the 1 KiB quarters"
        );

        // The third column each row names: the written address that means $A000, the one
        // register of the eight whose effect is not a bank.
        core.mapper_write(port_mirroring, 0x01);
        assert_eq!(Nm::Horizontal, core.mirroring(), "$5007 = {mode}: $A000");
        core.mapper_write(port_mirroring, 0x00);
        assert_eq!(
            Nm::Vertical,
            core.mirroring(),
            "$5007 = {mode}: $A000 again"
        );
    }
}

/// The three registers are at `$5000`, `$5001` and `$5007` under **mask `$F007`**, so they repeat
/// every eight bytes through the whole of `$5000-$5FFF` and `$5002`-`$5006` are not decoded.
#[cfg(feature = "board-215")]
#[test]
fn req_core_102_mapper_215_decodes_5000_5001_and_5007_under_mask_f007() {
    let mut core = MapperRom::new(215).prg(64).chr(256).load();
    core.mapper_write(0x5001, 0x00);
    let plain = core.prg_windows();
    assert_eq!([0, 1, 30, 31], plain, "the MMC3's own power-on layout");

    // $5FF8 is $5000: the mask reads A15-A12 and A2-A0 and nothing between.
    core.mapper_write(0x5FF8, 0x80 | 0x05);
    assert_eq!(
        [10, 11, 10, 11],
        core.prg_windows(),
        "the NROM-128 override through the last alias of $5000"
    );

    // $5002-$5006 are five addresses the mask names and the page does not.
    for address in [0x5002u16, 0x5003, 0x5004, 0x5005, 0x5006] {
        core.mapper_write(address, 0xFF);
    }
    assert_eq!(
        [10, 11, 10, 11],
        core.prg_windows(),
        "$5002-$5006 are not decoded"
    );

    core.mapper_write(0x5FF0, 0x00);
    assert_eq!(plain, core.prg_windows(), "and back to the MMC3's banking");
}

/// nesdev ("INES Mapper 215"), `$5000`: bit 7 "Ignore PRG bank from MMC3; apply bits 0-3/5",
/// bits 0-3 "Select 16 KiB PRG-ROM bank at CPU `$8000-$BFFF` and `$C000-$FFFF`" and bit 5
/// "0: Do not replace bit 0 (NROM-128) / 1: Replace bit 0 with CPU A14 (NROM-256)".
///
/// The same override, and the same two sentences, as mappers 114, 115, 123 and 187.
#[cfg(feature = "board-215")]
#[test]
fn req_core_102_mapper_215_overrides_the_prg_banking_with_nrom_128_and_nrom_256() {
    let mut core = MapperRom::new(215).prg(64).chr(256).load();
    core.mapper_write(0x5001, 0x00);
    core.mmc3_bank(6, 0x03);
    core.mmc3_bank(7, 0x04);
    assert_eq!([3, 4, 30, 31], core.prg_windows());

    // NROM-128: the same 16 KiB bank 5 at both windows, so banks 10 and 11 twice.
    core.mapper_write(0x5000, 0x80 | 0x05);
    assert_eq!([10, 11, 10, 11], core.prg_windows());

    // NROM-256: bit 0 replaced by CPU A14, so one 32 KiB view out of the even/odd pair 4/5.
    core.mapper_write(0x5000, 0x80 | 0x20 | 0x05);
    assert_eq!([8, 9, 10, 11], core.prg_windows());

    // "0: Use PRG bank from MMC3; ignore bits 0-3/5" -- and the chip's registers were never
    // touched, so they come straight back.
    core.mapper_write(0x5000, 0x20 | 0x05);
    assert_eq!([3, 4, 30, 31], core.prg_windows());
}

/// `$5001`'s outer banks on **submapper 0**, the UNL-8237: `..cp CCPP`, "Select 256 KiB Outer
/// PRG-ROM bank (PRG A18/A19)" at bits 1-0 and "Select 256 KiB Outer CHR-ROM bank (CHR A18/A19)"
/// at bits 3-2.
///
/// 1.5 MiB of each, so that all four outer banks of both halves are inside the image and none of
/// them wraps: the numbered banks only carry eight bits, so the CHR half is read through
/// `chr_peek_view`'s offsets as mappers 114 and 115 already are.
#[cfg(feature = "board-215")]
#[test]
fn req_core_102_mapper_215_puts_the_outer_256_kib_banks_above_the_mmc3_on_the_unl_8237() {
    let mut core = MapperRom::new(215).prg(192).chr(1536).load();
    core.mmc3_bank(6, 0x01);
    core.mmc3_bank(2, 0x05);

    for outer in 0..4u8 {
        core.mapper_write(0x5001, outer);
        assert_eq!(
            1 + outer * 32,
            core.prg_windows()[0],
            "$5001 bits 1-0 = {outer}: PRG A18/A19, 32 banks of 8 KiB each"
        );
    }

    for outer in 0..4u8 {
        core.mapper_write(0x5001, outer << 2);
        assert_eq!(
            Some((5 + usize::from(outer) * 256) * 1024),
            core.cartridge()
                .unwrap()
                .chr_peek_view()
                .map(|view| view.offsets()[4]),
            "$5001 bits 3-2 = {outer}: CHR A18/A19, 256 banks of 1 KiB each"
        );
    }
}

/// `$5001`'s outer banks on **submapper 1**, the UNL-8237A: the page's diagram is two bit rows
/// overlaid, `P.PP` over `CCC.`, and this test is what pins the reading **Q-171** records —
/// PRG A20/A19/A18 at bits 3/1/0 and CHR A20/A19/A18 at bits 3/2/1.
///
/// Bits 0 and 2 are what discriminate it: **bit 0 moves PRG and not CHR**, which the alternative
/// reading (CHR at bits 2/1/0) could not produce, and **bit 2 moves CHR and not PRG**. Bit 1
/// moves both, one line apart, and bit 3 is A20 on both — 1.5 MiB of each is what puts all three
/// of those lines inside the image, so every one of the six assertions below is a real read and
/// not a wrap.
#[cfg(feature = "board-215")]
#[test]
fn req_core_102_mapper_215_reads_5001_the_other_way_on_the_unl_8237a() {
    let mut core = MapperRom::new(215).submapper(1).prg(192).chr(1536).load();
    core.mmc3_bank(6, 0x01);
    core.mmc3_bank(2, 0x05);

    let chr = |core: &NesCore| {
        core.cartridge()
            .unwrap()
            .chr_peek_view()
            .map(|view| view.offsets()[4])
    };

    // (the value written to $5001, the PRG outer bank, the CHR outer bank)
    for (written, prg_outer, chr_outer) in [
        (0x00u8, 0u8, 0usize),
        (0x01, 1, 0), // bit 0: PRG A18 only.
        (0x02, 2, 1), // bit 1: PRG A19 and CHR A18.
        (0x04, 0, 2), // bit 2: CHR A19 only.
        (0x08, 4, 4), // bit 3: A20 on both; only the PRG half is inside the image.
        (0x03, 3, 1),
    ] {
        core.mapper_write(0x5001, written);
        assert_eq!(
            1 + prg_outer * 32,
            core.prg_windows()[0],
            "$5001 = {written:#04X}: PRG outer bank {prg_outer}"
        );
        assert_eq!(
            Some((5 + chr_outer * 256) * 1024),
            chr(&core),
            "$5001 = {written:#04X}: CHR outer bank {chr_outer}"
        );
    }
}

/// nesdev ("INES Mapper 215"), `$5000` bit 6: "0: Do not replace PRG A17 and CHR A17 from the
/// MMC3 (256 KiB outer bank) / 1: Replace PRG A17 and CHR A17 with `$5001` bits 4 and 5,
/// respectively (128 KiB outer bank)".
///
/// So the bit takes one line away from the chip on each table and hands it to `$5001`: the
/// register value that carries the line is the same before and after, and what changes is where
/// the line comes from.
#[cfg(feature = "board-215")]
#[test]
fn req_core_102_mapper_215_replaces_prg_a17_and_chr_a17_from_5001_under_5000_bit_6() {
    let mut core = MapperRom::new(215).prg(64).chr(256).load();
    core.mapper_write(0x5001, 0x00);
    // Bank 17 is PRG A17 set with A16-A13 holding 1; bank $85 is CHR A17 set with $05 below it.
    core.mmc3_bank(6, 0x11);
    core.mmc3_bank(2, 0x85);
    assert_eq!(0x11, core.prg_windows()[0], "A17 from the MMC3");
    assert_eq!(0x85, core.chr_windows()[4], "and CHR A17 from the MMC3");

    // Bit 6 set, $5001 bits 4 and 5 clear: both lines are now zero whatever the chip holds.
    core.mapper_write(0x5000, 0x40);
    assert_eq!(
        0x01,
        core.prg_windows()[0],
        "PRG A17 replaced by $5001 bit 4"
    );
    assert_eq!(
        0x05,
        core.chr_windows()[4],
        "CHR A17 replaced by $5001 bit 5"
    );

    // And each line on its own, which is the "respectively" of the page's sentence.
    core.mapper_write(0x5001, 0x10);
    assert_eq!([0x11, 0x05], [core.prg_windows()[0], core.chr_windows()[4]]);
    core.mapper_write(0x5001, 0x20);
    assert_eq!([0x01, 0x85], [core.prg_windows()[0], core.chr_windows()[4]]);
    core.mapper_write(0x5001, 0x30);
    assert_eq!([0x11, 0x85], [core.prg_windows()[0], core.chr_windows()[4]]);

    // Bit 6 clear again: the chip drives both lines and `$5001` bits 4-5 reach nothing.
    core.mapper_write(0x5000, 0x00);
    assert_eq!([0x11, 0x85], [core.prg_windows()[0], core.chr_windows()[4]]);
    core.mapper_write(0x5001, 0x00);
    assert_eq!([0x11, 0x85], [core.prg_windows()[0], core.chr_windows()[4]]);
}

/// nesdev ("INES Mapper 215"), *Note*: "The Outer Bank Register is reset to its power-up value if
/// a Reset has been detected via an interruption of the M2 signal. This makes it difficult to dump
/// these cartridges using a Kazzo device."
///
/// The power-up value is **`$xF`** — the low nibble fixed, the high one unknown, which **Q-171**
/// records — so what a reset must restore is an outer bank of 3 on both halves of submapper 0,
/// and `$5000` and `$5007` with it.
#[cfg(feature = "board-215")]
#[test]
fn amd_061_mapper_215_resets_the_outer_bank_register_to_its_power_up_value() {
    let core = MapperRom::new(215).prg(192).chr(1536).load();
    let power_on = (core.prg_windows(), core.chr_windows(), core.mirroring());
    // $xF on a 1.5 MiB image is the fourth 256 KiB bank of each half, which is a layout the
    // chip's own zeroed registers could not produce.
    assert_eq!(
        [96, 97, 126, 127],
        power_on.0,
        "the outer bank is 3 out of reset, over the MMC3's own [0, 1, -2, -1]"
    );

    let mut core = core;
    core.mapper_write(0x5007, 0x03);
    core.mapper_write(0x5001, 0x00);
    core.mapper_write(0x5000, 0x80 | 0x20 | 0x05);
    core.mapper_write(0x8001, 0x06); // $5007 = 3: the written $8001 means the MMC3's $8000.
    core.mapper_write(0xA000, 0x07); // and the written $A000 its $8001.
    assert_ne!(
        power_on,
        (core.prg_windows(), core.chr_windows(), core.mirroring())
    );

    core.reset();
    assert_eq!(
        power_on,
        (core.prg_windows(), core.chr_windows(), core.mirroring()),
        "a reset gives the power-on layout back, $5001 included"
    );
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// `registry::board_memory_sizes` has **no arm** for this number, which is the same choice, and
/// the same reason, as mappers 121, 123 and 187: every register of the board is **below `$6000`**,
/// so nothing forces the question and the header decides (mapper 97's precedent in that
/// function).
///
/// What that has to mean on the bus is this test: a battery-backed image gets its 8 KiB at
/// `$6000-$7FFF` and the board does not decode a byte of it.
#[cfg(feature = "board-215")]
#[test]
fn req_core_095_mapper_215_leaves_6000_7fff_to_the_header_and_decodes_nothing_there() {
    let mut core = MapperRom::new(215).prg(64).chr(256).battery().load();
    core.mapper_write(0x5001, 0x00);
    core.mmc3_bank(6, 0x03);
    // The MMC3's own $A001: chip enable and write enable.
    core.mapper_write(0xA001, 0x80);
    let windows = (core.prg_windows(), core.chr_windows());

    for (address, value) in [(0x6000u16, 0x11u8), (0x6800, 0x22), (0x7FFF, 0x33)] {
        core.write_memory(address, value);
        assert_eq!(
            value,
            core.read_memory(address),
            "{address:#06X} is PRG-RAM and the board does not decode it"
        );
    }

    assert_eq!(
        windows,
        (core.prg_windows(), core.chr_windows()),
        "and no write below $8000 outside $5000-$5FFF moved a window"
    );
}

// ------------------------------------------- the Gouder board of the Street Fighter IV
// ------------------------------------------- backport (208), lot MF-UNL-11 step 2

/// The page's 256-byte table, rebuilt from the **structure** it has rather than copied out of
/// `Mmc3Gouder208::PROTECTION_TABLE` — which is the point: a transcription check wants two
/// independent encodings of the same data to agree.
///
/// The structure is four groups of four rows; within a group the even rows share one layout and
/// the odd rows another, and within a row the live half is one four-byte cycle written twice. So
/// eight four-byte cycles and the half each group fills are the whole of it, against 256
/// literals on the board's side.
#[cfg(feature = "board-208")]
fn mapper_208_protection_table() -> [u8; 256] {
    // (the live half is the low eight columns, the four-byte cycle for even rows, for odd rows)
    let groups: [(bool, [u8; 4], [u8; 4]); 4] = [
        // Rows $0-$3 and $8-$B: columns 0-7 are $59 and the cycle is in columns 8-15.
        (false, [0x59, 0x49, 0x19, 0x09], [0x51, 0x41, 0x11, 0x01]),
        // Rows $4-$7 and $C-$F: the cycle is in columns 0-7 and columns 8-15 are $00.
        (true, [0x00, 0x10, 0x40, 0x50], [0x08, 0x18, 0x48, 0x58]),
        (false, [0x58, 0x48, 0x18, 0x08], [0x50, 0x40, 0x10, 0x00]),
        (true, [0x01, 0x11, 0x41, 0x51], [0x09, 0x19, 0x49, 0x59]),
    ];

    std::array::from_fn(|index| {
        let (row, column) = (index >> 4, index & 0x0F);
        let (cycle_low, even, odd) = groups[row >> 2];
        let cycle = if row & 1 == 0 { even } else { odd };
        if (column < 8) == cycle_low {
            cycle[column & 0x03]
        } else if cycle_low {
            0x00
        } else {
            0x59
        }
    })
}

/// nesdev ("INES Mapper 208"): the **Protection Table Index** at `$5000-$57FF` "specifies the
/// index into the 256-byte table that will be used for the next write to `$5800-$5FFF`", and a
/// write to `$5800-$5FFF` stores "the CPU data `XORed` with the table entry indexed by the last
/// write to `$5000-$57FF`", readable back from the same window.
///
/// §7.2 of `docs/review/UNL-census.md` asks for "write an index, write data, read back and assert
/// the XOR against the table". This walks **all 256 indices**, which is what the census's "the
/// page's 256-byte table … goes in the source as a `const` array with the page cited" has to mean
/// if the array is to be worth anything: the whole of it is read back through the mechanism.
///
/// §7.2's second half — "a wrong code leaves the board in the state the page describes" — is
/// **not** on the page, which describes no wrong-code behaviour at all (**Q-171**). What is on
/// the page is the mechanism, and the complement of that bullet is the last block below: the same
/// datum under a *different* index stores a different byte, which is the whole of what a wrong
/// code does.
#[cfg(feature = "board-208")]
#[test]
fn req_core_102_mapper_208_xors_a_write_against_the_pages_own_table() {
    let mut core = MapperRom::new(208).prg(16).chr(32).load();
    let table = mapper_208_protection_table();

    // A datum of zero stores the table entry itself, so this reads the whole array out through
    // the board's own path.
    for (index, &entry) in table.iter().enumerate() {
        core.mapper_write(0x5000, index as u8);
        core.mapper_write(0x5800, 0x00);
        assert_eq!(
            entry,
            core.read_memory(0x5800),
            "index {index:#04X}: the entry itself, XORed with a datum of zero"
        );
    }

    // And the XOR is a XOR: $FF complements the entry, and writing the entry stores zero.
    for index in [0x00usize, 0x09, 0x1B, 0x40, 0x55, 0x8A, 0xC3, 0xFF] {
        core.mapper_write(0x5000, index as u8);
        core.mapper_write(0x5800, 0xFF);
        assert_eq!(table[index] ^ 0xFF, core.read_memory(0x5800));
        core.mapper_write(0x5800, table[index]);
        assert_eq!(0x00, core.read_memory(0x5800));
    }

    // The same datum under two indices whose entries differ stores two different bytes, which is
    // the whole of what writing the wrong code does.
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5800, 0x5A);
    let right = core.read_memory(0x5800);
    core.mapper_write(0x5000, 0x40);
    core.mapper_write(0x5800, 0x5A);
    assert_ne!(
        right,
        core.read_memory(0x5800),
        "a different index, a different byte"
    );
}

/// "Writes to one of the four protection registers determined by the two lower bits of the
/// address being written to", and a read "determined by the two lower bits of the address being
/// read" — so the window is four registers wide and 2 KiB deep.
#[cfg(feature = "board-208")]
#[test]
fn req_core_102_mapper_208_keeps_four_protection_registers_under_the_two_low_address_bits() {
    let mut core = MapperRom::new(208).prg(16).chr(32).load();
    // One index for all four, so that the four stored bytes differ only by the datum.
    core.mapper_write(0x5000, 0x00);
    for register in 0..4u16 {
        core.mapper_write(0x5800 + register, 0x10 * register as u8);
    }

    let entry = mapper_208_protection_table()[0];
    for register in 0..4u16 {
        assert_eq!(
            entry ^ (0x10 * register as u8),
            core.read_memory(0x5800 + register),
            "register {register}"
        );
    }

    // The two bits are the *only* two: $5FFC-$5FFF is $5800-$5803 again, and $57FF is the index
    // register and not a fifth protection register.
    for register in 0..4u16 {
        assert_eq!(
            entry ^ (0x10 * register as u8),
            core.read_memory(0x5FFC + register),
            "the top of the window aliases register {register}"
        );
    }
}

/// nesdev ("INES Mapper 208"), *PRG Bank and Mirroring* (`$4800-$4FFF`, `$6800-$6FFF`): bits 4
/// and 0 "Select 32KB page of PRG-ROM at `$8000`" and bit 5 the mirroring, "0: Vertical,
/// 1: Horizontal". "The initial value of this register is `$11`" — page 3, vertical.
///
/// The chip's `$A000` is inert, which is **Q-171**: the page's register list has no `$A000`
/// section and puts mirroring among the board's own additional registers.
#[cfg(feature = "board-208")]
#[test]
fn req_core_102_mapper_208_banks_32_kib_and_drives_the_mirroring_from_4800_and_6800() {
    let mut core = MapperRom::new(208).prg(16).chr(32).load();
    assert_eq!(
        ([12, 13, 14, 15], Nm::Vertical),
        (core.prg_windows(), core.mirroring()),
        "the initial value of the register is $11: page 3, vertical"
    );

    // Bit 4 above bit 0, which is the diagram's one connector joining the two.
    for (written, page) in [(0x00u8, 0u8), (0x01, 1), (0x10, 2), (0x11, 3)] {
        core.mapper_write(0x4800, written);
        assert_eq!(
            [4 * page, 4 * page + 1, 4 * page + 2, 4 * page + 3],
            core.prg_windows(),
            "$4800 = {written:#04X}: 32 KiB page {page}"
        );
        // The same register answers in the other window the page gives it.
        core.mapper_write(0x4800, 0x00);
        core.mapper_write(0x6800, written);
        assert_eq!(
            [4 * page, 4 * page + 1, 4 * page + 2, 4 * page + 3],
            core.prg_windows(),
            "$6800 = {written:#04X}: the same register"
        );
    }

    core.mapper_write(0x4800, 0x20);
    assert_eq!(Nm::Horizontal, core.mirroring());
    core.mapper_write(0x6800, 0x00);
    assert_eq!(Nm::Vertical, core.mirroring());

    // The chip's own mirroring register drives nothing (Q-171).
    core.mapper_write(0xA000, 0x01);
    assert_eq!(
        Nm::Vertical,
        core.mirroring(),
        "$A000 is inert on this board"
    );

    // And the mask is A14, A12 and A11 and not A13: $4800 and $6800 match, $5800 and $7800 do
    // not, and neither does $4000.
    let windows = core.prg_windows();
    for address in [0x4000u16, 0x5800, 0x7800, 0x6000] {
        core.mapper_write(address, 0x11);
        assert_eq!(
            windows,
            core.prg_windows(),
            "{address:#06X} is not the bank register"
        );
    }
}

/// The chip's own half: "It uses MMC3-compatible registers for CHR ROM bankswitching and IRQ
/// control", the *Banks* section listing the two 2 KiB and four 1 KiB windows "(same as MMC3)"
/// with their swap — and no PRG window at all, the board's 32 KiB page covering `$8000-$FFFF`.
#[cfg(feature = "board-208")]
#[test]
fn req_core_102_mapper_208_keeps_the_mmc3s_chr_banking_and_loses_its_prg_banking() {
    let mut core = MapperRom::new(208).prg(16).chr(32).load();
    core.mapper_write(0x4800, 0x00);
    for register in 0..6u8 {
        core.mmc3_bank(register, 0x04 + register * 2);
    }
    assert_eq!(
        [0x04, 0x05, 0x06, 0x07, 0x08, 0x0A, 0x0C, 0x0E],
        core.chr_windows(),
        "R0 and R1 over the 2 KiB halves, R2-R5 over the 1 KiB quarters"
    );

    // $8000 bit 7 swaps the halves, exactly as on the chip.
    core.mapper_write(0x8000, 0x80);
    assert_eq!(
        [0x08, 0x0A, 0x0C, 0x0E, 0x04, 0x05, 0x06, 0x07],
        core.chr_windows()
    );

    // R6 and R7 reach nothing: the board banks 32 KiB at a time.
    let windows = core.prg_windows();
    core.mmc3_bank(6, 0x07);
    core.mmc3_bank(7, 0x03);
    assert_eq!(
        windows,
        core.prg_windows(),
        "the chip's PRG lines are not connected"
    );
}

/// Submapper 1, the copyright-stripped hack, verbatim: "There are no additional registers, even
/// as the ROM image retains some leftovers of code writing to `$5800`. Instead of switching PRG
/// ROM banks MMC3-style, there is only one 32k PRG ROM bank at CPU `$8000`. Its value is
/// determined by MMC3 bank register 6; the actual bank number is the bank register value SHR 2.
/// CHR ROM bankswitching, mirroring and IRQ control are performed using the normal MMC3
/// registers."
///
/// So all four of the board's ports are dead there, `$5800` included — the leftover writes the
/// page mentions must reach nothing and the window must read open bus, which this core answers as
/// zero — and the mirroring comes back to the chip.
#[cfg(feature = "board-208")]
#[test]
fn req_core_102_mapper_208_submapper_1_takes_its_one_page_from_the_mmc3s_r6() {
    let mut core = MapperRom::new(208).submapper(1).prg(16).chr(32).load();
    // R6 powers on at 0, so page 0; the chip's own R7 of 1 reaches nothing here.
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // "The actual bank number is the bank register value SHR 2": an 8 KiB bank number read as a
    // 32 KiB one.
    for (register_value, page) in [(0x00u8, 0u8), (0x04, 1), (0x0B, 2), (0x0F, 3)] {
        core.mmc3_bank(6, register_value);
        assert_eq!(
            [4 * page, 4 * page + 1, 4 * page + 2, 4 * page + 3],
            core.prg_windows(),
            "R6 = {register_value:#04X}: page {page}"
        );
    }

    // "Mirroring … performed using the normal MMC3 registers", so $A000 is live here and the
    // board's own register is not.
    core.mapper_write(0xA000, 0x01);
    assert_eq!(Nm::Horizontal, core.mirroring());
    let windows = core.prg_windows();
    core.mapper_write(0x4800, 0x11);
    core.mapper_write(0x6800, 0x11);
    assert_eq!(
        (windows, Nm::Horizontal),
        (core.prg_windows(), core.mirroring()),
        "the board's own bank and mirroring register does not exist on submapper 1"
    );

    // The $5800 leftovers reach nothing and read open bus.
    core.mapper_write(0x5000, 0x00);
    core.mapper_write(0x5800, 0x5A);
    assert_eq!(
        0x00,
        core.read_memory(0x5800),
        "no protection register here"
    );
}

/// `registry::board_memory_sizes` gives **submapper 0 no PRG-RAM**, because its `$6800-$6FFF`
/// register *is* the board's decode of that window — the same arm, and the same reason, as
/// mappers 114, 115 and 182. `Cartridge::cpu_write` returns at `$6000-$7FFF` whenever the
/// cartridge has any PRG-RAM, so the register answering there at all **is** the assertion.
///
/// Submapper 1 is excluded from that arm: it has no such register, so the header decides, which
/// is mapper 97's precedent in that function.
#[cfg(feature = "board-208")]
#[test]
fn req_core_095_mapper_208_gives_submapper_0_no_prg_ram_and_leaves_submapper_1_the_headers() {
    // Submapper 0, with a battery header that would otherwise put 8 KiB at $6000-$7FFF.
    let mut core = MapperRom::new(208).prg(16).chr(32).battery().load();
    core.mapper_write(0xA001, 0x80); // the chip's own enable, so nothing else gates the window
    core.mapper_write(0x6800, 0x00);
    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "the $6800 register answers, so there is no RAM in front of it"
    );
    core.write_memory(0x6000, 0x42);
    assert_eq!(0x00, core.read_memory(0x6000), "and the window is open bus");

    // Submapper 1 has no register there, so the header's 8 KiB is where it would be.
    let mut core = MapperRom::new(208)
        .submapper(1)
        .prg(16)
        .chr(32)
        .battery()
        .load();
    core.mapper_write(0xA001, 0x80);
    let windows = core.prg_windows();
    for address in [0x6000u16, 0x6800, 0x7FFF] {
        core.write_memory(address, 0x33);
        assert_eq!(0x33, core.read_memory(address), "{address:#06X} is PRG-RAM");
    }
    assert_eq!(windows, core.prg_windows(), "and no window moved");
}

/// nesdev ("INES Mapper 208"): the four IRQ ports at `$C000-$FFFF` are "identical to the
/// respective MMC3 registers", so the board's counter is the chip's — scanline for scanline, and
/// **revision B**, this build's default, the page naming no NEC or MMC3A behaviour.
#[cfg(feature = "board-208")]
#[test]
fn req_core_114_mapper_208_keeps_the_mmc3s_scanline_counter() {
    for submapper in [0u8, 1] {
        let mut core = MapperRom::new(208)
            .submapper(submapper)
            .prg(16)
            .chr(32)
            .load();
        core.mapper_write(0xC000, 0x04);
        core.mapper_write(0xC001, 0x00);
        core.mapper_write(0xE001, 0x00);
        assert!(!core.cartridge().unwrap().irq_asserted());

        // Four rises of PPU A12 past the reload, which is the chip's own condition.
        for clock in 0..8i64 {
            let cartridge = core.cartridge_mut().unwrap();
            cartridge.on_ppu_address_bus(0x0000, clock * 200);
            cartridge.on_ppu_address_bus(0x1000, clock * 200 + 100);
        }
        assert!(
            core.cartridge().unwrap().irq_asserted(),
            "submapper {submapper}: the chip's counter reaches zero"
        );

        core.mapper_write(0xE000, 0x00);
        assert!(!core.cartridge().unwrap().irq_asserted());
    }
}

// ------------------------------------------- the Kasheng A9461 (219), lot MF-UNL-11 step 3

/// nesdev ("INES Mapper 219"), *Extended PRG-ROM Bank Index (`$8000`, `$23-$26`)/Data (`$8001`)*:
/// the index names one of four 8 KiB windows — "6: `$8000-$9FFF`, 5: `$A000-$BFFF`,
/// 4: `$C000-$DFFF`, 3: `$E000-$FFFF`" — and the datum is `..BB BB..`, "bits 0-3 (**reversed
/// order**) of 8 KiB PRG-ROM bank number".
///
/// §7.2 of `docs/review/UNL-census.md` asks for "with `$8002` bit 5 set, the four PRG windows are
/// independent; with it clear, the board is an MMC3". Both halves are here, and the four banks
/// chosen are **1, 2, 4 and 8** — one bit each — because that is what pins the reversal: a bank of
/// 1 is written as `$20`, the value's *top* live bit, and a bank of 8 as `$04`, its bottom one.
#[cfg(feature = "board-219")]
#[test]
fn req_core_102_mapper_219_banks_four_independent_prg_windows_in_extended_mode() {
    let mut core = MapperRom::new(219).prg(16).chr(32).load();
    assert_eq!(
        [0, 1, 14, 15],
        core.prg_windows(),
        "the MMC3's own power-on layout, masked to 128 KiB"
    );

    // $8002 bit 5 opens the extended mode, and nothing of the mode's own registers has been
    // written yet, so all four windows are bank 0.
    core.mapper_write(0x8002, 0x20);
    assert_eq!([0, 0, 0, 0], core.prg_windows());

    // (the index, the window it names, the datum, the bank that datum reverses to)
    for (index, slot, datum, bank) in [
        (0x26u8, 0usize, 0x20u8, 1u8),
        (0x25, 1, 0x10, 2),
        (0x24, 2, 0x08, 4),
        (0x23, 3, 0x04, 8),
    ] {
        core.mapper_write(0x8000, index);
        core.mapper_write(0x8001, datum);
        assert_eq!(
            bank,
            core.prg_windows()[slot],
            "index {index:#04X}: datum {datum:#04X} is bank {bank} at window {slot}"
        );
    }
    assert_eq!(
        [1, 2, 4, 8],
        core.prg_windows(),
        "and the four windows are independent"
    );

    // "with it clear, the board is an MMC3": the chip kept its own registers all along, because
    // the odd writes above never reached it.
    core.mapper_write(0x8002, 0x00);
    assert_eq!([0, 1, 14, 15], core.prg_windows());

    // And the extended registers survive the trip, which is what makes them registers.
    core.mapper_write(0x8002, 0x20);
    assert_eq!([1, 2, 4, 8], core.prg_windows());
}

/// "MMC3 Bank Register (`$8000`/`$8001`) … Used when `$8002` bit 5=0. … PRG-ROM and CHR-ROM banks
/// masked to **128 KiB**", with the two bits of `$5002`/`$5003` above that mask: "bit 0 / bit 1 of
/// 128 KiB outer PRG-ROM and CHR-ROM bank".
///
/// So in MMC3 mode the chip drives PRG A13-A16 and CHR A10-A16 and the outer register drives
/// A17-A18 of both — which this test shows by giving the chip bank numbers whose top bit the mask
/// drops and then handing that bit back from `$5002`.
#[cfg(feature = "board-219")]
#[test]
fn req_core_102_mapper_219_is_an_mmc3_masked_to_128_kib_under_its_outer_bank() {
    // 512 KiB of each, so that all four outer banks of both halves are inside the image.
    let mut core = MapperRom::new(219).prg(64).chr(512).load();
    core.mmc3_bank(6, 0x13);
    core.mmc3_bank(7, 0x05);
    core.mmc3_bank(2, 0x85);
    assert_eq!(
        [0x03, 0x05, 0x0E, 0x0F],
        core.prg_windows(),
        "PRG A17 is not the chip's: $13 is bank 3 and the fixed windows are 14 and 15"
    );
    assert_eq!(
        0x05,
        core.chr_windows()[4],
        "and CHR A17 is not the chip's either: $85 is bank 5"
    );

    // (the two writes, the outer bank they leave, the PRG window it puts bank 3 in)
    // $5002 is bit 0, `.... ...B`; $5003 is bit 1 and it is **bit 5** of the datum, `..B. ....`.
    let chr = |core: &NesCore| {
        core.cartridge()
            .unwrap()
            .chr_peek_view()
            .map(|view| view.offsets()[4])
    };
    for (bit0, bit1, outer) in [
        (0x01u8, 0x00u8, 1u8),
        (0x01, 0x20, 3),
        (0x00, 0x20, 2),
        (0x00, 0x00, 0),
    ] {
        core.mapper_write(0x5002, bit0);
        core.mapper_write(0x5003, bit1);
        // 16 PRG banks of 8 KiB per outer bank, and 128 CHR banks of 1 KiB.
        assert_eq!(
            [
                0x03 + outer * 0x10,
                0x05 + outer * 0x10,
                0x0E + outer * 0x10,
                0x0F + outer * 0x10
            ],
            core.prg_windows(),
            "outer bank {outer}: PRG A17-A18"
        );
        assert_eq!(
            Some((5 + usize::from(outer) * 128) * 1024),
            chr(&core),
            "outer bank {outer}: CHR A17-A18"
        );
    }

    // The page's "The Outer Bank register must be initialized to $03 for the menu to appear" is a
    // statement about the menu's own code, so the board powers on at zero and a reset says so.
    core.reset();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
}

/// The three CHR sections of the extended mode, which are one latch and two switches:
///
/// * *CHR-ROM Bank Number Latch Index (`$8000`, `$08-$1E`, **even**)*, datum `.... .654` — "bits
///   4-6 OR'd with next CHR-ROM switch's data";
/// * *CHR-ROM Pseudo 2 KiB switch Index (`$8000`, `$09-$0F`, **odd**)*, `..00 1RR1`, `RR` picking
///   one of the four quarters of PPU `$0000-$0FFF`, with the datum "OR'd with bit 1 SHR 1 of the
///   value written to `$8000`";
/// * *CHR-ROM 1 KiB switch Index (`$8000`, `$11-$1F`, odd)*, `..01 RR.1`, `RR` picking one of the
///   four quarters of PPU `$1000-$1FFF`.
///
/// **The "pseudo 2 KiB" of that title is the index bit**, and the first block below is what it
/// means: one bank number written to two consecutive quarters comes out as a consecutive *pair*,
/// because the odd quarter's own index bit is bit 0 of its bank. That is a 2 KiB switch built out
/// of two 1 KiB ones, and it is the only place in this board where an address bit reaches a bank
/// number.
#[cfg(feature = "board-219")]
#[test]
fn req_core_102_mapper_219_switches_chr_through_a_latch_and_two_pseudo_switches() {
    let mut core = MapperRom::new(219).prg(16).chr(256).load();
    core.mapper_write(0x8002, 0x20);

    // The pseudo 2 KiB switch: `$09`, `$0B`, `$0D`, `$0F` are the four quarters of $0000-$0FFF,
    // and the same datum -- bank $0A, the field being `BBBB BBB.` -- lands as $0A in the even
    // quarters and $0B in the odd ones.
    for index in [0x09u8, 0x0B, 0x0D, 0x0F] {
        core.mapper_write(0x8000, index);
        core.mapper_write(0x8001, 0x0A << 1);
    }
    assert_eq!(
        [0x0A, 0x0B, 0x0A, 0x0B],
        core.chr_windows()[..4],
        "one bank number over four quarters is two 2 KiB pairs"
    );

    // The 1 KiB switch over $1000-$1FFF: `$11`, `$15`, `$19`, `$1D`, and no index bit reaches
    // the bank here.
    for (index, bank) in [(0x11u8, 0x20u8), (0x15, 0x21), (0x19, 0x22), (0x1D, 0x23)] {
        core.mapper_write(0x8000, index);
        core.mapper_write(0x8001, bank << 1);
    }
    assert_eq!(
        [0x20, 0x21, 0x22, 0x23],
        core.chr_windows()[4..],
        "four independent 1 KiB windows over the second pattern table"
    );

    // The latch supplies bits 4-6 of every switch that follows it, from its own bits 0-2.
    core.mapper_write(0x8000, 0x08);
    core.mapper_write(0x8001, 0x02);
    core.mapper_write(0x8000, 0x11);
    core.mapper_write(0x8001, 0x03 << 1);
    assert_eq!(
        0x23,
        core.chr_windows()[4],
        "bank $03 with the latch's $02 shifted into bits 4-6"
    );

    // `..01 ...0` is the same latch register as `..00 1..0`, which is what the page's "or" means.
    core.mapper_write(0x8000, 0x10);
    core.mapper_write(0x8001, 0x05);
    core.mapper_write(0x8000, 0x11);
    core.mapper_write(0x8001, 0x03 << 1);
    assert_eq!(0x53, core.chr_windows()[4], "$10 is the latch too");

    // It reaches the pseudo 2 KiB switch in the same words.
    core.mapper_write(0x8000, 0x09);
    core.mapper_write(0x8001, 0x04 << 1);
    assert_eq!(0x54, core.chr_windows()[0]);
}

/// **Q-171, the reading the page forces**: its `R RRRR` field is five bits wide and its extended
/// index values run to `$26`, which needs six — so the sixth is bit 5, the mode bit itself. The
/// extended PRG indices `$23-$26` carry that bit **set** and all three CHR index patterns carry
/// it **clear**, yet every one of the four sections says "used only if `$8002` bit 5=1".
///
/// Both statements can only hold if the **mode** is latched by a write to `$8002` and the
/// **index** by any even address of the window, which is what this test asserts from both sides:
/// a `$8000` write with bit 5 set does not open the mode, and a `$8000` write with bit 5 clear
/// does not close it.
#[cfg(feature = "board-219")]
#[test]
fn q_171_mapper_219_latches_the_mode_bit_from_8002_and_the_index_from_every_even_address() {
    let mut core = MapperRom::new(219).prg(16).chr(32).load();

    // A $8000 write carrying bit 5 does not open the extended mode: were it the index alone that
    // decided, this would, and the CHR sections could never be reached.
    core.mapper_write(0x8000, 0x26);
    core.mapper_write(0x8001, 0x20);
    assert_eq!(
        [0, 1, 14, 15],
        core.prg_windows(),
        "$8000 does not latch the mode, so that write was the chip's own R6 index and datum"
    );

    core.mapper_write(0x8002, 0x20);
    core.mapper_write(0x8000, 0x26);
    core.mapper_write(0x8001, 0x20);
    assert_eq!(1, core.prg_windows()[0], "$8002 does latch it");

    // And a $8000 write with bit 5 clear -- which every CHR index is -- does not close it.
    core.mapper_write(0x8000, 0x09);
    core.mapper_write(0x8001, 0x0A << 1);
    assert_eq!(
        (1u8, 0x0Au8),
        (core.prg_windows()[0], core.chr_windows()[0]),
        "a CHR index leaves the mode alone and switches CHR"
    );

    // $8002 with bit 5 clear closes it, and the chip's registers are where they were.
    core.mapper_write(0x8002, 0x00);
    assert_eq!([0, 1, 14, 15], core.prg_windows());
}

/// **Q-171**: the page gives the outer bank register `Mask: $5003`, which taken literally decodes
/// A14, A12, A1 and A0 and **not A15** — so `$D002` and `$D003`, the MMC3's own IRQ latch and
/// reload, would be the outer bank register as well. The window implemented is `$F003`: the two
/// addresses the page's section titles name and their aliases inside `$5000-$5FFF`.
#[cfg(feature = "board-219")]
#[test]
fn q_171_mapper_219_decodes_the_outer_bank_inside_5000_5fff_and_not_at_d002() {
    let mut core = MapperRom::new(219).prg(64).chr(32).load();
    core.mmc3_bank(6, 0x01);
    assert_eq!(0x01, core.prg_windows()[0]);

    // An alias inside the window: $5FFE is $5002 and $5FFF is $5003.
    core.mapper_write(0x5FFE, 0x01);
    assert_eq!(0x11, core.prg_windows()[0]);
    core.mapper_write(0x5FFF, 0x20);
    assert_eq!(0x31, core.prg_windows()[0]);
    core.mapper_write(0x5002, 0x00);
    core.mapper_write(0x5003, 0x00);
    assert_eq!(0x01, core.prg_windows()[0]);

    // $D002 and $D003 are the chip's IRQ latch and reload, and they do not bank anything.
    core.mapper_write(0xD002, 0x01);
    core.mapper_write(0xD003, 0x20);
    assert_eq!(
        0x01,
        core.prg_windows()[0],
        "$D002/$D003 are not the outer bank register"
    );

    // And that pair really did reach the counter: $D002 is a latch of 1, $D003 reloads, and the
    // IRQ fires on the rise after it.
    core.mapper_write(0xE001, 0x00);
    for clock in 0..4i64 {
        let cartridge = core.cartridge_mut().unwrap();
        cartridge.on_ppu_address_bus(0x0000, clock * 200);
        cartridge.on_ppu_address_bus(0x1000, clock * 200 + 100);
    }
    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "the write at $D002/$D003 was the IRQ latch and reload"
    );
}

/// `registry::board_memory_sizes` has **no arm** for this number: both of its registers are below
/// `$6000`, so nothing forces the question and the header decides — mappers 121, 123 and 187's
/// choice, and mapper 97's precedent in that function.
///
/// It is also the other half of the `$F003` reading above: the page's `Mask: $5003` taken
/// literally would decode `$7002`/`$7003` too, and then the window could not be RAM at all.
#[cfg(feature = "board-219")]
#[test]
fn req_core_095_mapper_219_leaves_6000_7fff_to_the_header_and_decodes_nothing_there() {
    let mut core = MapperRom::new(219).prg(32).chr(32).battery().load();
    core.mapper_write(0xA001, 0x80);
    core.mmc3_bank(6, 0x01);
    let windows = (core.prg_windows(), core.chr_windows());

    for (address, value) in [
        (0x6000u16, 0x11u8),
        (0x7002, 0x22),
        (0x7003, 0x33),
        (0x7FFF, 0x44),
    ] {
        core.write_memory(address, value);
        assert_eq!(
            value,
            core.read_memory(address),
            "{address:#06X} is PRG-RAM and the board does not decode it"
        );
    }

    assert_eq!(
        windows,
        (core.prg_windows(), core.chr_windows()),
        "and no write in $6000-$7FFF moved a window"
    );
}

// -------------------- MF-UNL-11's three save-state round trips and three resets,
// -------------------- §7.1's second common thing

/// What a board of this lot shows, and nothing of the machine around it: the four PRG windows,
/// the eight CHR window **offsets** and the mirroring.
///
/// The offsets and not the bytes, for `chr_offsets`'s own reason: all three boards move a CHR
/// window by 128 or 256 banks and the fixture's fill byte reads the same 256 banks apart.
#[cfg(any(feature = "board-215", feature = "board-208", feature = "board-219"))]
fn unl11_layout(core: &NesCore) -> ([u8; 4], [usize; 8], Nm) {
    (core.prg_windows(), chr_offsets(core), core.mirroring())
}

/// REQ-CORE-090/091: every register of mapper 215 is at `$5000`, `$5001` or `$5007`, and
/// `req_core_090_the_mapper_state_survives_a_save_and_restore` writes none of those — that sweep
/// dirties `$8000`-`$E001` and `$6000`. So the round trip is pinned here with all three of the
/// board's own registers moved first (MF-UNL-11 step 4).
///
/// The scramble register is part of it: with `$5007` restored to a different row the *same* later
/// write would mean a different MMC3 register, so a block that dropped it would round-trip the
/// windows and still diverge on the next write. The assertion on the re-saved bytes is what
/// covers that.
#[cfg(feature = "board-215")]
#[test]
fn req_core_090_mapper_215_round_trips_its_three_registers() {
    let mut core = MapperRom::new(215).prg(192).chr(1536).battery().load();
    core.mapper_write(0x5007, 0x04); // A scramble row that is neither the identity nor row 1.
    core.mapper_write(0x5001, 0x26); // Outer banks on both halves, and CHR A17.
    core.mapper_write(0x5000, 0x40); // The 128 KiB mode, so that A17 comes from $5001.
    core.mapper_write(0xA000, 0x05); // $5007 = 4: the written $A000 means the MMC3's $8000.
    core.mapper_write(0x8001, 0x11); // and the written $8001 its $8001, so R1 = $11.
    core.mapper_write(0xC000, 0x20); // The counter, so that it has run.
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    let layout = unl11_layout(&core);

    // Somewhere the chip's own registers could not reach: the outer banks away and the override
    // on, which is a 256 KiB move on both halves plus an NROM window.
    core.mapper_write(0x5001, 0x00);
    core.mapper_write(0x5000, 0x80 | 0x03);
    assert_ne!(
        layout,
        unl11_layout(&core),
        "the registers moved the layout"
    );

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(layout, unl11_layout(&core), "and the state brought it back");
    assert_eq!(
        state,
        core.save_state(true).unwrap(),
        "the whole block, $5007 and the derived fields included"
    );
}

/// REQ-CORE-090/091: mapper 208's four registers are at `$4800`, `$5000`, `$5800` and `$6800`,
/// none of which the generic sweep writes — and the four **protection** registers are state no
/// window shows, so the only thing that can pin them is the re-saved block together with a read
/// of the window itself (MF-UNL-11 step 4).
#[cfg(feature = "board-208")]
#[test]
fn req_core_090_mapper_208_round_trips_its_bank_index_and_four_protection_registers() {
    let mut core = MapperRom::new(208).prg(16).chr(256).battery().load();
    core.mapper_write(0x4800, 0x22); // Page 2, horizontal.
    core.mmc3_bank(0, 0x12);
    core.mmc3_bank(2, 0x85);
    core.mapper_write(0x5000, 0x8A); // An index whose entry is neither $00 nor $59.
    for register in 0..4u16 {
        core.mapper_write(0x5800 + register, 0x40 + register as u8);
    }
    core.mapper_write(0xC000, 0x20);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    let layout = unl11_layout(&core);
    let protection: [u8; 4] = std::array::from_fn(|r| core.read_memory(0x5800 + r as u16));

    // Move the page, the mirroring and all four protection registers.
    core.mapper_write(0x6800, 0x01);
    core.mapper_write(0x5000, 0x00);
    for register in 0..4u16 {
        core.mapper_write(0x5800 + register, 0xFF);
    }
    assert_ne!(layout, unl11_layout(&core), "the register moved the layout");
    assert_ne!(
        protection,
        std::array::from_fn(|r| core.read_memory(0x5800 + r as u16)),
        "and the protection registers moved too"
    );

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(layout, unl11_layout(&core), "and the state brought it back");
    assert_eq!(
        protection,
        std::array::from_fn(|r| core.read_memory(0x5800 + r as u16)),
        "the four protection registers included"
    );
    assert_eq!(
        state,
        core.save_state(true).unwrap(),
        "the whole block, the index register included"
    );
}

/// REQ-CORE-090/091: mapper 219 keeps **sixteen bytes of extended bank registers** plus the mode
/// bit, the index, the CHR latch and the outer bank — and none of its own ports is one the generic
/// sweep writes except `$8000`/`$8001`, which mean something else entirely while the mode bit is
/// set (MF-UNL-11 step 4).
///
/// The round trip is taken **in** extended mode, which is the state that can go wrong: with the
/// mode bit dropped the block would restore an MMC3's windows from the chip's registers and look
/// right until the next odd write.
#[cfg(feature = "board-219")]
#[test]
fn req_core_090_mapper_219_round_trips_its_extended_mode_registers() {
    let mut core = MapperRom::new(219).prg(64).chr(512).battery().load();
    core.mapper_write(0x5002, 0x01); // The outer bank, both bits.
    core.mapper_write(0x5003, 0x20);
    core.mmc3_bank(6, 0x03); // The chip's own registers, for the mode bit to hide.
    core.mapper_write(0xA000, 0x01); // and its mirroring, which the board never takes over.
    core.mapper_write(0x8002, 0x20); // Extended mode.
    for (index, datum) in [(0x26u8, 0x20u8), (0x25, 0x10), (0x24, 0x08), (0x23, 0x04)] {
        core.mapper_write(0x8000, index);
        core.mapper_write(0x8001, datum);
    }
    core.mapper_write(0x8000, 0x08); // The CHR latch.
    core.mapper_write(0x8001, 0x03);
    for index in [0x09u8, 0x0B, 0x11, 0x15] {
        core.mapper_write(0x8000, index);
        core.mapper_write(0x8001, 0x0A << 1);
    }
    core.mapper_write(0xC000, 0x20);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    let layout = unl11_layout(&core);

    // Out of extended mode and with the outer bank away, which is a layout no extended register
    // could produce.
    core.mapper_write(0x8002, 0x00);
    core.mapper_write(0x5002, 0x00);
    core.mapper_write(0x5003, 0x00);
    assert_ne!(layout, unl11_layout(&core), "the mode bit moved the layout");

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(layout, unl11_layout(&core), "and the state brought it back");
    assert_eq!(
        state,
        core.save_state(true).unwrap(),
        "the whole block, the sixteen extended banks and the latch included"
    );

    // The restored mode bit has to be the mode bit and not just its effect: one more odd write
    // must still be an extended register and not the chip's $8001.
    core.mapper_write(0x8000, 0x24);
    core.mapper_write(0x8001, 0x20);
    assert_eq!(
        1 + 0x30,
        core.prg_windows()[2],
        "still in extended mode after the restore, outer bank 3 included"
    );
}

/// AMD-061's own sweep,
/// `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state`, reaches all three
/// numbers — and for the save-state sweep's reason it dirties none of their own registers. This
/// one does, and then asks for the power-on layout back (MF-UNL-11 step 4).
///
/// The power-on states are not all zeroes. Mapper 215's `$5001` is **`$xF`**, so a reset puts the
/// outer bank back to 3 on both halves — the page's own *Note* requires it, "The Outer Bank
/// Register is reset to its power-up value if a Reset has been detected via an interruption of the
/// M2 signal" — and mapper 208's bank register is **`$11`**, page 3 with vertical mirroring, which
/// `Cartridge::reset` cannot know: it puts the *header's* mirroring back immediately before the
/// board's own `reset` runs, so the board has to state it.
#[cfg(all(feature = "board-215", feature = "board-208", feature = "board-219"))]
#[test]
fn amd_061_a_reset_puts_every_board_of_mf_unl_11_back_to_its_power_on_state() {
    for mapper in [208usize, 215, 219] {
        let core = MapperRom::new(mapper).prg(64).chr(512).load();
        let power_on = unl11_layout(&core);

        let mut core = core;
        for register in 0..8u8 {
            core.mapper_write(0x8000, register);
            core.mapper_write(0x8001, register + 0x11);
        }
        match mapper {
            215 => {
                core.mapper_write(0x5007, 0x01);
                core.mapper_write(0x5001, 0x1A);
                core.mapper_write(0x5000, 0xE5);
            }
            208 => {
                core.mapper_write(0x4800, 0x21);
                core.mapper_write(0x5000, 0x8A);
                core.mapper_write(0x5800, 0x5A);
            }
            219 => {
                core.mapper_write(0x5002, 0x01);
                core.mapper_write(0x5003, 0x20);
                core.mapper_write(0x8002, 0x20);
                core.mapper_write(0x8000, 0x26);
                core.mapper_write(0x8001, 0x20);
                core.mapper_write(0x8000, 0x11);
                core.mapper_write(0x8001, 0x0A << 1);
            }
            _ => {}
        }
        assert_ne!(
            power_on,
            unl11_layout(&core),
            "mapper {mapper}: the board's own registers must move something"
        );

        core.reset();
        assert_eq!(
            power_on,
            unl11_layout(&core),
            "mapper {mapper}: a reset must give the power-on layout back"
        );
        assert!(
            !core.cartridge().unwrap().irq_asserted(),
            "mapper {mapper}: and no pending IRQ"
        );
    }
}

// -------------------- MF-UNL-12 step 1: mapper 262, the Sachen UNL-SHERO

/// nesdev ("NES 2.0 Mapper 262"), the `$4100` **write** diagram: `.M.. 8421`, with `M` the
/// "Select CHR-ROM/-RAM" bit — "0: CHR-ROM, 1: CHR-RAM".
///
/// **Q-172: `M` is bit 6 and not bit 7.** The diagram's three lines are `Bit 7654 3210`, a rule of
/// nine dashes, and `.M.. 8421`; aligned, `M` sits where the header line has **6** and bit 7 is a
/// `.`. The *read* diagram's `J` is the one at bit 7 — the two are not symmetrical — so this test
/// asserts the select from both sides: a write with bit 7 set and bit 6 clear leaves the CHR-ROM
/// in place, and one with bit 6 set swaps the chip.
///
/// The second half is the chip's own address lines, which the page does not state at all. Q-172
/// §1.3 recorded two readings and MF-UNL-12 took mapper 119's — the chip banked by bits 0-2 of the
/// bank number the chip put in the window. **`docs/review/Q-XCHECK.md` §CHANGE 7 settled it the
/// other way and MF-FIX applied it**: the chip is **unbanked**, PPU A0-A12 straight through, which
/// is what all three projects that support the number do — Mesen 2 one 8 KiB page
/// (`MMC3_StreetHeroes.h:12-13`, `:45`), Nestopia UE the same
/// (`NstBoardSachenStreetHeroes.cpp:158`), and puNES one slot per 1 KiB window keyed on
/// `(address >> 10) & 7` rather than on the written bank (`mapper_262.c:91`).
///
/// So what the second half asserts now is the **inverse** of what it asserted: every bank number
/// shows the same kilobyte for a given window, and what distinguishes the kilobytes is the PPU
/// address. Both directions are checked, because "unbanked" is only interesting against a bank
/// change that used to move it — `$11`, `$19` and `$12` through R2 are the three the old reading
/// split two-to-one.
#[cfg(feature = "board-262")]
#[test]
fn req_core_102_mapper_262_selects_the_chr_ram_chip_from_4100_bit_6() {
    let mut core = MapperRom::new(262).prg(16).chr(512).load();
    core.mmc3_bank(0, 0x10);
    core.mmc3_bank(2, 0x11);
    assert_eq!(0x10, peek(&core, 0x0000), "the CHR-ROM, at power-on");
    assert_eq!(0x11, peek(&core, 0x1000), "and its 1 KiB half");

    // Bit 7 is a `.` in the diagram: it latches into the register and selects nothing.
    core.mapper_write(0x4100, 0x80);
    assert_eq!(0x10, peek(&core, 0x0000), "bit 7 is not the select");

    // Bit 6 is.
    core.mapper_write(0x4100, 0x40);
    chr_write(&mut core, 0x0000, 0x5A);
    chr_write(&mut core, 0x1000, 0x3C);
    assert_eq!(0x5A, peek(&core, 0x0000), "the CHR-RAM chip");
    assert_eq!(0x3C, peek(&core, 0x1000));

    // The chip is unbanked: window 4's bank is R2, and **no** value of it moves what $1000
    // shows. $11 and $19 agreed under the old reading too ($11 & 7 == $19 & 7); $12 is the one
    // that told the two readings apart, and it must now read the byte written above.
    for bank in [0x19u8, 0x12, 0x00, 0xFF, 0x37] {
        core.mmc3_bank(2, bank);
        assert_eq!(
            0x3C,
            peek(&core, 0x1000),
            "bank {bank:#04X}: the CHR-RAM chip is unbanked, so PPU $1000 is always its own              kilobyte (CHANGE 7)"
        );
    }

    // And what *does* distinguish the kilobytes is the PPU address: $1800 is the next 1 KiB
    // window along, so a write there must leave $1000 alone.
    chr_write(&mut core, 0x1800, 0x7E);
    assert_eq!(
        0x3C,
        peek(&core, 0x1000),
        "$1000 is untouched by a $1800 write"
    );
    assert_eq!(0x7E, peek(&core, 0x1800), "$1800 is its own kilobyte");

    // Back to the ROM, which no PPU write while the RAM was selected can have touched. R2 holds
    // $37 from the loop above, and the four-screen board's CHR A18 lines are all clear.
    core.mapper_write(0x4100, 0x00);
    assert_eq!(0x10, peek(&core, 0x0000));
    assert_eq!(0x37, peek(&core, 0x1000));
}

/// The four high CHR bits of `$4100`, each "Select 256 KiB CHR-ROM bank" for one 2 KiB region —
/// CHR A18, which is what takes this board above 256 KiB of CHR-ROM.
///
/// **Q-172: the four lines are not in address order.** Each annotation line's `+` names its bit,
/// and read against the header they are bit 3 → `$0000-$07FF`, bit 2 → `$0800-$0FFF`, bit 1 →
/// **`$1800-$1FFF`**, bit 0 → **`$1000-$17FF`**. So the four regions in address order take bits
/// 3, 2, **0**, **1**: the `8421` the diagram labels those four bits with is their *weight*, and
/// the bottom two regions are crossed over against it. One bit at a time is the only way to show
/// it, and that crossing is the whole point of the loop below.
///
/// The offsets and not the bytes, for `chr_offsets`'s own reason: a 256-bank move reads the same
/// fill byte.
#[cfg(feature = "board-262")]
#[test]
fn q_172_mapper_262_crosses_the_two_bottom_chr_a18_lines_against_their_regions() {
    let mut core = MapperRom::new(262).prg(16).chr(512).load();
    // One distinct bank per window: R0 and R1 are the two 2 KiB registers, R2-R5 the four 1 KiB.
    core.mmc3_bank(0, 0x00);
    core.mmc3_bank(1, 0x10);
    for register in 2..6u8 {
        core.mmc3_bank(register, 0x20 + register);
    }
    let base = [0x00usize, 0x01, 0x10, 0x11, 0x22, 0x23, 0x24, 0x25];
    assert_eq!(
        base.map(|bank| bank * 1024),
        chr_offsets(&core),
        "the chip's own eight windows, all inside the first 256 KiB"
    );

    // (the bit of $4100, the two 1 KiB windows the page gives it)
    for (bit, windows) in [(3u8, [0usize, 1]), (2, [2, 3]), (0, [4, 5]), (1, [6, 7])] {
        core.mapper_write(0x4100, 1 << bit);
        let expected: [usize; 8] = std::array::from_fn(|window| {
            (base[window] + if windows.contains(&window) { 0x100 } else { 0 }) * 1024
        });
        assert_eq!(
            expected,
            chr_offsets(&core),
            "$4100 bit {bit} is CHR A18 over windows {windows:?} and over no other"
        );
    }

    // All four at once, which is the second 256 KiB half everywhere.
    core.mapper_write(0x4100, 0x0F);
    assert_eq!(
        base.map(|bank| (bank + 0x100) * 1024),
        chr_offsets(&core),
        "and the board reaches every bank of a 512 KiB CHR-ROM"
    );

    // The line is a line and not an addend: setting it twice is setting it once.
    core.mapper_write(0x4100, 0x0F);
    assert_eq!(base.map(|bank| (bank + 0x100) * 1024), chr_offsets(&core));
}

/// "as well **4 KiB of additional VRAM for four-screen mirroring**" — the board carries the
/// nametable the arrangement needs, so the mirroring is its wiring and not byte 6 of a header.
/// `ppu::VRAM_SIZE` is 4 KiB, so this core's own VRAM *is* those four pages, which is mapper 77's
/// reading of the same sentence.
///
/// **Q-172**: the page also says "`$A000`: As normal MMC3", and the two cannot both be literally
/// true — a four-screen board drives CIRAM A10 **and** A11 from the cartridge, so the chip's one
/// mirroring register reaches nothing. The second half of this test is that choice: every value of
/// `$A000` leaves all four pages distinct, and so does a reset.
#[cfg(feature = "board-262")]
#[test]
fn q_172_mapper_262_is_four_screen_from_its_wiring_and_its_a000_is_inert() {
    let pages = |core: &NesCore| {
        [0x2000u16, 0x2400, 0x2800, 0x2C00].map(|address| core.map_nametable(address))
    };
    for bits in [0x00u8, 0x01, 0x08, 0x09] {
        let mut core = MapperRom::new(262).prg(16).chr(32).mirroring(bits).load();
        assert_eq!(Nm::FourScreen, core.mirroring(), "header bits {bits:#04X}");
        assert_eq!(
            [0x0000, 0x0400, 0x0800, 0x0C00],
            pages(&core),
            "header bits {bits:#04X}: four pages, four distinct kilobytes of VRAM"
        );

        for value in [0x00u8, 0x01, 0x02, 0x03, 0xFF] {
            core.mapper_write(0xA000, value);
            assert_eq!(
                Nm::FourScreen,
                core.mirroring(),
                "header {bits:#04X}, $A000 = {value:#04X}"
            );
            assert_eq!([0x0000, 0x0400, 0x0800, 0x0C00], pages(&core));
        }

        core.reset();
        assert_eq!(Nm::FourScreen, core.mirroring(), "and after a reset");
        assert_eq!([0x0000, 0x0400, 0x0800, 0x0C00], pages(&core));
    }
}

/// The *read* side of `$4100`: `J... ....`, "Jumper value — 0: Game title is *Street Heroes*;
/// 1: Game title is *侍魂 (Shìhún)* (Samurai Spirits)".
///
/// **Q-172**: the jumper is the board's wiring and **no header field can state which of the two
/// titles a dump is** — the page defines no submapper either — so this build answers **0**, the
/// page's first-listed title, and records the gap rather than inventing a heuristic. What this
/// test pins is that the port answers at all, under the page's own "probably `$C100`" mask: A15
/// clear, A14 set, A8 set, every other line free.
#[cfg(feature = "board-262")]
#[test]
fn q_172_mapper_262_answers_its_title_jumper_under_the_pages_c100_mask() {
    // No PRG-RAM declared, so that $6000-$7FFF reaches the board and the whole mask can be shown.
    let core = MapperRom::new(262).prg(16).chr(32).load();
    for address in [0x4100u16, 0x4101, 0x41FF, 0x4300, 0x5F00, 0x7F00] {
        assert_eq!(
            0x00,
            core.read_memory(address),
            "{address:#06X} is inside `address & $C100 == $4100`: the jumper, 0 for Street Heroes"
        );
    }
}

// -------------------- MF-UNL-12 step 2: mapper 292, the BMW8544 / UNL-DRAGONFIGHTER

/// A CPU read **with its side effects**, which is the only path that fills one of mapper 292's
/// data registers: `NesCore::read_memory` is `Cartridge::try_cpu_peek` and goes to
/// `Mapper::cpu_peek_override`, which this board deliberately leaves side-effect-free.
#[cfg(feature = "board-292")]
fn cpu_read(core: &mut NesCore, address: u16) -> Option<u8> {
    core.cartridge_mut().unwrap().try_cpu_read(address)
}

/// What the game does, in the order the page describes it: select a data register with a **write**
/// to `$6000-$7FFF`, put the datum somewhere in **zero page**, then **read** `$6000-$7FFF` so the
/// board moves the latched byte into the register.
///
/// The order matters and the test would be wrong without it: the `$6000` write is itself a CPU
/// write and so is itself latched, which is why the zero-page write comes second.
#[cfg(feature = "board-292")]
fn dragon_fill(core: &mut NesCore, register: u8, value: u8) {
    // `A` and `B` are "Always 1" and `R` is bit 5.
    core.mapper_write(0x6000, 0xC0 | (register << 5));
    core.write_memory(0x0010, value);
    cpu_read(core, 0x6000);
}

/// nesdev ("NES 2.0 Mapper 292"): "There are four extra registers that are filled by writing or
/// reading **(!)** from the CPU `$6000-$7FFF` address range when PRG-RAM is enabled in the MMC3's
/// `A001` register", and of each data register: "It is filled by *reading* from `$6000-$7FFF` …
/// with the value that had been latched during the last CPU write to any address."
///
/// **This is the test §7.2 asks for — "a read at `$6000-$7FFF` changes the CHR bank, which is the
/// test that proves the read path is wired"** — and it asserts the other half too: a *write* to
/// the same window selects a register and fills nothing, even though the written byte is itself
/// what the latch then holds.
#[cfg(feature = "board-292")]
#[test]
fn req_core_102_mapper_292_fills_a_data_register_from_a_read_of_6000_7fff() {
    let mut core = MapperRom::new(292).prg(32).chr(512).load();
    // "when PRG-RAM is enabled in the MMC3's A001 register".
    core.mapper_write(0xA001, 0x80);
    assert_eq!(
        [0, 1, 0, 1, 0, 1, 2, 3],
        core.chr_windows(),
        "power-on: both XOR registers zero, so the chip's own zeroed banks show through"
    );

    // Extra Data Register #1, `.BAA AAAA`: bits 5-0 are the 4 KiB bank at PPU $1000-$1FFF.
    core.mapper_write(0x6000, 0xE0); // A, B and R = 1.
    core.write_memory(0x0010, 0x05); // "The game writes the desired values to zero page."
    assert_eq!(
        [0, 1, 0, 1, 0, 1, 2, 3],
        core.chr_windows(),
        "neither the write to $6000 nor the one to zero page fills a data register"
    );

    cpu_read(&mut core, 0x6000);
    assert_eq!(
        [0, 1, 0, 1, 20, 21, 22, 23],
        core.chr_windows(),
        "the read does: 4 KiB bank 5 is 1 KiB banks 20-23"
    );

    // The latch moving on its own moves nothing, and a write to the window still moves nothing —
    // which together are what make the *read* the trigger.
    core.write_memory(0x0010, 0x0A);
    assert_eq!([0, 1, 0, 1, 20, 21, 22, 23], core.chr_windows());
    core.mapper_write(0x6000, 0xE0);
    assert_eq!([0, 1, 0, 1, 20, 21, 22, 23], core.chr_windows());

    // And the whole window reads, not just $6000.
    core.write_memory(0x0010, 0x0A);
    cpu_read(&mut core, 0x7FFF);
    assert_eq!(
        [0, 1, 0, 1, 40, 41, 42, 43],
        core.chr_windows(),
        "$7FFF is the same register window"
    );

    // With the chip's `$A001` enable clear the board's window answers nothing at all, which is
    // the page's own condition on all four registers.
    core.mapper_write(0xA001, 0x00);
    core.mapper_write(0x6000, 0xC0); // Would select register 0 if it were decoded.
    core.write_memory(0x0010, 0x7F);
    cpu_read(&mut core, 0x6000);
    assert_eq!(
        [0, 1, 0, 1, 40, 41, 42, 43],
        core.chr_windows(),
        "PRG-RAM disabled: neither the select nor the fill happens"
    );
}

/// The page's three *Effective CHR banks* blocks, each with its own C line, checked against the
/// arithmetic they describe — §7.2's "the XOR values differ per pattern table" and "the
/// effective-bank arithmetic checked against the page's own worked table".
///
/// `chrBank2K = extraRegister[0] ^ (mmc3Register[0] >> 1)` at PPU `$0000-$07FF`;
/// `chrBank2K = ((extraRegister[1] << 1) & 0x80) ^ (mmc3Register[1] >> 1)` at `$0800-$0FFF`, where
/// the only thing the extra register contributes is **CHR A18**; and
/// `chrBank4K = extraRegister[1] & 0x3F` at `$1000-$1FFF`, where the chip contributes **nothing**
/// and "CHR A18: Always 0, i.e. restricted to the first 256 KiB CHR-ROM half".
#[cfg(feature = "board-292")]
#[test]
fn req_core_102_mapper_292_computes_each_regions_bank_the_way_its_own_c_line_does() {
    let mut core = MapperRom::new(292).prg(32).chr(512).load();
    core.mapper_write(0xA001, 0x80);
    core.mmc3_bank(0, 0x2A); // R0: a 2 KiB bank of $15.
    core.mmc3_bank(1, 0x40); // R1: a 2 KiB bank of $20.

    dragon_fill(&mut core, 0, 0x03);
    assert_eq!(
        [0x16 * 2 * 1024, (0x16 * 2 + 1) * 1024],
        chr_offsets(&core)[..2],
        "$0000-$07FF: $03 XOR $15 is $16, and CHR A10 is PPU A10"
    );
    assert_eq!(
        [0x20 * 2 * 1024, (0x20 * 2 + 1) * 1024],
        chr_offsets(&core)[2..4],
        "$0800-$0FFF: the chip's R1 alone, Extra0 reaching nothing here"
    );

    // The XOR is an XOR and not an OR: $FF against the same R0 gives the complement.
    dragon_fill(&mut core, 0, 0xFF);
    assert_eq!(
        [(0xFF ^ 0x15) * 2 * 1024, ((0xFF ^ 0x15) * 2 + 1) * 1024],
        chr_offsets(&core)[..2],
        "$FF XOR $15 is $EA"
    );
    dragon_fill(&mut core, 0, 0x00);
    assert_eq!(
        [0x15 * 2 * 1024, (0x15 * 2 + 1) * 1024],
        chr_offsets(&core)[..2],
        "and zero leaves the chip's own bank"
    );

    // Extra1 bit 6 is CHR A18 of the $0800-$0FFF pair, and bits 5-0 the 4 KiB bank of
    // $1000-$1FFF: one register, two regions, and the test moves both at once.
    dragon_fill(&mut core, 1, 0x40 | 0x07);
    assert_eq!(
        [(0xA0 * 2) * 1024, (0xA0 * 2 + 1) * 1024],
        chr_offsets(&core)[2..4],
        "$0800-$0FFF: R1's $20 with CHR A18 from Extra1 bit 6 is $A0"
    );
    assert_eq!(
        [0x1C * 1024, 0x1D * 1024, 0x1E * 1024, 0x1F * 1024],
        chr_offsets(&core)[4..],
        "$1000-$1FFF: 4 KiB bank 7, and CHR A10-A11 are PPU A10-A11"
    );

    // "CHR A18: Always 0" there, so **no** value of Extra1 can take that region past the first
    // 256 KiB — bit 7 is outside the field and bit 6 belongs to the other region.
    dragon_fill(&mut core, 1, 0xFF);
    assert_eq!(
        [0xFC * 1024, 0xFD * 1024, 0xFE * 1024, 0xFF * 1024],
        chr_offsets(&core)[4..],
        "$FF is 4 KiB bank $3F: the top of the first 256 KiB and no further"
    );
}

/// **Q-172**: "Note that when Bit 7 in the MMC3's index register is set, MMC3 registers **2/3**
/// instead of register 0 apply" (and 4/5 instead of 1).
///
/// The reading implemented is the literal one: bit 7 of the chip's `$8000` is its CHR **inversion**
/// bit, so what the chip drives at PPU `$0000-$07FF` with it set *is* R2 and R3, one kilobyte each
/// — and taking the chip's own per-window 1 KiB bank number and shifting it right by one gives
/// "register 0" with the bit clear and "registers 2/3" with it set, with no code of its own. The
/// consequence the page's plural implies and this test pins is that the two kilobytes of
/// `$0000-$07FF` are then **independent**: R2 drives the first and R3 the second.
///
/// `Mmc3Board::chr_bank_of_window` is the accessor that makes it so, which is why this board
/// widens its `cfg` alongside mapper 262's.
#[cfg(feature = "board-292")]
#[test]
fn q_172_mapper_292_takes_registers_2_and_3_when_the_chips_chr_inversion_is_set() {
    let mut core = MapperRom::new(292).prg(32).chr(512).load();
    core.mapper_write(0xA001, 0x80);
    core.mmc3_bank(0, 0x2A); // R0 and R1, which the inversion moves out of the way.
    core.mmc3_bank(1, 0x40);
    core.mmc3_bank(2, 0x10); // R2 and R3, which it moves in.
    core.mmc3_bank(3, 0x21);
    core.mmc3_bank(4, 0x30); // R4 and R5, likewise for $0800-$0FFF.
    core.mmc3_bank(5, 0x41);
    dragon_fill(&mut core, 0, 0x00);

    assert_eq!(
        [0x15 * 2 * 1024, (0x15 * 2 + 1) * 1024],
        chr_offsets(&core)[..2],
        "inversion clear: register 0's own 2 KiB bank"
    );

    core.mapper_write(0x8000, 0x80);
    assert_eq!(
        [(0x10 >> 1) * 2 * 1024, ((0x21 >> 1) * 2 + 1) * 1024],
        chr_offsets(&core)[..2],
        "inversion set: R2 drives the first kilobyte and R3 the second"
    );
    assert_eq!(
        [(0x30 >> 1) * 2 * 1024, ((0x41 >> 1) * 2 + 1) * 1024],
        chr_offsets(&core)[2..4],
        "and R4/R5 the $0800-$0FFF pair, which is the note's second half"
    );
    assert_eq!(
        [0x00, 0x01, 0x02, 0x03].map(|bank: usize| bank * 1024),
        chr_offsets(&core)[4..],
        "$1000-$1FFF is Extra1's alone whichever way the chip is inverted"
    );

    // The XOR still applies through the inversion, which is what makes it registers 2/3 of the
    // *same* formula and not a different one.
    dragon_fill(&mut core, 0, 0x03);
    assert_eq!(
        [
            ((0x10 >> 1) ^ 0x03) * 2 * 1024,
            (((0x21 >> 1) ^ 0x03) * 2 + 1) * 1024
        ],
        chr_offsets(&core)[..2],
    );
}

/// **Q-172, and the one claim of §7.2 that this page overrides.** The plan says of this step that
/// "`Cartridge::cpu_read` already reaches that window, so **no new hook**"; the page's *Latch
/// Register* section says "All CPU writes, **including writes to zero page memory**, are visible
/// on the cartridge connector. The mapper keeps the data of the last CPU write to any address in a
/// temporary latch register … The game writes the desired values to zero page."
///
/// A zero-page write is RAM and reaches no cartridge in this core, so the latch is a new hook:
/// `CartridgeMemory::latch_cpu_write`, one store in `NesBus::write_internal` behind `board-292`.
/// This test is what it buys — the value the board stores comes from a write to `$0010`, an
/// address the cartridge is not mapped at — plus the two things the page says of the index
/// register's other bits.
#[cfg(feature = "board-292")]
#[test]
fn q_172_mapper_292_latches_the_last_cpu_write_to_any_address_zero_page_included() {
    let mut core = MapperRom::new(292).prg(32).chr(512).load();
    core.mapper_write(0xA001, 0x80);
    core.mapper_write(0x6000, 0xE0);

    // Four addresses no cartridge is mapped at, each the "last CPU write" in turn.
    for (address, value) in [
        (0x0010u16, 0x05u8),
        (0x00FF, 0x06),
        (0x0400, 0x07),
        (0x07FF, 0x08),
    ] {
        core.write_memory(address, value);
        cpu_read(&mut core, 0x6000);
        let bank = usize::from(value) * 4;
        assert_eq!(
            [bank, bank + 1, bank + 2, bank + 3].map(|b| b * 1024),
            chr_offsets(&core)[4..],
            "a write to {address:#06X} is what the next read of $6000 stores"
        );
    }

    // "It's hypothesized that clearing the A and B bits may switch the two CHR pattern table
    // halves to normal MMC3 CHR banking, but this has not yet been verified on real hardware.
    // The game keeps both bits enabled at all times." So the two bits are latched into the
    // register — which puts them in the savestate for free — and drive nothing.
    let before = chr_offsets(&core);
    core.mapper_write(0x6000, 0x20); // R = 1, A and B clear.
    core.write_memory(0x0010, 0x08);
    cpu_read(&mut core, 0x6000);
    assert_eq!(
        before,
        chr_offsets(&core),
        "A and B clear: the unverified hypothesis is not implemented"
    );

    // And `R` is bit 5 of the index and not bit 0: a value with bit 0 set and bit 5 clear picks
    // Extra Data Register **#0**, which is the XOR of the other pattern table.
    core.mmc3_bank(0, 0x00);
    core.mapper_write(0x6000, 0xC1);
    core.write_memory(0x0010, 0x11);
    cpu_read(&mut core, 0x6000);
    assert_eq!(
        [0x11 * 2 * 1024, (0x11 * 2 + 1) * 1024],
        chr_offsets(&core)[..2],
        "bit 0 of the index is not the select; bit 5 is"
    );
}

/// REQ-CORE-095: "There are four extra registers that are filled by writing or reading (!) from
/// the CPU `$6000-$7FFF` address range", so that window **is** the board's decode and
/// `registry::board_memory_sizes` gives the number no PRG-RAM — the arm 114, 115, 182, 208 and the
/// two MMC3 multicarts are already on.
///
/// Two things follow and both are asserted: a write there does not persist, and a read there
/// drives nothing on the bus (`Some(None)`, which leaves the caller's open-bus value), so a game
/// cannot mistake the register window for save RAM. The dump declares a battery, which is exactly
/// the case the row has to win against.
#[cfg(feature = "board-292")]
#[test]
fn req_core_095_mapper_292_has_no_prg_ram_behind_its_register_window() {
    let mut core = MapperRom::new(292).prg(32).chr(512).battery().load();
    core.mapper_write(0xA001, 0x80);
    for address in [0x6000u16, 0x6ABC, 0x7FFF] {
        core.write_memory(address, 0x5A);
        assert_ne!(
            0x5A,
            core.read_memory(address),
            "{address:#06X} is the register window and not RAM"
        );
        assert_eq!(
            None,
            cpu_read(&mut core, address),
            "{address:#06X}: the board decodes it and drives nothing"
        );
    }
}

// -------------------- MF-UNL-12 step 3: mapper 246, the G0151-1

/// nesdev ("INES Mapper 246"), the two register diagrams:
///
/// ```text
/// PRG-ROM bank select ($6000-$6003), write    CHR-ROM bank select ($6004-$6007), write
///  A~[0110 0000 000. .0rr] D~[..bb bbbb]       A~[0110 0000 000. .1rr] D~[bbbb bbbb]
///                      ||       PRG A18..A13                       ||     CHR A18..A11
/// ```
///
/// §7.2's first test bullet, "the eight bank registers", with each field's own width shown: the
/// PRG field is **six** bits and the CHR field **eight**, so a PRG register ignores what a CHR
/// register at the same datum would use.
#[cfg(feature = "board-246")]
#[test]
fn req_core_102_mapper_246_banks_four_prg_windows_and_four_chr_pairs() {
    let mut core = MapperRom::new(246).prg(64).chr(512).load();
    assert_eq!(
        [0, 0, 0, 0x3F],
        core.prg_windows(),
        "power-on: three zeroed registers and $6003 at $FF, masked to bank $3F"
    );
    assert_eq!([0, 1, 0, 1, 0, 1, 0, 1], core.chr_windows());

    for (register, bank) in [(0u16, 0x11u8), (1, 0x22), (2, 0x13), (3, 0x24)] {
        core.write_memory(0x6000 + register, bank);
    }
    assert_eq!([0x11, 0x22, 0x13, 0x24], core.prg_windows());

    // "PRG A18..A13" is six bits: bits 6 and 7 of the datum reach nothing.
    core.write_memory(0x6000, 0xC0 | 0x11);
    assert_eq!(
        [0x11, 0x22, 0x13, 0x24],
        core.prg_windows(),
        "the PRG field is six bits wide"
    );

    // "CHR A18..A11" is eight: the 2 KiB bank numbers go to $FF, which on a 512 KiB CHR-ROM is
    // the last pair.
    for (register, bank) in [(4u16, 0x02u8), (5, 0x11), (6, 0x40), (7, 0xFF)] {
        core.write_memory(0x6000 + register, bank);
    }
    assert_eq!(
        [
            0x02 * 2 * 1024,
            (0x02 * 2 + 1) * 1024,
            0x11 * 2 * 1024,
            (0x11 * 2 + 1) * 1024,
            0x40 * 2 * 1024,
            (0x40 * 2 + 1) * 1024,
            0xFF * 2 * 1024,
            (0xFF * 2 + 1) * 1024
        ],
        chr_offsets(&core),
        "four 2 KiB CHR windows, eight bits each"
    );

    // "Nametable mirroring is hard-wired and set by the iNES header": no register moves it.
    assert_eq!(Nm::Horizontal, core.mirroring());
    for register in 0..8u16 {
        core.write_memory(0x6000 + register, 0xFF);
        assert_eq!(Nm::Horizontal, core.mirroring(), "register {register}");
    }
}

/// **The address mask and the forced-read condition are the same number**, which this test is
/// written to show rather than to assume.
///
/// The diagrams give `A~[0110 0000 000. .0rr]` — `address & $FFE4 == $6000`, A4, A3 and the two
/// `r` bits free — and the page's first bullet gives "When reading from CPU address
/// `$FFE4-$FFE7`, `$FFEC-$FFEF`, `$FFF4-$FFF7`, or `$FFFC-$FFFF`, PRG A17 is forced high, as if
/// register `$6003` were OR'd with `$10`". Those four ranges are exactly
/// `address & $FFE4 == $FFE4`: A15-A5 all set with A2 set, and A4, A3, A1 and A0 free.
///
/// So the loop walks all 32 addresses of `$FFE0-$FFFF` and checks each one against the mask —
/// sixteen forced, sixteen not — which is §7.2's third test bullet done exhaustively. `$10` in the
/// six-bit PRG field is bit 4, i.e. **PRG A17**.
#[cfg(feature = "board-246")]
#[test]
fn req_core_102_mapper_246_forces_prg_a17_on_the_eight_addresses_of_its_own_mask() {
    // 512 KiB of PRG so that every one of the six bank bits is inside the image, and bank $3F
    // filled with $FF so the forced and unforced banks read differently.
    let mut core = MapperRom::new(246)
        .prg(64)
        .chr(32)
        .prg_bank_all_ones(0x2F)
        .load();
    // $6003 = $0F: bank $0F unforced, bank $1F forced. Neither is the power-on $3F.
    core.write_memory(0x6003, 0x0F);
    assert_eq!(0x0F, core.read_memory(0xE000), "the window's own bank");
    assert_eq!(
        0x0F,
        core.read_memory(0xFFE0),
        "and $FFE0, which is not forced"
    );

    for low in 0xE0..=0xFFu16 {
        let address = 0xFF00 | low;
        let forced = address & 0xFFE4 == 0xFFE4;
        assert_eq!(
            if forced { 0x1F } else { 0x0F },
            core.read_memory(address),
            "{address:#06X}: forced = {forced}"
        );
    }

    // The forcing is an OR of one bit and not an addend: with the bit already set in the register
    // the forced ranges read the same bank as the rest of the window.
    core.write_memory(0x6003, 0x1F);
    for low in 0xE0..=0xFFu16 {
        assert_eq!(
            0x1F,
            core.read_memory(0xFF00 | low),
            "$6003 already has bit 4: {:#06X}",
            0xFF00u16 | low
        );
    }

    // And bank $2F is the one filled with $FF, so the forcing is visible as a *different bank*
    // and not merely as a different number: $6003 = $0F forces to $1F, $6003 = $1F to $1F, and
    // $6003 = $2F to $3F.
    core.write_memory(0x6003, 0x2F);
    assert_eq!(
        0xFF,
        core.read_memory(0xE000),
        "the window shows bank $2F, the all-ones one"
    );
    // $FFF4 and not $FFFC: the last six bytes of the image are the fixture's own vectors, so
    // bank $3F does not read its fill byte there.
    assert_eq!(
        0x3F,
        core.read_memory(0xFFF4),
        "and a forced address reads bank $3F instead"
    );

    // The other three windows are untouched by all of this, which is what keeps them on
    // `Cartridge::read_rom_window`'s fast path. The fourth reads `$FF` because bank $2F is the
    // all-ones one and `prg_windows` reads a byte, not a bank number.
    assert_eq!([0, 0, 0, 0xFF], core.prg_windows());
}

/// §7.2's second test bullet: "a PRG-RAM write at `$6800-$6FFF` survives a round-trip and a write
/// at `$6000-$67FF` does **not** reach PRG-RAM (it is a bank register)".
///
/// **Q-172**: the page's mask makes the register window `$6000-$601F` and not `$6000-$67FF` — A11
/// to A5 must all be clear — so the bullet is Mesen's decode and the page's is narrower. The test
/// is written from the mask: the 32 addresses it names are registers, and an address the mask does
/// not name is the RAM chip mirrored, which `Mapper::prg_ram_window` cannot refuse.
#[cfg(feature = "board-246")]
#[test]
fn req_core_095_mapper_246_keeps_its_battery_ram_at_6800_and_its_registers_at_6000() {
    let mut core = MapperRom::new(246).prg(64).chr(32).battery().load();
    assert_eq!(
        0x0800,
        core.cartridge().unwrap().battery_ram().len(),
        "2 KiB, battery-banked, and the whole chip is the battery slice"
    );

    // The page's own window: a write survives a round trip.
    for (address, value) in [(0x6800u16, 0x5Au8), (0x6C00, 0x3C), (0x6FFF, 0xA5)] {
        core.write_memory(address, value);
        assert_eq!(
            value,
            core.read_memory(address),
            "{address:#06X} is the chip"
        );
    }

    // All 32 addresses the mask names are bank registers and reach no RAM. Bit 2 picks PRG from
    // CHR and bits 1-0 the register, with A4 and A3 free — so every one of them decodes.
    let windows = (core.prg_windows(), chr_offsets(&core));
    for low in 0x00..0x20u16 {
        let address = 0x6000 | low;
        core.write_memory(address, 0x07);
        assert_ne!(
            (core.prg_windows(), chr_offsets(&core)),
            windows,
            "{address:#06X} is a bank register"
        );
        // and the chip is untouched by it.
        assert_eq!(0x5A, core.read_memory(0x6800));
        for register in 0..8u16 {
            core.write_memory(0x6000 + register, if register == 3 { 0x0F } else { 0x00 });
        }
    }

    // An address the mask does **not** name: the 2 KiB chip mirrored, which is Q-172's third
    // point. $6400 is A10 of the chip and A11 clear, so it is the chip's own $0400.
    core.write_memory(0x6400, 0x11);
    assert_eq!(0x11, core.read_memory(0x6400));
    assert_eq!(
        0x11,
        core.read_memory(0x6C00),
        "$6400 and $6C00 are one byte of a 2 KiB chip"
    );
}

/// **Q-172**: "Register `$6003` is set to `$FF` at power-on **only**."
///
/// The word "only" is the whole of this test. AMD-061's own sweep asks every board for its
/// power-on state back after a reset, and this board cannot give it: the page says the register
/// has no reset line, so the bank the vectors are fetched through is whatever the game last wrote.
/// The other seven registers go back to zero, which the page gives no value for and which is this
/// build's default for a register no page states.
#[cfg(feature = "board-246")]
#[test]
fn amd_061_mapper_246_keeps_its_last_prg_bank_across_a_reset() {
    let mut core = MapperRom::new(246).prg(64).chr(512).load();
    assert_eq!([0, 0, 0, 0x3F], core.prg_windows(), "power-on");

    for register in 0..8u16 {
        core.write_memory(0x6000 + register, 0x11 + register as u8);
    }
    assert_eq!([0x11, 0x12, 0x13, 0x14], core.prg_windows());

    core.reset();
    assert_eq!(
        [0, 0, 0, 0x14],
        core.prg_windows(),
        "$6003 keeps its value -- `$FF` is a power-on value and not a reset value"
    );
    assert_eq!(
        [0, 1, 0, 1, 0, 1, 0, 1],
        core.chr_windows(),
        "and the four CHR registers do go back to zero"
    );

    // A second reset says the same thing, which is what makes it the register and not a leftover.
    core.reset();
    assert_eq!([0, 0, 0, 0x14], core.prg_windows());
}

// -------------------- MF-UNL-12's three save-state round trips and its reset,
// -------------------- §7.1's second common thing

/// What a board of this lot shows, and nothing of the machine around it: the four PRG windows, the
/// eight CHR window **offsets**, the eight CHR **bytes** and the mirroring.
///
/// The offsets *and* the bytes, where MF-UNL-11's own helper took only the offsets: mapper 262 has
/// two CHR chips and a register that picks between them, so two layouts can share every offset and
/// still read differently. The bytes are what tell the chips apart.
#[cfg(any(feature = "board-246", feature = "board-262", feature = "board-292"))]
fn unl12_layout(core: &NesCore) -> ([u8; 4], [usize; 8], [u8; 8], Nm) {
    (
        core.prg_windows(),
        chr_offsets(core),
        core.chr_windows(),
        core.mirroring(),
    )
}

/// REQ-CORE-090/091, §7.1 item 2: mapper 262's one register is at **`$4100`**, and
/// `req_core_090_the_mapper_state_survives_a_save_and_restore` dirties `$5000`, `$6000` and
/// `$8000-$E001` — so that sweep proves the chip's block round-trips and says nothing about this
/// board's own byte (MF-UNL-12 step 4).
///
/// The register is the interesting one precisely because it is **one byte with two jobs**: the CHR
/// chip select and four CHR A18 lines. A block that dropped it would restore the window table and
/// still read the wrong chip, which is why this test's layout carries the CHR bytes as well as the
/// offsets.
#[cfg(feature = "board-262")]
#[test]
fn req_core_090_mapper_262_round_trips_its_4100_register() {
    let mut core = MapperRom::new(262).prg(32).chr(512).battery().load();
    core.mmc3_bank(0, 0x10); // The chip's own registers, under the board's.
    core.mmc3_bank(2, 0x21);
    core.mapper_write(0x4100, 0x40 | 0x0F); // CHR-RAM, and all four A18 lines set.
    chr_write(&mut core, 0x0000, 0x5A); // Bytes in the chip the select names.
    chr_write(&mut core, 0x1000, 0x3C);
    core.mapper_write(0xC000, 0x20); // The counter, so that it has run.
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    let layout = unl12_layout(&core);
    // The CHR-RAM chip is what the PPU reads, and since MF-FIX made that chip **unbanked**
    // (CHANGE 7) each byte written into it appears at exactly the window its PPU address names
    // and nowhere else: $5A at window 0 ($0000) and $3C at window 4 ($1000), the other six
    // kilobytes still zero. Under the old bank-driven reading R0 = $10 and R2 = $21 spread the
    // same two bytes over all eight windows, which is the inversion the fix produces.
    assert_eq!(
        [0x5A, 0x00, 0x00, 0x00, 0x3C, 0x00, 0x00, 0x00],
        layout.2,
        "the CHR-RAM chip is what the PPU reads, unbanked"
    );

    // Back to the CHR-ROM with no A18 line, which is a layout the chip's own registers cannot
    // produce while the select is set.
    core.mapper_write(0x4100, 0x00);
    assert_ne!(layout, unl12_layout(&core), "the register moved the layout");

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(layout, unl12_layout(&core), "and the state brought it back");
    assert_eq!(
        state,
        core.save_state(true).unwrap(),
        "the whole block, the derived window table included"
    );
}

/// REQ-CORE-090/091, §7.1 item 2: mapper 292 keeps **three** register bytes, and only one of them
/// can be written at all — the other two are filled by a **read** of `$6000-$7FFF`, which no sweep
/// of this suite performs (MF-UNL-12 step 4).
///
/// The round trip is taken with the index left selecting Extra Data Register #1, which is the
/// state that can go wrong quietly: a block that restored the two data registers and dropped the
/// index would round-trip every window and then put the *next* fill in the wrong register.
/// The last four lines are that check.
#[cfg(feature = "board-292")]
#[test]
fn req_core_090_mapper_292_round_trips_the_two_registers_only_a_read_can_fill() {
    let mut core = MapperRom::new(292).prg(32).chr(512).battery().load();
    core.mapper_write(0xA001, 0x80);
    core.mmc3_bank(0, 0x2A); // The chip's own CHR registers, which the XOR sits on.
    core.mmc3_bank(1, 0x40);
    core.mmc3_bank(6, 0x03); // and its PRG bank, which the board never touches.
    core.mapper_write(0xA000, 0x01); // and its mirroring, likewise.
    dragon_fill(&mut core, 0, 0x33);
    dragon_fill(&mut core, 1, 0x40 | 0x1A);
    core.mapper_write(0xC000, 0x20);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    let layout = unl12_layout(&core);

    // Both data registers to zero, which no write could have done and which leaves the chip's
    // own banks showing through.
    dragon_fill(&mut core, 0, 0x00);
    dragon_fill(&mut core, 1, 0x00);
    assert_ne!(
        layout,
        unl12_layout(&core),
        "the two data registers moved the layout"
    );

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(layout, unl12_layout(&core), "and the state brought it back");
    assert_eq!(
        state,
        core.save_state(true).unwrap(),
        "the whole block, the index register included"
    );

    // The restored index has to *be* the index: one more fill, with no `$6000` write of its own,
    // must land in Extra Data Register #1.
    core.write_memory(0x0010, 0x07);
    cpu_read(&mut core, 0x6000);
    assert_eq!(
        [0x1C * 1024, 0x1D * 1024, 0x1E * 1024, 0x1F * 1024],
        chr_offsets(&core)[4..],
        "still selecting register #1 after the restore"
    );
}

/// REQ-CORE-090/091, §7.1 item 2: mapper 246's eight registers are at `$6000-$6007` and the
/// generic sweep writes `$6000` alone, so seven of the eight are this test's business — and the
/// battery RAM behind the same window is too (MF-UNL-12 step 4).
#[cfg(feature = "board-246")]
#[test]
fn req_core_090_mapper_246_round_trips_its_eight_bank_registers() {
    let mut core = MapperRom::new(246).prg(64).chr(512).battery().load();
    for register in 0..8u16 {
        core.write_memory(0x6000 + register, 0x11 + register as u8);
    }
    core.write_memory(0x6800, 0x5A); // The battery chip, behind the same window.
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    let layout = unl12_layout(&core);

    for register in 0..8u16 {
        core.write_memory(0x6000 + register, 0x00);
    }
    core.write_memory(0x6800, 0x00);
    assert_ne!(
        layout,
        unl12_layout(&core),
        "the eight registers moved the layout"
    );

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!(state.len(), loaded.consumed);
    assert_eq!(layout, unl12_layout(&core), "and the state brought it back");
    assert_eq!(0x5A, core.read_memory(0x6800), "the battery chip included");
    assert_eq!(
        state,
        core.save_state(true).unwrap(),
        "the whole block, the derived window table included"
    );

    // And the forced-A17 read follows the restored `$6003` and not the window table, which is the
    // one thing in this board that a correct window table could still get wrong.
    assert_eq!(
        0x14 | 0x10,
        core.read_memory(0xFFE4),
        "`$6003` came back as a register, not merely as a window"
    );
}

/// AMD-061's own sweep, `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state`,
/// reaches all three numbers — and for the save-state sweep's reason it dirties almost none of
/// their own registers. This one does, and then asks for the power-on layout back
/// (MF-UNL-12 step 4).
///
/// **Mapper 246 is deliberately not here**, and that is this lot's one exception to the shape:
/// its page says "Register `$6003` is set to `$FF` at power-on **only**", so the board cannot
/// come back to its power-on layout and must not be asked to.
/// `amd_061_mapper_246_keeps_its_last_prg_bank_across_a_reset` is that number's own test and
/// Q-172 §3 the reading.
#[cfg(all(feature = "board-262", feature = "board-292"))]
#[test]
fn amd_061_a_reset_puts_both_mmc3_boards_of_mf_unl_12_back_to_their_power_on_state() {
    for mapper in [262usize, 292] {
        let core = MapperRom::new(mapper).prg(32).chr(512).load();
        let power_on = unl12_layout(&core);

        let mut core = core;
        core.mapper_write(0xA001, 0x80);
        for register in 0..8u8 {
            core.mapper_write(0x8000, register);
            core.mapper_write(0x8001, register + 0x11);
        }
        match mapper {
            262 => core.mapper_write(0x4100, 0x40 | 0x0F),
            292 => {
                dragon_fill(&mut core, 0, 0x33);
                dragon_fill(&mut core, 1, 0x40 | 0x1A);
            }
            _ => {}
        }
        assert_ne!(
            power_on,
            unl12_layout(&core),
            "mapper {mapper}: the board's own registers must move something"
        );

        core.reset();
        assert_eq!(
            power_on,
            unl12_layout(&core),
            "mapper {mapper}: a reset must give the power-on layout back"
        );
        assert!(
            !core.cartridge().unwrap().irq_asserted(),
            "mapper {mapper}: and no pending IRQ"
        );
    }
}
