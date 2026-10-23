//! How the settings embedded in a profile reach the settings model (SPEC-03 REQ-PROF-041 to
//! REQ-PROF-044): which keys a profile applies, which stay the user's, and the values.

use std::collections::HashMap;

use zax_settings::catalogue::{Layer, SettingId};
use zax_settings::values::{Color, SettingValue, Vector3};

use crate::enums::{LayerAlignment, ProfileLayer, TvRegionMode};
use crate::pattern::RecordedPatternSlot;
use crate::settings::{LayerSettingsSet, ProfileSettings};
use crate::values::{ArgbColor, Vec3F};

/// The value a profile gives one applied setting, in the profile's own types.
#[derive(Debug, Clone, Copy)]
pub enum AppliedValue<'a> {
    /// A direction.
    Vector(Vec3F),
    /// A colour.
    Color(ArgbColor),
    /// A flag.
    Bool(bool),
    /// A whole number or enumerated number.
    Int(i32),
    /// A real number.
    Float(f32),
    /// A depth layer.
    Layer(ProfileLayer),
    /// A layer alignment.
    Alignment(LayerAlignment),
    /// A video region.
    TvMode(TvRegionMode),
    /// The memorised pattern slots.
    Slots(&'a [RecordedPatternSlot]),
}

const CLIPPING_KEYS: [SettingId; 4] = [
    SettingId::ClipLeft,
    SettingId::ClipBottom,
    SettingId::ClipRight,
    SettingId::ClipTop,
];

/// The keys a profile applies when it is loaded, the first list of REQ-PROF-041.
#[must_use]
pub fn applied_keys() -> Vec<SettingId> {
    let mut keys = vec![SettingId::LightDirection, SettingId::LightColor];
    keys.extend(CLIPPING_KEYS);
    keys.extend([
        SettingId::SceneMapEnable,
        SettingId::LayerDefaultLayer,
        SettingId::LayerAuto,
    ]);
    for layer in Layer::ALL {
        keys.extend([
            SettingId::LayerOffset(layer),
            SettingId::LayerAligned(layer),
            SettingId::LayerCastShadow(layer),
            SettingId::LayerRecvShadow(layer),
        ]);
    }
    keys.extend([
        SettingId::VideoGraphicQuality,
        SettingId::EditorSavedPatterns,
        SettingId::EditorSavedAnimPatterns,
        SettingId::ConvSameBgPalette,
        SettingId::ConvSameSpPalette,
        SettingId::ConvSpriteTolerance,
        SettingId::ConvScanDirection,
        SettingId::ConvEmptyTile,
        SettingId::CameraLocalCam,
        SettingId::EmuTvMode,
        SettingId::SceneUseNesBgColor,
    ]);
    keys
}

/// The keys the user's own settings keep when a profile is loaded, the second list of
/// REQ-PROF-041. AMD-031 retired `scene.tv_box` with the TV box, so the list is one key shorter
/// than the original: there is no longer a user preference to preserve.
#[must_use]
pub fn preserved_keys() -> Vec<SettingId> {
    vec![
        SettingId::LightIntensity,
        SettingId::RenderMeshAlgo,
        SettingId::VrRuntime,
        SettingId::SceneSkybox,
        SettingId::EmuCore,
        SettingId::EmuGameSpeed,
        SettingId::RenderShadow,
        SettingId::AudioVolume,
        SettingId::VideoFrameRate,
        SettingId::VideoVSync,
        SettingId::VrControlMethod,
        SettingId::VrSceneRatio,
        SettingId::InputEmulatedMenuButton,
        SettingId::VideoRenderMethod,
        SettingId::VideoResScale,
        SettingId::VideoSkipFrame,
        SettingId::AudioBgMusic,
        SettingId::VideoAntiAlias,
        SettingId::VrMirroring,
        SettingId::EmuAutoTvMode,
        SettingId::UiSortOption,
        SettingId::SceneCustomBgColor,
        SettingId::CameraFov,
    ]
}

/// The value of every applied key, in the profile's own types. A key whose source is absent
/// from the profile (no light, no clipping, no layers) maps to none. The shadow flags are the
/// model's own reset values, whatever the profile stores (REQ-PROF-042), and the NES
/// background colour is forced (REQ-PROF-041 point 3).
#[must_use]
pub fn values_to_apply(settings: &ProfileSettings) -> HashMap<SettingId, Option<AppliedValue<'_>>> {
    use AppliedValue as V;
    let mut values = HashMap::new();
    values.insert(
        SettingId::LightDirection,
        settings
            .light
            .as_ref()
            .and_then(|light| light.direction)
            .map(V::Vector),
    );
    values.insert(
        SettingId::LightColor,
        settings
            .light
            .as_ref()
            .and_then(|light| light.color)
            .map(V::Color),
    );
    values.insert(
        SettingId::SceneMapEnable,
        Some(V::Bool(settings.map_enable)),
    );
    values.insert(
        SettingId::VideoGraphicQuality,
        Some(V::Int(settings.graphic_quality)),
    );
    values.insert(
        SettingId::EditorSavedPatterns,
        Some(V::Slots(&settings.saved_patterns)),
    );
    values.insert(
        SettingId::EditorSavedAnimPatterns,
        Some(V::Slots(&settings.saved_anim_patterns)),
    );
    values.insert(
        SettingId::ConvSameBgPalette,
        Some(V::Bool(settings.same_background_palette)),
    );
    values.insert(
        SettingId::ConvSameSpPalette,
        Some(V::Bool(settings.same_sprite_palette)),
    );
    values.insert(
        SettingId::ConvSpriteTolerance,
        Some(V::Int(settings.sprite_tolerance)),
    );
    values.insert(
        SettingId::ConvScanDirection,
        Some(V::Int(settings.scan_direction)),
    );
    values.insert(SettingId::ConvEmptyTile, Some(V::Bool(settings.empty_tile)));
    values.insert(
        SettingId::CameraLocalCam,
        Some(V::Bool(settings.local_camera)),
    );
    values.insert(SettingId::EmuTvMode, Some(V::TvMode(settings.tv_mode)));

    // Forced, whatever the profile says (REQ-PROF-041 point 3).
    values.insert(SettingId::SceneUseNesBgColor, Some(V::Bool(true)));

    for (index, key) in CLIPPING_KEYS.into_iter().enumerate() {
        values.insert(
            key,
            settings
                .clipping
                .and_then(|clipping| clipping.get(index))
                .map(V::Float),
        );
    }

    values.insert(
        SettingId::LayerDefaultLayer,
        settings
            .layers
            .as_ref()
            .map(|layers| V::Layer(layers.default_layer)),
    );
    values.insert(
        SettingId::LayerAuto,
        settings.layers.as_ref().map(|layers| V::Bool(layers.auto)),
    );
    for (index, layer_id) in Layer::ALL.into_iter().enumerate() {
        let layer = settings
            .layers
            .as_ref()
            .and_then(|layers| layers.get(index));
        values.insert(
            SettingId::LayerOffset(layer_id),
            layer.map(|layer| V::Float(layer.offset)),
        );
        values.insert(
            SettingId::LayerAligned(layer_id),
            layer.map(|layer| V::Alignment(layer.aligned)),
        );

        // Reset, whatever the profile stores (REQ-PROF-042).
        values.insert(
            SettingId::LayerCastShadow(layer_id),
            Some(V::Bool(LayerSettingsSet::effective_cast_shadow(index))),
        );
        values.insert(
            SettingId::LayerRecvShadow(layer_id),
            Some(V::Bool(LayerSettingsSet::effective_recv_shadow(index))),
        );
    }
    values
}

/// The settings of a profile as values of the settings model, ready to be applied through its
/// normal write path. Only what the profile holds is offered; the shadow flags are left out, as
/// the model resets them itself (REQ-PERS-005 point 4, REQ-PROF-042).
#[must_use]
pub fn to_setting_values(settings: &ProfileSettings) -> Vec<(SettingId, SettingValue)> {
    let mut values = Vec::new();

    if let Some(direction) = settings.light.as_ref().and_then(|light| light.direction) {
        values.push((
            SettingId::LightDirection,
            SettingValue::Vector3(Vector3::new(direction.x, direction.y, direction.z)),
        ));
    }
    if let Some(color) = settings.light.as_ref().and_then(|light| light.color) {
        values.push((
            SettingId::LightColor,
            SettingValue::Color(to_setting_color(color)),
        ));
    }
    if let Some(clipping) = settings.clipping {
        values.extend([
            (SettingId::ClipLeft, SettingValue::Float(clipping.x)),
            (SettingId::ClipBottom, SettingValue::Float(clipping.y)),
            (SettingId::ClipRight, SettingValue::Float(clipping.z)),
            (SettingId::ClipTop, SettingValue::Float(clipping.w)),
        ]);
    }

    values.extend([
        (
            SettingId::SceneMapEnable,
            SettingValue::Bool(settings.map_enable),
        ),
        (
            SettingId::VideoGraphicQuality,
            SettingValue::Enum(settings.graphic_quality),
        ),
        (
            SettingId::ConvSameBgPalette,
            SettingValue::Bool(settings.same_background_palette),
        ),
        (
            SettingId::ConvSameSpPalette,
            SettingValue::Bool(settings.same_sprite_palette),
        ),
        (
            SettingId::ConvSpriteTolerance,
            SettingValue::Int(settings.sprite_tolerance),
        ),
        (
            SettingId::ConvScanDirection,
            SettingValue::Enum(settings.scan_direction),
        ),
        (
            SettingId::ConvEmptyTile,
            SettingValue::Bool(settings.empty_tile),
        ),
        (
            SettingId::CameraLocalCam,
            SettingValue::Bool(settings.local_camera),
        ),
        (
            SettingId::EmuTvMode,
            SettingValue::Enum(settings.tv_mode.raw()),
        ),
    ]);

    if let Some(layers) = &settings.layers {
        values.push((
            SettingId::LayerDefaultLayer,
            SettingValue::Enum(layers.default_layer.raw()),
        ));
        values.push((SettingId::LayerAuto, SettingValue::Bool(layers.auto)));
        for (layer_id, layer) in Layer::ALL.into_iter().zip(&layers.layers) {
            values.push((
                SettingId::LayerOffset(layer_id),
                SettingValue::Float(layer.offset),
            ));
            values.push((
                SettingId::LayerAligned(layer_id),
                SettingValue::Enum(layer.aligned.raw()),
            ));
        }
    }
    values
}

fn to_setting_color(color: ArgbColor) -> Color {
    Color::new(
        f32::from(color.r()) / 255.0,
        f32::from(color.g()) / 255.0,
        f32::from(color.b()) / 255.0,
        1.0,
    )
}
