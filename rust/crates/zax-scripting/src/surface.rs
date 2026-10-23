//! A description of the whole script-visible API: the globals of SPEC-05 §4.1, the types of
//! §4.2 and §4.3 with their members, and the constant tables of §4.4.
//!
//! It is produced from the very tables the runtime installs, so it cannot drift from what the
//! scripts actually see. That is what makes the static check of REQ-LUA-051 — resolving every
//! access chain of the 579 built-in scripts — executable by the clean side (DEC-R2-T7).

use std::collections::BTreeMap;

use crate::api::{CONSTANT_TABLES, FUNCTION_GLOBALS, GLOBAL_OBJECTS};
use crate::binding::dispatch::api;
use crate::binding::payload::HostType;
use crate::binding::table::MemberKind;

/// One member of a script-visible type (SPEC-05 §4.2, §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptApiMember {
    /// Member name, exactly as the specification spells it.
    pub name: String,
    /// Whether a script may assign it (`LE` in §4).
    pub writable: bool,
    /// Whether it is a method or a table function rather than a property.
    pub callable: bool,
    /// The name of the script-visible type the member yields, or the empty string when it
    /// yields a number, a boolean, a string or nil.
    pub result_type: String,
}

/// One script-visible type and its members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptApiType {
    /// Type name, as §4.2 and §4.3 name it.
    pub name: String,
    /// The members, by name.
    pub members: BTreeMap<String, ScriptApiMember>,
    /// The type `obj[i]` yields, or the empty string when the type is not indexable
    /// (REQ-LUA-045).
    pub element_type: String,
    /// The type `obj(...)` yields, or the empty string when the type is not callable (the
    /// constructors of §4.3).
    pub call_result: String,
    /// True when the type answers `obj[i]` (REQ-LUA-045), whatever the element type: the `[i]`
    /// rows of §4 include types whose element is a plain number, such as `DonnéesPerso` and
    /// the bound vectors.
    pub indexed: bool,
    /// True when `obj[i] = v` is allowed.
    pub index_writable: bool,
}

impl ScriptApiType {
    /// True when the description has an `[i]` row: the type answers `obj[i]` or names an element
    /// type.
    #[must_use]
    pub fn indexable_by_row(&self) -> bool {
        self.indexed || !self.element_type.is_empty()
    }

    /// True when the type is indexable by a number.
    #[must_use]
    pub fn indexable(&self) -> bool {
        self.indexed || !self.element_type.is_empty() || self.name.starts_with("Liste")
    }
}

/// A description of the whole script-visible API.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptApiSurface {
    /// Global name → the name of its type, for the host-object globals of §4.1.
    pub globals: BTreeMap<String, String>,
    /// Type name → its description.
    pub types: BTreeMap<String, ScriptApiType>,
    /// The constant tables of §4.4: table name → entry name → value.
    pub constants: BTreeMap<String, BTreeMap<String, f64>>,
    /// The globals of §4.1 that are plain functions.
    pub function_globals: Vec<String>,
}

impl ScriptApiSurface {
    /// Describes the API by reading back the tables the runtime installs.
    #[must_use]
    pub fn describe() -> Self {
        let api = api();
        let name_of = |id: Option<HostType>| {
            id.and_then(|id| api.get(id))
                .map_or_else(String::new, |definition| definition.name.to_owned())
        };

        let types = api
            .declared()
            .map(|definition| {
                let members = definition
                    .members
                    .iter()
                    .map(|member| {
                        let (writable, callable) = match &member.kind {
                            MemberKind::Property { set, .. } => (set.is_some(), false),
                            MemberKind::Method(_) => (false, true),
                        };
                        (
                            member.name.to_owned(),
                            ScriptApiMember {
                                name: member.name.to_owned(),
                                writable,
                                callable,
                                result_type: name_of(member.result),
                            },
                        )
                    })
                    .collect();

                (
                    definition.name.to_owned(),
                    ScriptApiType {
                        name: definition.name.to_owned(),
                        members,
                        element_type: name_of(definition.element),
                        call_result: name_of(definition.call_result),
                        indexed: definition.indexer.is_some(),
                        index_writable: definition.index_setter.is_some(),
                    },
                )
            })
            .collect();

        let globals = GLOBAL_OBJECTS
            .iter()
            .map(|(name, kind)| ((*name).to_owned(), name_of(Some(*kind))))
            .collect();

        let constants = CONSTANT_TABLES
            .iter()
            .map(|(name, entries)| {
                (
                    (*name).to_owned(),
                    entries
                        .iter()
                        .map(|(key, value)| ((*key).to_owned(), f64::from(*value)))
                        .collect(),
                )
            })
            .collect();

        Self {
            globals,
            types,
            constants,
            function_globals: FUNCTION_GLOBALS
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
        }
    }

    /// How many members the description holds, all types together.
    #[must_use]
    pub fn member_count(&self) -> usize {
        self.types
            .values()
            .map(|definition| definition.members.len())
            .sum()
    }
}
