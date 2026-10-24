//! The choice of the profile of one game, as data: the profiles that match the game by name, then
//! the two generic entries. No search, no text.

use std::path::PathBuf;

use super::pad::PadButton;
use super::rom_list::{normalise, RomEntry};
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

/// Rows the left and right directions move by.
const PAGE: usize = 8;

/// How many of the closest profiles head the list.
const CLOSEST: usize = 3;

/// The least likeness a title needs to be among the closest.
const CLOSE_ENOUGH: f32 = 0.3;

/// How alike two normalised names are, 0 to 1: 1 for the same name, 0.9 when one starts with the
/// other, otherwise the Dice coefficient of their pairs of letters (which a region tag or a few
/// letters of difference barely change).
fn likeness(left: &str, right: &str) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left == right {
        return 1.0;
    }
    if left.starts_with(right) || right.starts_with(left) {
        return 0.9;
    }
    let pairs = |text: &str| -> Vec<(char, char)> {
        let letters: Vec<char> = text.chars().collect();
        letters.windows(2).map(|pair| (pair[0], pair[1])).collect()
    };
    let (left, mut right) = (pairs(left), pairs(right));
    let total = left.len() + right.len();
    if total == 0 {
        return 0.0;
    }
    let mut shared = 0usize;
    for pair in left {
        if let Some(position) = right.iter().position(|candidate| *candidate == pair) {
            right.swap_remove(position);
            shared += 1;
        }
    }
    #[allow(clippy::cast_precision_loss)] // Name lengths are far below 2^24.
    let likeness = (2 * shared) as f32 / total as f32;
    likeness
}

impl ProfileListModel {
    /// The list for `rom` (owner request): the three profiles of `library` whose titles are the
    /// closest to its name, then the generic ones, then every other profile by title, so that any
    /// profile can be given to any ROM. The profile the game has now is selected.
    #[must_use]
    pub fn for_rom(rom: &RomEntry, library: &[ProfileChoice]) -> Self {
        let wanted = normalise(&rom.name);
        let mut closest: Vec<(f32, &ProfileChoice)> = library
            .iter()
            .filter(|choice| !choice.is_generic())
            .map(|choice| (likeness(&wanted, &normalise(&choice.title)), choice))
            .filter(|(score, _)| *score >= CLOSE_ENOUGH)
            .collect();
        closest.sort_by(|left, right| {
            right.0.total_cmp(&left.0).then_with(|| {
                left.1
                    .title
                    .to_lowercase()
                    .cmp(&right.1.title.to_lowercase())
            })
        });
        let mut choices: Vec<ProfileChoice> = closest
            .into_iter()
            .take(CLOSEST)
            .map(|(_, choice)| choice.clone())
            .collect();
        choices.extend(library.iter().filter(|choice| choice.is_generic()).cloned());
        let mut others: Vec<ProfileChoice> = library
            .iter()
            .filter(|choice| !choices.iter().any(|listed| listed.id == choice.id))
            .cloned()
            .collect();
        others.sort_by_cached_key(|choice| choice.title.to_lowercase());
        choices.extend(others);
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
            // A page at a time, stopping at the ends: the list holds every profile.
            PadButton::Left => self.selected = self.selected.saturating_sub(PAGE),
            PadButton::Right if count > 0 => {
                self.selected = (self.selected + PAGE).min(count - 1);
            }
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
    fn the_matching_profiles_come_first_then_the_generic_ones_then_every_other() {
        let rom = RomEntry::new(PathBuf::from("/roms/Super Mario Bros (U).nes"));
        let model = ProfileListModel::for_rom(&rom, &library());
        let ids: Vec<&str> = model.choices().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(["smb", "2d", "zelda"], ids.as_slice());
    }

    #[test]
    fn the_three_closest_titles_come_first_even_with_another_spelling() {
        let mut library = library();
        for (id, title) in [
            ("z2", "Zelda II - The Adventure of Link (U)"),
            ("z2b", "Zelda 2 Adventure"),
            ("mm", "Mega Man 2"),
            ("lz", "Legend Of Zelda"),
            ("ab", "Abadox"),
        ] {
            library.push(choice(id, title, false));
        }
        let rom = RomEntry::new(PathBuf::from(
            "/roms/Zelda II - The Adventure of Link (Europe).nes",
        ));
        let model = ProfileListModel::for_rom(&rom, &library);
        let ids: Vec<&str> = model.choices().iter().map(|c| c.id.as_str()).collect();
        assert_eq!("z2", ids[0]);
        assert!(ids[..3].contains(&"z2b"));
        // After the closest and the generic entry, the rest by title.
        let rest = &ids[ids.iter().position(|id| *id == "2d").unwrap() + 1..];
        let mut sorted = rest.to_vec();
        sorted.sort_by_key(|id| {
            model
                .choices()
                .iter()
                .find(|choice| choice.id == *id)
                .unwrap()
                .title
                .to_lowercase()
        });
        assert_eq!(sorted, rest);
    }

    #[test]
    fn likeness_is_one_for_the_same_name_and_low_for_another_game() {
        assert!((likeness("zelda", "zelda") - 1.0).abs() < f32::EPSILON);
        assert!(likeness("supermariobros", "zeldaii") < CLOSE_ENOUGH);
        assert!(likeness("zeldaiitheadventureoflink", "zelda2adventure") >= CLOSE_ENOUGH);
    }

    #[test]
    fn left_and_right_move_by_a_page_and_stop_at_the_ends() {
        let mut library = library();
        for index in 0..20 {
            library.push(choice(
                &format!("g{index:02}"),
                &format!("Game {index:02}"),
                false,
            ));
        }
        let rom = RomEntry::new(PathBuf::from("/roms/Zelda.nes"));
        let mut model = ProfileListModel::for_rom(&rom, &library);
        model.handle(PadButton::Right);
        assert_eq!(8, model.selected());
        model.handle(PadButton::Right);
        model.handle(PadButton::Right);
        assert_eq!(model.choices().len() - 1, model.selected());
        model.handle(PadButton::Left);
        model.handle(PadButton::Left);
        model.handle(PadButton::Left);
        model.handle(PadButton::Left);
        assert_eq!(0, model.selected());
    }

    #[test]
    fn the_current_profile_is_selected_and_a_chooses_it() {
        let mut rom = RomEntry::new(PathBuf::from("/roms/Game.nes"));
        rom.profile = Some(ProfileMatch {
            id: "2d".to_owned(),
            title: "2D Mode".to_owned(),
            two_dimensional: true,
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
        assert_eq!(2, model.selected());
        model.handle(PadButton::Down);
        assert_eq!(0, model.selected());
    }
}
