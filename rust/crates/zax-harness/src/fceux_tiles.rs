//! `fceux-tiles`: the measuring stick of lot FCEUX-6 — where FCEUX's **tile records** differ from
//! `zax-nes`'s, field by field, and whether the difference is the capture's or the emulation's.
//!
//! Lot FCEUX-1 could only say *how many* records matched on one ROM at one frame. That is not
//! enough to finish the capture: a record that differs differs in a particular field, on a
//! particular row, for a particular reason, and the only way to fix the capture is to be told
//! which. So this runs both engines side by side in one process and reports, per ROM and per
//! frame:
//!
//! * the record count on each side;
//! * the records identical on **all ten fields** — in order (which is what
//!   [`zax_nes::FrameBatch`] promises the renderer) and as a multiset (which is what the mesh
//!   builder would accept);
//! * for every differing record the set of fields that differ, with the record's row and column;
//! * the **class** of the frame:
//!   * `identical` — nothing differs;
//!   * `capture` — FCEUX's 256×240 picture for that frame is *identical* to Zaxnes's, so the two
//!     machines agree on what is on screen and only the capture disagrees. **These are ours to
//!     fix.**
//!   * `emulation` — the pictures already differ, so the two cores are not showing the same frame.
//!     A record difference there is FCEUX's emulation against Zaxnes's and out of this lot's
//!     scope (lot FCEUX-1 step 5 measured that disagreement: of 113 public ROMs that report a
//!     verdict, Zaxnes passes 95 and FCEUX 58).
//!
//! The class is the single most important column. Without it every table mixes two unrelated
//! things and no amount of work on the capture can make the numbers reach 100 %.
//!
//! One FCEUX per process is a hard rule (its machine is in file-scope globals), and one public ROM
//! takes the engine down (`other/oam3.nes`, lot FCEUX-1 §7), so a directory sweep gives each ROM
//! its own child process exactly as `fceux-run` does.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use zax_fceux::{Fceux, RamInit, Tile};
use zax_nes::rom::InesHeader;
use zax_nes::{NesCore, TileRecord};

use crate::golden_core::{collect_roms, hex, relative_name};
use crate::HarnessError;

pub const USAGE: &str = "zax-harness fceux-tiles <roms dir|rom.nes> <out.jsonl> [options]
    Runs both engines over every ROM and compares their tile records field by field, frame by
    frame. Needs the `fceux-core` feature and a built fork.
      --frames a,b,c    the frames to compare (default 60,300,1200,3000)
      --profiles-only   only the ROMs this tool knows a 3dn profile for
      --in-process      do not isolate each ROM in a child process
      --detail          print the differing fields and rows of every frame that differs";

/// The frames the baseline table of lot FCEUX-6 step 1 uses on the originals.
const DEFAULT_FRAMES: [u32; 4] = [60, 300, 1200, 3000];

/// How many distinct rows of differing records a row of the report names before it gives up and
/// just counts them. The point is to recognise a pattern (a status bar, a parallax split), not to
/// list 240 lines.
const MAX_ROWS_LISTED: usize = 12;

// ------------------------------------------------------------------------------------------------
// The ROM → 3dn profile map
// ------------------------------------------------------------------------------------------------

/// Which 3dn profile of `assets-neutral/profiles.json` (edition `full`) belongs to an original of
/// `G:/3DSen/test-originals`, matched on the dump's name.
///
/// The application has no automatic mapping — a profile is a per-game choice the user makes
/// (`zax-app`'s `choose_profile_for_rom`) — so this table is the comparison's own, and it is
/// listed in `docs/progress/FCEUX-6.md` so that a reader can check it. Each entry is a prefix of
/// the dump's file name, which is enough: the 98 dumps are named
/// `<title> [<region>][<flags>].nes`.
///
/// Three entries are not simply the game's own name, and each is a translated title of a game the
/// catalogue holds under its western one:
///
/// * `Akumajou Dracula` is *Castlevania*;
/// * `Rockman <n>` is *Mega Man <n>*, and `Rockman` alone is *Mega Man 1*;
/// * `Chou-WS-M` is *Chou Wakusei Senki: `MetaFight`*, the Japanese *Blaster Master* (and its header
///   agrees: MMC1, 8 PRG banks, 16 CHR banks, exactly the western dumps');
/// * `Takahashi Meijin no Bouken-j` is *Takahashi Meijin no Bouken-jima*, the Japanese *Adventure
///   Island*, and `Kiddy Sun in Fantasia` is an Asian hack of it (CNROM, 2 PRG, 4 CHR, the same
///   shape as the two Adventure Island dumps).
///
/// The originals with **no** profile in the catalogue are *Mega Man 4*, *Mega Man 5*, *Mega Man 6*
/// (and the `Rockman 4/5/6` dumps of the same games) and *Metal Force*; they are compared as
/// field-identity only.
const PROFILES: [(&str, &str); 19] = [
    ("Adventure Island", "Adventure Island"),
    ("Takahashi Meijin no Bouken-j", "Adventure Island"),
    ("Kiddy Sun in Fantasia", "Adventure Island"),
    ("Akumajou Dracula", "Castlevania"),
    ("Castlevania", "Castlevania"),
    ("Excitebike", "Excitebike"),
    ("Final Fantasy", "Final Fantasy"),
    ("Blaster Master", "Blaster Master"),
    ("Chou-WS-M", "Blaster Master"),
    ("Mega Man 3", "Mega Man 3"),
    ("Rockman 3", "Mega Man 3"),
    ("Mega Man 2", "Mega Man 2"),
    ("Rockman 2", "Mega Man 2"),
    ("Metroid", "Metroid"),
    ("Ninja Gaiden III", "Ninja Gaiden III"),
    ("Ninja Ryuuken Den III", "Ninja Gaiden III"),
    ("Super Mario Bros. 3", "Super Mario Bros. 3"),
    ("Super Mario Bros.", "Super Mario Bros."),
    ("Zelda II", "Zelda II: The Adventure of Link"),
];

/// The profile of the dump named `rom` (a path relative to the ROM root), or `None`.
///
/// The longest matching prefix wins, so that *Super Mario Bros. 3* is not taken for *Super Mario
/// Bros.* and *Mega Man 3* is not taken for *Mega Man*. `Mega Man` and `Rockman` alone are
/// *Mega Man 1*, and are matched last for the same reason.
#[must_use]
pub fn profile_for(rom: &str) -> Option<&'static str> {
    let base = Path::new(rom).file_name().map_or_else(
        || rom.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let best = PROFILES
        .iter()
        .filter(|(prefix, _)| base.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len());
    if let Some((_, title)) = best {
        return Some(title);
    }
    // `Mega Man [` / `Rockman [` — the first game, whose dumps carry no number at all.
    (base.starts_with("Mega Man [") || base.starts_with("Rockman [")).then_some("Mega Man 1")
}

// ------------------------------------------------------------------------------------------------
// Options
// ------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Options {
    pub roms: String,
    pub out: String,
    pub frames: Vec<u32>,
    pub profiles_only: bool,
    pub in_process: bool,
    pub detail: bool,
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
            profiles_only: false,
            in_process: false,
            detail: false,
        };
        let mut rest = rest.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--profiles-only" => options.profiles_only = true,
                "--in-process" => options.in_process = true,
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
                    options.frames.dedup();
                }
                _ => return Err(usage()),
            }
        }
        if options.frames.is_empty() {
            return Err(usage());
        }
        Ok(options)
    }
}

// ------------------------------------------------------------------------------------------------
// The comparison of one frame
// ------------------------------------------------------------------------------------------------

/// The ten fields of a record, flattened so that two records can be compared and the differing
/// fields named. The order is `TileRecord`'s own.
const FIELDS: [&str; 10] = [
    "plane0",
    "plane1",
    "x",
    "y",
    "index",
    "mirror_h",
    "mirror_v",
    "hidden",
    "palette",
    "background",
];

/// A record as a tuple of its ten fields, so that both engines' types compare without either
/// crate depending on the other.
type Key = (u64, u64, i16, i16, i32, bool, bool, bool, u8, bool);

const fn key_of_record(record: &TileRecord) -> Key {
    (
        record.plane0,
        record.plane1,
        record.x,
        record.y,
        record.index,
        record.mirror_h,
        record.mirror_v,
        record.hidden,
        record.palette,
        record.background,
    )
}

const fn key_of_tile(tile: &Tile) -> Key {
    (
        tile.plane0,
        tile.plane1,
        tile.x,
        tile.y,
        tile.index,
        tile.mirror_h,
        tile.mirror_v,
        tile.hidden,
        tile.palette,
        tile.background,
    )
}

/// Which of the ten fields differ between two records, as a bitmask over [`FIELDS`].
fn differing_fields(left: Key, right: Key) -> u16 {
    let mut mask = 0u16;
    if left.0 != right.0 {
        mask |= 1 << 0;
    }
    if left.1 != right.1 {
        mask |= 1 << 1;
    }
    if left.2 != right.2 {
        mask |= 1 << 2;
    }
    if left.3 != right.3 {
        mask |= 1 << 3;
    }
    if left.4 != right.4 {
        mask |= 1 << 4;
    }
    if left.5 != right.5 {
        mask |= 1 << 5;
    }
    if left.6 != right.6 {
        mask |= 1 << 6;
    }
    if left.7 != right.7 {
        mask |= 1 << 7;
    }
    if left.8 != right.8 {
        mask |= 1 << 8;
    }
    if left.9 != right.9 {
        mask |= 1 << 9;
    }
    mask
}

fn field_names(mask: u16) -> String {
    let names: Vec<&str> = FIELDS
        .iter()
        .enumerate()
        .filter(|(bit, _)| (mask & (1 << bit)) != 0)
        .map(|(_, name)| *name)
        .collect();
    names.join("+")
}

/// What one frame's comparison found.
#[derive(Debug, Clone, Default)]
pub struct FrameDiff {
    pub zaxnes: usize,
    pub fceux: usize,
    /// Records equal at the same position in both lists — the strictest reading, and the one the
    /// renderer sees (`FrameBatch::records` is ordered).
    pub same_in_order: usize,
    /// Records present on both sides with every field equal, order ignored.
    pub same_as_multiset: usize,
    /// Whether the two engines' 256×240 index pictures are identical for this frame.
    pub pixels_identical: bool,
    /// Differing records per set of differing fields.
    pub by_fields: HashMap<u16, usize>,
    /// Distinct `y` of the differing records, sorted.
    pub rows: Vec<i16>,
    /// Records with no partner at all on the other side.
    pub unmatched_zaxnes: usize,
    pub unmatched_fceux: usize,
}

impl FrameDiff {
    /// `identical`, `capture` (ours to fix) or `emulation` (FCEUX against Zaxnes).
    #[must_use]
    pub fn class(&self) -> &'static str {
        if self.zaxnes == self.fceux && self.same_in_order == self.zaxnes {
            "identical"
        } else if self.pixels_identical {
            "capture"
        } else {
            "emulation"
        }
    }

    /// The differing-field sets, most frequent first, as `plane0+plane1:106`.
    #[must_use]
    pub fn fields_summary(&self) -> String {
        let mut pairs: Vec<(u16, usize)> = self.by_fields.iter().map(|(m, c)| (*m, *c)).collect();
        // Count first, then the mask, so the order never depends on the hash map's.
        pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        pairs
            .iter()
            .map(|(mask, count)| format!("{}:{count}", field_names(*mask)))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The rows the differing records sit on, or a count when there are too many to name.
    #[must_use]
    pub fn rows_summary(&self) -> String {
        if self.rows.len() > MAX_ROWS_LISTED {
            return format!("{} rows", self.rows.len());
        }
        self.rows
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Compares one frame's records.
///
/// Pairing: **positional** while both lists are the same length, which is the normal case and the
/// strictest. When the lengths differ, the records are grouped by `(background, index)` and zipped
/// inside each group, with the leftovers counted as unmatched — a deterministic alignment that
/// still names the differing fields instead of reporting a bare count.
#[must_use]
pub fn compare_frame(zaxnes: &[TileRecord], fceux: &[Tile], pixels_identical: bool) -> FrameDiff {
    let left: Vec<Key> = zaxnes.iter().map(key_of_record).collect();
    let right: Vec<Key> = fceux.iter().map(key_of_tile).collect();

    let mut diff = FrameDiff {
        zaxnes: left.len(),
        fceux: right.len(),
        pixels_identical,
        ..FrameDiff::default()
    };

    diff.same_in_order = left.iter().zip(&right).filter(|(a, b)| a == b).count();
    diff.same_as_multiset = multiset_intersection(&left, &right);

    let mut rows = Vec::new();
    let mut note = |mask: u16, y: i16, rows: &mut Vec<i16>| {
        *diff.by_fields.entry(mask).or_insert(0) += 1;
        rows.push(y);
    };

    if left.len() == right.len() {
        for (a, b) in left.iter().zip(&right) {
            let mask = differing_fields(*a, *b);
            if mask != 0 {
                note(mask, a.3, &mut rows);
            }
        }
    } else {
        // Group by (background, index); inside a group the order is each engine's own.
        let mut groups: HashMap<(bool, i32), (Vec<Key>, Vec<Key>)> = HashMap::new();
        for key in &left {
            groups.entry((key.9, key.4)).or_default().0.push(*key);
        }
        for key in &right {
            groups.entry((key.9, key.4)).or_default().1.push(*key);
        }
        for (ours, theirs) in groups.values() {
            for (a, b) in ours.iter().zip(theirs) {
                let mask = differing_fields(*a, *b);
                if mask != 0 {
                    note(mask, a.3, &mut rows);
                }
            }
            if ours.len() > theirs.len() {
                diff.unmatched_zaxnes += ours.len() - theirs.len();
                for key in &ours[theirs.len()..] {
                    rows.push(key.3);
                }
            } else {
                diff.unmatched_fceux += theirs.len() - ours.len();
                for key in &theirs[ours.len()..] {
                    rows.push(key.3);
                }
            }
        }
    }

    rows.sort_unstable();
    rows.dedup();
    diff.rows = rows;
    diff
}

/// How many records appear in both lists with every field equal, counted with multiplicity.
fn multiset_intersection(left: &[Key], right: &[Key]) -> usize {
    let mut counts: HashMap<Key, i32> = HashMap::new();
    for key in left {
        *counts.entry(*key).or_insert(0) += 1;
    }
    let mut same = 0;
    for key in right {
        if let Some(slot) = counts.get_mut(key) {
            if *slot > 0 {
                *slot -= 1;
                same += 1;
            }
        }
    }
    same
}

// ------------------------------------------------------------------------------------------------
// Running one ROM
// ------------------------------------------------------------------------------------------------

/// Everything one ROM's comparison found.
#[derive(Debug, Clone, Default)]
pub struct RomDiff {
    pub frames: Vec<(u32, FrameDiff)>,
    /// The first frame of the whole run whose pictures differ, if any — what step 6 lists for the
    /// emulation-class ROMs.
    pub first_pixel_diff: Option<u32>,
    pub stopped_at: Option<String>,
}

/// Runs both engines over `image` and compares at each frame of `frames`.
///
/// The two engines are driven exactly as `golden-core` and `fceux-run` drive them — the file stem
/// as Zaxnes's name and the basename as FCEUX's, FCEUX's deterministic default RAM fill, a reset
/// after loading, no button pressed — so a difference here is the engines' and not the harness's.
fn compare_rom(
    engine: &mut Fceux,
    rom: &str,
    image: &[u8],
    frames: &[u32],
) -> Result<RomDiff, String> {
    let stem = Path::new(rom)
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let base = Path::new(rom).file_name().map_or_else(
        || rom.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );

    let mut zaxnes = NesCore::new();
    if !zaxnes.load_rom(image, &stem).success {
        return Err("Zaxnes refused the image".to_owned());
    }
    zaxnes.reset();

    engine.close_rom();
    engine
        .set_ram_init(RamInit::Default)
        .map_err(|e| e.to_string())?;
    engine.load_rom(image, &base).map_err(|e| e.to_string())?;
    engine.set_tile_capture(true);
    engine.reset().map_err(|e| e.to_string())?;

    let last = frames.iter().copied().max().unwrap_or(1);
    let mut ours = Vec::new();
    let mut theirs = Vec::new();
    let mut result = RomDiff::default();

    for frame in 1..=last {
        zaxnes.run_frame(0);
        if let Err(error) = engine.set_input(0, 0).and_then(|()| engine.run_frame()) {
            result.stopped_at = Some(format!("frame {frame}: {error}"));
            break;
        }

        zaxnes.pixel_frame_indices_into(&mut ours);
        if engine.frame_indices_into(&mut theirs).is_err() {
            result.stopped_at = Some(format!("frame {frame}: FCEUX gave no picture"));
            break;
        }
        let pixels_identical = ours == theirs;
        if !pixels_identical && result.first_pixel_diff.is_none() {
            result.first_pixel_diff = Some(frame);
        }

        if !frames.contains(&frame) {
            continue;
        }

        let capture = engine.take_tiles().unwrap_or_default();
        result.frames.push((
            frame,
            compare_frame(zaxnes.tile_records(), &capture.records, pixels_identical),
        ));
    }

    Ok(result)
}

/// The JSON row of one ROM.
fn row_for(engine: &mut Fceux, rom: &str, image: &[u8], options: &Options) -> Value {
    let mut row = Map::new();
    row.insert("rom".into(), json!(rom));
    row.insert("rom_sha256".into(), json!(hex(&Sha256::digest(image))));
    row.insert(
        "mapper".into(),
        InesHeader::try_parse(image).map_or(Value::Null, |header| json!(header.mapper)),
    );
    row.insert("profile".into(), json!(profile_for(rom)));

    match compare_rom(engine, rom, image, &options.frames) {
        Err(error) => {
            row.insert("load_ok".into(), json!(false));
            row.insert("load_error".into(), json!(error));
        }
        Ok(result) => {
            row.insert("load_ok".into(), json!(true));
            row.insert("first_pixel_diff".into(), json!(result.first_pixel_diff));
            if let Some(stopped) = result.stopped_at {
                row.insert("stopped_at".into(), json!(stopped));
            }
            let mut frames = Map::new();
            for (frame, diff) in &result.frames {
                frames.insert(
                    frame.to_string(),
                    json!({
                        "zaxnes": diff.zaxnes,
                        "fceux": diff.fceux,
                        "same_in_order": diff.same_in_order,
                        "same_as_multiset": diff.same_as_multiset,
                        "pixels_identical": diff.pixels_identical,
                        "class": diff.class(),
                        "fields": diff.fields_summary(),
                        "rows": diff.rows,
                        "unmatched_zaxnes": diff.unmatched_zaxnes,
                        "unmatched_fceux": diff.unmatched_fceux,
                    }),
                );
            }
            row.insert("frames".into(), Value::Object(frames));
        }
    }
    Value::Object(row)
}

// ------------------------------------------------------------------------------------------------
// The command
// ------------------------------------------------------------------------------------------------

/// The command line, parsed, run and printed. Its own function for the same reason
/// `fceux_run::command` is one: `main`'s `run` is at clippy's line budget.
///
/// # Errors
///
/// Whatever [`Options::parse`] or [`run`] returns.
pub fn command(args: &[String]) -> Result<(), HarnessError> {
    let options = Options::parse(args)?;
    print!("{}", run(&options)?);
    Ok(())
}

/// Runs the comparison and writes the rows, then prints the table.
///
/// # Errors
///
/// [`HarnessError`] when the ROMs cannot be walked, a file cannot be read or the output written,
/// or the FCEUX engine cannot be created.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let root = Path::new(&options.roms);
    let rows = if root.is_dir() && !options.in_process {
        rows_isolated(options, root)?
    } else {
        rows_in_process(options, root)?
    };

    let out = Path::new(&options.out);
    let mut writer = BufWriter::new(fs::File::create(out)?);
    for row in &rows {
        serde_json::to_writer(&mut writer, row).map_err(std::io::Error::from)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;

    Ok(report(&rows, options))
}

/// One child process per ROM, so that a ROM FCEUX cannot survive costs one row instead of the
/// whole sweep (lot FCEUX-1 §7: `other/oam3.nes` corrupts the heap and then *hangs* rather than
/// dying, so the child needs a deadline as well as isolation).
fn rows_isolated(options: &Options, roms: &Path) -> Result<Vec<Value>, HarnessError> {
    let mut files = Vec::new();
    collect_roms(roms, &mut files)?;
    files.sort_by_key(|file| relative_name(roms, file));

    let executable = std::env::current_exe()?;
    let scratch = Path::new(&options.out).with_extension("one.jsonl");
    let frames: Vec<String> = options.frames.iter().map(ToString::to_string).collect();
    let budget = Duration::from_secs(
        120 + u64::from(options.frames.iter().copied().max().unwrap_or(60)) / 5,
    );

    let mut out_rows = Vec::new();
    for file in &files {
        let name = relative_name(roms, file);
        if options.profiles_only && profile_for(&name).is_none() {
            continue;
        }

        let status = Command::new(&executable)
            .arg("fceux-tiles")
            .arg(file)
            .arg(&scratch)
            .arg("--frames")
            .arg(frames.join(","))
            .arg("--in-process")
            .stdout(std::process::Stdio::null())
            .spawn()
            .and_then(|child| crate::fceux_run::wait_or_kill(child, budget));

        let row = match (status, fs::read_to_string(&scratch)) {
            (Ok(status), Ok(text)) if status.success() => text
                .lines()
                .find(|line| !line.trim().is_empty())
                .and_then(|line| serde_json::from_str::<Value>(line).ok())
                .map(|mut row| {
                    if let Value::Object(fields) = &mut row {
                        fields.insert("rom".into(), json!(name));
                        fields.insert("profile".into(), json!(profile_for(&name)));
                    }
                    row
                }),
            _ => None,
        };

        out_rows.push(row.unwrap_or_else(|| {
            json!({
                "rom": name,
                "profile": profile_for(&name),
                "load_ok": false,
                "engine_crashed": true,
                "load_error": "the FCEUX engine took its process down or stopped answering",
            })
        }));
    }

    let _ = fs::remove_file(&scratch);
    Ok(out_rows)
}

fn rows_in_process(options: &Options, roms: &Path) -> Result<Vec<Value>, HarnessError> {
    let (root, mut files) = if roms.is_dir() {
        let mut found = Vec::new();
        collect_roms(roms, &mut found)?;
        (roms.to_path_buf(), found)
    } else {
        let root = roms.parent().unwrap_or(Path::new(".")).to_path_buf();
        (root, vec![roms.to_path_buf()])
    };
    files.sort_by_key(|file| relative_name(&root, file));

    let mut engine = Fceux::new().map_err(|error| HarnessError::Usage(error.to_string()))?;
    let mut out_rows = Vec::new();
    for file in &files {
        let name = relative_name(&root, file);
        if options.profiles_only && profile_for(&name).is_none() {
            continue;
        }
        let image = fs::read(file)?;
        out_rows.push(row_for(&mut engine, &name, &image, options));
    }
    Ok(out_rows)
}

/// The printed table: one line per ROM and frame that is not identical, then the counts per class.
fn report(rows: &[Value], options: &Options) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<52} {:>6} {:>6} {:>7} {:>7} {:>6} {:<10}",
        "ROM", "frame", "zaxnes", "fceux", "same", "px=", "class"
    );

    let mut classes: HashMap<&str, usize> = HashMap::new();
    let mut roms_all_identical = 0;
    let mut roms_total = 0;
    let mut crashed = 0;
    let mut refused = 0;
    let mut emulation_first: Vec<(String, Value)> = Vec::new();

    for row in rows {
        let name = row.get("rom").and_then(Value::as_str).unwrap_or("?");
        if row.get("engine_crashed").is_some() {
            crashed += 1;
            let _ = writeln!(
                out,
                "{name:<52} {:>6} the FCEUX engine did not come back",
                "-"
            );
            continue;
        }
        if row.get("load_ok") != Some(&Value::Bool(true)) {
            refused += 1;
            let _ = writeln!(
                out,
                "{name:<52} {:>6} refused: {}",
                "-",
                row.get("load_error").and_then(Value::as_str).unwrap_or("?")
            );
            continue;
        }
        roms_total += 1;
        let all_identical = rom_lines(&mut out, row, options, &mut classes);
        if all_identical {
            roms_all_identical += 1;
        } else if let Some(first) = row.get("first_pixel_diff").filter(|v| !v.is_null()) {
            emulation_first.push((name.to_owned(), first.clone()));
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{roms_all_identical} of {roms_total} ROMs identical at every compared frame\
         {}{}",
        if crashed == 0 {
            String::new()
        } else {
            format!("; {crashed} took the engine down")
        },
        if refused == 0 {
            String::new()
        } else {
            format!("; {refused} refused")
        }
    );
    let mut names: Vec<&&str> = classes.keys().collect();
    names.sort_unstable();
    for class in names {
        let _ = writeln!(out, "  {class}: {}", classes[*class]);
    }
    if !emulation_first.is_empty() {
        let _ = writeln!(
            out,
            "\nFirst frame whose pictures differ, for the ROMs that are not identical:"
        );
        for (name, frame) in &emulation_first {
            let _ = writeln!(out, "  {name}: {frame}");
        }
    }
    out
}

/// Writes one line per compared frame of `row` that is not identical, counting the classes as it
/// goes; returns whether every compared frame was identical.
fn rom_lines(
    out: &mut String,
    row: &Value,
    options: &Options,
    classes: &mut HashMap<&'static str, usize>,
) -> bool {
    let name = row.get("rom").and_then(Value::as_str).unwrap_or("?");
    let empty = Map::new();
    let frames = row
        .get("frames")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let mut all_identical = true;
    for frame in &options.frames {
        let Some(diff) = frames.get(&frame.to_string()) else {
            continue;
        };
        let class = diff.get("class").and_then(Value::as_str).unwrap_or("?");
        *classes.entry(class_name(class)).or_insert(0) += 1;
        if class == "identical" {
            continue;
        }
        all_identical = false;
        let pixels = if diff.get("pixels_identical") == Some(&Value::Bool(true)) {
            "yes"
        } else {
            "no"
        };
        let detail = if options.detail {
            format!(" @y {}", list(diff.get("rows")))
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "{name:<52} {frame:>6} {:>6} {:>6} {:>7} {pixels:>6} {class:<10} {}{detail}",
            number(diff, "zaxnes"),
            number(diff, "fceux"),
            number(diff, "same_in_order"),
            diff.get("fields").and_then(Value::as_str).unwrap_or(""),
        );
    }
    all_identical
}

/// A JSON array as `a,b,c`.
fn list(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map_or_else(String::new, |items| {
            items
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        })
}

/// `class` padded so the summary sorts in the order the classes matter in.
fn class_name(class: &str) -> &'static str {
    match class {
        "identical" => "1 identical",
        "capture" => "2 capture (ours to fix)",
        _ => "3 emulation (FCEUX vs Zaxnes)",
    }
}

fn number(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profile_map_takes_the_longest_prefix() {
        assert_eq!(
            profile_for("Super Mario Bros. 3 [USA][!!].nes"),
            Some("Super Mario Bros. 3")
        );
        assert_eq!(
            profile_for("Super Mario Bros. [World][!!].nes"),
            Some("Super Mario Bros.")
        );
        assert_eq!(profile_for("Mega Man 3 [USA][!!].nes"), Some("Mega Man 3"));
        assert_eq!(profile_for("Mega Man [USA][!!].nes"), Some("Mega Man 1"));
        assert_eq!(
            profile_for("Rockman [Japan][En][!!].nes"),
            Some("Mega Man 1")
        );
        assert_eq!(
            profile_for("Rockman 3-DWnS [Japan][!!].nes"),
            Some("Mega Man 3")
        );
        assert_eq!(
            profile_for("Akumajou Dracula [Japan][!!].nes"),
            Some("Castlevania")
        );
        assert_eq!(
            profile_for("Chou-WS-M [Japan][!!].nes"),
            Some("Blaster Master")
        );
        // No profile in the catalogue for these.
        assert_eq!(profile_for("Mega Man 4 [USA][!!].nes"), None);
        assert_eq!(profile_for("Metal Force [Korea][Unl][!].nes"), None);
    }

    #[test]
    fn a_frame_with_the_same_records_is_identical() {
        let record = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, true);
        let tile = Tile {
            plane0: 1,
            plane1: 2,
            x: 3,
            y: 4,
            index: 5,
            mirror_h: false,
            mirror_v: false,
            hidden: false,
            palette: 1,
            background: true,
        };
        let diff = compare_frame(&[record], &[tile], true);
        assert_eq!(diff.class(), "identical");
        assert_eq!(diff.same_in_order, 1);
        assert_eq!(diff.same_as_multiset, 1);
        assert!(diff.by_fields.is_empty());
    }

    #[test]
    fn a_differing_field_is_named_with_its_row() {
        let record = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, true);
        let tile = Tile {
            plane0: 1,
            plane1: 99,
            x: 3,
            y: 4,
            index: 5,
            mirror_h: false,
            mirror_v: false,
            hidden: false,
            palette: 2,
            background: true,
        };
        let diff = compare_frame(&[record], &[tile], true);
        assert_eq!(diff.class(), "capture");
        assert_eq!(diff.same_in_order, 0);
        assert_eq!(diff.fields_summary(), "plane1+palette:1");
        assert_eq!(diff.rows, vec![4]);
    }

    #[test]
    fn differing_pictures_make_the_frame_an_emulation_difference() {
        let record = TileRecord::new(1, 2, 3, 4, 5, false, false, false, 1, true);
        let diff = compare_frame(&[record], &[], false);
        assert_eq!(diff.class(), "emulation");
        assert_eq!(diff.unmatched_zaxnes, 1);
    }
}
