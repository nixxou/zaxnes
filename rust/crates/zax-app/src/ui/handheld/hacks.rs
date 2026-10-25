//! The hacks of one game as data (Rust-only, owner request): the widescreen, with its ratio, its
//! margin and the options of the game's built-in patch. The hacks screen of the console mode and
//! the hacks window of the desktop both show it; it draws nothing.
//!
//! The widescreen, its ratio and its margin are options of the game (kept in `games.json` with
//! the others, only when they differ from the global values); the options of the patch are kept
//! per patch in `emu.hack_options`. The ROM patch itself follows the widescreen when the game
//! starts: on, the patched ROM; off, the ROM as it was made.

use std::path::{Path, PathBuf};

use zax_nes::builtin_patches::{self, BuiltinPatch, HackOption};
use zax_settings::catalogue::{self, SettingId};
use zax_settings::games::GameOptions;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::pad::PadButton;
use super::rom_list::RomEntry;
use crate::application::AppHost;
use crate::ui::{keys, strings, text};

/// A row of the hacks of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HackRow {
    /// The widescreen, on or off.
    Widescreen,
    /// Its ratio: automatic (the view's) or a fixed one.
    Ratio,
    /// The margin it captures beyond the view.
    Margin,
    /// Option n of the game's built-in patch.
    PatchOption(usize),
}

/// The ratios of the widescreen row when it is on (`video.widescreen`): automatic, 3:2, 16:10,
/// 16:9.
const RATIOS: [i32; 4] = [1, 2, 3, 4];

/// What the screen asks of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HacksCommand {
    /// A value changed: write the hacks down.
    Save,
    /// Leave the screen.
    Close,
}

/// The hacks of one game and the selection.
#[derive(Debug, Clone)]
pub struct HacksModel {
    rom: PathBuf,
    name: String,
    /// Whether a hack is made for the game.
    capable: bool,
    /// The built-in patch made for it, if any.
    patch: Option<&'static BuiltinPatch>,
    selected: usize,
    /// The global values: the preferences, without the scripts' or a game's.
    global: SettingsModel,
    /// The options the game has of its own, the hacks' among them.
    own: GameOptions,
    /// The options of the patch, each with whether it is on.
    patch_options: Vec<(HackOption, bool)>,
}

impl HacksModel {
    /// The hacks of the game in `rom`, whose data has CRC-32 `crc` (none when the file could
    /// not be read), over the global values of `settings`.
    #[must_use]
    pub fn new(
        rom: PathBuf,
        crc: Option<u32>,
        settings: &SettingsModel,
        own: GameOptions,
        patch_options: impl Fn(&BuiltinPatch) -> Vec<(HackOption, bool)>,
    ) -> Self {
        let name = RomEntry::new(rom.clone()).name;
        let patch = crc.and_then(builtin_patches::for_data_crc);
        let mut global = settings.clone();
        global.clear_session_layer();
        global.clear_game_values();
        global.drain_changes().for_each(drop);
        Self {
            rom,
            name,
            capable: crc.is_some_and(builtin_patches::widescreen_capable),
            patch,
            selected: 0,
            global,
            own,
            patch_options: patch.map(patch_options).unwrap_or_default(),
        }
    }

    /// The hacks of the game in `rom`, as the host knows them.
    #[must_use]
    pub fn for_rom(host: &AppHost, rom: &Path) -> Self {
        let crc = host.rom_data_crc(rom);
        host.save_rom_crcs();
        Self::new(
            rom.to_path_buf(),
            crc,
            host.settings(),
            host.game_options(&rom.to_string_lossy()),
            |patch| host.patch_options(patch),
        )
    }

    /// The game's file.
    #[must_use]
    pub fn rom(&self) -> &Path {
        &self.rom
    }

    /// The game's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether a hack is made for the game.
    #[must_use]
    pub const fn is_capable(&self) -> bool {
        self.capable
    }

    /// The built-in patch made for the game, if any.
    #[must_use]
    pub const fn patch(&self) -> Option<&'static BuiltinPatch> {
        self.patch
    }

    /// The rows: none when no hack is made for the game.
    #[must_use]
    pub fn rows(&self) -> Vec<HackRow> {
        if !self.capable {
            return Vec::new();
        }
        let mut rows = vec![HackRow::Widescreen, HackRow::Ratio, HackRow::Margin];
        rows.extend((0..self.patch_options.len()).map(HackRow::PatchOption));
        rows
    }

    /// The index of the selected row.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// The options the game has of its own, the hacks' among them.
    #[must_use]
    pub fn own(&self) -> &GameOptions {
        &self.own
    }

    /// The options of the patch, each with whether it is on.
    #[must_use]
    pub fn patch_options(&self) -> &[(HackOption, bool)] {
        &self.patch_options
    }

    /// The value of `id` for the game: its own, or the global one.
    #[must_use]
    pub fn value(&self, id: SettingId) -> SettingValue {
        self.own
            .iter()
            .find(|(own, _)| *own == id)
            .map_or_else(|| self.global.get(id), |(_, value)| value.clone())
    }

    /// Whether the widescreen is on for the game.
    #[must_use]
    pub fn widescreen_on(&self) -> bool {
        self.value(SettingId::VideoWidescreen).as_int() != 0
    }

    /// Gives the game `value` for `id`; the global value is not kept as its own.
    fn set(&mut self, id: SettingId, value: SettingValue) {
        self.own.retain(|(own, _)| *own != id);
        if value != self.global.get(id) {
            self.own.push((id, value));
        }
    }

    /// Turns the widescreen on (the automatic ratio) or off.
    pub fn toggle_widescreen(&mut self) {
        let on = self.widescreen_on();
        self.set(
            SettingId::VideoWidescreen,
            SettingValue::Enum(if on { 0 } else { RATIOS[0] }),
        );
    }

    /// Steps `row` by `step`.
    pub fn step(&mut self, row: HackRow, step: i32) {
        match row {
            HackRow::Widescreen => self.toggle_widescreen(),
            HackRow::Ratio => {
                // From off, the first step turns the widescreen on.
                let current = self.value(SettingId::VideoWidescreen).as_int();
                let next = if current == 0 {
                    RATIOS[0]
                } else {
                    cycled(&RATIOS, current, step)
                };
                self.set(SettingId::VideoWidescreen, SettingValue::Enum(next));
            }
            HackRow::Margin => {
                let count = choices(SettingId::VideoWidescreenExtra).len();
                let values: Vec<i32> = (0..i32::try_from(count).unwrap_or(0)).collect();
                let current = self.value(SettingId::VideoWidescreenExtra).as_int();
                let next = cycled(&values, current, step);
                self.set(SettingId::VideoWidescreenExtra, SettingValue::Enum(next));
            }
            HackRow::PatchOption(index) => {
                if let Some((_, on)) = self.patch_options.get_mut(index) {
                    *on = !*on;
                }
            }
        }
    }

    /// The values `row` can take, as a list shows them (the console steps through them
    /// instead); empty for a switch.
    #[must_use]
    pub fn choices(&self, row: HackRow) -> Vec<String> {
        match row {
            HackRow::Ratio => RATIOS
                .iter()
                .map(|ratio| choice_label(SettingId::VideoWidescreen, *ratio))
                .collect(),
            HackRow::Margin => choices(SettingId::VideoWidescreenExtra)
                .iter()
                .map(|label| text::label_of_option(label))
                .collect(),
            HackRow::Widescreen | HackRow::PatchOption(_) => Vec::new(),
        }
    }

    /// The index in [`Self::choices`] of the value of `row`.
    #[must_use]
    pub fn choice_index(&self, row: HackRow) -> usize {
        match row {
            HackRow::Ratio => {
                let current = self.value(SettingId::VideoWidescreen).as_int();
                RATIOS
                    .iter()
                    .position(|ratio| *ratio == current)
                    .unwrap_or(0)
            }
            HackRow::Margin => {
                usize::try_from(self.value(SettingId::VideoWidescreenExtra).as_int()).unwrap_or(0)
            }
            HackRow::Widescreen | HackRow::PatchOption(_) => 0,
        }
    }

    /// Gives `row` the value `index` of [`Self::choices`]; a ratio turns the widescreen on.
    pub fn choose(&mut self, row: HackRow, index: usize) {
        match row {
            HackRow::Ratio => {
                if let Some(ratio) = RATIOS.get(index) {
                    self.set(SettingId::VideoWidescreen, SettingValue::Enum(*ratio));
                }
            }
            HackRow::Margin => {
                if let Ok(value) = i32::try_from(index) {
                    self.set(SettingId::VideoWidescreenExtra, SettingValue::Enum(value));
                }
            }
            HackRow::Widescreen | HackRow::PatchOption(_) => {}
        }
    }

    /// Whether `row` is on, for a switch.
    #[must_use]
    pub fn is_on(&self, row: HackRow) -> bool {
        match row {
            HackRow::Widescreen => self.widescreen_on(),
            HackRow::PatchOption(index) => self.patch_options.get(index).is_some_and(|(_, on)| *on),
            HackRow::Ratio | HackRow::Margin => false,
        }
    }

    /// One press of a button.
    pub fn handle(&mut self, button: PadButton) -> Option<HacksCommand> {
        let rows = self.rows();
        let count = rows.len();
        match button {
            PadButton::Up if count > 0 => self.selected = (self.selected + count - 1) % count,
            PadButton::Down if count > 0 => self.selected = (self.selected + 1) % count,
            PadButton::Left | PadButton::Right | PadButton::A => {
                let row = *rows.get(self.selected)?;
                self.step(row, if button == PadButton::Left { -1 } else { 1 });
                return Some(HacksCommand::Save);
            }
            PadButton::B => return Some(HacksCommand::Close),
            _ => {}
        }
        None
    }

    /// Writes the hacks down: the options of the game (in force at once when it plays) and the
    /// options of its patch.
    pub fn apply(&self, host: &mut AppHost) {
        host.set_game_options(&self.rom.to_string_lossy(), self.own.clone());
        let Some(patch) = self.patch else {
            return;
        };
        let stored = host.patch_options(patch);
        for ((option, on), (_, was)) in self.patch_options.iter().zip(stored) {
            if *on != was {
                host.set_patch_option(patch, option.key, *on);
            }
        }
    }

    /// The label of `row`, in the language in force.
    #[must_use]
    pub fn label(&self, row: HackRow) -> String {
        match row {
            HackRow::Widescreen => strings::get(keys::HACK_WIDESCREEN),
            HackRow::Ratio => strings::get(keys::HACK_RATIO),
            HackRow::Margin => strings::get(keys::HACK_MARGIN),
            HackRow::PatchOption(index) => self
                .patch_options
                .get(index)
                .map(|(option, _)| {
                    if strings::language() == strings::UiLanguage::French {
                        option.label_fr
                    } else {
                        option.label
                    }
                })
                .unwrap_or_default()
                .to_owned(),
        }
    }

    /// The value of `row` as the screen shows it.
    #[must_use]
    pub fn value_text(&self, row: HackRow) -> String {
        let switch = |on: bool| strings::get(if on { keys::HACK_ON } else { keys::HACK_OFF });
        match row {
            HackRow::Widescreen => switch(self.widescreen_on()),
            HackRow::Ratio => {
                let ratio = self
                    .value(SettingId::VideoWidescreen)
                    .as_int()
                    .max(RATIOS[0]);
                choice_label(SettingId::VideoWidescreen, ratio)
            }
            HackRow::Margin => choice_label(
                SettingId::VideoWidescreenExtra,
                self.value(SettingId::VideoWidescreenExtra).as_int(),
            ),
            HackRow::PatchOption(index) => {
                switch(self.patch_options.get(index).is_some_and(|(_, on)| *on))
            }
        }
    }

    /// Whether `row` has a value the game holds of its own rather than the global one.
    #[must_use]
    pub fn is_own(&self, row: HackRow) -> bool {
        let id = match row {
            HackRow::Widescreen | HackRow::Ratio => SettingId::VideoWidescreen,
            HackRow::Margin => SettingId::VideoWidescreenExtra,
            HackRow::PatchOption(_) => return false,
        };
        self.own.iter().any(|(own, _)| *own == id)
    }
}

/// The labels of the choices of `id`.
fn choices(id: SettingId) -> Vec<String> {
    catalogue::descriptor(id)
        .domain()
        .options()
        .map_or_else(Vec::new, |options| {
            options.iter().map(|option| option.label.clone()).collect()
        })
}

/// The label of the choice `value` of `id`, in the language in force.
fn choice_label(id: SettingId, value: i32) -> String {
    catalogue::descriptor(id)
        .domain()
        .options()
        .and_then(|options| {
            options
                .iter()
                .find(|option| (option.value - f64::from(value)).abs() < 1e-9)
        })
        .map_or_else(
            || value.to_string(),
            |option| text::label_of_option(&option.label),
        )
}

/// The value after `current`, `step` places on, wrapping round; the first when `current` is not
/// in the list.
fn cycled(values: &[i32], current: i32, step: i32) -> i32 {
    let count = i32::try_from(values.len()).unwrap_or(1).max(1);
    let here = values
        .iter()
        .position(|value| *value == current)
        .and_then(|index| i32::try_from(index).ok())
        .unwrap_or(0);
    let next = usize::try_from((here + step).rem_euclid(count)).unwrap_or(0);
    values.get(next).copied().unwrap_or(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEGA_MAN_2: u32 = 0x0FCF_C04D;

    fn model(crc: Option<u32>, own: GameOptions) -> HacksModel {
        HacksModel::new(
            PathBuf::from("/roms/Game.nes"),
            crc,
            &SettingsModel::new(),
            own,
            |patch| {
                patch
                    .options
                    .iter()
                    .map(|option| (*option, option.default))
                    .collect()
            },
        )
    }

    #[test]
    fn a_game_no_hack_is_made_for_has_no_rows() {
        assert_eq!(0, model(Some(0x1234_5678), Vec::new()).rows().len());
        assert_eq!(0, model(None, Vec::new()).rows().len());
    }

    #[test]
    fn the_widescreen_is_off_by_default_and_turns_on_with_the_automatic_ratio() {
        let mut hacks = model(Some(MEGA_MAN_2), Vec::new());
        assert_eq!(
            [
                HackRow::Widescreen,
                HackRow::Ratio,
                HackRow::Margin,
                HackRow::PatchOption(0),
                HackRow::PatchOption(1),
                HackRow::PatchOption(2),
            ]
            .as_slice(),
            hacks.rows().as_slice()
        );
        assert!(!hacks.widescreen_on());
        assert_eq!(0, hacks.own().len());
        hacks.toggle_widescreen();
        assert!(hacks.widescreen_on());
        assert_eq!(
            [(SettingId::VideoWidescreen, SettingValue::Enum(1))].as_slice(),
            hacks.own().as_slice()
        );
        // Off again is the global value: nothing of its own is kept.
        hacks.toggle_widescreen();
        assert_eq!(0, hacks.own().len());
    }

    #[test]
    fn the_ratio_and_the_margin_step_and_only_changes_are_kept() {
        let mut hacks = model(Some(MEGA_MAN_2), Vec::new());
        // From off, the ratio turns the widescreen on.
        hacks.step(HackRow::Ratio, 1);
        assert_eq!(1, hacks.value(SettingId::VideoWidescreen).as_int());
        hacks.step(HackRow::Ratio, -1);
        assert_eq!(4, hacks.value(SettingId::VideoWidescreen).as_int(), "16:9");
        // +20 % by default; a step up is +25 %, back down the global value.
        assert_eq!(4, hacks.value(SettingId::VideoWidescreenExtra).as_int());
        hacks.step(HackRow::Margin, 1);
        assert_eq!(5, hacks.value(SettingId::VideoWidescreenExtra).as_int());
        assert!(hacks.is_own(HackRow::Margin));
        hacks.step(HackRow::Margin, -1);
        assert!(!hacks.is_own(HackRow::Margin));
        // An option of the patch flips.
        let invincible = hacks.patch_options()[1].1;
        hacks.step(HackRow::PatchOption(1), 1);
        assert_eq!(!invincible, hacks.patch_options()[1].1);
    }

    #[test]
    fn other_options_of_the_game_are_kept() {
        let mut hacks = model(
            Some(MEGA_MAN_2),
            vec![(SettingId::RenderShadow, SettingValue::Bool(false))],
        );
        hacks.toggle_widescreen();
        assert_eq!(2, hacks.own().len());
        assert!(hacks
            .own()
            .contains(&(SettingId::RenderShadow, SettingValue::Bool(false))));
    }

    #[test]
    fn the_buttons_move_change_and_leave() {
        let mut hacks = model(Some(MEGA_MAN_2), Vec::new());
        assert_eq!(Some(HacksCommand::Save), hacks.handle(PadButton::A));
        assert!(hacks.widescreen_on());
        assert_eq!(None, hacks.handle(PadButton::Down));
        assert_eq!(1, hacks.selected_index());
        assert_eq!(None, hacks.handle(PadButton::Up));
        assert_eq!(None, hacks.handle(PadButton::Up));
        assert_eq!(5, hacks.selected_index(), "wraps round");
        assert_eq!(Some(HacksCommand::Close), hacks.handle(PadButton::B));
    }
}
