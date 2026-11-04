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

#[test]
fn req_core_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names() {
    let mut core = MapperRom::new(5).chr(32).load();
    core.write_memory(0x2001, 0x18); // Rendering on: the board can see the lines.
    core.write_memory(0x5203, 60);
    core.write_memory(0x5204, 0x80); // Enable.
    assert!(tick_until_irq(&mut core, 200_000) > 0);
    // MR-5-3: the first line of a frame only sets the in-frame flag and zeroes the counter, so
    // the counter reaches 60 on visible line 60, not on line 59.
    assert_eq!(60, core.ppu().scanline());
    // Bit 7 of $5204 reports the pending IRQ and reading it acknowledges; bit 6 says the
    // scan is inside a frame.
    assert_eq!(0xC0, core.read_memory(0x5204) & 0xC0); // A peek leaves it pending.
    assert!(irq(&core));
    let status = core.cartridge_mut().unwrap().try_cpu_read(0x5204).unwrap();
    assert_eq!(0xC0, status & 0xC0);
    assert!(!irq(&core));
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
