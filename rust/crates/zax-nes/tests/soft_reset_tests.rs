//! The soft reset of AMD-044 REQ-CORE-122: the RESET button of the console, as opposed to the
//! power cycle REQ-CORE-004 keeps calling a reset (DEC-02-Q3, unchanged).
#![allow(clippy::cast_possible_truncation)] // test code: the addresses are literals

mod common;

use common::mapper_fixture::{MapperMachine, MapperRom};
use common::{machine_with, minimal, minimal_rom};
use zax_nes::{ControllerPortType, NesCore};

/// Writes one PPU-bus byte through $2006 / $2007, as a game would.
fn write_vram(core: &mut NesCore, address: u16, value: u8) {
    core.write_memory(0x2006, (address >> 8) as u8);
    core.write_memory(0x2006, address as u8);
    core.write_memory(0x2007, value);
}

/// Reads one PPU-bus byte through $2006 / $2007. The first $2007 read returns the stale buffer
/// and loads the byte at the address; the second one hands that byte over.
fn read_vram(core: &mut NesCore, address: u16) -> u8 {
    core.write_memory(0x2006, (address >> 8) as u8);
    core.write_memory(0x2006, address as u8);
    core.ppu_read(7);
    core.ppu_read(7)
}

/// An NROM image declaring **no** CHR bank, which gives the cartridge CHR RAM. The three
/// vectors point at an endless loop.
fn chr_ram_image() -> Vec<u8> {
    const PRG_SIZE: usize = 2 * 16 * 1024;
    let mut image = vec![0u8; 16 + PRG_SIZE];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 0x02;
    image[5] = 0x00;
    image[6] = 0x01;

    image[16..19].copy_from_slice(&[0x4C, 0x00, 0x80]);
    let vectors = 16 + PRG_SIZE - 6;
    for i in 0..3 {
        image[vectors + 2 * i] = 0x00;
        image[vectors + 2 * i + 1] = 0x80;
    }
    image
}

/// REQ-CORE-122: the internal RAM, the save RAM and the nametable RAM survive a soft reset,
/// where the hard one clears all three (REQ-CORE-004, DEC-02-Q3).
#[test]
fn req_core_122_a_soft_reset_keeps_the_ram_and_the_save_ram() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    core.write_memory(0x0300, 0x55);
    core.write_memory(0x6000, 0x66);
    write_vram(&mut core, 0x2010, 0x77);

    core.run_frame(0);
    core.soft_reset();

    assert_eq!(0x55, core.read_memory(0x0300));
    assert_eq!(0x66, core.read_memory(0x6000));
    assert_eq!(0x77, read_vram(&mut core, 0x2010));
    assert_eq!(Some("Minimal.nes"), core.rom_name());

    // The hard reset still clears all three: DEC-02-Q3 is untouched.
    core.reset();
    assert_eq!(0x00, core.read_memory(0x0300));
    assert_eq!(0x00, core.read_memory(0x6000));
    assert_eq!(0x00, read_vram(&mut core, 0x2010));
}

/// REQ-CORE-122: the CHR RAM of a cartridge that has one survives a soft reset. The ROM
/// declares no CHR bank, so the mapper gives the PPU 8 KiB of writable pattern memory.
#[test]
fn req_core_122_a_soft_reset_keeps_the_chr_ram() {
    let mut core = machine_with(&chr_ram_image(), "ChrRam.nes");

    write_vram(&mut core, 0x0000, 0x5A);
    assert_eq!(0x5A, read_vram(&mut core, 0x0000));

    core.soft_reset();
    assert_eq!(0x5A, read_vram(&mut core, 0x0000));

    core.reset();
    assert_eq!(0x00, read_vram(&mut core, 0x0000));
}

/// REQ-CORE-122: A, X, Y and every flag but `I` are kept, S drops by 3 without anything being
/// written to the stack, and PC comes from the reset vector (nesdev, "CPU power up state",
/// *after reset*).
#[test]
fn req_core_122_the_cpu_keeps_its_registers_and_sets_i() {
    use zax_nes::cpu::Cpu6502;

    let mut core = machine_with(&minimal(), "Minimal.nes");
    {
        let (cpu, _) = core.cpu_and_bus_mut();
        cpu.a = 0x12;
        cpu.x = 0x34;
        cpu.y = 0x56;
        cpu.s = 0x80;
        cpu.set_flag(Cpu6502::FLAG_C | Cpu6502::FLAG_N, true);
        cpu.set_flag(Cpu6502::FLAG_I, false);
    }

    // Three bytes of the stack page, to show that the three decrements write nothing.
    core.write_memory(0x0180, 0xAA);
    core.write_memory(0x017F, 0xBB);
    core.write_memory(0x017E, 0xCC);

    core.soft_reset();

    let cpu = core.cpu();
    assert_eq!(0x12, cpu.a);
    assert_eq!(0x34, cpu.x);
    assert_eq!(0x56, cpu.y);
    assert_eq!(0x80 - 3, cpu.s);
    assert_eq!(0x8000, cpu.pc);
    assert!(cpu.get_flag(Cpu6502::FLAG_I));
    assert!(cpu.get_flag(Cpu6502::FLAG_C));
    assert!(cpu.get_flag(Cpu6502::FLAG_N));
    // No interrupt is left waiting.
    assert!(!cpu.nmi_pending());

    assert_eq!(0xAA, core.read_memory(0x0180));
    assert_eq!(0xBB, core.read_memory(0x017F));
    assert_eq!(0xCC, core.read_memory(0x017E));
}

/// REQ-CORE-122: the PPU follows its documented *after reset* state — PPUCTRL and PPUMASK at 0,
/// the write latch cleared, the `$2007` read buffer at 0 — while OAMADDR, the nametable RAM and
/// the OAM stay as they were.
#[test]
fn req_core_122_the_ppu_follows_its_after_reset_state() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    write_vram(&mut core, 0x2010, 0x77);
    core.write_memory(0x2000, 0x9E);
    core.write_memory(0x2001, 0x1E);
    core.write_memory(0x2003, 0x40);
    // One half of a $2006 pair leaves the write latch set.
    core.write_memory(0x2006, 0x21);

    core.soft_reset();

    let ppu = core.bus().ppu();
    assert_eq!(0, ppu.control());
    assert_eq!(0, ppu.mask());
    assert_eq!(0x40, ppu.oam_address());

    // The latch is cleared, so the next two writes form a complete address again and the read
    // lands on $2010 rather than on $2110.
    assert_eq!(0x77, read_vram(&mut core, 0x2010));
}

/// REQ-CORE-122: the APU goes silent as a `$4015 = 0` write would, and the frame IRQ flag goes
/// away. The `$4017` mode is unchanged.
#[test]
fn req_core_122_the_apu_is_silenced_and_4015_reads_zero() {
    let mut core = machine_with(&minimal(), "Minimal.nes");

    // Arm the two pulse channels and let their length counters load.
    core.write_memory(0x4015, 0x0F);
    core.write_memory(0x4000, 0x30);
    core.write_memory(0x4003, 0x08);
    core.write_memory(0x4004, 0x30);
    core.write_memory(0x4007, 0x08);
    core.run_frame(0);
    assert_ne!(0, core.bus().apu().peek_status() & 0x03);

    core.soft_reset();

    assert_eq!(0, core.bus().apu().peek_status());
    assert!(!core.bus().apu().irq_asserted());
}

/// REQ-CORE-006 as amended by AMD-044: the core frame counter restarts from 0 on a soft reset,
/// as it does when a ROM is loaded and on a hard reset.
#[test]
fn req_core_006_the_frame_counter_restarts_from_zero() {
    let mut core = machine_with(&minimal(), "Minimal.nes");
    for _ in 0..5 {
        core.run_frame(0);
    }
    assert_eq!(5, core.frame_count());

    core.soft_reset();
    assert_eq!(0, core.frame_count());

    core.run_frame(0);
    assert_eq!(1, core.frame_count());
}

/// REQ-CORE-122: the latch and the shift registers of the controller ports go to zero. The
/// buttons the player is holding are read again on the next frame, so a held A still reaches
/// the game.
#[test]
fn req_core_122_the_port_latch_and_shift_registers_are_cleared() {
    // The ports are driven directly: a host peek of $4016 reads back as 0 by REQ-CORE-081, so
    // it cannot show what the shift registers hold.
    fn strobe_and_read(core: &mut NesCore) -> u8 {
        let input = core.bus_mut().input_mut();
        // A get cycle for the strobe and a later one for the read, so the reload the first
        // write arms for the next put cycle has landed (see `InputPorts::write`).
        input.write(0x01, 10);
        input.write(0x00, 14);
        input.read(0, 0, 20, |_| false) & 0x01
    }

    let mut core = machine_with(&minimal(), "Minimal.nes");
    core.set_port_type(1, ControllerPortType::Zapper).unwrap();

    // The player holds A: the strobe latches it and the first read hands it over.
    core.run_frame(0x0001);
    assert_eq!(1, strobe_and_read(&mut core));

    core.soft_reset();

    // The latch is down and the shift registers are empty, so a read with no strobe returns 0
    // even though the button is still held.
    assert_eq!(
        0,
        core.bus_mut().input_mut().read(0, 0, 20, |_| false) & 0x01
    );

    // The port types survive, exactly as they do across a hard reset (REQ-CORE-004).
    assert_eq!(Ok(ControllerPortType::Zapper), core.port_type(1));

    // The next frame reads the held buttons again (AMD-044 §2).
    core.run_frame(0x0001);
    assert_eq!(1, strobe_and_read(&mut core));
}

/// REQ-CORE-122 complement: a home-made ROM that counts its resets in RAM shows 3 after three
/// soft resets. `INC $10` at $8000, then `JMP $8002` — the jump to itself: exactly one
/// increment per trip through the reset vector.
#[test]
fn req_core_122_a_rom_that_counts_its_resets_in_ram_shows_three() {
    let image = minimal_rom(0x01, Some(&[0xE6, 0x10, 0x4C, 0x02, 0x80]), None);
    let mut core = machine_with(&image, "Counter.nes");

    core.run_frame(0);
    assert_eq!(1, core.read_memory(0x0010));

    for _ in 0..2 {
        core.soft_reset();
        core.run_frame(0);
    }
    assert_eq!(3, core.read_memory(0x0010));

    // The hard reset, being a power cycle, loses the count (REQ-CORE-004, DEC-02-Q3).
    core.reset();
    core.run_frame(0);
    assert_eq!(1, core.read_memory(0x0010));
}

/// A soft reset with no ROM loaded does nothing at all (REQ-CORE-005).
#[test]
fn req_core_122_a_soft_reset_without_a_rom_does_nothing() {
    let mut core = NesCore::new();
    core.soft_reset();

    assert_eq!(None, core.rom_name());
    assert_eq!(0, core.frame_count());
}

// ------------------------------------------------------------------- the mapper (EVAL-SYNC-3)

/// **The mapper does not see the RESET button**, so its registers, its IRQ unit and its A12
/// watcher hold whatever the game last wrote into them.
///
/// nesdev's "Cartridge connector" lists every pin of both the 60-pin Famicom and the 72-pin NES
/// edge — power, the CPU address, data and control lines, the PPU lines, CIRAM, audio, `/IRQ`,
/// `M2`, `/ROMSEL`, `SYSTEM CLK`, the expansion pins and the CIC's two — and there is **no
/// `/RESET` among them**. nesdev's "CPU power up state" *after reset* says the internal RAM is
/// unchanged and does not mention the cartridge at all.
///
/// Until lot EVAL-SYNC-3 `NesCore::soft_reset` called `Cartridge::reset`, which put every board
/// back to power-on; `docs/review/AMD-REQ-CORE-122-mapper-reset.md` is the amendment that asks
/// of REQ-CORE-122. The MMC3 is the board to measure it on because it is the one with a licensed
/// library of games a player presses RESET on.
#[test]
fn req_core_122_a_soft_reset_leaves_the_mmc3_alone() {
    let mut core = MapperRom::new(4).prg(32).chr(128).load();
    assert_eq!(
        [0, 1, 30, 31],
        core.prg_windows(),
        "the MMC3's own power-on layout, so that the writes below can be seen to move it"
    );

    // Four bank registers and the mirroring, all of them things the game chose.
    core.mmc3_bank(6, 0x05);
    core.mmc3_bank(7, 0x09);
    core.mmc3_bank(0, 0x40);
    core.mmc3_bank(1, 0x42);
    core.mapper_write(0xA000, 0x01);
    let windows = core.prg_windows();
    let chr = core.chr_windows();
    let mirroring = core.mirroring();
    assert_eq!([5, 9, 30, 31], windows, "the writes have to land first");

    core.soft_reset();

    assert_eq!(
        windows,
        core.prg_windows(),
        "a press on RESET cannot take the PRG banks away: the chip never sees the line"
    );
    assert_eq!(chr, core.chr_windows(), "nor the CHR banks");
    assert_eq!(mirroring, core.mirroring(), "nor the mirroring it selected");

    // And the power cycle still does take them away (REQ-CORE-004, DEC-02-Q3 — unchanged).
    core.reset();
    assert_eq!([0, 1, 30, 31], core.prg_windows());
}

/// The same for the MMC3's **IRQ unit**, which is the half that decides where a game's status
/// bar splits: `$C000` (latch), `$C001` (reload), `$E001` (enable) survive a soft reset, so a
/// game that re-enters its reset vector without rewriting them keeps its split.
///
/// Measured through the **line** rather than through the registers, and the A12 rises are made
/// by hand as the other MMC3-clone IRQ tests of this workspace make them — rendering off, so
/// that only these rises clock the counter and the count is exact.
#[test]
fn req_core_122_a_soft_reset_keeps_the_mmc3_irq_armed() {
    /// Clocks the counter `times` times: one low and one high of the filtered A12 each, far
    /// enough apart in CPU cycles that the watcher's filter counts every one.
    fn clock_a12(core: &mut NesCore, times: i64, from: i64) {
        for i in 0..times {
            let cart = core.cartridge_mut().unwrap();
            cart.on_ppu_address_bus(0x0000, from + i * 100);
            cart.on_ppu_address_bus(0x1000, from + i * 100 + 50);
        }
    }

    let mut core = MapperRom::new(4).prg(32).chr(128).load();
    core.write_memory(0x2001, 0x00); // No rendering: only the rises below clock it.
                                     // A latch of 4, reloaded and enabled, as a game arms it in vblank.
    core.mapper_write(0xC000, 0x04);
    core.mapper_write(0xC001, 0x00);
    core.mapper_write(0xE001, 0x00);
    clock_a12(&mut core, 8, 10_000);
    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "the IRQ has to fire first, or the test below proves nothing"
    );

    // $E000 acknowledges and disables, which is what a handler does; $E001 re-enables, and the
    // latch is **not** rewritten. Then the reset.
    core.mapper_write(0xE000, 0x00);
    assert!(!core.cartridge().unwrap().irq_asserted());
    core.mapper_write(0xE001, 0x00);
    core.soft_reset();

    clock_a12(&mut core, 8, 20_000);
    assert!(
        core.cartridge().unwrap().irq_asserted(),
        "the latch and the enable survive a press on RESET, so the counter fires again with no \
         further write"
    );

    // A power cycle does clear them: nothing fires however long the counter is clocked.
    core.reset();
    core.write_memory(0x2001, 0x00);
    clock_a12(&mut core, 40, 30_000);
    assert!(
        !core.cartridge().unwrap().irq_asserted(),
        "a power cycle still puts the IRQ unit back (REQ-CORE-004, DEC-02-Q3)"
    );
}

/// **The exception, and the only board of this build that has one**: a reset-detecting multicart.
///
/// nesdev, "NES 2.0 Mapper 313" (reset-based multicarts around the MMC3 ASIC): *"When an
/// interruption of the M2 signal is detected, the higher address lines are automatically
/// incremented."* They see the reset through the **absence of M2**, not through a pin. nesdev,
/// "INES Mapper 215", says it of this board in so many words: *"The Outer Bank Register is reset
/// to its power-up value if a Reset has been detected via an interruption of the M2 signal. This
/// makes it difficult to dump these cartridges using a Kazzo device."*
///
/// There is **no image of a mapper-215 board under `assets-free/test-roms`** (§3.4 of
/// `docs/review/MF-6-inventory.md`: 48 MMC3-clone numbers, none with a licensed title), so the
/// image is the synthetic one `MapperRom` builds, as every other mapper-215 test of this
/// workspace uses.
#[cfg(feature = "board-215")]
#[test]
fn req_core_122_a_reset_detecting_multicart_puts_its_outer_bank_back() {
    let mut core = MapperRom::new(215).prg(192).chr(1536).load();
    // Out of the NROM override, so that the MMC3's own banking is what the windows show.
    core.mapper_write(0x5000, 0x00);
    core.mmc3_bank(6, 0x01);
    core.mapper_write(0x5001, 0x02);
    assert_eq!(
        1 + 2 * 32,
        core.prg_windows()[0],
        "outer bank 2 over MMC3 bank 1"
    );

    core.soft_reset();

    // $5001 powers up at $0F, whose two outer PRG bits are 3.
    assert_eq!(
        1 + 3 * 32,
        core.prg_windows()[0],
        "the outer bank goes back to its power-up value, which is the whole of what the page \
         says a detected reset does"
    );
    // And the MMC3 chip behind it is untouched: bank register 6 still holds 1.
    core.mapper_write(0x5001, 0x02);
    assert_eq!(
        1 + 2 * 32,
        core.prg_windows()[0],
        "the chip's own registers never saw the reset either"
    );
}

/// Lot FIDELITY step 34 (`emu.ppu_reset_line`, on): a reset reaches the PPU's /RST pin, as on the
/// front-loading NES. Until the pre-render line the PPU ignores `$2001` (and `$2000`, `$2005`,
/// `$2006`), and the Reset button sends it back to the top of the picture (the transistor netlist:
/// held at line 0 dot 0 while /RES is low, `after_reset_reg` cleared on dot 1 of line 261).
#[test]
fn fidelity_reset_line_shuts_the_ppu_writes_until_the_pre_render_line() {
    fn run_to(core: &mut NesCore, line: i32, dot: i32) {
        let bus = core.bus_mut();
        while !(bus.ppu().scanline() == line && bus.ppu().dot() >= dot) {
            bus.tick();
            bus.sync_ppu();
        }
    }
    let mut core = machine_with(&minimal(), "Minimal.nes");
    core.set_ppu_reset_line(true);
    core.set_power_on_top(true);
    for reset in [false, true] {
        core.run_frame(0);
        core.run_frame(0);
        if reset {
            core.soft_reset();
            assert_eq!(
                0,
                core.bus().ppu().scanline(),
                "the Reset button restarts the picture"
            );
        } else {
            core.reset();
        }
        core.write_memory(0x2001, 0x1E);
        assert!(
            !core.bus().ppu().rendering_enabled(),
            "ignored right after the reset"
        );
        run_to(&mut core, 260, 330);
        core.write_memory(0x2001, 0x1E);
        assert!(
            !core.bus().ppu().rendering_enabled(),
            "still ignored at the end of line 260"
        );
        run_to(&mut core, 261, 3);
        core.write_memory(0x2001, 0x1E);
        assert!(
            core.bus().ppu().rendering_enabled(),
            "taken from dot 1 of line 261"
        );
        core.write_memory(0x2001, 0x00);
    }

    core.set_ppu_reset_line(false);
    core.run_frame(0);
    core.soft_reset();
    core.write_memory(0x2001, 0x1E);
    assert!(core.bus().ppu().rendering_enabled(), "off: taken at once");
}

/// Lot FIDELITY step 36 (`emu.power_on_ppu_ram`): the power-on palette of blargg's console, as
/// his `power_up_palette` reads it, mirrors included; zeros by default.
#[test]
fn fidelity_power_on_palette_can_be_blarggs_console() {
    let mut core = machine_with(&minimal(), "Minimal.nes");
    let palette = |core: &mut NesCore| -> Vec<u8> {
        (0..32u16)
            .map(|index| {
                // A palette read is not buffered: one read hands the byte over.
                let address = 0x3F00 + index;
                core.write_memory(0x2006, (address >> 8) as u8);
                core.write_memory(0x2006, address as u8);
                core.ppu_read(7) & 0x3F
            })
            .collect()
    };
    core.reset();
    assert!(
        palette(&mut core).iter().all(|&byte| byte == 0),
        "zeros by default"
    );

    core.set_power_on_ppu_ram(zax_nes::PowerOnPpuRam::Blargg);
    core.reset();
    assert_eq!(
        palette(&mut core),
        zax_nes::BLARGG_POWER_UP_PALETTE.to_vec()
    );
}
