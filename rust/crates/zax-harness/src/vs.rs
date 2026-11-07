//! `zax-harness vs <rom.nes>`: loads a VS. System image, drives its cabinet and prints what the
//! core reports (lot MF-VS step 6).
//!
//! **What it is for.** The owner's brief puts the app's UI binding of the coin, service and DIP
//! inputs out of this lot and asks instead for "a harness command or option to set DIP switches
//! and insert a coin", so that a VS image can be loaded, configured and run before any UI exists.
//! This is that command, and it is also how R0 runs the owner's own dumps: it prints the detected
//! PPU model and hardware type, the load note with every gap named, the two port reads bit by bit,
//! and a digest of the frames it ran so that two runs can be compared.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use zax_nes::vs::VsSystem;
use zax_nes::NesCore;

use crate::HarnessError;

/// Usage of the `vs` subcommand.
pub const USAGE: &str = "zax-harness vs <rom.nes> [options]
  --frames <n>        frames to run (default 0, so the command only reports the load)
  --dip <byte>        the eight DIP switches at once, decimal or 0x-prefixed
  --switch <n>[=on|off]  one DIP switch, 1 to 8 (repeatable; bare means on)
  --coin <slot>       drop a coin in slot 1 or 2 before the run (repeatable)
  --service           hold the service button for the run
  --press <buttons>   player 1 buttons held every frame: a,b,start,select,up,down,left,right
  --quiet             print the one-line summary only";

/// What `vs` runs.
#[derive(Debug, Clone)]
pub struct Options {
    rom: PathBuf,
    frames: u32,
    dip: u8,
    coins: Vec<usize>,
    service: bool,
    press: u8,
    quiet: bool,
}

impl Options {
    /// Parses the arguments after `vs`.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] when an option is unknown, lacks its value, or names a switch, a
    /// slot or a button that does not exist.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let usage = || HarnessError::Usage(USAGE.to_owned());
        let (rom, mut rest) = arguments.split_first().ok_or_else(usage)?;
        let mut options = Self {
            rom: PathBuf::from(rom),
            frames: 0,
            dip: 0,
            coins: Vec::new(),
            service: false,
            press: 0,
            quiet: false,
        };

        while let Some((name, tail)) = rest.split_first() {
            if name == "--service" || name == "--quiet" {
                options.service |= name == "--service";
                options.quiet |= name == "--quiet";
                rest = tail;
                continue;
            }

            let (value, tail) = tail.split_first().ok_or_else(usage)?;
            match name.as_str() {
                "--frames" => {
                    options.frames = value.parse().map_err(|_| {
                        HarnessError::Usage(format!("--frames: {value:?} is not a count"))
                    })?;
                }
                "--dip" => options.dip = parse_byte(value)?,
                "--switch" => {
                    let (switch, on) = parse_switch(value)?;
                    let bit = 1u8 << (switch - 1);
                    if on {
                        options.dip |= bit;
                    } else {
                        options.dip &= !bit;
                    }
                }
                "--coin" => {
                    let slot = value
                        .parse::<usize>()
                        .ok()
                        .filter(|slot| *slot >= 1 && *slot <= VsSystem::COIN_SLOTS);
                    options.coins.push(slot.ok_or_else(|| {
                        HarnessError::Usage(format!("--coin: {value:?} is not slot 1 or 2"))
                    })?);
                }
                "--press" => options.press = parse_buttons(value)?,
                _ => return Err(usage()),
            }

            rest = tail;
        }

        Ok(options)
    }
}

/// FNV-1a's 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// One FNV-1a step.
const fn fnv(digest: u64, byte: u8) -> u64 {
    (digest ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3)
}

/// A byte written in decimal, or in hexadecimal behind `0x`.
fn parse_byte(value: &str) -> Result<u8, HarnessError> {
    let parsed = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(
            || value.parse::<u8>().ok(),
            |hex| u8::from_str_radix(hex, 16).ok(),
        );
    parsed.ok_or_else(|| HarnessError::Usage(format!("{value:?} is not a byte")))
}

/// `n`, `n=on` or `n=off`, with `n` a DIP switch from 1 to 8.
fn parse_switch(value: &str) -> Result<(usize, bool), HarnessError> {
    let bad = || HarnessError::Usage(format!("--switch: {value:?} is not 1 to 8 [=on|off]"));
    let (number, on) = match value.split_once('=') {
        None => (value, true),
        Some((number, "on" | "1" | "true")) => (number, true),
        Some((number, "off" | "0" | "false")) => (number, false),
        Some(_) => return Err(bad()),
    };

    let switch = number
        .parse::<usize>()
        .ok()
        .filter(|switch| *switch >= 1 && *switch <= VsSystem::DIP_SWITCHES)
        .ok_or_else(bad)?;
    Ok((switch, on))
}

/// A comma-separated list of the eight button names, as one port byte: A, B, Select, Start, Up,
/// Down, Left, Right from bit 0 to bit 7 (REQ-CORE-022).
fn parse_buttons(value: &str) -> Result<u8, HarnessError> {
    let mut byte = 0u8;
    for name in value.split(',').filter(|name| !name.is_empty()) {
        byte |= match name {
            "a" => 0x01,
            "b" => 0x02,
            "select" => 0x04,
            "start" => 0x08,
            "up" => 0x10,
            "down" => 0x20,
            "left" => 0x40,
            "right" => 0x80,
            _ => {
                return Err(HarnessError::Usage(format!(
                    "--press: {name:?} is not a button (a,b,select,start,up,down,left,right)"
                )))
            }
        };
    }

    Ok(byte)
}

/// Loads the image, drives the cabinet, runs `frames` frames and reports.
///
/// # Errors
///
/// [`HarnessError::Io`] when the file cannot be read, [`HarnessError::Rom`] when the core refuses
/// it or when it is not a VS. System image at all.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let image = std::fs::read(&options.rom)?;
    let name = file_name(&options.rom);
    let mut core = NesCore::new();
    let load = core.load_rom(&image, &name);
    if !load.success {
        return Err(HarnessError::Rom(
            load.error
                .unwrap_or_else(|| "the ROM was refused".to_owned()),
        ));
    }

    let mapper = core
        .cartridge()
        .map_or(-1, zax_nes::cartridges::Cartridge::mapper_number);
    let Some((model, hardware, wiring, source)) = core
        .vs_system()
        .map(|vs| (vs.ppu(), vs.hardware(), vs.wiring(), vs.source()))
    else {
        return Err(HarnessError::Rom(format!(
            "{name} is not a VS. System image: no VS bit in byte 7, no NES 2.0 console type 1, \
             and the mapper is {mapper} and not 99 or 151"
        )));
    };

    let mut out = String::new();
    let note = load.note.unwrap_or_default();

    if !options.quiet {
        writeln!(out, "{name}")?;
        writeln!(out, "  {note}")?;
        writeln!(out, "  ppu          {}", model.name())?;
        writeln!(out, "  hardware     {}", hardware.name())?;
        writeln!(out, "  wiring       {wiring:?}")?;
        writeln!(out, "  source       {source:?}")?;
        writeln!(out, "  mapper       {mapper}")?;
        writeln!(
            out,
            "  palette      {}",
            if model.palette_lut().is_some() {
                "remapped into the ordered palette"
            } else {
                "the ordered palette itself"
            }
        )?;
    }

    if let Some(vs) = core.vs_system_mut() {
        vs.set_dip_switches(options.dip);
        vs.set_service_button(options.service);
        for slot in &options.coins {
            vs.insert_coin(*slot);
        }
    }

    if let Some(vs) = core.vs_system().filter(|_| !options.quiet) {
        writeln!(
            out,
            "  dip          {:#04X} {}",
            vs.dip_switches(),
            switch_list(vs)
        )?;
        writeln!(out, "  service      {}", vs.service_button())?;
        writeln!(out, "  coins        1:{} 2:{}", vs.coin(1), vs.coin(2))?;
        writeln!(out, "  $4016        {:#04X}", vs.port_bits(0))?;
        writeln!(out, "  $4017        {:#04X}", vs.port_bits(1))?;
    }

    // Player 1's buttons in the low byte, player 2's in the high one; the cabinet's wiring decides
    // which port sees which stick, which is why the word the ports end up with is printed too.
    let word = u16::from(options.press);
    let mut digest = FNV_OFFSET;
    for _ in 0..options.frames {
        core.run_frame(word);
        // FNV-1a over the frame's 32 palette bytes and its picture, so that two runs of the same
        // image can be compared without keeping the pictures. A rotate-and-xor digest will not do
        // here: on a VS image whose palette RAM is still all zeroes every byte is the same
        // remapped colour, and that kind of digest cancels itself on constant input.
        for byte in core.ppu().snapshot_palette() {
            digest = fnv(digest, byte);
        }

        for byte in core.ppu().pixel_frame_with_mask() {
            digest = fnv(digest, *byte);
        }
    }

    if options.frames > 0 {
        if !options.quiet {
            writeln!(
                out,
                "  word         {:#06X}",
                core.bus_mut().input().input_word()
            )?;
        }

        writeln!(
            out,
            "vs: {name}, {}, {}, {} frames, palette digest {digest:#018X}",
            model.name(),
            hardware.name(),
            options.frames
        )?;
    } else {
        writeln!(
            out,
            "vs: {name}, {}, {}, loaded",
            model.name(),
            hardware.name()
        )?;
    }

    Ok(out)
}

/// `1..8` with the switches that are on, for the report.
fn switch_list(vs: &VsSystem) -> String {
    let on: Vec<String> = (1..=VsSystem::DIP_SWITCHES)
        .filter(|switch| vs.dip_switch(*switch))
        .map(|switch| switch.to_string())
        .collect();
    if on.is_empty() {
        "(all off)".to_owned()
    } else {
        format!("(on: {})", on.join(","))
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}
