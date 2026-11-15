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
    "  zax-harness dot-trace <rom.nes> --frames <first>-<last> [--alignment <0-3>] --out <file>
      (per dot: frame, scanline, dot, v, t, PPU bus address; needs the `dot-trace` feature)";

struct Options {
    rom: PathBuf,
    first: u32,
    last: u32,
    alignment: Option<u8>,
    out: PathBuf,
}

fn parse(args: &[String]) -> Result<Options, HarnessError> {
    let usage = || HarnessError::Usage(USAGE.to_owned());
    let mut rom = None;
    let mut frames = None;
    let mut alignment = None;
    let mut out = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg.starts_with("--") {
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
    let info = core.load_rom(&image, &name);
    if !info.success {
        return Err(HarnessError::Rom("Zaxnes refused the image".to_owned()));
    }
    core.reset();
    let mut text = String::new();
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
                let access = match record.access {
                    0 => "-".to_owned(),
                    a if a & 0x8000 != 0 => format!("W{:04X}={:02X}", a & 0x7FFF, a >> 16),
                    a => format!("R{:04X}", a & 0x7FFF),
                };
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
