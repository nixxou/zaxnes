//! Lot MAPPERS-P: one test per board, its expectations worked out from the puNES file each kind
//! cites (`cartridges/mappers/multicart_p.rs`). The sweeps of `mapper_tests` over
//! `SUPPORTED_MAPPERS` (load, reset, save-state round trip, PPU bus) cover the rest.
#![allow(clippy::unwrap_used)]

mod common;

#[allow(unused_imports)]
use common::mapper_fixture::{MapperMachine, MapperRom};

/// `mapper_055.c`: `$6000-$6FFF` is PRG-ROM from offset `$8000`.
#[cfg(feature = "board-55")]
#[test]
fn mappers_p_55_shows_rom_at_6000() {
    let core = MapperRom::new(55).prg(8).load();
    assert_eq!(4, core.read_memory(0x6000));
    assert_eq!([0, 1, 2, 3], core.prg_windows());
}

/// `mapper_081.c`: the address is the register, A2-A3 the PRG bank, A0-A1 the CHR bank.
#[cfg(feature = "board-81")]
#[test]
fn mappers_p_81_banks_from_the_address() {
    let mut core = MapperRom::new(81).prg(16).chr(32).load();
    core.mapper_write(0x800E, 0);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    assert_eq!(16, core.chr_windows()[0]);
}

/// `mapper_184.c` as puNES runs 122: `$6000-$7FFF` two 4 KiB CHR banks.
#[cfg(feature = "board-122")]
#[test]
fn mappers_p_122_banks_chr_from_6000() {
    let mut core = MapperRom::new(122).prg(4).chr(32).load();
    core.mapper_write(0x6000, 0x21);
    assert_eq!([4, 5, 6, 7, 8, 9, 10, 11], core.chr_windows());
}

/// `mapper_237.c`: A2 and D3-D4 the outer block, D0-D2 the inner bank in mode 0; A0 reads the DIP
/// switches instead of ROM.
#[cfg(feature = "board-237")]
#[test]
fn mappers_p_237_has_an_outer_block_and_a_dip_read() {
    let mut core = MapperRom::new(237).prg(64).chr(0).load();
    core.mapper_write(0x8004, 0x05);
    assert_eq!([10, 11, 14, 15], core.prg_windows());
    core.mapper_write(0x8001, 0x05);
    assert_eq!([0, 0, 0, 0], core.prg_windows());
}

/// `mapper_389.c`: NROM-256 from `$8000`'s A3-A7, then the UNROM mode of `$9000` bit 1 with the
/// inner bank from `$A000-$FFFF`.
#[cfg(feature = "board-389")]
#[test]
fn mappers_p_389_switches_between_nrom_and_unrom() {
    let mut core = MapperRom::new(389).prg(32).chr(64).load();
    core.mapper_write(0x8018, 0);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0x9002, 0);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0xA004, 0);
    assert_eq!([14, 15, 14, 15], core.prg_windows());
}

/// `mapper_517.c`: the bank is the written value; `$6000` reads the ADC's start bits.
#[cfg(feature = "board-517")]
#[test]
fn mappers_p_517_banks_and_reads_its_adc() {
    let mut core = MapperRom::new(517).prg(16).load();
    core.mapper_write(0x8000, 2);
    assert_eq!([4, 5, 14, 15], core.prg_windows());
    let bus = core.cartridge_mut().unwrap();
    assert_eq!(Some(0), bus.try_cpu_read(0x6000));
    assert_eq!(Some(1), bus.try_cpu_read(0x6000));
    assert_eq!(Some(1), bus.try_cpu_read(0x6000));
}

/// `mapper_281.c`: J.Y. ASIC, `$D003` the 256 KiB outer PRG bank (8 KiB mode, last bank fixed).
#[cfg(feature = "board-281")]
#[test]
fn mappers_p_281_places_the_asic_in_a_256k_outer_bank() {
    let mut core = MapperRom::new(281).prg(128).chr(256).load();
    core.mapper_write(0xD000, 0x02);
    core.mapper_write(0x8000, 1);
    core.mapper_write(0xD003, 1);
    assert_eq!([33, 32, 32, 63], core.prg_windows());
}

/// `mapper_282.c`: PRG from `$D003` bits 1-7.
#[cfg(feature = "board-282")]
#[test]
fn mappers_p_282_places_prg_from_d003_bit_1() {
    let mut core = MapperRom::new(282).prg(128).chr(256).load();
    core.mapper_write(0xD000, 0x02);
    core.mapper_write(0x8000, 1);
    core.mapper_write(0xD003, 2);
    assert_eq!([33, 32, 32, 63], core.prg_windows());
}

/// `mapper_295.c`: 128 KiB outer PRG and CHR banks.
#[cfg(feature = "board-295")]
#[test]
fn mappers_p_295_places_the_asic_in_128k_outer_banks() {
    let mut core = MapperRom::new(295).prg(64).chr(256).load();
    core.mapper_write(0xD000, 0x1A);
    core.mapper_write(0x8000, 1);
    core.mapper_write(0x9000, 5);
    core.mapper_write(0xD003, 1);
    assert_eq!([17, 16, 16, 31], core.prg_windows());
    assert_eq!(133, core.chr_windows()[0]);
}

/// `mapper_271.c`: D4-D7 the 32 KiB bank, D0-D3 the CHR bank.
#[cfg(feature = "board-271")]
#[test]
fn mappers_p_271_latches_prg_and_chr_from_the_data() {
    let mut core = MapperRom::new(271).prg(16).chr(64).load();
    core.mapper_write(0x8000, 0x13);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(24, core.chr_windows()[0]);
}

/// `mapper_335.c`: `$C000` D4 picks NROM-128 with D3 the low bank bit; `$8000` the CHR bank.
#[cfg(feature = "board-335")]
#[test]
fn mappers_p_335_switches_nrom_128_and_256() {
    let mut core = MapperRom::new(335).prg(16).chr(32).load();
    core.mapper_write(0xC000, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0xC000, 0x19);
    assert_eq!([6, 7, 6, 7], core.prg_windows());
    core.mapper_write(0x8000, 2);
    assert_eq!(16, core.chr_windows()[0]);
}

/// `mapper_338.c`: the address is the NROM-128 bank; `$F008` latched reads the DIP switches.
#[cfg(feature = "board-338")]
#[test]
fn mappers_p_338_latches_the_address_and_reads_its_dip() {
    let mut core = MapperRom::new(338).prg(16).chr(64).load();
    core.mapper_write(0x8003, 0);
    assert_eq!([6, 7, 6, 7], core.prg_windows());
    core.mapper_write(0xF008, 0);
    assert_eq!([0, 0, 0, 0], core.prg_windows());
}

/// `mapper_341.c`: A8 up the 32 KiB and CHR bank.
#[cfg(feature = "board-341")]
#[test]
fn mappers_p_341_banks_from_a8() {
    let mut core = MapperRom::new(341).prg(16).chr(32).load();
    core.mapper_write(0x8200, 0);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(16, core.chr_windows()[0]);
}

/// `mapper_343.c`: the latch is the inverted data and powers on as `$FF`.
#[cfg(feature = "board-343")]
#[test]
fn mappers_p_343_inverts_its_latch() {
    let mut core = MapperRom::new(343).prg(16).chr(32).load();
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0x8000, 0xFE);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
}

/// `mapper_352.c`: each reset moves to the next 32 KiB game.
#[cfg(feature = "board-352")]
#[test]
fn mappers_p_352_counts_resets() {
    let mut core = MapperRom::new(352).prg(16).chr(32).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    core.soft_reset();
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(8, core.chr_windows()[0]);
}

/// `mapper_381.c`: D0-D3 rotated into the inner bank, the reset count the outer 256 KiB bank.
#[cfg(feature = "board-381")]
#[test]
fn mappers_p_381_rotates_its_bank_and_counts_resets() {
    let mut core = MapperRom::new(381).prg(64).load();
    core.mapper_write(0x8000, 0x10);
    assert_eq!([2, 3, 30, 31], core.prg_windows());
    core.soft_reset();
    core.mapper_write(0x8000, 0x01);
    assert_eq!([36, 37, 62, 63], core.prg_windows());
}

/// `mapper_337.c`: `$8000` writes the high five bits, `$C000` the low three; mode 1 is NROM-256;
/// `$6000` is PRG-ROM bank 1.
#[cfg(feature = "board-337")]
#[test]
fn mappers_p_337_splits_its_register_and_shows_rom_at_6000() {
    let mut core = MapperRom::new(337).prg(32).load();
    core.mapper_write(0x8000, 0x40);
    core.mapper_write(0xC000, 0x02);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(1, core.read_memory(0x6000));
}

/// `mapper_350.c`: modes 2-3 with bit 5 use the second chip's 128 KiB.
#[cfg(feature = "board-350")]
#[test]
fn mappers_p_350_reaches_its_second_chip() {
    let mut core = MapperRom::new(350).prg(128).load();
    core.mapper_write(0x8000, 0x60);
    core.mapper_write(0xC000, 0x02);
    // Base `$20` (A19), inner bank 2 and its last bank 7.
    assert_eq!([68, 69, 78, 79], core.prg_windows());
}

/// `mapper_375.c`: A11 locks the address latch; A7 write-protects CHR-RAM.
#[cfg(feature = "board-375")]
#[test]
fn mappers_p_375_locks_its_latch_and_protects_chr_ram() {
    let mut core = MapperRom::new(375).prg(64).load();
    // A11 set: the first window takes the data's bank (3), the second the latched one.
    core.mapper_write(0x8880, 0x03);
    assert_eq!([6, 7, 0, 1], core.prg_windows());
    // Locked: only the data moves.
    core.mapper_write(0x8000, 0x01);
    assert_eq!([2, 3, 0, 1], core.prg_windows());
}

/// `mapper_358.c`: 282's PRG decode on the J.Y. ASIC.
#[cfg(feature = "board-358")]
#[test]
fn mappers_p_358_places_prg_from_d003_bit_1() {
    let mut core = MapperRom::new(358).prg(128).chr(256).load();
    core.mapper_write(0xD000, 0x02);
    core.mapper_write(0x8000, 1);
    core.mapper_write(0xD003, 2);
    assert_eq!([33, 32, 32, 63], core.prg_windows());
}

/// `mapper_354.c`: the latch answers from `$F000`; mode 1 is UNROM, mode 5 adds PRG-ROM at
/// `$6000`.
#[cfg(feature = "board-354")]
#[test]
fn mappers_p_354_latches_at_f000_and_maps_6000_in_mode_5() {
    let mut core = MapperRom::new(354).prg(64).load();
    core.mapper_write(0xE001, 0x02);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    core.mapper_write(0xF001, 0x02);
    assert_eq!([4, 5, 14, 15], core.prg_windows());
    core.mapper_write(0xF005, 0x83);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    assert_eq!(7, core.read_memory(0x6000));
}

/// `mapper_380.c`: A2-A6 the bank, A9 UNROM vs NROM; A8 (submapper 0) reads the DIP switches.
#[cfg(feature = "board-380")]
#[test]
fn mappers_p_380_banks_from_the_address_and_reads_its_dip() {
    let mut core = MapperRom::new(380).prg(64).load();
    core.mapper_write(0x8008, 0);
    assert_eq!([4, 5, 14, 15], core.prg_windows());
    core.mapper_write(0x8208, 0);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0x8100, 0);
    assert_eq!(0x34, core.read_memory(0x8134));
}

/// `mapper_382.c`: A0-A2 the outer bank, A3 NROM-256, A5 locks; the data is the inner bank.
#[cfg(feature = "board-382")]
#[test]
fn mappers_p_382_locks_its_outer_bank() {
    let mut core = MapperRom::new(382).prg(64).load();
    core.mapper_write(0x8021, 0xFF);
    // The data is ANDed with the ROM byte under the write: bank 0 holds zeros, so it is 0.
    assert_eq!([16, 17, 30, 31], core.prg_windows());
    core.mapper_write(0x8008, 0xFF);
    assert_eq!([16, 17, 30, 31], core.prg_windows());
}

/// `mapper_390.c`: `$8000-$BFFF` the CHR register, `$C000-$FFFF` the PRG register from the
/// address.
#[cfg(feature = "board-390")]
#[test]
fn mappers_p_390_takes_two_registers_from_the_address() {
    let mut core = MapperRom::new(390).prg(64).chr(64).load();
    // Mode `$20`: NROM-256, 32 KiB bank `$22 >> 1` = 17, which wraps to 1 on 512 KiB.
    core.mapper_write(0xC022, 0);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    // Mode `$30`: NROM-128, 16 KiB bank `$33` = 51, which wraps to 19.
    core.mapper_write(0xC033, 0);
    assert_eq!([38, 39, 38, 39], core.prg_windows());
    core.mapper_write(0x8003, 0);
    assert_eq!(24, core.chr_windows()[0]);
}

/// `mapper_388.c`: PRG A18-A19 from `$D003` bits 2-3.
#[cfg(feature = "board-388")]
#[test]
fn mappers_p_388_places_prg_from_d003_bits_2_and_3() {
    let mut core = MapperRom::new(388).prg(128).chr(256).load();
    core.mapper_write(0xD000, 0x02);
    core.mapper_write(0x8000, 1);
    core.mapper_write(0xD003, 0x04);
    assert_eq!([33, 32, 32, 63], core.prg_windows());
}

/// `mapper_396.c`: `$A000` the outer 128 KiB bank, elsewhere the UNROM inner bank.
#[cfg(feature = "board-396")]
#[test]
fn mappers_p_396_has_an_outer_bank_at_a000() {
    let mut core = MapperRom::new(396).prg(64).load();
    core.mapper_write(0xA000, 0x01);
    core.mapper_write(0x8000, 0x02);
    assert_eq!([20, 21, 30, 31], core.prg_windows());
}

/// `mapper_400.c`: `$7800` the outer bank, `$C000` the inner bank (bank 0's zeros masking the data).
#[cfg(feature = "board-400")]
#[test]
fn mappers_p_400_takes_its_outer_bank_at_7800() {
    let mut core = MapperRom::new(400).prg(64).chr(32).load();
    core.mapper_write(0x7800, 0x08);
    assert_eq!([16, 17, 30, 31], core.prg_windows());
}

/// `mapper_403.c`: `$4100` the PRG bank, `$4102` the mode and the CHR enable.
#[cfg(feature = "board-403")]
#[test]
fn mappers_p_403_takes_its_registers_at_4100() {
    let mut core = MapperRom::new(403).prg(32).chr(32).load();
    core.mapper_write(0x4100, 0x04);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0x4102, 0x05);
    assert_eq!([4, 5, 4, 5], core.prg_windows());
    core.mapper_write(0x8000, 0x02);
    assert_eq!(16, core.chr_windows()[0]);
}

/// `mapper_409.c`: a write to `$C000-$CFFF` makes its address the 16 KiB bank.
#[cfg(feature = "board-409")]
#[test]
fn mappers_p_409_banks_from_c000_addresses() {
    let mut core = MapperRom::new(409).prg(64).load();
    core.mapper_write(0xC003, 0);
    assert_eq!([6, 7, 62, 63], core.prg_windows());
    core.mapper_write(0x8005, 0);
    assert_eq!([6, 7, 62, 63], core.prg_windows());
}

/// `mapper_414.c`: A13 picks NROM-256 over NROM-128, A1 up the bank, A0 the mirroring.
#[cfg(feature = "board-414")]
#[test]
fn mappers_p_414_banks_from_the_address() {
    let mut core = MapperRom::new(414).prg(64).chr(32).load();
    core.mapper_write(0x8006, 0);
    assert_eq!([6, 7, 6, 7], core.prg_windows());
    core.mapper_write(0xA004, 0);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
}

/// `mapper_415.c`: the last 32 KiB fixed, D0-D3 the 8 KiB bank at `$6000`.
#[cfg(feature = "board-415")]
#[test]
fn mappers_p_415_maps_an_8k_bank_at_6000() {
    let mut core = MapperRom::new(415).prg(16).load();
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0x8000, 0x05);
    assert_eq!(5, core.read_memory(0x6000));
}

/// `mapper_428.c`: `$6001` D5-D7 the PRG bank, D4 NROM-256; `$6002` D6-D7 hand CHR bits to the
/// `$8000` latch.
#[cfg(feature = "board-428")]
#[test]
fn mappers_p_428_mixes_its_chr_latch() {
    let mut core = MapperRom::new(428).prg(32).chr(64).load();
    core.mapper_write(0x6001, 0x41);
    assert_eq!([4, 5, 4, 5], core.prg_windows());
    assert_eq!(8, core.chr_windows()[0]);
    core.mapper_write(0x6002, 0xC0);
    core.mapper_write(0x8000, 0x02);
    // `($6001 & 7) & !3 | ($8000 & 3)`: 8 KiB bank 2.
    assert_eq!(16, core.chr_windows()[0]);
}

/// `mapper_429.c`: powers on at 32 KiB bank 1; D2-D5 the 32 KiB bank, D0-D1 the CHR bank.
#[cfg(feature = "board-429")]
#[test]
fn mappers_p_429_powers_on_at_bank_1() {
    let mut core = MapperRom::new(429).prg(32).chr(32).load();
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0x8000, 0x0B);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    assert_eq!(24, core.chr_windows()[0]);
}

/// `mapper_421.c`: `$D003` bit 6 widens the inner PRG mask to 512 KiB.
#[cfg(feature = "board-421")]
#[test]
fn mappers_p_421_widens_its_inner_prg_mask() {
    let mut core = MapperRom::new(421).prg(128).chr(256).load();
    core.mapper_write(0xD000, 0x02);
    core.mapper_write(0x8000, 0x21);
    core.mapper_write(0xD003, 0x02);
    assert_eq!([33, 32, 32, 63], core.prg_windows());
    core.mapper_write(0xD003, 0x40);
    assert_eq!([33, 0, 0, 63], core.prg_windows());
}

/// `mapper_433.c`: D5 NROM-128 over NROM-256, D0-D4 the 16 KiB bank.
#[cfg(feature = "board-433")]
#[test]
fn mappers_p_433_switches_nrom_128_and_256() {
    let mut core = MapperRom::new(433).prg(32).load();
    core.mapper_write(0x8000, 0x05);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    core.mapper_write(0x8000, 0x25);
    assert_eq!([10, 11, 10, 11], core.prg_windows());
}

/// `mapper_434.c`: `$6000` the outer 128 KiB bank.
#[cfg(feature = "board-434")]
#[test]
fn mappers_p_434_takes_its_outer_bank_at_6000() {
    let mut core = MapperRom::new(434).prg(64).load();
    core.mapper_write(0x6000, 0x01);
    assert_eq!([16, 17, 30, 31], core.prg_windows());
}

/// `mapper_437.c`: `$5000-$5FFF` the address is the outer 128 KiB bank.
#[cfg(feature = "board-437")]
#[test]
fn mappers_p_437_takes_its_outer_bank_from_a_5000_address() {
    let mut core = MapperRom::new(437).prg(64).load();
    core.mapper_write(0x5001, 0);
    assert_eq!([16, 17, 30, 31], core.prg_windows());
}

/// `mapper_438.c`: A0 picks UNROM-like pairs; D1 up the CHR bank.
#[cfg(feature = "board-438")]
#[test]
fn mappers_p_438_pairs_its_banks_by_a0() {
    let mut core = MapperRom::new(438).prg(32).chr(32).load();
    core.mapper_write(0x8005, 0x04);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(16, core.chr_windows()[0]);
}

/// `mapper_481.c`: D7 the outer 128 KiB bank, D0-D2 the inner one.
#[cfg(feature = "board-481")]
#[test]
fn mappers_p_481_has_an_outer_bank_in_d7() {
    let mut core = MapperRom::new(481).prg(32).load();
    core.mapper_write(0x8000, 0x82);
    assert_eq!([20, 21, 30, 31], core.prg_windows());
}

/// `mapper_538.c`: the upper 24 KiB fixed to banks `$0D-$0F`; `$C000` D0 with D3 clear shows
/// bank `$0A` at `$8000`; `$6000` shows bank `latch | 1`.
#[cfg(feature = "board-538")]
#[test]
fn mappers_p_538_has_a_bank_at_6000_and_a_special_8000() {
    let mut core = MapperRom::new(538).prg(16).load();
    core.mapper_write(0xC000, 0x04);
    assert_eq!([4, 13, 14, 15], core.prg_windows());
    assert_eq!(5, core.read_memory(0x6000));
    core.mapper_write(0xC000, 0x03);
    assert_eq!([10, 13, 14, 15], core.prg_windows());
}

/// `mapper_541.c`: `$C000-$FFFF` writes; A1 picks NROM-128.
#[cfg(feature = "board-541")]
#[test]
fn mappers_p_541_banks_from_c000_addresses() {
    let mut core = MapperRom::new(541).prg(64).load();
    core.mapper_write(0xC018, 0);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0xC016, 0);
    assert_eq!([10, 11, 10, 11], core.prg_windows());
}

/// `mapper_564.c`: once D5 is set only D0-D1 and D4 move (D3 clear).
#[cfg(feature = "board-564")]
#[test]
fn mappers_p_564_shrinks_its_latch() {
    let mut core = MapperRom::new(564).prg(64).load();
    core.mapper_write(0x8000, 0x21);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0x8000, 0x06);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
}
