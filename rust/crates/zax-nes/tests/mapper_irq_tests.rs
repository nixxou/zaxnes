//! Mapper IRQ counters (REQ-CORE-114, REQ-CORE-100).

// Test values are small: the narrowing casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::unwrap_used
)]

mod common;

use common::mapper_fixture::{MapperMachine, MapperRom};
use zax_nes::NesCore;

/// Ticks the bus until the cartridge asserts its IRQ; the number of ticks, or -1 when none
/// comes within `budget`.
fn tick_until_irq(core: &mut NesCore, budget: i64) -> i64 {
    for cycle in 1..=budget {
        core.bus_mut().tick();
        if core.cartridge().unwrap().irq_asserted() {
            return cycle;
        }
    }

    -1
}

fn irq(core: &NesCore) -> bool {
    core.cartridge().unwrap().irq_asserted()
}

// ---------------------------------------------------------------- MMC3 and RAMBO-1

fn enable_scanline_counting(core: &mut NesCore) {
    core.write_memory(0x2000, 0x08);
    core.write_memory(0x2001, 0x18);
}

#[test]
fn req_core_114_the_mmc3_counter_fires_once_every_latch_plus_one_lines() {
    let mut core = MapperRom::new(4).chr(32).load();
    enable_scanline_counting(&mut core);
    core.mapper_write(0xC000, 20); // Latch.
    core.mapper_write(0xC001, 0x00); // Reload on the next clock.
    core.mapper_write(0xE001, 0x00); // Enable.
    assert!(tick_until_irq(&mut core, 200_000) > 0);
    core.mapper_write(0xE000, 0x00); // Acknowledge.
    core.mapper_write(0xE001, 0x00);
    assert!(!irq(&core));
    let period = tick_until_irq(&mut core, 200_000);
    // The counter reloads with 20 and then counts down to 0, so 21 clocks, one per rendered
    // line, the pre-render line included. A line is 341/3 CPU cycles.
    assert!(
        (20 * 341 / 3..=22 * 341 / 3).contains(&period),
        "period {period}"
    );
}

#[test]
fn req_core_114_the_mmc3_counter_does_not_run_when_both_tables_share_a_half_of_chr() {
    // With the background and the sprites both fetching from $0xxx, A12 never rises during
    // rendering and the counter stands still (nesdev; mmc3_test "A12 clocking").
    let mut core = MapperRom::new(4).chr(32).load();
    core.write_memory(0x2000, 0x00);
    core.write_memory(0x2001, 0x18);
    core.mapper_write(0xC000, 1);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    assert_eq!(-1, tick_until_irq(&mut core, 120_000));
}

#[test]
fn req_core_114_writing_2006_clocks_the_mmc3_counter_by_hand() {
    // Rendering off: the counter can still be clocked by moving the VRAM address across
    // $1000, which is how mmc3_test drives it.
    let mut core = MapperRom::new(4).chr(32).load();
    core.write_memory(0x2001, 0x00);
    core.mapper_write(0xC000, 1);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    for _ in 0..2 {
        assert!(!irq(&core));
        core.write_memory(0x2006, 0x00);
        core.write_memory(0x2006, 0x00);
        for _ in 0..8 {
            core.bus_mut().tick();
        }

        core.write_memory(0x2006, 0x10);
        core.write_memory(0x2006, 0x00);
        for _ in 0..8 {
            core.bus_mut().tick();
        }
    }

    assert!(irq(&core));
}

#[test]
fn req_core_114_clearing_the_mmc3_counter_through_c001_does_not_fire() {
    // "The IRQ flag is not set when the counter is cleared by writing to $C001" (mmc3_test
    // readme); the reload happens on the next clock instead.
    let mut core = MapperRom::new(4).chr(32).load();
    core.write_memory(0x2001, 0x00);
    core.mapper_write(0xC000, 5);
    core.mapper_write(0xE001, 0x00);
    core.mapper_write(0xC001, 0x00);
    assert!(!irq(&core));
}

#[test]
fn req_core_100_the_mmc3_counter_is_silent_while_irqs_are_disabled() {
    let mut core = MapperRom::new(4).chr(32).load();
    enable_scanline_counting(&mut core);
    core.mapper_write(0xC000, 2);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE000, 0x00); // Disabled.
    assert_eq!(-1, tick_until_irq(&mut core, 60_000));
}

#[test]
fn req_core_100_the_rambo1_counter_can_run_on_the_cpu_clock() {
    let mut core = MapperRom::new(64).chr(32).load();
    core.write_memory(0x2001, 0x00); // No rendering: only the CPU clock is left.
    core.mapper_write(0xC000, 10);
    core.mapper_write(0xC001, 0x01); // Cycle mode, reload pending.
    core.mapper_write(0xE001, 0x00);
    let cycles = tick_until_irq(&mut core, 1000);
    // One clock every four CPU cycles and 11 clocks to go from the reload to zero, less the
    // few cycles the register writes themselves consumed.
    assert!((4 * 10..=4 * 11 + 8).contains(&cycles), "cycles {cycles}");
}

// ---------------------------------------------------------------- VRC4

fn write_vrc(core: &mut NesCore, mapper: usize, group: u16, index: u16, value: u8) {
    let offset = match mapper {
        21 => ((index & 1) << 1) | ((index & 2) << 1),
        22 | 25 => ((index & 1) << 1) | ((index & 2) >> 1),
        _ => index,
    };
    core.write_memory(group | offset, value);
}

#[test]
fn req_core_100_the_vrc4_counter_counts_up_on_the_cpu_clock() {
    for mapper in [21usize, 23, 25, 27] {
        let mut core = MapperRom::new(mapper).chr(32).load();
        write_vrc(&mut core, mapper, 0xF000, 0, 0x00); // Latch low nibble.
        write_vrc(&mut core, mapper, 0xF000, 1, 0x0F); // Latch high nibble: $F0.
        write_vrc(&mut core, mapper, 0xF000, 2, 0x06); // Enable, cycle mode.
        let cycles = tick_until_irq(&mut core, 1000);
        // $F0 counts up to $FF and fires on the sixteenth clock.
        assert_eq!(16, cycles, "mapper {mapper}");
        // Acknowledging clears the line and reloads from the latch.
        write_vrc(&mut core, mapper, 0xF000, 3, 0x00);
        assert!(!irq(&core));
        assert_eq!(-1, tick_until_irq(&mut core, 8));
    }
}

#[test]
fn req_core_100_the_vrc4_counter_can_run_on_a_scanline_prescaler() {
    let mut core = MapperRom::new(23).chr(32).load();
    core.write_memory(0xF000, 0x0F);
    core.write_memory(0xF001, 0x0F); // Latch $FF: one clock is enough.
    core.write_memory(0xF002, 0x02); // Enable, scanline mode.
    let cycles = tick_until_irq(&mut core, 1000);
    // The prescaler divides the CPU clock by 341/3, which is one scanline.
    assert!((113..=115).contains(&cycles), "cycles {cycles}");
}

#[test]
fn req_core_100_vrc2_has_no_irq_counter() {
    let mut core = MapperRom::new(22).chr(32).load();
    core.write_memory(0xF000, 0x0F);
    core.write_memory(0xF001, 0x0F);
    core.write_memory(0xF002, 0x06);
    assert_eq!(-1, tick_until_irq(&mut core, 2000));
}

// ---------------------------------------------------------------- FCG and SS88006

#[test]
fn req_core_100_the_bandai_fcg_counter_counts_down_on_the_cpu_clock() {
    let mut core = MapperRom::new(16).chr(32).load();
    core.write_memory(0x800B, 0x64); // Latch low byte: 100.
    core.write_memory(0x800C, 0x00);
    core.write_memory(0x800A, 0x01); // Enable, counter loaded from the latch.
    let cycles = tick_until_irq(&mut core, 1000);
    // 100 down to 0 is 100 cycles, and the underflow on the next one fires.
    assert_eq!(101, cycles);
    core.write_memory(0x800A, 0x00); // Acknowledge and disable.
    assert!(!irq(&core));
}

#[test]
fn req_core_100_the_ss88006_counter_can_be_narrowed_to_four_bits() {
    let mut core = MapperRom::new(18).chr(32).load();
    for nibble in 0..4u16 {
        core.write_memory(0xE000 + nibble, if nibble == 0 { 0x05 } else { 0x00 });
    }

    core.write_memory(0xF000, 0x00); // Copy the latch into the counter.
    core.write_memory(0xF001, 0x09); // Enable, four-bit counter.
    assert_eq!(6, tick_until_irq(&mut core, 200));
}

// ---------------------------------------------------------------- MMC5

/// MF-4c, finding MR-5-4 and Q-121: the board finds its lines on its own bus, so the **first**
/// detected line of a frame is the pre-render line, which fetches exactly like a visible one.
/// Visible line 0 is therefore the first line the counter counts, and `$5203` = 60 fires on
/// visible line **59** — except on the very first frame after power-on, where the chip's repeat
/// counter starts at 0 and the pre-render line's first read detects nothing, so that frame fires
/// one line later. Both are asserted, because the transient is what a test that runs one frame
/// would otherwise pin by accident.
#[test]
fn req_core_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names() {
    let mut core = MapperRom::new(5).chr(32).load();
    core.write_memory(0x2001, 0x18); // Rendering on: the board can see the lines.
    core.write_memory(0x5203, 60);
    core.write_memory(0x5204, 0x80); // Enable.
    assert!(tick_until_irq(&mut core, 200_000) > 0);
    // MR-5-3: the first detected line only sets the in-frame flag and zeroes the counter, so the
    // counter reaches 60 sixty detections later, not fifty-nine.
    assert_eq!(60, core.ppu().scanline());
    // Bit 7 of $5204 reports the pending IRQ and reading it acknowledges; bit 6 says the
    // scan is inside a frame.
    assert_eq!(0xC0, core.read_memory(0x5204) & 0xC0); // A peek leaves it pending.
    assert!(irq(&core));
    let status = core.cartridge_mut().unwrap().try_cpu_read(0x5204).unwrap();
    assert_eq!(0xC0, status & 0xC0);
    assert!(!irq(&core));

    // In steady state the pre-render line is detected first, so the same target lands one line
    // earlier, frame after frame.
    for _ in 0..4 {
        assert!(tick_until_irq(&mut core, 200_000) > 0);
        assert_eq!(59, core.ppu().scanline());
        core.cartridge_mut().unwrap().try_cpu_read(0x5204);
    }
}

#[test]
fn req_core_100_the_mmc5_scanline_irq_stays_quiet_without_rendering() {
    let mut core = MapperRom::new(5).chr(32).load();
    core.write_memory(0x2001, 0x00);
    core.write_memory(0x5203, 60);
    core.write_memory(0x5204, 0x80);
    assert_eq!(-1, tick_until_irq(&mut core, 120_000));
}

// ---------------------------------------------------------------- Sunsoft FME-7 (69)

/// Invokes FME-7 command `command` with `parameter` (nesdev, "Sunsoft FME-7": the command
/// register is $8000-$9FFF, and writing the parameter at $A000-$BFFF "invokes the command").
fn fme7(core: &mut NesCore, command: u8, parameter: u8) {
    core.write_memory(0x8000, command);
    core.write_memory(0xA000, parameter);
}

/// "When the IRQ counter is decremented from $0000 to $FFFF an IRQ is generated", the counter
/// running off M2 — one step per CPU cycle — while command $D's `C` bit allows it
/// (REQ-CORE-100).
#[test]
fn req_core_100_the_fme7_counter_counts_down_on_the_cpu_clock() {
    let mut core = MapperRom::new(69).chr(32).load();
    fme7(&mut core, 0x0E, 0x40); // Counter low.
    fme7(&mut core, 0x0F, 0x00); // Counter high: $0040.
    fme7(&mut core, 0x0D, 0x81); // `C` and `T`.

    let cycles = tick_until_irq(&mut core, 1000);
    // $0040 steps to reach $0000 and one more to wrap to $FFFF, less the cycles the four
    // register writes above consumed between the counter being set and the counting starting.
    assert!((0x30..=0x41).contains(&cycles), "cycles {cycles}");
}

/// "Writes to the IRQ counter registers directly set the lower or upper eight bits of the
/// counter": commands $E and $F are halves of one 16-bit counter, not a latch and a reload.
#[test]
fn req_core_100_the_fme7_counter_is_sixteen_bits_set_half_at_a_time() {
    let mut core = MapperRom::new(69).chr(32).load();
    fme7(&mut core, 0x0E, 0x00);
    fme7(&mut core, 0x0F, 0x01); // $0100: far more than the low byte alone would give.
    fme7(&mut core, 0x0D, 0x81);

    let cycles = tick_until_irq(&mut core, 2000);
    assert!((0xF0..=0x101).contains(&cycles), "cycles {cycles}");
}

/// Command $D is `C... ...T`: `T` enables the IRQ and `C` the decrement, and the two are
/// independent. The wrap is not latched into a flag a later `T` could reveal.
#[test]
fn req_core_100_the_fme7_needs_both_command_d_bits() {
    // `C` without `T`: the counter runs and wraps, the line stays low.
    let mut counting = MapperRom::new(69).chr(32).load();
    fme7(&mut counting, 0x0E, 0x08);
    fme7(&mut counting, 0x0D, 0x80);
    assert_eq!(-1, tick_until_irq(&mut counting, 2000));
    // And enabling `T` afterwards does not reveal the wrap that already happened.
    fme7(&mut counting, 0x0D, 0x81);
    assert!(!irq(&counting));

    // `T` without `C`: the counter does not move, so it never reaches the wrap.
    let mut frozen = MapperRom::new(69).chr(32).load();
    fme7(&mut frozen, 0x0E, 0x08);
    fme7(&mut frozen, 0x0D, 0x01);
    assert_eq!(-1, tick_until_irq(&mut frozen, 2000));
}

/// "All writes to this register acknowledge an active IRQ" — command $D and nothing else.
#[test]
fn req_core_100_a_write_to_fme7_command_d_acknowledges_the_irq() {
    let mut core = MapperRom::new(69).chr(32).load();
    fme7(&mut core, 0x0E, 0x08);
    fme7(&mut core, 0x0D, 0x81);
    assert!(tick_until_irq(&mut core, 1000) > 0);

    // Another command does not clear it.
    fme7(&mut core, 0x0C, 0x00);
    assert!(irq(&core));

    // Command $D does, even when it leaves both bits set.
    fme7(&mut core, 0x0D, 0x81);
    assert!(!irq(&core));
}

// ---------------------------------------------------------------- the Acclaim MC-ACC (4.3)

/// Moves the PPU's VRAM address across $1000 and back, with cycles on either side, which is one
/// complete A12 cycle — a rise and a fall — driven by hand the way `mmc3_test` does.
fn toggle_a12(core: &mut NesCore) {
    core.write_memory(0x2006, 0x10);
    core.write_memory(0x2006, 0x00);
    for _ in 0..8 {
        core.bus_mut().tick();
    }

    core.write_memory(0x2006, 0x00);
    core.write_memory(0x2006, 0x00);
    for _ in 0..8 {
        core.bus_mut().tick();
    }
}

/// A mapper-4 image on NES 2.0 submapper 3, with its IRQ armed for **two** clocks — a latch of
/// 1, so the first clock reloads and the second lands on zero — and rendering off, so that only
/// the hand-driven A12 moves the counter.
///
/// Two clocks and not one, because the prescaler clocks on the **first** fall of each group of
/// eight: "Counter clocking happens once per 8 A12 cycles at first cycle" (MR-4-3, after
/// krikzz's research, which is also what Mesen 2 implements). One clock would fire on the first
/// fall on both boards and measure nothing.
fn mc_acc_armed(submapper: u8) -> NesCore {
    let mut core = MapperRom::new(4).chr(32).submapper(submapper).load();
    core.write_memory(0x2001, 0x00);
    core.mapper_write(0xC000, 1);
    core.mapper_write(0xC001, 0x00); // Reload on the next clock.
    core.mapper_write(0xE001, 0x00); // Enable.
    core
}

/// MR-4-3, nesdev ("MMC3"): "Acclaim's MC-ACC chip is their own variant of the MMC3 ... The only
/// known difference is that the scanline counter uses a ÷8 prescaler instead of filtering A12,
/// clocked by falling edges." So eight A12 cycles are needed where the MMC3 needs one
/// (REQ-CORE-100, *NES 2.0 submappers* "Submapper 3: MC-ACC chip variant").
#[test]
fn mr_4_3_the_mc_acc_clocks_once_per_eight_a12_falls() {
    let mut core = mc_acc_armed(3);

    // Fall 1 is the first of its group, so it clocks — and reloads the counter to the latch.
    // Falls 2 to 8 are swallowed by the prescaler.
    for fall in 1..9 {
        toggle_a12(&mut core);
        assert!(!irq(&core), "fired after {fall} falls");
    }

    // Fall 9 opens the next group and clocks the counter to zero.
    toggle_a12(&mut core);
    assert!(irq(&core), "the ninth fall must clock the counter again");
}

/// And a plain mapper-4 image — no submapper, or any submapper but 3 — is an MMC3: one filtered
/// A12 **rise** clocks it, so the very first toggle fires.
#[test]
fn mr_4_3_a_plain_mapper_4_image_is_not_an_mc_acc() {
    let mut core = MapperRom::new(4).chr(32).load();
    core.write_memory(0x2001, 0x00);
    core.mapper_write(0xC000, 1);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);

    // Every filtered rise clocks it, so the two clocks the latch needs are two toggles.
    toggle_a12(&mut core);
    assert!(!irq(&core));
    toggle_a12(&mut core);
    assert!(irq(&core), "an MMC3 clocks on every filtered rise");
    assert_eq!(4, core.cartridge().unwrap().mapper_number());

    // The same two toggles leave an MC-ACC silent: it is seven falls short of its second clock.
    let mut acc = mc_acc_armed(3);
    toggle_a12(&mut acc);
    toggle_a12(&mut acc);
    assert!(!irq(&acc));

    // And an MC-ACC still says it is mapper 4: the submapper is the only thing that tells them
    // apart, and REQ-CORE-011's message names the number.
    let acc = mc_acc_armed(3);
    assert_eq!(4, acc.cartridge().unwrap().mapper_number());
}

/// MR-4-3: on an MC-ACC "writing to $C001 resets pulse counter", which is how a game puts the
/// prescaler back in phase. Seven falls then a `$C001` write means eight more are needed.
#[test]
fn mr_4_3_writing_c001_resets_the_mc_acc_prescaler() {
    let mut core = mc_acc_armed(3);
    // Five falls: one clock, then four swallowed. Three more would reach the next group.
    for _ in 0..5 {
        toggle_a12(&mut core);
    }

    // The write puts the prescaler back to zero, so the next fall opens a fresh group and eight
    // more are needed after it.
    core.mapper_write(0xC001, 0x00);
    for fall in 1..9 {
        toggle_a12(&mut core);
        assert!(!irq(&core), "fired {fall} falls after the reset");
    }

    toggle_a12(&mut core);
    assert!(irq(&core));
}

/// The prescaler is part of the board's state, so a savestate taken mid-group restores it — and
/// a mapper-4 state is **byte-identical** to one written before this lot, the prescaler byte
/// being written only by the clone (REQ-CORE-090, REQ-CORE-091).
#[test]
fn req_core_090_the_mc_acc_prescaler_survives_a_save_and_restore() {
    let mut core = mc_acc_armed(3);
    // One clock then four swallowed falls: the prescaler holds 3.
    for _ in 0..5 {
        toggle_a12(&mut core);
    }

    let state = core.save_state(true).unwrap();
    for fall in 1..4 {
        toggle_a12(&mut core);
        assert!(!irq(&core), "fired {fall} falls after the save");
    }

    toggle_a12(&mut core);
    assert!(irq(&core), "the fourth fall opens the next group");

    core.try_load_state(&state, 0).unwrap();
    assert!(!irq(&core));
    for fall in 1..4 {
        toggle_a12(&mut core);
        assert!(!irq(&core), "fired {fall} falls after the restore");
    }

    toggle_a12(&mut core);
    assert!(
        irq(&core),
        "the restored prescaler still had three falls to go"
    );
}

// ---------------------------------------------------------------- VRC6

/// The VRC6 reaches the same three IRQ registers as the VRC4 at fixed addresses — $F000 the
/// latch, $F001 the control, $F002 the acknowledge — with mapper 26 swapping A0 and A1, so its
/// control and acknowledge are the other way round.
fn write_vrc6_irq(core: &mut NesCore, mapper: usize, index: u16, value: u8) {
    let offset = if mapper == 26 {
        ((index & 0x01) << 1) | ((index & 0x02) >> 1)
    } else {
        index
    };
    core.write_memory(0xF000 | offset, value);
}

#[test]
fn req_core_100_the_vrc6_counter_counts_up_on_the_cpu_clock() {
    for mapper in [24usize, 26] {
        let mut core = MapperRom::new(mapper).chr(32).load();
        // The VRC6 writes the whole latch at once, where a VRC4 needs two nibbles.
        write_vrc6_irq(&mut core, mapper, 0, 0xF0);
        // Enable, cycle mode. $F0 then counts up to $FF and fires on the sixteenth clock,
        // exactly as the VRC4 does.
        write_vrc6_irq(&mut core, mapper, 1, 0x06);
        assert_eq!(16, tick_until_irq(&mut core, 1000), "mapper {mapper}");

        // "writing any value to this register will acknowledge the pending IRQ, and copy the A
        // bit to the E bit" — A was clear, so the counter stops.
        write_vrc6_irq(&mut core, mapper, 2, 0x00);
        assert!(!irq(&core), "mapper {mapper}");
        assert_eq!(-1, tick_until_irq(&mut core, 300), "mapper {mapper}");
    }
}

#[test]
fn req_core_100_the_vrc6_counter_runs_on_the_scanline_prescaler_and_keeps_a_set() {
    let mut core = MapperRom::new(24).chr(32).load();
    // Latch $FF, so one clock is enough; then enable, scanline mode, with A set. The prescaler
    // divides the CPU clock by 341/3, which is one scanline.
    core.write_memory(0xF000, 0xFF);
    core.write_memory(0xF001, 0x03);
    let cycles = tick_until_irq(&mut core, 1000);
    assert!((113..=115).contains(&cycles), "cycles {cycles}");

    // A was set, so the acknowledge leaves the counter enabled and it fires again.
    core.write_memory(0xF002, 0x00);
    assert!(!irq(&core));
    let again = tick_until_irq(&mut core, 1000);
    assert!((113..=115).contains(&again), "cycles {again}");
}

#[test]
fn mf_5a_the_vrc6_and_the_vrc4_share_one_irq_counter() {
    // Lot MF-5a step 1 extracted `VrcIrq` out of `vrc2_vrc4.rs` instead of copying it. This is
    // the check that the two boards really are driving the same counter: given the same latch,
    // the same mode and the same number of ticks, a mapper-23 VRC4 and a mapper-24 VRC6 must
    // fire on the same cycle, through their two different register decodes.
    let mut vrc4 = MapperRom::new(23).chr(32).load();
    vrc4.write_memory(0xF000, 0x0A); // Latch low nibble.
    vrc4.write_memory(0xF001, 0x0C); // Latch high nibble: $CA.
    vrc4.write_memory(0xF002, 0x06); // Enable, cycle mode.

    let mut vrc6 = MapperRom::new(24).chr(32).load();
    vrc6.write_memory(0xF000, 0xCA); // The same latch, written whole.
    vrc6.write_memory(0xF001, 0x06);

    let expected = 0x100 - 0xCA;
    assert_eq!(expected, tick_until_irq(&mut vrc4, 1000));
    assert_eq!(expected, tick_until_irq(&mut vrc6, 1000));
}

// ---------------------------------------------------------------- VRC3 (73)

/// The VRC3's reload value arrives as **four** nibbles, at $8000, $9000, $A000 and $B000
/// (nesdev, "VRC3"), where a VRC4 has two and a VRC6 writes the byte whole.
fn write_vrc3_latch(core: &mut NesCore, latch: u16) {
    for nibble in 0..4u16 {
        let value = ((latch >> (nibble * 4)) & 0x0F) as u8;
        core.write_memory(0x8000 + nibble * 0x1000, value);
    }
}

#[test]
fn req_core_100_the_vrc3_counter_is_sixteen_bits_wide_and_counts_cpu_cycles_up() {
    // nesdev ("VRC3"): "The counter is 16 bits instead of 8 bits, and there is no scanline mode,
    // only CPU cycle mode"; it "is incremented each CPU cycle", and "when it overflows from
    // $FFFF, an IRQ is generated, and the counter is reloaded with the 16-bit latch value".
    let mut core = MapperRom::new(73).chr(32).load();
    write_vrc3_latch(&mut core, 0x01F0);
    // `.... .MEA`: E alone, so the counter is reloaded and counts all sixteen bits.
    core.write_memory(0xC000, 0x02);
    assert_eq!(0x1_0000 - 0x01F0, tick_until_irq(&mut core, 70_000));

    // "writing any value to this register will acknowledge the pending IRQ, and copy the A bit
    // to the E bit" — A was clear, so the counter stops.
    core.write_memory(0xD000, 0x00);
    assert!(!irq(&core));
    assert_eq!(-1, tick_until_irq(&mut core, 300));
}

#[test]
fn req_core_100_the_vrc3_eight_bit_mode_wraps_at_ff_and_leaves_bits_8_to_15_alone() {
    // nesdev ("VRC3"), the `M` bit: in 8-bit mode "IRQs are generated when the lower 8 bits
    // overflow from $FF". The same latch that takes 65 040 cycles above takes 16 here, which is
    // what says the mode is a counter **width** and not the VRC4's prescaler bypass.
    let mut core = MapperRom::new(73).chr(32).load();
    write_vrc3_latch(&mut core, 0x01F0);
    core.write_memory(0xC000, 0x07); // A, E and M.
    assert_eq!(0x100 - 0xF0, tick_until_irq(&mut core, 1000));

    // A is set, so the acknowledge leaves it running, and the low byte reloaded from the low
    // byte of the latch.
    core.write_memory(0xD000, 0x00);
    assert!(!irq(&core));
    assert_eq!(0x100 - 0xF0, tick_until_irq(&mut core, 1000));

    // Back to sixteen bits with E **clear**, so nothing is reloaded, and re-enabled by the
    // acknowledge: what is left to count is $10000 - $01F0 and not $10000 - $00F0, so bits 8-15
    // are still the $01 the first reload put there and no wrap of the low byte has touched them.
    core.write_memory(0xC000, 0x01);
    core.write_memory(0xD000, 0x00);
    assert_eq!(0x1_0000 - 0x01F0, tick_until_irq(&mut core, 70_000));
}

#[test]
fn req_core_100_the_vrc3_enable_without_a_reload_leaves_the_counter_armed() {
    // The case `req_core_100_the_vrc6_counter_*` pins for the VRC6, on this chip: a control
    // write with E clear reloads nothing and clocks nothing, and the acknowledge that copies a
    // set A into E starts the counter from wherever it was — here the $0000 it powered on with,
    // which is $100 8-bit clocks from its wrap.
    let mut core = MapperRom::new(73).chr(32).load();
    write_vrc3_latch(&mut core, 0xFFF0);
    core.write_memory(0xC000, 0x05); // A and M, E clear.
    assert_eq!(-1, tick_until_irq(&mut core, 300));

    core.write_memory(0xD000, 0x00);
    assert_eq!(0x100, tick_until_irq(&mut core, 1000));
}

#[test]
fn mf_6b_the_vrc3_and_the_vrc4_fire_on_the_same_cycle() {
    // MF-6b step 1 built the VRC3's counter as a width parameter on `VrcIrq` first — the shape
    // MR-4-3 gave the MC-ACC — and measured it: the extra latch byte moves `pending` inside
    // `Vrc2Vrc4`, `Vrc6` and `Vrc7`, those offsets are operands in the per-board blocks
    // `Board::irq_asserted` becomes inside `Cpu6502::branch`, and eleven of lot PR-1's thirteen
    // ROMs paid for it. So `Vrc3Irq` stands beside `VrcIrq` instead, and what the two chips do
    // share — the `.... .MEA` control register, the reload on `E`, the acknowledge that copies
    // `A` into `E`, and an up-counter that fires on the step out of its top value — is what this
    // test pins, in the shape `mf_5a_the_vrc6_and_the_vrc4_share_one_irq_counter` gave it: given
    // the same latch and the same 8-bit counting, through two register decodes with nothing in
    // common, a mapper-73 board and a mapper-23 VRC4 must fire on the same cycle.
    let mut vrc4 = MapperRom::new(23).chr(32).load();
    vrc4.write_memory(0xF000, 0x0A); // Latch low nibble.
    vrc4.write_memory(0xF001, 0x0C); // Latch high nibble: $CA.
    vrc4.write_memory(0xF002, 0x06); // Enable, cycle mode.

    let mut vrc3 = MapperRom::new(73).chr(32).load();
    write_vrc3_latch(&mut vrc3, 0x00CA);
    vrc3.write_memory(0xC000, 0x06); // Enable, 8-bit mode.

    let expected = 0x100 - 0xCA;
    assert_eq!(expected, tick_until_irq(&mut vrc4, 1000));
    assert_eq!(expected, tick_until_irq(&mut vrc3, 1000));
}

// ---------------------------------------------------------------- Irem H3001 (65)

/// nesdev ("INES Mapper 065"): "There's a 16-bit internal down counter which (when enabled),
/// decrements by 1 every CPU cycle. When the counter reaches 0, an IRQ is fired."
///
/// The reload value is written high byte first — "note that $9005 is the HIGH bits, not the low
/// bits" — so $9005 = $01 with $9006 = $F0 is 496 cycles and not the 61 441 the two registers
/// taken the other way round would give.
#[test]
fn req_core_100_the_h3001_counter_counts_down_one_per_cpu_cycle_and_fires_at_zero() {
    let mut core = MapperRom::new(65).chr(32).load();
    core.write_memory(0x9005, 0x01); // High byte of the reload value.
    core.write_memory(0x9006, 0xF0); // Low byte: $01F0.
    core.write_memory(0x9003, 0x80); // E: enable.
    core.write_memory(0x9004, 0x00); // Copy the reload value into the counter.
    assert_eq!(0x01F0, tick_until_irq(&mut core, 70_000));

    // "Any write to $9003 or $9004 will acknowledge the pending IRQ" — this one keeps E set.
    core.write_memory(0x9003, 0x80);
    assert!(!irq(&core));
}

/// The half of the page an implementation drops: "The counter stops at 0 -- it does not wrap and
/// isn't automatically reloaded", and "$9006 and $9005 set the reload value, but do not have any
/// effect on the actual counter".
#[test]
fn req_core_100_the_h3001_counter_stops_at_zero_and_only_9004_reloads_it() {
    let mut core = MapperRom::new(65).chr(32).load();
    core.write_memory(0x9005, 0x00);
    core.write_memory(0x9006, 0x20); // $0020.
    core.write_memory(0x9003, 0x80);
    core.write_memory(0x9004, 0x00);
    assert_eq!(0x20, tick_until_irq(&mut core, 1000));

    // Acknowledged with E still set, the counter is at zero and stopped: it does not wrap round
    // to $FFFF and come back 65 536 cycles later.
    core.write_memory(0x9003, 0x80);
    assert_eq!(-1, tick_until_irq(&mut core, 70_000));

    // A new reload value on its own changes nothing...
    core.write_memory(0x9005, 0x00);
    core.write_memory(0x9006, 0x40);
    assert_eq!(-1, tick_until_irq(&mut core, 1000));

    // ...and $9004 is what arms the counter again, with the value those two registers hold.
    core.write_memory(0x9004, 0x00);
    assert_eq!(0x40, tick_until_irq(&mut core, 1000));
}

/// The case `req_core_100_the_vrc6_counter_*` pins for the VRC6, on this chip: `$9003` carries no
/// reload of any kind, so a counter loaded while the enable bit was clear is still holding its
/// count when the enable arrives — and the cycles that passed in between did not move it.
#[test]
fn req_core_100_the_h3001_enable_without_a_reload_leaves_the_counter_armed() {
    let mut core = MapperRom::new(65).chr(32).load();
    core.write_memory(0x9005, 0x00);
    core.write_memory(0x9006, 0x64); // 100.
    core.write_memory(0x9003, 0x00); // E clear.
    core.write_memory(0x9004, 0x00); // Loaded, but not counting.
    assert_eq!(-1, tick_until_irq(&mut core, 50));

    core.write_memory(0x9003, 0x80);
    assert_eq!(100, tick_until_irq(&mut core, 1000));
}

// ---------------------------------------------------------------- Sunsoft-3 (67)

/// nesdev ("INES Mapper 067"), the `$C800` register: "Write the high then low byte of a 16-bit
/// CPU cycle count, much like PPUADDR. This directly affects the current count, not a reload
/// value." The count then "decreases by 1 every CPU cycle" and the IRQ comes on the wrap, so a
/// count of $01F0 fires on cycle 497 and not 496.
#[test]
fn req_core_100_the_sunsoft3_count_is_written_high_byte_first_and_fires_on_the_wrap() {
    let mut core = MapperRom::new(67).chr(32).load();
    core.write_memory(0xD800, 0x10); // P: count, and the $C800 toggle reset to "high next".
    core.write_memory(0xC800, 0x01); // High byte.
    core.write_memory(0xC800, 0xF0); // Low byte: $01F0.
    assert_eq!(0x01F0 + 1, tick_until_irq(&mut core, 70_000));
}

/// Two claims of the same paragraph: "the mapper asserts an IRQ and pauses itself", and — the
/// page's own correction of Disch, which Mesen 2 still has — "despite previous notes, writes to
/// this register do not acknowledge the IRQ".
#[test]
fn req_core_100_the_sunsoft3_pauses_itself_on_the_wrap_and_d800_does_not_acknowledge() {
    let mut core = MapperRom::new(67).chr(32).load();
    core.write_memory(0xD800, 0x10);
    core.write_memory(0xC800, 0x00);
    core.write_memory(0xC800, 0x10); // 16.
    assert_eq!(17, tick_until_irq(&mut core, 1000));

    // Paused: the count rolled on to $FFFF but nothing clocks it any more.
    for _ in 0..1000 {
        core.bus_mut().tick();
    }

    // And neither a pause nor a re-enable through $D800 clears the pending IRQ.
    core.write_memory(0xD800, 0x00);
    assert!(irq(&core), "$D800 with P clear acknowledged");
    core.write_memory(0xD800, 0x10);
    assert!(irq(&core), "$D800 with P set acknowledged");

    // The acknowledge is $8000's, and the count resumed from $FFFF, so what is left to it is a
    // whole sixteen bits: $10000 cycles to the next wrap.
    core.write_memory(0x8000, 0x00);
    assert!(!irq(&core));
    assert_eq!(0x1_0000, tick_until_irq(&mut core, 70_000));
}

/// "Interrupt Acknowledge ($8000). Mask: $8800. Any write to this address or any of its mirrors
/// acknowledges a pending IRQ." The mask decodes A15 and A11 and nothing else, so **every**
/// A11-clear page above $8000 is that register — which is also what keeps it apart from the seven
/// `$F800` registers, all of which have A11 set.
#[test]
fn req_core_100_the_sunsoft3_acknowledges_on_every_a11_clear_mirror_of_8000() {
    for address in [
        0x8000u16, 0x87FF, 0x9000, 0xA123, 0xC000, 0xD000, 0xE000, 0xF000,
    ] {
        let mut core = MapperRom::new(67).chr(32).load();
        core.write_memory(0xD800, 0x10);
        core.write_memory(0xC800, 0x00);
        core.write_memory(0xC800, 0x04);
        assert_eq!(5, tick_until_irq(&mut core, 1000), "{address:04X}");

        core.write_memory(address, 0x00);
        assert!(!irq(&core), "a write to {address:04X} did not acknowledge");
    }
}

/// The `$C800` toggle, and what resets it: "in addition to enabling/disabling counting, any write
/// to $D800 will reset the toggle so that the next write to $C800 will be the first write."
#[test]
fn req_core_100_a_d800_write_puts_the_sunsoft3_c800_toggle_back_to_the_high_byte() {
    let mut core = MapperRom::new(67).chr(32).load();
    // One write, so the toggle is left expecting the low byte...
    core.write_memory(0xC800, 0x00);
    // ...and $D800 takes it back to the high byte, which this $01 therefore becomes.
    core.write_memory(0xD800, 0x10);
    core.write_memory(0xC800, 0x01);
    core.write_memory(0xC800, 0xF0);
    assert_eq!(0x01F0 + 1, tick_until_irq(&mut core, 70_000));
}

/// The one test of this step that exists to keep the two boards apart, which §5 of
/// `docs/review/MF-6-inventory.md` asks for in as many words: 65 and 67 are "the same mechanism
/// with different wrap rules: 65 fires at zero, 67 on the wrap *past* zero, and a test that
/// drives both to zero in the same assertion is what keeps the two from being conflated".
///
/// Both counters are given the same count and both are clocked once per CPU cycle, so the H3001
/// fires on cycle N and the Sunsoft-3 one cycle later — and the H3001 then **stops** where the
/// Sunsoft-3 **pauses**, which is the other half of the difference.
#[test]
fn mf_6b_mapper_65_fires_at_zero_and_mapper_67_one_cycle_past_it() {
    const COUNT: i64 = 0x0123;

    let mut h3001 = MapperRom::new(65).chr(32).load();
    h3001.write_memory(0x9005, 0x01);
    h3001.write_memory(0x9006, 0x23);
    h3001.write_memory(0x9003, 0x80);
    h3001.write_memory(0x9004, 0x00);

    let mut sunsoft3 = MapperRom::new(67).chr(32).load();
    sunsoft3.write_memory(0xD800, 0x10);
    sunsoft3.write_memory(0xC800, 0x01);
    sunsoft3.write_memory(0xC800, 0x23);

    assert_eq!(COUNT, tick_until_irq(&mut h3001, 70_000));
    assert_eq!(COUNT + 1, tick_until_irq(&mut sunsoft3, 70_000));

    // Acknowledged and left enabled, the H3001's counter is stopped at zero for good; the
    // Sunsoft-3's is paused at $FFFF and a `P` write starts it again.
    h3001.write_memory(0x9003, 0x80);
    assert_eq!(-1, tick_until_irq(&mut h3001, 70_000));

    sunsoft3.write_memory(0x8000, 0x00);
    sunsoft3.write_memory(0xD800, 0x10);
    assert_eq!(0x1_0000, tick_until_irq(&mut sunsoft3, 70_000));
}

// ---------------------------------------------------------------- NES-EVENT (105)

/// The value the NES-EVENT's 30-bit counter must reach, with the DIP switches the board ships
/// with here: nesdev ("NES-EVENT"), "in the official tournament, `C` was closed, and the others
/// were open, so the counter had to reach $2800000" — $28000000, the page having dropped a digit,
/// which both its own formula (`[1D CBAx …]`, bit 29 set and `C` at bit 27) and the arithmetic
/// confirm: `0x2800_0000` M2 cycles at 1.789773 MHz is **6 min 15 s**, and the tournament's limit
/// was 6 min 21 s.
#[cfg(feature = "board-105")]
const NES_EVENT_TARGET: u32 = 0x2800_0000;

#[cfg(feature = "board-105")]
/// A mapper-105 core whose counter is already at `counter`, with the timer running.
///
/// **Why the state block and not 671 million bus ticks.** Nothing on this board writes the
/// counter — the DIP switches set the value it must *reach*, and `$A000`'s `I` bit only ever
/// resets it to zero — so the only way to observe the comparator at unit-test speed is to put
/// the counter there through a savestate. The offset is not hard-coded: the counter is run for a
/// distinctive number of cycles first and that number is then **found** in the block, which
/// fails loudly if it is not there exactly once. So this helper also pins that the counter is
/// part of the state at all (REQ-CORE-090, REQ-CORE-091).
fn nes_event_at_counter(counter: u32) -> NesCore {
    const MARKER: u32 = 0x0001_2345;

    let mut core = MapperRom::new(105).prg(32).chr(0).load();
    // `I` clear starts the timer. `mmc1_write` ticks the bus twice per serial bit, so the
    // counter only starts moving once the fifth write commits.
    core.mmc1_write(0xA000, 0x00);
    for _ in 0..MARKER {
        core.bus_mut().tick();
    }

    let mut state = core.save_state(true).unwrap();
    let needle = MARKER.to_le_bytes();
    let found: Vec<usize> = state
        .windows(4)
        .enumerate()
        .filter(|(_, window)| *window == needle)
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        1,
        found.len(),
        "the counter should appear exactly once in the state block, found {found:?}"
    );

    state[found[0]..found[0] + 4].copy_from_slice(&counter.to_le_bytes());
    let mut restored = NesCore::new();
    restored.try_load_state(&state, 0).unwrap();
    restored
}

#[cfg(feature = "board-105")]
#[test]
fn req_core_100_the_nes_event_counter_fires_at_its_dip_switch_target() {
    // nesdev ("NES-EVENT"): "a 30-bit up counter clocked by M2 that fires an IRQ when it reaches
    // a high enough value", and Disch's "the IRQ counter is 30 bits wide.. when it reaches the
    // following value, an IRQ is fired". Two cycles short of the target, the IRQ is two cycles
    // away — which pins the value exactly, not approximately.
    let mut core = nes_event_at_counter(NES_EVENT_TARGET - 2);
    assert!(!irq(&core));
    assert_eq!(2, tick_until_irq(&mut core, 10));
}

#[cfg(feature = "board-105")]
#[test]
fn req_core_100_the_nes_event_timer_bit_resets_the_counter_and_acknowledges() {
    // Disch: "when cleared, the IRQ counter counts up every cycle. When set, the IRQ counter is
    // reset to 0 and stays there (does not count), and the pending IRQ is acknowledged."
    let mut core = nes_event_at_counter(NES_EVENT_TARGET - 2);
    assert_eq!(2, tick_until_irq(&mut core, 10));

    core.mmc1_write(0xA000, 0x10); // `I` set.
    assert!(!irq(&core), "the timer bit did not acknowledge");
    assert_eq!(-1, tick_until_irq(&mut core, 2000), "it counted while held");

    // `I` clear again starts it from **zero**, not from where it was: a whole 0x28000000 cycles
    // away, so nothing comes within two thousand.
    core.mmc1_write(0xA000, 0x00);
    assert_eq!(
        -1,
        tick_until_irq(&mut core, 2000),
        "the counter resumed near the target instead of restarting"
    );
}

#[cfg(feature = "board-105")]
#[test]
fn req_core_100_only_the_nes_event_timer_bit_touches_its_counter() {
    // The case `req_core_100_the_vrc6_counter_*` pins for the VRC6 — an enable that is not a
    // reload — read onto a board where the enable and the reset are the same wire: what is left
    // to check is that the **other three** registers do not reset the counter. A hundred cycles
    // short of the target, two serial writes (ten bus ticks each) leave eighty to go.
    let mut core = nes_event_at_counter(NES_EVENT_TARGET - 100);
    core.mmc1_write(0xE000, 0x00);
    core.mmc1_write(0x8000, 0x0C);
    assert_eq!(80, tick_until_irq(&mut core, 1000));
}

// ---------------------------------------------------------------- Taito TC0690 (48)

/// The TC0690's four IRQ registers are the MMC3's in a different order: `$C000` the reload
/// value, `$C001` the clear, `$C002` the enable, `$C003` the acknowledge — and the value written
/// to `$C000` is **inverted**, "you can simply XOR the writes with $FF and it will work just like
/// MMC3" (nesdev, "INES Mapper 048").
fn arm_tc0690(core: &mut NesCore, latch: u8) {
    core.mapper_write(0xC000, latch ^ 0xFF);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xC002, 0x00);
}

/// Loads `v` through `$2006`, which drives A12 off the PPU's address bus with /RD high.
fn set_vram_address(core: &mut NesCore, address: u16) {
    core.write_memory(0x2006, (address >> 8) as u8);
    core.write_memory(0x2006, (address & 0xFF) as u8);
}

#[test]
fn req_core_100_the_tc0690_reload_value_is_inverted() {
    // The first of the page's two exceptions: "writing $06 to the reload reg on this mapper
    // would be like writing $F9 on MMC3". So the same twenty-line period an MMC3 gets from $14
    // needs $EB here — and $14 written straight through would give 236 lines instead of 21.
    let mut core = MapperRom::new(48).chr(32).load();
    enable_scanline_counting(&mut core);
    arm_tc0690(&mut core, 20);
    assert!(tick_until_irq(&mut core, 200_000) > 0);
    core.mapper_write(0xC003, 0x00);
    core.mapper_write(0xC002, 0x00);
    assert!(!irq(&core));
    let period = tick_until_irq(&mut core, 200_000);
    assert!(
        (20 * 341 / 3..=22 * 341 / 3).contains(&period),
        "period {period}"
    );

    // $14 written **without** the inversion is a latch of 235, which is an order of magnitude
    // more lines: the one figure that says which way round the XOR goes. The period is not
    // pinned exactly here because 236 clocks cross a vblank, during which A12 does not rise at
    // all, and where in the frame that falls depends on when the acknowledge landed.
    let mut raw = MapperRom::new(48).chr(32).load();
    enable_scanline_counting(&mut raw);
    raw.mapper_write(0xC000, 20);
    raw.mapper_write(0xC001, 0x00);
    raw.mapper_write(0xC002, 0x00);
    let long = tick_until_irq(&mut raw, 300_000);
    assert!(long > 10 * period, "period {long} against {period}");
}

#[test]
fn mf_6b_the_tc0690_counts_the_same_lines_as_the_mmc3_and_trips_four_cycles_later() {
    // The second exception, and what it is measured against: "the IRQ seems to trip a little
    // later than it does on MMC3. It looks like about a 4 CPU cycle delay from the normal MMC3
    // IRQ time." Two cores, the same ROM, the same PPU state and the same latch through two
    // different register sets — so the difference between the two cycle counts is the delay and
    // nothing else.
    //
    // This is also the step's `ppu_irq_horizon` test, the one §5 calls "the one test in the lot
    // whose failure is a scheduling bug rather than a mapper bug": the PPU runs behind the CPU
    // by whatever that horizon allows, so a horizon too long on this board would show here as a
    // difference that is not four.
    let mut mmc3 = MapperRom::new(4).chr(32).load();
    enable_scanline_counting(&mut mmc3);
    mmc3.mapper_write(0xC000, 20);
    mmc3.mapper_write(0xC001, 0x00);
    mmc3.mapper_write(0xE001, 0x00);

    let mut tc0690 = MapperRom::new(48).chr(32).load();
    enable_scanline_counting(&mut tc0690);
    arm_tc0690(&mut tc0690, 20);

    let on_the_mmc3 = tick_until_irq(&mut mmc3, 200_000);
    assert!(on_the_mmc3 > 0);
    assert_eq!(on_the_mmc3 + 4, tick_until_irq(&mut tc0690, 200_000));
}

#[test]
fn req_core_100_the_tc0690_irq_registers_are_the_mmc3_s_at_c000_to_c003() {
    let mut core = MapperRom::new(48).chr(32).load();
    enable_scanline_counting(&mut core);
    arm_tc0690(&mut core, 2);
    assert!(tick_until_irq(&mut core, 200_000) > 0);

    // **Neither `$C000` nor `$C001` acknowledges.** The page's two exceptions do not include
    // that and an MMC3's own latch and clear do not either; Mesen 2 clears the source in both
    // (`TaitoTc0690.h:60`, `:67`) with the comment "Flintstones expects either $C000 or $C001 to
    // clear the irq flag", which is the one divergence of this board a game can see.
    core.mapper_write(0xC000, 0x00);
    assert!(irq(&core), "$C000 acknowledged");
    core.mapper_write(0xC001, 0x00);
    assert!(irq(&core), "$C001 acknowledged");

    // `$C003` does, and it disables the counter with it, as the MMC3's `$E000` does.
    core.mapper_write(0xC003, 0x00);
    assert!(!irq(&core));
    assert_eq!(-1, tick_until_irq(&mut core, 60_000));

    // And `$C002` starts it again.
    core.mapper_write(0xC002, 0x00);
    assert!(tick_until_irq(&mut core, 200_000) > 0);
}

#[test]
fn req_core_100_an_a12_dip_shorter_than_the_filter_does_not_clock_the_tc0690() {
    // §5's own case for this step. The counter is the MMC3's and so is its filter: A12 must have
    // stayed low for `A12Watcher::MMC3_FILTER_DOTS` before a rise counts, which is what keeps
    // the two-dot nametable and attribute dips of every tile out of the count.
    let mut core = MapperRom::new(48).chr(32).load();
    core.write_memory(0x2001, 0x00); // Rendering off: only $2006 drives A12.

    // A12 high **before** the counter is armed, so the rise under test is the first one it sees.
    set_vram_address(&mut core, 0x1000);
    for _ in 0..8 {
        core.bus_mut().tick();
    }

    // Latch 0: on MMC3 revision B every clock that lands reaches zero and fires, so one
    // unfiltered rise is all it would take.
    arm_tc0690(&mut core, 0);

    // Three dots of low, under the ten-dot filter.
    set_vram_address(&mut core, 0x0000);
    core.bus_mut().tick();
    set_vram_address(&mut core, 0x1000);
    for _ in 0..8 {
        core.bus_mut().tick();
    }

    assert!(!irq(&core), "a three-dot dip clocked the counter");

    // Twenty-four dots of low, over it.
    set_vram_address(&mut core, 0x0000);
    for _ in 0..8 {
        core.bus_mut().tick();
    }

    set_vram_address(&mut core, 0x1000);
    for _ in 0..8 {
        core.bus_mut().tick();
    }

    assert!(
        irq(&core),
        "a twenty-four-dot dip did not clock the counter"
    );
}

// ------------------------------------- the J.Y. Company ASIC (35, 90, 209, 211), MF-UNL-4

/// Arms the board's counter: the source, the direction, the prescaler mask, then the prescaler
/// and counter values, then the enable.
///
/// `$C004` and `$C005` are written **after** `$C006`, because the page says the written value is
/// `XORed` with the content of that register before being stored; this helper leaves it at zero
/// so that a caller writes plain values.
#[cfg(feature = "board-jy-asic")]
fn arm_jy_irq(core: &mut NesCore, source: u8, direction: u8, prescaler: u8, counter: u8) {
    core.mapper_write(0xC001, (direction << 6) | source);
    core.mapper_write(0xC006, 0x00);
    core.mapper_write(0xC004, prescaler);
    core.mapper_write(0xC005, counter);
    core.mapper_write(0xC003, 0x00); // "$C003: IRQ Enable".
}

/// IRQ source 0, "CPU M2 rise": the prescaler and the counter in series on the CPU clock.
///
/// With the prescaler mask `$FF` and both values set so that one more count wraps each, the IRQ
/// comes on the very next cycle; with the counter one short of wrapping it takes 256 more.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_114_jy_asic_counts_cpu_m2_rises_on_source_0() {
    let mut core = MapperRom::new(209).prg(64).load();
    arm_jy_irq(&mut core, 0, 1, 0xFF, 0xFF);
    assert_eq!(
        1,
        tick_until_irq(&mut core, 8),
        "prescaler and counter wrap"
    );

    // One prescaler period is 256 CPU cycles, so a counter two short of wrapping is 512.
    core.mapper_write(0xC002, 0x00); // disable: acknowledges and resets the prescaler.
    assert!(!irq(&core));
    arm_jy_irq(&mut core, 0, 1, 0x00, 0xFE);
    assert_eq!(
        512,
        tick_until_irq(&mut core, 4096),
        "two prescaler periods"
    );
}

/// `$C001` bit 2, the prescaler mask: "0: $FF / 1: $07".
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_114_jy_asic_prescaler_mask_is_ff_or_07() {
    let mut core = MapperRom::new(209).prg(64).load();

    // Mask $07: the counter is clocked every eight CPU cycles, so a counter one short of
    // wrapping takes eight.
    core.mapper_write(0xC001, (1 << 6) | 0x04);
    core.mapper_write(0xC006, 0x00);
    core.mapper_write(0xC004, 0x00);
    core.mapper_write(0xC005, 0xFF);
    core.mapper_write(0xC003, 0x00);
    assert_eq!(8, tick_until_irq(&mut core, 64), "the $07 mask");

    core.mapper_write(0xC002, 0x00);
    arm_jy_irq(&mut core, 0, 1, 0x00, 0xFF);
    assert_eq!(256, tick_until_irq(&mut core, 1024), "the $FF mask");
}

/// `$C001` bits 7-6: "0: Counting disabled / 1: Increase / 2: Decrease / 3: Counting disabled".
///
/// And the page's wrap conditions, one per direction: the counter fires when it "is zero after
/// increasing or $FF after decreasing".
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_114_jy_asic_counts_up_down_or_not_at_all() {
    for direction in [0u8, 3] {
        let mut core = MapperRom::new(209).prg(64).load();
        arm_jy_irq(&mut core, 0, direction, 0xFF, 0xFF);
        assert_eq!(
            -1,
            tick_until_irq(&mut core, 2048),
            "direction {direction} must not count"
        );
    }

    // Up: $FF + 1 is zero, which fires.
    let mut core = MapperRom::new(209).prg(64).load();
    arm_jy_irq(&mut core, 0, 1, 0xFF, 0xFF);
    assert_eq!(1, tick_until_irq(&mut core, 8), "up, from $FF");

    // Down: 0 - 1 is $FF, which fires. The prescaler's own wrap is the same condition one level
    // up — "matches the prescaler mask after decreasing" — so a prescaler of zero wraps at once.
    let mut core = MapperRom::new(209).prg(64).load();
    arm_jy_irq(&mut core, 0, 2, 0x00, 0x00);
    assert_eq!(1, tick_until_irq(&mut core, 8), "down, from zero");
}

/// **Q-151**, second half: "If the AND-masked prescaler … wraps, meaning it is zero after
/// increasing or **matches the prescaler mask after decreasing**".
///
/// Mesen 2 writes `if(--prescaler == 0)`, which wraps off a prescaler of **one** — one count
/// earlier than the page. With the $07 mask and the prescaler set to 1, the page's rule needs two
/// clocks to reach the counter and Mesen's needs one; this test requires the page's.
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_151_jy_asic_prescaler_wraps_when_it_reaches_the_mask_going_down() {
    let mut core = MapperRom::new(209).prg(64).load();
    core.mapper_write(0xC001, (2 << 6) | 0x04); // down, mask $07, source 0.
    core.mapper_write(0xC006, 0x00);
    core.mapper_write(0xC004, 0x01); // prescaler 1.
    core.mapper_write(0xC005, 0x00); // counter 0: one clock of it fires.
    core.mapper_write(0xC003, 0x00);

    // 1 -> 0 clocks nothing; 0 -> 7 is the wrap, and that clock reaches the counter.
    core.bus_mut().tick();
    assert!(!irq(&core), "a prescaler of 1 going to 0 is not a wrap");
    core.bus_mut().tick();
    assert!(irq(&core), "0 going to the mask is the wrap");
}

/// **Q-151**, first half: "'Disabling' acknowledges a pending IRQ, inhibits counting, and resets
/// the prescaler to zero."
///
/// All three, and the third is the one Mesen 2 does not do. With the $FF mask, a prescaler left
/// at $FF is one clock from wrapping; disabled and re-enabled, the page's reset means it is 256
/// clocks away again.
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_151_jy_asic_disabling_the_irq_resets_the_prescaler_to_zero() {
    let mut core = MapperRom::new(209).prg(64).load();
    // Written through `write_memory` and not `mapper_write`, because this test counts single
    // cycles and that helper ticks the bus twice before every write.
    core.write_memory(0xC001, 1 << 6); // up, mask $FF, source 0.
    core.write_memory(0xC006, 0x00);
    core.write_memory(0xC004, 0xFE);
    core.write_memory(0xC005, 0xFF);
    core.write_memory(0xC003, 0x00);

    // One clock takes the prescaler to $FF, which is not yet a wrap.
    core.bus_mut().tick();
    assert!(!irq(&core));

    // Disable, then enable again without rewriting $C004: the prescaler starts from zero, so the
    // counter is a whole prescaler period away. Mesen 2 would leave it at $FF and fire next
    // cycle.
    core.write_memory(0xC002, 0x00);
    core.write_memory(0xC003, 0x00);
    assert_eq!(
        256,
        tick_until_irq(&mut core, 1024),
        "the prescaler was reset, not left at $FF"
    );

    // And "acknowledges a pending IRQ": $C000 with bit 0 clear does the same as $C002.
    assert!(irq(&core));
    core.write_memory(0xC000, 0x00);
    assert!(!irq(&core), "$C000 bit 0 clear acknowledges");
    core.write_memory(0xC003, 0x00);
    assert!(!irq(&core), "and re-enabling does not bring it back");
}

/// **Q-151**, third half: "inhibits counting" means the prescaler does not move while the IRQ is
/// disabled.
///
/// Mesen 2 clocks the prescaler and the counter whatever the enable says and tests it only before
/// asserting, so a long disabled stretch there advances the counter invisibly. The page's gloss
/// on disabling says otherwise, and this test requires the page's.
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_151_jy_asic_disabled_counting_does_not_advance_the_counter() {
    let mut core = MapperRom::new(209).prg(64).load();
    core.mapper_write(0xC001, (1 << 6) | 0x04); // up, mask $07, source 0.
    core.mapper_write(0xC006, 0x00);
    core.mapper_write(0xC004, 0x00);
    core.mapper_write(0xC005, 0xFF);

    // Not enabled: a thousand cycles must leave the counter where it is.
    for _ in 0..1000 {
        core.bus_mut().tick();
    }
    assert!(!irq(&core));

    // Enabled, the counter is still one clock of the prescaler from wrapping, which is eight
    // CPU cycles. If the disabled stretch had counted, those eight would be elsewhere.
    core.mapper_write(0xC003, 0x00);
    assert_eq!(8, tick_until_irq(&mut core, 64));
}

/// `$C006`, the `XOR` register: the page says the written value is `XORed` with the content of
/// `$C006` before being stored — on `$C004` and `$C005`, and not on `$C006` itself.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_114_jy_asic_xors_the_prescaler_and_counter_writes() {
    let mut core = MapperRom::new(209).prg(64).load();
    core.mapper_write(0xC001, 1 << 6); // up, mask $FF, source 0.
    core.mapper_write(0xC006, 0xFF);
    core.mapper_write(0xC004, 0x00); // stored as $FF.
    core.mapper_write(0xC005, 0x00); // stored as $FF.
    core.mapper_write(0xC003, 0x00);
    assert_eq!(
        1,
        tick_until_irq(&mut core, 8),
        "both values are the complement of what was written"
    );

    // And the same two writes with the XOR register clear are 256 * 256 cycles away.
    core.mapper_write(0xC002, 0x00);
    core.mapper_write(0xC006, 0x00);
    core.mapper_write(0xC004, 0x00);
    core.mapper_write(0xC005, 0x00);
    core.mapper_write(0xC003, 0x00);
    assert_eq!(-1, tick_until_irq(&mut core, 4096));
}

/// The census's own test: "with the source set to M2, the counter advances on CPU cycles and not
/// on A12 edges; with it set to A12, the reverse."
///
/// And the page's "unfiltered, eight per scanline", which is what keeps this board away from the
/// MMC3's A12 filter: every rise counts, however short the dip before it.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_114_jy_asic_irq_source_selection_picks_one_clock_and_not_the_other() {
    // Source 0: CPU cycles count, A12 rises do not.
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    arm_jy_irq(&mut core, 0, 1, 0xFF, 0xFF);
    for address in [0x0000u16, 0x1000, 0x0000, 0x1000] {
        core.cartridge_mut().unwrap().on_ppu_address_bus(address, 0);
    }
    assert!(!irq(&core), "source 0 must ignore A12");
    assert_eq!(1, tick_until_irq(&mut core, 8), "and count M2");

    // Source 1: A12 rises count, CPU cycles do not.
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    arm_jy_irq(&mut core, 1, 1, 0xFF, 0xFF);
    for _ in 0..600 {
        core.bus_mut().tick();
    }
    assert!(!irq(&core), "source 1 must ignore M2");
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x0000, 0);
    cartridge.on_ppu_address_bus(0x1000, 1);
    assert!(irq(&core), "one unfiltered A12 rise is one count");

    // Source 2: every PPU read counts, A12 or not — "170 per scanline".
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    arm_jy_irq(&mut core, 2, 1, 0xFF, 0xFF);
    for _ in 0..600 {
        core.bus_mut().tick();
    }
    assert!(!irq(&core), "source 2 must ignore M2");
    core.cartridge_mut().unwrap().on_ppu_address_bus(0x2000, 0);
    assert!(irq(&core), "a nametable fetch is a PPU read");
}

/// The dip the MMC3's filter would swallow and this board must not: "PPU A12 rise (unfiltered,
/// eight per scanline)".
///
/// Eight rises a scanline is the pattern of the fetch pipeline — the two-dot nametable and
/// attribute fetches at $2xxx between each pair of pattern fetches at $1xxx — and the TC0690's
/// own test in this file is the same experiment with the opposite expectation.
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_114_jy_asic_a12_counting_is_unfiltered() {
    let mut core = MapperRom::new(209).prg(64).chr(64).load();
    // Mask $07 and a counter one short, so the eighth rise of the line is the one that fires.
    core.mapper_write(0xC001, (1 << 6) | 0x04 | 0x01); // up, mask $07, source 1.
    core.mapper_write(0xC006, 0x00);
    core.mapper_write(0xC004, 0x00);
    core.mapper_write(0xC005, 0xFF);
    core.mapper_write(0xC003, 0x00);

    // Eight rises with two-dot dips between them, which an MMC3 counts as **one**.
    let cartridge = core.cartridge_mut().unwrap();
    for rise in 0..7 {
        cartridge.on_ppu_address_bus(0x2000, i64::from(rise) * 8);
        cartridge.on_ppu_address_bus(0x1000, i64::from(rise) * 8 + 2);
    }
    assert!(!irq(&core), "seven rises");

    let cartridge = core.cartridge_mut().unwrap();
    cartridge.on_ppu_address_bus(0x2000, 56);
    cartridge.on_ppu_address_bus(0x1000, 58);
    assert!(irq(&core), "the eighth rise of a line must count");
}

/// **Q-151**, fourth half: IRQ source 3, "CPU writes", has no hook in this core to hang on.
///
/// `Mapper::on_cpu_cycle` is told that a CPU cycle elapsed and not whether it was a write, and a
/// CPU write to RAM never reaches the cartridge at all — so the counter does not advance on
/// source 3. The ticket records the gap; this test pins the behaviour so that a lot which adds
/// the hook has to come and rewrite it.
#[cfg(feature = "board-jy-asic")]
#[test]
fn q_151_jy_asic_irq_source_3_selects_a_counter_that_does_not_advance() {
    let mut core = MapperRom::new(209).prg(64).load();
    arm_jy_irq(&mut core, 3, 1, 0xFF, 0xFF);
    assert_eq!(
        -1,
        tick_until_irq(&mut core, 4096),
        "source 3 is not clocked in this core"
    );

    // And the register is kept, so a save state carries the game's own selection: switching to
    // source 0 counts from where the game left the prescaler and the counter.
    core.mapper_write(0xC001, 1 << 6);
    assert_eq!(1, tick_until_irq(&mut core, 8));
}

/// `ppu_irq_horizon` answers `None` for every IRQ source, which is the property MF-6b's lots
/// pinned for every IRQ board and the census asks for by name.
///
/// It is `None` for the whole `IrqBoard` variant, not per source: MF-6b step 4 measured any
/// forward of that dispatch at `NesBus::catch_up_ppu` 188 → 190 instructions and `mmc5`
/// +9 214 379 over 300 frames. The stronger property is what the test requires.
#[cfg(feature = "board-jy-asic")]
#[test]
fn jy_asic_ppu_irq_horizon_is_none_for_every_source() {
    let mut core = MapperRom::new(209).prg(64).load();
    assert_eq!(None, core.cartridge().unwrap().ppu_irq_horizon());
    for source in 0u8..4 {
        for direction in 0u8..4 {
            core.mapper_write(0xC001, (direction << 6) | source);
            assert_eq!(
                None,
                core.cartridge().unwrap().ppu_irq_horizon(),
                "source {source}, direction {direction}"
            );
        }
    }
}

/// `$C000`-`$C007`'s mask is `$F007`, and the page says so in the opposite words to every other
/// window of the chip: "Writes are accepted even if address bit 11 is set."
#[cfg(feature = "board-jy-asic")]
#[test]
fn req_core_114_jy_asic_irq_registers_accept_address_bit_11() {
    let mut core = MapperRom::new(209).prg(64).load();
    // The whole arming sequence through the A11-set aliases of each register.
    core.mapper_write(0xC801, 1 << 6);
    core.mapper_write(0xCFFE, 0x00);
    core.mapper_write(0xC8FC, 0xFF);
    core.mapper_write(0xCBFD, 0xFF);
    core.mapper_write(0xC80B, 0x00);
    assert_eq!(1, tick_until_irq(&mut core, 8), "mask $F007, A11 accepted");

    // And A3-A10 are don't-care, so $CFFA is $C002.
    core.mapper_write(0xCFFA, 0x00);
    assert!(!irq(&core));
}

// ---------------------------------------------------------------- NES 2.0 mapper 298,
// ---------------------------------------------------------------- MF-UNL-7 step 4

/// Arms the counter of a `VRC4b` in cycle mode with a latch of $FE, so that it fires on the
/// second tick, and with the `A` bit of the control register set to `a`.
///
/// The addresses are the `VRC4b`'s own: "CPU 0x02s to VRC4 A0, CPU 0x01s to VRC4 A1", so $F000 is
/// the latch's low nibble, **$F002 its high one, $F001 the control register and $F003 the
/// acknowledge** — which is why the page can say "$F003" and mean the acknowledge without naming
/// a single register of the chip.
#[cfg(feature = "board-298")]
fn arm_vrc4b_counter(core: &mut NesCore, a: u8) {
    core.write_memory(0xF000, 0x0E); // Latch low nibble.
    core.write_memory(0xF002, 0x0F); // Latch high nibble: $FE.
    core.write_memory(0xF001, 0x06 | a); // Cycle mode, enabled, `A` as asked.
}

/// **The one documented difference of NES 2.0 mapper 298**, as a direct A/B against the `VRC4b`
/// this build already had — mapper 25 submapper 1 (nesdev, "NES 2.0 Mapper 298", MF-UNL-7 step 4):
///
/// > writing to $F003 only acknowledges a pending IRQ, but it does not move the 'A' control bit to
/// > the 'E' control bit [the way an original VRC4 does].
///
/// So with `A` clear, acknowledging on Konami's chip *disables* the counter and acknowledging on
/// the clone leaves it running — and the clone fires again where the original goes quiet.
#[cfg(feature = "board-298")]
#[test]
fn req_core_114_mapper_298_acknowledges_at_f003_without_copying_a_into_e() {
    // The original: `A` clear, so the acknowledge turns the counter off.
    let mut konami = MapperRom::new(25).submapper(1).chr(32).load();
    arm_vrc4b_counter(&mut konami, 0x00);
    assert_eq!(2, tick_until_irq(&mut konami, 1000), "the VRC4b fires");
    konami.write_memory(0xF003, 0x00);
    assert!(!irq(&konami), "and the acknowledge clears it");
    assert_eq!(
        -1,
        tick_until_irq(&mut konami, 1000),
        "A clear, copied into E: Konami's counter stops"
    );

    // The clone: the same sequence, and the counter keeps running because `E` was not touched.
    let mut clone = MapperRom::new(298).chr(32).load();
    arm_vrc4b_counter(&mut clone, 0x00);
    assert_eq!(2, tick_until_irq(&mut clone, 1000), "the clone fires too");
    clone.write_memory(0xF003, 0x00);
    assert!(!irq(&clone), "the acknowledge still clears the pending IRQ");
    assert_eq!(
        2,
        tick_until_irq(&mut clone, 1000),
        "but E is untouched, so the counter fires again"
    );

    // And the other way round: `A` set with `E` clear is the arrangement a VRC4 game uses to arm
    // the counter *from* an acknowledge. Konami's chip starts counting; the clone never does.
    let mut konami = MapperRom::new(25).submapper(1).chr(32).load();
    konami.write_memory(0xF001, 0x05); // Cycle mode, not enabled, A set.
    assert_eq!(-1, tick_until_irq(&mut konami, 300), "not enabled yet");
    konami.write_memory(0xF003, 0x00);
    assert_eq!(
        256,
        tick_until_irq(&mut konami, 1000),
        "A copied into E starts the counter from 0"
    );

    let mut clone = MapperRom::new(298).chr(32).load();
    clone.write_memory(0xF001, 0x05);
    assert_eq!(-1, tick_until_irq(&mut clone, 300), "not enabled yet");
    clone.write_memory(0xF003, 0x00);
    assert_eq!(
        -1,
        tick_until_irq(&mut clone, 1000),
        "and the clone's acknowledge cannot start it at all"
    );
}

/// nesdev ("INES Mapper 012"): "The Huang-1 ASIC functions exclusively as an MMC3A clone on this
/// PCB; **MMC3A-specific IRQ behavior is necessary for running *Dragon Ball Z 5***."
///
/// The MMC3A's rule is the one mapper 4 reaches through NES 2.0 submapper 4 and mappers 114 and 268
/// through their own pages: **a latch value of zero disables the IRQ**. This build's reading of it
/// is `Mmc3Board::clock_irq_counter`'s own — AMD-061 §8's criterion, where revision A "also demands
/// that the counter was moving", so a latch of `$00` gives **exactly one** IRQ (the pending `$C001`
/// reload lets the first clock through) where revision B fires on every rendered line.
///
/// A direct A/B against mapper 4 with the same image and the same sequence (MF-UNL-8 step 1).
#[cfg(feature = "board-huang1")]
#[test]
fn req_core_114_mapper_12_has_the_mmc3a_counter_whose_zero_latch_disables_the_irq() {
    // Both revisions let the first clock through, because both have a reload pending.
    let mut boards: Vec<(usize, NesCore)> = vec![(4, MapperRom::new(4).chr(32).load())];
    boards.push((12, MapperRom::new(12).chr(32).load()));
    for (mapper, core) in &mut boards {
        enable_scanline_counting(core);
        core.mapper_write(0xC000, 0); // Latch zero.
        core.mapper_write(0xC001, 0x00); // Reload on the next clock.
        core.mapper_write(0xE001, 0x00); // Enable.
        assert!(
            tick_until_irq(core, 200_000) > 0,
            "mapper {mapper}: the reload lets the first clock through"
        );

        // Acknowledge and re-enable, with the counter sitting at zero and no reload pending.
        core.mapper_write(0xE000, 0x00);
        core.mapper_write(0xE001, 0x00);
        assert!(!irq(core), "mapper {mapper}: acknowledged");
    }

    assert!(
        tick_until_irq(&mut boards[0].1, 200_000) > 0,
        "revision B fires again on every line"
    );
    assert_eq!(
        -1,
        tick_until_irq(&mut boards[1].1, 200_000),
        "the MMC3A's zero latch disables the IRQ from the second clock on"
    );

    // And a non-zero latch still fires repeatedly, so the counter is a counter and not switched
    // off.
    let mut huang = MapperRom::new(12).chr(32).load();
    enable_scanline_counting(&mut huang);
    huang.mapper_write(0xC000, 8);
    huang.mapper_write(0xC001, 0x00);
    huang.mapper_write(0xE001, 0x00);
    assert!(tick_until_irq(&mut huang, 200_000) > 0);
    huang.mapper_write(0xE000, 0x00);
    huang.mapper_write(0xE001, 0x00);
    assert!(tick_until_irq(&mut huang, 200_000) > 0);
}

/// Q-162: the census's own description of this difference is "a write to $F003 acknowledges an IRQ
/// and does **not reload the counter**", and that is not what separates the two chips — a VRC4's
/// acknowledge does not reload the counter either. Only `$F001` with `E` set does, which is the
/// page's "If E is set, the IRQ counter is reloaded with the latch value".
///
/// The page wins, which is MF-UNL-2's rule, and this test pins the half of the census's sentence
/// that **is** true of both chips: neither acknowledge reloads (MF-UNL-7 step 4).
#[cfg(feature = "board-298")]
#[test]
fn q_162_no_vrc4_acknowledge_reloads_the_counter_on_either_chip() {
    for mapper in [25usize, 298] {
        let mut core = if mapper == 25 {
            MapperRom::new(25).submapper(1).chr(32).load()
        } else {
            MapperRom::new(298).chr(32).load()
        };

        // Latch $FE, enabled in cycle mode with `A` set so that both chips keep counting through
        // the acknowledge. The counter reloads once, on the control write.
        arm_vrc4b_counter(&mut core, 0x01);
        assert_eq!(2, tick_until_irq(&mut core, 1000), "mapper {mapper}");

        // The firing itself reloaded the counter to $FE, so the next IRQ is two ticks away. An
        // acknowledge in the middle of that must not put it back to $FE a second time, or the
        // period would be three.
        core.write_memory(0xF003, 0x00);
        core.bus_mut().tick();
        assert!(!irq(&core), "mapper {mapper}: one tick in");
        core.write_memory(0xF003, 0x00);
        assert_eq!(
            1,
            tick_until_irq(&mut core, 1000),
            "mapper {mapper}: the second acknowledge did not reload"
        );
    }
}
