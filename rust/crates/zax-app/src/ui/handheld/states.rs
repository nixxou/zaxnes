//! The save state screen of the in-game menu (Rust-only, owner request): the numbered slots,
//! to save into or to load from, each with its date and, for the one selected, its thumbnail.
//! The two uses look apart (colour and title), so that a save is never taken for a load.

use super::pad::PadButton;
use crate::persistence::save_slots::EXPOSED_SLOT_COUNT;

/// Whether the screen saves or loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatesMode {
    Save,
    Load,
}

/// How a slot stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    /// No state.
    Empty,
    /// A state that loads as it is.
    Fits,
    /// A state made with the built-in patches set the other way: it loads once they are
    /// switched, which the host asks first (orange).
    OtherMode,
    /// A state made with another version of the built-in hack: it cannot load (red).
    OtherVersion,
}

/// What a press asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatesCommand {
    /// Save into this numbered slot.
    Save(i32),
    /// Load this numbered slot.
    Load(i32),
    /// Back to the in-game menu.
    Back,
}

/// Slots a press of Left or Right moves by.
const PAGE: usize = 5;

/// The screen: its use and the slot selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatesModel {
    mode: StatesMode,
    selected: usize,
}

impl StatesModel {
    /// The screen for `mode`, on the numbered slot `slot` (clamped to the slots there are).
    #[must_use]
    pub fn new(mode: StatesMode, slot: i32) -> Self {
        let index = usize::try_from(slot - 1).unwrap_or(0);
        Self {
            mode,
            selected: index.min(EXPOSED_SLOT_COUNT - 1),
        }
    }

    /// Whether it saves or loads.
    #[must_use]
    pub const fn mode(&self) -> StatesMode {
        self.mode
    }

    /// The index of the selected slot, from 0.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// The number of the selected slot, from 1.
    #[must_use]
    pub fn slot(&self) -> i32 {
        i32::try_from(self.selected + 1).unwrap_or(1)
    }

    /// One press of a button; `state_of` tells how a numbered slot stands. A on an empty slot
    /// loads nothing.
    pub fn handle(
        &mut self,
        button: PadButton,
        state_of: impl Fn(i32) -> SlotState,
    ) -> Option<StatesCommand> {
        let count = EXPOSED_SLOT_COUNT;
        match button {
            PadButton::Up => self.selected = (self.selected + count - 1) % count,
            PadButton::Down => self.selected = (self.selected + 1) % count,
            PadButton::Left | PadButton::L1 => self.selected = self.selected.saturating_sub(PAGE),
            PadButton::Right | PadButton::R1 => {
                self.selected = (self.selected + PAGE).min(count - 1);
            }
            PadButton::A => {
                let slot = self.slot();
                return match self.mode {
                    StatesMode::Save => Some(StatesCommand::Save(slot)),
                    StatesMode::Load if state_of(slot) == SlotState::Empty => None,
                    StatesMode::Load => Some(StatesCommand::Load(slot)),
                };
            }
            PadButton::B | PadButton::Menu => return Some(StatesCommand::Back),
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(state: SlotState) -> impl Fn(i32) -> SlotState {
        move |_| state
    }

    #[test]
    fn it_opens_on_the_slot_of_the_menu_and_wraps() {
        let mut model = StatesModel::new(StatesMode::Save, 3);
        assert_eq!(3, model.slot());
        model.handle(PadButton::Up, all(SlotState::Empty));
        model.handle(PadButton::Up, all(SlotState::Empty));
        model.handle(PadButton::Up, all(SlotState::Empty));
        assert_eq!(10, model.slot());
        model.handle(PadButton::Down, all(SlotState::Empty));
        assert_eq!(1, model.slot());
        assert_eq!(10, StatesModel::new(StatesMode::Load, 99).slot());
        assert_eq!(1, StatesModel::new(StatesMode::Load, -4).slot());
    }

    #[test]
    fn left_and_right_move_by_a_page_without_wrapping() {
        let mut model = StatesModel::new(StatesMode::Load, 1);
        model.handle(PadButton::Right, all(SlotState::Fits));
        assert_eq!(6, model.slot());
        model.handle(PadButton::R1, all(SlotState::Fits));
        assert_eq!(10, model.slot());
        model.handle(PadButton::Left, all(SlotState::Fits));
        model.handle(PadButton::L1, all(SlotState::Fits));
        assert_eq!(1, model.slot());
    }

    #[test]
    fn a_saves_anywhere_and_loads_only_a_state() {
        let mut save = StatesModel::new(StatesMode::Save, 4);
        assert_eq!(
            Some(StatesCommand::Save(4)),
            save.handle(PadButton::A, all(SlotState::Empty))
        );
        let mut load = StatesModel::new(StatesMode::Load, 4);
        assert_eq!(None, load.handle(PadButton::A, all(SlotState::Empty)));
        // The host asks before switching the patches, and refuses another version.
        for state in [
            SlotState::Fits,
            SlotState::OtherMode,
            SlotState::OtherVersion,
        ] {
            assert_eq!(
                Some(StatesCommand::Load(4)),
                load.handle(PadButton::A, all(state))
            );
        }
        assert_eq!(
            Some(StatesCommand::Back),
            load.handle(PadButton::B, all(SlotState::Fits))
        );
    }
}
