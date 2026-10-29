//! The hacks of a game (Rust-only, owner request): the ROM files a hack is made for, told by the
//! CRC of their data (kept in a cache of the user data), and the options of a built-in patch,
//! whether its game plays or not.

use std::path::Path;

use zax_nes::builtin_patches::{self, BuiltinPatch, HackOption};
use zax_settings::catalogue::{self, SettingId};
use zax_settings::values::SettingValue;

use super::hack_options::{is_on, with_option};
use super::{EmulationHost, GameView};
use crate::emulation::game_catalog::GameCatalog;
use crate::persistence::hack_compat_cache::Certificate;

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// The CRC-32 of the data of the ROM in `path` (iNES header excluded), read from the file
    /// only when the cache has not seen it as it is; none when it cannot be read.
    #[must_use]
    pub fn rom_data_crc(&self, path: &Path) -> Option<u32> {
        self.rom_crcs.borrow_mut().crc_of(path)
    }

    /// The CRC-32 the cache keeps for the ROM in `path`, without reading the file: none when
    /// the cache has not seen it as it is.
    #[must_use]
    pub fn known_rom_crc(&self, path: &Path) -> Option<u32> {
        self.rom_crcs.borrow_mut().known(path)
    }

    /// Writes the cache of the ROM CRCs when it learned something.
    pub fn save_rom_crcs(&self) {
        let written = self.rom_crcs.borrow_mut().save();
        if let Err(error) = written {
            self.log(&format!("the ROM CRC cache could not be written: {error}"));
        }
    }

    /// What the study of the ROM in `path` found, while the file is as it was then; none when
    /// it was not studied as it is (the list of the ROMs reads no file).
    #[must_use]
    pub fn rom_certificate(&self, path: &Path) -> Option<Certificate> {
        self.hack_compat.borrow_mut().certificate(path)
    }

    /// Studies the ROM in `path`, whose image is `image`: whether a widescreen hack fits it, by
    /// the CRC of its data or by the signatures of a hack pack. What it finds is kept, with the
    /// CRC, for the list of the ROMs; gives the CRC.
    pub fn certify_rom(&self, path: &Path, image: &[u8]) -> u32 {
        let crc = builtin_patches::data_crc(image);
        self.rom_crcs.borrow_mut().remember(path, crc);
        self.save_rom_crcs();
        let certificate = if builtin_patches::widescreen_capable(crc) {
            Certificate::Fits {
                patch: builtin_patches::for_data_crc(crc).map(|patch| patch.name.to_owned()),
                whole: true,
            }
        } else if let Some((patch, applied)) = builtin_patches::pack_for(image) {
            Certificate::Fits {
                patch: Some(patch.name.to_owned()),
                whole: applied.refused.is_empty(),
            }
        } else {
            Certificate::Unfit
        };
        let mut cache = self.hack_compat.borrow_mut();
        cache.certify(path, certificate);
        if let Err(error) = cache.save() {
            self.log(&format!(
                "the hack study cache could not be written: {error}"
            ));
        }
        crc
    }

    /// Whether the built-in patch of a ROM applies when it loads: the patches are allowed and
    /// the widescreen of the game is on (owner request: off, the game starts as it was made).
    #[must_use]
    pub fn builtin_patches_wanted(&self) -> bool {
        self.settings.get(SettingId::EmuBuiltinPatches).as_bool()
            && self.settings.get(SettingId::VideoWidescreen).as_int() != 0
    }

    /// Whether the advanced mode of the game's widescreen is on (its patch, when the patch is an
    /// advanced one, applies only then).
    #[must_use]
    pub fn advanced_patches_wanted(&self) -> bool {
        self.settings
            .get(SettingId::VideoWidescreenAdvanced)
            .as_bool()
    }

    /// Whether `patch` applies to its ROM when it loads, as the game's hacks are set.
    #[must_use]
    pub fn patch_applies(&self, patch: &BuiltinPatch) -> bool {
        self.builtin_patches_wanted() && (!patch.advanced || self.advanced_patches_wanted())
    }

    /// The built-in patch forced on the game in play (another version of the game a patch is
    /// made for), when it is.
    #[must_use]
    pub fn forced_patch(&self) -> Option<&'static BuiltinPatch> {
        zax_nes::builtin_patches::by_name(self.settings.get(SettingId::EmuForcedPatch).as_text())
    }

    /// Forces `patch` on the ROM in `rom` (an option of that game), its widescreen on (Auto)
    /// unless it already is.
    pub fn force_patch(&mut self, rom: &str, patch: &str) {
        let mut options = self.game_options(rom);
        options.retain(|(id, _)| *id != SettingId::EmuForcedPatch);
        options.push((SettingId::EmuForcedPatch, SettingValue::from(patch)));
        let on = options
            .iter()
            .any(|(id, value)| *id == SettingId::VideoWidescreen && value.as_int() != 0);
        if !on {
            options.retain(|(id, _)| *id != SettingId::VideoWidescreen);
            options.push((SettingId::VideoWidescreen, SettingValue::Enum(1)));
        }
        self.log(&format!("{patch} forced on {rom}"));
        self.set_game_options(rom, options);
    }

    /// Turns the widescreen of the game in play on (the automatic ratio, unless it already has
    /// one) or off, as an option of that game.
    pub(super) fn set_game_widescreen(&mut self, on: bool) {
        let Some(rom) = self.game_options_rom.clone() else {
            return;
        };
        let current = self.settings.get(SettingId::VideoWidescreen).as_int();
        if (current != 0) == on {
            return;
        }
        let mut options = self.game_options(&rom);
        options.retain(|(id, _)| *id != SettingId::VideoWidescreen);
        let mut global = self.settings.clone();
        global.clear_session_layer();
        global.clear_game_values();
        let wanted = SettingValue::Enum(i32::from(on));
        if global.get(SettingId::VideoWidescreen) != wanted {
            options.push((SettingId::VideoWidescreen, wanted));
        }
        self.set_game_options(&rom, options);
    }

    /// Turns the advanced mode of the widescreen of the game in play on or off.
    pub(super) fn set_game_advanced(&mut self, on: bool) {
        let Some(rom) = self.game_options_rom.clone() else {
            return;
        };
        let mut options = self.game_options(&rom);
        options.retain(|(id, _)| *id != SettingId::VideoWidescreenAdvanced);
        // The game keeps a value of its own only where it is not the default.
        let value = SettingValue::Bool(on);
        if &value != catalogue::descriptor(SettingId::VideoWidescreenAdvanced).default_value() {
            options.push((SettingId::VideoWidescreenAdvanced, value));
        }
        self.set_game_options(&rom, options);
    }

    /// The options of `patch`, each with whether it is on.
    #[must_use]
    pub fn patch_options(&self, patch: &BuiltinPatch) -> Vec<(HackOption, bool)> {
        let stored = self.settings.get(SettingId::EmuHackOptions);
        patch
            .options
            .iter()
            .map(|option| (*option, is_on(stored.as_text(), patch.name, option)))
            .collect()
    }

    /// Turns option `key` of `patch` on or off; it is told to the core at the next frame when
    /// that game plays.
    pub fn set_patch_option(&mut self, patch: &BuiltinPatch, key: &str, on: bool) {
        let text = with_option(
            self.settings.get(SettingId::EmuHackOptions).as_text(),
            patch.name,
            key,
            on,
        );
        self.update_settings(|model| {
            model.set(SettingId::EmuHackOptions, SettingValue::Text(text))
        });
    }
}
