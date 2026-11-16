//! **Where the PPU stands each time the CPU reaches an address**, on the Zaxnes engine with no
//! input — the instrument of lot ACCURACY-1, which compares that position with what `MesenCE` and
//! `NesHawk` report for the same ROM (an IRQ handler's first instruction, say).
//!
//! Nothing here changes the core: like `frame-trace` it steps instructions by hand through
//! `NesCore::cpu_and_bus_mut` with the PPU never deferred, so `scanline`/`dot` are exact at every
//! instruction, and it reads nothing but the program counter and the PPU position.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use zax_nes::cpu::CpuBus as _;
use zax_nes::NesCore;

use crate::HarnessError;

pub const USAGE: &str = "zax-harness exec-trace <rom.nes> --pc <hex> [--pc <hex> ...] [options]
    Powers the ROM on with no input and prints, for every time the CPU's next instruction is
    at one of the addresses, the frame, scanline, dot and CPU cycle. Frames are counted from
    power-on, one per vblank start.
      --frames <n>   frames to run (default 600)
      --follow <n>   after each hit, print the next n instructions disassembled, with their
                     scanline and dot (the position before each instruction)
      --out <file>   write there instead of standard output";

/// Instructions one frame may take before the run is called broken.
const MAX_PER_FRAME: usize = 200_000;

struct Options {
    rom: PathBuf,
    pcs: Vec<u16>,
    frames: u32,
    follow: usize,
    out: Option<PathBuf>,
}

fn hex(text: &str) -> Result<u16, HarnessError> {
    u16::from_str_radix(text.trim_start_matches("0x").trim_start_matches('$'), 16)
        .map_err(|_| HarnessError::Usage(format!("not a hex address: {text}\n{USAGE}")))
}

fn parse(args: &[String]) -> Result<Options, HarnessError> {
    let usage = || HarnessError::Usage(USAGE.to_owned());
    let mut rest = args.iter();
    let rom = PathBuf::from(rest.next().ok_or_else(usage)?);
    let mut options = Options {
        rom,
        pcs: Vec::new(),
        frames: 600,
        follow: 0,
        out: None,
    };
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(usage)?;
        match flag.as_str() {
            "--pc" => options.pcs.push(hex(value)?),
            "--follow" => options.follow = value.parse().map_err(|_| usage())?,
            "--frames" => options.frames = value.parse().map_err(|_| usage())?,
            "--out" => options.out = Some(PathBuf::from(value)),
            _ => return Err(usage()),
        }
    }
    if options.pcs.is_empty() {
        return Err(usage());
    }
    Ok(options)
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
    let info = core.load_rom(&image, &name);
    if !info.success {
        return Err(HarnessError::Rom("Zaxnes refused the image".to_owned()));
    }
    core.reset();
    let mut text = String::new();
    let (cpu, bus) = core.cpu_and_bus_mut();
    let mut following = 0usize;
    for frame in 1..=options.frames {
        bus.input_mut().begin_frame(0);
        bus.ppu_mut().clear_frame_boundary();
        for _ in 0..MAX_PER_FRAME {
            let pc = cpu.pc;
            if options.pcs.contains(&pc) {
                let _ = writeln!(
                    text,
                    "PC {pc:04X} f={frame} sl={} dot={} cyc={}",
                    bus.ppu().scanline(),
                    bus.ppu().dot(),
                    bus.cycle_count()
                );
                following = options.follow;
            }
            if following > 0 {
                following -= 1;
                let line = zax_nes::cpu::CpuTracer::capture(cpu, bus).format();
                let _ = writeln!(
                    text,
                    "  sl={:3} dot={:3} {line}",
                    bus.ppu().scanline(),
                    bus.ppu().dot()
                );
            }
            cpu.step(bus);
            if bus.ppu().vblank_just_started() {
                break;
            }
        }
    }
    match options.out {
        Some(path) => {
            fs::write(&path, text).map_err(|error| HarnessError::Rom(error.to_string()))?;
        }
        None => print!("{text}"),
    }
    Ok(())
}
