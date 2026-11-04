//! Findings of lot MR-1, the adversarial review of our mapper boards against Mesen 2
//! (`docs/review/MR-1-mappers.md`).
//!
//! Every test here reproduces one finding classified **our bug**. A test still carrying
//! `#[ignore = "MR-1: <finding id>"]` or `#[ignore = "MR-1b: <finding id>"]` fails on the
//! current code and waits for the lot that fixes its finding; run those with
//! `cargo test --release -p zax-nes --test mr1_review_tests -- --ignored`. A test without the
//! attribute belongs to a finding already fixed — lot MF-1 un-ignores one as it lands —
//! and runs in the normal suite as the regression guard of that fix.
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

/// Whether the cartridge is pulling the /IRQ line.
fn irq(core: &zax_nes::NesCore) -> bool {
    core.cartridge().unwrap().irq_asserted()
}

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

/// MR-BUS-1 on mapper 11, where the conflict is unconditional — "All commercially-released
/// games are subject to bus conflicts (submapper 2)" (nesdev, "INES Mapper 011"). At power-on
/// $8000-$FFFF shows 32 KiB bank 0, whose first 8 KiB bank the fixture fills with 0x00, so
/// every bit of the written value is pulled low and the latch keeps its reset value.
///
/// **Fixed in MF-2.** The CHR assertion MR-1 wrote was `[0; 8]`, which no CHR bank can produce:
/// `chr_windows` reports the eight 1 KiB banks a window shows, so 8 KiB bank 0 is `0..=7` and
/// not eight zeros. The finding's behaviour — the latch stays at CHR bank 0 instead of moving to
/// the bank 1 that `0x13` asks for — is what the literal now says. Everything else is as MR-1
/// wrote it.
#[test]
fn mr_bus_1_color_dreams_write_is_anded_with_the_rom_byte() {
    let mut core = MapperRom::new(11).prg(8).chr(16).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // $8100 holds 0x00 (PRG bank 0), so value &= 0x00 and the latch stays 0.
    core.mapper_write(0x8100, 0x13);

    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "the written 0x13 must be pulled to 0x00 by the ROM byte at $8100"
    );
    assert_eq!(
        [0, 1, 2, 3, 4, 5, 6, 7],
        core.chr_windows(),
        "CHR stays on 8 KiB bank 0; a latched 0x13 would have selected bank 1"
    );
}

/// MR-BUS-1 on mapper 2, where the conflict is gated on NES 2.0 submapper 2 — "Bus conflicts
/// occur, producing the bitwise AND of the written value and the value in ROM" (nesdev, "NES 2.0
/// submappers"). **Fixed in MF-2**, unchanged from MR-1 but for the `#[ignore]`.
#[test]
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

/// MR-66-1 — the `GxROM` board table gives mapper 66 bus conflicts, which Mesen does not
/// implement and which MR-1 therefore recorded as "no bus conflicts on either". Unconditional,
/// like Color Dreams'. **New in MF-2.**
#[test]
fn mr_66_1_gxrom_write_is_anded_with_the_rom_byte() {
    let mut core = MapperRom::new(66).prg(16).chr(32).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    // The register is `xxPP xxCC`: $11 asks for 32 KiB PRG bank 1 and 8 KiB CHR bank 1. $8100
    // sits in PRG bank 0, whose bytes are all 0x00, so the latch stays where it was.
    core.mapper_write(0x8100, 0x11);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], core.chr_windows());

    // From a window that is *not* zero-filled the AND lets bits through: $C100 is in 8 KiB bank
    // 2, filled with 0x02, so a written $11 latches $11 & $02 = $00 — still zero. $03 & $02 = 2,
    // which selects CHR bank 2 and leaves PRG where it is.
    core.mapper_write(0xC100, 0x03);
    assert_eq!([0, 1, 2, 3], core.prg_windows());
    assert_eq!([16, 17, 18, 19, 20, 21, 22, 23], core.chr_windows());
}

/// MR-34-2, the bus-conflict half — "the effective value is the value being written
/// bitwise-AND'd with the PRG-ROM content" (nesdev, "INES Mapper 034"), for **BNROM** only. The
/// work-RAM half was fixed in MF-1. **New in MF-2.**
#[test]
fn mr_34_2_bnrom_write_is_anded_with_the_rom_byte_and_nina_is_not() {
    // 8 KiB of CHR-ROM resolves to BNROM (MR-34-1), which has the conflicts.
    let mut bnrom = MapperRom::new(34).prg(16).chr(8).load();
    assert_eq!([0, 1, 2, 3], bnrom.prg_windows());
    bnrom.mapper_write(0x8100, 0x01);
    assert_eq!(
        [0, 1, 2, 3],
        bnrom.prg_windows(),
        "PRG bank 0 is zero-filled, so the BNROM latch cannot leave bank 0"
    );

    // A NINA-001 (submapper 1) has no conflicts, so the same write takes.
    let mut nina = MapperRom::new(34).prg(16).chr(8).submapper(1).load();
    nina.mapper_write(0x7FFD, 0x01);
    assert_eq!([4, 5, 6, 7], nina.prg_windows());
}

/// MR-BUS-1 on mapper 30 — the one board where the gate is not the submapper alone: "The Battery
/// bit must be specified if the game relies on bus conflicts not being present" for submapper 0,
/// and submapper 2 has them outright (nesdev, "UNROM 512"). **New in MF-2.**
#[test]
fn mr_bus_1_unrom512_conflicts_follow_the_submapper_and_the_battery_bit() {
    // Submapper 0 without a battery: conflicts, so the zero-filled bank 0 pulls the write down.
    let mut plain = MapperRom::new(30).prg(16).chr(0).load();
    assert_eq!([0, 1, 14, 15], plain.prg_windows());
    plain.mapper_write(0x8100, 0x01);
    assert_eq!([0, 1, 14, 15], plain.prg_windows());

    // Submapper 0 *with* a battery is how such a dump says "no conflicts" — and with that it is
    // self-flashable, so Q-117 puts its latch at $C000-$FFFF and $8100 is the flash window.
    let mut battery = MapperRom::new(30).prg(16).chr(0).battery().load();
    battery.mapper_write(0xC100, 0x01);
    assert_eq!([2, 3, 14, 15], battery.prg_windows());

    // Submapper 2 has them whatever the battery bit says.
    let mut declared = MapperRom::new(30)
        .prg(16)
        .chr(0)
        .battery()
        .submapper(2)
        .load();
    declared.mapper_write(0x8100, 0x01);
    assert_eq!([0, 1, 14, 15], declared.prg_windows());
}

/// MR-BUS-1 — and the boards that have **no** conflicts keep latching what the CPU wrote, which
/// is the other half of a gate decided at load. **New in MF-2.**
#[test]
fn mr_bus_1_a_board_without_conflicts_latches_the_written_value() {
    // Mapper 3's and 7's conflicts are gated on submapper 2, like mapper 2's.
    let mut cnrom = MapperRom::new(3).prg(8).chr(32).load();
    cnrom.mapper_write(0x8100, 0x03);
    assert_eq!([24, 25, 26, 27, 28, 29, 30, 31], cnrom.chr_windows());

    let mut declared = MapperRom::new(3).prg(8).chr(32).submapper(2).load();
    declared.mapper_write(0x8100, 0x03);
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7], declared.chr_windows());

    // And a board MR-BUS-1 does not list is untouched: the MMC1 takes its five writes.
    let mut mmc1 = MapperRom::new(1).prg(16).chr(32).load();
    mmc1.mmc1_write(0xE000, 0x05);
    assert_eq!([10, 11, 14, 15], mmc1.prg_windows());
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
fn mr_13_1_cprom_has_16_kib_of_chr_ram() {
    let core = MapperRom::new(13).prg(4).chr(0).load();

    assert_eq!(
        0x4000,
        core.cartridge().unwrap().memory().chr_ram().len(),
        "REQ-CORE-100: CPROM has 16 KiB of CHR-RAM"
    );
}

/// MR-13-2. nesdev ("CPROM") gives the board's nametable arrangement as "Vertical", with no
/// solder pad among its characteristics — wiring, not a header default — so a mis-headered
/// *Videomation* dump must still come up vertical. Mesen does the same
/// (`SetMirroringType(MirroringType::Vertical)` in `InitMapper`); we honoured the header.
#[test]
fn mr_13_2_cprom_mirroring_is_wired_vertical() {
    // Byte 6 bit 0 set: a header asking for vertical the other way round, and bit 3 clear.
    let core = MapperRom::new(13).prg(4).chr(0).mirroring(0x00).load();
    assert_eq!(zax_nes::NametableMirroring::Vertical, core.mirroring());

    let core = MapperRom::new(13).prg(4).chr(0).mirroring(0x01).load();
    assert_eq!(
        zax_nes::NametableMirroring::Vertical,
        core.mirroring(),
        "the arrangement is wired: the header cannot ask for horizontal"
    );
}

// ================================================================ mapper 15, Contra Function 16

/// MR-15-1. nesdev ("INES Mapper 015"): "CHR-RAM is write-protected in PRG banking modes 0 and
/// 3, and write-enabled in modes 1 and 2." AMD-021 R2-002 is silent on the point rather than
/// contradicting it, so REQ-CORE-100's documented behaviour applies and the absence is ours.
/// How the mode itself is decoded is Q-110's question and is untouched.
#[test]
fn mr_15_1_mapper_15_write_protects_chr_ram_in_modes_0_and_3() {
    let mut core = MapperRom::new(15).prg(16).chr(0).load();

    // Mode 1 ($8001) is write-enabled.
    core.mapper_write(0x8001, 0x00);
    core.cartridge_mut().unwrap().ppu_write(0x0000, 0x5A);
    assert_eq!(0x5A, core.chr_bank(0x0000), "mode 1 is write-enabled");

    for mode in [0u16, 3] {
        core.mapper_write(0x8000 | mode, 0x00);
        core.cartridge_mut().unwrap().ppu_write(0x0000, 0xA5);
        assert_eq!(
            0x5A,
            core.chr_bank(0x0000),
            "mode {mode} write-protects CHR-RAM"
        );
    }

    // And mode 2 is write-enabled again.
    core.mapper_write(0x8002, 0x00);
    core.cartridge_mut().unwrap().ppu_write(0x0000, 0xA5);
    assert_eq!(0xA5, core.chr_bank(0x0000), "mode 2 is write-enabled");
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
fn mr_34_1_bnrom_has_no_register_at_7ffd() {
    // No CHR-ROM: this is a BNROM, not a NINA-001.
    let mut core = MapperRom::new(34).prg(8).chr(0).load();
    assert_eq!([0, 1, 2, 3], core.prg_windows());

    core.mapper_write(0x7FFD, 0x01);

    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "a BNROM board has no register at $7FFD"
    );
    // MR-1's test asserted the write reached PRG-RAM here. MR-34-2, found in lot MR-1b, says
    // the board has no work RAM at all — "8 KiB of unbanked PRG-RAM" is the NINA boards' — so
    // the window is open bus and the write lands nowhere.
    assert_eq!(
        None,
        core.cartridge().unwrap().try_cpu_peek(0x7FFD),
        "a BNROM has no work RAM: $6000-$7FFF is open bus"
    );
}

/// MR-34-1's other half: the NINA-001 keeps its three registers, and the board is resolved
/// from the submapper when the dump declares one, from the CHR-ROM size otherwise — "BNROM
/// when the CHR-ROM size is 0-8 KiB, and NINA-001/NINA-002 when the CHR-ROM size is above
/// 8 KiB".
#[test]
fn mr_34_1_the_board_comes_from_the_submapper_then_from_the_chr_size() {
    // 8 KiB of CHR-ROM with no submapper is a BNROM, where MR-1's `is_empty()` test said NINA.
    let mut core = MapperRom::new(34).prg(8).chr(8).load();
    core.mapper_write(0x7FFD, 0x01);
    assert_eq!(
        [0, 1, 2, 3],
        core.prg_windows(),
        "8 KiB of CHR-ROM is BNROM"
    );

    // The same image with submapper 1 is a NINA-001, registers and work RAM included.
    let mut core = MapperRom::new(34).prg(8).chr(8).submapper(1).load();
    core.mapper_write(0x7FFD, 0x01);
    assert_eq!([4, 5, 6, 7], core.prg_windows(), "submapper 1 is NINA-001");
    assert_eq!(Some(0x01), core.cartridge().unwrap().try_cpu_peek(0x7FFD));

    // And submapper 2 is a BNROM however large its CHR-ROM.
    let mut core = MapperRom::new(34).prg(8).chr(32).submapper(2).load();
    core.mapper_write(0x7FFD, 0x01);
    assert_eq!([0, 1, 2, 3], core.prg_windows(), "submapper 2 is BNROM");
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
fn mr_30_2_unrom512_does_not_drive_6000_7fff() {
    let core = MapperRom::new(30).prg(8).chr(0).load();

    assert_eq!(
        None,
        core.cartridge().unwrap().try_cpu_peek(0x6000),
        "UNROM 512 has no RAM at $6000-$7FFF: the bus stays open"
    );
}

/// MR-30-3. NES 2.0 submapper 3 is "No bus conflicts, mapper-controlled horizontal/vertical
/// nametable arrangement": bit 7 of the register picks H or V, not screen A or B.
#[test]
fn mr_30_3_unrom512_submapper_3_switches_horizontal_and_vertical() {
    // Submapper 3 has no bus conflicts, so it is self-flashable and its latch is at
    // $C000-$FFFF (Q-117).
    let mut core = MapperRom::new(30).prg(8).chr(0).submapper(3).load();
    assert_eq!(zax_nes::NametableMirroring::Vertical, core.mirroring());

    core.mapper_write(0xC000, 0x80);
    assert_eq!(zax_nes::NametableMirroring::Horizontal, core.mirroring());

    core.mapper_write(0xC000, 0x00);
    assert_eq!(zax_nes::NametableMirroring::Vertical, core.mirroring());
}

/// And byte 6 `%....1..0` still selects the switchable one-screen arrangement, which is what
/// our `FourScreen` header used to mean on every mapper-30 image (MR-30-1's other half).
///
/// The battery bit is set because MF-2's MR-BUS-1 gave this board conflicts: on a submapper-0
/// UNROM 512 "the Battery bit must be specified if the game relies on bus conflicts not being
/// present" (nesdev, "UNROM 512"), and without it the zero-filled PRG bank 0 at $8000 would pull
/// the written $80 down to $00. Byte 6 stays `%....1.1.0` on bits 3 and 0, so the arrangement
/// this test is about is unchanged; what the bit buys is a register write that reaches the
/// register. It also makes the board self-flashable, so Q-117 puts its latch at $C000-$FFFF.
#[test]
fn mr_30_1_unrom512_byte6_08_is_switchable_one_screen() {
    let mut core = MapperRom::new(30)
        .prg(8)
        .chr(0)
        .mirroring(0x08)
        .battery()
        .load();
    assert_eq!(zax_nes::NametableMirroring::SingleScreenA, core.mirroring());

    core.mapper_write(0xC000, 0x80);
    assert_eq!(zax_nes::NametableMirroring::SingleScreenB, core.mirroring());
}

/// A self-flashable UNROM 512 image: submapper 1, "No bus conflicts", which is what the
/// self-flashing procedure needs. 64 KiB of PRG — four 16 KiB banks — and 8 KiB PRG bank 0 is
/// filled with $FF so that a programmed byte (flash can only clear bits) is observable without
/// erasing first.
fn flashable_unrom512() -> zax_nes::NesCore {
    MapperRom::new(30)
        .prg(8)
        .chr(0)
        .submapper(1)
        .prg_bank_all_ones(0)
        .load()
}

/// nesdev ("UNROM 512"), erase a 4 KiB sector, verbatim: `$C000:$01, $9555:$AA`;
/// `$C000:$00, $AAAA:$55`; `$C000:$01, $9555:$80`; `$C000:$01, $9555:$AA`;
/// `$C000:$00, $AAAA:$55`; `$C000:BANK, ADDR:$30`.
fn erase_sector(core: &mut zax_nes::NesCore, bank: u8, target: u16) {
    for (unlock_bank, address, value) in [
        (0x01u8, 0x9555u16, 0xAAu8),
        (0x00, 0xAAAA, 0x55),
        (0x01, 0x9555, 0x80),
        (0x01, 0x9555, 0xAA),
        (0x00, 0xAAAA, 0x55),
    ] {
        core.mapper_write(0xC000, unlock_bank);
        core.mapper_write(address, value);
    }

    core.mapper_write(0xC000, bank);
    core.mapper_write(target, 0x30);
}

/// nesdev ("UNROM 512"), program one byte, verbatim: `$C000:$01, $9555:$AA`;
/// `$C000:$00, $AAAA:$55`; `$C000:$01, $9555:$A0`; `$C000:BANK, ADDR:DATA`.
fn program_byte(core: &mut zax_nes::NesCore, bank: u8, target: u16, data: u8) {
    for (unlock_bank, address, value) in [
        (0x01u8, 0x9555u16, 0xAAu8),
        (0x00, 0xAAAA, 0x55),
        (0x01, 0x9555, 0xA0),
    ] {
        core.mapper_write(0xC000, unlock_bank);
        core.mapper_write(address, value);
    }

    core.mapper_write(0xC000, bank);
    core.mapper_write(target, data);
}

/// MR-30-4 and **Q-117 as R0 answered it** — the board carries flash, not mask ROM, and a game
/// can program it, in the bank of its choosing.
///
/// On a self-flashable configuration the bank latch is at $C000-$FFFF only and $8000-$BFFF is
/// the flash write window, so the page's sequences can put a bank write in front of every
/// command byte without disturbing the chip. The chip then sees `bank × $4000 +
/// (address & $3FFF)`: the unlocks land on chip $5555 (bank 1, $9555) and chip $2AAA (bank 0,
/// $AAAA), the SST39SF040's own command addresses, and the last write of each sequence reaches
/// **the bank the driver chose**. MF-2's reading had every byte-write fall in bank 0 and every
/// erase in bank 21; this is the one the page's own sequences imply.
#[test]
fn q_117_unrom512_programs_a_byte_of_its_own_flash() {
    let mut core = flashable_unrom512();

    // Erase the 4 KiB sector that $9000 shows through 16 KiB bank 1 — 8 KiB bank 2 of the
    // fixture, filled with $02.
    erase_sector(&mut core, 0x01, 0x9000);

    core.mapper_write(0xC000, 0x01); // Back to bank 1 to read it.
    assert_eq!(
        0xFF,
        core.read_memory(0x9000),
        "an erased sector reads back $FF, which is the page's verification read"
    );
    // And only that 4 KiB sector. $8000 is the same 8 KiB bank 2 below the sector, $B000 is
    // 8 KiB bank 3 above it; the fixture fills each with its own number.
    assert_eq!(0x02, core.read_memory(0x8000));
    assert_eq!(0x03, core.read_memory(0xB000));

    // Program one byte into 16 KiB bank 0, whose first half the fixture filled with $FF.
    program_byte(&mut core, 0x00, 0x9000, 0x5A);

    core.mapper_write(0xC000, 0x00);
    assert_eq!(0x5A, core.read_memory(0x9000), "the byte is programmed");
    assert_eq!(
        0xFF,
        core.read_memory(0x9001),
        "and only that byte: its neighbour is untouched"
    );
}

/// Q-117 — the two sequences reach whichever bank the driver names, which is the whole point of
/// the latch being out of the window: a board that can only ever program bank 0 could not
/// reprogram itself.
#[test]
fn q_117_unrom512_flashes_the_bank_the_driver_selects() {
    let mut core = flashable_unrom512();

    // Erase the same offset in two different 16 KiB banks, 1 and 3.
    erase_sector(&mut core, 0x01, 0x9000);
    erase_sector(&mut core, 0x03, 0x9000);

    for (bank, expected) in [(0x01u8, 0xFFu8), (0x03, 0xFF), (0x02, 0x04)] {
        core.mapper_write(0xC000, bank);
        assert_eq!(expected, core.read_memory(0x9000), "16 KiB bank {bank}");
    }

    // And a byte programmed into bank 1 is not a byte programmed into bank 3.
    program_byte(&mut core, 0x01, 0x9000, 0x3C);
    core.mapper_write(0xC000, 0x01);
    assert_eq!(0x3C, core.read_memory(0x9000));
    core.mapper_write(0xC000, 0x03);
    assert_eq!(0xFF, core.read_memory(0x9000));
}

/// Q-117 — the chip recognises its sequences on **chip** addresses. The same CPU address with
/// another bank latched is another chip address and unlocks nothing: $9555 in bank 0 is chip
/// $1555, not $5555.
#[test]
fn q_117_unrom512_unlocks_only_on_the_chips_own_command_addresses() {
    let mut core = flashable_unrom512();

    // The byte-program prologue with every bank write replaced by bank 2: the CPU addresses are
    // the page's, the chip addresses are not.
    for (address, value) in [(0x9555u16, 0xAAu8), (0xAAAA, 0x55), (0x9555, 0xA0)] {
        core.mapper_write(0xC000, 0x02);
        core.mapper_write(address, value);
    }
    core.mapper_write(0xC000, 0x00);
    core.mapper_write(0x9000, 0x5A);

    core.mapper_write(0xC000, 0x00);
    assert_eq!(
        0xFF,
        core.read_memory(0x9000),
        "an unlock on the wrong chip address must program nothing"
    );
}

/// MR-30-4 — an aborted sequence does not carry over: the state machine must not be left
/// half-open. **New in MF-2**, rewritten for Q-117.
#[test]
fn mr_30_4_unrom512_an_incomplete_flash_sequence_changes_nothing() {
    let mut core = flashable_unrom512();

    // The prologue of a byte write, then an unrecognised write instead of the $A0: the sequence
    // aborts, and the data write that follows must not be taken as the byte to program.
    core.mapper_write(0xC000, 0x01);
    core.mapper_write(0x9555, 0xAA);
    core.mapper_write(0xC000, 0x00);
    core.mapper_write(0xAAAA, 0x55);
    core.mapper_write(0xC000, 0x00);
    core.mapper_write(0x9000, 0x00); // Not $A0 at chip $5555: abort.
    core.mapper_write(0x9000, 0x5A);

    core.mapper_write(0xC000, 0x00);
    assert_eq!(
        0xFF,
        core.read_memory(0x9000),
        "an aborted sequence must not program anything"
    );
}

/// MR-30-4 — and a board **with** bus conflicts cannot flash at all, because they "interfere
/// with the self-flashing procedure" (nesdev, "UNROM 512", submapper 2). On that configuration
/// the latch keeps the whole of $8000-$FFFF (Q-117), so every byte of the sequence is a bank
/// write and nothing else. **New in MF-2.**
#[test]
fn mr_30_4_unrom512_with_bus_conflicts_cannot_flash() {
    let mut core = MapperRom::new(30).prg(8).chr(0).submapper(2).load();

    // The same erase sequence, which a conflicting board must not act on. The written values
    // are ANDed with the ROM anyway, so the sequence could never be recognised on hardware.
    // Every one of those writes is ANDed with the zero-filled PRG bank 0 at $8000, so the
    // register never moves and the windows stay [0, 1, 6, 7].
    erase_sector(&mut core, 0x01, 0x9000);

    assert_eq!(
        0x00,
        core.read_memory(0x9000),
        "a submapper-2 board must not erase its own ROM"
    );
    assert_eq!(0x01, core.read_memory(0xB000));
}

/// MR-30-6 — "The cartridge VRAM for the '4-screen with cartridge VRAM' comes from the last
/// 8 KiB of CHR-RAM", with the four nametables at PPU $2000-$2FFF and scratch RAM at
/// $3000-$3EFF (nesdev, "UNROM 512"). Byte 6 `%....1..1` is that mode (MR-30-1, fixed in MF-1).
/// **New in MF-2.**
#[test]
fn mr_30_6_unrom512_four_screen_comes_from_the_last_8_kib_of_chr_ram() {
    // Byte 6 bit 3 and bit 0 both set, and 32 KiB of CHR-RAM so the last 8 KiB is bank 3.
    let mut core = MapperRom::new(30)
        .prg(8)
        .chr(0)
        .mirroring(0x09)
        .battery()
        .load();

    // All four nametables come from the board, not from console VRAM.
    for table in 0..4 {
        assert_eq!(
            zax_nes::cartridges::Cartridge::SERVED_BY_CARTRIDGE,
            core.cartridge().unwrap().nametable_page(table),
            "table {table}"
        );
    }

    // Each of the four is its own kilobyte: writing the table number into the first byte of
    // each and reading all four back must give four different values.
    for table in 0..4u16 {
        core.cartridge_mut()
            .unwrap()
            .write_nametable(0x2000 + table * 0x0400, 0x10 + table as u8);
    }

    for table in 0..4u16 {
        assert_eq!(
            0x10 + table as u8,
            core.cartridge()
                .unwrap()
                .read_nametable(0x2000 + table * 0x0400),
            "nametable {table} must not be mirrored onto another"
        );
    }

    // And $3000-$3EFF is the scratch RAM that follows them, not a mirror of $2000.
    core.cartridge_mut().unwrap().write_nametable(0x3000, 0x77);
    assert_eq!(0x77, core.cartridge().unwrap().read_nametable(0x3000));
    assert_eq!(
        0x10,
        core.cartridge().unwrap().read_nametable(0x2000),
        "the scratch window must not alias nametable 0"
    );
}

/// MR-30-6 — and a board in any other arrangement leaves the nametables to the console, which
/// is every mapper-30 dump that is not `%....1..1`. **New in MF-2.**
#[test]
fn mr_30_6_unrom512_outside_four_screen_leaves_the_nametables_to_the_console() {
    for byte6 in [0x00, 0x01, 0x08] {
        let core = MapperRom::new(30).prg(8).chr(0).mirroring(byte6).load();
        assert_eq!(
            zax_nes::cartridges::Cartridge::FOLLOW_MIRRORING,
            core.cartridge().unwrap().nametable_page(0),
            "byte 6 {byte6:#04X}"
        );
    }
}

/// MR-30-5. NES 2.0 submapper 4 is "No bus conflicts, an additional register controls LEDs
/// inside the cartridge shell", at `D~[BYGR bygr] A~[10.. .... .... ....]` — $8000-$BFFF. A
/// write there must not move the PRG bank; the bank register is at $C000-$FFFF.
#[test]
fn mr_30_5_unrom512_submapper_4_does_not_bank_through_the_led_register() {
    let mut core = MapperRom::new(30).prg(8).chr(0).submapper(4).load();
    assert_eq!([0, 1, 6, 7], core.prg_windows());

    core.mapper_write(0x8000, 0x02); // The LED register.
    assert_eq!(
        [0, 1, 6, 7],
        core.prg_windows(),
        "an LED write must not move the PRG bank"
    );

    core.mapper_write(0xC000, 0x02); // The bank register.
    assert_eq!([4, 5, 6, 7], core.prg_windows());
}

// ================================================================ mapper 225, 64-in-1

/// MR-225-1. nesdev ("INES Mapper 225"): "$5800-5803: `[.... RRRR]` RAM (readable/writable)",
/// "16 bits of RAM -- 4 bits in each of the 4 regs", mirrored through $5FFF. We implemented
/// nothing there, so those addresses read as open bus.
#[test]
fn mr_225_1_multicart_225_has_four_nibbles_of_ram() {
    let mut core = MapperRom::new(225).prg(16).chr(32).load();

    for register in 0..4u16 {
        core.mapper_write(0x5800 + register, 0xF0 | (register as u8 + 1));
    }

    for register in 0..4u16 {
        assert_eq!(
            Some(register as u8 + 1),
            core.cartridge().unwrap().try_cpu_peek(0x5800 + register),
            "register {register} keeps four bits"
        );
        // Mirrored through $5FFF.
        assert_eq!(
            Some(register as u8 + 1),
            core.cartridge().unwrap().try_cpu_peek(0x5FFC + register),
            "register {register} is mirrored"
        );
    }
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

/// MR-1-3. nesdev ("MMC1"): "For SXROM, the upper `S` (bit 3) selects the SRAM's A14, and the
/// lower `S` (bit 2) selects A13", and "The SOROM board only implements the upper `S` bit".
/// SOROM is the 16 KiB board. Our `(chr0 >> 2) & 0x03` reduced, modulo a 16 KiB RAM, to bit 2
/// — one bit off — so Genghis Khan and Nobunaga's Ambition saved into the half that is not
/// battery-backed and read back from the one that is.
///
/// MR-1b recorded this test as a fixture gap: `MapperRom` had no PRG-RAM knob and a 16 KiB
/// declaration needs NES 2.0 byte 10. `MapperRom::prg_ram` is that knob.
#[test]
fn mr_1_3_mmc1_sorom_selects_its_ram_chip_with_bit_3_alone() {
    // 8 KiB of work RAM plus 8 KiB battery-backed: a SOROM.
    let mut core = MapperRom::new(1)
        .prg(16)
        .chr(0)
        .battery()
        .prg_ram(0x2000, 0x2000)
        .load();
    assert_eq!(0x4000, core.cartridge().unwrap().memory().prg_ram().len());

    core.write_memory(0x6000, 0x11); // Chip 0.
    core.mmc1_write(0xA000, 0x08); // CHR bank 0 with the upper S bit set: chip 1.
    core.write_memory(0x6000, 0x22);
    assert_eq!(0x22, core.read_memory(0x6000));

    core.mmc1_write(0xA000, 0x00);
    assert_eq!(
        0x11,
        core.read_memory(0x6000),
        "bit 3 clear is chip 0 again"
    );

    core.mmc1_write(0xA000, 0x04);
    assert_eq!(
        0x11,
        core.read_memory(0x6000),
        "the lower S bit is not wired on a SOROM: it must not move the selector"
    );
}

/// The 32 KiB SXROM does implement both `S` bits: the fix of MR-1-3 must not reach it.
#[test]
fn mr_1_3_an_sxrom_still_uses_both_s_bits() {
    let mut core = MapperRom::new(1)
        .prg(16)
        .chr(0)
        .battery()
        .prg_ram(0, 0x8000)
        .load();
    assert_eq!(0x8000, core.cartridge().unwrap().memory().prg_ram().len());

    for (bits, value) in [(0x00, 0x10), (0x04, 0x11), (0x08, 0x12), (0x0C, 0x13)] {
        core.mmc1_write(0xA000, bits);
        core.write_memory(0x6000, value);
    }

    for (bits, value) in [(0x00, 0x10), (0x04, 0x11), (0x08, 0x12), (0x0C, 0x13)] {
        core.mmc1_write(0xA000, bits);
        assert_eq!(value, core.read_memory(0x6000), "S bits {bits:#04X}");
    }
}

/// MR-1-5. nesdev ("MMC1"): "Even if the SOROM and SZROM boards utilizes a battery, it is
/// connected to only one PRG-RAM chip. The first RAM chip will not retain its data, but the
/// second one will." We persisted the whole PRG-RAM whenever the header declared a battery.
#[test]
fn mr_1_5_only_the_second_sorom_ram_chip_is_battery_backed() {
    let core = MapperRom::new(1)
        .prg(16)
        .chr(0)
        .battery()
        .prg_ram(0x2000, 0x2000)
        .load();

    assert_eq!(
        0x2000,
        core.cartridge().unwrap().battery_ram().len(),
        "the battery reaches one of the two chips, the second"
    );
}

/// The SXROM board is not covered by that sentence and keeps its whole 32 KiB battery-backed
/// (Q-113).
#[test]
fn mr_1_5_an_sxrom_keeps_its_whole_ram_battery_backed() {
    let core = MapperRom::new(1)
        .prg(16)
        .chr(0)
        .battery()
        .prg_ram(0, 0x8000)
        .load();

    assert_eq!(0x8000, core.cartridge().unwrap().battery_ram().len());
}

// ================================================================ mapper 4, MMC3 / MMC6

/// MR-4-9, second half — **AMD-061 §8, closing Q-114.** "Le sous-mappeur NES 2.0 d'une image
/// l'emporte sur la révision par défaut : une image marquée sous-mappeur 4 (MMC3 NEC / Sharp
/// MMC3A) a le compteur d'IRQ de révision A (un `$C000` chargé à `$00` ne produit qu'une seule
/// IRQ) ; toutes les autres images du mapper 4, dont toutes les images iNES 1.0, gardent la
/// révision B de REQ-CORE-100."
///
/// Revision B asserts whenever the counter is zero after a clock, so a latch of $00 gives an IRQ
/// on every A12 rise. Revision A also requires the counter to have been moving, so the pending
/// `$C001` reload lets the first clock through and nothing lets the next one. **New in MF-2.**
#[test]
fn amd_061_mmc3_submapper_4_has_the_revision_a_irq_counter() {
    for (submapper, expected) in [(None, 3), (Some(2), 3), (Some(4), 1)] {
        let mut rom = MapperRom::new(4).prg(8).chr(16);
        if let Some(submapper) = submapper {
            rom = rom.submapper(submapper);
        }

        let mut core = rom.load();
        core.write_memory(0x2001, 0x00); // No rendering: only the rises below clock it.
        core.mapper_write(0xC000, 0x00); // Latch $00.
        core.mapper_write(0xC001, 0x00); // Reload on the next clock.
        core.mapper_write(0xE001, 0x00); // Enable.

        // Three A12 rises, each acknowledged so the next one can be counted on its own.
        let mut irqs = 0;
        for i in 0..3 {
            {
                let cart = core.cartridge_mut().unwrap();
                cart.on_ppu_address_bus(0x0000, 10_000 + i * 100);
                cart.on_ppu_address_bus(0x1000, 10_000 + i * 100 + 50);
            }

            if irq(&core) {
                irqs += 1;
                core.mapper_write(0xE000, 0x00); // Acknowledge.
                core.mapper_write(0xE001, 0x00); // And re-enable.
            }
        }

        assert_eq!(
            expected, irqs,
            "submapper {submapper:?}: a latch of $00 must give {expected} IRQ(s) over three rises"
        );
    }
}

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

/// MR-4-6. The MMC6 "contains 1 KB of internal PRG RAM", at "CPU $7000-$7FFF: 1 KB PRG RAM,
/// mirrored" (nesdev, "MMC6"); Mesen's `GetSaveRamSize() = 0x400` is the same number. We took
/// the header's 8 KiB, which changed no byte a game could read — `mmc6_offset` masks with
/// `0x03FF` — but exposed and saved eight times the cartridge's RAM.
///
/// MR-1b called this test missing for want of a per-board RAM-size hook; the hook is
/// `registry::board_memory_sizes`, added with this fix.
#[test]
fn mr_4_6_mmc6_carries_one_kib_of_prg_ram() {
    let core = MapperRom::new(4).prg(8).chr(16).submapper(1).load();

    assert_eq!(
        0x0400,
        core.cartridge().unwrap().memory().prg_ram().len(),
        "the MMC6 carries 1 KiB of internal PRG RAM"
    );
}

/// A plain MMC3 keeps the header's 8 KiB: the fix of MR-4-6 must not reach it.
#[test]
fn mr_4_6_a_plain_mmc3_keeps_its_eight_kib() {
    let core = MapperRom::new(4).prg(8).chr(16).load();

    assert_eq!(0x2000, core.cartridge().unwrap().memory().prg_ram().len());
}

/// The save-state guarantee the MR-4-6 family of fixes needs: a blob written when the board's
/// RAM was larger still loads into the smaller one. MR-4-6 (MMC6, 8 KiB -> 1 KiB), MR-13-1
/// (CPROM, 32 KiB -> 16 KiB of CHR-RAM) and MR-30-2 (UNROM 512, 8 KiB -> none) all shrink a
/// memory `Cartridge::save_state` writes as a length-prefixed blob.
#[test]
fn mf_1_a_state_written_with_a_larger_board_ram_still_loads() {
    let mut writer = zax_nes::state::StateWriter::with_capacity(0x2000);
    writer.write_blob(&vec![0x5A; 0x2000]);
    let bytes = writer.into_vec();

    let mut reader = zax_nes::state::StateReader::new(&bytes, 0).unwrap();
    let mut smaller = vec![0u8; 0x0400];
    reader.read_blob_prefix_into(&mut smaller).unwrap();

    assert_eq!(vec![0x5A; 0x0400], smaller, "the prefix that fits is kept");
}

/// And the other way round: a blob written when the RAM was *smaller* leaves the rest of the
/// larger one at its power-on value.
#[test]
fn mf_1_a_state_written_with_a_smaller_board_ram_still_loads() {
    let mut writer = zax_nes::state::StateWriter::with_capacity(0x0400);
    writer.write_blob(&vec![0x5A; 0x0400]);
    let bytes = writer.into_vec();

    let mut larger = vec![0u8; 0x2000];
    let mut reader = zax_nes::state::StateReader::new(&bytes, 0).unwrap();
    reader.read_blob_prefix_into(&mut larger).unwrap();

    assert_eq!([0x5A, 0x5A], [larger[0], larger[0x03FF]]);
    assert_eq!([0x00, 0x00], [larger[0x0400], larger[0x1FFF]]);
}

/// MR-4-8. nesdev ("MMC6"): "If neither bank is enabled for reading, the $7000-$7FFF area is
/// open bus." We answered a driven `0` there. $6000-$6FFF, which the board does not decode at
/// all, was already open bus.
#[test]
fn mr_4_8_mmc6_window_is_open_bus_when_no_half_is_readable() {
    let mut core = MapperRom::new(4).prg(8).chr(16).submapper(1).load();

    // RAM disabled: $A001 is held at $00, so neither half reads.
    assert_eq!(None, core.cartridge().unwrap().try_cpu_peek(0x7000));
    assert_eq!(None, core.cartridge().unwrap().try_cpu_peek(0x6000));

    // RAM enabled, but $A001 still $00: still no readable half.
    core.mapper_write(0x8000, 0x20);
    assert_eq!(None, core.cartridge().unwrap().try_cpu_peek(0x7000));

    // Only the high half readable ($A001 bit 7): it answers, and the low half "reads back as
    // zero" rather than leaving the bus open.
    core.mapper_write(0xA001, 0xC0);
    core.write_memory(0x7200, 0x5A);
    assert_eq!(Some(0x5A), core.cartridge().unwrap().try_cpu_peek(0x7200));
    assert_eq!(Some(0x00), core.cartridge().unwrap().try_cpu_peek(0x7000));
}

/// MR-4-9. nesdev ("NES 2.0 submappers"), mapper 4: "Submapper 2: MMC3C with hard-wired
/// mirroring". The $A000 writes must be ignored, as they already are on a four-screen header.
///
/// Submapper 4, "NEC MMC3 (Loading latch with 0 disables IRQ)", is the rev A IRQ that
/// REQ-CORE-100 pins to rev B for this board; it is a spec question, filed as Q-114, and not
/// implemented here.
#[test]
fn mr_4_9_mmc3_submapper_2_has_hard_wired_mirroring() {
    let mut core = MapperRom::new(4).prg(8).chr(16).submapper(2).load();
    let wired = core.mirroring();

    core.mapper_write(0xA000, 0x01);
    assert_eq!(wired, core.mirroring(), "$A000 is ignored on submapper 2");
    core.mapper_write(0xA000, 0x00);
    assert_eq!(wired, core.mirroring());

    // A plain MMC3 still follows $A000.
    let mut core = MapperRom::new(4).prg(8).chr(16).load();
    core.mapper_write(0xA000, 0x01);
    assert_eq!(zax_nes::NametableMirroring::Horizontal, core.mirroring());
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

/// MR-64-3, the other side of the threshold: a low of 48 dots or more **does** clock the
/// counter, so the filter is long and not infinite. Two counting rises take a latch of 1 from
/// the reload to zero. **New in MF-2.**
#[test]
fn mr_64_3_rambo1_accepts_a_low_of_sixteen_m2_cycles() {
    let mut core = MapperRom::new(64).prg(8).chr(16).load();
    core.write_memory(0x2001, 0x00); // No rendering: the PPU drives no A12 of its own.
    core.mapper_write(0xC000, 0x01); // Latch 1.
    core.mapper_write(0xC001, 0x00); // Scanline mode, reload pending.
    core.mapper_write(0xE001, 0x00); // Enable.

    {
        let cart = core.cartridge_mut().unwrap();
        for i in 0..2 {
            cart.on_ppu_address_bus(0x0000, 10_000 + i * 200);
            cart.on_ppu_address_bus(0x1000, 10_000 + i * 200 + 48);
        }
    }

    // The counter is at zero; MR-64-4's delay is what stands between it and the line.
    core.bus_mut().tick();
    core.bus_mut().tick();
    assert!(
        irq(&core),
        "a 48-dot low is sixteen M2 cycles: the counter must have moved twice"
    );
}

/// MR-64-4 — "the actual interrupt triggers slightly later. Specifically, it is delayed until M2
/// falls twice after the PPU A12 rise" in scanline mode (nesdev, "RAMBO-1"). We pulled the line
/// on the rise itself. **New in MF-2.**
#[test]
fn mr_64_4_rambo1_asserts_two_m2_cycles_after_the_a12_rise() {
    let mut core = MapperRom::new(64).prg(8).chr(16).load();
    core.write_memory(0x2001, 0x00);
    core.mapper_write(0xC000, 0x01);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);

    {
        let cart = core.cartridge_mut().unwrap();
        for i in 0..2 {
            cart.on_ppu_address_bus(0x0000, 10_000 + i * 200);
            cart.on_ppu_address_bus(0x1000, 10_000 + i * 200 + 48);
        }
    }

    assert!(!irq(&core), "not on the rise itself");
    core.bus_mut().tick();
    assert!(!irq(&core), "nor one M2 fall later");
    core.bus_mut().tick();
    assert!(irq(&core), "the line falls on the second M2 fall");
}

/// MR-64-4, cycle mode — there the interrupt "triggers one M2 cycle later than one would
/// naively expect", one cycle and not two. **New in MF-2.**
#[test]
fn mr_64_4_rambo1_cycle_mode_asserts_one_m2_cycle_later() {
    let mut core = MapperRom::new(64).prg(8).chr(16).load();
    core.write_memory(0x2001, 0x00);
    core.mapper_write(0xC000, 0x01); // Latch 1.
    core.mapper_write(0xC001, 0x01); // Cycle mode, reload pending.
    core.mapper_write(0xE001, 0x00);

    // One clock every four CPU cycles; the first reloads to 1 and the second reaches zero, so
    // the counter lands eight cycles after the divider was cleared by the $C001 write. The
    // assertion is what this test is about, so it is read one cycle at a time around it.
    let mut asserted_at = -1i32;
    for cycle in 1..=40 {
        core.bus_mut().tick();
        if irq(&core) {
            asserted_at = cycle;
            break;
        }
    }

    assert!(asserted_at > 0, "the cycle-mode counter never reached zero");

    // Re-run with the delay removed from the expectation: the counter reaches zero on a
    // multiple of four after the $C001 write, and the line follows one cycle later, so the
    // cycle it lands on is never a multiple of four.
    assert_ne!(
        0,
        asserted_at % 4,
        "cycle {asserted_at} is a clock cycle: the one-M2 delay is missing"
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
fn mr_64_7_rambo1_prg_mode_swaps_r6_and_r15() {
    let mut core = MapperRom::new(64).prg(8).chr(16).load();

    core.mapper_write(0x8000, 6); // Register 6.
    core.mapper_write(0x8001, 2);
    core.mapper_write(0x8000, 7); // Register 7.
    core.mapper_write(0x8001, 3);
    core.mapper_write(0x8000, 15); // Register 15.
    core.mapper_write(0x8001, 4);
    assert_eq!(
        [2, 3, 4, 7],
        core.prg_windows(),
        "mode 0: R6, R7, RF, fixed"
    );

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

/// MR-5-5. nesdev ("MMC5"), the ExRAM table for modes %00 and %01: a CPU read of $5C00-$5FFF
/// "returns open bus". We answered a driven 0. Same family as MR-4-8 and MR-30-2.
#[test]
fn mr_5_5_mmc5_exram_reads_are_open_bus_in_the_nametable_modes() {
    let mut core = MapperRom::new(5).prg(8).chr(16).load();

    core.mapper_write(0x5104, 0x02); // ExRAM in RAM mode, so the CPU can fill it.
    core.write_memory(0x5C00, 0x5A);
    assert_eq!(Some(0x5A), core.cartridge().unwrap().try_cpu_peek(0x5C00));

    for mode in [0x00, 0x01] {
        core.mapper_write(0x5104, mode);
        assert_eq!(
            None,
            core.cartridge().unwrap().try_cpu_peek(0x5C00),
            "ExRAM mode {mode} leaves the CPU side of $5C00-$5FFF open"
        );
    }
}

// ================================================================ mapper 156, Daou Infosys

/// MR-156-1. $C014 is a two-bit field with three states, not a single H/V bit.
///
/// nesdev ("INES Mapper 156"): the register holds "00: Vertical, 01: Horizontal, 1x:
/// One-screen, page 0", and "Mirroring defaults to one-screen". We read bit 0 only
/// (`daou306.rs:74-78`), as does Mesen (`DaouInfosys.h:64`), so a board that has left
/// one-screen can never return to it.
#[test]
fn mr_156_1_daou306_mirroring_is_a_two_bit_field() {
    let mut core = MapperRom::new(156).prg(8).chr(16).load();

    core.mapper_write(0xC014, 0x01); // Horizontal.
    core.mapper_write(0xC014, 0x02); // One-screen, page 0.

    assert_eq!(
        zax_nes::NametableMirroring::SingleScreenA,
        core.mirroring(),
        "a $C014 value of 2 selects one-screen page 0, not vertical"
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

/// MR-21-3. The whole variant table, one row per chip. `register_index` must decode the
/// address through that chip's two address lines, so a write to the CHR-0 register of group
/// $B000 — the address whose index is 0 under the variant's own wiring — must land on CHR slot
/// 0 and nowhere else.
///
/// nesdev ("VRC2 and VRC4"), the variant table: VRC2a (22) A1 A0; VRC2b (23) A0 A1; VRC2c (25)
/// A1 A0; VRC4a (21) A1 A2; VRC4b (25) A1 A0; VRC4c (21) A6 A7; VRC4d (25) A3 A2; VRC4e (23)
/// A2 A3; VRC4f (23) A0 A1. The submapper of each row comes from "NES 2.0 submappers".
/// VRC4c's A6/A7 is the pair MR-1b singled out: no OR of A0-A3 can reach it.
#[test]
fn mr_21_3_every_vrc_variant_decodes_its_own_address_lines() {
    // (mapper, submapper, the line that supplies index bit 1, the chip's name).
    let variants = [
        (22u16, None, 0u32, "VRC2a"),
        (23, Some(3), 1, "VRC2b"),
        (25, Some(3), 0, "VRC2c"),
        (21, Some(1), 2, "VRC4a"),
        (25, Some(1), 0, "VRC4b"),
        (21, Some(2), 7, "VRC4c"),
        (25, Some(2), 2, "VRC4d"),
        (23, Some(2), 3, "VRC4e"),
        (23, Some(1), 1, "VRC4f"),
    ];

    for (mapper, submapper, bit1_line, name) in variants {
        let mut rom = MapperRom::new(usize::from(mapper)).prg(8).chr(16);
        if let Some(submapper) = submapper {
            rom = rom.submapper(submapper);
        }
        let mut core = rom.load();

        // Index 2 selects CHR slot 1 of the group, so the address that carries only the
        // bit-1 line must move slot 1 and leave slot 0 alone.
        core.mapper_write(0xB000 | (1 << bit1_line), 0x07);

        let mut expected = [0u8; 8];
        // VRC2a ignores the low bit of its CHR registers.
        expected[1] = if mapper == 22 { 3 } else { 7 };
        assert_eq!(expected, core.chr_windows(), "{name}");
    }
}

/// And mappers 23 and 25 without a submapper keep the OR'd heuristic, which is the only place
/// MR-1b leaves it: a VRC2b or VRC4e dump that does not declare itself still works.
#[test]
fn mr_21_3_submapper_zero_keeps_the_ord_heuristic() {
    for mapper in [21usize, 23, 25] {
        let mut core = MapperRom::new(mapper).prg(8).chr(16).load();
        assert_eq!([0, 1, 6, 7], core.prg_windows(), "mapper {mapper}");

        // Both candidate wirings of the number reach register 2, so both move CHR slot 1.
        let lines: [u32; 2] = match mapper {
            21 => [2, 7],
            23 => [1, 3],
            _ => [0, 2],
        };
        for line in lines {
            core.mapper_write(0xB000 | (1 << line), 0x07);
            assert_eq!(7, core.chr_windows()[1], "mapper {mapper}, A{line}");
            core.mapper_write(0xB000 | (1 << line), 0x00);
        }
    }
}

/// A VRC2b (mapper 23 submapper 3) has no IRQ counter, where a submapper-0 mapper-23 dump
/// keeps one because the variant is unknown (MR-21-3).
#[test]
fn mr_21_3_a_declared_vrc2b_has_no_irq_counter() {
    let mut core = MapperRom::new(23).prg(8).chr(16).submapper(3).load();

    core.mapper_write(0xF000, 0x0F);
    core.mapper_write(0xF001, 0x0F); // Latch $FF: one clock would be enough.
    core.mapper_write(0xF002, 0x06); // Enable, cycle mode.
    tick(&mut core, 2000);

    assert!(
        !core.cartridge().unwrap().irq_asserted(),
        "the VRC2 has no IRQ counter"
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

// MR-18-1's test (`mr_18_1_jaleco_ss88006_irq_fires_one_cycle_earlier`) was withdrawn by lot
// MR-1b. It asserted Mesen's `if(--counter == 0)`, which fires when the counter leaves 1. The
// only statement nesdev ("INES Mapper 018") makes about the firing point is "When it wraps, an
// IRQ is generated" — a counter wraps on the clock that decrements a zero, which is our model
// (`jaleco_ss88006.rs:174-179`). The sentence is in the page's notes rather than a
// cycle-level description, so the finding is `unknown` leaning ours rather than "Mesen in
// error". See `docs/review/MR-1-mappers.md`, MR-1b section.
