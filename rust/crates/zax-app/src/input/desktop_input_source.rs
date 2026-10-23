//! Samples the keyboard, the pointer and the gamepads into one [`InputSnapshot`] per displayed
//! frame (SPEC-08 §3), through the bindings and the pad assignment.

use zax_platform::input::calibration;
use zax_platform::input::{GamepadDevice, GamepadHub, KeyboardDevice, PhysicalControl, PointerDevice};

use super::gamepad_assignment::GamepadAssignment;
use super::input_action::{InputAction, PLAYER_COUNT};
use super::input_bindings::InputBindings;
use super::input_snapshot::InputSnapshot;
use super::rebinding_session::{PadAccess, RebindingSession};
use crate::persistence::input_binding_store::{InputBindingStore, NullInputBindingStore};

/// The devices a sample reads, lent by their owner for the duration of the call.
pub struct DeviceRefs<'a> {
    /// The keyboard, if there is one.
    pub keyboard: Option<&'a dyn KeyboardDevice>,
    /// The pointer, if there is one.
    pub pointer: Option<&'a mut dyn PointerDevice>,
    /// The gamepads.
    pub hub: &'a mut dyn GamepadHub,
}

/// The input layer as the loop sees it.
pub struct DesktopInputSource {
    bindings: InputBindings,
    pads: GamepadAssignment,
    rebinding: RebindingSession,
    store: Box<dyn InputBindingStore>,
    saved_revision: u64,
    triggered: Vec<String>,
    held_commands: Vec<String>,
    last_pointer: [f32; 2],
    has_pointer: bool,
    /// Whether the interface has the keyboard (a text field has focus), so that no action is
    /// read from it.
    pub keyboard_captured_by_ui: bool,
    /// Whether the pointer is over the interface.
    pub pointer_over_ui: bool,
    /// Size of the render target, which the Zapper aims in.
    pub render_target_size: [f32; 2],
}

impl DesktopInputSource {
    /// Creates a source over `bindings`, writing them through `store` whenever they change.
    #[must_use]
    pub fn new(bindings: InputBindings, store: Box<dyn InputBindingStore>) -> Self {
        let saved_revision = bindings.revision();
        Self {
            bindings,
            pads: GamepadAssignment::new(),
            rebinding: RebindingSession::new(),
            store,
            saved_revision,
            triggered: Vec::new(),
            held_commands: Vec::new(),
            last_pointer: [0.0, 0.0],
            has_pointer: false,
            keyboard_captured_by_ui: false,
            pointer_over_ui: false,
            render_target_size: [1.0, 1.0],
        }
    }

    /// A source over the default bindings that is never written anywhere.
    #[must_use]
    pub fn unsaved() -> Self {
        Self::new(InputBindings::new(), Box::new(NullInputBindingStore))
    }

    /// The bindings.
    #[must_use]
    pub const fn bindings(&self) -> &InputBindings {
        &self.bindings
    }

    /// The pad assignment.
    #[must_use]
    pub const fn pads(&self) -> &GamepadAssignment {
        &self.pads
    }

    /// The capture in progress.
    #[must_use]
    pub const fn rebinding(&self) -> &RebindingSession {
        &self.rebinding
    }

    /// The commands whose binding went down on the last sample, on their rising edge.
    #[must_use]
    pub fn triggered_commands(&self) -> &[String] {
        &self.triggered
    }

    /// The store the bindings are written through.
    #[must_use]
    pub fn store(&self) -> &dyn InputBindingStore {
        self.store.as_ref()
    }

    /// Loads the bindings from the store, and takes the result as the saved state.
    pub fn load(&mut self) {
        self.store.load(&mut self.bindings);
        self.saved_revision = self.bindings.revision();
    }

    /// Edits the bindings and the pad assignment, then writes the bindings if they changed
    /// (REQ-INP-051: the original only writes at exit; writing after each change as well is
    /// the recommended behaviour, so nothing is lost on a hard kill).
    pub fn edit<R>(
        &mut self,
        edit: impl FnOnce(&mut InputBindings, &mut GamepadAssignment, &mut RebindingSession) -> R,
    ) -> R {
        let result = edit(&mut self.bindings, &mut self.pads, &mut self.rebinding);
        self.flush_changes();
        result
    }

    /// Writes the bindings when they changed since the last write.
    pub fn flush_changes(&mut self) {
        let revision = self.bindings.revision();
        if revision != self.saved_revision {
            self.store.save(&self.bindings);
            self.saved_revision = revision;
        }
    }

    /// Samples every device once for the displayed frame.
    pub fn sample(&mut self, real_seconds: f32, devices: DeviceRefs<'_>) -> InputSnapshot {
        let DeviceRefs {
            keyboard,
            pointer,
            hub,
        } = devices;
        hub.poll();
        let hub: &dyn GamepadHub = hub;
        self.pads.update(hub, &mut self.bindings, real_seconds);
        // A capture consumes the sample: no action and no command is produced from the command
        // that is being captured (REQ-INP-047, DEC-08-2).
        let access = PadAccess {
            pads: &self.pads,
            hub,
        };
        self.rebinding
            .capture(&mut self.bindings, Some(access), keyboard);
        let suppressed = self.rebinding.suppresses_actions();
        let (player_one, player_two) = if suppressed {
            (InputAction::NONE, InputAction::NONE)
        } else {
            (
                self.sample_player(0, keyboard, hub),
                self.sample_player(1, keyboard, hub),
            )
        };
        self.collect_commands(suppressed, keyboard, hub);
        let has_pointer = pointer.is_some();
        let (position, left_button, right_button, wheel) = match pointer {
            Some(pointer) => (
                pointer.position(),
                pointer.left_button(),
                pointer.right_button(),
                pointer.take_wheel_notches(),
            ),
            None => ([0.0, 0.0], false, false, 0.0),
        };
        let moved = self.has_pointer && pointer_differs(position, self.last_pointer);
        self.last_pointer = position;
        self.has_pointer = has_pointer;
        self.flush_changes();
        InputSnapshot {
            player_one,
            player_two,
            pointer: position,
            pointer_size: self.render_target_size,
            left_button,
            right_button,
            wheel,
            pointer_over_ui: self.pointer_over_ui,
            pointer_moved: moved,
        }
    }

    /// Ends the sample: a capture no longer suppresses the actions.
    pub fn end_sample(&mut self) {
        self.rebinding.end_sample();
    }

    fn sample_player(
        &mut self,
        player: usize,
        keyboard: Option<&dyn KeyboardDevice>,
        hub: &dyn GamepadHub,
    ) -> InputAction {
        let mut mask = InputAction::NONE;
        if let Some(keyboard) = keyboard.filter(|_| !self.keyboard_captured_by_ui) {
            mask |= self
                .bindings
                .keyboard(player)
                .sample(|control| keyboard_pressed(keyboard, control));
        }
        if let Some(pad) = self.pads.device(hub, player) {
            if let Some(set) = self.pads.gamepad_bindings(hub, &mut self.bindings, player) {
                mask |= set.sample(|control| gamepad_pressed(pad, control));
            }
        }
        mask
    }

    fn collect_commands(
        &mut self,
        suppressed: bool,
        keyboard: Option<&dyn KeyboardDevice>,
        hub: &dyn GamepadHub,
    ) {
        self.triggered.clear();
        if suppressed {
            // Nothing is pressed as far as the commands are concerned, so the next real press
            // is a rising edge again.
            self.held_commands.clear();
            return;
        }
        let mut pressed: Vec<String> = Vec::new();
        for player in 0..PLAYER_COUNT {
            if let Some(keyboard) = keyboard.filter(|_| !self.keyboard_captured_by_ui) {
                self.bindings
                    .keyboard(player)
                    .sample_commands(|control| keyboard_pressed(keyboard, control), &mut pressed);
            }
            if let Some(pad) = self.pads.device(hub, player) {
                if let Some(set) = self.pads.gamepad_bindings(hub, &mut self.bindings, player) {
                    set.sample_commands(|control| gamepad_pressed(pad, control), &mut pressed);
                }
            }
        }
        for command in &pressed {
            if !self.held_commands.contains(command) {
                self.held_commands.push(command.clone());
                self.triggered.push(command.clone());
            }
        }
        self.held_commands.retain(|held| pressed.contains(held));
    }
}

#[allow(clippy::float_cmp)] // the C# compares the pointer vectors exactly
fn pointer_differs(left: [f32; 2], right: [f32; 2]) -> bool {
    left != right
}

fn keyboard_pressed(keyboard: &dyn KeyboardDevice, control: PhysicalControl) -> bool {
    control.is_keyboard() && keyboard.is_down(control.code)
}

fn gamepad_pressed(pad: &dyn GamepadDevice, control: PhysicalControl) -> bool {
    control.is_gamepad() && calibration::is_pressed(pad.value(control))
}
