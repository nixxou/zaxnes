//! The default bindings (SPEC-08 §4.2, §4.3).

use zax_platform::input::{standard_gamepad, Key, PhysicalControl};

use super::binding_set::BindingSet;
use super::binding_target::BindingDeviceKind;
use super::input_action::InputAction;
use super::key_layout::{self, KeyLayout};

/// Player one's default keys. `QuickSave` and `QuickLoad` have no default key: the combinations of
/// REQ-INP-104 are the only default route to the quick slot (SPEC-08 §4.2, §4.3).
pub const PLAYER_ONE_KEYBOARD: [(InputAction, Key); 20] = [
    (InputAction::LEFT, Key::Left),
    (InputAction::RIGHT, Key::Right),
    (InputAction::UP, Key::Up),
    (InputAction::DOWN, Key::Down),
    (InputAction::A, Key::X),
    (InputAction::B, Key::Z),
    (InputAction::SELECT, Key::Space),
    (InputAction::START, Key::Enter),
    (InputAction::ROT_LEFT, Key::F5),
    (InputAction::ROT_RIGHT, Key::F6),
    (InputAction::ROT_UP, Key::F7),
    (InputAction::ROT_DOWN, Key::F8),
    (InputAction::ZOOM_IN, Key::F9),
    (InputAction::ZOOM_OUT, Key::F10),
    (InputAction::MENU, Key::Escape),
    (InputAction::ROLL_BACK, Key::Comma),
    (InputAction::FAST_FORWARD, Key::Period),
    // Lot MAPPERS-N: the VS. System cabinet on 5, 6 and 9, the arcade emulators' keys.
    (InputAction::COIN_1, Key::Number5),
    (InputAction::COIN_2, Key::Number6),
    (InputAction::SERVICE, Key::Number9),
];

/// Player one's default keys of the `RetroArch` layout (lot KEYS, owner request): the pad as in
/// [`PLAYER_ONE_KEYBOARD`] (owner: the pad does not move), the camera on the keypad (4 and 6
/// turn, 8 and 2 tilt, 9 and 3 zoom; its 5, a fixed shortcut, is the default view), the rewind on
/// R and the held fast forward on L as in `RetroArch`. The menu has no key: F1 opens the console
/// mode, `RetroArch`'s menu, and Escape quits. Without a keypad the fixed keys of
/// [`crate::input::desktop_input_source`] take the camera: right Control with the arrows, Page Up
/// and Page Down, Home.
pub const PLAYER_ONE_KEYBOARD_RETROARCH: [(InputAction, Key); 19] = [
    (InputAction::LEFT, Key::Left),
    (InputAction::RIGHT, Key::Right),
    (InputAction::UP, Key::Up),
    (InputAction::DOWN, Key::Down),
    (InputAction::A, Key::X),
    (InputAction::B, Key::Z),
    (InputAction::SELECT, Key::Space),
    (InputAction::START, Key::Enter),
    (InputAction::ROT_LEFT, Key::Keypad4),
    (InputAction::ROT_RIGHT, Key::Keypad6),
    (InputAction::ROT_UP, Key::Keypad8),
    (InputAction::ROT_DOWN, Key::Keypad2),
    (InputAction::ZOOM_IN, Key::Keypad9),
    (InputAction::ZOOM_OUT, Key::Keypad3),
    (InputAction::ROLL_BACK, Key::R),
    (InputAction::FAST_FORWARD, Key::L),
    (InputAction::COIN_1, Key::Number5),
    (InputAction::COIN_2, Key::Number6),
    (InputAction::SERVICE, Key::Number9),
];

/// Player one's default keys in `layout`.
#[must_use]
pub fn player_one_keyboard(layout: KeyLayout) -> &'static [(InputAction, Key)] {
    match layout {
        KeyLayout::Zaxnes => &PLAYER_ONE_KEYBOARD,
        KeyLayout::RetroArch => &PLAYER_ONE_KEYBOARD_RETROARCH,
    }
}

/// The default key of `action` in `layout`, [`PhysicalControl::NONE`] when it has none.
fn default_key(layout: KeyLayout, action: InputAction) -> PhysicalControl {
    player_one_keyboard(layout)
        .iter()
        .find(|(candidate, _)| *candidate == action)
        .map_or(PhysicalControl::NONE, |(_, key)| {
            PhysicalControl::key(key.code())
        })
}

/// Brings player one's keyboard `set` from the defaults of `from` to those of `to` (lot KEYS):
/// every action still on its default key of `from` takes its default key of `to`; an action
/// the player bound to a key of their own keeps it. Returns whether anything moved.
pub fn bring_to_layout(set: &mut BindingSet, from: KeyLayout, to: KeyLayout) -> bool {
    if from == to {
        return false;
    }
    let mut moved = false;
    for action in crate::input::input_action::ALL {
        let old = default_key(from, action);
        let new = default_key(to, action);
        if old == new || set.get_action(action) != old {
            continue;
        }
        set.set_action(action, new);
        moved = true;
    }
    moved
}

/// The defaults of a gamepad that answers to the standard numbering (SPEC-08 §4.3).
#[must_use]
pub fn standard_gamepad_set() -> [(InputAction, PhysicalControl); 17] {
    [
        (
            InputAction::LEFT,
            PhysicalControl::button(standard_gamepad::D_PAD_LEFT),
        ),
        (
            InputAction::RIGHT,
            PhysicalControl::button(standard_gamepad::D_PAD_RIGHT),
        ),
        (
            InputAction::UP,
            PhysicalControl::button(standard_gamepad::D_PAD_UP),
        ),
        (
            InputAction::DOWN,
            PhysicalControl::button(standard_gamepad::D_PAD_DOWN),
        ),
        (
            InputAction::A,
            PhysicalControl::button(standard_gamepad::SOUTH),
        ),
        (
            InputAction::B,
            PhysicalControl::button(standard_gamepad::WEST),
        ),
        (
            InputAction::SELECT,
            PhysicalControl::button(standard_gamepad::BACK),
        ),
        (
            InputAction::START,
            PhysicalControl::button(standard_gamepad::START),
        ),
        (
            InputAction::ROLL_BACK,
            PhysicalControl::button(standard_gamepad::NORTH),
        ),
        (
            InputAction::FAST_FORWARD,
            PhysicalControl::button(standard_gamepad::EAST),
        ),
        (
            InputAction::ROT_LEFT,
            PhysicalControl::axis_negative(standard_gamepad::RIGHT_STICK_X),
        ),
        (
            InputAction::ROT_RIGHT,
            PhysicalControl::axis_positive(standard_gamepad::RIGHT_STICK_X),
        ),
        (
            InputAction::ROT_UP,
            PhysicalControl::axis_positive(standard_gamepad::RIGHT_STICK_Y),
        ),
        (
            InputAction::ROT_DOWN,
            PhysicalControl::axis_negative(standard_gamepad::RIGHT_STICK_Y),
        ),
        (
            InputAction::ZOOM_IN,
            PhysicalControl::trigger(standard_gamepad::LEFT_TRIGGER),
        ),
        (
            InputAction::ZOOM_OUT,
            PhysicalControl::trigger(standard_gamepad::RIGHT_TRIGGER),
        ),
        (
            InputAction::MENU,
            PhysicalControl::button(standard_gamepad::RIGHT_STICK),
        ),
    ]
}

/// Player one's keyboard: the defaults of §4.2, those of the layout in force (lot KEYS).
#[must_use]
pub fn create_player_one_keyboard() -> BindingSet {
    let mut set = BindingSet::new(BindingDeviceKind::Keyboard);
    for &(action, key) in player_one_keyboard(key_layout::layout()) {
        set.set_action(action, PhysicalControl::key(key.code()));
    }
    set
}

/// Player two's keyboard: nothing bound.
#[must_use]
pub const fn create_player_two_keyboard() -> BindingSet {
    BindingSet::new(BindingDeviceKind::Keyboard)
}

/// The keyboard of `player`: 0 for player one, anything else for player two.
#[must_use]
pub fn create_keyboard(player: usize) -> BindingSet {
    if player == 0 {
        create_player_one_keyboard()
    } else {
        create_player_two_keyboard()
    }
}

/// The defaults of a handheld console's pad (Rust-only, owner request), whose face buttons the
/// platform names by their labels (A right, B bottom, X top, Y left): NES A and B on A and B,
/// zoom on the shoulders, fast forward on R2. X, Y and L2 are the handheld interface's (default
/// view, 2D/3D, camera memory); no rewind on a handheld.
#[must_use]
pub fn handheld_gamepad_set() -> [(InputAction, PhysicalControl); 15] {
    [
        (
            InputAction::LEFT,
            PhysicalControl::button(standard_gamepad::D_PAD_LEFT),
        ),
        (
            InputAction::RIGHT,
            PhysicalControl::button(standard_gamepad::D_PAD_RIGHT),
        ),
        (
            InputAction::UP,
            PhysicalControl::button(standard_gamepad::D_PAD_UP),
        ),
        (
            InputAction::DOWN,
            PhysicalControl::button(standard_gamepad::D_PAD_DOWN),
        ),
        (
            InputAction::A,
            PhysicalControl::button(standard_gamepad::SOUTH),
        ),
        (
            InputAction::B,
            PhysicalControl::button(standard_gamepad::EAST),
        ),
        (
            InputAction::SELECT,
            PhysicalControl::button(standard_gamepad::BACK),
        ),
        (
            InputAction::START,
            PhysicalControl::button(standard_gamepad::START),
        ),
        (
            InputAction::FAST_FORWARD,
            PhysicalControl::trigger(standard_gamepad::RIGHT_TRIGGER),
        ),
        (
            InputAction::ROT_LEFT,
            PhysicalControl::axis_negative(standard_gamepad::RIGHT_STICK_X),
        ),
        (
            InputAction::ROT_RIGHT,
            PhysicalControl::axis_positive(standard_gamepad::RIGHT_STICK_X),
        ),
        (
            InputAction::ROT_UP,
            PhysicalControl::axis_positive(standard_gamepad::RIGHT_STICK_Y),
        ),
        (
            InputAction::ROT_DOWN,
            PhysicalControl::axis_negative(standard_gamepad::RIGHT_STICK_Y),
        ),
        (
            InputAction::ZOOM_OUT,
            PhysicalControl::button(standard_gamepad::LEFT_BUMPER),
        ),
        (
            InputAction::ZOOM_IN,
            PhysicalControl::button(standard_gamepad::RIGHT_BUMPER),
        ),
    ]
}

/// A gamepad set: the standard defaults when `standard`, otherwise empty (a pad without a
/// mapping reports raw indices, REQ-INP-021).
#[must_use]
pub fn create_gamepad(standard: bool) -> BindingSet {
    let mut set = BindingSet::new(BindingDeviceKind::Gamepad);
    if standard {
        for (action, control) in standard_gamepad_set() {
            set.set_action(action, control);
        }
    }
    set
}

/// The gamepad set of the desktop, which plays as its console mode (Rust-only, owner request):
/// the handheld defaults with the face buttons where a handheld has them, not by their labels,
/// as Mesen has them: NES A on the right button (Xbox B), NES B on the bottom one (Xbox A). The
/// rollback, which a handheld has not, goes on the right stick's click.
#[must_use]
pub fn create_desktop_console_gamepad(standard: bool) -> BindingSet {
    let mut set = create_former_desktop_console_gamepad(standard);
    if standard {
        set.set_action(
            InputAction::ROLL_BACK,
            PhysicalControl::button(standard_gamepad::RIGHT_STICK),
        );
    }
    set
}

/// The desktop's console set before the rollback had a button: what a pad saved then still has.
#[must_use]
pub fn create_former_desktop_console_gamepad(standard: bool) -> BindingSet {
    let mut set = create_handheld_gamepad(standard);
    if standard {
        set.set_action(
            InputAction::A,
            PhysicalControl::button(standard_gamepad::EAST),
        );
        set.set_action(
            InputAction::B,
            PhysicalControl::button(standard_gamepad::SOUTH),
        );
    }
    set
}

/// The gamepad set of a handheld console: the handheld defaults when `standard`.
#[must_use]
pub fn create_handheld_gamepad(standard: bool) -> BindingSet {
    let mut set = BindingSet::new(BindingDeviceKind::Gamepad);
    if standard {
        for (action, control) in handheld_gamepad_set() {
            set.set_action(action, control);
        }
    }
    set
}

/// The gamepad set of the Android package (owner request: as the desktop's console mode): the
/// handheld defaults, the face buttons by their labels as Android reports them, and the rollback
/// on the right stick's click as on the desktop.
#[must_use]
pub fn create_android_gamepad(standard: bool) -> BindingSet {
    let mut set = create_handheld_gamepad(standard);
    if standard {
        set.set_action(
            InputAction::ROLL_BACK,
            PhysicalControl::button(standard_gamepad::RIGHT_STICK),
        );
    }
    set
}
