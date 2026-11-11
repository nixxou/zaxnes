//! The model of the shaders window (Rust-only, owner request), kept apart from its `ImGui` drawing
//! so that the console mode can show the same choices with a gamepad later: the catalogue of
//! presets of the shader pack, the menu of every preset of the pack, copying one slot to
//! another, the state of the preset in use and the parameters of a preset, read on a thread of
//! their own.

use std::path::Path;

use zax_settings::model::SettingsModel;
use zax_settings::shader_slots::{self, ShaderSlot, OFF, PRESET};
use zax_settings::values::SettingValue;

/// The kind of look a preset of the catalogue gives, its submenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// A cathode-ray tube: scanlines, mask, glow.
    Crt,
    /// A tube fed by a composite (NTSC) signal: the colour bleed of the console's own output.
    Composite,
    /// A TV around the picture (Mega Bezel), or its screen alone.
    Bezel,
    /// A handheld's LCD.
    Lcd,
    /// Pixel art smoothed or scaled sharp, without a tube.
    Smoothing,
    /// Several presets one after the other: a pre-filter, then a tube (owner request).
    Combination,
}

impl Family {
    /// The families in the order of the menu.
    pub const ALL: [Self; 6] = [
        Self::Crt,
        Self::Composite,
        Self::Bezel,
        Self::Lcd,
        Self::Smoothing,
        Self::Combination,
    ];

    /// The key of its name in the string tables.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Crt => "shaders.family.crt",
            Self::Composite => "shaders.family.composite",
            Self::Bezel => "shaders.family.bezel",
            Self::Lcd => "shaders.family.lcd",
            Self::Smoothing => "shaders.family.smoothing",
            Self::Combination => "shaders.family.combination",
        }
    }
}

/// What a preset asks of the graphics card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    /// Fit for a weak card or a handheld.
    Light,
    /// A desktop card.
    Medium,
    /// A strong card.
    Heavy,
}

impl Cost {
    /// The key of its name in the string tables.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Light => "shaders.cost.light",
            Self::Medium => "shaders.cost.medium",
            Self::Heavy => "shaders.cost.heavy",
        }
    }
}

/// The display modes a preset is put forward for, in the recommended part of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Featured {
    /// Not put forward.
    No,
    /// In every mode.
    Always,
    /// In the 4:3 modes.
    Narrow,
    /// In the widescreen modes.
    Wide,
}

/// A preset of the catalogue: a path in the shader pack and what choosing it sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CatalogPreset {
    /// The key of its name in the string tables.
    pub label: &'static str,
    /// Its path in the pack (libretro/slang-shaders).
    pub path: &'static str,
    /// Its submenu.
    pub family: Family,
    /// What it asks of the card.
    pub cost: Cost,
    /// Where it is recommended.
    pub featured: Featured,
    /// The parameters it starts with in the 4:3 modes, beside the preset's own.
    pub parameters: &'static [(&'static str, f32)],
    /// The parameters it starts with in the widescreen modes.
    pub wide_parameters: &'static [(&'static str, f32)],
    /// Where it draws (`shader_slots::AREA_*`).
    pub area: i32,
}

/// The Mega Bezel screen at the aspect of the picture it reads (PAR), for the widescreen modes:
/// a widescreen picture gets a widescreen TV, where the preset's own guess squeezes it into 4:3
/// (which suits the 4:3 modes, left at the preset's own).
const MEGA_BEZEL_TV: &[(&str, f32)] = &[("HSM_ASPECT_RATIO_MODE", 5.0)];
/// The Mega Bezel screen over the whole area it draws in (the picture, of any aspect).
const MEGA_BEZEL_SCREEN: &[(&str, f32)] = &[("HSM_ASPECT_RATIO_MODE", 6.0)];

const fn entry(
    label: &'static str,
    path: &'static str,
    family: Family,
    cost: Cost,
    featured: Featured,
) -> CatalogPreset {
    CatalogPreset {
        label,
        path,
        family,
        cost,
        featured,
        parameters: &[],
        wide_parameters: &[],
        area: shader_slots::AREA_AUTO,
    }
}

const fn tv(entry: CatalogPreset) -> CatalogPreset {
    CatalogPreset {
        wide_parameters: MEGA_BEZEL_TV,
        ..entry
    }
}

const fn screen(entry: CatalogPreset) -> CatalogPreset {
    CatalogPreset {
        parameters: MEGA_BEZEL_SCREEN,
        wide_parameters: MEGA_BEZEL_SCREEN,
        area: shader_slots::AREA_PICTURE,
        ..entry
    }
}

/// The catalogue: the presets most played with NES-era consoles, from the light to the heavy
/// (the guides and the libretro forums put forward Guest Advanced and its NTSC variant, Mega
/// Bezel, Royale, Lottes, Easymode, and zfast or CRT-Pi for the weak cards).
pub const CATALOGUE: [CatalogPreset; 32] = [
    entry(
        "shaders.preset.guest",
        "crt/crt-guest-advanced.slangp",
        Family::Crt,
        Cost::Medium,
        Featured::Always,
    ),
    entry(
        "shaders.preset.easymode",
        "crt/crt-easymode.slangp",
        Family::Crt,
        Cost::Light,
        Featured::No,
    ),
    entry(
        "shaders.preset.zfast",
        "crt/zfast-crt.slangp",
        Family::Crt,
        Cost::Light,
        Featured::Always,
    ),
    entry(
        "shaders.preset.crt_pi",
        "crt/crt-pi.slangp",
        Family::Crt,
        Cost::Light,
        Featured::Always,
    ),
    entry(
        "shaders.preset.fakelottes",
        "crt/fakelottes.slangp",
        Family::Crt,
        Cost::Light,
        Featured::No,
    ),
    entry(
        "shaders.preset.mattias",
        "crt/crt-mattias.slangp",
        Family::Crt,
        Cost::Medium,
        Featured::No,
    ),
    entry(
        "shaders.preset.nes_mini",
        "crt/crt-nes-mini.slangp",
        Family::Crt,
        Cost::Light,
        Featured::No,
    ),
    entry(
        "shaders.preset.lottes",
        "crt/crt-lottes.slangp",
        Family::Crt,
        Cost::Medium,
        Featured::No,
    ),
    entry(
        "shaders.preset.hyllian",
        "crt/crt-hyllian.slangp",
        Family::Crt,
        Cost::Medium,
        Featured::No,
    ),
    entry(
        "shaders.preset.geom",
        "crt/crt-geom.slangp",
        Family::Crt,
        Cost::Medium,
        Featured::No,
    ),
    entry(
        "shaders.preset.royale",
        "crt/crt-royale.slangp",
        Family::Crt,
        Cost::Heavy,
        Featured::No,
    ),
    entry(
        "shaders.preset.guest_ntsc",
        "crt/crt-guest-advanced-ntsc.slangp",
        Family::Composite,
        Cost::Medium,
        Featured::Always,
    ),
    entry(
        "shaders.preset.ntsc_nes",
        "ntsc/ntsc-256px-composite-scanline.slangp",
        Family::Composite,
        Cost::Light,
        Featured::No,
    ),
    entry(
        "shaders.preset.blargg",
        "ntsc/blargg.slangp",
        Family::Composite,
        Cost::Light,
        Featured::No,
    ),
    entry(
        "shaders.preset.zfast_composite",
        "crt/zfast-crt-composite.slangp",
        Family::Composite,
        Cost::Light,
        Featured::No,
    ),
    entry(
        "shaders.preset.fakelottes_ntsc",
        "presets/crt-plus-signal/fakelottes-ntsc-composite.slangp",
        Family::Composite,
        Cost::Light,
        Featured::No,
    ),
    entry(
        "shaders.preset.royale_nes_rf",
        "presets/crt-royale-fast/crt-royale-fast-ntsc-rf-slotmask-nes.slangp",
        Family::Composite,
        Cost::Heavy,
        Featured::No,
    ),
    tv(entry(
        "shaders.preset.mega_bezel",
        "bezel/Mega_Bezel/Presets/MBZ__3__STD.slangp",
        Family::Bezel,
        Cost::Heavy,
        Featured::Narrow,
    )),
    tv(entry(
        "shaders.preset.mega_bezel_light",
        "bezel/Mega_Bezel/Presets/MBZ__5__POTATO.slangp",
        Family::Bezel,
        Cost::Medium,
        Featured::No,
    )),
    tv(entry(
        "shaders.preset.mega_bezel_reflect",
        "bezel/Mega_Bezel/Presets/MBZ__1__ADV.slangp",
        Family::Bezel,
        Cost::Heavy,
        Featured::No,
    )),
    screen(entry(
        "shaders.preset.mega_bezel_screen",
        "bezel/Mega_Bezel/Presets/Variations/Screen-Only/Default/_4__STD-SCREEN-ONLY.slangp",
        Family::Bezel,
        Cost::Medium,
        Featured::Always,
    )),
    screen(entry(
        "shaders.preset.mega_bezel_screen_ntsc",
        "bezel/Mega_Bezel/Presets/Variations/Screen-Only/Default/_4__STD-SCREEN-ONLY__NTSC.slangp",
        Family::Bezel,
        Cost::Medium,
        Featured::No,
    )),
    screen(entry(
        "shaders.preset.mega_bezel_screen_light",
        "bezel/Mega_Bezel/Presets/Variations/Screen-Only/Default/_5__POTATO.slangp",
        Family::Bezel,
        Cost::Light,
        Featured::Always,
    )),
    screen(entry(
        "shaders.preset.mega_bezel_screen_smooth",
        "bezel/Mega_Bezel/Presets/Base_CRT_Presets/MBZ__0__SMOOTH-ADV-SCREEN-ONLY__GDV.slangp",
        Family::Bezel,
        Cost::Heavy,
        Featured::No,
    )),
    entry(
        "shaders.preset.lcd",
        "handheld/lcd-grid-v2.slangp",
        Family::Lcd,
        Cost::Light,
        Featured::Narrow,
    ),
    entry(
        "shaders.preset.sharp",
        "pixel-art-scaling/sharp-bilinear-simple.slangp",
        Family::Smoothing,
        Cost::Light,
        Featured::No,
    ),
    entry(
        "shaders.preset.xbrz",
        "edge-smoothing/xbrz/xbrz-freescale.slangp",
        Family::Smoothing,
        Cost::Medium,
        Featured::No,
    ),
    entry(
        "shaders.preset.scalefx",
        "edge-smoothing/scalefx/scalefx.slangp",
        Family::Smoothing,
        Cost::Medium,
        Featured::Always,
    ),
    // The combinations (owner request), stored as the presets one after the other
    // (`shader_pack::COMBINATION_SEPARATOR`): ScaleFX before the Guest tube made for a smoothed
    // picture, the one the libretro forums give, and the owner's three with nnedi3.
    entry(
        "shaders.preset.scalefx_guest_hd",
        "edge-smoothing/scalefx/scalefx.slangp|crt/crt-guest-advanced-hd.slangp",
        Family::Combination,
        Cost::Heavy,
        Featured::Always,
    ),
    entry(
        "shaders.preset.nnedi3_hyllian",
        "edge-smoothing/nnedi3/nnedi3-nns32-4x-rgb.slangp|crt/crt-hyllian.slangp",
        Family::Combination,
        Cost::Heavy,
        Featured::No,
    ),
    entry(
        "shaders.preset.nnedi3_easymode",
        "edge-smoothing/nnedi3/nnedi3-nns32-4x-rgb.slangp|crt/crt-easymode.slangp",
        Family::Combination,
        Cost::Heavy,
        Featured::Always,
    ),
    entry(
        "shaders.preset.nnedi3_mattias",
        "edge-smoothing/nnedi3/nnedi3-nns32-4x-rgb.slangp|crt/crt-mattias.slangp|scanlines/scanline.slangp",
        Family::Combination,
        Cost::Heavy,
        Featured::No,
    ),
];

/// The name of `slot`, in the language in force.
#[must_use]
pub fn slot_label(slot: ShaderSlot) -> String {
    crate::ui::strings::get(match slot {
        ShaderSlot::TwoD => "shaders.slot.2d",
        ShaderSlot::TwoDWide => "shaders.slot.2d_ws",
        ShaderSlot::ThreeD => "shaders.slot.3d",
        ShaderSlot::ThreeDWide => "shaders.slot.3d_ws",
    })
}

/// The name a stored preset is shown by: its catalogue name; for a combination outside the
/// catalogue, the names of its presets joined by " + " (a preset outside the catalogue by its
/// file name).
#[must_use]
pub fn preset_label(stored: &str) -> String {
    if let Some(preset) = catalogue_entry(stored) {
        return crate::ui::strings::get(preset.label);
    }
    if !stored.contains(crate::shader_pack::COMBINATION_SEPARATOR) {
        return stored.to_owned();
    }
    crate::shader_pack::parts(stored)
        .into_iter()
        .map(|part| {
            catalogue_entry(part).map_or_else(
                || {
                    Path::new(part).file_stem().map_or_else(
                        || part.to_owned(),
                        |stem| stem.to_string_lossy().into_owned(),
                    )
                },
                |preset| crate::ui::strings::get(preset.label),
            )
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// A pre-filter (owner request): a preset that enlarges or smooths the picture before the
/// preset of the slot, stored in front of it as a combination ("pre-filter|preset").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreFilter {
    /// The key of its name in the string tables.
    pub label: &'static str,
    /// Its path in the pack.
    pub path: &'static str,
}

/// The pre-filters a slot offers, from the lightest. Each ends larger than the picture it reads,
/// once the resampling to the window that ends nnedi3 and xBRZ draws at its own input's size
/// (`zax_platform::slang_chain::parse_combination`).
pub const PRE_FILTERS: [PreFilter; 5] = [
    PreFilter {
        label: "shaders.prefilter.xbrz2",
        path: "edge-smoothing/xbrz/2xbrz-linear.slangp",
    },
    PreFilter {
        label: "shaders.prefilter.xbrz4",
        path: "edge-smoothing/xbrz/4xbrz-linear.slangp",
    },
    PreFilter {
        label: "shaders.prefilter.scalefx",
        path: "edge-smoothing/scalefx/scalefx.slangp",
    },
    PreFilter {
        label: "shaders.prefilter.nnedi3_2",
        path: "edge-smoothing/nnedi3/nnedi3-nns16-2x-rgb.slangp",
    },
    PreFilter {
        label: "shaders.prefilter.nnedi3_4",
        path: "edge-smoothing/nnedi3/nnedi3-nns32-4x-rgb.slangp",
    },
];

/// The pre-filter in front of the stored preset `stored`, if it has one of [`PRE_FILTERS`], and
/// the preset behind it (the whole of `stored` otherwise).
#[must_use]
pub fn split_pre_filter(stored: &str) -> (Option<&'static PreFilter>, &str) {
    if let Some((first, rest)) = stored.split_once(crate::shader_pack::COMBINATION_SEPARATOR) {
        let first = first.replace('\\', "/");
        if let Some(filter) = PRE_FILTERS.iter().find(|filter| filter.path == first) {
            return (Some(filter), rest);
        }
    }
    (None, stored)
}

/// `preset` behind the pre-filter `filter`, as the settings store it.
#[must_use]
pub fn with_pre_filter(filter: Option<&PreFilter>, preset: &str) -> String {
    match filter {
        Some(filter) if !preset.is_empty() => format!(
            "{}{}{preset}",
            filter.path,
            crate::shader_pack::COMBINATION_SEPARATOR
        ),
        _ => preset.to_owned(),
    }
}

/// The pre-filter of `slot` in `model`.
#[must_use]
pub fn pre_filter_of(model: &SettingsModel, slot: ShaderSlot) -> Option<&'static PreFilter> {
    split_pre_filter(model.get(slot.ids().preset).as_text()).0
}

/// Puts `filter` in front of the preset of `slot`, or takes the one there away; the preset and
/// its parameters stay.
pub fn set_pre_filter(model: &mut SettingsModel, slot: ShaderSlot, filter: Option<&PreFilter>) {
    let ids = slot.ids();
    let stored = model.get(ids.preset).as_text().to_owned();
    let (_, preset) = split_pre_filter(&stored);
    model.set(ids.preset, with_pre_filter(filter, preset).as_str());
}

/// The name of `filter`, "None" for none.
#[must_use]
pub fn pre_filter_label(filter: Option<&PreFilter>) -> String {
    crate::ui::strings::get(filter.map_or("shaders.prefilter.none", |filter| filter.label))
}

/// What a slot shows, as the console's row of it steps through: the slot it follows, nothing,
/// then the presets of the catalogue (a preset of its own, chosen on the desktop, is shown and
/// left at the first step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotPick {
    /// [`shader_slots::FOLLOW`].
    Follow,
    /// [`shader_slots::OFF`].
    Off,
    /// A preset, by its path in the pack.
    Preset(String),
}

/// Every pick of `slot`, in the order the console steps through them.
#[must_use]
pub fn slot_picks(slot: ShaderSlot) -> Vec<SlotPick> {
    let mut picks = Vec::with_capacity(CATALOGUE.len() + 2);
    if slot.fallback().is_some() {
        picks.push(SlotPick::Follow);
    }
    picks.push(SlotPick::Off);
    picks.extend(
        CATALOGUE
            .iter()
            .map(|preset| SlotPick::Preset(preset.path.to_owned())),
    );
    picks
}

/// The pick `slot` holds in `model`.
#[must_use]
pub fn slot_pick(model: &SettingsModel, slot: ShaderSlot) -> SlotPick {
    let ids = slot.ids();
    match model.get(ids.mode).as_int() {
        shader_slots::FOLLOW if slot.fallback().is_some() => SlotPick::Follow,
        // A combination of the catalogue is one of the picks; otherwise the preset behind the
        // pre-filter, which has a row of its own.
        PRESET => {
            let value = model.get(ids.preset);
            let stored = value.as_text();
            if catalogue_entry(stored).is_some() {
                SlotPick::Preset(stored.to_owned())
            } else {
                SlotPick::Preset(split_pre_filter(stored).1.to_owned())
            }
        }
        _ => SlotPick::Off,
    }
}

/// Puts `pick` in `slot`: a preset with what the catalogue starts it with ([`choose_preset`]).
pub fn set_slot_pick(model: &mut SettingsModel, slot: ShaderSlot, pick: &SlotPick) {
    let ids = slot.ids();
    match pick {
        SlotPick::Follow => {
            model.set(ids.mode, SettingValue::Enum(shader_slots::FOLLOW));
        }
        SlotPick::Off => {
            model.set(ids.mode, SettingValue::Enum(OFF));
        }
        SlotPick::Preset(path) => choose_preset(model, slot, path),
    }
}

/// The text of `pick` in `slot`'s row.
#[must_use]
pub fn pick_label(slot: ShaderSlot, pick: &SlotPick) -> String {
    use crate::ui::strings;
    match pick {
        SlotPick::Follow => slot.fallback().map_or_else(String::new, |parent| {
            strings::format(&strings::get("shaders.mode.follow"), &[&slot_label(parent)])
        }),
        SlotPick::Off => strings::get("shaders.mode.off"),
        SlotPick::Preset(path) if path.is_empty() => strings::get("shaders.none"),
        SlotPick::Preset(path) => preset_label(path),
    }
}

/// Whether `preset` is put forward for `slot`.
#[must_use]
pub const fn featured_for(preset: &CatalogPreset, slot: ShaderSlot) -> bool {
    let (_, wide) = mode_of(slot);
    match preset.featured {
        Featured::No => false,
        Featured::Always => true,
        Featured::Narrow => !wide,
        Featured::Wide => wide,
    }
}

/// The catalogue entry of the stored preset `path`, if it is one.
#[must_use]
pub fn catalogue_entry(path: &str) -> Option<&'static CatalogPreset> {
    let path = path.replace('\\', "/");
    CATALOGUE.iter().find(|preset| preset.path == path)
}

/// The parameters a preset of the catalogue starts with in `slot`, as the settings store them
/// (none for a preset outside it, save the Mega Bezel intro turned off): what `Reset all`
/// returns it to.
#[must_use]
pub fn starting_parameters(path: &str, slot: ShaderSlot) -> String {
    let (_, wide) = mode_of(slot);
    // A catalogue preset behind a pre-filter starts as the preset alone does.
    let parameters: Vec<(String, f32)> = catalogue_entry(path)
        .or_else(|| catalogue_entry(split_pre_filter(path).1))
        .map(|preset| {
            if wide {
                preset.wide_parameters
            } else {
                preset.parameters
            }
            .iter()
            .map(|(name, value)| ((*name).to_owned(), *value))
            .collect()
        })
        .unwrap_or_default();
    // The Mega Bezel presets open on an animated intro of their own at every game loaded: the
    // pack starts them without it (owner request), the player can turn it back on.
    let mut parameters = parameters;
    if path
        .to_ascii_lowercase()
        .replace('\\', "/")
        .contains("mega_bezel/")
        && !parameters.iter().any(|(name, _)| name == MEGA_BEZEL_INTRO)
    {
        parameters.insert(0, (MEGA_BEZEL_INTRO.to_owned(), 0.0));
    }
    shader_slots::format_parameters(&parameters)
}

/// The Mega Bezel parameter of its intro: off, at the game's load, at the preset's, repeated.
const MEGA_BEZEL_INTRO: &str = "HSM_INTRO_WHEN_TO_SHOW";

/// Sets `slot` to the preset `path` of the pack, with the catalogue's parameters and area when
/// it is one of its entries (none and the automatic area otherwise). A preset without a
/// pre-filter of its own keeps the one of the slot (owner request: the pre-filter is a row of its
/// own); a combination of the catalogue brings its own, or none.
pub fn choose_preset(model: &mut SettingsModel, slot: ShaderSlot, path: &str) {
    let ids = slot.ids();
    // Owner review: a pre-filter that came with a combination of the catalogue goes with it; only
    // one the player put on the pre-filter row stays.
    let current = model.get(ids.preset).as_text().to_owned();
    let came_with_combination =
        catalogue_entry(&current).is_some_and(|preset| preset.family == Family::Combination);
    let kept = if came_with_combination
        || split_pre_filter(path).0.is_some()
        || catalogue_entry(path).is_some_and(|preset| preset.family == Family::Combination)
    {
        None
    } else {
        pre_filter_of(model, slot)
    };
    let path = with_pre_filter(kept, path);
    let path = path.as_str();
    let area = catalogue_entry(path)
        .or_else(|| catalogue_entry(split_pre_filter(path).1))
        .map_or(shader_slots::AREA_AUTO, |preset| preset.area);
    model.set(ids.mode, SettingValue::Enum(PRESET));
    model.set(ids.preset, path);
    model.set(ids.params, starting_parameters(path, slot).as_str());
    model.set(ids.area, SettingValue::Enum(area));
}

/// A folder of the shader pack, for the menu of every preset.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresetFolder {
    /// Its name.
    pub name: String,
    /// Its folders that hold presets, by name.
    pub folders: Vec<PresetFolder>,
    /// Its presets: the name shown and the path in the pack.
    pub presets: Vec<(String, String)>,
}

impl PresetFolder {
    /// The presets (`*.slangp`) under `folder`, `relative` being its path in the pack.
    #[must_use]
    pub fn read(folder: &Path, relative: &str) -> Self {
        let mut node = Self {
            name: folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            ..Self::default()
        };
        let Ok(entries) = std::fs::read_dir(folder) else {
            return node;
        };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if relative.is_empty() {
                name.clone()
            } else {
                format!("{relative}/{name}")
            };
            if entry.path().is_dir() {
                let child = Self::read(&entry.path(), &path);
                if !child.is_empty() {
                    node.folders.push(child);
                }
            } else if let Some(stem) = name.strip_suffix(".slangp") {
                node.presets.push((stem.to_owned(), path));
            }
        }
        node
    }

    /// Whether it holds no preset at any depth.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.presets.is_empty() && self.folders.iter().all(Self::is_empty)
    }
}

/// The display mode of `slot`: (3D, widescreen).
#[must_use]
pub const fn mode_of(slot: ShaderSlot) -> (bool, bool) {
    match slot {
        ShaderSlot::TwoD => (false, false),
        ShaderSlot::TwoDWide => (false, true),
        ShaderSlot::ThreeD => (true, false),
        ShaderSlot::ThreeDWide => (true, true),
    }
}

/// Copies the shader `from` shows (its own, or the one it follows) into `to`: the preset, its
/// parameters and its area, `to` then using that preset itself. The lines stay `to`'s own, as
/// they belong to the display mode (240 suit the flat picture, the 3D scene keeps the window's).
/// A source without a shader turns `to` off.
pub fn copy_slot(model: &mut SettingsModel, from: ShaderSlot, to: ShaderSlot) {
    if from == to {
        return;
    }
    let (three_d, wide) = mode_of(from);
    let target = to.ids();
    match shader_slots::resolve(model, three_d, wide) {
        Some(choice) => {
            let source = choice.slot.ids();
            let area = model.get(source.area).as_int();
            model.set(target.mode, SettingValue::Enum(PRESET));
            model.set(target.preset, choice.preset.as_str());
            model.set(
                target.params,
                shader_slots::format_parameters(&choice.parameters).as_str(),
            );
            model.set(target.area, SettingValue::Enum(area));
        }
        None => {
            model.set(target.mode, SettingValue::Enum(OFF));
        }
    }
}

/// What the renderer is doing with the shaders, which the application writes every frame for
/// the window to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShaderState {
    /// No preset asked for.
    #[default]
    None,
    /// The renderer is OpenGL: the shaders need Vulkan or DirectX 12.
    NoWgpu,
    /// The preset is compiling.
    Compiling,
    /// The preset filters the picture.
    Active,
    /// The preset was refused.
    Refused,
}

/// The state of the shaders for the window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShaderReport {
    /// What the renderer is doing.
    pub state: ShaderState,
    /// The preset concerned, as a file.
    pub preset: String,
    /// Why it was refused.
    pub problem: String,
    /// The slot of the display mode on screen.
    pub mode_slot: Option<ShaderSlot>,
}

#[cfg(feature = "shaders")]
pub use parameters::ParameterCache;

#[cfg(feature = "shaders")]
mod parameters {
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{self, Receiver, TryRecvError};

    use zax_platform::slang_chain::{preset_parameters, ParameterInfo};

    /// The parameters of the preset the window shows, read on a thread (a Mega Bezel preset
    /// reads some thirty shader files).
    #[derive(Default)]
    pub struct ParameterCache {
        preset: Option<PathBuf>,
        loading: Option<Receiver<Result<Vec<ParameterInfo>, String>>>,
        result: Option<Result<Vec<ParameterInfo>, String>>,
    }

    impl ParameterCache {
        /// Asks for the parameters of `preset`; nothing is read again for the same one.
        pub fn want(&mut self, preset: &Path) {
            if self.preset.as_deref() == Some(preset) {
                self.poll();
                return;
            }
            self.preset = Some(preset.to_path_buf());
            self.result = None;
            let (sender, receiver) = mpsc::channel();
            let path = preset.to_path_buf();
            let started = std::thread::Builder::new()
                .name("zax slang parameters".to_owned())
                .spawn(move || {
                    let _ = sender.send(preset_parameters(&path));
                });
            if started.is_err() {
                self.result = Some(preset_parameters(preset));
                return;
            }
            self.loading = Some(receiver);
        }

        fn poll(&mut self) {
            let Some(loading) = &self.loading else {
                return;
            };
            match loading.try_recv() {
                Ok(result) => {
                    self.result = Some(result);
                    self.loading = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.result = Some(Err("the parameters could not be read".to_owned()));
                    self.loading = None;
                }
            }
        }

        /// The parameters, an error, or none while they are being read.
        #[must_use]
        pub fn result(&self) -> Option<&Result<Vec<ParameterInfo>, String>> {
            self.result.as_ref()
        }
    }
}

/// The value a parameter is shown at: the player's, or the preset's own.
#[must_use]
pub fn parameter_value(overrides: &[(String, f32)], name: &str, initial: f32) -> f32 {
    overrides
        .iter()
        .find(|(known, _)| known == name)
        .map_or(initial, |(_, value)| *value)
}

/// `value` on the grid of `step` from `minimum`, within the range.
#[must_use]
pub fn snap(value: f32, minimum: f32, maximum: f32, step: f32) -> f32 {
    let value = if step > 0.0 {
        minimum + ((value - minimum) / step).round() * step
    } else {
        value
    };
    value.clamp(minimum.min(maximum), maximum.max(minimum))
}

/// The `printf` format of a slider of `step`: as many decimals as the step has, at most four.
#[must_use]
pub fn slider_format(step: f32) -> &'static str {
    let mut decimals = 0;
    let mut scaled = step.abs();
    while decimals < 4 && scaled > 0.0 && (scaled - scaled.round()).abs() > 1.0e-4 {
        scaled *= 10.0;
        decimals += 1;
    }
    match decimals {
        0 => "%.0f",
        1 => "%.1f",
        2 => "%.2f",
        3 => "%.3f",
        _ => "%.4f",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zax_settings::catalogue::SettingId;

    #[test]
    fn copying_takes_the_preset_shown_and_keeps_the_lines() {
        let mut model = SettingsModel::new();
        model.set(SettingId::Shader2dMode, SettingValue::Enum(PRESET));
        model.set(SettingId::Shader2dPreset, "a.slangp");
        model.set(SettingId::Shader2dParams, "X=1");
        model.set(SettingId::Shader2dArea, SettingValue::Enum(2));
        // The 2D wide slot follows the 2D one: copying it gives 3D the 2D preset.
        copy_slot(&mut model, ShaderSlot::TwoDWide, ShaderSlot::ThreeD);
        assert_eq!(model.get(SettingId::Shader3dMode).as_int(), PRESET);
        assert_eq!(model.get(SettingId::Shader3dPreset).as_text(), "a.slangp");
        assert_eq!(model.get(SettingId::Shader3dParams).as_text(), "X=1");
        assert_eq!(model.get(SettingId::Shader3dArea).as_int(), 2);
        assert_eq!(model.get(SettingId::Shader3dLines).as_int(), 0);
        // A slot without a shader turns the target off.
        model.set(SettingId::Shader2dMode, SettingValue::Enum(OFF));
        copy_slot(&mut model, ShaderSlot::TwoD, ShaderSlot::ThreeD);
        assert_eq!(model.get(SettingId::Shader3dMode).as_int(), OFF);
    }

    #[test]
    fn values_snap_to_the_step_and_formats_follow_it() {
        assert!((snap(0.337, 0.0, 1.0, 0.05) - 0.35).abs() < 1.0e-6);
        assert!((snap(7.0, 0.0, 5.0, 0.5) - 5.0).abs() < 1.0e-6);
        assert_eq!(slider_format(1.0), "%.0f");
        assert_eq!(slider_format(0.05), "%.2f");
        assert_eq!(slider_format(0.5), "%.1f");
        assert!((parameter_value(&[("A".to_owned(), 2.0)], "A", 1.0) - 2.0).abs() < 1.0e-6);
        assert!((parameter_value(&[], "A", 1.0) - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn choosing_a_catalogue_preset_sets_its_parameters_and_area() {
        let mut model = SettingsModel::new();
        let screen = catalogue_entry(
            "bezel/Mega_Bezel/Presets/Variations/Screen-Only/Default/_4__STD-SCREEN-ONLY.slangp",
        )
        .expect("the Mega Bezel screen");
        choose_preset(&mut model, ShaderSlot::TwoDWide, screen.path);
        assert_eq!(model.get(SettingId::Shader2dWsMode).as_int(), PRESET);
        assert_eq!(
            model.get(SettingId::Shader2dWsPreset).as_text(),
            screen.path
        );
        assert_eq!(
            model.get(SettingId::Shader2dWsParams).as_text(),
            "HSM_INTRO_WHEN_TO_SHOW=0;HSM_ASPECT_RATIO_MODE=6"
        );
        assert_eq!(
            model.get(SettingId::Shader2dWsArea).as_int(),
            shader_slots::AREA_PICTURE
        );
        assert_eq!(
            starting_parameters(screen.path, ShaderSlot::TwoD),
            "HSM_INTRO_WHEN_TO_SHOW=0;HSM_ASPECT_RATIO_MODE=6"
        );
        let tv = catalogue_entry("bezel/Mega_Bezel/Presets/MBZ__3__STD.slangp").expect("the TV");
        assert_eq!(
            starting_parameters(tv.path, ShaderSlot::TwoD),
            "HSM_INTRO_WHEN_TO_SHOW=0"
        );
        assert_eq!(
            starting_parameters(tv.path, ShaderSlot::ThreeDWide),
            "HSM_INTRO_WHEN_TO_SHOW=0;HSM_ASPECT_RATIO_MODE=5"
        );
        choose_preset(&mut model, ShaderSlot::TwoDWide, "crt/other.slangp");
        assert_eq!(model.get(SettingId::Shader2dWsParams).as_text(), "");
        // Owner review before 0.5.0: the tube without the TV is put forward in every slot, the
        // TV only where a 4:3 picture has room for it.
        assert!(featured_for(screen, ShaderSlot::ThreeDWide));
        assert!(featured_for(screen, ShaderSlot::TwoD));
        assert!(featured_for(tv, ShaderSlot::TwoD));
        assert!(!featured_for(tv, ShaderSlot::TwoDWide));
    }

    /// Every preset of the catalogue reads with librashader from the copy of slang-shaders the
    /// pack is made of (`ZAXNES_SLANG_SHADERS`): its passes, its textures and its parameters. No
    /// card is needed; a missing copy skips the test, as the pack build does.
    #[cfg(feature = "shaders")]
    #[test]
    fn every_preset_of_the_catalogue_reads() {
        let Some(root) = std::env::var_os("ZAXNES_SLANG_SHADERS").map(std::path::PathBuf::from)
        else {
            return;
        };
        for preset in &CATALOGUE {
            let resolved: Vec<String> = crate::shader_pack::parts(preset.path)
                .into_iter()
                .map(|part| root.join(part).to_string_lossy().into_owned())
                .collect();
            let path = std::path::PathBuf::from(resolved.join("|"));
            let parameters = zax_platform::slang_chain::preset_parameters(&path);
            assert!(parameters.is_ok(), "{}: {parameters:?}", preset.path);
        }
    }

    #[test]
    fn the_catalogue_is_in_the_pack_list() {
        let list = include_str!("../../shaders/pack-files.txt");
        for preset in &CATALOGUE {
            for part in crate::shader_pack::parts(preset.path) {
                assert!(list.lines().any(|line| line == part), "{part}");
            }
        }
    }
}
