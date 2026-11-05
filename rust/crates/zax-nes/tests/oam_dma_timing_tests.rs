//! OAM DMA: the 513 or 514 cycles it steals from the CPU and the transfer itself
//! (nesdev "DMA"; REQ-CORE-110 cycle accounting, REQ-CORE-111 timing).
//!
//! A `$4014` write is a **write** cycle, and a DMA unit may only halt the CPU on a read, so the
//! transfer starts on the next cycle the CPU reads on — for real code the opcode fetch that
//! follows the `STA $4014`. These tests therefore drive that read themselves rather than
//! expecting the whole transfer to happen inside the write.

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used)]

mod common;

use std::fmt::Write as _;

use common::loaded_core;
use zax_nes::cpu::CpuBus;
use zax_nes::state::{StateReader, StateWriter};
use zax_nes::NesBus;

/// The write to `$4014`, then the read the DMA unit halts on, as a CPU would make them.
/// Returns the cycles stolen from the CPU: everything beyond the two cycles the two accesses
/// cost on their own.
fn oam_dma_stall(bus: &mut NesBus, page: u8) -> i64 {
    let before = bus.cycle_count();
    bus.write(0x4014, page);
    bus.read(0x8000);
    bus.cycle_count() - before - 2
}

#[test]
fn req_core_111_oam_dma_steals_513_cycles_on_an_even_cycle_and_514_on_an_odd_one() {
    let mut stalls = Vec::new();

    for padding in [0, 1] {
        let (_, mut bus) = loaded_core(None);

        // The reset sequence left the clock at 7; one extra tick flips its parity.
        for _ in 0..padding {
            bus.tick();
        }

        stalls.push(oam_dma_stall(&mut bus, 0x02));
        assert_eq!(1, bus.oam_dma_count());
    }

    stalls.sort_unstable();
    assert_eq!(vec![513, 514], stalls);
    assert!(stalls.contains(&NesBus::OAM_DMA_CYCLES_EVEN));
    assert!(stalls.contains(&NesBus::OAM_DMA_CYCLES_ODD));
}

/// The halt may not land on a write: a `$4014` write followed by more writes leaves the DMA
/// unit waiting, and the transfer starts on the first read after them (nesdev, "DMA": "If the
/// CPU is writing, it ignores the halt and the DMA unit waits until the next cycle to try
/// again").
#[test]
fn req_core_111_the_oam_transfer_waits_for_a_read_cycle() {
    let (_, mut bus) = loaded_core(None);

    bus.write(0x4014, 0x02);
    let after_request = bus.cycle_count();
    for _ in 0..3 {
        bus.write(0x0010, 0x00);
    }
    assert_eq!(3, bus.cycle_count() - after_request);

    bus.read(0x8000);
    let stall = bus.cycle_count() - after_request - 4;
    assert!([513, 514].contains(&stall), "{stall}");
}

#[test]
fn req_core_111_oam_dma_copies_the_whole_page_into_oam() {
    let (_, mut bus) = loaded_core(None);

    for i in 0..=255u8 {
        bus.poke(0x0200 + u16::from(i), 255 - i);
    }

    oam_dma_stall(&mut bus, 0x02);

    for i in 0..=255u8 {
        // Bits 2 to 4 of a sprite's byte 2 do not exist in the chip and read back as 0
        // (nesdev, "PPU OAM"; oam_read, ppu_open_bus test 10).
        let expected = if (i & 0x03) == 0x02 {
            (255 - i) & 0xE3
        } else {
            255 - i
        };

        assert_eq!(expected, bus.ppu().oam()[usize::from(i)]);
    }
}

#[test]
fn req_core_111_oam_dma_starts_at_the_current_oam_address() {
    let (_, mut bus) = loaded_core(None);

    for i in 0..=255u8 {
        bus.poke(0x0300 + u16::from(i), i);
    }

    bus.poke(0x2003, 0x10); // OAMADDR = $10.
    oam_dma_stall(&mut bus, 0x03);

    // The transfer wraps around inside OAM.
    let oam = bus.ppu().oam();
    assert_eq!(0x00, oam[0x10]);
    assert_eq!(0xEF, oam[0xFF]);
    assert_eq!(0xF0, oam[0x00]);
    assert_eq!(0xFF, oam[0x0F]);
}

/// A transfer a `$4014` write has only **asked** for does not survive a state load, because
/// nothing of `NesBus::dma` is in the savestate format (REQ-CORE-090, REQ-CORE-091).
///
/// This pins a **divergence**, not a requirement: the write is a write cycle, so the transfer
/// waits for the next read cycle, and the instruction boundary in between is a valid save point.
/// `load_state` clears `dma`, so the state reloads as if the `STA $4014` had not run. The window,
/// its scope and the three fields a fix would add behind a block version are in `Q-120`; a lot
/// that fixes it should fail here, read the ticket, and replace this test with its opposite.
///
/// A transfer that is *running* genuinely cannot be caught: all 513 or 514 of its cycles happen
/// inside one `CpuBus::read`.
// Q-120
#[test]
fn req_core_091_a_pending_oam_transfer_is_dropped_by_a_state_load() {
    let (_, mut bus) = loaded_core(None);
    for i in 0..=255u8 {
        bus.poke(0x0200 + u16::from(i), 255 - i);
    }

    bus.write(0x4014, 0x02);
    let mut writer = StateWriter::with_capacity(0x1000);
    bus.save_state(&mut writer);
    let bytes = writer.into_vec();

    // Without the save and load in between, this read would steal 513 or 514 cycles and fill OAM
    // — which `req_core_111_oam_dma_steals_513_cycles_on_an_even_cycle_and_514_on_an_odd_one` and
    // `req_core_111_oam_dma_copies_the_whole_page_into_oam` are exactly about.
    let mut reader = StateReader::new(&bytes, 0).unwrap();
    bus.load_state(&mut reader).unwrap();

    let before = bus.cycle_count();
    bus.read(0x8000);
    assert_eq!(0, bus.cycle_count() - before - 1, "the request is gone");
    assert_eq!([0; 8], bus.ppu().oam()[..8], "and OAM was never written");
}

/// Starts a DPCM sample at `rate` and takes its load fetch, so that the channel is playing and
/// its memory reader will interrupt an OAM transfer somewhere inside it.
fn start_a_dmc_sample(bus: &mut NesBus, rate: u8) {
    bus.write(0x4010, rate & 0x0F); // No IRQ, no loop.
    bus.write(0x4012, 0x00); // Sample at $C000.
    bus.write(0x4013, 0xFF); // 4081 bytes: the whole run below is one sample.
    bus.write(0x4015, 0x10);
    for _ in 0..3 {
        bus.write(0x0000, 0x00); // The load delay, on cycles a DMA may not halt.
    }
    bus.read(0x8000);
    assert_eq!(1, bus.dmc_dma_count());
}

/// The fast OAM loop (lot MF-4a-ter step 2) is the DMA state machine, cycle for cycle.
///
/// For four DPCM rates, an OAM transfer is driven with the sample playing and the DMC's request
/// aimed at a chosen point of it: `lead` is how many cycles short of the reader's own question the
/// `$4014` write lands, so leads 0 to 15 put the fetch in the transfer's first cycles, 254 to 257
/// in the middle and 504 to 515 across its end and just after it. Sixteen consecutive leads cover
/// both parities eight times over, which is the boundary the fast loop's `free / 2` exists for: a
/// batch that ran one cycle past the cycle the request rose on would let the put cycle after it
/// consume the DMC's halt cycle and the fetch would land one cycle early.
///
/// Three things are recorded per row: the cycles the transfer stole from the CPU, how many DMC
/// fetches landed inside it, and a hash of the OAM it left — which moves if a byte was read out
/// of step, written twice or missed.
///
/// The expected table was produced by this same test at `dadb94c5` (this lot's step 1, whose only
/// change is the horizon function this test's own setup calls), where the state machine ran every
/// one of these cycles on its own. It is asserted character for character.
#[test]
fn req_core_113_the_fast_oam_loop_is_the_state_machine_cycle_for_cycle() {
    let mut log = String::new();

    for rate in [0u8, 8, 14, 15] {
        for lead in (0..16u32).chain(254..258).chain(504..516) {
            let (_, mut bus) = loaded_core(None);
            for i in 0..=255u8 {
                bus.poke(0x0300 + u16::from(i), i.wrapping_mul(7) ^ 0x5A);
            }
            start_a_dmc_sample(&mut bus, rate);

            // Up to `lead` cycles short of the reader's question, on write cycles, which a DMA
            // may not halt.
            let horizon = bus
                .apu()
                .cycles_until_dmc_fetch()
                .expect("the sample is still playing");
            for _ in 0..horizon.saturating_sub(lead) {
                bus.write(0x0000, 0x00);
            }

            let fetches = bus.dmc_dma_count();
            let stall = oam_dma_stall(&mut bus, 0x03);
            let hash = bus.ppu().oam().iter().fold(0u32, |acc, b| {
                acc.wrapping_mul(31).wrapping_add(u32::from(*b))
            });

            writeln!(
                log,
                "{rate:2} {lead:3} {stall} {} {hash:08X}",
                bus.dmc_dma_count() - fetches
            )
            .unwrap();
        }
    }

    assert_eq!(OAM_DMA_WITH_DPCM, log);
}

/// `rate`, `lead`, cycles stolen, DMC fetches inside the transfer, OAM hash — as the DMA
/// state machine alone produced them at `dadb94c5`. See the test above.
#[rustfmt::skip]
const OAM_DMA_WITH_DPCM: &str = concat!(
    " 0   0 516 1 DCF93400\n",
    " 0   1 515 1 DCF93400\n",
    " 0   2 516 1 DCF93400\n",
    " 0   3 515 1 DCF93400\n",
    " 0   4 516 1 DCF93400\n",
    " 0   5 515 1 DCF93400\n",
    " 0   6 516 1 DCF93400\n",
    " 0   7 515 1 DCF93400\n",
    " 0   8 516 1 DCF93400\n",
    " 0   9 515 1 DCF93400\n",
    " 0  10 516 1 DCF93400\n",
    " 0  11 515 1 DCF93400\n",
    " 0  12 516 1 DCF93400\n",
    " 0  13 515 1 DCF93400\n",
    " 0  14 516 1 DCF93400\n",
    " 0  15 515 1 DCF93400\n",
    " 0 254 516 1 DCF93400\n",
    " 0 255 515 1 DCF93400\n",
    " 0 256 516 1 DCF93400\n",
    " 0 257 515 1 DCF93400\n",
    " 0 504 516 1 DCF93400\n",
    " 0 505 515 1 DCF93400\n",
    " 0 506 516 1 DCF93400\n",
    " 0 507 515 1 DCF93400\n",
    " 0 508 516 1 DCF93400\n",
    " 0 509 515 1 DCF93400\n",
    " 0 510 516 1 DCF93400\n",
    " 0 511 514 1 DCF93400\n",
    " 0 512 515 1 DCF93400\n",
    " 0 513 516 1 DCF93400\n",
    " 0 514 517 1 DCF93400\n",
    " 0 515 513 0 DCF93400\n",
    " 8   0 516 1 DCF93400\n",
    " 8   1 515 1 DCF93400\n",
    " 8   2 516 1 DCF93400\n",
    " 8   3 515 1 DCF93400\n",
    " 8   4 516 1 DCF93400\n",
    " 8   5 515 1 DCF93400\n",
    " 8   6 516 1 DCF93400\n",
    " 8   7 515 1 DCF93400\n",
    " 8   8 516 1 DCF93400\n",
    " 8   9 515 1 DCF93400\n",
    " 8  10 516 1 DCF93400\n",
    " 8  11 515 1 DCF93400\n",
    " 8  12 516 1 DCF93400\n",
    " 8  13 515 1 DCF93400\n",
    " 8  14 516 1 DCF93400\n",
    " 8  15 515 1 DCF93400\n",
    " 8 254 516 1 DCF93400\n",
    " 8 255 515 1 DCF93400\n",
    " 8 256 516 1 DCF93400\n",
    " 8 257 515 1 DCF93400\n",
    " 8 504 516 1 DCF93400\n",
    " 8 505 515 1 DCF93400\n",
    " 8 506 516 1 DCF93400\n",
    " 8 507 515 1 DCF93400\n",
    " 8 508 516 1 DCF93400\n",
    " 8 509 515 1 DCF93400\n",
    " 8 510 516 1 DCF93400\n",
    " 8 511 514 1 DCF93400\n",
    " 8 512 515 1 DCF93400\n",
    " 8 513 516 1 DCF93400\n",
    " 8 514 517 1 DCF93400\n",
    " 8 515 513 0 DCF93400\n",
    "14   0 516 1 DCF93400\n",
    "14   1 515 1 DCF93400\n",
    "14   2 516 1 DCF93400\n",
    "14   3 515 1 DCF93400\n",
    "14   4 516 1 DCF93400\n",
    "14   5 515 1 DCF93400\n",
    "14   6 516 1 DCF93400\n",
    "14   7 515 1 DCF93400\n",
    "14   8 516 1 DCF93400\n",
    "14   9 515 1 DCF93400\n",
    "14  10 516 1 DCF93400\n",
    "14  11 515 1 DCF93400\n",
    "14  12 516 1 DCF93400\n",
    "14  13 515 1 DCF93400\n",
    "14  14 516 1 DCF93400\n",
    "14  15 515 1 DCF93400\n",
    "14 254 516 1 DCF93400\n",
    "14 255 515 1 DCF93400\n",
    "14 256 516 1 DCF93400\n",
    "14 257 515 1 DCF93400\n",
    "14 504 516 1 DCF93400\n",
    "14 505 515 1 DCF93400\n",
    "14 506 516 1 DCF93400\n",
    "14 507 515 1 DCF93400\n",
    "14 508 516 1 DCF93400\n",
    "14 509 515 1 DCF93400\n",
    "14 510 516 1 DCF93400\n",
    "14 511 514 1 DCF93400\n",
    "14 512 515 1 DCF93400\n",
    "14 513 516 1 DCF93400\n",
    "14 514 517 1 DCF93400\n",
    "14 515 513 0 DCF93400\n",
    "15   0 518 2 DCF93400\n",
    "15   1 517 2 DCF93400\n",
    "15   2 518 2 DCF93400\n",
    "15   3 517 2 DCF93400\n",
    "15   4 518 2 DCF93400\n",
    "15   5 517 2 DCF93400\n",
    "15   6 518 2 DCF93400\n",
    "15   7 517 2 DCF93400\n",
    "15   8 518 2 DCF93400\n",
    "15   9 517 2 DCF93400\n",
    "15  10 518 2 DCF93400\n",
    "15  11 517 2 DCF93400\n",
    "15  12 518 2 DCF93400\n",
    "15  13 517 2 DCF93400\n",
    "15  14 518 2 DCF93400\n",
    "15  15 517 2 DCF93400\n",
    "15 254 516 1 DCF93400\n",
    "15 255 515 1 DCF93400\n",
    "15 256 516 1 DCF93400\n",
    "15 257 515 1 DCF93400\n",
    "15 504 516 1 DCF93400\n",
    "15 505 515 1 DCF93400\n",
    "15 506 516 1 DCF93400\n",
    "15 507 515 1 DCF93400\n",
    "15 508 516 1 DCF93400\n",
    "15 509 515 1 DCF93400\n",
    "15 510 516 1 DCF93400\n",
    "15 511 514 1 DCF93400\n",
    "15 512 515 1 DCF93400\n",
    "15 513 516 1 DCF93400\n",
    "15 514 517 1 DCF93400\n",
    "15 515 513 0 DCF93400\n",
);

#[test]
fn req_core_111_a_sta_4014_instruction_costs_its_own_cycles_plus_the_stall() {
    // LDA #$02 / STA $4014 / JMP $8000
    let program = [0xA9, 0x02, 0x8D, 0x14, 0x40, 0x4C, 0x00, 0x80];
    let (mut cpu, mut bus) = loaded_core(Some(&program));

    let mut cycles = 0;
    while bus.oam_dma_count() == 0 {
        cycles = cpu.step(&mut bus);
    }

    // STA absolute is its own 4 cycles; the 513 or 514 stolen cycles fall on the instruction
    // after it, because that is where the CPU's next read cycle is.
    assert_eq!(4, cycles);

    let next = cpu.step(&mut bus);
    assert!([516, 517].contains(&next), "{next}");
}
