//! The `render` command: renders the synthetic frames offscreen through an EGL context and
//! writes PNG captures plus a statistics file (SPEC-01 REQ-ARCH-031, SPEC-06 REQ-SCN-057; port
//! of `RenderExport.cs`).
//!
//! Only the synthetic frames are ported; the C# command also replays a ROM (`--rom`), a JSON
//! frame description (`--frames <file.json>`), prints draw-list reports (`--explain`) and
//! times frames (`--bench`).

pub mod scenes;

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use zax_engine3d::palette::Rgb24;
use zax_engine3d::settings::RenderAlgorithm;
use zax_render::assets::{png_image, NeutralSceneAssets, SceneAssets};
use zax_render::geometry;
use zax_render::numerics::Vector3;
use zax_render::offscreen::{ContextApi, OffscreenContext};
use zax_render::scene::SkyboxKind;
use zax_render::statistics::RenderStatistics;
use zax_render::SceneRenderer;

use crate::HarnessError;
use scenes::SyntheticScene;

/// Usage text of the command.
pub const USAGE: &str = "\
zax-harness render --out <dir> [--frames synthetic[:<scene>]] [options]

  --out <dir>              where the PNG captures and render.json go (required)
  --frames <source>        'synthetic' (default) renders the built-in scenes,
                           'synthetic:<scene>' one of them
  --assets <dir>           folder holding the cubemaps the skybox uses; without it the
                           skyboxes are the procedural ones (REQ-PUB-003)
  --size <w>x<h>           capture size in pixels (default 640x480)
  --fov <degrees>          field of view, 45 to 95 (default 70)
  --angles <rx>,<ry>[,<rz>]  camera setpoint angles in degrees (default 0.81,0.81,0)
  --distance <d>           camera setpoint distance (default 104 / atan(FOV/2))
  --mesh <algorithm>       greedy (default), marching or multicolor (REQ-E3D-340)
  --no-shadows             render with the shadows off
  --shadow-map <texels>    side of the shadow map (default 4096, AMD-034)
  --anti-alias <n>         video.anti_alias: 0 off (default), 1 MSAA 4x, 2 MSAA 8x (AMD-032)
  --skybox <name>          default, color, gradient or fantasy (REQ-SCN-049)
  --bg-color <r,g,b>       the custom background colour, 0-255 (REQ-SCN-046)
  --real-time <seconds>    real time the star twinkle uses (REQ-SCN-050)
  --sky-bodies             light the procedural sun, moon and clouds (REQ-PUB-003, Q-064)
  --list-scenes            print the built-in synthetic scene names and exit

Scenes: framing, sky, shadow, no-shadow, layer-shadow-off, nudge, alpha, hud, clip,
        backdrop, heavy.";

/// The parsed command line.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Independent switches of the command line.
pub struct Options {
    /// Output directory.
    pub output: PathBuf,
    /// One scene, or `None` for all of them.
    pub scene: Option<String>,
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
            scene: None,
            assets: None,
            width: 640,
            height: 480,
            fov: geometry::DEFAULT_FOV,
            angles: None,
            distance: None,
            mesh: RenderAlgorithm::Greedy,
            shadows: true,
            shadow_map_size: geometry::SHADOW_MAP_SIZE,
            anti_alias: 0,
            skybox: SkyboxKind::Default,
            background: Rgb24::default(),
            real_time: 0.0,
            sky_bodies: false,
            list_scenes: false,
        };

        let mut rest = arguments.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--no-shadows" => options.shadows = false,
                "--sky-bodies" => options.sky_bodies = true,
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
            "--frames" => self.scene = parse_frames(value)?,
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
            _ => return Err(format!("unknown argument '{key}'")),
        }
        Ok(())
    }
}

/// `synthetic` or `synthetic:<scene>`; the other frame sources are not ported.
fn parse_frames(value: &str) -> Result<Option<String>, String> {
    match value.split_once(':') {
        None if value.eq_ignore_ascii_case("synthetic") => Ok(None),
        Some((source, scene)) if source.eq_ignore_ascii_case("synthetic") => {
            Ok(Some(scene.to_owned()))
        }
        _ => Err(format!(
            "--frames '{value}': only the synthetic frames are ported"
        )),
    }
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
    let names: Vec<&str> = match &options.scene {
        Some(name) => vec![name.as_str()],
        None => scenes::NAMES.to_vec(),
    };
    let mut built = names
        .into_iter()
        .map(|name| scenes::build(name, options.mesh))
        .collect::<Result<Vec<_>, _>>()?;
    for scene in &mut built {
        apply(options, scene);
    }

    let context = OffscreenContext::new(options.width, options.height, ContextApi::DesktopCore)?;

    // AMD-024 REQ-PUB-001: the scene assets are only the folder --assets names, as in the
    // application. Without it the renderer produces the procedural skyboxes of REQ-PUB-003.
    let assets = NeutralSceneAssets::try_open(options.assets.as_deref())
        .map(|assets| Box::new(assets) as Box<dyn SceneAssets>);
    let mut renderer = SceneRenderer::new(context.gl(), assets)?;

    let mut report = Vec::with_capacity(built.len());
    let mut text = String::new();
    for scene in &built {
        renderer.render(&context.target(), &scene.scene, &scene.frame())?;
        let pixels = context.read_pixels();
        let file = options.output.join(format!("{}.png", scene.name));
        write_png(&file, options, &pixels)?;

        let statistics = CaptureStatistics::measure(&pixels);
        let counts = renderer.statistics();
        report.push(describe(scene, counts, &statistics, options));

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
            "{:<18} {:>6} colours  {:>5} opaque  {:>4} alpha  {:>4} HUD  {:>5} casters{msaa}{shadow}",
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

/// One entry of `render.json`, in the format of the C# harness.
fn describe(
    scene: &SyntheticScene,
    render: &RenderStatistics,
    capture: &CaptureStatistics,
    options: &Options,
) -> String {
    format!(
        "{{ \"scene\": \"{}\", \"description\": \"{}\", \"width\": {}, \"height\": {}, \
         \"opaqueTiles\": {}, \"transparentTiles\": {}, \"hudTiles\": {}, \"shadowCasters\": {}, \
         \"drawCalls\": {}, \"triangles\": {}, \"residentMeshes\": {}, \"antiAliasSamples\": {}, \
         \"shadowMapSize\": {}, \"requestedShadowMapSize\": {}, \"distinctColors\": {}, \
         \"backgroundPixels\": {}, \"meanLuma\": {:.4} }}",
        scene.name,
        scene.description,
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
