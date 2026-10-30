//! What the model knows about one setting: its key, shape, default, domain, scope, moment of
//! effect, Lua name, profile field and libretro discretisation (SPEC-09 §4.1, REQ-PERS-001,
//! REQ-ARCH-021).

use std::borrow::Cow;

use crate::catalogue::{self, SettingId};
use crate::core_options;
use crate::domain::{Domain, SettingOption};
use crate::values::SettingValue;

/// Scope of a setting: who owns its current value and what happens to it when a profile loads
/// (SPEC-09 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingScope {
    /// U: a user preference, global to the application, persisted and kept by a profile load
    /// (REQ-PERS-005 point 2).
    User,
    /// P: a scene parameter, replaced by the value of the profile at a load of another game
    /// (REQ-PERS-005 point 1).
    Profile,
    /// S: never persisted, back to its default at every program start.
    Session,
    /// D: derived, read only, computed from other settings or from the platform.
    Derived,
}

/// When a change takes effect, for a frontend that has to warn about a restart (REQ-PERS-084).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingEffect {
    /// `imm.`: at the next rendered frame at the latest.
    Immediate,
    /// `jeu`: at the next game load.
    NextGameLoad,
    /// `ROM`: at the next ROM load.
    NextRomLoad,
    /// `redém.`: at the next program start.
    Restart,
    /// No effect at all: the value is stored and handed back only.
    NoEffect,
}

/// The shape of a setting's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKind {
    Bool,
    Int,
    Float,
    /// A whole number out of a finite named set (column "Domaine").
    Enum,
    Color,
    Vector3,
    Text,
    Rect,
    /// A value stored and handed back without interpretation (REQ-PERS-016, REQ-PERS-013 point 7).
    Opaque,
}

/// The theme a setting belongs to: the grouping of the options window (AMD-010 REQ-UI-202) and
/// the libretro option category (REQ-PERS-083).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingTheme {
    Video,
    Render,
    Camera,
    Light,
    Scene,
    Clipping,
    Layers,
    Emulation,
    Conversion,
    Audio,
    Input,
    Interface,
    /// RetroAchievements (Rust-only).
    Achievements,
    /// VR, out of scope (REQ-PERS-011).
    Vr,
    /// Profile editor, out of scope (REQ-PERS-016).
    Editor,
    /// Movie recording, out of scope (REQ-PERS-016).
    Movie,
    Internal,
}

/// Everything known about one setting.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingDescriptor {
    id: SettingId,
    kind: SettingKind,
    default: SettingValue,
    scope: SettingScope,
    effect: SettingEffect,
    theme: SettingTheme,
    domain: Domain,
    lua_path: Option<String>,
    lua_aliases: Vec<&'static str>,
    lua_writable: bool,
    profile_field: Option<&'static str>,
    ui_label: Option<&'static str>,
    text_options: Option<Vec<&'static str>>,
    libretro_values: Option<Vec<SettingOption>>,
}

impl SettingDescriptor {
    /// Creates a descriptor; the default is reshaped to `kind`.
    pub(crate) fn new(
        id: SettingId,
        kind: SettingKind,
        default: SettingValue,
        scope: SettingScope,
        effect: SettingEffect,
        theme: SettingTheme,
    ) -> Self {
        debug_assert!(
            catalogue::is_valid_key(&id.key()),
            "{} is not a stable key",
            id.key()
        );
        Self {
            id,
            kind,
            default: default.coerce_to(kind),
            scope,
            effect,
            theme,
            domain: Domain::unbounded(),
            lua_path: None,
            lua_aliases: Vec::new(),
            lua_writable: true,
            profile_field: None,
            ui_label: None,
            text_options: None,
            libretro_values: None,
        }
    }

    pub(crate) fn with_domain(mut self, domain: Domain) -> Self {
        self.domain = domain;
        self
    }

    pub(crate) fn with_lua(mut self, path: impl Into<String>) -> Self {
        self.lua_path = Some(path.into());
        self
    }

    pub(crate) fn with_alias(mut self, alias: &'static str) -> Self {
        self.lua_aliases.push(alias);
        self
    }

    pub(crate) fn read_only_in_lua(mut self) -> Self {
        self.lua_writable = false;
        self
    }

    pub(crate) fn with_field(mut self, number: &'static str) -> Self {
        self.profile_field = Some(number);
        self
    }

    pub(crate) fn with_label(mut self, label: &'static str) -> Self {
        self.ui_label = Some(label);
        self
    }

    pub(crate) fn with_text_options(mut self, options: &[&'static str]) -> Self {
        self.text_options = Some(options.to_vec());
        self
    }

    pub(crate) fn with_libretro(mut self, values: Vec<SettingOption>) -> Self {
        self.libretro_values = Some(values);
        self
    }

    /// The identifier of the setting.
    #[must_use]
    pub fn id(&self) -> SettingId {
        self.id
    }

    /// The stable key, for instance `camera.fov` (REQ-PERS-080).
    #[must_use]
    pub fn key(&self) -> Cow<'static, str> {
        self.id.key()
    }

    /// The shape of the value.
    #[must_use]
    pub fn kind(&self) -> SettingKind {
        self.kind
    }

    /// The value of the column "Défaut" (REQ-PERS-002).
    #[must_use]
    pub fn default_value(&self) -> &SettingValue {
        &self.default
    }

    /// The scope of the column "Portée" (SPEC-09 §2).
    #[must_use]
    pub fn scope(&self) -> SettingScope {
        self.scope
    }

    /// The moment of the column "Effet" (REQ-PERS-084).
    #[must_use]
    pub fn effect(&self) -> SettingEffect {
        self.effect
    }

    /// The theme of the column "Thème" (REQ-PERS-083).
    #[must_use]
    pub fn theme(&self) -> SettingTheme {
        self.theme
    }

    /// The domain of the column "Domaine" (REQ-PERS-003).
    #[must_use]
    pub fn domain(&self) -> &Domain {
        &self.domain
    }

    /// The Lua access path with `Setting.` left off, or none when the setting is not exposed to
    /// scripts (REQ-PERS-008).
    #[must_use]
    pub fn lua_path(&self) -> Option<&str> {
        self.lua_path.as_deref()
    }

    /// The extra Lua names of the same setting, such as `SamePalette`.
    #[must_use]
    pub fn lua_aliases(&self) -> &[&'static str] {
        &self.lua_aliases
    }

    /// Whether Lua may assign this setting; the members marked "lecture" in §4.2 may not
    /// (REQ-PERS-007).
    #[must_use]
    pub fn is_lua_writable(&self) -> bool {
        self.lua_writable
    }

    /// The field number in the settings message embedded in profiles, such as `6.1.3`, or none.
    #[must_use]
    pub fn profile_field(&self) -> Option<&'static str> {
        self.profile_field
    }

    /// The label the desktop settings panel shows, or none.
    #[must_use]
    pub fn ui_label(&self) -> Option<&'static str> {
        self.ui_label
    }

    /// The finite ordered texts a text setting accepts, or none when any text will do
    /// (AMD-035 §1).
    #[must_use]
    pub fn text_options(&self) -> Option<&[&'static str]> {
        self.text_options.as_deref()
    }

    /// The finite ordered list of values exposed as a libretro core option, or none
    /// (REQ-PERS-081, SPEC-09 §4.6).
    #[must_use]
    pub fn libretro_values(&self) -> Option<&[SettingOption]> {
        self.libretro_values.as_deref()
    }

    /// Whether the setting is exposed as a libretro core option.
    #[must_use]
    pub fn is_libretro_exposed(&self) -> bool {
        self.libretro_values()
            .is_some_and(|values| !values.is_empty())
    }

    /// The libretro core option key of this setting (REQ-PERS-083).
    #[must_use]
    pub fn libretro_key(&self) -> String {
        core_options::core_option_key(&self.key())
    }

    /// Whether the setting is persisted in `settings.json`: the user preferences, which include
    /// `ui.fullscreen` (REQ-PERS-020).
    #[must_use]
    pub fn is_persisted(&self) -> bool {
        self.scope == SettingScope::User
    }

    /// Brings `text` back into the text options: the stored text when the list admits it, ignoring
    /// case and surrounding spaces, and the default otherwise (AMD-035 §1).
    #[must_use]
    pub fn coerce_text(&self, text: &str) -> String {
        let Some(options) = self.text_options().filter(|options| !options.is_empty()) else {
            return text.to_owned();
        };
        let trimmed = text.trim();
        match options
            .iter()
            .find(|option| option.eq_ignore_ascii_case(trimmed))
        {
            Some(option) => (*option).to_owned(),
            None => self.default.as_text().to_owned(),
        }
    }
}
