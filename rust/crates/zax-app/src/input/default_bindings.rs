//! The default bindings (SPEC-08 §4.2, §4.3).

use zax_platform::input::{standard_gamepad, Key, PhysicalControl};

use super::binding_set::BindingSet;
use super::binding_target::BindingDeviceKind;
use super::input_action::InputAction;

/// Player one's default keys. `QuickSave` and `QuickLoad` have no default key: the combinations of
/// REQ-INP-104 are the only default route to the quick slot (SPEC-08 §4.2, §4.3).
pub const PLAYER_ONE_KEYBOARD: [(InputAction, Key); 17] = [
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
];

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

/// Player one's keyboard: the defaults of §4.2.
#[must_use]
pub fn create_player_one_keyboard() -> BindingSet {
    let mut set = BindingSet::new(BindingDeviceKind::Keyboard);
    for (action, key) in PLAYER_ONE_KEYBOARD {
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
