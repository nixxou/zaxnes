//! The cheats in the host (lot CHEATS): the list of the game in play, kept in its own file
//! (`cheats/<rom>.json` in the user data, named as the game's other files), put in force on the
//! core whenever it changed or the core lost it, a switch for all of them, the selection that
//! `RetroArch`'s T, Y and U keys move and toggle, and the cheat database a list is filled from.

use std::path::{Path, PathBuf};

use zax_cheats::search::{Comparison, Operand, RamSearch, ValueSize, CPU_RAM, WORK_RAM};
use zax_cheats::{Cheat, CheatList};
use zax_settings::catalogue::SettingId;

use super::{EmulationHost, GameCatalog, GameView};
use crate::cheat_database::{CheatDatabase, FoundSheet};
use crate::ui::{keys, strings};
use crate::user_data_paths::write_atomic;

/// The name of the folder of the cheat files, in the user data and beside the program.
pub const CHEATS_FOLDER: &str = "cheats";

/// The cheats of the game in play.
#[derive(Debug)]
pub struct CheatState {
    /// The game whose list this is: the key its save files are named after.
    rom: Option<String>,
    /// The list, as the player sees it and as its file keeps it.
    list: CheatList,
    /// The version of what the core should have; the core's own stamp says what it has.
    stamp: u64,
    /// Whether the cheats are on at all (the switch of the window and of `RetroArch`'s U with
    /// nothing selected); not kept, every game starts with them on.
    on: bool,
    /// The cheat T and Y move to and U toggles.
    selected: usize,
    /// The engine refused the Game Genie codes of the list (they do nothing there).
    game_genie_refused: bool,
    /// The database last opened, with the path it was opened from (`None`: the built-in one).
    database: Option<(Option<PathBuf>, CheatDatabase)>,
    /// The data CRC of the game, read as it started (the core may be away later).
    crc: Option<u32>,
    /// The RAM search.
    search: RamSearchState,
}

impl Default for CheatState {
    fn default() -> Self {
        Self {
            rom: None,
            list: CheatList::default(),
            stamp: 1,
            on: true,
            selected: 0,
            game_genie_refused: false,
            database: None,
            crc: None,
            search: RamSearchState::default(),
        }
    }
}

/// What a game has to look its sheet up by: its data CRC and its file name (an archive entry's
/// own name, without the extension).
#[must_use]
pub fn sheet_keys(rom_path: &str, image: Option<&[u8]>) -> (Option<u32>, String) {
    let crc = image.map(zax_nes::builtin_patches::data_crc);
    let path = Path::new(rom_path);
    let stem = crate::emulation::rom_file::archive_member(path).map_or_else(
        || {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        },
        |(_, entry)| {
            let name = entry.rsplit(['/', '\\']).next().unwrap_or(&entry);
            name.rsplit_once('.')
                .map_or(name, |(stem, _)| stem)
                .to_owned()
        },
    );
    (crc, stem)
}

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// The cheats of the game in play.
    #[must_use]
    pub fn cheats(&self) -> &CheatList {
        &self.cheats.list
    }

    /// Whether the cheats are on at all.
    #[must_use]
    pub const fn cheats_on(&self) -> bool {
        self.cheats.on
    }

    /// Turns all the cheats on or off, their own switches kept.
    pub fn set_cheats_on(&mut self, on: bool) {
        if self.cheats.on != on {
            self.cheats.on = on;
            self.cheats.stamp += 1;
        }
    }

    /// Whether the engine of the game refused its Game Genie codes.
    #[must_use]
    pub const fn cheats_game_genie_refused(&self) -> bool {
        self.cheats.game_genie_refused
    }

    /// The cheat `T`, `Y` and `U` act on.
    #[must_use]
    pub const fn selected_cheat(&self) -> usize {
        self.cheats.selected
    }

    /// Replaces the list of the game in play, writes its file and puts it in force.
    pub fn replace_cheats(&mut self, list: CheatList) {
        if list == self.cheats.list {
            return;
        }
        self.cheats.list = list;
        self.cheats.selected = self
            .cheats
            .selected
            .min(self.cheats.list.cheats.len().saturating_sub(1));
        self.cheats.stamp += 1;
        self.save_cheats();
    }

    /// The file the cheats of `rom` (the key of its save files) are kept in.
    #[must_use]
    pub fn cheat_file(&self, rom: &str) -> PathBuf {
        self.paths
            .root
            .join(CHEATS_FOLDER)
            .join(format!("{rom}.json"))
    }

    fn save_cheats(&mut self) {
        let Some(rom) = self.cheats.rom.clone() else {
            return;
        };
        if !self.paths.is_writable {
            return;
        }
        let file = self.cheat_file(&rom);
        let result = if self.cheats.list.cheats.is_empty() {
            match std::fs::remove_file(&file) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
                _ => Ok(()),
            }
        } else {
            self.cheats
                .list
                .to_json()
                .map_err(std::io::Error::other)
                .and_then(|json| {
                    if let Some(folder) = file.parent() {
                        std::fs::create_dir_all(folder)?;
                    }
                    write_atomic(&file, json.as_bytes())
                })
        };
        if let Err(error) = result {
            (self.log)(&format!("Cheats: {} not written: {error}", file.display()));
        }
    }

    /// The cheats of the game that just started: read from its file when it is another game
    /// than the last; put in force on the core in any case.
    pub(super) fn start_cheats(&mut self) {
        // Named as the save files, but after the entry itself for a game in an archive: the
        // games of one archive are versions of one another, whose codes differ.
        let rom = self
            .current_rom_path()
            .map(|path| sheet_keys(path, None).1)
            .filter(|stem| !stem.is_empty())
            .or_else(|| self.slots.rom().map(str::to_owned));
        if rom != self.cheats.rom {
            self.cheats.list = CheatList::default();
            self.cheats.selected = 0;
            self.cheats.on = true;
            if let Some(rom) = &rom {
                let file = self.cheat_file(rom);
                match std::fs::read_to_string(&file) {
                    Ok(text) => match CheatList::from_json(&text) {
                        Ok(imported) => {
                            for (description, error) in &imported.skipped {
                                (self.log)(&format!("Cheats: {description} skipped: {error}"));
                            }
                            self.cheats.list.cheats = imported.cheats;
                        }
                        Err(error) => {
                            (self.log)(&format!("Cheats: {} unreadable: {error}", file.display()));
                        }
                    },
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        (self.log)(&format!("Cheats: {} unreadable: {error}", file.display()));
                    }
                }
            }
            self.cheats.rom = rom;
            self.cheats.search = RamSearchState::default();
        }
        self.cheats.crc = self
            .session
            .as_ref()
            .filter(|session| session.core_is_home())
            .and_then(|session| session.core().original_image())
            .map(zax_nes::builtin_patches::data_crc);
        self.cheats.stamp += 1;
        self.sync_cheats();
    }

    /// Puts the cheats in force on the core when it does not have this version yet (a change,
    /// or a new ROM that dropped them). Only with the core here.
    pub(super) fn sync_cheats(&mut self) {
        self.run_ram_search();
        let stamp = self.cheats.stamp;
        let Some(session) = self
            .session
            .as_mut()
            .filter(|session| session.has_rom() && session.core_is_home())
        else {
            return;
        };
        let core = session.core_mut();
        if core.cheats_stamp() == stamp {
            return;
        }
        let none = CheatList::default();
        let list = if self.cheats.on {
            &self.cheats.list
        } else {
            &none
        };
        self.cheats.game_genie_refused = !core.set_cheats_stamped(list, stamp);
    }

    /// `RetroArch`'s T and Y: the previous or next cheat, named in a notification.
    pub(super) fn step_selected_cheat(&mut self, step: isize) {
        let count = self.cheats.list.cheats.len();
        if count == 0 {
            self.notify(&strings::get(keys::CHEATS_NONE_NOTICE));
            return;
        }
        let selected = self.cheats.selected.min(count - 1);
        self.cheats.selected = selected
            .checked_add_signed(step)
            .map_or(count - 1, |next| next % count);
        self.notify_selected_cheat();
    }

    /// `RetroArch`'s U: the selected cheat on or off.
    pub(super) fn toggle_selected_cheat(&mut self) {
        let mut list = self.cheats.list.clone();
        let Some(cheat) = list.cheats.get_mut(self.cheats.selected) else {
            self.notify(&strings::get(keys::CHEATS_NONE_NOTICE));
            return;
        };
        cheat.enabled = !cheat.enabled;
        if cheat.enabled {
            self.set_cheats_on(true);
        }
        self.replace_cheats(list);
        self.notify_selected_cheat();
    }

    fn notify_selected_cheat(&mut self) {
        let Some(cheat) = self.cheats.list.cheats.get(self.cheats.selected) else {
            return;
        };
        let state = strings::get(if cheat.enabled && self.cheats.on {
            keys::CHEATS_STATE_ON
        } else {
            keys::CHEATS_STATE_OFF
        });
        let number = (self.cheats.selected + 1).to_string();
        let count = self.cheats.list.cheats.len().to_string();
        let description = first_line(&cheat.description).to_owned();
        let message = strings::format(
            &strings::get(keys::CHEATS_SELECTED_NOTICE),
            &[&number, &count, &description, &state],
        );
        self.notify(&message);
    }

    /// The cheat database in use: the one the settings name, else `cheats.zip` or a `cheats`
    /// folder of sheets beside the program, else `cheats/database.zip` or `cheats/database` in
    /// the user data.
    #[must_use]
    pub fn cheat_database_path(&self) -> Option<PathBuf> {
        let chosen = self
            .settings
            .get(SettingId::CheatsDatabase)
            .as_text()
            .to_owned();
        if !chosen.is_empty() {
            return Some(PathBuf::from(chosen));
        }
        let program = &self.paths.executable_directory;
        let user = self.paths.root.join(CHEATS_FOLDER);
        [
            program.join("cheats.zip"),
            program.join(CHEATS_FOLDER),
            user.join("database.zip"),
            user.join("database"),
        ]
        .into_iter()
        .find(|path| path.exists())
    }

    /// Chooses the cheat database (an empty path goes back to the default places).
    pub fn set_cheat_database(&mut self, path: &str) {
        self.settings.set(SettingId::CheatsDatabase, path);
        self.cheats.database = None;
    }

    /// The sheet of the game in play in the cheat database, `Ok(None)` without a database or
    /// without a sheet for it; the error says what did not read.
    ///
    /// # Errors
    ///
    /// The database or its sheet did not read.
    pub fn cheat_sheet(&mut self) -> Result<Option<FoundSheet>, String> {
        // The database the player chose or put in place, else the one built in.
        let path = self.cheat_database_path();
        let shown = path.as_ref().map_or_else(
            || crate::cheat_database::BUILTIN_NAME.to_owned(),
            |path| path.display().to_string(),
        );
        if self
            .cheats
            .database
            .as_ref()
            .is_none_or(|(opened, _)| *opened != path)
        {
            let database = match &path {
                Some(path) => CheatDatabase::open(path),
                None => CheatDatabase::builtin(),
            }
            .map_err(|error| format!("{shown}: {error}"))?;
            self.cheats.database = Some((path, database));
        }
        let Some(rom) = self.current_rom_path().map(str::to_owned) else {
            return Ok(None);
        };
        let crc = self.cheats.crc;
        let (_, stem) = sheet_keys(&rom, None);
        let Some((_, database)) = self.cheats.database.as_ref() else {
            return Ok(None);
        };
        database
            .sheet_for(crc, &stem)
            .map_err(|error| format!("{shown}: {error}"))
    }
    /// A byte of the game's memory as the CPU reads it, for the RAM search; `None` without a
    /// game or while the core is away.
    #[must_use]
    pub fn read_game_memory(&self, address: u16) -> Option<u8> {
        self.session
            .as_ref()
            .filter(|session| session.has_rom() && session.core_is_home())
            .map(|session| session.core().read_memory(address))
    }
}

/// What the RAM search window asks of the host, carried out at the next tick, the core here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RamSearchAction {
    /// A new search: every address a candidate with its value now.
    Start {
        /// How values are read.
        size: ValueSize,
        /// Whether 8 and 16-bit values compare signed.
        signed: bool,
    },
    /// Keep the candidates whose value compares to the operand.
    Filter(Comparison, Operand),
    /// Take the last filter back.
    Undo,
    /// Rule one address out.
    Remove(u16),
    /// Drop the search.
    Clear,
}

/// How many candidates the window lists, their current values read every tick.
pub const SEARCH_ROWS: usize = 500;

/// What the RAM search window shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RamSearchView {
    /// Whether a search is under way.
    pub active: bool,
    /// How values are read.
    pub size: Option<ValueSize>,
    /// How many candidates are left.
    pub count: usize,
    /// The first [`SEARCH_ROWS`] candidates: address, value at the last step, value now.
    pub rows: Vec<(u16, i64, Option<i64>)>,
    /// Whether a filter can be taken back.
    pub can_undo: bool,
}

/// The search and what was asked of it.
#[derive(Debug, Default)]
pub struct RamSearchState {
    search: Option<RamSearch>,
    pending: Vec<RamSearchAction>,
    view: RamSearchView,
    undo_depth: usize,
}

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// Asks the RAM search for `action`, done at the next tick.
    pub fn ram_search(&mut self, action: RamSearchAction) {
        self.cheats.search.pending.push(action);
    }

    /// What the RAM search window shows, as of the last tick.
    #[must_use]
    pub const fn ram_search_view(&self) -> &RamSearchView {
        &self.cheats.search.view
    }

    /// The search's work of a tick, the core here: what was asked, then the values now.
    fn run_ram_search(&mut self) {
        let Some(session) = self
            .session
            .as_ref()
            .filter(|session| session.has_rom() && session.core_is_home())
        else {
            return;
        };
        let core = session.core();
        let read = |address| core.read_memory(address);
        let state = &mut self.cheats.search;
        for action in std::mem::take(&mut state.pending) {
            match action {
                RamSearchAction::Start { size, signed } => {
                    let mut search = RamSearch::new(&[CPU_RAM, WORK_RAM], size, signed);
                    search.start(read);
                    state.search = Some(search);
                    state.undo_depth = 0;
                }
                RamSearchAction::Filter(comparison, operand) => {
                    if let Some(search) = state.search.as_mut() {
                        search.filter(comparison, operand, read);
                        state.undo_depth += 1;
                    }
                }
                RamSearchAction::Undo => {
                    if state.search.as_mut().is_some_and(RamSearch::undo) {
                        state.undo_depth = state.undo_depth.saturating_sub(1);
                    }
                }
                RamSearchAction::Remove(address) => {
                    if let Some(search) = state.search.as_mut() {
                        search.remove(address);
                    }
                }
                RamSearchAction::Clear => state.search = None,
            }
        }
        state.view = match &state.search {
            None => RamSearchView::default(),
            Some(search) => {
                let candidates = search.candidates();
                let shown = &candidates[..candidates.len().min(SEARCH_ROWS)];
                let now = search.current_values_of(shown, read);
                RamSearchView {
                    active: true,
                    size: Some(search.size()),
                    count: candidates.len(),
                    rows: shown
                        .iter()
                        .zip(now)
                        .map(|(candidate, now)| (candidate.address, candidate.previous, now))
                        .collect(),
                    can_undo: state.undo_depth > 0,
                }
            }
        };
    }
}

/// The codes that set the value at `address` to `value` every frame, as `size` reads it: one
/// raw code for a byte or two BCD digits, two for a 16-bit value (low byte first). `None` when
/// the value does not fit.
#[must_use]
pub fn codes_for_value(address: u16, size: ValueSize, value: i64) -> Option<String> {
    let byte = |value: i64| u8::try_from(value.rem_euclid(256)).ok();
    match size {
        ValueSize::Byte => {
            (-128..=255).contains(&value).then_some(())?;
            Some(format!("{address:04X}:{:02X}", byte(value)?))
        }
        ValueSize::Bcd => {
            (0..=99).contains(&value).then_some(())?;
            let digits = u8::try_from(value / 10 * 16 + value % 10).ok()?;
            Some(format!("{address:04X}:{digits:02X}"))
        }
        ValueSize::Word => {
            (-32768..=65535).contains(&value).then_some(())?;
            let high = address.checked_add(1)?;
            Some(format!(
                "{address:04X}:{:02X}+{high:04X}:{:02X}",
                byte(value)?,
                byte(value.div_euclid(256))?
            ))
        }
    }
}

/// The first line of a description (the database's notes come under it).
#[must_use]
pub fn first_line(description: &str) -> &str {
    description.lines().next().unwrap_or_default().trim()
}

/// A cheat made of the codes typed by the player, on; named after its codes without a
/// description.
///
/// # Errors
///
/// The first code that does not read.
pub fn typed_cheat(description: &str, codes: &str) -> Result<Cheat, zax_cheats::CodeError> {
    let codes = zax_cheats::CheatCode::parse_list(codes)?;
    let description = if description.trim().is_empty() {
        codes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("+")
    } else {
        description.trim().to_owned()
    };
    Ok(Cheat {
        description,
        codes,
        enabled: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_found_value_becomes_codes() {
        assert_eq!(
            codes_for_value(0x075A, ValueSize::Byte, 9).as_deref(),
            Some("075A:09")
        );
        assert_eq!(
            codes_for_value(0x075A, ValueSize::Byte, -1).as_deref(),
            Some("075A:FF")
        );
        assert_eq!(codes_for_value(0x075A, ValueSize::Byte, 256), None);
        assert_eq!(
            codes_for_value(0x07DE, ValueSize::Bcd, 42).as_deref(),
            Some("07DE:42")
        );
        assert_eq!(codes_for_value(0x07DE, ValueSize::Bcd, 100), None);
        assert_eq!(
            codes_for_value(0x0300, ValueSize::Word, 0x1234).as_deref(),
            Some("0300:34+0301:12")
        );
        let cheat = typed_cheat("", "0300:34+0301:12").unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(cheat.codes.len(), 2);
        assert_eq!(cheat.description, "0300:34+0301:12");
    }

    #[test]
    fn a_game_in_an_archive_is_looked_up_by_its_entry() {
        let member = crate::emulation::rom_file::member_path(
            Path::new("roms/Castlevania.7z"),
            "Castlevania (U) (PRG1) [!].nes",
        );
        let (crc, stem) = sheet_keys(&member.to_string_lossy(), None);
        assert_eq!(crc, None);
        assert_eq!(stem, "Castlevania (U) (PRG1) [!]");
        assert_eq!(
            sheet_keys("roms/Mega Man 3 (USA).nes", None).1,
            "Mega Man 3 (USA)"
        );
    }
}
