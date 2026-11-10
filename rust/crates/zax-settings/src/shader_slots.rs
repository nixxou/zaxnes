//! The slang shaders of the wgpu renderer (Rust-only, owner request): one slot per display
//! mode, 2D, 2D widescreen, 3D and 3D widescreen, each with its preset, the lines of the picture
//! the preset reads and the parameters the player changed.
//!
//! A slot other than 2D can follow another one (2D wide -> 2D, 3D -> 2D, 3D wide -> 3D): it then
//! takes that slot's preset and parameters, which is how one preset set once covers every mode.
//! The lines stay the mode's own: 240 suit the flat picture, the 3D scene keeps the window's.
//!
//! The parameters are kept as the differences from the preset, `NAME=VALUE` pairs separated by
//! `;`: a preset that changes its defaults keeps the player's own values, and a parameter the
//! preset no longer has is ignored by the renderer.

use crate::catalogue::SettingId;
use crate::model::SettingsModel;

/// `shader.<slot>.mode`: the slot takes the choice of its fallback.
pub const FOLLOW: i32 = 0;
/// `shader.<slot>.mode`: no shader in this mode.
pub const OFF: i32 = 1;
/// `shader.<slot>.mode`: the preset of `shader.<slot>.preset`.
pub const PRESET: i32 = 2;

/// The lines of the console's picture, the default of the 2D slots.
pub const CONSOLE_LINES: i32 = 240;
/// The most lines `shader.<slot>.lines` takes (0 is the window's own).
pub const MAXIMUM_LINES: i32 = 4320;

/// One of the four slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderSlot {
    /// The flat picture.
    TwoD,
    /// The flat picture with the widescreen margins.
    TwoDWide,
    /// The voxel scene.
    ThreeD,
    /// The voxel scene with the widescreen margins.
    ThreeDWide,
}

/// The four settings of a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotIds {
    pub mode: SettingId,
    pub preset: SettingId,
    pub lines: SettingId,
    pub params: SettingId,
}

impl ShaderSlot {
    /// The slots, in settings order.
    pub const ALL: [Self; 4] = [Self::TwoD, Self::TwoDWide, Self::ThreeD, Self::ThreeDWide];

    /// The slot of a display mode.
    #[must_use]
    pub const fn of(three_d: bool, wide: bool) -> Self {
        match (three_d, wide) {
            (false, false) => Self::TwoD,
            (false, true) => Self::TwoDWide,
            (true, false) => Self::ThreeD,
            (true, true) => Self::ThreeDWide,
        }
    }

    /// The slot this one follows in [`FOLLOW`] mode; the 2D slot follows none.
    #[must_use]
    pub const fn fallback(self) -> Option<Self> {
        match self {
            Self::TwoD => None,
            Self::TwoDWide | Self::ThreeD => Some(Self::TwoD),
            Self::ThreeDWide => Some(Self::ThreeD),
        }
    }

    /// The name of the slot.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TwoD => "2D",
            Self::TwoDWide => "2D widescreen",
            Self::ThreeD => "3D",
            Self::ThreeDWide => "3D widescreen",
        }
    }

    /// The label of the [`FOLLOW`] choice of the slots following this one.
    #[must_use]
    pub const fn follow_label(self) -> &'static str {
        match self {
            Self::TwoD => "Same as 2D",
            Self::TwoDWide => "Same as 2D widescreen",
            Self::ThreeD => "Same as 3D",
            Self::ThreeDWide => "Same as 3D widescreen",
        }
    }

    /// The default of `shader.<slot>.lines`: the console's 240 in 2D, where the presets meet the
    /// pixels they were written for; the window's own in 3D, where 240 lines would crush the
    /// voxels.
    #[must_use]
    pub const fn default_lines(self) -> i32 {
        match self {
            Self::TwoD | Self::TwoDWide => CONSOLE_LINES,
            Self::ThreeD | Self::ThreeDWide => 0,
        }
    }

    /// The settings of the slot.
    #[must_use]
    pub const fn ids(self) -> SlotIds {
        match self {
            Self::TwoD => SlotIds {
                mode: SettingId::Shader2dMode,
                preset: SettingId::Shader2dPreset,
                lines: SettingId::Shader2dLines,
                params: SettingId::Shader2dParams,
            },
            Self::TwoDWide => SlotIds {
                mode: SettingId::Shader2dWsMode,
                preset: SettingId::Shader2dWsPreset,
                lines: SettingId::Shader2dWsLines,
                params: SettingId::Shader2dWsParams,
            },
            Self::ThreeD => SlotIds {
                mode: SettingId::Shader3dMode,
                preset: SettingId::Shader3dPreset,
                lines: SettingId::Shader3dLines,
                params: SettingId::Shader3dParams,
            },
            Self::ThreeDWide => SlotIds {
                mode: SettingId::Shader3dWsMode,
                preset: SettingId::Shader3dWsPreset,
                lines: SettingId::Shader3dWsLines,
                params: SettingId::Shader3dWsParams,
            },
        }
    }
}

/// The shader a display mode uses, once the fallbacks are followed.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderChoice {
    /// The slot that holds the preset (the mode's own, or the one it follows).
    pub slot: ShaderSlot,
    /// The `.slangp` file.
    pub preset: String,
    /// The lines of the picture the preset reads, 0 for the window's own: those of the
    /// display mode's slot, even when it follows another one.
    pub lines: u32,
    /// The parameters the player changed, in the order they were written.
    pub parameters: Vec<(String, f32)>,
}

/// The shader of the display mode (`three_d`, `wide`), or none: a slot that is off, a preset
/// mode without a file, or a 2D slot left to follow.
#[must_use]
pub fn resolve(model: &SettingsModel, three_d: bool, wide: bool) -> Option<ShaderChoice> {
    let own = ShaderSlot::of(three_d, wide);
    let lines = u32::try_from(model.get(own.ids().lines).as_int()).unwrap_or(0);
    let mut slot = own;
    loop {
        let ids = slot.ids();
        match model.get(ids.mode).as_int() {
            PRESET => {
                let preset = model.get(ids.preset).as_text().trim().to_owned();
                if preset.is_empty() {
                    return None;
                }
                let parameters = parse_parameters(model.get(ids.params).as_text());
                return Some(ShaderChoice {
                    slot,
                    preset,
                    lines,
                    parameters,
                });
            }
            FOLLOW => slot = slot.fallback()?,
            _ => return None,
        }
    }
}

/// Whether `name` can be a parameter of a slang shader (`#pragma parameter NAME ...`).
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The parameters of `shader.<slot>.params`: `NAME=VALUE` pairs separated by `;`. A pair that
/// does not read (no `=`, a name with other characters, a value that is not a finite number) is
/// skipped; a name given twice keeps its last value, at the place of its first.
#[must_use]
pub fn parse_parameters(text: &str) -> Vec<(String, f32)> {
    let mut parameters: Vec<(String, f32)> = Vec::new();
    for pair in text.split(';') {
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let Ok(value) = value.trim().parse::<f32>() else {
            continue;
        };
        if !valid_name(name) || !value.is_finite() {
            continue;
        }
        match parameters.iter_mut().find(|(known, _)| known == name) {
            Some(entry) => entry.1 = value,
            None => parameters.push((name.to_owned(), value)),
        }
    }
    parameters
}

/// The text of `parameters` for `shader.<slot>.params`.
#[must_use]
pub fn format_parameters(parameters: &[(String, f32)]) -> String {
    let mut text = String::new();
    for (name, value) in parameters {
        if !valid_name(name) || !value.is_finite() {
            continue;
        }
        if !text.is_empty() {
            text.push(';');
        }
        text.push_str(name);
        text.push('=');
        text.push_str(&value.to_string());
    }
    text
}

/// `text` with the parameter `name` set to `value`, or back to the preset's own with `None`.
#[must_use]
pub fn with_parameter(text: &str, name: &str, value: Option<f32>) -> String {
    let mut parameters = parse_parameters(text);
    match value {
        Some(value) => match parameters.iter_mut().find(|(known, _)| known == name) {
            Some(entry) => entry.1 = value,
            None => parameters.push((name.to_owned(), value)),
        },
        None => parameters.retain(|(known, _)| known != name),
    }
    format_parameters(&parameters)
}
