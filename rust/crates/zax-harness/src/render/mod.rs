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
use zax_engine3d::space_bend::SpaceBend;
use zax_nes::ppu::SpriteLimit;
use zax_render::assets::{png_image, NeutralSceneAssets, SceneAssets};
use zax_render::camera::CameraView;
use zax_render::geometry;
use zax_render::numerics::Vector3;
use zax_render::scene::SkyboxKind;
use zax_render::statistics::RenderStatistics;
use zax_render::style::{AmbientOcclusion, StyleSettings};
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

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
  --dump-every <n>         with --dump-frame, also write every n-th frame to <file>_<frame>.json
  --widescreen <pixels>    widescreen margin the host offers on each side (default 0)
  --widescreen-y <pixels>  widescreen margin the host offers above and below (default 0)
  --poke <frame>:<addr>=<value>,...  CPU writes (hex) made before those frames
  --no-builtin-patches     restore --load-state without the built-in ROM patches
  --advanced               apply the built-in patches of an advanced widescreen mode too
  --hack-options <bits>    the built-in patch's option bits ($500A, decimal; default its own)
  --sprite-limit <mode>    kept (default), removed or smart: the 8 sprites per line of the picture
  --run-ahead <n>          run-ahead frames, 0 to 3 (default 0, so captures stay comparable)
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
  --anti-alias <n>         video.anti_alias: 0 off, 1 MSAA 4x (default), 2 MSAA 8x (AMD-032),
                           3 SMAA on the scene's depth (AMD-060)
  --atlas-budget <texels>  ceiling of the texels the object textures may occupy; beyond it a
                           pattern's tiles are not drawn and one line is logged per pattern
                           (AMD-051 REQ-E3D-644). No ceiling by default
  --delay-mesh <frames>    emulated frames the meshes of each 3D object are held back, the test
                           case of AMD-051 REQ-E3D-646. 0 (default) holds nothing back
  --skybox <name>          default, color, gradient or fantasy (REQ-SCN-049)
  --bg-color <r,g,b>       the custom background colour, 0-255 (REQ-SCN-046)
  --no-nes-bg-color        scene.use_nes_bg_color false, so --bg-color shows behind the scene
                           (the conditioning AMD-050 REQ-SCN-076 needs to be observable)
  --bend <lr>,<tb>,<arc>[,<centre>]  the curvature of AMD-043 REQ-SCN-066, in degrees; the
                           centre is 0 for the screen centre (default), 1 for its bottom edge,
                           or any fraction between (RO-5 part 2: the setting offers quarters).
                           All 0 by default, which is the capture of before the amendment.
                           Any angle is taken, on or off the grid the settings offer (RO-1
                           point 7: 5 degrees between -30 and 30, 30 beyond)
  --no-bend-2d             keep the 2D-mode rectangle flat whatever --bend says (REQ-SCN-074)
  --no-backdrop-lighting   light the backdrop with the diffuse term of REQ-SCN-028 instead of
                           the model of AMD-049 REQ-SCN-075 — the look of before the amendment
  --style <name>           render.visual_style of AMD-055 REQ-SCN-078: classic (the default) or
                           enhanced. The numbers 0, 4 and the accepted 1 are taken too
  --style-param <id>=<v>[,<id>=<v>...]  the Enhanced parameters of REQ-SCN-079, by the Lua
                           identifiers the amendment makes normative: spl.ao, ao.strength,
                           ao.radius, ao.thickness, ao.quality, spl.pixel_shadow,
                           spl.voxel_grid, spl.reflections, ssr.shine, ssr.clarity, ssr.length,
                           ssr.quality. Each value goes through the setting's own normalisation,
                           as a write from the options window or from Lua does. The curvature
                           parameters of AMD-043 are --bend's, not these
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
    /// `--dump-every`: with `--dump-frame`, also a frame every this many frames.
    pub dump_every: u32,
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
    /// Run-ahead frames (AMD-041 REQ-LOOP-064), 0 by default: the suites of frames, tiles and
    /// conversion results are only comparable at N equal (REQ-ARCH-012 amended), so a plain
    /// `render` is the campaign's own capture whatever the application's default is.
    pub run_ahead: u8,
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
    /// `video.anti_alias`, 0 to 3 — the fourth level being the depth-detecting SMAA of AMD-060.
    pub anti_alias: i32,
    /// Skybox setting.
    pub skybox: SkyboxKind,
    /// Custom background colour.
    pub background: Rgb24,
    /// Real time of the star twinkle.
    pub real_time: f32,
    /// Whether the sun, the moon and the clouds are lit.
    pub sky_bodies: bool,
    /// The curvature of AMD-043 REQ-SCN-066: `--bend <lr>,<tb>,<arc>[,<centre>]`, in degrees,
    /// all four 0 by default so that a capture taken without them is the one of before the
    /// amendment.
    pub bend: SpaceBend,
    /// `--no-bend-2d`: AMD-046 REQ-SCN-074 turned off, so the 2D-mode rectangle stays flat
    /// whatever `--bend` says. On by default, as the setting is.
    pub bend_two_dimensional_mode: bool,
    /// `--no-nes-bg-color`: `scene.use_nes_bg_color` false, so the custom background colour
    /// shows behind the scene. The conditioning AMD-050 REQ-SCN-076's criteria need — with the
    /// default settings the transparency of the 2D rectangle is not observable at all.
    pub use_nes_background_color: bool,
    /// `--no-backdrop-lighting`: `render.backdrop_lighting` false, so the backdrop of REQ-SCN-048
    /// is lit with the plain diffuse term of REQ-SCN-028 instead of the almost-matt model of
    /// AMD-049 REQ-SCN-075. On by default, as the desktop setting is; this is the flag that
    /// reproduces the look of before the amendment, which the goldens were captured in.
    pub backdrop_lighting: bool,
    /// `--style <name>`: `render.visual_style` of AMD-055 REQ-SCN-078, as the setting holds it.
    /// **0 (Classic) by default**, which is the setting's own default and the capture of before
    /// the amendment.
    pub style: i32,
    /// `--style-param <id>=<value>,…`: the writes to the twelve parameters of REQ-SCN-079, in the
    /// order they were given, each already resolved to the setting its Lua identifier stands for.
    /// Empty by default, so every parameter keeps the default of the amendment.
    ///
    /// They are kept as writes rather than as a built value so that each one goes through the
    /// setting's **own** normalisation (AMD-040 §3) when the style is built — the very path a
    /// write from the options window or from a Lua script takes.
    pub style_params: Vec<(zax_settings::catalogue::SettingId, f64)>,
    /// `--atlas-budget <texels>`: the ceiling AMD-051 REQ-E3D-644 is verified against, over and
    /// above whatever the driver refuses. `None` — the default — leaves the driver as the only
    /// limit, so a capture taken without it is the capture of before the amendment.
    pub atlas_budget: Option<u64>,
    /// `--delay-mesh <frames>`: emulated frames the meshes of every 3D object are held back,
    /// the test case of AMD-051 REQ-E3D-646 ("un objet dont la construction est retardée de
    /// 10 frames par le harnais"). 0 holds nothing back.
    pub delay_mesh: i32,
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
            dump_every: 0,
            widescreen: 0,
            widescreen_y: 0,
            builtin_patches: true,
            advanced: false,
            png_nes: None,
            load_state: None,
            movie: None,
            hack_options: None,
            sprite_limit: SpriteLimit::Kept,
            run_ahead: 0,
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
            // Q-087 point 1: the default follows the setting's own, so the two cannot diverge
            // and a plain `render` reports the 4 samples REQ-PERS-089's criterion names.
            anti_alias: zax_settings::catalogue::descriptor(
                zax_settings::catalogue::SettingId::VideoAntiAlias,
            )
            .default_value()
            .as_int(),
            skybox: SkyboxKind::Default,
            background: Rgb24::default(),
            real_time: 0.0,
            sky_bodies: false,
            // All three default to what the application defaults to, so that a capture taken
            // without the new flags is the capture of before this lot (REQ-SCN-066: the four
            // angles are 0; REQ-SCN-074: the 2D rectangle bends; REQ-PERS-005: the NES
            // background colour is used).
            bend: SpaceBend::NONE,
            bend_two_dimensional_mode: true,
            use_nes_background_color: true,
            backdrop_lighting: true,
            // AMD-055, the same rule: Classic and the twelve defaults of REQ-SCN-079, so a
            // capture taken without the two new flags is the capture of before the amendment.
            style: zax_settings::styles::CLASSIC_STYLE,
            style_params: Vec::new(),
            // AMD-051: both default to the application's own behaviour — no budget of our own
            // and no delay — so a capture taken without them is the capture of before the
            // amendment.
            atlas_budget: None,
            delay_mesh: 0,
            backend: Backend::Gl,
            bench: 0,
            explain: false,
            list_scenes: false,
        };

        let mut rest = arguments.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--no-shadows" => options.shadows = false,
                "--no-bend-2d" => options.bend_two_dimensional_mode = false,
                "--no-nes-bg-color" => options.use_nes_background_color = false,
                "--no-backdrop-lighting" => options.backdrop_lighting = false,
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

    /// `frames` as a run-ahead (AMD-041 REQ-LOOP-064), or the message to print.
    fn run_ahead_of(frames: i32) -> Result<u8, String> {
        u8::try_from(frames)
            .ok()
            .filter(|frames| *frames <= zax_nes::NesCore::MAX_RUN_AHEAD)
            .ok_or_else(|| "--run-ahead: 0 to 3".to_owned())
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
            // AMD-043 REQ-SCN-066: the three angles in degrees and, optionally, the arc centre
            // (0 the screen centre, 1 its bottom edge, and since RO-5 part 2 any fraction
            // between). The values go through `SpaceBend::new` as they are, without the snapping
            // of the setting's domain, so that R0 can capture an angle or a pivot between two
            // rungs if it wants one.
            "--bend" => self.bend = parse_bend(value)?,
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
            "--dump-every" => {
                self.dump_every = u32::try_from(integer(value)?)
                    .map_err(|_| "--dump-every: a positive count".to_owned())?;
            }
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
            "--run-ahead" => self.run_ahead = Self::run_ahead_of(integer(value)?)?,
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
            _ => return self.set_scene(key, value),
        }
        Ok(())
    }

    /// The options of how the scene is drawn, rather than of what is played: the second half of
    /// [`Options::set`], split off it so that each stays readable.
    fn set_scene(&mut self, key: &str, value: &str) -> Result<(), String> {
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
        let unsigned = |value: &str| {
            value
                .parse::<u64>()
                .map_err(|_| format!("{key} expects a count, got '{value}'"))
        };
        match key {
            "--shadow-map" => self.shadow_map_size = integer(value)?,
            // AMD-060 REQ-SCN-085: the domain is the setting's own, which that amendment grew to
            // four rungs. The harness has to reach the fourth to capture it at all.
            "--anti-alias" => {
                self.anti_alias = integer(value)?;
                if !(0..=zax_render::geometry::SMAA_DEPTH_LEVEL).contains(&self.anti_alias) {
                    return Err("--anti-alias expects 0, 1, 2 or 3.".to_owned());
                }
            }
            // AMD-055 REQ-SCN-078 and REQ-SCN-079: the style, and its parameters by the Lua
            // identifiers the amendment makes normative.
            "--style" => self.style = parse_style(value)?,
            "--style-param" => self.style_params.extend(parse_style_params(value)?),
            // AMD-051 §3 point 8: the budget is in texels, because a pattern carries a variable
            // number of objects of variable sizes (REQ-E3D-564).
            "--atlas-budget" => self.atlas_budget = Some(unsigned(value)?),
            "--delay-mesh" => self.delay_mesh = integer(value)?,
            "--skybox" => self.skybox = parse_skybox(value)?,
            "--bg-color" => self.background = parse_color(value)?,
            "--real-time" => self.real_time = number(value)?,
            "--bench" => self.bench = integer(value)?,
            "--backend" => self.backend = Backend::parse(value)?,
            _ => return Err(format!("unknown argument '{key}'")),
        }
        Ok(())
    }

    /// The style the run renders with, built the way the application builds it: `--style` and
    /// every `--style-param` write go into a **settings model**, which validates and snaps each of
    /// them (REQ-PERS-003, AMD-040 §3 — so `ao.strength` 9 lands on 5, `spl.voxel_grid` 4 on 3 and
    /// the style 1 on 4), and the model is then read into the one value the renderer holds.
    ///
    /// Twin of `zax_app::scene_script_bridge::style_of`, which it cannot share: `zax-render` owns
    /// [`StyleSettings`] and does not depend on `zax-settings`. Going through the model rather
    /// than filling the fields is what makes a capture comparable with the application's picture —
    /// the normalisation is the same code, not a second reading of it.
    #[must_use]
    pub fn style_settings(&self) -> StyleSettings {
        let mut model = zax_settings::model::SettingsModel::new();
        model.set(SettingId::RenderVisualStyle, SettingValue::Int(self.style));
        for (id, value) in &self.style_params {
            #[allow(clippy::cast_possible_truncation)] // A rung of 0 to 5.
            model.set(*id, SettingValue::Float(*value as f32));
        }

        let level = |id: SettingId| model.get(id).as_int();
        StyleSettings {
            enhanced: StyleSettings::style_of(level(SettingId::RenderVisualStyle)),
            ao: AmbientOcclusion::of(level(SettingId::RenderStyleAo)),
            ao_strength: level(SettingId::RenderStyleAoStrength),
            ao_radius: level(SettingId::RenderStyleAoRadius),
            ao_thickness: level(SettingId::RenderStyleAoThickness),
            ao_quality: level(SettingId::RenderStyleAoQuality),
            pixel_shadow: level(SettingId::RenderStylePixelShadow) != 0,
            voxel_grid: level(SettingId::RenderStyleVoxelGrid),
            reflections: level(SettingId::RenderStyleReflections) != 0,
            ssr_shine: level(SettingId::RenderStyleSsrShine),
            ssr_clarity: level(SettingId::RenderStyleSsrClarity),
            ssr_length: level(SettingId::RenderStyleSsrLength),
            ssr_quality: level(SettingId::RenderStyleSsrQuality),
        }
    }
}

/// As the C# does: any value starting with `synthetic` (case ignored) is the synthetic frames,
/// all of them or, after a colon, the scene named; anything else is the path of a JSON frame
/// description.
/// `--bend <left-right>,<top-bottom>,<arc>[,<centre>]`: the curvature of AMD-043 REQ-SCN-066.
///
/// The angles reach `SpaceBend::new` as they are, **without** the snapping the setting's domain
/// applies, so that a capture can be taken at an angle between two rungs. Since RO-1 point 7 the
/// grid the settings offer is finer near 0 — 5 degrees between −30 and 30, 30 degrees beyond — and
/// this parser is still the one that takes anything, which is what a comparison of 0, 15 and 30
/// degrees needs. The centre is 0 for the screen centre (the default), 1 for the bottom edge of
/// the frame, and — since RO-5 part 2, which gave the setting the rungs 0, 0.25, 0.5, 0.75 and 1 —
/// any fraction between the two, snapping included none, so that a capture can be taken at a pivot
/// the menu does not offer.
fn parse_bend(value: &str) -> Result<SpaceBend, String> {
    let parts: Vec<&str> = value.split(',').collect();
    if !(3..=4).contains(&parts.len()) {
        return Err("--bend expects <left-right>,<top-bottom>,<arc>[,<centre>]".to_owned());
    }
    let angle = |index: usize| -> Result<f32, String> {
        parts[index]
            .trim()
            .parse::<f32>()
            .map_err(|_| format!("--bend: '{}' is not a number", parts[index]))
    };
    let centre = match parts.get(3).map(|centre| centre.trim()) {
        None => 0.0,
        Some(text) => {
            let parsed = text
                .parse::<f32>()
                .map_err(|_| format!("--bend centre: a number from 0 to 1, got '{text}'"))?;
            if !(0.0..=1.0).contains(&parsed) {
                return Err(format!("--bend centre: a number from 0 to 1, got '{text}'"));
            }
            parsed
        }
    };
    Ok(SpaceBend::new(angle(0)?, angle(1)?, angle(2)?, centre, 1.0))
}

/// `--style <classic|enhanced>`: the value `render.visual_style` holds (AMD-055 REQ-SCN-078).
///
/// The two names and the three numbers the requirement admits; anything else is an error rather
/// than the silent "every other value is Classic" the **setting** applies, because a misspelt
/// flag on a capture campaign must not quietly produce the Classic picture.
fn parse_style(value: &str) -> Result<i32, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "classic" | "0" => Ok(zax_settings::styles::CLASSIC_STYLE),
        "enhanced" | "4" => Ok(zax_settings::styles::ENHANCED_STYLE),
        // The identifier AMD-040 wrote, which REQ-SCN-078 accepts and reads back as 4.
        "1" => Ok(zax_settings::styles::ENHANCED_STYLE_ALIAS),
        other => Err(format!(
            "--style expects classic or enhanced (0, 4 or 1), got '{other}'"
        )),
    }
}

/// `--style-param <id>=<value>,…`: the writes to the twelve parameters of REQ-SCN-079.
///
/// The identifier is the amendment's own (`spl.ao`, `ao.strength`, …), resolved through the very
/// table the Lua methods use (`zax_settings::styles`), so the flag and
/// `Setting:SetVisualStyleParameter` cannot name a parameter differently. The **value** is kept
/// as given and normalised later, by the setting's own domain.
fn parse_style_params(value: &str) -> Result<Vec<(SettingId, f64)>, String> {
    let mut writes = Vec::new();
    for item in value.split(',').filter(|item| !item.trim().is_empty()) {
        let (name, number) = item.split_once('=').ok_or_else(|| {
            format!(
                "--style-param expects <id>=<value> items, got '{}'",
                item.trim()
            )
        })?;
        let name = name.trim();
        let setting = zax_settings::styles::parameter(zax_settings::styles::ENHANCED_STYLE, name)
            .filter(|setting| STYLE_PARAMETERS.contains(setting))
            .ok_or_else(|| {
                if zax_settings::styles::parameter(zax_settings::styles::ENHANCED_STYLE, name)
                    .is_some()
                {
                    // The four curvature parameters of AMD-043 are known identifiers that this
                    // flag cannot serve: the curvature is not part of the style (REQ-SCN-078) and
                    // reaches the scene through `SceneState::bend`.
                    format!("--style-param: '{name}' is a curvature parameter; use --bend")
                } else {
                    format!("--style-param: unknown parameter '{name}'")
                }
            })?;
        let number = number
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("--style-param {name}: '{}' is not a number", number.trim()))?;
        writes.push((setting, number));
    }
    Ok(writes)
}

/// The twelve parameters of REQ-SCN-079, which are the ones `--style-param` sets. The four
/// curvature identifiers of AMD-043, which `zax_settings::styles` also knows, are deliberately
/// absent: `--bend` is what sets those.
const STYLE_PARAMETERS: [SettingId; 12] = [
    SettingId::RenderStyleAo,
    SettingId::RenderStyleAoStrength,
    SettingId::RenderStyleAoRadius,
    SettingId::RenderStyleAoThickness,
    SettingId::RenderStyleAoQuality,
    SettingId::RenderStylePixelShadow,
    SettingId::RenderStyleVoxelGrid,
    SettingId::RenderStyleReflections,
    SettingId::RenderStyleSsrShine,
    SettingId::RenderStyleSsrClarity,
    SettingId::RenderStyleSsrLength,
    SettingId::RenderStyleSsrQuality,
];

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
    // AMD-051 REQ-E3D-644: no ceiling of our own unless `--atlas-budget` sets one.
    engine.set_texture_budget(options.atlas_budget);

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
        // AMD-051 REQ-E3D-644 / REQ-E3D-646: what the renderer left out, and why. Nothing on a
        // healthy capture, which is every capture taken without `--atlas-budget`.
        for line in engine.take_diagnostics() {
            writeln!(text, "{:<18} {line}", "")?;
        }
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
            dump_every: options.dump_every,
            widescreen: options.widescreen,
            widescreen_y: options.widescreen_y,
            builtin_patches: options.builtin_patches,
            advanced: options.advanced,
            png_nes: options.png_nes.as_deref(),
            load_state: options.load_state.as_deref(),
            movie: options.movie.as_deref(),
            hack_options: options.hack_options,
            sprite_limit: options.sprite_limit,
            run_ahead: options.run_ahead,
            save_state: options.save_state.as_deref(),
            dump_mem: options.dump_mem.as_deref(),
            trace_ram: options.trace_ram.as_deref(),
            delay_mesh: options.delay_mesh,
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

    // AMD-043 REQ-SCN-066 and AMD-046 REQ-SCN-074, and the conditioning AMD-050 REQ-SCN-076's
    // criteria need. All three are the application's own defaults unless a flag says otherwise,
    // so a capture taken without them is the capture of before this lot.
    state.bend = options.bend;
    state.bend_two_dimensional_mode = options.bend_two_dimensional_mode;
    state.use_nes_background_color = options.use_nes_background_color;
    // AMD-049 REQ-SCN-075, same rule: on unless `--no-backdrop-lighting` says otherwise.
    state.settings.backdrop_lighting = options.backdrop_lighting;
    // AMD-055 REQ-SCN-078 and REQ-SCN-079: Classic and the twelve defaults unless `--style` and
    // `--style-param` say otherwise, so a capture taken without them is the capture of before the
    // amendment — which is what point 7 of this lot's brief asks of every default capture.
    state.settings.style = options.style_settings();

    // AMD-032 §2: video.anti_alias as the application applies it.
    state.settings.set_anti_alias(options.anti_alias);
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

/// Whether the run renders in the Enhanced style (AMD-055 REQ-SCN-078): `--style enhanced`, or
/// the accepted 1.
fn state_is_enhanced(options: &Options) -> bool {
    StyleSettings::style_of(options.style)
}

/// One entry of `render.json`, in the format of the C# harness, with the run-ahead of AMD-041
/// added at the end (REQ-ARCH-012 amended) and the four keys of AMD-055 after it, when the
/// Enhanced style is selected.
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
    // AMD-055: what the Enhanced style asked of the renderer, so that a campaign capture of one
    // effect carries the three counters the lot's points 1 and 2 are read on. Written **only**
    // when Enhanced is selected, the same way `--bench` writes its two keys only when it ran: a
    // capture at the default settings produces the entry of before the amendment, byte for byte.
    let style = if state_is_enhanced(options) {
        format!(
            ", \"visualStyle\": {}, \"styleProgramsResident\": {}, \
             \"screenEffectBytes\": {}, \"screenEffectPasses\": {}",
            zax_settings::styles::canonical_style(options.style),
            render.style_programs,
            render.screen_effect_bytes,
            render.screen_effect_passes
        )
    } else {
        String::new()
    };
    // AMD-060 REQ-SCN-086, the same shape for the same reason: the filter's two counters are
    // written **only** at `video.anti_alias` = 3, so a capture at any other level -- the three
    // the goldens are taken at included -- produces the entry of before the amendment, byte for
    // byte.
    let smaa = if zax_render::geometry::smaa_depth(options.anti_alias) {
        format!(
            ", \"smaaPasses\": {}, \"smaaBytes\": {}, \"smaaProgramsResident\": {}",
            render.smaa_passes, render.smaa_bytes, render.smaa_programs
        )
    } else {
        String::new()
    };
    format!(
        "{{ \"scene\": \"{}\", \"description\": \"{}\", \"width\": {}, \"height\": {}, \
         \"opaqueTiles\": {}, \"transparentTiles\": {}, \"hudTiles\": {}, \"shadowCasters\": {}, \
         \"drawCalls\": {}, \"triangles\": {}, \"residentMeshes\": {}, \"antiAliasSamples\": {}, \
         \"shadowMapSize\": {}, \"requestedShadowMapSize\": {}, \"distinctColors\": {}, \
         \"backgroundPixels\": {}, \"meanLuma\": {:.4}, \"runAhead\": {}{bench}{style}{smaa} }}",
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
        capture.mean_luma,
        // REQ-ARCH-012 amended: a frame, tile or conversion sequence is only comparable with
        // another one at **equal N**, so N goes in the export. The key is last, so a golden
        // captured before AMD-041 still parses (`RenderScene` names the fields it compares).
        options.run_ahead
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
