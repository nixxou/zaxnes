//! The widescreen extension's execution zone: the CPU's instruction fetches (opcodes, operands
//! and its dummy reads of the program counter) tell themselves from data reads, and the bus
//! shows a hack pack's code to the former only in `$2000`-`$3FFF`.

// Test addresses and bytes are small, and the PPU's scroll accessors are masked bit-fields, so
// they are never negative.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

mod common;

use common::{loaded_core, TestBus};
use zax_nes::cpu::CpuBus;

#[test]
fn opcodes_and_operands_are_code_fetches_and_data_is_not() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xAD, 0x00, 0x03]); // LDA $0300
    let mut cpu = bus.cpu();
    cpu.step(&mut bus);
    assert_eq!(vec![0x1000, 0x1001, 0x1002], bus.code_log);
    assert_eq!(vec![0x1000, 0x1001, 0x1002, 0x0300], bus.access_log);
}

#[test]
fn the_dummy_reads_of_the_program_counter_are_code_fetches() {
    // NOP: the opcode, then a dummy read of the next byte.
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xEA]);
    let mut cpu = bus.cpu();
    cpu.step(&mut bus);
    assert_eq!(vec![0x1000, 0x1001], bus.code_log);

    // A taken branch across a page: the operand, the dummy read, the page fix-up.
    let mut bus = TestBus::new();
    bus.load(0x10F0, &[0xD0, 0x20]); // BNE +$20 with Z clear
    let mut cpu = bus.create_cpu(0x10F0, 0xFD);
    cpu.step(&mut bus);
    assert_eq!(vec![0x10F0, 0x10F1, 0x10F2, 0x1012], bus.code_log);

    // RTS: the dummy read of the PC, then of the return address.
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0x60]);
    bus.load(0x01FE, &[0x02, 0x30]); // return to $3003
    let mut cpu = bus.create_cpu(0x1000, 0xFD);
    cpu.step(&mut bus);
    assert_eq!(vec![0x1000, 0x1001, 0x3002], bus.code_log);
    assert_eq!(0x3003, cpu.pc);
}

#[test]
fn an_interrupt_entry_reads_the_program_counter_as_code() {
    let mut bus = TestBus::new();
    bus.load(0x1000, &[0xEA, 0xEA]);
    bus.load(0xFFFA, &[0x00, 0x40]);
    let mut cpu = bus.cpu();
    bus.nmi_asserted = true;
    cpu.step(&mut bus);
    bus.code_log.clear();
    cpu.step(&mut bus);
    assert_eq!(
        vec![0x1001, 0x1001],
        bus.code_log,
        "the two reads before the pushes"
    );
    assert_eq!(0x4000, cpu.pc);
}

/// The minimal ROM with `program` at $8000, the extension unlocked, `code` the pack's code and
/// the zone on, page `page`.
fn zone_machine(program: &[u8], code: &[u8], page: u8) -> (zax_nes::cpu::Cpu6502, zax_nes::NesBus) {
    let (cpu, mut bus) = loaded_core(Some(program));
    for byte in *b"WIDE" {
        bus.write(0x5000, byte);
    }
    bus.ppu_mut().wide_mut().set_code(code);
    bus.write(0x500B, 0x80 | page);
    (cpu, bus)
}

#[test]
fn code_in_the_zone_runs_and_its_data_reads_still_reach_the_ppu() {
    // At $2000: LDA $2002 / STA $0300 / LDA #$5A / STA $0301 / JMP $200A (an endless loop).
    let code = [
        0xAD, 0x02, 0x20, 0x8D, 0x00, 0x03, 0xA9, 0x5A, 0x8D, 0x01, 0x03, 0x4C, 0x0B, 0x20,
    ];
    let (mut cpu, mut bus) = zone_machine(&[0x4C, 0x00, 0x20], &code, 0);
    for _ in 0..8 {
        cpu.step(&mut bus);
    }
    assert_eq!(0x5A, bus.ram()[0x0301], "the code ran from the zone");
    assert_ne!(
        0xAD,
        bus.ram()[0x0300],
        "LDA $2002 read the PPU, not the code"
    );
    assert_eq!(0x200B, cpu.pc, "looping in the zone");
}

#[test]
fn the_paged_part_shows_the_page_asked_for() {
    let mut code = vec![0xFF; 0x800 + 2 * 0x1800];
    code[0] = 0x11;
    code[0x800 + 0x1800] = 0x22; // page 1 at $2800
    let (_, mut bus) = zone_machine(&[0x4C, 0x00, 0x80], &code, 1);
    assert_eq!(0x11, bus.read_code(0x2000));
    assert_eq!(0x22, bus.read_code(0x2800));
    assert_eq!(0xFF, bus.read_code(0x3FFF), "past the code");
}

#[test]
fn without_the_zone_a_fetch_there_is_a_plain_read() {
    let (_, mut bus) = zone_machine(&[0x4C, 0x00, 0x80], &[0x11; 16], 0);
    bus.write(0x500B, 0x00);
    let fetched = bus.read_code(0x2000);
    assert_ne!(0x11, fetched, "the zone off: the PPU answers");
    // Data reads never see the code, the zone on or off.
    bus.write(0x500B, 0x80);
    assert_ne!(0x11, bus.read(0x2000));
    assert_eq!(0x11, bus.read_code(0x2000));
}

#[test]
fn with_its_data_the_zone_reaches_the_extension_at_3000_and_the_game_still_the_ppu() {
    // The game, at $8000: LDA #$22 / STA $3205 (a PPU mirror) / JMP $2000.
    let program = [0xA9, 0x22, 0x8D, 0x05, 0x32, 0x4C, 0x00, 0x20];
    // The zone, at $2000: LDA #$40 / STA $3003 / LDA $3003 / STA $0300 / LDA #$77 / STA $3210 /
    // JMP $2010 (an endless loop).
    let code = [
        0xA9, 0x40, 0x8D, 0x03, 0x30, 0xAD, 0x03, 0x30, 0x8D, 0x00, 0x03, 0xA9, 0x77, 0x8D, 0x10,
        0x32, 0x4C, 0x10, 0x20,
    ];
    let (mut cpu, mut bus) = zone_machine(&program, &code, 0x40);
    for _ in 0..10 {
        cpu.step(&mut bus);
    }
    assert_eq!(0x2010, cpu.pc, "looping in the zone");
    let wide = bus.ppu().wide();
    assert!(wide.zone_data());
    assert_eq!(
        Some(0x40),
        wide.read(0x5003),
        "the zone's STA $3003: the margin asked for"
    );
    assert_eq!(0x40, bus.ram()[0x0300], "the zone's LDA $3003 read it back");
    assert_eq!(
        Some(0x77),
        wide.read(0x5210),
        "the zone's STA $3210: the extension's RAM"
    );
    assert_eq!(
        Some(0x00),
        wide.read(0x5205),
        "the game's STA $3205 went to the PPU"
    );
    // $5000-$5FFF is the cartridge's: the game reads no register there.
    assert_ne!(0x40, bus.read(0x5003));
    // The zone's code stops at $2FFF: $3000 is no code fetch.
    assert_eq!(None, bus.ppu().wide().zone_offset(0x3000));
    assert!(bus.ppu().wide().zone_offset(0x2FFF).is_some());
}

// ---------------------------------------------------------------------------------------------
// A code fetch from the zone and the DMA units (lot ZR-1).
//
// The zone is `$2000`-`$3FFF` — the PPU's registers — and the byte a fetch takes there comes from
// the pack's code buffer, never from the bus. So the fetch itself must read no register, whatever
// the DMA units are doing on the cycle it lands on. The halt's **own** repeated read is a
// different thing and still happens: the CPU is stopped with `$2xxx` on the address bus.
//
// `acfa6e20` broke exactly this by routing the halted zone fetch through
// `NesBus::halted_read_cycle`, which since that commit ends in a real read on both its paths. It
// cost Super Mario Bros. 3 the row of ground above its status bar: 3 301 register reads in 1 500
// frames, 420 of `$2002` and 411 of `$2007`.
// ---------------------------------------------------------------------------------------------

/// The PPU's internal `v`, from the four halves the public accessors expose.
fn vram_address(bus: &zax_nes::NesBus) -> u16 {
    let ppu = bus.ppu();
    let bits = (ppu.fine_y() << 12)
        | (ppu.nametable_select() << 10)
        | (ppu.coarse_y() << 5)
        | ppu.coarse_x();
    (bits & 0x7FFF) as u16
}

/// Rendering off, the `$2007` step 1, a recognisable run in VRAM at `$2345`, `v` back at `$2345`
/// with the read buffer filled from it, and the write toggle **set** by a lone `$2006` write.
///
/// Every observable the fetch must not touch is then live: `v`, the buffer, and `w`.
fn prime_the_ppu(bus: &mut zax_nes::NesBus) {
    bus.write(0x2000, 0x00); // Rendering flags clear, the $2007 step 1.
    bus.write(0x2001, 0x00);
    bus.write(0x2006, 0x23);
    bus.write(0x2006, 0x45);
    for byte in [0x11, 0x22, 0x33, 0x44] {
        bus.write(0x2007, byte);
    }
    bus.write(0x2006, 0x23);
    bus.write(0x2006, 0x45);
    bus.read(0x2007); // The buffer is filled from $2345; v is $2346.
    bus.write(0x2006, 0x11); // A first write and no second: w is set, v untouched.
}

/// Arms the DMC and starts a sample. The load fetch a `$4015` write queues may not halt a cycle
/// for two or three more (nesdev, "DMA"), so on return the request is **held**: `dma` is
/// non-zero and no unit may take the bus yet.
fn hold_a_dmc_request(bus: &mut zax_nes::NesBus) {
    bus.write(0x4010, 0x00); // Fastest rate, no IRQ, no loop.
    bus.write(0x4012, 0x00); // Sample at $C000.
    bus.write(0x4013, 0x01); // 17 bytes, so the fetch is not the last one.
    bus.write(0x4015, 0x10);
}

/// The zone on with 64 `NOP`s of pack code, the PPU primed, and nothing pending.
fn zone_fetch_machine() -> zax_nes::NesBus {
    let (_, mut bus) = zone_machine(&[0x4C, 0x00, 0x80], &[0xEA; 64], 0);
    prime_the_ppu(&mut bus);
    bus
}

/// REQ-CORE-113. A **held** DMC request is the one state where `dma` is non-zero and no unit may
/// halt. The fetch must then be what it is on any quiet cycle: one tick and a byte out of the
/// code buffer, with the PPU's `v`, read buffer and `w` exactly where they were.
///
/// The control spends the same single cycle on a write of CPU RAM, which touches no register and
/// which a DMA unit may not halt either, so the two machines differ only by the fetch.
#[test]
fn req_core_113_a_zone_fetch_under_a_held_dmc_request_touches_no_ppu_register() {
    let mut bus = zone_fetch_machine();
    hold_a_dmc_request(&mut bus);
    let mut control = bus.clone();

    let before = vram_address(&bus);
    assert_eq!(
        0xEA,
        bus.read_code(0x2007),
        "the byte came from the pack's code, not from $2007"
    );
    control.write(0x0000, 0x00);

    assert_eq!(0, bus.dmc_dma_count(), "the request is still held");
    assert_eq!(before, vram_address(&bus), "the fetch did not move v");
    assert_eq!(
        vram_address(&control),
        vram_address(&bus),
        "v is the control's"
    );
    assert_eq!(
        control.read(0x2007),
        bus.read(0x2007),
        "the read buffer is the control's: the fetch did not refill it"
    );

    // The write toggle, last: the probe consumes it. A second $2006 write loads v only if the
    // first one is still remembered, which a read of $2002 — or of any mirror of it — would have
    // forgotten.
    bus.write(0x2006, 0x99);
    control.write(0x2006, 0x99);
    assert_eq!(
        vram_address(&control),
        vram_address(&bus),
        "w is the control's: the fetch did not clear the write toggle"
    );
    assert_eq!(0x1199, vram_address(&bus), "w was still set in both");

    // And the request the fetch declined to serve is still a request.
    for _ in 0..8 {
        bus.read(0x8000);
    }
    assert_eq!(1, bus.dmc_dma_count(), "the held fetch ran once it was due");
}

/// REQ-CORE-113. The halted case, where the DMA unit **does** repeat the CPU's read: a zone fetch
/// must make exactly **one read fewer** than the data read it stands for, that one being the
/// CPU's own, which for a fetch comes from the code buffer instead.
///
/// `$2007` is the instrument because its read is the only one whose effect accumulates: `v` steps
/// once per read, so the two machines' `v` count the reads for us and the test never has to name
/// how many repeats the DMA itself makes.
#[test]
fn req_core_113_a_halted_zone_fetch_reads_one_time_fewer_than_the_data_read_it_stands_for() {
    let mut bus = zone_fetch_machine();
    hold_a_dmc_request(&mut bus);
    // A write cannot be halted, so these only let the load's delay run out: the next read cycle
    // is the halt.
    for _ in 0..3 {
        bus.write(0x0000, 0x00);
    }

    let before = vram_address(&bus);
    let mut data = bus.clone();
    let mut fetch = bus;
    data.read(0x2007);
    assert_eq!(0xEA, fetch.read_code(0x2007));

    assert_eq!(1, data.dmc_dma_count(), "the data read was halted");
    assert_eq!(1, fetch.dmc_dma_count(), "and so was the fetch");
    assert!(
        vram_address(&data) > before,
        "the halt repeats the CPU's read, so the data read moved v more than once"
    );
    assert_eq!(
        vram_address(&data) - 1,
        vram_address(&fetch),
        "the fetch made the halt's own reads and not the CPU's"
    );
}

/// REQ-CORE-113. The same, with an OAM transfer pending instead — a different unit, a different
/// path through `NesBus::run_dma`, and 512 cycles instead of three or four.
#[test]
fn req_core_113_a_zone_fetch_halted_by_an_oam_transfer_lets_it_finish_and_reads_nothing_itself() {
    let mut bus = zone_fetch_machine();
    for (index, slot) in bus.ram_mut()[0x0200..0x0300].iter_mut().enumerate() {
        *slot = (index as u8) ^ 0x5A;
    }
    bus.write(0x4014, 0x02); // The transfer halts the next read cycle.

    let before = vram_address(&bus);
    let mut data = bus.clone();
    let mut fetch = bus;
    data.read(0x2007);
    assert_eq!(0xEA, fetch.read_code(0x2007));

    assert_eq!(1, data.oam_dma_count(), "the data read was halted");
    assert_eq!(1, fetch.oam_dma_count(), "and so was the fetch");
    for (index, byte) in fetch.ppu().oam().iter().enumerate() {
        // Every byte as the page held it, except that bits 2-4 of a sprite's attribute byte are
        // not implemented and read back as 0 (nesdev, "PPU OAM").
        let mask = if index % 4 == 2 { 0xE3 } else { 0xFF };
        assert_eq!(((index as u8) ^ 0x5A) & mask, *byte, "OAM byte {index}");
    }
    assert_eq!(
        data.ppu().oam(),
        fetch.ppu().oam(),
        "the transfer is the data read's, byte for byte"
    );
    assert!(vram_address(&data) > before);
    assert_eq!(
        vram_address(&data) - 1,
        vram_address(&fetch),
        "the fetch made the halt's own reads and not the CPU's"
    );
}
