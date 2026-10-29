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
use zax_nes::hack_pack;
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
    /// The patch of another ROM of the game applied all the same (forced), on or off.
    Force,
    /// The widescreen, on or off.
    Widescreen,
    /// Its ratio: automatic (the view's) or a fixed one.
    Ratio,
    /// The margin it captures beyond the view.
    Margin,
    /// The advanced mode, for a game whose patch is an advanced one: the patch applies.
    Advanced,
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
    /// The CRC-32 of its data, when the file could be read.
    crc: Option<u32>,
    /// The built-in patch made for another ROM of the same game, when no hack is made for this
    /// one (its profile or its name says the game).
    other_rom: Option<&'static BuiltinPatch>,
    /// Whether that patch can be forced on this ROM: its data has the size the patch was made
    /// for.
    forceable: bool,
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
        let capable = crc.is_some_and(builtin_patches::widescreen_capable);
        let other_rom = (!capable).then(|| other_rom_patch(None, &name)).flatten();
        Self {
            rom,
            name,
            capable,
            patch,
            crc,
            other_rom,
            forceable: true,
            selected: 0,
            global,
            own,
            patch_options: patch.or(other_rom).map(patch_options).unwrap_or_default(),
        }
    }

    /// The hacks of the game in `rom`, whose 3D profile is called `title` if it has one, as the
    /// host knows them.
    #[must_use]
    pub fn for_rom(host: &AppHost, rom: &Path, title: Option<&str>) -> Self {
        let crc = host.rom_data_crc(rom);
        host.save_rom_crcs();
        let mut hacks = Self::new(
            rom.to_path_buf(),
            crc,
            host.settings(),
            host.game_options(&rom.to_string_lossy()),
            |patch| host.patch_options(patch),
        );
        if !hacks.capable {
            // Not by its CRC: by the signatures of a hack pack (another version of the game, a ROM
            // hack), the hooks finding the bytes they replace.
            let image = std::fs::read(rom).ok();
            if let Some((patch, _)) = image.as_deref().and_then(builtin_patches::pack_for) {
                hacks.capable = true;
                hacks.patch = Some(patch);
                hacks.patch_options = host.patch_options(patch);
                return hacks;
            }
            hacks.other_rom = other_rom_patch(title, &hacks.name);
            if let Some(other) = hacks.other_rom {
                // A forced patch applies to data of the size it was made for only; a hack pack is
                // never forced (its signatures decide, above).
                let size = image.map(|image| builtin_patches::rom_data(&image).len());
                hacks.forceable = !hack_pack::is_pack(other.bps)
                    && size.is_some()
                    && size == builtin_patches::source_size(other);
                hacks.patch_options = host.patch_options(other);
            }
        }
        hacks
    }

    /// Whether the patch of another ROM of the game is forced on this one.
    #[must_use]
    pub fn forced(&self) -> bool {
        self.other_rom
            .is_some_and(|other| self.value(SettingId::EmuForcedPatch).as_text() == other.name)
    }

    /// The built-in patch made for another ROM of the game, when this one is such a version.
    #[must_use]
    pub const fn other_rom_patch(&self) -> Option<&'static BuiltinPatch> {
        self.other_rom
    }

    /// Whether the patch of another ROM of the game can be forced on this one.
    #[must_use]
    pub const fn forceable(&self) -> bool {
        self.other_rom.is_some() && self.forceable
    }

    /// Forces the patch of another ROM of the game on this one, the widescreen on (Auto), or
    /// takes both back.
    pub fn toggle_force(&mut self) {
        let Some(other) = self.other_rom.filter(|_| self.forceable) else {
            return;
        };
        if self.forced() {
            self.set(SettingId::EmuForcedPatch, SettingValue::from(""));
            self.set(SettingId::VideoWidescreen, SettingValue::Enum(0));
        } else {
            self.set(SettingId::EmuForcedPatch, SettingValue::from(other.name));
            if !self.widescreen_on() {
                self.set(SettingId::VideoWidescreen, SettingValue::Enum(RATIOS[0]));
            }
        }
    }

    /// Why the widescreen cannot be turned on, for a version of a game the hack is made for
    /// another ROM of: the ROM it is made for and its CRC, this one's name and CRC.
    #[must_use]
    pub fn other_rom_message(&self) -> Option<String> {
        let patch = self.other_rom?;
        let file = self.rom.file_name().map_or_else(
            || self.name.clone(),
            |name| name.to_string_lossy().into_owned(),
        );
        let crc = self
            .crc
            .map_or_else(|| "?".to_owned(), |crc| format!("{crc:08X}"));
        // Once forced, what is forced rather than why it cannot be.
        let template = if self.forced() {
            keys::HACKS_FORCED_NOTE
        } else {
            keys::HACKS_OTHER_ROM
        };
        Some(
            strings::format(
                &strings::get(template),
                &[
                    patch.game,
                    patch.rom_name,
                    &format!("{:08X}", patch.source_crc),
                    &file,
                    &crc,
                ],
            ) + &if self.forceable {
                String::new()
            } else if hack_pack::is_pack(patch.bps) {
                format!(" {}", strings::get(keys::HACKS_PACK_SIGNATURES))
            } else {
                format!(" {}", strings::get(keys::HACKS_FORCE_SIZE))
            },
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

    /// The built-in patch the game plays with: the one made for it, or the one forced on it.
    #[must_use]
    pub fn patch(&self) -> Option<&'static BuiltinPatch> {
        self.patch
            .or_else(|| self.other_rom.filter(|_| self.forced()))
    }

    /// The rows: none when no hack is made for the game; for another version of a game one is
    /// made for, the forcing first, then the rest once it is forced.
    #[must_use]
    pub fn rows(&self) -> Vec<HackRow> {
        let mut rows = Vec::new();
        if !self.capable {
            if !self.forceable() {
                return rows;
            }
            rows.push(HackRow::Force);
            if !self.forced() {
                return rows;
            }
        }
        rows.extend([HackRow::Widescreen, HackRow::Ratio, HackRow::Margin]);
        let advanced_patch = self.patch().is_some_and(|patch| patch.advanced);
        if advanced_patch {
            rows.push(HackRow::Advanced);
        }
        // The options of an advanced patch are those of its mode, but for the aids of its hack
        // pack, which apply without it.
        let all = !advanced_patch || self.advanced_on();
        let patch = self.patch();
        rows.extend(
            self.patch_options
                .iter()
                .enumerate()
                .filter(|(_, (option, _))| {
                    all || patch.is_some_and(|patch| patch.is_aid(option.key))
                })
                .map(|(index, _)| HackRow::PatchOption(index)),
        );
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

    /// Whether the advanced mode of the widescreen is on for the game.
    #[must_use]
    pub fn advanced_on(&self) -> bool {
        self.value(SettingId::VideoWidescreenAdvanced).as_bool()
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
            HackRow::Force => self.toggle_force(),
            HackRow::Widescreen => self.toggle_widescreen(),
            HackRow::Advanced => {
                let on = !self.advanced_on();
                self.set(SettingId::VideoWidescreenAdvanced, SettingValue::Bool(on));
                // The advanced mode is a mode of the widescreen.
                if on && !self.widescreen_on() {
                    self.set(SettingId::VideoWidescreen, SettingValue::Enum(RATIOS[0]));
                }
            }
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
            HackRow::Force | HackRow::Widescreen | HackRow::Advanced | HackRow::PatchOption(_) => {
                Vec::new()
            }
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
            HackRow::Force | HackRow::Widescreen | HackRow::Advanced | HackRow::PatchOption(_) => 0,
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
            HackRow::Force | HackRow::Widescreen | HackRow::Advanced | HackRow::PatchOption(_) => {}
        }
    }

    /// Whether `row` is on, for a switch.
    #[must_use]
    pub fn is_on(&self, row: HackRow) -> bool {
        match row {
            HackRow::Force => self.forced(),
            HackRow::Widescreen => self.widescreen_on(),
            HackRow::Advanced => self.advanced_on(),
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
        let Some(patch) = self.patch() else {
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
            HackRow::Force => strings::get(keys::HACK_FORCE),
            HackRow::Widescreen => strings::get(keys::HACK_WIDESCREEN),
            HackRow::Ratio => strings::get(keys::HACK_RATIO),
            HackRow::Margin => strings::get(keys::HACK_MARGIN),
            HackRow::Advanced => strings::get(keys::HACK_ADVANCED),
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
            HackRow::Force => switch(self.forced()),
            HackRow::Widescreen => switch(self.widescreen_on()),
            HackRow::Advanced => switch(self.advanced_on()),
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
            HackRow::Force => SettingId::EmuForcedPatch,
            HackRow::Widescreen | HackRow::Ratio => SettingId::VideoWidescreen,
            HackRow::Margin => SettingId::VideoWidescreenExtra,
            HackRow::Advanced => SettingId::VideoWidescreenAdvanced,
            HackRow::PatchOption(_) => return false,
        };
        self.own.iter().any(|(own, _)| *own == id)
    }
}

/// The built-in patch of the game a ROM is a version of, by the title of its 3D profile or else by
/// its name: what tells a ROM the hack is not made for from a game that has none.
#[must_use]
pub fn other_rom_patch(title: Option<&str>, name: &str) -> Option<&'static BuiltinPatch> {
    title
        .and_then(builtin_patches::for_game_name)
        .or_else(|| builtin_patches::for_game_name(name))
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
    fn another_rom_of_a_game_a_hack_is_made_for_says_which_one_it_needs() {
        let hacks = HacksModel::new(
            PathBuf::from("/roms/Zelda II - The Adventure of Link (Europe).nes"),
            Some(0x0123_ABCD),
            &SettingsModel::new(),
            Vec::new(),
            |_| Vec::new(),
        );
        // Only the forcing until the patch is forced.
        assert_eq!([HackRow::Force].as_slice(), hacks.rows().as_slice());
        let message = hacks.other_rom_message().unwrap();
        for part in [
            "Zelda II - The Adventure of Link (USA)",
            "BA322865",
            "Zelda II - The Adventure of Link (Europe).nes",
            "0123ABCD",
        ] {
            assert!(message.contains(part), "{part} in {message}");
        }
        assert_eq!(
            None,
            model(Some(0x1234_5678), Vec::new()).other_rom_message()
        );
    }

    #[test]
    fn the_patch_of_another_rom_can_be_forced_and_taken_back() {
        let mut hacks = HacksModel::new(
            PathBuf::from("/roms/Zelda II - The Adventure of Link (Europe).nes"),
            Some(0x0123_ABCD),
            &SettingsModel::new(),
            Vec::new(),
            |_| Vec::new(),
        );
        assert_eq!([HackRow::Force].as_slice(), hacks.rows().as_slice());
        assert!(hacks.patch().is_none());
        hacks.step(HackRow::Force, 1);
        assert!(hacks.forced());
        assert!(hacks.widescreen_on());
        assert_eq!(
            Some(builtin_patches::ZELDA2_USA),
            hacks.patch().map(|patch| patch.name)
        );
        assert_eq!(
            [
                HackRow::Force,
                HackRow::Widescreen,
                HackRow::Ratio,
                HackRow::Margin,
                HackRow::Advanced
            ]
            .as_slice(),
            hacks.rows().as_slice()
        );
        hacks.step(HackRow::Force, 1);
        assert!(!hacks.forced());
        assert_eq!(0, hacks.own().len(), "nothing of its own is left");
    }

    #[test]
    fn the_advanced_mode_is_on_by_default_and_only_off_is_kept() {
        const SMB3: u32 = 0x2E63_01ED;
        let mut hacks = model(Some(SMB3), Vec::new());
        assert!(hacks.advanced_on());
        assert!(hacks.rows().contains(&HackRow::Advanced));
        assert_eq!(0, hacks.own().len());
        hacks.step(HackRow::Advanced, 1);
        assert!(!hacks.advanced_on());
        assert_eq!(
            [(
                SettingId::VideoWidescreenAdvanced,
                SettingValue::Bool(false)
            )]
            .as_slice(),
            hacks.own().as_slice()
        );
        // On again is the default; the widescreen comes on with it.
        hacks.step(HackRow::Advanced, 1);
        assert!(hacks.advanced_on());
        assert_eq!(
            [(SettingId::VideoWidescreen, SettingValue::Enum(1))].as_slice(),
            hacks.own().as_slice()
        );
    }

    #[test]
    fn the_aids_of_a_hack_pack_stay_without_the_advanced_mode() {
        const MEGA_MAN_6: u32 = 0x9887_98A8;
        let mut hacks = model(Some(MEGA_MAN_6), Vec::new());
        assert_eq!(
            hacks.patch_options().len(),
            hacks
                .rows()
                .iter()
                .filter(|row| matches!(row, HackRow::PatchOption(_)))
                .count()
        );
        hacks.step(HackRow::Advanced, 1);
        assert!(!hacks.advanced_on());
        // "Invincible" is a component of the pack of its own; the others are of the mode.
        let options: Vec<_> = hacks
            .rows()
            .into_iter()
            .filter_map(|row| match row {
                HackRow::PatchOption(index) => Some(hacks.patch_options()[index].0.key),
                _ => None,
            })
            .collect();
        assert_eq!(["invincible"].as_slice(), options.as_slice());
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
                HackRow::Advanced,
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
        assert_eq!(6, hacks.selected_index(), "wraps round");
        assert_eq!(Some(HacksCommand::Close), hacks.handle(PadButton::B));
    }
}
