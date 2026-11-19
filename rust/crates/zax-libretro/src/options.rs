//! The core options `RetroArch` shows (the libretro "variables"): their keys, their choices, and
//! how their values make [`Options`]. The first choice of each is its default.

use zax_nes::NesRegion;
use zax_nes::builtin_patches::HackOption;
use zax_nes::ppu::SpriteLimit;

use crate::core::{Options, RegionChoice};
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

const RESOLUTIONS: [(u32, u32); 5] = [
    (1280, 720),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
    (960, 540),
];
/// The prefix of the options of the game's hack: the option's own key follows.
const HACK: &str = "zaxnes_hack_";

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
        }
    }

    /// With these values, shown as they are.
    fn values<S: AsRef<str>>(mut self, values: impl IntoIterator<Item = S>) -> Self {
        self.values = values
            .into_iter()
            .map(|value| (value.as_ref().to_owned(), None))
            .collect();
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
        self.values.first().map_or("", |(value, _)| value)
    }

    /// The declaration of the libretro API before categories (`SET_VARIABLES`):
    /// `"Name; value|value"`, the default first.
    #[must_use]
    pub fn legacy(&self) -> String {
        let values: Vec<&str> = self
            .values
            .iter()
            .map(|(value, _)| value.as_str())
            .collect();
        format!("{}; {}", self.label, values.join("|"))
    }
}

/// The options to declare, the game's hack options (`hacks`, each with its default) after the
/// others.
#[must_use]
#[allow(clippy::too_many_lines)] // One entry per option.
pub fn declarations(hacks: &[HackOption]) -> Vec<Declaration> {
    let resolutions = RESOLUTIONS.map(|(width, height)| format!("{width}x{height}"));
    let mut list = vec![
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
        Declaration::new(
            RESOLUTION,
            "3D resolution (restart)",
            "Resolution (restart)",
            "three_d",
            "The size the 3D is drawn at.",
        )
        .values(resolutions),
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
        .switch(true),
        Declaration::new(
            STYLE,
            "3D visual style",
            "Visual style",
            "three_d",
            "Classic is 3dSen's look; Enhanced adds the application's lighting and occlusion.",
        )
        .values(["Classic", "Enhanced"]),
        Declaration::new(
            REFLECTIONS,
            "3D reflections (Enhanced style)",
            "Reflections (Enhanced style)",
            "three_d",
            "Reflections on the floor, with the Enhanced style only.",
        )
        .switch(false),
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
        .switch(false),
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
            "In 2D, a picture between two frames along the camera's motion. Needs a display at \
             120 Hz with RetroArch's refresh rate set to it (at 60 Hz the game runs at half \
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
    for option in hacks {
        list.push(
            Declaration::new(
                &format!("{HACK}{}", option.key),
                &format!("Hack: {}", option.label),
                option.label,
                "hacks",
                "An option of this game's hack.",
            )
            .switch(option.default),
        );
    }
    list
}

/// The options the values give: `value` answers a key with its value, none when the frontend
/// has none; `hacks` are the game's hack options.
#[must_use]
pub fn read(value: &dyn Fn(&str) -> Option<String>, hacks: &[HackOption]) -> Options {
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
        render: RenderOptions {
            anti_alias: choice(&ANTI_ALIASES, value(ANTI_ALIAS), defaults.render.anti_alias),
            shadows: enabled(SHADOWS, defaults.render.shadows),
            enhanced: value(STYLE).map_or(defaults.render.enhanced, |text| text == "Enhanced"),
            reflections: enabled(REFLECTIONS, defaults.render.reflections),
            skybox: choice(&SKIES, value(SKY), defaults.render.skybox),
        },
        optimized_lua: enabled(FAST_LUA, defaults.optimized_lua),
        extra_percent: value(EXTRA)
            .and_then(|text| text.trim_end_matches('%').parse().ok())
            .filter(|percent| EXTRA_PERCENT.contains(percent))
            .unwrap_or(defaults.extra_percent),
        strict_edges: enabled(STRICT, defaults.strict_edges),
        smooth_120: enabled(SMOOTH, defaults.smooth_120),
        smooth_ahead: enabled(SMOOTH_AHEAD, defaults.smooth_ahead),
        force_patch: enabled(FORCE, defaults.force_patch),
        overscan: OVERSCAN.map(|key| {
            value(key)
                .and_then(|text| text.trim_end_matches(" px").parse().ok())
                .filter(|pixels| OVERSCAN_PIXELS.contains(pixels))
                .unwrap_or(0)
        }),
        hacks: hacks
            .iter()
            .map(|option| {
                let key = format!("{HACK}{}", option.key);
                (option.key.to_owned(), enabled(&key, option.default))
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
        assert!(
            list.iter()
                .any(|option| option.key == "zaxnes_hack_invincible"
                    && option.legacy() == "Hack: Invincible; disabled|enabled"
                    && option.category == "hacks")
        );
        let none = read(&|_| None, &[OPTION]);
        assert_eq!(
            none,
            Options {
                hacks: vec![("invincible".to_owned(), false)],
                ..Options::default()
            }
        );
        let set = read(
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
                ..RenderOptions::default()
            }
        );
    }
}
