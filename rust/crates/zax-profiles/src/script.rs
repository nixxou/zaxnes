//! A Lua script of a profile (SPEC-03 REQ-PROF-038, `Script`).

use crate::layout::FieldMask;
use crate::tags;
use crate::variant::PatternVariant;

/// A Lua script of a profile. Compilation and execution belong to SPEC-05; a script that fails
/// to compile does not stop the profile from loading.
#[derive(Debug, Clone, Default)]
pub struct ProfileScript {
    /// Script name. The file holds it twice, in fields 1 and 5; field 1 is authoritative and
    /// field 5 is redundant (REQ-PROF-038), and both are written back from this value.
    pub name: Option<String>,
    /// Whether the script receives the frame hooks. A disabled script still receives `Start`
    /// and `End` (SPEC-05 REQ-LUA-007).
    pub enabled: bool,
    /// The `error` flag as stored. It is always false in the built-in profiles and SPEC-05
    /// REQ-LUA-001 requires ignoring it at load time: every script starts without error.
    pub stored_error_flag: bool,
    /// Full Lua source, or none when the field is absent.
    pub source: Option<String>,
    /// Raw tag string, or none when the field is absent.
    pub tag_string: Option<String>,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl ProfileScript {
    /// Creates a script.
    #[must_use]
    pub fn new(
        name: Option<&str>,
        enabled: bool,
        source: Option<&str>,
        tag_string: Option<&str>,
    ) -> Self {
        Self {
            name: name.map(str::to_owned),
            enabled,
            stored_error_flag: false,
            source: source.map(str::to_owned),
            tag_string: tag_string.map(str::to_owned),
            recorded_fields: None,
        }
    }

    /// Tags of this script, split and trimmed (REQ-PROF-036).
    #[must_use]
    pub fn tags(&self) -> Vec<&str> {
        tags::parse(self.tag_string.as_deref())
    }

    /// True when this script is bound to every variant (SPEC-05 REQ-LUA-005).
    #[must_use]
    pub fn binds_to_every_variant(&self) -> bool {
        tags::is_universal(self.tag_string.as_deref())
    }

    /// True when this script is bound to `variant` (REQ-LUA-006).
    #[must_use]
    pub fn binds(&self, variant: &PatternVariant) -> bool {
        tags::binds(self.tag_string.as_deref(), variant.tag_string.as_deref())
    }
}
