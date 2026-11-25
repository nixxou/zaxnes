//! The save state screen of the in-game menu (Rust-only, owner request): the numbered slots,
//! to save into or to load from, each with its date and, for the one selected, its thumbnail.
//! The two uses look apart (colour and title), so that a save is never taken for a load. The
//! Load screen starts with the auto slot (lot LAUNCH-OUTSIDE), which the Save screen has not.

use super::pad::PadButton;
use crate::persistence::save_slots::{numbered_slot_id, AUTO_SLOT_ID, EXPOSED_SLOT_COUNT};

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatesCommand {
    /// Save into this numbered slot.
    Save(i32),
    /// Load this slot, by its identifier (a number, or the auto slot).
    Load(String),
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
        let mut model = Self { mode, selected: 0 };
        model.selected = index.min(EXPOSED_SLOT_COUNT - 1) + model.first_numbered();
        model
    }

    /// The row of the first numbered slot: 1 on the Load screen, under the auto slot.
    const fn first_numbered(&self) -> usize {
        match self.mode {
            StatesMode::Save => 0,
            StatesMode::Load => 1,
        }
    }

    /// How many rows the screen has.
    #[must_use]
    pub const fn count(&self) -> usize {
        EXPOSED_SLOT_COUNT + self.first_numbered()
    }

    /// The identifier of the slot on row `index`.
    #[must_use]
    pub fn slot_id_at(&self, index: usize) -> String {
        if index < self.first_numbered() {
            AUTO_SLOT_ID.to_owned()
        } else {
            numbered_slot_id(i32::try_from(index - self.first_numbered() + 1).unwrap_or(1))
        }
    }

    /// The identifier of the selected slot.
    #[must_use]
    pub fn slot_id(&self) -> String {
        self.slot_id_at(self.selected)
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

    /// The number of the selected slot, from 1; none on the auto slot.
    #[must_use]
    pub fn slot(&self) -> Option<i32> {
        self.selected
            .checked_sub(self.first_numbered())
            .and_then(|index| i32::try_from(index + 1).ok())
    }

    /// One press of a button; `state_of` tells how a slot stands, by its identifier. A on an
    /// empty slot loads nothing.
    pub fn handle(
        &mut self,
        button: PadButton,
        state_of: impl Fn(&str) -> SlotState,
    ) -> Option<StatesCommand> {
        let count = self.count();
        match button {
            PadButton::Up => self.selected = (self.selected + count - 1) % count,
            PadButton::Down => self.selected = (self.selected + 1) % count,
            PadButton::Left | PadButton::L1 => self.selected = self.selected.saturating_sub(PAGE),
            PadButton::Right | PadButton::R1 => {
                self.selected = (self.selected + PAGE).min(count - 1);
            }
            PadButton::A => {
                let id = self.slot_id();
                return match (self.mode, self.slot()) {
                    (StatesMode::Save, Some(slot)) => Some(StatesCommand::Save(slot)),
                    (StatesMode::Save, None) => None,
                    (StatesMode::Load, _) if state_of(&id) == SlotState::Empty => None,
                    (StatesMode::Load, _) => Some(StatesCommand::Load(id)),
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

    fn all(state: SlotState) -> impl Fn(&str) -> SlotState {
        move |_| state
    }

    #[test]
    fn it_opens_on_the_slot_of_the_menu_and_wraps() {
        let mut model = StatesModel::new(StatesMode::Save, 3);
        assert_eq!(Some(3), model.slot());
        model.handle(PadButton::Up, all(SlotState::Empty));
        model.handle(PadButton::Up, all(SlotState::Empty));
        model.handle(PadButton::Up, all(SlotState::Empty));
        assert_eq!(Some(10), model.slot());
        model.handle(PadButton::Down, all(SlotState::Empty));
        assert_eq!(Some(1), model.slot());
        assert_eq!(Some(10), StatesModel::new(StatesMode::Load, 99).slot());
        assert_eq!(Some(1), StatesModel::new(StatesMode::Load, -4).slot());
    }

    #[test]
    fn left_and_right_move_by_a_page_without_wrapping() {
        let mut model = StatesModel::new(StatesMode::Save, 1);
        model.handle(PadButton::Right, all(SlotState::Fits));
        assert_eq!(Some(6), model.slot());
        model.handle(PadButton::R1, all(SlotState::Fits));
        assert_eq!(Some(10), model.slot());
        model.handle(PadButton::Left, all(SlotState::Fits));
        model.handle(PadButton::L1, all(SlotState::Fits));
        assert_eq!(Some(1), model.slot());
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
                Some(StatesCommand::Load("4".to_owned())),
                load.handle(PadButton::A, all(state))
            );
        }
        assert_eq!(
            Some(StatesCommand::Back),
            load.handle(PadButton::B, all(SlotState::Fits))
        );
    }

    /// Lot LAUNCH-OUTSIDE: the Load screen starts with the auto slot, which loads by its
    /// identifier; the Save screen has no such row.
    #[test]
    fn the_load_screen_heads_with_the_auto_slot() {
        let mut load = StatesModel::new(StatesMode::Load, 1);
        assert_eq!(EXPOSED_SLOT_COUNT + 1, load.count());
        assert_eq!("1", load.slot_id());
        load.handle(PadButton::Up, all(SlotState::Fits));
        assert_eq!(
            (None, AUTO_SLOT_ID.to_owned()),
            (load.slot(), load.slot_id())
        );
        assert_eq!(
            Some(StatesCommand::Load(AUTO_SLOT_ID.to_owned())),
            load.handle(PadButton::A, all(SlotState::Fits))
        );
        let save = StatesModel::new(StatesMode::Save, 1);
        assert_eq!(EXPOSED_SLOT_COUNT, save.count());
        assert_eq!("1", save.slot_id());
    }
}
