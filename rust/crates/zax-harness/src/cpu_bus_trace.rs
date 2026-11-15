//! **The CPU's bus cycle by cycle** (diagnostic of the CPU against the 2A03 netlist, after lot
//! FIDPLUS-A): every read and write the CPU makes, one line per CPU cycle, for the differential
//! against `VisualNes`'s transistor-level 2A03 (`ext-tools/visualnes-bench/benchcpu`).
//!
//! The program is loaded as the bench loads it: 32 KiB of PRG at `$8000`, RESET at `$8000`, NMI
//! and IRQ at `$9000`. Cycles the CPU did not make itself (a DMA stealing them inside the bus) show
//! as one `DMA <n>` line, `n` the cycles stolen.
//!
//! Output: `cycle address R|W data` per line, the cycle counted from the first opcode fetch at
//! `$8000`.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use zax_nes::cpu::CpuBus;
use zax_nes::NesCore;

use crate::HarnessError;

/// The command's usage line.
pub const USAGE: &str = "  zax-harness cpu-bus-trace <program.bin> --cycles <n> [--alignment <0-3>] [--phase <dots>] [--dmc <timer>,<bits>] [--fidelity-plus] --out <file>
      (per CPU cycle: address, R/W, data; the program at $8000 as the 2A03 netlist bench loads it)";

struct Options {
    program: PathBuf,
    cycles: usize,
    alignment: Option<u8>,
    phase: i32,
    dmc: Option<(i32, i32)>,
    fidelity_plus: bool,
    out: PathBuf,
}

fn parse(args: &[String]) -> Result<Options, HarnessError> {
    let usage = || HarnessError::Usage(USAGE.to_owned());
    let mut rest = args.iter();
    let program = PathBuf::from(rest.next().ok_or_else(usage)?);
    let mut cycles = None;
    let mut alignment = None;
    let mut phase = 0;
    let mut dmc = None;
    let mut fidelity_plus = false;
    let mut out = None;
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--cycles" => {
                cycles = Some(
                    rest.next()
                        .ok_or_else(usage)?
                        .parse()
                        .map_err(|_| usage())?,
                );
            }
            "--alignment" => {
                alignment = Some(
                    rest.next()
                        .ok_or_else(usage)?
                        .parse()
                        .map_err(|_| usage())?,
                );
            }
            "--phase" => {
                phase = rest
                    .next()
                    .ok_or_else(usage)?
                    .parse()
                    .map_err(|_| usage())?;
            }
            "--dmc" => {
                let value = rest.next().ok_or_else(usage)?;
                let (timer, bits) = value.split_once(',').ok_or_else(usage)?;
                dmc = Some((
                    timer.parse().map_err(|_| usage())?,
                    bits.parse().map_err(|_| usage())?,
                ));
            }
            "--fidelity-plus" => fidelity_plus = true,
            "--out" => out = Some(PathBuf::from(rest.next().ok_or_else(usage)?)),
            _ => return Err(usage()),
        }
    }
    Ok(Options {
        program,
        cycles: cycles.ok_or_else(usage)?,
        alignment,
        phase,
        dmc,
        fidelity_plus,
        out: out.ok_or_else(usage)?,
    })
}

/// An NROM image holding `program` as the bench's PRG RAM: 32 KiB at `$8000`, vectors set.
fn image_of(program: &[u8]) -> Vec<u8> {
    let mut prg = vec![0u8; 0x8000];
    let length = program.len().min(0x7FFA);
    prg[..length].copy_from_slice(&program[..length]);
    prg[0x7FFA..].copy_from_slice(&[0x00, 0x90, 0x00, 0x80, 0x00, 0x90]);
    let mut image = vec![b'N', b'E', b'S', 0x1A, 2, 1, 0, 0];
    image.resize(16, 0);
    image.extend_from_slice(&prg);
    image.extend_from_slice(&[0u8; 0x2000]);
    image
}

/// The bus seen through a recorder: every access is a line.
struct Recorder<'a, B: CpuBus> {
    bus: &'a mut B,
    lines: Vec<(i64, u16, bool, u8)>,
}

impl<B: CpuBus> CpuBus for Recorder<'_, B> {
    fn read(&mut self, address: u16) -> u8 {
        let cycle = self.bus.cycle_count();
        let value = self.bus.read(address);
        self.lines.push((cycle, address, true, value));
        value
    }

    fn read_code(&mut self, address: u16) -> u8 {
        let cycle = self.bus.cycle_count();
        let value = self.bus.read_code(address);
        self.lines.push((cycle, address, true, value));
        value
    }

    fn write(&mut self, address: u16, value: u8) {
        let cycle = self.bus.cycle_count();
        self.bus.write(address, value);
        self.lines.push((cycle, address, false, value));
    }

    fn peek(&self, address: u16) -> u8 {
        self.bus.peek(address)
    }

    fn nmi_asserted(&self) -> bool {
        self.bus.nmi_asserted()
    }

    fn irq_asserted(&self) -> bool {
        self.bus.irq_asserted()
    }

    fn cycle_count(&self) -> i64 {
        self.bus.cycle_count()
    }

    fn dma_gave_back_at(&self) -> i64 {
        self.bus.dma_gave_back_at()
    }
}

/// The command.
///
/// # Errors
/// A bad argument, an unreadable program, a refused image or an unwritable output file.
pub fn command(args: &[String]) -> Result<(), HarnessError> {
    let options = parse(args)?;
    let program =
        fs::read(&options.program).map_err(|error| HarnessError::Rom(error.to_string()))?;
    let mut core = Box::new(NesCore::new());
    core.set_builtin_patches(false);
    core.set_advanced_patches(false);
    core.set_fidelity_plus(options.fidelity_plus);
    // The netlist bench powers its RAM on at zero.
    core.set_power_on_ram(zax_nes::PowerOnRam::Zero);
    // The PPU's start against the CPU's, as `dot-trace --phase` (the netlist's power-on).
    core.set_phase_offset(options.phase);
    if let Some(alignment) = options.alignment {
        core.set_cpu_ppu_alignment(alignment);
    }
    if !core
        .load_rom(&image_of(&program), "cpu-bus-trace.nes")
        .success
    {
        return Err(HarnessError::Rom("Zaxnes refused the image".to_owned()));
    }
    core.reset();
    let (cpu, bus) = core.cpu_and_bus_mut();
    if let Some((timer, bits)) = options.dmc {
        bus.apu_mut().diagnostic_set_dmc_phase(timer, bits);
    }
    let mut recorder = Recorder {
        bus,
        lines: Vec::new(),
    };
    // Up to the first opcode fetch at $8000 (the reset sequence before it is the bench's too, but
    // its start is not aligned the same way): then `cycles` cycles.
    let mut start = None;
    let mut guard = 0;
    while guard < options.cycles * 8 + 100_000 {
        guard += 1;
        cpu.step(&mut recorder);
        if start.is_none() {
            start = recorder
                .lines
                .iter()
                .position(|&(_, address, read, _)| address == 0x8000 && read);
        }
        if let Some(first) = start {
            if recorder.lines.len() >= first + options.cycles {
                break;
            }
        }
    }
    let first = start.ok_or_else(|| HarnessError::Rom("the program never ran".to_owned()))?;
    let origin = recorder.lines[first].0;
    let mut text = String::new();
    let mut expected = origin;
    for &(cycle, address, read, value) in recorder.lines[first..].iter().take(options.cycles) {
        if cycle > expected {
            let _ = writeln!(text, "{} DMA {}", expected - origin, cycle - expected);
        }
        let _ = writeln!(
            text,
            "{} {address:04X} {} {value:02X}",
            cycle - origin,
            if read { 'R' } else { 'W' }
        );
        expected = cycle + 1;
    }
    fs::write(&options.out, text).map_err(|error| HarnessError::Rom(error.to_string()))?;
    Ok(())
}
