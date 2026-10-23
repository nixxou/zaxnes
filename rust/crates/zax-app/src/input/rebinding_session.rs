//! The capture of a new binding by the key configuration window (SPEC-08 REQ-INP-040 to
//! REQ-INP-047, Q-033).

use zax_platform::input::{GamepadHub, KeyboardDevice, PhysicalControl};

use super::binding_set::BindingSet;
use super::binding_target::{BindingDeviceKind, BindingTarget};
use super::gamepad_assignment::GamepadAssignment;
use super::input_action::{InputAction, PLAYER_COUNT};
use super::input_bindings::InputBindings;

/// The pad state a session needs when it captures from a gamepad.
#[derive(Clone, Copy)]
pub struct PadAccess<'a> {
    /// Who plays with which pad.
    pub pads: &'a GamepadAssignment,
    /// The connected pads.
    pub hub: &'a dyn GamepadHub,
}

/// Listens for the next control pressed and binds it to a target.
#[derive(Debug, Clone)]
pub struct RebindingSession {
    player: usize,
    device_kind: BindingDeviceKind,
    target: BindingTarget,
    is_listening: bool,
    captured_this_sample: bool,
    last_captured: PhysicalControl,
    held_at_start: Vec<PhysicalControl>,
    primed: bool,
}

impl Default for RebindingSession {
    fn default() -> Self {
        Self::new()
    }
}

impl RebindingSession {
    /// Creates a session on player one's keyboard, not listening.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            player: 0,
            device_kind: BindingDeviceKind::Keyboard,
            target: BindingTarget::Empty,
            is_listening: false,
            captured_this_sample: false,
            last_captured: PhysicalControl::NONE,
            held_at_start: Vec::new(),
            primed: false,
        }
    }

    /// The player whose bindings are edited.
    #[must_use]
    pub const fn player(&self) -> usize {
        self.player
    }

    /// The kind of device whose bindings are edited.
    #[must_use]
    pub const fn device_kind(&self) -> BindingDeviceKind {
        self.device_kind
    }

    /// What the control being captured will be bound to.
    #[must_use]
    pub const fn target(&self) -> &BindingTarget {
        &self.target
    }

    /// Whether a capture is waiting for a control.
    #[must_use]
    pub const fn is_listening(&self) -> bool {
        self.is_listening
    }

    /// Whether a control was captured on the current sample.
    #[must_use]
    pub const fn captured_this_sample(&self) -> bool {
        self.captured_this_sample
    }

    /// The last control captured.
    #[must_use]
    pub const fn last_captured(&self) -> PhysicalControl {
        self.last_captured
    }

    /// Whether the actions and commands of the devices are ignored: during a capture, and on
    /// the sample that ended it (REQ-INP-047).
    #[must_use]
    pub const fn suppresses_actions(&self) -> bool {
        self.is_listening || self.captured_this_sample
    }

    /// The set being edited: the keyboard of the player, or the set of the pad assigned to it.
    pub fn active_set<'b>(
        &self,
        bindings: &'b mut InputBindings,
        pads: Option<PadAccess<'_>>,
    ) -> Option<&'b mut BindingSet> {
        match self.device_kind {
            BindingDeviceKind::Keyboard => Some(bindings.keyboard_mut(self.player)),
            BindingDeviceKind::Gamepad => {
                let access = pads?;
                access.pads.gamepad_bindings(access.hub, bindings, self.player)
            }
        }
    }

    /// Edits the bindings of `player`; the capture in progress stops.
    pub fn select_player(&mut self, player: usize) {
        self.stop();
        self.player = player.min(PLAYER_COUNT - 1);
    }

    /// Edits the bindings of a kind of device; the capture in progress stops.
    pub fn select_device_kind(&mut self, kind: BindingDeviceKind) {
        self.stop();
        self.device_kind = kind;
    }

    /// The assigned pad changed: the capture in progress stops.
    pub fn notify_gamepad_changed(&mut self) {
        self.stop();
    }

    /// Starts listening for `target`. Activating the field erases its binding at once, so
    /// stopping afterwards leaves the action empty (REQ-INP-042, REQ-INP-045). Returns false
    /// when the target is empty or there is no set to edit.
    pub fn begin(
        &mut self,
        bindings: &mut InputBindings,
        pads: Option<PadAccess<'_>>,
        target: BindingTarget,
    ) -> bool {
        self.stop();
        if target.is_empty() {
            return false;
        }
        let Some(set) = self.active_set(bindings, pads) else {
            return false;
        };
        set.clear(&target);
        self.target = target;
        self.is_listening = true;
        self.primed = false;
        self.held_at_start.clear();
        true
    }

    /// Stops listening; the erased binding stays erased.
    pub fn stop(&mut self) {
        self.is_listening = false;
        self.target = BindingTarget::Empty;
        self.primed = false;
        self.held_at_start.clear();
    }

    /// Stops listening and forgets the last capture: the window closed.
    pub fn close(&mut self) {
        self.stop();
        self.captured_this_sample = false;
        self.last_captured = PhysicalControl::NONE;
    }

    /// Looks at the devices once per sample and binds the first control that was not already
    /// held when the capture began (Q-033). Returns the control bound, or
    /// [`PhysicalControl::NONE`].
    pub fn capture(
        &mut self,
        bindings: &mut InputBindings,
        pads: Option<PadAccess<'_>>,
        keyboard: Option<&dyn KeyboardDevice>,
    ) -> PhysicalControl {
        self.captured_this_sample = false;
        if !self.is_listening {
            return PhysicalControl::NONE;
        }
        let mut controls: Vec<PhysicalControl> = Vec::new();
        match self.device_kind {
            BindingDeviceKind::Keyboard => {
                // Keys only, modifiers included; mouse buttons are not part of the vocabulary
                // (REQ-INP-012, REQ-INP-042).
                if let Some(keyboard) = keyboard {
                    let mut keys: Vec<i32> = Vec::new();
                    keyboard.collect_held_keys(&mut keys);
                    keys.sort_unstable();
                    controls.extend(keys.into_iter().map(PhysicalControl::key));
                }
            }
            BindingDeviceKind::Gamepad => {
                // Buttons, hat, stick half-axes and triggers of the assigned pad, non-standard
                // controls included (REQ-INP-021, REQ-INP-042).
                if let Some(device) = pads.and_then(|access| access.pads.device(access.hub, self.player)) {
                    device.collect_active_controls(&mut controls);
                }
            }
        }
        if !self.primed {
            // Q-033: whatever is already held when the field is activated is ignored, so the
            // click or key press that opened the field never becomes the binding.
            self.held_at_start.clone_from(&controls);
            self.primed = true;
            return PhysicalControl::NONE;
        }
        let captured = controls
            .iter()
            .copied()
            .find(|control| !self.held_at_start.contains(control))
            .unwrap_or(PhysicalControl::NONE);
        // A control released since the field was activated becomes capturable again.
        self.held_at_start.retain(|held| controls.contains(held));
        if !captured.is_bound() {
            return PhysicalControl::NONE;
        }
        let target = std::mem::take(&mut self.target);
        self.stop();
        if let Some(set) = self.active_set(bindings, pads) {
            set.set(target, captured);
        }
        self.captured_this_sample = true;
        self.last_captured = captured;
        captured
    }

    /// Ends the sample: the capture no longer suppresses the actions.
    pub fn end_sample(&mut self) {
        self.captured_this_sample = false;
    }

    /// Starts listening for the single action `action`.
    pub fn begin_action(
        &mut self,
        bindings: &mut InputBindings,
        pads: Option<PadAccess<'_>>,
        action: InputAction,
    ) -> bool {
        self.begin(bindings, pads, BindingTarget::of(action))
    }

    /// Starts listening for the command `command`.
    pub fn begin_command(
        &mut self,
        bindings: &mut InputBindings,
        pads: Option<PadAccess<'_>>,
        command: &str,
    ) -> bool {
        self.begin(bindings, pads, BindingTarget::of_command(command))
    }
}
