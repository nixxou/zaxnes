//! Lot MAPPERS-N: one test per board, its expectations worked out from the Mesen2 file each
//! board cites (`cartridges/mappers/mmc3_clones.rs`). The four sweeps of `mapper_tests` over
//! `SUPPORTED_MAPPERS` (load, reset, save-state round trip, PPU bus) cover the rest.
#![allow(clippy::unwrap_used)]

mod common;

#[allow(unused_imports)]
use common::mapper_fixture::{MapperMachine, MapperRom};

/// `MMC3_250.h:9`: the port is A13-A15 and A10, the value the low address byte.
#[cfg(feature = "board-250")]
#[test]
fn mappers_n_250_decodes_port_and_value_from_the_address() {
    let mut core = MapperRom::new(250).prg(16).load();
    core.mapper_write(0x8006, 0x00); // $8000 <- 6
    core.mapper_write(0x8403, 0x00); // $8001 <- 3
    assert_eq!([3, 1, 14, 15], core.prg_windows());
}

/// `MMC3_45.h:57-75`: four registers written in turn; PRG mask and offset.
#[cfg(feature = "board-45")]
#[test]
fn mappers_n_45_masks_and_offsets_prg_from_the_four_registers() {
    let mut core = MapperRom::new(45).prg(64).chr(256).load();
    for value in [0x00, 0x10, 0x0F, 0x30] {
        core.mapper_write(0x6000, value);
    }
    core.mmc3_bank(6, 3);
    assert_eq!([0x13, 0x11, 0x1E, 0x1F], core.prg_windows());
}

/// `MMC3_52.h:40-62`: one register, locked by its bit 7.
#[cfg(feature = "board-52")]
#[test]
fn mappers_n_52_offsets_prg_and_locks() {
    let mut core = MapperRom::new(52).prg(32).chr(256).load();
    core.mapper_write(0xA001, 0x80);
    core.mapper_write(0x6000, 0x89);
    core.mmc3_bank(6, 3);
    assert_eq!([0x13, 0x11, 0x1E, 0x1F], core.prg_windows());
    core.mapper_write(0x6000, 0x0B); // locked: RAM, not the register
    assert_eq!([0x13, 0x11, 0x1E, 0x1F], core.prg_windows());
}

/// `MMC3_44.h:28-42`: `$A001` selects the block.
#[cfg(feature = "board-44")]
#[test]
fn mappers_n_44_selects_the_block_at_a001() {
    let mut core = MapperRom::new(44).prg(64).chr(256).load();
    core.mapper_write(0xA001, 0x02);
    core.mmc3_bank(6, 3);
    assert_eq!([0x23, 0x21, 0x2E, 0x2F], core.prg_windows());
}

/// `MMC3_49.h:36-55`: block in MMC3 mode, a 32 KiB bank otherwise.
#[cfg(feature = "board-49")]
#[test]
fn mappers_n_49_has_a_block_mode_and_a_32_kib_mode() {
    let mut core = MapperRom::new(49).prg(64).chr(256).load();
    core.mapper_write(0xA001, 0x80);
    core.mapper_write(0x6000, 0x41);
    core.mmc3_bank(6, 3);
    assert_eq!([0x13, 0x11, 0x1E, 0x1F], core.prg_windows());
    core.mapper_write(0x6000, 0x20);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
}

/// `MMC3_205.h:33-44`.
#[cfg(feature = "board-205")]
#[test]
fn mappers_n_205_selects_the_block_below_8000() {
    let mut core = MapperRom::new(205).prg(64).chr(512).load();
    core.mapper_write(0x6000, 0x02);
    core.mmc3_bank(6, 3);
    assert_eq!([0x23, 0x21, 0x2E, 0x2F], core.prg_windows());
}

/// `MMC3_134.h:17-42`: `$6001` bit 1 is PRG A18.
#[cfg(feature = "board-134")]
#[test]
fn mappers_n_134_takes_prg_a18_from_6001() {
    let mut core = MapperRom::new(134).prg(64).chr(512).load();
    core.mapper_write(0x6001, 0x02);
    core.mmc3_bank(6, 3);
    assert_eq!([0x23, 0x21, 0x3E, 0x3F], core.prg_windows());
}

/// `MMC3_196.h:20-34`: a write to `$6000-$6FFF` switches to one 32 KiB bank.
#[cfg(feature = "board-196")]
#[test]
fn mappers_n_196_switches_to_a_32_kib_bank() {
    let mut core = MapperRom::new(196).prg(16).chr(256).load();
    core.mapper_write(0x6000, 0x21);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
}

/// `MMC3_197.h:9-21`: R0 drives 4 KiB, R2 and R3 2 KiB each.
#[cfg(feature = "board-197")]
#[test]
fn mappers_n_197_drives_4_and_2_kib_chr() {
    let mut core = MapperRom::new(197).prg(16).chr(64).load();
    core.mmc3_bank(0, 2);
    core.mmc3_bank(2, 4);
    core.mmc3_bank(3, 6);
    assert_eq!([4, 5, 6, 7, 8, 9, 12, 13], core.chr_windows());
}

/// `MMC3_245.h:30-46`: `R0` bit 1 is PRG A19, the fixed banks included.
#[cfg(feature = "board-245")]
#[test]
fn mappers_n_245_takes_prg_a19_from_r0() {
    let mut core = MapperRom::new(245).prg(128).chr_ram(8192).load();
    core.mmc3_bank(0, 2);
    core.mmc3_bank(6, 3);
    assert_eq!([0x43, 0x41, 0x7E, 0x7F], core.prg_windows());
}

/// `MMC3_249.h:21-46`: `$5000` bit 1 scrambles the bank lines.
#[cfg(feature = "board-249")]
#[test]
fn mappers_n_249_scrambles_the_bank_lines() {
    let mut core = MapperRom::new(249).prg(128).chr(256).load();
    core.mmc3_bank(6, 3);
    core.mapper_write(0x5000, 0x02);
    assert_eq!([0x09, 0x01, 0x7E, 0x7F], core.prg_windows());
}

/// `MMC3_238.h:9-24`: the latch at `$4020-$7FFF`.
#[cfg(feature = "board-238")]
#[test]
fn mappers_n_238_reads_back_its_protection_latch() {
    let mut core = MapperRom::new(238).prg(16).chr(256).load();
    core.mapper_write(0x6000, 0x01);
    assert_eq!(0x02, core.read_memory(0x6000));
    core.mapper_write(0x4100, 0x03);
    assert_eq!(0x03, core.read_memory(0x5000));
}

/// `Mapper253.h:87-139`: PRG at exactly `$8010`/`$A010`, the CHR registers a nibble at a time,
/// and banks 4/5 reading the 2 KiB CHR-RAM until `$88` is written to the first register.
#[cfg(feature = "board-253")]
#[test]
fn mappers_n_253_banks_prg_and_maps_chr_ram_on_banks_4_and_5() {
    let mut core = MapperRom::new(253).prg(32).chr(512).load();
    core.mapper_write(0x8010, 5);
    core.mapper_write(0xA010, 7);
    assert_eq!([5, 7, 30, 31], core.prg_windows());
    // $B000/$B004: window 0, low nibble then high nibble (and CHR A12+ from value >> 4).
    core.mapper_write(0xB000, 0x09);
    core.mapper_write(0xB004, 0x10);
    assert_eq!(0x09, core.chr_windows()[0]);
    // Bank 4 is CHR-RAM: a PPU write reads back.
    core.mapper_write(0xB000, 0x04);
    core.mapper_write(0xB004, 0x00);
    core.cartridge_mut().unwrap().ppu_write(0x0010, 0xA5);
    assert_eq!(0xA5, core.chr_bank(0x0010));
    // $88 in the first register forces CHR-ROM.
    core.mapper_write(0xB000, 0x08);
    core.mapper_write(0xB004, 0x08);
    assert_ne!(0xA5, core.chr_bank(0x0010));
}

/// `Mapper253.h:71-85`: the counter steps every 114 CPU cycles and fires on wrapping to zero.
#[cfg(feature = "board-253")]
#[test]
fn mappers_n_253_irq_fires_after_the_counter_wraps() {
    let mut core = MapperRom::new(253).prg(32).chr(512).load();
    core.mapper_write(0xF000, 0x0E);
    core.mapper_write(0xF004, 0x0F); // reload $FE: two steps to wrap
    core.mapper_write(0xF008, 0x02);
    for _ in 0..(114 * 2 + 10) {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
}

/// `Mapper58.h:19-29`: the address is the register; bit 6 picks 16 or 32 KiB.
#[cfg(feature = "board-58")]
#[test]
fn mappers_n_58_banks_from_the_address() {
    let mut core = MapperRom::new(58).prg(16).chr(64).load();
    core.mapper_write(0x8000 | 0x40 | 0x03 | (0x05 << 3), 0);
    assert_eq!([6, 7, 6, 7], core.prg_windows());
    assert_eq!(40, core.chr_windows()[0]);
    core.mapper_write(0x8002, 0);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
}

/// `Mapper241.h:13-16`: the data is a 32 KiB bank.
#[cfg(feature = "board-241")]
#[test]
fn mappers_n_241_switches_32_kib() {
    let mut core = MapperRom::new(241).prg(32).load();
    core.mapper_write(0x8000, 3);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
}

/// `Mapper227.h:14-46`: UNROM-like mode with the last bank of the 128 KiB block fixed.
#[cfg(feature = "board-227")]
#[test]
fn mappers_n_227_has_an_unrom_mode() {
    let mut core = MapperRom::new(227).prg(64).load();
    // prg bank 2, L set, mode 0, S clear: (2, 2 | 7).
    core.mapper_write(0x8000 | (2 << 2) | 0x200, 0);
    assert_eq!([4, 5, 14, 15], core.prg_windows());
}

/// `Bmc235.h:24-55`: with 1 MiB of PRG-ROM, chip select 1 is open bus.
#[cfg(feature = "board-235")]
#[test]
fn mappers_n_235_reads_open_bus_on_a_missing_chip() {
    let mut core = MapperRom::new(235).prg(128).load();
    core.mapper_write(0x8000 | 0x0003, 0);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0x8000 | 0x0100, 0);
    // Open bus: whatever the bus held, not the ROM's bank number pattern.
    let windows = core.prg_windows();
    assert_ne!([0, 1, 2, 3], windows);
}

/// `Mapper230.h:19-50`: power-on in *Contra* mode, RESET switches to the multicart.
#[cfg(feature = "board-230")]
#[test]
fn mappers_n_230_switches_mode_on_reset() {
    let mut core = MapperRom::new(230).prg(64).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.mapper_write(0x8000, 3);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    core.soft_reset();
    assert_eq!([16, 17, 18, 19], core.prg_windows());
    core.soft_reset();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
}

/// `Mapper240.h:9-18`: the register is at `$4020-$5FFF`.
#[cfg(feature = "board-240")]
#[test]
fn mappers_n_240_takes_its_register_below_8000() {
    let mut core = MapperRom::new(240).prg(32).chr(64).load();
    core.mapper_write(0x4100, 0x23);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(24, core.chr_windows()[0]);
}

/// `UnlD1038.h:22-41`: A8 makes `$8000-$FFFF` read the DIP switches (none set).
#[cfg(feature = "board-59")]
#[test]
fn mappers_n_59_reads_its_dip_switches() {
    let mut core = MapperRom::new(59).prg(16).chr(64).load();
    core.mapper_write(0x8000 | 0x80 | 0x20, 0);
    assert_eq!([4, 5, 4, 5], core.prg_windows());
    core.mapper_write(0x8100, 0);
    assert_eq!([0, 0, 0, 0], core.prg_windows());
}

/// `Mapper233.h:14-30`: RESET toggles PRG A20 (the second 1 MiB chip... here bit 5 of the bank).
#[cfg(feature = "board-233")]
#[test]
fn mappers_n_233_toggles_a_bank_bit_on_reset() {
    let mut core = MapperRom::new(233).prg(128).load();
    core.soft_reset();
    assert_eq!([64, 65, 66, 67], core.prg_windows());
}

/// `Bmc51.h:27-39`: in mode 1, `$6000` shows 8 KiB page `$23 | bank << 2`.
#[cfg(feature = "board-51")]
#[test]
fn mappers_n_51_maps_rom_at_6000() {
    let mut core = MapperRom::new(51).prg(64).load();
    core.mapper_write(0x8000, 0x02);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(0x2B, core.read_memory(0x6100));
}

/// `Mapper60.h:18-27`: each RESET selects the next of four games.
#[cfg(feature = "board-60")]
#[test]
fn mappers_n_60_steps_through_its_games_on_reset() {
    let mut core = MapperRom::new(60).prg(8).chr(32).load();
    assert_eq!([0, 1, 0, 1], core.prg_windows());
    core.soft_reset();
    assert_eq!([2, 3, 2, 3], core.prg_windows());
    assert_eq!(8, core.chr_windows()[0]);
}

/// `Mapper120.h:15-27`: `$41FF` banks `$6000-$7FFF`; `$8000` is pages 8-11.
#[cfg(feature = "board-120")]
#[test]
fn mappers_n_120_banks_6000_from_41ff() {
    let mut core = MapperRom::new(120).prg(16).load();
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    core.mapper_write(0x41FF, 5);
    assert_eq!(5, core.read_memory(0x6100));
}

/// `Lh32.h:16-30`: work RAM at `$C000-$DFFF`.
#[cfg(feature = "board-125")]
#[test]
fn mappers_n_125_has_work_ram_at_c000() {
    let mut core = MapperRom::new(125).prg(16).load();
    core.mapper_write(0xC123, 0x5A);
    assert_eq!(0x5A, core.read_memory(0xC123));
    core.mapper_write(0x6000, 3);
    assert_eq!(3, core.read_memory(0x6100));
    assert_eq!(
        [12, 13, 0x5A, 15],
        [
            core.prg_windows()[0],
            core.prg_windows()[1],
            core.read_memory(0xC123),
            core.prg_windows()[3]
        ]
    );
}

/// `Mapper170.h:14-27`: the protection latch.
#[cfg(feature = "board-170")]
#[test]
fn mappers_n_170_reads_back_its_latch() {
    let mut core = MapperRom::new(170).prg(4).load();
    core.mapper_write(0x6502, 0x40);
    assert_eq!(0x80 | 0x77, core.read_memory(0x7777));
}

/// `MMC3_224.h:20-41`: `$5000` bit 2 is PRG A19.
#[cfg(feature = "board-224")]
#[test]
fn mappers_n_224_takes_prg_a19_from_5000() {
    let mut core = MapperRom::new(224).prg(128).chr_ram(8192).load();
    core.mapper_write(0x5000, 0x04);
    core.mmc3_bank(6, 3);
    assert_eq!([0x43, 0x41, 0x7E, 0x7F], core.prg_windows());
}

/// `ResetTxrom.h:9-26`: each RESET moves to the next 128 KiB game.
#[cfg(feature = "board-313")]
#[test]
fn mappers_n_313_steps_through_its_games_on_reset() {
    let mut core = MapperRom::new(313).prg(64).chr(512).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.soft_reset();
    assert_eq!(16 + 14, core.prg_windows()[2]);
}

/// `MMC3_BmcF15.h:14-29`: two 16 KiB banks, written while the work RAM is enabled.
#[cfg(feature = "board-259")]
#[test]
fn mappers_n_259_banks_16_kib_from_its_outer_register() {
    let mut core = MapperRom::new(259).prg(32).chr(256).load();
    core.mapper_write(0xA001, 0x80);
    core.mapper_write(0x6000, 0x03);
    assert_eq!([6, 7, 6, 7], core.prg_windows());
    core.mapper_write(0x6000, 0x0B);
    assert_eq!([20, 21, 22, 23], core.prg_windows());
}

/// `MMC3_Bmc411120C.h:15-33`: the address written at `$6000-$7FFF` is the outer register.
#[cfg(feature = "board-287")]
#[test]
fn mappers_n_287_offsets_prg_from_the_written_address() {
    let mut core = MapperRom::new(287).prg(64).chr(512).load();
    core.mapper_write(0x6001, 0);
    core.mmc3_bank(6, 3);
    assert_eq!([0x13, 0x11, 0x1E, 0x1F], core.prg_windows());
    core.mapper_write(0x6008 | 0x10, 0);
    assert_eq!([0x34, 0x35, 0x36, 0x37], core.prg_windows());
}

/// `Mapper40.h:20-41`: `$A000` arms 4096 CPU cycles, `$8000` acknowledges.
#[cfg(feature = "board-40")]
#[test]
fn mappers_n_40_fires_after_4096_cycles() {
    let mut core = MapperRom::new(40).prg(8).load();
    assert_eq!([4, 5, 0, 7], core.prg_windows());
    assert_eq!(6, core.read_memory(0x6100));
    core.mapper_write(0xA000, 0);
    for _ in 0..4100 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
    core.mapper_write(0x8000, 0);
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// `Mapper42.h:15-60`: the last 32 KiB fixed, `$E000` banks `$6000`.
#[cfg(feature = "board-42")]
#[test]
fn mappers_n_42_banks_6000() {
    let mut core = MapperRom::new(42).prg(16).chr(64).load();
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0xE000, 3);
    assert_eq!(3, core.read_memory(0x6100));
}

/// `Mapper106.h:31-49`: sixteen ports under mask `$000F`.
#[cfg(feature = "board-106")]
#[test]
fn mappers_n_106_maps_its_sixteen_ports() {
    let mut core = MapperRom::new(106).prg(64).chr(256).load();
    core.mapper_write(0x8008, 0x02);
    core.mapper_write(0x8009, 0x05);
    assert_eq!([0x12, 0x05], [core.prg_windows()[0], core.prg_windows()[1]]);
    core.mapper_write(0x8004, 0x21);
    assert_eq!(0x21, core.chr_windows()[4]);
}

/// `FrontFareast.h`: format 6 powers on with 16 KiB pages 0 and 7, and a `$8000` write in alt
/// mode banks PRG from the high six bits and CHR from the low two.
#[cfg(feature = "board-6")]
#[test]
fn mappers_n_6_banks_from_one_port() {
    let mut core = MapperRom::new(6).prg(16).chr(64).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.mapper_write(0x8000, (3 << 2) | 0x02);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    assert_eq!(16, core.chr_windows()[0]);
}

/// `FrontFareast.h`: format 17's ports at `$4504-$4517`, and the shared 16-bit IRQ counter.
#[cfg(feature = "board-17")]
#[test]
fn mappers_n_17_ports_and_irq() {
    let mut core = MapperRom::new(17).prg(16).chr(64).load();
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0x4505, 3);
    core.mapper_write(0x4512, 9);
    assert_eq!(3, core.prg_windows()[1]);
    assert_eq!(9, core.chr_windows()[2]);
    core.mapper_write(0x4502, 0xF0);
    core.mapper_write(0x4503, 0xFF);
    for _ in 0..0x20 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
}

/// `Fk23C.h`: `$5000`/`$5001` give the PRG mode and outer bank; `$A001` bit 5 banks 32 KiB of
/// WRAM over `$6000` and the next bank over `$4000-$5FFF`.
#[cfg(feature = "board-176")]
#[test]
fn mappers_n_176_outer_bank_and_banked_wram() {
    let mut core = MapperRom::new(176).prg(64).chr(256).load();
    assert_eq!([0, 1, 62, 63], core.prg_windows());
    core.mapper_write(0x5010, 0x01);
    core.mapper_write(0x5011, 0x10);
    assert_eq!([0x20, 0x21, 0x3E, 0x3F], core.prg_windows());
    core.mapper_write(0xA001, 0x21);
    core.mapper_write(0x6000, 0x55);
    core.mapper_write(0x4100, 0x66);
    assert_eq!(0x55, core.read_memory(0x6000));
    assert_eq!(0x66, core.read_memory(0x4100));
    core.mapper_write(0xA001, 0x22);
    assert_eq!(0x66, core.read_memory(0x6100));
}

/// `Kaiser202.h`: register 4 switches `$6000` to PRG-ROM bank R3; the counter fires at `$FFFF`.
#[cfg(feature = "board-142")]
#[test]
fn mappers_n_142_switches_6000_to_rom_and_counts() {
    let mut core = MapperRom::new(142).prg(16).chr_ram(8192).load();
    core.mapper_write(0xE000, 4); // register 3
    core.mapper_write(0xF000, 9);
    core.mapper_write(0xE000, 5); // register 4: use ROM
    core.mapper_write(0xF000, 0x04);
    assert_eq!(9, core.read_memory(0x6100));
    core.mapper_write(0xB000, 0x0F);
    core.mapper_write(0xA000, 0x0F);
    core.mapper_write(0x9000, 0x0F);
    core.mapper_write(0x8000, 0x08); // reload $FFF8
    core.mapper_write(0xC000, 0x02);
    for _ in 0..16 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
}

/// `Kaiser7031.h`: `$8000-$FFFF` shows 2 KiB pages 15 down to 0, `$6000-$7FFF` four registers.
#[cfg(feature = "board-305")]
#[test]
fn mappers_n_305_maps_2_kib_pages() {
    let mut core = MapperRom::new(305).prg(8).load();
    // 2 KiB page 15 lies in 8 KiB bank 3; page 0 in bank 0.
    assert_eq!(3, core.read_memory(0x8100));
    assert_eq!(0, core.read_memory(0xF900));
    core.mapper_write(0x8800, 9); // register 1: $6800-$6FFF, page 9 in bank 2
    assert_eq!(2, core.read_memory(0x6900));
}

/// `MMC3_254.h:15-28`: work RAM reads are exclusive-ored with `$A001` until `$8000` is written.
#[cfg(feature = "board-254")]
#[test]
fn mappers_n_254_xors_work_ram_reads() {
    let mut core = MapperRom::new(254).prg(16).chr(64).load();
    core.mapper_write(0xA001, 0x80);
    core.mapper_write(0x6000, 0x0F);
    assert_eq!(0x0F ^ 0x80, core.read_memory(0x6000));
    core.mapper_write(0x8000, 0x00);
    assert_eq!(0x0F, core.read_memory(0x6000));
}

/// `Edu2000.h:18-28`: bits 6-7 of the register bank 32 KiB of work RAM at `$6000`.
#[cfg(feature = "board-329")]
#[test]
fn mappers_n_329_banks_its_work_ram() {
    let mut core = MapperRom::new(329).prg(16).load();
    core.mapper_write(0x6000, 0x11);
    core.mapper_write(0x8000, 0x41);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0x6000, 0x22);
    core.mapper_write(0x8000, 0x01);
    assert_eq!(0x11, core.read_memory(0x6000));
    core.mapper_write(0x8000, 0x41);
    assert_eq!(0x22, core.read_memory(0x6000));
}

/// nesdev ("INES Mapper 108") and puNES `mapper_108.c`: submapper 1's register is `$F000-$FFFF`
/// only, so a write at `$A000` (which Mesen2 `Bb.h` takes) leaves `$6000` alone; the register
/// powers on at 0. An iNES 1.0 image without CHR-ROM and with horizontal mirroring is submapper 1.
#[cfg(feature = "board-108")]
#[test]
fn mappers_n_108_submapper_register_ranges() {
    for rom in [
        MapperRom::new(108).prg(16).submapper(1),
        MapperRom::new(108).prg(16),
    ] {
        let mut core = rom.load();
        assert_eq!(0, core.read_memory(0x6000));
        core.mapper_write(0xA000, 3);
        assert_eq!(0, core.read_memory(0x6000));
        core.mapper_write(0xF000, 5);
        assert_eq!(5, core.read_memory(0x6000));
        assert_eq!([12, 13, 14, 15], core.prg_windows());
    }
    // Submapper 3 takes `$8000`; submapper 4 keeps `$6000` on the last bank.
    let mut core = MapperRom::new(108).prg(16).submapper(3).load();
    core.mapper_write(0x8000, 6);
    assert_eq!(6, core.read_memory(0x6000));
    let mut core = MapperRom::new(108).prg(4).chr(16).submapper(4).load();
    core.mapper_write(0x8000, 1);
    assert_eq!(3, core.read_memory(0x6100));
    assert_eq!([8, 9, 10, 11, 12, 13, 14, 15], core.chr_windows());
}

/// Mesen2 `Fk23C.h:160`: the `$5xx0` CHR-RAM bit selects CHR-RAM only when the cartridge has
/// some ("_selectChrRam && _chrRamSize > 0"); a NES 2.0 image declaring none keeps reading its
/// CHR-ROM (Zhi Li Xiao Zhuang Yuan writes `$5FF0 = $60` and draws from CHR-ROM).
#[cfg(feature = "board-176")]
#[test]
fn mappers_n_176_chr_ram_bit_without_chr_ram() {
    let mut core = MapperRom::new(176)
        .prg(16)
        .chr(64)
        .prg_ram(8192, 0)
        .chr_ram(0)
        .load();
    core.mapper_write(0x5FF0, 0x60);
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
}
