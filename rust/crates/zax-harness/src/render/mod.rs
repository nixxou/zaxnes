//! The `render` command: renders the synthetic frames offscreen through an EGL context and
//! writes PNG captures plus a statistics file (SPEC-01 REQ-ARCH-031, SPEC-06 REQ-SCN-057; port
//! of `RenderExport.cs`).
//!
//! The frames are the synthetic scenes, a ROM run headless (`--rom`) or a JSON frame
//! description (`--frames <file.json>`). `--explain` prints the draw-list report of each scene and
//! `--bench <frames>` times the rendering of each one (REQ-SCN-064).

pub mod draw_list_report;
pub mod engine;
pub mod frame_json;
pub mod input_script;
pub mod profile;
pub mod replay;
pub mod rom;
pub mod scenes;
pub mod timed_host;

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use zax_engine3d::palette::{nes_color, Rgb24};
use zax_engine3d::settings::RenderAlgorithm;
use zax_nes::ppu::SpriteLimit;
use zax_render::assets::{png_image, NeutralSceneAssets, SceneAssets};
use zax_render::camera::CameraView;
use zax_render::geometry;
use zax_render::numerics::Vector3;
use zax_render::scene::SkyboxKind;
use zax_render::statistics::RenderStatistics;

use crate::{json_text, HarnessError};
use engine::{Backend, Engine};
use input_script::InputScript;
use rom::RomRequest;
use scenes::SyntheticScene;

/// Usage text of the command.
pub const USAGE: &str = "\
zax-harness render --out <dir> [--frames synthetic[:<scene>]] [options]
zax-harness render --out <dir> --frames <file.json> --profile <title|file> [options]
zax-harness render --out <dir> --rom <file.nes> [--profile <title|file>] [--run <n>] [options]

  --out <dir>              where the PNG captures and render.json go (required)
  --frames <source>        'synthetic' (default) renders the built-in scenes,
                           'synthetic:<scene>' one of them, a path the frame a JSON file
                           describes
  --rom <file.nes>         run a real ROM headless and capture it (REQ-ARCH-031)
  --run <n>                emulated frames to run before capturing (default 120)
  --sav <file>             battery-backed PRG-RAM to import before the run (REQ-CORE-095)
  --dump-frame <file.json> write the captured frame as a '--frames' JSON file
  --widescreen <pixels>    widescreen margin the host offers on each side (default 0)
  --widescreen-y <pixels>  widescreen margin the host offers above and below (default 0)
  --poke <frame>:<addr>=<value>,...  CPU writes (hex) made before those frames
  --no-builtin-patches     restore --load-state without the built-in ROM patches
  --advanced               apply the built-in patches of an advanced widescreen mode too
  --hack-options <bits>    the built-in patch's option bits ($500A, decimal; default its own)
  --sprite-limit <mode>    kept (default), removed or smart: the 8 sprites per line of the picture
  --png-nes <file.png>     write the tile records of the last frame composed flat, as wide
                           (and as tall) as the widescreen margins make them
  --load-state <file>      restore a state block (written by --save-state) after the reset
  --movie <file.zmv>       play a movie of the application: its start state is restored, its
                           hack options set and its inputs replace --press for its frames
                           (counted from its start, as --run, --poke and --press count them);
                           its notes are printed
  --save-state <file>      write the state block of the machine after the run
  --dump-mem <file>        write CPU $0000-$07FF, $6000-$7FFF, then $5000-$5FFF (14 KiB)
                           after the run
  --trace-ram <file>       write CPU $0000-$07FF after every frame run, one block of 2 KiB each
  --press <spec>           port 1 buttons to hold, a comma separated list of
                           <first>[-<last>]:<buttons>, the frames counting from the reset and
                           the buttons being a|b|select|start|up|down|left|right joined by '+'
                           (for example 200:start,260:start,400-700:right)
  --profile <title|file>   profile file, or title of an entry of <assets>/profiles.json
  --assets <dir>           folder holding profiles.json (profile lookup) and the cubemaps the
                           scene uses; without it the skyboxes are the procedural ones
                           (REQ-PUB-003)
  --size <w>x<h>           capture size in pixels (default 640x480)
  --fov <degrees>          field of view, 45 to 95 (default 70)
  --angles <rx>,<ry>[,<rz>]  camera setpoint angles in degrees (default 0.81,0.81,0)
  --distance <d>           camera setpoint distance (default 104 / atan(FOV/2))
  --mesh <algorithm>       greedy (default), marching or multicolor (REQ-E3D-340)
  --no-shadows             render with the shadows off
  --group-draws            reorder the opaque tiles by draw state (the handheld setting)
  --shadow-map <texels>    side of the shadow map (default 4096, AMD-034)
  --anti-alias <n>         video.anti_alias: 0 off (default), 1 MSAA 4x, 2 MSAA 8x (AMD-032)
  --skybox <name>          default, color, gradient or fantasy (REQ-SCN-049)
  --bg-color <r,g,b>       the custom background colour, 0-255 (REQ-SCN-046)
  --real-time <seconds>    real time the star twinkle uses (REQ-SCN-050)
  --sky-bodies             light the procedural sun, moon and clouds (REQ-PUB-003, Q-064)
  --backend <name>         gl (default), or vulkan or dx12 with a harness built with the
                           wgpu feature (procedural skies only)
  --bench <frames>         also time that many frames per scene and report them
  --explain                print the draw list grouped by 3D object, with the NES
                           colours each group's texels resolve to (colour fidelity)
  --list-scenes            print the built-in synthetic scene names and exit

Scenes: framing, sky, shadow, no-shadow, layer-shadow-off, nudge, alpha, hud, clip,
        backdrop, heavy.";

/// Where `render` takes its frames from when there is no ROM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameSource {
    /// The built-in scenes: one, or `None` for all of them.
    Synthetic(Option<String>),
    /// A JSON frame description (`--frames <file.json>`).
    Json(PathBuf),
}

/// The parsed command line.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Independent switches of the command line.
pub struct Options {
    /// Output directory.
    pub output: PathBuf,
    /// Where the frames come from when no ROM is given.
    pub frames: FrameSource,
    /// The ROM to run.
    pub rom: Option<PathBuf>,
    /// Emulated frames to run before the capture.
    pub run_frames: i32,
    /// The port 1 buttons to hold.
    pub press: InputScript,
    /// CPU writes made before given frames: (frame, address, value).
    pub pokes: Vec<(u32, u16, u8)>,
    /// Battery-backed PRG-RAM to import.
    pub battery: Option<PathBuf>,
    /// Where to write the captured frame as JSON.
    pub dump_frame: Option<PathBuf>,
    /// Widescreen margin offered on each side, in pixels.
    pub widescreen: i32,
    /// Widescreen margin offered above and below, in pixels.
    pub widescreen_y: i32,
    /// Whether --load-state restores with the built-in ROM patches.
    pub builtin_patches: bool,
    /// Whether the built-in patches of an advanced widescreen mode apply.
    pub advanced: bool,
    /// Where to write the composed tile records of the last frame.
    pub png_nes: Option<PathBuf>,
    /// State block restored after the reset.
    pub load_state: Option<PathBuf>,
    /// The movie to play.
    pub movie: Option<PathBuf>,
    /// The built-in patch's option bits, when given.
    pub hack_options: Option<u8>,
    /// The limit of eight sprites per line of the picture.
    pub sprite_limit: SpriteLimit,
    /// Where to write the state block after the run.
    pub save_state: Option<PathBuf>,
    /// Where to write the work RAM and the cartridge RAM after the run.
    pub dump_mem: Option<PathBuf>,
    /// `--trace-ram`: the work RAM after every frame.
    pub trace_ram: Option<PathBuf>,
    /// The profile file or title.
    pub profile: Option<String>,
    /// Asset folder for the skybox cubemaps.
    pub assets: Option<PathBuf>,
    /// Capture width.
    pub width: i32,
    /// Capture height.
    pub height: i32,
    /// Field of view in degrees.
    pub fov: f32,
    /// Camera setpoint angles.
    pub angles: Option<Vector3>,
    /// Camera setpoint distance.
    pub distance: Option<f32>,
    /// Mesh algorithm.
    pub mesh: RenderAlgorithm,
    /// Whether shadows are on.
    pub shadows: bool,
    /// Whether the opaque tiles are reordered by draw state.
    pub group_draws: bool,
    /// Shadow map side asked for.
    pub shadow_map_size: i32,
    /// `video.anti_alias`, 0 to 2.
    pub anti_alias: i32,
    /// Skybox setting.
    pub skybox: SkyboxKind,
    /// Custom background colour.
    pub background: Rgb24,
    /// Real time of the star twinkle.
    pub real_time: f32,
    /// Whether the sun, the moon and the clouds are lit.
    pub sky_bodies: bool,
    /// The renderer.
    pub backend: Backend,
    /// Frames to time per scene; 0 times nothing.
    pub bench: i32,
    /// Print the camera and the draw-list report of each scene.
    pub explain: bool,
    /// Only print the scene names.
    pub list_scenes: bool,
}

impl Options {
    /// Parses the arguments after `render`.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] on an unknown argument, a missing value or a missing `--out`.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let usage = |message: String| HarnessError::Usage(format!("{message}\n\n{USAGE}"));
        let mut output = None;
        let mut options = Self {
            output: PathBuf::new(),
            frames: FrameSource::Synthetic(None),
            rom: None,
            run_frames: 120,
            press: InputScript::default(),
            pokes: Vec::new(),
            battery: None,
            dump_frame: None,
            widescreen: 0,
            widescreen_y: 0,
            builtin_patches: true,
            advanced: false,
            png_nes: None,
            load_state: None,
            movie: None,
            hack_options: None,
            sprite_limit: SpriteLimit::Kept,
            save_state: None,
            dump_mem: None,
            trace_ram: None,
            profile: None,
            assets: None,
            width: 640,
            height: 480,
            fov: geometry::DEFAULT_FOV,
            angles: None,
            distance: None,
            mesh: RenderAlgorithm::Greedy,
            shadows: true,
            group_draws: false,
            shadow_map_size: geometry::SHADOW_MAP_SIZE,
            anti_alias: 0,
            skybox: SkyboxKind::Default,
            background: Rgb24::default(),
            real_time: 0.0,
            sky_bodies: false,
            backend: Backend::Gl,
            bench: 0,
            explain: false,
            list_scenes: false,
        };

        let mut rest = arguments.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--no-shadows" => options.shadows = false,
                "--no-builtin-patches" => options.builtin_patches = false,
                "--advanced" => options.advanced = true,
                "--group-draws" => options.group_draws = true,
                "--sky-bodies" => options.sky_bodies = true,
                "--explain" => options.explain = true,
                "--list-scenes" => options.list_scenes = true,
                "--out" => {
                    let value = rest
                        .next()
                        .ok_or_else(|| usage("--out requires a value".into()))?;
                    output = Some(PathBuf::from(value));
                }
                key => {
                    let value = rest
                        .next()
                        .ok_or_else(|| usage(format!("{key} requires a value")))?;
                    options.set(key, value).map_err(usage)?;
                }
            }
        }

        if !options.list_scenes {
            options.output = output.ok_or_else(|| usage("--out <dir> is required.".to_owned()))?;
        }
        Ok(options)
    }

    /// Applies one `key value` option; the error is the message to print.
    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let number = |value: &str| {
            value
                .parse::<f32>()
                .map_err(|_| format!("{key} expects a number, got '{value}'"))
        };
        let integer = |value: &str| {
            value
                .parse::<i32>()
                .map_err(|_| format!("{key} expects an integer, got '{value}'"))
        };
        match key {
            "--frames" => self.frames = parse_frames(value),
            "--rom" => self.rom = Some(PathBuf::from(value)),
            "--run" => self.run_frames = integer(value)?,
            "--press" => {
                self.press = InputScript::parse(value).map_err(|error| error.to_string())?;
            }
            "--poke" => {
                for item in value.split(',').filter(|item| !item.is_empty()) {
                    let parse = || -> Option<(u32, u16, u8)> {
                        let (frame, rest) = item.split_once(':')?;
                        let (address, byte) = rest.split_once('=')?;
                        Some((
                            frame.parse().ok()?,
                            u16::from_str_radix(address.trim_start_matches('$'), 16).ok()?,
                            u8::from_str_radix(byte.trim_start_matches('$'), 16).ok()?,
                        ))
                    };
                    self.pokes
                        .push(parse().ok_or_else(|| format!("--poke: bad entry '{item}'"))?);
                }
            }
            "--sav" => self.battery = Some(PathBuf::from(value)),
            "--dump-frame" => self.dump_frame = Some(PathBuf::from(value)),
            "--widescreen" => self.widescreen = integer(value)?,
            "--widescreen-y" => self.widescreen_y = integer(value)?,
            "--png-nes" => self.png_nes = Some(PathBuf::from(value)),
            "--load-state" => self.load_state = Some(PathBuf::from(value)),
            "--movie" => self.movie = Some(PathBuf::from(value)),
            "--hack-options" => {
                self.hack_options = Some(
                    u8::try_from(integer(value)?)
                        .map_err(|_| "--hack-options: 0 to 255".to_owned())?,
                );
            }
            "--sprite-limit" => {
                self.sprite_limit = match value {
                    "kept" => SpriteLimit::Kept,
                    "removed" => SpriteLimit::Removed,
                    "smart" => SpriteLimit::Smart,
                    _ => {
                        return Err(format!(
                            "--sprite-limit: kept, removed or smart, got '{value}'"
                        ))
                    }
                };
            }
            "--save-state" => self.save_state = Some(PathBuf::from(value)),
            "--dump-mem" => self.dump_mem = Some(PathBuf::from(value)),
            "--trace-ram" => self.trace_ram = Some(PathBuf::from(value)),
            "--profile" => self.profile = Some(value.to_owned()),
            "--assets" => self.assets = Some(PathBuf::from(value)),
            "--size" => (self.width, self.height) = parse_size(value)?,
            "--fov" => self.fov = number(value)?,
            "--angles" => {
                let parts = value
                    .split(',')
                    .map(number)
                    .collect::<Result<Vec<_>, _>>()?;
                self.angles = Some(match parts[..] {
                    [rx, ry] => Vector3::new(rx, ry, 0.0),
                    [rx, ry, rz] => Vector3::new(rx, ry, rz),
                    _ => return Err(format!("--angles expects <rx>,<ry>[,<rz>], got '{value}'")),
                });
            }
            "--distance" => self.distance = Some(number(value)?),
            "--mesh" => self.mesh = parse_mesh(value)?,
            "--shadow-map" => self.shadow_map_size = integer(value)?,
            "--anti-alias" => {
                self.anti_alias = integer(value)?;
                if !(0..=2).contains(&self.anti_alias) {
                    return Err("--anti-alias expects 0, 1 or 2.".to_owned());
                }
            }
            "--skybox" => self.skybox = parse_skybox(value)?,
            "--bg-color" => self.background = parse_color(value)?,
            "--real-time" => self.real_time = number(value)?,
            "--bench" => self.bench = integer(value)?,
            "--backend" => self.backend = Backend::parse(value)?,
            _ => return Err(format!("unknown argument '{key}'")),
        }
        Ok(())
    }
}

/// As the C# does: any value starting with `synthetic` (case ignored) is the synthetic frames,
/// all of them or, after a colon, the scene named; anything else is the path of a JSON frame
/// description.
fn parse_frames(value: &str) -> FrameSource {
    let synthetic = value
        .get(.."synthetic".len())
        .is_some_and(|head| head.eq_ignore_ascii_case("synthetic"));
    if !synthetic {
        return FrameSource::Json(PathBuf::from(value));
    }

    FrameSource::Synthetic(value.split_once(':').map(|(_, scene)| scene.to_owned()))
}

fn parse_size(value: &str) -> Result<(i32, i32), String> {
    value
        .split_once(['x', 'X'])
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .filter(|&(w, h): &(i32, i32)| w > 0 && h > 0)
        .ok_or_else(|| format!("--size expects <width>x<height>, got '{value}'"))
}

fn parse_mesh(value: &str) -> Result<RenderAlgorithm, String> {
    match value.to_ascii_lowercase().as_str() {
        "greedy" => Ok(RenderAlgorithm::Greedy),
        "marching" => Ok(RenderAlgorithm::Marching),
        "multicolor" => Ok(RenderAlgorithm::GreedyMulticolor),
        _ => Err("--mesh expects greedy, marching or multicolor.".to_owned()),
    }
}

fn parse_skybox(value: &str) -> Result<SkyboxKind, String> {
    match value.to_ascii_lowercase().as_str() {
        "default" => Ok(SkyboxKind::Default),
        "color" => Ok(SkyboxKind::Color),
        "gradient" => Ok(SkyboxKind::Gradient),
        "fantasy" => Ok(SkyboxKind::Fantasy),
        _ => Err("--skybox expects default, color, gradient or fantasy.".to_owned()),
    }
}

fn parse_color(value: &str) -> Result<Rgb24, String> {
    let error = || format!("--bg-color expects <r>,<g>,<b>, got '{value}'");
    let parts: Vec<u8> = value
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|_| error())?;
    match parts[..] {
        [r, g, b] => Ok(Rgb24::new(r, g, b)),
        _ => Err(error()),
    }
}

/// Pixel statistics of a capture, the kind of measurement the golden-image tests rely on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureStatistics {
    /// Number of distinct RGB values.
    pub distinct_colors: usize,
    /// Pixels equal to the colour of the top-left corner.
    pub background_pixels: usize,
    /// Mean of `0.299 R + 0.587 G + 0.114 B` over the capture, 0 to 1.
    pub mean_luma: f64,
}

impl CaptureStatistics {
    /// Measures a capture given as rows of RGBA bytes.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Pixel counts are far below 2^52.
    pub fn measure(pixels: &[u8]) -> Self {
        let packed = |pixel: &[u8]| {
            (i32::from(pixel[0]) << 16) | (i32::from(pixel[1]) << 8) | i32::from(pixel[2])
        };
        let corner = pixels.get(..4).map(packed);
        let mut colors = HashSet::new();
        let mut background = 0;
        let mut luma = 0.0;
        let mut count = 0;
        for pixel in pixels.chunks_exact(4) {
            let value = packed(pixel);
            colors.insert(value);
            if Some(value) == corner {
                background += 1;
            }
            luma += ((0.299 * f64::from(pixel[0]))
                + (0.587 * f64::from(pixel[1]))
                + (0.114 * f64::from(pixel[2])))
                / 255.0;
            count += 1;
        }

        Self {
            distinct_colors: colors.len(),
            background_pixels: background,
            mean_luma: if count == 0 {
                0.0
            } else {
                luma / f64::from(count)
            },
        }
    }
}

/// Runs the command and returns the text it prints.
///
/// # Errors
///
/// [`HarnessError`] when a scene cannot be built, no GL context can be created, the renderer
/// fails or a file cannot be written.
pub fn run(options: &Options) -> Result<String, HarnessError> {
    if options.list_scenes {
        return Ok(scenes::NAMES.join("\n"));
    }

    fs::create_dir_all(&options.output)?;
    let mut built = build_scenes(options)?;
    for scene in &mut built {
        apply(options, scene);
    }

    // AMD-024 REQ-PUB-001: the scene assets are only the folder --assets names, as in the
    // application. Without it the renderer produces the procedural skyboxes of REQ-PUB-003.
    let assets = NeutralSceneAssets::try_open(options.assets.as_deref())
        .map(|assets| Box::new(assets) as Box<dyn SceneAssets>);
    let mut engine = Engine::open(
        options.backend,
        options.width,
        options.height,
        assets,
        options.group_draws,
    )?;

    let mut report = Vec::with_capacity(built.len());
    let mut text = String::new();
    for scene in &built {
        engine.render(&scene.scene, &scene.frame())?;
        let pixels = engine.read_pixels();
        let file = options.output.join(format!("{}.png", scene.name));
        write_png(&file, options, &pixels)?;
        if scene.nes_picture.len() >= NES_PIXELS {
            write_nes_picture(
                &options.output.join(format!("{}-2d.png", scene.name)),
                &scene.nes_picture,
                scene.nes_picture_height,
            )?;
        }

        if options.explain {
            text.push_str(&explain(scene, engine.last_view()));
        }

        let statistics = CaptureStatistics::measure(&pixels);
        let measured = if options.bench > 0 {
            Some(bench(&mut engine, scene, options.bench)?)
        } else {
            None
        };
        let counts = engine.statistics();
        report.push(describe(scene, counts, &statistics, measured, options));

        let timing = measured.map_or_else(String::new, |time| {
            format!(
                "  {:.2} ms median, {:.2} ms p99, {} GL draws",
                time.median_ms, time.p99_ms, counts.batches
            )
        });

        let msaa = if counts.anti_alias_samples > 1 {
            format!("  MSAA {}x", counts.anti_alias_samples)
        } else {
            String::new()
        };

        // AMD-034 §2: the granted size is only worth printing when it is not the one asked
        // for, which is how a hardware fallback gets reported.
        let shadow = if counts.shadow_map_size == counts.requested_shadow_map_size {
            String::new()
        } else {
            format!(
                "  shadow map {} (asked {})",
                counts.shadow_map_size, counts.requested_shadow_map_size
            )
        };
        writeln!(
            text,
            "{:<18} {:>6} colours  {:>5} opaque  {:>4} alpha  {:>4} HUD  {:>5} casters{msaa}{shadow}{timing}",
            scene.name,
            statistics.distinct_colors,
            counts.opaque_tiles,
            counts.transparent_tiles,
            counts.hud_tiles,
            counts.shadow_casters
        )?;
    }

    fs::write(
        options.output.join("render.json"),
        format!("[\n  {}\n]\n", report.join(",\n  ")),
    )?;
    write!(
        text,
        "render: {} capture(s) of {}x{} written to {}",
        built.len(),
        options.width,
        options.height,
        options.output.display()
    )?;
    Ok(text)
}

/// Pixels of the picture the core produces, 256 x 240.
const NES_PIXELS: usize = 256 * 240;

/// The scenes to render: the ROM run, the JSON frame, or the synthetic scenes.
fn build_scenes(options: &Options) -> Result<Vec<SyntheticScene>, HarnessError> {
    let profile = || profile::load(options.profile.as_deref(), options.assets.as_deref());
    if let Some(rom) = &options.rom {
        let frames = u32::try_from(options.run_frames.max(0)).unwrap_or(1);
        return Ok(vec![rom::capture(&RomRequest {
            rom,
            frames,
            profile: &profile()?,
            mesh: options.mesh,
            input: &options.press,
            pokes: &options.pokes,
            battery: options.battery.as_deref(),
            dump_frame: options.dump_frame.as_deref(),
            widescreen: options.widescreen,
            widescreen_y: options.widescreen_y,
            builtin_patches: options.builtin_patches,
            advanced: options.advanced,
            png_nes: options.png_nes.as_deref(),
            load_state: options.load_state.as_deref(),
            movie: options.movie.as_deref(),
            hack_options: options.hack_options,
            sprite_limit: options.sprite_limit,
            save_state: options.save_state.as_deref(),
            dump_mem: options.dump_mem.as_deref(),
            trace_ram: options.trace_ram.as_deref(),
        })?]);
    }

    match &options.frames {
        FrameSource::Json(path) => Ok(vec![replay::from_json(path, &profile()?, options.mesh)?]),
        FrameSource::Synthetic(scene) => {
            let names: Vec<&str> = match scene {
                Some(name) => vec![name.as_str()],
                None => scenes::NAMES.to_vec(),
            };
            names
                .into_iter()
                .map(|name| scenes::build(name, options.mesh))
                .collect()
        }
    }
}

/// The options that change the scene state, as the application applies them.
fn apply(options: &Options, scene: &mut SyntheticScene) {
    let state = &mut scene.scene;
    state.settings.fov = options.fov;
    state.camera.set_fov(options.fov);
    if let Some(angles) = options.angles {
        state.camera.current_mut().angles = angles;
    }
    if let Some(distance) = options.distance {
        state.camera.current_mut().set_distance(distance);
    }
    state.camera.snap_to_setpoints();
    state.settings.skybox = options.skybox;
    state.settings.custom_background_color = options.background;
    state.real_time = options.real_time;

    // Q-064: the three bodies exist and are parametric, but REQ-SCN-050 leaves them off; this
    // is what a capture turns them on with.
    if options.sky_bodies {
        let bodies = &mut state.settings.sky_bodies;
        bodies.set_sun_gain(1.0);
        bodies.set_moon_gain(1.0);
        bodies.set_cloud_gain(0.6);
        bodies.set_cloud_coverage(0.3);
    }

    if !options.shadows {
        state.settings.shadows = false;
        scene.engine.settings_mut().shadow = false;
    }

    // AMD-032 §2: video.anti_alias as the application applies it.
    state
        .settings
        .set_anti_alias_samples(geometry::anti_alias_samples(options.anti_alias));
    state.settings.set_shadow_map_size(options.shadow_map_size);
}

#[allow(clippy::cast_sign_loss)] // The size is positive (checked when parsing).
fn write_png(file: &Path, options: &Options, pixels: &[u8]) -> Result<(), HarnessError> {
    let encoded = png_image::encode_rgba(options.width as u32, options.height as u32, pixels)?;
    fs::write(file, encoded)?;
    Ok(())
}

/// Writes the flat NES picture, one colour index per pixel (REQ-CORE-048).
/// The 2D view: the picture as wide and tall as its widescreen margins make it, `rows` rows.
fn write_nes_picture(file: &Path, indices: &[u8], rows: usize) -> Result<(), HarnessError> {
    // A JSON frame carries no picture height: the NES picture.
    let rows = if rows == 0 { 240 } else { rows };
    let width = indices.len() / rows;
    let pixels: Vec<u8> = indices
        .iter()
        .take(width * rows)
        .flat_map(|&index| {
            let color = nes_color(usize::from(index));
            [color.r, color.g, color.b, 255]
        })
        .collect();
    let (Ok(width), Ok(rows)) = (u32::try_from(width), u32::try_from(rows)) else {
        return Ok(());
    };
    fs::write(file, png_image::encode_rgba(width, rows, &pixels)?)?;
    Ok(())
}

/// One entry of `render.json`, in the format of the C# harness.
fn describe(
    scene: &SyntheticScene,
    render: &RenderStatistics,
    capture: &CaptureStatistics,
    timing: Option<FrameTimes>,
    options: &Options,
) -> String {
    let bench = timing.map_or_else(String::new, |time| {
        format!(
            ", \"medianFrameMs\": {:.3}, \"p99FrameMs\": {:.3}",
            time.median_ms, time.p99_ms
        )
    });
    format!(
        "{{ \"scene\": \"{}\", \"description\": \"{}\", \"width\": {}, \"height\": {}, \
         \"opaqueTiles\": {}, \"transparentTiles\": {}, \"hudTiles\": {}, \"shadowCasters\": {}, \
         \"drawCalls\": {}, \"triangles\": {}, \"residentMeshes\": {}, \"antiAliasSamples\": {}, \
         \"shadowMapSize\": {}, \"requestedShadowMapSize\": {}, \"distinctColors\": {}, \
         \"backgroundPixels\": {}, \"meanLuma\": {:.4}{bench} }}",
        json_text::escape(&scene.name),
        json_text::escape(&scene.description),
        options.width,
        options.height,
        render.opaque_tiles,
        render.transparent_tiles,
        render.hud_tiles,
        render.shadow_casters,
        render.draw_calls,
        render.triangles,
        render.resident_meshes,
        render.anti_alias_samples,
        render.shadow_map_size,
        render.requested_shadow_map_size,
        capture.distinct_colors,
        capture.background_pixels,
        capture.mean_luma
    )
}

/// The frame times `--bench` measured, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameTimes {
    /// Median frame time.
    pub median_ms: f64,
    /// 99th percentile frame time.
    pub p99_ms: f64,
}

/// Warm-up frames rendered before the timing, so that the mesh and texture uploads do not count.
const WARM_UP_FRAMES: usize = 8;

/// Renders `count` frames and returns the median and the 99th percentile frame time, the
/// measurement REQ-SCN-064 asks for.
fn bench(
    engine: &mut Engine,
    scene: &SyntheticScene,
    count: i32,
) -> Result<FrameTimes, HarnessError> {
    let count = usize::try_from(count).unwrap_or(0).max(1);
    for _ in 0..WARM_UP_FRAMES {
        engine.render(&scene.scene, &scene.frame())?;
    }
    engine.finish();

    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        let start = Instant::now();
        engine.render(&scene.scene, &scene.frame())?;
        engine.finish();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    samples.sort_by(f64::total_cmp);
    Ok(FrameTimes {
        median_ms: samples[count / 2],
        p99_ms: samples[(count - 1).min(count * 99 / 100)],
    })
}

/// The camera lines and the draw-list report of one scene (`--explain`).
fn explain(scene: &SyntheticScene, camera: &CameraView) -> String {
    let state = &scene.scene;
    let angles = state.camera.current().angles;
    let clipping = state.clipping;
    let rect = state.visible_rectangle();
    let mut text = format!(
        "camera: fov {:.1}  d {:.2}  angles ({:.2}, {:.2}, {:.2})\n        \
         eye ({:.2}, {:.2}, {:.2})  T' ({:.2}, {:.2})  P* ({:.2}, {:.2})\n\
         clipping: ({}, {}, {}, {})  R = [{}, {}] x [{}, {}]\n",
        camera.fov,
        camera.distance,
        angles.x,
        angles.y,
        angles.z,
        camera.eye.x,
        camera.eye.y,
        camera.eye.z,
        camera.target.x,
        camera.target.y,
        camera.aim_point.x,
        camera.aim_point.y,
        clipping.x,
        clipping.y,
        clipping.z,
        clipping.w,
        rect.min_x,
        rect.max_x,
        rect.min_y,
        rect.max_y
    );
    if let Some(frame) = scene.engine.published_frame() {
        text.push_str(&draw_list_report::describe(
            &scene.draw_list,
            frame,
            scene.engine.patterns(),
        ));
    }
    text
}
