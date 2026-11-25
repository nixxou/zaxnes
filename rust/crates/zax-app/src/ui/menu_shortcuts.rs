//! The fixed shortcuts of the menu (SPEC-10 REQ-UI-204, Q-030): which key, with which
//! modifiers, runs which command.

use zax_platform::input::Key;

use super::commands::UiCommand;
use super::keys;
use super::strings;
use crate::emulation::commands::{Command, CommandArgs};
pub use crate::input::key_layout::{layout, set_layout, KeyLayout};
use crate::persistence::save_slots::EXPOSED_SLOT_COUNT;

/// The modifier keys a shortcut asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShortcutModifiers {
    /// Control is held.
    pub control: bool,
    /// Shift is held.
    pub shift: bool,
    /// Alt is held.
    pub alt: bool,
}

impl ShortcutModifiers {
    /// No modifier.
    pub const NONE: Self = Self {
        control: false,
        shift: false,
        alt: false,
    };
    /// Control only.
    pub const CONTROL: Self = Self {
        control: true,
        shift: false,
        alt: false,
    };
    /// Shift only.
    pub const SHIFT: Self = Self {
        control: false,
        shift: true,
        alt: false,
    };
    /// Alt only.
    pub const ALT: Self = Self {
        control: false,
        shift: false,
        alt: true,
    };
}

/// What a shortcut runs: a command of the host's model or one the interface owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutCommand {
    /// A command of the host.
    Host(Command),
    /// A command of the interface.
    Ui(UiCommand),
}

impl ShortcutCommand {
    /// The stable name of the command.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Host(command) => command.name(),
            Self::Ui(command) => command.name(),
        }
    }
}

/// One fixed shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuShortcut {
    /// The key, as a physical position.
    pub key: Key,
    /// The modifiers that must be held, exactly.
    pub modifiers: ShortcutModifiers,
    /// The command it runs.
    pub command: ShortcutCommand,
    /// Its argument.
    pub args: CommandArgs,
    /// How the menu spells it.
    pub text: String,
}

/// The keys of the ten exposed slots, `F1` to `F10`.
pub const SLOT_KEYS: [Key; EXPOSED_SLOT_COUNT] = [
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
];

/// Every fixed shortcut, with the texts of the language in force.
#[must_use]
pub fn all() -> Vec<MenuShortcut> {
    match layout() {
        KeyLayout::Zaxnes => zaxnes(),
        KeyLayout::RetroArch => retroarch(),
    }
}

/// The shortcuts of both layouts.
fn common() -> Vec<MenuShortcut> {
    let shortcut = |key, modifiers, command, text: &str| MenuShortcut {
        key,
        modifiers,
        command,
        args: CommandArgs::default(),
        text: text.to_owned(),
    };
    let host = ShortcutCommand::Host;
    let mut list = vec![
        shortcut(
            Key::O,
            ShortcutModifiers::CONTROL,
            host(Command::OpenRom),
            "Ctrl+O",
        ),
        shortcut(
            Key::P,
            ShortcutModifiers::NONE,
            host(Command::PauseResume),
            "P",
        ),
        shortcut(
            Key::R,
            ShortcutModifiers::CONTROL,
            host(Command::Reset),
            "Ctrl+R",
        ),
        shortcut(
            Key::J,
            ShortcutModifiers::NONE,
            host(Command::FrameBack),
            "J",
        ),
        shortcut(
            Key::K,
            ShortcutModifiers::NONE,
            host(Command::FrameAdvance),
            "K",
        ),
        shortcut(
            Key::F11,
            ShortcutModifiers::NONE,
            host(Command::ToggleTwoDimensional),
            "F11",
        ),
        shortcut(
            Key::Enter,
            ShortcutModifiers::ALT,
            host(Command::ToggleFullscreen),
            &strings::get(keys::SHORTCUT_ALT_ENTER),
        ),
        shortcut(
            Key::Comma,
            ShortcutModifiers::CONTROL,
            host(Command::ShowOptions),
            "Ctrl+,",
        ),
        shortcut(
            Key::Q,
            ShortcutModifiers::CONTROL,
            host(Command::Quit),
            "Ctrl+Q",
        ),
        shortcut(
            Key::M,
            ShortcutModifiers::CONTROL,
            host(Command::AnnotateMovie),
            "Ctrl+M",
        ),
        // The debug overlay of DEC-10-Q7's neighbourhood: an addition, off by default, on a key
        // no default game binding uses.
        shortcut(
            Key::F12,
            ShortcutModifiers::NONE,
            ShortcutCommand::Ui(UiCommand::ToggleDebugOverlay),
            "F12",
        ),
    ];
    list.extend([
        shortcut(
            Key::F1,
            ShortcutModifiers::NONE,
            ShortcutCommand::Ui(UiCommand::ToggleConsole),
            "F1",
        ),
        shortcut(
            Key::S,
            ShortcutModifiers::CONTROL,
            ShortcutCommand::Ui(UiCommand::SaveStates),
            "Ctrl+S",
        ),
        shortcut(
            Key::L,
            ShortcutModifiers::CONTROL,
            ShortcutCommand::Ui(UiCommand::LoadStates),
            "Ctrl+L",
        ),
    ]);
    list
}

/// The `RetroArch` layout (lot KEYS): its hotkeys where Zaxnes has the function, the current slot
/// on F2, F4, F6 and F7 instead of one key per slot, the save state screens on Shift+F2 and
/// Shift+F4, the camera's default view on Home and the keypad's 5, Escape twice to quit.
fn retroarch() -> Vec<MenuShortcut> {
    let shortcut = |key, modifiers, command, text: &str| MenuShortcut {
        key,
        modifiers,
        command,
        args: CommandArgs::default(),
        text: text.to_owned(),
    };
    let host = ShortcutCommand::Host;
    let shift = strings::get(keys::SHORTCUT_SHIFT_PREFIX);
    let mut list = common();
    list.extend([
        shortcut(Key::H, ShortcutModifiers::NONE, host(Command::Reset), "H"),
        shortcut(
            Key::F,
            ShortcutModifiers::NONE,
            host(Command::ToggleFullscreen),
            "F",
        ),
        shortcut(
            Key::F2,
            ShortcutModifiers::NONE,
            host(Command::SaveCurrentSlot),
            "F2",
        ),
        shortcut(
            Key::F4,
            ShortcutModifiers::NONE,
            host(Command::LoadCurrentSlot),
            "F4",
        ),
        shortcut(
            Key::F6,
            ShortcutModifiers::NONE,
            host(Command::PreviousSlot),
            "F6",
        ),
        shortcut(
            Key::F7,
            ShortcutModifiers::NONE,
            host(Command::NextSlot),
            "F7",
        ),
        shortcut(
            Key::F2,
            ShortcutModifiers::SHIFT,
            ShortcutCommand::Ui(UiCommand::SaveStates),
            &format!("{shift}+F2"),
        ),
        shortcut(
            Key::F4,
            ShortcutModifiers::SHIFT,
            ShortcutCommand::Ui(UiCommand::LoadStates),
            &format!("{shift}+F4"),
        ),
        shortcut(
            Key::Home,
            ShortcutModifiers::NONE,
            host(Command::ResetView),
            &strings::get(keys::SHORTCUT_HOME),
        ),
        shortcut(
            Key::Keypad5,
            ShortcutModifiers::NONE,
            host(Command::ResetView),
            &strings::get(keys::SHORTCUT_KEYPAD_5),
        ),
        shortcut(
            Key::F3,
            ShortcutModifiers::NONE,
            host(Command::ToggleFpsCounter),
            "F3",
        ),
        shortcut(
            Key::F9,
            ShortcutModifiers::NONE,
            host(Command::ToggleMute),
            "F9",
        ),
        shortcut(
            Key::KeypadAdd,
            ShortcutModifiers::NONE,
            host(Command::VolumeUp),
            &strings::get(keys::SHORTCUT_KEYPAD_ADD),
        ),
        shortcut(
            Key::KeypadSubtract,
            ShortcutModifiers::NONE,
            host(Command::VolumeDown),
            &strings::get(keys::SHORTCUT_KEYPAD_SUBTRACT),
        ),
        shortcut(
            Key::Escape,
            ShortcutModifiers::NONE,
            ShortcutCommand::Ui(UiCommand::EscapeQuit),
            &strings::get(keys::SHORTCUT_ESCAPE),
        ),
    ]);
    list.extend(retroarch_tools());
    list
}

/// The keys of step 2 of lot KEYS in the `RetroArch` layout: the movie, the screenshot and the
/// shader presets.
fn retroarch_tools() -> [MenuShortcut; 4] {
    let shortcut = |key, command, text: &str| MenuShortcut {
        key,
        modifiers: ShortcutModifiers::NONE,
        command: ShortcutCommand::Host(command),
        args: CommandArgs::default(),
        text: text.to_owned(),
    };
    [
        shortcut(Key::O, Command::ToggleMovieRecording, "O"),
        shortcut(Key::F8, Command::TakeScreenshot, "F8"),
        shortcut(Key::M, Command::NextShader, "M"),
        shortcut(Key::N, Command::PreviousShader, "N"),
    ]
}

/// The layout Zaxnes always had: one key per slot, Home for the console mode too.
fn zaxnes() -> Vec<MenuShortcut> {
    let host = ShortcutCommand::Host;
    let mut list = common();
    list.push(MenuShortcut {
        key: Key::Home,
        modifiers: ShortcutModifiers::NONE,
        command: ShortcutCommand::Ui(UiCommand::ToggleConsole),
        args: CommandArgs::default(),
        text: strings::get(keys::SHORTCUT_HOME),
    });
    for (index, key) in SLOT_KEYS.into_iter().enumerate() {
        let slot = i32::try_from(index + 1).unwrap_or(i32::MAX);
        // AMD-013 (Q-030): Ctrl+Fn loads, Shift ("Maj") + Fn saves.
        list.push(MenuShortcut {
            key,
            modifiers: ShortcutModifiers::CONTROL,
            command: host(Command::LoadSlot),
            args: CommandArgs::of_number(slot),
            text: format!("Ctrl+F{slot}"),
        });
        list.push(MenuShortcut {
            key,
            modifiers: ShortcutModifiers::SHIFT,
            command: host(Command::SaveSlot),
            args: CommandArgs::of_number(slot),
            text: format!("{}+F{slot}", strings::get(keys::SHORTCUT_SHIFT_PREFIX)),
        });
    }
    list
}

/// The keys that act without a modifier. REQ-INP-106 lets an implementation add fixed shortcuts
/// provided they use no key of the table of §4.2, which is what this list has to keep true.
#[must_use]
pub fn bare_keys() -> Vec<Key> {
    distinct(
        all()
            .into_iter()
            .filter(|shortcut| shortcut.modifiers == ShortcutModifiers::NONE)
            .map(|shortcut| shortcut.key),
    )
}

/// The keys the application watches for a rising edge.
#[must_use]
pub fn watched_keys() -> Vec<Key> {
    distinct(all().into_iter().map(|shortcut| shortcut.key))
}

fn distinct(keys: impl Iterator<Item = Key>) -> Vec<Key> {
    let mut list = Vec::new();
    for key in keys {
        if !list.contains(&key) {
            list.push(key);
        }
    }
    list
}

/// The shortcut text of `command` (empty when the command has none).
#[must_use]
pub fn text_of(command: ShortcutCommand) -> String {
    all()
        .into_iter()
        .find(|shortcut| shortcut.command == command)
        .map(|shortcut| shortcut.text)
        .unwrap_or_default()
}

/// The commands the shortcuts pressed now with exactly `modifiers` run.
#[must_use]
pub fn resolve(
    pressed: &[Key],
    modifiers: ShortcutModifiers,
) -> Vec<(ShortcutCommand, CommandArgs)> {
    if pressed.is_empty() {
        return Vec::new();
    }
    all()
        .into_iter()
        .filter(|shortcut| shortcut.modifiers == modifiers && pressed.contains(&shortcut.key))
        .map(|shortcut| (shortcut.command, shortcut.args))
        .collect()
}

/// The keys among `pressed` that a shortcut with exactly `modifiers` (some modifier at least)
/// uses.
#[must_use]
pub fn resolve_keys(pressed: &[Key], modifiers: ShortcutModifiers) -> Vec<Key> {
    if modifiers == ShortcutModifiers::NONE {
        return Vec::new();
    }
    distinct(
        all()
            .into_iter()
            .filter(|shortcut| shortcut.modifiers == modifiers && pressed.contains(&shortcut.key))
            .map(|shortcut| shortcut.key),
    )
}
