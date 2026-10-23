//! `meshes --profile <file|name> [options]`: loads a profile, feeds it frames and exports the
//! geometry the 3D engine produces as Wavefront OBJ plus a JSON instance list (SPEC-04B
//! REQ-E3D-640, REQ-E3D-641; REQ-ARCH-031). Port of `MeshExport.cs`.
//!
//! No ROM and no window are involved: the frames are either synthetic — the tiles of the
//! profile's own patterns laid out over the background grid, so that the recognition of
//! SPEC-04 matches them — or read from a frame dump, a small text format documented in
//! [`USAGE`].
//!
//! The scene frame of REQ-E3D-400 is left handed. By default the OBJ is written right handed
//! (`z' = −z`, winding reversed) so that any viewer shows it the right way round;
//! `--handedness left` writes the raw scene coordinates instead.

mod frames;
mod json;
mod obj;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::settings::RenderAlgorithm;
use zax_profiles::format;

use crate::HarnessError;

pub use frames::{read_frame_dump, synthetic_frames, DumpFrame};

/// The command line of the command.
pub const USAGE: &str = "\
zax-harness meshes --profile <file|name> [options]

  --profile <p>     a built-in profile of $ZAXNES_PROFILES/profiles (name or path),
                    or any .3dn / .bytes file
  --out <dir>       output directory (default: mesh-export next to the profile name)
  --algorithm <a>   global mesh algorithm: greedy (default), marching, multicolor
  --frames <file>   a frame dump to replay instead of the synthetic frames
  --frame-count <n> number of synthetic frames (default 1)
  --max-patterns <n> patterns used by the synthetic frames (default: as many as fit)
  --handedness <h>  right (default) or left; see the remarks
  --local           also export each 3D object's tile meshes in their local frame

Output, in <dir>:
  summary.json          profile, per-geometry counts, mesh totals, build time
  frame<NNNN>.obj       the drawn tiles of that frame, placed in the scene
  frame<NNNN>.json      the tiles to draw of REQ-E3D-641, one entry per tile
  objects.obj           with --local, every 3D object's tile meshes, one group each

Frame dump format (one directive per line, # starts a comment):
  palette <64 hex chars>                      the 32 PPU palette bytes
  bg <x> <y> <pal> <low16> <high16> <fx> <fy>  a background tile
  sp <x> <y> <pal> <low16> <high16> <fx> <fy> <hidden>  a sprite tile
  end                                         closes the frame and starts the next";

/// The parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Two flags of the command line, kept apart.
pub struct Options {
    /// The profile to load.
    pub profile: String,
    /// The output directory.
    pub output: PathBuf,
    /// The global mesh algorithm (REQ-E3D-500).
    pub algorithm: RenderAlgorithm,
    /// A frame dump to replay, or `None` for synthetic frames.
    pub frames: Option<PathBuf>,
    /// Number of synthetic frames.
    pub frame_count: usize,
    /// Patterns the synthetic frames may use; 0 means "as many as fit".
    pub max_patterns: usize,
    /// True to write the OBJ in a right-handed frame.
    pub right_handed: bool,
    /// True to also export the local tile meshes of every 3D object.
    pub local: bool,
}

impl Options {
    /// Parses the arguments after the command name.
    ///
    /// # Errors
    /// [`HarnessError::Usage`] when an argument is missing, unknown or malformed.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let usage = |message: String| HarnessError::Usage(format!("{message}\n\n{USAGE}"));
        let mut profile = None;
        let mut options = Self {
            profile: String::new(),
            output: PathBuf::new(),
            algorithm: RenderAlgorithm::Greedy,
            frames: None,
            frame_count: 1,
            max_patterns: 0,
            right_handed: true,
            local: false,
        };
        let mut output = None;

        let mut rest = arguments.iter();
        while let Some(argument) = rest.next() {
            let Some(key) = argument.strip_prefix("--") else {
                return Err(usage(format!("unexpected argument: {argument}")));
            };

            if key == "local" {
                options.local = true;
                continue;
            }

            let value = rest
                .next()
                .ok_or_else(|| usage(format!("--{key} needs a value")))?;
            let number = |value: &str| {
                value
                    .parse::<i64>()
                    .map_err(|_| usage(format!("--{key}: {value} is not a number")))
            };
            match key {
                "profile" => profile = Some(value.clone()),
                "out" => output = Some(PathBuf::from(value)),
                "algorithm" => {
                    options.algorithm = match value.to_ascii_lowercase().as_str() {
                        "greedy" => RenderAlgorithm::Greedy,
                        "marching" => RenderAlgorithm::Marching,
                        "multicolor" | "multicolour" => RenderAlgorithm::GreedyMulticolor,
                        _ => return Err(usage(format!("unknown algorithm: {value}"))),
                    };
                }
                "frames" => options.frames = Some(PathBuf::from(value)),
                "frame-count" => {
                    options.frame_count = usize::try_from(number(value)?.max(1))
                        .map_err(|_| usage(format!("--{key}: {value} is too large")))?;
                }
                "max-patterns" => {
                    options.max_patterns = usize::try_from(number(value)?.max(0))
                        .map_err(|_| usage(format!("--{key}: {value} is too large")))?;
                }
                "handedness" => {
                    options.right_handed = match value.to_ascii_lowercase().as_str() {
                        "right" => true,
                        "left" => false,
                        _ => return Err(usage(format!("unknown handedness: {value}"))),
                    };
                }
                _ => return Err(usage(format!("unknown option: --{key}"))),
            }
        }

        let profile = profile.ok_or_else(|| usage("--profile is required".to_owned()))?;
        options.output = output.unwrap_or_else(|| {
            let stem = Path::new(&profile).file_stem().map_or_else(
                || profile.clone(),
                |stem| stem.to_string_lossy().into_owned(),
            );
            Path::new("mesh-export").join(stem)
        });
        options.profile = profile;
        Ok(options)
    }
}

/// Runs the command and returns the summary it wrote.
///
/// # Errors
/// [`HarnessError`] when the profile is missing or refused, a frame dump does not parse or a
/// file cannot be written.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let path = resolve_profile(&options.profile)
        .ok_or_else(|| HarnessError::Profile(format!("profile not found: {}", options.profile)))?;
    let name = path.display().to_string();
    let profile = format::load(&fs::read(&path)?, None, Some(&name))
        .map_err(|error| HarnessError::Profile(format!("{name}: {error}")))?
        .profile;

    fs::create_dir_all(&options.output)?;

    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine.settings_mut().render_algorithm = options.algorithm;

    let watch = Instant::now();
    engine.load_profile(&profile, true)?;
    let load_milliseconds = watch.elapsed().as_secs_f64() * 1000.0;

    let frames = match &options.frames {
        Some(dump) => read_frame_dump(&fs::read_to_string(dump)?)?,
        None => synthetic_frames(&engine, options.frame_count, options.max_patterns),
    };

    let mut tiles = 0;
    for (n, frame) in frames.iter().enumerate() {
        let counter =
            i32::try_from(n).map_err(|_| HarnessError::Format("too many frames".to_owned()))?;
        frame.convert(&mut engine, counter);
        engine.publish();
        let list = engine.build_draw_list_of(FrameRole::Converting).clone();

        let stem = options.output.join(format!("frame{n:04}"));
        fs::write(
            stem.with_extension("obj"),
            obj::scene(&list, options.right_handed)?,
        )?;
        let text = json::frame(&list, engine.converting_frame(), n)?;
        fs::write(stem.with_extension("json"), text)?;
        tiles += list.tiles().len();
    }

    if options.local {
        fs::write(
            options.output.join("objects.obj"),
            obj::objects(&engine, options.right_handed)?,
        )?;
    }

    let file_name = path
        .file_name()
        .map_or_else(|| name.clone(), |file| file.to_string_lossy().into_owned());
    let summary = json::summary(&json::SummaryInput {
        profile_name: &file_name,
        profile: &profile,
        engine: &engine,
        options,
        load_milliseconds,
        frames: frames.len(),
        tiles,
    })?;
    fs::write(options.output.join("summary.json"), &summary)?;
    Ok(summary)
}

/// Finds a profile: a path as given, or a file of `$ZAXNES_PROFILES/profiles` by name, with or
/// without its extension.
fn resolve_profile(profile: &str) -> Option<PathBuf> {
    let given = PathBuf::from(profile);
    if given.is_file() {
        return Some(given);
    }

    let directory = PathBuf::from(env::var_os("ZAXNES_PROFILES")?).join("profiles");
    [
        profile.to_owned(),
        format!("{profile}.bytes"),
        format!("{profile}.3dn"),
    ]
    .into_iter()
    .map(|candidate| directory.join(candidate))
    .find(|candidate| candidate.is_file())
}

/// A float with at most `places` decimals and no trailing zero, as the C# custom format
/// `0.####` writes it.
pub(crate) fn decimals(value: f32, places: usize) -> String {
    let text = format!("{value:.places$}");
    if !text.contains('.') {
        return text;
    }

    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}
