//! `fceux-compare <zaxnes.jsonl> <fceux.jsonl> [options]`: puts a `golden-core` run and an
//! [`crate::fceux_run`] run side by side, ROM by ROM.
//!
//! **This is information, not a pass/fail gate.** Two independent emulators disagreeing on a
//! pixel is the expected case, not a defect: the question the table answers is *where* and *how
//! often*, so that a game emulated badly by one engine can be given the other.
//!
//! What is compared, and what is not:
//!
//! * `pixel_sha256` at the chosen checkpoints — directly comparable, because both engines hash
//!   the same 256×240 image of NES colour indices (0 to 63 a pixel).
//! * `blargg_status` and `blargg_text` — the test ROM's own verdict, which is the most meaningful
//!   row of all: it is the ROM saying whether the engine passed, not us saying the two agree.
//! * `region`, `load_ok`, `mapper` — whether the two engines even accepted the ROM.
//! * **not** the audio digests. Both are hashed the same way, but the two APUs filter and resample
//!   differently by design, so they would differ on every ROM and the column would carry no
//!   information. The sample *counts* are compared instead, since those must agree if the two are
//!   running at the same rate for the same number of frames.
//! * **not** `tiles_sha256`: FCEUX has no tile records (step 6 of lot FCEUX-1).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::HarnessError;

pub const USAGE: &str = "zax-harness fceux-compare <zaxnes.jsonl> <fceux.jsonl> [options]
    --frames <a,b,c>   checkpoints to compare the pictures at (default 60,300,1200)
    --all              list every ROM, not only the ones that differ
    Puts a golden-core run and an fceux-run side by side. Information, not a gate.";

/// The checkpoints compared when none are named: the three the lot asks for.
const DEFAULT_CHECKPOINTS: [&str; 3] = ["60", "300", "1200"];

#[derive(Debug, Clone)]
pub struct Options {
    pub zaxnes: PathBuf,
    pub fceux: PathBuf,
    pub checkpoints: Vec<String>,
    pub all: bool,
}

impl Options {
    /// Parses the command line.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] when an argument is missing or unknown.
    pub fn parse(args: &[String]) -> Result<Self, HarnessError> {
        let mut positional = Vec::new();
        let mut checkpoints: Vec<String> = Vec::new();
        let mut all = false;

        let mut rest = args.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--all" => all = true,
                "--frames" => {
                    let list = rest
                        .next()
                        .ok_or_else(|| HarnessError::Usage(USAGE.to_owned()))?;
                    checkpoints = list
                        .split(',')
                        .map(|frame| frame.trim().to_owned())
                        .filter(|frame| !frame.is_empty())
                        .collect();
                }
                other if other.starts_with("--") => {
                    return Err(HarnessError::Usage(format!(
                        "unknown option {other}\n{USAGE}"
                    )));
                }
                other => positional.push(other.to_owned()),
            }
        }

        let [zaxnes, fceux] = positional.as_slice() else {
            return Err(HarnessError::Usage(USAGE.to_owned()));
        };

        Ok(Self {
            zaxnes: PathBuf::from(zaxnes),
            fceux: PathBuf::from(fceux),
            checkpoints: if checkpoints.is_empty() {
                DEFAULT_CHECKPOINTS.iter().map(|&f| f.to_owned()).collect()
            } else {
                checkpoints
            },
            all,
        })
    }
}

/// How one ROM came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// Both engines loaded it and the pictures match at every compared checkpoint.
    Agree,
    /// Both loaded it; the pictures differ somewhere.
    Differ,
    /// Only one engine loaded it.
    OneSided,
    /// Neither loaded it.
    NeitherLoaded,
    /// One of the two files has no row for it.
    Missing,
}

/// The subcommand: parse, run, print. One function so that `main`'s dispatch arm is one line,
/// which is what keeps that function inside its own line budget.
///
/// # Errors
///
/// Whatever [`Options::parse`] and [`run`] return.
pub fn command(arguments: &[String]) -> Result<(), HarnessError> {
    print!("{}", run(&Options::parse(arguments)?)?);
    Ok(())
}

/// Runs the comparison and returns the report.
///
/// # Errors
///
/// [`HarnessError`] when either file cannot be read or is not the expected JSON Lines.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let zaxnes = read_rows(&options.zaxnes)?;
    let fceux = read_rows(&options.fceux)?;

    let mut names: Vec<&String> = zaxnes.keys().chain(fceux.keys()).collect();
    names.sort_unstable();
    names.dedup();

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut picture_rows = Vec::new();
    let mut blargg_rows = Vec::new();
    let mut sample_mismatches = Vec::new();

    for name in &names {
        let left = zaxnes.get(*name);
        let right = fceux.get(*name);

        let verdict = match (left, right) {
            (Some(left), Some(right)) => compare_one(
                name,
                left,
                right,
                &options.checkpoints,
                &mut picture_rows,
                &mut blargg_rows,
                &mut sample_mismatches,
            ),
            _ => Verdict::Missing,
        };

        *counts.entry(label(verdict)).or_default() += 1;
    }

    let mut report = String::new();
    let _ = writeln!(
        report,
        "# FCEUX vs Zaxnes, {} ROMs, pictures at frames {}\n",
        names.len(),
        options.checkpoints.join(", ")
    );
    let _ = writeln!(
        report,
        "Zaxnes: {}\nFCEUX:  {}\n",
        options.zaxnes.display(),
        options.fceux.display()
    );

    let _ = writeln!(report, "| Outcome | ROMs |\n|---|---|");
    for (outcome, count) in &counts {
        let _ = writeln!(report, "| {outcome} | {count} |");
    }

    if !blargg_rows.is_empty() {
        let _ = writeln!(
            report,
            "\n## The test ROMs' own verdicts ({} ROMs run the blargg protocol)\n",
            blargg_rows.len()
        );
        let _ = writeln!(report, "| ROM | Zaxnes | FCEUX |\n|---|---|---|");
        for (name, left, right) in &blargg_rows {
            if options.all || left != right {
                let _ = writeln!(report, "| {name} | {left} | {right} |");
            }
        }
    }

    let shown: Vec<&(String, String)> = picture_rows.iter().collect();
    if !shown.is_empty() {
        let _ = writeln!(report, "\n## Pictures\n");
        let _ = writeln!(report, "| ROM | Checkpoints |\n|---|---|");
        for (name, detail) in shown {
            let _ = writeln!(report, "| {name} | {detail} |");
        }
    }

    if !sample_mismatches.is_empty() {
        let _ = writeln!(
            report,
            "\n## Sample counts ({} ROMs where they differ)\n\nHow many samples each engine had \
             handed over by the last checkpoint, both at 48 kHz for the same number of frames. A \
             small, *constant* shortfall on one side is its resampler holding a tail rather than a \
             wrong rate; a difference that grows with the frame count, or one that varies from ROM \
             to ROM, is worth looking into.\n",
            sample_mismatches.len()
        );
        let _ = writeln!(report, "| ROM | Zaxnes | FCEUX |\n|---|---|---|");
        for (name, left, right) in &sample_mismatches {
            let _ = writeln!(report, "| {name} | {left} | {right} |");
        }
    }

    Ok(report)
}

#[allow(clippy::too_many_arguments)] // Each is a distinct output of the one comparison.
fn compare_one(
    name: &str,
    left: &Value,
    right: &Value,
    checkpoints: &[String],
    picture_rows: &mut Vec<(String, String)>,
    blargg_rows: &mut Vec<(String, String, String)>,
    sample_mismatches: &mut Vec<(String, String, String)>,
) -> Verdict {
    let loaded = |row: &Value| row.get("load_ok") == Some(&Value::Bool(true));
    match (loaded(left), loaded(right)) {
        (false, false) => return Verdict::NeitherLoaded,
        (true, false) | (false, true) => {
            let detail = format!(
                "Zaxnes {}, FCEUX {}{}",
                if loaded(left) { "loaded" } else { "refused" },
                if loaded(right) { "loaded" } else { "refused" },
                right
                    .get("load_error")
                    .and_then(Value::as_str)
                    .map(|error| format!(" ({error})"))
                    .unwrap_or_default()
            );
            picture_rows.push((name.to_owned(), detail));
            return Verdict::OneSided;
        }
        (true, true) => {}
    }

    // The test ROM's own verdict, which outranks any digest.
    let status = |row: &Value| {
        row.get("blargg_status")
            .and_then(Value::as_u64)
            .map(|code| {
                let text = row
                    .get("blargg_text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("")
                    .trim()
                    .to_owned();
                // 0 is "passed" in blargg's protocol; 0x80 means still running.
                let verdict = match code {
                    0 => "Passed".to_owned(),
                    0x80 => "still running".to_owned(),
                    other => format!("Failed ({other})"),
                };
                if text.is_empty() {
                    verdict
                } else {
                    format!("{verdict} — {text}")
                }
            })
    };
    if let (Some(left), Some(right)) = (status(left), status(right)) {
        blargg_rows.push((name.to_owned(), left, right));
    }

    let samples = |row: &Value, frame: &str| -> Option<String> {
        row.get("audio_i16_sha256_samples")?
            .get(frame)?
            .as_str()?
            .split(':')
            .nth(1)
            .map(str::to_owned)
    };
    if let Some(last) = checkpoints.last() {
        if let (Some(left), Some(right)) = (samples(left, last), samples(right, last)) {
            if left != right {
                sample_mismatches.push((name.to_owned(), left, right));
            }
        }
    }

    let digest = |row: &Value, frame: &str| -> Option<String> {
        row.get("pixel_sha256")?
            .get(frame)?
            .as_str()
            .map(str::to_owned)
    };

    let mut agree = Vec::new();
    let mut differ = Vec::new();
    for frame in checkpoints {
        match (digest(left, frame), digest(right, frame)) {
            (Some(left), Some(right)) if left == right => agree.push(frame.clone()),
            (Some(_), Some(_)) => differ.push(frame.clone()),
            // A checkpoint beyond the frames either side was run for; not a disagreement.
            _ => {}
        }
    }

    if differ.is_empty() && !agree.is_empty() {
        Verdict::Agree
    } else if differ.is_empty() {
        Verdict::Missing
    } else {
        picture_rows.push((
            name.to_owned(),
            format!(
                "differ at {}{}",
                differ.join(", "),
                if agree.is_empty() {
                    String::new()
                } else {
                    format!("; agree at {}", agree.join(", "))
                }
            ),
        ));
        Verdict::Differ
    }
}

const fn label(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Agree => "the pictures agree at every compared checkpoint",
        Verdict::Differ => "the pictures differ somewhere",
        Verdict::OneSided => "only one engine loaded it",
        Verdict::NeitherLoaded => "neither engine loaded it",
        Verdict::Missing => "no comparable row in one of the two files",
    }
}

/// The rows of a JSON Lines file, by their `rom` field.
fn read_rows(path: &Path) -> Result<BTreeMap<String, Value>, HarnessError> {
    let text = fs::read_to_string(path)?;
    let mut rows = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let row: Value = serde_json::from_str(line).map_err(|error| {
            HarnessError::Usage(format!(
                "{}: line {} is not JSON ({error})",
                path.display(),
                number + 1
            ))
        })?;
        let Some(name) = row.get("rom").and_then(Value::as_str) else {
            return Err(HarnessError::Usage(format!(
                "{}: line {} has no \"rom\" field",
                path.display(),
                number + 1
            )));
        };
        rows.insert(name.to_owned(), row);
    }

    Ok(rows)
}
