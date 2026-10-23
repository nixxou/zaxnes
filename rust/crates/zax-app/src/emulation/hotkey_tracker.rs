//! Turns the hotkey bits of the two players into the events of SPEC-08 REQ-INP-100 to
//! REQ-INP-104: the menu press, rollback, fast forward and the quick save and load, including
//! the Select-and-stick combinations.

use crate::flags::flag_set;
use crate::input::input_action::InputAction;
use crate::input::press_tracker::{EdgeDetector, PressKind, PressTracker};

flag_set! {
    /// What the hotkeys asked for on one displayed frame.
    pub struct HotkeyEvents(u32) {
        const MENU_SHORT = 1 << 0;
        const MENU_LONG = 1 << 1;
        const ROLLBACK = 1 << 2;
        const FAST_FORWARD = 1 << 3;
        const QUICK_SAVE = 1 << 4;
        const QUICK_LOAD = 1 << 5;
    }
}

/// How long Select must be held before a stick direction counts as a combination, in real
/// seconds.
pub const SELECT_HOLD_SECONDS: f32 = 0.1;

/// Edge detection over the hotkeys, once per displayed frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct HotkeyTracker {
    menu: PressTracker,
    combinations: [PlayerCombination; 2],
    rollback: EdgeDetector,
    quick_save: EdgeDetector,
    quick_load: EdgeDetector,
}

impl HotkeyTracker {
    /// Creates a tracker with nothing held.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How long Menu has been held, in real seconds.
    #[must_use]
    pub const fn menu_held_seconds(&self) -> f32 {
        self.menu.held_seconds()
    }

    /// Feeds one frame's actions of both players and returns the events.
    pub fn update(&mut self, player_one: InputAction, player_two: InputAction, real_seconds: f32) -> HotkeyEvents {
        let mask = player_one | player_two;
        let mut events = HotkeyEvents::NONE;
        match self.menu.update(mask.contains(InputAction::MENU), real_seconds) {
            PressKind::Short => events |= HotkeyEvents::MENU_SHORT,
            PressKind::Long => events |= HotkeyEvents::MENU_LONG,
            PressKind::None => {}
        }
        if self.rollback.rising(mask.contains(InputAction::ROLL_BACK)) {
            events |= HotkeyEvents::ROLLBACK;
        }
        // REQ-INP-103: a request per displayed frame while the key is held.
        if mask.contains(InputAction::FAST_FORWARD) {
            events |= HotkeyEvents::FAST_FORWARD;
        }
        if self.quick_save.rising(mask.contains(InputAction::QUICK_SAVE)) {
            events |= HotkeyEvents::QUICK_SAVE;
        }
        if self.quick_load.rising(mask.contains(InputAction::QUICK_LOAD)) {
            events |= HotkeyEvents::QUICK_LOAD;
        }
        events |= self.combinations[0].update(player_one, real_seconds);
        events |= self.combinations[1].update(player_two, real_seconds);
        events
    }

    /// [`update`](Self::update) with a single mask standing for player one.
    pub fn update_mask(&mut self, mask: InputAction, real_seconds: f32) -> HotkeyEvents {
        self.update(mask, InputAction::NONE, real_seconds)
    }
}

/// Select held, then a stick direction released: quick save (down) or quick load (up).
#[derive(Debug, Clone, Copy, Default)]
struct PlayerCombination {
    rot_down: EdgeDetector,
    rot_up: EdgeDetector,
    select_held: f32,
}

impl PlayerCombination {
    fn update(&mut self, mask: InputAction, real_seconds: f32) -> HotkeyEvents {
        let select = mask.contains(InputAction::SELECT);
        // The arming is tested against the hold measured before this frame, so a frame in which
        // Select and the stick appear together never fires.
        let armed = select && self.select_held >= SELECT_HOLD_SECONDS;
        self.select_held = if select { self.select_held + real_seconds } else { 0.0 };
        let mut events = HotkeyEvents::NONE;
        if self.rot_down.falling(mask.contains(InputAction::ROT_DOWN)) && armed {
            events |= HotkeyEvents::QUICK_SAVE;
        }
        if self.rot_up.falling(mask.contains(InputAction::ROT_UP)) && armed {
            events |= HotkeyEvents::QUICK_LOAD;
        }
        events
    }
}
