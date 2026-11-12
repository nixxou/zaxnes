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

/// The halted read's cost when the memory reader's dry cycle passes on a **write** cycle — which
/// no DMA unit may halt — and the read comes `writes - 1` further write cycles after it.
///
/// `writes` is therefore the **slip**: how many cycles in a row the CPU was writing on from the
/// request, which is the quantity item 1c is about. It is also the only way to move the parity of
/// the cycle a reload's halt lands on, which is what item 1b is about — burning cycles before the
/// dry cycle does not, because the dry cycle is set by the DMC timer and a cycle burnt before it
/// takes a cycle off the horizon, so the sum is constant; and every entry of both rate tables is
/// even, so a reload's dry cycle always has the same parity (the phase `DmcChannel::reset` fixes
/// on purpose, for blargg's `sync_dmc`).
fn cost_with_the_read_displaced<P: TimingProfile>(writes: u32) -> i64 {
    let mut bus = loaded_bus::<P>();
    start_a_dmc_sample(&mut bus);
    step_to_the_cycle_before_the_reader_runs_dry(&mut bus, 0);
    for _ in 0..writes {
        bus.write(0x0000, 0x00);
    }
    assert_eq!(1, bus.dmc_dma_count(), "{}: no fetch on a write", P::NAME);
    let cost = cost_of_a_read(&mut bus, 0x8000);
    assert_eq!(2, bus.dmc_dma_count(), "{}: the fetch ran", P::NAME);
    cost
}

/// **Item 1b** — a fetch is a **fixed four** cycles, with no get/put alignment.
///
/// `dmc_dma_countdown = 3` plus the request cycle, and there is no `% 2` anywhere on NesHawk's DMC
/// DMA path. On `Standard` the alignment cycle is spent when the cycle after the dummy read is not
/// a get cycle, so the fetch is three cycles or four.
///
/// Measured at **even** slips alone (2 and 4), so that it is item 1b's parity being tested and not
/// item 1c's slip: 1c takes a cycle off an *odd* slip, and the test below is the one that says so.
#[test]
fn item_1b_the_fetch_is_a_fixed_four_cycles_whatever_the_parity() {
    let neshawk: Vec<i64> = [2, 4]
        .into_iter()
        .map(cost_with_the_read_displaced::<NesHawkCompat>)
        .collect();
    let standard: Vec<i64> = (1..5)
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

/// **Item 1c** — the halt lands on the request cycle **even when the CPU is writing on it**, so an
/// **odd** slip costs one cycle less than an even one.
///
/// NesHawk lowers `RDY` on the request cycle whatever the CPU is doing there and its write
/// microstep runs anyway; `RunCpuOne`'s `+2` then gives the fetch two cycles back once the CPU has
/// been writing for three cycles. The arithmetic (`timing_profile.rs`) is that NesHawk's stalled
/// read runs `slip + 3` cycles after the request for an odd slip and `slip + 4` for an even one,
/// where this core waits for the first read cycle and always spends `slip + 4`.
///
/// **Lot NESHAWK-PROFILE-6 retracted this item and lot NESHAWK-PROFILE-8 restored it, by
/// measurement**: 705 sample fetches of tree frames 11 to 46 of the *Kirby* `.bk2`, read off
/// BizHawk's own bus stream with no cross-machine calibration anywhere, answer slip 0 → four
/// cycles (648 of 648), slip 1 → **three** (46 of 46) and slip 2 → four (11 of 11), with no
/// exceptions. The constant's note has the table and why lot 6's table measured the slip on the
/// wrong machine.
///
/// Slips 1 to 4 — a 6502 never writes on more than three cycles in a row, so 4 is past anything a
/// program can produce and is measured only to show the rule is the slip's parity and nothing
/// else.
///
/// **The item is on** since lot NESHAWK-PROFILE-10 (its gate closed by lot NESHAWK-PROFILE-9
/// step 1: with item 12 the movie's reset lands on NesHawk's own cycle, so the slip this item
/// fires on is read off a CPU in the same place inside the instruction as NesHawk's). The test
/// profile that carried the mechanism while it waited is gone, as item 11's was; `NesHawkCompat`
/// is measured directly.
#[test]
fn item_1c_an_odd_run_of_write_cycles_costs_the_fetch_one_cycle_less() {
    let neshawk: Vec<i64> = (1..5)
        .map(cost_with_the_read_displaced::<NesHawkCompat>)
        .collect();
    let standard: Vec<i64> = (1..5)
        .map(cost_with_the_read_displaced::<Standard>)
        .collect();

    assert_eq!(
        vec![4, 5, 4, 5],
        neshawk,
        "neshawk: three halted cycles and the read at an odd slip, four and the read at an even one"
    );
    assert_ne!(
        neshawk, standard,
        "the item must be visible: the two profiles answer the same four slips differently"
    );
    // What `Standard` does instead, pinned so that a change to *its* rule fails here too: the
    // halt lands on the first read cycle and the alignment cycle is spent or not by **that
    // cycle's** parity, which is a different function of the slip from the one above.
    assert_eq!(
        vec![5, 4, 5, 4],
        standard,
        "standard: the alignment follows the halted read's own parity"
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

/// **Item 7** — a DMC fetch that falls inside an OAM transfer makes that transfer **one cycle
/// longer** than it costs this core.
///
/// NesHawk's `RunCpuOne` gates its OAM block on `apu.dmc_dma_countdown != 1 && !dmc_realign`, and
/// the DMC block sets `dmc_realign` on the fetch cycle, so the transfer stands still on the fetch
/// cycle *and on the one after it*. This core's `run_dma` lets the DMC's halt and re-read cycles
/// double as the transfer's own and only the fetch cycle is taken from it.
///
/// Measured at the fastest DMC rate, so that a fetch certainly lands inside the 513 cycles: the
/// test asks the bus how many fetches ran inside the transfer and holds the total against
/// `513/514 + 1` per fetch on `Standard` and `+ 2` per fetch on `NesHawkCompat`.
#[test]
fn item_7_a_fetch_inside_an_oam_transfer_costs_the_transfer_a_cycle_more() {
    /// `(the halted read's cost, the fetches that ran inside it, the parity of the `$4014` write's
    /// own cycle)`.
    ///
    /// `shift` write cycles are burnt first, which moves the whole set-up — and so the cycle the
    /// inside fetch lands on — by that many cycles: one of the two phases is the one this core
    /// spends an alignment cycle on and the other is not, and that is what makes the test
    /// differential. Without the sweep both profiles answer two cycles at this set-up's own phase
    /// and the item cannot be seen.
    fn transfer_with_a_fetch_inside<P: TimingProfile>(shift: u32) -> (i64, i64, i64) {
        let mut bus = loaded_bus::<P>();
        for _ in 0..shift {
            bus.write(0x0000, 0x00);
        }
        bus.write(0x4010, 0x0F); // The fastest rate: 54 cycles a bit, 432 a byte.
        bus.write(0x4012, 0x00);
        bus.write(0x4013, 0x0F);
        bus.write(0x4015, 0x10);
        // Three bytes of the sample, so that the output unit is in its periodic rhythm and the
        // next request is a whole shift register (432 cycles) away rather than wherever the
        // power-on timer phase left it — `$4010` sets the rate index and does not reload the
        // timer, as the hardware does not.
        for _ in 0..4000 {
            if bus.dmc_dma_count() >= 3 {
                break;
            }
            bus.read(0x8000);
        }
        let fetches = bus.dmc_dma_count();
        assert_eq!(3, fetches, "{}: the sample is playing", P::NAME);
        let parity = bus.cycle_count() % 2;
        bus.write(0x4014, 0x02); // The transfer: it halts the next read cycle.
        let cost = cost_of_a_read(&mut bus, 0x8000);
        (cost, bus.dmc_dma_count() - fetches, parity)
    }

    /// The same transfer with **no** DPCM sample at all, its `$4014` write lined up on the same
    /// parity: each profile's own base, which is what the inside fetch is charged against.
    ///
    /// It has to be each profile's own, and at that parity, since item 13: the two profiles take
    /// the hardware's 513 and 514 cycles on **opposite** phases of the write's own cycle, so a
    /// base read off the other profile — which is how this test was written until lot
    /// NESHAWK-PROFILE-9 — is the wrong figure by one.
    fn transfer_alone<P: TimingProfile>(parity: i64) -> i64 {
        let mut bus = loaded_bus::<P>();
        while bus.cycle_count() % 2 != parity {
            bus.write(0x0000, 0x00); // No DMA unit may halt a write cycle.
        }
        bus.write(0x4014, 0x02);
        cost_of_a_read(&mut bus, 0x8000)
    }

    /// What one inside fetch costs the transfer, on profile `P`, at both phases.
    fn per_fetch<P: TimingProfile>() -> Vec<i64> {
        (0..2)
            .map(|shift| {
                let (cost, fetches, parity) = transfer_with_a_fetch_inside::<P>(shift);
                let base = transfer_alone::<P>(parity);
                assert_eq!(
                    1,
                    fetches,
                    "{}, shift {shift}: one fetch inside the transfer, got {fetches} in {cost} \
                     cycles",
                    P::NAME
                );
                // 513 or 514 for the transfer and one cycle for the CPU's own read.
                assert!(
                    (514..=515).contains(&base),
                    "{}, shift {shift}: a transfer with no fetch inside it is 513 or 514 cycles, \
                     got {base}",
                    P::NAME
                );
                cost - base
            })
            .collect()
    }

    let standard = per_fetch::<Standard>();
    let neshawk = per_fetch::<NesHawkCompat>();
    println!("  cycles per inside fetch: standard {standard:?}  neshawk {neshawk:?}");

    // **The item**: NesHawk's `dmc_realign` skips the OAM cycle *after* the fetch as well, so its
    // transfer loses **two** cycles to a fetch that falls inside it, at both phases.
    assert_eq!(vec![2, 2], neshawk, "neshawk: two cycles per inside fetch");
    // **And `Standard` loses two here as well, which is the honest reading of this set-up** (lot
    // NESHAWK-PROFILE-9 step 3): its fetch may only read on a **get** cycle, so it takes the cycle
    // the transfer would have read on and the pair that follows moves by two. The two profiles
    // arrive at the same figure by different routes, which is why item 7 was measured *exactly
    // inert* on the *Kirby* movie in lot NESHAWK-PROFILE-2 step 4.
    //
    // Until this lot the test read the base off `Standard` and asserted NesHawk was two cycles
    // above it, which looked like a difference of one cycle between the profiles. It was **item
    // 13's** cycle: the two transfers are 513 and 514 on opposite phases of the `$4014` write, so
    // a base taken from the other profile is the wrong figure by one. The base is now each
    // profile's own, at the same phase, and the difference it was pinning is gone.
    assert_eq!(
        vec![2, 2],
        standard,
        "standard: the get cycle the fetch takes and the one the pair moves by"
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

/// **Item 10**: the PPU runs a start-up frame after a power-on and after a soft reset.
///
/// Differential, and of the one quantity the item is about: the CPU cycles a profile spends in
/// its **first** frame, counted from a power-on and again from a soft reset. NesHawk's PPU runs
/// `241 * 341 - start_up_offset` dots there (`PPU.run.cs` `NewDeadPPU`, with `ppudead = 1` and
/// `start_up_offset = 4` from `PPU.cs` `Reset()`), where this core's runs a whole frame from its
/// own power-on position at the start of vblank — and NesHawk's `regs_reset()` puts its PPU back
/// to the top of the frame on a **soft** reset too, where `Ppu2C02::soft_reset` deliberately does
/// not (Q-078).
///
/// The second and later frames must be a frame long on both, or the item would be a permanent
/// difference of the frame rate rather than of the start-up.
#[test]
fn item_10_the_ppu_runs_a_start_up_frame_after_a_power_on_and_a_reset() {
    /// `(the first frame from the power-on, the next one, the first frame after a soft reset)`.
    fn frames<P: TimingProfile>() -> (i64, i64, i64) {
        use zax_nes::{NesCoreOf, RunFrame as _};

        let mut core = NesCoreOf::<P>::new();
        assert!(
            core.load_rom(&minimal_rom(0x01, None, None), "Minimal.nes")
                .success
        );
        let first = core.run_frame(0);
        let second = core.run_frame(0);
        core.soft_reset();
        let after_reset = core.run_frame(0);
        (first, second, after_reset)
    }

    let plain = frames::<Standard>();
    let hawk = frames::<NesHawkCompat>();
    println!("  standard {plain:?}");
    println!("  neshawk  {hawk:?}");

    // A frame is 29 780 or 29 781 cycles and `run_frame` stops at the end of the instruction that
    // crossed the boundary, so a frame as it is *reported* is a frame give or take an instruction.
    // The second frame is one on both profiles: this item is about the start-up and nothing else.
    for (name, length) in [
        ("standard, the second frame", plain.1),
        ("neshawk, the second frame", hawk.1),
        ("standard, the first frame", plain.0),
    ] {
        assert!(
            (29_770..=29_785).contains(&length),
            "{name} is a frame long, got {length}"
        );
    }

    // And NesHawk's first is its start-up frame: `241 * 341 - 4 = 82 177` dots, which at three
    // dots a CPU cycle is 27 392 cycles and so 2 389 short of a frame.
    assert!(
        (27_380..=27_395).contains(&hawk.0),
        "neshawk: the first frame is the start-up frame of {} dots, got {}",
        241 * 341 - 4,
        hawk.0
    );
    assert!(
        (2_380..=2_395).contains(&(plain.0 - hawk.0)),
        "the start-up frame is 2 389 cycles short of a frame, got {}",
        plain.0 - hawk.0
    );

    // The reset does it again, and to the dot: the two start-up frames are the same length
    // because NesHawk's soft reset puts the PPU back to the top of the frame as a power-on does.
    assert_eq!(
        hawk.0, hawk.2,
        "neshawk: the frame a soft reset lands in is the same start-up frame"
    );
    // The standard profile's reset does not restart the frame at all (Q-078), so its own frame
    // after one is a frame minus however far into the frame the reset fell — never 27 392.
    assert_ne!(
        hawk.2, plain.2,
        "standard: the soft reset does not restart the frame, so it is not the start-up frame"
    );

    // And the half of the item a frame-level measurement cannot see: the start-up frame is
    // 82 177 dots **from the top of the frame**, wherever the reset found the PPU. NesHawk gets
    // that from `regs_reset()`'s `ppur = new PPUREGS()`; this core has to put the position back,
    // which is what `Ppu2C02::run_start_up_dots` does before it counts.
    let position = |lag: u32| {
        let mut ppu = Ppu2C02::new();
        ppu.reset();
        ppu.run_cpu_cycles(lag, None);
        ppu.run_start_up_dots(NesHawkCompat::PPU_START_UP_DOTS_SKIPPED, None);
        (ppu.scanline(), ppu.dot())
    };
    assert_eq!(
        position(0),
        position(1_234),
        "the start-up frame starts from the top of the frame, not from where the reset fell"
    );
}

/// **Item 11** — the DMC's timer powers on 1 020 CPU cycles from its first bit, not one period.
///
/// NesHawk's `DMCUnit` constructor sets `timer = 1020` beside `timer_reload = DMC_RATE[0]` (428 on
/// NTSC), and `Run()` counts it down one per CPU cycle out of `RunOneFirst()`, so its output unit's
/// first clock is on CPU cycle 1 020 where this core's is on 428. Both units then take the sample
/// buffer on every **eighth** output clock, so the phase of every sample fetch of a game is
/// `1020 - 428 = 592` cycles apart for ever.
///
/// Measured through the fetches themselves rather than through the timer, because the timer is not
/// public and the fetches are what the difference is *about*: a long sample is started a few
/// hundred cycles after the power-on, and the cycle of the **second** fetch — the first one the
/// output unit asks for, as against the one the `$4015` write starts — is the phase.
///
/// **The item is on** since lot NESHAWK-PROFILE-6 answered Q-182 against item 1c, which is what
/// lot NESHAWK-PROFILE-5 shipped it off and waiting for. The test profile that carried it while it
/// was off is gone with the wait; `NesHawkCompat` is measured directly.
#[test]
fn item_11_the_dmc_timer_powers_on_1020_cycles_from_its_first_bit() {
    /// The CPU cycle the memory reader next runs dry on, counted from the power-on itself.
    ///
    /// The **request** and not the fetch: item 1b makes a `Standard` fetch cost three cycles or
    /// four by parity where a NesHawk one is a fixed four, so the cycle a fetch *ends* on carries
    /// that item's cycle as well as this one's. The request is the output unit's own eighth clock
    /// and nothing else.
    fn second_request<P: TimingProfile>() -> i64 {
        let rom = RomImage::try_load(&minimal_rom(0x01, None, None), "Minimal.nes").unwrap();
        let mut bus = NesBusOf::<P>::new(Ppu2C02::new(), Apu2A03::new(), InputPorts::new());
        bus.attach(Some(Cartridge::create(rom).unwrap()));
        bus.ppu_mut().reset();
        // The one call that reads the item, and the origin every cycle below is counted from:
        // `nes_core.rs` `power_on` runs it before the CPU's own seven reset cycles.
        bus.apu_mut().reset::<P>();
        let origin = bus.cycle_count();

        bus.write(0x4010, 0x00); // Rate 0: 428 CPU cycles a bit, 3 424 a byte, no IRQ, no loop.
        bus.write(0x4012, 0x00); // Sample at $C000.
        bus.write(0x4013, 0x0F); // 241 bytes, so none of the fetches below is the last.
        bus.write(0x4015, 0x10); // Start it, which starts a write-style fetch of its own.

        // Read cycles of PRG ROM, which have no side effect and which a DMA unit **may** halt, so
        // that the write-started load fetch is taken at all: a loop of writes would never take it,
        // because a write cycle can be halted on neither profile.
        while bus.dmc_dma_count() < 1 {
            bus.read(0x8000);
            assert!(
                bus.cycle_count() - origin < 1_000,
                "{}: the load fetch a $4015 write starts is taken at once",
                P::NAME
            );
        }

        // And now the horizon the memory reader itself reports, which is the output unit's eighth
        // clock counted from here.
        let horizon = bus
            .apu()
            .cycles_until_dmc_fetch()
            .expect("the sample is playing");
        bus.cycle_count() - origin + i64::from(horizon)
    }

    let plain = second_request::<Standard>();
    let hawk = second_request::<NesHawkCompat>();
    println!("  standard {plain}, neshawk {hawk}, apart {}", hawk - plain);

    // The output unit takes the buffer on its eighth clock, so with the timer started at `T` the
    // second fetch is a cycle or two after `T + 1 + 7 * 428`. `Standard`'s `T` is `428 - 1`.
    let expected = |timer: i32| i64::from(timer) + 1 + 7 * 428;
    for (name, measured, timer) in [
        ("standard", plain, 428 - 1),
        ("neshawk", hawk, NesHawkCompat::DMC_POWER_ON_TIMER),
    ] {
        let want = expected(timer);
        assert!(
            (want..=want + 8).contains(&measured),
            "{name}: the second fetch is the eighth output clock from a timer of {timer}, so near \
             {want}; got {measured}"
        );
    }

    // And the item itself, which is the gap and not either number: 1 020 - 428.
    assert_eq!(
        592,
        hawk - plain,
        "the two phases are `1020 - 428` cycles apart"
    );

    // The half the fetch cycle cannot show: 592 is **even**, so the parity a reload's alignment
    // cycle turns on — `DmcChannel::reset`'s own note, and what blargg's `sync_dmc` loop in
    // `dmc_dma_during_read4` and `sprdma_and_dmc_dma` depends on — is the same either way. That is
    // what made the item safe to *try*; what stopped it being shipped is the movie, below.
    assert_eq!(
        0,
        (hawk - plain) % 2,
        "the item moves the phase by an even number of cycles"
    );

    // **The record of the choice.** The item ships ON, which is `1019` here and NesHawk's own
    // `timer = 1020` under the convention this core's timer runs one lower than NesHawk's at every
    // instant (`DmcChannel::clock_timer`, and the parity trap in `docs/review/NESHAWK-PROFILE.md`).
    assert_eq!(
        1019,
        NesHawkCompat::DMC_POWER_ON_TIMER,
        "item 11 ships ON: this is NesHawk's `timer = 1020` in this core's convention"
    );
    assert_ne!(
        plain, hawk,
        "and the two profiles do not power their DMC timer on alike"
    );
}

/// **Item 13** — an OAM transfer's wait cycle is at the **end**, so the two profiles charge the
/// hardware's own 513 and 514 cycles on **opposite** phases.
///
/// Measured first, from BizHawk's own bus stream over the frames where the two machines are in step
/// (lot NESHAWK-PROFILE-9 step 3, round 1): every transfer of that window reads on an even reported
/// cycle, spends one leading dead cycle when the `$4014` write lands on an even one and none when
/// it lands on an odd one, and ends with **one cycle carrying no bus access at all**. This core
/// reads on the same cycles — a reported cycle is this core's plus one — and instead spends the
/// halt cycle and an alignment cycle when it needs one, with nothing at the end. The totals are
/// therefore not offset but **swapped**, which is the one thing neither a cycle at the boot nor a
/// parity flip can produce, and this test is that swap.
#[test]
fn item_13_an_oam_transfer_pays_its_wait_cycle_at_the_end() {
    /// The transfer's own cost in CPU cycles, the `$4014` write displaced by `phase` write cycles.
    fn transfer<P: TimingProfile>(phase: u32) -> i64 {
        let mut bus = loaded_bus::<P>();
        for _ in 0..phase {
            bus.write(0x0000, 0x00);
        }
        bus.write(0x4014, 0x02);
        // The halted read's cost, less the CPU's own read, is the transfer.
        cost_of_a_read(&mut bus, 0x8000) - 1
    }

    let standard: Vec<i64> = (0..4).map(transfer::<Standard>).collect();
    let neshawk: Vec<i64> = (0..4).map(transfer::<NesHawkCompat>).collect();
    println!("  standard {standard:?}  neshawk {neshawk:?}");

    // Both machines take one of the hardware's own two figures on every phase (nesdev, "DMA").
    for (name, costs) in [("standard", &standard), ("neshawk", &neshawk)] {
        assert!(
            costs.iter().all(|cost| (513..=514).contains(cost)),
            "{name}: a transfer is 513 or 514 cycles, got {costs:?}"
        );
        assert!(
            costs.contains(&513) && costs.contains(&514),
            "{name}: both figures are reachable by moving the write's own cycle, got {costs:?}"
        );
    }
    // And on each phase they take the other one: that is the item.
    for (phase, (plain, hawk)) in standard.iter().zip(&neshawk).enumerate() {
        assert_eq!(
            1027,
            plain + hawk,
            "phase {phase}: the two profiles take the two figures the other way round, \
             got standard {plain} and neshawk {hawk}"
        );
    }
}

/// **Item 12** — a frame the movie taps RESET after ends at the PPU dot the frame boundary falls
/// on, before the instruction that straddles it.
///
/// NesHawk clocks the two chips together: `PPU.run.cs` `TickPPU_preVBL()` sets
/// `nes.frame_is_done = true` at a dot, with the CPU clocked from inside the same loop, and
/// `NES.Core.cs` `FrameAdvance` opens the next frame with `cpu.NESSoftReset()` — `opcode =
/// VOP_RESET; mi = 0;`, which abandons the instruction in flight. So its reset's seven cycles
/// begin on the boundary dot's own CPU cycle. This core's frame loop breaks *after* the straddling
/// instruction, which on the *Kirby* `.bk2` put the reset **3 CPU cycles late** and every DMC
/// fetch of the rest of the movie on the wrong cycle of the game's code (lot NESHAWK-PROFILE-8
/// step 1, lot NESHAWK-PROFILE-9 step 1).
///
/// Measured as the two frames the same state can run: the ordinary one and
/// [`NesCoreOf::run_frame_before_soft_reset`]'s. The gap is how much of the straddling instruction
/// the ordinary frame runs past the dot, so it is 0 when the dot does not fall on an instruction's
/// **first** cycle — this core cannot stop between an instruction's cycles — and the minimal ROM's
/// `JMP $8000` loop is 3 cycles, so the dot walks through all three over consecutive frames and
/// the table below is the whole behaviour.
#[test]
fn item_12_the_frame_before_a_reset_ends_at_the_boundary_dot() {
    use zax_nes::{NesCoreOf, RunFrame as _};

    /// `(the ordinary frame's length, the before-reset frame's length, and whether each left the
    /// PPU past the boundary dot)`, both run from the state `frames` ordinary frames leave.
    fn pair<P: TimingProfile>(frames: u32) -> (i64, i64, bool, bool) {
        let build = || {
            let mut core = NesCoreOf::<P>::new();
            assert!(
                core.load_rom(&minimal_rom(0x01, None, None), "Minimal.nes")
                    .success
            );
            core
        };
        let (mut ordinary, mut early) = (build(), build());
        for _ in 0..frames {
            ordinary.run_frame(0);
            early.run_frame(0);
        }
        (
            ordinary.run_frame(0),
            early.run_frame_before_soft_reset(0),
            ordinary.bus().ppu().vblank_just_started(),
            early.bus().ppu().vblank_just_started(),
        )
    }

    let mut gaps = (Vec::new(), Vec::new());
    for frames in 0..8 {
        let plain = pair::<Standard>(frames);
        let hawk = pair::<NesHawkCompat>(frames);
        println!("  after {frames} frame(s): standard {plain:?}  neshawk {hawk:?}");

        // `Standard`: the two calls are the same frame, to the cycle and to the flag.
        assert_eq!(
            (plain.0, plain.2),
            (plain.1, plain.3),
            "standard: a frame before a reset is the frame it always was"
        );
        assert!(plain.2, "standard: the frame ends past the boundary dot");

        // `NesHawkCompat`: the before-reset frame never runs **past** the ordinary one, and when
        // it ends earlier it ends short of the dot — which is the item.
        assert!(
            hawk.1 <= hawk.0 && hawk.0 - hawk.1 < 8,
            "neshawk: the before-reset frame gives up part of one instruction, got {} against {}",
            hawk.1,
            hawk.0
        );
        assert!(
            hawk.2,
            "neshawk: the ordinary frame ends past the boundary dot"
        );
        assert_eq!(
            hawk.3,
            hawk.0 == hawk.1,
            "neshawk: the before-reset frame has passed the boundary dot exactly when it did not \
             end early, got {hawk:?}"
        );
        gaps.0.push(plain.0 - plain.1);
        gaps.1.push(hawk.0 - hawk.1);
    }

    println!("  gaps: standard {:?}  neshawk {:?}", gaps.0, gaps.1);
    assert_eq!(gaps.0, vec![0; 8], "standard: the item is not implemented");
    // The item has to *fire*: the dot falls on the first cycle of the `JMP` on a third of the
    // frames, and a change that stopped the early break would otherwise pass every assertion
    // above by doing nothing at all.
    assert!(
        gaps.1.contains(&3),
        "neshawk: not one of the eight frames ended at the boundary dot, got {:?}",
        gaps.1
    );
}

// ---------------------------------------------------------------- the interrupt-phase ROM
//
// Lot NESHAWK-PROFILE-3 step 2. The *Kirby* `.bk2` parts because the two machines take the
// vblank NMI at different instruction boundaries of the game's frame-sync spin loop, which is
// `LDA $39` / `BEQ` taken. This ROM is that construct isolated, with the one variable the game
// cannot vary: whether the taken branch crosses a page.
//
// It is a ROM of this tree's own, written from the public 6502 documentation; nothing about it
// comes from BizHawk. The same bytes are what BizHawk is given (see
// `the_interrupt_phase_rom_is_written_out_for_bizhawk`), so the two machines answer the same
// program rather than two copies of it.

/// Where in the 32 KiB PRG the NMI handler and the two spin loops sit, as CPU addresses.
const PHASE_NMI: u16 = 0x8080;
const PHASE_SPIN_A: u16 = 0x8100;
const PHASE_SPIN_B: u16 = 0x81FE;

/// Frames each variant is measured over. A frame is 29 780 or 29 781 CPU cycles and the loop's
/// period is 6 or 7, so the loop's phase relative to the vblank walks by itself and 64 frames
/// cover every phase many times — no delay slide has to be written.
const PHASE_FRAMES: usize = 0x40;

/// The program of the interrupt-phase ROM, as the 32 KiB PRG image starting at `$8000`.
///
/// `docs/progress/NESHAWK-PROFILE-3.md` step 2 carries the same listing with its comments.
fn phase_program() -> Vec<u8> {
    let mut prg = vec![0u8; 0x8000];
    let mut put = |at: u16, bytes: &[u8]| {
        let start = usize::from(at) - 0x8000;
        prg[start..start + bytes.len()].copy_from_slice(bytes);
    };
    put(
        0x8000,
        &[
            0x78, // SEI
            0xD8, // CLD
            0xA2, 0xFF, // LDX #$FF
            0x9A, // TXS
            0xA9, 0x00, // LDA #$00
            0x8D, 0x00, 0x20, // STA $2000   NMI off while warming up
            0x8D, 0x01, 0x20, // STA $2001
            0xAD, 0x02, 0x20, // LDA $2002   two vblanks
            0x10, 0xFB, // BPL $800D
            0xAD, 0x02, 0x20, // LDA $2002
            0x10, 0xFB, // BPL $8012
            0xA9, 0x00, // LDA #$00    ($02) = $0300: variant A's answers
            0x85, 0x02, // STA $02
            0xA9, 0x03, // LDA #$03
            0x85, 0x03, // STA $03
            0xA0, 0x00, // LDY #$00
            0xA9, 0x80, // LDA #$80
            0x8D, 0x00, 0x20, // STA $2000   NMI on
            // variant A, at $8026
            0xA9, 0x00, // LDA #$00
            0x85, 0x00, // STA $00     lower the semaphore
            0x4C, 0x00, 0x81, // JMP $8100
            0xC8, // INY         <- $802D, the handler raised it
            0xC0, 0x40, // CPY #$40
            0xD0, 0xF4, // BNE $8026
            // variant B, at $8032
            0xA9, 0x04, // LDA #$04    ($02) = $0400
            0x85, 0x03, // STA $03
            0xA0, 0x00, // LDY #$00
            0xA9, 0x00, // LDA #$00    <- $8038
            0x85, 0x00, // STA $00
            0x4C, 0xFE, 0x81, // JMP $81FE
            0xC8, // INY         <- $803F
            0xC0, 0x40, // CPY #$40
            0xD0, 0xF4, // BNE $8038
            // done, at $8044
            0xA9, 0x00, // LDA #$00
            0x8D, 0x00, 0x20, // STA $2000   NMI off: nothing writes past the two tables
            0xA9, 0x5A, // LDA #$5A
            0x8D, 0xFF, 0x02, // STA $02FF   the host's "finished" flag
            0x4C, 0x4E, 0x80, // JMP $804E
        ],
    );
    put(
        PHASE_NMI,
        &[
            0x48, // PHA
            0x8A, // TXA
            0x48, // PHA
            0xBA, // TSX          S = s-5, so the pushed PCL is at $0104+X
            0xBD, 0x04, 0x01, // LDA $0104,X
            0x91, 0x02, // STA ($02),Y  the answer: the interrupted PC's low byte
            0xA9, 0x01, // LDA #$01
            0x85, 0x00, // STA $00      raise the semaphore
            0x68, // PLA
            0xAA, // TAX
            0x68, // PLA
            0x40, // RTI
        ],
    );
    // Variant A: `(byte)PC` is $04 and the displacement -4, so the sum does not borrow and the
    // branch does NOT cross a page: three cycles, and NesHawk's `branch_irq_hack` applies.
    put(
        PHASE_SPIN_A,
        &[
            0xA5, 0x00, // LDA $00
            0xF0, 0xFC, // BEQ $8100
            0x4C, 0x2D, 0x80, // JMP $802D
        ],
    );
    // Variant B: `(byte)PC` is $02 and the displacement -4, so the sum borrows and the branch
    // DOES cross a page: four cycles, and NesHawk's `branch_irq_hack` does not apply.
    put(
        PHASE_SPIN_B,
        &[
            0xA5, 0x00, // LDA $00
            0xF0, 0xFC, // BEQ $81FE
            0x4C, 0x3F, 0x80, // JMP $803F
        ],
    );
    prg
}

/// The whole `.nes` image of the interrupt-phase ROM.
fn phase_rom() -> Vec<u8> {
    minimal_rom(0x01, Some(&phase_program()), Some(PHASE_NMI))
}

/// Runs the interrupt-phase ROM on profile `P` and answers its two tables of 64 answers:
/// variant A's (`$0300`) and variant B's (`$0400`).
fn phase_tables<P: TimingProfile>() -> (Vec<u8>, Vec<u8>) {
    use zax_nes::{NesCoreOf, RunFrame as _};

    let mut core = NesCoreOf::<P>::new();
    assert!(core.load_rom(&phase_rom(), "InterruptPhase.nes").success);
    // Two variants of 64 frames each, plus the warm-up: 200 is comfortably enough, and the
    // ROM's own flag says whether it finished rather than the count being trusted.
    for _ in 0..200 {
        core.run_frame(0);
        if core.read_memory(0x02FF) == 0x5A {
            break;
        }
    }
    assert_eq!(
        0x5A,
        core.read_memory(0x02FF),
        "{}: the phase ROM finished both variants",
        P::NAME
    );
    let read = |base: u16| {
        (0..PHASE_FRAMES)
            .map(|i| core.read_memory(base + u16::try_from(i).unwrap()))
            .collect::<Vec<u8>>()
    };
    (read(0x0300), read(0x0400))
}

/// What the two profiles answer the interrupt-phase ROM — printed, so that step 2's table can be
/// read, and asserted on the shape both machines must have.
#[test]
fn interrupt_phase_the_two_profiles_answer_the_spin_loop() {
    let (standard_a, standard_b) = phase_tables::<Standard>();
    let (neshawk_a, neshawk_b) = phase_tables::<NesHawkCompat>();
    let show = |name: &str, table: &[u8]| {
        let text: Vec<String> = table.iter().map(|byte| format!("{byte:02X}")).collect();
        println!("  {name}: {}", text.join(" "));
    };
    show("standard A (no page cross)", &standard_a);
    show("neshawk  A (no page cross)", &neshawk_a);
    show("standard B (page cross)   ", &standard_b);
    show("neshawk  B (page cross)   ", &neshawk_b);

    // Variant A can only ever report the `LDA` at $8100 or the `BEQ` at $8102 as the instruction
    // the NMI displaced; variant B the `LDA` at $81FE or the `BEQ` at $8200.
    for (name, table, wanted) in [
        ("standard A", &standard_a, [0x00, 0x02]),
        ("neshawk A", &neshawk_a, [0x00, 0x02]),
        ("standard B", &standard_b, [0xFE, 0x00]),
        ("neshawk B", &neshawk_b, [0xFE, 0x00]),
    ] {
        assert!(
            table.iter().all(|byte| wanted.contains(byte)),
            "{name}: every answer is one of the loop's two instructions, got {table:02X?}"
        );
    }
    // And both profiles must exercise both answers, or the ROM measured nothing: the loop's
    // phase walks because a frame is not a multiple of its period.
    for (name, table) in [
        ("standard A", &standard_a),
        ("neshawk A", &neshawk_a),
        ("standard B", &standard_b),
        ("neshawk B", &neshawk_b),
    ] {
        assert!(
            table.iter().any(|byte| *byte != table[0]),
            "{name}: the phase walked, so the table is not one value, got {table:02X?}"
        );
    }
}

/// Writes the interrupt-phase ROM out, so that BizHawk can be given **the bytes this test ran**.
///
/// Ignored by default because it writes a file. `ZAXNES_DUMP_ROM=<path> cargo test -p zax-nes
/// --test neshawk_profile_tests interrupt_phase_rom -- --ignored`.
#[test]
#[ignore = "writes a file; the instrument of NESHAWK-PROFILE-3 step 2"]
fn the_interrupt_phase_rom_is_written_out_for_bizhawk() {
    let Ok(path) = std::env::var("ZAXNES_DUMP_ROM") else {
        println!("  set ZAXNES_DUMP_ROM to the .nes path to write");
        return;
    };
    let image = phase_rom();
    std::fs::write(&path, &image).expect("the path is writable");
    println!("  wrote {} bytes to {path}", image.len());
}
