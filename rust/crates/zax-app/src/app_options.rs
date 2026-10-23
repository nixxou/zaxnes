//! What the command line asks of the application. The parsing of the arguments belongs to the
//! command-line front end; this is the record the application consumes.

/// Frames a smoke test renders when it is not told how many.
pub const DEFAULT_SMOKE_TEST_FRAMES: u32 = 60;

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
    /// Emulated frames a smoke test must reach; 0 for the rendered-frames criterion.
    pub smoke_emulated_frames: u64,
    /// Where a smoke test writes the picture of its last frame.
    pub smoke_shot_path: Option<String>,
    /// Where the interface screenshots go.
    pub ui_shot_directory: Option<String>,
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
            smoke_emulated_frames: 0,
            smoke_shot_path: None,
            ui_shot_directory: None,
        }
    }
}
