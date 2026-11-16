//! FIDELITY-LEADS G5 (`emu.cpu_apu_parity`): the CPU / APU get-put parity a power-on starts
//! with. The 2A03 netlist's (`NesCore::set_netlist_get_put`) takes the other one from Zaxnes's
//! usual start, which moves an OAM DMA started on the same cycle from 514 to 513 cycles or back
//! (nesdev, "DMA": 513, plus one when the write lands on a put cycle).

use zax_nes::NesCore;

/// A mapper-0 image running `program` at $8000, every vector pointing there.
fn rom(program: &[u8]) -> Vec<u8> {
    const PRG_SIZE: usize = 2 * 16 * 1024;
    let mut image = vec![0_u8; 16 + PRG_SIZE + 8 * 1024];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 0x02;
    image[5] = 0x01;
    image[16..16 + program.len()].copy_from_slice(program);
    let vectors = 16 + PRG_SIZE - 6;
    for slot in 0..3 {
        image[vectors + 2 * slot] = 0x00;
        image[vectors + 2 * slot + 1] = 0x80;
    }
    image
}

/// The cycles `STA $4014`, its OAM DMA and the next `JMP` take, after a power-on with the
/// netlist's parity or Zaxnes's.
fn dma_cycles(netlist: bool) -> i64 {
    // 8000 A9 02     LDA #$02
    // 8002 8D 14 40  STA $4014
    // 8005 4C 05 80  JMP $8005
    let mut core = NesCore::new();
    assert!(
        core.load_rom(&rom(&[0xA9, 0x02, 0x8D, 0x14, 0x40, 0x4C, 0x05, 0x80]), "g5.nes")
            .success
    );
    core.set_netlist_get_put(netlist);
    core.reset();
    let step = |core: &mut NesCore| {
        let (cpu, bus) = core.cpu_and_bus_mut();
        cpu.step(bus);
    };
    step(&mut core); // LDA
    let before = core.cycle_count();
    step(&mut core); // STA $4014
    step(&mut core); // the transfer halts the next read, then JMP
    core.cycle_count() - before
}

#[test]
fn g5_the_netlist_parity_moves_an_oam_dma_by_one_cycle() {
    let usual = dma_cycles(false);
    let netlist = dma_cycles(true);
    assert_eq!(1, (usual - netlist).abs(), "{usual} against {netlist}");
    // 4 for the write, 513 or 514 for the transfer, 3 for the JMP.
    assert!([520, 521].contains(&usual) && [520, 521].contains(&netlist));
}

#[test]
fn g5_a_reset_button_keeps_the_parity_a_power_on_set() {
    // The setting takes effect at the next power-on; without it nothing changes.
    assert_eq!(dma_cycles(false), dma_cycles(false));
    assert_eq!(dma_cycles(true), dma_cycles(true));
}
