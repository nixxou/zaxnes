//! What a physical control is bound to: a player action or a command of the menu (SPEC-08
//! REQ-INP-012, REQ-INP-045).

use super::input_action::{self, InputAction};

/// The prefix that marks a command in a persisted key.
const COMMAND_PREFIX: &str = "cmd:";

/// The target of a binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum BindingTarget {
    /// Nothing: what an unreadable key parses to.
    #[default]
    Empty,
    /// A single player action.
    Action(InputAction),
    /// A command of the menu, by its stable name.
    Command(String),
}

impl BindingTarget {
    /// The target for `action`, empty when `action` is the empty set.
    #[must_use]
    pub fn of(action: InputAction) -> Self {
        if action.is_empty() {
            Self::Empty
        } else {
            Self::Action(action)
        }
    }

    /// The target for the command `command`.
    #[must_use]
    pub fn of_command(command: &str) -> Self {
        Self::Command(command.to_owned())
    }

    /// Whether this is a player action.
    #[must_use]
    pub const fn is_action(&self) -> bool {
        matches!(self, Self::Action(_))
    }

    /// Whether this is a command.
    #[must_use]
    pub const fn is_command(&self) -> bool {
        matches!(self, Self::Command(_))
    }

    /// Whether this is nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// The action, or the empty set for a command or nothing.
    #[must_use]
    pub const fn action(&self) -> InputAction {
        match self {
            Self::Action(action) => *action,
            _ => InputAction::NONE,
        }
    }

    /// The command name, for a command.
    #[must_use]
    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Command(command) => Some(command),
            _ => None,
        }
    }

    /// The persisted key: the action's name, or `cmd:` and the command's.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Action(action) => input_action::name_of(*action).to_owned(),
            Self::Command(command) => format!("{COMMAND_PREFIX}{command}"),
        }
    }

    /// Reads a persisted key; an unknown or empty one gives [`Self::Empty`].
    #[must_use]
    pub fn parse(key: &str) -> Self {
        if key.trim().is_empty() {
            return Self::Empty;
        }
        let is_command = key
            .get(..COMMAND_PREFIX.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(COMMAND_PREFIX));
        if is_command {
            let command = &key[COMMAND_PREFIX.len()..];
            return if command.is_empty() {
                Self::Empty
            } else {
                Self::of_command(command)
            };
        }
        input_action::parse(key).map_or(Self::Empty, Self::of)
    }
}

impl std::fmt::Display for BindingTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.key())
    }
}

/// The kind of device a binding set belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingDeviceKind {
    /// Physical key positions.
    Keyboard = 0,
    /// A gamepad's controls.
    Gamepad = 1,
}
