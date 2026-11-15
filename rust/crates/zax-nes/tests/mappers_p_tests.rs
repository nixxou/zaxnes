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

/// `mapper_525.c`: `$8000` D1 up the 16 KiB bank, `$B000-$B007` eight 1 KiB CHR banks.
#[cfg(feature = "board-525")]
#[test]
fn mappers_p_525_banks_chr_by_the_kilobyte() {
    let mut core = MapperRom::new(525).prg(16).chr(64).load();
    core.mapper_write(0x8000, 0x04);
    assert_eq!([4, 5, 14, 15], core.prg_windows());
    core.mapper_write(0xB003, 9);
    assert_eq!(9, core.chr_windows()[3]);
}

/// `mapper_554.c`: reading `$CAB8` latches A2-A5 (bank 14) as the CHR and `$6000` bank.
#[cfg(feature = "board-554")]
#[test]
fn mappers_p_554_latches_on_a_read() {
    let mut core = MapperRom::new(554).prg(16).chr(128).load();
    assert_eq!([10, 11, 6, 7], core.prg_windows());
    let bus = core.cartridge_mut().unwrap();
    assert_eq!(Some(6), bus.try_cpu_read(0xCAB8));
    assert_eq!(Some(14), bus.try_cpu_read(0x6000));
    assert_eq!(112, core.chr_windows()[0]);
}

/// `mapper_297.c`: power-on latch mode (UNROM-like, `$8000` D4-D5); `$4100` bit 0 hands the
/// windows to the MMC1 in outer 128 KiB bank 1.
#[cfg(feature = "board-297")]
#[test]
fn mappers_p_297_switches_between_its_latch_and_the_mmc1() {
    let mut core = MapperRom::new(297).prg(32).chr(64).load();
    assert_eq!([0, 1, 6, 7], core.prg_windows());
    core.mapper_write(0x8000, 0x20);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0x4100, 0x01);
    assert_eq!([16, 17, 30, 31], core.prg_windows());
}

/// `mapper_374.c`: each reset moves the MMC1 to the next 128 KiB PRG / 128 KiB CHR game.
#[cfg(feature = "board-374")]
#[test]
fn mappers_p_374_counts_resets() {
    let mut core = MapperRom::new(374).prg(64).chr(256).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.soft_reset();
    assert_eq!([16, 17, 30, 31], core.prg_windows());
    assert_eq!(128, core.chr_bank(0x0000));
}

/// `mapper_404.c`: `$6000` the outer bank (D6 halves the inner mask), D7 locks it.
#[cfg(feature = "board-404")]
#[test]
fn mappers_p_404_takes_its_outer_bank_at_6000_and_locks() {
    let mut core = MapperRom::new(404).prg(128).chr(256).load();
    core.mapper_write(0x6000, 0x41);
    assert_eq!([16, 17, 30, 31], core.prg_windows());
    // D6 clear: a 256 KiB inner mask, so the fixed bank is the MMC1's own last (30-31).
    core.mapper_write(0x6000, 0x80);
    assert_eq!([0, 1, 30, 31], core.prg_windows());
    core.mapper_write(0x6000, 0x01);
    assert_eq!([0, 1, 30, 31], core.prg_windows());
}

/// `mapper_550.c`: a 32 KiB latch until `$7000-$7FFF` A1-A2 select the MMC1.
#[cfg(feature = "board-550")]
#[test]
fn mappers_p_550_switches_from_its_latch_to_the_mmc1() {
    let mut core = MapperRom::new(550).prg(64).chr(256).load();
    core.mapper_write(0x8000, 0x30);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0x7006, 0);
    assert_eq!([48, 49, 62, 63], core.prg_windows());
    assert_eq!(96, core.chr_bank(0x0000));
}

/// Sets R6 and R7 of an MMC3 and enables its work RAM, the common start of the MMC3 tests.
#[allow(dead_code)]
fn mmc3_start(core: &mut zax_nes::NesCore, r6: u8, r7: u8) {
    core.mapper_write(0xA001, 0x80);
    core.mmc3_bank(6, r6);
    core.mmc3_bank(7, r7);
}

/// `mapper_267.c`: `$6000` D1-D2 and D5 the outer 256 KiB bank, D7 locks it.
#[cfg(feature = "board-267")]
#[test]
fn mappers_p_267_locks_its_outer_bank() {
    let mut core = MapperRom::new(267).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x82);
    assert_eq!([33, 34, 62, 63], core.prg_windows());
    core.mapper_write(0x6000, 0x00);
    assert_eq!([33, 34, 62, 63], core.prg_windows());
}

/// `mapper_322.c`: with A5 clear the latched address is an NROM bank, A0-A1 picking NROM-256.
#[cfg(feature = "board-322")]
#[test]
fn mappers_p_322_has_an_nrom_mode() {
    let mut core = MapperRom::new(322).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6003, 0);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
}

/// `mapper_334.c`: `$6000` even addresses the 32 KiB bank; `$6002` reads the DIP bit.
#[cfg(feature = "board-334")]
#[test]
fn mappers_p_334_maps_32k_banks() {
    let mut core = MapperRom::new(334).prg(32).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x05);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
    core.mapper_write(0x6001, 0x02);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
}

/// `mapper_345.c`: D2-D3 clear is the `AxROM` mode.
#[cfg(feature = "board-345")]
#[test]
fn mappers_p_345_has_an_axrom_mode() {
    let mut core = MapperRom::new(345).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0x6000, 0x44);
    assert_eq!([17, 18, 30, 31], core.prg_windows());
}

/// `mapper_361.c`: `$7000-$7FFF` D4-D7 the outer 128 KiB bank.
#[cfg(feature = "board-361")]
#[test]
fn mappers_p_361_takes_its_outer_bank_at_7000() {
    let mut core = MapperRom::new(361).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x20);
    assert_eq!([1, 2, 14, 15], core.prg_windows());
    core.mapper_write(0x7000, 0x20);
    assert_eq!([33, 34, 46, 47], core.prg_windows());
}

/// `mapper_372.c`: mapper 45's four registers in turn: CHR offset, PRG offset, CHR mask, PRG mask.
#[cfg(feature = "board-372")]
#[test]
fn mappers_p_372_writes_its_registers_in_turn() {
    let mut core = MapperRom::new(372).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    for value in [0x00, 0x10, 0x0F, 0x30] {
        core.mapper_write(0x6000, value);
    }
    assert_eq!([17, 18, 30, 31], core.prg_windows());
}

/// `mapper_377.c`: `$6000-$6FFF` D1-D2 and D5 the outer 128 KiB bank.
#[cfg(feature = "board-377")]
#[test]
fn mappers_p_377_takes_its_outer_bank_at_6000() {
    let mut core = MapperRom::new(377).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x02);
    assert_eq!([17, 18, 30, 31], core.prg_windows());
}

/// `mapper_395.c`: `$6000` D4-D5 and D3 the outer bank, `$6010` D0 and D3 (halving the mask).
#[cfg(feature = "board-395")]
#[test]
fn mappers_p_395_has_two_outer_registers() {
    let mut core = MapperRom::new(395).prg(128).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x10);
    assert_eq!([33, 34, 62, 63], core.prg_windows());
    core.mapper_write(0x6010, 0x08);
    assert_eq!([33, 34, 46, 47], core.prg_windows());
}

/// `mapper_399.c`: PRG 0, `reg[0]`, `reg[1]`, last; `$8001` with D7 set writes `reg[1]`.
#[cfg(feature = "board-399")]
#[test]
fn mappers_p_399_has_its_own_banks() {
    let mut core = MapperRom::new(399).prg(16).chr(64).load();
    assert_eq!([0, 0, 1, 15], core.prg_windows());
    core.mapper_write(0x8001, 0x85);
    assert_eq!([0, 0, 5, 15], core.prg_windows());
    core.mapper_write(0x8000, 0x03);
    assert_eq!(12, core.chr_windows()[0]);
}

/// `mapper_410.c`: four registers in turn until `reg[3]` D6 locks them.
#[cfg(feature = "board-410")]
#[test]
fn mappers_p_410_locks_its_registers() {
    let mut core = MapperRom::new(410).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    for value in [0x00, 0x10, 0x0F, 0x70] {
        core.mapper_write(0x6000, value);
    }
    assert_eq!([17, 18, 30, 31], core.prg_windows());
    core.mapper_write(0x6000, 0x00);
    assert_eq!([17, 18, 30, 31], core.prg_windows());
}

/// `mapper_411.c`: `$5000` D6 the NROM mode, D1 NROM-256, D0/D2/D3 the bank.
#[cfg(feature = "board-411")]
#[test]
fn mappers_p_411_has_nrom_modes_at_5000() {
    let mut core = MapperRom::new(411).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x5001, 0x00);
    // Bank D0 | D2 = 4 in 16 KiB units, NROM-256: 32 KiB bank 2.
    core.mapper_write(0x5000, 0x46);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
}

/// `mapper_420.c`: `$6000` D7 the NROM-256 mode, D1-D3 the bank.
#[cfg(feature = "board-420")]
#[test]
fn mappers_p_420_has_an_nrom_mode() {
    let mut core = MapperRom::new(420).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x84);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
}

/// `mapper_455.c`: power-on NROM mode; `$4100` with D0 clear hands the windows to the MMC3.
#[cfg(feature = "board-455")]
#[test]
fn mappers_p_455_starts_in_nrom_mode() {
    let mut core = MapperRom::new(455).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    assert_eq!([0, 1, 0, 1], core.prg_windows());
    // MMC3 mode; A0 widens the inner mask to 256 KiB, which covers the outer bit too.
    core.mapper_write(0x4101, 0x04);
    assert_eq!([1, 2, 30, 31], core.prg_windows());
}

/// `mapper_456.c`: `$4100` the outer 128 KiB bank.
#[cfg(feature = "board-456")]
#[test]
fn mappers_p_456_takes_its_outer_bank_at_4100() {
    let mut core = MapperRom::new(456).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x4100, 0x02);
    assert_eq!([33, 34, 46, 47], core.prg_windows());
}

/// `mapper_457.c`: `$6000` D0-D2 the outer bank, D3 a 256 KiB inner mask.
#[cfg(feature = "board-457")]
#[test]
fn mappers_p_457_widens_its_inner_bank() {
    let mut core = MapperRom::new(457).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x01);
    assert_eq!([17, 18, 30, 31], core.prg_windows());
    core.mapper_write(0x6000, 0x08);
    assert_eq!([1, 2, 30, 31], core.prg_windows());
}

/// `mapper_516.c`: a chip write with A4 set latches A0-A1 as the outer PRG bank.
#[cfg(feature = "board-516")]
#[test]
fn mappers_p_516_latches_a4_writes() {
    let mut core = MapperRom::new(516).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x8011, 2);
    assert_eq!([17, 18, 30, 31], core.prg_windows());
}

/// `mapper_393.c`: `$6000` A5-A4 = 11 is UNROM from the chip-write latch.
#[cfg(feature = "board-393")]
#[test]
fn mappers_p_393_has_an_unrom_mode() {
    let mut core = MapperRom::new(393).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6031, 0);
    core.mapper_write(0xC000, 0x03);
    assert_eq!([22, 23, 30, 31], core.prg_windows());
}

/// `mapper_412.c`: `$6002` D1 the NROM mode with `reg[0]` the CHR bank.
#[cfg(feature = "board-412")]
#[test]
fn mappers_p_412_has_an_nrom_mode() {
    let mut core = MapperRom::new(412).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x6000, 0x08);
    core.mapper_write(0x6002, 0x16);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(16, core.chr_windows()[0]);
}

/// `mapper_384.c`: a `VRC4e` whose `$6800-$6FFF` register is the outer 128 KiB bank.
#[cfg(feature = "board-384")]
#[test]
fn mappers_p_384_takes_its_outer_bank_at_6800() {
    let mut core = MapperRom::new(384).prg(64).chr(256).prg_ram(8192, 0).load();
    core.mapper_write(0x8000, 0x01);
    core.mapper_write(0x6800, 0x01);
    assert_eq!([17, 17, 30, 31], core.prg_windows());
    core.mapper_write(0x6800, 0x08);
    core.mapper_write(0x6800, 0x01);
    assert_eq!([1, 1, 14, 15], core.prg_windows());
}

/// `mapper_447.c`: the `$6000` address latch's D2 is an NROM mode repeating the two registers.
#[cfg(feature = "board-447")]
#[test]
fn mappers_p_447_has_an_nrom_mode() {
    let mut core = MapperRom::new(447)
        .prg(128)
        .chr(256)
        .prg_ram(8192, 0)
        .load();
    core.mapper_write(0x6006, 0);
    assert_eq!([96, 97, 96, 97], core.prg_windows());
}

/// `mapper_524.c`: a `VRC2b` (`$8000` the first PRG bank) whose `$F00x` is its own IRQ.
#[cfg(feature = "board-524")]
#[test]
fn mappers_p_524_is_a_vrc2b() {
    let mut core = MapperRom::new(524).prg(16).chr(64).load();
    core.mapper_write(0x8000, 0x03);
    core.mapper_write(0xF008, 0x00);
    assert_eq!([3, 1, 14, 15], core.prg_windows());
}

/// `mapper_308.c`: a `VRC2b`; `$F001` enables its IRQ and `$F003` sets the high counter.
#[cfg(feature = "board-308")]
#[test]
fn mappers_p_308_is_a_vrc2b() {
    let mut core = MapperRom::new(308).prg(16).chr(64).load();
    core.mapper_write(0xA000, 0x05);
    core.mapper_write(0xF003, 0x10);
    core.mapper_write(0xF001, 0x00);
    assert_eq!([0, 5, 14, 15], core.prg_windows());
}

/// `mapper_340.c`: A5 clear is UNROM from A0-A4 and A7; A5 with A0 clear NROM-256.
#[cfg(feature = "board-340")]
#[test]
fn mappers_p_340_switches_unrom_and_nrom() {
    let mut core = MapperRom::new(340).prg(64).load();
    core.mapper_write(0x8003, 0);
    assert_eq!([6, 7, 14, 15], core.prg_windows());
    core.mapper_write(0x8024, 0);
    assert_eq!([8, 9, 10, 11], core.prg_windows());
}

/// `mapper_311.c`: `$4022` the 32 KiB bank; `$5000` 4 KiB bank 17 and `$6000` 8 KiB bank 9.
#[cfg(feature = "board-311")]
#[test]
fn mappers_p_311_maps_rom_below_8000() {
    let mut core = MapperRom::new(311).prg(16).load();
    core.mapper_write(0x4022, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(8, core.read_memory(0x5000));
    assert_eq!(9, core.read_memory(0x6000));
}

/// `mapper_357.c` with the DIP switch at 0: the *SMB2J* layout.
#[cfg(feature = "board-357")]
#[test]
fn mappers_p_357_has_the_smb2j_layout() {
    let mut core = MapperRom::new(357).prg(16).load();
    assert_eq!([1, 0, 4, 10], core.prg_windows());
    core.mapper_write(0x4022, 0x02);
    assert_eq!([1, 0, 5, 10], core.prg_windows());
}

/// `mapper_368.c`: `$4022` D0-D2 pick the `$C000` bank; `$4122` reads back `$8A | (D & $35)`.
#[cfg(feature = "board-368")]
#[test]
fn mappers_p_368_has_a_readable_irq_register() {
    let mut core = MapperRom::new(368).prg(16).load();
    assert_eq!([1, 0, 4, 8], core.prg_windows());
    core.mapper_write(0x4022, 0x01);
    assert_eq!([1, 0, 3, 8], core.prg_windows());
    core.mapper_write(0x4122, 0x01);
    assert_eq!(0x8B, core.read_memory(0x4122));
}

/// `mapper_416.c`: `$4020` the *SMB2J* `$C000` bank, `$8000` D3 the NROM modes.
#[cfg(feature = "board-416")]
#[test]
fn mappers_p_416_switches_to_nrom() {
    let mut core = MapperRom::new(416).prg(16).chr(32).load();
    core.mapper_write(0x4020, 0x01);
    assert_eq!([0, 1, 4, 3], core.prg_windows());
    core.mapper_write(0x8000, 0x88);
    assert_eq!([12, 13, 14, 15], core.prg_windows());
}

/// `mapper_417.c`: three 8 KiB PRG registers, the last bank fixed.
#[cfg(feature = "board-417")]
#[test]
fn mappers_p_417_has_three_prg_registers() {
    let mut core = MapperRom::new(417).prg(16).chr(64).load();
    core.mapper_write(0x8000, 5);
    core.mapper_write(0x8001, 6);
    core.mapper_write(0x8002, 7);
    core.mapper_write(0x8012, 9);
    assert_eq!([5, 6, 7, 15], core.prg_windows());
    assert_eq!(9, core.chr_windows()[2]);
}

/// `mapper_178.c` as 551: `$4803` is an 8 KiB CHR bank; `$4800-$48FF` decode by A0-A1.
#[cfg(feature = "board-551")]
#[test]
fn mappers_p_551_banks_chr_through_4803() {
    let mut core = MapperRom::new(551).prg(32).chr(64).load();
    core.mapper_write(0x4803, 0x03);
    assert_eq!(24, core.chr_windows()[0]);
    core.mapper_write(0x4880, 0x02);
    core.mapper_write(0x4881, 0x02);
    assert_eq!([4, 5, 14, 15], core.prg_windows());
}

/// `mapper_126.c` as 422: `$6000` D0-D2 the outer 128 KiB bank; `$6003` D0-D1 = 3 is NROM-256.
#[cfg(feature = "board-422")]
#[test]
fn mappers_p_422_has_an_outer_bank_and_nrom_modes() {
    let mut core = MapperRom::new(422).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    // D6 set: a 128 KiB inner mask, D0 the outer bank.
    core.mapper_write(0x6000, 0x41);
    assert_eq!([17, 18, 30, 31], core.prg_windows());
    core.mapper_write(0x6003, 0x03);
    assert_eq!([16, 17, 18, 19], core.prg_windows());
}

/// `mapper_126.c` as 534: the same outer bank.
#[cfg(feature = "board-534")]
#[test]
fn mappers_p_534_has_an_outer_bank() {
    let mut core = MapperRom::new(534).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    // D6 clear: a 256 KiB inner mask, D1 the outer 256 KiB bank.
    core.mapper_write(0x6000, 0x02);
    assert_eq!([33, 34, 62, 63], core.prg_windows());
}

/// `mapper_543.c`: power-on PRG register `$0E`; four `$5000` writes shift the outer bank in by D3.
#[cfg(feature = "board-543")]
#[test]
fn mappers_p_543_shifts_its_outer_bank_in() {
    let mut core = MapperRom::new(543).prg(128).chr(0).load();
    assert_eq!([28, 29, 30, 31], core.prg_windows());
    for value in [0x08, 0x00, 0x00] {
        core.mapper_write(0x5000, value);
    }
    assert_eq!([28, 29, 30, 31], core.prg_windows());
    core.mapper_write(0x5000, 0x00);
    assert_eq!([60, 61, 62, 63], core.prg_windows());
}

/// `mapper_370.c`: mode 1 (`$5001`) takes a CIRAM page per table from R0/R1 D7, inverted; the
/// outer bank is `$50xx` D3-D5.
#[cfg(feature = "board-370")]
#[test]
fn mappers_p_370_pages_ciram_from_chr_registers() {
    let mut core = MapperRom::new(370).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x5009, 0);
    assert_eq!([17, 18, 30, 31], core.prg_windows());
    core.mmc3_bank(0, 0x80);
    core.mmc3_bank(1, 0x00);
    core.mapper_write(0x5001, 0);
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(
        [0, 0, 1, 1],
        [0, 1, 2, 3].map(|table| cartridge.nametable_page(table))
    );
}

/// `mapper_417.c`: `$8050-$8053` D0 the CIRAM page of each table.
#[cfg(feature = "board-417")]
#[test]
fn mappers_p_417_pages_ciram_per_table() {
    let mut core = MapperRom::new(417).prg(16).chr(64).load();
    for (index, value) in [1u8, 0, 0, 1].into_iter().enumerate() {
        core.mapper_write(0x8050 + u16::try_from(index).unwrap(), value);
    }
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(
        [1, 0, 0, 1],
        [0, 1, 2, 3].map(|table| cartridge.nametable_page(table))
    );
}

/// `mapper_344.c`: the address at `$6000` the outer bank; A2 the NROM-256 mode from R6; A3 the
/// DIP read over `$8000-$FFFF` (0).
#[cfg(feature = "board-344")]
#[test]
fn mappers_p_344_has_nrom_and_a_dip_read() {
    let mut core = MapperRom::new(344).prg(64).chr(256).load();
    mmc3_start(&mut core, 5, 2);
    core.mapper_write(0x6001, 0);
    assert_eq!([21, 18, 30, 31], core.prg_windows());
    core.mapper_write(0x6005, 0);
    assert_eq!([20, 21, 22, 23], core.prg_windows());
    core.mapper_write(0x6008, 0);
    assert_eq!(0, core.read_memory(0x8001)); // The CPU's cached path must not answer ROM there (`Mapper::rom_windows`).
    assert_eq!(None, core.cartridge().unwrap().read_rom_window(0x8001));
}

/// `mapper_432.c` (nesdev "NES 2.0 Mapper 432"): `$6001` D6 is NROM-128 from R6/R7; `$6000` D0
/// the solder-pad read.
#[cfg(feature = "board-432")]
#[test]
fn mappers_p_432_has_nrom_128_and_a_pad_read() {
    let mut core = MapperRom::new(432).prg(64).chr(256).load();
    mmc3_start(&mut core, 5, 6);
    core.mapper_write(0x6001, 0x10);
    assert_eq!([37, 38, 62, 63], core.prg_windows());
    core.mapper_write(0x6001, 0x50);
    assert_eq!([37, 38, 37, 38], core.prg_windows());
    core.mapper_write(0x6000, 0x01);
    assert_eq!(0, core.read_memory(0x8001)); // The CPU's cached path must not answer ROM there (`Mapper::rom_windows`).
    assert_eq!(None, core.cartridge().unwrap().read_rom_window(0x8001));
}

/// `mapper_356.c`: four registers in turn; `reg[2]` D6 is four-screen.
#[cfg(feature = "board-356")]
#[test]
fn mappers_p_356_has_a_four_screen_mode() {
    let mut core = MapperRom::new(356).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    for value in [0x00, 0x10, 0x6F, 0x30] {
        core.mapper_write(0x6000, value);
    }
    assert_eq!([17, 18, 30, 31], core.prg_windows());
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(
        zax_nes::NametableMirroring::FourScreen,
        cartridge.mirroring()
    );
}

/// `mapper_269.c`: without CHR-ROM the CHR is the PRG with its data lines permuted (D0 to D6);
/// four `$5000` writes set the offsets and masks.
#[cfg(feature = "board-269")]
#[test]
fn mappers_p_269_reads_chr_from_permuted_prg() {
    let mut core = MapperRom::new(269).prg(16).chr(0).load();
    core.mmc3_bank(2, 8);
    assert_eq!(0x40, core.chr_windows()[4]);
    mmc3_start(&mut core, 1, 2);
    for value in [0x00, 0x04, 0x0F, 0x38] {
        core.mapper_write(0x5000, value);
    }
    // PRG offset 4 ORed with the chip's banks under mask 7: 1, 2 and the fixed 6, 7.
    assert_eq!([5, 6, 6, 7], core.prg_windows());
}

/// `mapper_527.c`: D7 of CHR register 0 (`$B001` high nibble) pages `$2000`/`$2400`.
#[cfg(feature = "board-527")]
#[test]
fn mappers_p_527_pages_ciram_from_chr() {
    let mut core = MapperRom::new(527).prg(16).chr(256).load();
    core.mapper_write(0xB001, 0x08);
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(
        [1, 1, 0, 0],
        [0, 1, 2, 3].map(|table| cartridge.nametable_page(table))
    );
}

/// `mapper_353.c`: `$A080` selects game 1; `$E080` game 3, whose `$C000`/`$E000` are R6/R7.
#[cfg(feature = "board-353")]
#[test]
fn mappers_p_353_selects_games_from_x080() {
    let mut core = MapperRom::new(353).prg(128).chr(512).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0xA080, 0);
    assert_eq!([33, 34, 62, 63], core.prg_windows());
    core.mapper_write(0xE080, 0);
    assert_eq!([97, 98, 113, 114], core.prg_windows());
}

/// `mapper_100.c`: the select byte loads PRG and CHR registers directly; `$46`/`$47` load
/// `prg[2]` and `prg[1]`, `$80` the CHR pair at `$1000`.
#[cfg(feature = "board-100")]
#[test]
fn mappers_p_100_loads_raw_bank_registers() {
    let mut core = MapperRom::new(100).prg(32).chr(64).load();
    assert_eq!([0, 1, 30, 31], core.prg_windows());
    core.mmc3_bank(0x06, 5);
    core.mmc3_bank(0x46, 9);
    core.mmc3_bank(0x47, 12);
    assert_eq!([5, 12, 9, 31], core.prg_windows());
    core.mmc3_bank(0x80, 20);
    core.mmc3_bank(0x85, 33);
    assert_eq!([0, 1, 2, 33, 20, 21, 6, 7], core.chr_windows());
}

/// `mapper_559.c`: the chip's index lines are A10/A11, so `$8000` is PRG register 0 and
/// `$9C00` the board's `$C000` bank; `$9C04-$9C07` are the nametable registers; `$B400` (A10)
/// takes the data's high nibble.
#[cfg(feature = "board-559")]
#[test]
fn mappers_p_559_indexes_by_a10_a11() {
    let mut core = MapperRom::new(559).prg(64).chr(256).load();
    assert_eq!([0, 1, 62, 31], core.prg_windows());
    core.mapper_write(0x8000, 0x25);
    core.mapper_write(0x9C00, 0x07);
    assert_eq!([5, 1, 7, 31], core.prg_windows());
    core.mapper_write(0xB000, 0x03);
    core.mapper_write(0xB400, 0x10);
    assert_eq!(0x13, core.chr_windows()[0]);
    core.mapper_write(0x9C05, 0xE1);
    let cartridge = core.cartridge().unwrap();
    assert_eq!(0, cartridge.nametable_page(0));
    assert_eq!(1, cartridge.nametable_page(1));
}

/// `mapper_557.c`: R6/R7 masked to 4 bits, banks `$0E`/`$0F` fixed, CHR fixed, R5 D5 horizontal.
#[cfg(feature = "board-557")]
#[test]
fn mappers_p_557_is_a_namcot_118_with_fixed_chr() {
    let mut core = MapperRom::new(557).prg(32).chr(8).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.mmc3_bank(6, 0x13);
    core.mmc3_bank(7, 0x05);
    core.mmc3_bank(0, 0x40);
    assert_eq!([3, 5, 14, 15], core.prg_windows());
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
}

/// `mapper_555.c`: `$5800` the outer PRG bank and mask, `$5800-$5FFF` reads the timer status,
/// `$5000-$57FF` is RAM, and mode 2 puts a CHR register with D6 on the CHR-RAM.
#[cfg(feature = "board-555")]
#[test]
fn mappers_p_555_has_an_outer_bank_ram_at_5000_and_mixed_chr() {
    let mut core = MapperRom::new(555).prg(64).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    assert_eq!([1, 2, 6, 7], core.prg_windows());
    core.mapper_write(0x5800, 0x05);
    assert_eq!([33, 34, 46, 47], core.prg_windows());
    assert_eq!(0x5C, core.read_memory(0x5800));
    core.mapper_write(0x5010, 0x77);
    assert_eq!(0x77, core.read_memory(0x5010));
    core.mapper_write(0x5800, 0x02);
    core.mmc3_bank(2, 0x41);
    core.mmc3_bank(3, 0x05);
    let windows = core.chr_windows();
    assert_eq!((0, 5), (windows[4], windows[5]));
}

/// `mapper_471.c`: the written address is the 32 KiB PRG and 8 KiB CHR bank; a filtered A12
/// rise asserts the IRQ and any write acknowledges it.
#[cfg(feature = "board-471")]
#[test]
fn mappers_p_471_asserts_on_a12_and_acknowledges_on_write() {
    let mut core = MapperRom::new(471).prg(32).chr(64).load();
    core.mapper_write(0x8005, 0);
    assert_eq!([20, 21, 22, 23], core.prg_windows());
    assert_eq!(40, core.chr_windows()[0]);
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x0000, 0);
    cartridge.on_ppu_address_bus(0x1000, 40);
    assert!(cartridge.irq_asserted());
    core.mapper_write(0x8005, 0);
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// `mapper_526.c`: `$8008-$800B` the four PRG banks, `$8000-$8007` the CHR banks; the counter
/// asserts at bit 12 (4096 cycles) and `$800F` acknowledges.
#[cfg(feature = "board-526")]
#[test]
fn mappers_p_526_banks_and_counts_cpu_cycles() {
    let mut core = MapperRom::new(526).prg(32).chr(64).load();
    assert_eq!([28, 29, 30, 31], core.prg_windows());
    core.mapper_write(0x8009, 7);
    core.mapper_write(0x8003, 33);
    assert_eq!([28, 7, 30, 31], core.prg_windows());
    assert_eq!(33, core.chr_windows()[3]);
    core.mapper_write(0x800D, 0);
    for _ in 0..4100 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
    core.mapper_write(0x800D, 0);
    core.mapper_write(0x800F, 0);
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// `mapper_539.c`: banks `$0C`, `$A000`'s register, `$0E`, `$0F` and `$0D` at `$6000`, with RAM
/// pieces over `$6000-$60FF`, `$8200-$82FF` and `$C000-$D1FF`; `$F000` D3 the mirroring; the
/// CHR-RAM powers up as `$00 $00 $FF $FF`.
#[cfg(feature = "board-539")]
#[test]
fn mappers_p_539_lays_ram_over_its_rom() {
    let mut core = MapperRom::new(539).prg(32).chr(0).load();
    core.mapper_write(0xA000, 5);
    assert_eq!([12, 5, 0, 15], core.prg_windows());
    assert_eq!(13, core.read_memory(0x6100));
    core.mapper_write(0xC010, 0x44);
    core.mapper_write(0x8210, 0x55);
    core.mapper_write(0x6010, 0x66);
    assert_eq!(
        [0x44, 0x55, 0x66],
        [0xC010, 0x8210, 0x6010].map(|address| core.read_memory(address))
    );
    let cartridge = core.cartridge().unwrap();
    assert_eq!(
        zax_nes::NametableMirroring::Horizontal,
        cartridge.mirroring()
    );
    assert_eq!([0, 0, 0xFF, 0xFF], [0, 1, 2, 3].map(|a| core.chr_bank(a)));
    core.mapper_write(0xF000, 0);
    assert_eq!(
        zax_nes::NametableMirroring::Vertical,
        core.cartridge().unwrap().mirroring()
    );
}

/// `mapper_558.c`: `$5000`/`$5100` the 32 KiB bank (D0/D1 swapped while `$5300` D0 is clear,
/// D0-D1 forced on while `$5300` D2 is clear); `$5000` reads `$5200` D2 back without an EEPROM
/// and the 93C66's DO (idle high) with one.
#[cfg(feature = "board-558")]
#[test]
fn mappers_p_558_banks_and_reads_its_eeprom_port() {
    let mut core = MapperRom::new(558).prg(128).chr(0).load();
    assert_eq!([12, 13, 14, 15], core.prg_windows());
    core.mapper_write(0x5300, 0x05);
    core.mapper_write(0x5000, 0x02);
    core.mapper_write(0x5100, 0x01);
    assert_eq!([72, 73, 74, 75], core.prg_windows());
    core.mapper_write(0x5300, 0x04);
    core.mapper_write(0x5000, 0x01);
    assert_eq!([72, 73, 74, 75], core.prg_windows());
    core.mapper_write(0x5200, 0x00);
    assert_eq!(0, core.read_memory(0x5000));
    core.mapper_write(0x6000, 0x12);
    assert_eq!(0x12, core.read_memory(0x6000));

    let mut core = MapperRom::new(558)
        .prg(128)
        .chr(0)
        .prg_ram(0x2000, 512)
        .load();
    core.mapper_write(0x5200, 0x00);
    assert_eq!(0x04, core.read_memory(0x5000));
}

/// `mapper_272.c`: `$8000`/`$A000` the PRG banks, `$B000`/`$B001` a CHR register's nibbles,
/// `$8004` the single-screen mirroring; `$C008` enables the counter, whose 84th low-plane
/// pattern fetch asserts the IRQ, and `$C004` acknowledges.
#[cfg(feature = "board-272")]
#[test]
fn mappers_p_272_counts_pattern_fetches() {
    let mut core = MapperRom::new(272).prg(32).chr(256).load();
    core.mapper_write(0x8000, 3);
    core.mapper_write(0xA000, 4);
    core.mapper_write(0xB000, 0x05);
    core.mapper_write(0xB001, 0x02);
    assert_eq!([3, 4, 30, 31], core.prg_windows());
    assert_eq!(0x25, core.chr_windows()[0]);
    core.mapper_write(0x8004, 0x03);
    assert_eq!(
        zax_nes::NametableMirroring::SingleScreenB,
        core.cartridge().unwrap().mirroring()
    );
    core.mapper_write(0xC008, 0);
    let cartridge = core.cartridge_mut().unwrap();
    for fetch in 0..84 {
        assert!(!cartridge.irq_asserted(), "fetch {fetch}");
        cartridge.on_ppu_address_bus(0x0000, 0);
        cartridge.on_ppu_address_bus(0x0008, 0);
    }
    assert!(cartridge.irq_asserted());
    core.mapper_write(0xC004, 0);
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// `mapper_347.c`, the real mask-ROM layout: `$C000` and `$6C00` from `$9000`'s A0-A3, `$7000`
/// from `$8000`'s above `$10000`, the RAM pieces at `$6000`, `$B800` and `$CC00`.
#[cfg(feature = "board-347")]
#[test]
fn mappers_p_347_maps_its_pieces() {
    let mut core = MapperRom::new(347).prg(16).chr(0).load();
    assert_eq!([12, 13, 7, 15], core.prg_windows());
    core.mapper_write(0x9004, 0);
    core.mapper_write(0x8003, 0);
    assert_eq!(
        [2, 2, 9],
        [0xC100, 0x6C00, 0x7000].map(|address| core.read_memory(address))
    );
    core.mapper_write(0xB810, 0x21);
    core.mapper_write(0xCC10, 0x22);
    core.mapper_write(0x6010, 0x23);
    assert_eq!(
        [0x21, 0x22, 0x23],
        [0xB810, 0xCC10, 0x6010].map(|address| core.read_memory(address))
    );
    core.mapper_write(0x8008, 0);
    assert_eq!(
        zax_nes::NametableMirroring::Horizontal,
        core.cartridge().unwrap().mirroring()
    );
}

/// `mapper_442.c`: `$5000` D6 and D0-D4 the 32 KiB bank; with D7 set a pattern fetch takes CHR
/// A12 from PPU A9 latched on the last rise of A13.
#[cfg(feature = "board-442")]
#[test]
fn mappers_p_442_banks_and_latches_a9_for_its_1bpp_mode() {
    let mut core = MapperRom::new(442).prg(256).chr(8).load();
    core.mapper_write(0x5000, 0x43);
    assert_eq!([140, 141, 142, 143], core.prg_windows());
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x0000, 0);
    cartridge.on_ppu_address_bus(0x2200, 2);
    cartridge.on_ppu_address_bus(0x0000, 4);
    assert_eq!(0, core.chr_bank(0x0000));
    core.mapper_write(0x5000, 0xC3);
    assert_eq!(4, core.chr_bank(0x0000));
}

/// `mapper_452.c`: a write to ROM latches address and data; data D3 the four 8 KiB banks from
/// the address, D4-D5 the window the work RAM covers, where a write is a RAM write.
#[cfg(feature = "board-452")]
#[test]
fn mappers_p_452_latches_rom_writes_and_moves_its_ram() {
    let mut core = MapperRom::new(452).prg(32).chr(0).load();
    assert_eq!([0, 1, 0, 1], core.prg_windows());
    core.mapper_write(0xA00A, 0x18);
    assert_eq!([4, 0, 6, 7], core.prg_windows());
    core.mapper_write(0xA010, 0x77);
    assert_eq!(0x77, core.read_memory(0xA010));
    assert_eq!([4, 0, 6, 7], core.prg_windows());
}

/// `mapper_362.c`: game 0 takes the outer banks from D7-D8 of the CHR register of the window
/// the PPU last fetched from; a reset of an image above 512 KiB switches to game 1 (`$40`).
#[cfg(feature = "board-362")]
#[test]
fn mappers_p_362_follows_the_fetched_chr_window() {
    let mut core = MapperRom::new(362).prg(128).chr(1024).load();
    core.mapper_write(0xB000, 0x00);
    core.mapper_write(0xB001, 0x18);
    core.mapper_write(0x8000, 0x03);
    assert_eq!([51, 49, 62, 63], core.prg_windows());
    core.cartridge_mut().unwrap().on_ppu_address_bus(0x0400, 0);
    assert_eq!([3, 1, 14, 15], core.prg_windows());
    assert_eq!(None, core.cartridge().unwrap().read_rom_window(0x8000));
    core.soft_reset();
    assert_eq!([67, 65, 78, 79], core.prg_windows());
}

/// `mapper_398.c`: with D7 of the last write's low address byte set, a 32 KiB bank from its
/// D6-D7 and D2 of the fetched window's CHR register, an 8 KiB CHR bank from D6 and D0-D2.
#[cfg(feature = "board-398")]
#[test]
fn mappers_p_398_banks_from_the_address_and_the_fetched_window() {
    let mut core = MapperRom::new(398).prg(64).chr(1024).load();
    core.mapper_write(0xB0C0, 0x05);
    assert_eq!([28, 29, 30, 31], core.prg_windows());
    // 8 KiB bank `$4D`, whose first 1 KiB bank is 616 (`$68` in the fixture's byte).
    assert_eq!(0x68, core.chr_windows()[0]);
    core.mapper_write(0x8000, 0x02);
    assert_eq!([2, 1, 14, 15], core.prg_windows());
}

/// `mapper_359.c`: `$9000` the outer PRG bank, `$9001` the masks, `$9003` the outer CHR bank;
/// the IRQ counts CPU cycles from `$C000`/`$C001`, or in its A12 mode filtered A12 rises.
#[cfg(feature = "board-359")]
#[test]
fn mappers_p_359_banks_and_counts_both_ways() {
    let mut core = MapperRom::new(359).prg(64).chr(256).load();
    assert_eq!([60, 61, 63, 63], core.prg_windows());
    assert_eq!(0, core.read_memory(0x6000));
    core.mapper_write(0x9000, 0x08);
    core.mapper_write(0x9001, 0x03);
    assert_eq!([28, 29, 31, 31], core.prg_windows());
    core.mapper_write(0xA000, 0x85);
    assert_eq!(5, core.chr_windows()[0]);
    core.mapper_write(0x9003, 0x01);
    assert_eq!(0x85, core.chr_windows()[0]);

    core.mapper_write(0xC000, 0x10);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xC002, 0x01);
    for _ in 0..20 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());

    // The A12 mode first, disabled, so that the CPU-cycle counter stops before the reload.
    core.mapper_write(0xC002, 0x02);
    core.mapper_write(0xC001, 0x02);
    core.mapper_write(0xC003, 0x01);
    for rise in 0..3 {
        assert!(!core.cartridge().unwrap().irq_asserted(), "rise {rise}");
        for _ in 0..8 {
            core.bus_mut().tick();
        }
        let cartridge = core.cartridge_mut().unwrap();
        cartridge.on_ppu_address_bus(0x0000, 0);
        cartridge.on_ppu_address_bus(0x1000, 0);
    }
    assert!(core.cartridge().unwrap().irq_asserted());
}

/// `mapper_359.c` as 540: four 2 KiB CHR banks from registers 0, 1, 6 and 7.
#[cfg(feature = "board-540")]
#[test]
fn mappers_p_540_has_2_kib_chr_banks() {
    let mut core = MapperRom::new(540).prg(64).chr(256).load();
    core.mapper_write(0xA000, 3);
    core.mapper_write(0xB002, 5);
    let windows = core.chr_windows();
    assert_eq!(
        [6, 7, 10, 11],
        [windows[0], windows[1], windows[4], windows[5]]
    );
}

/// `mapper_528.c`: an FME-7 command and its data in one write at `$A000`/`$C000`, `$C000` adding
/// the outer bank `$10`; command 8 at 1 is the work RAM at `$6000`; commands `$D`-`$F` the
/// VRC7 IRQ (control, acknowledge, latch).
#[cfg(feature = "board-528")]
#[test]
fn mappers_p_528_writes_fme7_commands_with_a_vrc7_irq() {
    let mut core = MapperRom::new(528).prg(64).chr(512).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.mapper_write(0xA009, 5);
    assert_eq!([5, 1, 14, 15], core.prg_windows());
    core.mapper_write(0xC00A, 3);
    assert_eq!([21, 19, 46, 47], core.prg_windows());
    assert_eq!(16, core.read_memory(0x6000));
    core.mapper_write(0xC000, 2);
    assert_eq!(2, core.chr_windows()[0]);
    core.mapper_write(0xA008, 1);
    core.mapper_write(0x6000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6000));

    core.mapper_write(0xA00F, 0xFE);
    core.mapper_write(0xA00D, 0x06);
    for _ in 0..3 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
    core.mapper_write(0xA00E, 0);
    assert!(!core.cartridge().unwrap().irq_asserted());
}

/// `mapper_369.c`: `$4100` the mode: `$01` NROM-256 bank 1, `$13` the *SMB2J* layout with ROM
/// bank `$0E` at `$6000` and a 12-bit CPU-cycle IRQ, `$37` the MMC3 at `$10`.
#[cfg(feature = "board-369")]
#[test]
fn mappers_p_369_switches_between_its_games() {
    let mut core = MapperRom::new(369).prg(64).chr(512).load();
    core.mapper_write(0x4100, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    assert_eq!(8, core.chr_windows()[0]);
    core.mapper_write(0x4100, 0x13);
    core.mapper_write(0xE000, 0x02);
    assert_eq!([12, 13, 10, 15], core.prg_windows());
    assert_eq!(14, core.read_memory(0x6000));
    core.mapper_write(0xA000, 0x02);
    for _ in 0..4096 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted());
    core.mapper_write(0x8000, 0);
    assert!(!core.cartridge().unwrap().irq_asserted());
    core.mapper_write(0x4100, 0x37);
    mmc3_start(&mut core, 1, 2);
    assert_eq!([17, 18, 30, 31], core.prg_windows());
}

/// `mapper_556.c`: four registers in turn at `$5000`; the outer PRG bank and mask apply to the
/// MMC3, and `reg[2]` D7 hands the board to a VRC4 (A0|A2, A1|A3) with the same outer bank.
#[cfg(feature = "board-556")]
#[test]
fn mappers_p_556_switches_from_mmc3_to_vrc4() {
    let mut core = MapperRom::new(556).prg(128).chr(512).load();
    mmc3_start(&mut core, 1, 2);
    for value in [0x00, 0x20, 0x0F, 0x30] {
        core.mapper_write(0x5000, value);
    }
    assert_eq!([33, 34, 46, 47], core.prg_windows());
    for value in [0x00, 0x20, 0x8F, 0x30] {
        core.mapper_write(0x5000, value);
    }
    core.mapper_write(0x8000, 0x05);
    core.mapper_write(0xA000, 0x06);
    assert_eq!([37, 38, 46, 47], core.prg_windows());
}

/// `mapper_351.c`: `$5001` the outer PRG bank of the MMC3; `$5000` = 2 hands the board to the
/// MMC1 (16 KiB banks through the same outer bank); `$5002` D4 an NROM-256 bank from `$5001`.
#[cfg(feature = "board-351")]
#[test]
fn mappers_p_351_switches_between_three_chips() {
    let mut core = MapperRom::new(351).prg(128).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mapper_write(0x5001, 0x40);
    assert_eq!([33, 34, 62, 63], core.prg_windows());
    core.mapper_write(0x5000, 0x02);
    core.mmc1_write(0x8000, 0x0C);
    core.mmc1_write(0xE000, 0x03);
    assert_eq!([38, 39, 62, 63], core.prg_windows());
    core.mapper_write(0x5002, 0x10);
    assert_eq!([32, 33, 34, 35], core.prg_windows());
    assert_eq!(0x50, core.read_memory(0x5000));
}

/// `mapper_512.c`: `$4100` D1 puts every CHR window on the CHR-RAM beside the CHR-ROM; `$4100`
/// = 1 makes the four tables the CHR-RAM's second 4 KiB once the chip's mirroring is written.
#[cfg(feature = "board-512")]
#[test]
fn mappers_p_512_maps_chr_ram_and_four_screen_tables() {
    let mut core = MapperRom::new(512).prg(32).chr(256).load();
    mmc3_start(&mut core, 1, 2);
    core.mmc3_bank(2, 5);
    assert_eq!(5, core.chr_windows()[4]);
    core.mapper_write(0x4100, 0x02);
    assert_eq!(0, core.chr_windows()[4]);
    core.mapper_write(0x4100, 0x01);
    core.mapper_write(0xA000, 0x00);
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(
        zax_nes::cartridges::Cartridge::SERVED_BY_CARTRIDGE,
        cartridge.nametable_page(2)
    );
}

/// `mapper_560.c`: any write toggles the 32 KiB bank (power-on 1); a write to table 1 is copied
/// to the extension RAM, whose byte for the latched tile is the CHR bank of the pattern fetches
/// in bank 1; in bank 0 the fetch's plane bit A3 becomes CHR A16.
#[cfg(feature = "board-560")]
#[test]
fn mappers_p_560_reads_chr_through_the_nametable() {
    let mut core = MapperRom::new(560).prg(32).chr(128).load();
    assert_eq!([4, 5, 6, 7], core.prg_windows());
    core.mapper_write(0x8000, 0);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!(64, core.chr_bank(0x0008));
    core.write_memory(0x2006, 0x24);
    core.write_memory(0x2006, 0x05);
    core.write_memory(0x2007, 0x03);
    core.mapper_write(0x8000, 0);
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x0000, 0);
    cartridge.on_ppu_address_bus(0x2405, 2);
    assert_eq!(6, core.chr_bank(0x0000));
}

/// `mapper_413.c`: `$E000` D6-D7 pick the register (`reg[1]` the bank at `$8000`); `$C000` D7
/// shifts the serial address in, `$D000` D1 makes reads advance it, and `$C000-$CFFF` reads
/// the misc ROM; `$8000` the IRQ latch, `$B000` the enable, A12 rises the clock.
#[cfg(feature = "board-413")]
#[test]
fn mappers_p_413_reads_its_misc_rom_serially_and_counts_a12() {
    let mut image = MapperRom::new(413).prg(16).chr(32).image();
    image.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    let mut core = zax_nes::NesCore::new();
    assert!(core.load_rom(&image, "Mapper413.nes").success);
    core.mapper_write(0xE000, 0x45);
    // `$C100` is the serial port, at address 0.
    assert_eq!([5, 0, 0x11, 4], core.prg_windows());
    core.mapper_write(0xC000, 0x80);
    core.mapper_write(0xC000, 0x00);
    core.mapper_write(0xD000, 0x02);
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(Some(0x33), cartridge.try_cpu_read(0xC000));
    assert_eq!(Some(0x44), cartridge.try_cpu_read(0xC000));

    core.mapper_write(0x8000, 1);
    core.mapper_write(0xB000, 0);
    let cartridge = core.cartridge_mut().unwrap();
    for (rise, clock) in [(0, 0), (1, 40)] {
        assert!(!cartridge.irq_asserted(), "rise {rise}");
        cartridge.on_ppu_address_bus(0x0000, clock);
        cartridge.on_ppu_address_bus(0x1000, clock + 20);
    }
    assert!(cartridge.irq_asserted());
}

/// `mapper_406.c` with `SST39SF040.c`: submapper 0 swaps A0 and A1 on the way to the MMC3, the
/// PRG-ROM is an MX29F040 (IDs `$C2`/`$A4`, unlock at chip `$5555`/`$2AAA`) that programs by
/// `AND`ing, and a sector erase reads `& $77` with D6 toggling until it ends.
#[cfg(feature = "board-406")]
#[test]
fn mappers_p_406_crosses_a0_a1_and_flashes_its_prg() {
    let mut core = MapperRom::new(406).prg(64).chr(32).submapper(0).load();
    let read =
        |core: &mut zax_nes::NesCore, address| core.cartridge_mut().unwrap().try_cpu_read(address);
    // `$8002` is the chip's `$8001`: R6 = 2, R7 = 1.
    for (register, bank) in [(6, 2), (7, 1)] {
        core.mapper_write(0x8000, register);
        core.mapper_write(0x8002, bank);
    }
    assert_eq!([2, 1, 62, 63], core.prg_windows());
    // Chip `$5555` is bank 2 at `$9555`, chip `$2AAA` bank 1 at `$AAAA`.
    let unlock = |core: &mut zax_nes::NesCore, command| {
        core.mapper_write(0x9555, 0xAA);
        core.mapper_write(0xAAAA, 0x55);
        core.mapper_write(0x9555, command);
    };
    unlock(&mut core, 0x90);
    assert_eq!(Some(0xC2), read(&mut core, 0x8000));
    assert_eq!(Some(0xA4), read(&mut core, 0x8001));
    core.mapper_write(0xA002, 0xF0);
    assert_eq!(Some(62), read(&mut core, 0xC000));
    unlock(&mut core, 0xA0);
    core.mapper_write(0xC000, 0x0F);
    assert_eq!(Some(0x3E & 0x0F), read(&mut core, 0xC000));
    assert_ne!(core.export_flash_save(), Vec::<u8>::new());
    unlock(&mut core, 0x80);
    core.mapper_write(0x9555, 0xAA);
    core.mapper_write(0xAAAA, 0x55);
    core.mapper_write(0xC000, 0x30);
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(Some(0x77), cartridge.try_cpu_read(0xE000));
    cartridge.step_cpu_cycle();
    assert_eq!(Some(0x37), cartridge.try_cpu_read(0xE000));
    for _ in 1..0x1_0000 {
        cartridge.step_cpu_cycle();
    }
    assert_eq!(Some(0xFF), cartridge.try_cpu_read(0xC000));
    assert_eq!(Some(2), core.cartridge_mut().unwrap().try_cpu_read(0x8000));
}

/// `mapper_451.c`: `$E000` A0-A1 place the PRG and the 8 KiB CHR bank, `$A000` A0 the mirroring;
/// the flash is an AMIC A29040B (IDs `$37`/`$86`, unlock at chip `$0555`/`$02AA`).
#[cfg(feature = "board-451")]
#[test]
fn mappers_p_451_banks_from_the_address_and_answers_its_ids() {
    let mut core = MapperRom::new(451).prg(64).chr(16).load();
    assert_eq!([0, 0x10, 0x20, 0x30], core.prg_windows());
    core.mapper_write(0xE003, 0);
    assert_eq!([0, 0x19, 0x29, 0x30], core.prg_windows());
    assert_eq!([8, 9, 10, 11, 12, 13, 14, 15], core.chr_windows());
    core.mapper_write(0x8555, 0xAA);
    core.mapper_write(0x82AA, 0x55);
    core.mapper_write(0x8555, 0x90);
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(Some(0x37), cartridge.try_cpu_read(0x8000));
    assert_eq!(Some(0x86), cartridge.try_cpu_read(0x8001));
    core.mapper_write(0x8000, 0xF0);
    assert_eq!(
        Some(0x19),
        core.cartridge_mut().unwrap().try_cpu_read(0xA000)
    );
}

/// `mapper_547.c`: `$D200-$D400` the PRG banks (D6 adds `$10`), `$E000` bank `$4F`; `$D000`
/// a 4 KiB work-RAM bank; `$DC00`/`$DD00` the JIS code in and its glyph's tile out; a nametable
/// write with `$DA00` D0 set lands in the QT RAM, whose byte a tile fetch latches to turn the
/// background pattern fetch to the kanji CHR-ROM (D6) while sprites see the CHR-RAM.
#[cfg(feature = "board-547")]
#[test]
fn mappers_p_547_reads_kanji_through_its_qt_ram() {
    let mut core = MapperRom::new(547).prg(128).chr(256).load();
    assert_eq!([0, 0, 0, 0x4F], core.prg_windows());
    core.mapper_write(0xD200, 0x45);
    core.mapper_write(0xD300, 0x03);
    assert_eq!([0x15, 3, 0, 0x4F], core.prg_windows());

    core.mapper_write(0xDC00, 0x21);
    core.mapper_write(0xDD00, 0x21);
    core.mapper_write(0xDB00, 0x06);
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(Some(0x84 | 0x02), cartridge.try_cpu_read(0xDC00));
    assert_eq!(Some(0xC0), cartridge.try_cpu_read(0xDD00));

    core.mapper_write(0xD000, 0x09);
    core.write_memory(0x6000, 0x5A);
    core.mapper_write(0xD000, 0x01);
    assert_eq!(
        Some(0x5A),
        core.cartridge_mut().unwrap().try_cpu_read(0x6000)
    );

    core.mapper_write(0xDA00, 0x01);
    core.write_memory(0x2006, 0x24);
    core.write_memory(0x2006, 0x05);
    core.write_memory(0x2007, 0xC5);
    core.mapper_write(0xDA00, 0x00);
    core.write_memory(0x2006, 0x00);
    core.write_memory(0x2006, 0x00);
    core.write_memory(0x2007, 0x77);
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x2405, 0);
    assert_eq!(20, core.chr_bank(0x0000));
    assert_eq!(0xFF, core.chr_bank(0x0008));
    let cartridge = core.cartridge().unwrap();
    assert_eq!(
        0x77,
        cartridge.ppu_peek(0x0000, zax_nes::cartridges::ChrFetch::Sprite)
    );
}

/// `mapper_561.c`: the misc ROM (load `$0300`, init `$0300`) is run at power-on from `$0700`;
/// the latch selects 16 KiB banks in mode 0, `$42FC` turns the latch off so that `$8000-$FFFF`
/// writes land in the copier RAM, `$43FC` D0 clear the four 8 KiB registers; `$4025` D7 runs
/// the FDS-style timer (D6 restarting it), asserting after 150 cycles (3 a cycle, 448).
#[cfg(feature = "board-561")]
#[test]
fn mappers_p_561_boots_its_trainer_and_writes_its_ram() {
    let mut image = MapperRom::new(561).prg(16).chr(8).image();
    image.extend_from_slice(&[0x00, 0x03, 0x00, 0x03, 0x60]);
    let mut core = zax_nes::NesCore::new();
    assert!(core.load_rom(&image, "Mapper561.nes").success);
    assert_eq!(0x60, core.read_memory(0x0300));
    assert_eq!(
        [0x20, 0x00, 0x03, 0x6C, 0xFC, 0xFF],
        std::array::from_fn::<u8, 6, _>(|i| core.read_memory(0x0700 + u16::try_from(i).unwrap()))
    );

    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.mapper_write(0x8000, 0x0C);
    assert_eq!([8, 9, 14, 15], core.prg_windows());
    core.mapper_write(0x42FC, 0x00);
    core.mapper_write(0x8100, 0x99);
    assert_eq!([0x99, 9, 14, 15], core.prg_windows());
    core.mapper_write(0x43FC, 0x00);
    assert_eq!(
        [3, 0x1D & 0x0F, 0x1E & 0x0F, 0x1F & 0x0F],
        core.prg_windows()
    );

    core.mapper_write(0x4025, 0xC0);
    let cartridge = core.cartridge_mut().unwrap();
    for _ in 0..149 {
        cartridge.step_cpu_cycle();
    }
    assert!(!cartridge.irq_asserted());
    cartridge.step_cpu_cycle();
    assert!(cartridge.irq_asserted());
}

/// `OneBus.c` through `mapper_256.c`: `$4107`/`$4108` and the MMC3 alias place the PRG banks,
/// `$2016`/`$2017` and `$2012-$2015` the CHR windows (a CPU write to `$2016` is the chip's, not
/// a PPU mirror), `$4106` the mirroring, `$4101`/`$4104` an A12 counter; with `$4030` D4 the
/// `$4012`/`$4013`/`$4015` writes run a PCM channel that feeds `$4011` and raises an IRQ.
#[cfg(feature = "board-256")]
#[test]
fn mappers_p_256_decodes_its_onebus_registers() {
    let mut core = MapperRom::new(256).prg(64).chr(64).load();
    assert_eq!([0, 1, 62, 63], core.prg_windows());
    core.mapper_write(0x4107, 5);
    assert_eq!([5, 1, 62, 63], core.prg_windows());
    core.mapper_write(0x8000, 6);
    core.mapper_write(0x8001, 7);
    assert_eq!([7, 1, 62, 63], core.prg_windows());
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());
    core.mapper_write(0x2016, 0x08);
    core.mapper_write(0x2012, 0x20);
    assert_eq!([8, 9, 2, 3, 0x20, 5, 6, 7], core.chr_windows());

    core.mapper_write(0x4101, 2);
    core.mapper_write(0x4104, 0);
    let cartridge = core.cartridge_mut().unwrap();
    for (rise, clock) in [(0, 0), (1, 40)] {
        assert!(!cartridge.irq_asserted(), "rise {rise}");
        cartridge.on_ppu_address_bus(0x0000, clock);
        cartridge.on_ppu_address_bus(0x1000, clock + 20);
    }
    assert!(cartridge.irq_asserted());
    core.mapper_write(0x4103, 0);

    core.mapper_write(0x4030, 0x11);
    core.mapper_write(0x4012, 0x00);
    core.mapper_write(0x4013, 0x00);
    core.mapper_write(0x4015, 0x10);
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.take_dac_write();
    let mut written = None;
    for _ in 0..0xE1 {
        cartridge.step_cpu_cycle();
        written = written.or(cartridge.take_dac_write());
    }
    assert_eq!(Some(7 >> 1), written);
    assert!(!cartridge.irq_asserted());
    for _ in 0..0xE1 {
        cartridge.step_cpu_cycle();
    }
    assert!(cartridge.irq_asserted());
    assert_eq!(0x80, cartridge.apu_status(0x00) & 0x80);
}

/// `mapper_436.c`: submapper 0 takes its outer PRG bank from `$4100` D6 and `$410F` D5; `$410A`
/// places the inner block.
#[cfg(feature = "board-436")]
#[test]
fn mappers_p_436_places_the_onebus_in_an_outer_bank() {
    let mut core = MapperRom::new(436).prg(256).chr(64).load();
    // `$410F` = `$FF` at power-on: D5 is A21 of the 8 KiB bank (`$400`), out of 2 MiB.
    assert_eq!([0, 1, 62, 63], core.prg_windows());
    core.mapper_write(0x410F, 0x00);
    core.mapper_write(0x4100, 0x40);
    assert_eq!([0, 1, 62, 63], core.prg_windows());
    // `$410A` above the mode's mask (`$3F` in mode 0) is the inner block.
    core.mapper_write(0x4100, 0x00);
    core.mapper_write(0x410A, 0x40);
    assert_eq!([0x40, 0x41, 0x7E, 0x7F], core.prg_windows());
}

/// `mapper_394.c`: `$5001` D4 clear runs the MMC3 (power-on), set the J.Y. ASIC; both through
/// the outer PRG bank of `$5001` D0-D1 and `$5003` D3, the MMC3's also through `$5001` D3's NROM
/// mode (a 32 KiB bank from `$5003` D0-D2).
#[cfg(feature = "board-394")]
#[test]
fn mappers_p_394_switches_between_the_mmc3_and_the_jy_asic() {
    let mut core = MapperRom::new(394).prg(64).chr(256).load();
    // `$5001` = `$0F`: outer bank `$60` (wrapped to 512 KiB), the MMC3's power-on banks (R6 and
    // R7 at 0) inside it.
    assert_eq!([32, 32, 62, 63], core.prg_windows());
    core.mapper_write(0x5001, 0x08);
    assert_eq!([0, 0, 30, 31], core.prg_windows());
    core.mapper_write(0x5003, 0x92);
    core.mapper_write(0x5001, 0x00);
    assert_eq!([4, 5, 6, 7], core.prg_windows());

    core.mapper_write(0x5001, 0x18);
    assert_eq!([28, 29, 30, 31], core.prg_windows());
    core.mapper_write(0xD000, 0x02);
    core.mapper_write(0x8000, 0x05);
    assert_eq!([5, 0, 0, 31], core.prg_windows());
    core.mapper_write(0x5001, 0x08);
    assert_eq!([0, 0, 30, 31], core.prg_windows());
}

/// `mapper_446.c`: unlocked, `$8000` is the outer bank and `$A000-$FFFF` banks `$3D-$3F`, and
/// the PRG-ROM is a flash chip (IDs `$01`/`$76`, unlock at chip `$0AAA`/`$0555`); `$5000` D7 locks
/// a personality inside the outer bank: UNROM by its latch, the MMC3 (submapper 1; submapper 0
/// turns mode 1 into the SNROM), the VRC6 with its sound.
#[cfg(feature = "board-446")]
#[test]
fn mappers_p_446_flashes_its_menu_and_locks_a_personality() {
    let mut core = MapperRom::new(446).prg(64).chr(64).load();
    assert_eq!([0, 61, 62, 63], core.prg_windows());
    core.mapper_write(0x5001, 0x08);
    assert_eq!([8, 61, 62, 63], core.prg_windows());
    core.mapper_write(0x5001, 0x00);
    core.mapper_write(0x8AAA, 0xAA);
    core.mapper_write(0x8555, 0x55);
    core.mapper_write(0x8AAA, 0x90);
    let cartridge = core.cartridge_mut().unwrap();
    assert_eq!(Some(0x01), cartridge.try_cpu_read(0x8000));
    assert_eq!(Some(0x76), cartridge.try_cpu_read(0x8001));
    core.mapper_write(0x8000, 0xF0);
    assert_eq!([0, 61, 62, 63], core.prg_windows());

    core.mapper_write(0x5001, 0x08);
    core.mapper_write(0x5000, 0x80);
    assert_eq!([8, 9, 62, 63], core.prg_windows());
    core.mapper_write(0x8000, 0x01);
    assert_eq!([10, 11, 62, 63], core.prg_windows());
    // Locked: `$5000-$5FFF` no longer moves anything.
    core.mapper_write(0x5001, 0x00);
    assert_eq!([10, 11, 62, 63], core.prg_windows());

    let mut core = MapperRom::new(446).prg(64).chr(64).submapper(1).load();
    core.mapper_write(0x5001, 0x10);
    core.mapper_write(0x5003, 0xF0);
    core.mapper_write(0x5000, 0x81);
    assert_eq!([16, 17, 30, 31], core.prg_windows());

    let mut core = MapperRom::new(446).prg(64).chr(64).load();
    core.mapper_write(0x5001, 0x10);
    core.mapper_write(0x5003, 0xF0);
    core.mapper_write(0x5000, 0x8E);
    assert_eq!([16, 17, 18, 31], core.prg_windows());
    assert!(core.cartridge().unwrap().expansion_audio_output().is_some());
}

/// `mapper_342.c` (COOLGIRL): power-on 16 KiB banks `prg.a` = 0 and `prg.c` = `$FE` through the
/// `$F8` mask; unlocked, `$5000-$5007` set the base, mask and personality (here `UxROM`, then the
/// MMC3); the top 8 MiB of the bank space is the flash's save area, programmed by the JEDEC
/// sequence while `$5007` D2 is set, and `$98` at `$0AAA` shows the CFI table.
#[cfg(feature = "board-342")]
#[test]
fn mappers_p_342_runs_its_personalities_and_flashes_its_save_area() {
    let mut core = MapperRom::new(342).prg(32).load();
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    // `UxROM` (mapper 1) with the whole bank space.
    core.mapper_write(0x5002, 0x00);
    core.mapper_write(0x5006, 0x01);
    core.mapper_write(0x8000, 0x05);
    assert_eq!([10, 11, 30, 31], core.prg_windows());

    // The save area: base `$FE00` maps `prg.a` 0 to bank `$1FC00`.
    core.mapper_write(0x5000, 0xFE);
    core.mapper_write(0x5001, 0x00);
    core.mapper_write(0x5006, 0x00);
    core.mapper_write(0x5007, 0x04);
    assert_eq!(0, core.read_memory(0x8000));
    for (address, value) in [
        (0x8AAA, 0xAA),
        (0x8555, 0x55),
        (0x8AAA, 0xA0),
        (0x8000, 0x42),
    ] {
        core.mapper_write(address, value);
    }
    assert_eq!(0x42, core.read_memory(0x8000));
    core.mapper_write(0x8AAA, 0x98);
    assert_eq!(0x51, core.read_memory(0x8020));
    core.mapper_write(0x8000, 0xF0);
    assert_eq!(0x42, core.read_memory(0x8000));

    // The MMC3 (mapper 20): its counter runs at dot 260 of the lines the board sees end.
    core.mapper_write(0x5007, 0x00);
    core.mapper_write(0x5006, 0x14);
    core.mapper_write(0xC000, 1);
    core.mapper_write(0xC001, 0);
    core.mapper_write(0xE001, 0);
    let cartridge = core.cartridge_mut().unwrap();
    for line in 0..2 {
        assert!(!cartridge.irq_asserted(), "line {line}");
        let base = line * 341;
        for (address, dot) in [
            (0x2400, 0),
            (0x2400, 1),
            (0x2400, 2),
            (0x23C0, 3),
            (0x1000, 263),
        ] {
            cartridge.on_ppu_address_bus(address, base + dot);
        }
    }
    assert!(cartridge.irq_asserted());
}

/// `mapper_562.c` (Venus Turbo Game Doctor): the BIOS at `$4800-$4FFF`; power-on mode 0 with
/// the latch on (16 KiB bank from the latch, `$C000` bank 7); `$42FC` with D1 clear turns the
/// latch off and `$8000-$FFFF` writes then land in the DRAM; the 16-bit counter asserts on
/// reaching its target.
#[cfg(feature = "board-562")]
#[test]
fn mappers_p_562_runs_the_game_doctor_copier() {
    let mut core = MapperRom::new(562).prg(32).chr(8).load();
    // The BIOS's jump table at `$4FF1`.
    assert_eq!(0x4C, core.read_memory(0x4FF1));
    assert_eq!([0, 1, 14, 15], core.prg_windows());
    core.mapper_write(0x8000, 0x05);
    assert_eq!([10, 11, 14, 15], core.prg_windows());
    // `$4408`: `prg[0] << 2` (`$05 >> 2`) with the latch's D0-D1.
    assert_eq!(0x05, core.read_memory(0x4408));

    core.mapper_write(0x42FD, 0x00);
    core.mapper_write(0x8100, 0x77);
    assert_eq!(0x77, core.read_memory(0x8100));

    core.mapper_write(0x440C, 0x00);
    core.mapper_write(0x440D, 0x10);
    core.mapper_write(0x440C, 0x80);
    let cartridge = core.cartridge_mut().unwrap();
    for _ in 0..0x10 {
        cartridge.step_cpu_cycle();
    }
    assert!(!cartridge.irq_asserted());
    cartridge.step_cpu_cycle();
    assert!(cartridge.irq_asserted());
}

/// `mapper_355.c`: a 1 KiB NES 2.0 misc ROM is the PIC16C54's program; the PIC reads the CPU's
/// last address (A12 on port B D0) and its port A D0 drives the IRQ (`pic16c5x_wr`). The program
/// here: `tris A` with W = 0 (a `tris` write releases the line), wait for A12, `clrf PORTA`
/// (D0 low asserts it).
#[cfg(feature = "board-355")]
#[test]
fn mappers_p_355_runs_its_pic_on_the_cpu_address_bus() {
    let mut image = MapperRom::new(355).prg(4).chr(8).image();
    image[14] = 1;
    let mut program = vec![0u8; 0x400];
    let words: [(usize, u16); 7] = [
        (0, 0xC00),     // movlw 0
        (1, 0x005),     // tris PORTA
        (2, 0x706),     // btfss PORTB, 0
        (3, 0xA02),     // goto 2
        (4, 0x065),     // clrf PORTA
        (5, 0xA05),     // goto 5
        (0x1FF, 0xA00), // reset vector: goto 0
    ];
    for (word, opcode) in words {
        program[word * 2..word * 2 + 2].copy_from_slice(&opcode.to_le_bytes());
    }
    image.extend_from_slice(&program);
    let mut core = zax_nes::NesCore::new();
    assert!(core.load_rom(&image, "Mapper355.nes").success);
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.try_cpu_read(0x8000);
    for _ in 0..80 {
        cartridge.step_cpu_cycle();
    }
    assert!(!cartridge.irq_asserted());
    cartridge.try_cpu_read(0x9000);
    for _ in 0..40 {
        cartridge.step_cpu_cycle();
    }
    assert!(cartridge.irq_asserted());
}

/// puNES `unif.c`: a UNIF image is read as the NES 2.0 image it describes; `UNL-CHINA_ER_SAN2`
/// is mapper 532 with a battery (`find_board`), its `FONT` chunk the misc ROM, and the board runs
/// mapper 19 (`map_init_532`). An unknown board is refused by name.
#[cfg(feature = "board-532")]
#[test]
fn mappers_p_532_loads_from_unif_with_its_font() {
    fn chunk(image: &mut Vec<u8>, id: [u8; 4], data: &[u8]) {
        image.extend_from_slice(&id);
        image.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        image.extend_from_slice(data);
    }
    let unif = |board: &[u8]| {
        let mut image = b"UNIF".to_vec();
        image.extend_from_slice(&7u32.to_le_bytes());
        image.resize(32, 0);
        chunk(&mut image, *b"MAPR", board);
        let prg: Vec<u8> = (0..4u8).flat_map(|bank| vec![bank; 0x2000]).collect();
        chunk(&mut image, *b"PRG0", &prg);
        chunk(&mut image, *b"CHR0", &[0x11; 0x2000]);
        chunk(&mut image, *b"FONT", &[0xAA; 0x100]);
        chunk(&mut image, *b"MIRR", &[1]);
        image
    };
    let mut core = zax_nes::NesCore::new();
    let load = core.load_rom(&unif(b"UNL-CHINA_ER_SAN2\0"), "ersan2.unf");
    assert!(load.success, "{:?}", load.error);
    let cartridge = core.cartridge().unwrap();
    assert_eq!(532, cartridge.mapper_number());
    assert!(cartridge.rom().header().has_battery);
    assert_eq!(0xAA, *cartridge.rom().raw().last().unwrap());
    assert_eq!(3, core.read_memory(0xE100));

    let load = core.load_rom(&unif(b"NO-SUCH-BOARD\0"), "x.unf");
    assert!(!load.success);
    assert!(format!("{:?}", load.error).contains("NO-SUCH-BOARD"));
}

/// Mesen `StudyBox.h`: a `.studybox` tape packaged with a 256 KiB BIOS loads as mapper 186
/// (a bare iNES 186 stays refused); `$4201` pages the BIOS at `$8000`, `$4400-$4FFF` is work-RAM
/// page 8; a drive command clocked in on `$4202` D4's falling edges (`$86`, the next page) reads
/// the page's lead-in, then asserts the IRQ and serves the page's bytes at `$4200` while the
/// tape's audio plays as the expansion audio.
#[cfg(feature = "board-186")]
#[test]
fn mappers_p_186_reads_a_study_box_tape() {
    fn chunk(file: &mut Vec<u8>, id: [u8; 4], body: &[u8]) {
        file.extend_from_slice(&id);
        file.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
        file.extend_from_slice(body);
    }
    fn page(lead_in: u32, audio: u32, data: &[u8]) -> Vec<u8> {
        let mut body = lead_in.to_le_bytes().to_vec();
        body.extend_from_slice(&audio.to_le_bytes());
        body.extend_from_slice(data);
        body
    }
    // 8 kHz, mono, 16-bit, a constant 1000.
    let mut wave = b"RIFF\0\0\0\0WAVE".to_vec();
    let mut format = 1u16.to_le_bytes().to_vec();
    format.extend_from_slice(&1u16.to_le_bytes());
    format.extend_from_slice(&8000u32.to_le_bytes());
    format.extend_from_slice(&16000u32.to_le_bytes());
    format.extend_from_slice(&2u16.to_le_bytes());
    format.extend_from_slice(&16u16.to_le_bytes());
    chunk(&mut wave, *b"fmt ", &format);
    let samples: Vec<u8> = (0..4000).flat_map(|_| 1000i16.to_le_bytes()).collect();
    chunk(&mut wave, *b"data", &samples);

    let mut tape = b"STBX".to_vec();
    tape.extend_from_slice(&4u32.to_le_bytes());
    tape.extend_from_slice(&0x100u32.to_le_bytes());
    chunk(&mut tape, *b"PAGE", &page(0, 4, &[0xC5, 1, 2, 3, 4, 0]));
    chunk(&mut tape, *b"PAGE", &page(10, 18, &[0x5A, 9, 9, 9, 9, 1]));
    let mut audio = 0u32.to_le_bytes().to_vec();
    audio.extend_from_slice(&wave);
    chunk(&mut tape, *b"AUDI", &audio);

    let bios: Vec<u8> = (0..16u8).flat_map(|page| vec![page; 0x4000]).collect();
    let image = zax_nes::rom::package_study_box(&tape, &bios).unwrap();
    assert!(zax_nes::rom::package_study_box(&tape, &bios[..0x1000]).is_err());
    let mut core = zax_nes::NesCore::new();
    let load = core.load_rom(&image, "lesson.studybox");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(186, core.cartridge().unwrap().mapper_number());
    assert_eq!([0, 0, 0, 0], core.prg_windows());
    core.mapper_write(0x4201, 5);
    assert_eq!([5, 5, 0, 0], core.prg_windows());
    core.mapper_write(0x4400, 0x42);
    assert_eq!(0x42, core.read_memory(0x4400));

    // `$86`, MSB first: D4 high, then low with the bit in D7.
    for bit in (0..8).rev() {
        core.mapper_write(0x4202, 0x10);
        core.mapper_write(0x4202, ((0x86u8 >> bit) & 1) << 7);
    }
    core.mapper_write(0x4202, 0x03);
    let cartridge = core.cartridge_mut().unwrap();
    // Lead-in: 8 samples at 8 kHz, 1 789 CPU cycles from the command's last bit (the writes
    // since then ran a few).
    let mut cycles = 0;
    while !cartridge.irq_asserted() && cycles < 4000 {
        cartridge.step_cpu_cycle();
        cycles += 1;
    }
    assert!((1770..1789).contains(&cycles), "{cycles}");
    assert_eq!(Some(0xAA), cartridge.try_cpu_read(0x4200));
    assert!(!cartridge.irq_asserted());
    for _ in 0..7820 {
        cartridge.step_cpu_cycle();
    }
    assert!(cartridge.irq_asserted());
    assert_eq!(Some(0x5A), cartridge.try_cpu_read(0x4200));
    assert_eq!(Some(1000), cartridge.expansion_audio_output());

    let state = core.save_state(true).unwrap();
    assert!(core.try_load_state(&state, 0).is_ok());
    assert_eq!(state, core.save_state(true).unwrap());
}

/// The COOLGIRL's save area goes through `export_flash_save`/`import_flash_save`, so a host keeps
/// it on disk like a flash chip's sectors: what a game wrote comes back in a new session.
#[cfg(feature = "board-342")]
#[test]
fn mappers_p_342_save_area_survives_a_new_session() {
    let select_save = |core: &mut zax_nes::NesCore| {
        core.mapper_write(0x5000, 0xFE);
        core.mapper_write(0x5001, 0x00);
        core.mapper_write(0x5007, 0x04);
    };
    let mut core = MapperRom::new(342).prg(32).load();
    assert_eq!(Vec::<u8>::new(), core.export_flash_save());
    select_save(&mut core);
    for (address, value) in [
        (0x8AAA, 0xAA),
        (0x8555, 0x55),
        (0x8AAA, 0xA0),
        (0x8123, 0x42),
    ] {
        core.mapper_write(address, value);
    }
    let file = core.export_flash_save();
    assert_eq!(b"ZXCG", &file[..4]);

    let mut again = MapperRom::new(342).prg(32).load();
    assert!(!again.import_flash_save(b"not a save"));
    assert!(again.import_flash_save(&file));
    select_save(&mut again);
    assert_eq!(0x42, again.read_memory(0x8123));
}
