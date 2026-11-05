//! What a DMC sample fetch does to the cycle it halts the CPU on (nesdev "DMA";
//! REQ-CORE-113).
//!
//! The DMA unit stops the CPU with its address still on the bus, so the read the CPU was making
//! happens **twice**. On `$4016` that clocks the controller's shift register one extra time and
//! the program loses a bit — the glitch `nes-test-roms/dmc_dma_during_read4/dma_4016_read`
//! counts and the reason several commercial games read the pad twice and compare. On `$2007` it
//! advances the PPU's address and read buffer, which is what `dma_2007_read` prints.

mod common;

use common::loaded_core;
use zax_nes::cpu::CpuBus;
use zax_nes::NesBus;

/// Arms a DMC sample fetch and burns its start delay on **write** cycles, which a DMA unit may
/// not halt: the bus comes back with the fetch pending and the next read cycle halted.
fn arm_a_pending_dmc_fetch(bus: &mut NesBus) {
    bus.write(0x4010, 0x00); // Fastest rate, no IRQ, no loop.
    bus.write(0x4012, 0x00); // Sample at $C000, whose low five bits decode to $4000.
    bus.write(0x4013, 0x01); // 17 bytes, so the fetch is not the last one.
    bus.write(0x4015, 0x10);

    // The fetch a $4015 write starts is asked for two or three cycles later; a write cannot be
    // halted, so these only let the delay run out.
    for _ in 0..3 {
        bus.write(0x0000, 0x00);
    }
}

/// Starts a long DPCM sample at the slowest rate and takes its **load** fetch, so that the
/// sample buffer is full and the memory reader's next question is a whole shift register away.
fn start_a_dmc_sample(bus: &mut NesBus) {
    bus.write(0x4010, 0x00); // Rate 0: 428 CPU cycles a bit, no IRQ, no loop.
    bus.write(0x4012, 0x00); // Sample at $C000.
    bus.write(0x4013, 0x0F); // 241 bytes, so no fetch below is the last one.
    bus.write(0x4015, 0x10);

    // The load fetch is asked for two or three cycles after the write; a write cycle cannot be
    // halted, so these only let the delay run out.
    for _ in 0..3 {
        bus.write(0x0000, 0x00);
    }
    bus.read(0x8000);
    assert_eq!(1, bus.dmc_dma_count());
}

/// `Apu2A03::cycles_until_dmc_fetch` names the cycle the memory reader asks on, to the cycle and
/// in both directions.
///
/// This is the bound lot MF-4a-ter's fast OAM loop rests on: it skips the DMA state machine for
/// the cycles the horizon covers, so a horizon one cycle too long would let a DMC fetch land late
/// and one cycle too short would only cost speed. The two halves of the assertion below are
/// exactly those two failures. Asked at sixteen phases of the timer, since it is the timer's
/// remainder that the first of the eight bits depends on.
#[test]
fn req_core_113_the_dmc_fetch_horizon_names_the_cycle_the_reader_asks_on() {
    for phase in 0..16u32 {
        let (_, mut bus) = loaded_core(None);
        start_a_dmc_sample(&mut bus);
        for _ in 0..phase {
            bus.write(0x0000, 0x00);
        }

        // The bus is not batching here, so the APU is caught up and the horizon is a count of
        // bus cycles from now with no lag to take off it.
        let horizon = bus
            .apu()
            .cycles_until_dmc_fetch()
            .expect("the sample is still playing");
        assert!(horizon > 1, "phase {phase}: {horizon}");

        // One cycle short of it the reader has not asked yet: the read cycle there runs plainly.
        let mut probe = bus.clone();
        for _ in 0..horizon - 1 {
            probe.write(0x0000, 0x00);
        }
        probe.read(0x8000);
        assert_eq!(1, probe.dmc_dma_count(), "phase {phase}, horizon {horizon}");

        // At it, it has: the next read cycle is halted and the fetch runs.
        for _ in 0..horizon {
            bus.write(0x0000, 0x00);
        }
        bus.read(0x8000);
        assert_eq!(2, bus.dmc_dma_count(), "phase {phase}, horizon {horizon}");
    }
}

/// A channel with no bytes left answers "never", and nothing a halted CPU can do changes that:
/// only a `$4015` write or a fetch's own `supply_byte` can put bytes back, and a halted CPU makes
/// neither. That is what lets the fast loop run a whole transfer without asking again.
#[test]
fn req_core_113_the_dmc_fetch_horizon_is_none_while_no_sample_plays() {
    let (_, mut bus) = loaded_core(None);
    assert_eq!(None, bus.apu().cycles_until_dmc_fetch());

    start_a_dmc_sample(&mut bus);
    assert!(bus.apu().cycles_until_dmc_fetch().is_some());

    // $4015 bit 4 clear stops the sample.
    bus.write(0x4015, 0x00);
    assert_eq!(None, bus.apu().cycles_until_dmc_fetch());
}

/// A lone DMC fetch halting a read of an address **outside** `$4000`-`$401F` takes its three or
/// four cycles by the parity of the halt, and repeats the CPU's read on each idle cycle.
///
/// That is the sequence `NesBus::dmc_fetch_cycles` runs straight instead of through the DMA state
/// machine (lot MF-4a-ter step 3). `only_the_halt_cycle_of_a_dma_reaches_the_controller` above
/// pins the same counts for a halted `$4016` read, which is the `internal` case and still goes
/// through the state machine, so between them both halves are covered.
#[test]
fn req_core_113_a_lone_dmc_fetch_takes_three_cycles_or_four_by_its_alignment() {
    let mut stalls = Vec::new();
    let mut reads = Vec::new();

    for padding in [0, 1] {
        let (_, mut bus) = loaded_core(None);
        for _ in 0..padding {
            bus.tick(); // Flips the parity the reset sequence left behind.
        }
        arm_a_pending_dmc_fetch(&mut bus);

        // $2002 is read by the halt and again by every idle cycle of the DMA; each read clears
        // the address latch and returns the open bus in its low five bits, but the PPU counts
        // them, which is what `dma_2007_read` and `double_2007_read` are built on. Here the
        // cheaper witness is the cycle count plus the extra read the halt itself makes.
        let before = bus.cycle_count();
        bus.read(0x8000);
        stalls.push(bus.cycle_count() - before - 1);
        reads.push(bus.dmc_dma_count());
    }

    stalls.sort_unstable();
    assert_eq!(
        vec![
            i64::from(NesBus::DMC_DMA_CYCLES_ALIGNED),
            i64::from(NesBus::DMC_DMA_CYCLES),
        ],
        stalls
    );
    assert_eq!(vec![1, 1], reads);
}

/// Reads `$4016` until the controller answers 1, as `dma_4016_read` does, and returns how many
/// reads that took. With one button pressed and the strobe released, an untouched controller
/// takes eight.
fn reads_until_the_controller_answers(bus: &mut NesBus, dma: bool) -> u32 {
    bus.input_mut().begin_frame(0x0080); // Port 0, bit 7: Right.
    bus.write(0x4016, 0x01);
    bus.write(0x4016, 0x00);

    if dma {
        arm_a_pending_dmc_fetch(bus);
    }

    let mut reads = 0;
    while reads < 32 {
        reads += 1;
        if bus.read(0x4016) & 0x01 != 0 {
            break;
        }
    }
    reads
}

#[test]
fn req_core_113_the_halt_cycle_reads_4016_again_and_the_program_loses_a_bit() {
    let (_, mut bus) = loaded_core(None);
    let without = reads_until_the_controller_answers(&mut bus, false);

    let (_, mut bus) = loaded_core(None);
    let with = reads_until_the_controller_answers(&mut bus, true);

    assert_eq!(8, without);
    assert_eq!(1, bus.dmc_dma_count());

    // The halt cycle clocked the shift register once on its own, so the eighth bit arrives one
    // read early. That is `dma_4016_read`'s "08 08 07 08 08" in one assertion.
    assert_eq!(7, with);
}

/// The idle cycles of the DMA — the DMC's dummy read and any alignment — do **not** clock the
/// controller a second and third time: /OE stays active for them, so the port sees the halt
/// cycle and nothing else (nesdev, "DMA"). Were they counted, the program would lose three bits
/// instead of one.
#[test]
fn req_core_113_only_the_halt_cycle_of_a_dma_reaches_the_controller() {
    let (_, mut bus) = loaded_core(None);
    bus.input_mut().begin_frame(0x0080);
    bus.write(0x4016, 0x01);
    bus.write(0x4016, 0x00);

    arm_a_pending_dmc_fetch(&mut bus);
    let before = bus.cycle_count();
    bus.read(0x4016);
    let stolen = bus.cycle_count() - before - 1;

    // Three or four cycles were spent inside the DMA, and exactly one of them was a read the
    // controller answered: the next read must still be bit 1 of 8, not bit 3 or 4.
    assert!(
        [
            i64::from(NesBus::DMC_DMA_CYCLES_ALIGNED),
            i64::from(NesBus::DMC_DMA_CYCLES),
        ]
        .contains(&stolen),
        "{stolen}"
    );

    let mut reads = 1; // The halted one above.
    while reads < 32 {
        reads += 1;
        if bus.read(0x4016) & 0x01 != 0 {
            break;
        }
    }
    assert_eq!(7, reads);
}

/// A DMC sample fetch drives the **external** data bus and not the internal one.
///
/// Both halves come from `AccuracyCoin`. `Internal Data Bus` test 2: a fetch lands between the
/// operands and the read of `LDA $4015`, and "This DMC DMA does not update the internal data bus.
/// Only the external one. Hence the read from $4015 not reflecting the DMA's read in bit 5."
/// `DMA + Open Bus` test 2 is the other way round: the byte the fetch brought in **is** what an
/// open-bus read on the next cycle returns. The rule is what lets `Implied Dummy Reads` fetch an
/// opcode out of `$4015` and still read bit 5 of the byte that was on the internal bus before the
/// fetch (REQ-CORE-113).
#[test]
fn req_core_113_a_dmc_fetch_drives_the_external_bus_and_not_the_internal_one() {
    /// Arms a fetch with the two buses both holding $20, and burns the start delay on writes.
    /// The sample is at $C000, which is zero in this ROM, so the byte the fetch brings in is
    /// $00 — the opposite of the $20 the buses hold, which is what makes both halves visible.
    fn armed_with_20_on_the_bus() -> NesBus {
        let (_, mut bus) = loaded_core(None);
        bus.write(0x4010, 0x00); // Fastest rate, no IRQ, no loop.
        bus.write(0x4012, 0x00); // Sample at $C000.
        bus.write(0x4013, 0x01); // 17 bytes, so the fetch is not the last one.
        bus.write(0x4015, 0x10);
        for _ in 0..3 {
            bus.write(0x0000, 0x20); // A write drives both buses and cannot be halted.
        }
        bus
    }

    // The internal half. The halted read is `$4015`: its halt cycle reads the register again —
    // bit 5 coming from the $20 on the internal bus — then the fetch runs, then the CPU gets its
    // read. Bit 5 is still the internal bus's, which the fetch did not touch.
    let mut bus = armed_with_20_on_the_bus();
    let status = bus.read(0x4015);
    assert_eq!(1, bus.dmc_dma_count(), "the fetch did happen");
    assert_eq!(
        0x20,
        status & 0x20,
        "bit 5 is the internal bus, not the sample byte"
    );

    // The external half. The halted read is an unmapped address, so what it returns *is* the
    // pins: the fetch drove them with the sample byte, so $00 and not the $20 of the internal
    // bus.
    let mut bus = armed_with_20_on_the_bus();
    let open = bus.read(0x5000);
    assert_eq!(1, bus.dmc_dma_count(), "the fetch did happen");
    assert_eq!(0x00, open, "the pins carry the byte the fetch read");

    // With no fetch in the way the same read gives the $20 both buses hold.
    let (_, mut bus) = loaded_core(None);
    bus.write(0x0000, 0x20);
    assert_eq!(0x20, bus.read(0x5000));
}
