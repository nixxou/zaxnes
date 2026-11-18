//! The options of the handheld interface as data: a short list of rows with stepped values (no
//! slider, no text). The values are the settings of the desktop options window; the rows only
//! choose which ones a handheld needs and how they are stepped.

use zax_settings::catalogue::{self, SettingId};
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::overlay::CameraPreset;
use super::pad::PadButton;
use crate::emulation::host::hack_options;
use crate::persistence::app_settings::{AppSettings as _, AppSettingsMut as _, TvRegionSetting};
use crate::render_scaling::{DIRECT_METHOD, RENDER_TEXTURE_METHOD};
use crate::ui::{keys, shader_panel, strings, text};
use zax_settings::shader_slots::ShaderSlot;

/// The volumes the row steps through.
const VOLUME_LEVELS: [f32; 11] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

/// The frame caps the row offers, in frames per second.
pub const FRAME_CAPS: [i32; 2] = [60, 30];

/// A row of the options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionRow {
    /// The language of the interface.
    Language,
    /// The volume.
    Volume,
    /// The resolution the scene is drawn at.
    RenderScale,
    /// The shadows.
    Shadows,
    /// The table the NES colours are shown in (`video.nes_palette`).
    Palette,
    /// The picture's edges cut (`video.overscan_*`, Rust-only): opens its own screen.
    Overscan,
    /// The frame cap: 60 every refresh, 30 every other refresh of a 60 Hz screen.
    FrameCap,
    /// The frame counter.
    ShowFps,
    /// The frame profiler.
    Profiler,
    /// The camera preset.
    Camera,
    /// How many emulated frames a displayed frame may run when the game is late.
    CatchUp,
    /// The optimised Lua paths (`unsafe`), on or off.
    LuaOptimized,
    /// The experimental Lua paths (they need the optimised ones), on or off.
    LuaExperimental,
    /// The scripts beside the drawing: off, automatic (while late) or always.
    Pipeline,
    /// No conversion and no frame hooks while the 2D picture is shown, on or off.
    Pause3dIn2d,
    /// Only the shown frame of a catch-up converted, on or off.
    CatchUpLast3d,
    /// The 3D at half the refresh rate, the game at full speed, on or off.
    HalfRate3d,
    /// The TV region: automatic (from the ROM header, at the next game start), NTSC or PAL.
    Region,
    /// The gamepad bindings.
    Controls,
    /// RetroAchievements (Rust-only): opens its own screen.
    Achievements,
    /// The touch pad (Rust-only, a touch screen only): no, yes or automatic.
    TouchPad,
    /// A setting stepped through the choices of its domain, its label and its choices those of
    /// the catalogue: the touch screen's (size, vibrations, orientation).
    Choice(SettingId),
    /// A setting of the desktop's options window shown as a stepped row
    /// ([`super::advanced::ROW_SETTINGS`] names every one the console offers).
    Setting(SettingId),
    /// A slot of the slang shaders (owner request: the console mode offers them as the desktop's
    /// window does): it steps through following, nothing and the presets of the catalogue.
    Shader(ShaderSlot),
    /// The pre-filter of a slot of the slang shaders: none, then those of
    /// [`shader_panel::PRE_FILTERS`] (owner request).
    ShaderPreFilter(ShaderSlot),
    /// Sub-option `n` of the shader family of a slot (lot REWRITE-UI): its choices or its steps,
    /// as `shader_looks::sub_options` lists them for the family the slot holds.
    ShaderOption(ShaderSlot, u8),
    /// The graphics API of the shaders page: OpenGL draws none, Vulkan draws them (owner
    /// decision: the slang shaders are offered on Vulkan only). Applied at the next start.
    ShaderBackend,
    /// A row that climbs to another page of the tree (RO-3).
    Page(OptionsPage),
    /// A group title: a label and nothing else (RO-3). Up and down skip it, and it is drawn
    /// smaller and dimmed. The field is the key of its text.
    Group(&'static str),
    /// The mode of the options: simple or advanced (`ui.options_advanced`, lot REWRITE-UI).
    Mode,
    /// The performance / fidelity preset (`ui.options_preset`): stepping it writes the preset.
    Preset,
    /// The widescreen of every game by default, on or off (`video.widescreen`).
    WidescreenOn,
    /// A switch of a setting the desktop's classic window does not show (the widescreen ones).
    Switch(SettingId),
    /// The common default of a hack pack option (`hack_options::COMMON_OPTIONS`).
    HackDefault(&'static str),
    /// The page's settings back to their defaults: the chosen preset's values where it manages
    /// a setting, the setting's own default elsewhere (lot REWRITE-UI).
    Reset,
}

/// A page of the console options (RO-3, the owner's mockup): the main page climbs to the
/// *Optimisations* page and to the *Advanced options* hub, and the hub to seven pages — RO-3's six
/// and the audio one AUDIO-2 adds — two of which have a page of their own.
///
/// The same pages serve the global options and a game's, with a list of their own for each:
/// [`Self::rows`] and [`Self::game_rows`]. A game cannot set everything the console can, so its
/// pages are shorter and three of the hub's seven do not exist for it.
// Q-107 §6
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsPage {
    /// The performance switches together (owner request, before RO-3).
    Optimizations,
    /// The hub of the six pages below.
    Advanced,
    /// How the picture reaches the screen, and what it costs.
    Display,
    /// What the 3D scene looks like.
    Rendering3d,
    /// The effects the Enhanced style adds (AMD-055 REQ-SCN-079).
    EnhancedEffects,
    /// The curvature of AMD-043 REQ-SCN-066.
    Geometry,
    /// The light, the shadows and the ambient occlusion.
    LightShadows,
    /// What the console puts out, before any 3D.
    NesPicture,
    /// The camera.
    Camera,
    /// The emulation.
    Emulation,
    /// What comes out of the sound device (AUDIO-2). The volume is the main page's row; what is
    /// here is the choice of output, which the work order of AUDIO-2 asks the console to offer
    /// too. It is a page of its own rather than a row of the main page because that page is the
    /// owner's mockup, pinned row for row by
    /// `desktop_options_golden_tests::ro_3_the_handheld_main_pages_are_the_mockups`.
    Audio,
    /// The slang shaders of the four slots (owner request, the desktop's Shaders window).
    Shaders,
    /// The touch screen (Android, owner request): the pad, its size and vibrations, the
    /// orientation.
    Touch,
    /// The settings the chosen preset proposes (lot REWRITE-UI): its rows depend on the preset,
    /// so the options model gives them ([`HandheldOptionsModel::rows`]).
    PresetOptions,
    /// The widescreen and the hacks: the defaults of every game with a widescreen patch.
    Widescreen,
    /// The simple mode's pages: a few rows of the advanced ones (the window's simple tabs).
    SimpleDisplay,
    SimpleNesPicture,
    SimpleRendering3d,
    SimpleAudio,
    SimpleEmulation,
}

impl OptionsPage {
    /// Every page, for the tests that walk them all.
    pub const ALL: [Self; 20] = [
        Self::Optimizations,
        Self::Advanced,
        Self::Display,
        Self::Rendering3d,
        Self::EnhancedEffects,
        Self::Geometry,
        Self::LightShadows,
        Self::NesPicture,
        Self::Camera,
        Self::Emulation,
        Self::Audio,
        Self::Shaders,
        Self::Touch,
        Self::PresetOptions,
        Self::Widescreen,
        Self::SimpleDisplay,
        Self::SimpleNesPicture,
        Self::SimpleRendering3d,
        Self::SimpleAudio,
        Self::SimpleEmulation,
    ];

    /// The key of the page's title.
    #[must_use]
    pub const fn title_key(self) -> &'static str {
        match self {
            Self::Optimizations => keys::HANDHELD_OPTION_OPTIMIZATIONS,
            Self::Advanced => keys::HANDHELD_OPTION_ADVANCED,
            Self::Display | Self::SimpleDisplay => keys::TAB_DISPLAY,
            Self::Rendering3d | Self::SimpleRendering3d => keys::TAB_RENDER_3D,
            Self::EnhancedEffects => keys::PAGE_ENHANCED_EFFECTS,
            Self::Geometry => keys::PAGE_GEOMETRY,
            Self::LightShadows => keys::TAB_LIGHT,
            Self::NesPicture | Self::SimpleNesPicture => keys::TAB_NES_PICTURE,
            Self::Camera => keys::TAB_CAMERA,
            Self::Emulation | Self::SimpleEmulation => keys::TAB_EMULATION,
            Self::Audio | Self::SimpleAudio => keys::TAB_AUDIO,
            Self::Shaders => "shaders.title",
            Self::Touch => keys::HANDHELD_OPTION_TOUCH_SCREEN,
            Self::PresetOptions => "options.new.preset_options",
            Self::Widescreen => "options.new.tab.widescreen",
        }
    }

    /// The rows of the page in the **global** options.
    #[must_use]
    pub const fn rows(self) -> &'static [OptionRow] {
        match self {
            Self::Optimizations => &OPTIMIZATION_ROWS,
            Self::Advanced => &ADVANCED_HUB,
            Self::Display => &DISPLAY_ROWS,
            Self::Rendering3d => &RENDERING_3D_ROWS,
            Self::EnhancedEffects => &ENHANCED_EFFECTS_ROWS,
            Self::Geometry => &GEOMETRY_ROWS,
            Self::LightShadows => &LIGHT_SHADOWS_ROWS,
            Self::NesPicture => &NES_PICTURE_ROWS,
            Self::Camera => &CAMERA_ROWS,
            Self::Emulation => &EMULATION_ROWS,
            Self::Audio => &AUDIO_ROWS,
            Self::Shaders => &SHADER_ROWS,
            Self::Touch => &TOUCH_SCREEN_ROWS,
            // The model gives the chosen preset's rows.
            Self::PresetOptions => &[OptionRow::Reset],
            Self::Widescreen => &WIDESCREEN_ROWS,
            Self::SimpleDisplay => &SIMPLE_DISPLAY_ROWS,
            Self::SimpleNesPicture => &SIMPLE_NES_PICTURE_ROWS,
            Self::SimpleRendering3d => &SIMPLE_RENDERING_3D_ROWS,
            Self::SimpleAudio => &SIMPLE_AUDIO_ROWS,
            Self::SimpleEmulation => &SIMPLE_EMULATION_ROWS,
        }
    }

    /// The rows of the page in the options of one **game**; empty for a page a game has not got.
    #[must_use]
    pub const fn game_rows(self) -> &'static [OptionRow] {
        super::game_options::page_rows(self)
    }
}

/// The rows of the main page in the **simple** mode (lot REWRITE-UI): the mode, the preset and
/// what it proposes, then the window's simple tabs as pages.
pub const SIMPLE_ROWS: [OptionRow; 15] = [
    OptionRow::Mode,
    OptionRow::Preset,
    OptionRow::Page(OptionsPage::PresetOptions),
    OptionRow::Language,
    OptionRow::Volume,
    OptionRow::WidescreenOn,
    OptionRow::Region,
    OptionRow::Page(OptionsPage::SimpleDisplay),
    OptionRow::Page(OptionsPage::SimpleNesPicture),
    OptionRow::Page(OptionsPage::SimpleRendering3d),
    OptionRow::Page(OptionsPage::SimpleAudio),
    OptionRow::Page(OptionsPage::SimpleEmulation),
    OptionRow::Controls,
    OptionRow::Achievements,
    OptionRow::Reset,
];

/// [`SIMPLE_ROWS`] where a touch screen is offered (Android): the orientation and the touch
/// screen's page, as in [`TOUCH_ROWS`].
pub const SIMPLE_TOUCH_ROWS: [OptionRow; 17] = [
    OptionRow::Mode,
    OptionRow::Preset,
    OptionRow::Page(OptionsPage::PresetOptions),
    OptionRow::Language,
    OptionRow::Volume,
    OptionRow::WidescreenOn,
    OptionRow::Region,
    OptionRow::Choice(SettingId::UiOrientation),
    OptionRow::Page(OptionsPage::SimpleDisplay),
    OptionRow::Page(OptionsPage::SimpleNesPicture),
    OptionRow::Page(OptionsPage::SimpleRendering3d),
    OptionRow::Page(OptionsPage::SimpleAudio),
    OptionRow::Page(OptionsPage::SimpleEmulation),
    OptionRow::Controls,
    OptionRow::Page(OptionsPage::Touch),
    OptionRow::Achievements,
    OptionRow::Reset,
];

/// *Display* in the simple mode.
pub const SIMPLE_DISPLAY_ROWS: [OptionRow; 6] = [
    OptionRow::ShowFps,
    OptionRow::Setting(SettingId::VideoVSync),
    OptionRow::Setting(SettingId::VideoAntiAlias),
    OptionRow::Setting(SettingId::VideoSkipFrame),
    OptionRow::Page(OptionsPage::Shaders),
    OptionRow::Reset,
];

/// *NES picture* in the simple mode.
pub const SIMPLE_NES_PICTURE_ROWS: [OptionRow; 4] = [
    OptionRow::Palette,
    OptionRow::Setting(SettingId::VideoCompositeSignal),
    OptionRow::Overscan,
    OptionRow::Reset,
];

/// *3D rendering* in the simple mode.
pub const SIMPLE_RENDERING_3D_ROWS: [OptionRow; 7] = [
    OptionRow::Setting(SettingId::RenderVisualStyle),
    OptionRow::Setting(SettingId::RenderStyleReflections),
    OptionRow::Page(OptionsPage::Geometry),
    OptionRow::Setting(SettingId::LightIntensity),
    OptionRow::Shadows,
    OptionRow::Setting(SettingId::CameraFov),
    OptionRow::Reset,
];

/// *Audio* in the simple mode.
pub const SIMPLE_AUDIO_ROWS: [OptionRow; 3] = [
    OptionRow::Volume,
    OptionRow::Setting(SettingId::AudioConsoleProfile),
    OptionRow::Reset,
];

/// *Emulation* in the simple mode.
pub const SIMPLE_EMULATION_ROWS: [OptionRow; 6] = [
    OptionRow::Setting(SettingId::EmuRunAhead),
    OptionRow::Setting(SettingId::EmuSmoothScrollAhead),
    OptionRow::Setting(SettingId::EmuRainbowNetwork),
    OptionRow::Setting(SettingId::EmuExactRate),
    OptionRow::Setting(SettingId::EmuRealtimeInput),
    OptionRow::Reset,
];

/// *Widescreen & Hacks*, the advanced mode's page: the defaults of every game with a widescreen
/// patch (a game's own choices, in its hacks screen, come first).
pub const WIDESCREEN_ROWS: [OptionRow; 12] = [
    OptionRow::Choice(SettingId::VideoWidescreen),
    OptionRow::Choice(SettingId::VideoWidescreenExtra),
    OptionRow::Switch(SettingId::VideoWidescreenAdvanced),
    OptionRow::HackDefault("no_slowdown"),
    OptionRow::HackDefault("invincible"),
    OptionRow::HackDefault("level_select"),
    OptionRow::HackDefault("margin_enemies"),
    OptionRow::HackDefault("margin_freeze"),
    OptionRow::HackDefault("margin_freeze_tint"),
    OptionRow::Setting(SettingId::VideoWidescreenStrictEdges),
    OptionRow::Switch(SettingId::EmuBuiltinPatches),
    OptionRow::Reset,
];

/// The rows of the main page in the **advanced** mode, in order (RO-3: the owner's mockup).
/// "Camera view" and "Show FPS" left it for the *Advanced options* hub; the shaders came up from
/// it (owner request). Lot REWRITE-UI put the mode, the preset, the widescreen page and the
/// reset row round it.
pub const ROWS: [OptionRow; 18] = [
    OptionRow::Mode,
    OptionRow::Preset,
    OptionRow::Page(OptionsPage::PresetOptions),
    OptionRow::Language,
    OptionRow::Volume,
    OptionRow::RenderScale,
    OptionRow::Shadows,
    OptionRow::Palette,
    OptionRow::Overscan,
    OptionRow::Region,
    OptionRow::Page(OptionsPage::Widescreen),
    OptionRow::Page(OptionsPage::Shaders),
    OptionRow::Page(OptionsPage::Optimizations),
    OptionRow::Page(OptionsPage::Advanced),
    OptionRow::Controls,
    OptionRow::Achievements,
    OptionRow::Profiler,
    OptionRow::Reset,
];

/// The rows of the main page where the touch pad is offered (Android): its row after the
/// controls, which is where it was before RO-3.
pub const TOUCH_ROWS: [OptionRow; 20] = [
    OptionRow::Mode,
    OptionRow::Preset,
    OptionRow::Page(OptionsPage::PresetOptions),
    OptionRow::Language,
    OptionRow::Volume,
    OptionRow::RenderScale,
    OptionRow::Shadows,
    OptionRow::Palette,
    OptionRow::Overscan,
    OptionRow::Region,
    // Owner request: the orientation of the screen.
    OptionRow::Choice(SettingId::UiOrientation),
    OptionRow::Page(OptionsPage::Widescreen),
    OptionRow::Page(OptionsPage::Shaders),
    OptionRow::Page(OptionsPage::Optimizations),
    OptionRow::Page(OptionsPage::Advanced),
    OptionRow::Controls,
    OptionRow::Page(OptionsPage::Touch),
    OptionRow::Achievements,
    OptionRow::Profiler,
    OptionRow::Reset,
];

/// The rows of the touch screen's page (owner request): the pad, its size, its vibrations and its
/// rewind and fast-forward buttons.
pub const TOUCH_SCREEN_ROWS: [OptionRow; 6] = [
    OptionRow::TouchPad,
    OptionRow::Choice(SettingId::InputTouchScale),
    OptionRow::Choice(SettingId::InputTouchHapticDpad),
    OptionRow::Choice(SettingId::InputTouchHapticButtons),
    OptionRow::Choice(SettingId::InputTouchExtra),
    OptionRow::Reset,
];

/// The values of a setting's choices, in their order.
fn choice_values(id: SettingId) -> Vec<i32> {
    catalogue::descriptor(id)
        .domain()
        .options()
        .unwrap_or_default()
        .iter()
        .map(|option| {
            #[allow(clippy::cast_possible_truncation)] // A choice's value.
            let value = option.value as i32;
            value
        })
        .collect()
}

/// The label of the choice the setting `id` holds.
fn choice_text(id: SettingId, settings: &SettingsModel) -> String {
    let current = settings.get(id).as_int();
    catalogue::descriptor(id)
        .domain()
        .label_of(f64::from(current))
        .map_or_else(|| current.to_string(), text::label_of_option)
}

/// The values of the touch pad row (`input.touch_pad`): no, yes, automatic.
const TOUCH_PADS: [i32; 3] = [0, 1, 2];

fn touch_pad(settings: &SettingsModel) -> i32 {
    settings.get(SettingId::InputTouchPad).as_int()
}

/// The rows of the "Optimizations" page (owner request: the performance switches together). RO-3
/// moved the smooth Lua collector here from the flat advanced page, where it was the one
/// performance switch not with the others.
pub const OPTIMIZATION_ROWS: [OptionRow; 10] = [
    OptionRow::FrameCap,
    OptionRow::HalfRate3d,
    OptionRow::CatchUp,
    OptionRow::CatchUpLast3d,
    OptionRow::Pause3dIn2d,
    OptionRow::Pipeline,
    OptionRow::LuaOptimized,
    OptionRow::LuaExperimental,
    OptionRow::Setting(SettingId::EmuLuaSmoothGc),
    OptionRow::Reset,
];

/// The rows of the *Shaders* page: the four slots, as the desktop's window has them (owner
/// request). They are the same in a build without the shaders, which keeps the settings the
/// desktop shares; such a build simply draws none.
pub const SHADER_ROWS: [OptionRow; 11] = [
    OptionRow::ShaderBackend,
    OptionRow::Setting(SettingId::VideoShaderResolution),
    OptionRow::Shader(ShaderSlot::TwoD),
    OptionRow::ShaderPreFilter(ShaderSlot::TwoD),
    OptionRow::Shader(ShaderSlot::TwoDWide),
    OptionRow::ShaderPreFilter(ShaderSlot::TwoDWide),
    OptionRow::Shader(ShaderSlot::ThreeD),
    OptionRow::ShaderPreFilter(ShaderSlot::ThreeD),
    OptionRow::Shader(ShaderSlot::ThreeDWide),
    OptionRow::ShaderPreFilter(ShaderSlot::ThreeDWide),
    OptionRow::Reset,
];

/// The *Advanced options* hub: six pages where RO-2 left one flat list of 56 rows. It repeats
/// nothing the other pages offer — the palette and the shadows are the main page's, the nine
/// overscan settings the overscan screen's, the performance switches the Optimisations page's.
pub const ADVANCED_HUB: [OptionRow; 7] = [
    OptionRow::Page(OptionsPage::Display),
    OptionRow::Page(OptionsPage::Rendering3d),
    OptionRow::Page(OptionsPage::LightShadows),
    OptionRow::Page(OptionsPage::NesPicture),
    OptionRow::Page(OptionsPage::Camera),
    OptionRow::Page(OptionsPage::Emulation),
    OptionRow::Page(OptionsPage::Audio),
];

/// *Advanced options › Display*. The render scale is here **and** on the main page: the one
/// setting the mockup gives two entry points on purpose.
pub const DISPLAY_ROWS: [OptionRow; 12] = [
    OptionRow::ShowFps,
    OptionRow::Setting(SettingId::VideoVSync),
    OptionRow::Setting(SettingId::VideoFrameRate),
    OptionRow::Setting(SettingId::VideoSkipFrame),
    OptionRow::Group(keys::GROUP_RESOLUTION),
    OptionRow::Setting(SettingId::VideoResScale),
    OptionRow::Setting(SettingId::VideoDynamicResolution),
    OptionRow::Setting(SettingId::VideoRenderMethod),
    OptionRow::Setting(SettingId::VideoAntiAlias),
    OptionRow::Group(keys::GROUP_ADVANCED),
    OptionRow::Setting(SettingId::VideoMaxQueuedFrames),
    OptionRow::Reset,
];

/// *Advanced options › 3D rendering*.
pub const RENDERING_3D_ROWS: [OptionRow; 6] = [
    OptionRow::Setting(SettingId::RenderVisualStyle),
    OptionRow::Page(OptionsPage::EnhancedEffects),
    OptionRow::Page(OptionsPage::Geometry),
    OptionRow::Setting(SettingId::SceneSkybox),
    OptionRow::Setting(SettingId::RenderMeshAlgo),
    OptionRow::Reset,
];

/// *3D rendering › Enhanced effects* (AMD-055 REQ-SCN-079), the reflections and their four
/// gradations under a title of their own.
pub const ENHANCED_EFFECTS_ROWS: [OptionRow; 8] = [
    OptionRow::Setting(SettingId::RenderStyleVoxelGrid),
    OptionRow::Group(keys::GROUP_REFLECTIONS),
    OptionRow::Setting(SettingId::RenderStyleReflections),
    OptionRow::Setting(SettingId::RenderStyleSsrShine),
    OptionRow::Setting(SettingId::RenderStyleSsrClarity),
    OptionRow::Setting(SettingId::RenderStyleSsrLength),
    OptionRow::Setting(SettingId::RenderStyleSsrQuality),
    OptionRow::Reset,
];

/// *3D rendering › Geometry (curvature)* (AMD-043 REQ-SCN-066, AMD-046 §3).
pub const GEOMETRY_ROWS: [OptionRow; 6] = [
    OptionRow::Setting(SettingId::SceneBendLr),
    OptionRow::Setting(SettingId::SceneBendTb),
    OptionRow::Setting(SettingId::SceneArcH),
    OptionRow::Setting(SettingId::SceneArcCenter),
    OptionRow::Setting(SettingId::SceneBend2dMode),
    OptionRow::Reset,
];

/// *Advanced options › Light and shadows*. The shadow switch itself is the main page's row.
pub const LIGHT_SHADOWS_ROWS: [OptionRow; 10] = [
    OptionRow::Setting(SettingId::LightIntensity),
    OptionRow::Setting(SettingId::RenderBackdropLighting),
    OptionRow::Setting(SettingId::RenderStylePixelShadow),
    OptionRow::Group(keys::GROUP_AMBIENT_OCCLUSION),
    OptionRow::Setting(SettingId::RenderStyleAo),
    OptionRow::Setting(SettingId::RenderStyleAoStrength),
    OptionRow::Setting(SettingId::RenderStyleAoRadius),
    OptionRow::Setting(SettingId::RenderStyleAoThickness),
    OptionRow::Setting(SettingId::RenderStyleAoQuality),
    OptionRow::Reset,
];

/// *Advanced options › NES picture*. The palette is the main page's row and the overscan its own
/// screen, so what is left is what the console puts out beyond them.
pub const NES_PICTURE_ROWS: [OptionRow; 4] = [
    OptionRow::Setting(SettingId::VideoRemoveSpriteLimit),
    // Lot FIDPLUS-D2b: the composite signal (Fidélité+) and the emphasis in 3D.
    OptionRow::Setting(SettingId::VideoCompositeSignal),
    OptionRow::Setting(SettingId::VideoEmphasis3d),
    OptionRow::Reset,
];

/// *Advanced options › Camera*: the preset row that left the main page, and the field of view.
pub const CAMERA_ROWS: [OptionRow; 3] = [
    OptionRow::Camera,
    OptionRow::Setting(SettingId::CameraFov),
    OptionRow::Reset,
];

/// *Advanced options › Emulation*. The region row of the main page sets the same two settings in
/// one step; these are the two apart, as the mockup asks.
///
/// KIRBY-MERGE added the smooth scrolling under the run-ahead, because it spends one of the
/// run-ahead's frames and does nothing without it. The debugging view that goes with it
/// (`emu.smooth_scroll_view`) is **not** here: like `audio.latency_ms` it stays a desktop row,
/// this screen being a short curated list.
pub const EMULATION_ROWS: [OptionRow; 41] = [
    OptionRow::Setting(SettingId::EmuEngine),
    // Lot CARTDB: the cartridge database and the power-on RAM, as in the window.
    OptionRow::Setting(SettingId::EmuCartridgeDb),
    OptionRow::Setting(SettingId::EmuPowerOnRam),
    OptionRow::Setting(SettingId::EmuPowerOnPpuRam),
    OptionRow::Setting(SettingId::EmuPpuRevision),
    OptionRow::Setting(SettingId::EmuMmc3Revision),
    OptionRow::Setting(SettingId::EmuCpuRevision),
    OptionRow::Setting(SettingId::EmuOamaddrWriteCorruption),
    OptionRow::Setting(SettingId::EmuOamCorruption),
    OptionRow::Setting(SettingId::EmuPpuBusConflicts),
    OptionRow::Setting(SettingId::EmuOamaddrBug),
    OptionRow::Setting(SettingId::EmuOamDecay),
    OptionRow::Setting(SettingId::EmuRainbowNetwork),
    OptionRow::Setting(SettingId::EmuVsDip1),
    OptionRow::Setting(SettingId::EmuVsDip2),
    OptionRow::Setting(SettingId::EmuVsDip3),
    OptionRow::Setting(SettingId::EmuVsDip4),
    OptionRow::Setting(SettingId::EmuVsDip5),
    OptionRow::Setting(SettingId::EmuVsDip6),
    OptionRow::Setting(SettingId::EmuVsDip7),
    OptionRow::Setting(SettingId::EmuVsDip8),
    OptionRow::Setting(SettingId::EmuPowerOnTop),
    OptionRow::Setting(SettingId::EmuPpu2006Delay),
    OptionRow::Setting(SettingId::EmuPpu2006Glitch),
    OptionRow::Setting(SettingId::EmuPpu2000Glitch),
    OptionRow::Setting(SettingId::EmuPaletteCorruption),
    OptionRow::Setting(SettingId::EmuPpuResetLine),
    OptionRow::Setting(SettingId::EmuCpuPpuAlignment),
    OptionRow::Setting(SettingId::EmuCpuApuParity),
    OptionRow::Setting(SettingId::EmuFidelityPlus),
    OptionRow::Setting(SettingId::EmuFastCore),
    OptionRow::Setting(SettingId::EmuExactRate),
    OptionRow::Setting(SettingId::EmuRealtimeInput),
    OptionRow::Setting(SettingId::EmuLatePresent),
    OptionRow::Setting(SettingId::EmuRunAhead),
    OptionRow::Setting(SettingId::EmuSmoothScrollAhead),
    OptionRow::Setting(SettingId::EmuGameSpeed),
    OptionRow::Setting(SettingId::EmuAutoTvMode),
    OptionRow::Setting(SettingId::EmuTvMode),
    OptionRow::Setting(SettingId::InputEmulatedMenuButton),
    OptionRow::Reset,
];

/// *Advanced options › Audio* (AUDIO-2): the choice of output. The volume stays on the main page,
/// and `audio.latency_ms` is deliberately not here — AUDIO-1 left it out of the console screen
/// (owner request: that screen is a short curated list), and nothing of this lot changes that.
pub const AUDIO_ROWS: [OptionRow; 3] = [
    OptionRow::Setting(SettingId::AudioClassicOutput),
    OptionRow::Setting(SettingId::AudioConsoleProfile),
    OptionRow::Reset,
];

/// The values of the palette row (`video.nes_palette`): the setting's choices.
fn palettes() -> Vec<i32> {
    catalogue::descriptor(SettingId::VideoNesPalette)
        .domain()
        .options()
        .unwrap_or_default()
        .iter()
        .map(|option| {
            #[allow(clippy::cast_possible_truncation)] // A choice's index.
            let value = option.value as i32;
            value
        })
        .collect()
}

fn palette(settings: &SettingsModel) -> i32 {
    settings.get(SettingId::VideoNesPalette).as_int()
}

/// The values of the catch-up row (`emu.catch_up`): none, then a limit of 1 to 5 frames.
const CATCH_UPS: [i32; 6] = [0, 1, 2, 3, 4, 5];

fn catch_up(settings: &SettingsModel) -> i32 {
    settings.get(SettingId::EmuCatchUp).as_int()
}

/// The values of the pipeline row (`emu.pipeline`): off, auto, always.
const PIPELINES: [i32; 3] = [0, 1, 2];

fn pipeline(settings: &SettingsModel) -> i32 {
    settings.get(SettingId::EmuPipeline).as_int()
}

/// The choices of the region row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionChoice {
    /// `emu.auto_tv_mode` on: the region of the ROM header, applied when a game starts.
    Auto,
    /// NTSC, at once.
    Ntsc,
    /// PAL, at once.
    Pal,
}

impl RegionChoice {
    /// The choices, in the order the row cycles through them.
    pub const ALL: [Self; 3] = [Self::Auto, Self::Ntsc, Self::Pal];

    /// The choice `settings` stand for: Auto while the automatic mode is on, else the region.
    #[must_use]
    pub fn of(settings: &SettingsModel) -> Self {
        if settings.auto_tv_mode() {
            Self::Auto
        } else if settings.tv_mode() == TvRegionSetting::Pal {
            Self::Pal
        } else {
            Self::Ntsc
        }
    }

    fn apply(self, settings: &mut SettingsModel) {
        settings.set_auto_tv_mode(self == Self::Auto);
        match self {
            Self::Auto => {}
            Self::Ntsc => settings.set_tv_mode(TvRegionSetting::Ntsc),
            Self::Pal => settings.set_tv_mode(TvRegionSetting::Pal),
        }
    }
}

/// What the options ask of the application beyond the settings they change themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsCommand {
    /// Leave the options.
    Close,
    /// Show the list of the gamepad bindings.
    OpenControls,
    /// Show the overscan's screen.
    OpenOverscan,
    /// Show the RetroAchievements screen.
    OpenAchievements,
    /// Step the camera preset.
    StepCamera(i32),
}

/// How deep the pages go (RO-3): main › *Advanced options* › *3D rendering* › *Enhanced effects*.
pub const MAX_DEPTH: usize = 3;

/// The index of the first row of `rows` a selection may rest on: a group title is a label, not a
/// row. A page with nothing but group titles would be a mistake, and a test forbids it; the fall
/// back of 0 keeps the index in range either way.
#[must_use]
pub fn first_selectable(rows: &[OptionRow]) -> usize {
    rows.iter()
        .position(|row| !matches!(row, OptionRow::Group(_)))
        .unwrap_or(0)
}

/// The selection `from` moved `step` places through `rows`, wrapping round and **skipping the
/// group titles** (RO-3). `step` is −1 or 1.
#[must_use]
pub fn step_selection(rows: &[OptionRow], from: usize, step: i32) -> usize {
    let count = rows.len();
    if count == 0 {
        return 0;
    }
    let mut at = from;
    // At most one turn round the list: a page of nothing but titles leaves the selection where
    // it was rather than looping for ever.
    for _ in 0..count {
        at = if step < 0 {
            (at + count - 1) % count
        } else {
            (at + 1) % count
        };
        if !matches!(rows[at], OptionRow::Group(_)) {
            return at;
        }
    }
    from
}

/// The list and its selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandheldOptionsModel {
    selected: usize,
    /// The pages open over the main one, innermost last: each the page shown and the row of the
    /// page under it to come back to. RO-3 made this a trail rather than one page, the hub of
    /// the mockup being three levels deep.
    trail: [(OptionsPage, usize); MAX_DEPTH],
    depth: usize,
    /// The mode of the options and the chosen preset, as the settings last held them
    /// ([`Self::sync`]): they decide the main page's rows and the preset page's.
    advanced: bool,
    preset: usize,
    /// The rows of the shaders page as the settings last held them: each slot's sub-options
    /// follow the family it holds.
    shader_rows: Vec<OptionRow>,
}

impl Default for HandheldOptionsModel {
    fn default() -> Self {
        Self::new()
    }
}

impl HandheldOptionsModel {
    /// The options on their first row.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selected: 0,
            trail: [(OptionsPage::Advanced, 0); MAX_DEPTH],
            depth: 0,
            advanced: false,
            preset: 2,
            shader_rows: Vec::new(),
        }
    }

    /// Takes the mode and the preset from `settings`. A mode that changed leaves the pages for
    /// the main one, on its mode row (the two main pages differ).
    pub fn sync(&mut self, settings: &SettingsModel) {
        let advanced = crate::ui::presets::advanced(settings);
        self.preset = crate::ui::presets::chosen(settings);
        self.shader_rows = shader_rows(settings);
        if advanced != self.advanced {
            self.advanced = advanced;
            self.depth = 0;
            self.selected = 0;
        }
    }

    /// Whether the options are in their advanced mode.
    #[must_use]
    pub const fn advanced(&self) -> bool {
        self.advanced
    }

    /// The rows of the main page of the advanced mode: the Android one where a touch screen is
    /// offered.
    #[must_use]
    pub fn main_rows() -> &'static [OptionRow] {
        if zax_platform::input::touch::offered() {
            &TOUCH_ROWS
        } else {
            &ROWS
        }
    }

    /// The rows of the main page of the simple mode.
    #[must_use]
    pub fn simple_main_rows() -> &'static [OptionRow] {
        if zax_platform::input::touch::offered() {
            &SIMPLE_TOUCH_ROWS
        } else {
            &SIMPLE_ROWS
        }
    }

    /// The rows of the page shown.
    #[must_use]
    pub fn rows(&self) -> &[OptionRow] {
        match self.depth.checked_sub(1) {
            Some(last) => match self.trail[last].0 {
                OptionsPage::PresetOptions => preset_rows(self.preset),
                OptionsPage::Shaders => &self.shader_rows,
                page => page.rows(),
            },
            None if self.advanced => Self::main_rows(),
            None => Self::simple_main_rows(),
        }
    }

    /// Whether the "Optimizations" page is shown.
    #[must_use]
    pub fn in_optimizations(&self) -> bool {
        self.page() == Some(OptionsPage::Optimizations)
    }

    /// The page shown, if any; none on the main page.
    #[must_use]
    pub fn page(&self) -> Option<OptionsPage> {
        self.depth.checked_sub(1).map(|last| self.trail[last].0)
    }

    /// The pages open over the main one, outermost first: what the title band spells out.
    #[must_use]
    pub fn path(&self) -> Vec<OptionsPage> {
        self.trail[..self.depth]
            .iter()
            .map(|(page, _)| *page)
            .collect()
    }

    /// The selected row.
    #[must_use]
    pub fn selected(&self) -> OptionRow {
        let rows = self.rows();
        rows.get(self.selected)
            .copied()
            .unwrap_or_else(|| rows[first_selectable(rows)])
    }

    /// The index of the selected row.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// Climbs to `page`, remembering the row to come back to. Does nothing at [`MAX_DEPTH`].
    fn open(&mut self, page: OptionsPage) {
        if self.depth >= MAX_DEPTH {
            return;
        }
        self.trail[self.depth] = (page, self.selected);
        self.depth += 1;
        self.selected = first_selectable(self.rows());
    }

    /// One press of a button; a value row changes `settings` itself.
    pub fn handle(
        &mut self,
        button: PadButton,
        settings: &mut SettingsModel,
    ) -> Option<OptionsCommand> {
        self.sync(settings);
        let command = self.press(button, settings);
        self.sync(settings);
        command
    }

    fn press(&mut self, button: PadButton, settings: &mut SettingsModel) -> Option<OptionsCommand> {
        match button {
            // The reset row takes the page's settings back (lot REWRITE-UI).
            PadButton::A | PadButton::Left | PadButton::Right
                if self.selected() == OptionRow::Reset =>
            {
                let preset = self.preset;
                let on_preset_page = self.page() == Some(OptionsPage::PresetOptions);
                reset_page(self.rows(), settings, preset, on_preset_page);
            }
            PadButton::Up => self.selected = step_selection(self.rows(), self.selected, -1),
            PadButton::Down => self.selected = step_selection(self.rows(), self.selected, 1),
            PadButton::Left | PadButton::Right | PadButton::A
                if matches!(self.selected(), OptionRow::Page(_)) =>
            {
                if let OptionRow::Page(page) = self.selected() {
                    self.open(page);
                }
            }
            // A control only Zaxnes's own engine has does not step while another is chosen.
            PadButton::Left | PadButton::Right | PadButton::A
                if engine_lock(self.selected(), settings).is_some() => {}
            PadButton::Left => return self.step(settings, -1),
            PadButton::Right => return self.step(settings, 1),
            PadButton::A => {
                return match self.selected() {
                    OptionRow::Controls => Some(OptionsCommand::OpenControls),
                    OptionRow::Achievements => Some(OptionsCommand::OpenAchievements),
                    _ => self.step(settings, 1),
                };
            }
            // B climbs back one page at a time, and leaves the options from the main one.
            PadButton::B => match self.depth.checked_sub(1) {
                Some(last) => {
                    self.depth = last;
                    self.selected = self.trail[last].1;
                }
                None => return Some(OptionsCommand::Close),
            },
            _ => {}
        }
        None
    }

    fn step(&self, settings: &mut SettingsModel, step: i32) -> Option<OptionsCommand> {
        step_row(self.selected(), settings, step)
    }
}

/// Steps one of the rows lot REWRITE-UI added: the mode, the preset, the widescreen and the
/// switches of the widescreen page.
fn step_options_row(row: OptionRow, settings: &mut SettingsModel, step: i32) {
    match row {
        OptionRow::Mode => {
            let advanced = crate::ui::presets::advanced(settings);
            settings.set(SettingId::UiOptionsAdvanced, SettingValue::Bool(!advanced));
        }
        OptionRow::Preset => {
            let count = crate::ui::presets::PRESETS.len();
            let indices: Vec<usize> = (0..count).collect();
            let next = *cycled(&indices, &crate::ui::presets::chosen(settings), step);
            crate::ui::presets::choose(settings, next);
        }
        OptionRow::WidescreenOn => {
            let on = settings.get(SettingId::VideoWidescreen).as_int() != 0;
            settings.set(
                SettingId::VideoWidescreen,
                SettingValue::Enum(i32::from(!on)),
            );
        }
        OptionRow::Switch(id) => {
            let on = settings.get(id).as_bool();
            settings.set(id, SettingValue::Bool(!on));
        }
        OptionRow::HackDefault(key) => {
            let stored = settings.get(SettingId::EmuHackOptions);
            let on = hack_default(stored.as_text(), key);
            let text = hack_options::with_common(stored.as_text(), key, !on);
            settings.set(SettingId::EmuHackOptions, SettingValue::Text(text));
        }
        _ => {}
    }
}

/// Steps a row of the shaders page: a slot's family, the graphics API, a pre-filter or a
/// sub-option of the family.
fn step_shader_row(row: OptionRow, settings: &mut SettingsModel, step: i32) {
    match row {
        OptionRow::Shader(slot) => {
            let picks = shader_panel::slot_picks(slot);
            let next = cycled(&picks, &shader_panel::slot_pick(settings, slot), step).clone();
            shader_panel::set_slot_pick(settings, slot, &next);
        }
        OptionRow::ShaderBackend => {
            // OpenGL (0) and Vulkan (1); DirectX 12 is the desktop's, and a console is not one.
            let next = i32::from(settings.get(SettingId::VideoBackend).as_int() != 1);
            settings.set(SettingId::VideoBackend, SettingValue::Enum(next));
        }
        OptionRow::ShaderPreFilter(slot) => {
            let choices: Vec<Option<&shader_panel::PreFilter>> = std::iter::once(None)
                .chain(shader_panel::pre_filters().map(Some))
                .collect();
            let next = *cycled(&choices, &shader_panel::pre_filter_of(settings, slot), step);
            crate::ui::shader_looks::set_pre_filter(settings, slot, next);
        }
        OptionRow::ShaderOption(slot, index) => {
            if let Some(state) = crate::ui::shader_looks::state_of(settings, slot) {
                if let Some(option) = crate::ui::shader_looks::sub_options(&state)
                    .get(usize::from(index))
                    .copied()
                {
                    crate::ui::shader_looks::step_sub_option(settings, slot, option, step);
                }
            }
        }
        _ => {}
    }
}

/// Steps `row` by `step` in `settings`: the value rows change the settings themselves, the camera
/// row asks the application.
pub fn step_row(row: OptionRow, settings: &mut SettingsModel, step: i32) -> Option<OptionsCommand> {
    match row {
        OptionRow::Language => {
            let codes = language_codes();
            let current = settings.language();
            let next = *cycled(&codes, &current.as_str(), step);
            settings.set_language(next);
        }
        OptionRow::Volume => settings.set_volume(stepped_volume(settings.volume(), step)),
        OptionRow::RenderScale => set_render_scale(settings, step),
        OptionRow::Shadows => {
            let on = settings.shadow();
            settings.set_shadow(!on);
        }
        OptionRow::Palette => {
            let next = *cycled(&palettes(), &palette(settings), step);
            settings.set(SettingId::VideoNesPalette, SettingValue::Enum(next));
        }
        OptionRow::FrameCap => {
            let next = *cycled(&FRAME_CAPS, &frame_cap(settings), step);
            settings.set_vsync(if next == 30 { 2 } else { 1 });
        }
        OptionRow::Profiler => {
            let on = settings.get(SettingId::UiProfiler).as_bool();
            settings.set(SettingId::UiProfiler, SettingValue::Bool(!on));
        }
        OptionRow::Pause3dIn2d => {
            let on = settings.get(SettingId::EmuPause3dIn2d).as_bool();
            settings.set(SettingId::EmuPause3dIn2d, SettingValue::Bool(!on));
        }
        OptionRow::CatchUpLast3d => {
            let on = settings.get(SettingId::EmuCatchUpLast3d).as_bool();
            settings.set(SettingId::EmuCatchUpLast3d, SettingValue::Bool(!on));
        }
        OptionRow::HalfRate3d => {
            let on = settings.get(SettingId::VideoHalfRate3d).as_bool();
            settings.set(SettingId::VideoHalfRate3d, SettingValue::Bool(!on));
        }
        OptionRow::ShowFps => {
            let on = settings.show_fps();
            settings.set_show_fps(!on);
        }
        OptionRow::Camera => return Some(OptionsCommand::StepCamera(step)),
        OptionRow::Overscan => return Some(OptionsCommand::OpenOverscan),
        OptionRow::CatchUp => {
            let next = *cycled(&CATCH_UPS, &catch_up(settings), step);
            settings.set(SettingId::EmuCatchUp, SettingValue::Enum(next));
        }
        OptionRow::LuaOptimized => {
            let on = settings.get(SettingId::EmuLuaOptimized).as_bool();
            settings.set(SettingId::EmuLuaOptimized, SettingValue::Bool(!on));
        }
        OptionRow::LuaExperimental => {
            let on = settings.get(SettingId::EmuLuaExperimental).as_bool();
            settings.set(SettingId::EmuLuaExperimental, SettingValue::Bool(!on));
        }
        OptionRow::Pipeline => {
            let next = *cycled(&PIPELINES, &pipeline(settings), step);
            settings.set(SettingId::EmuPipeline, SettingValue::Enum(next));
        }
        OptionRow::Region => {
            cycled(&RegionChoice::ALL, &RegionChoice::of(settings), step).apply(settings);
        }
        OptionRow::TouchPad => {
            let next = *cycled(&TOUCH_PADS, &touch_pad(settings), step);
            settings.set(SettingId::InputTouchPad, SettingValue::Enum(next));
        }
        OptionRow::Choice(id) => {
            let values = choice_values(id);
            if !values.is_empty() {
                let next = *cycled(&values, &settings.get(id).as_int(), step);
                settings.set(id, SettingValue::Enum(next));
            }
        }
        OptionRow::Controls | OptionRow::Page(_) | OptionRow::Group(_) | OptionRow::Reset => {}
        OptionRow::Mode
        | OptionRow::Preset
        | OptionRow::WidescreenOn
        | OptionRow::Switch(_)
        | OptionRow::HackDefault(_) => step_options_row(row, settings, step),
        OptionRow::Setting(id) => super::advanced::step(id, settings, step),
        OptionRow::Achievements => return Some(OptionsCommand::OpenAchievements),
        OptionRow::Shader(_)
        | OptionRow::ShaderBackend
        | OptionRow::ShaderPreFilter(_)
        | OptionRow::ShaderOption(..) => step_shader_row(row, settings, step),
    }
    None
}

/// The rows of the preset page for preset `index`: the settings it proposes, then the reset
/// row, which writes the preset again.
#[must_use]
pub fn preset_rows(index: usize) -> &'static [OptionRow] {
    static ROWS: std::sync::LazyLock<Vec<Vec<OptionRow>>> = std::sync::LazyLock::new(|| {
        crate::ui::presets::PRESETS
            .iter()
            .map(|preset| {
                preset
                    .values
                    .iter()
                    .filter(|&&(_, _, shown)| shown)
                    .map(|&(id, ..)| preset_row(id))
                    .chain(std::iter::once(OptionRow::Reset))
                    .collect()
            })
            .collect()
    });
    &ROWS[index.min(ROWS.len() - 1)]
}

/// The console row of a setting a preset proposes: the interface's own row where it has one.
const fn preset_row(id: SettingId) -> OptionRow {
    match id {
        SettingId::EmuPipeline => OptionRow::Pipeline,
        SettingId::EmuLuaOptimized => OptionRow::LuaOptimized,
        SettingId::EmuLuaExperimental => OptionRow::LuaExperimental,
        SettingId::EmuPause3dIn2d => OptionRow::Pause3dIn2d,
        SettingId::EmuCatchUpLast3d => OptionRow::CatchUpLast3d,
        SettingId::VideoHalfRate3d => OptionRow::HalfRate3d,
        SettingId::RenderShadow => OptionRow::Shadows,
        id => OptionRow::Setting(id),
    }
}

/// Whether the common default of hack option `key` is on (the packs' own value when unset).
fn hack_default(stored: &str, key: &str) -> bool {
    hack_options::common_value(stored, key).unwrap_or_else(|| {
        hack_options::COMMON_OPTIONS
            .iter()
            .find(|&&(common, _)| common == key)
            .is_some_and(|&(_, on)| on)
    })
}

/// The settings a row of the global options changes, for its page's reset row.
fn reset_ids(row: OptionRow) -> Vec<SettingId> {
    match row {
        OptionRow::Language => vec![SettingId::UiLanguage],
        OptionRow::WidescreenOn => vec![SettingId::VideoWidescreen],
        OptionRow::Setting(id) | OptionRow::Switch(id) | OptionRow::Choice(id) => vec![id],
        OptionRow::ShaderBackend => vec![SettingId::VideoBackend],
        OptionRow::Mode
        | OptionRow::Preset
        | OptionRow::HackDefault(_)
        | OptionRow::Reset
        | OptionRow::Shader(_)
        | OptionRow::ShaderPreFilter(_)
        | OptionRow::ShaderOption(..) => Vec::new(),
        // The others are the rows a game may set too: the same settings.
        row => super::game_options::row_settings(row).to_vec(),
    }
}

/// The reset row of a page: its settings back to the chosen preset's values where it manages
/// them, to their own defaults elsewhere; the common hack defaults dropped. On the preset page
/// it writes the preset again.
fn reset_page(rows: &[OptionRow], settings: &mut SettingsModel, preset: usize, on_preset: bool) {
    if on_preset {
        crate::ui::presets::choose(settings, preset);
        return;
    }
    let ids: Vec<SettingId> = rows.iter().flat_map(|&row| reset_ids(row)).collect();
    crate::ui::presets::reset_with(settings, &crate::ui::presets::PRESETS[preset], &ids);
    if rows
        .iter()
        .any(|row| matches!(row, OptionRow::HackDefault(_)))
    {
        let stored = settings.get(SettingId::EmuHackOptions);
        let text = hack_options::without_common(stored.as_text());
        settings.set(SettingId::EmuHackOptions, SettingValue::Text(text));
    }
}

/// The label of `row`, in the language in force.
#[must_use]
pub fn label(row: OptionRow) -> String {
    match row {
        OptionRow::Language => text::label_of(catalogue::descriptor(SettingId::UiLanguage)),
        OptionRow::Volume => text::label_of(catalogue::descriptor(SettingId::AudioVolume)),
        OptionRow::RenderScale => strings::get(keys::HANDHELD_OPTION_RENDER_SCALE),
        OptionRow::Shadows => text::label_of(catalogue::descriptor(SettingId::RenderShadow)),
        OptionRow::Palette => text::label_of(catalogue::descriptor(SettingId::VideoNesPalette)),
        OptionRow::Overscan => strings::get(keys::HANDHELD_OPTION_OVERSCAN),
        OptionRow::FrameCap => strings::get(keys::HANDHELD_OPTION_FRAME_CAP),
        OptionRow::ShowFps => text::label_of(catalogue::descriptor(SettingId::UiShowFps)),
        OptionRow::Profiler => text::label_of(catalogue::descriptor(SettingId::UiProfiler)),
        OptionRow::Pause3dIn2d => text::label_of(catalogue::descriptor(SettingId::EmuPause3dIn2d)),
        OptionRow::CatchUpLast3d => {
            text::label_of(catalogue::descriptor(SettingId::EmuCatchUpLast3d))
        }
        OptionRow::HalfRate3d => text::label_of(catalogue::descriptor(SettingId::VideoHalfRate3d)),
        OptionRow::Camera => strings::get(keys::HANDHELD_OPTION_CAMERA),
        OptionRow::CatchUp => strings::get(keys::HANDHELD_OPTION_CATCH_UP),
        OptionRow::LuaOptimized => {
            text::label_of(catalogue::descriptor(SettingId::EmuLuaOptimized))
        }
        OptionRow::LuaExperimental => {
            text::label_of(catalogue::descriptor(SettingId::EmuLuaExperimental))
        }
        OptionRow::Pipeline => strings::get(keys::HANDHELD_OPTION_PIPELINE),
        OptionRow::Region => strings::get(keys::HANDHELD_OPTION_REGION),
        OptionRow::TouchPad => strings::get(keys::HANDHELD_OPTION_TOUCH_PAD),
        OptionRow::Choice(id) | OptionRow::Switch(id) => text::label_of(catalogue::descriptor(id)),
        OptionRow::Setting(id) => super::advanced::label(id),
        OptionRow::Controls => strings::get(keys::HANDHELD_OPTION_CONTROLS),
        OptionRow::Achievements => strings::get(keys::TAB_ACHIEVEMENTS),
        OptionRow::Page(page) => strings::get(page.title_key()),
        OptionRow::Group(key) => strings::get(key),
        OptionRow::Shader(slot) => shader_panel::slot_label(slot),
        OptionRow::ShaderPreFilter(_) => format!("  {}", strings::get("shaders.prefilter")),
        OptionRow::ShaderBackend => strings::get("shaders.backend"),
        // Its text depends on the family the slot holds: `row_label`.
        OptionRow::ShaderOption(..) => String::new(),
        OptionRow::Mode => strings::get("options.new.mode"),
        OptionRow::Preset => strings::get("options.new.preset_title"),
        OptionRow::WidescreenOn => strings::get("options.new.widescreen_default"),
        OptionRow::HackDefault(key) => strings::get(&format!("options.new.hack.{key}")),
        OptionRow::Reset => strings::get(keys::BUTTON_RESET_DEFAULTS),
    }
}

/// The label of `row` with the settings at hand: a shader sub-option's depends on the family its
/// slot holds; any other row's is [`label`]'s.
#[must_use]
pub fn row_label(row: OptionRow, settings: &SettingsModel) -> String {
    match row {
        OptionRow::ShaderOption(slot, index) => shader_option(settings, slot, index)
            .map_or_else(String::new, |(label, _)| format!("  {label}")),
        row => label(row),
    }
}

/// The label and the value of sub-option `index` of the family `slot` holds.
fn shader_option(
    settings: &SettingsModel,
    slot: ShaderSlot,
    index: u8,
) -> Option<(String, String)> {
    let state = crate::ui::shader_looks::state_of(settings, slot)?;
    let option = *crate::ui::shader_looks::sub_options(&state).get(usize::from(index))?;
    Some(crate::ui::shader_looks::sub_option_text(&state, option))
}

/// The rows of the shaders page for `settings`: the graphics API and the resolution, then each
/// slot, its pre-filter and the sub-options of its family when it holds a preset, then the reset
/// row (lot REWRITE-UI).
#[must_use]
pub fn shader_rows(settings: &SettingsModel) -> Vec<OptionRow> {
    let mut rows = vec![
        OptionRow::ShaderBackend,
        OptionRow::Setting(SettingId::VideoShaderResolution),
    ];
    for slot in [
        ShaderSlot::TwoD,
        ShaderSlot::TwoDWide,
        ShaderSlot::ThreeD,
        ShaderSlot::ThreeDWide,
    ] {
        rows.push(OptionRow::Shader(slot));
        if settings.get(slot.ids().mode).as_int() != zax_settings::shader_slots::PRESET {
            continue;
        }
        rows.push(OptionRow::ShaderPreFilter(slot));
        if let Some(state) = crate::ui::shader_looks::state_of(settings, slot) {
            let count = crate::ui::shader_looks::sub_options(&state).len();
            rows.extend(
                (0..count)
                    .filter_map(|index| u8::try_from(index).ok())
                    .map(|index| OptionRow::ShaderOption(slot, index)),
            );
        }
    }
    rows.push(OptionRow::Reset);
    rows
}

/// The value of `row` as the screen shows it; empty for a row that opens a list.
#[must_use]
pub fn value_text(row: OptionRow, settings: &SettingsModel, camera: CameraPreset) -> String {
    let switch = |on: bool| text::label_of_option(if on { "enabled" } else { "disabled" });
    match row {
        OptionRow::Language => text::label_of_option(&settings.language()),
        OptionRow::Volume => format!("{} %", volume_percent(settings.volume())),
        OptionRow::RenderScale => render_scale_options()
            .get(render_scale_index(settings))
            .map_or_else(
                || text::label_of_option("Auto"),
                |label| text::label_of_option(label),
            ),
        OptionRow::Shadows => switch(settings.shadow()),
        OptionRow::Palette => catalogue::descriptor(SettingId::VideoNesPalette)
            .domain()
            .options()
            .unwrap_or_default()
            .get(usize::try_from(palette(settings)).unwrap_or(0))
            .map_or_else(String::new, |option| text::label_of_option(&option.label)),
        OptionRow::Overscan => super::overscan::summary(settings),
        OptionRow::FrameCap => format!("{} fps", frame_cap(settings)),
        OptionRow::ShowFps => switch(settings.show_fps()),
        OptionRow::Profiler => switch(settings.get(SettingId::UiProfiler).as_bool()),
        OptionRow::Pause3dIn2d => switch(settings.get(SettingId::EmuPause3dIn2d).as_bool()),
        OptionRow::CatchUpLast3d => switch(settings.get(SettingId::EmuCatchUpLast3d).as_bool()),
        OptionRow::HalfRate3d => switch(settings.get(SettingId::VideoHalfRate3d).as_bool()),
        OptionRow::Camera => camera_label(camera),
        OptionRow::CatchUp => match catch_up(settings) {
            0 => text::label_of_option("None"),
            frames => frames.to_string(),
        },
        OptionRow::LuaOptimized => switch(settings.get(SettingId::EmuLuaOptimized).as_bool()),
        OptionRow::LuaExperimental => switch(settings.get(SettingId::EmuLuaExperimental).as_bool()),
        OptionRow::Pipeline => text::label_of_option(match pipeline(settings) {
            1 => "Auto",
            2 => "Always",
            _ => "Off",
        }),
        OptionRow::Region => match RegionChoice::of(settings) {
            RegionChoice::Auto => strings::get(keys::HANDHELD_REGION_AUTO),
            RegionChoice::Ntsc => "NTSC".to_owned(),
            RegionChoice::Pal => "PAL".to_owned(),
        },
        OptionRow::Choice(id) => choice_text(id, settings),
        OptionRow::TouchPad => match touch_pad(settings) {
            0 => strings::get(keys::HANDHELD_TOUCH_PAD_OFF),
            1 => strings::get(keys::HANDHELD_TOUCH_PAD_ON),
            _ => text::label_of_option("Auto"),
        },
        OptionRow::Controls | OptionRow::Page(_) | OptionRow::Group(_) | OptionRow::Reset => {
            String::new()
        }
        OptionRow::Mode => strings::get(if crate::ui::presets::advanced(settings) {
            "options.new.mode.advanced"
        } else {
            "options.new.mode.simple"
        }),
        OptionRow::Preset => {
            let chosen = crate::ui::presets::chosen(settings);
            let name = crate::ui::presets::name(chosen);
            if crate::ui::presets::matches(settings, &crate::ui::presets::PRESETS[chosen]) {
                name
            } else {
                format!("{name} {}", strings::get("options.new.modified"))
            }
        }
        OptionRow::WidescreenOn => switch(settings.get(SettingId::VideoWidescreen).as_int() != 0),
        OptionRow::Switch(id) => switch(settings.get(id).as_bool()),
        OptionRow::HackDefault(key) => switch(hack_default(
            settings.get(SettingId::EmuHackOptions).as_text(),
            key,
        )),
        OptionRow::Setting(id) => super::advanced::value_text(id, settings),
        OptionRow::Achievements => switch(settings.get(SettingId::AchievementsEnabled).as_bool()),
        OptionRow::Shader(slot) => {
            shader_panel::pick_label(slot, &shader_panel::slot_pick(settings, slot))
        }
        OptionRow::ShaderPreFilter(slot) => {
            shader_panel::pre_filter_label(shader_panel::pre_filter_of(settings, slot))
        }
        OptionRow::ShaderOption(slot, index) => {
            shader_option(settings, slot, index).map_or_else(String::new, |(_, value)| value)
        }
        OptionRow::ShaderBackend => strings::get(if !cfg!(feature = "wgpu") {
            "shaders.backend.not_built"
        } else if settings.get(SettingId::VideoBackend).as_int() == 0 {
            "shaders.backend.gl"
        } else {
            "shaders.backend.vulkan"
        }),
    }
}

/// Q-103: the note the page shows after the value of the Run-ahead row — one of the advanced
/// options — while the game being played forces its **effective** value to 0. `None` for every
/// other row, and for that row when nothing forces it. `forced` is
/// `GameSession::forces_run_ahead_to_zero` of the running game.
///
/// Shorter than the window's note of the same thing (`ui::options::forced_run_ahead_note`), a
/// console row having one line and no room beside it.
// Q-103
#[must_use]
pub fn forced_run_ahead_note(row: OptionRow, forced: bool) -> Option<String> {
    (forced && row == OptionRow::Setting(SettingId::EmuRunAhead))
        .then(|| strings::get(keys::HANDHELD_RUN_AHEAD_FORCED))
}

/// Why `row` is locked on the console: it is a control only Zaxnes's own engine has (the
/// widescreen, the hack packs, the sprite limit, the run-ahead) and another engine is chosen in
/// `model`. The same list as the desktop window's, `ui::options::engine_reason`.
#[must_use]
pub fn engine_lock(row: OptionRow, model: &SettingsModel) -> Option<&'static str> {
    match row {
        OptionRow::Setting(id) => crate::ui::options::engine_reason(id, model),
        _ => None,
    }
}

/// The note a console row shows after its value while [`engine_lock`] locks it.
#[must_use]
pub fn engine_lock_note(row: OptionRow, model: &SettingsModel) -> Option<String> {
    engine_lock(row, model).map(|_| strings::get(keys::HANDHELD_ZAXNES_ONLY))
}

/// The name of a camera preset, in the language in force.
#[must_use]
pub fn camera_label(camera: CameraPreset) -> String {
    strings::get(match camera {
        CameraPreset::Orbit => keys::MENU_CAMERA_ORBITAL,
        CameraPreset::FirstPerson => keys::MENU_CAMERA_FIRST_PERSON,
        CameraPreset::ResetView => keys::HANDHELD_CAMERA_RESET,
    })
}

/// The frame cap in force: 30 with a swap every other refresh, 60 otherwise.
#[must_use]
pub fn frame_cap(settings: &SettingsModel) -> i32 {
    if settings.vsync() == 2 {
        30
    } else {
        60
    }
}

/// The value after `current`, `step` places on, wrapping round; the first when `current` is not
/// in the list.
fn cycled<'a, T: PartialEq>(values: &'a [T], current: &T, step: i32) -> &'a T {
    let count = i32::try_from(values.len()).unwrap_or(1).max(1);
    let here = values
        .iter()
        .position(|value| value == current)
        .and_then(|index| i32::try_from(index).ok())
        .unwrap_or(0);
    let next = usize::try_from((here + step).rem_euclid(count)).unwrap_or(0);
    &values[next]
}

fn language_codes() -> Vec<&'static str> {
    catalogue::descriptor(SettingId::UiLanguage)
        .text_options()
        .map_or_else(Vec::new, <[&str]>::to_vec)
}

fn nearest_volume_level(volume: f32) -> usize {
    VOLUME_LEVELS
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - volume).abs().total_cmp(&(*b - volume).abs()))
        .map_or(0, |(index, _)| index)
}

fn stepped_volume(volume: f32, step: i32) -> f32 {
    let last = VOLUME_LEVELS.len() - 1;
    let index = nearest_volume_level(volume);
    let next = if step < 0 {
        index.saturating_sub(1)
    } else {
        (index + 1).min(last)
    };
    VOLUME_LEVELS[next]
}

fn volume_percent(volume: f32) -> usize {
    nearest_volume_level(volume) * 10
}

fn render_scale_options() -> Vec<&'static str> {
    catalogue::descriptor(SettingId::VideoResScale)
        .domain()
        .options()
        .map_or_else(Vec::new, |options| {
            options.iter().map(|option| option.label.as_str()).collect()
        })
}

/// The index of the resolution scale in force: the scale only applies to the "Render Tex"
/// method, so the other method is "Native"; the automatic resolution comes after the scales.
fn render_scale_index(settings: &SettingsModel) -> usize {
    if settings.get(SettingId::VideoDynamicResolution).as_bool() {
        return render_scale_options().len();
    }
    if settings.render_method() == RENDER_TEXTURE_METHOD {
        usize::try_from(settings.get(SettingId::VideoResScale).as_int()).unwrap_or(0)
    } else {
        0
    }
}

fn set_render_scale(settings: &mut SettingsModel, step: i32) {
    let scales = i32::try_from(render_scale_options().len())
        .unwrap_or(1)
        .max(1);
    let here = i32::try_from(render_scale_index(settings)).unwrap_or(0);
    let next = (here + step).rem_euclid(scales + 1);
    let automatic = next == scales;
    settings.set(
        SettingId::VideoDynamicResolution,
        SettingValue::Bool(automatic),
    );
    if automatic {
        settings.set_render_method(RENDER_TEXTURE_METHOD);
        return;
    }
    settings.set(SettingId::VideoResScale, SettingValue::Enum(next));
    settings.set_render_method(if next > 0 {
        RENDER_TEXTURE_METHOD
    } else {
        DIRECT_METHOD
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Settings in the advanced mode, the one whose tree these tests walk (lot REWRITE-UI: a
    /// fresh model opens the simple one).
    fn advanced_settings() -> SettingsModel {
        let mut settings = SettingsModel::new();
        settings.set(SettingId::UiOptionsAdvanced, SettingValue::Bool(true));
        settings
    }

    /// Lot REWRITE-UI: the simple mode's main page, the mode row switching to the advanced one,
    /// the preset row writing its preset and the reset row keeping it.
    #[test]
    fn the_simple_mode_switches_and_the_preset_and_reset_rows_act() {
        let mut settings = SettingsModel::new();
        let mut model = HandheldOptionsModel::new();
        model.sync(&settings);
        assert_eq!(HandheldOptionsModel::simple_main_rows(), model.rows());
        assert_eq!(OptionRow::Mode, model.selected());
        model.handle(PadButton::A, &mut settings);
        assert!(crate::ui::presets::advanced(&settings));
        assert_eq!(HandheldOptionsModel::main_rows(), model.rows());
        model.handle(PadButton::A, &mut settings);
        assert!(!crate::ui::presets::advanced(&settings));

        // The preset row: right writes the next preset.
        model.handle(PadButton::Down, &mut settings);
        assert_eq!(OptionRow::Preset, model.selected());
        model.handle(PadButton::Left, &mut settings);
        let chosen = crate::ui::presets::chosen(&settings);
        assert_eq!(1, chosen);
        assert!(crate::ui::presets::matches(
            &settings,
            &crate::ui::presets::PRESETS[chosen]
        ));

        // A setting the preset manages, changed, then the reset row of its page.
        settings.set(SettingId::EmuRunAhead, SettingValue::Int(3));
        settings.set(SettingId::EmuRainbowNetwork, SettingValue::Bool(true));
        let rows = SIMPLE_EMULATION_ROWS;
        reset_page(&rows, &mut settings, chosen, false);
        assert_eq!(
            1,
            settings.get(SettingId::EmuRunAhead).as_int(),
            "the preset's value"
        );
        assert_eq!(
            SettingsModel::new().get(SettingId::EmuRainbowNetwork),
            settings.get(SettingId::EmuRainbowNetwork),
            "the setting's own default"
        );
        assert_eq!(1, crate::ui::presets::chosen(&settings), "the preset stays");
    }

    /// The preset page shows what each preset proposes, then the reset row.
    #[test]
    fn the_preset_page_shows_the_proposed_settings() {
        for (index, preset) in crate::ui::presets::PRESETS.iter().enumerate() {
            let rows = preset_rows(index);
            let shown = preset.values.iter().filter(|&&(_, _, shown)| shown).count();
            assert_eq!(shown + 1, rows.len());
            assert_eq!(Some(&OptionRow::Reset), rows.last());
        }
    }

    /// The pages to open to reach `row`, outermost first; none for a row of no page.
    fn path_to(row: OptionRow) -> Option<Vec<OptionsPage>> {
        fn search(rows: &[OptionRow], wanted: OptionRow, path: &mut Vec<OptionsPage>) -> bool {
            for candidate in rows {
                if *candidate == wanted {
                    return true;
                }
                if let OptionRow::Page(page) = candidate {
                    path.push(*page);
                    if search(page.rows(), wanted, path) {
                        return true;
                    }
                    path.pop();
                }
            }
            false
        }
        let mut path = Vec::new();
        search(HandheldOptionsModel::main_rows(), row, &mut path).then_some(path)
    }

    /// Walks to `row` wherever it is in the tree: out to the main page, then down its path.
    fn go_to(model: &mut HandheldOptionsModel, row: OptionRow, settings: &mut SettingsModel) {
        model.sync(settings);
        while model.page().is_some() {
            model.handle(PadButton::B, settings);
        }
        let path = path_to(row).expect("a reachable row");
        for page in path {
            down_to(model, OptionRow::Page(page), settings);
            model.handle(PadButton::A, settings);
        }
        down_to(model, row, settings);
    }

    /// Presses Down until `row` is selected; a row the page has not got fails rather than loops.
    fn down_to(model: &mut HandheldOptionsModel, row: OptionRow, settings: &mut SettingsModel) {
        for _ in 0..256 {
            if model.selected() == row {
                return;
            }
            model.handle(PadButton::Down, settings);
        }
        panic!("{row:?} is not on the page");
    }

    #[test]
    fn the_optimizations_page_opens_and_goes_back_to_its_row() {
        let mut settings = advanced_settings();
        let mut model = HandheldOptionsModel::new();
        model.sync(&settings);
        let row = OptionRow::Page(OptionsPage::Optimizations);
        go_to(&mut model, row, &mut settings);
        assert_eq!(None, model.handle(PadButton::A, &mut settings));
        assert!(model.in_optimizations());
        assert_eq!(OPTIMIZATION_ROWS[0], model.selected());
        assert_eq!(None, model.handle(PadButton::B, &mut settings));
        assert!(!model.in_optimizations());
        assert_eq!(row, model.selected());
        assert_eq!(
            Some(OptionsCommand::Close),
            model.handle(PadButton::B, &mut settings)
        );
    }

    /// RO-3: the three levels of the mockup — main › *Advanced options* › *3D rendering* ›
    /// *Enhanced effects* — open one at a time, and B climbs back to the row that opened each.
    #[test]
    fn ro_3_the_three_levels_open_and_b_climbs_back_one_at_a_time() {
        let mut settings = advanced_settings();
        let mut model = HandheldOptionsModel::new();
        model.sync(&settings);
        for page in [
            OptionsPage::Advanced,
            OptionsPage::Rendering3d,
            OptionsPage::EnhancedEffects,
        ] {
            down_to(&mut model, OptionRow::Page(page), &mut settings);
            assert_eq!(None, model.handle(PadButton::A, &mut settings));
            assert_eq!(Some(page), model.page());
        }
        assert_eq!(
            vec![
                OptionsPage::Advanced,
                OptionsPage::Rendering3d,
                OptionsPage::EnhancedEffects
            ],
            model.path()
        );
        assert_eq!(MAX_DEPTH, model.path().len(), "the tree is three deep");
        // A fourth level does not exist, and asking for one changes nothing.
        let before = model.clone();
        model.handle(PadButton::A, &mut settings);
        assert_eq!(Some(OptionsPage::EnhancedEffects), model.page());

        model = before;
        for page in [OptionsPage::Rendering3d, OptionsPage::Advanced] {
            assert_eq!(None, model.handle(PadButton::B, &mut settings));
            assert_eq!(Some(page), model.page());
            // The selection comes back to the row that opened the page below.
            assert!(matches!(model.selected(), OptionRow::Page(_)));
        }
        assert_eq!(None, model.handle(PadButton::B, &mut settings));
        assert_eq!(None, model.page());
        assert_eq!(OptionRow::Page(OptionsPage::Advanced), model.selected());
        assert_eq!(
            Some(OptionsCommand::Close),
            model.handle(PadButton::B, &mut settings)
        );
    }

    /// RO-3: a group title is a label. Up and down skip it in both directions, on every page of
    /// the tree that has one, and no page is nothing but titles.
    #[test]
    fn ro_3_a_group_title_is_never_selected() {
        let mut settings = advanced_settings();
        for page in OptionsPage::ALL {
            let rows = page.rows();
            assert!(
                rows.iter().any(|row| !matches!(row, OptionRow::Group(_))),
                "{page:?} is nothing but group titles"
            );
            if !rows.iter().any(|row| matches!(row, OptionRow::Group(_))) {
                continue;
            }
            let mut model = HandheldOptionsModel::new();
            go_to(&mut model, OptionRow::Page(page), &mut settings);
            model.handle(PadButton::A, &mut settings);
            assert_eq!(Some(page), model.page());
            // Twice round the page in each direction: a title is never where the selection lands.
            for step in [PadButton::Down, PadButton::Up] {
                for _ in 0..2 * rows.len() {
                    assert!(
                        !matches!(model.selected(), OptionRow::Group(_)),
                        "{page:?} selected a group title at {}",
                        model.selected_index()
                    );
                    model.handle(step, &mut settings);
                }
            }
            // And the page opens on a row, not on its first title.
            assert!(!matches!(rows[first_selectable(rows)], OptionRow::Group(_)));
        }
    }

    /// RO-3: the smooth Lua collector left the flat advanced page for the Optimisations page,
    /// where the other performance switches are, and it still steps.
    #[test]
    fn ro_3_the_optimizations_page_holds_the_smooth_collector() {
        let row = OptionRow::Setting(SettingId::EmuLuaSmoothGc);
        assert!(OPTIMIZATION_ROWS.contains(&row));
        assert_eq!(
            Some(vec![OptionsPage::Optimizations]),
            path_to(row),
            "it is on the Optimisations page and nowhere else"
        );
        let mut settings = advanced_settings();
        let mut model = HandheldOptionsModel::new();
        model.sync(&settings);
        go_to(&mut model, row, &mut settings);
        let before = settings.get(SettingId::EmuLuaSmoothGc).as_bool();
        model.handle(PadButton::A, &mut settings);
        assert_eq!(!before, settings.get(SettingId::EmuLuaSmoothGc).as_bool());
    }

    /// RO-3: the two rows the mockup takes off the main page are reachable where it puts them,
    /// and the render scale is the one row the main page keeps a second copy of.
    #[test]
    fn ro_3_the_main_page_is_the_mockups() {
        assert!(!ROWS.contains(&OptionRow::Camera));
        assert!(!ROWS.contains(&OptionRow::ShowFps));
        assert_eq!(
            Some(vec![OptionsPage::Advanced, OptionsPage::Camera]),
            path_to(OptionRow::Camera)
        );
        assert_eq!(
            Some(vec![OptionsPage::Advanced, OptionsPage::Display]),
            path_to(OptionRow::ShowFps)
        );
        assert_eq!(Some(Vec::new()), path_to(OptionRow::RenderScale));
        assert_eq!(
            Some(vec![OptionsPage::Advanced, OptionsPage::Display]),
            path_to(OptionRow::Setting(SettingId::VideoResScale)),
            "the one setting with two entry points"
        );
    }

    #[test]
    fn the_region_row_cycles_auto_ntsc_and_pal() {
        let mut settings = advanced_settings();
        settings.set_auto_tv_mode(true);
        let mut model = HandheldOptionsModel::new();
        go_to(&mut model, OptionRow::Region, &mut settings);
        assert_eq!(RegionChoice::Auto, RegionChoice::of(&settings));
        model.handle(PadButton::Right, &mut settings);
        assert!(!settings.auto_tv_mode());
        assert_eq!(TvRegionSetting::Ntsc, settings.tv_mode());
        model.handle(PadButton::Right, &mut settings);
        assert_eq!(TvRegionSetting::Pal, settings.tv_mode());
        assert_eq!(
            "PAL",
            value_text(OptionRow::Region, &settings, CameraPreset::Orbit)
        );
        model.handle(PadButton::Right, &mut settings);
        assert!(settings.auto_tv_mode());
        model.handle(PadButton::Left, &mut settings);
        assert_eq!(RegionChoice::Pal, RegionChoice::of(&settings));
    }

    #[test]
    fn the_volume_steps_by_ten_percent_and_stops_at_the_ends() {
        let mut settings = advanced_settings();
        let mut model = HandheldOptionsModel::new();
        model.sync(&settings);
        go_to(&mut model, OptionRow::Volume, &mut settings);
        model.handle(PadButton::Right, &mut settings);
        assert_eq!(100, volume_percent(settings.volume()));
        model.handle(PadButton::Left, &mut settings);
        model.handle(PadButton::Left, &mut settings);
        assert_eq!(80, volume_percent(settings.volume()));
        for _ in 0..20 {
            model.handle(PadButton::Left, &mut settings);
        }
        assert_eq!(0, volume_percent(settings.volume()));
    }

    #[test]
    fn the_render_scale_row_switches_the_render_method_with_it() {
        let mut settings = advanced_settings();
        let mut model = HandheldOptionsModel::new();
        model.sync(&settings);
        go_to(&mut model, OptionRow::RenderScale, &mut settings);
        assert_eq!(0, render_scale_index(&settings));
        model.handle(PadButton::Right, &mut settings);
        assert_eq!(RENDER_TEXTURE_METHOD, settings.render_method());
        assert_eq!(1, settings.get(SettingId::VideoResScale).as_int());
        assert!((settings.res_scale_factor() - 2.0 / 3.0).abs() < 1.0e-6);
        model.handle(PadButton::Left, &mut settings);
        assert_eq!(DIRECT_METHOD, settings.render_method());
        assert!((settings.res_scale_factor() - 1.0).abs() < 1.0e-6);
        // Before Native comes the automatic resolution, then the smallest scale.
        model.handle(PadButton::Left, &mut settings);
        assert!(settings.get(SettingId::VideoDynamicResolution).as_bool());
        assert_eq!(RENDER_TEXTURE_METHOD, settings.render_method());
        assert_eq!(
            "Auto",
            value_text(OptionRow::RenderScale, &settings, CameraPreset::Orbit)
        );
        model.handle(PadButton::Left, &mut settings);
        assert!(!settings.get(SettingId::VideoDynamicResolution).as_bool());
        assert_eq!(
            4,
            settings.get(SettingId::VideoResScale).as_int(),
            "wraps round"
        );
    }

    #[test]
    fn switches_and_the_frame_cap_toggle() {
        let mut settings = advanced_settings();
        let mut model = HandheldOptionsModel::new();
        model.sync(&settings);
        go_to(&mut model, OptionRow::Shadows, &mut settings);
        let shadows = settings.shadow();
        model.handle(PadButton::A, &mut settings);
        assert_eq!(!shadows, settings.shadow());
        go_to(&mut model, OptionRow::FrameCap, &mut settings);
        model.handle(PadButton::A, &mut settings);
        assert_eq!(30, frame_cap(&settings));
        assert_eq!(2, settings.vsync());
        model.handle(PadButton::A, &mut settings);
        assert_eq!(60, frame_cap(&settings));
        assert_eq!(1, settings.vsync());
        go_to(&mut model, OptionRow::ShowFps, &mut settings);
        model.handle(PadButton::Right, &mut settings);
        assert!(settings.show_fps());
    }

    #[test]
    fn the_language_row_cycles_the_catalogue_languages() {
        let mut settings = advanced_settings();
        let mut model = HandheldOptionsModel::new();
        go_to(&mut model, OptionRow::Language, &mut settings);
        model.handle(PadButton::Right, &mut settings);
        assert_eq!("fr", settings.language());
        model.handle(PadButton::Right, &mut settings);
        assert_eq!("en", settings.language());
    }

    #[test]
    fn the_camera_and_controls_rows_ask_the_application() {
        let mut settings = advanced_settings();
        let mut model = HandheldOptionsModel::new();
        model.sync(&settings);
        go_to(&mut model, OptionRow::Camera, &mut settings);
        assert_eq!(
            Some(OptionsCommand::StepCamera(-1)),
            model.handle(PadButton::Left, &mut settings)
        );
        assert_eq!(
            Some(OptionsCommand::StepCamera(1)),
            model.handle(PadButton::A, &mut settings)
        );
        go_to(&mut model, OptionRow::Controls, &mut settings);
        assert_eq!(
            Some(OptionsCommand::OpenControls),
            model.handle(PadButton::A, &mut settings)
        );
        assert_eq!(None, model.handle(PadButton::Right, &mut settings));
        assert_eq!(
            Some(OptionsCommand::Close),
            model.handle(PadButton::B, &mut settings)
        );
    }
}
