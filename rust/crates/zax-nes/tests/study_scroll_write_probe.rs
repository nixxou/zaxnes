//! Lot FIDELITY step 30: the `$2005` write probe, the Zaxnes half of a differential study against
//! `TriCNES` (MIT, Chris Siebert) run on the same program.
//!
//! Ignored: it needs `ZAXNES_SCROLL_PROBE_ROM` (an NROM program that writes `$2005` once per frame
//! with rendering on, sliding one dot per frame) and writes `ZAXNES_SCROLL_PROBE_OUT`. Each change
//! of `t` on a rendered line is logged ("w"), then `v` at the first step that ends on line
//! `ZAXNES_VLINE` between dots `ZAXNES_VDOT_MIN` and `ZAXNES_VDOT_MAX` ("vline").
//! `ZAXNES_ALIGNMENT` selects the CPU/PPU alignment.

// A study tool: an unexpected error should stop it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fmt::Write as _;

use zax_nes::NesCore;

fn env_i32(name: &str, default: i32) -> i32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[test]
#[ignore = "study tool: needs ZAXNES_SCROLL_PROBE_ROM and ZAXNES_SCROLL_PROBE_OUT"]
fn study_scroll_write_probe() {
    let (Ok(rom), Ok(out)) = (
        std::env::var("ZAXNES_SCROLL_PROBE_ROM"),
        std::env::var("ZAXNES_SCROLL_PROBE_OUT"),
    ) else {
        eprintln!("skipped: ZAXNES_SCROLL_PROBE_ROM / ZAXNES_SCROLL_PROBE_OUT not set");
        return;
    };
    let mut core = NesCore::new();
    core.set_cpu_ppu_alignment(u8::try_from(env_i32("ZAXNES_ALIGNMENT", 3)).unwrap());
    // `ZAXNES_PHASE`: move the power-on dot (e.g. -11 to start on dot 0 as TriCNES does).
    core.set_phase_offset(env_i32("ZAXNES_PHASE", 0));
    assert!(core.load_rom(&std::fs::read(rom).unwrap(), "probe").success);
    let (vline, dot_min, dot_max) = (
        env_i32("ZAXNES_VLINE", 100),
        env_i32("ZAXNES_VDOT_MIN", 270),
        env_i32("ZAXNES_VDOT_MAX", 320),
    );
    let steps = usize::try_from(env_i32("ZAXNES_SCROLL_PROBE_STEPS", 6_000_000)).unwrap();

    let mut log = String::new();
    let mut t = core.bus().ppu().vram_t();
    let mut done = false;
    core.bus_mut().defer_ppu();
    for _ in 0..steps {
        let (cpu, bus) = core.cpu_and_bus_mut();
        cpu.step(bus);
        core.bus_mut().sync_ppu();
        let ppu = core.bus().ppu();
        let (line, dot) = (ppu.scanline(), ppu.dot());
        if ppu.vram_t() != t {
            t = ppu.vram_t();
            if line < 240 || line == 261 {
                writeln!(log, "w {t:04X} v={line} h={dot}").unwrap();
            }
        }
        if line == vline && (dot_min..dot_max).contains(&dot) {
            if !done {
                writeln!(log, "vline {:04X} v={line} h={dot}", ppu.vram_v()).unwrap();
                done = true;
            }
        } else {
            done = false;
        }
    }
    core.bus_mut().end_deferred_ppu();
    std::fs::write(out, log).unwrap();
}
