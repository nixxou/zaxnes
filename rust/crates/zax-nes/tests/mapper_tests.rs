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
/// Kids board (96) it is the 4 KiB CHR-RAM page the PPU address bus last latched, and on the
/// GTROM (111) it is the 8 KiB CHR half of a 32 KiB chip whose other half is the nametables.
#[cfg(any(
    feature = "board-77",
    feature = "board-96",
    feature = "board-111",
    feature = "board-119"
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
    //   `IrqBoard` variant -- behind `board-mmc3-scramble` (MF-UNL-2 step 3).
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
    let additions: [(i32, bool); 73] = [
        (19, true),
        (24, true),
        (26, true),
        (33, small),
        (36, jv001),
        (37, multicart),
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
        (80, taito_x1),
        (82, taito_x1),
        (85, true),
        (86, jaleco),
        (88, namcot),
        (89, small),
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
        (111, cfg!(feature = "board-111")),
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
        (140, small),
        (147, jv001),
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
        (172, jv001),
        (173, jv001),
        (180, true),
        (182, scramble),
        (184, small),
        (185, small),
        (187, scramble),
        (192, chrram),
        (194, chrram),
        (195, chrram),
        (206, namcot),
        (207, taito_x1),
        (210, true),
        (232, small),
        (243, sachen),
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
#[cfg(feature = "board-111")]
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
/// where the bank's four bits sit in the byte, and **123's bit order is the page's**: its
/// diagram `[.M31 .2S0]` puts the bank's bits 3, 2, 1 and 0 at register bits 5, 2, 4 and 0, and
/// the page flags it with "(notice the bit order)". Q-144 records that Mesen 2 reads register
/// bit 3 where the page's diagram has bit 4.
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
/// 195's own page names one more thing this build takes from the header rather than the board:
/// "A compatible circuit board (with unknown PCB code) adds 4 KiB of non-battery-backed PRG-RAM
/// (recognizable as such in the NES 2.0 header) mapped to CPU $5000-$5FFF" — *天神之剑*. That
/// window is **not** implemented and the reason is the page's own parenthesis: the header is the
/// only indicator and this core has no $5000-$5FFF RAM window on any board. The test pins what
/// the board does carry instead.
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
#[cfg(feature = "board-116")]
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
