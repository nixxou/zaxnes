//! The libretro core option table, generated from the settings catalogue (REQ-PERS-081 to
//! REQ-PERS-085, SPEC-09 §4.6). Only user preferences with a discretisation are exposed: the P and
//! S settings belong to the profile or to the script (REQ-PERS-085), and the host-owned ones are not
//! listed by §4.6.

use std::sync::LazyLock;

use crate::catalogue::{self, SettingId};
use crate::meta::{SettingKind, SettingScope, SettingTheme};
use crate::model::{SettingLayer, SettingsModel};
use crate::num;
use crate::values::{Color, SettingValue};

/// One libretro core option generated from the model (SPEC-09 §4.6, REQ-PERS-083).
#[derive(Debug, Clone, PartialEq)]
pub struct LibretroCoreOption {
    /// Core option key: `zaxnes_` and the stable key with the separators replaced.
    pub key: String,
    /// The setting behind the option.
    pub setting: SettingId,
    /// Human-readable name: the UI label when there is one.
    pub label: String,
    /// Category, taken from the theme of the setting.
    pub category: &'static str,
    /// The finite ordered list of value labels (REQ-PERS-081).
    pub values: Vec<String>,
    /// The label of the default value, which is always in the list.
    pub default_value: String,
}

/// Prefix of every libretro core option key (REQ-PERS-083, AMD-029 §3).
const KEY_PREFIX: &str = "zaxnes_";

static OPTIONS: LazyLock<Vec<LibretroCoreOption>> = LazyLock::new(|| {
    catalogue::descriptors()
        .filter(|descriptor| {
            descriptor.is_libretro_exposed() && descriptor.scope() == SettingScope::User
        })
        .map(|descriptor| {
            let values = descriptor.libretro_values().unwrap_or_default();
            LibretroCoreOption {
                key: descriptor.libretro_key(),
                setting: descriptor.id(),
                label: descriptor
                    .ui_label()
                    .map_or_else(|| descriptor.key().into_owned(), str::to_owned),
                category: category_of(descriptor.theme()),
                values: values.iter().map(|value| value.label.clone()).collect(),
                default_value: default_label(descriptor.kind(), descriptor.default_value(), values),
            }
        })
        .collect()
});

/// The core options, in the order of the catalogue.
#[must_use]
pub fn all() -> &'static [LibretroCoreOption] {
    &OPTIONS
}

/// The option whose core key is `core_option_key`, or none.
#[must_use]
pub fn find(core_option_key: &str) -> Option<&'static LibretroCoreOption> {
    all().iter().find(|option| option.key == core_option_key)
}

/// Applies the value `label` of the core option `core_option_key` to `model`, on the same path as a
/// change from the UI (REQ-PERS-085). Returns false when the key or the label is unknown.
pub fn try_apply(model: &mut SettingsModel, core_option_key: &str, label: &str) -> bool {
    let Some(option) = find(core_option_key) else {
        return false;
    };
    let descriptor = catalogue::descriptor(option.setting);
    let Some(values) = descriptor.libretro_values() else {
        return false;
    };
    let Some(candidate) = values
        .iter()
        .find(|candidate| candidate.label.eq_ignore_ascii_case(label))
    else {
        return false;
    };
    let value = if descriptor.kind() == SettingKind::Color {
        // The colour options are the presets of DEC-09-Q5, held as packed RGB.
        SettingValue::Color(Color::from_rgb(num::truncate_to_i32(candidate.value)))
    } else {
        SettingValue::Float(num::to_f32(candidate.value)).coerce_to(descriptor.kind())
    };
    model.set_layer(option.setting, value, SettingLayer::User);
    true
}

/// The core option key of a stable key: `zaxnes_` and the key with `.`, `[` and `]` replaced by `_`
/// (REQ-PERS-083).
#[must_use]
pub fn core_option_key(setting_key: &str) -> String {
    let replaced: String = setting_key
        .chars()
        .map(|c| if matches!(c, '.' | '[' | ']') { '_' } else { c })
        .collect();
    format!("{KEY_PREFIX}{replaced}")
}

/// The category name of a theme; the English form of the themes of §4.6.
fn category_of(theme: SettingTheme) -> &'static str {
    match theme {
        SettingTheme::Video => "Video",
        SettingTheme::Render => "Render",
        SettingTheme::Camera => "Camera",
        SettingTheme::Light => "Light",
        SettingTheme::Scene => "Scene",
        SettingTheme::Clipping => "Clipping",
        SettingTheme::Layers => "Layers",
        SettingTheme::Emulation => "Emulation",
        SettingTheme::Conversion => "Conversion",
        SettingTheme::Audio => "Audio",
        SettingTheme::Input => "Input",
        SettingTheme::Interface => "Interface",
        _ => "Advanced",
    }
}

/// The label of the default value. REQ-PERS-081 requires the list to contain the default; for a
/// colour that means the preset equal to the default colour.
fn default_label(
    kind: SettingKind,
    default: &SettingValue,
    values: &[crate::domain::SettingOption],
) -> String {
    // An exposed option always has values (see `SettingDescriptor::is_libretro_exposed`), so the
    // first label exists; the empty fallback is never used.
    let first = values
        .first()
        .map(|value| value.label.clone())
        .unwrap_or_default();
    if kind == SettingKind::Color {
        let wanted = default.as_color();
        return values
            .iter()
            .find(|option| Color::from_rgb(num::truncate_to_i32(option.value)) == wanted)
            .map_or(first, |option| option.label.clone());
    }
    let number = default.as_double();
    values
        .iter()
        .find(|option| (option.value - number).abs() < 1e-6)
        .map_or(first, |option| option.label.clone())
}
