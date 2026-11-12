//! The core options `RetroArch` shows (the libretro "variables"): their keys, their choices, and
//! how their values make [`Options`]. The first choice of each is its default.

use zax_nes::builtin_patches::HackOption;
use zax_nes::ppu::SpriteLimit;
use zax_nes::NesRegion;

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

/// The options to declare, as key and `"Description; choice|choice"`, the game's hack options
/// (`hacks`, each with its default) after the others.
#[must_use]
pub fn declarations(hacks: &[HackOption]) -> Vec<(String, String)> {
    let resolutions: Vec<String> = RESOLUTIONS
        .iter()
        .map(|(width, height)| format!("{width}x{height}"))
        .collect();
    let mut list = vec![
        (
            RENDERER.to_owned(),
            "Renderer (restart); 3D for games with a profile|2D only".to_owned(),
        ),
        (
            RESOLUTION.to_owned(),
            format!("3D resolution (restart); {}", resolutions.join("|")),
        ),
        (
            ANTI_ALIAS.to_owned(),
            format!(
                "3D anti-aliasing; {}",
                ANTI_ALIASES.map(|(name, _)| name).join("|")
            ),
        ),
        (SHADOWS.to_owned(), "3D shadows; enabled|disabled".to_owned()),
        (STYLE.to_owned(), "3D visual style; Classic|Enhanced".to_owned()),
        (
            REFLECTIONS.to_owned(),
            "3D reflections (Enhanced style); disabled|enabled".to_owned(),
        ),
        (
            SKY.to_owned(),
            format!("3D sky; {}", SKIES.map(|(name, _)| name).join("|")),
        ),
        (
            FAST_LUA.to_owned(),
            "3D optimised Lua (restart, unsafe code); disabled|enabled".to_owned(),
        ),
        (
            WIDESCREEN.to_owned(),
            "Widescreen (games with a widescreen hack); enabled|disabled".to_owned(),
        ),
        (
            RATIO.to_owned(),
            format!(
                "Widescreen ratio; {}",
                RATIOS.map(|(name, _)| name).join("|")
            ),
        ),
        (
            ADVANCED.to_owned(),
            "Widescreen advanced mode; enabled|disabled".to_owned(),
        ),
        (
            EXTRA.to_owned(),
            format!(
                "Widescreen extra margin (3D objects whole at the edges); {}",
                EXTRA_PERCENT.map(|percent| format!("{percent}%")).join("|")
            ),
        ),
        (
            STRICT.to_owned(),
            "Widescreen strict edges (3D); disabled|enabled".to_owned(),
        ),
        (
            FORCE.to_owned(),
            "Widescreen on other versions of a game (restart); disabled|enabled".to_owned(),
        ),
        (REGION.to_owned(), "Region; Auto|NTSC|PAL|Dendy".to_owned()),
        (
            SPRITE_LIMIT.to_owned(),
            "Sprite limit; 8 per line|Removed|Smart".to_owned(),
        ),
        (
            PALETTE.to_owned(),
            format!("Palette; {}", PALETTES.join("|")),
        ),
    ];
    for (key, name) in OVERSCAN.iter().zip(OVERSCAN_NAMES) {
        list.push((
            (*key).to_owned(),
            format!(
                "Overscan {name} (left and right without widescreen); {}",
                OVERSCAN_PIXELS.map(|pixels| format!("{pixels} px")).join("|")
            ),
        ));
    }
    for option in hacks {
        let choices = if option.default {
            "enabled|disabled"
        } else {
            "disabled|enabled"
        };
        list.push((
            format!("{HACK}{}", option.key),
            format!("Hack: {}; {choices}", option.label),
        ));
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
        assert!(list
            .iter()
            .any(|(key, text)| key == "zaxnes_hack_invincible"
                && text == "Hack: Invincible; disabled|enabled"));
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
