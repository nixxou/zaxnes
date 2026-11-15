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
