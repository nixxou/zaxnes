//! The frame counter, its IRQ and the $4015 status (REQ-CORE-111, REQ-CORE-081,
//! REQ-CORE-004, REQ-CORE-070).

#![allow(clippy::float_cmp)] // Exact figures.

mod common;

use common::{loaded_core as loaded_bus, machine_with, minimal};
use zax_nes::cpu::CpuBus;
use zax_nes::{NesCore, NesRegion, RegionTiming};

const NTSC_FOUR_STEP_PERIOD: usize = 29830;

fn loaded_core() -> NesCore {
    machine_with(&minimal(), "Apu.nes")
}

fn tick(core: &mut NesCore, cycles: usize) {
    for _ in 0..cycles {
        core.bus_mut().tick();
    }
}

#[test]
fn req_core_111_the_four_step_sequence_raises_the_frame_irq() {
    let mut core = loaded_core();

    // Four-step mode, IRQ enabled.
    core.write_memory(0x4017, 0x00);
    assert!(!core.apu().irq_asserted());

    tick(&mut core, NTSC_FOUR_STEP_PERIOD + 8);

    assert!(core.apu().irq_asserted());

    // Reading $4015 clears the flag.
    assert_eq!(0x40, core.bus_mut().apu_mut().read_status(0) & 0x40);
    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_111_the_inhibit_bit_suppresses_the_frame_irq() {
    let mut core = loaded_core();

    core.write_memory(0x4017, 0x40); // Four-step mode, IRQ inhibited.
    tick(&mut core, 2 * NTSC_FOUR_STEP_PERIOD);

    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_111_the_five_step_sequence_never_raises_the_frame_irq() {
    let mut core = loaded_core();

    core.write_memory(0x4017, 0x80); // Five-step mode.
    tick(&mut core, 2 * 37282);

    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_111_setting_the_inhibit_bit_clears_a_pending_frame_irq() {
    let mut core = loaded_core();
    core.write_memory(0x4017, 0x00);
    tick(&mut core, NTSC_FOUR_STEP_PERIOD + 8);

    assert!(core.apu().irq_asserted());

    core.write_memory(0x4017, 0x40);
    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_081_peeking_4015_does_not_clear_the_frame_irq() {
    let mut core = loaded_core();
    core.write_memory(0x4017, 0x00);
    tick(&mut core, NTSC_FOUR_STEP_PERIOD + 8);

    assert_eq!(0x40, core.apu().peek_status() & 0x40);
    assert!(core.apu().irq_asserted());

    // Only a real CPU read clears it.
    core.bus_mut().apu_mut().read_status(0);
    assert!(!core.apu().irq_asserted());
}

/// `AccuracyCoin`'s `Frame Counter IRQ` 6 and 7 (REQ-CORE-111, REQ-CORE-081): a `$4015` read
/// arms the clear of bit 6 for the first **get** cycle strictly after it — an even cycle in this
/// core's numbering — so a second read one cycle later sees bit 6 gone only when the first read
/// was on a put cycle.
///
/// The ROM makes its pair of reads one cycle apart with `$1F`, `SLO Absolute,X`, over `$4015`;
/// here the two reads are driven on the bus directly, which is the same two cycles.
#[test]
fn req_core_111_a_4015_read_clears_bit_6_on_the_next_get_cycle() {
    // Returns the two reads' bit 6, the first read landing on a put cycle or a get one.
    fn two_reads_one_cycle_apart(first_on_put: bool) -> (u8, u8) {
        let (_cpu, mut bus) = loaded_bus(None);
        // Four-step mode, IRQ enabled, then long enough for the flag to be set.
        bus.write(0x4017, 0x00);
        for _ in 0..NTSC_FOUR_STEP_PERIOD + 8 {
            bus.tick();
        }

        // Align so that the next cycle — the first read's — has the wanted parity. A get cycle
        // is an even one, so a put cycle is odd.
        while ((bus.cycle_count() + 1) & 1 == 1) != first_on_put {
            bus.tick();
        }

        let first = bus.read(0x4015);
        assert_eq!(
            first_on_put,
            bus.cycle_count() & 1 == 1,
            "the first read did not land where the test meant it to"
        );
        let second = bus.read(0x4015);
        (first & 0x40, second & 0x40)
    }

    // Test 6: read on a put cycle, read again on the get cycle after it. The clear lands on
    // that get cycle, so the second read sees bit 6 clear.
    assert_eq!((0x40, 0x00), two_reads_one_cycle_apart(true));

    // Test 7: read on a get cycle, read again on the put cycle after it. The clear waits for
    // the get cycle after *that*, so the second read still sees bit 6 set.
    assert_eq!((0x40, 0x40), two_reads_one_cycle_apart(false));
}

/// The other half of the rule above: the IRQ **line** goes low on the read itself, whatever the
/// parity, which is what blargg's `07.irq_flag_timing` and `08.irq_timing` pin (Q-122).
#[test]
fn req_core_111_the_frame_irq_line_falls_on_the_read_and_not_on_the_get_cycle() {
    for first_on_put in [true, false] {
        let (_cpu, mut bus) = loaded_bus(None);
        bus.write(0x4017, 0x00);
        for _ in 0..NTSC_FOUR_STEP_PERIOD + 8 {
            bus.tick();
        }
        while ((bus.cycle_count() + 1) & 1 == 1) != first_on_put {
            bus.tick();
        }

        assert!(bus.apu_mut().irq_asserted());
        bus.read(0x4015);
        assert!(
            !bus.apu_mut().irq_asserted(),
            "the line should be low at once, first read on a put cycle: {first_on_put}"
        );
    }
}

#[test]
fn req_core_081_4015_reports_the_channels_whose_length_counter_is_running() {
    let mut core = loaded_core();

    // Enable pulse 1 and the triangle, then load their length counters.
    core.write_memory(0x4015, 0x05);
    core.write_memory(0x4003, 0xF8); // Pulse 1 length index 31 → 30.
    core.write_memory(0x400B, 0xF8); // Triangle.

    assert_eq!(0x05, core.apu().peek_status() & 0x0F);

    // A disabled channel's counter is cleared at once.
    core.write_memory(0x4015, 0x04);
    assert_eq!(0x04, core.apu().peek_status() & 0x0F);
}

#[test]
fn req_core_081_a_length_counter_is_not_loaded_while_its_channel_is_disabled() {
    let mut core = loaded_core();

    core.write_memory(0x4015, 0x00);
    core.write_memory(0x4003, 0xF8);

    assert_eq!(0x00, core.apu().peek_status() & 0x0F);
}

#[test]
fn req_core_111_half_frames_count_the_length_counters_down() {
    let mut core = loaded_core();

    core.write_memory(0x4015, 0x01); // Pulse 1 on.
    core.write_memory(0x4000, 0x00); // Length counter not halted.
    core.write_memory(0x4003, 0x08); // Length index 1 → 254.
    core.write_memory(0x4017, 0x00); // Four-step mode, sequencer restarted.

    assert_eq!(0x01, core.apu().peek_status() & 0x01);

    // 254 half frames, two per sequence period, plus a margin.
    tick(&mut core, 128 * NTSC_FOUR_STEP_PERIOD);

    assert_eq!(0x00, core.apu().peek_status() & 0x01);
}

#[test]
fn req_core_111_a_halted_length_counter_never_reaches_zero() {
    let mut core = loaded_core();

    core.write_memory(0x4015, 0x01);
    core.write_memory(0x4000, 0x20); // Halt flag set.
    core.write_memory(0x4003, 0x08);
    core.write_memory(0x4017, 0x00);

    tick(&mut core, 300 * NTSC_FOUR_STEP_PERIOD);

    assert_eq!(0x01, core.apu().peek_status() & 0x01);
}

#[test]
fn req_core_004_reset_silences_the_apu_and_clears_its_irq() {
    let mut core = loaded_core();
    core.write_memory(0x4015, 0x0F);
    core.write_memory(0x4003, 0xF8);
    core.write_memory(0x4017, 0x00);
    tick(&mut core, NTSC_FOUR_STEP_PERIOD + 8);

    assert!(core.apu().irq_asserted());

    core.reset();

    assert!(!core.apu().irq_asserted());
    assert_eq!(0x00, core.apu().peek_status());
    assert_eq!(0, core.apu().available_samples());
}

#[test]
fn req_core_070_one_frame_produces_the_documented_number_of_samples() {
    // Cross-check of the per-frame figure the resampler is calibrated on.
    assert_eq!(
        800.0,
        RegionTiming::for_region(NesRegion::Ntsc).samples_per_frame(48_000)
    );
    assert_eq!(
        960.0,
        RegionTiming::for_region(NesRegion::Pal).samples_per_frame(48_000)
    );
    assert_eq!(
        960.0,
        RegionTiming::for_region(NesRegion::Dendy).samples_per_frame(48_000)
    );
    assert_eq!(
        800.0,
        RegionTiming::for_region(NesRegion::Multi).samples_per_frame(48_000)
    );
}

/// The 2A07's four-step sequence closes two CPU cycles later than this core used to think, and
/// the whole of `pal_apu_tests` 04 to 08, 10 and 11 hangs on it (REQ-CORE-015, REQ-CORE-111).
///
/// The nesdev "APU Frame Counter" page gives each console's steps in **APU** cycles, and a CPU
/// cycle is half of one: the 2A03's 14914 / 14914.5 / 14915 are CPU 29828 / 29829 / 29830, and
/// the 2A07's 16626 / 16626.5 / 16627 are CPU 33252 / 33253 / 33254. This measures the period
/// end to end rather than reading the table back, so a table edited by hand cannot satisfy it
/// without being right.
#[test]
fn req_core_015_each_region_closes_its_four_step_sequence_where_nesdev_says() {
    for (region, period) in [
        (NesRegion::Ntsc, 29830),
        (NesRegion::Pal, 33254),
        (NesRegion::Dendy, 29830), // The 2A03's APU, on a 312-line console.
        (NesRegion::Multi, 29830),
    ] {
        let mut core = NesCore::new();
        core.set_region(region);
        assert!(core.load_rom(&minimal(), "Apu.nes").success);
        core.set_region(region);

        // Four-step mode, IRQ enabled. The reset lands three or four cycles after the write.
        core.write_memory(0x4017, 0x00);
        core.bus_mut().apu_mut().read_status(0);

        let mut cycles = 0;
        while !core.apu().irq_asserted() && cycles < 2 * period {
            core.bus_mut().tick();
            cycles += 1;
        }

        // The $4017 poke leaves the sequencer reset pending, and the CPU sees the flag on the
        // cycle after the one it is raised on; both offsets are the same for every region, so
        // the count is the region's period plus one.
        assert_eq!(period + 1, cycles, "{region:?}");
    }
}
