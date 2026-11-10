//! The `NesHawkCompat` profile's items, one test each (lot NESHAWK-PROFILE step 3).
//!
//! Every item is a difference between BizHawk's NesHawk core and this one that
//! `docs/progress/NESHAWK-PROFILE.md` step 1 read out of NesHawk's source
//! (`src/BizHawk.Emulation.Cores/Consoles/Nintendo/NES/APU.cs` and `NES.Core.cs`, MIT), quoted
//! there against the function it came from. The tests below are written **differentially** — each
//! measures the same thing on both profiles and asserts the gap the item predicts — so that they
//! say what the item *is* rather than pinning a number that only one profile's arithmetic
//! produces, and so that a regression on the `Standard` profile fails them too.
//!
//! No `REQ-` identifier: the proposed wording for one is in the progress note.
//!
//! Sources, for the record: **BizHawk** is MIT and nothing is copied from it — the profile is
//! written from the behaviour its source describes. **Mesen 2** is GPL-3.0 and the `Standard`
//! arms, which agree with it, already carry their attribution in `nes_bus.rs` and
//! `apu/dmc_channel.rs`.

// Test helpers: a failed setup panics, as `tests/common/mod.rs` does for the same reason.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::minimal_rom;
use zax_nes::apu::Apu2A03;
use zax_nes::cartridges::Cartridge;
use zax_nes::cpu::{Cpu6502, CpuBus};
use zax_nes::input::InputPorts;
use zax_nes::ppu::Ppu2C02;
use zax_nes::rom::RomImage;
use zax_nes::{NesBusOf, NesHawkCompat, Standard, TimingProfile};

/// A powered-on bus on profile `P`, with the minimal ROM attached — `common::loaded_core` for a
/// profile other than `Standard`.
fn loaded_bus<P: TimingProfile>() -> NesBusOf<P> {
    let rom = RomImage::try_load(&minimal_rom(0x01, None, None), "Minimal.nes").unwrap();
    let mut bus = NesBusOf::<P>::new(Ppu2C02::new(), Apu2A03::new(), InputPorts::new());
    bus.attach(Some(Cartridge::create(rom).unwrap()));
    bus.ppu_mut().reset();
    let mut cpu = Cpu6502::new();
    cpu.reset(&mut bus);
    bus
}

/// Starts a long DPCM sample at the slowest rate and takes its **load** fetch, so that the sample
/// buffer is full and the memory reader's next question is a whole shift register away.
///
/// `dmc_dma_timing_tests::start_a_dmc_sample` for the `Standard` profile; the same sequence works
/// on both, because a load a `$4015` write starts is delayed on both (two or three cycles) and a
/// write cycle can be halted on neither.
fn start_a_dmc_sample<P: TimingProfile>(bus: &mut NesBusOf<P>) {
    bus.write(0x4010, 0x00); // Rate 0: 428 CPU cycles a bit, no IRQ, no loop.
    bus.write(0x4012, 0x00); // Sample at $C000.
    bus.write(0x4013, 0x0F); // 241 bytes, so no fetch below is the last one.
    bus.write(0x4015, 0x10);

    for _ in 0..4 {
        bus.write(0x0000, 0x00);
    }
    bus.read(0x8000);
    assert_eq!(1, bus.dmc_dma_count(), "{}: the load fetch ran", P::NAME);
}

/// Steps `bus` on **write** cycles — which no DMA unit may halt — until the cycle the memory
/// reader will run dry on is the next one, so that the caller's next access *is* that cycle.
///
/// `phase` extra write cycles are burnt first, which moves the parity of the cycle the request
/// rises on: that parity is what decides whether the `Standard` profile's fetch needs its
/// alignment cycle, and the `NesHawkCompat` profile's item 1b says it decides nothing.
fn step_to_the_cycle_before_the_reader_runs_dry<P: TimingProfile>(
    bus: &mut NesBusOf<P>,
    phase: u32,
) {
    for _ in 0..phase {
        bus.write(0x0000, 0x00);
    }
    let horizon = bus
        .apu()
        .cycles_until_dmc_fetch()
        .expect("the sample is still playing");
    assert!(horizon > 1, "{}: horizon {horizon}", P::NAME);
    for _ in 0..horizon - 1 {
        bus.write(0x0000, 0x00);
    }
}

/// The cycles a read cycle costs: 1 when nothing halts it, 1 plus the DMA's cycles when something
/// does.
fn cost_of_a_read<P: TimingProfile>(bus: &mut NesBusOf<P>, address: u16) -> i64 {
    let before = bus.cycle_count();
    bus.read(address);
    bus.cycle_count() - before
}

/// **Item 1a** — the halt lands on the **request cycle itself**.
///
/// On `NesHawkCompat` the read cycle the memory reader runs dry on is already halted:
/// `DMCUnit.Run()` lowers `RDY`, and `APU.RunOneFirst()` calls it before `cpu.ExecuteOne()`. On
/// `Standard` — and on Mesen 2, and on the nesdev *DMA* page read the other way — the request is
/// sampled before the cycle, so that read runs plainly and the **next** read cycle is halted.
///
/// Sixteen phases of the DMC timer, because the request's cycle is what the item is about.
#[test]
fn item_1a_the_halt_lands_on_the_request_cycle_itself() {
    for phase in 0..16 {
        let mut bus = loaded_bus::<NesHawkCompat>();
        start_a_dmc_sample(&mut bus);
        step_to_the_cycle_before_the_reader_runs_dry(&mut bus, phase);
        assert_eq!(
            5,
            cost_of_a_read(&mut bus, 0x8000),
            "neshawk, phase {phase}: the dry cycle's own read is halted, four cycles and the read"
        );
        assert_eq!(2, bus.dmc_dma_count(), "neshawk, phase {phase}");
        assert_eq!(
            1,
            cost_of_a_read(&mut bus, 0x8000),
            "neshawk, phase {phase}: the read after it is plain"
        );

        let mut bus = loaded_bus::<Standard>();
        start_a_dmc_sample(&mut bus);
        step_to_the_cycle_before_the_reader_runs_dry(&mut bus, phase);
        assert_eq!(
            1,
            cost_of_a_read(&mut bus, 0x8000),
            "standard, phase {phase}: the dry cycle's own read is not halted"
        );
        assert_eq!(
            1,
            bus.dmc_dma_count(),
            "standard, phase {phase}: and no fetch has run yet"
        );
        assert!(
            cost_of_a_read(&mut bus, 0x8000) >= 4,
            "standard, phase {phase}: the next read cycle is the halted one"
        );
        assert_eq!(2, bus.dmc_dma_count(), "standard, phase {phase}");
    }
}

/// **Item 1b** — a fetch is a **fixed four** cycles, with no get/put alignment.
///
/// `dmc_dma_countdown = 3` plus the request cycle, and there is no `% 2` anywhere on NesHawk's DMC
/// DMA path. On `Standard` the alignment cycle is spent when the cycle after the dummy read is not
/// a get cycle, so the fetch is three cycles or four.
///
/// **How the parity is moved.** Burning cycles does not move it: the memory reader's dry cycle is
/// set by the DMC timer, which counts down once per CPU cycle, so a cycle burnt before it takes a
/// cycle off the horizon and the sum is constant — and every entry of both rate tables is even, so
/// a reload's dry cycle always has the same parity (which is the phase `DmcChannel::reset` fixes
/// on purpose, for blargg's `sync_dmc`). What *does* move it is letting the dry cycle itself pass
/// on a **write** cycle, which no DMA may halt, and then putting the read one or two cycles later:
/// the halt moves with the read and the alignment follows it.
#[test]
fn item_1b_the_fetch_is_a_fixed_four_cycles_whatever_the_parity() {
    /// The halted read's cost when the dry cycle passes on a write and the read comes `late`
    /// cycles after it.
    fn cost_with_the_read_displaced<P: TimingProfile>(late: u32) -> i64 {
        let mut bus = loaded_bus::<P>();
        start_a_dmc_sample(&mut bus);
        step_to_the_cycle_before_the_reader_runs_dry(&mut bus, 0);
        for _ in 0..=late {
            bus.write(0x0000, 0x00);
        }
        assert_eq!(1, bus.dmc_dma_count(), "{}: no fetch on a write", P::NAME);
        let cost = cost_of_a_read(&mut bus, 0x8000);
        assert_eq!(2, bus.dmc_dma_count(), "{}: the fetch ran", P::NAME);
        cost
    }

    let neshawk: Vec<i64> = (0..2)
        .map(cost_with_the_read_displaced::<NesHawkCompat>)
        .collect();
    let standard: Vec<i64> = (0..2)
        .map(cost_with_the_read_displaced::<Standard>)
        .collect();

    assert_eq!(
        vec![5, 5],
        neshawk,
        "neshawk: four halted cycles and the read, at either parity"
    );
    assert!(
        standard.contains(&4) && standard.contains(&5),
        "standard: the alignment cycle is spent at one parity and not at the other, got \
         {standard:?}"
    );
}

/// The four values a program reads out of `$4015` on the four cycles after a sample's last fetch.
///
/// **The read that triggers the fetch is a read of `$4015` itself**, which is what makes the
/// window visible: a DMA unit halts the CPU on a read cycle and gives it back on the cycle after
/// the fetch, so the first cycle a program can read anything on is the one after the fetch — and
/// if that read is of `$4015`, the first value in the answer is the earliest a program can have.
/// (`dmc_dma_during_read4` is built on the same halt of an internal register.)
///
/// `$4015` answers with the register as it stood at the **start** of the cycle the read is on
/// (`Apu2A03::read_status`), so a read on cycle `F + 1 + k` reports the state at the end of
/// `F + k`, the fetch's own cycle being `F`.
fn status_after_the_last_fetch<P: TimingProfile>(irq_enabled: bool) -> Vec<u8> {
    let mut bus = loaded_bus::<P>();
    bus.write(0x4010, if irq_enabled { 0x80 } else { 0x00 });
    bus.write(0x4012, 0x00); // Sample at $C000.
    bus.write(0x4013, 0x00); // One byte: the load fetch is also the last fetch.
    bus.write(0x4015, 0x10);

    // The load fetch waits two or three cycles, and the two profiles do not pick the same number
    // (item 3 reads the DMC timer's parity where this core reads the APU cycle's). So the delay is
    // burnt on **write** cycles — which no DMA may halt — until a read *would* be the halted one,
    // asked of a clone so that the real bus is left standing on it. That way the read below is the
    // one the DMA halts on either profile.
    let mut guard = 0;
    while {
        let mut probe = bus.clone();
        probe.read(0x4015);
        probe.dmc_dma_count() == 0
    } {
        bus.write(0x0000, 0x00);
        guard += 1;
        assert!(guard < 16, "{}: the load fetch never came due", P::NAME);
    }

    let mut status = vec![bus.read(0x4015)];
    assert_eq!(1, bus.dmc_dma_count(), "{}: the one fetch ran", P::NAME);
    for _ in 0..3 {
        status.push(bus.read(0x4015));
    }
    status
}

/// **Item 2** — the sample length is applied three cycles after the fetch, so bit 4 of `$4015`
/// goes on reading **set** after the last byte of a sample has been read.
///
/// `DMCUnit.Fetch()` only sets `apu.dmc_reload_countdown = 3`; `APU.RunOneLast()` spends it and
/// decrements `sample_length` when it runs out, at the end of the fetch's cycle plus two. This
/// core — and Mesen 2 — counts the byte at the fetch itself.
///
/// Two of those three cycles are visible to a program and the third is not, on either machine: the
/// fetch's own cycle `F` is a halted cycle, so nothing can read `$4015` on it. The reads below are
/// on `F + 1`, `F + 2`, `F + 3` and `F + 4` and report the ends of `F`, `F + 1`, `F + 2` and
/// `F + 3`. It is this window that *Kirby* reads at `$823C` and branches on at `$8241`
/// (EVAL-SYNC-4 §3), which is why the item is in the profile at all.
#[test]
fn item_2_bit_4_of_4015_stays_set_after_the_last_fetch() {
    let standard = status_after_the_last_fetch::<Standard>(false);
    let neshawk = status_after_the_last_fetch::<NesHawkCompat>(false);

    let set = |status: &[u8]| status.iter().filter(|&&byte| (byte & 0x10) != 0).count();

    assert_eq!(
        0,
        set(&standard),
        "standard counts the byte at the fetch, so bit 4 is already clear: {standard:02X?}"
    );
    assert_eq!(
        2,
        set(&neshawk),
        "neshawk holds it over the two cycles of the window a program can see: {neshawk:02X?}"
    );
    assert!(
        (neshawk[0] & 0x10) != 0 && (neshawk[1] & 0x10) != 0 && (neshawk[2] & 0x10) == 0,
        "neshawk: set, set, clear — the countdown runs out at the end of F + 2: {neshawk:02X?}"
    );
}

/// **Item 2's other half** — the DMC IRQ is raised three cycles late too, because it is the same
/// `RunOneLast()` block that raises it. Read as bit 7 of `$4015`, over the same window.
#[test]
fn item_2_the_dmc_irq_is_raised_three_cycles_late() {
    let standard = status_after_the_last_fetch::<Standard>(true);
    let neshawk = status_after_the_last_fetch::<NesHawkCompat>(true);

    // A `$4015` **read** clears the frame IRQ flag but not the DMC's (nesdev), so bit 7 stays set
    // across the reads once it is up.
    assert!(
        (standard[0] & 0x80) != 0,
        "standard raises it at the fetch: {standard:02X?}"
    );
    assert!(
        (neshawk[0] & 0x80) == 0 && (neshawk[1] & 0x80) == 0 && (neshawk[2] & 0x80) != 0,
        "neshawk raises it at the end of F + 2: {neshawk:02X?}"
    );
}

/// **Item 3** — a `$4015` write with bit 4 set while a sample is **already playing** and the
/// buffer is empty turns the pending reload into a write-style fetch: displaced by two or three
/// cycles, and three cycles long instead of four.
///
/// `set_lenctr_en` tests `!sample_buffer_filled` as a sibling of the restart rather than as a
/// child of it, so `call_from_write` is set mid-sample; `Run()` then takes its `else` branch,
/// which never lowers `RDY`, and `RunCpuOne` starts the halting a cycle later — three halted
/// cycles, not four. On this core's own chip the same write queues nothing at all
/// (`DmcChannel::set_enabled` returns 0 when `bytes_remaining != 0`) and the reload runs where it
/// would have run.
///
/// Measured as "how many read cycles pass before the fetch, and what the halted one costs".
#[test]
fn item_3_a_mid_sample_4015_enable_displaces_the_fetch_and_shortens_it() {
    /// `(read cycles that ran plainly first, the halted read's cost)`.
    fn fetch_after_a_mid_sample_enable<P: TimingProfile>() -> (u32, i64) {
        let mut bus = loaded_bus::<P>();
        start_a_dmc_sample(&mut bus);
        step_to_the_cycle_before_the_reader_runs_dry(&mut bus, 0);

        // One more write cycle, so that the reader runs dry on a cycle no DMA may halt: the
        // request is now up and the sample buffer is empty, with a sample still playing — which
        // is exactly the state item 3 is about.
        bus.write(0x0000, 0x00);
        assert_eq!(1, bus.dmc_dma_count(), "{}: no fetch yet", P::NAME);

        // The mid-sample enable. A write, so it cannot be halted either.
        bus.write(0x4015, 0x10);

        for plain in 0..8 {
            let cost = cost_of_a_read(&mut bus, 0x8000);
            if bus.dmc_dma_count() == 2 {
                return (plain, cost);
            }
            assert_eq!(1, cost, "{}: an un-halted read costs one cycle", P::NAME);
        }
        panic!("{}: the fetch never ran", P::NAME);
    }

    let (standard_plain, standard_cost) = fetch_after_a_mid_sample_enable::<Standard>();
    let (neshawk_plain, neshawk_cost) = fetch_after_a_mid_sample_enable::<NesHawkCompat>();

    // This core's own chip: the write queued nothing, so the very next read cycle is halted and
    // the fetch costs its usual three or four cycles.
    assert_eq!(
        0, standard_plain,
        "standard: the next read cycle is the halted one"
    );
    assert!(
        standard_cost == 4 || standard_cost == 5,
        "standard: {standard_cost}"
    );

    // NesHawk's: the fetch is pushed two or three cycles out, and then costs three.
    assert!(
        neshawk_plain >= 2,
        "neshawk: the fetch is displaced, got {neshawk_plain} plain read cycles"
    );
    assert_eq!(
        4, neshawk_cost,
        "neshawk: three halted cycles and the read the CPU finally makes"
    );
}

/// The profile's state travels in a savestate, and only on the profile that has any.
///
/// Item 2's reload countdown is the one piece of state the two profiles do not share. It is
/// written **only** on the `NesHawkCompat` profile, which is what keeps `NesCore::STATE_VERSION`
/// and every block written before this lot exactly as they were; the check that a block of one
/// profile is refused by the other is step 4's, and rests on the core name at the head of it.
#[test]
fn the_reload_countdown_survives_a_savestate_round_trip() {
    use zax_nes::NesCoreOf;

    let mut core = NesCoreOf::<NesHawkCompat>::new();
    assert!(
        core.load_rom(&minimal_rom(0x01, None, None), "Minimal.nes")
            .success
    );

    let mut saved = Vec::new();
    core.save_state_to(false, &mut saved)
        .expect("a ROM is loaded");
    let restored = core.try_load_state(&saved, 0).expect("its own block");
    assert_eq!(
        saved.len(),
        restored.consumed,
        "the NesHawk core reads back every byte it wrote, the countdown among them"
    );
}
