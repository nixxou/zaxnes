//! The key configuration window as data: the sections and rows, and the glue between a row and
//! the rebinding session (AMD-010 REQ-UI-203, SPEC-08 REQ-INP-040 to REQ-INP-049).

use zax_platform::input::GamepadHub;

use super::commands::UiCommand;
use super::keys;
use super::strings;
use crate::emulation::commands::Command;
use crate::input::gamepad_assignment::GamepadAssignment;
use crate::input::input_action::{self, InputAction};
use crate::input::rebinding_session::{PadAccess, RebindingSession};
use crate::input::{BindingDeviceKind, BindingSet, BindingTarget, InputBindings};

/// One line of the window: the thing to bind and whose bindings it edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyConfigRow {
    /// The label shown.
    pub label: String,
    /// What the binding runs.
    pub target: BindingTarget,
    /// The player whose bindings the row edits.
    pub player: usize,
}

/// A titled group of rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyConfigSection {
    /// The title, in the language in force.
    pub title: String,
    /// The rows.
    pub rows: Vec<KeyConfigRow>,
}

/// The commands a binding cannot run, because they need an argument a key cannot carry.
pub const COMMANDS_NEEDING_ARGUMENTS: [Command; 11] = [
    Command::OpenRecentRom,
    Command::LaunchGame,
    Command::SaveSlot,
    Command::LoadSlot,
    Command::DeleteSlot,
    Command::SetSpeed,
    Command::SetRegion,
    Command::SetPortType,
    Command::SetCameraMode,
    Command::SetSkybox,
    Command::SetVolume,
];

/// The eight NES buttons, in the order of the window.
pub const NES_BUTTONS: [InputAction; 8] = [
    InputAction::LEFT,
    InputAction::RIGHT,
    InputAction::UP,
    InputAction::DOWN,
    InputAction::A,
    InputAction::B,
    InputAction::SELECT,
    InputAction::START,
];

/// The six camera movements.
pub const CAMERA_ACTIONS: [InputAction; 6] = [
    InputAction::ROT_LEFT,
    InputAction::ROT_RIGHT,
    InputAction::ROT_UP,
    InputAction::ROT_DOWN,
    InputAction::ZOOM_IN,
    InputAction::ZOOM_OUT,
];

/// The five hotkeys.
pub const HOTKEYS: [InputAction; 5] = [
    InputAction::MENU,
    InputAction::ROLL_BACK,
    InputAction::FAST_FORWARD,
    InputAction::QUICK_SAVE,
    InputAction::QUICK_LOAD,
];

/// The sections of the window, rebuilt when the language changes.
#[derive(Debug, Clone)]
pub struct KeyConfigModel {
    sections: Vec<KeyConfigSection>,
    built_for: Option<u32>,
}

impl Default for KeyConfigModel {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyConfigModel {
    /// Builds the sections in the language in force.
    #[must_use]
    pub fn new() -> Self {
        let mut model = Self {
            sections: Vec::new(),
            built_for: None,
        };
        model.rebuild();
        model
    }

    /// The sections, in the language in force.
    pub fn sections(&mut self) -> &[KeyConfigSection] {
        self.rebuild();
        &self.sections
    }

    /// Every row of every section.
    pub fn all_rows(&mut self) -> impl Iterator<Item = &KeyConfigRow> {
        self.rebuild();
        self.sections.iter().flat_map(|section| &section.rows)
    }

    fn rebuild(&mut self) {
        let generation = strings::generation();
        if self.built_for == Some(generation) {
            return;
        }
        self.built_for = Some(generation);
        let action_rows = |actions: &[InputAction], player: usize| {
            actions
                .iter()
                .map(|action| action_row(*action, player))
                .collect()
        };
        let command_rows = Command::ALL
            .iter()
            .filter(|command| !COMMANDS_NEEDING_ARGUMENTS.contains(command))
            .map(|command| command_row(command.name(), strings::get(&command.label_key())))
            .chain(UiCommand::ALL.iter().map(|command| {
                command_row(
                    command.name(),
                    strings::get(&super::keys::command(command.name())),
                )
            }))
            .collect();
        self.sections = vec![
            KeyConfigSection {
                title: strings::get(keys::KEYS_PLAYER_ONE),
                rows: action_rows(&NES_BUTTONS, 0),
            },
            KeyConfigSection {
                title: strings::get(keys::KEYS_PLAYER_TWO),
                rows: action_rows(&NES_BUTTONS, 1),
            },
            KeyConfigSection {
                title: strings::get(keys::KEYS_CAMERA),
                rows: action_rows(&CAMERA_ACTIONS, 0),
            },
            KeyConfigSection {
                title: strings::get(keys::KEYS_HOTKEYS),
                rows: action_rows(&HOTKEYS, 0),
            },
            KeyConfigSection {
                title: strings::get(keys::KEYS_COMMANDS),
                rows: command_rows,
            },
        ];
    }
}

fn action_row(action: InputAction, player: usize) -> KeyConfigRow {
    KeyConfigRow {
        label: input_action::name_of(action).to_owned(),
        target: BindingTarget::of(action),
        player,
    }
}

fn command_row(name: &str, label: String) -> KeyConfigRow {
    KeyConfigRow {
        label,
        target: BindingTarget::of_command(name),
        player: 0,
    }
}

/// The set a row of the window shows for `player` and `kind`: the keyboard's, or that of the pad
/// assigned to the player. None when there is no such pad.
#[must_use]
pub fn set_of<'b>(
    bindings: &'b InputBindings,
    pads: &GamepadAssignment,
    player: usize,
    kind: BindingDeviceKind,
) -> Option<&'b BindingSet> {
    match kind {
        BindingDeviceKind::Keyboard => Some(bindings.keyboard(player)),
        BindingDeviceKind::Gamepad => bindings.find_gamepad(pads.assigned_identity(player)?),
    }
}

/// Starts the capture of `row` on `kind`. Activating the cell erases its binding at once
/// (REQ-INP-042). Returns whether the session listens.
pub fn begin(
    session: &mut RebindingSession,
    bindings: &mut InputBindings,
    pads: &GamepadAssignment,
    hub: &dyn GamepadHub,
    row: &KeyConfigRow,
    kind: BindingDeviceKind,
) -> bool {
    session.select_player(row.player);
    session.select_device_kind(kind);
    session.begin(bindings, Some(PadAccess { pads, hub }), row.target.clone())
}

/// Whether the session listens for the cell of `row` on `kind`.
#[must_use]
pub fn is_listening_for(
    session: &RebindingSession,
    row: &KeyConfigRow,
    kind: BindingDeviceKind,
) -> bool {
    session.is_listening()
        && session.player() == row.player
        && session.device_kind() == kind
        && *session.target() == row.target
}
