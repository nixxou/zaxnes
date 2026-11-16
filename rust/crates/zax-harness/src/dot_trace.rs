//! **The PPU dot by dot** (lot FIDPLUS-2): `v`, `t` and the PPU bus address after every dot of
//! chosen frames, on the Zaxnes engine with no input, for the differential against the
//! transistor netlist (`VisualNes` bench, `docs/review/FIDELITY-LEADS.md` A1, A3c).
//!
//! Needs `zax-nes`'s diagnostic feature `ppu-dot-trace` (this crate's `dot-trace` feature): with
//! it on, the PPU runs every traced dot through its per-dot step (no span, no idle skip). The
//! batched paths are proved equal to that step elsewhere; the trace is the per-dot model itself.
//!
//! Output: one line per dot, `frame scanline dot v t bus access` (hex; bus `----` when the PPU
//! put no address on its bus during the dot; access `-`, `R2002` or `W2006=21` for a CPU access
//! to a PPU register on that dot).

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use zax_nes::NesCore;

use crate::HarnessError;

/// The command's usage line.
pub const USAGE: &str =
    "  zax-harness dot-trace <rom.nes> --frames <first>-<last> [--alignment <0-3>] [--phase <dots>] [--flip-parity] [--bus-conflicts] [--fidelity-plus] [--region ntsc|pal] [--dot-phase <0-4>] [--power-on-vblank] --out <file>
      (per dot: frame, scanline, dot, v, t, PPU bus address; needs the `dot-trace` feature;
      --region pal and --dot-phase for the 2C07 differential, lot FIDPLUS-C C3c;
      --power-on-vblank: the PPU's power-on at vblank, as before `emu.power_on_top`, which the
      2C02 netlist traces of lot FIDPLUS-2 were paired against)";

#[allow(clippy::struct_excessive_bools)] // One flag per command-line switch.
struct Options {
    rom: PathBuf,
    first: u32,
    last: u32,
    alignment: Option<u8>,
    phase: i32,
    flip_parity: bool,
    bus_conflicts: bool,
    fidelity_plus: bool,
    pal: bool,
    dot_phase: i32,
    power_on_vblank: bool,
    out: PathBuf,
}

fn parse(args: &[String]) -> Result<Options, HarnessError> {
    let usage = || HarnessError::Usage(USAGE.to_owned());
    let mut rom = None;
    let mut frames = None;
    let mut alignment = None;
    let mut phase = 0;
    let mut flip_parity = false;
    let mut bus_conflicts = false;
    let mut fidelity_plus = false;
    let mut pal = false;
    let mut dot_phase = 0;
    let mut power_on_vblank = false;
    let mut out = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--flip-parity" {
            flip_parity = true;
            index += 1;
        } else if arg == "--bus-conflicts" {
            bus_conflicts = true;
            index += 1;
        } else if arg == "--fidelity-plus" {
            fidelity_plus = true;
            index += 1;
        } else if arg == "--power-on-vblank" {
            power_on_vblank = true;
            index += 1;
        } else if arg.starts_with("--") {
            let value = args.get(index + 1).ok_or_else(usage)?;
            match arg.as_str() {
                "--frames" => {
                    let (a, b) = value.split_once('-').unwrap_or((value, value));
                    frames = Some((
                        a.parse::<u32>().map_err(|_| usage())?,
                        b.parse::<u32>().map_err(|_| usage())?,
                    ));
                }
                "--alignment" => alignment = Some(value.parse::<u8>().map_err(|_| usage())?),
                "--phase" => phase = value.parse::<i32>().map_err(|_| usage())?,
                "--dot-phase" => dot_phase = value.parse::<i32>().map_err(|_| usage())?,
                "--region" => {
                    pal = match value.as_str() {
                        "pal" => true,
                        "ntsc" => false,
                        _ => return Err(usage()),
                    };
                }
                "--out" => out = Some(PathBuf::from(value)),
                _ => return Err(usage()),
            }
            index += 2;
        } else {
            rom = Some(PathBuf::from(arg));
            index += 1;
        }
    }
    let (first, last) = frames.ok_or_else(usage)?;
    if first == 0 || last < first || alignment.is_some_and(|a| a > 3) {
        return Err(usage());
    }
    Ok(Options {
        rom: rom.ok_or_else(usage)?,
        first,
        last,
        alignment,
        phase,
        flip_parity,
        bus_conflicts,
        fidelity_plus,
        pal,
        dot_phase,
        power_on_vblank,
        out: out.ok_or_else(usage)?,
    })
}

/// The command.
///
/// # Errors
/// A bad argument, an unreadable or refused ROM, or an unwritable output file.
pub fn command(args: &[String]) -> Result<(), HarnessError> {
    let options = parse(args)?;
    let image = fs::read(&options.rom).map_err(|error| HarnessError::Rom(error.to_string()))?;
    let name = options
        .rom
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let mut core = Box::new(NesCore::new());
    core.set_builtin_patches(false);
    core.set_advanced_patches(false);
    if let Some(alignment) = options.alignment {
        core.set_cpu_ppu_alignment(alignment);
    }
    // The CPU/PPU phase at power-on, in dots (study `study/access-phase`), to match the netlist's.
    core.set_phase_offset(options.phase);
    if options.power_on_vblank {
        core.set_power_on_top(false);
    }
    let info = core.load_rom(&image, &name);
    if !info.success {
        return Err(HarnessError::Rom("Zaxnes refused the image".to_owned()));
    }
    if options.pal {
        core.set_region(zax_nes::NesRegion::Pal);
    }
    core.reset();
    core.bus_mut()
        .ppu_mut()
        .set_dot_accumulator_phase(options.dot_phase);
    // `emu.ppu_bus_conflicts` (FIDELITY-LEADS C1), off in the application by default; after
    // the load, which builds the PPU afresh.
    core.set_bus_conflicts(options.bus_conflicts);
    core.set_fidelity_plus(options.fidelity_plus);
    if options.flip_parity {
        core.bus_mut().ppu_mut().flip_frame_parity();
    }
    let mut text = String::new();
    let show_phase = std::env::var_os("ZAX_TRACE_PHASE").is_some();
    // ZAX_PAL_2001 = ten comma-separated delays (switch on, accumulators 0-4, then off; 0 keeps
    // the model): a measurement override of the PAL `$2001` switch delay.
    if let Ok(list) = std::env::var("ZAX_PAL_2001") {
        let mut delays = [0_i32; 10];
        for (slot, item) in delays.iter_mut().zip(list.split(',')) {
            *slot = item.trim().parse().unwrap_or(0);
        }
        zax_nes::ppu::Ppu2C02::set_pal_switch_override(delays);
    }
    // ZAX_PAL_LATCH = five comma-separated $2007 latch delays (accumulators 0-4; 0 keeps the
    // model): a measurement override.
    if let Ok(list) = std::env::var("ZAX_PAL_LATCH") {
        let mut dots = [0_i32; 5];
        for (slot, item) in dots.iter_mut().zip(list.split(',')) {
            *slot = item.trim().parse().unwrap_or(0);
        }
        zax_nes::ppu::Ppu2C02::set_pal_latch_override(dots);
    }
    for frame in 1..=options.last {
        let traced = frame >= options.first;
        if traced {
            core.bus_mut().ppu_mut().start_dot_trace();
        }
        core.run_frame(0);
        if traced {
            for record in core.bus_mut().ppu_mut().take_dot_trace() {
                let bus = if record.bus == 0xFFFF {
                    "----".to_owned()
                } else {
                    format!("{:04X}", record.bus)
                };
                let mut access = match record.access {
                    0 => "-".to_owned(),
                    a if a & 0x8000 != 0 => {
                        format!("W{:04X}={:02X}", a & 0x7FFF, (a >> 16) & 0xFF)
                    }
                    a => format!("R{:04X}", a & 0x7FFF),
                };
                // ZAX_TRACE_PHASE: the PPU's dot accumulator at each access, after `@`.
                if record.access != 0 && show_phase {
                    let _ = write!(access, "@{}", (record.access >> 24).saturating_sub(1));
                }
                let _ = writeln!(
                    text,
                    "{frame} {} {} {:04X} {:04X} {bus} {access}",
                    record.scanline, record.dot, record.v, record.t
                );
            }
        }
    }
    fs::write(&options.out, text).map_err(HarnessError::Io)?;
    Ok(())
}
