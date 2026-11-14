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
// A code fetch from the zone and the DMA units (lots ZR-1 and ZR-2).
//
// The zone is `$2000`-`$3FFF` — the PPU's registers — and the byte a fetch takes there comes from
// the pack's code buffer, never from the bus. So **nothing** about the fetch may read a register:
// not the fetch itself (lot ZR-1), and not the halt cycle's or the dummy cycles' repeats of the
// fetch address inside `run_dma` and `dmc_only_dma` (lot ZR-2). On real hardware those repeats
// always happen, because the CPU is stopped with its address on the bus; but no NES runs code from
// the PPU's register window, so for a zone fetch there is no hardware behaviour to preserve.
//
// `acfa6e20` broke the first half by routing the halted zone fetch through
// `NesBus::halted_read_cycle`, which since that commit ends in a real read on both its paths; it
// cost Super Mario Bros. 3 the row of ground above its status bar. ZR-1 took that read away and
// the row stayed broken, because 3 166 of the DMA's own repeats were left — 441 of `$2002` and
// 442 of `$2007` in 1 500 frames, and a `$2007` read during rendering moves `v` by a coarse X and
// a Y at once (MF-4d step 6).
//
// The DMA units' reads of their **own** addresses are not this read, and every test below asserts
// they are untouched: the sample fetch happens, the transfer lands in OAM byte for byte, and the
// access spends exactly the cycles it would have spent.
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
    // Written right after power-on, which the PPU's reset line would ignore (lot FIDELITY step
    // 34): the machine is one without it.
    bus.ppu_mut().set_reset_line(false);
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

/// Lets a held DMC request come due: a write cannot be halted, so these three spend the load's
/// delay without giving the unit the bus. The next **read** cycle is the halt.
fn let_the_dmc_request_come_due(bus: &mut zax_nes::NesBus) {
    for _ in 0..3 {
        bus.write(0x0000, 0x00);
    }
}

/// REQ-CORE-113. The halted case: the zone fetch must make **none** of the reads the data read it
/// stands for makes — neither the CPU's own (lot ZR-1) nor the halt's and the dummy cycles'
/// repeats (lot ZR-2).
///
/// `$2007` is the instrument because its read is the only one whose effect accumulates: `v` steps
/// once per read. So `v` counts the reads for us, and the claim is the sharpest one there is —
/// after the fetch `v` is **exactly** where it was, while the data read moved it.
#[test]
fn req_core_113_a_halted_zone_fetch_makes_none_of_the_reads_the_data_read_it_stands_for_makes() {
    let mut bus = zone_fetch_machine();
    hold_a_dmc_request(&mut bus);
    let_the_dmc_request_come_due(&mut bus);

    let before = vram_address(&bus);
    let mut data = bus.clone();
    let mut fetch = bus;
    data.read(0x2007);
    assert_eq!(0xEA, fetch.read_code(0x2007));

    assert_eq!(1, data.dmc_dma_count(), "the data read was halted");
    assert_eq!(
        1,
        fetch.dmc_dma_count(),
        "and so was the fetch, and it fetched"
    );
    assert!(
        vram_address(&data) > before,
        "the halt repeats the CPU's read, so the data read moved v more than once"
    );
    assert_eq!(
        before,
        vram_address(&fetch),
        "the fetch moved v not at all: it made neither its own read nor the halt's"
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
        before,
        vram_address(&fetch),
        "the fetch moved v not at all, and the transfer ran all the same"
    );
}

// ---------------------------------------------------------------------------------------------
// The same two halts **during rendering** (lot ZR-2), which is the state the picture broke in.
//
// Rendering off, a `$2007` read steps `v` by the control register's stride. During rendering it
// moves a coarse X and a Y at once instead (MF-4d step 6), which is the rule that turned the
// reads ZR-1 left behind into a visible break at Super Mario Bros. 3's status-bar split. So these
// two run the halt where it did its damage, and they ask for more than `v`: the write toggle, the
// read buffer, and the cycles the access spent.
//
// The counterpart is a **control** rather than the data read: a halted read of `$8000`, which is
// the same halt, the same unit and the same cycles, on an address that is not a register. A
// halted zone fetch must be indistinguishable from it in the PPU — that is the whole rule, and
// it is also what pins the cycles, because during rendering the PPU moves `v` by itself and two
// machines only agree on it if they stopped on the same dot.
// ---------------------------------------------------------------------------------------------

/// Everything the fetch must leave alone, read out of `bus` and consuming it, because the last two
/// probes are destructive.
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    /// Where the PPU stopped, which counts the cycles the access spent.
    position: (i32, i32),
    /// `v`.
    vram: u16,
    /// What `$2007` hands over, which is the read buffer as the access left it.
    buffered: u8,
    /// What a second `$2006` write makes of `v`: it loads the low byte only while the write
    /// toggle is still set, so this is the toggle (a read of `$2002`, or of any mirror of it,
    /// would have cleared it).
    write_toggle_probe: u16,
}

fn observe(mut bus: zax_nes::NesBus) -> Observed {
    bus.sync_ppu();
    let position = (bus.ppu().scanline(), bus.ppu().dot());
    let vram = vram_address(&bus);
    let buffered = bus.read(0x2007);
    bus.write(0x2006, 0x99);
    Observed {
        position,
        vram,
        buffered,
        write_toggle_probe: vram_address(&bus),
    }
}

/// Whether the PPU stands well inside a visible line, fetching: where a `$2007` read moves `v`
/// by a coarse X and a Y at once rather than by the control register's stride.
fn rendering_a_visible_line(bus: &mut zax_nes::NesBus) -> bool {
    bus.sync_ppu();
    (16..200).contains(&bus.ppu().scanline()) && (16..240).contains(&bus.ppu().dot())
}

/// Turns rendering on and spends CPU cycles — writes of work RAM, which no unit may halt and
/// which touch no register — until it gets there. One frame is under 30 000 CPU cycles, so the
/// bound is three frames' worth and the assertion after the loop never fires.
fn render_into_a_visible_line(bus: &mut zax_nes::NesBus) {
    bus.write(0x2001, 0x18); // Background and sprites shown: the PPU fetches.
    for _ in 0..90_000 {
        if rendering_a_visible_line(bus) {
            break;
        }
        bus.write(0x0000, 0x00);
    }
    assert!(
        rendering_a_visible_line(bus),
        "the PPU never reached a visible line"
    );
}

/// The machine of the halted tests above, rendering, with the zone on and the PPU primed.
fn rendering_zone_fetch_machine() -> zax_nes::NesBus {
    let mut bus = zone_fetch_machine();
    render_into_a_visible_line(&mut bus);
    bus
}

/// Asserts that a zone fetch of `address` left the PPU exactly as the control's halted read of
/// `$8000` left it, both halted from `armed`.
fn a_zone_fetch_is_the_controls_halted_read(armed: &zax_nes::NesBus, address: u16, what: &str) {
    let mut control = armed.clone();
    let mut fetch = armed.clone();
    control.read(0x8000);
    assert_eq!(
        0xEA,
        fetch.read_code(address),
        "{what}: the byte came from the pack's code"
    );
    assert_eq!(
        control.dmc_dma_count(),
        fetch.dmc_dma_count(),
        "{what}: the DMC fetched as it did for the control"
    );
    assert_eq!(
        control.oam_dma_count(),
        fetch.oam_dma_count(),
        "{what}: the transfer ran as it did for the control"
    );
    assert_eq!(
        control.ppu().oam(),
        fetch.ppu().oam(),
        "{what}: OAM is the control's, byte for byte"
    );
    assert_eq!(
        observe(control),
        observe(fetch),
        "{what}: the PPU is not the control's — the fetch read a register, \
         or spent the wrong cycles"
    );
}

/// REQ-CORE-113. A zone fetch halted by a DMC sample fetch, during rendering: `v`, the write
/// toggle and the read buffer are the control's, and so are the cycles.
///
/// Fetched at a `$2007` mirror and at a `$2002` mirror, which are the two reads with something to
/// lose: `$2007` moves `v` and refills the buffer, `$2002` clears the toggle. At the tip of ZR-1
/// those are 442 and 441 reads of Super Mario Bros. 3's 1 500 frames.
#[test]
fn req_core_113_a_zone_fetch_halted_by_a_dmc_fetch_during_rendering_moves_no_ppu_register() {
    for address in [0x2007, 0x2002] {
        let mut armed = rendering_zone_fetch_machine();
        hold_a_dmc_request(&mut armed);
        let_the_dmc_request_come_due(&mut armed);
        a_zone_fetch_is_the_controls_halted_read(&armed, address, &format!("${address:04X}"));
    }
}

/// REQ-CORE-113. The same during rendering with an OAM transfer — the other unit, the other path
/// through `NesBus::run_dma`, and 512 cycles of repeats instead of two or three.
#[test]
fn req_core_113_a_zone_fetch_halted_by_an_oam_transfer_during_rendering_moves_no_ppu_register() {
    for address in [0x2007, 0x2002] {
        let mut armed = rendering_zone_fetch_machine();
        for (index, slot) in armed.ram_mut()[0x0200..0x0300].iter_mut().enumerate() {
            *slot = (index as u8) ^ 0x5A;
        }
        armed.write(0x4014, 0x02); // The transfer halts the next read cycle.
        a_zone_fetch_is_the_controls_halted_read(&armed, address, &format!("${address:04X}"));
    }
}
