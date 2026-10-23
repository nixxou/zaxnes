//! The declaration of the script-visible API as data: each type is a name, its members and its
//! metamethods. The runtime installs these tables and the API description of the harness reads
//! them back, so the two cannot drift apart (REQ-LUA-051).

use std::collections::HashMap;
use std::sync::Arc;

use mlua::Value;

use super::cx::{Call, Cx};
use super::payload::{HostType, Payload};

/// What a method or a table function answers: no value, or one.
pub(crate) type Reply = Option<Value>;

type Shared<F> = Arc<F>;

/// Reads a property.
pub(crate) type GetFn = Shared<dyn Fn(&Cx, &Payload) -> mlua::Result<Value> + Send + Sync>;

/// Writes a property. The payload of a value type is the script's own copy and may be changed.
pub(crate) type SetFn = Shared<dyn Fn(&Cx, &mut Payload, &Value) -> mlua::Result<()> + Send + Sync>;

/// A method or a table function. The first argument is the receiver, whatever the script
/// passed there (REQ-LUA-081).
pub(crate) type CallFn = Shared<dyn Fn(&Call) -> mlua::Result<Reply> + Send + Sync>;

/// Reads `obj[key]` for a numeric key (REQ-LUA-045).
pub(crate) type IndexFn =
    Shared<dyn Fn(&Cx, &Payload, &Value) -> mlua::Result<Value> + Send + Sync>;

/// Writes `obj[key] = value` for a numeric key.
pub(crate) type IndexSetFn =
    Shared<dyn Fn(&Cx, &Payload, &Value, &Value) -> mlua::Result<()> + Send + Sync>;

/// A binary or unary operator; a unary one receives its operand twice, as Lua passes it.
pub(crate) type OperatorFn =
    Shared<dyn Fn(&Cx, &Value, &Value) -> mlua::Result<Value> + Send + Sync>;

/// `a == b` for two values of the type.
pub(crate) type EqualFn = Shared<dyn Fn(&Payload, &Payload) -> bool + Send + Sync>;

/// `tostring(a)`.
pub(crate) type TextFn = Shared<dyn Fn(&Payload) -> String + Send + Sync>;

/// How a member is used.
#[derive(Clone)]
pub(crate) enum MemberKind {
    /// A property, optionally writable.
    Property { get: GetFn, set: Option<SetFn> },
    /// A method or a table function.
    Method(CallFn),
}

/// One member of a type.
pub(crate) struct Member {
    /// Name, exactly as the specification spells it.
    pub name: &'static str,
    pub kind: MemberKind,
    /// The type the member yields, when it yields one.
    pub result: Option<HostType>,
}

/// The description of one Lua-visible host type: its members and its metamethods.
pub(crate) struct TypeDef {
    pub id: HostType,
    pub name: &'static str,
    /// Properties and methods, in declaration order.
    pub members: Vec<Member>,
    lookup: HashMap<&'static str, usize>,
    /// The type of `obj[i]`, for the static check of REQ-LUA-051.
    pub element: Option<HostType>,
    pub indexer: Option<IndexFn>,
    pub index_setter: Option<IndexSetFn>,
    /// `obj(...)`, the constructor of a value class (§4.3).
    pub call: Option<CallFn>,
    /// The type `call` produces.
    pub call_result: Option<HostType>,
    pub equal: Option<EqualFn>,
    pub add: Option<OperatorFn>,
    pub subtract: Option<OperatorFn>,
    pub multiply: Option<OperatorFn>,
    pub divide: Option<OperatorFn>,
    pub negate: Option<OperatorFn>,
    pub text: Option<TextFn>,
}

impl TypeDef {
    /// Creates the description of `id` named `name`.
    pub fn new(id: HostType, name: &'static str) -> Self {
        Self {
            id,
            name,
            members: Vec::new(),
            lookup: HashMap::new(),
            element: None,
            indexer: None,
            index_setter: None,
            call: None,
            call_result: None,
            equal: None,
            add: None,
            subtract: None,
            multiply: None,
            divide: None,
            negate: None,
            text: None,
        }
    }

    /// The member called `name`, with its position.
    pub fn member(&self, name: &str) -> Option<(usize, &Member)> {
        let position = *self.lookup.get(name)?;
        Some((position, &self.members[position]))
    }

    fn push(&mut self, member: Member) -> &mut Self {
        // A later declaration of a name replaces the earlier one, as the dictionary of the
        // original did.
        match self.lookup.get(member.name) {
            Some(&position) => self.members[position] = member,
            None => {
                self.lookup.insert(member.name, self.members.len());
                self.members.push(member);
            }
        }

        self
    }

    /// Declares a read-only property.
    pub fn read_only(
        &mut self,
        name: &'static str,
        get: impl Fn(&Cx, &Payload) -> mlua::Result<Value> + Send + Sync + 'static,
        result: Option<HostType>,
    ) -> &mut Self {
        self.push(Member {
            name,
            kind: MemberKind::Property {
                get: Arc::new(get),
                set: None,
            },
            result,
        })
    }

    /// Declares a writable property.
    pub fn property(
        &mut self,
        name: &'static str,
        get: impl Fn(&Cx, &Payload) -> mlua::Result<Value> + Send + Sync + 'static,
        set: impl Fn(&Cx, &mut Payload, &Value) -> mlua::Result<()> + Send + Sync + 'static,
        result: Option<HostType>,
    ) -> &mut Self {
        self.push(Member {
            name,
            kind: MemberKind::Property {
                get: Arc::new(get),
                set: Some(Arc::new(set)),
            },
            result,
        })
    }

    /// Declares a method or a table function.
    pub fn function(
        &mut self,
        name: &'static str,
        call: impl Fn(&Call) -> mlua::Result<Reply> + Send + Sync + 'static,
        result: Option<HostType>,
    ) -> &mut Self {
        self.push(Member {
            name,
            kind: MemberKind::Method(Arc::new(call)),
            result,
        })
    }

    /// Declares `name` as another name of the already declared member `existing`.
    ///
    /// # Panics
    ///
    /// When `existing` is not declared: the tables are built from literals, so this is a
    /// mistake in the declaration itself and the first test that builds the API shows it.
    #[allow(clippy::panic)] // A declaration error must stop the build of the tables.
    pub fn alias(&mut self, name: &'static str, existing: &str) -> &mut Self {
        let Some((position, _)) = self.member(existing) else {
            panic!(
                "no member '{existing}' on '{}' to alias as '{name}'",
                self.name
            );
        };

        // The alias shares the behaviour of the original: the same functions answer for both.
        let original = &self.members[position];
        let member = Member {
            name,
            kind: original.kind.clone(),
            result: original.result,
        };
        self.push(member)
    }
}

/// Every declared type, by type.
pub(crate) struct Api {
    types: Vec<Option<TypeDef>>,
}

impl Api {
    pub fn new(declared: Vec<TypeDef>) -> Self {
        let mut types: Vec<Option<TypeDef>> = (0..HostType::COUNT).map(|_| None).collect();
        for definition in declared {
            let slot = definition.id.index();
            types[slot] = Some(definition);
        }

        Self { types }
    }

    /// The description of `id`, or `None` for a type nobody declared.
    pub fn get(&self, id: HostType) -> Option<&TypeDef> {
        self.types[id.index()].as_ref()
    }

    /// Every declared type.
    pub fn declared(&self) -> impl Iterator<Item = &TypeDef> {
        self.types.iter().flatten()
    }
}
