//! The Konami VRC7 board of mapper 85 (REQ-CORE-100's table does not list it, which
//! REQ-CORE-102 allows; its FM sound is REQ-CORE-075, optional, and lives in
//! `vrc7_audio_tests.rs`).
//!
//! Read from the public nesdev pages "VRC7", "VRC IRQ" and "NES 2.0 submappers", cross-checked
//! against `third_party/mesen2`'s `Konami/VRC7.h`. **No mapper-85 test ROM exists anywhere in
//! `assets-free/test-roms`** — step 4 of `docs/progress/MF-5b.md` records the sweep of all 330
//! headers that establishes it — so every assertion below is a sentence of one of those pages
//! turned into an assertion, and R0's two commercial titles (*Lagrange Point*, submapper 2, and
//! *Tiny Toon Adventures 2* (J), submapper 1) are what check the whole.

// Register values, bank numbers and periods are small literals.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::unwrap_used
)]

mod common;

use common::mapper_fixture::{MapperMachine, MapperRom};
use zax_nes::NametableMirroring;
use zax_nes::NesCore;

/// A machine with a synthetic mapper-85 cartridge at `submapper`, powered on: 64 PRG banks of
/// 8 KiB so that a six-bit bank register has somewhere to point, and 16 CHR banks of 1 KiB.
fn core_85(submapper: u8) -> NesCore {
    MapperRom::new(85)
        .prg(64)
        .chr(16)
        .submapper(submapper)
        .load()
}

/// The `VRC7a` of *Lagrange Point*: A4 selects the register.
fn vrc7a() -> NesCore {
    core_85(2)
}

/// The `VRC7b` of *Tiny Toon Adventures 2*: A3 selects the register.
fn vrc7b() -> NesCore {
    core_85(1)
}

/// A dump that names no submapper, which is every iNES 1.0 image: both lines select.
fn vrc7_either() -> NesCore {
    MapperRom::new(85).prg(64).chr(16).load()
}

// ------------------------------------------------------------------------------ PRG

#[test]
fn req_core_100_vrc7_switches_three_8_kib_prg_banks_and_wires_the_last() {
    // nesdev, "VRC7": three "8 KB PRG bank" registers at $8000, $8010 and $9000, and
    // "$E000-$FFFF" is absent from the table — the last bank is wired there.
    let mut core = vrc7a();
    assert_eq!([0, 1, 2, 63], core.prg_windows(), "the power-on layout");

    core.mapper_write(0x8000, 0x07);
    core.mapper_write(0x8010, 0x11);
    core.mapper_write(0x9000, 0x2A);
    assert_eq!([0x07, 0x11, 0x2A, 63], core.prg_windows());

    // Six bits: "..PP PPPP". Bits 6 and 7 latch nothing, so $C7 is bank 7 again.
    core.mapper_write(0x8000, 0xC7);
    assert_eq!(0x07, core.prg_windows()[0]);

    // The last window never moves, whatever is written at $E000 — which is a control register
    // and not a bank register on this board.
    core.mapper_write(0xE000, 0x05);
    assert_eq!(63, core.prg_windows()[3]);
}

#[test]
fn req_core_100_vrc7_ignores_a5_to_a11_in_a_register_address() {
    // "Registers are selected by A12-A15" and one further line, so every address of a $1000
    // page that agrees on the select line is the same register. $8F60 and $8000 are the same
    // register on either variant, A5-A11 being decoded by nothing.
    for submapper in [1, 2] {
        let mut core = core_85(submapper);
        core.mapper_write(0x8F60, 0x09);
        assert_eq!(0x09, core.prg_windows()[0]);
        core.mapper_write(0x8EC0, 0x0A);
        assert_eq!(0x0A, core.prg_windows()[0]);
    }
}

// ------------------------------------------------------------------- the select line

#[test]
fn req_core_100_a_vrc7a_selects_registers_with_a4_and_a_vrc7b_with_a3() {
    // nesdev, "VRC7" and "NES 2.0 submappers": "VRC7b uses A3 to select registers ($x008), used
    // in Tiny Toon Adventures (Submapper 1). VRC7a uses A4 to select registers ($x010), used in
    // Lagrange Point (Submapper 2)."
    //
    // The second PRG register is the one place the difference is visible without touching the
    // sound: on a VRC7a, $8008 is a mirror of $8000 and $8010 is PRG 1; on a VRC7b it is the
    // other way round.
    let mut a = vrc7a();
    a.mapper_write(0x8010, 0x15);
    assert_eq!([0, 0x15, 2, 63], a.prg_windows(), "A4 selects on a VRC7a");
    a.mapper_write(0x8008, 0x16);
    assert_eq!(
        [0x16, 0x15, 2, 63],
        a.prg_windows(),
        "A3 is decoded by nothing on a VRC7a, so $8008 is $8000"
    );

    let mut b = vrc7b();
    b.mapper_write(0x8008, 0x15);
    assert_eq!([0, 0x15, 2, 63], b.prg_windows(), "A3 selects on a VRC7b");
    b.mapper_write(0x8010, 0x16);
    assert_eq!(
        [0x16, 0x15, 2, 63],
        b.prg_windows(),
        "A4 is decoded by nothing on a VRC7b, so $8010 is $8000"
    );
}

#[test]
fn req_core_100_a_vrc7_without_a_submapper_answers_to_both_lines() {
    // Submapper 0 — and every iNES 1.0 dump, which is most of them — names neither variant, so
    // both lines select: the OR'd reading `Vrc2Vrc4` falls back on for mappers 21, 23 and 25,
    // and the one Mesen applies unconditionally (`VRC7.h:91-94`).
    let mut core = vrc7_either();
    core.mapper_write(0x8008, 0x15);
    assert_eq!([0, 0x15, 2, 63], core.prg_windows(), "$8008 reaches PRG 1");
    core.mapper_write(0x8010, 0x16);
    assert_eq!([0, 0x16, 2, 63], core.prg_windows(), "and so does $8010");

    // A3 and A4 both set is still the second register, not a third.
    core.mapper_write(0x8018, 0x17);
    assert_eq!([0, 0x17, 2, 63], core.prg_windows());
    // And the first register is still reachable.
    core.mapper_write(0x8000, 0x18);
    assert_eq!([0x18, 0x17, 2, 63], core.prg_windows());
}

// ------------------------------------------------------------------------------ CHR

#[test]
fn req_core_100_vrc7_switches_eight_1_kib_chr_banks() {
    // nesdev, "VRC7": eight 1 KiB CHR registers, two per page from $A000 to $D010 (or $D008).
    // The whole byte latches — the page gives "BBBB BBBB" and no mask.
    for (submapper, second) in [(2u8, 0x10u16), (1, 0x08)] {
        let mut core = core_85(submapper);
        assert_eq!([0; 8], core.chr_windows(), "the power-on layout");

        let banks = [3u8, 1, 4, 1, 5, 9, 2, 6];
        for (slot, bank) in banks.iter().enumerate() {
            let page = 0xA000 + ((slot as u16) / 2) * 0x1000;
            let address = page + if slot % 2 == 1 { second } else { 0 };
            core.mapper_write(address, *bank);
        }

        assert_eq!(banks, core.chr_windows());
    }
}

#[test]
fn req_core_100_a_vrc7_chr_register_takes_the_whole_byte() {
    // 16 CHR banks here, so bank $F1 wraps to 1 and bank $F0 to 0: the point is that bits 4-7
    // reach the window at all, which a four-bit field would not let them do.
    let mut core = MapperRom::new(85).prg(8).chr(256).submapper(2).load();
    core.mapper_write(0xA000, 0xF1);
    assert_eq!(0xF1, core.chr_windows()[0]);
    core.mapper_write(0xD010, 0x80);
    assert_eq!(0x80, core.chr_windows()[7]);
}

// ------------------------------------------------------------------- $E000's four bits

#[test]
fn req_core_100_vrc7_picks_its_arrangement_from_the_two_low_bits_of_e000() {
    // nesdev, "VRC7", `$E000` = `RS.. ..MM`: "Mirroring (0: vertical; 1: horizontal; 2/3:
    // one-screen)". Mesen names the two one-screen rows `ScreenAOnly` and `ScreenBOnly`
    // (`VRC7.h:70-77`), which are this crate's `SingleScreenA` and `SingleScreenB`.
    let rows = [
        (0x00u8, NametableMirroring::Vertical),
        (0x01, NametableMirroring::Horizontal),
        (0x02, NametableMirroring::SingleScreenA),
        (0x03, NametableMirroring::SingleScreenB),
    ];

    let mut core = vrc7a();
    for (value, expected) in rows {
        // The high bits of the register change nothing about the arrangement.
        core.mapper_write(0xE000, value | 0xC0);
        assert_eq!(
            expected,
            core.cartridge().unwrap().mirroring(),
            "${value:02X}"
        );
    }
}

#[test]
fn req_core_100_a_vrc7_is_vertical_at_power_on_whatever_the_header_says() {
    // The board drives CIRAM A10 from `$E000` alone and has no pass-through mode, so the
    // power-on value of 0 is **vertical** even on a dump whose header byte 6 says horizontal.
    // Mesen does the same (`UpdateState` is called from `InitMapper`). This is the one board in
    // the build whose power-on state overrides the header's arrangement, and a reset puts it
    // back there (AMD-061 §5).
    let mut core = MapperRom::new(85)
        .prg(8)
        .chr(16)
        .submapper(2)
        .mirroring(0x01) // Byte 6 bit 0 set: horizontal.
        .load();
    assert_eq!(
        NametableMirroring::Vertical,
        core.cartridge().unwrap().mirroring()
    );

    core.mapper_write(0xE000, 0x03);
    assert_eq!(
        NametableMirroring::SingleScreenB,
        core.cartridge().unwrap().mirroring()
    );

    core.reset();
    assert_eq!(
        NametableMirroring::Vertical,
        core.cartridge().unwrap().mirroring()
    );
    // Eight PRG banks on this image, so the wired last one is 7.
    assert_eq!([0, 1, 2, 7], core.prg_windows(), "and the banks come back");
}

#[test]
fn req_core_100_vrc7_enables_prg_ram_with_bit_7_of_e000() {
    // nesdev, "VRC7": `$E000` bit 7 is "WRAM enable (1: enable WRAM, 0: protect)", over the
    // fixed "8 KB PRG-RAM bank" at $6000-$7FFF. Mesen maps the window `NoAccess` while the bit
    // is clear (`VRC7.h:18-26`), so a read is open bus and not a stale byte.
    let mut core = vrc7a();
    // Power-on is $E000 = 0, so the window is protected.
    core.write_memory(0x6000, 0x5A);
    assert_ne!(0x5A, core.read_memory(0x6000));

    core.mapper_write(0xE000, 0x80);
    core.write_memory(0x6000, 0x5A);
    assert_eq!(0x5A, core.read_memory(0x6000));

    // Disabling it again hides what was written without losing it.
    core.mapper_write(0xE000, 0x00);
    assert_ne!(0x5A, core.read_memory(0x6000));
    core.mapper_write(0xE000, 0x80);
    assert_eq!(0x5A, core.read_memory(0x6000));
}

// ------------------------------------------------------------------------------ the IRQ

#[test]
fn req_core_100_the_vrc7_reaches_the_vrc_irq_through_its_own_three_registers() {
    // nesdev, "VRC7": the IRQ latch is at `$E010`/`$E008`, the control register at `$F000` and
    // the acknowledge at `$F010`/`$F008` — one page further on than the VRC6's, and the latch
    // behind the select line where the VRC6 writes it at the page's first address.
    //
    // The counter itself is `VrcIrq`, which `mapper_irq_tests.rs` covers for the VRC4 and the
    // VRC6; what is checked here is this board's decode.
    let mut core = vrc7a();
    core.mapper_write(0xE010, 0xFD); // Latch: three cycles short of the top.
    core.mapper_write(0xF000, 0x06); // E and M: enabled, cycle mode.

    assert!(!core.cartridge().unwrap().irq_asserted());
    for _ in 0..3 {
        core.bus_mut().tick();
    }
    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "$FD, $FE, $FF then the wrap: three cycles"
    );

    // "Writing any value to this register will acknowledge the pending IRQ."
    core.mapper_write(0xF010, 0x00);
    assert!(!core.cartridge().unwrap().irq_asserted());

    // And the latch is behind the select line: on a VRC7a, $E000 is the control register, so
    // writing a latch value there moves the arrangement instead of the counter.
    core.mapper_write(0xE000, 0x01);
    assert_eq!(
        NametableMirroring::Horizontal,
        core.cartridge().unwrap().mirroring()
    );
}

#[test]
fn mf_5b_the_vrc7_and_the_vrc6_share_one_irq_counter() {
    // MF-5a step 1 extracted `VrcIrq` so that the VRC4 and the VRC6 drive one counter; this is
    // the same check for the third board on it. Given the same latch, the same mode and the
    // same number of ticks, a mapper-24 VRC6 and a mapper-85 VRC7 must fire on the same cycle,
    // through their two different register decodes.
    let mut vrc6 = MapperRom::new(24).chr(32).load();
    vrc6.write_memory(0xF000, 0xCA); // Latch, written whole.
    vrc6.write_memory(0xF001, 0x06); // Enable, cycle mode.

    let mut vrc7 = vrc7a();
    vrc7.write_memory(0xE010, 0xCA); // The same latch, one page further on.
    vrc7.write_memory(0xF000, 0x06);

    let tick_until_irq = |core: &mut NesCore| -> i64 {
        for cycle in 1..=1000 {
            core.bus_mut().tick();
            if core.cartridge().unwrap().irq_asserted() {
                return cycle;
            }
        }

        -1
    };

    let expected = 0x100 - 0xCA;
    assert_eq!(expected, tick_until_irq(&mut vrc6));
    assert_eq!(expected, tick_until_irq(&mut vrc7));
}

#[test]
fn req_core_100_the_vrc7_counter_runs_on_the_scanline_prescaler_too() {
    // The `M` bit clear is the prescaler mode: "a prescaler which divides the CPU clock by
    // 341", so a step every 341/3 CPU cycles. Checked through this board's own decode, because
    // a board that routed `$F000` to the latch instead of the control register would still pass
    // the cycle-mode test above by accident.
    let mut core = vrc7a();
    core.mapper_write(0xE010, 0xFF);
    core.mapper_write(0xF000, 0x02); // E set, M clear.

    for _ in 0..113 {
        core.bus_mut().tick();
    }
    assert!(
        !core.cartridge().unwrap().irq_asserted(),
        "341/3 is more than 113 cycles"
    );
    core.bus_mut().tick();
    assert!(core.cartridge().unwrap().irq_asserted());
}
