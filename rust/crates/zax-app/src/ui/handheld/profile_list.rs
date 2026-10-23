//! The choice of the profile of one game, as data: the profiles that match the game by name, then
//! the two generic entries. No search, no text.

use std::path::PathBuf;

use super::pad::PadButton;
use super::rom_list::{matching_profiles, RomEntry};
use crate::ui::profile_chooser::ProfileChoice;

/// What a button of the list asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileListCommand {
    /// Play the game with the profile `id`, and remember the pair.
    Choose {
        /// The game's file.
        rom: PathBuf,
        /// The library entry.
        id: String,
    },
    /// Leave the list.
    Close,
}

/// The profiles offered for one game.
#[derive(Debug, Clone)]
pub struct ProfileListModel {
    rom: PathBuf,
    name: String,
    choices: Vec<ProfileChoice>,
    selected: usize,
}

impl ProfileListModel {
    /// The list for `rom`: the profiles of `library` that match its name, then the generic ones.
    /// The profile the game has now is selected.
    #[must_use]
    pub fn for_rom(rom: &RomEntry, library: &[ProfileChoice]) -> Self {
        let mut choices: Vec<ProfileChoice> = matching_profiles(library, &rom.name)
            .into_iter()
            .cloned()
            .collect();
        choices.extend(library.iter().filter(|choice| choice.is_generic()).cloned());
        let selected = rom
            .profile
            .as_ref()
            .and_then(|current| choices.iter().position(|choice| choice.id == current.id))
            .unwrap_or(0);
        Self {
            rom: rom.path.clone(),
            name: rom.name.clone(),
            choices,
            selected,
        }
    }

    /// The name of the game.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The profiles.
    #[must_use]
    pub fn choices(&self) -> &[ProfileChoice] {
        &self.choices
    }

    /// The index of the selected profile.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// One press of a button.
    pub fn handle(&mut self, button: PadButton) -> Option<ProfileListCommand> {
        let count = self.choices.len();
        match button {
            PadButton::Up if count > 0 => self.selected = (self.selected + count - 1) % count,
            PadButton::Down if count > 0 => self.selected = (self.selected + 1) % count,
            PadButton::A => {
                return self
                    .choices
                    .get(self.selected)
                    .map(|choice| ProfileListCommand::Choose {
                        rom: self.rom.clone(),
                        id: choice.id.clone(),
                    });
            }
            PadButton::B => return Some(ProfileListCommand::Close),
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::rom_list::ProfileMatch;
    use super::*;
    use zax_profiles::catalogue::ProfileOrigin;
    use zax_profiles::library::GameProfileSource;

    fn choice(id: &str, title: &str, generic: bool) -> ProfileChoice {
        ProfileChoice {
            id: id.to_owned(),
            title: title.to_owned(),
            source: GameProfileSource::User,
            is_two_dimensional: generic,
            is_default_three_d: false,
            origin: ProfileOrigin::User,
        }
    }

    fn library() -> Vec<ProfileChoice> {
        vec![
            choice("2d", "2D Mode", true),
            choice("smb", "Super Mario Bros", false),
            choice("zelda", "Zelda", false),
        ]
    }

    #[test]
    fn the_matching_profiles_come_before_the_generic_ones() {
        let rom = RomEntry::new(PathBuf::from("/roms/Super Mario Bros (U).nes"));
        let model = ProfileListModel::for_rom(&rom, &library());
        let ids: Vec<&str> = model.choices().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(["smb", "2d"], ids.as_slice());
    }

    #[test]
    fn the_current_profile_is_selected_and_a_chooses_it() {
        let mut rom = RomEntry::new(PathBuf::from("/roms/Game.nes"));
        rom.profile = Some(ProfileMatch {
            id: "2d".to_owned(),
            title: "2D Mode".to_owned(),
        });
        let mut model = ProfileListModel::for_rom(&rom, &library());
        assert_eq!(0, model.selected());
        assert_eq!(
            Some(ProfileListCommand::Choose {
                rom: PathBuf::from("/roms/Game.nes"),
                id: "2d".to_owned()
            }),
            model.handle(PadButton::A)
        );
        assert_eq!(Some(ProfileListCommand::Close), model.handle(PadButton::B));
    }

    #[test]
    fn the_selection_wraps() {
        let rom = RomEntry::new(PathBuf::from("/roms/Zelda.nes"));
        let mut model = ProfileListModel::for_rom(&rom, &library());
        model.handle(PadButton::Up);
        assert_eq!(1, model.selected());
        model.handle(PadButton::Down);
        assert_eq!(0, model.selected());
    }
}
