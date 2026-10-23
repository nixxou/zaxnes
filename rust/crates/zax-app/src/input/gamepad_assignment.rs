//! Which gamepad plays which player (SPEC-08 §2.4, REQ-INP-023 to REQ-INP-025): the pad that
//! was used last is reconnected after a short delay, and the first pad to show activity takes
//! the first free player.

use zax_platform::input::{GamepadDevice, GamepadHub, PhysicalControl};

use super::binding_set::BindingSet;
use super::input_action::PLAYER_COUNT;
use super::input_bindings::InputBindings;

/// How long the program waits at start-up before it reconnects the pads it remembers, in real
/// seconds, so that the SDL hot-plug events have arrived.
pub const STARTUP_DELAY_SECONDS: f32 = 0.5;

fn same_identity(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}

/// The player each connected pad is assigned to.
#[derive(Debug, Clone, Default)]
pub struct GamepadAssignment {
    assigned: [Option<String>; PLAYER_COUNT],
    since_start: f32,
    startup_done: bool,
}

impl GamepadAssignment {
    /// Creates an assignment with nobody assigned.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the start-up reconnection has happened.
    #[must_use]
    pub const fn startup_complete(&self) -> bool {
        self.startup_done
    }

    /// The identity of the pad assigned to `player`, whether or not it is connected.
    #[must_use]
    pub fn assigned_identity(&self, player: usize) -> Option<&str> {
        self.assigned.get(player).and_then(Option::as_deref)
    }

    /// The connected pad assigned to `player`.
    #[must_use]
    pub fn device<'h>(
        &self,
        hub: &'h dyn GamepadHub,
        player: usize,
    ) -> Option<&'h dyn GamepadDevice> {
        find(hub, self.assigned_identity(player)?)
    }

    /// The binding set of the pad assigned to `player`: the set of its identity, created from
    /// the §4.3 defaults and remembered at once the first time the pad is seen (REQ-INP-025).
    pub fn gamepad_bindings<'b>(
        &self,
        hub: &dyn GamepadHub,
        bindings: &'b mut InputBindings,
        player: usize,
    ) -> Option<&'b mut BindingSet> {
        let device = self.device(hub, player)?;
        Some(bindings.gamepad_mut(device.identity(), device.is_standard()))
    }

    /// Advances the start-up delay and assigns pads that show activity.
    pub fn update(
        &mut self,
        hub: &dyn GamepadHub,
        bindings: &mut InputBindings,
        real_seconds: f32,
    ) {
        if !self.startup_done {
            self.since_start += real_seconds;
            if self.since_start < STARTUP_DELAY_SECONDS {
                return;
            }
            self.reconnect_remembered(hub, bindings);
            self.startup_done = true;
        }
        self.assign_active_devices(hub, bindings);
    }

    /// Assigns `device` to `player`, taking it from the other player if it had it, or frees the
    /// player when `device` is `None`.
    pub fn assign(
        &mut self,
        bindings: &mut InputBindings,
        player: usize,
        device: Option<&dyn GamepadDevice>,
    ) {
        let player = player.min(PLAYER_COUNT - 1);
        let Some(device) = device else {
            self.assigned[player] = None;
            bindings.set_last_gamepad(player, None);
            return;
        };
        for other in (0..PLAYER_COUNT).filter(|other| *other != player) {
            if self.assigned[other]
                .as_deref()
                .is_some_and(|identity| same_identity(identity, device.identity()))
            {
                self.assigned[other] = None;
                bindings.set_last_gamepad(other, None);
            }
        }
        self.assigned[player] = Some(device.identity().to_owned());
        // REQ-INP-025: whatever the means, the set of this identity is loaded, and created from
        // the defaults the first time, as soon as the pad is assigned.
        bindings.gamepad_mut(device.identity(), device.is_standard());
        bindings.set_last_gamepad(player, Some(device.identity()));
    }

    /// Does the start-up reconnection now instead of waiting for the delay.
    pub fn skip_startup_delay(&mut self, hub: &dyn GamepadHub, bindings: &mut InputBindings) {
        if self.startup_done {
            return;
        }
        self.reconnect_remembered(hub, bindings);
        self.startup_done = true;
    }

    fn reconnect_remembered(&mut self, hub: &dyn GamepadHub, bindings: &mut InputBindings) {
        for player in 0..PLAYER_COUNT {
            let Some(identity) = bindings.last_gamepad(player).map(str::to_owned) else {
                continue;
            };
            if self.is_taken(&identity, Some(player)) {
                continue;
            }
            let Some(device) = find(hub, &identity) else {
                continue;
            };
            self.assigned[player] = Some(device.identity().to_owned());
            bindings.gamepad_mut(device.identity(), device.is_standard());
        }
    }

    fn assign_active_devices(&mut self, hub: &dyn GamepadHub, bindings: &mut InputBindings) {
        let Some(mut free) = self.first_player_without_pad(hub) else {
            return;
        };
        let mut active: Vec<PhysicalControl> = Vec::new();
        for device in hub.devices() {
            if self.is_taken(device.identity(), None) {
                continue;
            }
            active.clear();
            device.collect_active_controls(&mut active);
            if active.is_empty() {
                continue;
            }
            self.assign(bindings, free, Some(device.as_ref()));
            match self.first_player_without_pad(hub) {
                Some(next) => free = next,
                None => return,
            }
        }
    }

    fn first_player_without_pad(&self, hub: &dyn GamepadHub) -> Option<usize> {
        (0..PLAYER_COUNT).find(|player| self.device(hub, *player).is_none())
    }

    /// Whether another player than `player` (any player when `None`) holds `identity`.
    fn is_taken(&self, identity: &str, player: Option<usize>) -> bool {
        (0..PLAYER_COUNT)
            .filter(|other| Some(*other) != player)
            .any(|other| {
                self.assigned[other]
                    .as_deref()
                    .is_some_and(|assigned| same_identity(assigned, identity))
            })
    }
}

fn find<'h>(hub: &'h dyn GamepadHub, identity: &str) -> Option<&'h dyn GamepadDevice> {
    hub.devices()
        .iter()
        .map(Box::as_ref)
        .find(|device| same_identity(device.identity(), identity))
}
