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

// Test values are small: the narrowing casts cannot lose anything. Board and register names
// (ExRAM, VRC2a, ORing) read as prose here, as in `cartridges/mappers/mod.rs`.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
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

// MR-11-1's test (`mr_11_1_color_dreams_prg_field_is_four_bits`) was withdrawn by lot MR-1b.
// nesdev ("INES Mapper 011", "Color Dreams") prints the latch as `CCCC LLPP`: the PRG field
// is **two** bits, bits 2-3 are "Used for lockout defeat", and the board holds 128 KiB of PRG
// in four 32 KiB banks. Our `0x03` mask is the documented field; Mesen's `value & 0x0F` is
// the oversized-ROM relaxation its own comment describes. See `docs/review/MR-1-mappers.md`,
// MR-1b section.

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
    assert_eq!(
        0x01,
        core.read_memory(0x7FFD),
        "the write still reaches RAM"
    );
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

// MR-1-2's test (`mr_1_2_mmc1_suprom_outer_bank_follows_the_last_chr_register`) was
// withdrawn by lot MR-1b. It asserted Mesen's "the CHR register written last" convention as
// if it were the hardware's; nesdev ("MMC1") says that in 4 KiB CHR mode the `P` and `S` bits
// of *both* CHR registers must hold the same value, "or the PRG-ROM and/or RAM will be
// bankswitched/enabled as the PPU renders" — the bank follows whichever register PPU A12
// selects at that instant, which is neither side's model, and which no SUROM/SXROM title
// creates (they are 8 KiB CHR-RAM boards and run in 8 KiB CHR mode, where our `chr0` and
// Mesen's expression coincide). See `docs/review/MR-1-mappers.md`, MR-1b section.

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

// ================================================================ mapper 4, MMC3 / MMC6

/// MR-4-1. The MMC6 $A001 read-enable and write-enable bits are the other way round.
///
/// Mesen (`Mappers/Nintendo/MMC3.h:142-144`):
///
///     firstBankAccess = (RegA001 & 0x10 ? Write : 0) | (RegA001 & 0x20 ? Read : 0);
///     lastBankAccess  = (RegA001 & 0x40 ? Write : 0) | (RegA001 & 0x80 ? Read : 0);
///
/// so for each 512-byte half the *lower* bit of the pair enables writing and the upper one
/// enables reading. Ours reads the pair the other way (`mmc3.rs:51-57`): bit 4 enables
/// reading the low half and bit 5 enables writing it, and likewise 6/7 for the high half.
///
/// Observed by storing a byte with both bits set, then clearing the one that is really the
/// write enable: the byte must still read back.
#[test]
#[ignore = "MR-1: MR-4-1"]
fn mr_4_1_mmc6_a001_read_and_write_bits_are_not_swapped() {
    let mut core = MapperRom::new(4).prg(8).chr(16).submapper(1).load();

    core.mapper_write(0x8000, 0x20); // MMC6 work RAM enabled (bit 5 of $8000).
    core.mapper_write(0xA001, 0x30); // Low half: read and write enabled either way round.
    core.write_memory(0x7000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x7000));

    // Bit 5 alone: Mesen reads this as "read enabled, write disabled".
    core.mapper_write(0xA001, 0x20);

    assert_eq!(
        0x5A,
        core.read_memory(0x7000),
        "$A001 bit 5 enables *reading* the low half, not writing it"
    );
}

// MR-4-2's test (`mr_4_2_mmc3_a12_needs_a_fall_before_the_first_rise`) was withdrawn by lot
// MR-1b. It asserted Mesen's `_a12LowClock > 0` guard as if it were the hardware's: nesdev
// ("MMC3") says the counter is "triggered on a rising edge after the line has remained low
// for three falling edges of M2", a condition on how long A12 *has been* low, which at
// power-on (PPU `v = 0`) is already satisfied. The first rise therefore does clock the
// counter, as our `A12Watcher` has it, and it is Mesen that misses that clock. See
// `docs/review/MR-1-mappers.md`, MR-1b section.

/// MR-4-7. On an MMC6 the chip holds $A001 at zero for as long as PRG-RAM is disabled
/// through $8000 bit 5, so a write to $A001 made while the RAM is off must be lost.
///
/// nesdev ("MMC6"): "When PRG RAM is disabled via $8000, the mapper continuously sets $A001
/// to $00, and so all writes to $A001 are ignored." We store the value unconditionally
/// (`mmc3.rs:259`), so it takes effect as soon as the game sets bit 5 of $8000.
#[test]
#[ignore = "MR-1b: MR-4-7"]
fn mr_4_7_mmc6_a001_is_held_at_zero_while_ram_is_disabled() {
    let mut core = MapperRom::new(4).prg(8).chr(16).submapper(1).load();

    // RAM still disabled ($8000 bit 5 clear): this write must be ignored by the chip.
    core.mapper_write(0xA001, 0xF0);
    core.mapper_write(0x8000, 0x20); // Now enable the RAM.

    // $A001 is still 0, so neither half is writable or readable yet.
    core.write_memory(0x7000, 0x5A);
    assert_eq!(
        0x00,
        core.read_memory(0x7000),
        "an $A001 written while the RAM was disabled must not grant access"
    );
}

// ================================================================ mapper 64, RAMBO-1

// MR-1's `rambo_rise` helper went with the two tests below that used it.

// MR-64-1's test (`mr_64_1_rambo1_counter_always_decrements`) was withdrawn by lot MR-1b. It
// asserted Mesen's `latch + 2` reload with an unconditional decrement. Both descriptions of
// the counter on nesdev ("RAMBO-1") put the decrement in its own `else` branch — a clock that
// reloads does not also decrement, which is our structure — and under the page's cycle-level
// pseudo-code a latch of L fires on the (L+1)-th clock after a $C001 write, as ours does,
// where Mesen's fires on the (L+2)-th. The only clause that would favour Mesen is the
// "ORed with 1" of the older description, of which the page says it "remains undiscovered"
// how it works. See `docs/review/MR-1-mappers.md`, MR-1b section.

// MR-64-2's test (`mr_64_2_rambo1_2kib_chr_window_keeps_bit_0`) was withdrawn by lot MR-1b,
// and this is the one finding of MR-1 whose proposed fix would have broken a board that is
// right today. nesdev ("RAMBO-1"): "With 2KiB banks (where K=0), only even-numbered CHR banks
// can be selected. The lowest bit in the register is ignored and instead passes PPU A10
// directly through to CHR A10." That is our `map_chr_2k(slot, b(0) >> 1)`; Mesen's
// `_registers[0] + 1` is the error.

/// MR-64-3. RAMBO-1's A12 filter is much longer than the MMC3's — and longer than Mesen's
/// figure for it too.
///
/// nesdev ("RAMBO-1"), the cycle-level pseudo-code, on each M2 falling edge: while PA12 = 0,
/// "If 4-bit counter < 16: 4-bit counter++"; on PA12 = 1, "If 4-bit counter = 16: Clock the
/// 8-bit counter." So A12 must have been low across **sixteen M2 falling edges**, about 48
/// PPU dots. We require ten dots (`a12_watcher.rs:27`) and Mesen 31 (`Rambo1.h:166`,
/// `UpdateVramAddress<30>`), so we count rises this board ignores.
///
/// Twenty dots of low is over our threshold and under both of the others, so no number of
/// such pulses may ever clock the counter.
#[test]
#[ignore = "MR-1b: MR-64-3"]
fn mr_64_3_rambo1_a12_filter_is_sixteen_m2_cycles() {
    let mut core = MapperRom::new(64).prg(8).chr(16).load();
    core.mapper_write(0xC000, 0x01);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);

    let cart = core.cartridge_mut().unwrap();
    for i in 0..10 {
        cart.on_ppu_address_bus(0x0000, 1000 + i * 100);
        cart.on_ppu_address_bus(0x1000, 1020 + i * 100);
    }

    assert!(
        !cart.irq_asserted(),
        "a 20-dot low is too short for RAMBO-1's filter: the counter never moves"
    );
}

/// MR-64-7. The PRG mode bit **swaps** R6 with R15 and leaves R7 at $A000; it does not
/// rotate the three switchable windows.
///
/// nesdev ("RAMBO-1"), PRG banking: with P ($8000 & $40) = $00 the windows are R6, R7, RF;
/// with P = $40 they are RF, R7, R6. The correction comes from the hardware tests of the
/// "RAMBO-1 Mapper Investigation" thread ("when P=1, $A000=R7, $C000=R6"), so Mesen carries
/// the old, wrong table too (`Rambo1.h:97-105`) and MR-1 found our rotation "equivalent"
/// against it. Ours is `rambo1.rs:42-50`.
#[test]
#[ignore = "MR-1b: MR-64-7"]
fn mr_64_7_rambo1_prg_mode_swaps_r6_and_r15() {
    let mut core = MapperRom::new(64).prg(8).chr(16).load();

    core.mapper_write(0x8000, 6); // Register 6.
    core.mapper_write(0x8001, 2);
    core.mapper_write(0x8000, 7); // Register 7.
    core.mapper_write(0x8001, 3);
    core.mapper_write(0x8000, 15); // Register 15.
    core.mapper_write(0x8001, 4);
    assert_eq!([2, 3, 4, 7], core.prg_windows(), "mode 0: R6, R7, RF, fixed");

    core.mapper_write(0x8000, 0x40); // PRG mode on, register select back to 0.

    assert_eq!(
        [4, 3, 2, 7],
        core.prg_windows(),
        "mode 1 swaps R6 and RF: RF, R7, R6, fixed"
    );
}

// ================================================================ mapper 5, MMC5

/// MR-5-1. With 8x8 sprites the PPU uses the "A" CHR register set for everything; the
/// last-written-set rule applies only while the PPU is *not* rendering.
///
/// Mesen (`Mappers/Nintendo/MMC5.h:192`):
///
///     bool chrA = !largeSprites
///              || (_splitTileNumber >= 32 && _splitTileNumber < 40)
///              || (!_ppuInFrame && _lastChrReg <= 0x5127);
///
/// The first clause makes 8x8 mode unconditionally set A. We use the last-written set as
/// *the* 8x8 rule instead (`mmc5.rs:163-167`):
///
///     let use_b = if self.sprite_size16 { fetch == ChrFetch::Background }
///                 else { self.last_set_was_b };
///
/// so a game in 8x8 mode that last wrote $5128-$512B renders from set B throughout.
#[test]
#[ignore = "MR-1: MR-5-1"]
fn mr_5_1_mmc5_8x8_sprites_always_use_chr_set_a() {
    let mut core = MapperRom::new(5).prg(8).chr(64).load();
    // Sprite size is 8x8 at power-on (`sprite_size16` false), the case under test.

    core.mapper_write(0x5127, 0x03); // Set A, 8 KiB bank 3.
    core.mapper_write(0x512B, 0x05); // Set B, 8 KiB bank 5 — written last.

    assert_eq!(
        [24, 25, 26, 27, 28, 29, 30, 31],
        core.chr_windows(),
        "8x8 sprites render from set A (bank 3), not from the last-written set B"
    );
}

/// MR-5-2. In ExRAM modes 0 and 1 the CPU can only store a byte while the PPU is rendering;
/// a write outside rendering must not take effect.
///
/// nesdev ("MMC5"), the ExRAM table for modes %00 and %01: "In these modes writes are only
/// allowed when the PPU is rendering. If writes are attempted during V-blank, they may be
/// ignored or cause a corruption at that memory address." Mesen takes the second reading and
/// writes 0 (`MMC5.h:420-426`); ignoring the write is the cheaper of the two and equally
/// conforming. What the source rules out is ours: storing the value, which we do in every
/// mode but 3 (`mmc5.rs:421-427`) with no rendering test.
///
/// The byte is observable through the nametable path, since modes 0 and 1 are the ones that
/// show ExRAM to the PPU, and the assertion below holds under either fix.
#[test]
#[ignore = "MR-1b: MR-5-2"]
fn mr_5_2_mmc5_exram_write_outside_rendering_does_not_store_the_value() {
    let mut core = MapperRom::new(5).prg(8).chr(16).load();

    core.mapper_write(0x5104, 0x00); // ExRAM mode 0.
    core.mapper_write(0x5105, 0x02); // Nametable 0 reads ExRAM.
    core.write_memory(0x5C00, 0x5A); // The PPU is not rendering.

    assert_eq!(
        0,
        core.cartridge().unwrap().read_nametable(0x2000),
        "a mode-0 ExRAM write outside rendering must not store the value"
    );
}

/// MR-5-3. The MMC5's scanline counter starts at zero on the first line it detects, with no
/// compare on that line, so a target of 1 fires on the **second** visible line.
///
/// nesdev ("MMC5"), scanline detection: "if the 'in-frame' flag (register $5204) was clear,
/// it becomes set, and the internal 8-bit scanline counter is reset to zero; but if it was
/// already set, the scanline counter is incremented, then compared against the value written
/// to $5203." We set `scanline_counter = scanline + 1` from the PPU's own line number and
/// compare at once (`mmc5.rs:506-509`), so every split is one scanline high.
#[test]
#[ignore = "MR-1b: MR-5-3"]
fn mr_5_3_mmc5_scanline_target_one_fires_on_the_second_line() {
    let mut core = MapperRom::new(5).prg(8).chr(16).load();

    core.mapper_write(0x5203, 0x01); // Target scanline 1.
    core.mapper_write(0x5204, 0x80); // Scanline IRQ enabled.

    let cart = core.cartridge_mut().unwrap();
    cart.on_ppu_scanline(0, true);
    assert!(
        !cart.irq_asserted(),
        "the first detected line only sets in-frame and zeroes the counter"
    );

    cart.on_ppu_scanline(1, true);
    assert!(
        cart.irq_asserted(),
        "the second line increments the counter to 1 and matches the target"
    );
}

// ================================================================ mappers 21-27, VRC2/VRC4

// Our `register_index` (`vrc2_vrc4.rs:52-59`) always uses the **OR'd heuristic** wiring
// Mesen applies only when the NES 2.0 submapper is absent (`VRC2_4.h:208-247`,
// `_useHeuristics`). When the submapper is known — and always for mappers 22 and 27 —
// Mesen uses one exact wiring instead (`VRC2_4.h:249-305`). For mapper 22 that is
// `A0 = (addr >> 1) & 1`, `A1 = addr & 1`, with no OR term at all.
//
// Mapper 27's widened CHR field and the VRC2 mirroring mask are *not* findings:
// AMD-021 R2-004 records both as accepted divergences ("Divergences acceptées (D)").

/// MR-21-1. Mapper 22 is VRC2a, whose register lines are known exactly: ORing in the
/// mapper-25 candidate bits makes us decode a different register for the same address.
///
/// At $B004 the exact VRC2a wiring gives A1 = 0, A0 = 0, i.e. CHR register 0; our OR'd
/// `A1 = (a | (a >> 2)) & 1` picks up address bit 2 and yields register 2, which lands on
/// CHR slot 1.
#[test]
#[ignore = "MR-1: MR-21-1"]
fn mr_21_1_vrc2a_uses_the_exact_register_wiring() {
    let mut core = MapperRom::new(22).prg(8).chr(16).load();

    core.mapper_write(0xB004, 0x05);

    // VRC2a ignores the low bit of its CHR registers, so 5 selects 1 KiB bank 2.
    assert_eq!(
        [2, 0, 0, 0, 0, 0, 0, 0],
        core.chr_windows(),
        "$B004 is CHR register 0 on VRC2a, not register 2"
    );
}

/// MR-21-2. The VRC2 has no PRG swap mode: all of $9000-$9003 is the mirroring register
/// (`VRC2_4.h:165-166`, the `_variant <= VRCVariant::VRC2c` arm covering `0x9000-0x9003`).
/// Only VRC4 splits $9002-$9003 off as the PRG mode (`VRC2_4.h:177-178`).
///
/// We split it on every variant (`vrc2_vrc4.rs:191-200`, `if index < 2`), so on mapper 22 a
/// write to $9003 swaps the PRG layout instead of setting mirroring.
#[test]
#[ignore = "MR-1: MR-21-2"]
fn mr_21_2_vrc2a_has_no_prg_swap_mode() {
    let mut core = MapperRom::new(22).prg(8).chr(16).load();
    assert_eq!([0, 1, 6, 7], core.prg_windows());

    core.mapper_write(0x9003, 0x02);

    assert_eq!(
        [0, 1, 6, 7],
        core.prg_windows(),
        "$9003 is the mirroring register on a VRC2: the PRG layout must not move"
    );
}

// ================================================================ mapper 32, Irem G-101

/// MR-32-1. NES 2.0 submapper 1 is *Major League*, whose board ties CIRAM A10 high: the
/// mirroring is fixed one-screen and the PRG mode is fixed at 0.
///
/// Mesen handles both halves (`Mappers/Irem/IremG101.h:23-27` and `:60-62`):
///
///     if(_romInfo.SubMapperID == 1) { SetMirroringType(MirroringType::ScreenAOnly); }
///     ...
///     _prgMode = (value & 0x02) >> 1;
///     if(_romInfo.SubMapperID == 1) { _prgMode = 0; }
///
/// We have no submapper handling on this board (`iremg101.rs`), so *Major League* takes its
/// mirroring from the header and then from every $9000 write.
#[test]
#[ignore = "MR-1: MR-32-1"]
fn mr_32_1_irem_g101_submapper_1_is_fixed_one_screen() {
    let core = MapperRom::new(32).prg(8).chr(16).submapper(1).load();

    assert_eq!(
        zax_nes::NametableMirroring::SingleScreenA,
        core.mirroring(),
        "submapper 1 ties CIRAM A10 high: one-screen whatever the header says"
    );
}

// ================================================================ mapper 16, Bandai FCG

/// Advances the CPU by `cycles` cycles without touching the cartridge bus.
fn tick(core: &mut zax_nes::NesCore, cycles: usize) {
    for _ in 0..cycles {
        core.bus_mut().tick();
    }
}

/// MR-16-1. On the FCG-1/2 — NES 2.0 submapper 4 — $x00B and $x00C write the IRQ
/// **counter** directly; there is no latch. Only the LZ93D50 (submapper 5) has the latch
/// that $x00A copies across.
///
/// Mesen branches on it in three places (`Mappers/Bandai/BandaiFcg.h`, the
/// `_romInfo.SubMapperID != 4` guards on cases `0x0A`, `0x0B` and `0x0C`), quoting the wiki:
/// "On the FCG-1/2 (Submapper 4), writing to these two registers directly modifies the
/// counter itself; all such games therefore disable counting before changing the counter
/// value."
///
/// We always write the latch (`bandai_fcg.rs:119-121`). The two models agree as long as the
/// game sets the value *before* enabling — which is why this has not shown up — but diverge
/// the moment a value is written while the counter is running.
#[test]
#[ignore = "MR-1: MR-16-1"]
fn mr_16_1_bandai_submapper_4_writes_the_counter_directly() {
    let mut core = MapperRom::new(16).prg(8).chr(16).submapper(4).load();

    core.mapper_write(0x800B, 0xFF); // Counter low byte.
    core.mapper_write(0x800C, 0xFF); // Counter high byte: 0xFFFF either way.
    core.mapper_write(0x800A, 0x01); // Enable counting.

    // Clear the high byte while the counter runs: on an FCG-1/2 the counter itself drops to
    // its low byte (at most 0xFF), so it must expire well inside 400 cycles.
    core.mapper_write(0x800C, 0x00);
    tick(&mut core, 400);

    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "$x00C must write the counter, not a latch the counter never reads"
    );
}

// ================================================================ mapper 18, Jaleco SS88006

/// MR-18-1. The SS88006 IRQ fires on the cycle whose decrement *produces* zero, not on the
/// cycle that observes a zero already there — our counter is one CPU cycle late.
///
/// Mesen (`Mappers/Jaleco/JalecoSs88006.h:91-100`):
///
///     uint16_t counter = _irqCounter & _irqMask[_irqCounterSize];
///     if(--counter == 0) { SetIrqSource(...); }
///
/// We test the counter before decrementing and fire when it is already zero
/// (`jaleco_ss88006.rs:171-179`), which adds a cycle to every period. With a reload value of
/// 3 the third cycle fires; ours needs a fourth.
#[test]
#[ignore = "MR-1: MR-18-1"]
fn mr_18_1_jaleco_ss88006_irq_fires_one_cycle_earlier() {
    let mut core = MapperRom::new(18).prg(8).chr(16).load();

    core.mapper_write(0xE000, 0x03); // IRQ latch nibble 0 = 3.
    core.mapper_write(0xF000, 0x00); // Reload the counter from the latch.
    core.mapper_write(0xF001, 0x01); // Enable, 16-bit counter.

    tick(&mut core, 3);

    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "a reload value of 3 expires on the third cycle, not the fourth"
    );
}
