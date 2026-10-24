//! The scripts of the loaded profile as the runtime holds them (SPEC-05 §2, §3.1).

use mlua::Function;
use zax_engine3d::frame::FrameInstanceId;
use zax_engine3d::instance::PatternInstance;
use zax_profiles::script::ProfileScript;
use zax_profiles::tags;

/// The ten reserved hook names of SPEC-05 §2, in the order the runtime stores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptHook {
    /// Called at every game start (REQ-LUA-020).
    Start = 0,
    /// Called once per emulated frame, step 2 (REQ-LUA-025).
    Update = 1,
    /// Called once per linked instance, step 4 (REQ-LUA-025).
    UpdateS = 2,
    /// Called once per emulated frame, step 5 (REQ-LUA-025).
    LateUpdate = 3,
    /// Called at every game end (REQ-LUA-021).
    End = 4,
    /// Called before a pattern instance's geometry is built (REQ-LUA-030).
    BuildPattern = 5,
    /// Called after a savestate is restored (REQ-LUA-072).
    Load = 6,
    /// Called before a savestate is captured (REQ-LUA-071).
    Save = 7,
    /// Called before each emulated frame when the script pad is on (REQ-LUA-032).
    InputProcess = 8,
    /// Never called (REQ-LUA-031).
    DetectShape = 9,
}

impl ScriptHook {
    /// How many reserved hook names there are.
    pub const COUNT: usize = 10;

    /// The reserved hook names, in the order of the enumeration.
    pub const ALL: [Self; Self::COUNT] = [
        Self::Start,
        Self::Update,
        Self::UpdateS,
        Self::LateUpdate,
        Self::End,
        Self::BuildPattern,
        Self::Load,
        Self::Save,
        Self::InputProcess,
        Self::DetectShape,
    ];

    /// The global name the hook is defined under.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Update => "Update",
            Self::UpdateS => "UpdateS",
            Self::LateUpdate => "LateUpdate",
            Self::End => "End",
            Self::BuildPattern => "BuildPattern",
            Self::Load => "Load",
            Self::Save => "Save",
            Self::InputProcess => "InputProcess",
            Self::DetectShape => "DetectShape",
        }
    }
}

/// One script of the loaded profile as the runtime holds it: its name, its tags, its live
/// `enable` and error flags, and a function per hook it defines.
#[allow(clippy::struct_excessive_bools)] // Independent flags of the spec, not a state machine.
pub struct LoadedScript {
    file_index: usize,
    name: String,
    tag_string: String,
    universal: bool,
    tags: Vec<String>,
    /// The live `enable` flag (REQ-LUA-007). A script may change another's through
    /// `Script:Get(name).Enable`; the change applies at the next hook call.
    pub(crate) enabled: bool,
    /// True once compilation, the chunk or a hook raised a Lua error: the script receives no
    /// hook at all until the profile is reloaded (REQ-LUA-062).
    pub(crate) failed: bool,
    hooks: [Option<Function>; ScriptHook::COUNT],
    /// The instances this script's `UpdateS` must visit this frame (step 3).
    pub(crate) pending: Vec<FrameInstanceId>,
    /// Whether its `UpdateS` receives a light `shape` and no `shape2D`
    /// (`runtime::light_shapes`), decided once from the source when the profile loads.
    pub(crate) light_update_s: bool,
    /// The Lua value of this script (`self`), kept once made so that a hook call finds it
    /// without a lookup.
    pub(crate) handle: Option<mlua::Value>,
}

impl LoadedScript {
    pub(crate) fn new(source: &ProfileScript, file_index: usize) -> Self {
        let tag_string = source.tag_string.clone().unwrap_or_default();
        Self {
            file_index,
            name: source.name.clone().unwrap_or_default(),
            universal: tags::is_universal(Some(&tag_string)),
            tags: tags::parse(Some(&tag_string))
                .into_iter()
                .map(str::to_owned)
                .collect(),
            tag_string,
            enabled: source.enabled,
            failed: false,
            handle: None,
            hooks: std::array::from_fn(|_| None),
            pending: Vec::new(),
            light_update_s: false,
        }
    }

    /// Position in the profile's script list, which is the chunk order (REQ-LUA-011).
    #[must_use]
    pub const fn file_index(&self) -> usize {
        self.file_index
    }

    /// Script name (REQ-PROF-038 field 1).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Raw tag string (REQ-LUA-005).
    #[must_use]
    pub fn tag_string(&self) -> &str {
        &self.tag_string
    }

    /// True when the tag string contains `*` anywhere (REQ-LUA-005).
    #[must_use]
    pub const fn universal(&self) -> bool {
        self.universal
    }

    /// The non-empty, trimmed tags.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// The live `enable` flag (REQ-LUA-007).
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// True once compilation, the chunk or a hook raised a Lua error (REQ-LUA-062).
    #[must_use]
    pub const fn failed(&self) -> bool {
        self.failed
    }

    /// True when the script is enabled and not in error (SPEC-05 §2, "script actif").
    #[must_use]
    pub const fn active(&self) -> bool {
        self.enabled && !self.failed
    }

    /// Whether the script defines `hook`.
    #[must_use]
    pub const fn defines(&self, hook: ScriptHook) -> bool {
        self.hooks[hook as usize].is_some()
    }

    pub(crate) fn hook(&self, hook: ScriptHook) -> Option<&Function> {
        self.hooks[hook as usize].as_ref()
    }

    pub(crate) fn set_hook(&mut self, hook: ScriptHook, function: Function) {
        self.hooks[hook as usize] = Some(function);
    }

    /// True when this script is linked to `instance` (REQ-LUA-006): the tag string contains
    /// `*`, or one of its tags equals one of the instance's.
    #[must_use]
    pub fn is_linked_to(&self, instance: &PatternInstance) -> bool {
        self.universal || self.tags.iter().any(|tag| instance.has_tag(tag))
    }
}

impl std::fmt::Debug for LoadedScript {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LoadedScript")
            .field("name", &self.name)
            .field("enabled", &self.enabled)
            .field("failed", &self.failed)
            .finish_non_exhaustive()
    }
}
