//! The settings model as scripts see it: the `Setting` object of SPEC-05 REQ-LUA-089 with the
//! members, aliases, domains and defaults of SPEC-09 (REQ-PERS-007 to REQ-PERS-017, tables
//! §4.1 and §4.2).

use zax_engine3d::settings::{ConversionSettings, RenderAlgorithm, ZAlignment, ZLayer};
use zax_engine3d::vectors::Vec3;
use zax_profiles::values::Vec4F;
use zax_settings::catalogue::{Layer, Port, SettingId};
use zax_settings::model::{SettingLayer, SettingsModel};
use zax_settings::values::{Color, SettingValue, Vector3};

use crate::values::LuaColor;

/// How many layers `Layer[i]` addresses.
pub const LAYER_COUNT: usize = 6;

/// How many margins `Clipping[i]` addresses.
pub const CLIPPING_COUNT: usize = 4;

/// The settings keys of the four clip margins, in Lua index order: 0 is the left margin, 1 the
/// bottom, 2 the right, 3 the top (DEC-R2-T1).
const CLIPPING_IDS: [SettingId; CLIPPING_COUNT] = [
    SettingId::ClipLeft,
    SettingId::ClipBottom,
    SettingId::ClipRight,
    SettingId::ClipTop,
];

/// The characteristic smoothing time of SPEC-09 §4.3.
pub const DEFAULT_SMOOTH_TIME: f32 = 0.4;

/// The VR scene scale of SPEC-09 §4.3.
pub const VR_SCALE: f32 = 0.005;

/// The desktop scene scale of SPEC-09 §4.3.
pub const NON_VR_SCALE: f32 = 1.0;

/// Value of `app.engine_mode` that means ROM (§4.1.10).
pub const ROM_ENGINE_MODE: i32 = 0;

/// Value of `app.engine_mode` that means PROFILE (§4.1.10).
pub const PROFILE_ENGINE_MODE: i32 = 1;

/// Where the directional light keeps its state (SPEC-09 §4.1.3, REQ-PERS-005 point 2).
#[derive(Debug, Clone, Default)]
pub enum ScriptLight {
    /// Backed by the settings model: the three light values are settings, which is what
    /// SPEC-09 §4.1.3 requires.
    #[default]
    Model,
    /// Holds its own state: the headless default of the tests.
    Detached {
        /// Propagation direction; `(1, -1, 1)` by default.
        direction: Vec3,
        /// Colour, white by default.
        color: LuaColor,
        /// Intensity, 1 by default.
        intensity: f32,
    },
}

impl ScriptLight {
    /// A light that holds its own state, in its default configuration.
    #[must_use]
    pub const fn detached() -> Self {
        Self::Detached {
            direction: Vec3::new(1.0, -1.0, 1.0),
            color: LuaColor::WHITE,
            intensity: 1.0,
        }
    }
}

/// The settings model as scripts see it.
///
/// This holds no value of its own besides the detached light: it is a typed view over the one
/// [`SettingsModel`] of the emulator instance, so a write from Lua, a change from the options
/// window, a profile load and a libretro core option all meet in the same place and all go
/// through the same validation and the same notification (REQ-PERS-007).
///
/// Every write made through this object is a *script* write. On a user preference the model
/// puts it in the session layer, which the game's end discards (REQ-PERS-010, DEC-09-Q3); on a
/// profile setting it changes the live scene value, which relaunching the same game keeps
/// (REQ-PERS-006).
///
/// The conversion pipeline keeps its own slice of these values in [`ConversionSettings`], which
/// it reads every frame. [`ScriptSettings::sync`] mirrors the model into that slice, so the
/// engine sees a script's write at the next frame and a profile load reaches it by the same
/// single path.
pub struct ScriptSettings {
    model: SettingsModel,
    light: ScriptLight,
    pending: Vec<SettingId>,
    changed: Option<Box<dyn FnMut(SettingId)>>,
    view_reset: bool,
    resets_view: bool,
    save_requested: Option<Box<dyn FnMut()>>,
    load_requested: Option<Box<dyn FnMut()>>,
}

impl Default for ScriptSettings {
    /// A view with a model of its own and the light backed by it.
    fn default() -> Self {
        Self::new(SettingsModel::new(), ScriptLight::Model)
    }
}

impl ScriptSettings {
    /// Creates the view over `model`.
    #[must_use]
    pub fn new(model: SettingsModel, light: ScriptLight) -> Self {
        Self {
            model,
            light,
            pending: Vec::new(),
            changed: None,
            view_reset: false,
            resets_view: true,
            save_requested: None,
            load_requested: None,
        }
    }

    /// The settings of the emulator instance this view reads and writes.
    #[must_use]
    pub const fn model(&self) -> &SettingsModel {
        &self.model
    }

    /// The same, for writing. The host calls [`ScriptSettings::sync`] afterwards, so that the
    /// change reaches the pipeline and the callbacks.
    pub fn model_mut(&mut self) -> &mut SettingsModel {
        &mut self.model
    }

    /// Sets what is told, with the identifier of SPEC-09 §4.1, after a value really changed
    /// (REQ-PERS-009), so that the host can apply the effect it owns.
    pub fn set_on_changed(&mut self, callback: impl FnMut(SettingId) + 'static) {
        self.changed = Some(Box::new(callback));
    }

    /// Sets whether a change of field of view or of the kept scene ratio asks for a view reset
    /// (on by default). A host that does not wire the C# `ScriptSettings.ViewReset` callback,
    /// as the C# harness does not, turns it off so that such a write leaves the camera alone.
    pub const fn set_resets_view(&mut self, resets_view: bool) {
        self.resets_view = resets_view;
    }

    /// True once since the last call when a change of field of view or of the kept scene ratio
    /// asks for a view reset (SPEC-06 REQ-SCN-012, REQ-PERS-011, REQ-LUA-095). The runtime
    /// answers it with [`ScriptCamera::reset_view`](crate::ScriptCamera::reset_view).
    pub fn take_view_reset(&mut self) -> bool {
        std::mem::take(&mut self.view_reset)
    }

    /// Sets what `Setting:Save()` asks of the host: write the settings file now (REQ-PERS-015).
    /// Without it the call does nothing.
    pub fn set_on_save_requested(&mut self, callback: impl FnMut() + 'static) {
        self.save_requested = Some(Box::new(callback));
    }

    /// Sets what `Setting:Load()` asks of the host: re-read the settings file and apply only
    /// the user preferences it holds (REQ-PERS-015).
    pub fn set_on_load_requested(&mut self, callback: impl FnMut() + 'static) {
        self.load_requested = Some(Box::new(callback));
    }

    pub(crate) fn request_save(&mut self) {
        if let Some(callback) = &mut self.save_requested {
            callback();
        }
    }

    pub(crate) fn request_load(&mut self) {
        if let Some(callback) = &mut self.load_requested {
            callback();
        }
    }

    // typed reads and writes

    /// The boolean value of `id`.
    #[must_use]
    pub fn flag(&self, id: SettingId) -> bool {
        self.model.get(id).as_bool()
    }

    /// The integer value of `id`.
    #[must_use]
    pub fn int(&self, id: SettingId) -> i32 {
        self.model.get(id).as_int()
    }

    /// The real value of `id`.
    #[must_use]
    pub fn real(&self, id: SettingId) -> f32 {
        self.model.get(id).as_float()
    }

    /// A script write of a boolean.
    pub fn write_flag(&mut self, id: SettingId, value: bool) {
        self.write(id, SettingValue::Bool(value));
    }

    /// A script write of an integer; the model coerces it to an enum where the setting is one.
    pub fn write_int(&mut self, id: SettingId, value: i32) {
        self.write(id, SettingValue::Int(value));
    }

    /// A script write of a real.
    pub fn write_real(&mut self, id: SettingId, value: f32) {
        self.write(id, SettingValue::Float(value));
    }

    fn write(&mut self, id: SettingId, value: SettingValue) {
        self.model.set_layer(id, value, SettingLayer::Session);
        self.absorb();
    }

    /// The custom background colour (REQ-SCN-046).
    #[must_use]
    pub fn custom_bg_color(&self) -> LuaColor {
        to_lua(self.model.get(SettingId::SceneCustomBgColor).as_color())
    }

    /// Writes the custom background colour.
    pub fn set_custom_bg_color(&mut self, color: LuaColor) {
        self.write(
            SettingId::SceneCustomBgColor,
            SettingValue::Color(from_lua(color)),
        );
    }

    // light

    /// Propagation direction of the light.
    #[must_use]
    pub fn light_direction(&self) -> Vec3 {
        match &self.light {
            ScriptLight::Model => {
                let value = self.model.get(SettingId::LightDirection).as_vector3();
                Vec3::new(value.x, value.y, value.z)
            }
            ScriptLight::Detached { direction, .. } => *direction,
        }
    }

    /// Writes the direction of the light.
    pub fn set_light_direction(&mut self, direction: Vec3) {
        match &mut self.light {
            ScriptLight::Model => self.write(
                SettingId::LightDirection,
                SettingValue::Vector3(Vector3::new(direction.x, direction.y, direction.z)),
            ),
            ScriptLight::Detached { direction: own, .. } => *own = direction,
        }
    }

    /// Colour of the light.
    #[must_use]
    pub fn light_color(&self) -> LuaColor {
        match &self.light {
            ScriptLight::Model => to_lua(self.model.get(SettingId::LightColor).as_color()),
            ScriptLight::Detached { color, .. } => *color,
        }
    }

    /// Writes the colour of the light.
    pub fn set_light_color(&mut self, color: LuaColor) {
        match &mut self.light {
            ScriptLight::Model => {
                self.write(SettingId::LightColor, SettingValue::Color(from_lua(color)));
            }
            ScriptLight::Detached { color: own, .. } => *own = color,
        }
    }

    /// Intensity of the light.
    #[must_use]
    pub fn light_intensity(&self) -> f32 {
        match &self.light {
            ScriptLight::Model => self.real(SettingId::LightIntensity),
            ScriptLight::Detached { intensity, .. } => *intensity,
        }
    }

    /// Writes the intensity of the light.
    pub fn set_light_intensity(&mut self, intensity: f32) {
        match &mut self.light {
            ScriptLight::Model => self.write_real(SettingId::LightIntensity, intensity),
            ScriptLight::Detached { intensity: own, .. } => *own = intensity,
        }
    }

    // clipping and layers

    /// The clip margin of Lua index `index`: 0 left, 1 bottom, 2 right, 3 top. An index outside
    /// the range addresses the nearest margin.
    #[must_use]
    pub fn clipping(&self, index: usize) -> f32 {
        self.real(CLIPPING_IDS[index.min(CLIPPING_COUNT - 1)])
    }

    /// Writes the clip margin of Lua index `index`.
    pub fn set_clipping(&mut self, index: usize, value: f32) {
        self.write_real(CLIPPING_IDS[index.min(CLIPPING_COUNT - 1)], value);
    }

    /// The four margins as a vector, which `Clipping.target` exposes.
    #[must_use]
    pub fn clipping_vector(&self) -> Vec4F {
        Vec4F::new(
            self.clipping(0),
            self.clipping(1),
            self.clipping(2),
            self.clipping(3),
        )
    }

    /// Writes the four margins at once.
    pub fn set_clipping_vector(&mut self, value: Vec4F) {
        self.set_clipping(0, value.x);
        self.set_clipping(1, value.y);
        self.set_clipping(2, value.z);
        self.set_clipping(3, value.w);
    }

    fn layer_of(index: usize) -> Layer {
        Layer::ALL[index.min(LAYER_COUNT - 1)]
    }

    /// Depth offset of layer `index` in scene units.
    #[must_use]
    pub fn layer_offset(&self, index: usize) -> f32 {
        self.real(SettingId::LayerOffset(Self::layer_of(index)))
    }

    /// Writes the depth offset of a layer.
    pub fn set_layer_offset(&mut self, index: usize, value: f32) {
        self.write_real(SettingId::LayerOffset(Self::layer_of(index)), value);
    }

    /// Depth alignment of a layer: 0 FRONT, 1 CENTER, 2 BACK.
    #[must_use]
    pub fn layer_aligned(&self, index: usize) -> i32 {
        self.int(SettingId::LayerAligned(Self::layer_of(index)))
    }

    /// Writes the depth alignment of a layer, bounded to `[0, 2]`.
    pub fn set_layer_aligned(&mut self, index: usize, value: i32) {
        self.write_int(
            SettingId::LayerAligned(Self::layer_of(index)),
            value.clamp(0, 2),
        );
    }

    /// The derived alignment coefficient of a layer, read only (REQ-PERS-013 point 8).
    #[must_use]
    pub fn layer_coef(&self, index: usize) -> f32 {
        self.real(SettingId::LayerCoef(Self::layer_of(index)))
    }

    /// Whether a layer casts shadows.
    #[must_use]
    pub fn layer_cast_shadow(&self, index: usize) -> bool {
        self.flag(SettingId::LayerCastShadow(Self::layer_of(index)))
    }

    /// Writes whether a layer casts shadows.
    pub fn set_layer_cast_shadow(&mut self, index: usize, value: bool) {
        self.write_flag(SettingId::LayerCastShadow(Self::layer_of(index)), value);
    }

    /// Whether a layer receives shadows.
    #[must_use]
    pub fn layer_recv_shadow(&self, index: usize) -> bool {
        self.flag(SettingId::LayerRecvShadow(Self::layer_of(index)))
    }

    /// Writes whether a layer receives shadows.
    pub fn set_layer_recv_shadow(&mut self, index: usize, value: bool) {
        self.write_flag(SettingId::LayerRecvShadow(Self::layer_of(index)), value);
    }

    // derived (REQ-PERS-013)

    /// The engine mode of SPEC-09 §4.1.10: always [`PROFILE_ENGINE_MODE`], the ROM mode being
    /// the editor's. AMD-020 (Q-052) makes the three members of the row required in Lua, read
    /// only.
    #[must_use]
    pub fn engine_mode(&self) -> i32 {
        self.int(SettingId::AppEngineMode)
    }

    /// The value last written to the scene ratio, for the host (reading it gives 1).
    #[must_use]
    pub fn stored_scene_ratio(&self) -> f32 {
        self.model.get_stored(SettingId::VrSceneRatio).as_float()
    }

    /// The end-of-game resets of REQ-PERS-012, applied to the whole model.
    pub fn end_game(&mut self) {
        self.model.end_of_game();
        self.absorb();
    }

    /// The port types, which `Setting:SetInputType` records in the model so that
    /// `Setting.ZapperEnable` and the model agree (SPEC-08 REQ-INP-060, REQ-PERS-013 point 5).
    /// The input layer stays the authority on what the ports actually do.
    pub fn set_port_types(&mut self, first: i32, second: i32) {
        for (port, kind) in Port::ALL.into_iter().zip([first, second]) {
            self.model.set_layer(
                SettingId::InputPortType(port),
                SettingValue::Enum(kind),
                SettingLayer::Session,
            );
        }

        self.absorb();
    }

    // mirror into the pipeline

    /// Takes the changes the model made since the last call, tells the host about them and
    /// queues them for the next [`ScriptSettings::sync`].
    pub fn absorb(&mut self) {
        let changes: Vec<SettingId> = self.model.drain_changes().map(|change| change.id).collect();
        for id in changes {
            self.pending.push(id);
            if let Some(callback) = &mut self.changed {
                callback(id);
            }

            // A change of field of view or of the kept scene ratio resets the view
            // (REQ-SCN-002, REQ-SCN-012, REQ-PERS-011).
            if self.resets_view && matches!(id, SettingId::CameraFov | SettingId::VrSceneRatio) {
                self.view_reset = true;
            }
        }
    }

    /// Copies every setting changed since the last call into the slice the conversion pipeline
    /// reads. This is the only place the engine's working copy is written from the model, so a
    /// Lua write and a profile load reach it the same way.
    pub fn sync(&mut self, conversion: &mut ConversionSettings) {
        self.absorb();
        for id in std::mem::take(&mut self.pending) {
            self.mirror(id, conversion);
        }
    }

    /// Pushes every mirrored value into the engine slice, which a game start needs.
    pub fn mirror_all(&mut self, conversion: &mut ConversionSettings) {
        self.absorb();
        self.pending.clear();
        let mirrored = [
            SettingId::ConvEmptyTile,
            SettingId::ConvScanDirection,
            SettingId::ConvSameBgPalette,
            SettingId::ConvSameSpPalette,
            SettingId::ConvSpriteTolerance,
            SettingId::RenderMeshAlgo,
            SettingId::RenderShadow,
            SettingId::SceneUseNesBgColor,
            SettingId::SceneCustomBgColor,
            SettingId::LayerDefaultLayer,
            SettingId::LayerAuto,
            // One clip margin stands for the four: the slice takes them as a vector.
            SettingId::ClipLeft,
        ];
        for id in mirrored {
            self.mirror(id, conversion);
        }

        for layer in Layer::ALL {
            self.mirror(SettingId::LayerOffset(layer), conversion);
        }
    }

    fn mirror(&self, id: SettingId, conversion: &mut ConversionSettings) {
        match id {
            SettingId::ConvEmptyTile => conversion.empty_tile = self.flag(id),
            SettingId::ConvScanDirection => conversion.scan_direction = self.int(id),
            SettingId::ConvSameBgPalette => conversion.same_background_palette = self.flag(id),
            SettingId::ConvSameSpPalette => conversion.same_sprite_palette = self.flag(id),
            SettingId::ConvSpriteTolerance => conversion.sprite_tolerance = self.int(id),
            SettingId::RenderMeshAlgo => conversion.render_algorithm = algorithm_of(self.int(id)),
            SettingId::RenderShadow => conversion.shadow = self.flag(id),
            SettingId::SceneUseNesBgColor => conversion.use_nes_background_color = self.flag(id),
            SettingId::SceneCustomBgColor => {
                conversion.custom_background_color = self.custom_bg_color().to_rgb24();
            }
            SettingId::LayerDefaultLayer => {
                conversion.default_layer = ZLayer::from_raw(self.int(id));
            }
            SettingId::LayerAuto => conversion.auto_layer = self.flag(id),
            SettingId::ClipLeft
            | SettingId::ClipBottom
            | SettingId::ClipRight
            | SettingId::ClipTop => {
                conversion.clipping = self.clipping_vector();
            }
            SettingId::LayerOffset(layer)
            | SettingId::LayerAligned(layer)
            | SettingId::LayerCastShadow(layer)
            | SettingId::LayerRecvShadow(layer) => {
                let index = usize::from(layer.index());
                let target = conversion.layer_mut(ZLayer::from_raw(i32::from(layer.index())));
                target.offset = self.layer_offset(index);
                target.alignment = alignment_of(self.layer_aligned(index));
                target.cast_shadow = self.layer_cast_shadow(index);
                target.receive_shadow = self.layer_recv_shadow(index);
            }
            _ => {}
        }
    }
}

fn to_lua(color: Color) -> LuaColor {
    LuaColor::new(color.r, color.g, color.b, color.a)
}

fn from_lua(color: LuaColor) -> Color {
    Color::new(color.r, color.g, color.b, color.a)
}

const fn algorithm_of(raw: i32) -> RenderAlgorithm {
    match raw {
        1 => RenderAlgorithm::Marching,
        2 => RenderAlgorithm::GreedyMulticolor,
        _ => RenderAlgorithm::Greedy,
    }
}

const fn alignment_of(raw: i32) -> ZAlignment {
    match raw {
        0 => ZAlignment::Front,
        1 => ZAlignment::Center,
        2 => ZAlignment::Back,
        _ => ZAlignment::Other,
    }
}
