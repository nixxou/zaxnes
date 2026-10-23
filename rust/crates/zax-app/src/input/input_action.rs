//! The actions a player can trigger and the masks they travel in (SPEC-08 REQ-INP-002).

use zax_render::controls::CameraAction;

use crate::flags::flag_set;

flag_set! {
    /// A set of input actions: the eight NES buttons, the six camera movements and the five
    /// hotkeys.
    pub struct InputAction(u32) {
        const A = 0x01;
        const B = 0x02;
        const SELECT = 0x04;
        const START = 0x08;
        const UP = 0x10;
        const DOWN = 0x20;
        const LEFT = 0x40;
        const RIGHT = 0x80;
        const ZOOM_IN = 0x100;
        const ZOOM_OUT = 0x200;
        const ROT_LEFT = 0x400;
        const ROT_RIGHT = 0x800;
        const ROT_UP = 0x1000;
        const ROT_DOWN = 0x2000;
        const MENU = 0x4000;
        const ROLL_BACK = 0x8000;
        const FAST_FORWARD = 0x10000;
        const QUICK_SAVE = 0x20000;
        const QUICK_LOAD = 0x40000;
        /// The eight bits the NES controller sees.
        const CONTROLLER_BYTE = 0xFF;
        /// The six camera movements.
        const CAMERA_MASK = 0x100 | 0x200 | 0x400 | 0x800 | 0x1000 | 0x2000;
        /// The five hotkeys.
        const HOTKEY_MASK = 0x4000 | 0x8000 | 0x10000 | 0x20000 | 0x40000;
    }
}

/// Number of bindable actions.
pub const COUNT: usize = 19;
/// Number of players.
pub const PLAYER_COUNT: usize = 2;

/// Every bindable action, in the order of the key configuration window.
pub const ALL: [InputAction; COUNT] = [
    InputAction::LEFT,
    InputAction::RIGHT,
    InputAction::UP,
    InputAction::DOWN,
    InputAction::A,
    InputAction::B,
    InputAction::SELECT,
    InputAction::START,
    InputAction::ROT_LEFT,
    InputAction::ROT_RIGHT,
    InputAction::ROT_UP,
    InputAction::ROT_DOWN,
    InputAction::ZOOM_IN,
    InputAction::ZOOM_OUT,
    InputAction::MENU,
    InputAction::ROLL_BACK,
    InputAction::FAST_FORWARD,
    InputAction::QUICK_SAVE,
    InputAction::QUICK_LOAD,
];

/// The persisted name of a single action, or the empty string for anything else.
#[must_use]
pub fn name_of(action: InputAction) -> &'static str {
    const NAMES: [&str; COUNT] = [
        "Left", "Right", "Up", "Down", "A", "B", "Select", "Start", "RotLeft", "RotRight",
        "RotUp", "RotDown", "ZoomIn", "ZoomOut", "Menu", "RollBack", "FastForward", "QuickSave",
        "QuickLoad",
    ];
    ALL.iter()
        .position(|candidate| *candidate == action)
        .map_or("", |index| NAMES[index])
}

/// The action named `name`, ignoring case.
#[must_use]
pub fn parse(name: &str) -> Option<InputAction> {
    if name.trim().is_empty() {
        return None;
    }
    ALL.iter()
        .copied()
        .find(|action| name_of(*action).eq_ignore_ascii_case(name))
}

/// The byte the NES controller sees.
#[must_use]
pub const fn controller_byte(mask: InputAction) -> u8 {
    (mask.bits() & InputAction::CONTROLLER_BYTE.bits()).to_le_bytes()[0]
}

/// The camera movements of `mask`, for the renderer's controls.
#[must_use]
pub fn to_camera_actions(mask: InputAction) -> CameraAction {
    let mut actions = CameraAction::NONE;
    for (action, camera) in [
        (InputAction::ROT_LEFT, CameraAction::ROTATE_LEFT),
        (InputAction::ROT_RIGHT, CameraAction::ROTATE_RIGHT),
        (InputAction::ROT_UP, CameraAction::ROTATE_UP),
        (InputAction::ROT_DOWN, CameraAction::ROTATE_DOWN),
        (InputAction::ZOOM_IN, CameraAction::ZOOM_IN),
        (InputAction::ZOOM_OUT, CameraAction::ZOOM_OUT),
    ] {
        if mask.contains(action) {
            actions = actions | camera;
        }
    }
    actions
}
