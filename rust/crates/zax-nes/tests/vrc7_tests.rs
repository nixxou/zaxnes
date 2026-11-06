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

// ------------------------------------------------------------------- save states

#[test]
fn req_core_090_a_vrc7_state_round_trips_its_arrangement_and_banks() {
    // REQ-CORE-090: the banks, the arrangement and `$E000`'s two gating bits come back from the
    // state. The arrangement is the interesting one on this board, because it comes from
    // `$E000` and from nowhere else: a load that re-derived it from the register rather than
    // taking the state's would agree by luck here and not after a mid-frame change.
    let mut core = vrc7a();
    core.mapper_write(0x8000, 0x11);
    core.mapper_write(0x8010, 0x12);
    core.mapper_write(0x9000, 0x13);
    for slot in 0..8u16 {
        let address = 0xA000 + (slot / 2) * 0x1000 + if slot % 2 == 1 { 0x10 } else { 0 };
        core.mapper_write(address, (slot + 1) as u8);
    }

    core.mapper_write(0xE000, 0x82); // WRAM enabled, one-screen A, not silenced.
    core.write_memory(0x6000, 0x5A);

    let state = core.save_state(true).unwrap();
    let windows = core.prg_windows();
    let chr = core.chr_windows();

    // Drive it elsewhere: other banks, the other arrangement, the PRG-RAM protected.
    core.mapper_write(0x8000, 0x01);
    core.mapper_write(0xA000, 0x00);
    core.mapper_write(0xE000, 0x01);
    for _ in 0..3 {
        core.run_frame(0);
    }

    core.try_load_state(&state, 0).unwrap();
    assert_eq!(windows, core.prg_windows(), "the three PRG banks");
    assert_eq!(chr, core.chr_windows(), "the eight CHR banks");
    assert_eq!(
        NametableMirroring::SingleScreenA,
        core.cartridge().unwrap().mirroring(),
        "the arrangement comes back from the state"
    );
    assert_eq!(0x5A, core.read_memory(0x6000), "and the PRG-RAM enable");
}

#[test]
fn req_core_091_a_vrc7_state_brings_back_the_irq_counter() {
    // REQ-CORE-091: the shared `VrcIrq` comes back mid-count, through this board's own decode,
    // and fires again from where the state left it rather than from the latch.
    let mut core = vrc7a();
    core.mapper_write(0xE010, 0xF0); // Latch $F0: sixteen cycles to the top.
    core.mapper_write(0xF000, 0x06); // E and M: enabled, cycle mode.
    for _ in 0..10 {
        core.bus_mut().tick();
    }

    assert!(!core.cartridge().unwrap().irq_asserted());
    let state = core.save_state(true).unwrap();

    // Drive it elsewhere: let it fire, acknowledge it, and disable the counter.
    for _ in 0..20 {
        core.bus_mut().tick();
    }
    assert!(core.cartridge().unwrap().irq_asserted(), "it fires first");
    core.mapper_write(0xF010, 0x00);
    core.mapper_write(0xF000, 0x00);
    for _ in 0..200 {
        core.bus_mut().tick();
    }

    core.try_load_state(&state, 0).unwrap();
    assert!(
        !core.cartridge().unwrap().irq_asserted(),
        "the restored counter is still short of its top"
    );

    // Six more cycles than the state had run: $FA through $FF, then the wrap.
    for _ in 0..5 {
        core.bus_mut().tick();
    }
    assert!(
        !core.cartridge().unwrap().irq_asserted(),
        "and it counts from where the state left it, not from the latch"
    );
    core.bus_mut().tick();
    assert!(core.cartridge().unwrap().irq_asserted());
}

#[test]
fn req_core_092_a_silent_vrc7_saves_and_restores_byte_for_byte() {
    // REQ-CORE-092: "Sauvegarder puis restaurer immédiatement ne modifie rien." This is the
    // assertion that caught the VRC7's first mixer level: a **standing** DC offset on `M` made
    // the chip's `M` non-zero even with no note playing, so the high-pass filter carried state
    // that REQ-CORE-074 then cleared on the restore, and the bytes differed by three of an `f64`
    // at -0.4476 (that is -14 670 / 32 768). Q-128 is the ticket; the offset is now carried per
    // **sounding** channel, so a chip with nothing playing has an `M` of exactly 0.
    //
    // The chip is left silent on purpose, and the companion test
    // `req_core_092_a_sounding_apu_is_not_byte_identical_on_any_board` records why a sounding
    // one cannot be compared this way.
    let mut core = vrc7a();
    core.mapper_write(0x8000, 0x11);
    core.mapper_write(0xE000, 0x81);
    core.write_memory(0x6000, 0x42);
    for _ in 0..2 {
        core.run_frame(0);
    }

    let state = core.save_state(true).unwrap();
    core.try_load_state(&state, 0).unwrap();
    assert_eq!(
        state,
        core.save_state(true).unwrap(),
        "a save, a restore and a save must give the same bytes"
    );
}

/// The companion of the test above, and the reason it keeps the chip silent: a state saved while
/// **any** expansion chip is sounding is not byte-identical after an immediate restore, and the
/// cause is in the APU's own load path rather than in any board.
///
/// `Apu2A03::load_state` re-derives its cached level with `M` forced to 0 —
/// `self.mixer_output = mixer::clip(self.mixer_apu, 0)` — because "`M` and the two integers of
/// the test point are not in the block: the board restores its own sound state and the bus pushes
/// `M` again before the next cycle", which is what lets a block written before AMD-054 still
/// load. So the `f64` of that cached level is `to_float(A − M)` when it is saved and
/// `to_float(A)` when it is read back, and the two differ by exactly `M / 32768`.
///
/// Demonstrated on **two** boards, a mapper-24 VRC6 and this lot's mapper-85 VRC7, so that the
/// property is visibly the APU's and not the VRC7's. Without this test, the one above would read
/// as a weaker version of a general invariant; the general invariant does not hold, and what the
/// VRC7 had to stop doing (Q-128) was reporting an `M` while **silent**.
#[test]
fn mf_5b_a_sounding_expansion_chip_is_not_byte_identical_on_any_board() {
    let round_trip_differs = |core: &mut NesCore| -> bool {
        for _ in 0..2 {
            core.run_frame(0);
        }

        let state = core.save_state(true).unwrap();
        core.try_load_state(&state, 0).unwrap();
        state != core.save_state(true).unwrap()
    };

    // A VRC6 with its first pulse running: $9000 volume 15 mode M, $9002 enable.
    let mut vrc6 = MapperRom::new(24).chr(32).load();
    vrc6.write_memory(0x9000, 0x8F);
    vrc6.write_memory(0x9001, 0x40);
    vrc6.write_memory(0x9002, 0x80);
    assert!(
        round_trip_differs(&mut vrc6),
        "a sounding VRC6 is not byte-identical either -- the cause is the APU's load path"
    );

    // And the VRC7, with one channel keyed on.
    let mut vrc7 = vrc7a();
    vrc7.write_memory(0x9010, 0x30);
    vrc7.write_memory(0x9030, 0x10);
    vrc7.write_memory(0x9010, 0x20);
    vrc7.write_memory(0x9030, 0x1B);
    assert!(
        round_trip_differs(&mut vrc7),
        "and so is a sounding VRC7, for the same reason and no other"
    );
}
