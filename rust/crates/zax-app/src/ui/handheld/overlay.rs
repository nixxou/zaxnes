//! The in-game menu as data: the rows, the selection, the slot chosen for the states and the
//! camera preset. A press of a button answers with the [`OverlayCommand`] it means.

use super::pad::PadButton;
use super::states::StatesMode;
use crate::persistence::save_slots::EXPOSED_SLOT_COUNT;

/// A row of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayItem {
    /// Back to the game.
    Resume,
    /// Save the state in the chosen slot.
    SaveState,
    /// Load the state of the chosen slot.
    LoadState,
    /// Fast forward on or off.
    FastForward,
    /// The camera preset.
    Camera,
    /// The options.
    Options,
    /// Stop the game and show the ROM list.
    QuitToList,
    /// Leave the application.
    ExitToLauncher,
}

/// The rows, in order.
pub const ITEMS: [OverlayItem; 8] = [
    OverlayItem::Resume,
    OverlayItem::SaveState,
    OverlayItem::LoadState,
    OverlayItem::FastForward,
    OverlayItem::Camera,
    OverlayItem::Options,
    OverlayItem::QuitToList,
    OverlayItem::ExitToLauncher,
];

/// The views of the camera the menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraPreset {
    /// The camera orbits the scene (third person).
    Orbit,
    /// The camera looks from the player (first person).
    FirstPerson,
    /// The view comes back to its starting place.
    ResetView,
}

impl CameraPreset {
    /// The presets, in the order the row cycles through them.
    pub const ALL: [Self; 3] = [Self::Orbit, Self::FirstPerson, Self::ResetView];

    /// The preset `step` places further, wrapping round.
    #[must_use]
    pub fn stepped(self, step: i32) -> Self {
        let count = i32::try_from(Self::ALL.len()).unwrap_or(1);
        let here = Self::ALL
            .iter()
            .position(|preset| *preset == self)
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(0);
        let next = usize::try_from((here + step).rem_euclid(count)).unwrap_or(0);
        Self::ALL[next]
    }
}

/// What the menu asks the application to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayCommand {
    /// Close the menu and go on.
    Resume,
    /// Open the save state screen, to save or to load, on this numbered slot.
    ChooseState(StatesMode, i32),
    /// Fast forward on or off.
    ToggleFastForward,
    /// Apply this camera preset.
    SetCamera(CameraPreset),
    /// Show the options.
    Options,
    /// Stop the game, back to the ROM list.
    QuitToList,
    /// Leave the application.
    ExitToLauncher,
}

/// The menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayModel {
    selected: usize,
    slot: i32,
    camera: CameraPreset,
}

impl Default for OverlayModel {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayModel {
    /// The menu on "Resume", slot 1, the orbiting camera.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selected: 0,
            slot: 1,
            camera: CameraPreset::Orbit,
        }
    }

    /// The selected row.
    #[must_use]
    pub fn selected(&self) -> OverlayItem {
        ITEMS[self.selected]
    }

    /// The index of the selected row.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// The numbered slot the states use.
    #[must_use]
    pub const fn slot(&self) -> i32 {
        self.slot
    }

    /// The camera preset shown on its row.
    #[must_use]
    pub const fn camera(&self) -> CameraPreset {
        self.camera
    }

    /// Makes `slot` the numbered slot the state rows show (the last one used).
    pub const fn set_slot(&mut self, slot: i32) {
        self.slot = slot;
    }

    /// Opens the menu again on its first row; the slot and the camera stay.
    pub const fn reopen(&mut self) {
        self.selected = 0;
    }

    /// One press of a button.
    pub fn handle(&mut self, button: PadButton) -> Option<OverlayCommand> {
        match button {
            PadButton::Up => self.selected = (self.selected + ITEMS.len() - 1) % ITEMS.len(),
            PadButton::Down => self.selected = (self.selected + 1) % ITEMS.len(),
            PadButton::Left => return self.step(-1),
            PadButton::Right => return self.step(1),
            PadButton::A => return Some(self.activate()),
            PadButton::B | PadButton::Menu => return Some(OverlayCommand::Resume),
            _ => {}
        }
        None
    }

    /// Left and right change the value of the row that has one; a new camera preset is applied
    /// at once, so the view behind the menu shows it.
    fn step(&mut self, step: i32) -> Option<OverlayCommand> {
        match self.selected() {
            OverlayItem::SaveState | OverlayItem::LoadState => {
                let count = i32::try_from(EXPOSED_SLOT_COUNT).unwrap_or(1);
                self.slot = (self.slot - 1 + step).rem_euclid(count) + 1;
                None
            }
            OverlayItem::Camera => Some(OverlayCommand::SetCamera(self.step_camera(step))),
            _ => None,
        }
    }

    /// Moves the camera preset `step` places on and returns it.
    pub fn step_camera(&mut self, step: i32) -> CameraPreset {
        self.camera = self.camera.stepped(step);
        self.camera
    }

    fn activate(&self) -> OverlayCommand {
        match self.selected() {
            OverlayItem::Resume => OverlayCommand::Resume,
            OverlayItem::SaveState => OverlayCommand::ChooseState(StatesMode::Save, self.slot),
            OverlayItem::LoadState => OverlayCommand::ChooseState(StatesMode::Load, self.slot),
            OverlayItem::FastForward => OverlayCommand::ToggleFastForward,
            OverlayItem::Camera => OverlayCommand::SetCamera(self.camera),
            OverlayItem::Options => OverlayCommand::Options,
            OverlayItem::QuitToList => OverlayCommand::QuitToList,
            OverlayItem::ExitToLauncher => OverlayCommand::ExitToLauncher,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go_to(model: &mut OverlayModel, item: OverlayItem) {
        while model.selected() != item {
            model.handle(PadButton::Down);
        }
    }

    #[test]
    fn the_selection_wraps_and_back_resumes() {
        let mut model = OverlayModel::new();
        assert_eq!(None, model.handle(PadButton::Up));
        assert_eq!(OverlayItem::ExitToLauncher, model.selected());
        assert_eq!(None, model.handle(PadButton::Down));
        assert_eq!(OverlayItem::Resume, model.selected());
        assert_eq!(Some(OverlayCommand::Resume), model.handle(PadButton::B));
        assert_eq!(Some(OverlayCommand::Resume), model.handle(PadButton::Menu));
    }

    #[test]
    fn the_slot_steps_on_the_state_rows_only_and_wraps() {
        let mut model = OverlayModel::new();
        model.handle(PadButton::Right);
        assert_eq!(1, model.slot(), "Resume has no value");
        go_to(&mut model, OverlayItem::SaveState);
        model.handle(PadButton::Right);
        assert_eq!(
            Some(OverlayCommand::ChooseState(StatesMode::Save, 2)),
            model.handle(PadButton::A)
        );
        model.handle(PadButton::Left);
        model.handle(PadButton::Left);
        assert_eq!(10, model.slot());
        go_to(&mut model, OverlayItem::LoadState);
        assert_eq!(
            Some(OverlayCommand::ChooseState(StatesMode::Load, 10)),
            model.handle(PadButton::A)
        );
    }

    #[test]
    fn every_row_answers_its_command() {
        let mut model = OverlayModel::new();
        for (item, command) in [
            (OverlayItem::FastForward, OverlayCommand::ToggleFastForward),
            (OverlayItem::Options, OverlayCommand::Options),
            (OverlayItem::QuitToList, OverlayCommand::QuitToList),
            (OverlayItem::ExitToLauncher, OverlayCommand::ExitToLauncher),
        ] {
            go_to(&mut model, item);
            assert_eq!(Some(command), model.handle(PadButton::A));
        }
    }

    #[test]
    fn the_camera_row_cycles_the_presets() {
        let mut model = OverlayModel::new();
        go_to(&mut model, OverlayItem::Camera);
        assert_eq!(
            Some(OverlayCommand::SetCamera(CameraPreset::FirstPerson)),
            model.handle(PadButton::Right)
        );
        assert_eq!(
            Some(OverlayCommand::SetCamera(CameraPreset::FirstPerson)),
            model.handle(PadButton::A)
        );
        model.handle(PadButton::Left);
        model.handle(PadButton::Left);
        assert_eq!(CameraPreset::ResetView, model.camera());
    }

    #[test]
    fn reopening_returns_to_the_first_row_and_keeps_the_slot() {
        let mut model = OverlayModel::new();
        go_to(&mut model, OverlayItem::LoadState);
        model.handle(PadButton::Right);
        model.reopen();
        assert_eq!(OverlayItem::Resume, model.selected());
        assert_eq!(2, model.slot());
    }
}
