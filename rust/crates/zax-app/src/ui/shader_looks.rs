//! The shader families (lot REWRITE-UI, owner request): instead of a list of near-identical
//! presets, a family ("CRT Guest", "Mega Bezel"...) and a few sub-options a player understands —
//! choices that pick the preset file of the family (the TV frame, the signal, the model) and
//! steps that set a few of its parameters (the curvature, the scanlines, the mask, the
//! brightness). The same idea has the same name in every family.
//!
//! The parameter names, ranges and defaults come from libretro/slang-shaders at the commit of
//! the pack (`shaders/pack-files.txt`); the values of the steps are a first proposal, to be
//! tuned by eye. A step of no parameter is the preset's own value. The desktop keeps the full
//! parameter editor under them; the console offers the families and their sub-options only.

use zax_settings::model::SettingsModel;
use zax_settings::shader_slots::{self, ShaderSlot, AREA_AUTO, AREA_PICTURE, PRESET};
use zax_settings::values::SettingValue;

use super::shader_panel::{self, Cost, Featured, PreFilter};

/// One parameter and its value.
type Param = (&'static str, f32);

/// A family of looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Its key: `shaders.look.<key>` is its name, `.help` its tooltip.
    pub key: &'static str,
    /// Where it is put forward.
    pub featured: Featured,
    /// The choices that pick the preset file of the family.
    pub picks: &'static [Pick],
    /// The preset files of the family, each with the choices it stands for.
    pub variants: &'static [Variant],
    /// The steps that set its parameters.
    pub knobs: &'static [Knob],
}

/// A choice between preset files of a family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    /// The key of its label (`shaders.option.<label>`).
    pub label: &'static str,
    /// The keys of its choices (`shaders.choice.<choice>`).
    pub choices: &'static [&'static str],
}

/// A preset file of a family.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Variant {
    /// The choice of each pick it stands for, in the order of [`Look::picks`].
    pub choice: &'static [usize],
    /// Its path in the pack.
    pub path: &'static str,
    /// The path used instead behind a pre-filter (a preset made for a smoothed picture).
    pub smoothed: Option<&'static str>,
    /// The parameters it is set with in the 4:3 modes: those that make it this variant.
    pub parameters: &'static [Param],
    /// The parameters it is set with in the widescreen modes.
    pub wide_parameters: &'static [Param],
    /// Where it draws (`shader_slots::AREA_*`).
    pub area: i32,
    /// What it asks of the graphics card.
    pub cost: Cost,
}

/// A sub-option of steps, each a set of parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knob {
    /// The key of its label (`shaders.option.<label>`).
    pub label: &'static str,
    /// The keys of its steps (`shaders.step.<step>`).
    pub steps: &'static [&'static str],
    /// The parameters of each step, for the variants of `paths`; a variant in none of them has
    /// not got the sub-option.
    pub maps: &'static [KnobMap],
}

/// The parameters of a knob's steps for some variants of its family.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KnobMap {
    /// The variants, by path (the plain path, not the smoothed one).
    pub paths: &'static [&'static str],
    /// The parameters of each step; an empty one is the preset's own value.
    pub steps: &'static [&'static [Param]],
}

impl Knob {
    /// The map of the variant at `path`, if the variant has the knob.
    #[must_use]
    pub fn map_for(&self, path: &str) -> Option<&'static KnobMap> {
        self.maps.iter().find(|map| map.paths.contains(&path))
    }
}

impl Look {
    /// Its name, in the language in force.
    #[must_use]
    pub fn name(&self) -> String {
        super::strings::get(&format!("shaders.look.{}", self.key))
    }

    /// What its first variant asks of the card: the family's cost as the menus show it.
    #[must_use]
    pub fn cost(&self) -> Cost {
        self.variants
            .first()
            .map_or(Cost::Light, |variant| variant.cost)
    }

    /// Whether the pack of this build holds it (the light pack of Android holds a few).
    #[must_use]
    pub fn carried(&self) -> bool {
        self.variants
            .iter()
            .any(|variant| crate::shader_pack::contains(variant.path))
    }
}

// --- The tables ---------------------------------------------------------------------------

const MB: &str = "bezel/Mega_Bezel/Presets/";
const MB_POTATO: &str = "bezel/Mega_Bezel/Presets/MBZ__5__POTATO.slangp";
const MB_STD: &str = "bezel/Mega_Bezel/Presets/MBZ__3__STD.slangp";
const MB_ADV: &str = "bezel/Mega_Bezel/Presets/MBZ__1__ADV.slangp";
const MB_POTATO_NTSC: &str =
    "bezel/Mega_Bezel/Presets/Base_CRT_Presets/MBZ__5__POTATO__GDV-MINI-NTSC.slangp";
const MB_STD_NTSC: &str = "bezel/Mega_Bezel/Presets/Base_CRT_Presets/MBZ__3__STD__GDV-NTSC.slangp";
const MB_ADV_NTSC: &str = "bezel/Mega_Bezel/Presets/Base_CRT_Presets/MBZ__1__ADV__GDV-NTSC.slangp";
const MB_S_POTATO: &str =
    "bezel/Mega_Bezel/Presets/Variations/Screen-Only/Default/_5__POTATO.slangp";
const MB_S_POTATO_NTSC: &str =
    "bezel/Mega_Bezel/Presets/Variations/Screen-Only/Default/_5__POTATO__NTSC.slangp";
const MB_S_STD: &str =
    "bezel/Mega_Bezel/Presets/Variations/Screen-Only/Default/_4__STD-SCREEN-ONLY.slangp";
const MB_S_STD_NTSC: &str =
    "bezel/Mega_Bezel/Presets/Variations/Screen-Only/Default/_4__STD-SCREEN-ONLY__NTSC.slangp";
const MB_S_ADV: &str =
    "bezel/Mega_Bezel/Presets/Base_CRT_Presets/MBZ__0__SMOOTH-ADV-SCREEN-ONLY__GDV.slangp";
const MB_S_ADV_NTSC: &str =
    "bezel/Mega_Bezel/Presets/Base_CRT_Presets/MBZ__0__SMOOTH-ADV-SCREEN-ONLY__GDV-NTSC.slangp";

/// The Mega Bezel TV at the aspect of the picture it reads, in the widescreen modes.
const MB_TV_WIDE: &[Param] = &[("HSM_ASPECT_RATIO_MODE", 5.0)];
/// The Mega Bezel screen over the whole picture.
const MB_SCREEN: &[Param] = &[("HSM_ASPECT_RATIO_MODE", 6.0)];

const fn plain(choice: &'static [usize], path: &'static str, cost: Cost) -> Variant {
    Variant {
        choice,
        path,
        smoothed: None,
        parameters: &[],
        wide_parameters: &[],
        area: AREA_AUTO,
        cost,
    }
}

const fn tv(choice: &'static [usize], path: &'static str, cost: Cost) -> Variant {
    Variant {
        wide_parameters: MB_TV_WIDE,
        ..plain(choice, path, cost)
    }
}

const fn screen(choice: &'static [usize], path: &'static str, cost: Cost) -> Variant {
    Variant {
        parameters: MB_SCREEN,
        wide_parameters: MB_SCREEN,
        area: AREA_PICTURE,
        ..plain(choice, path, cost)
    }
}

const fn tagged(
    choice: &'static [usize],
    path: &'static str,
    parameters: &'static [Param],
    cost: Cost,
) -> Variant {
    Variant {
        parameters,
        wide_parameters: parameters,
        ..plain(choice, path, cost)
    }
}

const NONE_LIGHT_NORMAL_STRONG: &[&str] = &["none", "light", "normal", "strong"];
const NONE_NORMAL_STRONG: &[&str] = &["none", "normal", "strong"];
const DARK_NORMAL_BRIGHT: &[&str] = &["darker", "normal", "brighter"];

const MB_ALL: &[&str] = &[
    MB_POTATO,
    MB_STD,
    MB_ADV,
    MB_POTATO_NTSC,
    MB_STD_NTSC,
    MB_ADV_NTSC,
    MB_S_POTATO,
    MB_S_POTATO_NTSC,
    MB_S_STD,
    MB_S_STD_NTSC,
    MB_S_ADV,
    MB_S_ADV_NTSC,
];

const GUEST: &str = "crt/crt-guest-advanced.slangp";
const GUEST_NTSC: &str = "crt/crt-guest-advanced-ntsc.slangp";
const GUEST_HD: &str = "crt/crt-guest-advanced-hd.slangp";
const GUEST_ALL: &[&str] = &[GUEST, GUEST_NTSC];

const ZFAST: &str = "crt/zfast-crt.slangp";
const ZFAST_COMPOSITE: &str = "crt/zfast-crt-composite.slangp";
const CRT_PI: &str = "crt/crt-pi.slangp";
const EASYMODE: &str = "crt/crt-easymode.slangp";
const NES_MINI: &str = "crt/crt-nes-mini.slangp";
const FAKELOTTES: &str = "crt/fakelottes.slangp";
const FAKELOTTES_NTSC: &str = "presets/crt-plus-signal/fakelottes-ntsc-composite.slangp";

const LOTTES: &str = "crt/crt-lottes.slangp";
const HYLLIAN: &str = "crt/crt-hyllian.slangp";
const GEOM: &str = "crt/crt-geom.slangp";
const MATTIAS: &str = "crt/crt-mattias.slangp";

const ROYALE: &str = "crt/crt-royale.slangp";
const ROYALE_RF: &str = "presets/crt-royale-fast/crt-royale-fast-ntsc-rf-slotmask-nes.slangp";

const NTSC_COMPOSITE_LINES: &str = "ntsc/ntsc-256px-composite-scanline.slangp";
const NTSC_SVIDEO_LINES: &str = "ntsc/ntsc-256px-svideo-scanline.slangp";
const NTSC_COMPOSITE: &str = "ntsc/ntsc-256px-composite.slangp";
const NTSC_SVIDEO: &str = "ntsc/ntsc-256px-svideo.slangp";
const BLARGG: &str = "ntsc/blargg.slangp";

const LCD: &str = "handheld/lcd-grid-v2.slangp";

/// Every family, in the order of the menus.
pub const LOOKS: [Look; 8] = [
    // CRT Guest Advanced: the reference tube of the libretro forums.
    Look {
        key: "guest",
        featured: Featured::Always,
        picks: &[Pick {
            label: "signal",
            choices: &["rgb", "svideo", "composite", "rf"],
        }],
        variants: &[
            Variant {
                smoothed: Some(GUEST_HD),
                ..plain(&[0], GUEST, Cost::Medium)
            },
            tagged(
                &[1],
                GUEST_NTSC,
                &[("cust_artifacting", 0.0), ("cust_fringing", 0.0)],
                Cost::Medium,
            ),
            tagged(
                &[2],
                GUEST_NTSC,
                &[("cust_artifacting", 1.0), ("cust_fringing", 1.0)],
                Cost::Medium,
            ),
            tagged(
                &[3],
                GUEST_NTSC,
                &[("cust_artifacting", 2.0), ("cust_fringing", 2.0)],
                Cost::Medium,
            ),
        ],
        knobs: &[
            Knob {
                label: "curvature",
                steps: NONE_LIGHT_NORMAL_STRONG,
                maps: &[KnobMap {
                    paths: GUEST_ALL,
                    steps: &[
                        &[],
                        &[("warpX", 0.02), ("warpY", 0.03), ("csize", 0.02)],
                        &[("warpX", 0.03), ("warpY", 0.04), ("csize", 0.02)],
                        &[("warpX", 0.06), ("warpY", 0.08), ("csize", 0.02)],
                    ],
                }],
            },
            Knob {
                label: "scanlines",
                steps: NONE_LIGHT_NORMAL_STRONG,
                maps: &[KnobMap {
                    paths: GUEST_ALL,
                    steps: &[
                        &[("no_scanlines", 1.0)],
                        &[("scanline1", 3.0), ("scanline2", 5.0)],
                        &[],
                        &[("scanline1", 10.0), ("scanline2", 14.0)],
                    ],
                }],
            },
            Knob {
                label: "mask",
                steps: &["none", "fine", "trinitron", "slot"],
                maps: &[KnobMap {
                    paths: GUEST_ALL,
                    steps: &[
                        &[("shadowMask", -1.0)],
                        &[],
                        &[("shadowMask", 6.0)],
                        &[("slotmask", 0.5), ("slotmask1", 0.5)],
                    ],
                }],
            },
            Knob {
                label: "brightness",
                steps: DARK_NORMAL_BRIGHT,
                maps: &[KnobMap {
                    paths: GUEST_ALL,
                    steps: &[&[("post_br", 0.85)], &[], &[("post_br", 1.2)]],
                }],
            },
        ],
    },
    // Mega Bezel: a TV round the picture, or its screen alone.
    Look {
        key: "mega_bezel",
        featured: Featured::Always,
        picks: &[
            Pick {
                label: "frame",
                choices: &["yes", "no"],
            },
            Pick {
                label: "quality",
                choices: &["light", "standard", "advanced"],
            },
            Pick {
                label: "signal",
                choices: &["rgb", "composite"],
            },
        ],
        variants: &[
            tv(&[0, 1, 0], MB_STD, Cost::Heavy),
            tv(&[0, 0, 0], MB_POTATO, Cost::Medium),
            tv(&[0, 2, 0], MB_ADV, Cost::Heavy),
            tv(&[0, 0, 1], MB_POTATO_NTSC, Cost::Medium),
            tv(&[0, 1, 1], MB_STD_NTSC, Cost::Heavy),
            tv(&[0, 2, 1], MB_ADV_NTSC, Cost::Heavy),
            screen(&[1, 0, 0], MB_S_POTATO, Cost::Light),
            screen(&[1, 1, 0], MB_S_STD, Cost::Medium),
            screen(&[1, 2, 0], MB_S_ADV, Cost::Heavy),
            screen(&[1, 0, 1], MB_S_POTATO_NTSC, Cost::Light),
            screen(&[1, 1, 1], MB_S_STD_NTSC, Cost::Medium),
            screen(&[1, 2, 1], MB_S_ADV_NTSC, Cost::Heavy),
        ],
        knobs: &[
            Knob {
                label: "curvature",
                steps: NONE_LIGHT_NORMAL_STRONG,
                maps: &[KnobMap {
                    paths: MB_ALL,
                    steps: &[
                        &[("HSM_CURVATURE_MODE", 0.0)],
                        &[
                            ("HSM_CURVATURE_2D_SCALE_LONG_AXIS", 50.0),
                            ("HSM_CURVATURE_2D_SCALE_SHORT_AXIS", 50.0),
                        ],
                        &[],
                        &[
                            ("HSM_CURVATURE_2D_SCALE_LONG_AXIS", 200.0),
                            ("HSM_CURVATURE_2D_SCALE_SHORT_AXIS", 200.0),
                        ],
                    ],
                }],
            },
            // The reflections of the frame on the TV: Standard and Advanced with the frame.
            Knob {
                label: "reflections",
                steps: NONE_NORMAL_STRONG,
                maps: &[KnobMap {
                    paths: &[MB_STD, MB_ADV, MB_STD_NTSC, MB_ADV_NTSC],
                    steps: &[
                        &[("HSM_REFLECT_GLOBAL_AMOUNT", 0.0)],
                        &[],
                        &[("HSM_REFLECT_GLOBAL_AMOUNT", 80.0)],
                    ],
                }],
            },
        ],
    },
    // The light tubes, for a weak card or a handheld.
    Look {
        key: "crt_light",
        featured: Featured::Always,
        picks: &[
            Pick {
                label: "model",
                choices: &["zfast", "crt_pi", "easymode", "nes_mini", "fakelottes"],
            },
            Pick {
                label: "signal",
                choices: &["rgb", "composite"],
            },
        ],
        variants: &[
            plain(&[0, 0], ZFAST, Cost::Light),
            plain(&[0, 1], ZFAST_COMPOSITE, Cost::Light),
            plain(&[1, 0], CRT_PI, Cost::Light),
            plain(&[2, 0], EASYMODE, Cost::Light),
            plain(&[3, 0], NES_MINI, Cost::Light),
            plain(&[4, 0], FAKELOTTES, Cost::Light),
            plain(&[4, 1], FAKELOTTES_NTSC, Cost::Light),
        ],
        knobs: &[
            Knob {
                label: "curvature",
                steps: NONE_LIGHT_NORMAL_STRONG,
                maps: &[
                    KnobMap {
                        paths: &[CRT_PI],
                        steps: &[
                            &[("CURVATURE_X", 0.0), ("CURVATURE_Y", 0.0)],
                            &[("CURVATURE_X", 0.05), ("CURVATURE_Y", 0.08)],
                            &[],
                            &[("CURVATURE_X", 0.2), ("CURVATURE_Y", 0.3)],
                        ],
                    },
                    KnobMap {
                        paths: &[FAKELOTTES, FAKELOTTES_NTSC],
                        steps: &[
                            &[("warpX", 0.0), ("warpY", 0.0)],
                            &[("warpX", 0.02), ("warpY", 0.02)],
                            &[],
                            &[("warpX", 0.06), ("warpY", 0.08)],
                        ],
                    },
                ],
            },
            Knob {
                label: "scanlines",
                steps: NONE_LIGHT_NORMAL_STRONG,
                maps: &[
                    KnobMap {
                        paths: &[ZFAST, ZFAST_COMPOSITE],
                        steps: &[
                            &[("LOWLUMSCAN", 0.0), ("HILUMSCAN", 0.0)],
                            &[("LOWLUMSCAN", 3.0), ("HILUMSCAN", 4.0)],
                            &[],
                            &[("LOWLUMSCAN", 9.0), ("HILUMSCAN", 12.0)],
                        ],
                    },
                    KnobMap {
                        paths: &[CRT_PI],
                        steps: &[
                            &[("SCANLINE_GAP_BRIGHTNESS", 1.0)],
                            &[("SCANLINE_GAP_BRIGHTNESS", 0.4)],
                            &[],
                            &[("SCANLINE_GAP_BRIGHTNESS", 0.0)],
                        ],
                    },
                    KnobMap {
                        paths: &[EASYMODE],
                        steps: &[
                            &[("SCANLINE_STRENGTH", 0.0)],
                            &[("SCANLINE_STRENGTH", 0.4)],
                            &[("SCANLINE_STRENGTH", 0.7)],
                            &[],
                        ],
                    },
                    KnobMap {
                        paths: &[NES_MINI],
                        steps: &[
                            &[("INTENSITY", 0.0)],
                            &[],
                            &[("INTENSITY", 0.3)],
                            &[("INTENSITY", 0.5)],
                        ],
                    },
                    KnobMap {
                        paths: &[FAKELOTTES, FAKELOTTES_NTSC],
                        steps: &[
                            &[("SCANLINE_SINE_COMP_B", 0.0)],
                            &[("SCANLINE_SINE_COMP_B", 0.2)],
                            &[],
                            &[("SCANLINE_SINE_COMP_B", 0.7)],
                        ],
                    },
                ],
            },
            Knob {
                label: "mask",
                steps: NONE_NORMAL_STRONG,
                maps: &[
                    KnobMap {
                        paths: &[ZFAST, ZFAST_COMPOSITE],
                        steps: &[&[("MASK_DARK", 0.0)], &[], &[("MASK_DARK", 0.75)]],
                    },
                    KnobMap {
                        paths: &[CRT_PI],
                        steps: &[
                            &[("MASK_BRIGHTNESS", 1.0)],
                            &[],
                            &[("MASK_BRIGHTNESS", 0.4)],
                        ],
                    },
                    KnobMap {
                        paths: &[EASYMODE],
                        steps: &[&[("MASK_STRENGTH", 0.0)], &[], &[("MASK_STRENGTH", 0.6)]],
                    },
                ],
            },
            Knob {
                label: "brightness",
                steps: DARK_NORMAL_BRIGHT,
                maps: &[
                    KnobMap {
                        paths: &[ZFAST, ZFAST_COMPOSITE],
                        steps: &[&[("BRIGHTBOOST", 0.85)], &[], &[("BRIGHTBOOST", 1.5)]],
                    },
                    KnobMap {
                        paths: &[CRT_PI],
                        steps: &[&[("BLOOM_FACTOR", 1.0)], &[], &[("BLOOM_FACTOR", 2.0)]],
                    },
                    KnobMap {
                        paths: &[EASYMODE],
                        steps: &[&[("BRIGHT_BOOST", 1.0)], &[], &[("BRIGHT_BOOST", 1.5)]],
                    },
                    KnobMap {
                        paths: &[NES_MINI],
                        steps: &[&[("BRIGHTBOOST", 0.0)], &[], &[("BRIGHTBOOST", 0.3)]],
                    },
                ],
            },
        ],
    },
    // The classic tubes.
    Look {
        key: "crt_classic",
        featured: Featured::No,
        picks: &[Pick {
            label: "model",
            choices: &["lottes", "hyllian", "geom", "mattias"],
        }],
        variants: &[
            plain(&[0], LOTTES, Cost::Medium),
            plain(&[1], HYLLIAN, Cost::Medium),
            plain(&[2], GEOM, Cost::Medium),
            plain(&[3], MATTIAS, Cost::Medium),
        ],
        knobs: &[
            Knob {
                label: "curvature",
                steps: NONE_LIGHT_NORMAL_STRONG,
                maps: &[
                    KnobMap {
                        paths: &[LOTTES],
                        steps: &[
                            &[("warpX", 0.0), ("warpY", 0.0)],
                            &[("warpX", 0.02), ("warpY", 0.02)],
                            &[],
                            &[("warpX", 0.06), ("warpY", 0.08)],
                        ],
                    },
                    KnobMap {
                        paths: &[HYLLIAN],
                        steps: &[
                            &[],
                            &[("h_curvature", 1.0), ("h_radius", 8.0)],
                            &[("h_curvature", 1.0), ("h_radius", 5.0)],
                            &[("h_curvature", 1.0), ("h_radius", 2.5)],
                        ],
                    },
                    KnobMap {
                        paths: &[GEOM],
                        steps: &[&[("CURVATURE", 0.0)], &[("R", 4.0)], &[], &[("R", 1.5)]],
                    },
                    KnobMap {
                        paths: &[MATTIAS],
                        steps: &[
                            &[("CURVATURE", 0.0)],
                            &[("CURVATURE", 0.25)],
                            &[],
                            &[("CURVATURE", 1.0)],
                        ],
                    },
                ],
            },
            Knob {
                label: "scanlines",
                steps: NONE_LIGHT_NORMAL_STRONG,
                maps: &[
                    KnobMap {
                        paths: &[LOTTES],
                        steps: &[
                            &[("hardScan", 0.0)],
                            &[("hardScan", -4.0)],
                            &[],
                            &[("hardScan", -14.0)],
                        ],
                    },
                    KnobMap {
                        paths: &[HYLLIAN],
                        steps: &[
                            &[("SCANLINES_STRENGTH", 0.0)],
                            &[("SCANLINES_STRENGTH", 0.4)],
                            &[],
                            &[("SCANLINES_STRENGTH", 1.0)],
                        ],
                    },
                ],
            },
            Knob {
                label: "mask",
                steps: NONE_NORMAL_STRONG,
                maps: &[
                    KnobMap {
                        paths: &[HYLLIAN],
                        steps: &[&[("MASK_STRENGTH", 0.0)], &[("MASK_STRENGTH", 0.5)], &[]],
                    },
                    KnobMap {
                        paths: &[GEOM],
                        steps: &[&[("DOTMASK", 0.0)], &[], &[("DOTMASK", 0.6)]],
                    },
                ],
            },
            Knob {
                label: "brightness",
                steps: DARK_NORMAL_BRIGHT,
                maps: &[
                    KnobMap {
                        paths: &[LOTTES],
                        steps: &[&[("brightBoost", 0.8)], &[], &[("brightBoost", 1.3)]],
                    },
                    KnobMap {
                        paths: &[HYLLIAN],
                        steps: &[&[("BRIGHTBOOST", 0.85)], &[], &[("BRIGHTBOOST", 1.5)]],
                    },
                ],
            },
        ],
    },
    // CRT Royale, the heaviest tube.
    Look {
        key: "royale",
        featured: Featured::No,
        picks: &[Pick {
            label: "signal",
            choices: &["rgb", "rf"],
        }],
        variants: &[
            plain(&[0], ROYALE, Cost::Heavy),
            plain(&[1], ROYALE_RF, Cost::Heavy),
        ],
        knobs: &[
            Knob {
                label: "curvature",
                steps: NONE_LIGHT_NORMAL_STRONG,
                maps: &[
                    KnobMap {
                        paths: &[ROYALE],
                        steps: &[
                            &[],
                            &[("geom_mode_runtime", 1.0), ("geom_radius", 4.0)],
                            &[("geom_mode_runtime", 1.0), ("geom_radius", 2.0)],
                            &[("geom_mode_runtime", 1.0), ("geom_radius", 1.5)],
                        ],
                    },
                    KnobMap {
                        paths: &[ROYALE_RF],
                        steps: &[
                            &[],
                            &[("geom_curvature", 1.0), ("geom_R", 6.0)],
                            &[("geom_curvature", 1.0), ("geom_R", 4.0)],
                            &[("geom_curvature", 1.0), ("geom_R", 2.5)],
                        ],
                    },
                ],
            },
            Knob {
                label: "brightness",
                steps: DARK_NORMAL_BRIGHT,
                maps: &[
                    KnobMap {
                        paths: &[ROYALE],
                        steps: &[
                            &[("levels_contrast", 0.843_75)],
                            &[],
                            &[("levels_contrast", 1.203_125)],
                        ],
                    },
                    KnobMap {
                        paths: &[ROYALE_RF],
                        steps: &[
                            &[("levels_contrast", 0.5625)],
                            &[],
                            &[("levels_contrast", 0.843_75)],
                        ],
                    },
                ],
            },
        ],
    },
    // The console's composite signal alone, without a tube.
    Look {
        key: "composite",
        featured: Featured::No,
        picks: &[
            Pick {
                label: "model",
                choices: &["ntsc", "blargg"],
            },
            Pick {
                label: "signal",
                choices: &["composite", "svideo"],
            },
            Pick {
                label: "scanlines",
                choices: &["yes", "no"],
            },
        ],
        variants: &[
            plain(&[0, 0, 0], NTSC_COMPOSITE_LINES, Cost::Light),
            plain(&[0, 1, 0], NTSC_SVIDEO_LINES, Cost::Light),
            plain(&[0, 0, 1], NTSC_COMPOSITE, Cost::Light),
            plain(&[0, 1, 1], NTSC_SVIDEO, Cost::Light),
            plain(&[1, 0, 1], BLARGG, Cost::Light),
        ],
        knobs: &[
            Knob {
                label: "artifacts",
                steps: NONE_NORMAL_STRONG,
                maps: &[KnobMap {
                    paths: &[BLARGG],
                    steps: &[
                        &[],
                        &[("afacts", 0.5), ("fring", 0.5)],
                        &[("afacts", 1.0), ("fring", 1.0)],
                    ],
                }],
            },
            Knob {
                label: "sharpness",
                steps: &["soft", "normal", "sharp"],
                maps: &[KnobMap {
                    paths: &[BLARGG],
                    steps: &[&[("ntsc_sharp", -0.5)], &[], &[("ntsc_sharp", 0.6)]],
                }],
            },
        ],
    },
    // A handheld's LCD.
    Look {
        key: "lcd",
        featured: Featured::Narrow,
        picks: &[],
        variants: &[plain(&[], LCD, Cost::Light)],
        knobs: &[
            Knob {
                label: "brightness",
                steps: DARK_NORMAL_BRIGHT,
                maps: &[KnobMap {
                    paths: &[LCD],
                    steps: &[&[("gain", 1.0)], &[], &[("gain", 2.0)]],
                }],
            },
            Knob {
                label: "black_level",
                steps: NONE_NORMAL_STRONG,
                maps: &[KnobMap {
                    paths: &[LCD],
                    steps: &[&[("blacklevel", 0.0)], &[], &[("blacklevel", 0.15)]],
                }],
            },
            Knob {
                label: "subpixels",
                steps: &["rgb", "bgr"],
                maps: &[KnobMap {
                    paths: &[LCD],
                    steps: &[&[], &[("BGR", 1.0)]],
                }],
            },
        ],
    },
    // Pixel art smoothed, without a tube (in front of a tube, it is the pre-filter).
    Look {
        key: "smoothing",
        featured: Featured::No,
        picks: &[Pick {
            label: "method",
            choices: &["sharp", "xbrz", "scalefx"],
        }],
        variants: &[
            plain(
                &[0],
                "pixel-art-scaling/sharp-bilinear-simple.slangp",
                Cost::Light,
            ),
            plain(
                &[1],
                "edge-smoothing/xbrz/xbrz-freescale.slangp",
                Cost::Medium,
            ),
            plain(&[2], "edge-smoothing/scalefx/scalefx.slangp", Cost::Medium),
        ],
        knobs: &[],
    },
];

/// The families the pack of this build holds.
pub fn looks() -> impl Iterator<Item = (usize, &'static Look)> {
    LOOKS.iter().enumerate().filter(|(_, look)| look.carried())
}

// --- What a slot holds -------------------------------------------------------------------------

/// The family a slot holds and its sub-options as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookState {
    /// The family, an index of [`LOOKS`].
    pub look: usize,
    /// Its variant, an index of [`Look::variants`].
    pub variant: usize,
    /// The step of each knob: none for a knob the variant has not got or a value set by hand.
    pub steps: Vec<Option<usize>>,
}

impl LookState {
    /// The family.
    #[must_use]
    pub const fn look(&self) -> &'static Look {
        &LOOKS[self.look]
    }

    /// The variant.
    #[must_use]
    pub const fn variant(&self) -> &'static Variant {
        &LOOKS[self.look].variants[self.variant]
    }
}

/// The value `name` holds among `overrides`, if it is set.
fn set_value(overrides: &[(String, f32)], name: &str) -> Option<f32> {
    overrides
        .iter()
        .find(|(known, _)| known == name)
        .map(|(_, value)| *value)
}

fn same(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.0e-4
}

/// The step of `map` the parameters `overrides` hold: the one whose parameters are all set at
/// its values, or the preset's own step when none of the map's parameters is set.
fn step_of(map: &KnobMap, overrides: &[(String, f32)]) -> Option<usize> {
    let names = || {
        map.steps
            .iter()
            .flat_map(|step| step.iter())
            .map(|(name, _)| *name)
    };
    let set_names: Vec<&str> = names()
        .filter(|name| set_value(overrides, name).is_some())
        .collect();
    map.steps.iter().position(|step| {
        if step.is_empty() {
            return set_names.is_empty();
        }
        step.iter()
            .all(|(name, value)| set_value(overrides, name).is_some_and(|set| same(set, *value)))
            && set_names
                .iter()
                .all(|name| step.iter().any(|(listed, _)| listed == name))
    })
}

/// The family `slot` holds in `model`, read from its preset (behind its pre-filter) and its
/// parameters; none for a preset of no family (chosen in the menu of every preset, or an older
/// combination).
#[must_use]
pub fn state_of(model: &SettingsModel, slot: ShaderSlot) -> Option<LookState> {
    let ids = slot.ids();
    if model.get(ids.mode).as_int() != PRESET {
        return None;
    }
    let value = model.get(ids.preset);
    let (_, preset) = shader_panel::split_pre_filter(value.as_text());
    let preset = preset.replace('\\', "/");
    let overrides = shader_slots::parse_parameters(model.get(ids.params).as_text());
    let (_, wide) = shader_panel::mode_of(slot);
    for (look_index, look) in LOOKS.iter().enumerate() {
        let candidates: Vec<usize> = look
            .variants
            .iter()
            .enumerate()
            .filter(|(_, variant)| variant.path == preset || variant.smoothed == Some(&preset))
            .map(|(index, _)| index)
            .collect();
        let Some(&first) = candidates.first() else {
            continue;
        };
        // Among the variants of the same file, the one whose own parameters are set.
        let variant = candidates
            .iter()
            .copied()
            .find(|&index| {
                let variant = &look.variants[index];
                let own = if wide {
                    variant.wide_parameters
                } else {
                    variant.parameters
                };
                !own.is_empty()
                    && own.iter().all(|(name, value)| {
                        set_value(&overrides, name).is_some_and(|set| same(set, *value))
                    })
            })
            .unwrap_or(first);
        let path = look.variants[variant].path;
        let steps = look
            .knobs
            .iter()
            .map(|knob| knob.map_for(path).and_then(|map| step_of(map, &overrides)))
            .collect();
        return Some(LookState {
            look: look_index,
            variant,
            steps,
        });
    }
    None
}

/// Whether `knob` of the family is offered by the variant `state` holds.
#[must_use]
pub fn has_knob(state: &LookState, knob: usize) -> bool {
    state.look().knobs[knob]
        .map_for(state.variant().path)
        .is_some()
}

// --- Choosing -------------------------------------------------------------------------------

/// The preset's own step of each knob for the variant at `path`.
fn default_steps(look: &Look, path: &str) -> Vec<Option<usize>> {
    look.knobs
        .iter()
        .map(|knob| {
            knob.map_for(path)
                .and_then(|map| map.steps.iter().position(|step| step.is_empty()))
        })
        .collect()
}

/// Sets `slot` to the variant `variant` of the family `look`, its knobs at `steps` where it has
/// them (their own step elsewhere), behind the pre-filter the slot has.
fn apply(
    model: &mut SettingsModel,
    slot: ShaderSlot,
    look: usize,
    variant: usize,
    steps: &[Option<usize>],
) {
    let family = &LOOKS[look];
    let chosen = &family.variants[variant];
    let ids = slot.ids();
    let filter = if model.get(ids.mode).as_int() == PRESET {
        shader_panel::pre_filter_of(model, slot)
    } else {
        None
    };
    let path = match (filter, chosen.smoothed) {
        (Some(_), Some(smoothed)) => smoothed,
        _ => chosen.path,
    };
    let (_, wide) = shader_panel::mode_of(slot);
    let own = if wide {
        chosen.wide_parameters
    } else {
        chosen.parameters
    };
    let mut parameters: Vec<(String, f32)> = own
        .iter()
        .map(|(name, value)| ((*name).to_owned(), *value))
        .collect();
    // The Mega Bezel starts without its intro.
    if path.starts_with(MB) {
        parameters.insert(0, ("HSM_INTRO_WHEN_TO_SHOW".to_owned(), 0.0));
    }
    let defaults = default_steps(family, chosen.path);
    for (index, knob) in family.knobs.iter().enumerate() {
        let Some(map) = knob.map_for(chosen.path) else {
            continue;
        };
        let step = steps
            .get(index)
            .copied()
            .flatten()
            .or(defaults[index])
            .filter(|step| *step < map.steps.len());
        if let Some(step) = step {
            for (name, value) in map.steps[step] {
                parameters.retain(|(known, _)| known != name);
                parameters.push(((*name).to_owned(), *value));
            }
        }
    }
    model.set(ids.mode, SettingValue::Enum(PRESET));
    model.set(
        ids.preset,
        shader_panel::with_pre_filter(filter, path).as_str(),
    );
    model.set(
        ids.params,
        shader_slots::format_parameters(&parameters).as_str(),
    );
    model.set(ids.area, SettingValue::Enum(chosen.area));
}

/// Chooses the family `look` for `slot`, at its first variant and the presets' own steps; the
/// pre-filter the slot has stays.
pub fn choose_look(model: &mut SettingsModel, slot: ShaderSlot, look: usize) {
    if look < LOOKS.len() {
        apply(model, slot, look, 0, &[]);
    }
}

/// The variant of `look` with the choices `choice`, or the nearest: the first with the choice
/// of pick `kept` (the one just changed) and the most other choices in common.
fn variant_for(look: &Look, choice: &[usize], kept: usize) -> Option<usize> {
    look.variants
        .iter()
        .enumerate()
        .filter(|(_, variant)| variant.choice.get(kept) == choice.get(kept))
        .filter(|(_, variant)| crate::shader_pack::contains(variant.path))
        .max_by_key(|(index, variant)| {
            let common = variant
                .choice
                .iter()
                .zip(choice)
                .filter(|(a, b)| a == b)
                .count();
            // The first of equals.
            (common, usize::MAX - index)
        })
        .map(|(index, _)| index)
}

/// Whether choice `choice` of pick `pick` leads to a variant the pack holds.
#[must_use]
pub fn choice_offered(look: &Look, pick: usize, choice: usize) -> bool {
    look.variants.iter().any(|variant| {
        variant.choice.get(pick) == Some(&choice) && crate::shader_pack::contains(variant.path)
    })
}

/// Sets pick `pick` of the family `slot` holds to `choice`: the variant of those choices (or the
/// nearest), the knobs kept at their steps where the new variant has them.
pub fn choose_pick(model: &mut SettingsModel, slot: ShaderSlot, pick: usize, choice: usize) {
    let Some(state) = state_of(model, slot) else {
        return;
    };
    let look = state.look();
    let mut wanted = state.variant().choice.to_vec();
    let Some(entry) = wanted.get_mut(pick) else {
        return;
    };
    *entry = choice;
    if let Some(variant) = variant_for(look, &wanted, pick) {
        apply(model, slot, state.look, variant, &state.steps);
    }
}

/// Sets knob `knob` of the family `slot` holds to step `step`: its parameters, and nothing else.
pub fn choose_step(model: &mut SettingsModel, slot: ShaderSlot, knob: usize, step: usize) {
    let Some(state) = state_of(model, slot) else {
        return;
    };
    let Some(map) = state
        .look()
        .knobs
        .get(knob)
        .and_then(|knob| knob.map_for(state.variant().path))
    else {
        return;
    };
    let Some(values) = map.steps.get(step) else {
        return;
    };
    let ids = slot.ids();
    let mut text = model.get(ids.params).as_text().to_owned();
    for (name, _) in map.steps.iter().flat_map(|step| step.iter()) {
        text = shader_slots::with_parameter(&text, name, None);
    }
    for (name, value) in *values {
        text = shader_slots::with_parameter(&text, name, Some(*value));
    }
    model.set(ids.params, text.as_str());
}

/// Puts `filter` in front of the preset of `slot` (or takes the one there away); a family whose
/// variant has a version made for a smoothed picture switches to it, its parameters kept.
pub fn set_pre_filter(model: &mut SettingsModel, slot: ShaderSlot, filter: Option<&PreFilter>) {
    let state = state_of(model, slot);
    shader_panel::set_pre_filter(model, slot, filter);
    let Some(state) = state else {
        return;
    };
    let variant = state.variant();
    let Some(smoothed) = variant.smoothed else {
        return;
    };
    let path = if filter.is_some() {
        smoothed
    } else {
        variant.path
    };
    let ids = slot.ids();
    model.set(
        ids.preset,
        shader_panel::with_pre_filter(filter, path).as_str(),
    );
}

// --- Labels ------------------------------------------------------------------------------------

/// The label of pick or knob `key`, in the language in force.
#[must_use]
pub fn option_label(key: &str) -> String {
    super::strings::get(&format!("shaders.option.{key}"))
}

/// The label of choice `key`.
#[must_use]
pub fn choice_label(key: &str) -> String {
    super::strings::get(&format!("shaders.choice.{key}"))
}

/// The label of step `key`.
#[must_use]
pub fn step_label(key: &str) -> String {
    super::strings::get(&format!("shaders.step.{key}"))
}

/// The label of a knob's step as the slot holds it: "Custom" for a value set by hand.
#[must_use]
pub fn step_text(knob: &Knob, step: Option<usize>) -> String {
    step.and_then(|step| knob.steps.get(step))
        .map_or_else(|| step_label("custom"), |key| step_label(key))
}

/// The sub-options of a family as a flat list, picks first: what the console's rows step through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubOption {
    /// Pick `n` of [`Look::picks`].
    Pick(usize),
    /// Knob `n` of [`Look::knobs`].
    Knob(usize),
}

/// The sub-options `state` offers: every pick of more than one offered choice, and the knobs its
/// variant has.
#[must_use]
pub fn sub_options(state: &LookState) -> Vec<SubOption> {
    let look = state.look();
    let picks = look
        .picks
        .iter()
        .enumerate()
        .filter(|(index, pick)| {
            (0..pick.choices.len())
                .filter(|choice| choice_offered(look, *index, *choice))
                .count()
                > 1
        })
        .map(|(index, _)| SubOption::Pick(index));
    let knobs = (0..look.knobs.len())
        .filter(|knob| has_knob(state, *knob))
        .map(SubOption::Knob);
    picks.chain(knobs).collect()
}

/// The label and the value of `option` as `state` holds it.
#[must_use]
pub fn sub_option_text(state: &LookState, option: SubOption) -> (String, String) {
    let look = state.look();
    match option {
        SubOption::Pick(pick) => {
            let entry = &look.picks[pick];
            let choice = state.variant().choice.get(pick).copied().unwrap_or(0);
            (
                option_label(entry.label),
                entry
                    .choices
                    .get(choice)
                    .map_or_else(String::new, |key| choice_label(key)),
            )
        }
        SubOption::Knob(knob) => {
            let entry = &look.knobs[knob];
            (
                option_label(entry.label),
                step_text(entry, state.steps.get(knob).copied().flatten()),
            )
        }
    }
}

/// Steps `option` of the family `slot` holds by `step`, wrapping round: the next offered choice
/// of a pick, the next step of a knob.
pub fn step_sub_option(model: &mut SettingsModel, slot: ShaderSlot, option: SubOption, step: i32) {
    let Some(state) = state_of(model, slot) else {
        return;
    };
    let look = state.look();
    let cycle = |count: usize, from: usize| -> usize {
        let count = i32::try_from(count).unwrap_or(1).max(1);
        let from = i32::try_from(from).unwrap_or(0);
        usize::try_from((from + step).rem_euclid(count)).unwrap_or(0)
    };
    match option {
        SubOption::Pick(pick) => {
            let Some(entry) = look.picks.get(pick) else {
                return;
            };
            let mut choice = state.variant().choice.get(pick).copied().unwrap_or(0);
            for _ in 0..entry.choices.len() {
                choice = cycle(entry.choices.len(), choice);
                if choice_offered(look, pick, choice) {
                    break;
                }
            }
            choose_pick(model, slot, pick, choice);
        }
        SubOption::Knob(knob) => {
            let Some(map) = look
                .knobs
                .get(knob)
                .and_then(|entry| entry.map_for(state.variant().path))
            else {
                return;
            };
            let from = state.steps.get(knob).copied().flatten().unwrap_or(0);
            choose_step(model, slot, knob, cycle(map.steps.len(), from));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOT: ShaderSlot = ShaderSlot::TwoD;

    #[test]
    fn every_family_is_coherent() {
        for look in &LOOKS {
            for variant in look.variants {
                assert_eq!(look.picks.len(), variant.choice.len(), "{}", look.key);
                for (pick, choice) in variant.choice.iter().enumerate() {
                    assert!(*choice < look.picks[pick].choices.len(), "{}", look.key);
                }
            }
            // No two variants stand for the same choices.
            for (index, variant) in look.variants.iter().enumerate() {
                assert!(
                    !look.variants[..index]
                        .iter()
                        .any(|other| other.choice == variant.choice),
                    "{}",
                    look.key
                );
            }
            for knob in look.knobs {
                for map in knob.maps {
                    assert_eq!(
                        knob.steps.len(),
                        map.steps.len(),
                        "{} {}",
                        look.key,
                        knob.label
                    );
                    assert!(
                        map.steps.iter().filter(|step| step.is_empty()).count() == 1,
                        "{} {}: one step is the preset's own",
                        look.key,
                        knob.label
                    );
                    for path in map.paths {
                        assert!(
                            look.variants.iter().any(|variant| variant.path == *path),
                            "{} {}: {path}",
                            look.key,
                            knob.label
                        );
                    }
                }
            }
        }
    }

    /// Every preset file of the families is in the pack (the list the build zips).
    #[test]
    fn every_variant_is_in_the_pack() {
        let list = include_str!("../../shaders/pack-files.txt");
        for look in &LOOKS {
            for variant in look.variants {
                for path in std::iter::once(variant.path).chain(variant.smoothed) {
                    assert!(list.lines().any(|line| line == path), "{path}");
                }
            }
        }
    }

    #[test]
    fn a_family_its_picks_and_steps_read_back() {
        let mut model = SettingsModel::new();
        let guest = LOOKS.iter().position(|look| look.key == "guest").unwrap();
        choose_look(&mut model, SLOT, guest);
        let state = state_of(&model, SLOT).unwrap();
        assert_eq!((guest, 0), (state.look, state.variant));
        // The preset's own steps: flat, normal scanlines, fine mask, normal brightness.
        assert_eq!(vec![Some(0), Some(2), Some(1), Some(1)], state.steps);

        choose_step(&mut model, SLOT, 0, 3);
        choose_pick(&mut model, SLOT, 0, 2);
        let state = state_of(&model, SLOT).unwrap();
        assert_eq!(2, state.variant, "composite");
        assert_eq!(
            Some(3),
            state.steps[0],
            "the curvature kept through the signal"
        );

        // A value set by hand reads as custom.
        let ids = SLOT.ids();
        let text =
            shader_slots::with_parameter(model.get(ids.params).as_text(), "warpX", Some(0.11));
        model.set(ids.params, text.as_str());
        assert_eq!(None, state_of(&model, SLOT).unwrap().steps[0]);
    }

    #[test]
    fn the_mega_bezel_frame_switches_the_file_and_the_area() {
        let mut model = SettingsModel::new();
        let bezel = LOOKS
            .iter()
            .position(|look| look.key == "mega_bezel")
            .unwrap();
        choose_look(&mut model, SLOT, bezel);
        assert_eq!(MB_STD, model.get(SLOT.ids().preset).as_text());
        choose_pick(&mut model, SLOT, 0, 1);
        assert_eq!(MB_S_STD, model.get(SLOT.ids().preset).as_text());
        assert_eq!(AREA_PICTURE, model.get(SLOT.ids().area).as_int());
        let state = state_of(&model, SLOT).unwrap();
        assert!(!has_knob(&state, 1), "no reflections without the frame");
    }

    #[test]
    fn a_pre_filter_takes_guest_to_its_smoothed_version_and_back() {
        let mut model = SettingsModel::new();
        let guest = LOOKS.iter().position(|look| look.key == "guest").unwrap();
        choose_look(&mut model, SLOT, guest);
        let filter = &shader_panel::PRE_FILTERS[2];
        set_pre_filter(&mut model, SLOT, Some(filter));
        assert_eq!(
            format!(
                "{}{}{GUEST_HD}",
                filter.path,
                crate::shader_pack::COMBINATION_SEPARATOR
            ),
            model.get(SLOT.ids().preset).as_text()
        );
        assert_eq!(Some(guest), state_of(&model, SLOT).map(|state| state.look));
        set_pre_filter(&mut model, SLOT, None);
        assert_eq!(GUEST, model.get(SLOT.ids().preset).as_text());
    }
}
