//! The bindings of one device: which physical control triggers which target (SPEC-08 §2).

use zax_platform::input::PhysicalControl;

use super::binding_target::{BindingDeviceKind, BindingTarget};
use super::input_action::InputAction;

/// The bindings of one keyboard or one gamepad, in insertion order.
///
/// Every mutation bumps [`revision`](Self::revision), which the persistence layer compares with
/// the revision it last wrote.
#[derive(Debug, Clone)]
pub struct BindingSet {
    kind: BindingDeviceKind,
    bindings: Vec<(BindingTarget, PhysicalControl)>,
    revision: u64,
}

impl BindingSet {
    /// Creates an empty set for a device of `kind`.
    #[must_use]
    pub const fn new(kind: BindingDeviceKind) -> Self {
        Self {
            kind,
            bindings: Vec::new(),
            revision: 0,
        }
    }

    /// The kind of device this set belongs to.
    #[must_use]
    pub const fn kind(&self) -> BindingDeviceKind {
        self.kind
    }

    /// How many bindings the set holds.
    #[must_use]
    pub fn count(&self) -> usize {
        self.bindings.len()
    }

    /// The bindings, in insertion order.
    #[must_use]
    pub fn bindings(&self) -> &[(BindingTarget, PhysicalControl)] {
        &self.bindings
    }

    /// How many times the set changed.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    fn eligible(&self, control: PhysicalControl) -> bool {
        match self.kind {
            BindingDeviceKind::Keyboard => control.is_keyboard(),
            BindingDeviceKind::Gamepad => control.is_gamepad(),
        }
    }

    /// The control bound to `target`, or [`PhysicalControl::NONE`].
    #[must_use]
    pub fn get(&self, target: &BindingTarget) -> PhysicalControl {
        self.bindings
            .iter()
            .find(|(candidate, _)| candidate == target)
            .map_or(PhysicalControl::NONE, |(_, control)| *control)
    }

    /// The control bound to `action`, or [`PhysicalControl::NONE`].
    #[must_use]
    pub fn get_action(&self, action: InputAction) -> PhysicalControl {
        self.get(&BindingTarget::of(action))
    }

    /// Binds `control` to `target`; an unbound control clears the binding. Returns false when
    /// the target is empty or the control does not belong to this kind of device (REQ-INP-042:
    /// a keyboard set only accepts keys, a gamepad set only pad controls; mouse buttons are not
    /// bindable at all, REQ-INP-012).
    pub fn set(&mut self, target: BindingTarget, control: PhysicalControl) -> bool {
        if target.is_empty() {
            return false;
        }
        if !control.is_bound() {
            self.clear(&target);
            return true;
        }
        if !self.eligible(control) {
            return false;
        }
        self.insert(target, control);
        self.revision += 1;
        true
    }

    /// [`set`](Self::set) for a single action.
    pub fn set_action(&mut self, action: InputAction, control: PhysicalControl) -> bool {
        self.set(BindingTarget::of(action), control)
    }

    fn insert(&mut self, target: BindingTarget, control: PhysicalControl) {
        match self.bindings.iter_mut().find(|(candidate, _)| *candidate == target) {
            Some(entry) => entry.1 = control,
            None => self.bindings.push((target, control)),
        }
    }

    /// Removes the binding of `target`.
    pub fn clear(&mut self, target: &BindingTarget) {
        let before = self.bindings.len();
        self.bindings.retain(|(candidate, _)| candidate != target);
        if self.bindings.len() != before {
            self.revision += 1;
        }
    }

    /// Removes every binding.
    pub fn clear_all(&mut self) {
        if self.bindings.is_empty() {
            return;
        }
        self.bindings.clear();
        self.revision += 1;
    }

    /// Replaces every binding with `bindings`, dropping the ones this device cannot hold.
    pub fn replace(&mut self, bindings: impl IntoIterator<Item = (BindingTarget, PhysicalControl)>) {
        self.bindings.clear();
        for (target, control) in bindings {
            if !target.is_empty() && control.is_bound() && self.eligible(control) {
                self.insert(target, control);
            }
        }
        self.revision += 1;
    }

    /// The actions whose control `is_pressed`.
    pub fn sample(&self, mut is_pressed: impl FnMut(PhysicalControl) -> bool) -> InputAction {
        let mut mask = InputAction::NONE;
        for (target, control) in &self.bindings {
            if target.is_action() && is_pressed(*control) {
                mask |= target.action();
            }
        }
        mask
    }

    /// Appends to `commands` the commands whose control `is_pressed`.
    pub fn sample_commands(
        &self,
        mut is_pressed: impl FnMut(PhysicalControl) -> bool,
        commands: &mut Vec<String>,
    ) {
        for (target, control) in &self.bindings {
            if let Some(command) = target.command() {
                if is_pressed(*control) {
                    commands.push(command.to_owned());
                }
            }
        }
    }
}
