//! The home screen as data: the `.nes` files of the ROM folder, sorted, with the selection, the
//! A-Z jump of the shoulder buttons and the 3D profile each game matches. It draws nothing.

use std::path::{Path, PathBuf};

use super::pad::PadButton;
use crate::ui::profile_chooser::ProfileChoice;

/// The 3D profile a ROM plays with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileMatch {
    /// The identifier of the library entry.
    pub id: String,
    /// Its title.
    pub title: String,
    /// Whether it is the "2D Mode" entry, which is no 3D profile.
    pub two_dimensional: bool,
}

/// One game of the ROM folder.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct RomEntry {
    /// The file.
    pub path: PathBuf,
    /// The name shown: the file name without its extension.
    pub name: String,
    /// The profile remembered or found for the game, if any.
    pub profile: Option<ProfileMatch>,
    /// The CRC-32 of its data (iNES header excluded), when the file could be read.
    pub crc: Option<u32>,
    /// Whether a widescreen hack is made for the game (Rust-only, owner request).
    pub widescreen: bool,
    /// Whether the widescreen is on for the game.
    pub widescreen_on: bool,
    /// The name of the built-in patch of the game, when it is made for another ROM of it (the
    /// profile or the name says the game, its data says another version): [W] in red.
    pub widescreen_other: Option<&'static str>,
    /// Whether that patch is forced on the game all the same.
    pub widescreen_forced: bool,
    /// Whether the ROM was not studied yet (its hacks screen never opened, the game never
    /// started): `widescreen_other` is the patch its profile or its name says, which may fit it
    /// or not ([W] in dark grey).
    pub widescreen_unstudied: bool,
}

impl RomEntry {
    /// An entry for `path`, without a profile.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        Self {
            path,
            name,
            profile: None,
            crc: None,
            widescreen: false,
            widescreen_on: false,
            widescreen_other: None,
            widescreen_forced: false,
            widescreen_unstudied: false,
        }
    }

    /// The title of its 3D profile, when it has one (not the "2D Mode" entry).
    #[must_use]
    pub fn profile_title(&self) -> Option<&str> {
        self.profile
            .as_ref()
            .filter(|profile| !profile.two_dimensional)
            .map(|profile| profile.title.as_str())
    }

    /// The letter the A-Z jump files the game under; `#` for a name that starts otherwise.
    #[must_use]
    pub fn initial(&self) -> char {
        self.name
            .chars()
            .find(|character| character.is_alphanumeric())
            .filter(char::is_ascii_alphabetic)
            .map_or('#', |letter| letter.to_ascii_uppercase())
    }
}

/// What a button of the home screen asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomListCommand {
    /// Play the selected game with its profile (or in 2D when it has none).
    Play(PathBuf),
    /// Choose the profile of the selected game.
    ChooseProfile(PathBuf),
    /// Open the options of the selected game.
    GameOptions(PathBuf),
    /// Open the options.
    Options,
    /// Turn the widescreen of the selected game on or off (Rust-only, owner request).
    ToggleWidescreen(PathBuf),
    /// Open the hacks of the selected game.
    Hacks(PathBuf),
}

/// The ROM files of `directory`, sorted by name without regard to case. A folder that cannot be
/// read is an empty list.
#[must_use]
pub fn scan(directory: &Path) -> Vec<RomEntry> {
    let Ok(listing) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut entries: Vec<RomEntry> = listing
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("nes"))
        })
        .map(RomEntry::new)
        .collect();
    entries.sort_by_cached_key(|entry| (entry.name.to_lowercase(), entry.path.clone()));
    entries
}

/// The name reduced to its letters and digits, lower case: "Super Mario Bros. (U)" and
/// "`super_mario_bros`" meet at "supermariobros" once the tag is cut.
#[must_use]
pub fn normalise(name: &str) -> String {
    let untagged = name.split(['(', '[']).next().unwrap_or_default();
    untagged
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The profiles of `choices` that suit a ROM called `name`: the generic entries never do. The
/// closest come first (same name, then a title the name starts with).
#[must_use]
pub fn matching_profiles<'a>(choices: &'a [ProfileChoice], name: &str) -> Vec<&'a ProfileChoice> {
    let wanted = normalise(name);
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<(u8, &ProfileChoice)> = choices
        .iter()
        .filter(|choice| !choice.is_generic())
        .filter_map(|choice| {
            let title = normalise(&choice.title);
            if title.is_empty() {
                None
            } else if title == wanted {
                Some((0, choice))
            } else if wanted.starts_with(&title) || title.starts_with(&wanted) {
                Some((1, choice))
            } else {
                None
            }
        })
        .collect();
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().map(|(_, choice)| choice).collect()
}

/// The list and its selection.
#[derive(Debug, Clone, Default)]
pub struct RomListModel {
    entries: Vec<RomEntry>,
    selected: usize,
}

impl RomListModel {
    /// A list of `entries`, which must already be in order.
    #[must_use]
    pub const fn new(entries: Vec<RomEntry>) -> Self {
        Self {
            entries,
            selected: 0,
        }
    }

    /// The games.
    #[must_use]
    pub fn entries(&self) -> &[RomEntry] {
        &self.entries
    }

    /// The games, to give them their profiles.
    pub fn entries_mut(&mut self) -> &mut [RomEntry] {
        &mut self.entries
    }

    /// Replaces the games, keeping the selection on the same file when it is still there.
    pub fn replace(&mut self, entries: Vec<RomEntry>) {
        let kept = self.selected_entry().map(|entry| entry.path.clone());
        self.entries = entries;
        self.selected = kept
            .and_then(|path| self.entries.iter().position(|entry| entry.path == path))
            .unwrap_or(0);
    }

    /// The index of the selected game.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// The selected game.
    #[must_use]
    pub fn selected_entry(&self) -> Option<&RomEntry> {
        self.entries.get(self.selected)
    }

    /// Selects the game with this file, when the list has it.
    pub fn select_path(&mut self, path: &Path) {
        if let Some(index) = self.entries.iter().position(|entry| entry.path == path) {
            self.selected = index;
        }
    }

    /// Moves the selection by `delta` rows, wrapping round the ends.
    pub fn move_by(&mut self, delta: isize) {
        let count = self.entries.len();
        if count == 0 {
            return;
        }
        let count = isize::try_from(count).unwrap_or(isize::MAX);
        let current = isize::try_from(self.selected).unwrap_or(0);
        self.selected = usize::try_from((current + delta).rem_euclid(count)).unwrap_or(0);
    }

    /// Jumps to the first game of the next letter, or of the previous one: from the middle of a
    /// letter, backwards goes to the start of that letter first. Wraps round the ends.
    pub fn jump_letter(&mut self, forward: bool) {
        let Some(current) = self.selected_entry().map(RomEntry::initial) else {
            return;
        };
        let first_of = |letter: char| self.entries.iter().position(|e| e.initial() == letter);
        let letters: Vec<char> = {
            let mut letters: Vec<char> = self.entries.iter().map(RomEntry::initial).collect();
            letters.dedup();
            letters
        };
        let Some(position) = letters.iter().position(|letter| *letter == current) else {
            return;
        };
        let start_of_current = first_of(current).unwrap_or(0);
        let target = if forward {
            letters[(position + 1) % letters.len()]
        } else if self.selected > start_of_current {
            current
        } else {
            letters[(position + letters.len() - 1) % letters.len()]
        };
        self.selected = first_of(target).unwrap_or(0);
    }

    /// One press of a button.
    pub fn handle(&mut self, button: PadButton) -> Option<RomListCommand> {
        match button {
            PadButton::Up => self.move_by(-1),
            PadButton::Down => self.move_by(1),
            PadButton::Left => self.move_by(-PAGE),
            PadButton::Right => self.move_by(PAGE),
            PadButton::L1 => self.jump_letter(false),
            PadButton::R1 => self.jump_letter(true),
            PadButton::A => {
                return self
                    .selected_entry()
                    .map(|entry| RomListCommand::Play(entry.path.clone()));
            }
            PadButton::X => {
                return self
                    .selected_entry()
                    .map(|entry| RomListCommand::ChooseProfile(entry.path.clone()));
            }
            PadButton::Y => return Some(RomListCommand::Options),
            PadButton::L2 => {
                return self
                    .selected_entry()
                    .filter(|entry| entry.widescreen || entry.widescreen_other.is_some())
                    .map(|entry| RomListCommand::ToggleWidescreen(entry.path.clone()));
            }
            PadButton::Start => {
                return self
                    .selected_entry()
                    .map(|entry| RomListCommand::Hacks(entry.path.clone()));
            }
            PadButton::Select => {
                return self
                    .selected_entry()
                    .map(|entry| RomListCommand::GameOptions(entry.path.clone()));
            }
            _ => {}
        }
        None
    }
}

/// Rows the left and right directions move by.
const PAGE: isize = 8;

/// The first row to draw so that `selected` shows in a window of `rows` rows over `total`.
#[must_use]
pub const fn first_visible(selected: usize, rows: usize, total: usize) -> usize {
    if rows == 0 || total <= rows {
        return 0;
    }
    let centred = selected.saturating_sub(rows / 2);
    let last_start = total - rows;
    if centred > last_start {
        last_start
    } else {
        centred
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zax_profiles::catalogue::ProfileOrigin;
    use zax_profiles::library::GameProfileSource;

    fn list(names: &[&str]) -> RomListModel {
        RomListModel::new(
            names
                .iter()
                .map(|name| RomEntry::new(PathBuf::from(format!("/roms/{name}.nes"))))
                .collect(),
        )
    }

    fn choice(title: &str, generic: bool) -> ProfileChoice {
        ProfileChoice {
            id: title.to_lowercase(),
            title: title.to_owned(),
            source: GameProfileSource::User,
            is_two_dimensional: generic,
            is_default_three_d: false,
            origin: ProfileOrigin::User,
        }
    }

    #[test]
    fn the_scan_keeps_the_nes_files_in_alphabetical_order() {
        let folder = std::env::temp_dir().join(format!("zax-rom-list-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        for name in ["b.nes", "A.NES", "c.txt", "Zelda.nes"] {
            std::fs::write(folder.join(name), b"x").unwrap();
        }
        let names: Vec<String> = scan(&folder).into_iter().map(|entry| entry.name).collect();
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(["A", "b", "Zelda"], names.as_slice());
        assert_eq!(
            Vec::<RomEntry>::new(),
            scan(Path::new("/this/folder/does/not/exist"))
        );
    }

    #[test]
    fn the_selection_wraps_round_the_ends() {
        let mut model = list(&["a", "b", "c"]);
        model.handle(PadButton::Up);
        assert_eq!(2, model.selected());
        model.handle(PadButton::Down);
        assert_eq!(0, model.selected());
        model.handle(PadButton::Right);
        assert_eq!(2, model.selected(), "a page is 8 rows, wrapped over 3");
    }

    #[test]
    fn the_shoulders_jump_from_letter_to_letter() {
        let mut model = list(&[
            "1942",
            "Adventure",
            "Alien",
            "Bomb",
            "Castle",
            "Cave",
            "Zip",
        ]);
        assert_eq!('#', model.selected_entry().unwrap().initial());
        model.handle(PadButton::R1);
        assert_eq!("Adventure", model.selected_entry().unwrap().name);
        model.handle(PadButton::R1);
        assert_eq!("Bomb", model.selected_entry().unwrap().name);
        model.handle(PadButton::Down);
        model.handle(PadButton::Down);
        assert_eq!("Cave", model.selected_entry().unwrap().name);
        // Backwards from the middle of a letter goes to its start, then to the letter before.
        model.handle(PadButton::L1);
        assert_eq!("Castle", model.selected_entry().unwrap().name);
        model.handle(PadButton::L1);
        assert_eq!("Bomb", model.selected_entry().unwrap().name);
        model.jump_letter(true);
        model.jump_letter(true);
        model.jump_letter(true);
        assert_eq!(
            "1942",
            model.selected_entry().unwrap().name,
            "forwards wraps"
        );
    }

    #[test]
    fn the_buttons_become_commands() {
        let mut model = list(&["a", "b"]);
        model.handle(PadButton::Down);
        let path = PathBuf::from("/roms/b.nes");
        assert_eq!(
            Some(RomListCommand::Play(path.clone())),
            model.handle(PadButton::A)
        );
        assert_eq!(
            Some(RomListCommand::ChooseProfile(path)),
            model.handle(PadButton::X)
        );
        assert_eq!(Some(RomListCommand::Options), model.handle(PadButton::Y));
        assert!(matches!(
            model.handle(PadButton::Select),
            Some(RomListCommand::GameOptions(_))
        ));
        assert_eq!(None, model.handle(PadButton::B));
        assert_eq!(None, RomListModel::default().handle(PadButton::A));
    }

    #[test]
    fn a_replaced_list_keeps_the_selection_on_the_same_file() {
        let mut model = list(&["a", "b", "c"]);
        model.select_path(Path::new("/roms/c.nes"));
        model.replace(list(&["a", "c"]).entries().to_vec());
        assert_eq!("c", model.selected_entry().unwrap().name);
        model.replace(Vec::new());
        assert!(model.selected_entry().is_none());
    }

    #[test]
    fn a_profile_matches_by_name_without_the_tags_and_never_by_a_generic_entry() {
        let choices = [
            choice("2D Mode", true),
            choice("Super Mario Bros", false),
            choice("Zelda", false),
        ];
        let titles = |name: &str| -> Vec<&str> {
            matching_profiles(&choices, name)
                .into_iter()
                .map(|choice| choice.title.as_str())
                .collect()
        };
        assert_eq!(
            ["Super Mario Bros"],
            titles("Super_Mario_Bros. (U) [!]").as_slice()
        );
        assert_eq!(["Zelda"], titles("zelda ii").as_slice());
        assert_eq!(Vec::<&str>::new(), titles("Metroid"));
        assert_eq!(Vec::<&str>::new(), titles("2D Mode"));
        assert_eq!(Vec::<&str>::new(), titles("(U)"));
    }

    #[test]
    fn the_window_follows_the_selection() {
        assert_eq!(0, first_visible(3, 10, 8));
        assert_eq!(0, first_visible(2, 10, 30));
        assert_eq!(15, first_visible(20, 10, 30));
        assert_eq!(20, first_visible(29, 10, 30));
        assert_eq!(0, first_visible(5, 0, 30));
    }
}
