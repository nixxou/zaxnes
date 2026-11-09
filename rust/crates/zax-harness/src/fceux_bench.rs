//! `fceux-bench`: what the tile capture costs, on either engine, measured by the same code.
//!
//! Lot FCEUX-1 measured FCEUX's core against Zaxnes's and reported FCEUX about 6× faster. That
//! figure was taken **without the tile capture**, which is the only configuration the 3D path can
//! actually use, so it answered the wrong question. Lot FCEUX-6 step 7 replaces it.
//!
//! The rules the brief sets, and why each one is in the code rather than in the note:
//!
//! * **one engine per process.** `--engine` takes exactly one, so a run holds one machine and one
//!   set of warm caches. Running both side by side in one process (as `fceux-tiles` does, where
//!   it is the point) would have each perturb the other's measurement.
//! * **the capture on**, because that is what the 3D renderer needs. `--capture off` exists only
//!   on FCEUX and only to separate the engine's own cost from the capture's; on Zaxnes the
//!   capture cannot be turned off at all — REQ-CORE-033 makes it unconditional so that a
//!   savestate cannot depend on whether a host asked for records — so a Zaxnes figure is always
//!   a with-capture figure.
//! * **two laps per frame, reported separately.** `run_frame` and then reading the records out.
//!   The split matters because the two engines hand over records differently: Zaxnes fills a
//!   buffer it owns and lends a slice, while the FFI sink of `zax-fceux` **allocates a `Vec` and
//!   copies every record inside the frame**, which a host could avoid. Reporting one total would
//!   charge the capture for a cost that belongs to the binding.
//!
//! No 3D, no `Host`, no Lua, no scene: this times the core and the capture and nothing else, so
//! that the number means one thing. `bench-core` is the one that walks the whole frame pipeline.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use zax_fceux::{Fceux, RamInit};
use zax_nes::NesCore;

use crate::HarnessError;

pub const USAGE: &str = "zax-harness fceux-bench <rom.nes> --engine fceux|zaxnes [options]
    Times one engine with the tile capture on, over the same loop for both, so the cost of the
    capture is comparable. Needs the `fceux-core` feature and a built fork.
      --engine fceux|zaxnes  which core to time (required; one per process on purpose)
      --frames <n>           frames to time (default 20000)
      --warmup <n>           frames run first and not timed (default 600)
      --capture on|off       FCEUX only; Zaxnes's capture is unconditional (default on)
      --ram zero|default|ff  FCEUX's power-on fill (default zero, as fceux-tiles)";

/// Which core to time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Fceux,
    Zaxnes,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub rom: PathBuf,
    pub engine: Engine,
    pub frames: u32,
    pub warmup: u32,
    pub capture: bool,
    pub ram: RamInit,
}

impl Options {
    /// # Errors
    ///
    /// [`HarnessError::Usage`] when an option is unknown, missing its value or unparsable, or
    /// when `--engine` is absent — it has no default on purpose, so that a figure in a note can
    /// always be traced back to the engine that produced it.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let usage = || HarnessError::Usage(USAGE.to_owned());
        let (rom, rest) = arguments.split_first().ok_or_else(usage)?;
        let mut options = Self {
            rom: PathBuf::from(rom),
            engine: Engine::Zaxnes,
            frames: 20_000,
            warmup: 600,
            capture: true,
            ram: RamInit::Zero,
        };
        let mut engine_given = false;

        let mut index = 0;
        while index < rest.len() {
            let flag = rest[index].as_str();
            let value = rest.get(index + 1);
            let take = || value.ok_or_else(usage).map(String::as_str);
            match flag {
                "--engine" => {
                    options.engine = match take()? {
                        "fceux" => Engine::Fceux,
                        "zaxnes" => Engine::Zaxnes,
                        other => {
                            return Err(HarnessError::Usage(format!(
                                "--engine: {other:?} is not fceux or zaxnes\n{USAGE}"
                            )))
                        }
                    };
                    engine_given = true;
                    index += 2;
                }
                "--frames" | "--warmup" => {
                    let count: u32 = take()?.parse().map_err(|_| {
                        HarnessError::Usage(format!("{flag}: not a frame count\n{USAGE}"))
                    })?;
                    if flag == "--frames" {
                        options.frames = count;
                    } else {
                        options.warmup = count;
                    }
                    index += 2;
                }
                "--capture" => {
                    options.capture = match take()? {
                        "on" => true,
                        "off" => false,
                        other => {
                            return Err(HarnessError::Usage(format!(
                                "--capture: {other:?} is not on or off\n{USAGE}"
                            )))
                        }
                    };
                    index += 2;
                }
                "--ram" => {
                    options.ram = match take()? {
                        "zero" => RamInit::Zero,
                        "default" => RamInit::Default,
                        "ff" => RamInit::Ff,
                        other => {
                            return Err(HarnessError::Usage(format!(
                                "--ram: {other:?} is not zero, default or ff\n{USAGE}"
                            )))
                        }
                    };
                    index += 2;
                }
                other => {
                    return Err(HarnessError::Usage(format!(
                        "unknown option {other:?}\n{USAGE}"
                    )))
                }
            }
        }

        if !engine_given {
            return Err(HarnessError::Usage(format!(
                "--engine is required: fceux or zaxnes\n{USAGE}"
            )));
        }
        if options.frames == 0 {
            return Err(HarnessError::Usage(
                "--frames: must be at least 1".to_owned(),
            ));
        }
        Ok(options)
    }
}

/// One stage's running total, as `bench_core::Tally` keeps it: the sum and the worst frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tally {
    pub total: f64,
    pub max: f64,
}

impl Tally {
    fn add(&mut self, seconds: f64) {
        self.total += seconds;
        self.max = self.max.max(seconds);
    }
}

/// What one run measured.
#[derive(Debug, Clone, Copy, Default)]
pub struct Measurement {
    /// `run_frame` alone — the emulation, plus the capture inside it.
    pub core: Tally,
    /// Reading the frame's records out of the engine afterwards.
    pub records: Tally,
    /// Both, per frame.
    pub whole: Tally,
    /// Records seen, summed over the timed frames.
    pub record_count: u64,
    /// Frames actually timed, which is fewer than asked for if the engine stopped early.
    pub timed: u32,
}

impl Measurement {
    /// Frames per second over the timed frames, counting both laps.
    #[must_use]
    pub fn frames_per_second(&self) -> f64 {
        f64::from(self.timed) / self.whole.total.max(f64::EPSILON)
    }
}

/// The subcommand: parse, run, print. One function so that `main`'s dispatch arm is one line,
/// which is what keeps that function inside its own line budget.
///
/// # Errors
///
/// Whatever [`Options::parse`] and [`run`] return.
pub fn command(arguments: &[String]) -> Result<(), HarnessError> {
    println!("{}", run(&Options::parse(arguments)?)?);
    Ok(())
}

/// Times the engine `options` names. Its own function so the two arms share the reporting.
///
/// # Errors
///
/// [`HarnessError::Rom`] when the image cannot be read or either engine refuses it, and
/// [`HarnessError::Usage`] when FCEUX reports a failure mid-run.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let image = std::fs::read(&options.rom)?;
    let measurement = match options.engine {
        Engine::Zaxnes => time_zaxnes(options, &image)?,
        Engine::Fceux => time_fceux(options, &image)?,
    };
    Ok(report(options, &measurement))
}

/// Zaxnes's core. The capture is unconditional (REQ-CORE-033), so this is always a with-capture
/// figure; `tile_records` lends the buffer the capture filled, which is why its lap is short.
fn time_zaxnes(options: &Options, image: &[u8]) -> Result<Measurement, HarnessError> {
    let stem = Path::new(&options.rom)
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let mut core = NesCore::new();
    if !core.load_rom(image, &stem).success {
        return Err(HarnessError::Rom("Zaxnes refused the image".to_owned()));
    }
    core.reset();

    let mut out = Measurement::default();
    for counter in 0..(options.warmup + options.frames) {
        let timed = counter >= options.warmup;
        let start = Instant::now();
        core.run_frame(0);
        let after_core = Instant::now();
        // The records as a host reads them. `black_box` so that a release build cannot decide
        // the loop has no effect and delete the read — the lap would then measure nothing.
        let records = core.tile_records();
        let count = records.len();
        std::hint::black_box(records);
        let after_records = Instant::now();

        if timed {
            out.core.add(after_core.duration_since(start).as_secs_f64());
            out.records
                .add(after_records.duration_since(after_core).as_secs_f64());
            out.whole
                .add(after_records.duration_since(start).as_secs_f64());
            out.record_count += count as u64;
            out.timed += 1;
        }
    }
    Ok(out)
}

/// FCEUX's core through the fork's C API. With `--capture on` the C side builds the records inside
/// `run_frame` and the FFI sink copies them into an owned `Vec` there; `take_tiles` then hands
/// that over. Both halves are timed where they fall.
fn time_fceux(options: &Options, image: &[u8]) -> Result<Measurement, HarnessError> {
    let base = Path::new(&options.rom).file_name().map_or_else(
        || options.rom.to_string_lossy().into_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let mut engine = Fceux::new().map_err(|error| HarnessError::Usage(error.to_string()))?;
    engine
        .set_ram_init(options.ram)
        .map_err(|error| HarnessError::Usage(error.to_string()))?;
    engine
        .load_rom(image, &base)
        .map_err(|error| HarnessError::Rom(error.to_string()))?;
    engine.set_tile_capture(options.capture);
    engine
        .reset()
        .map_err(|error| HarnessError::Usage(error.to_string()))?;

    let mut out = Measurement::default();
    for counter in 0..(options.warmup + options.frames) {
        let timed = counter >= options.warmup;
        let start = Instant::now();
        engine
            .set_input(0, 0)
            .and_then(|()| engine.run_frame())
            .map_err(|error| HarnessError::Usage(format!("frame {}: {error}", counter + 1)))?;
        let after_core = Instant::now();
        let count = if options.capture {
            let capture = engine.take_tiles().unwrap_or_default();
            let count = capture.records.len();
            std::hint::black_box(&capture);
            count
        } else {
            0
        };
        let after_records = Instant::now();

        if timed {
            out.core.add(after_core.duration_since(start).as_secs_f64());
            out.records
                .add(after_records.duration_since(after_core).as_secs_f64());
            out.whole
                .add(after_records.duration_since(start).as_secs_f64());
            out.record_count += count as u64;
            out.timed += 1;
        }
    }
    Ok(out)
}

/// A record count as an `f64` for the average below. The counts are at most a few million
/// (4096 records a frame at the capture's ceiling), so the mantissa is nowhere near tested.
#[allow(clippy::cast_precision_loss)]
fn precise(count: u64) -> f64 {
    count as f64
}

fn report(options: &Options, measurement: &Measurement) -> String {
    let frames = f64::from(measurement.timed.max(1));
    let engine = match options.engine {
        Engine::Fceux => "fceux",
        Engine::Zaxnes => "zaxnes",
    };
    let capture = match (options.engine, options.capture) {
        (Engine::Zaxnes, _) => "on (unconditional)",
        (Engine::Fceux, true) => "on",
        (Engine::Fceux, false) => "off",
    };
    let mut text = format!(
        "fceux-bench: {} engine {engine}, capture {capture} ({} frames timed after {} of warm-up)",
        options.rom.display(),
        measurement.timed,
        options.warmup
    );
    for (name, tally) in [
        ("run_frame", measurement.core),
        ("records read", measurement.records),
        ("total", measurement.whole),
    ] {
        let _ = write!(
            text,
            "\n  {name:<16} {:>8.4} ms/frame  max {:>8.3} ms",
            tally.total * 1000.0 / frames,
            tally.max * 1000.0
        );
    }
    let _ = write!(
        text,
        "\n  {:<16} {:>8.0}",
        "frames/s",
        measurement.frames_per_second()
    );
    let _ = write!(
        text,
        "\n  {:<16} {:>8.1}",
        "records/frame",
        precise(measurement.record_count) / frames
    );
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn the_engine_has_no_default_so_a_figure_can_always_be_traced_to_one() {
        let error = Options::parse(&args(&["rom.nes"])).unwrap_err();
        assert!(
            format!("{error}").contains("--engine is required"),
            "{error}"
        );
    }

    #[test]
    fn the_options_parse_and_the_defaults_are_the_briefs() {
        let options = Options::parse(&args(&["rom.nes", "--engine", "fceux"])).unwrap();
        assert_eq!(options.engine, Engine::Fceux);
        assert_eq!(options.frames, 20_000);
        assert_eq!(options.warmup, 600);
        assert!(options.capture);
        assert_eq!(options.ram, RamInit::Zero);

        let options = Options::parse(&args(&[
            "rom.nes",
            "--engine",
            "zaxnes",
            "--frames",
            "100",
            "--warmup",
            "7",
            "--capture",
            "off",
            "--ram",
            "ff",
        ]))
        .unwrap();
        assert_eq!(options.engine, Engine::Zaxnes);
        assert_eq!(options.frames, 100);
        assert_eq!(options.warmup, 7);
        assert!(!options.capture);
        assert_eq!(options.ram, RamInit::Ff);
    }

    #[test]
    fn a_bad_value_names_the_option_rather_than_printing_the_usage_alone() {
        for bad in [
            vec!["rom.nes", "--engine", "mesen"],
            vec!["rom.nes", "--engine", "fceux", "--capture", "maybe"],
            vec!["rom.nes", "--engine", "fceux", "--ram", "random"],
            vec!["rom.nes", "--engine", "fceux", "--frames", "lots"],
            vec!["rom.nes", "--engine", "fceux", "--jiggle"],
            vec!["rom.nes", "--engine", "fceux", "--frames", "0"],
        ] {
            let error = Options::parse(&args(&bad)).unwrap_err();
            assert!(matches!(error, HarnessError::Usage(_)), "{bad:?}");
        }
    }
}
