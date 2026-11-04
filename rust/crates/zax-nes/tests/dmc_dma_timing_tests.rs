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
