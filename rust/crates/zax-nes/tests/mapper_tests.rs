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

#[test]
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
    // * 78, the Irem IF-12 and the Jaleco board of Uchuusen (MF-3 step 7).
    let extra: Vec<i32> = Cartridge::SUPPORTED_MAPPERS
        .iter()
        .copied()
        .filter(|mapper| !required.contains(mapper))
        .collect();
    assert_eq!(vec![69, 78, 118, 156, 180], extra);

    // Ascending, which is what the registry's own comment promises.
    assert!(Cartridge::SUPPORTED_MAPPERS.windows(2).all(|p| p[0] < p[1]));
}

#[test]
fn req_core_102_any_other_mapper_is_refused() {
    // 116 is the one REQ-CORE-101 leaves optional: not implemented, so refused with the
    // message of REQ-CORE-011 like any other number outside the table.
    for mapper in [6usize, 19, 116, 200] {
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

    assert_eq!(core.export_battery_ram(), []);
    assert!(!core.import_battery_ram(&[1, 2, 3]));
}
