//! What the command line asks of the application (`AppOptions.cs`): the record the application
//! consumes, and the parsing of the arguments.

use std::str::FromStr;

use zax_platform::pacing::MAXIMUM_VSYNC;

/// Frames a smoke test renders when it is not told how many.
pub const DEFAULT_SMOKE_TEST_FRAMES: u32 = 60;

/// The usage text: what `--help` prints, and what follows a usage error. It is the text of the
/// C# program, and a test compares the two.
pub const USAGE: &str = r#"Zaxnes - NES emulator with 3D rendering

Usage:
  Zax.App --rom <path> [--profile <title-or-file>] [options]
  Zax.App --list-profiles
  Zax.App --import-3dsen <folder-or-bundle>

Options:
  --rom <path>            The .nes ROM to play. Without it only the empty scene shows.
  --profile <name>        Built-in profile title, or the path of a profile file.
                          Defaults to the "2D Mode" entry (SPEC-06 REQ-SCN-055).
  --assets <dir>          Optional folder of your own asset files (profiles.json,
                          cubemaps, meshes, textures, audio, fonts). Nothing is
                          bundled; without it the app uses what the code generates.
  --font <file>           Interface font: a file, or a name in the system font folder.
  --builtin-font          Use the built-in ImGui font (no accents) instead of a
                          system font.
  --userdata <dir>        Where settings, save states and battery RAM go.
                          Default: %LOCALAPPDATA%\Zaxnes, or userdata\ in portable mode.
  --list-profiles         Print the built-in catalogue and exit.
  --import-3dsen <path>   Import the profiles of your own 3dSen installation into your
                          profile folder, print the summary and exit. Give the folder
                          holding 3dSen.exe, or the profile bundle itself. Nothing is
                          downloaded and the installation is only read (AMD-030).
  --size <w>x<h>          Window size in pixels (default 1280x720).
  --no-audio              Do not open the audio device.

Settings overrides (this run only; without them the saved settings are used):
  --fov <degrees>         Field of view, 45 to 95 (setting default 70).
  --no-shadows            Start with the shadows off.
  --vsync [0|1|2]         Synchronise buffer swaps: 0 off, 1 every refresh,
                          2 every other refresh. Bare, it means 1.
  --fps <60|90|120>       Target frame rate when vsync is off (setting default 90).
  --no-render-skip        Redraw the scene even without a new emulated frame.
  --skybox <name>         default, color, gradient or fantasy (REQ-SCN-049).
  --smoke-test            Render frames without waiting for input, then exit with 0.
  --smoke-frames <count>  Frames to render in smoke-test mode (default 60).
  --smoke-emulated <n>    Stop the smoke run after n emulated frames (60 = 1 s NTSC).
  --smoke-shot <file.png> Write the last frame of the smoke run to that PNG.
  --ui-shots <dir>        Capture one PNG per interface screen, then exit.
  -h, --help              Show this help and exit.

Keyboard (player 1, SPEC-08 section 4.2):
  Arrows    directions         X / Z       A / B
  Space     Select             Enter       Start
  F5 / F6   rotate left/right  F7 / F8     rotate up/down
  F9 / F10  zoom in/out        Mouse drag  orbit, wheel zooms
  Esc       pause / resume (hold it 0.3 s to reset the view)
  , / .     rollback / fast forward
  Select + stick down / up     quick save / quick load

Menu shortcuts (AMD-010 REQ-UI-204; see spec/changes/Q-030.md for Ctrl+Fn):
  P         pause / resume     Ctrl+R      reset
  Ctrl+Fn   load state n       Shift+Fn    save state n
  F11       2D / 3D mode       Alt+Enter   full screen
  Ctrl+Q    quit               Ctrl+,      options
"#;

/// The Rust-only options, which the C# program does not have. They are kept out of [`USAGE`] so
/// that the test comparing it with the C# text stays meaningful; `--help` prints both.
pub const RUST_ONLY_USAGE: &str = "
Rust-only options:
  --handheld              Gamepad-only interface for handheld consoles (also ZAXNES_HANDHELD=1,
                          and automatic on aarch64 Linux without an X11 or Wayland display).
  --roms <dir>            Folder of .nes files the handheld ROM list shows
                          (default: the roms folder of the user data).
";

/// A command line that cannot be read. The messages are those of the C# program.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OptionsError {
    /// An option that takes a value was the last argument.
    #[error("{option} requires a value.")]
    MissingValue {
        /// The option.
        option: &'static str,
    },
    /// An argument that no option names.
    #[error("Unknown argument '{argument}'.")]
    UnknownArgument {
        /// The argument.
        argument: String,
    },
    /// A window size that is not `<width>x<height>`.
    #[error("--size expects <width>x<height>, got '{value}'.")]
    Size {
        /// The value given.
        value: String,
    },
    /// A value that is not a number.
    #[error("{option} expects a number, got '{value}'.")]
    Number {
        /// The option.
        option: &'static str,
        /// The value given.
        value: String,
    },
    /// A value that is not a whole, non-negative number.
    #[error("{option} expects a whole, non-negative number, got '{value}'.")]
    Index {
        /// The option.
        option: &'static str,
        /// The value given.
        value: String,
    },
    /// A frame count that is not a positive integer.
    #[error("a frame count must be a positive integer, got '{value}'.")]
    FrameCount {
        /// The value given.
        value: String,
    },
    /// A `--vsync` value outside 0, 1 and 2.
    #[error("--vsync expects 0, 1 or 2, got '{value}'.")]
    VSync {
        /// The value given.
        value: String,
    },
    /// A skybox name that is not one of the four.
    #[error("--skybox expects default, color, gradient or fantasy.")]
    Skybox,
}

/// The options of one run.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // a record of independent command-line switches, as the C# one
pub struct AppOptions {
    /// Print the usage and exit.
    pub show_help: bool,
    /// List the profiles and exit.
    pub list_profiles: bool,
    /// The ROM to start.
    pub rom_path: Option<String>,
    /// The 3dSen installation or bundle to import profiles from.
    pub import_three_d_sen_path: Option<String>,
    /// The profile to play: a file, an identifier or a title.
    pub profile: Option<String>,
    /// The folder of neutral assets.
    pub asset_root: Option<String>,
    /// The font file of the interface.
    pub font_file: Option<String>,
    /// Use the built-in font.
    pub built_in_font: bool,
    /// The user data folder, overriding the portable and per-user choices.
    pub user_data_root: Option<String>,
    /// Window width in pixels.
    pub width: u32,
    /// Window height in pixels.
    pub height: u32,
    /// Whether the audio device is opened.
    pub audio: bool,
    /// Shadows on or off for this run.
    pub shadows: Option<bool>,
    /// Field of view for this run.
    pub fov: Option<f32>,
    /// Vertical synchronisation for this run: 0, 1 or 2.
    pub vsync: Option<i32>,
    /// Frame rate cap for this run.
    pub frame_rate: Option<i32>,
    /// Render skip on or off for this run.
    pub skip_render: Option<bool>,
    /// Skybox for this run.
    pub skybox: Option<i32>,
    /// Run a smoke test: render, play, exit.
    pub smoke_test: bool,
    /// Frames a smoke test renders.
    pub smoke_test_frames: u32,
    /// The locale of the operating system (`fr-FR`), which picks the language of the interface
    /// while no `ui.language` has been saved; `None` when unknown.
    pub system_locale: Option<String>,
    /// Emulated frames a smoke test must reach; 0 for the rendered-frames criterion.
    pub smoke_emulated_frames: u64,
    /// Where a smoke test writes the picture of its last frame.
    pub smoke_shot_path: Option<String>,
    /// Where the interface screenshots go.
    pub ui_shot_directory: Option<String>,
    /// The gamepad-only interface of handheld consoles (Rust-only). `--handheld` sets it; the
    /// environment and the platform can too, see [`crate::handheld_mode`].
    pub handheld: bool,
    /// The folder the handheld ROM list shows (Rust-only); `None` for the `roms` folder of the
    /// user data.
    pub roms_directory: Option<String>,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            show_help: false,
            list_profiles: false,
            rom_path: None,
            import_three_d_sen_path: None,
            profile: None,
            asset_root: None,
            font_file: None,
            built_in_font: false,
            user_data_root: None,
            width: 1280,
            height: 720,
            audio: true,
            shadows: None,
            fov: None,
            vsync: None,
            frame_rate: None,
            skip_render: None,
            skybox: None,
            smoke_test: false,
            smoke_test_frames: DEFAULT_SMOKE_TEST_FRAMES,
            system_locale: None,
            smoke_emulated_frames: 0,
            smoke_shot_path: None,
            ui_shot_directory: None,
            handheld: false,
            roms_directory: None,
        }
    }
}

impl AppOptions {
    /// Parses the command line, the arguments after the program name.
    ///
    /// # Errors
    ///
    /// An argument is unknown or malformed, or an option is missing its value.
    pub fn parse(arguments: &[&str]) -> Result<Self, OptionsError> {
        let mut options = Self::default();
        let mut index = 0;
        while let Some(&argument) = arguments.get(index) {
            match argument {
                "-h" | "--help" | "-?" | "/?" => options.show_help = true,
                "--list-profiles" => {
                    options.list_profiles = true;
                }
                "--import-3dsen" => {
                    options.import_three_d_sen_path =
                        Some(next_value(arguments, &mut index, "--import-3dsen")?.to_owned());
                }
                "--rom" => {
                    options.rom_path = Some(next_value(arguments, &mut index, "--rom")?.to_owned());
                }
                "--profile" => {
                    options.profile =
                        Some(next_value(arguments, &mut index, "--profile")?.to_owned());
                }
                "--assets" => {
                    options.asset_root =
                        Some(next_value(arguments, &mut index, "--assets")?.to_owned());
                }
                "--font" => {
                    options.font_file =
                        Some(next_value(arguments, &mut index, "--font")?.to_owned());
                }
                "--builtin-font" => {
                    options.built_in_font = true;
                }
                "--userdata" => {
                    options.user_data_root =
                        Some(next_value(arguments, &mut index, "--userdata")?.to_owned());
                }
                "--size" => {
                    let value = next_value(arguments, &mut index, "--size")?;
                    (options.width, options.height) = parse_size(value)?;
                }
                "--fov" => {
                    let value = next_value(arguments, &mut index, "--fov")?;
                    options.fov = Some(value.trim().parse().map_err(|_| OptionsError::Number {
                        option: "--fov",
                        value: value.to_owned(),
                    })?);
                }
                "--no-shadows" => {
                    options.shadows = Some(false);
                }
                "--no-audio" => {
                    options.audio = false;
                }
                // SPEC-09 gives video.vsync three values, so the flag takes an optional one;
                // bare, it means "every refresh" as it always did.
                "--vsync" => {
                    options.vsync = Some(next_vsync(arguments, &mut index)?);
                }
                "--fps" => {
                    let value = next_value(arguments, &mut index, "--fps")?;
                    options.frame_rate = Some(parse_index(value, "--fps")?);
                }
                "--no-render-skip" => {
                    options.skip_render = Some(false);
                }
                "--skybox" => {
                    let value = next_value(arguments, &mut index, "--skybox")?;
                    options.skybox = Some(skybox_of(value)?);
                }
                "--ui-shots" => {
                    options.ui_shot_directory =
                        Some(next_value(arguments, &mut index, "--ui-shots")?.to_owned());
                }
                "--smoke-test" => {
                    options.smoke_test = true;
                }
                "--smoke-emulated" => {
                    let value = next_value(arguments, &mut index, "--smoke-emulated")?;
                    options.smoke_test = true;
                    options.smoke_test_frames = u32::MAX;
                    options.smoke_emulated_frames = parse_count(value)?;
                }
                "--smoke-shot" => {
                    options.smoke_test = true;
                    options.smoke_shot_path =
                        Some(next_value(arguments, &mut index, "--smoke-shot")?.to_owned());
                }
                "--smoke-frames" => {
                    let value = next_value(arguments, &mut index, "--smoke-frames")?;
                    options.smoke_test = true;
                    options.smoke_test_frames = parse_count(value)?;
                }
                other => options.parse_rust_only(other, arguments, &mut index)?,
            }
            index += 1;
        }
        Ok(options)
    }

    /// The options only the Rust program has ([`RUST_ONLY_USAGE`]).
    fn parse_rust_only(
        &mut self,
        argument: &str,
        arguments: &[&str],
        index: &mut usize,
    ) -> Result<(), OptionsError> {
        match argument {
            "--handheld" => self.handheld = true,
            "--roms" => {
                self.roms_directory = Some(next_value(arguments, index, "--roms")?.to_owned());
            }
            unknown => {
                return Err(OptionsError::UnknownArgument {
                    argument: unknown.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Whether this run only prints something and exits: the single case where the console is
    /// written to without a window (`Program.PrintsAndExits`).
    #[must_use]
    pub const fn prints_and_exits(&self) -> bool {
        self.show_help || self.list_profiles || self.import_three_d_sen_path.is_some()
    }
}

/// The value of the option at `index`, which moves onto it.
fn next_value<'a>(
    arguments: &[&'a str],
    index: &mut usize,
    option: &'static str,
) -> Result<&'a str, OptionsError> {
    let value = arguments
        .get(*index + 1)
        .ok_or(OptionsError::MissingValue { option })?;
    *index += 1;
    Ok(value)
}

/// The value of `--vsync`: the 0, 1 or 2 that follows it, or 1 when it is used bare. A value that
/// is not a whole number is left for the next argument.
fn next_vsync(arguments: &[&str], index: &mut usize) -> Result<i32, OptionsError> {
    let Some(value) = arguments.get(*index + 1) else {
        return Ok(1);
    };
    let Ok(vsync) = value.trim().parse::<i32>() else {
        return Ok(1);
    };
    if !(0..=MAXIMUM_VSYNC).contains(&vsync) {
        return Err(OptionsError::VSync {
            value: (*value).to_owned(),
        });
    }
    *index += 1;
    Ok(vsync)
}

fn parse_size(value: &str) -> Result<(u32, u32), OptionsError> {
    let error = || OptionsError::Size {
        value: value.to_owned(),
    };
    let (width, height) = value.split_once(['x', 'X']).ok_or_else(error)?;
    if height.contains(['x', 'X']) {
        return Err(error());
    }
    match (width.trim().parse::<u32>(), height.trim().parse::<u32>()) {
        (Ok(width), Ok(height)) if width > 0 && height > 0 => Ok((width, height)),
        _ => Err(error()),
    }
}

/// A count of frames or emulated frames: a positive integer.
fn parse_count<T: FromStr + Default + PartialOrd>(value: &str) -> Result<T, OptionsError> {
    value
        .trim()
        .parse::<T>()
        .ok()
        .filter(|count| *count > T::default())
        .ok_or_else(|| OptionsError::FrameCount {
            value: value.to_owned(),
        })
}

/// A whole, non-negative number, such as a frame rate.
fn parse_index(value: &str, option: &'static str) -> Result<i32, OptionsError> {
    value
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|index| *index >= 0)
        .ok_or_else(|| OptionsError::Index {
            option,
            value: value.to_owned(),
        })
}

/// The skybox setting of `--skybox`: −1 Default, 0 Color, 1 Gradient, 2 Fantasy (REQ-SCN-049).
fn skybox_of(value: &str) -> Result<i32, OptionsError> {
    match value.to_lowercase().as_str() {
        "default" => Ok(-1),
        "color" | "colour" => Ok(0),
        "gradient" => Ok(1),
        "fantasy" => Ok(2),
        _ => Err(OptionsError::Skybox),
    }
}
