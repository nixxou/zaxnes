//! Lot FIDELITY step 29: the OAM corruption probe, the Zaxnes half of a differential study against
//! `TriCNES` (MIT, Chris Siebert) run on the same program.
//!
//! Ignored: it needs `ZAXNES_OAM_PROBE_ROM` (an NROM program that switches the rendering off and
//! on mid-line, sliding one dot per try) and writes `ZAXNES_OAM_PROBE_OUT`. OAM is filled with
//! byte = address; two lines after the rendering comes back, every byte that differs is logged and
//! the pattern restored. `ZAXNES_ALIGNMENT` selects the CPU/PPU alignment.

// A study tool: an unexpected error should stop it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fmt::Write as _;

use zax_nes::NesCore;

#[test]
#[ignore = "study tool: needs ZAXNES_OAM_PROBE_ROM and ZAXNES_OAM_PROBE_OUT"]
fn study_oam_corruption_probe() {
    let (Ok(rom), Ok(out)) = (
        std::env::var("ZAXNES_OAM_PROBE_ROM"),
        std::env::var("ZAXNES_OAM_PROBE_OUT"),
    ) else {
        eprintln!("skipped: ZAXNES_OAM_PROBE_ROM / ZAXNES_OAM_PROBE_OUT not set");
        return;
    };
    let alignment = std::env::var("ZAXNES_ALIGNMENT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3);
    let mut core = NesCore::new();
    core.set_cpu_ppu_alignment(alignment);
    // `ZAXNES_PHASE`: move the power-on dot (e.g. -11 to start on dot 0 as TriCNES does).
    core.set_phase_offset(std::env::var("ZAXNES_PHASE").ok().and_then(|value| value.parse().ok()).unwrap_or(0));
    assert!(core.load_rom(&std::fs::read(rom).unwrap(), "probe").success);
    let pattern: [u8; 256] = std::array::from_fn(|index| index.to_le_bytes()[0]);
    *core.bus_mut().ppu_mut().oam_mut() = pattern;

    let mut log = String::new();
    let mut rendering = false;
    let mut pending: Option<(i32, i32)> = None; // (line to check on, lines left)
    // `ZAXNES_OAM_PROBE_LINE6=1`: check OAM on line 6 after the rendering comes back (programs
    // that switch it back on in vblank, where the corruption lands at the pre-render line).
    let line6 = std::env::var("ZAXNES_OAM_PROBE_LINE6").is_ok_and(|value| value == "1");
    let steps: usize = std::env::var("ZAXNES_OAM_PROBE_STEPS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1_500_000);
    core.bus_mut().defer_ppu();
    for _ in 0..steps {
        let (cpu, bus) = core.cpu_and_bus_mut();
        cpu.step(bus);
        core.bus_mut().sync_ppu();
        let ppu = core.bus().ppu();
        let now = ppu.rendering_enabled();
        let (line, dot) = (ppu.scanline(), ppu.dot());
        if now != rendering {
            rendering = now;
            writeln!(log, "{} v={line} h={dot}", if now { "on" } else { "off" }).unwrap();
            if now {
                pending = Some((line, if line6 { -1 } else { 2 }));
            }
        }
        if let Some((seen, left)) = pending {
            if line != seen || left < 0 {
                if (left < 0 && line == 6 && seen != 6) || left == 0 {
                    let oam = core.bus().ppu().oam();
                    let mut diff = String::new();
                    for (index, &byte) in oam.iter().enumerate() {
                        let mask = if index & 3 == 2 { 0xE3 } else { 0xFF };
                        if byte & mask != pattern[index] & mask {
                            write!(diff, " {index}:{byte:02X}").unwrap();
                        }
                    }
                    writeln!(
                        log,
                        "oam v={line} diff{}",
                        if diff.is_empty() { " none" } else { &diff }
                    )
                    .unwrap();
                    *core.bus_mut().ppu_mut().oam_mut() = pattern;
                    pending = None;
                } else if left > 0 {
                    pending = Some((line, left - 1));
                }
            }
        }
    }
    core.bus_mut().end_deferred_ppu();
    std::fs::write(out, log).unwrap();
}
