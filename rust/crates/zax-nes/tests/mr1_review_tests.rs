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

// ================================================================ mapper 41, Caltron 6-in-1

// The outer register at $6000-$67FF carries its three fields in the *address*. Mesen
// (`Mappers/Ntdec/Caltron41.h:33-37`):
//
//     _prgBank = addr & 0x07;
//     _chrBank = (_chrBank & 0x03) | ((addr >> 1) & 0x0C);
//     SetMirroringType(addr & 0x20 ? Horizontal : Vertical);
//
// i.e. address bits 0-2 are the 32 KiB PRG bank, bits 3-4 the CHR outer bank and bit 5 the
// mirroring — the `..MC CPPP` layout. We mask the address to five bits and then read PRG
// from bits 0-1 and mirroring from bit 2 (`multicart.rs:261-264`), which is the same layout
// shifted: PRG loses its top bit and mirroring lands on a PRG bit. The CHR outer field
// agrees. No spec amendment covers mapper 41, so REQ-CORE-100's nesdev behaviour governs.

/// MR-41-1. The PRG field is three bits: a Caltron 6-in-1 has six 32 KiB games, so banks
/// 4-7 must be reachable. With `outer & 0x03` the menu can only ever start games 0-3.
#[test]
#[ignore = "MR-1: MR-41-1"]
fn mr_41_1_caltron_prg_field_is_three_bits() {
    // 256 KiB: eight 32 KiB banks.
    let mut core = MapperRom::new(41).prg(32).chr(16).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    core.mapper_write(0x6004, 0);

    assert_eq!(
        [16, 17, 18, 19],
        core.prg_windows(),
        "address bit 2 must reach PRG A17"
    );
}

/// MR-41-2. The mirroring bit is address bit 5, which our five-bit mask discards entirely.
#[test]
#[ignore = "MR-1: MR-41-2"]
fn mr_41_2_caltron_mirroring_comes_from_address_bit_5() {
    let mut core = MapperRom::new(41).prg(32).chr(16).load();

    core.mapper_write(0x6020, 0);

    assert_eq!(zax_nes::NametableMirroring::Horizontal, core.mirroring());
}

/// MR-41-3. The inner CHR register at $8000-$FFFF only latches while the PRG bank is 4-7.
/// Mesen gates it (`Caltron41.h:41-45`, quoting the nesdev note "the Inner CHR Bank Select
/// only can be written while the PRG ROM bank is 4, 5, 6, or 7"); we latch unconditionally
/// (`multicart.rs:311-316`).
#[test]
#[ignore = "MR-1: MR-41-3"]
fn mr_41_3_caltron_inner_chr_is_gated_on_the_prg_bank() {
    let mut core = MapperRom::new(41).prg(32).chr(16).load();

    core.mapper_write(0x6000, 0); // PRG bank 0, so the inner CHR register is closed.
    core.mapper_write(0x8000, 0x03);

    assert_eq!(
        [0, 1, 2, 3, 4, 5, 6, 7],
        core.chr_windows(),
        "the inner CHR write must be ignored while the PRG bank is below 4"
    );
}

// ================================================================ mapper 34, BNROM/NINA-001

/// MR-34-1. On a BNROM board there is no register at $7FFD-$7FFF: those three bytes are
/// plain PRG-RAM.
///
/// Mesen resolves the two mapper-34 boards into different classes at construction
/// (`MapperFactory.cpp:320-326`: CHR-ROM present -> `Nina01`, absent -> `BnRom`).
/// `BnRom` keeps the default $8000-$FFFF register range (`Mappers/Irem/BnRom.h`), so a write
/// to $7FFD reaches PRG-RAM and nothing else. `Nina01` is the board with the $7FFD-$7FFF
/// registers (`Mappers/Unlicensed/Nina01.h:11-12`).
///
/// We use one board for both and decode $7FFD-$7FFF unconditionally
/// (`multicart.rs:80-89`), so on a CHR-RAM mapper-34 image a PRG-RAM write in the last
/// three bytes of $7FFF silently switches the 32 KiB PRG bank.
#[test]
#[ignore = "MR-1: MR-34-1"]
fn mr_34_1_bnrom_has_no_register_at_7ffd() {
    // No CHR-ROM: this is a BNROM, not a NINA-001.
    let mut core = MapperRom::new(34).prg(8).chr(0).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    core.mapper_write(0x7FFD, 0x01);

    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "$7FFD is PRG-RAM on a BNROM board, not the PRG bank register"
    );
    assert_eq!(0x01, core.read_memory(0x7FFD), "the write still reaches RAM");
}

// ================================================================ mapper 30, UNROM 512

/// MR-30-1. UNROM 512 encodes its four mirroring choices in byte 6 bits 3 and 0 together,
/// using the combination iNES otherwise calls illegal. Mesen decodes all four
/// (`Mappers/Homebrew/UnRom512.h:45-50`):
///
///     case 0: Horizontal; case 1: Vertical;
///     case 8: ScreenAOnly + mapper-controlled; case 9: FourScreens
///
/// Our header parser collapses both 8 and 9 to `FourScreen` (`ines_header.rs:170-176`, bit 3
/// alone) and the board then treats any `FourScreen` header as mapper-controlled one-screen
/// (`discrete.rs:405-406`, `:416-422`), so a genuinely four-screen image (byte 6 = $09)
/// loses its four nametables at power-on.
#[test]
#[ignore = "MR-1: MR-30-1"]
fn mr_30_1_unrom512_byte6_09_is_four_screen() {
    let core = MapperRom::new(30).prg(8).chr(0).mirroring(0x09).load();

    assert_eq!(
        zax_nes::NametableMirroring::FourScreen,
        core.mirroring(),
        "byte 6 = $09 selects four-screen, not mapper-controlled one-screen"
    );
}

/// MR-30-2. The UNROM 512 board carries no work RAM: Mesen returns 0 from both
/// `GetWorkRamSize()` and `GetSaveRamSize()` (`UnRom512.h:21-22`), leaving $6000-$7FFF
/// undriven. We give every board the header's 8 KiB default (`mod.rs:52-57`) and answer
/// reads there (`mod.rs:378-387`).
#[test]
#[ignore = "MR-1: MR-30-2"]
fn mr_30_2_unrom512_does_not_drive_6000_7fff() {
    let core = MapperRom::new(30).prg(8).chr(0).load();

    assert_eq!(
        None,
        core.cartridge().unwrap().try_cpu_peek(0x6000),
        "UNROM 512 has no RAM at $6000-$7FFF: the bus stays open"
    );
}

// ================================================================ mapper 1, MMC1

/// MR-1-1. A write with the reset bit set must be processed even when it lands within two
/// CPU cycles of the previous write.
///
/// Mesen (`Mappers/Nintendo/MMC1.h:208-213`):
///
///     //If the reset bit is set, process the write even if another write just occurred
///     //(fixes bug in Shinsenden)
///     if((value & 0x80) || currentCycle - _lastWriteCycle >= 2) { ProcessBitWrite(addr, value); }
///
/// We test the two-cycle filter *before* looking at bit 7 (`mmc1.rs:146-151`), so the reset
/// is swallowed: `control` keeps its PRG mode instead of having bits 2-3 forced.
#[test]
#[ignore = "MR-1: MR-1-1"]
fn mr_1_1_mmc1_reset_bit_bypasses_the_consecutive_write_filter() {
    let mut core = MapperRom::new(1).prg(16).chr(16).load();

    core.mmc1_write(0x8000, 0x00); // 32 KiB PRG mode, screen A.
    core.mmc1_write(0xE000, 0x02); // PRG register 2 -> 32 KiB bank 1.
    assert_eq!([4, 5, 6, 7], core.prg_windows());

    // Two raw writes on the same CPU cycle: the second carries the reset bit.
    core.bus_mut().tick();
    core.bus_mut().tick();
    core.write_memory(0x8000, 0x00);
    core.write_memory(0x8000, 0x80);

    // Reset forces bits 2-3 of $8000: 16 KiB mode, $8000 switchable, last bank at $C000.
    assert_eq!(
        [4, 5, 14, 15],
        core.prg_windows(),
        "the reset bit must take effect despite the two-cycle filter"
    );
}

/// MR-1-2. On a 512 KiB board the PRG A18 line comes from bit 4 of the CHR register that
/// was written *last*, not always from the $A000 one.
///
/// Mesen (`MMC1.h:124`): `extraReg = (_lastChrReg == 0xC000 && _chrMode) ? _chrReg1 : _chrReg0`,
/// then `prgBankSelect = extraReg & 0x10`. In 8 KiB CHR mode this reduces to `_chrReg0`,
/// which is what we always use (`mmc1.rs:37-41`) — so SUROM titles that stay in 8 KiB CHR
/// mode (*Dragon Warrior III* and *IV*) agree. In 4 KiB CHR mode with $C000 written last
/// we select the wrong 256 KiB half.
///
/// The same `extraReg` feeds the SXROM PRG-RAM bank (`MMC1.h:136`), where we also read
/// `chr0` unconditionally (`mmc1.rs:129-131`); one fix covers both.
#[test]
#[ignore = "MR-1: MR-1-2"]
fn mr_1_2_mmc1_suprom_outer_bank_follows_the_last_chr_register() {
    let mut core = MapperRom::new(1).prg(64).chr(16).load(); // 512 KiB.

    core.mmc1_write(0x8000, 0x1F); // 4 KiB CHR, 16 KiB PRG, $8000 switchable.
    core.mmc1_write(0xA000, 0x00); // CHR 0: A18 clear.
    core.mmc1_write(0xC000, 0x10); // CHR 1: A18 set, and written last.
    core.mmc1_write(0xE000, 0x02);

    // Mesen: page0 = 2 | 0x10 = 18, page1 = 0x0F | 0x10 = 31 (16 KiB pages).
    assert_eq!(
        [36, 37, 62, 63],
        core.prg_windows(),
        "bit 4 of the last-written CHR register drives PRG A18"
    );
}

/// MR-1-4. NES 2.0 submapper 5 is the fixed-PRG MMC1 (SEROM, SHROM, SH1ROM): no PRG
/// banking at all. Mesen short-circuits it (`MMC1.h:152-156`, `SelectPrgPage2x(0, 0)`);
/// we have no submapper handling on this board, so the PRG register still banks.
#[test]
#[ignore = "MR-1: MR-1-4"]
fn mr_1_4_mmc1_submapper_5_has_fixed_prg() {
    let mut core = MapperRom::new(1).prg(4).chr(16).submapper(5).load();

    core.mmc1_write(0x8000, 0x0C);
    core.mmc1_write(0xE000, 0x01);

    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "submapper 5 fixes the whole 32 KiB: the PRG register must do nothing"
    );
}
