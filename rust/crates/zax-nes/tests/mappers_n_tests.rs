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

/// MR-34-1, NINA-001: a `$7FFD` write switches the 32 KiB bank **and** reaches the PRG-RAM
/// underneath, and the CPU reads the new bank at once. The register used to be answered as
/// unhandled, so the write took the plain PRG-RAM path, which does not refresh the cached read
/// windows; Impossible Mission II, which switches banks from code it runs in that RAM, crashed.
#[test]
fn mappers_n_34_nina_switch_refreshes_the_read_windows() {
    let mut core = MapperRom::new(34).prg(8).chr(16).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    core.mapper_write(0x7FFD, 1);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(1, core.read_memory(0x7FFD));
    // The CPU's own read path, through the cached windows.
    let cartridge = core.cartridge().unwrap();
    assert_eq!(Some(4), cartridge.read_rom_window(0x8100));
    assert_eq!(Some(7), cartridge.read_rom_window(0xE100));
}

/// nesdev ("INES Mapper 015"): the images "claiming mapper 15" that are not the K-1029
/// multicarts are mapper hacks, for which emulators "must not enforce CHR-RAM
/// write-protection". An image declaring battery RAM is one of them, so mode 0 writes CHR-RAM
/// there (Bao Xiao San Guo); the K-1029 keeps MR-15-1's protection
/// (`mr_15_1_mapper_15_write_protects_chr_ram_in_modes_0_and_3`).
#[test]
fn mappers_n_15_hack_images_do_not_write_protect_chr_ram() {
    let mut core = MapperRom::new(15).prg(16).chr(0).battery().load();
    for mode in [0u16, 3] {
        core.mapper_write(0x8000 | mode, 0x00);
        let value = 0x30 | u8::try_from(mode).unwrap();
        core.cartridge_mut().unwrap().ppu_write(0x0000, value);
        assert_eq!(value, core.chr_bank(0x0000), "mode {mode}");
    }
}

/// nesdev ("INES Mapper 111") and puNES `mapper_111.c`: with CHR-ROM, mapper 111 is a
/// non-serialized MMC1 — one write loads a whole register, A13-A14 picking it — whose 6-bit CHR
/// banks reach 256 KiB. Without CHR-ROM it stays GTROM.
#[cfg(feature = "board-111")]
#[test]
fn mappers_n_111_with_chr_rom_is_a_parallel_mmc1() {
    let mut core = MapperRom::new(111).prg(16).chr(256).load();
    core.mapper_write(0x8000, 0x1C); // PRG mode 3, 4 KiB CHR
    core.mapper_write(0xA000, 0x3F); // CHR 0: the last 4 KiB of 256 KiB
    core.mapper_write(0xC000, 0x01);
    core.mapper_write(0xE000, 0x05);
    assert_eq!([252, 253, 254, 255, 4, 5, 6, 7], core.chr_windows());
    assert_eq!([10, 11, 14, 15], core.prg_windows());
}

/// Mesen2 `Mapper234.h` and puNES `mapper_234.c`: a CPU **read** of `$FF80-$FF9F` latches the
/// ROM byte under it into register 0 (here 3, bank 3 of the fixture's last page, so CNROM mode and
/// 32 KiB bank 3), which then locks while its low six bits are nonzero; register 1 at
/// `$FFE8-$FFF7` always takes the byte.
#[cfg(feature = "board-234")]
#[test]
fn mappers_n_234_latches_on_reads_and_locks() {
    let mut core = MapperRom::new(234).prg(32).chr(64).load();
    let latched = core.cartridge_mut().unwrap().try_cpu_read(0xFF80).unwrap();
    let bank = usize::from(latched & 0x0F) * 4;
    let expected: [u8; 4] = std::array::from_fn(|i| u8::try_from((bank + i) % 32).unwrap());
    assert_eq!(expected, core.prg_windows());
    // Locked: a later read of another register-0 address changes nothing.
    if latched & 0x3F != 0 {
        core.cartridge_mut().unwrap().try_cpu_read(0xFF90);
        assert_eq!(expected, core.prg_windows());
    }
}

/// puNES `mapper_252.c`: a CHR register value whose bits under `$FE` equal `$06` (6 or 7) maps
/// the 2 KiB CHR-RAM; writing CHR through a window holding `$C8` moves the selection to `$04`/`$05`.
/// Mesen2 `Waixing252.h` gives the register decode: CHR 0 at `$B000`/`$B004`.
#[cfg(feature = "board-252")]
#[test]
fn mappers_n_252_selects_chr_ram_by_mask_and_compare() {
    let mut core = MapperRom::new(252).prg(32).chr(128).load();
    core.mapper_write(0xB000, 0x06); // CHR 0 = 6: CHR-RAM page 0
    core.cartridge_mut().unwrap().ppu_write(0x0000, 0xA5);
    assert_eq!(0xA5, core.chr_bank(0x0000));
    // CHR 1 = $C8, then a CHR write through it switches the compare to $04.
    core.mapper_write(0xB008, 0x08);
    core.mapper_write(0xB00C, 0x0C);
    core.cartridge_mut().unwrap().ppu_write(0x0400, 0x00);
    assert_eq!(6, core.chr_bank(0x0000), "6 is CHR-ROM again");
    core.mapper_write(0xB000, 0x04);
    assert_eq!(0xA5, core.chr_bank(0x0000), "4 now maps CHR-RAM page 0");
}

/// Mesen2 `MMC3_165.h`: the left half shows R0 or R1 by its MMC2-style latch, in 4 KiB pages,
/// switched by a fetch of `$0FD8`/`$0FE8` and applied at the next PPU address; page 0 is the
/// 4 KiB CHR-RAM.
#[cfg(feature = "board-165")]
#[test]
fn mappers_n_165_latches_its_chr_halves_and_maps_page_0_to_ram() {
    let mut core = MapperRom::new(165).prg(32).chr(256).load();
    core.mmc3_bank(0, 4); // R0: 4 KiB page 1
    core.mmc3_bank(1, 8); // R1: 4 KiB page 2
    assert_eq!([4, 5, 6, 7], core.chr_windows()[..4]);
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x0FE8, 10);
    cartridge.on_ppu_address_bus(0x0000, 20);
    assert_eq!([8, 9, 10, 11], core.chr_windows()[..4]);
    // R1 = 0: the left half is the CHR-RAM.
    core.mmc3_bank(1, 0);
    core.cartridge_mut().unwrap().ppu_write(0x0123, 0x5A);
    assert_eq!(
        0x5A,
        core.cartridge()
            .unwrap()
            .ppu_peek(0x0123, zax_nes::cartridges::ChrFetch::Background)
    );
}

/// nesdev ("INES Mapper 198") and puNES `mapper_198.c`: 640 KiB of PRG, bank numbers from `$50`
/// folding onto `$40-$4F`, the MMC3's fixed pages on `$4E`/`$4F`, and 4 KiB of RAM at
/// `$5000-$5FFF` apart from the WRAM at `$6000`.
#[cfg(feature = "board-198")]
#[test]
fn mappers_n_198_folds_its_banks_and_has_ram_at_5000() {
    let mut core = MapperRom::new(198).prg(80).chr_ram(8192).load();
    core.mmc3_bank(6, 0x45);
    core.mmc3_bank(7, 0x5A); // folds to $4A
    assert_eq!([0x45, 0x4A, 0x4E, 0x4F], core.prg_windows());
    core.mapper_write(0xA001, 0x80); // WRAM on
    core.mapper_write(0x5123, 0x11);
    core.mapper_write(0x6123, 0x22);
    assert_eq!(0x11, core.read_memory(0x5123));
    assert_eq!(0x22, core.read_memory(0x6123));
}

/// nesdev ("INES Mapper 199"): the FS309's 8 KiB of CHR-RAM are unbanked, and `$5000-$5FFF` is
/// page 2 of its 32 KiB of PRG-RAM (puNES `wram_fix_mmc3_199`). No genuine dump was at hand: the
/// Hackset's "199" images are mapper-176 games, as the page warns some are.
#[cfg(feature = "board-199")]
#[test]
fn mappers_n_199_has_unbanked_chr_ram_and_ram_at_5000() {
    let mut core = MapperRom::new(199)
        .prg(64)
        .chr(0)
        .chr_ram(8192)
        .battery()
        .load();
    core.mmc3_bank(0, 0x10);
    core.cartridge_mut().unwrap().ppu_write(0x0000, 0x77);
    core.mmc3_bank(0, 0x20);
    assert_eq!(0x77, core.chr_bank(0x0000));
    core.mapper_write(0xA001, 0x80);
    core.mapper_write(0x5010, 0x33);
    assert_eq!(0x33, core.read_memory(0x5010));
    assert_eq!(0x33, core.cartridge().unwrap().battery_ram()[0x2010]);
}

/// nesdev "INES Mapper 168" and puNES `mapper_168.c`: PRG bank from bits 7-6, CHR bank
/// `(reg & $0F) ^ 8`, U2 (banks 8-15) protected until the control bit falls, and the IRQ on bit
/// 10 of a free-running M2 counter.
#[cfg(feature = "board-168")]
#[test]
fn mappers_n_168_banks_protects_u2_and_counts() {
    let mut core = MapperRom::new(168).prg(8).chr(0).load();
    assert_eq!([0, 1, 6, 7], core.prg_windows());
    core.mapper_write(0xB000, 0x83); // PRG bank 2, CHR bank 3 ^ 8 = 11, in U2
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.cartridge_mut().unwrap().ppu_write(0x1000, 0x55);
    assert_eq!(0, core.chr_bank(0x1000), "U2 protected at power-on");
    core.mapper_write(0xF080, 0xFF); // control bit high: frozen
    core.mapper_write(0xF000, 0x00); // 1 -> 0: protection off, counter runs
    core.cartridge_mut().unwrap().ppu_write(0x1000, 0x55);
    assert_eq!(0x55, core.chr_bank(0x1000));
    assert!(!core.cartridge().unwrap().irq_asserted());
    for _ in 0..1100 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
    core.mapper_write(0xF080, 0xFF);
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// nesdev "INES Mapper 188" and Mesen2 `BandaiKaraoke.h`: bit 4 picks the internal ROM (`& 7`) or
/// the expansion (`| 8`), `$C000` is the internal ROM's last bank, bit 5 the mirroring, and
/// `$6000-$7FFF` reads the microphone's two buttons active low.
#[cfg(feature = "board-188")]
#[test]
fn mappers_n_188_banks_both_roms_and_reads_the_microphone() {
    // Bus conflicts: the writes go to the fixed bank, filled with $FF.
    let mut core = MapperRom::new(188)
        .prg(32)
        .chr(0)
        .prg_bank_all_ones(14)
        .load();
    core.mapper_write(0xC000, 0x13);
    assert_eq!([6, 7, 0xFF, 15], core.prg_windows());
    core.mapper_write(0xC000, 0x02);
    assert_eq!([20, 21, 0xFF, 15], core.prg_windows());
    assert_eq!(0x03, core.read_memory(0x6000) & 0x07);
    core.run_frame(0x01); // controller 1's A
    assert_eq!(0x02, core.read_memory(0x6000) & 0x07);
}

/// nesdev "INES Mapper 031": eight 4 KiB pages written at `$5000-$5FFF` (A0-A2), `$5FFF` set to
/// `$FF` at power-on. The fixture fills each 8 KiB bank with its number.
#[cfg(feature = "board-31")]
#[test]
fn mappers_n_31_banks_4_kib_pages_from_5000() {
    let mut core = MapperRom::new(31).prg(16).chr(0).load();
    assert_eq!(15, core.read_memory(0xF010));
    core.mapper_write(0x5FF8, 3);
    assert_eq!(1, core.read_memory(0x8010));
    core.mapper_write(0x5001, 10);
    assert_eq!(5, core.read_memory(0x9010));
}

/// Mesen2 `Mapper103.h` and puNES `mapper_103.c`: the last 32 KiB at `$8000`, RAM at `$6000`
/// and over `$B800-$D7FF` until `$F000` bit 4 maps ROM there, the RAM written either way.
#[cfg(feature = "board-103")]
#[test]
fn mappers_n_103_overlays_work_ram_and_writes_it_always() {
    let mut core = MapperRom::new(103).prg(16).chr(0).load();
    assert_eq!(12, core.read_memory(0x8010));
    core.mapper_write(0xB800, 0x77);
    assert_eq!(0x77, core.read_memory(0xB800));
    core.mapper_write(0x8000, 5);
    core.mapper_write(0xF000, 0x10);
    assert_eq!(13, core.read_memory(0xB800));
    assert_eq!(5, core.read_memory(0x6010));
    core.mapper_write(0x6001, 0x99);
    core.mapper_write(0xF000, 0x00);
    assert_eq!(0x99, core.read_memory(0x6001));
    assert_eq!(0x77, core.read_memory(0xB800));
}

/// Mesen2 `Kaiser7022.h` and FCEUX `175.cpp`: `$A000` holds a bank that takes effect, PRG and
/// CHR together, when the CPU reads `$FFFC`.
#[cfg(feature = "board-175")]
#[test]
fn mappers_n_175_applies_its_bank_on_a_read_of_fffc() {
    let mut core = MapperRom::new(175).prg(16).chr(64).load();
    core.mapper_write(0xA000, 3);
    assert_eq!(0, core.read_memory(0x8010));
    core.cartridge_mut().unwrap().try_cpu_read(0xFFFC);
    assert_eq!(6, core.read_memory(0x8010));
    assert_eq!(6, core.read_memory(0xC010));
    assert_eq!(24, core.chr_bank(0x0000));
}

/// Mesen2 `Kaiser7037.h` and puNES `mapper_307.c`: 4 KiB layout RAM, ROM 15, R6, ROM -4, RAM,
/// R7, last 8 KiB; registers through `$8000`/`$8001`; a CIRAM page per table.
#[cfg(feature = "board-307")]
#[test]
fn mappers_n_307_mixes_rom_and_ram_and_pages_each_table() {
    let mut core = MapperRom::new(307).prg(16).chr(0).load();
    for (index, value) in [(6, 3), (7, 9), (2, 1), (5, 1)] {
        core.mapper_write(0x8000, index);
        core.mapper_write(0x8001, value);
    }
    assert_eq!(3, core.read_memory(0x8010));
    assert_eq!(14, core.read_memory(0xA010));
    assert_eq!(9, core.read_memory(0xC010));
    assert_eq!(7, core.read_memory(0x7010));
    core.mapper_write(0xB010, 0x55);
    core.mapper_write(0x6010, 0x66);
    assert_eq!(
        (0x55, 0x66),
        (core.read_memory(0xB010), core.read_memory(0x6010))
    );
    let board = core.cartridge().unwrap();
    assert_eq!(
        [1, 0, 0, 1],
        [0, 1, 2, 3].map(|table| board.nametable_page(table))
    );
}

/// Mesen2 `Kaiser7017.h` and puNES `mapper_303.c`: the bank from the address of a `$4A00-$4AFF`
/// write, mapped by a `$5100` write, `$C000` bank 2; a 16-bit counter from `$4020`/`$4021` that
/// raises the IRQ at 0, read and acknowledged at `$4030`.
#[cfg(feature = "board-303")]
#[test]
fn mappers_n_303_banks_by_address_and_counts_down() {
    let mut core = MapperRom::new(303).prg(16).chr(0).load();
    assert_eq!([0, 1, 4, 5], core.prg_windows());
    core.mapper_write(0x4A4C, 0); // (A >> 2) & 3 = 3, (A >> 4) & 4 = 4: bank 7
    assert_eq!([0, 1, 4, 5], core.prg_windows());
    core.mapper_write(0x5100, 0);
    assert_eq!([14, 15, 4, 5], core.prg_windows());
    core.mapper_write(0x4020, 0x10);
    core.mapper_write(0x4021, 0x00);
    for _ in 0..0x20 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
    assert_eq!(Some(1), core.cartridge_mut().unwrap().try_cpu_read(0x4030));
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// Mesen2 `Supervision.h`: power-on at the last 32 KiB pair ($80/$81 of 16 KiB, wrapped); `$6000`
/// writes the outer register, `$8000` the inner; `$6000-$7FFF` reads ROM.
#[cfg(feature = "board-53")]
#[test]
fn mappers_n_53_banks_from_6000_and_8000() {
    let mut core = MapperRom::new(53).prg(64).chr(0).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    core.mapper_write(0x6000, 0x11); // game mode, outer 1
    core.mapper_write(0x8000, 0x02);
    assert_eq!([20, 21, 30, 31], core.prg_windows());
    assert_eq!(31, core.read_memory(0x6010)); // ((8 << 1) | $0F) = $1F
}

/// Mesen2 `Mapper487.h`: outer `$4180`, NINA `$4100`, Color Dreams `$8000` in the second half.
#[cfg(feature = "board-487")]
#[test]
fn mappers_n_487_switches_nina_and_color_dreams_halves() {
    let mut core = MapperRom::new(487).prg(128).chr(1024).load();
    core.mapper_write(0x4180, 0x04);
    core.mapper_write(0x4100, 0x03);
    assert_eq!([16, 17, 18, 19], core.prg_windows());
    assert_eq!(19 * 8, usize::from(core.chr_windows()[0])); // CHR (4 << 2) | 3, in 1 KiB
    core.mapper_write(0x4180, 0x24); // second half
    core.mapper_write(0x8000, 0x11); // Color Dreams: inner register (1 << 3) | 1
    assert_eq!((4 + 0x10) * 4, core.prg_windows()[0]); // 32 KiB 4, 512 KiB further on
}

/// Mesen2 `Dance2000.h`: `$5000` PRG (16 KiB, `$C000` bank 0), `$5200` mode; bit 6 of `$5000`
/// opens the bus over `$8000-$FFFF`; mode bit 1 makes the nametable address pick CHR page 0.
#[cfg(feature = "board-518")]
#[test]
fn mappers_n_518_opens_the_bus_and_follows_the_nametable() {
    let mut core = MapperRom::new(518).prg(16).chr(0).load();
    core.mapper_write(0x5000, 0x03);
    assert_eq!([6, 7, 0, 1], core.prg_windows());
    core.mapper_write(0x5200, 0x06); // 32 KiB mode and the nametable switch
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.ppu_write(0x1000, 0x5A); // page 1
    cartridge.on_ppu_address_bus(0x2C00, 0);
    assert_eq!(0x5A, core.chr_bank(0x0000));
    core.mapper_write(0x5000, 0x43);
    assert_eq!(None, core.cartridge_mut().unwrap().try_cpu_read(0x8000));
}

/// Mesen2 `Eh8813A.h`: the bank in the address of a write with A8 clear, CHR from the value; A6
/// replaces the low nibble of every read address with the DIP switches (none set).
#[cfg(feature = "board-519")]
#[test]
fn mappers_n_519_banks_by_address_and_reads_the_dip_switches() {
    let mut core = MapperRom::new(519).prg(16).chr(64).load();
    core.mapper_write(0x8083, 0x05);
    assert_eq!([6, 7, 6, 7], core.prg_windows());
    assert_eq!(40, core.chr_windows()[0]);
    core.mapper_write(0x8104, 0x00); // A8 set: ignored
    assert_eq!([6, 7, 6, 7], core.prg_windows());
}

/// Mesen2 `FaridSlrom.h` and puNES `mapper_323.c`: an MMC1 whose 16 KiB PRG pages become
/// `(outer & $F0) >> 1 | (page & 7)` and 4 KiB CHR pages `(outer & $F0) << 1 | (page & $1F)`,
/// the outer register written at `$6000` while the RAM is enabled, until bit 3 locks it.
#[cfg(feature = "board-323")]
#[test]
fn mappers_n_323_confines_the_mmc1_to_a_game_and_locks() {
    let mut core = MapperRom::new(323).prg(128).chr(256).load();
    core.mapper_write(0x6000, 0x10);
    assert_eq!([0x10, 0x11, 0x1E, 0x1F], core.prg_windows());
    assert_eq!(0x80, core.chr_bank(0x0000));
    core.mmc1_write(0xE000, 0x03);
    assert_eq!([0x16, 0x17, 0x1E, 0x1F], core.prg_windows());
    core.mapper_write(0x6000, 0x28); // game 2, locked
    assert_eq!([0x26, 0x27, 0x2E, 0x2F], core.prg_windows());
    core.mapper_write(0x6000, 0x30);
    assert_eq!([0x26, 0x27, 0x2E, 0x2F], core.prg_windows());
    assert_eq!(0x30, core.read_memory(0x6000)); // the RAM takes every write
}

/// Mesen2 `Sachen9602.h` and puNES `mapper_513.c`: bits 6-7 of a CHR register write pick the
/// 512 KiB block of the switchable PRG windows; the fixed ones stay at `$3E`/`$3F`.
#[cfg(feature = "board-513")]
#[test]
fn mappers_n_513_takes_the_prg_block_from_chr_writes() {
    let mut core = MapperRom::new(513).prg(256).chr(0).load();
    core.mmc3_bank(6, 5);
    core.mmc3_bank(0, 0x80);
    let windows = core.prg_windows();
    assert_eq!((0x85, 0x3E, 0x3F), (windows[0], windows[2], windows[3]));
}

/// puNES `mapper_530.c` (and Mesen2 `Ax5705.h` for the decode): a `VRC4f` whose A12 is CPU A3 and
/// whose PRG register sees bits 1 and 3 crossed.
#[cfg(feature = "board-530")]
#[test]
fn mappers_n_530_crosses_its_lines_into_a_vrc4() {
    let mut core = MapperRom::new(530).prg(16).chr(256).load();
    core.mapper_write(0x8000, 0x08); // bit 3 reaches the chip as bit 1: PRG 0 = 2
    assert_eq!(2, core.prg_windows()[0]);
    core.mapper_write(0xA008, 0x05); // A3 is A12: $B008, CHR 0's low nibble
    assert_eq!(5, core.chr_windows()[0]);
}

/// nesdev "NES 2.0 Mapper 529" and puNES `mapper_529.c`: a `VRC4e` whose `$A00x` register is one
/// 16 KiB bank at `$8000`, the last bank fixed at `$C000`, `$800x` without effect, and the
/// EEPROM's data out on D0 at `$5000` (1, no EEPROM emulated).
#[cfg(feature = "board-529")]
#[test]
fn mappers_n_529_maps_one_16_kib_bank_from_a000() {
    let mut core = MapperRom::new(529).prg(16).chr(256).load();
    core.mapper_write(0xA000, 3);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    core.mapper_write(0x8000, 5);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    core.mapper_write(0xB000, 5);
    assert_eq!(5, core.chr_windows()[0]);
    assert_eq!(Some(1), core.cartridge_mut().unwrap().try_cpu_read(0x5000));
}

/// nesdev "NES 2.0 Mapper 266": CPU A13/A14 crossed into a `VRC4e`, `$900C` giving PRG A16-A15
/// over the chip's own 8 KiB banks, CHR 0 at CPU `$D000`.
#[cfg(feature = "board-266")]
#[test]
fn mappers_n_266_crosses_a13_a14_and_adds_an_outer_bank() {
    let mut core = MapperRom::new(266).prg(32).chr(256).load();
    core.mapper_write(0x8000, 1);
    core.mapper_write(0x900C, 2);
    let windows = core.prg_windows();
    assert_eq!((9, 11), (windows[0], windows[3]));
    core.mapper_write(0xD000, 5);
    assert_eq!(5, core.chr_windows()[0]);
}

/// Mesen2 `Rt01.h` and puNES `mapper_328.c`: one 32 KiB bank, and `$F2` with random bits 0, 2
/// and 3 at `$CE80-$CEFF` and `$FE80-$FEFF`.
#[cfg(feature = "board-328")]
#[test]
fn mappers_n_328_answers_its_random_ranges() {
    let core = MapperRom::new(328).prg(4).chr(8).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    for address in [0xCE80, 0xCEFF, 0xFE80] {
        assert_eq!(0xF2, core.read_memory(address) & 0xF2);
    }
    assert_eq!(2, core.read_memory(0xCE7F));
}

/// nesdev "NES 2.0 Mapper 260" and puNES `mapper_260.c`: an MMC3 under a PRG base, NROM modes,
/// and the lock of `$5000` bit 7.
#[cfg(feature = "board-260")]
#[test]
fn mappers_n_260_bases_the_mmc3_and_locks() {
    let mut core = MapperRom::new(260).prg(64).chr(512).load();
    core.mapper_write(0x5001, 0x10);
    core.mmc3_bank(6, 3);
    assert_eq!([0x23, 0x21, 0x3E, 0x3F], core.prg_windows());
    core.mapper_write(0x5000, 0x85); // NROM-256, locked
    core.mapper_write(0x5001, 0x06);
    assert_eq!([0x20, 0x21, 0x22, 0x23], core.prg_windows());
    assert_eq!(
        Some(0x50),
        core.cartridge_mut().unwrap().try_cpu_read(0x5000)
    );
}

/// Mesen2 `UnlDripGame.h`: with `$800A` bit 2 an attribute fetch returns the palette stored at
/// `$C000-$FFFF` for the tile whose nametable byte was fetched just before, four times over;
/// `$8008`/`$8009` arm a CPU-cycle IRQ; a channel's FIFO status reads at `$5000`.
#[cfg(feature = "board-284")]
#[test]
fn mappers_n_284_extends_the_attributes_and_counts() {
    let mut core = MapperRom::new(284).prg(16).chr(0).load();
    core.mapper_write(0x800B, 3);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    core.mapper_write(0xC000 + 0x45, 0x02); // table 0 (vertical), tile $45
    core.mapper_write(0x800A, 0x04); // vertical, extended attributes on
    let vram = [0x11u8; 0x800];
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(-2, cartridge.nametable_page(0));
    cartridge.on_ppu_address_bus(0x2045, 0);
    assert_eq!(0xAA, cartridge.read_nametable_from_vram(0x23C1, &vram));
    assert_eq!(0x11, cartridge.read_nametable_from_vram(0x2045, &vram));
    assert_eq!(Some(0x40), cartridge.try_cpu_read(0x5000));
    core.mapper_write(0x8008, 0x20);
    core.mapper_write(0x8009, 0x80);
    for _ in 0..0x30 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
}

/// Mesen2 `Rainbow.cpp` and Broke Studio's mapper doc: `$4100` the PRG mode, 16-bit bank
/// registers, the FPGA RAM port at `$415C-$415F` seen again at `$5000`, the CPU-cycle IRQ, and a
/// fill-mode nametable once three identical fetches put the board in frame.
#[cfg(feature = "board-682")]
#[test]
fn mappers_n_682_banks_fills_and_counts() {
    let mut core = MapperRom::new(682).prg(32).chr(0).chr_ram(0x8000).load();
    let read =
        |core: &mut zax_nes::NesCore, address| core.cartridge_mut().unwrap().try_cpu_read(address);
    assert_eq!(Some(3), read(&mut core, 0xE000));
    core.mapper_write(0x4100, 0x03);
    core.mapper_write(0x410C, 0x00);
    core.mapper_write(0x411C, 0x09);
    assert_eq!(Some(9), read(&mut core, 0xC000));
    core.mapper_write(0x415D, 0x10);
    core.mapper_write(0x415E, 0x01);
    core.mapper_write(0x415F, 0xAB);
    assert_eq!(Some(0xAB), read(&mut core, 0x5010));
    assert_eq!(Some(0x20), read(&mut core, 0x4160));
    core.mapper_write(0x4124, 0x77);
    core.mapper_write(0x412A, 0x20);
    let vram = [0x11u8; 0x800];
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(-2, cartridge.nametable_page(0));
    assert_eq!(0x11, cartridge.read_nametable_from_vram(0x2045, &vram));
    for _ in 0..3 {
        cartridge.on_ppu_address_bus(0x2000, 0);
    }
    cartridge.on_ppu_address_bus(0x2045, 0);
    assert_eq!(0x77, cartridge.read_nametable_from_vram(0x2045, &vram));
    core.mapper_write(0x4158, 0x00);
    core.mapper_write(0x4159, 0x20);
    core.mapper_write(0x415A, 0x01);
    for _ in 0..0x30 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
}

/// Mesen2 `Rainbow.cpp`: with `$415A` bit 2, a read of `$4011` acknowledges the CPU IRQ and
/// answers the audio DAC doubled instead of the open bus.
#[cfg(feature = "board-682")]
#[test]
fn mappers_n_682_acknowledges_on_a_4011_read() {
    use zax_nes::cpu::CpuBus;
    let mut core = MapperRom::new(682).prg(32).chr(0).chr_ram(0x8000).load();
    core.mapper_write(0x4159, 0x10);
    core.mapper_write(0x415A, 0x05);
    for _ in 0..0x20 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
    core.bus_mut().write(0x2003, 0x04);
    core.bus_mut().write(0x2004, 0x30);
    assert_eq!(0, core.bus_mut().read(0x4011));
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// Mesen2 `FlashS29.h`, as `Rainbow.cpp` wires it to PRG-ROM: `$AAA`/`$555` unlock, `$A0`
/// program (AND), `$90` software ID (manufacturer `$01`, S29AL008 `$5B`), `$80`/`$30` sector erase;
/// the written sectors reach the flash save file.
#[cfg(feature = "board-682")]
#[test]
fn mappers_n_682_programs_and_erases_its_prg_flash() {
    let mut core = MapperRom::new(682).prg(32).chr(0).chr_ram(0x8000).load();
    let read =
        |core: &mut zax_nes::NesCore, address| core.cartridge_mut().unwrap().try_cpu_read(address);
    let unlock = |core: &mut zax_nes::NesCore, command| {
        core.mapper_write(0x8AAA, 0xAA);
        core.mapper_write(0x8555, 0x55);
        core.mapper_write(0x8AAA, command);
    };
    unlock(&mut core, 0xA0);
    core.mapper_write(0xE000, 0x01);
    assert_eq!(Some(0x01), read(&mut core, 0xE000));
    assert_eq!(Some(0x03), read(&mut core, 0xE001));
    unlock(&mut core, 0x90);
    assert_eq!(Some(0x01), read(&mut core, 0x8000));
    assert_eq!(Some(0x5B), read(&mut core, 0x8002));
    core.mapper_write(0x8000, 0xF0);
    assert_eq!(Some(0x00), read(&mut core, 0x8002));
    unlock(&mut core, 0x80);
    core.mapper_write(0x8AAA, 0xAA);
    core.mapper_write(0x8555, 0x55);
    core.mapper_write(0xE000, 0x30);
    assert_eq!(Some(0xFF), read(&mut core, 0xE000));
    assert_eq!(Some(0xFF), read(&mut core, 0x8000));
    assert_ne!(core.export_flash_save(), Vec::<u8>::new());
}

/// Broke Studio's `mapper-doc.md` and `rainbow-net.md`: a message written at the `$4194` page of
/// FPGA RAM goes to the ESP on a `$4192` write; the answer lands at the `$4193` page with `$4191`
/// bit 7 set, and a `$4191` write acknowledges it. Files round-trip; Wi-Fi reports "not connected"
/// with the network option off (the default).
#[cfg(feature = "board-682")]
#[test]
fn mappers_n_682_talks_to_its_esp() {
    let mut core = MapperRom::new(682).prg(32).chr(0).chr_ram(0x8000).load();
    let send = |core: &mut zax_nes::NesCore, message: &[u8]| {
        for (index, byte) in message.iter().enumerate() {
            core.mapper_write(0x4800 + u16::try_from(index).unwrap(), *byte);
        }
        core.mapper_write(0x4192, 0);
    };
    let answer = |core: &mut zax_nes::NesCore, length: u16| -> Vec<u8> {
        let cartridge = core.cartridge_mut().unwrap();
        assert_eq!(Some(0x80), cartridge.try_cpu_read(0x4191).map(|v| v & 0x80));
        (0..length)
            .map(|offset| cartridge.try_cpu_read(0x4900 + offset).unwrap())
            .collect()
    };
    core.mapper_write(0x4190, 0x01);
    core.mapper_write(0x4193, 0x01);
    core.mapper_write(0x4194, 0x00);
    send(&mut core, &[1, 0]);
    assert_eq!(vec![2, 0, 0], answer(&mut core, 3));
    core.mapper_write(0x4191, 0);
    send(&mut core, &[1, 9]);
    assert_eq!(vec![4, 4, 0, 0, 0], answer(&mut core, 5));
    core.mapper_write(0x4191, 0);
    send(&mut core, &[4, 39, 0, 0, 5]);
    send(&mut core, &[4, 46, b'a', b'b', b'c']);
    send(&mut core, &[2, 44, 0]);
    send(&mut core, &[2, 45, 3]);
    assert_eq!(vec![5, 22, 3, b'a', b'b', b'c'], answer(&mut core, 6));
    core.mapper_write(0x4191, 0);
    assert_eq!(
        Some(0x00),
        core.cartridge_mut().unwrap().try_cpu_read(0x4191)
    );
}
