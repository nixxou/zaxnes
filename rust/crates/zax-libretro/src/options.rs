//! The core options `RetroArch` shows (the libretro "variables"): their keys, their choices, and
//! how their values make [`Options`]. The first choice of each is its default.

use zax_nes::builtin_patches::{HackOption, COMMON_OPTIONS};
use zax_nes::ppu::SpriteLimit;
use zax_nes::NesRegion;
use zax_settings::catalogue::SettingId;
use zax_settings::core_options::{self, LibretroCoreOption};
use zax_settings::meta::{SettingEffect, SettingKind};
use zax_settings::presets::{PRESETS, V};

use crate::machine::{self, Machine, Variant};
use crate::picture::{self, Picture};

use crate::core::{Options, RegionChoice, ThreeDAspect};
use crate::three_d::RenderOptions;

/// The value of the choice named `text` among `choices`, `default` for none.
fn choice(choices: &[(&str, i32)], text: Option<String>, default: i32) -> i32 {
    text.and_then(|text| choices.iter().find(|(name, _)| *name == text))
        .map_or(default, |(_, value)| *value)
}

const WIDESCREEN: &str = "zaxnes_widescreen";
const RATIO: &str = "zaxnes_widescreen_ratio";
const ADVANCED: &str = "zaxnes_widescreen_advanced";
const REGION: &str = "zaxnes_region";
const SPRITE_LIMIT: &str = "zaxnes_sprite_limit";
const PALETTE: &str = "zaxnes_palette";
const RENDERER: &str = "zaxnes_renderer";
const RESOLUTION: &str = "zaxnes_3d_resolution";
const ANTI_ALIAS: &str = "zaxnes_3d_anti_alias";
const SHADOWS: &str = "zaxnes_3d_shadows";
const STYLE: &str = "zaxnes_3d_style";
const REFLECTIONS: &str = "zaxnes_3d_reflections";
const SKY: &str = "zaxnes_3d_sky";
const FAST_LUA: &str = "zaxnes_3d_fast_lua";
const EXTRA: &str = "zaxnes_widescreen_extra";
const SMOOTH: &str = "zaxnes_smooth_120";
const SMOOTH_AHEAD: &str = "zaxnes_smooth_ahead";
/// The 120 Hz scrolling in 3D as well.
const SMOOTH_3D: &str = "zaxnes_smooth_3d";
const STRICT: &str = "zaxnes_widescreen_strict_edges";
const FORCE: &str = "zaxnes_widescreen_other_versions";
const OVERSCAN: [&str; 4] = [
    "zaxnes_overscan_top",
    "zaxnes_overscan_bottom",
    "zaxnes_overscan_left",
    "zaxnes_overscan_right",
];
const OVERSCAN_NAMES: [&str; 4] = ["top", "bottom", "left", "right"];
/// The pixels a side of the overscan may leave out.
const OVERSCAN_PIXELS: [u8; 9] = [0, 4, 8, 12, 16, 20, 24, 28, 32];
/// The application's `WIDESCREEN_EXTRA_PERCENT`.
const EXTRA_PERCENT: [u16; 14] = [0, 5, 10, 15, 20, 25, 30, 40, 50, 75, 100, 150, 200, 300];

/// The anti-aliasing levels of `video.anti_alias`, by their value, the default first.
const ANTI_ALIASES: [(&str, i32); 4] = [("MSAA 4x", 1), ("Off", 0), ("MSAA 8x", 2), ("SMAA", 3)];
/// The skies of `scene.skybox`, by their value, the default first.
const SKIES: [(&str, i32); 4] = [
    ("Default", -1),
    ("Colour", 0),
    ("Gradient", 1),
    ("Fantasy", 2),
];

/// The 3D resolutions, by their stored value: the 16:9 size the option has always stored, of
/// which the height counts (the width follows the 3D's shape, `zaxnes_3d_aspect`).
const RESOLUTIONS: [(u32, u32); 8] = [
    (426, 240),
    (640, 360),
    (854, 480),
    (960, 540),
    (1280, 720),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
];
/// The resolution without a choice.
const DEFAULT_RESOLUTION: (u32, u32) = (1280, 720);
/// The shape of the 3D picture.
const ASPECT: &str = "zaxnes_3d_aspect";
const ASPECTS: [&str; 3] = ["16:9", "4:3", "Widescreen ratio"];
/// The prefix of the options of the game's hack: the option's own key follows.
const HACK: &str = "zaxnes_hack_";
/// The performance / fidelity preset (`zax_settings::presets`).
const PRESET: &str = "zaxnes_preset";
/// The presets' names, in the order of `PRESETS`.
const PRESET_NAMES: [&str; 5] = ["Fastest", "Fast", "Balanced", "Faithful", "Most faithful"];
/// The preset in force without a choice: Balanced, as in the application.
const DEFAULT_PRESET: usize = 2;
/// The value of an option a preset sets that leaves it to the preset.
const FOLLOW: &str = "preset";
/// The settings a preset writes that the core options carry through the catalogue.
const PRESET_CATALOGUE: [SettingId; 3] = [
    SettingId::VideoCompositeSignal,
    SettingId::EmuOamDecay,
    SettingId::EmuOamaddrWriteCorruption,
];
/// The settings a preset writes that no core option shows: the quality of the Enhanced style.
const PRESET_HIDDEN: [SettingId; 4] = [
    SettingId::RenderStyleSsrQuality,
    SettingId::RenderStyleAo,
    SettingId::RenderStyleAoQuality,
    SettingId::RenderStylePixelShadow,
];

/// The value preset `preset` writes for `id`, as the catalogue's number.
fn preset_value(preset: usize, id: SettingId) -> Option<i32> {
    PRESETS[preset.min(PRESETS.len() - 1)]
        .values
        .iter()
        .find(|(setting, ..)| *setting == id)
        .map(|(_, value, _)| match *value {
            V::B(on) => i32::from(on),
            V::I(number) | V::E(number) => number,
        })
}

/// A crosshair where a Zapper is aimed.
const CROSSHAIR: &str = "zaxnes_zapper_crosshair";

/// Which loop runs the console.
const CORE_VARIANT: &str = "zaxnes_core_variant";
const VARIANTS: [&str; 3] = ["Standard", "Fidelity+", "Fast"];

/// A line of explanation for each console setting (the application's tooltips, shortened).
const MACHINE_INFO: [(SettingId, &str); 20] = [
    (
        SettingId::EmuCartridgeDb,
        "Completes what an old iNES header cannot say (chip revision, save RAM, region) from a \
         database of about 10 000 dumps. Never changes the mapper.",
    ),
    (
        SettingId::EmuPowerOnRam,
        "What the work RAM holds at power-on. The FCEUX pattern is what a few games need; \
         Zeros is Mesen's; Random changes at every power-on.",
    ),
    (
        SettingId::EmuPowerOnPpuRam,
        "What the video chip's memories hold at power-on.",
    ),
    (
        SettingId::EmuCpuPpuAlignment,
        "How the processor's and the video chip's clocks line up at power-on (0 to 3, as \
         AccuracyCoin numbers them); a few defects depend on it.",
    ),
    (
        SettingId::EmuCpuApuParity,
        "Where the processor's and the sound unit's two-cycle rhythm starts at power-on.",
    ),
    (
        SettingId::EmuPpuRevision,
        "The video chip's revision. Before the 2C02G, the palette cannot be read back.",
    ),
    (
        SettingId::EmuCpuRevision,
        "The processor's revision; the letterless RP2A03 of the first Famicoms has a simpler \
         noise channel.",
    ),
    (
        SettingId::EmuMmc3Revision,
        "The MMC3 revision of a cartridge that has one; the cartridge's follows the header and \
         the database.",
    ),
    (
        SettingId::EmuOamCorruption,
        "Switching the picture off in the middle of a line damages a row of sprite memory, as \
         on the console.",
    ),
    (
        SettingId::EmuPpuBusConflicts,
        "A game reading video memory while the picture is drawn collides with the video \
         chip's own reads, as on the console.",
    ),
    (
        SettingId::EmuOamaddrBug,
        "A sprite memory address left at 8 or more copies eight bytes over the first eight \
         (2C02G and later, NTSC).",
    ),
    (
        SettingId::EmuOamaddrWriteCorruption,
        "A defect reported for the 2C02G; most consoles do not show it.",
    ),
    (
        SettingId::EmuOamDecay,
        "Sprite memory fades when the picture is off for long; sprites shown again unsent come \
         back scrambled.",
    ),
    (
        SettingId::EmuPowerOnTop,
        "The video chip starts at the top of the picture, as the console does.",
    ),
    (
        SettingId::EmuPpu2006Delay,
        "A new video address takes effect three dots later, as on the console.",
    ),
    (
        SettingId::EmuPpu2006Glitch,
        "A new video address landing on the tile step mixes the scroll, as on the console.",
    ),
    (
        SettingId::EmuPpu2000Glitch,
        "Control and scroll writes on precise dots glitch a line, on some consoles.",
    ),
    (
        SettingId::EmuPaletteCorruption,
        "Switching the picture off mid-line can change palette colours (alignment 2).",
    ),
    (
        SettingId::EmuPpuResetLine,
        "The front-loading NES: the video chip ignores the game's writes for half a frame after \
         power-on or Reset.",
    ),
    (
        SettingId::AudioConsoleProfile,
        "The sound through the filters of one real console (measured by BeesNES); None keeps \
         the usual sound.",
    ),
];

/// A line of explanation for each picture setting.
const PICTURE_INFO: [(SettingId, &str); 13] = [
    (
        SettingId::VideoTvModel,
        "The television the colours go through: a modern screen, or the tube of a NTSC, \
         Japanese or PAL set.",
    ),
    (SettingId::VideoTvHue, "The television's tint knob."),
    (
        SettingId::VideoTvSaturation,
        "The television's colour knob.",
    ),
    (
        SettingId::VideoTvContrast,
        "The television's contrast knob.",
    ),
    (
        SettingId::VideoTvBrightness,
        "The television's brightness knob.",
    ),
    (
        SettingId::VideoCompositeSignal,
        "Decodes the picture from the console's composite signal, as a television did: colour \
         bleeding, dot crawl. 2D only.",
    ),
    (
        SettingId::VideoSignalKind,
        "The cable: composite, S-Video (sharper) or RF (the aerial plug, with snow and ghost).",
    ),
    (
        SettingId::VideoDecoderSharpness,
        "How sharp the television's decoder is.",
    ),
    (
        SettingId::VideoCombFilter,
        "A comb filter, as better televisions had: less colour bleeding on edges.",
    ),
    (
        SettingId::VideoFrozenDotCrawl,
        "Every frame at the same signal phase: no dot crawl.",
    ),
    (
        SettingId::VideoRfNoise,
        "The snow of the RF signal, in percent.",
    ),
    (
        SettingId::VideoRfGhost,
        "The ghost of the RF signal, in percent.",
    ),
    (
        SettingId::VideoEmphasis3d,
        "The 3D scene takes the picture's most common colour emphasis (some games tint the \
         screen with it).",
    ),
];

/// The prefix of the options common to every game's hack (`COMMON_OPTIONS`).
const AID: &str = "zaxnes_aid_";
/// A common option's value that leaves each pack its own default.
const PACK_DEFAULT: &str = "pack default";
/// A game's hack option's value that follows the common option of the same key.
const COMMON_SETTING: &str = "common setting";

const RATIOS: [(&str, f32); 4] = [
    ("16:9", 16.0 / 9.0),
    ("16:10", 16.0 / 10.0),
    ("3:2", 3.0 / 2.0),
    ("21:9", 21.0 / 9.0),
];

/// The palettes, by their index in `zax_engine3d::palette`.
const PALETTES: [&str; 11] = [
    "Zaxnes",
    "Mesen",
    "FCEUX",
    "Nestopia YUV",
    "Nestopia RGB",
    "Smooth FBX",
    "NES Classic FBX",
    "Composite Direct FBX",
    "PVM Style FBX",
    "Sony CXA2025AS",
    "Wavebeam",
];

/// A submenu of the core options (libretro's option categories, v2): its key, its name and its
/// line of explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Category {
    pub key: &'static str,
    pub label: &'static str,
    pub info: &'static str,
}

/// The submenus, in the order the frontend lists them.
pub const CATEGORIES: [Category; 5] = [
    Category {
        key: "image",
        label: "Image",
        info: "The picture of the console: palette, sprites, overscan.",
    },
    Category {
        key: "three_d",
        label: "3D",
        info: "The 3D of the games that have a profile.",
    },
    Category {
        key: "widescreen",
        label: "Widescreen and scrolling",
        info: "The widescreen of the games with a widescreen hack, and the 120 Hz scrolling.",
    },
    Category {
        key: "hacks",
        label: "Hack aids",
        info: "The options of the game's hack (aids, testing aids).",
    },
    Category {
        key: "emulation",
        label: "Emulation",
        info: "The console itself.",
    },
];

/// One core option: its key (never renamed, the frontend keeps its value under it), its name in
/// the flat list and in its submenu, its line of explanation, its submenu, and its values, each
/// with the label shown (none: the value itself). The first value is the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub key: String,
    pub label: String,
    pub label_in_category: String,
    pub info: String,
    pub category: &'static str,
    pub values: Vec<(String, Option<String>)>,
    pub default: String,
}

impl Declaration {
    fn new(key: &str, label: &str, in_category: &str, category: &'static str, info: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            label_in_category: in_category.to_owned(),
            info: info.to_owned(),
            category,
            values: Vec::new(),
            default: String::new(),
        }
    }

    /// With these values, shown as they are.
    fn values<S: AsRef<str>>(mut self, values: impl IntoIterator<Item = S>) -> Self {
        self.values = values
            .into_iter()
            .map(|value| (value.as_ref().to_owned(), None))
            .collect();
        self.default = self
            .values
            .first()
            .map(|(value, _)| value.clone())
            .unwrap_or_default();
        self
    }

    /// With a first value, its default, that leaves the option to the preset.
    fn following_preset(mut self) -> Self {
        self.values
            .insert(0, (FOLLOW.to_owned(), Some("Preset".to_owned())));
        FOLLOW.clone_into(&mut self.default);
        self
    }

    /// A switch, on by default when `on`.
    fn switch(self, on: bool) -> Self {
        self.values(if on {
            ["enabled", "disabled"]
        } else {
            ["disabled", "enabled"]
        })
    }

    /// The default value.
    #[must_use]
    pub fn default_value(&self) -> &str {
        &self.default
    }

    /// The declaration of the libretro API before categories (`SET_VARIABLES`):
    /// `"Name; value|value"`, the default first.
    #[must_use]
    pub fn legacy(&self) -> String {
        let values: Vec<&str> = std::iter::once(self.default.as_str())
            .chain(
                self.values
                    .iter()
                    .map(|(value, _)| value.as_str())
                    .filter(|value| *value != self.default),
            )
            .collect();
        format!("{}; {}", self.label, values.join("|"))
    }
}

/// The settings of the application the core exposes with the catalogue's own libretro values
/// (`zax_settings::core_options`, keys `zaxnes_<setting>`), so that their steps are the
/// application's: the setting, its name in the flat list and in its submenu, its submenu, and its
/// line of explanation.
const CATALOGUE: [(SettingId, &str, &str, &str, &str); 7] = [
    (
        SettingId::CameraFov,
        "3D field of view",
        "Field of view",
        "three_d",
        "The camera's angle of view, in degrees; a profile may set its own.",
    ),
    (
        SettingId::LightIntensity,
        "3D light",
        "Light",
        "three_d",
        "The strength of the scene's light.",
    ),
    (
        SettingId::SceneBendLr,
        "3D bend left-right",
        "Bend left-right",
        "three_d",
        "Curves the scene from left to right, in degrees.",
    ),
    (
        SettingId::SceneBendTb,
        "3D bend top-bottom",
        "Bend top-bottom",
        "three_d",
        "Curves the scene from top to bottom, in degrees.",
    ),
    (
        SettingId::SceneArcH,
        "3D arc horizontal",
        "Arc horizontal",
        "three_d",
        "Rolls the scene round a horizontal arc, in degrees.",
    ),
    (
        SettingId::SceneArcCenter,
        "3D arc centre",
        "Arc centre",
        "three_d",
        "Where the arc's pivot is, from the screen's centre to its edge.",
    ),
    (
        SettingId::SceneBend2dMode,
        "3D bend the 2D picture too",
        "Bend the 2D picture too",
        "three_d",
        "The 2D picture shown in the 3D view bends like the scene; off keeps it flat.",
    ),
];

/// The catalogue's core option of `setting`.
fn catalogue_option(setting: SettingId) -> Option<&'static LibretroCoreOption> {
    core_options::all()
        .iter()
        .find(|option| option.setting == setting)
}

/// The options to declare, the game's hack options (`hacks`, each with its default) after the
/// others.
#[must_use]
#[allow(clippy::too_many_lines)] // One entry per option.
pub fn declarations(hacks: &[HackOption]) -> Vec<Declaration> {
    let resolutions = RESOLUTIONS.map(|(width, height)| format!("{width}x{height}"));
    let mut preset = Declaration::new(
        PRESET,
        "Preset",
        "Preset",
        "",
        "Performance or fidelity, as the application's presets: Fastest and Fast for handhelds \
         (fast core, lighter 3D), Balanced, Faithful and Most faithful (Fidelity+, the console's \
         defects). Each option set to Preset follows it; another value wins. What a core has no \
         use for (run-ahead, catch-up, automatic resolution) is left to the frontend.",
    )
    .values(PRESET_NAMES);
    PRESET_NAMES[DEFAULT_PRESET].clone_into(&mut preset.default);
    let mut list = vec![
        preset,
        // Image.
        Declaration::new(
            PALETTE,
            "Palette",
            "Palette",
            "image",
            "The colours of the console's picture.",
        )
        .values(PALETTES),
        Declaration::new(
            SPRITE_LIMIT,
            "Sprite limit",
            "Sprite limit",
            "image",
            "8 per line is the console's (sprites flicker); Removed draws them all; Smart removes \
             it only where the game does not rely on it.",
        )
        .values(["8 per line", "Removed", "Smart"]),
    ];
    for (key, name) in OVERSCAN.iter().zip(OVERSCAN_NAMES) {
        list.push(
            Declaration::new(
                key,
                &format!("Overscan {name}"),
                &format!("Overscan {name}"),
                "image",
                "The pixels left out on that side, as a TV hid them. Left and right only \
                 without widescreen.",
            )
            .values(OVERSCAN_PIXELS.map(|pixels| format!("{pixels} px"))),
        );
    }
    list.extend([
        // 3D.
        Declaration::new(
            RENDERER,
            "Renderer (restart)",
            "Renderer (restart)",
            "three_d",
            "3D for the games that have a 3D profile (in RetroArch's system folder, \
             Zaxnes/profiles, or carried by the core), or 2D only. Needs the glcore or gl video \
             driver.",
        )
        .values(["3D for games with a profile", "2D only"]),
        {
            let mut resolution = Declaration::new(
                RESOLUTION,
                "3D resolution (restart)",
                "Resolution (restart)",
                "three_d",
                "The height the 3D is drawn at; 240p to 480p give the look of the 3D consoles of \
                 the 1990s and suit a CRT. The width follows the 3D aspect ratio.",
            )
            .values(resolutions);
            for (value, label) in &mut resolution.values {
                *label = value
                    .split_once('x')
                    .map(|(_, height)| format!("{height}p"));
            }
            resolution.default = format!("{}x{}", DEFAULT_RESOLUTION.0, DEFAULT_RESOLUTION.1);
            resolution
        },
        Declaration::new(
            ASPECT,
            "3D aspect ratio (restart)",
            "Aspect ratio (restart)",
            "three_d",
            "The shape of the 3D picture: 16:9; 4:3 (bezels made for 4:3, a CRT); or the \
             widescreen ratio for a widened game, 4:3 for the others.",
        )
        .values(ASPECTS),
        Declaration::new(
            ANTI_ALIAS,
            "3D anti-aliasing",
            "Anti-aliasing",
            "three_d",
            "Smooths the edges of the 3D.",
        )
        .values(ANTI_ALIASES.map(|(name, _)| name)),
        Declaration::new(
            SHADOWS,
            "3D shadows",
            "Shadows",
            "three_d",
            "The shadows the blocks cast.",
        )
        .switch(true)
        .following_preset(),
        Declaration::new(
            STYLE,
            "3D visual style",
            "Visual style",
            "three_d",
            "Classic is 3dSen's look; Enhanced adds the application's lighting and occlusion.",
        )
        .values(["Classic", "Enhanced"])
        .following_preset(),
        Declaration::new(
            REFLECTIONS,
            "3D reflections (Enhanced style)",
            "Reflections (Enhanced style)",
            "three_d",
            "Reflections on the floor, with the Enhanced style only.",
        )
        .switch(false)
        .following_preset(),
        Declaration::new(
            SKY,
            "3D sky",
            "Sky",
            "three_d",
            "The sky behind the scene; Default is the profile's.",
        )
        .values(SKIES.map(|(name, _)| name)),
        Declaration::new(
            FAST_LUA,
            "3D optimised Lua (restart, unsafe code)",
            "Optimised Lua (restart, unsafe code)",
            "three_d",
            "Faster paths for the profiles' Lua scripts, written with unsafe code.",
        )
        .switch(false)
        .following_preset(),
        // Widescreen and scrolling.
        Declaration::new(
            WIDESCREEN,
            "Widescreen (games with a widescreen hack)",
            "Widescreen",
            "widescreen",
            "Widens the games Zaxnes has a widescreen hack for; the others keep their 4:3 \
             picture.",
        )
        .switch(true),
        Declaration::new(
            RATIO,
            "Widescreen ratio",
            "Ratio",
            "widescreen",
            "The shape of the widened picture. Keep RetroArch's aspect ratio on Core provided.",
        )
        .values(RATIOS.map(|(name, _)| name)),
        Declaration::new(
            ADVANCED,
            "Widescreen advanced mode",
            "Advanced mode",
            "widescreen",
            "Applies the games' advanced hacks too (more of the game widened, objects in the \
             margins).",
        )
        .switch(true),
        Declaration::new(
            EXTRA,
            "Widescreen extra margin",
            "Extra margin",
            "widescreen",
            "The margins and the height that much larger, so that the 3D objects at the edges \
             are whole.",
        )
        .values(EXTRA_PERCENT.map(|percent| format!("{percent}%"))),
        Declaration::new(
            STRICT,
            "Widescreen strict edges (3D)",
            "Strict edges (3D)",
            "widescreen",
            "In 3D, cuts what lies past the widescreen margins.",
        )
        .switch(false),
        Declaration::new(
            FORCE,
            "Widescreen on other versions of a game (restart)",
            "Other versions of a game (restart)",
            "widescreen",
            "Forces the hack of another version of the game on this ROM (ROM hacks, \
             translations). May fail.",
        )
        .switch(false),
        Declaration::new(
            SMOOTH,
            "Smooth scrolling 120 Hz",
            "Smooth scrolling 120 Hz",
            "widescreen",
            "In 2D, a picture between two frames along the camera's motion (in 3D too with the \
             option below). Needs a display at 120 Hz with RetroArch's refresh rate set to it (at 60 Hz the game runs at half \
             speed). Leave RetroArch's run-ahead off.",
        )
        .switch(false),
        Declaration::new(
            SMOOTH_AHEAD,
            "Smooth scrolling of every game",
            "Smooth scrolling of every game",
            "widescreen",
            "For the games whose hack does not report its camera: the motion found by running a \
             frame ahead.",
        )
        .switch(true),
        Declaration::new(
            SMOOTH_3D,
            "Smooth scrolling in 3D too",
            "Smooth scrolling in 3D too",
            "widescreen",
            "The 120 Hz scrolling slides the 3D scene's playfield as well, the 3D drawn twice a \
             frame.",
        )
        .switch(false),
        // Emulation.
        Declaration::new(
            REGION,
            "Region",
            "Region",
            "emulation",
            "The console the game runs on; Auto follows the ROM.",
        )
        .values(["Auto", "NTSC", "PAL", "Dendy"]),
    ]);
    for (setting, label, in_category, category, info) in CATALOGUE {
        let Some(option) = catalogue_option(setting) else {
            continue;
        };
        let mut declaration = Declaration::new(&option.key, label, in_category, category, info)
            .values(&option.values);
        declaration.default.clone_from(&option.default_value);
        list.push(declaration);
    }
    for (key, _) in COMMON_OPTIONS {
        let label = common_label(key);
        list.push(
            Declaration::new(
                &format!("{AID}{key}"),
                &format!("All games: {label}"),
                &format!("All games: {label}"),
                "hacks",
                "The default of this aid in every game whose hack has it; a game's own option \
                 below, when not on Common setting, wins.",
            )
            .values([PACK_DEFAULT, "enabled", "disabled"]),
        );
    }
    list.push(
        Declaration::new(
            CORE_VARIANT,
            "Core variant",
            "Core variant",
            "emulation",
            "Standard; Fidelity+ (the accesses on the clock edges of the chips' transistor-level \
             model: closest to the console, slower, NTSC only so far); Fast (several times \
             faster on the common boards, the same pictures and sounds).",
        )
        .values(VARIANTS)
        .following_preset(),
    );
    list.push(
        Declaration::new(
            CROSSHAIR,
            "Zapper crosshair",
            "Zapper crosshair",
            "emulation",
            "A crosshair where the Zapper is aimed, in 2D and in 3D. Plug \
             the Zapper into a port in RetroArch's Controls (device type Zapper) and aim with a \
             light gun or the mouse; in 3D it aims through the camera, as in the application.",
        )
        .switch(true),
    );
    for id in machine::SETTINGS {
        if let Some(declaration) = catalogue_declaration(id, "emulation") {
            list.push(declaration);
        }
    }
    for id in picture::SETTINGS {
        let category = if id == SettingId::VideoEmphasis3d {
            "three_d"
        } else {
            "image"
        };
        if let Some(declaration) = catalogue_declaration(id, category) {
            list.push(declaration);
        }
    }
    for option in hacks {
        let common = COMMON_OPTIONS.iter().any(|(key, _)| *key == option.key);
        let declaration = Declaration::new(
            &format!("{HACK}{}", option.key),
            &format!("Hack: {}", option.label),
            option.label,
            "hacks",
            if common {
                "An option of this game's hack. Common setting follows the All games option of \
                 the same name."
            } else {
                "An option of this game's hack."
            },
        );
        list.push(if common {
            declaration.values([COMMON_SETTING, "enabled", "disabled"])
        } else {
            declaration.switch(option.default)
        });
    }
    list
}

/// The steps the core offers for the catalogue settings that are a numeric range: the setting,
/// its lowest and highest values and the step (the defaults fall on a step).
const RANGES: [(SettingId, i32, i32, i32); 6] = [
    (SettingId::VideoTvHue, -30, 30, 5),
    (SettingId::VideoTvSaturation, 0, 200, 10),
    (SettingId::VideoTvContrast, 50, 150, 5),
    (SettingId::VideoTvBrightness, -50, 50, 5),
    (SettingId::VideoRfNoise, 0, 100, 10),
    (SettingId::VideoRfGhost, 0, 100, 5),
];

/// The values of the range setting `id`, from [`RANGES`].
fn range_values(id: SettingId) -> Option<Vec<i32>> {
    RANGES
        .iter()
        .find(|(setting, ..)| *setting == id)
        .map(|&(_, low, high, step)| {
            (0..=(high - low) / step)
                .map(|index| low + index * step)
                .collect()
        })
}

/// The explanation of the console or picture setting `id`.
fn catalogue_info(id: SettingId) -> &'static str {
    if matches!(
        id,
        SettingId::EmuVsDip1
            | SettingId::EmuVsDip2
            | SettingId::EmuVsDip3
            | SettingId::EmuVsDip4
            | SettingId::EmuVsDip5
            | SettingId::EmuVsDip6
            | SettingId::EmuVsDip7
            | SettingId::EmuVsDip8
    ) {
        return "A DIP switch of a VS. System cabinet; each game gives them its own meaning \
                (difficulty, lives, coins). VS. System games only.";
    }
    MACHINE_INFO
        .iter()
        .chain(PICTURE_INFO.iter())
        .find(|(setting, _)| *setting == id)
        .map_or("", |(_, info)| info)
}

/// The core option of the setting `id`, from the catalogue: its label, its choices (a switch,
/// the catalogue's choices or the steps of [`RANGES`]), its default, and "(restart)" when it
/// waits for the next load.
fn catalogue_declaration(id: SettingId, category: &'static str) -> Option<Declaration> {
    let descriptor = zax_settings::catalogue::descriptor(id);
    let label = descriptor.ui_label()?;
    let later = matches!(
        descriptor.effect(),
        SettingEffect::NextRomLoad | SettingEffect::NextGameLoad | SettingEffect::Restart
    );
    let label = if later {
        format!("{label} (restart)")
    } else {
        label.to_owned()
    };
    let key = core_options::core_option_key(&descriptor.key());
    let default = machine::default_value(id);
    let declaration = Declaration::new(&key, &label, &label, category, catalogue_info(id));
    if descriptor.kind() == SettingKind::Bool {
        let declaration = declaration.switch(default != 0);
        return Some(if PRESET_CATALOGUE.contains(&id) {
            declaration.following_preset()
        } else {
            declaration
        });
    }
    if let Some(steps) = range_values(id) {
        let mut declaration = declaration.values(steps.iter().map(ToString::to_string));
        declaration.default = default.to_string();
        return Some(declaration);
    }
    let choices = descriptor.domain().options()?;
    let mut declaration = declaration.values(choices.iter().map(|choice| choice.label.as_str()));
    if let Some(choice) = choices
        .iter()
        .find(|choice| choice_value(choice.value) == default)
    {
        declaration.default.clone_from(&choice.label);
    }
    Some(declaration)
}

/// The value the option text `text` gives the catalogue setting `id`, none when it is not one
/// of its values.
fn catalogue_value(id: SettingId, text: &str) -> Option<i32> {
    let descriptor = zax_settings::catalogue::descriptor(id);
    if descriptor.kind() == SettingKind::Bool {
        return match text {
            "enabled" => Some(1),
            "disabled" => Some(0),
            _ => None,
        };
    }
    if let Some(steps) = range_values(id) {
        return text.parse().ok().filter(|value| steps.contains(value));
    }
    descriptor
        .domain()
        .options()?
        .iter()
        .find(|choice| choice.label == text)
        .map(|choice| choice_value(choice.value))
}

/// The values the options give the catalogue settings `ids`.
fn read_values(ids: &[SettingId], value: &dyn Fn(&str) -> Option<String>) -> Vec<(SettingId, i32)> {
    ids.iter()
        .filter_map(|&id| {
            let descriptor = zax_settings::catalogue::descriptor(id);
            let text = value(&core_options::core_option_key(&descriptor.key()))?;
            catalogue_value(id, &text).map(|number| (id, number))
        })
        .collect()
}

/// A catalogue choice's value as the integer the setting holds.
fn choice_value(value: f64) -> i32 {
    // The choices are small whole numbers.
    #[allow(clippy::cast_possible_truncation)]
    let value = value.round() as i32;
    value
}

/// The console settings the values give.
fn read_machine(value: &dyn Fn(&str) -> Option<String>) -> Machine {
    let variant = match value(CORE_VARIANT).as_deref() {
        Some("Fidelity+") => Variant::FidelityPlus,
        Some("Fast") => Variant::Fast,
        _ => Variant::Standard,
    };
    let values = read_values(&machine::SETTINGS, value);
    Machine { variant, values }
}

/// Gives `options` what preset `preset` writes wherever its option is on Preset (or has no
/// value): the core variant, the 3D's style, shadows, reflections and optimised Lua, the
/// composite signal, two console defects, and the Enhanced style's quality.
fn follow_preset(options: &mut Options, preset: usize, value: &dyn Fn(&str) -> Option<String>) {
    let follows = |key: &str| value(key).is_none_or(|text| text == FOLLOW);
    let on = |id: SettingId| preset_value(preset, id).map(|number| number != 0);
    if follows(CORE_VARIANT) {
        options.machine.variant = if on(SettingId::EmuFidelityPlus) == Some(true) {
            Variant::FidelityPlus
        } else if on(SettingId::EmuFastCore) == Some(true) {
            Variant::Fast
        } else {
            Variant::Standard
        };
    }
    let render = &mut options.render;
    if follows(SHADOWS) {
        render.shadows = on(SettingId::RenderShadow).unwrap_or(render.shadows);
    }
    if follows(STYLE) {
        render.enhanced = preset_value(preset, SettingId::RenderVisualStyle)
            .map_or(render.enhanced, |style| style != 0);
    }
    if follows(REFLECTIONS) {
        render.reflections = on(SettingId::RenderStyleReflections).unwrap_or(render.reflections);
    }
    if follows(FAST_LUA) {
        options.optimized_lua = on(SettingId::EmuLuaOptimized).unwrap_or(options.optimized_lua);
    }
    render.preset = PRESET_HIDDEN
        .iter()
        .filter_map(|&id| preset_value(preset, id).map(|number| (id, number)))
        .collect();
    for id in PRESET_CATALOGUE {
        let key = core_options::core_option_key(&zax_settings::catalogue::descriptor(id).key());
        if !follows(&key) {
            continue;
        }
        let Some(number) = preset_value(preset, id) else {
            continue;
        };
        let values = if id == SettingId::VideoCompositeSignal {
            &mut options.picture.values
        } else {
            &mut options.machine.values
        };
        values.retain(|(setting, _)| *setting != id);
        values.push((id, number));
    }
}

/// The label of the common option `key`: the one a pack gives it.
fn common_label(key: &str) -> String {
    zax_nes::builtin_patches::patches()
        .iter()
        .flat_map(|patch| patch.options)
        .find(|option| option.key == key)
        .map_or_else(|| key.replace('_', " "), |option| option.label.to_owned())
}

/// The options the values give: `value` answers a key with its value, none when the frontend
/// has none; `hacks` are the game's hack options.
#[must_use]
pub fn read(value: &dyn Fn(&str) -> Option<String>, hacks: &[HackOption]) -> Options {
    let preset = value(PRESET)
        .and_then(|text| PRESET_NAMES.iter().position(|name| *name == text))
        .unwrap_or(DEFAULT_PRESET);
    let mut options = read_explicit(value, hacks);
    follow_preset(&mut options, preset, value);
    options
}

/// The options as the values give them, a preset aside.
fn read_explicit(value: &dyn Fn(&str) -> Option<String>, hacks: &[HackOption]) -> Options {
    let defaults = Options::default();
    let enabled = |key: &str, default: bool| value(key).map_or(default, |text| text == "enabled");
    Options {
        widescreen: enabled(WIDESCREEN, defaults.widescreen),
        ratio: value(RATIO)
            .and_then(|text| RATIOS.iter().find(|(name, _)| *name == text))
            .map_or(defaults.ratio, |(_, ratio)| *ratio),
        advanced: enabled(ADVANCED, defaults.advanced),
        region: match value(REGION).as_deref() {
            Some("NTSC") => RegionChoice::Forced(NesRegion::Ntsc),
            Some("PAL") => RegionChoice::Forced(NesRegion::Pal),
            Some("Dendy") => RegionChoice::Forced(NesRegion::Dendy),
            _ => RegionChoice::Auto,
        },
        sprite_limit: match value(SPRITE_LIMIT).as_deref() {
            Some("Removed") => SpriteLimit::Removed,
            Some("Smart") => SpriteLimit::Smart,
            _ => SpriteLimit::Kept,
        },
        palette: value(PALETTE)
            .and_then(|text| PALETTES.iter().position(|name| *name == text))
            .and_then(|index| u8::try_from(index).ok())
            .unwrap_or(0),
        three_d: value(RENDERER).is_none_or(|text| text != "2D only"),
        resolution: value(RESOLUTION)
            .and_then(|text| {
                RESOLUTIONS
                    .iter()
                    .find(|(width, height)| format!("{width}x{height}") == text)
                    .copied()
            })
            .unwrap_or(defaults.resolution),
        three_d_aspect: match value(ASPECT).as_deref() {
            Some("4:3") => ThreeDAspect::FourThree,
            Some("Widescreen ratio") => ThreeDAspect::Widescreen,
            _ => ThreeDAspect::Wide,
        },
        render: RenderOptions {
            anti_alias: choice(&ANTI_ALIASES, value(ANTI_ALIAS), defaults.render.anti_alias),
            shadows: enabled(SHADOWS, defaults.render.shadows),
            enhanced: value(STYLE).map_or(defaults.render.enhanced, |text| text == "Enhanced"),
            reflections: enabled(REFLECTIONS, defaults.render.reflections),
            skybox: choice(&SKIES, value(SKY), defaults.render.skybox),
            preset: Vec::new(),
            catalogue: CATALOGUE
                .iter()
                .filter_map(|(setting, ..)| catalogue_option(*setting))
                .map(|option| {
                    let chosen = value(&option.key)
                        .filter(|text| option.values.contains(text))
                        .unwrap_or_else(|| option.default_value.clone());
                    (option.key.clone(), chosen)
                })
                .collect(),
        },
        optimized_lua: enabled(FAST_LUA, defaults.optimized_lua),
        extra_percent: value(EXTRA)
            .and_then(|text| text.trim_end_matches('%').parse().ok())
            .filter(|percent| EXTRA_PERCENT.contains(percent))
            .unwrap_or(defaults.extra_percent),
        strict_edges: enabled(STRICT, defaults.strict_edges),
        smooth_120: enabled(SMOOTH, defaults.smooth_120),
        smooth_ahead: enabled(SMOOTH_AHEAD, defaults.smooth_ahead),
        smooth_3d: enabled(SMOOTH_3D, defaults.smooth_3d),
        force_patch: enabled(FORCE, defaults.force_patch),
        overscan: OVERSCAN.map(|key| {
            value(key)
                .and_then(|text| text.trim_end_matches(" px").parse().ok())
                .filter(|pixels| OVERSCAN_PIXELS.contains(pixels))
                .unwrap_or(0)
        }),
        machine: read_machine(value),
        zapper_crosshair: enabled(CROSSHAIR, defaults.zapper_crosshair),
        picture: Picture {
            values: read_values(&picture::SETTINGS, value),
        },
        hacks: hacks
            .iter()
            .map(|option| {
                let own = value(&format!("{HACK}{}", option.key));
                let on = match own.as_deref() {
                    Some("enabled") => true,
                    Some("disabled") => false,
                    // Common setting, or none: the common option, else the pack's default.
                    _ => match value(&format!("{AID}{}", option.key)).as_deref() {
                        Some("enabled") => true,
                        Some("disabled") => false,
                        _ => option.default,
                    },
                };
                (option.key.to_owned(), on)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTION: HackOption = HackOption {
        key: "invincible",
        label: "Invincible",
        label_fr: "Invincible",
        bit: 8,
        default: false,
    };

    #[test]
    fn the_first_choice_is_the_default_and_values_come_back() {
        let list = declarations(&[OPTION]);
        assert!(list
            .iter()
            .any(|option| option.key == "zaxnes_hack_invincible"
                && option.legacy() == "Hack: Invincible; common setting|enabled|disabled"
                && option.category == "hacks"));
        let none = read_explicit(&|_| None, &[OPTION]);
        assert_eq!(none.render.catalogue.len(), CATALOGUE.len());
        assert_eq!(
            none,
            Options {
                hacks: vec![("invincible".to_owned(), false)],
                render: RenderOptions {
                    catalogue: none.render.catalogue.clone(),
                    ..RenderOptions::default()
                },
                ..Options::default()
            }
        );
        let set = read_explicit(
            &|key| match key {
                "zaxnes_widescreen" => Some("disabled".to_owned()),
                "zaxnes_widescreen_ratio" => Some("21:9".to_owned()),
                "zaxnes_region" => Some("PAL".to_owned()),
                "zaxnes_palette" => Some("Mesen".to_owned()),
                "zaxnes_hack_invincible" => Some("enabled".to_owned()),
                "zaxnes_3d_anti_alias" => Some("SMAA".to_owned()),
                "zaxnes_3d_style" => Some("Enhanced".to_owned()),
                "zaxnes_3d_sky" => Some("Fantasy".to_owned()),
                "zaxnes_widescreen_extra" => Some("25%".to_owned()),
                "zaxnes_overscan_top" => Some("8 px".to_owned()),
                _ => None,
            },
            &[OPTION],
        );
        assert!(!set.widescreen);
        assert!((set.ratio - 21.0 / 9.0).abs() < 1e-6);
        assert_eq!(set.region, RegionChoice::Forced(NesRegion::Pal));
        assert_eq!(set.palette, 1);
        assert_eq!(set.hacks, vec![("invincible".to_owned(), true)]);
        assert_eq!(set.extra_percent, 25);
        assert_eq!(set.overscan, [8, 0, 0, 0]);
        assert_eq!(
            set.render,
            RenderOptions {
                anti_alias: 3,
                enhanced: true,
                skybox: 2,
                catalogue: none.render.catalogue.clone(),
                ..RenderOptions::default()
            }
        );
    }

    /// The 3D settings of the catalogue keep the application's values and default; a value the
    /// list does not hold falls back to the default.
    #[test]
    fn the_catalogue_settings_come_with_their_values() {
        let list = declarations(&[]);
        let fov = list
            .iter()
            .find(|option| option.key == "zaxnes_camera_fov")
            .expect("the field of view is declared");
        assert_eq!(fov.category, "three_d");
        assert_eq!(fov.default_value(), "70");
        assert!(fov.legacy().starts_with("3D field of view; 70|"));
        let read_back = read(
            &|key| match key {
                "zaxnes_camera_fov" => Some("90".to_owned()),
                "zaxnes_light_intensity" => Some("not a value".to_owned()),
                _ => None,
            },
            &[],
        );
        let chosen = |key: &str| {
            read_back
                .render
                .catalogue
                .iter()
                .find(|(option, _)| option == key)
                .map(|(_, label)| label.clone())
        };
        assert_eq!(chosen("zaxnes_camera_fov").as_deref(), Some("90"));
        assert_eq!(chosen("zaxnes_light_intensity").as_deref(), Some("1.00"));
    }

    /// A game's aid follows its own option, else the All games one, else the pack's default.
    #[test]
    fn a_game_option_wins_over_the_common_one_over_the_pack() {
        let on = |values: &[(&str, &str)]| {
            let options = read(
                &|key| {
                    values
                        .iter()
                        .find(|(name, _)| *name == key)
                        .map(|(_, value)| (*value).to_owned())
                },
                &[OPTION],
            );
            options.hacks[0].1
        };
        assert!(!on(&[]));
        assert!(on(&[("zaxnes_aid_invincible", "enabled")]));
        assert!(on(&[
            ("zaxnes_aid_invincible", "enabled"),
            ("zaxnes_hack_invincible", "common setting"),
        ]));
        assert!(!on(&[
            ("zaxnes_aid_invincible", "enabled"),
            ("zaxnes_hack_invincible", "disabled"),
        ]));
        assert!(on(&[
            ("zaxnes_aid_invincible", "pack default"),
            ("zaxnes_hack_invincible", "enabled"),
        ]));
        let list = declarations(&[]);
        assert!(list
            .iter()
            .any(|option| option.key == "zaxnes_aid_margin_freeze"
                && option.default_value() == "pack default"));
    }

    /// The console settings come from the catalogue: every default is one of the option's
    /// values, and a value read back is the catalogue's.
    #[test]
    fn the_console_settings_come_from_the_catalogue() {
        let list = declarations(&[]);
        let emulation: Vec<&Declaration> = list
            .iter()
            .filter(|option| option.category == "emulation")
            .collect();
        assert!(emulation.len() > machine::SETTINGS.len());
        for option in &emulation {
            assert!(
                option
                    .values
                    .iter()
                    .any(|(value, _)| *value == option.default),
                "{}: default {} not among its values",
                option.key,
                option.default
            );
        }
        let ram = list
            .iter()
            .find(|option| option.key == "zaxnes_emu_power_on_ram")
            .expect("the power-on RAM is declared");
        assert_eq!(ram.default, "FCEUX pattern");
        assert!(ram.label.ends_with("(restart)"));
        let options = read(
            &|key| match key {
                "zaxnes_core_variant" => Some("Fast".to_owned()),
                "zaxnes_emu_power_on_ram" => Some("Zeros".to_owned()),
                "zaxnes_emu_oam_decay" => Some("disabled".to_owned()),
                _ => None,
            },
            &[],
        );
        assert_eq!(options.machine.variant, Variant::Fast);
        assert!(options
            .machine
            .values
            .contains(&(SettingId::EmuPowerOnRam, 0)));
        assert!(options
            .machine
            .values
            .contains(&(SettingId::EmuOamDecay, 0)));
    }

    /// The picture settings: a numeric range comes in steps whose default is one, and a value
    /// read back off the steps falls back to the default.
    #[test]
    fn the_picture_settings_come_in_steps() {
        let list = declarations(&[]);
        let saturation = list
            .iter()
            .find(|option| option.key == "zaxnes_video_tv_saturation")
            .expect("the saturation is declared");
        assert_eq!(saturation.category, "image");
        assert_eq!(saturation.default, "100");
        assert!(saturation.values.iter().any(|(value, _)| value == "150"));
        let emphasis = list
            .iter()
            .find(|option| option.key == "zaxnes_video_emphasis_3d")
            .expect("the emphasis in 3D is declared");
        assert_eq!(emphasis.category, "three_d");
        let options = read(
            &|key| match key {
                "zaxnes_video_tv_saturation" => Some("150".to_owned()),
                "zaxnes_video_tv_hue" => Some("7".to_owned()),
                "zaxnes_video_composite_signal" => Some("enabled".to_owned()),
                _ => None,
            },
            &[],
        );
        let look = options.picture.tv_look();
        assert_eq!(look.knobs.saturation, 150);
        assert_eq!(look.knobs.hue, 0);
        assert!(options.picture.composite().is_some());
    }

    /// The preset fills the options left on Preset; an option given a value keeps it.
    #[test]
    fn a_preset_fills_what_is_left_to_it() {
        let read_with = |values: &[(&str, &str)]| {
            read(
                &|key| {
                    values
                        .iter()
                        .find(|(name, _)| *name == key)
                        .map(|(_, value)| (*value).to_owned())
                },
                &[],
            )
        };
        // Balanced, without a choice: the standard core, the Enhanced style with shadows.
        let balanced = read_with(&[]);
        assert_eq!(balanced.machine.variant, Variant::Standard);
        assert!(balanced.render.enhanced && balanced.render.shadows);
        // Fastest: the fast core, optimised Lua, Classic style without shadows.
        let fastest = read_with(&[("zaxnes_preset", "Fastest")]);
        assert_eq!(fastest.machine.variant, Variant::Fast);
        assert!(fastest.optimized_lua);
        assert!(!fastest.render.enhanced && !fastest.render.shadows);
        // Most faithful: Fidelity+, the OAMADDR write corruption on.
        let faithful = read_with(&[("zaxnes_preset", "Most faithful")]);
        assert_eq!(faithful.machine.variant, Variant::FidelityPlus);
        assert!(faithful
            .machine
            .values
            .contains(&(SettingId::EmuOamaddrWriteCorruption, 1)));
        // An option given a value wins over the preset.
        let mixed = read_with(&[
            ("zaxnes_preset", "Fastest"),
            ("zaxnes_core_variant", "Standard"),
            ("zaxnes_3d_shadows", "enabled"),
            ("zaxnes_emu_oamaddr_write_corruption", "enabled"),
        ]);
        assert_eq!(mixed.machine.variant, Variant::Standard);
        assert!(mixed.render.shadows);
        assert!(mixed
            .machine
            .values
            .contains(&(SettingId::EmuOamaddrWriteCorruption, 1)));
        let list = declarations(&[]);
        assert_eq!(list[0].key, "zaxnes_preset");
        assert_eq!(list[0].category, "");
        assert_eq!(list[0].default, "Balanced");
        assert!(list
            .iter()
            .any(|option| option.key == "zaxnes_3d_shadows" && option.default == "preset"));
    }
}
