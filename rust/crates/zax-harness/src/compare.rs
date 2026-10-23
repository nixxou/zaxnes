//! The `compare` command: how close two captures are, the measure the golden-image tests use
//! and the one to read a C# capture against a Rust one (REQ-ARCH-031). It has no counterpart in
//! the C# harness.
//!
//! Each argument is a PNG file or a folder of PNG files; for folders, every PNG of the
//! reference folder is looked up by name in the capture folder.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use zax_render::assets::png_image;

use crate::HarnessError;

/// Text of `zax-harness help` for this command.
pub const USAGE: &str =
    "zax-harness compare <reference> <capture> [--tolerance <levels>] [--min <percent>]

Compares two PNG captures, or two folders of them by file name. A pixel matches when each of its
three colour channels is within the tolerance of the reference's. The command fails when a
capture is missing, differs in size, or matches on less than the minimum share of its pixels.

  --tolerance <levels>   largest channel difference that still matches (default 2)
  --min <percent>        smallest share of matching pixels, 0 to 100 (default 99.5)";

/// The parsed command line.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// The reference file or folder.
    pub reference: PathBuf,
    /// The capture file or folder.
    pub capture: PathBuf,
    /// Largest channel difference that still matches.
    pub tolerance: u8,
    /// Smallest share of matching pixels, in percent.
    pub minimum: f64,
}

impl Options {
    /// Parses the arguments after `compare`.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] on a missing path, a bad number or an unknown option.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let usage = |message: String| HarnessError::Usage(format!("{message}\n\n{USAGE}"));
        let mut paths = Vec::new();
        let (mut tolerance, mut minimum) = (2_u8, 99.5_f64);
        let mut rest = arguments.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                key @ ("--tolerance" | "--min") => {
                    let value = rest
                        .next()
                        .ok_or_else(|| usage(format!("{key} requires a value")))?;
                    let bad = || usage(format!("{key} expects a number, got '{value}'"));
                    if key == "--tolerance" {
                        tolerance = value.parse().map_err(|_| bad())?;
                    } else {
                        minimum = value
                            .parse::<f64>()
                            .ok()
                            .filter(|percent| (0.0..=100.0).contains(percent))
                            .ok_or_else(bad)?;
                    }
                }
                option if option.starts_with("--") => {
                    return Err(usage(format!("unknown argument '{option}'")));
                }
                path => paths.push(PathBuf::from(path)),
            }
        }

        let [reference, capture] = <[PathBuf; 2]>::try_from(paths)
            .map_err(|_| usage("compare takes a reference and a capture".to_owned()))?;
        Ok(Self {
            reference,
            capture,
            tolerance,
            minimum,
        })
    }
}

/// Share of pixels, 0 to 1, whose three channels are within `tolerance` of the reference's;
/// both buffers are rows of RGBA bytes of the same size.
#[must_use]
#[allow(clippy::cast_precision_loss)] // Pixel counts are far below 2^52.
pub fn matching_share(capture: &[u8], reference: &[u8], tolerance: u8) -> f64 {
    let close = capture
        .chunks_exact(4)
        .zip(reference.chunks_exact(4))
        .filter(|(a, b)| (0..3).all(|channel| a[channel].abs_diff(b[channel]) <= tolerance))
        .count();
    close as f64 / (reference.len() / 4).max(1) as f64
}

/// Compares the captures and returns the report, one line per picture.
///
/// # Errors
///
/// [`HarnessError::Io`] or [`HarnessError::Png`] when a file cannot be read, and
/// [`HarnessError::Mismatch`] (the report as message) when a capture is missing, has another
/// size or matches on less than the minimum share.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let pairs = if options.reference.is_dir() {
        let mut names: Vec<_> = fs::read_dir(&options.reference)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<_, _>>()?;
        names.retain(|path| path.extension().is_some_and(|ext| ext == "png"));
        names.sort();
        names
            .into_iter()
            .filter_map(|path| {
                let capture = options.capture.join(path.file_name()?);
                Some((path, capture))
            })
            .collect()
    } else {
        vec![(options.reference.clone(), options.capture.clone())]
    };

    let mut report = String::new();
    let mut failures = 0_usize;
    for (reference, capture) in &pairs {
        let name = reference
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        match share(reference, capture, options.tolerance)? {
            Share::Missing => {
                failures += 1;
                writeln!(report, "{name:<28} MISSING {}", capture.display())?;
            }
            Share::SizeDiffers => {
                failures += 1;
                writeln!(report, "{name:<28} SIZE DIFFERS")?;
            }
            Share::Matching(share) => {
                let percent = share * 100.0;
                let failed = percent < options.minimum;
                failures += usize::from(failed);
                writeln!(
                    report,
                    "{name:<28} {percent:>8.3} % within {} levels{}",
                    options.tolerance,
                    if failed { "  FAIL" } else { "" }
                )?;
            }
        }
    }

    let summary = format!(
        "compare: {} picture(s), {failures} off (tolerance {}, minimum {} %)",
        pairs.len(),
        options.tolerance,
        options.minimum
    );
    if failures > 0 || pairs.is_empty() {
        return Err(HarnessError::Mismatch(format!("{report}{summary}")));
    }
    Ok(format!("{report}{summary}"))
}

enum Share {
    Missing,
    SizeDiffers,
    Matching(f64),
}

fn share(reference: &Path, capture: &Path, tolerance: u8) -> Result<Share, HarnessError> {
    if !capture.exists() {
        return Ok(Share::Missing);
    }

    let reference = png_image::load(reference)?;
    let capture = png_image::load(capture)?;
    if (reference.width, reference.height) != (capture.width, capture.height) {
        return Ok(Share::SizeDiffers);
    }
    Ok(Share::Matching(matching_share(
        &capture.pixels,
        &reference.pixels,
        tolerance,
    )))
}
