//! Every binding set of the program: one keyboard per player and one set per gamepad
//! identity (SPEC-08 §2, REQ-INP-025).

use super::binding_set::BindingSet;
use super::default_bindings;
use super::input_action::PLAYER_COUNT;

/// The keyboards, the remembered gamepads and the pad each player used last.
#[derive(Debug, Clone)]
pub struct InputBindings {
    keyboards: [BindingSet; PLAYER_COUNT],
    gamepads: Vec<(String, BindingSet)>,
    last_gamepads: [Option<String>; PLAYER_COUNT],
    structure_revision: u64,
    /// Whether the pads get the handheld defaults (`default_bindings::handheld_gamepad_set`).
    handheld: bool,
}

impl Default for InputBindings {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether two sets hold the same bindings, in whatever order.
fn same_bindings(left: &BindingSet, right: &BindingSet) -> bool {
    left.bindings().len() == right.bindings().len()
        && left
            .bindings()
            .iter()
            .all(|binding| right.bindings().contains(binding))
}

fn same_identity(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}

impl InputBindings {
    /// Creates the bindings of a first launch: the default keyboards, no gamepad.
    #[must_use]
    pub fn new() -> Self {
        Self {
            keyboards: [
                default_bindings::create_keyboard(0),
                default_bindings::create_keyboard(1),
            ],
            gamepads: Vec::new(),
            last_gamepads: [None, None],
            structure_revision: 0,
            handheld: false,
        }
    }

    /// Gives the pads the handheld defaults from now on. A pad whose saved set is still the
    /// desktop default, which an earlier version gave every pad, gets them at once; a set the
    /// player changed is kept.
    pub fn use_handheld_defaults(&mut self) {
        self.handheld = true;
        let desktop = default_bindings::create_gamepad(true);
        let handheld = default_bindings::create_handheld_gamepad(true);
        for (_, set) in &mut self.gamepads {
            if same_bindings(set, &desktop) {
                set.replace(handheld.bindings().iter().cloned());
            }
        }
    }

    /// The default set of a `standard` pad (or an empty one) on this device.
    fn default_gamepad(&self, standard: bool) -> BindingSet {
        if self.handheld {
            default_bindings::create_handheld_gamepad(standard)
        } else {
            default_bindings::create_gamepad(standard)
        }
    }

    /// How many changes the bindings went through: the sets' own changes plus the gamepads
    /// added or forgotten and the last-used pads. The persistence layer writes when it moved.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.structure_revision
            + self.keyboards.iter().map(BindingSet::revision).sum::<u64>()
            + self
                .gamepads
                .iter()
                .map(|(_, set)| set.revision())
                .sum::<u64>()
    }

    /// The identities of the gamepads that have a set, in the order they were added.
    pub fn gamepad_identities(&self) -> impl Iterator<Item = &str> {
        self.gamepads.iter().map(|(identity, _)| identity.as_str())
    }

    /// The keyboard set of `player` (0 or 1; a larger number reads as player two).
    #[must_use]
    pub fn keyboard(&self, player: usize) -> &BindingSet {
        &self.keyboards[player.min(PLAYER_COUNT - 1)]
    }

    /// The keyboard set of `player`, for editing.
    pub fn keyboard_mut(&mut self, player: usize) -> &mut BindingSet {
        &mut self.keyboards[player.min(PLAYER_COUNT - 1)]
    }

    /// Whether `identity` has a set.
    #[must_use]
    pub fn has_gamepad(&self, identity: &str) -> bool {
        self.find_gamepad(identity).is_some()
    }

    /// The set of `identity`, if it has one.
    #[must_use]
    pub fn find_gamepad(&self, identity: &str) -> Option<&BindingSet> {
        self.gamepads
            .iter()
            .find(|(candidate, _)| same_identity(candidate, identity))
            .map(|(_, set)| set)
    }

    /// The set of `identity`, created with the defaults of a `standard` pad (or empty) when the
    /// pad is new.
    pub fn gamepad_mut(&mut self, identity: &str, standard: bool) -> &mut BindingSet {
        let existing = self
            .gamepads
            .iter()
            .position(|(candidate, _)| same_identity(candidate, identity));
        let position = if let Some(position) = existing {
            position
        } else {
            let defaults = self.default_gamepad(standard);
            self.gamepads.push((identity.to_owned(), defaults));
            self.structure_revision += 1;
            self.gamepads.len() - 1
        };
        &mut self.gamepads[position].1
    }

    /// The pad `player` used last.
    #[must_use]
    pub fn last_gamepad(&self, player: usize) -> Option<&str> {
        self.last_gamepads[player.min(PLAYER_COUNT - 1)].as_deref()
    }

    /// Remembers the pad `player` used last.
    pub fn set_last_gamepad(&mut self, player: usize, identity: Option<&str>) {
        let slot = &mut self.last_gamepads[player.min(PLAYER_COUNT - 1)];
        let unchanged = match (slot.as_deref(), identity) {
            (Some(current), Some(next)) => same_identity(current, next),
            (None, None) => true,
            _ => false,
        };
        if unchanged {
            return;
        }
        *slot = identity.map(str::to_owned);
        self.structure_revision += 1;
    }

    /// Puts `player`'s keyboard back to its defaults.
    pub fn reset_keyboard(&mut self, player: usize) {
        let defaults = default_bindings::create_keyboard(player);
        self.keyboard_mut(player)
            .replace(defaults.bindings().iter().cloned());
    }

    /// Puts the pad `identity` back to the defaults of a `standard` pad.
    pub fn reset_gamepad(&mut self, identity: &str, standard: bool) {
        let defaults = self.default_gamepad(standard);
        self.gamepad_mut(identity, standard)
            .replace(defaults.bindings().iter().cloned());
    }

    /// Puts everything back to a first launch: default keyboards, no pad remembered.
    pub fn reset_all(&mut self) {
        for player in 0..PLAYER_COUNT {
            self.reset_keyboard(player);
            self.last_gamepads[player] = None;
        }
        self.gamepads.clear();
        self.structure_revision += 1;
    }
}
