//! The Namco 163 board of mapper 19 and the two ASICs cut from it on mapper 210, with the
//! chip's expansion audio (REQ-CORE-075, which leaves it optional).
//!
//! Read from the public nesdev pages "INES Mapper 019", "INES Mapper 210" and "Namco 163 audio",
//! cross-checked against `third_party/mesen2`'s `Namco163.h` and `Namco163Audio.h`. **No test ROM
//! for either mapper exists anywhere in `assets-free/test-roms`** — step 7 of
//! `docs/progress/MF-5a.md` records the sweep that establishes it — so every assertion below is
//! a sentence of one of those pages turned into an assertion, and R0's commercial titles are what
//! checks the whole.
//!
//! Two provisional choices are pinned here rather than left to drift: **Q-126**, the two PPU modes
//! this build decodes and does not serve, and **Q-127**, the level of the sound.

// Register values, bank numbers and periods are small literals.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::unwrap_used
)]

mod common;

use common::mapper_fixture::MapperRom;
use zax_nes::apu::mixer;
use zax_nes::cartridges::{Cartridge, ChrFetch};
use zax_nes::cpu::CpuBus;
use zax_nes::NametableMirroring;
use zax_nes::NesCore;

/// A machine with a synthetic mapper-19 cartridge, powered on: 32 PRG banks of 8 KiB so that a
/// six-bit bank register has somewhere to point, and 256 CHR banks of 1 KiB so that a value of
/// $E0 and above names a real one.
fn core_163() -> NesCore {
    core_for(MapperRom::new(19).prg(32).chr(256))
}

/// The same for mapper 210, at the submapper given.
fn core_210(submapper: u8) -> NesCore {
    core_for(MapperRom::new(210).prg(32).chr(256).submapper(submapper))
}

fn core_for(rom: MapperRom) -> NesCore {
    let image = rom.image();
    let mut core = NesCore::new();
    let load = core.load_rom(&image, "Namco.nes");
    assert!(load.success, "{:?}", load.error);
    core
}

/// The first byte of the 8 KiB window at `address`, which the fixture fills with the bank number.
fn prg_bank(core: &NesCore, address: u16) -> u8 {
    core.read_memory(address)
}

/// The CHR byte at `address`, which the fixture fills with the 1 KiB bank number.
fn chr_bank(core: &NesCore, address: u16) -> u8 {
    core.cartridge()
        .unwrap()
        .ppu_peek(address, ChrFetch::Background)
}

fn cartridge(core: &NesCore) -> &Cartridge {
    core.cartridge().unwrap()
}

/// `M` as the test point of REQ-CORE-076 reports it.
fn expansion(core: &NesCore) -> i32 {
    core.apu().mixer_expansion()
}

/// Points `$F800`'s address port at `position`, with the auto-increment in `increment`.
///
/// **`$F800` is one byte doing two jobs and they overlap**, which is the chip's own doing and not
/// this helper's: nesdev gives the board "KKKK DCBA", whose "upper nybble must equal b0100 to
/// enable writes", and the sound page gives the same byte's low seven bits to the RAM address and
/// bit 7 to the auto-increment. `$40` is therefore *both* the write-enable key and bit 6 of the
/// address, so a game that enables PRG-RAM writes moves the port to $40 by the same stroke. Mesen
/// reads it the same way (`Namco163.h:273-281` writes `_writeProtect` and calls the sound's
/// `WriteRegister` with the same byte). Every test below therefore sets the port for what it is
/// about to do and never assumes the other half of the register.
fn set_port(core: &mut NesCore, position: u8, increment: bool) {
    core.write_memory(0xF800, (position & 0x7F) | if increment { 0x80 } else { 0 });
}

/// Writes `bytes` into the chip's RAM from `position`, through the port with the auto-increment
/// set — which is how a game loads a wavetable.
fn load_ram(core: &mut NesCore, position: u8, bytes: &[u8]) {
    set_port(core, position, true);
    for byte in bytes {
        core.write_memory(0x4800, *byte);
    }
}

/// The byte the chip's RAM holds at `position`, without moving the port.
fn peek_ram(core: &mut NesCore, position: u8) -> u8 {
    set_port(core, position, false);
    core.read_memory(0x4800)
}

/// A **CPU** read, which has the side effects a host accessor must not have (REQ-CORE-081).
fn cpu_read(core: &mut NesCore, address: u16) -> u8 {
    core.bus_mut().read(address)
}

/// Runs CPU cycles until `M` moves, and answers how many it took; 0 if it never did.
///
/// Every test that counts cycles does it this way rather than from an absolute tick count,
/// because `NesCore::write_memory` and `NesCore::read_memory` run a CPU cycle of their own — so
/// the multiplexer is already part-way through its fifteen when a test finishes writing its
/// registers, and only the **interval** between two of its updates is a number the chip fixes.
fn ticks_to_change(core: &mut NesCore) -> usize {
    let before = expansion(core);
    for tick in 1..=1000 {
        core.bus_mut().tick();
        if expansion(core) != before {
            return tick;
        }
    }

    0
}

/// One channel's eight register bytes, as the page lays them out.
fn channel_registers(frequency: u32, length: u8, wave_address: u8, volume: u8) -> [u8; 8] {
    [
        (frequency & 0xFF) as u8,
        0, // phase, low
        ((frequency >> 8) & 0xFF) as u8,
        0, // phase, mid
        (((frequency >> 16) & 0x03) as u8) | (length & 0xFC),
        0, // phase, high
        wave_address,
        volume & 0x0F,
    ]
}

// ---------------------------------------------------------------- PRG and CHR banking

/// nesdev: "$E000-$E7FF: PRG Select 1 … Select 8 KiB PRG ROM bank at $8000-$9FFF", and the same
/// for $E800 at $A000 and $F000 at $C000, each six bits wide. $E000-$FFFF is absent from the
/// table: the last bank is wired there.
#[test]
fn req_core_100_namco163_switches_three_8_kib_prg_banks_and_wires_the_last() {
    let mut core = core_163();

    core.write_memory(0xE000, 0x11);
    core.write_memory(0xE800, 0x12);
    core.write_memory(0xF000, 0x13);

    assert_eq!(0x11, prg_bank(&core, 0x8000));
    assert_eq!(0x12, prg_bank(&core, 0xA000));
    assert_eq!(0x13, prg_bank(&core, 0xC000));
    assert_eq!(
        31,
        prg_bank(&core, 0xE010),
        "the last bank is wired at $E000 (past the fixture's reset-vector JMP)"
    );

    // Six bits: the two above the field are the sound disable and pin 22 on $E000, the two
    // CHR-RAM disables on $E800, and nothing at all on $F000. None of them is a bank bit.
    core.write_memory(0xE000, 0xC0 | 0x05);
    core.write_memory(0xE800, 0xC0 | 0x06);
    core.write_memory(0xF000, 0xC0 | 0x07);
    assert_eq!(0x05, prg_bank(&core, 0x8000));
    assert_eq!(0x06, prg_bank(&core, 0xA000));
    assert_eq!(0x07, prg_bank(&core, 0xC000));

    // "each register occupying an $800-byte range": every address in the range is the register.
    core.write_memory(0xE7FF, 0x09);
    assert_eq!(0x09, prg_bank(&core, 0x8000));
}

/// nesdev: "$8000-$BFFF: Eight 1KB CHR-ROM bank selectors", one per $800 of address space.
#[test]
fn req_core_100_namco163_switches_eight_1_kib_chr_banks() {
    let mut core = core_163();

    for window in 0..8u16 {
        core.write_memory(0x8000 + window * 0x800, 0x20 + window as u8);
    }

    for window in 0..8u16 {
        assert_eq!(
            0x20 + window as u8,
            chr_bank(&core, window * 0x400),
            "CHR window {window}"
        );
    }

    // The register is the whole $800 range, and the eighth one is at $B800.
    core.write_memory(0xBFFF, 0x31);
    assert_eq!(0x31, chr_bank(&core, 0x1C00));
}

/// **Q-126.** "Values $E0-$FF can select nametable RAM when enabled by bits in $E800." This build
/// does not serve CIRAM as pattern data and shows the CHR-ROM bank the register names instead,
/// i.e. exactly what the chip does when both of `$E800`'s disable bits are set. The test pins the
/// provisional choice, so a lot that implements the mode has to come here and rewrite it.
#[test]
fn q_126_namco163_shows_chr_rom_where_e0_asks_for_ciram() {
    let mut core = core_163();

    // $E0 in the first window, with the disable bits clear — the case the chip would serve from
    // CIRAM page 0.
    core.write_memory(0x8000, 0xE0);
    assert_eq!(0xE0, chr_bank(&core, 0x0000));

    // $E1 would be CIRAM page 1; this build gives bank $E1 either way.
    core.write_memory(0x8000, 0xE1);
    assert_eq!(0xE1, chr_bank(&core, 0x0000));

    // And with the disable bit set, which is the case the two agree on, the answer is the same.
    core.write_memory(0xE800, 0x40);
    assert_eq!(0xE1, chr_bank(&core, 0x0000));
    core.write_memory(0xE800, 0x80);
    assert_eq!(0xE1, chr_bank(&core, 0x0000));
}

// ---------------------------------------------------------------- the nametable selects

/// nesdev, the $C000-$D800 registers: a value of $E0 or above selects CIRAM, and "even values
/// select A nametables, odd select B" — so the low bit is the page, one per logical table.
#[test]
fn req_core_100_namco163_picks_a_ciram_page_per_table_from_the_select() {
    let mut core = core_163();

    core.write_memory(0xC000, 0xE1);
    core.write_memory(0xC800, 0xE0);
    core.write_memory(0xD000, 0xFF);
    core.write_memory(0xD800, 0xE2);

    assert_eq!(1, cartridge(&core).nametable_page(0));
    assert_eq!(0, cartridge(&core).nametable_page(1));
    assert_eq!(1, cartridge(&core).nametable_page(2));
    assert_eq!(0, cartridge(&core).nametable_page(3));

    // Only the low bit is the page: $E0 and $EE are both page 0, $E1 and $EF both page 1.
    core.write_memory(0xC000, 0xEE);
    assert_eq!(0, cartridge(&core).nametable_page(0));
    core.write_memory(0xC000, 0xEF);
    assert_eq!(1, cartridge(&core).nametable_page(0));
}

/// **Q-126.** A select below $E0 asks for a **CHR-ROM** nametable, which this build does not
/// serve: the table falls back to the cartridge's own mirroring, so a board whose game has not
/// written the registers behaves like an ordinary one. `FOLLOW_MIRRORING` is the answer, and
/// never `SERVED_BY_CARTRIDGE` — the measurement that forbids a third arm of
/// `Board::read_nametable` is Q-123's, taken in step 1 of this same lot.
#[test]
fn q_126_namco163_keeps_its_nametables_on_ciram_where_a_select_asks_for_chr_rom() {
    let mut core = core_163();

    // Power-on: every select is 0, which is below $E0.
    for table in 0..4 {
        assert_eq!(
            Cartridge::FOLLOW_MIRRORING,
            cartridge(&core).nametable_page(table)
        );
    }

    core.write_memory(0xC000, 0x40);
    core.write_memory(0xC800, 0xDF);
    for table in 0..2 {
        assert_eq!(
            Cartridge::FOLLOW_MIRRORING,
            cartridge(&core).nametable_page(table),
            "table {table} must follow the mirroring, not be served by the cartridge"
        );
    }

    // And the header's mirroring is what the PPU then uses, untouched by the board.
    assert_eq!(
        NametableMirroring::Horizontal,
        core.cartridge().unwrap().mirroring()
    );
}

// ---------------------------------------------------------------- the IRQ counter

/// nesdev: the two halves of $5000/$5800 form "a 15-bit CPU cycle up-counter" which "increments
/// on each CPU cycle while enabled", and "upon reaching $7FFF, an IRQ is fired".
#[test]
fn req_core_100_the_namco163_counter_counts_up_on_the_cpu_clock() {
    let mut core = core_163();

    // $7FFF - 20, enabled: twenty cycles to the top.
    let start = 0x7FFF - 20;
    core.write_memory(0x5000, (start & 0xFF) as u8);
    core.write_memory(0x5800, 0x80 | ((start >> 8) & 0x7F) as u8);

    for cycle in 0..19 {
        core.bus_mut().tick();
        assert!(
            !cartridge(&core).irq_asserted(),
            "the IRQ fired {} cycles early",
            20 - cycle
        );
    }

    core.bus_mut().tick();
    assert!(
        cartridge(&core).irq_asserted(),
        "the IRQ must fire at $7FFF"
    );

    // "Writes acknowledge pending IRQs": either half does it.
    core.write_memory(0x5000, 0x00);
    assert!(!cartridge(&core).irq_asserted());
}

/// The enable is bit 7 of the high half, and the counter does not move without it. The page's
/// own wording — "increments on each CPU cycle **while enabled**" — is the whole of this.
#[test]
fn req_core_100_the_namco163_counter_stands_still_while_disabled() {
    let mut core = core_163();

    let start = 0x7FFF - 5;
    core.write_memory(0x5000, (start & 0xFF) as u8);
    core.write_memory(0x5800, ((start >> 8) & 0x7F) as u8); // no bit 7.

    for _ in 0..100 {
        core.bus_mut().tick();
    }

    assert!(!cartridge(&core).irq_asserted());
    // And the counter is where it was left, which the two readable halves report.
    assert_eq!((start & 0xFF) as u8, core.read_memory(0x5000));
    assert_eq!(((start >> 8) & 0x7F) as u8, core.read_memory(0x5800));
}

/// The counter **stops** at $7FFF rather than wrapping: the page fires the IRQ "upon reaching
/// $7FFF" and says nothing of a reload, so the line stays asserted until a write acknowledges it.
#[test]
fn req_core_100_the_namco163_counter_stops_at_its_top_rather_than_wrapping() {
    let mut core = core_163();

    core.write_memory(0x5000, 0xFE);
    core.write_memory(0x5800, 0xFF); // enabled, counter $7FFE.

    core.bus_mut().tick();
    assert!(cartridge(&core).irq_asserted());

    // A thousand cycles later it is still $7FFF and the IRQ is still up: no wrap, no second edge.
    for _ in 0..1000 {
        core.bus_mut().tick();
    }

    assert!(cartridge(&core).irq_asserted());
    assert_eq!(0xFF, core.read_memory(0x5000));
    assert_eq!(0xFF, core.read_memory(0x5800));
}

/// Both halves read back what was written, enable bit included: "reads do not" acknowledge, so a
/// game may poll the counter.
#[test]
fn req_core_100_the_namco163_counter_reads_back_without_acknowledging() {
    let mut core = core_163();

    core.write_memory(0x5000, 0x34);
    core.write_memory(0x5800, 0x80 | 0x12);
    assert_eq!(0x34, core.read_memory(0x5000));
    assert_eq!(0x92, core.read_memory(0x5800));

    // Reading does not clear a pending IRQ.
    core.write_memory(0x5000, 0xFE);
    core.write_memory(0x5800, 0xFF);
    core.bus_mut().tick();
    assert!(cartridge(&core).irq_asserted());
    let _ = core.read_memory(0x5000);
    let _ = core.read_memory(0x5800);
    assert!(
        cartridge(&core).irq_asserted(),
        "a read must not acknowledge"
    );
}

// ---------------------------------------------------------------- the internal RAM

/// nesdev: "$4800-$4FFF: Chip RAM Data Port", with the position and the auto-increment in
/// `$F800` — "KKKK DCBA" carries the write protect, and the sound page gives bit 7 of the same
/// byte to the increment and the low seven bits to the address.
#[test]
fn req_core_075_the_namco163_ram_port_reads_and_writes_the_chips_128_bytes() {
    let mut core = core_163();

    // No auto-increment: every write lands on the same byte.
    set_port(&mut core, 0x05, false);
    core.write_memory(0x4800, 0xAA);
    core.write_memory(0x4800, 0xBB);
    assert_eq!(0xBB, peek_ram(&mut core, 0x05));

    // With bit 7, each access steps the port on — a **CPU** read as well as a write.
    load_ram(&mut core, 0x10, &[1, 2, 3, 4]);
    set_port(&mut core, 0x10, true);
    assert_eq!(1, cpu_read(&mut core, 0x4800));
    assert_eq!(2, cpu_read(&mut core, 0x4800));
    assert_eq!(3, cpu_read(&mut core, 0x4800));
    assert_eq!(4, cpu_read(&mut core, 0x4800));

    // The host accessor is the side-effect-free counterpart (REQ-CORE-081): it answers the same
    // byte every time and never moves the port.
    set_port(&mut core, 0x10, true);
    assert_eq!(1, core.read_memory(0x4800));
    assert_eq!(1, core.read_memory(0x4800));

    // Seven bits: the port wraps inside the 128 bytes rather than running past them.
    set_port(&mut core, 0x7F, true);
    core.write_memory(0x4800, 0x5A);
    core.write_memory(0x4800, 0x6B);
    assert_eq!(0x5A, peek_ram(&mut core, 0x7F));
    assert_eq!(0x6B, peek_ram(&mut core, 0x00));
}

/// The chip's RAM is the tail of PRG-RAM, so REQ-CORE-095's single battery slice carries it —
/// which is what makes a wavetable survive a power cycle on a battery-backed cartridge, and what
/// makes this build's save file the same two parts in the same order as Mesen's.
#[test]
fn req_core_095_the_namco163_internal_ram_is_inside_the_battery_slice() {
    let mut core = core_for(MapperRom::new(19).prg(32).chr(256).battery());

    load_ram(&mut core, 0x00, &[0xDE, 0xAD, 0xBE, 0xEF]);
    // And an ordinary PRG-RAM byte for the first half. $40 enables the writes and moves the port
    // with it, which is why the wavetable goes in first.
    core.write_memory(0xF800, 0x40);
    core.write_memory(0x6000, 0x11);

    let battery = cartridge(&core).battery_ram();
    assert_eq!(
        4 * 0x800 + 0x80,
        battery.len(),
        "the four 2 KiB windows and the chip's 128 bytes"
    );
    assert_eq!([0xDE, 0xAD, 0xBE, 0xEF], battery[4 * 0x800..4 * 0x800 + 4]);

    // The CPU window never reaches the tail: $6000-$7FFF is the four windows alone, so a write
    // at the top of it cannot overwrite the wavetable.
    core.write_memory(0x7FFF, 0x99);
    let battery = cartridge(&core).battery_ram();
    assert_eq!([0xDE, 0xAD, 0xBE, 0xEF], battery[4 * 0x800..4 * 0x800 + 4]);
    assert_eq!(0x99, battery[4 * 0x800 - 1]);
    assert_eq!(0x11, battery[0]);
}

/// Without a battery the RAM is still there — the chip has it whatever the header says — and the
/// slice a host would persist is empty.
#[test]
fn req_core_095_a_namco163_without_a_battery_still_has_the_chips_ram() {
    let mut core = core_163();
    load_ram(&mut core, 0x20, &[0x77]);
    assert_eq!(0x77, peek_ram(&mut core, 0x20));
    assert_eq!(0, cartridge(&core).battery_ram().len());
}

// ---------------------------------------------------------------- the write protection

/// nesdev: "Upper nybble must equal b0100 to enable writes", and bits A-D then "protect four
/// 2 KiB windows of external RAM ($6000-$67FF through $7800-$7FFF)" one by one.
#[test]
fn req_core_100_namco163_protects_its_prg_ram_by_2_kib_window() {
    let mut core = core_163();
    let windows = [0x6000u16, 0x6800, 0x7000, 0x7800];

    // The wrong upper nibble: nothing takes, whatever the low bits say.
    core.write_memory(0xF800, 0x00);
    for address in windows {
        core.write_memory(address, 0x11);
    }
    for address in windows {
        assert_eq!(
            0,
            core.read_memory(address),
            "write protected at {address:04X}"
        );
    }

    // $40: writes enabled and no window protected.
    core.write_memory(0xF800, 0x40);
    for (index, address) in windows.into_iter().enumerate() {
        core.write_memory(address, 0x20 + index as u8);
    }
    for (index, address) in windows.into_iter().enumerate() {
        assert_eq!(0x20 + index as u8, core.read_memory(address));
    }

    // One bit per window, bit 0 for the lowest: the protected windows keep their byte and the
    // others take the new one.
    core.write_memory(0xF800, 0x40 | 0x05); // windows 0 and 2 protected.
    for address in windows {
        core.write_memory(address, 0x99);
    }
    assert_eq!(0x20, core.read_memory(0x6000), "window 0 is protected");
    assert_eq!(0x99, core.read_memory(0x6800));
    assert_eq!(0x22, core.read_memory(0x7000), "window 2 is protected");
    assert_eq!(0x99, core.read_memory(0x7800));

    // Reads are never gated: the bytes above were read back through the protection.
}

// ---------------------------------------------------------------- mapper 210

/// nesdev, "INES Mapper 210": submapper 1 is the Namcot 175, whose mirroring is "fixed
/// hardwired mirroring per cartridge" — so `$E000`'s top two bits are not mirroring bits there,
/// and the board leaves the header's arrangement alone.
#[test]
fn req_core_100_the_namcot_175_keeps_the_headers_mirroring() {
    let mut core = core_210(1);
    assert_eq!(
        NametableMirroring::Horizontal,
        core.cartridge().unwrap().mirroring()
    );

    for value in [0x00u8, 0x40, 0x80, 0xC0] {
        core.write_memory(0xE000, value | 0x07);
        assert_eq!(
            NametableMirroring::Horizontal,
            core.cartridge().unwrap().mirroring(),
            "$E000 = {value:02X} must not move a 175's mirroring"
        );
        assert_eq!(
            0x07,
            prg_bank(&core, 0x8000),
            "the six bank bits still take"
        );
    }

    // And its nametables follow that mirroring rather than a select of its own.
    for table in 0..4 {
        assert_eq!(
            Cartridge::FOLLOW_MIRRORING,
            cartridge(&core).nametable_page(table)
        );
    }
}

/// Submapper 1 is also the only one of the three with PRG-RAM on mapper 210, and "$C000-$C7FF:
/// PRG-RAM enable (Mapper 175 only)" is one bit.
#[test]
fn req_core_100_the_namcot_175_enables_its_prg_ram_with_one_bit() {
    let mut core = core_210(1);

    core.write_memory(0x6000, 0x11);
    assert_eq!(0, core.read_memory(0x6000), "disabled at power-on");

    core.write_memory(0xC000, 0x01);
    core.write_memory(0x6000, 0x22);
    assert_eq!(0x22, core.read_memory(0x6000));

    core.write_memory(0xC000, 0x00);
    core.write_memory(0x6000, 0x33);
    assert_eq!(
        0x22,
        core.read_memory(0x6000),
        "disabling hides the write, and keeps what was there"
    );
}

/// Submapper 2 is the Namcot 340, which "allows runtime selection via bits 7-6 of register
/// $E000, supporting one-screen A, vertical, one-screen B, and horizontal modes" — the page's
/// four rows, in its order.
#[test]
fn req_core_100_the_namcot_340_picks_its_mirroring_from_e000() {
    let mut core = core_210(2);

    for (bits, expected) in [
        (0x00u8, NametableMirroring::SingleScreenA),
        (0x40, NametableMirroring::Vertical),
        (0x80, NametableMirroring::Horizontal),
        (0xC0, NametableMirroring::SingleScreenB),
    ] {
        core.write_memory(0xE000, bits | 0x09);
        assert_eq!(
            expected,
            core.cartridge().unwrap().mirroring(),
            "$E000 = {bits:02X}"
        );
        assert_eq!(
            0x09,
            prg_bank(&core, 0x8000),
            "the six bank bits still take"
        );
    }
}

/// "Mapper 340 contains no WRAM", so $6000-$7FFF is open bus on it — and neither 210 variant has
/// the chip's internal RAM, its IRQ counter or its sound.
#[test]
fn req_core_100_the_namcot_340_has_no_wram_and_neither_210_has_sound_or_an_irq() {
    let mut core = core_210(2);

    core.write_memory(0x6000, 0x55);
    assert_ne!(0x55, core.read_memory(0x6000));

    for submapper in [1u8, 2] {
        let mut core = core_210(submapper);
        // The IRQ registers are not decoded: a write there enables nothing.
        core.write_memory(0x5000, 0xFE);
        core.write_memory(0x5800, 0xFF);
        for _ in 0..200 {
            core.bus_mut().tick();
        }
        assert!(
            !cartridge(&core).irq_asserted(),
            "submapper {submapper} has no IRQ counter"
        );

        // And no expansion audio at all, so `M` stays at 0 for good.
        assert_eq!(None, cartridge(&core).expansion_audio_output());
        assert_eq!(0, expansion(&core));
    }
}

/// **Q-126.** Submapper 0 of mapper 210 is not assigned by the page. This build takes the
/// Namcot 175, the variant that leaves the header's mirroring alone, so that a 175 image is never
/// given a moving mirroring by what are bank bits to it.
#[test]
fn q_126_mapper_210_submapper_0_is_read_as_a_namcot_175() {
    let mut core = core_210(0);

    core.write_memory(0xE000, 0xC0 | 0x03);
    assert_eq!(
        NametableMirroring::Horizontal,
        core.cartridge().unwrap().mirroring(),
        "submapper 0 must behave as a 175 and keep the header's mirroring"
    );

    // And it has the 175's PRG-RAM enable at $C000 rather than a nametable select.
    core.write_memory(0xC000, 0x01);
    core.write_memory(0x6000, 0x44);
    assert_eq!(0x44, core.read_memory(0x6000));
}

// ---------------------------------------------------------------- the sound

/// **Q-127**, the level: the page gives a range per submapper rather than one figure, so the
/// provisional choice is 12.0 dB over an APU pulse at volume 15 — the midpoint of submapper 3's
/// documented 11.0-13.0 dB band — on Q-118's anchor. The table is a straight line, which is what
/// the page's "sum the channel outputs, and divide the output volume by the number of active
/// channels" makes it.
#[test]
fn req_core_075_the_namco163_table_is_linear_through_an_apu_pulse_at_12_db() {
    assert_eq!(226, mixer::NAMCO163_LEVELS, "15 x 15 steps and the zero");
    assert_eq!(0, mixer::NAMCO163_TABLE[0], "step 0 is silence");

    let top = f64::from(mixer::PULSE_TABLE[15]) * mixer::NAMCO163_GAIN;
    assert!(
        (f64::from(mixer::NAMCO163_TABLE[225]) - top).abs() <= 2.0,
        "the top step is {} and should be about {top:.0}",
        mixer::NAMCO163_TABLE[225]
    );

    // A straight line: every entry is its share of the top one, to the rounding.
    for step in 0..mixer::NAMCO163_LEVELS {
        let expected = top * step as f64 / 225.0;
        assert!(
            (f64::from(mixer::NAMCO163_TABLE[step]) - expected).abs() <= 2.0,
            "step {step} is {} and should be about {expected:.0}",
            mixer::NAMCO163_TABLE[step]
        );
    }

    // 12.0 dB is an amplitude ratio of about 3.98, and the top step is therefore about 19 467 —
    // the same order as the VRC6's 19 886, so `clip` needs nothing it does not already have.
    assert!((mixer::NAMCO163_GAIN - 3.981_071_705_534_972).abs() < 1e-12);
}

/// Power-on silence, and a channel that has a volume but no waveform is still silence: the sample
/// is 0 and `sample * volume` is 0.
#[test]
fn req_core_075_a_namco163_is_silent_until_a_waveform_is_loaded() {
    let mut core = core_163();
    for _ in 0..200 {
        core.bus_mut().tick();
    }
    assert_eq!(0, expansion(&core), "silent at power-on");

    // Channel 7 ($78-$7F) at full volume, with the wavetable still all zeroes.
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 15));
    for _ in 0..200 {
        core.bus_mut().tick();
    }
    assert_eq!(
        0,
        expansion(&core),
        "a zero waveform is silence at any volume"
    );
}

/// nesdev: "It takes exactly 15 CPU cycles to update and output one channel." With one channel
/// enabled that is the whole update rate, so the output moves on every fifteenth cycle and on no
/// other.
#[test]
fn req_core_075_the_namco163_updates_one_channel_every_fifteen_cpu_cycles() {
    let mut core = core_163();

    // A waveform that alternates $F and $0 every sample, and channel 7 reading it at full
    // volume with a frequency of exactly one sample per update — so `M` moves on every update
    // and on no other cycle.
    load_ram(&mut core, 0x00, &[0x0F, 0x0F]);
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 15));

    // Synchronise on an update first: `write_memory` runs a CPU cycle of its own, so the
    // multiplexer is part-way through its fifteen when the registers are in.
    assert_ne!(0, ticks_to_change(&mut core), "the chip must update at all");
    for _ in 0..4 {
        assert_eq!(
            15,
            ticks_to_change(&mut core),
            "one channel enabled: one update every fifteen CPU cycles"
        );
    }

    // 15 x 15 = 225 is one channel's full swing, which is the table's top step, and it is what
    // the $F samples give.
    let mut peak = 0;
    for _ in 0..30 {
        core.bus_mut().tick();
        peak = peak.max(expansion(&core));
    }

    assert_eq!(mixer::NAMCO163_TABLE[225], peak);
}

/// The waveform is read as 4-bit nibbles, "little-endian", so an even sample position is the low
/// nibble of its byte and an odd one the high nibble.
#[test]
fn req_core_075_a_namco163_waveform_is_nibbles_low_first() {
    let mut core = core_163();

    // Two bytes: samples 0, 1, 2, 3 are $1, $2, $3, $4.
    load_ram(&mut core, 0x00, &[0x21, 0x43]);
    // A frequency of exactly $10000 moves the sample position by one per update, so the four
    // samples come out in order.
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 1));

    // Synchronise on an update, then take one level per update.
    assert_ne!(0, ticks_to_change(&mut core));
    let mut seen = vec![expansion(&core)];
    for _ in 0..3 {
        for _ in 0..15 {
            core.bus_mut().tick();
        }

        seen.push(expansion(&core));
    }

    // Volume 1, so the level is the table entry of the sample itself. Which of the four the run
    // starts on depends on where the setup left the phase, so the four are compared as a cycle.
    let wanted = [
        mixer::NAMCO163_TABLE[1],
        mixer::NAMCO163_TABLE[2],
        mixer::NAMCO163_TABLE[3],
        mixer::NAMCO163_TABLE[4],
    ];
    let start = wanted
        .iter()
        .position(|level| *level == seen[0])
        .unwrap_or_else(|| panic!("{:?} is not one of {wanted:?}", seen[0]));
    for (step, level) in seen.iter().enumerate() {
        assert_eq!(wanted[(start + step) % 4], *level, "sample {step}");
    }
}

/// The waveform length is "(64-L)*4 4-bit samples", i.e. "256 - %LLLLLL00": the sample position
/// wraps inside it, so a four-sample waveform repeats every four updates.
#[test]
fn req_core_075_a_namco163_waveform_wraps_at_its_length() {
    let mut core = core_163();

    load_ram(&mut core, 0x00, &[0x21, 0x43]);
    // L = $3F << 2 = $FC, so the length is 256 - 252 = 4 samples.
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 1));

    assert_ne!(0, ticks_to_change(&mut core));
    let mut seen = vec![expansion(&core)];
    for _ in 0..7 {
        for _ in 0..15 {
            core.bus_mut().tick();
        }

        seen.push(expansion(&core));
    }

    let first: Vec<i32> = seen[..4].to_vec();
    assert_eq!(first, seen[4..], "the four samples repeat");
    assert_eq!(
        4,
        first.iter().collect::<std::collections::HashSet<_>>().len()
    );
}

/// The waveform address of `+6` is "Address of waveform (in 4-bit samples)", so it offsets the
/// sample position by **nibbles** and not by bytes.
///
/// The phase wraps at the waveform length and the address is added **after** that wrap, so over
/// four updates a channel at address `A` walks nibbles `A` to `A + 3`. With six distinct nibbles
/// loaded, the set of four samples names `A` outright: one nibble of address shifts the set by
/// one, where a byte would shift it by two.
#[test]
fn req_core_075_the_namco163_wave_address_is_counted_in_nibbles() {
    // Nibbles 0 to 5 are $1 to $6, and the length is four samples.
    let waveform = [0x21u8, 0x43, 0x65];

    let samples_at = |core: &mut NesCore, address: u8| -> Vec<usize> {
        load_ram(core, 0x00, &waveform);
        load_ram(core, 0x78, &channel_registers(0x10000, 0xFC, address, 1));

        let mut seen = Vec::new();
        for _ in 0..4 {
            // Exactly fifteen ticks is exactly one update, whatever the counter stood at.
            for _ in 0..15 {
                core.bus_mut().tick();
            }

            // Volume 1, so the level is the table entry of the sample itself.
            let level = expansion(core);
            let sample = (1..=6usize)
                .find(|sample| mixer::NAMCO163_TABLE[*sample] == level)
                .unwrap_or_else(|| panic!("address {address}: level {level} is not a sample"));
            seen.push(sample);
        }

        seen.sort_unstable();
        seen
    };

    let mut core = core_163();
    assert_eq!(vec![1, 2, 3, 4], samples_at(&mut core, 0x00));
    assert_eq!(
        vec![2, 3, 4, 5],
        samples_at(&mut core, 0x01),
        "one nibble of address must move the window by one sample"
    );
    assert_eq!(
        vec![3, 4, 5, 6],
        samples_at(&mut core, 0x02),
        "two nibbles -- one byte -- must move it by two"
    );
}

/// nesdev: `$7F` bits 6-4 are the channel count, "when C=0, only channel 8 enabled; when C=1,
/// channels 8 and 7 enabled" and so on — the **top** channels, counting down.
#[test]
fn req_core_075_the_namco163_channel_count_comes_from_7f_and_enables_the_top_channels() {
    let mut core = core_163();

    // A flat waveform of $F for every channel to read, and both channel 6 and channel 7 set up
    // at volume 15. $7F carries channel 7's volume *and* the count, so it is written last.
    load_ram(&mut core, 0x00, &[0xFF, 0xFF]);
    load_ram(&mut core, 0x70, &channel_registers(0x10000, 0xFC, 0x00, 15));
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 15));

    // C = 0: channel 7 alone. The sum is one channel's 225 over one channel.
    for _ in 0..200 {
        core.bus_mut().tick();
    }
    assert_eq!(mixer::NAMCO163_TABLE[225], expansion(&core));

    // C = 1: channels 7 and 6, and the page's rule is to "divide the output volume by the number
    // of active channels" — so two channels both at 225 still give 225.
    load_ram(&mut core, 0x7F, &[0x10 | 15]);
    for _ in 0..400 {
        core.bus_mut().tick();
    }
    assert_eq!(
        mixer::NAMCO163_TABLE[225],
        expansion(&core),
        "the divided sum is still one channel's swing"
    );

    // With channel 6 silent instead, two channels halve the output.
    load_ram(&mut core, 0x70, &channel_registers(0x10000, 0xFC, 0x00, 0));
    for _ in 0..400 {
        core.bus_mut().tick();
    }
    assert_eq!(
        mixer::NAMCO163_TABLE[112],
        expansion(&core),
        "225 and 0 over two channels is 112"
    );

    // And channel 5 is still out of the mix at C = 1, however loud its registers are: the count
    // enables the **top** channels and nothing else.
    load_ram(&mut core, 0x68, &channel_registers(0x10000, 0xFC, 0x00, 15));
    for _ in 0..400 {
        core.bus_mut().tick();
    }
    assert_eq!(mixer::NAMCO163_TABLE[112], expansion(&core));
}

/// The count also decides how long a channel waits for its turn: "higher channel counts result
/// in lower update frequencies", one channel every fifteen cycles round-robin.
#[test]
fn req_core_075_the_namco163_multiplexer_shares_fifteen_cycles_between_the_channels() {
    let mut core = core_163();

    load_ram(&mut core, 0x00, &[0x21, 0x43]);
    // Channel 7 only, then eight channels; the waveform moves by one sample per update either
    // way, so the number of ticks between two changes is 15 x the channel count. The volume is
    // 15 rather than 1 so that the sum stays visible once it is divided by eight.
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 15));

    assert_ne!(0, ticks_to_change(&mut core));
    assert_eq!(
        15,
        ticks_to_change(&mut core),
        "one channel: every 15 cycles"
    );

    // Eight channels: only channel 7 has a waveform to read, so `M` moves on its turn alone --
    // which comes round every 8 x 15 = 120 cycles.
    load_ram(&mut core, 0x7F, &[0x70 | 15]);
    assert_ne!(0, ticks_to_change(&mut core));
    assert_eq!(
        120,
        ticks_to_change(&mut core),
        "eight channels: every 120 cycles"
    );
}

/// nesdev, `$E000`: "Disable sound if set". It stops the multiplexer, so every channel holds the
/// output it last had and nothing moves until the bit is cleared.
#[test]
fn req_core_075_e000_bit_6_freezes_every_namco163_channel_in_place() {
    let mut core = core_163();

    load_ram(&mut core, 0x00, &[0x21, 0x43]);
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 1));

    assert_ne!(0, ticks_to_change(&mut core));
    let held = expansion(&core);
    assert_ne!(0, held);

    // Disable: a thousand cycles and the level does not budge.
    core.write_memory(0xE000, 0x40);
    for _ in 0..1000 {
        core.bus_mut().tick();
        assert_eq!(held, expansion(&core), "the sound must be frozen");
    }

    // Clearing it lets the multiplexer go on from where it stopped.
    core.write_memory(0xE000, 0x00);
    assert_ne!(0, ticks_to_change(&mut core));
    assert_ne!(held, expansion(&core));
}

/// `M` is positive and the APU **subtracts** it (AMD-061 §6.2), which is the polarity `clip`
/// implements and the one every other expansion-audio board in this build uses.
#[test]
fn req_core_075_a_namco163_subtracts_its_m_from_the_apu_mix() {
    let mut core = core_163();

    load_ram(&mut core, 0x00, &[0xFF, 0xFF]);
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 15));
    for _ in 0..200 {
        core.bus_mut().tick();
    }

    let m = expansion(&core);
    assert!(m > 0, "M must be positive: {m}");
    assert_eq!(mixer::NAMCO163_TABLE[225], m);
}

/// The phase accumulator is written **back into the chip's RAM**, so a game can read its own
/// channel's phase through `$4800` — the page puts the 24-bit value in `w[$7D/$7B/$79]`.
#[test]
fn req_core_075_the_namco163_phase_is_stored_back_into_the_chips_ram() {
    let mut core = core_163();

    load_ram(&mut core, 0x00, &[0xFF, 0xFF]);
    // Frequency $10000, so one update adds exactly one to the high byte of the phase.
    load_ram(&mut core, 0x78, &channel_registers(0x10000, 0xFC, 0x00, 15));

    for _ in 0..15 {
        core.bus_mut().tick();
    }

    // Channel 7's phase high byte is $7D.
    core.write_memory(0xF800, 0x40 | 0x7D);
    assert_eq!(
        1,
        core.read_memory(0x4800),
        "one update, one sample position"
    );

    for _ in 0..15 {
        core.bus_mut().tick();
    }

    core.write_memory(0xF800, 0x40 | 0x7D);
    assert_eq!(2, core.read_memory(0x4800));
}

// ---------------------------------------------------------------- save states

/// REQ-CORE-090 and REQ-CORE-091: everything the board and the chip hold comes back from a state,
/// and the machine goes on from exactly where it was.
///
/// The check is not field-by-field but **behavioural**: two machines are driven apart, one is
/// saved and restored, and the two then have to produce the same `M` on every one of 600 CPU
/// cycles. That covers the phase accumulators (which live in the chip's RAM), the per-channel
/// outputs, the multiplexer's counter and which channel it is on, the cached `M`, and the
/// wavetable itself.
#[test]
fn req_core_090_a_namco163_state_round_trips_through_the_sound() {
    let mut core = core_163();
    load_ram(&mut core, 0x00, &[0x21, 0x43, 0x65, 0x87]);
    load_ram(&mut core, 0x70, &channel_registers(0x00123, 0xF8, 0x02, 9));
    load_ram(&mut core, 0x78, &channel_registers(0x01234, 0xFC, 0x00, 15));
    load_ram(&mut core, 0x7F, &[0x10 | 15]); // two channels, channel 7 at full volume.
    for _ in 0..400 {
        core.bus_mut().tick();
    }

    let state = core.save_state(true).unwrap();
    let expected: Vec<i32> = (0..600)
        .map(|_| {
            core.bus_mut().tick();
            expansion(&core)
        })
        .collect();

    // Drive the machine somewhere else entirely, then come back.
    core.write_memory(0xE000, 0x40); // the sound disabled, for good measure.
    for _ in 0..1000 {
        core.bus_mut().tick();
    }

    core.try_load_state(&state, 0).unwrap();
    let after: Vec<i32> = (0..600)
        .map(|_| {
            core.bus_mut().tick();
            expansion(&core)
        })
        .collect();

    assert_eq!(
        expected, after,
        "the sound must resume where the state left it"
    );
    assert!(
        expected.iter().any(|level| *level != expected[0]),
        "the test is vacuous unless M actually moves"
    );
}

/// The chip's 128 bytes are PRG-RAM, so `Cartridge::save_state` carries them — the wavetable and
/// the channel registers come back byte for byte, and so does the address port that reads them.
#[test]
fn req_core_091_a_namco163_state_brings_back_the_wavetable_and_the_port() {
    let mut core = core_163();
    load_ram(&mut core, 0x10, &[0xDE, 0xAD, 0xBE, 0xEF]);
    set_port(&mut core, 0x11, true);

    let state = core.save_state(true).unwrap();

    // Overwrite all of it, port included.
    load_ram(&mut core, 0x10, &[0, 0, 0, 0]);
    set_port(&mut core, 0x7A, false);

    core.try_load_state(&state, 0).unwrap();
    assert_eq!(0xDE, peek_ram(&mut core, 0x10));
    assert_eq!(
        [0xAD, 0xBE, 0xEF],
        [
            peek_ram(&mut core, 0x11),
            peek_ram(&mut core, 0x12),
            peek_ram(&mut core, 0x13)
        ]
    );

    // And the port is back at $11 with its auto-increment, which a CPU read then steps on.
    core.try_load_state(&state, 0).unwrap();
    assert_eq!(0xAD, cpu_read(&mut core, 0x4800));
    assert_eq!(0xBE, cpu_read(&mut core, 0x4800));
}

/// The IRQ counter and its pending flag are part of the state: a restore puts the line back where
/// it was rather than where the new run had got to.
#[test]
fn req_core_091_a_namco163_state_brings_back_the_irq_counter() {
    let mut core = core_163();
    let start = 0x7FFF - 30;
    core.write_memory(0x5000, (start & 0xFF) as u8);
    core.write_memory(0x5800, 0x80 | ((start >> 8) & 0x7F) as u8);
    for _ in 0..10 {
        core.bus_mut().tick();
    }

    let state = core.save_state(true).unwrap();
    let low = core.read_memory(0x5000);
    let high = core.read_memory(0x5800);

    // Let it fire, then come back to before the edge.
    for _ in 0..200 {
        core.bus_mut().tick();
    }
    assert!(cartridge(&core).irq_asserted());

    core.try_load_state(&state, 0).unwrap();
    assert!(
        !cartridge(&core).irq_asserted(),
        "the restored counter is short of its top"
    );
    assert_eq!(low, core.read_memory(0x5000));
    assert_eq!(high, core.read_memory(0x5800));

    // And it fires again from there, which is the check that the enable bit came back too.
    let mut fired = false;
    for _ in 0..60 {
        core.bus_mut().tick();
        fired |= cartridge(&core).irq_asserted();
    }
    assert!(fired, "the restored counter must go on counting");
}

/// REQ-CORE-091: a block written **before** this board existed still loads. The format version
/// sits right after the four magic bytes, so an older one is what a state from a build without
/// the Namco 163 carries — and the core has to read it without the board's own block.
#[test]
fn req_core_091_a_block_written_before_the_namco163_still_loads() {
    let mut core = core_163();
    load_ram(&mut core, 0x78, &channel_registers(0x01234, 0xFC, 0x00, 15));
    for _ in 0..200 {
        core.bus_mut().tick();
    }

    let mut state = core.save_state(true).unwrap();
    state[4..8].copy_from_slice(&NesCore::STATE_VERSION_4.to_le_bytes());

    core.try_load_state(&state, 0).unwrap();
    core.bus_mut().tick();
    assert_eq!(
        0,
        expansion(&core),
        "an older block leaves the chip silent rather than failing"
    );
}

/// A mapper-210 state round-trips too, and the 340's mirroring — which is a register on that
/// variant and the header's on the other two — comes back from the state and not from `$E000`.
#[test]
fn req_core_090_a_mapper_210_state_round_trips_its_mirroring_and_banks() {
    let mut core = core_210(2);
    core.write_memory(0xE000, 0x40 | 0x05); // vertical, bank 5.
    core.write_memory(0xE800, 0x06);
    core.write_memory(0x8000, 0x21);

    let state = core.save_state(true).unwrap();

    core.write_memory(0xE000, 0xC0 | 0x11); // one-screen B, bank 17.
    core.write_memory(0x8000, 0x33);

    core.try_load_state(&state, 0).unwrap();
    assert_eq!(
        NametableMirroring::Vertical,
        core.cartridge().unwrap().mirroring()
    );
    assert_eq!(0x05, prg_bank(&core, 0x8000));
    assert_eq!(0x06, prg_bank(&core, 0xA000));
    assert_eq!(0x21, chr_bank(&core, 0x0000));
}
