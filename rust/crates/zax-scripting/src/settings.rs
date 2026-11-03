//! The settings model as scripts see it: the `Setting` object of SPEC-05 REQ-LUA-089 with the
//! members, aliases, domains and defaults of SPEC-09 (REQ-PERS-007 to REQ-PERS-017, tables
//! §4.1 and §4.2).

use zax_engine3d::settings::{ConversionSettings, RenderAlgorithm, ZAlignment, ZLayer};
use zax_engine3d::vectors::Vec3;
use zax_profiles::values::Vec4F;
use zax_settings::catalogue::{Layer, Port, SettingId};
use zax_settings::meta::SettingKind;
use zax_settings::model::{SettingLayer, SettingsModel};
use zax_settings::styles;
use zax_settings::values::{Color, SettingValue, Vector3};

use crate::values::number_format;

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

/// `video.anti_alias` = no multisampling (AMD-032 §2, REQ-PERS-017's labels).
const ANTI_ALIAS_NONE: i32 = 0;

/// `video.anti_alias` = FFAA, which is MSAA 4× (AMD-032 §2). Zaxnes has no 2×, so this is what
/// both `Setting.Msaa = 2` and `= 4` ask for (RO-2 part 4).
const ANTI_ALIAS_FFAA: i32 = 1;

/// `Setting.Msaa` normalised as AMD-059 REQ-LUA-116 §34 requires: `v ≤ 1 → 0`, `2 ≤ v ≤ 3 → 2`,
/// `v ≥ 4 → 4`. This is the value the member stores and hands back, whatever the write does to
/// the real anti-aliasing.
#[must_use]
pub const fn normalise_msaa(written: i32) -> i32 {
    if written <= 1 {
        0
    } else if written <= 3 {
        2
    } else {
        4
    }
}

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
    changed: Option<Box<dyn FnMut(SettingId) + Send>>,
    view_reset: bool,
    resets_view: bool,
    save_requested: Option<Box<dyn FnMut() + Send>>,
    load_requested: Option<Box<dyn FnMut() + Send>>,
    /// The style parameters written since their last reset, by their position in
    /// `styles::all_parameters`, in the order of their first write. This is the log
    /// `Setting.VisualStyleParameters` reads (AMD-059 REQ-LUA-116); the *values* are not kept
    /// here, they are the settings' own.
    written_styles: Vec<u8>,
    /// Whether this run shows the gamepad-only interface — a handheld build (`PortMaster`, Android)
    /// or `--handheld` on a desktop. A fact of the process, not of the model, so the host sets it
    /// when the session is made ([`ScriptSettings::set_console_interface`]).
    ///
    /// RO-2 part 4 is the one thing that reads it: a Lua write of `Setting.Msaa` never touches the
    /// real anti-aliasing there, a handheld's budget being the frame itself.
    console_interface: bool,
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
            written_styles: Vec::new(),
            console_interface: false,
        }
    }

    /// Tells the view that this run shows the gamepad-only interface (RO-2 part 4). Off by
    /// default, which is the desktop.
    pub const fn set_console_interface(&mut self, console: bool) {
        self.console_interface = console;
    }

    /// Whether this run shows the gamepad-only interface.
    #[must_use]
    pub const fn console_interface(&self) -> bool {
        self.console_interface
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
    pub fn set_on_changed(&mut self, callback: impl FnMut(SettingId) + Send + 'static) {
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
    pub fn set_on_save_requested(&mut self, callback: impl FnMut() + Send + 'static) {
        self.save_requested = Some(Box::new(callback));
    }

    /// Sets what `Setting:Load()` asks of the host: re-read the settings file and apply only
    /// the user preferences it holds (REQ-PERS-015).
    pub fn set_on_load_requested(&mut self, callback: impl FnMut() + Send + 'static) {
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

    /// A script write of `Setting.Msaa` (AMD-059 REQ-LUA-116, amended by the owner in RO-2 part
    /// 4). `written` is the raw number; the normalised value `normalise_msaa` gives is stored in
    /// `video.msaa_2_0` and is what a read gives back, exactly as the amendment specifies.
    ///
    /// In 3dSen 2.0 the member sets the real anti-aliasing for the next session, and AMD-059 made
    /// it a stored value with no effect (divergence D) so that a script could not change the
    /// picture behind the user's back. The owner's rule now lets it act — as a **session** value,
    /// so it is in force until the game ends and the user's preference comes back afterwards
    /// (REQ-PERS-010), and never persisted — unless one of three things holds
    /// ([`Self::msaa_drives_anti_alias`]).
    ///
    /// Zaxnes has no 2× multisampling (`video.anti_alias`: None, FFAA = 4 samples, SMAA = 8), so
    /// the mapping is 0 → None and 2 or 4 → FFAA, which AMD-032 §2 makes MSAA 4×.
    // Q-107 §4
    pub fn write_msaa(&mut self, written: i32) {
        let normalised = normalise_msaa(written);
        self.write_int(SettingId::VideoMsaa20, normalised);
        if self.msaa_drives_anti_alias() {
            let level = if normalised == 0 {
                ANTI_ALIAS_NONE
            } else {
                ANTI_ALIAS_FFAA
            };
            self.write_int(SettingId::VideoAntiAlias, level);
        }
    }

    /// Whether a write of `Setting.Msaa` reaches `video.anti_alias` (RO-2 part 4). It does not
    /// when any of the owner's three clauses holds:
    ///
    /// 1. the program shows the console interface — a handheld build or `--handheld` — where the
    ///    anti-aliasing is off by default (`ui/handheld/defaults.rs`) because the frame is the
    ///    whole budget, and no script may spend it;
    /// 2. the user's **global** `video.anti_alias` is above 0: the user asked for anti-aliasing,
    ///    and a script may not take it away. Note that the shipped default is 1 (AMD-052
    ///    REQ-PERS-089), so this is the ordinary desktop case: the rule is one-directional, a
    ///    script can add anti-aliasing where there is none and never remove it;
    /// 3. the game in play has a per-game anti-aliasing option (RO-2 part 2), **0 included**: a
    ///    choice made for this very game outranks a choice made for every game.
    ///
    /// Clause 2 is the one Q-107 §4 asks R1 to confirm: the owner's rule words it as a test of the
    /// **value**, and justifies it as a test of the user's **will**, which the shipped default of
    /// 1 keeps apart. The letter of the rule is what is written here, being the only one of the
    /// two that can never let a script degrade the picture of a user who asked for nothing.
    // Q-107 §4
    fn msaa_drives_anti_alias(&self) -> bool {
        !self.console_interface
            && self.model.preference(SettingId::VideoAntiAlias).as_int() <= 0
            && !self.model.has_game_value(SettingId::VideoAntiAlias)
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

    // the visual style and its parameters (AMD-040 REQ-LUA-114, AMD-059 REQ-LUA-116)

    /// The value of the style parameter `id` of style `style`, or its default when it has never
    /// been written (REQ-LUA-114). 0 when the style or the identifier is unknown — which is why
    /// a script cannot tell "unknown" from "zero" by reading, as the requirement intends.
    ///
    /// The value is read as a **double**, not as an integer. For the fifteen whole-number
    /// parameters that is the same number bit for bit (`SettingValue::as_double` of an `Int` is
    /// `f64::from(i32)`); `spl.arc_center` needs it since RO-5 part 2, its rungs being 0, 0.25,
    /// 0.5, 0.75 and 1 — an integer read would give back 1 for a setting holding 0.5.
    #[must_use]
    pub fn visual_style_parameter(&self, style: i32, id: &str) -> f64 {
        styles::parameter(style, id).map_or(0.0, |key| self.model.get(key).as_double())
    }

    /// Records `value`, normalised as AMD-040 §3 requires, in the setting the parameter stands
    /// for (REQ-LUA-114). Writing from a script and writing from the options window reach the
    /// same value, the rounding living in the setting's domain.
    ///
    /// Returns false, without recording anything, when the style is unknown or when the
    /// identifier does not exist for that style.
    /// RO-2 part 3 (owner decision): `styles::snap_write` is now the setting's own grid for every
    /// parameter, the curvature angles included, so a script and the menu round the same way and
    /// this is the only normalisation there is. RO-1 point 7 had needed two, and
    /// `record_style_parameter` with its normalisation handed in went with it.
    pub fn set_visual_style_parameter(&mut self, style: i32, id: &str, value: f64) -> bool {
        let (Some(key), Some(position)) = (
            styles::parameter(style, id),
            styles::parameter_position(style, id),
        ) else {
            return false;
        };

        let snapped = styles::snap_write(key, value);
        // A parameter whose setting is a real keeps the snapped value as it is; the other fifteen
        // go through the integer path they always went through, `to_i32` and all — RO-5 part 2
        // changes `spl.arc_center` and must change nothing else to the bit.
        if zax_settings::catalogue::descriptor(key).kind() == SettingKind::Float {
            self.write_real(key, number_format::to_f32(snapped));
        } else {
            self.write_int(key, number_format::to_i32(snapped));
        }

        // AMD-059 REQ-LUA-116: a call that answers true adds the parameter to the log, or leaves
        // it where it is when it is already there — the order is that of the **first** write.
        let position = u8::try_from(position).unwrap_or(u8::MAX);
        if !self.written_styles.contains(&position) {
            self.written_styles.push(position);
        }

        true
    }

    /// Every parameter of `style` back to its default (REQ-LUA-114); nothing happens when the
    /// style is unknown.
    ///
    /// The defaults are written like any other script write, so they land in the session layer
    /// and the user's own preference comes back at the end of the game (REQ-PERS-010): a script
    /// may undo a curvature for its game without erasing what the user chose.
    ///
    /// AMD-059 REQ-LUA-116: the reset also takes the style's parameters out of the log.
    pub fn reset_visual_style_parameters(&mut self, style: i32) {
        let canonical = styles::canonical_style(style);
        for (_, key) in styles::parameters_of(style) {
            // The descriptor's own default value, whatever its shape: for the fifteen whole
            // numbers that is the `Int` the integer path used to write, and for the real one
            // (`spl.arc_center` since RO-5 part 2) it is the `Float` an integer would have lost.
            let default = zax_settings::catalogue::descriptor(*key)
                .default_value()
                .clone();
            self.write(*key, default);
        }

        self.written_styles.retain(|position| {
            styles::parameter_of_position(usize::from(*position))
                .is_none_or(|(owner, _, _)| owner != canonical)
        });
    }

    /// Every parameter of **every** style back to its default, and the log emptied: what
    /// `Setting.VisualStyleParameters = nil` does (AMD-059 REQ-LUA-116).
    pub fn reset_all_visual_style_parameters(&mut self) {
        for style in styles::STYLES_WITH_PARAMETERS {
            self.reset_visual_style_parameters(style);
        }

        // A style whose parameters are all at their default must not keep entries either.
        self.written_styles.clear();
    }

    /// The style parameters written since their last reset, with the rung each one holds now:
    /// what `Setting.VisualStyleParameters` reads (AMD-059 REQ-LUA-116). The log keeps the order
    /// of the first write; the values are read from the settings, so the list is the state at
    /// the moment it is read.
    pub fn written_style_parameters(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.written_styles.iter().filter_map(|position| {
            let (_, _, key) = styles::parameter_of_position(usize::from(*position))?;
            // As a double, for the reason given on `visual_style_parameter`: the rung of a value
            // of 0.5 is not the rung of 1.
            let rung = styles::rung_of(key, self.model.get(key).as_double());
            Some((*position, u8::try_from(rung).unwrap_or(u8::MAX)))
        })
    }

    /// Puts the parameters of `entries` back, exactly: every other parameter goes to its default
    /// first, then each entry is written again (AMD-059 REQ-LUA-116,
    /// `Setting.VisualStyleParameters = <list>`). `entries` are the pairs
    /// [`ScriptSettings::written_style_parameters`] gives.
    pub fn restore_visual_style_parameters(&mut self, entries: &[(u8, u8)]) {
        self.reset_all_visual_style_parameters();
        for (position, rung) in entries {
            let Some((style, id, key)) = styles::parameter_of_position(usize::from(*position))
            else {
                continue;
            };

            // The rung's own value: it is already a rung, so the normalisation leaves it where it
            // is and the round trip REQ-LUA-116 promises holds. RO-1 point 7 had to write this
            // through a rounding of its own, the 30° one of a Lua write coarsening a 15° rung to
            // 30°; RO-2 part 3 made the two roundings one, so the ordinary write will do.
            let value = styles::value_of_rung(key, usize::from(*rung));
            self.set_visual_style_parameter(style, id, value);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> ScriptSettings {
        ScriptSettings::new(SettingsModel::new(), ScriptLight::Model)
    }

    /// RO-2 part 3 (owner decision): a **script** write of a curvature angle snaps on the same
    /// grid as the menu, to the nearest rung, ties away from zero. So 45 still reads back 60 and
    /// every multiple of 30 is untouched — a profile written for 3dSen behaves as it did — but 15
    /// now reads back 15 where RO-1's 30° pre-rounding gave 0, and 75 gives 90 where it gave 60.
    #[test]
    fn ro_2_a_script_write_of_a_curvature_angle_snaps_on_the_menu_grid() {
        let mut settings = settings();
        for (written, expected) in [
            (45.0, 60.0),
            (75.0, 90.0),
            (-75.0, -90.0),
            (15.0, 15.0),
            (-15.0, -15.0),
            (12.5, 15.0),
            (27.5, 30.0),
            (40.0, 30.0),
        ] {
            assert!(settings.set_visual_style_parameter(4, "spl.bend_zx", written));
            assert_eq!(
                expected,
                settings.visual_style_parameter(4, "spl.bend_zx"),
                "{written}"
            );
        }
        // Every rung of the amendment's own grid is written and read back unchanged.
        for coarse in (-180..=180).step_by(30) {
            assert!(settings.set_visual_style_parameter(4, "spl.bend_zx", f64::from(coarse)));
            assert_eq!(
                f64::from(coarse),
                settings.visual_style_parameter(4, "spl.bend_zx"),
                "{coarse}"
            );
        }
    }

    /// RO-5 part 2 (owner decision): `spl.arc_center` takes five rungs — 0, 0.25, 0.5, 0.75, 1.
    /// What a 3dSen profile does is unchanged, it being able to write only 0 or 1, and a read of
    /// either gives back the same number it always gave; a value in between lands on the nearest
    /// quarter, ties away from zero.
    #[test]
    fn ro_5_a_script_writes_and_reads_the_five_rungs_of_the_arc_centre() {
        let mut settings = settings();

        // The compatibility that matters: 0 and 1, written and read back, exactly as before.
        for end in [0.0, 1.0] {
            assert!(settings.set_visual_style_parameter(4, "spl.arc_center", end));
            assert_eq!(end, settings.visual_style_parameter(4, "spl.arc_center"));
        }

        for (written, expected) in [
            (0.0, 0.0),
            (0.25, 0.25),
            (0.5, 0.5),
            (0.75, 0.75),
            (1.0, 1.0),
            (0.1, 0.0),
            (0.4, 0.5),
            (0.6, 0.5),
            (0.9, 1.0),
            // A tie goes away from zero.
            (0.125, 0.25),
            (0.875, 1.0),
            // Out of range, as before.
            (7.0, 1.0),
            (-7.0, 0.0),
        ] {
            assert!(settings.set_visual_style_parameter(4, "spl.arc_center", written));
            assert_eq!(
                expected,
                settings.visual_style_parameter(4, "spl.arc_center"),
                "{written}"
            );
        }

        // A read of what the **player** chose in the menu, which a script never wrote: 0.5 reads
        // back as 0.5 and not as the 1 an integer read would have given. A model of its own,
        // because a script's write above went to the session layer, which masks the user's.
        let mut menu = ScriptSettings::new(SettingsModel::new(), ScriptLight::Model);
        for chosen in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            menu.model_mut()
                .set(SettingId::SceneArcCenter, SettingValue::Float(chosen));
            assert_eq!(
                f64::from(chosen),
                menu.visual_style_parameter(4, "spl.arc_center"),
                "{chosen}"
            );
        }
    }

    /// RO-5 part 2: a reset takes the arc centre back to 0, its own default, through the write of a
    /// real — an integer write would have worked too, but only by luck, the default being 0.
    #[test]
    fn ro_5_a_reset_takes_the_arc_centre_back_to_the_screen_centre() {
        let mut settings = settings();
        assert!(settings.set_visual_style_parameter(4, "spl.arc_center", 0.75));
        assert_eq!(0.75, settings.visual_style_parameter(4, "spl.arc_center"));
        settings.reset_visual_style_parameters(4);
        assert_eq!(0.0, settings.visual_style_parameter(4, "spl.arc_center"));
    }

    /// AMD-059 REQ-LUA-116, with the five rungs of RO-5 part 2: the write log reports the rung the
    /// setting really holds, and putting the list back restores the quarter **exactly**.
    #[test]
    fn ro_5_the_write_log_carries_an_intermediate_arc_centre_there_and_back() {
        let mut settings = settings();
        assert!(settings.set_visual_style_parameter(4, "spl.arc_center", 0.75));
        let log: Vec<(u8, u8)> = settings.written_style_parameters().collect();
        // `spl.arc_center` is the fourth parameter of the table and 0.75 is its fourth rung.
        assert_eq!(vec![(3_u8, 3_u8)], log);

        settings.reset_all_visual_style_parameters();
        assert_eq!(0.0, settings.visual_style_parameter(4, "spl.arc_center"));
        settings.restore_visual_style_parameters(&log);
        assert_eq!(0.75, settings.visual_style_parameter(4, "spl.arc_center"));
    }

    /// A script **read** gives back whatever the player chose, fine rung included — which it did
    /// before RO-2 part 3 as well; now a script can write those rungs too.
    #[test]
    fn ro_1_a_script_read_gives_back_the_fine_rung_the_player_chose() {
        let mut settings = settings();
        for chosen in [15, -5, 25, -20] {
            settings
                .model_mut()
                .set(SettingId::SceneBendLr, SettingValue::Int(chosen));
            assert_eq!(
                f64::from(chosen),
                settings.visual_style_parameter(4, "spl.bend_zx"),
                "{chosen}"
            );
        }
    }
}
