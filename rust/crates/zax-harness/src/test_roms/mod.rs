//! `test-roms <roms dir> <out.json> [--filter <substring>]`: runs every `.nes` under a
//! directory of **public** test ROMs headless — no window, no audio device — and records one
//! result per ROM.
//!
//! The ROMs themselves are not in the repository: the directory is given on the command line or
//! in `ZAXNES_TEST_ROMS`, the way `golden-core` reads the owner's ROM folder. The collection
//! this lot was written against is `assets-free/test-roms/`:
//! `nes-test-roms/` (the community collection), `holy-mapperel/testroms/` (board detection) and
//! `AccuracyCoin/AccuracyCoin.nes`.
//!
//! Each ROM is read with the protocol **its own documentation** describes, never by guessing:
//!
//! * [`blargg`] — the status byte at `$6000` with the `DE B0 61` signature at `$6001-$6003`
//!   (`nes-test-roms/*/readme.txt`, section "Output at $6000");
//! * [`screen`] — ROMs that report only on screen: Holy Mapperel, whose documentation says so
//!   outright ("All this information is displayed on the screen"), `AccuracyCoin`'s result table,
//!   and the blargg "done" ROMs whose readme names the text that means "pass".
//!
//! Anything none of those can read is `manual`, and the PNG of the final frame is the evidence.
//!
//! **This command changes nothing in the emulation.** Every reader goes through accessors the
//! core already had — `read_memory`, `ppu().vram()`, `map_nametable`, `pixel_frame_with_mask`,
//! `run_frame`, `soft_reset` — so nothing was added to a per-cycle or per-access path.

pub mod blargg;
pub mod coin;
pub mod machine;
pub mod mapperel;
pub mod report;
pub mod screen;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{json, Map, Value};
use zax_core_api::NesRegion;
use zax_nes::rom::InesHeader;
use zax_nes::NesCore;

use crate::HarnessError;

/// The options of `test-roms`.
pub const USAGE: &str = "zax-harness test-roms <roms dir> <out.json> [options]
  --filter <substring>  only the ROMs whose relative path contains it
  --shots <dir>         where the final-frame PNGs go (default: beside <out.json>)
  --no-shots            write no PNG at all
  --markdown <file>     also write the report: a table per suite and a line per failure
  --reuse               do not run anything: read <out.json> back and rewrite the report
  --merge <file.json>   start from that campaign's rows; the ROMs run now replace theirs
                        (with --filter, re-measures one suite without re-running the rest)
  --frames <n>          cap the frame budget of every ROM at n (default: per suite)
  --quiet               do not print one line per ROM
  --engine <name>       which core: zaxnes|neshawk (default zaxnes). `neshawk` is
                        `zax-neshawk`, the Rust port of BizHawk's own core, so that a verdict
                        can be held against NesHawk's by the same readers on the same ROMs
  --neshawk-gamedb <d>  BizHawk's gamedb directory, which `--engine neshawk` needs to pick the
                        board BizHawk picks; defaults to $ZAXNES_BIZHAWK_GAMEDB
  --coin-run-all <f>    AccuracyCoin only: drive it as `scripts/coin_bizhawk.lua` drives BizHawk
                        (press Start at frame 120, wait for every result byte, give up at frame
                        100000) and write the Lua's own lines to <f>, so that the run can be
                        diffed against `bizhawk-runs/coin-neshawk.txt`. A DIFFERENT instrument
                        from the menu walk, which is why it is a separate flag";

/// The environment variable the gated `cargo test` subset reads.
pub const ROMS_DIR_VARIABLE: &str = "ZAXNES_TEST_ROMS";

/// What `test-roms` runs.
#[derive(Debug, Clone)]
pub struct Options {
    roms: PathBuf,
    out: PathBuf,
    filter: Option<String>,
    shots: Option<PathBuf>,
    no_shots: bool,
    markdown: Option<PathBuf>,
    reuse: bool,
    merge: Option<PathBuf>,
    frame_cap: Option<u32>,
    quiet: bool,
    /// Which core runs the collection.
    engine: TestEngine,
    /// BizHawk's `gamedb` directory, for `--engine neshawk`.
    neshawk_gamedb: Option<PathBuf>,
    /// Where `--coin-run-all` writes the Lua-shaped lines.
    coin_run_all: Option<PathBuf>,
}

impl Options {
    /// Parses the arguments after `test-roms`.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] when the directory or the output file is missing, an option is
    /// unknown or lacks its value, or `--frames` is not a count.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let usage = || HarnessError::Usage(USAGE.to_owned());
        let [roms, out, tail @ ..] = arguments else {
            return Err(usage());
        };
        let mut rest = tail;
        let mut options = Self {
            roms: PathBuf::from(roms),
            out: PathBuf::from(out),
            filter: None,
            shots: None,
            no_shots: false,
            markdown: None,
            reuse: false,
            merge: None,
            frame_cap: None,
            quiet: false,
            engine: TestEngine::Zaxnes,
            neshawk_gamedb: None,
            coin_run_all: None,
        };
        while let Some((name, tail)) = rest.split_first() {
            if matches!(name.as_str(), "--no-shots" | "--quiet" | "--reuse") {
                options.no_shots |= name == "--no-shots";
                options.quiet |= name == "--quiet";
                options.reuse |= name == "--reuse";
                rest = tail;
                continue;
            }
            let (value, tail) = tail.split_first().ok_or_else(usage)?;
            match name.as_str() {
                "--filter" => options.filter = Some(value.clone()),
                "--shots" => options.shots = Some(PathBuf::from(value)),
                "--markdown" => options.markdown = Some(PathBuf::from(value)),
                "--merge" => options.merge = Some(PathBuf::from(value)),
                "--engine" => {
                    options.engine = match value.as_str() {
                        "zaxnes" => TestEngine::Zaxnes,
                        "neshawk" => TestEngine::Neshawk,
                        other => {
                            return Err(HarnessError::Usage(format!(
                                "--engine {other:?} is not zaxnes or neshawk\n{USAGE}"
                            )))
                        }
                    };
                }
                "--neshawk-gamedb" => options.neshawk_gamedb = Some(PathBuf::from(value)),
                "--coin-run-all" => options.coin_run_all = Some(PathBuf::from(value)),
                "--frames" => {
                    options.frame_cap = Some(
                        value
                            .parse::<u32>()
                            .map_err(|_| {
                                HarnessError::Usage(format!("--frames: {value:?} is not a count"))
                            })?
                            .max(1),
                    );
                }
                _ => return Err(usage()),
            }
            rest = tail;
        }
        Ok(options)
    }

    /// Where the PNG of a ROM's final frame goes: `--shots`, or the directory of `<out.json>`.
    fn shots_dir(&self) -> Option<PathBuf> {
        if self.no_shots {
            return None;
        }
        self.shots.clone().or_else(|| {
            Some(match self.out.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent.join("shots"),
                _ => PathBuf::from("shots"),
            })
        })
    }
}

/// How a ROM came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The ROM reported success.
    Pass,
    /// The ROM reported a failure; the code and its message come with it.
    Fail,
    /// The frame budget ran out before the ROM reported anything.
    Timeout,
    /// Zaxnes does not run this mapper or this file format.
    Unsupported,
    /// The ROM only reports on screen or by sound and no automatic reading is reliable; the
    /// PNG of the final frame is the evidence.
    Manual,
}

impl Outcome {
    /// The word written in the JSON and in the report.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Timeout => "timeout",
            Self::Unsupported => "unsupported",
            Self::Manual => "manual",
        }
    }
}

/// One ROM's row.
#[derive(Debug, Clone)]
pub struct RomResult {
    /// Path below the ROM directory, with `/` separators.
    pub rom: String,
    /// The suite the report groups the ROM under.
    pub suite: String,
    /// iNES mapper from the header, `None` when the header does not parse.
    pub mapper: Option<i32>,
    /// NES 2.0 submapper, `None` for an iNES 1.0 header.
    pub submapper: Option<i32>,
    /// The verdict.
    pub outcome: Outcome,
    /// The result code the ROM gave, when it gives one.
    pub code: Option<u32>,
    /// What the ROM said, or why the harness could not read it.
    pub message: String,
    /// Which reader was used.
    pub protocol: &'static str,
    /// Emulated frames run.
    pub frames: u32,
    /// Wall-clock milliseconds the run took.
    pub milliseconds: u64,
    /// Whatever the suite's reader wants to keep (sub-results, detected board, tallies).
    pub detail: Map<String, Value>,
    /// The PNG written next to the JSON, relative to it.
    pub shot: Option<String>,
}

impl RomResult {
    fn to_json(&self) -> Value {
        let mut row = Map::new();
        row.insert("rom".into(), json!(self.rom));
        row.insert("suite".into(), json!(self.suite));
        row.insert("mapper".into(), json!(self.mapper));
        row.insert("submapper".into(), json!(self.submapper));
        row.insert("result".into(), json!(self.outcome.label()));
        row.insert("code".into(), json!(self.code));
        row.insert("message".into(), json!(self.message));
        row.insert("protocol".into(), json!(self.protocol));
        row.insert("frames".into(), json!(self.frames));
        row.insert("ms".into(), json!(self.milliseconds));
        row.insert("shot".into(), json!(self.shot));
        if !self.detail.is_empty() {
            row.insert("detail".into(), Value::Object(self.detail.clone()));
        }
        Value::Object(row)
    }
}

/// How many of each outcome a campaign produced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// `.nes` files the filter kept.
    pub total: usize,
    /// Rows that passed.
    pub pass: usize,
    /// Rows that failed.
    pub fail: usize,
    /// Rows whose budget ran out.
    pub timeout: usize,
    /// Rows Zaxnes cannot run.
    pub unsupported: usize,
    /// Rows only a human can read.
    pub manual: usize,
}

impl Summary {
    fn count(&mut self, outcome: Outcome) {
        self.total += 1;
        match outcome {
            Outcome::Pass => self.pass += 1,
            Outcome::Fail => self.fail += 1,
            Outcome::Timeout => self.timeout += 1,
            Outcome::Unsupported => self.unsupported += 1,
            Outcome::Manual => self.manual += 1,
        }
    }
}

/// Runs the campaign: every `.nes` under `roms`, in path order, to `<out.json>` (a JSON array)
/// with one PNG of the final frame per ROM.
///
/// # Errors
///
/// [`HarnessError`] when the ROM directory cannot be walked, or a file cannot be read, encoded
/// or written.
pub fn run(options: &Options) -> Result<Summary, HarnessError> {
    // `--reuse` rewrites the report from a campaign already on disk. Running 331 ROMs is
    // minutes of emulation, and a change to the wording of the report is no reason to spend
    // them again — nor to let the committed report come from a different run than its JSON.
    let results = if options.reuse {
        reread(&options.out)?
    } else if let Some(base) = &options.merge {
        // Re-measuring one suite after a reader change, without re-running the other 320
        // ROMs: the rows of `base` are kept, and every ROM this run touched replaces its own.
        let mut merged = reread(base)?;
        let fresh = collect(options)?;
        merged.retain(|row| !fresh.iter().any(|new| new.rom == row.rom));
        merged.extend(fresh);
        merged.sort_by(|left, right| left.rom.cmp(&right.rom));
        merged
    } else {
        collect(options)?
    };
    let mut summary = Summary::default();
    for result in &results {
        summary.count(result.outcome);
    }

    if !options.reuse {
        let rows: Vec<Value> = results.iter().map(RomResult::to_json).collect();
        let document = json!({
            "roms_dir": options.roms.display().to_string(),
            "filter": options.filter,
            "summary": {
                "total": summary.total,
                "pass": summary.pass,
                "fail": summary.fail,
                "timeout": summary.timeout,
                "unsupported": summary.unsupported,
                "manual": summary.manual,
            },
            "results": rows,
        });
        if let Some(parent) = options.out.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(
            &options.out,
            serde_json::to_vec_pretty(&document).map_err(std::io::Error::from)?,
        )?;
    }

    if let Some(file) = &options.markdown {
        if let Some(parent) = file.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(file, report::markdown(&options.roms, &results, summary)?)?;
    }

    Ok(summary)
}

/// Reads the rows of a campaign back out of the JSON it wrote.
///
/// Only the fields the report uses are restored; `detail` comes back whole, so nothing that
/// was recorded is lost.
///
/// # Errors
///
/// [`HarnessError::Io`] when the file cannot be read and [`HarnessError::Format`] when it is
/// not a campaign document.
fn reread(path: &Path) -> Result<Vec<RomResult>, HarnessError> {
    let text = fs::read_to_string(path)?;
    let document: Value = serde_json::from_str(&text).map_err(std::io::Error::from)?;
    let rows = document
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            HarnessError::Format(format!(
                "{}: no \"results\" array; is this a test-roms JSON?",
                path.display()
            ))
        })?;

    rows.iter()
        .map(|row| {
            let text_of = |key: &str| {
                row.get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            let rom = text_of("rom");
            let label = text_of("result");
            let outcome = match label.as_str() {
                "pass" => Outcome::Pass,
                "fail" => Outcome::Fail,
                "timeout" => Outcome::Timeout,
                "unsupported" => Outcome::Unsupported,
                "manual" => Outcome::Manual,
                _ => {
                    return Err(HarnessError::Format(format!(
                        "{}: {rom:?} has an unknown result {label:?}",
                        path.display()
                    )))
                }
            };

            Ok(RomResult {
                suite: if text_of("suite").is_empty() {
                    suite_of(&rom)
                } else {
                    text_of("suite")
                },
                rom,
                mapper: row
                    .get("mapper")
                    .and_then(Value::as_i64)
                    .and_then(|mapper| i32::try_from(mapper).ok()),
                submapper: row
                    .get("submapper")
                    .and_then(Value::as_i64)
                    .and_then(|value| i32::try_from(value).ok()),
                outcome,
                code: row
                    .get("code")
                    .and_then(Value::as_u64)
                    .and_then(|code| u32::try_from(code).ok()),
                message: text_of("message"),
                protocol: "reused",
                frames: row
                    .get("frames")
                    .and_then(Value::as_u64)
                    .and_then(|frames| u32::try_from(frames).ok())
                    .unwrap_or(0),
                milliseconds: row.get("ms").and_then(Value::as_u64).unwrap_or(0),
                detail: row
                    .get("detail")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default(),
                shot: row
                    .get("shot")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
            })
        })
        .collect()
}

/// Runs every ROM the options select and returns the rows, in path order.
///
/// # Errors
///
/// [`HarnessError`] when the ROM directory cannot be walked, or a ROM or a PNG cannot be read
/// or written.
pub fn collect(options: &Options) -> Result<Vec<RomResult>, HarnessError> {
    let mut files = Vec::new();
    collect_roms(&options.roms, &mut files)?;
    let mut named: Vec<(String, PathBuf)> = files
        .into_iter()
        .map(|file| (relative_name(&options.roms, &file), file))
        .filter(|(name, _)| {
            options
                .filter
                .as_ref()
                .is_none_or(|needle| name.contains(needle.as_str()))
        })
        .collect();
    named.sort_by(|left, right| left.0.cmp(&right.0));

    let shots = options.shots_dir();
    // One database load for the whole collection: `NesCarts.xml` is 2 MB and parsing it per ROM
    // would dominate a 290-ROM run.
    let neshawk_db = match options.engine {
        TestEngine::Zaxnes => None,
        TestEngine::Neshawk => Some(
            match options
                .neshawk_gamedb
                .clone()
                .or_else(|| std::env::var_os("ZAXNES_BIZHAWK_GAMEDB").map(PathBuf::from))
            {
                Some(dir) => zax_neshawk::gamedb::GameDb::load_from_dir(&dir),
                None => zax_neshawk::gamedb::GameDb::empty(),
            },
        ),
    };
    let run_all = options.coin_run_all.as_deref();
    let mut results = Vec::with_capacity(named.len());
    for (name, file) in named {
        let image = fs::read(&file)?;
        let mut result = match options.engine {
            TestEngine::Zaxnes => run_one_zaxnes(&name, &image, options.frame_cap, run_all),
            TestEngine::Neshawk => run_one_neshawk(
                &name,
                &image,
                options.frame_cap,
                run_all,
                neshawk_db.as_ref(),
            ),
        };
        if let Some(dir) = &shots {
            result.result.shot = write_shot(dir, &name, &result.frame)?;
        }

        if !options.quiet {
            println!(
                "{:<11} {name} {}",
                result.result.outcome.label(),
                result.result.message
            );
        }
        results.push(result.result);
    }

    Ok(results)
}

/// `zax-neshawk`'s region as the harness's own enumeration.
fn neshawk_region(nes: &zax_neshawk::Nes) -> NesRegion {
    match nes.region() {
        zax_neshawk::DisplayType::Pal => NesRegion::Pal,
        zax_neshawk::DisplayType::Dendy => NesRegion::Dendy,
        zax_neshawk::DisplayType::Ntsc => NesRegion::Ntsc,
    }
}

/// The database a `--engine neshawk` run with no `gamedb` directory uses: empty, so every lookup
/// misses and detection falls through to the iNES header.
static EMPTY_DB: std::sync::OnceLock<zax_neshawk::gamedb::GameDb> = std::sync::OnceLock::new();

/// Which core `--engine` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestEngine {
    /// Zaxnes's own core.
    Zaxnes,
    /// `zax-neshawk`, the Rust port of BizHawk's NesHawk (lot NESHAWK-REWRITE).
    Neshawk,
}

/// A finished run: the row and the final frame, kept so the caller can write the PNG.
struct Finished {
    result: RomResult,
    /// The 256 x 240 index image of the last frame, empty when the ROM never ran.
    frame: Vec<u8>,
}

/// Loads `image`, runs it with the reader its suite documents and returns the row.
fn run_one_zaxnes(
    name: &str,
    image: &[u8],
    frame_cap: Option<u32>,
    run_all: Option<&Path>,
) -> Finished {
    let started = Instant::now();
    let header = InesHeader::try_parse(image).ok();
    let mut result = RomResult {
        rom: name.to_owned(),
        suite: suite_of(name),
        mapper: header.as_ref().map(|header| header.mapper),
        submapper: header
            .as_ref()
            .filter(|header| header.is_nes20)
            .map(|header| header.submapper),
        outcome: Outcome::Unsupported,
        code: None,
        message: String::new(),
        protocol: "none",
        frames: 0,
        milliseconds: 0,
        detail: Map::new(),
        shot: None,
    };

    let mut core = NesCore::new();
    // `ZAXNES_OAM_CORRUPTION=1` runs the campaign with the core's off-by-default OAM row
    // corruption (lot FIDELITY step 9), which `AccuracyCoin`'s `OAM Corruption` row needs.
    core.set_oam_corruption(std::env::var("ZAXNES_OAM_CORRUPTION").is_ok_and(|value| value != "0"));
    // The built-in widescreen patches rewrite game code; a test ROM must run as it is.
    core.set_builtin_patches(false);
    core.set_advanced_patches(false);
    let stem = Path::new(name)
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let load = core.load_rom(image, &stem);
    if !load.success {
        result.message = load.error.unwrap_or_else(|| format!("{:?}", load.failure));
        result.milliseconds = elapsed_ms(started);
        return Finished {
            result,
            frame: Vec::new(),
        };
    }

    // The core detects the region but does not apply it: REQ-CORE-013 leaves that to the host,
    // because a player's choice must win over a header. Here the harness *is* the host, and a
    // ROM made for one console has to be run on that console or its result means nothing.
    let region = region_of(name, load.region);
    core.set_region(region);
    result
        .detail
        .insert("region".into(), json!(format!("{region:?}")));

    core.reset();
    read_verdict(name, frame_cap, run_all, &mut core, result, started)
}

/// The same for `zax-neshawk`. Loading is all that differs: the port detects its board from
/// BizHawk's own databases and powers on inside `Nes::load`, where `NesCore` separates the two.
fn run_one_neshawk(
    name: &str,
    image: &[u8],
    frame_cap: Option<u32>,
    run_all: Option<&Path>,
    db: Option<&zax_neshawk::gamedb::GameDb>,
) -> Finished {
    let started = Instant::now();
    let header = InesHeader::try_parse(image).ok();
    let mut result = RomResult {
        rom: name.to_owned(),
        suite: suite_of(name),
        mapper: header.as_ref().map(|header| header.mapper),
        submapper: header
            .as_ref()
            .filter(|header| header.is_nes20)
            .map(|header| header.submapper),
        outcome: Outcome::Unsupported,
        code: None,
        message: String::new(),
        protocol: "none",
        frames: 0,
        milliseconds: 0,
        detail: Map::new(),
        shot: None,
    };
    result.detail.insert("engine".into(), json!("neshawk"));

    let db = db.unwrap_or_else(|| EMPTY_DB.get_or_init(zax_neshawk::gamedb::GameDb::empty));
    let nes = match zax_neshawk::Nes::load(
        image,
        db,
        zax_neshawk::controllers::PortDevice::ControllerNes,
        zax_neshawk::controllers::PortDevice::ControllerNes,
    ) {
        Ok(nes) => nes,
        Err(why) => {
            result.message = why.to_string();
            result.milliseconds = elapsed_ms(started);
            return Finished {
                result,
                frame: Vec::new(),
            };
        }
    };
    // The owner's marker: a mapper with no ported NesHawk board runs on Zaxnes's board, and the
    // row has to say so or a verdict from it would read as NesHawk's (owner, 2026-11-12).
    let from_zaxnes = nes
        .load_report
        .as_ref()
        .is_some_and(|report| report.board_from_zaxnes);
    let board = nes
        .load_report
        .as_ref()
        .map_or("?", |report| report.board_name)
        .to_owned();
    // `neshawk_board`, not `board`: `mapperel.rs` already writes `board` with the name the ROM
    // prints on its own screen, and the two would overwrite each other.
    result.detail.insert("neshawk_board".into(), json!(board));
    result
        .detail
        .insert("board_from_zaxnes".into(), json!(from_zaxnes));

    let mut core = machine::NeshawkTestMachine::new(nes);
    // `Nes::load` applied the region off the header, which is the port's own contract — but a ROM
    // made for one console has to be *run* on that console or its result means nothing, and
    // `pal_apu_tests`'s ROMs state nothing in their iNES v1 headers. `region_of` is the harness's
    // own rule, so the port follows it too; `_display_type` is read by `HardReset`, so the
    // machine is powered on again after the change.
    let wanted = region_of(name, neshawk_region(&core.nes));
    if wanted != neshawk_region(&core.nes) {
        core.nes.set_display_type(match wanted {
            NesRegion::Pal => zax_neshawk::DisplayType::Pal,
            NesRegion::Dendy => zax_neshawk::DisplayType::Dendy,
            _ => zax_neshawk::DisplayType::Ntsc,
        });
        core.nes.hard_reset();
    }
    result
        .detail
        .insert("region".into(), json!(format!("{wanted:?}")));

    let mut finished = read_verdict(name, frame_cap, run_all, &mut core, result, started);
    if from_zaxnes {
        // In the message as well as in `detail`, because the message is what the markdown report
        // and the one-line-per-ROM output print.
        if !finished.result.message.is_empty() {
            finished.result.message.push_str(" / ");
        }
        finished
            .result
            .message
            .push_str("board from Zaxnes, not NesHawk-exact");
    }
    finished
}

/// The half that does not care which core it is: the plan, the reader, the row.
fn read_verdict<M: machine::TestMachine>(
    name: &str,
    frame_cap: Option<u32>,
    run_all: Option<&Path>,
    core: &mut M,
    mut result: RomResult,
    started: Instant,
) -> Finished {
    let plan = Plan::of(name, frame_cap);
    let verdict = match plan.protocol {
        Protocol::BlarggThenScreen => match blargg::run(core, plan.frames) {
            blargg::Read::Verdict(verdict) => verdict,
            // The ROM never armed the protocol, so it is one of the suites that only draws its
            // result. The frames already spent probing count towards the budget, and the rest
            // is capped: a result screen appears in seconds, while the demos of
            // `nes-test-roms/other/` never settle at all and would each eat the full budget.
            blargg::Read::NoProtocol { frames } => screen::run(
                core,
                plan.frames
                    .saturating_sub(frames)
                    .min(screen::FALLBACK_BUDGET),
                plan.screen,
            )
            .after(frames),
        },
        Protocol::Screen => screen::run(core, plan.frames, plan.screen),
        Protocol::Mapperel => mapperel::run(core, result.mapper, plan.frames),
        Protocol::AccuracyCoin => match run_all {
            Some(path) => {
                let (lines, passed, frames) = coin::run_all(core, 100_000);
                let text = format!(
                    "{}
pass {passed}/{} frames {frames}
",
                    lines.join(
                        "
"
                    ),
                    lines.len()
                );
                if let Err(why) = fs::write(path, text) {
                    eprintln!("test-roms: --coin-run-all: {why}");
                }
                let mut verdict = coin::run(core, plan.frames);
                verdict
                    .detail
                    .insert("coin_run_all_passed".into(), json!(passed));
                verdict
                    .detail
                    .insert("coin_run_all_frames".into(), json!(frames));
                verdict
            }
            None => coin::run(core, plan.frames),
        },
    };

    result.protocol = verdict.protocol;
    result.outcome = verdict.outcome;
    result.code = verdict.code;
    result.message = verdict.message;
    result.frames = verdict.frames;
    result.detail.extend(verdict.detail);
    result.milliseconds = elapsed_ms(started);

    let mut frame = Vec::new();
    core.picture_into(&mut frame);
    Finished { result, frame }
}

/// What a reader concluded.
#[derive(Debug, Clone)]
pub struct Verdict {
    /// Which reader spoke.
    pub protocol: &'static str,
    /// The verdict.
    pub outcome: Outcome,
    /// The code the ROM gave.
    pub code: Option<u32>,
    /// What the ROM said.
    pub message: String,
    /// Emulated frames run.
    pub frames: u32,
    /// Suite-specific extras.
    pub detail: Map<String, Value>,
}

impl Verdict {
    /// A verdict with no code and no extras.
    #[must_use]
    pub fn new(protocol: &'static str, outcome: Outcome, message: String, frames: u32) -> Self {
        Self {
            protocol,
            outcome,
            code: None,
            message,
            frames,
            detail: Map::new(),
        }
    }

    /// The same verdict, with `frames` added to its own count: what a reader that took over
    /// from another returns, so the row says how long the whole run was.
    #[must_use]
    pub fn after(mut self, frames: u32) -> Self {
        self.frames = self.frames.saturating_add(frames);
        self
    }
}

/// Which reader runs a ROM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Protocol {
    /// Try the `$6000` protocol first and fall back to the screen: half of the community
    /// collection predates the protocol (the 2005 PPU, sprite-hit and sprite-overflow suites
    /// report on screen only), and nothing in a file says which era a ROM belongs to.
    BlarggThenScreen,
    /// A ROM known to report on screen only.
    Screen,
    /// Holy Mapperel: a screen with a documented layout, read field by field.
    Mapperel,
    /// `AccuracyCoin`: press Start at the menu's top, then read its 147 result bytes.
    AccuracyCoin,
}

/// The reader and the frame budget of one ROM.
#[derive(Debug, Clone, Copy)]
struct Plan {
    protocol: Protocol,
    screen: screen::Kind,
    frames: u32,
}

impl Plan {
    /// The plan of the ROM at relative path `name`.
    ///
    /// A ROM that uses the `$6000` protocol stops as soon as its status leaves `$80`, so a
    /// budget only decides how long a hang is given before it is called a timeout.
    fn of(name: &str, frame_cap: Option<u32>) -> Self {
        let lower = name.to_ascii_lowercase();
        let (protocol, frames) = if lower.contains("accuracycoin") {
            (Protocol::AccuracyCoin, screen::frame_budget(&lower))
        } else if lower.contains("holy-mapperel") {
            (Protocol::Mapperel, screen::frame_budget(&lower))
        } else if screen::is_screen_rom(&lower) {
            (Protocol::Screen, screen::frame_budget(&lower))
        } else {
            (Protocol::BlarggThenScreen, blargg::frame_budget(&lower))
        };

        Self {
            protocol,
            screen: screen::Kind::of(&lower),
            frames: frame_cap.map_or(frames, |cap| cap.min(frames).max(1)),
        }
    }
}

/// Writes the PNG of a ROM's final frame and returns its path relative to `dir`'s parent.
fn write_shot(dir: &Path, name: &str, frame: &[u8]) -> Result<Option<String>, HarnessError> {
    if frame.len() < 256 * 240 {
        return Ok(None);
    }

    let file = dir.join(format!("{}.png", name.replace(['/', '\\'], "_")));
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut rgb = Vec::with_capacity(256 * 240 * 3);
    for &index in &frame[..256 * 240] {
        let color = zax_engine3d::palette::nes_color(usize::from(index & 0x3F));
        rgb.extend_from_slice(&[color.r, color.g, color.b]);
    }
    fs::write(
        &file,
        zax_render::assets::png_image::encode_rgb(256, 240, &rgb)?,
    )?;
    Ok(file
        .file_name()
        .map(|file| file.to_string_lossy().into_owned()))
}

/// Which console the ROM at relative path `name` is to be run on.
///
/// `detected` is what the header says, which is right for almost everything. One suite is
/// not: `nes-test-roms/pal_apu_tests/` is, in the words of its own readme, "PAL NES APU
/// Tests [...] tested on a PAL NES and all give a passing result", yet every one of its ten
/// files carries a plain NTSC header. Run under NTSC they measure the wrong frame sequencer
/// and seven of the ten report a failure that says nothing about Zaxnes. The directory name
/// and the readme agree, so the suite is run on the console it was written for.
fn region_of(name: &str, detected: NesRegion) -> NesRegion {
    if name.to_ascii_lowercase().contains("pal_apu_tests") {
        NesRegion::Pal
    } else {
        detected
    }
}

/// The suite a ROM is grouped under: the first two path components of the community
/// collection (`nes-test-roms/mmc3_test`), the first one elsewhere.
fn suite_of(name: &str) -> String {
    let mut parts = name.split('/');
    let Some(first) = parts.next() else {
        return String::new();
    };
    match parts.next() {
        Some(second) if first == "nes-test-roms" => format!("{first}/{second}"),
        _ => first.to_owned(),
    }
}

fn collect_roms(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), HarnessError> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_roms(&path, found)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("nes"))
        {
            found.push(path);
        }
    }

    Ok(())
}

/// The path of `file` below `root`, with `/` separators.
fn relative_name(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// The text a test ROM wrote, cleaned up for one line of JSON: runs of control characters
/// become a single `/`, and anything outside printable ASCII a `?`.
///
/// The newer blargg builds colour their output with ANSI escapes — `cpu_dummy_writes` writes
/// `ESC [ 0 ; 3 7 m` before every line — so a CSI sequence is dropped whole. Keeping the
/// letters of the escape instead would scatter `0;37m` through every message.
#[must_use]
pub fn one_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1B}' {
            // CSI: `ESC [` then parameter and intermediate bytes, ended by a final byte in
            // `@`..`~`. Anything else after ESC is a two-character sequence.
            if characters.peek() == Some(&'[') {
                characters.next();
                while characters
                    .next_if(|next| !matches!(next, '\u{40}'..='\u{7E}'))
                    .is_some()
                {}
            }
            characters.next();
            continue;
        }

        if character.is_ascii_graphic() || character == ' ' {
            if pending && !out.is_empty() {
                out.push_str(" / ");
            }
            pending = false;
            out.push(character);
        } else if character.is_ascii() || character.is_whitespace() {
            pending = true;
        } else {
            pending = false;
            out.push('?');
        }
    }

    out.trim().to_owned()
}
