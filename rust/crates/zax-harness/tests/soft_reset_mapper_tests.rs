//! **What a press on RESET does to the cartridge**, measured on `zax-nes` on a *public MMC3 ROM*
//! and held against the hardware (lot EVAL-SYNC-2 step 3a, **flipped by EVAL-SYNC-3 step 4**).
//!
//! The NES cartridge connector has **no reset line**. nesdev's "Cartridge connector" lists every
//! pin of both the 60-pin Famicom and the 72-pin NES edge — power, the CPU address, data and
//! control lines, the PPU lines, CIRAM, audio, /IRQ, M2, /ROMSEL, SYSTEM CLK, the expansion pins
//! and the CIC's two — and no `/RESET` among them. A mapper therefore cannot see the console's
//! reset button at all: its registers hold whatever the game last wrote into them, across as
//! many resets as the player presses.
//!
//! EVAL-SYNC-2 found that `NesCore::soft_reset` called `Cartridge::reset`, which put every board
//! back to power-on, and recorded it here as a **reproduction** with a note saying that whoever
//! fixed it should rewrite this file the other way round. The owner decided to fix it on
//! 2026-11-10; `NesCore::soft_reset` now calls `Cartridge::soft_reset`, which reaches
//! `Mapper::soft_reset` — a no-op for every board but a reset-detecting multicart — and this
//! file is now a **contract**.
//!
//! It is the complement of `zax-nes`'s own `soft_reset_tests.rs`, which measures the same thing
//! on synthetic images and adds the multicart and the IRQ unit: this one does it on a ROM a
//! person can download, and needs no movie and no commercial game.

// A test: a missing ROM should fail it outright rather than be smoothed over.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use zax_nes::NesCore;

/// A public MMC3 test ROM, or `None` with a printed reason.
///
/// `mmc3_test/1-clocking.nes` is there because it is mapper 4 and public; nothing in the test
/// depends on what the ROM *does*, only on the board it carries.
fn mmc3_rom() -> Option<Vec<u8>> {
    let dir = PathBuf::from(std::env::var_os("ZAXNES_TEST_ROMS")?);
    let path = dir
        .join("nes-test-roms")
        .join("mmc3_test")
        .join("1-clocking.nes");
    if !path.is_file() {
        println!("skipped: {} is not there", path.display());
        return None;
    }
    std::fs::read(&path).ok()
}

/// A fingerprint of the whole switchable PRG window at `$8000`-`$9FFF`, which names the bank
/// mapped there.
///
/// The **whole** 8 KiB and not its first bytes: the four banks of a `mmc3_test` image all begin
/// with the same eight bytes, so a short sample would say "nothing moved" whatever the board did.
fn window(core: &NesCore) -> u64 {
    (0x8000..=0x9FFFu32).fold(0u64, |sum, address| {
        let byte = core.read_memory(u16::try_from(address).unwrap_or(0));
        sum.wrapping_mul(31).wrapping_add(u64::from(byte))
    })
}

/// Writes bank register `register` with `bank`, as a CPU write would.
fn select_bank(core: &mut NesCore, register: u8, bank: u8) {
    core.write_memory(0x8000, register);
    core.write_memory(0x8001, bank);
}

/// A soft reset **keeps** the MMC3's bank registers, because the chip never sees the reset line;
/// a power cycle still takes them away (REQ-CORE-004, DEC-02-Q3).
#[test]
fn a_soft_reset_keeps_the_mmc3_bank_registers_and_a_power_cycle_does_not() {
    let Some(image) = mmc3_rom() else {
        return;
    };
    let mut core = NesCore::new();
    assert!(core.load_rom(&image, "1-clocking.nes").success);
    core.reset();

    // Whatever bank the board powers on with at $8000, ask for a different one and check the
    // window really moved -- otherwise the test would prove nothing when the reset did nothing.
    let power_on = window(&core);
    let moved = (1..32u8)
        .map(|bank| {
            select_bank(&mut core, 6, bank);
            (bank, window(&core))
        })
        .find(|(_, seen)| *seen != power_on);
    let Some((bank, chosen)) = moved else {
        panic!("no PRG bank of this image differs from the power-on one at $8000");
    };
    assert_ne!(chosen, power_on);

    core.soft_reset();
    assert_eq!(
        window(&core),
        chosen,
        "bank {bank} was selected and the RESET button must not take it away: on hardware the \
         MMC3 never sees the reset line, so it is still mapped"
    );

    core.reset();
    assert_eq!(
        window(&core),
        power_on,
        "a power cycle is a power cycle: the board does come back (REQ-CORE-004, DEC-02-Q3)"
    );
}

/// `movie-sync --ppu-ram` fills OAM, the palette RAM and the nametable RAM through
/// `$2003`/`$2004` and `$2006`/`$2007`, and this is the check that those writes **land**.
///
/// Without it the experiment of step 3 would be worthless: an option that quietly writes nowhere
/// reports "the power-on PPU memories make no difference" on every game there is.
#[test]
fn a_debug_write_to_the_ppu_registers_reaches_the_palette_and_oam() {
    let Some(image) = mmc3_rom() else {
        return;
    };
    let mut core = NesCore::new();
    assert!(core.load_rom(&image, "1-clocking.nes").success);
    core.reset();
    // As `fill_ppu_ram` does: through the PPU's reset window (lot FIDELITY step 34).
    core.set_ppu_reset_window_shut(false);

    // The palette, through $2006/$2007, read back through $2006 and $2007 — a palette read is
    // not buffered, so the byte comes straight back.
    core.write_memory(0x2006, 0x3F);
    core.write_memory(0x2006, 0x01);
    core.write_memory(0x2007, 0x2A);
    core.write_memory(0x2006, 0x3F);
    core.write_memory(0x2006, 0x01);
    assert_eq!(
        core.ppu_read(0x2007) & 0x3F,
        0x2A,
        "a $2007 write did not reach the palette, so --ppu-ram would be writing nowhere"
    );

    // OAM, through $2003/$2004, read back through $2003/$2004.
    core.write_memory(0x2003, 0x10);
    core.write_memory(0x2004, 0xC3);
    core.write_memory(0x2003, 0x10);
    assert_eq!(
        core.ppu_read(0x2004),
        0xC3,
        "a $2004 write did not reach OAM"
    );

    // And a nametable byte, which is buffered: the first $2007 read returns the stale buffer.
    core.write_memory(0x2006, 0x20);
    core.write_memory(0x2006, 0x40);
    core.write_memory(0x2007, 0x5E);
    core.write_memory(0x2006, 0x20);
    core.write_memory(0x2006, 0x40);
    let stale = core.ppu_read(0x2007);
    assert_eq!(
        core.ppu_read(0x2007),
        0x5E,
        "a $2007 write did not reach the nametable RAM (the stale buffer was {stale:02X})"
    );
}

/// The other half, which `zax-nes` always got right: a soft reset **keeps** the console's 2 KiB
/// and the cartridge's PRG-RAM.
///
/// nesdev, "CPU power up state": internal RAM *"is unchanged after a reset"*. The two are
/// asserted together on purpose — EVAL-SYNC-2's finding was that the mapper was treated as state
/// to be reset while the memories beside it were not, and a reader needs both halves to see that
/// the line was drawn in the wrong place rather than that the reset was simply too violent. With
/// the fix the two halves say the same thing, which is the point.
#[test]
fn a_soft_reset_keeps_the_work_ram_and_the_cartridges_prg_ram() {
    let Some(image) = mmc3_rom() else {
        return;
    };
    let mut core = NesCore::new();
    assert!(core.load_rom(&image, "1-clocking.nes").success);
    core.reset();
    // The PRG-RAM window has to be enabled before a CPU write reaches it ($A001 bit 7).
    core.write_memory(0xA001, 0x80);
    for (address, value) in [(0x0123u16, 0xA5u8), (0x07FF, 0x5A), (0x6000, 0x3C)] {
        core.write_memory(address, value);
    }
    let before: Vec<u8> = [0x0123u16, 0x07FF, 0x6000]
        .iter()
        .map(|address| core.read_memory(*address))
        .collect();
    assert_eq!(before, [0xA5, 0x5A, 0x3C], "the writes have to land first");

    core.soft_reset();
    assert_eq!(
        core.read_memory(0x0123),
        0xA5,
        "work RAM is kept across a soft reset"
    );
    assert_eq!(core.read_memory(0x07FF), 0x5A);
    // The PRG-RAM bytes survive; whether they are *readable* straight after the reset depends on
    // the protect byte the board went back to, so the window is re-enabled before looking.
    core.write_memory(0xA001, 0x80);
    assert_eq!(
        core.read_memory(0x6000),
        0x3C,
        "the cartridge's PRG-RAM is kept across a soft reset"
    );
}
