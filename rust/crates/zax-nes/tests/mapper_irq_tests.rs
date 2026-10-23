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
use zax_nes::Machine;

/// Ticks the bus until the cartridge asserts its IRQ; the number of ticks, or -1 when none
/// comes within `budget`.
fn tick_until_irq(core: &mut Machine, budget: i64) -> i64 {
    for cycle in 1..=budget {
        core.bus_mut().tick();
        if core.cartridge().unwrap().irq_asserted() {
            return cycle;
        }
    }

    -1
}

fn irq(core: &Machine) -> bool {
    core.cartridge().unwrap().irq_asserted()
}

// ---------------------------------------------------------------- MMC3 and RAMBO-1

fn enable_scanline_counting(core: &mut Machine) {
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

fn write_vrc(core: &mut Machine, mapper: usize, group: u16, index: u16, value: u8) {
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
    assert_eq!(59, core.ppu().scanline());
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
