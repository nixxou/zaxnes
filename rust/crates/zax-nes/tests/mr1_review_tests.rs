//! Findings of lot MR-1, the adversarial review of our mapper boards against Mesen 2
//! (`docs/review/MR-1-mappers.md`).
//!
//! Every test here reproduces one finding classified **our bug** and therefore **fails on
//! the current code**. They all carry `#[ignore = "MR-1: <finding id>"]` so that the suite
//! stays green: MR-1 is a review lot and fixes no board. Run them with
//! `cargo test --release -p zax-nes --test mr1_review_tests -- --ignored`.
//!
//! Each test asserts the behaviour the review concluded is correct — Mesen 2's, or the
//! spec's where REQ-CORE-100 pins it ("le comportement documenté sur nesdev"). The comment
//! above each test cites our code and Mesen's.
//!
//! They are grouped here rather than appended to `mapper_tests.rs` so that the lot that
//! fixes a board can find, un-ignore and move its test in one place.

// Test values are small: the narrowing casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::unwrap_used
)]

mod common;

use common::mapper_fixture::{MapperMachine, MapperRom};

// ================================================================ shared: bus conflicts
//
// MR-BUS-1. Mesen ANDs the written value with the PRG-ROM byte the same address holds
// before calling the board's register write, for the boards whose latch sits on the data
// bus with no CPU-write isolation: `BaseMapper::WriteRam`,
// `third_party/mesen2/Core/NES/BaseMapper.cpp:884-896`
//
//     if(_hasBusConflicts) { uint8_t prgValue = _prgPages[addr >> 8][(uint8_t)addr];
//                            value &= prgValue; }
//
// `HasBusConflicts()` is true for ColorDreams unconditionally
// (`Mappers/Unlicensed/ColorDreams.h:10`) and for submapper 2 of UNROM, CNROM and AXROM
// (`Nintendo/UNROM.h:21`, `Nintendo/CNROM.h:19`, `Nintendo/AXROM.h:20`).
//
// We have no such path at all: `Cartridge::cpu_write`
// (`src/cartridges/mod.rs:390-410`) hands the value to `write_register` unchanged.

/// MR-BUS-1 on mapper 11, where Mesen applies the conflict unconditionally. At power-on
/// $8000-$FFFF shows 32 KiB bank 0, whose first 8 KiB bank the fixture fills with 0x00, so
/// every bit of the written value is pulled low and the latch keeps its reset value.
#[test]
#[ignore = "MR-1: MR-BUS-1"]
fn mr_bus_1_color_dreams_write_is_anded_with_the_rom_byte() {
    let mut core = MapperRom::new(11).prg(8).chr(16).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // $8100 holds 0x00 (PRG bank 0). Mesen: value &= 0x00, so the latch stays 0.
    core.mapper_write(0x8100, 0x13);

    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "the written 0x13 must be pulled to 0x00 by the ROM byte at $8100"
    );
    assert_eq!([0; 8], core.chr_windows());
}

/// MR-BUS-1 on mapper 2, where Mesen gates the conflict on NES 2.0 submapper 2.
#[test]
#[ignore = "MR-1: MR-BUS-1"]
fn mr_bus_1_uxrom_submapper_2_write_is_anded_with_the_rom_byte() {
    let mut core = MapperRom::new(2).prg(8).chr(16).submapper(2).load();
    assert_eq!([0, 1, 6, 7], core.prg_windows());

    // $8100 holds 0x00, so bank 3 cannot be selected through it.
    core.mapper_write(0x8100, 0x03);

    assert_eq!([0, 1, 6, 7], core.prg_windows());
}

/// A plain mapper 2 (submapper 0) has no conflict: the same write must take effect. This
/// one passes today and guards the fix of MR-BUS-1 against over-reach.
#[test]
fn mr_bus_1_uxrom_without_a_submapper_has_no_conflict() {
    let mut core = MapperRom::new(2).prg(8).chr(16).load();
    core.mapper_write(0x8100, 0x03);
    assert_eq!([6, 7, 6, 7], core.prg_windows());
}

// ================================================================ mapper 11, Color Dreams

/// MR-11-1. The PRG field of the Color Dreams latch is four bits, not two.
///
/// Mesen: `SelectPrgPage(0, value & 0x0F)` with 32 KiB pages,
/// `Mappers/Unlicensed/ColorDreams.h:22`. Ours masks with `0x03`:
/// `src/cartridges/mappers/discrete.rs:283`, so a Color Dreams image larger than 128 KiB
/// can never reach its upper half.
///
/// The two writes are chosen so the result is the same whether or not MR-BUS-1 is fixed:
/// each written value is a subset of the bits the target address already holds.
#[test]
#[ignore = "MR-1: MR-11-1"]
fn mr_11_1_color_dreams_prg_field_is_four_bits() {
    // 512 KiB: sixteen 32 KiB banks, so bank 12 exists.
    let mut core = MapperRom::new(11).prg(64).chr(16).load();

    // $E100 holds 0x03 (8 KiB bank 3), so 0x03 survives the AND. 32 KiB bank 3.
    core.mapper_write(0xE100, 0x03);
    assert_eq!([12, 13, 14, 15], core.prg_windows());

    // $E100 now holds 0x0F (8 KiB bank 15), so 0x0C survives the AND. 32 KiB bank 12.
    core.mapper_write(0xE100, 0x0C);

    assert_eq!(
        [48, 49, 50, 51],
        core.prg_windows(),
        "bits 2-3 of the latch must reach PRG A17-A18"
    );
}

// ================================================================ mapper 13, CPROM

/// MR-13-1. CPROM carries 16 KiB of CHR-RAM, not the 32 KiB our header default allocates.
///
/// REQ-CORE-100 pins it in the mapper table ("13 | CPROM | CHR-RAM 16 Kio") and Mesen
/// agrees: `GetChrRamSize() override { return 0x4000; }`, `Mappers/Nintendo/CpRom.h:9`.
/// We take the size from the header, which gives `DEFAULT_CHR_RAM_SIZE` = 32 KiB
/// (`src/rom/ines_header.rs:56`, `:154`) because the board never overrides it.
///
/// The upper 16 KiB is unreachable through the windows, so no pattern byte changes; what
/// changes is the RAM the cartridge exposes and the size of the save-state blob.
#[test]
#[ignore = "MR-1: MR-13-1"]
fn mr_13_1_cprom_has_16_kib_of_chr_ram() {
    let core = MapperRom::new(13).prg(4).chr(0).load();

    assert_eq!(
        0x4000,
        core.cartridge().unwrap().memory().chr_ram().len(),
        "REQ-CORE-100: CPROM has 16 KiB of CHR-RAM"
    );
}
