//! `neshawk-tiles`: where the **NesHawk port's** tile records differ from `zax-nes`'s, field by
//! field — lot FCEUX-6's `fceux-tiles` pointed at the other engine.
//!
//! It is the same measuring stick, deliberately: [`crate::fceux_tiles::compare_records`] is the
//! comparison, so the ten-field mask, the multiset count, the OAM-rotation check, the row
//! histograms and the readable record pairs are the ones lot FCEUX-6 built and argued for. Only
//! the engine on the right changes.
//!
//! # The class column is the whole point
//!
//! A record difference means one of two very different things, and a table that mixes them says
//! nothing:
//!
//! * **`capture`** — the two engines' 256×240 pictures for that frame are *identical*, so the
//!   machines agree on what is on screen and only the capture disagrees. **Those are this lot's
//!   to answer for.**
//! * **`emulation`** — the pictures already differ, so the two cores are not showing the same
//!   frame. A record difference there is NesHawk's emulation against Zaxnes's, which is the point
//!   of the whole lot rather than a defect of the capture: the two are *different machines*, and
//!   `docs/review/NESHAWK-REWRITE.md` reports NesHawk's own test-ROM verdicts beside Zaxnes's.
//! * **`uniform`** — Zaxnes's picture is a single colour, so two identical pictures prove much
//!   less than usual and the frame is evidence about the capture in neither direction.
//!
//! # What `fceux-tiles` needed and this does not
//!
//! No child process per ROM: `zax-neshawk` keeps no process-global state and no C, so any number
//! of machines live at once and a bad cartridge cannot take the process down. Both engines run in
//! one process over the whole sweep.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde_json::{json, Map, Value};
use zax_core_api::Engine;
use zax_neshawk::NesHawkEngine;

use crate::golden_core::{collect_roms, relative_name};
use crate::tile_diff::{compare_records, field_names, FrameDiff};
use crate::HarnessError;

/// The frames compared when `--frames` says nothing: lot FCEUX-6's own list.
const DEFAULT_FRAMES: &[u32] = &[60, 300, 1200, 3000];

pub const USAGE: &str = "zax-harness neshawk-tiles <roms dir|rom.nes> <out.jsonl> [options]
    Runs zax-nes and zax-neshawk over every ROM and compares their tile records field by
    field, frame by frame -- lot FCEUX-6's `fceux-tiles` pointed at the NesHawk port.
      --frames a,b,c    the frames to compare (default 60,300,1200,3000)
      --records <n>     print the first n differing record pairs of each frame, field by field
      --detail          print the differing fields and rows of every frame that differs
      --markdown <file> also write the report as a table
      --ram neshawk|zero   what the port's power-on writes into the 2 KiB of work RAM:
                        `neshawk` is NesHawk's own default (FCEUX's 00 00 00 00 FF FF FF FF),
                        `zero` is what zax-nes fills -- which is what makes the two machines
                        start from the SAME memory, and so what makes a record difference
                        evidence about the capture rather than about the power-on (default: zero)
      --neshawk-gamedb <dir>  BizHawk's gamedb directory, so that the port picks the board
                        BizHawk picks; defaults to $ZAXNES_BIZHAWK_GAMEDB";

#[derive(Debug, Clone)]
pub struct Options {
    pub roms: String,
    pub out: String,
    pub frames: Vec<u32>,
    pub records: usize,
    pub detail: bool,
    pub markdown: Option<String>,
    pub gamedb: Option<String>,
    /// The port's power-on work-RAM pattern, empty for NesHawk's own default. `vec![0]` is
    /// `zax-nes`'s all-zero fill, which is the default here: see `--ram` in [`USAGE`].
    pub ram: Vec<u8>,
}

impl Options {
    /// Parses `<roms> <out.jsonl> [options]`.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] on a missing argument or an unparsable frame list.
    pub fn parse(args: &[String]) -> Result<Self, HarnessError> {
        let usage = || HarnessError::Usage(USAGE.to_owned());
        let [roms, out, rest @ ..] = args else {
            return Err(usage());
        };
        let mut options = Self {
            roms: roms.clone(),
            out: out.clone(),
            frames: DEFAULT_FRAMES.to_vec(),
            records: 0,
            detail: false,
            markdown: None,
            gamedb: std::env::var("ZAXNES_BIZHAWK_GAMEDB").ok(),
            ram: vec![0],
        };
        let mut rest = rest.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--detail" => options.detail = true,
                "--frames" => {
                    let list = rest.next().ok_or_else(usage)?;
                    options.frames = list
                        .split(',')
                        .map(|part| part.trim().parse::<u32>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| {
                            HarnessError::Usage(format!("{list:?} is not a frame list\n{USAGE}"))
                        })?;
                    options.frames.sort_unstable();
                }
                "--records" => {
                    let count = rest.next().ok_or_else(usage)?;
                    options.records = count
                        .parse()
                        .map_err(|_| HarnessError::Usage(format!("{count:?} is not a count")))?;
                }
                "--ram" => {
                    let which = rest.next().ok_or_else(usage)?;
                    options.ram = match which.as_str() {
                        "neshawk" => Vec::new(),
                        "zero" => vec![0],
                        _ => return Err(usage()),
                    };
                }
                "--markdown" => options.markdown = Some(rest.next().ok_or_else(usage)?.clone()),
                "--neshawk-gamedb" => options.gamedb = Some(rest.next().ok_or_else(usage)?.clone()),
                _ => return Err(usage()),
            }
        }
        Ok(options)
    }
}

/// What one ROM's run found.
#[derive(Debug, Default)]
struct RomDiff {
    frames: Vec<(u32, FrameDiff)>,
    /// The first frame whose pictures differ at all, which is where the two machines part.
    first_pixel_diff: Option<u32>,
    stopped_at: Option<String>,
    /// Whether the port ran this cartridge on a Zaxnes board rather than a ported NesHawk one,
    /// in which case a difference says nothing about NesHawk's board.
    board_from_zaxnes: bool,
    board: String,
}

/// Runs both engines over `image` and compares at each frame of `frames`.
///
/// Both are driven exactly as `golden-core` drives Zaxnes — the file stem as the name, a reset
/// after loading, no button pressed — so a difference here is the engines' and not the harness's.
fn compare_rom(rom: &str, image: &[u8], options: &Options) -> Result<RomDiff, String> {
    let stem = Path::new(rom)
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());

    let mut zaxnes = zax_nes::NesCore::new();
    if !zaxnes.load_rom(image, &stem).success {
        return Err("Zaxnes refused the image".to_owned());
    }
    zaxnes.reset();

    let mut neshawk = NesHawkEngine::new();
    if let Some(dir) = &options.gamedb {
        neshawk.set_gamedb_dir(Path::new(dir));
    }
    neshawk.set_tile_capture(true);
    neshawk.set_initial_wram_pattern(options.ram.clone());
    let loaded = neshawk.load_rom(image, &stem);
    if !loaded.success {
        return Err(format!(
            "the NesHawk port refused the image: {}",
            loaded.error.unwrap_or_default()
        ));
    }
    neshawk.reset();

    let mut result = RomDiff {
        board_from_zaxnes: neshawk.board_is_from_zaxnes(),
        board: neshawk.board_name().unwrap_or("?").to_owned(),
        ..RomDiff::default()
    };

    let last = options.frames.iter().copied().max().unwrap_or(1);
    let mut ours = Vec::new();
    let mut theirs = Vec::new();

    for frame in 1..=last {
        zaxnes.run_frame(0);
        neshawk.run_frame(0);

        zaxnes.pixel_frame_indices_into(&mut ours);
        neshawk.pixel_frame_indices_into(&mut theirs);
        let pixels_identical = ours == theirs;
        if !pixels_identical && result.first_pixel_diff.is_none() {
            result.first_pixel_diff = Some(frame);
        }

        if !options.frames.contains(&frame) {
            continue;
        }

        // A single-colour picture makes "the pictures are identical" much weaker evidence, so
        // the class says `uniform` rather than letting `capture` claim the frame.
        let pixels_uniform = ours
            .first()
            .is_some_and(|first| ours.iter().all(|p| p == first));

        let batch = neshawk.frame_batch();
        let diff = compare_records(
            zaxnes.frame_batch().records,
            batch.records,
            pixels_identical,
            pixels_uniform,
            options.records,
        );
        result.frames.push((frame, diff));
    }

    Ok(result)
}

/// The class of a frame, which is the column to read first.
fn class_of(diff: &FrameDiff) -> &'static str {
    if diff.zaxnes == diff.fceux && diff.same_in_order == diff.zaxnes {
        return "identical";
    }
    if !diff.pixels_identical {
        return "emulation";
    }
    if diff.pixels_uniform {
        return "uniform";
    }
    "capture"
}

fn row_for(rom: &str, image: &[u8], options: &Options) -> Value {
    let mut row = Map::new();
    row.insert("rom".into(), json!(rom));
    match compare_rom(rom, image, options) {
        Err(error) => {
            row.insert("error".into(), json!(error));
        }
        Ok(result) => {
            row.insert("board".into(), json!(result.board));
            row.insert("board_from_zaxnes".into(), json!(result.board_from_zaxnes));
            row.insert("first_pixel_diff".into(), json!(result.first_pixel_diff));
            if let Some(stopped) = result.stopped_at {
                row.insert("stopped_at".into(), json!(stopped));
            }
            let frames: Vec<Value> = result
                .frames
                .iter()
                .map(|(frame, diff)| {
                    let mut entry = Map::new();
                    entry.insert("frame".into(), json!(frame));
                    entry.insert("class".into(), json!(class_of(diff)));
                    entry.insert("zaxnes".into(), json!(diff.zaxnes));
                    entry.insert("neshawk".into(), json!(diff.fceux));
                    entry.insert("same_in_order".into(), json!(diff.same_in_order));
                    entry.insert("same_as_multiset".into(), json!(diff.same_as_multiset));
                    entry.insert("pixels_identical".into(), json!(diff.pixels_identical));
                    entry.insert("pixels_uniform".into(), json!(diff.pixels_uniform));
                    entry.insert("unmatched_zaxnes".into(), json!(diff.unmatched_zaxnes));
                    entry.insert("unmatched_neshawk".into(), json!(diff.unmatched_fceux));
                    entry.insert("oam_rotation".into(), json!(diff.oam_rotation));
                    if options.detail {
                        entry.insert("rows".into(), json!(diff.rows));
                        let fields: Vec<Value> = diff
                            .by_fields
                            .iter()
                            .map(|((background, mask), count)| {
                                json!({
                                    "background": background,
                                    "fields": field_names(*mask),
                                    "count": count,
                                })
                            })
                            .collect();
                        entry.insert("by_fields".into(), json!(fields));
                    }
                    if !diff.pairs.is_empty() {
                        let pairs: Vec<Value> = diff
                            .pairs
                            .iter()
                            .map(|(a, b)| json!({ "zaxnes": a, "neshawk": b }))
                            .collect();
                        entry.insert("pairs".into(), json!(pairs));
                    }
                    Value::Object(entry)
                })
                .collect();
            row.insert("frames".into(), json!(frames));
        }
    }
    Value::Object(row)
}

/// `neshawk-tiles <roms> <out.jsonl> [options]`.
///
/// # Errors
///
/// A usage error, or the output file could not be written.
pub fn command(args: &[String]) -> Result<(), HarnessError> {
    let options = Options::parse(args)?;
    let text = run(&options)?;
    print!("{text}");
    if let Some(path) = &options.markdown {
        fs::write(path, &text)?;
    }
    Ok(())
}

/// Runs the sweep and returns the report.
///
/// # Errors
///
/// The ROM directory could not be read, or the output file could not be written.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let roms = Path::new(&options.roms);
    // `<roms dir|rom.nes>`: a single file is one row, which is how a difference found in the
    // sweep is then looked at with `--detail` and `--records`.
    let mut list = Vec::new();
    if roms.is_dir() {
        collect_roms(roms, &mut list)?;
        list.sort_by_key(|file| relative_name(roms, file));
    } else {
        list.push(roms.to_path_buf());
    }
    let root = if roms.is_dir() {
        roms.to_path_buf()
    } else {
        roms.parent().unwrap_or(Path::new("")).to_path_buf()
    };
    let mut report_rows = Vec::new();
    let file = fs::File::create(&options.out)?;
    let mut out = BufWriter::new(file);

    for path in &list {
        let Ok(image) = fs::read(path) else { continue };
        let name = relative_name(&root, path);
        let row = row_for(&name, &image, options);
        writeln!(out, "{row}")?;
        report_rows.push(row);
    }
    out.flush()?;
    Ok(report(&report_rows, options))
}

/// The report: one line per ROM per frame, then the totals that matter.
fn report(rows: &[Value], options: &Options) -> String {
    use std::fmt::Write as _;
    let mut text = String::new();
    let _ = writeln!(
        text,
        "neshawk-tiles: {} ROMs, frames {:?}\n",
        rows.len(),
        options.frames
    );
    let _ = writeln!(
        text,
        "| ROM | frame | class | zaxnes | neshawk | same (order) | same (set) |"
    );
    let _ = writeln!(text, "|---|---|---|---|---|---|---|");

    let mut totals: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut capture_records = (0u64, 0u64);
    let mut fallback = 0usize;

    for row in rows {
        let rom = row.get("rom").and_then(Value::as_str).unwrap_or("?");
        if let Some(error) = row.get("error").and_then(Value::as_str) {
            let _ = writeln!(text, "| {rom} | - | error | - | - | - | {error} |");
            *totals.entry("error").or_default() += 1;
            continue;
        }
        if row
            .get("board_from_zaxnes")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            fallback += 1;
        }
        for frame in row
            .get("frames")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let get = |key: &str| frame.get(key).and_then(Value::as_u64).unwrap_or(0);
            let class = frame.get("class").and_then(Value::as_str).unwrap_or("?");
            *totals.entry(class).or_default() += 1;
            if class == "capture" {
                capture_records.0 += get("same_in_order");
                capture_records.1 += get("zaxnes").max(get("neshawk"));
            }
            let _ = writeln!(
                text,
                "| {rom} | {} | {class} | {} | {} | {} | {} |",
                get("frame"),
                get("zaxnes"),
                get("neshawk"),
                get("same_in_order"),
                get("same_as_multiset"),
            );
        }
    }

    let _ = writeln!(text, "\nframes by class:");
    for (class, count) in &totals {
        let _ = writeln!(text, "  {class:12} {count}");
    }
    if capture_records.1 > 0 {
        let (same, total) = capture_records;
        #[allow(clippy::cast_precision_loss)] // a ratio of record counts, printed to 2 decimals
        let percent = (same as f64) * 100.0 / (total as f64);
        let _ = writeln!(
            text,
            "\ncapture-class frames: {same} of {total} records identical in order ({percent:.2} %)"
        );
    }
    let _ = writeln!(
        text,
        "{fallback} ROMs ran on a Zaxnes board rather than a ported NesHawk one, where a \
         difference says nothing about NesHawk's board"
    );
    text
}
