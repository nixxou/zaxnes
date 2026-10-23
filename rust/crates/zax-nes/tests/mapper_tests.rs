//! Mapper behaviour on numbered-bank images (REQ-CORE-100, REQ-CORE-102, REQ-CORE-039,
//! REQ-CORE-033, AMD-021). Save state and battery import/export tests of the C# suite
//! belong to the `NesCore` facade (T05).

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

fn peek(core: &zax_nes::NesCore, address: u16) -> u8 {
    core.cartridge()
        .unwrap()
        .ppu_peek(address, ChrFetch::Background)
}

#[test]
fn req_core_100_every_required_mapper_is_registered() {
    let required: [i32; 31] = [
        0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 13, 15, 16, 18, 21, 22, 23, 25, 27, 28, 30, 32, 34, 41, 64,
        66, 71, 87, 101, 225, 228,
    ];
    assert_eq!(&required[..], Cartridge::SUPPORTED_MAPPERS);
}

#[test]
fn req_core_102_any_other_mapper_is_refused() {
    // 116 is the one REQ-CORE-101 leaves optional: not implemented, so refused with the
    // message of REQ-CORE-011 like any other number outside the table.
    for mapper in [6usize, 19, 69, 116, 200] {
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
        core.write_memory(0xE000, 0xFF); // Same cycle as the write above: dropped.
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
    // 8x8 sprites: the set written last serves everything.
    core.write_memory(0x2000, 0x00);
    assert_eq!(16, peek(&core, 0x0000, ChrFetch::Background));
    assert_eq!(16, peek(&core, 0x0000, ChrFetch::Sprite));
    // 8x16 sprites: background from $5128-$512B, sprites from $5120-$5127.
    core.write_memory(0x2000, 0x20);
    assert_eq!(16, peek(&core, 0x0000, ChrFetch::Background));
    assert_eq!(0, peek(&core, 0x0000, ChrFetch::Sprite));
    assert_eq!(4, peek(&core, 0x1000, ChrFetch::Sprite));
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

// ---------------------------------------------------------------- the small boards

#[test]
fn req_core_100_color_dreams_splits_its_register_between_prg_and_chr() {
    let mut core = MapperRom::new(11).prg(16).chr(64).load();
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
    // AMD-021 R2-002: layout 1 fixes the last 16 KiB bank of the whole image at $C000, not
    // the last bank of a 64 KiB block.
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
fn amd_021_r2_002_mapper_15_acceptance_vectors() {
    // A numbered 1 MiB image, 64 banks of 16 KiB, so 16 KiB bank n is the pair of 8 KiB banks
    // (2n, 2n + 1).
    let mut core = MapperRom::new(15).prg(128).chr(8).load();
    // Power-on shows the first 32 KiB.
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    // $8001 <- $05: layout 1, B = 5 -> 5 / 63 (the last 16 KiB bank of the image).
    core.write_memory(0x8001, 0x05);
    assert_eq!([10, 11, 126, 127], core.prg_windows());
    // $8002 <- $85: layout 2, B = 5, h = 1 -> 8 KiB bank 11 in all four windows.
    core.write_memory(0x8002, 0x85);
    assert_eq!([11, 11, 11, 11], core.prg_windows());
    // $8000 <- $40: bit 6 set -> horizontal.
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
    for mapper in [21usize, 22, 23, 25, 27] {
        let mut core = MapperRom::new(mapper).prg(16).chr(32).load();
        core.write_memory(0x8000, 0x03);
        core.write_memory(0xA000, 0x05);
        assert_eq!([3, 5, 14, 15], core.prg_windows(), "mapper {mapper}");
        // $9002 bit 1 moves the first switchable bank to $C000.
        core.write_memory(0x9000 | vrc_register(mapper, 2), 0x02);
        assert_eq!([14, 5, 3, 15], core.prg_windows(), "mapper {mapper}");
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
    let mut core = MapperRom::new(30).prg(32).chr(0).mirroring(0x08).load();
    core.write_memory(0x8000, 0x45); // PRG bank 5, CHR-RAM bank 2, screen A.
    assert_eq!([10, 11, 30, 31], core.prg_windows());
    assert_eq!(Nm::SingleScreenA, core.mirroring());
    core.write_memory(0x8000, 0xC5);
    assert_eq!(Nm::SingleScreenB, core.mirroring());
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
    let mut core = MapperRom::new(34).prg(16).chr(0).load();
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
    core.write_memory(0x6002 | 0x04 | 0x08, 0x00); // PRG 32 KiB bank 2, horizontal, CHR high 1.
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
    core.mapper_write(0x8000, 0x40); // PRG mode 1 rotates the three windows.
    assert_eq!([9, 3, 5, 15], core.prg_windows());
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
    let mut core = MapperRom::new(66).prg(16).chr(32).load();
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
fn amd_021_r2_003_mapper_101_decodes_address_bits_13_and_14() {
    // A numbered 32 KiB CHR image: 8 KiB CHR bank n is the run of 1 KiB banks starting at 8n.
    let mut core = MapperRom::new(101).prg(4).chr(32).load();
    // $6000 <- $06 -> bank 6 & 3 = 2.
    core.write_memory(0x6000, 0x06);
    assert_eq!(2 * 8, core.chr_bank(0x0000));
    // $E000 <- $03 -> bank 3: bits 13 and 14 of $E000 are both set.
    core.write_memory(0xE000, 0x03);
    assert_eq!(3 * 8, core.chr_bank(0x0000));
    // $8000 <- $01: bits 13 and 14 clear, no change.
    core.write_memory(0x8000, 0x01);
    assert_eq!(3 * 8, core.chr_bank(0x0000));
    // $A000 and $C000 carry only one of the two bits, so they are not decoded either.
    core.write_memory(0xA000, 0x01);
    core.write_memory(0xC000, 0x02);
    assert_eq!(3 * 8, core.chr_bank(0x0000));
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
