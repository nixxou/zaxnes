//! The performance / fidelity presets of the options (lot REWRITE-UI, owner's table
//! `docs/design/PRESETS.csv`): the five steps of the window's slider and of the console's preset
//! row. Each writes the whole set of settings it manages; the chosen one is remembered in
//! `ui.options_preset`, the mode of the options in `ui.options_advanced`.

use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

/// A value a preset writes (`SettingValue` holds a `String` variant, so it cannot sit in a
/// constant table itself).
#[derive(Clone, Copy)]
pub enum V {
    B(bool),
    I(i32),
    E(i32),
}

impl V {
    #[must_use]
    pub const fn value(self) -> SettingValue {
        match self {
            Self::B(on) => SettingValue::Bool(on),
            Self::I(number) => SettingValue::Int(number),
            Self::E(number) => SettingValue::Enum(number),
        }
    }
}

/// One preset of the slider. Every preset writes the whole set of settings the slider manages,
/// so that a mode puts back what another one changed; each value is either proposed (shown
/// under the slider, to be changed) or set and hidden (owner, 2026-11-18).
pub struct Preset {
    /// Its key in the string tables: `options.preset.<key>.name`, `.summary`, `.acts`.
    pub key: &'static str,
    /// The values the preset writes, in the order the proposed ones are shown, each with
    /// whether it is proposed.
    pub values: &'static [(SettingId, V, bool)],
}

/// The five presets, from the fastest to the most faithful (owner's table,
/// `docs/design/PRESETS.csv`, 2026-11-18).
pub const PRESETS: [Preset; 5] = [
    Preset {
        key: "fastest",
        values: &[
            (SettingId::EmuFastCore, V::B(true), true),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(0), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(true), true),
            (SettingId::EmuPipeline, V::E(1), true),
            (SettingId::EmuLuaOptimized, V::B(true), true),
            (SettingId::EmuLuaExperimental, V::B(true), true),
            (SettingId::VideoDynamicResolution, V::B(true), true),
            (SettingId::VideoResScale, V::E(0), true),
            (SettingId::VideoHalfRate3d, V::B(false), true),
            (SettingId::VideoCompositeSignal, V::B(false), false),
            (SettingId::RenderShadow, V::B(false), true),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(0), true),
            (SettingId::RenderStyleReflections, V::I(0), false),
            (SettingId::RenderStyleSsrQuality, V::I(0), false),
            (SettingId::RenderStyleAo, V::I(0), false),
            (SettingId::RenderStyleAoQuality, V::I(0), false),
            (SettingId::RenderStylePixelShadow, V::I(0), false),
        ],
    },
    Preset {
        key: "fast",
        values: &[
            (SettingId::EmuFastCore, V::B(true), true),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(true), true),
            (SettingId::EmuPipeline, V::E(0), true),
            (SettingId::EmuLuaOptimized, V::B(false), true),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(true), true),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::VideoCompositeSignal, V::B(false), false),
            (SettingId::RenderShadow, V::B(true), true),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(0), true),
            (SettingId::RenderStyleReflections, V::I(0), false),
            (SettingId::RenderStyleSsrQuality, V::I(0), false),
            (SettingId::RenderStyleAo, V::I(0), false),
            (SettingId::RenderStyleAoQuality, V::I(0), false),
            (SettingId::RenderStylePixelShadow, V::I(0), false),
        ],
    },
    Preset {
        key: "balanced",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(false), false),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), false),
            (SettingId::EmuLatePresent, V::B(false), false),
            (SettingId::EmuCatchUpLast3d, V::B(true), true),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::VideoCompositeSignal, V::B(false), false),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(1), false),
            (SettingId::RenderStyleAo, V::I(0), true),
            (SettingId::RenderStyleAoQuality, V::I(1), false),
            (SettingId::RenderStylePixelShadow, V::I(0), true),
        ],
    },
    Preset {
        key: "faithful",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(false), true),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(false), true),
            (SettingId::EmuLatePresent, V::B(false), true),
            (SettingId::EmuCatchUpLast3d, V::B(false), true),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::VideoCompositeSignal, V::B(true), true),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), false),
            (SettingId::EmuOamaddrWriteCorruption, V::B(false), false),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(2), true),
            (SettingId::RenderStyleAo, V::I(1), true),
            (SettingId::RenderStyleAoQuality, V::I(1), true),
            (SettingId::RenderStylePixelShadow, V::I(1), true),
        ],
    },
    Preset {
        key: "most_faithful",
        values: &[
            (SettingId::EmuFastCore, V::B(false), false),
            (SettingId::EmuFidelityPlus, V::B(true), true),
            (SettingId::EmuRunAhead, V::I(1), true),
            (SettingId::EmuRealtimeInput, V::B(true), true),
            (SettingId::EmuExactRate, V::B(true), true),
            (SettingId::EmuLatePresent, V::B(false), true),
            (SettingId::EmuCatchUpLast3d, V::B(false), false),
            (SettingId::EmuPause3dIn2d, V::B(false), false),
            (SettingId::EmuPipeline, V::E(0), false),
            (SettingId::EmuLuaOptimized, V::B(false), false),
            (SettingId::EmuLuaExperimental, V::B(false), false),
            (SettingId::VideoDynamicResolution, V::B(false), false),
            (SettingId::VideoResScale, V::E(0), false),
            (SettingId::VideoHalfRate3d, V::B(false), false),
            (SettingId::VideoCompositeSignal, V::B(true), true),
            (SettingId::RenderShadow, V::B(true), false),
            (SettingId::EmuOamDecay, V::B(false), true),
            (SettingId::EmuOamaddrWriteCorruption, V::B(true), true),
            (SettingId::RenderVisualStyle, V::I(4), true),
            (SettingId::RenderStyleReflections, V::I(0), true),
            (SettingId::RenderStyleSsrQuality, V::I(2), true),
            (SettingId::RenderStyleAo, V::I(1), true),
            (SettingId::RenderStyleAoQuality, V::I(2), true),
            (SettingId::RenderStylePixelShadow, V::I(1), true),
        ],
    },
];

/// Whether the settings still hold every value `preset` writes.
#[must_use]
pub fn matches(model: &SettingsModel, preset: &Preset) -> bool {
    preset
        .values
        .iter()
        .all(|&(id, value, _)| model.get(id) == value.value())
}

/// Takes `ids` back to their defaults: the value `preset` gives the ones it manages, the
/// setting's own default to the others (owner, 2026-11-18: Default keeps the preset).
pub fn reset_with(model: &mut SettingsModel, preset: &Preset, ids: &[SettingId]) {
    for &id in ids {
        match preset.values.iter().find(|&&(managed, ..)| managed == id) {
            Some(&(_, value, _)) => {
                model.set(id, value.value());
            }
            None => {
                model.reset(id);
            }
        }
    }
}

/// The preset chosen last (`ui.options_preset`).
#[must_use]
pub fn chosen(model: &SettingsModel) -> usize {
    usize::try_from(model.get(SettingId::UiOptionsPreset).as_int())
        .unwrap_or(2)
        .min(PRESETS.len() - 1)
}

/// Chooses preset `index`: writes every value it manages and remembers it.
pub fn choose(model: &mut SettingsModel, index: usize) {
    let index = index.min(PRESETS.len() - 1);
    for &(id, value, _) in PRESETS[index].values {
        model.set(id, value.value());
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // Five presets.
    model.set(SettingId::UiOptionsPreset, SettingValue::Enum(index as i32));
}

/// Whether the options are in their advanced mode (`ui.options_advanced`).
#[must_use]
pub fn advanced(model: &SettingsModel) -> bool {
    model.get(SettingId::UiOptionsAdvanced).as_bool()
}

/// The name of preset `index`, in the language in force.
#[must_use]
pub fn name(index: usize) -> String {
    crate::ui::strings::get(&format!(
        "options.preset.{}.name",
        PRESETS[index.min(PRESETS.len() - 1)].key
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choosing_a_preset_writes_it_and_remembers_it() {
        let mut model = SettingsModel::new();
        for (index, preset) in PRESETS.iter().enumerate() {
            choose(&mut model, index);
            assert_eq!(chosen(&model), index);
            assert!(matches(&model, preset));
        }
    }

    #[test]
    fn every_preset_manages_the_same_settings() {
        let ids = |preset: &Preset| preset.values.iter().map(|&(id, ..)| id).collect::<Vec<_>>();
        for preset in &PRESETS {
            assert_eq!(ids(preset), ids(&PRESETS[0]), "{}", preset.key);
        }
    }
}
