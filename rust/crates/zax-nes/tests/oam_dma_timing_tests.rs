//! OAM DMA: the 513 or 514 cycles it steals from the CPU and the transfer itself
//! (nesdev "DMA"; REQ-CORE-110 cycle accounting, REQ-CORE-111 timing).

mod common;

use common::loaded_core;
use zax_nes::cpu::CpuBus;
use zax_nes::NesBus;

#[test]
fn req_core_111_oam_dma_steals_513_cycles_on_an_even_cycle_and_514_on_an_odd_one() {
    let mut stalls = Vec::new();

    for padding in [0, 1] {
        let (_, mut bus) = loaded_core(None);

        // The reset sequence left the clock at 7; one extra tick flips its parity.
        for _ in 0..padding {
            bus.tick();
        }

        let before = bus.cycle_count();
        bus.write(0x4014, 0x02);
        let elapsed = bus.cycle_count() - before;

        // The write cycle itself is not part of the stall.
        stalls.push(elapsed - 1);
        assert_eq!(1, bus.oam_dma_count());
    }

    stalls.sort_unstable();
    assert_eq!(vec![513, 514], stalls);
    assert!(stalls.contains(&NesBus::OAM_DMA_CYCLES_EVEN));
    assert!(stalls.contains(&NesBus::OAM_DMA_CYCLES_ODD));
}

#[test]
fn req_core_111_oam_dma_copies_the_whole_page_into_oam() {
    let (_, mut bus) = loaded_core(None);

    for i in 0..=255u8 {
        bus.poke(0x0200 + u16::from(i), 255 - i);
    }

    bus.write(0x4014, 0x02);

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
    bus.write(0x4014, 0x03);

    // The transfer wraps around inside OAM.
    let oam = bus.ppu().oam();
    assert_eq!(0x00, oam[0x10]);
    assert_eq!(0xEF, oam[0xFF]);
    assert_eq!(0xF0, oam[0x00]);
    assert_eq!(0xFF, oam[0x0F]);
}

#[test]
fn req_core_111_a_sta_4014_instruction_costs_its_own_cycles_plus_the_stall() {
    // LDA #$02 / STA $4014 / JMP $8000
    let program = [0xA9, 0x02, 0x8D, 0x14, 0x40, 0x4C, 0x00, 0x80];
    let (mut cpu, mut bus) = loaded_core(Some(&program));

    let mut cycles = 0;
    while bus.oam_dma_count() == 0 {
        cycles = cpu.step(&mut bus);
    }

    // STA absolute is 4 cycles, plus a 513 or 514 cycle stall.
    assert!([517, 518].contains(&cycles), "{cycles}");
}
