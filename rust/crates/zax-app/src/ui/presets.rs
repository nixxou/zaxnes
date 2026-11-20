//! The performance / fidelity presets of the options (lot REWRITE-UI, owner's table
//! `docs/design/PRESETS.csv`): the five steps of the window's slider and of the console's preset
//! row. Each writes the whole set of settings it manages; the chosen one is remembered in
//! `ui.options_preset`, the mode of the options in `ui.options_advanced`.

use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

pub use zax_settings::presets::{Preset, PRESETS, V};

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
