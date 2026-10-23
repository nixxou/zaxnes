//! The settings of one emulator instance: a value for each setting of the catalogue, layered
//! defaults → user preferences → profile values → session writes, validated on every write and
//! reported when the effective value really changes (REQ-PERS-003, REQ-PERS-005, REQ-PERS-009).

use std::collections::BTreeMap;
use std::vec::Drain;

use crate::catalogue::{self, Layer, Port, SettingId};
use crate::meta::{SettingDescriptor, SettingEffect, SettingScope, SettingTheme};
use crate::num;
use crate::values::SettingValue;

/// Which layer of the model a write goes to (REQ-PERS-005, REQ-PERS-010).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingLayer {
    /// The user's own choice, from the options window, the menus or a libretro core option. For a
    /// U setting this is the preference that gets persisted; for a P setting it changes the live
    /// scene value, and for an S setting the session value.
    User,
    /// The value a profile brought in. Only P settings have one (REQ-PERS-005).
    Profile,
    /// A script's write. On a U setting it is a session value that masks the preference until the
    /// game ends and is never persisted (DEC-09-Q3, REQ-PERS-010).
    Session,
    /// A value an explicitly given command-line option asks for, for this run only. It masks the
    /// persisted preference without replacing it (Q-066).
    CommandLine,
}

/// One effective change of a setting (REQ-PERS-009).
#[derive(Debug, Clone, PartialEq)]
pub struct SettingChange {
    /// The setting that changed.
    pub id: SettingId,
    /// Its new effective value.
    pub value: SettingValue,
    /// When the change takes effect (REQ-PERS-084).
    pub effect: SettingEffect,
}

/// The storage tiers of the model, which a write reaches through its scope and layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
    User,
    Profile,
    Session,
    Command,
}

/// The values of every setting, with the derived ones computed on each read (REQ-PERS-013).
///
/// Two models in one process are independent: each owns its values (REQ-PERS-082).
#[derive(Debug, Clone)]
pub struct SettingsModel {
    user: BTreeMap<SettingId, SettingValue>,
    profile: BTreeMap<SettingId, SettingValue>,
    session: BTreeMap<SettingId, SettingValue>,
    command: BTreeMap<SettingId, SettingValue>,
    core_count: i32,
    display_refresh_rate: f32,
    tv_data: Option<serde_json::Value>,
    changes: Vec<SettingChange>,
}

impl Default for SettingsModel {
    /// A model in which every setting holds its default (REQ-PERS-002).
    fn default() -> Self {
        Self {
            user: BTreeMap::new(),
            profile: BTreeMap::new(),
            session: BTreeMap::new(),
            command: BTreeMap::new(),
            core_count: 1,
            display_refresh_rate: 60.0,
            tv_data: None,
            changes: Vec::new(),
        }
    }
}

impl SettingsModel {
    /// Creates a model in which every setting holds its default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of available cores, at least 1; feeds the validation of `emu.core`
    /// (REQ-PERS-003 rule 2).
    #[must_use]
    pub fn core_count(&self) -> i32 {
        self.core_count
    }

    /// Sets the number of available cores; a count below 1 is read as 1.
    pub fn set_core_count(&mut self, count: i32) {
        self.core_count = count.max(1);
    }

    /// The refresh rate of the display, which `video.target_framerate` uses when vsync is on
    /// (REQ-PERS-013 point 1).
    #[must_use]
    pub fn display_refresh_rate(&self) -> f32 {
        self.display_refresh_rate
    }

    /// Sets the refresh rate of the display, and announces the derived frame rates it feeds.
    pub fn set_display_refresh_rate(&mut self, rate: f32) {
        if self.display_refresh_rate.to_bits() == rate.to_bits() {
            return;
        }
        self.display_refresh_rate = rate;
        self.raise_derived(SettingId::VideoTargetFramerate);
        self.raise_derived(SettingId::VideoTargetTimePerFrame);
    }

    /// The region timing block `emu.tv_data` hands back (REQ-PERS-013 point 7). The core owns its
    /// shape, so the model only carries it.
    #[must_use]
    pub fn tv_data(&self) -> Option<&serde_json::Value> {
        self.tv_data.as_ref()
    }

    /// Sets the region timing block.
    pub fn set_tv_data(&mut self, tv_data: Option<serde_json::Value>) {
        self.tv_data = tv_data;
    }

    /// The effective value of `id`.
    #[must_use]
    pub fn get(&self, id: SettingId) -> SettingValue {
        self.read(catalogue::descriptor(id))
    }

    /// The value stored for `id`, before the read-time rules of REQ-PERS-011 apply. For
    /// `vr.scene_ratio` this is the value a script wrote, which the model keeps although reading
    /// gives 1.
    #[must_use]
    pub fn get_stored(&self, id: SettingId) -> SettingValue {
        self.read_stored(catalogue::descriptor(id))
    }

    /// Whether a script's session value is masking the preference for `id`.
    #[must_use]
    pub fn has_session_value(&self, id: SettingId) -> bool {
        self.session.contains_key(&id)
    }

    /// Whether a command-line override is masking the stored value for `id` (Q-066).
    #[must_use]
    pub fn has_command_line_value(&self, id: SettingId) -> bool {
        self.command.contains_key(&id)
    }

    /// The user preferences to persist: every U setting whose stored value differs from its default
    /// (REQ-PERS-020), in catalogue order.
    pub fn user_preferences(&self) -> impl Iterator<Item = (SettingId, &SettingValue)> + '_ {
        catalogue::descriptors()
            .filter(|descriptor| descriptor.is_persisted())
            .filter_map(|descriptor| {
                self.user
                    .get(&descriptor.id())
                    .filter(|stored| *stored != descriptor.default_value())
                    .map(|stored| (descriptor.id(), stored))
            })
    }

    /// Writes `value` to `id` in the user layer and reports whether the effective value changed.
    pub fn set(&mut self, id: SettingId, value: impl Into<SettingValue>) -> bool {
        self.set_layer(id, value, SettingLayer::User)
    }

    /// Writes `value` to `id` in `layer`, validated first (REQ-PERS-003), and reports whether the
    /// effective value changed (REQ-PERS-009).
    pub fn set_layer(
        &mut self,
        id: SettingId,
        value: impl Into<SettingValue>,
        layer: SettingLayer,
    ) -> bool {
        self.write(catalogue::descriptor(id), value.into(), layer)
    }

    /// Takes `id` back to its default, in the layer that owns it, as the "Default values" button of
    /// the options window does (AMD-010 REQ-UI-202). Reports whether the effective value changed.
    pub fn reset(&mut self, id: SettingId) -> bool {
        let descriptor = catalogue::descriptor(id);
        if descriptor.scope() == SettingScope::Derived {
            return false;
        }
        let before = self.read(descriptor);
        self.user.remove(&id);
        self.session.remove(&id);
        self.command.remove(&id);
        if descriptor.scope() == SettingScope::Profile {
            self.profile.remove(&id);
        }
        self.notify_if_changed(descriptor, &before)
    }

    /// Takes every setting of `theme` back to its default.
    pub fn reset_theme(&mut self, theme: SettingTheme) {
        for descriptor in catalogue::of_theme(theme) {
            self.reset(descriptor.id());
        }
    }

    /// Applies the settings a profile embeds, for a game different from the previous one
    /// (REQ-PERS-005). Relaunching the same game must not call this (REQ-PERS-006).
    ///
    /// Beyond the values of the profile, three rules of the specification apply: `scene.use_nes_bg_color`
    /// is forced true, the layer shadow flags go back to "layer 2 casts, layers 2 to 5 receive"
    /// (REQ-PERS-005 points 3 and 4), and both input ports go back to Gamepad (REQ-PERS-012, DEC-09-Q6).
    pub fn apply_profile(&mut self, values: impl IntoIterator<Item = (SettingId, SettingValue)>) {
        // Every P setting starts again from its default, so that a field absent from the profile
        // takes the default of REQ-PROF-040 and not the value the previous game left.
        for descriptor in catalogue::of_scope(SettingScope::Profile) {
            let before = self.read(descriptor);
            self.profile.remove(&descriptor.id());
            self.notify_if_changed(descriptor, &before);
        }
        for (id, value) in values {
            let descriptor = catalogue::descriptor(id);
            if descriptor.scope() == SettingScope::Profile {
                self.write(descriptor, value, SettingLayer::Profile);
            }
        }
        self.set_layer(SettingId::SceneUseNesBgColor, true, SettingLayer::Profile);
        for layer in Layer::ALL {
            self.set_layer(
                SettingId::LayerCastShadow(layer),
                catalogue::default_cast_shadow(layer),
                SettingLayer::Profile,
            );
            self.set_layer(
                SettingId::LayerRecvShadow(layer),
                catalogue::default_recv_shadow(layer),
                SettingLayer::Profile,
            );
        }
        self.reset_input_ports();
    }

    /// Both input ports go back to Gamepad, at every program start and at every load of a different
    /// game, before the start-up hook (REQ-PERS-012, DEC-09-Q6).
    pub fn reset_input_ports(&mut self) {
        for port in Port::ALL {
            self.reset(SettingId::InputPortType(port));
        }
    }

    /// The automatic resets at the end of a game (REQ-PERS-012): the session layer is dropped, and
    /// `emu.game_speed` goes back to 0.
    pub fn end_of_game(&mut self) {
        self.clear_session_layer();
        self.reset(SettingId::EmuGameSpeed);
    }

    /// Drops every value a script wrote, restoring the user's preferences and the session defaults
    /// (REQ-PERS-010).
    pub fn clear_session_layer(&mut self) {
        if self.session.is_empty() {
            return;
        }
        let before: Vec<(&'static SettingDescriptor, SettingValue)> = self
            .session
            .keys()
            .map(|id| {
                let descriptor = catalogue::descriptor(*id);
                (descriptor, self.read(descriptor))
            })
            .collect();
        self.session.clear();
        for (descriptor, previous) in &before {
            self.notify_if_changed(descriptor, previous);
        }
    }

    /// Takes the changes made since the last call, in the order they happened (REQ-PERS-009).
    pub fn drain_changes(&mut self) -> Drain<'_, SettingChange> {
        self.changes.drain(..)
    }

    fn read(&self, descriptor: &SettingDescriptor) -> SettingValue {
        // REQ-PERS-011: on desktop the VR runtime always reads 0 and the scene ratio always reads 1,
        // whatever is stored.
        match descriptor.id() {
            SettingId::VrRuntime => SettingValue::Int(0),
            SettingId::VrSceneRatio => SettingValue::Float(1.0),
            _ => self.read_stored(descriptor),
        }
    }

    fn read_stored(&self, descriptor: &SettingDescriptor) -> SettingValue {
        if descriptor.scope() == SettingScope::Derived {
            return self.derive(descriptor);
        }
        let id = descriptor.id();
        if let Some(session) = self.session.get(&id) {
            return session.clone();
        }
        // An explicitly given command-line option wins over what the file holds, and only over that:
        // a script's write for the current game still masks it (Q-066).
        if let Some(command) = self.command.get(&id) {
            return command.clone();
        }
        let stored = if descriptor.scope() == SettingScope::Profile {
            self.profile.get(&id)
        } else {
            self.user.get(&id)
        };
        stored
            .cloned()
            .unwrap_or_else(|| descriptor.default_value().clone())
    }

    fn write(
        &mut self,
        descriptor: &SettingDescriptor,
        value: SettingValue,
        layer: SettingLayer,
    ) -> bool {
        let id = descriptor.id();
        if descriptor.scope() == SettingScope::Derived {
            // Writing SceneScale writes SceneRatio (REQ-PERS-013 point 3); the other derived members
            // refuse the write.
            return id == SettingId::SceneSceneScale
                && self.set_layer(SettingId::VrSceneRatio, value, layer);
        }
        // The VR runtime ignores every write without error, so a profile testing it takes the desktop
        // branch (REQ-PERS-011). The movie mode is frozen at NONE (REQ-PERS-016).
        if id == SettingId::VrRuntime || (id == SettingId::MovieMode && value.as_int() != 0) {
            return false;
        }
        let clean = descriptor.domain().coerce(
            &value.coerce_to(descriptor.kind()),
            descriptor.default_value(),
            self.core_count,
        );
        let before = self.read(descriptor);
        let stored_before = self.read_stored(descriptor);
        // The session layer exists for one reason: a script's write on a user preference must not
        // become the preference (REQ-PERS-010, DEC-09-Q3). A script's write on a profile setting is a
        // change of the live scene value, which REQ-PERS-006 keeps, so it goes to the profile layer.
        let tier = match descriptor.scope() {
            SettingScope::Session => Tier::Session,
            SettingScope::Profile if layer == SettingLayer::CommandLine => Tier::Command,
            SettingScope::Profile => Tier::Profile,
            SettingScope::User | SettingScope::Derived => match layer {
                SettingLayer::Session => Tier::Session,
                SettingLayer::CommandLine => Tier::Command,
                SettingLayer::User | SettingLayer::Profile => Tier::User,
            },
        };
        let unchanged_stored = stored_before == clean;
        self.tier_mut(tier).insert(id, clean);
        // The user's own choice ends the command-line override of that one setting: from now on the
        // value in force is a preference again, and it is persisted (Q-066).
        if layer == SettingLayer::User && tier != Tier::Command {
            self.command.remove(&id);
        }
        // The VR scene ratio reads 1 either way, so the change that matters, and that asks the scene
        // for the view reset of REQ-SCN-002, is the one of the stored value.
        if id == SettingId::VrSceneRatio {
            if unchanged_stored {
                return false;
            }
            let after = self.read(descriptor);
            self.raise(descriptor, after);
            return true;
        }
        self.notify_if_changed(descriptor, &before)
    }

    fn tier_mut(&mut self, tier: Tier) -> &mut BTreeMap<SettingId, SettingValue> {
        match tier {
            Tier::User => &mut self.user,
            Tier::Profile => &mut self.profile,
            Tier::Session => &mut self.session,
            Tier::Command => &mut self.command,
        }
    }

    fn notify_if_changed(&mut self, descriptor: &SettingDescriptor, before: &SettingValue) -> bool {
        let after = self.read(descriptor);
        if *before == after {
            return false;
        }
        self.raise(descriptor, after);
        self.raise_dependents(descriptor.id());
        true
    }

    fn raise(&mut self, descriptor: &SettingDescriptor, value: SettingValue) {
        self.changes.push(SettingChange {
            id: descriptor.id(),
            value,
            effect: descriptor.effect(),
        });
    }

    fn raise_derived(&mut self, id: SettingId) {
        let descriptor = catalogue::descriptor(id);
        let value = self.derive(descriptor);
        self.raise(descriptor, value);
    }

    /// Tells the subscribers about the derived values a change makes stale (REQ-PERS-013), so that a
    /// subsystem can watch `video.target_framerate` instead of every setting that feeds it.
    fn raise_dependents(&mut self, id: SettingId) {
        match id {
            SettingId::VideoVSync | SettingId::VideoFrameRate => {
                self.raise_derived(SettingId::VideoTargetFramerate);
                self.raise_derived(SettingId::VideoTargetTimePerFrame);
            }
            SettingId::VideoResScale => self.raise_derived(SettingId::VideoResScaleFactor),
            SettingId::SceneSkybox => self.raise_derived(SettingId::SceneSkyboxEnable),
            SettingId::SceneRootScale | SettingId::VrSceneRatio => {
                self.raise_derived(SettingId::SceneSceneScale);
            }
            SettingId::EmuTvMode => self.raise_derived(SettingId::EmuTvData),
            SettingId::LayerAligned(layer) => self.raise_derived(SettingId::LayerCoef(layer)),
            SettingId::InputPortType(_) => self.raise_derived(SettingId::InputZapperEnable),
            _ => {}
        }
    }

    /// The derived values of REQ-PERS-013.
    fn derive(&self, descriptor: &SettingDescriptor) -> SettingValue {
        match descriptor.id() {
            SettingId::VideoTargetFramerate => SettingValue::Float(self.target_framerate()),
            SettingId::VideoTargetTimePerFrame => {
                let rate = self.target_framerate();
                SettingValue::Float(if rate <= 0.0 { 0.0 } else { 1.0 / rate })
            }
            // RootScale × SceneRatio; the ratio reads 1 on desktop, so this is RootScale.
            SettingId::SceneSceneScale => {
                SettingValue::Float(self.get(SettingId::SceneRootScale).as_float())
            }
            SettingId::SceneSkyboxEnable => {
                SettingValue::Bool(self.get(SettingId::SceneSkybox).as_int() > 0)
            }
            SettingId::InputZapperEnable => SettingValue::Bool(
                Port::ALL
                    .iter()
                    .any(|port| self.get(SettingId::InputPortType(*port)).as_int() == 1),
            ),
            SettingId::VideoResScaleFactor => {
                SettingValue::Float(match self.get(SettingId::VideoResScale).as_int() {
                    1 => 2.0 / 3.0,
                    2 => 0.5,
                    3 => 1.0 / 3.0,
                    4 => 0.25,
                    _ => 1.0,
                })
            }
            SettingId::EmuTvData => SettingValue::Opaque(self.tv_data.clone()),
            SettingId::LayerCoef(layer) => SettingValue::Float(catalogue::alignment_coefficient(
                self.get(SettingId::LayerAligned(layer)).as_int(),
            )),
            _ => descriptor.default_value().clone(),
        }
    }

    fn target_framerate(&self) -> f32 {
        match self.get(SettingId::VideoVSync).as_int() {
            1 => self.display_refresh_rate,
            2 => self.display_refresh_rate / 2.0,
            _ => num::i32_to_f32(self.get(SettingId::VideoFrameRate).as_int()),
        }
    }
}
