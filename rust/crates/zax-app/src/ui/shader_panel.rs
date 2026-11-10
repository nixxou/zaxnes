//! The model of the shaders window (Rust-only, owner request), kept apart from its `ImGui` drawing
//! so that the console mode can show the same choices with a gamepad later: the quick presets,
//! the folder of the slang shaders, copying one slot to another, the state of the preset in use
//! and the parameters of a preset, read on a thread of their own.

use std::path::{Path, PathBuf};

use zax_settings::model::SettingsModel;
use zax_settings::shader_slots::{self, ShaderSlot, OFF, PRESET};
use zax_settings::values::SettingValue;

/// A preset the window offers in one click, from the folder of the slang shaders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickPreset {
    /// The key of its name in the string tables.
    pub label: &'static str,
    /// Its path in `libretro/slang-shaders`.
    pub path: &'static str,
}

/// The quick presets: a few of the most used ones, from the light to the heavy.
pub const QUICK_PRESETS: [QuickPreset; 5] = [
    QuickPreset {
        label: "shaders.quick.crt",
        path: "crt/crt-easymode.slangp",
    },
    QuickPreset {
        label: "shaders.quick.guest",
        path: "crt/crt-guest-advanced.slangp",
    },
    QuickPreset {
        label: "shaders.quick.mega_bezel",
        path: "bezel/Mega_Bezel/Presets/MBZ__3__STD.slangp",
    },
    QuickPreset {
        label: "shaders.quick.mega_bezel_light",
        path: "bezel/Mega_Bezel/Presets/MBZ__5__POTATO.slangp",
    },
    QuickPreset {
        label: "shaders.quick.lcd",
        path: "handheld/lcd-grid-v2.slangp",
    },
];

/// Where the slang-shaders collection is looked for when `shader.library` is empty, under the
/// folder of the program and under the user folder.
const LIBRARY_NAMES: [&str; 2] = ["shaders", "slang-shaders"];

/// The folder of the slang shaders: `shader.library` when set, otherwise the first of
/// `shaders` or `slang-shaders` beside the program or in the user folder `user_root`.
#[must_use]
pub fn library_folder(model: &SettingsModel, user_root: &Path) -> Option<PathBuf> {
    let chosen = model
        .get(zax_settings::catalogue::SettingId::ShaderLibrary)
        .as_text()
        .trim()
        .to_owned();
    if !chosen.is_empty() {
        return Some(PathBuf::from(chosen));
    }
    let program = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    program
        .into_iter()
        .chain(std::iter::once(user_root.to_path_buf()))
        .flat_map(|root| LIBRARY_NAMES.map(|name| root.join(name)))
        .find(|folder| folder.is_dir())
}

/// The quick presets found in `library`, with their full paths.
#[must_use]
pub fn available_quick_presets(library: Option<&Path>) -> Vec<(QuickPreset, PathBuf)> {
    let Some(library) = library else {
        return Vec::new();
    };
    QUICK_PRESETS
        .into_iter()
        .map(|quick| (quick, library.join(quick.path)))
        .filter(|(_, path)| path.is_file())
        .collect()
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

/// `preset` relative to `library` when it is inside, for a shorter display.
#[must_use]
pub fn display_path(preset: &str, library: Option<&Path>) -> String {
    let path = Path::new(preset);
    library
        .and_then(|library| path.strip_prefix(library).ok())
        .map_or_else(
            || preset.to_owned(),
            |relative| relative.display().to_string(),
        )
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
    /// The preset concerned.
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
    fn a_path_inside_the_library_shows_relative() {
        let library = Path::new("G:/slang-shaders");
        assert_eq!(
            display_path("G:/slang-shaders/crt/crt-easymode.slangp", Some(library)),
            Path::new("crt/crt-easymode.slangp").display().to_string()
        );
        assert_eq!(display_path("D:/x.slangp", Some(library)), "D:/x.slangp");
    }
}
