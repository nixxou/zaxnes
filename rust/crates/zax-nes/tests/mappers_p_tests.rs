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
    assert_eq!(0, core.read_memory(0x8001));
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
    assert_eq!(0, core.read_memory(0x8001));
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
    assert_eq!(zax_nes::NametableMirroring::FourScreen, cartridge.mirroring());
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
