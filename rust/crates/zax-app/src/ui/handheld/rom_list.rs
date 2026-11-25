//! The home screen as data: the `.nes` files of the ROM folder, sorted, with the selection and the
//! 3D profile each game matches; the shoulder buttons are the widescreen and the hacks of the
//! selected game (owner request). It draws nothing.

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
    /// Whether this row is the `Resume` entry of AMD-053 REQ-LOOP-076 rather than a ROM file: it
    /// heads the list while the resume point stands, and its `name` is the title of its game.
    pub resume: bool,
    /// Whether this row is "Back to library" (lot LAUNCH-OUTSIDE) rather than a ROM file: it
    /// heads the list of the files given on the command line and goes back to the ROM folder.
    pub back_to_library: bool,
    /// The address of a game of a page of games (web build), not downloaded yet: playing it
    /// downloads it (owner request, 2026-11-20).
    pub remote: Option<String>,
    /// The kind of archive the row's file is, `ZIP` or `7Z`, shown as a badge; none for a file of
    /// a game.
    pub archive: Option<&'static str>,
    /// The number of games of the row's archive when it holds several: the row is a folder of
    /// them, which opens on its list (owner request).
    pub archive_games: usize,
    /// Whether this row heads the list of an archive's games and goes back to the folder.
    pub close_archive: bool,
}

impl RomEntry {
    /// An entry for `path`, without a profile.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        let archive = archive_kind(&path);
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
            resume: false,
            back_to_library: false,
            remote: None,
            archive,
            archive_games: 0,
            close_archive: false,
        }
    }

    /// The row of a game of a page of games, at `url`, shown as `name`; its path is only a key
    /// of the list, no file.
    #[must_use]
    pub fn remote_row(index: usize, name: &str, url: &str) -> Self {
        Self {
            name: name.to_owned(),
            remote: Some(url.to_owned()),
            ..Self::new(PathBuf::from(format!("{REMOTE_FOLDER}/{index}")))
        }
    }

    /// The `Resume` row of AMD-053 REQ-LOOP-076, carrying the title of its game.
    #[must_use]
    pub fn resume_row(game: &str) -> Self {
        Self {
            name: game.to_owned(),
            resume: true,
            ..Self::new(PathBuf::new())
        }
    }

    /// The row heading the games of the archive `archive`: its name, and B or A on it goes back
    /// to the folder.
    #[must_use]
    pub fn close_archive_row(archive: &Path) -> Self {
        Self {
            close_archive: true,
            archive: None,
            ..Self::new(archive.to_path_buf())
        }
    }

    /// Whether the row is a game or a folder of games: what a letter jumps to and the filter keeps.
    #[must_use]
    pub const fn is_listed_game(&self) -> bool {
        !self.resume && !self.back_to_library && !self.close_archive
    }

    /// Whether the row is an archive of several games, which opens on its list.
    #[must_use]
    pub const fn is_folder(&self) -> bool {
        self.archive_games > 1 && !self.close_archive
    }

    /// The "Back to library" row of lot LAUNCH-OUTSIDE.
    #[must_use]
    pub fn library_row() -> Self {
        Self {
            back_to_library: true,
            ..Self::new(PathBuf::new())
        }
    }

    /// Whether the row is a game's, not the Resume or the "Back to library" row.
    #[must_use]
    pub const fn is_game(&self) -> bool {
        !self.resume && !self.back_to_library && !self.close_archive && !self.is_folder()
    }

    /// The title of its 3D profile, when it has one (not the "2D Mode" entry).
    #[must_use]
    pub fn profile_title(&self) -> Option<&str> {
        self.profile
            .as_ref()
            .filter(|profile| !profile.two_dimensional)
            .map(|profile| profile.title.as_str())
    }

    /// The letter the game is filed under, which the title shows; `#` for a name that starts
    /// otherwise.
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
    /// L1 went down on the selected game (Rust-only, owner request): its widescreen's settings show.
    Widescreen(PathBuf),
    /// Open the hacks of the selected game, its aids (Rust-only, owner request).
    Hacks(PathBuf),
    /// Open the achievements of the selected game (R2, while RetroAchievements is on).
    Achievements(PathBuf),
    /// Load the resume point (AMD-053 REQ-LOOP-076): the `Resume` row at the head of the list.
    ResumeLast,
    /// Leave the files of the command line for the ROM folder (lot LAUNCH-OUTSIDE).
    BackToLibrary,
    /// Show the games of the archive (a folder row).
    OpenArchive(PathBuf),
    /// Back from the games of an archive to its folder (its heading row, or B).
    CloseArchive,
}

/// `ZIP` or `7Z` for an archive's path, by its extension.
fn archive_kind(path: &Path) -> Option<&'static str> {
    if !crate::emulation::rom_file::is_archive(path) {
        return None;
    }
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase());
    Some(if extension.as_deref() == Some("7z") {
        "7Z"
    } else {
        "ZIP"
    })
}

/// The folder the paths of the remote rows are under; nothing is ever written there.
pub const REMOTE_FOLDER: &str = "/zaxnes-web/remote";

/// The rows of `remote` games added to the `local` ones, sorted with them by name; a remote game
/// whose name a local file already has is left out (the copy downloaded before).
#[must_use]
pub fn with_remote_games(
    mut local: Vec<RomEntry>,
    remote: &[crate::web_bridge::RemoteGame],
) -> Vec<RomEntry> {
    if remote.is_empty() {
        return local;
    }
    let present: std::collections::HashSet<String> = local
        .iter()
        .map(|entry| entry.name.to_lowercase())
        .collect();
    local.extend(
        remote
            .iter()
            .enumerate()
            .filter(|(_, game)| !present.contains(&game.name.to_lowercase()))
            .map(|(index, game)| RomEntry::remote_row(index, &game.name, &game.url)),
    );
    local.sort_by_cached_key(|entry| (entry.name.to_lowercase(), entry.path.clone()));
    local
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
        // The listing's own type, read with it: no look at each file (owner request). A link is
        // followed.
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|kind| kind.is_file() || kind.is_symlink() && entry.path().is_file())
        })
        .map(|entry| {
            let path = entry.path();
            if let Some(metadata) = entry.metadata().ok().filter(std::fs::Metadata::is_file) {
                crate::persistence::rom_crc_cache::remember_stamp(&path, &metadata);
            }
            path
        })
        .filter(|path| {
            // Lot MAPPERS-P: UNIF images and Study Box tapes too; lot FDS: disk images; and the
            // `.zip` and `.7z` archives of a game (owner request), by their extension alone.
            crate::emulation::rom_file::is_rom_file(path)
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
    // The GoodSets' `zz_` (and `z_`) that sends a version to the end of a list (owner request).
    let lower = untagged.to_lowercase();
    let untagged = ["zz_", "z_"]
        .iter()
        .find(|prefix| lower.starts_with(*prefix))
        .map_or(untagged, |prefix| &untagged[prefix.len()..]);
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

    /// Selects the next game after the selected one whose name starts with `letter` (any case),
    /// round the end of the list (owner request: a letter typed on the list). Returns whether
    /// one was found.
    pub fn jump_to_initial(&mut self, letter: char) -> bool {
        let wanted = letter.to_ascii_uppercase();
        let count = self.entries.len();
        let found = (1..=count)
            .map(|step| (self.selected + step) % count.max(1))
            .find(|index| {
                self.entries
                    .get(*index)
                    .is_some_and(|entry| entry.is_listed_game() && entry.initial() == wanted)
            });
        if let Some(index) = found {
            self.selected = index;
        }
        found.is_some()
    }

    /// Selects the row `index`, when the list has it.
    pub fn select_index(&mut self, index: usize) {
        if index < self.entries.len() {
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

    /// One press of a button.
    pub fn handle(&mut self, button: PadButton) -> Option<RomListCommand> {
        match button {
            PadButton::Up => self.move_by(-1),
            PadButton::Down => self.move_by(1),
            PadButton::Left => self.move_by(-PAGE),
            PadButton::Right => self.move_by(PAGE),
            // REQ-LOOP-076: the Resume row loads the resume point, as a slot load does; Start
            // plays, as A does (owner request).
            PadButton::A | PadButton::Start => return self.selected_entry().map(activate),
            PadButton::X => {
                return self
                    .selected_entry()
                    .filter(|entry| entry.is_game())
                    .map(|entry| RomListCommand::ChooseProfile(entry.path.clone()));
            }
            PadButton::Y => return Some(RomListCommand::Options),
            // Back from the games of an archive, which its heading row says.
            PadButton::B => {
                return self
                    .entries
                    .first()
                    .filter(|entry| entry.close_archive)
                    .map(|_| RomListCommand::CloseArchive);
            }
            // L1 the widescreen, R1 the hacks; Start plays, as A does (owner request).
            PadButton::L1 => {
                return self
                    .selected_entry()
                    .filter(|entry| entry.is_game())
                    .map(|entry| RomListCommand::Widescreen(entry.path.clone()));
            }
            PadButton::R1 => {
                return self
                    .selected_entry()
                    .filter(|entry| entry.is_game())
                    .map(|entry| RomListCommand::Hacks(entry.path.clone()));
            }
            PadButton::R2 => {
                return self
                    .selected_entry()
                    .filter(|entry| entry.is_game())
                    .map(|entry| RomListCommand::Achievements(entry.path.clone()));
            }
            PadButton::Select => {
                return self
                    .selected_entry()
                    .filter(|entry| entry.is_game())
                    .map(|entry| RomListCommand::GameOptions(entry.path.clone()));
            }
            _ => {}
        }
        None
    }
}

/// What A (or Start) does on `entry`.
fn activate(entry: &RomEntry) -> RomListCommand {
    if entry.resume {
        RomListCommand::ResumeLast
    } else if entry.back_to_library {
        RomListCommand::BackToLibrary
    } else if entry.close_archive {
        RomListCommand::CloseArchive
    } else if entry.is_folder() {
        RomListCommand::OpenArchive(entry.path.clone())
    } else {
        RomListCommand::Play(entry.path.clone())
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

    #[test]
    fn the_remote_games_join_the_list_by_name_without_the_local_copies() {
        use crate::web_bridge::RemoteGame;
        let local = vec![
            RomEntry::new(PathBuf::from("/roms/Castlevania.nes")),
            RomEntry::new(PathBuf::from("/roms/Zelda.nes")),
        ];
        let remote = [
            RemoteGame {
                name: "Metal Force".to_owned(),
                url: "https://a/m.zip".to_owned(),
            },
            RemoteGame {
                name: "castlevania".to_owned(),
                url: "https://a/c.zip".to_owned(),
            },
        ];
        let list = with_remote_games(local, &remote);
        let names: Vec<&str> = list.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(vec!["Castlevania", "Metal Force", "Zelda"], names);
        assert_eq!(Some("https://a/m.zip"), list[1].remote.as_deref());
        assert_eq!(None, list[0].remote);
    }

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
            favorite: false,
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

    /// AMD-053 REQ-LOOP-076: the Resume row heads the list, A and Start load the resume point,
    /// and the buttons that act on a ROM file do nothing on it.
    #[test]
    fn req_loop_076_the_resume_row_heads_the_list_and_loads_the_resume_point() {
        let mut entries = list(&["a", "b"]).entries().to_vec();
        entries.insert(0, RomEntry::resume_row("Zelda II"));
        let mut model = RomListModel::new(entries);

        assert_eq!(0, model.selected());
        assert_eq!("Zelda II", model.selected_entry().unwrap().name);
        assert_eq!(Some(RomListCommand::ResumeLast), model.handle(PadButton::A));
        assert_eq!(
            Some(RomListCommand::ResumeLast),
            model.handle(PadButton::Start)
        );
        // Nothing of a ROM file applies to it.
        assert_eq!(None, model.handle(PadButton::X));
        assert_eq!(None, model.handle(PadButton::Select));
        assert_eq!(None, model.handle(PadButton::L1));
        assert_eq!(None, model.handle(PadButton::R1));
        assert_eq!(None, model.handle(PadButton::L2));

        // The rows below it are the ROM files, untouched.
        model.handle(PadButton::Down);
        assert_eq!(
            Some(RomListCommand::Play(PathBuf::from("/roms/a.nes"))),
            model.handle(PadButton::A)
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

    /// Owner request: the `zz_` of the `GoodSets` does not hide the game's name.
    #[test]
    fn the_goodsets_zz_prefix_is_not_part_of_the_name() {
        assert_eq!(
            normalise("Super Mario Bros."),
            normalise("zz_Super Mario Bros. [World][GoodSet][p1][!]")
        );
        assert_eq!(normalise("Zelda"), normalise("z_Zelda (U)"));
        assert_eq!("zzyzx", normalise("Zzyzx"));
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
