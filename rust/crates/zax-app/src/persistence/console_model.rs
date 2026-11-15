//! Console models of the menu bar (lot FIDPLUS-1, `docs/review/FIDELITY-LEADS.md` G1).
//!
//! A model is a shortcut: choosing one sets, in one go, the region and the revision options the
//! console implies (`emu.tv_mode`, `emu.ppu_revision`, `emu.oamaddr_bug`) and turns the region's
//! automatic detection off. Nothing of its own is persisted: the menu ticks the model whose values
//! all match the settings in force, so changing one option by hand simply leaves no model ticked.
//!
//! What each model sets is only what a reference names. nesdev ("PPU registers"): palette reads
//! are "supported by the 2C02G, 2C02H, and PAL PPUs"; the OAMADDR defect at the start of rendering
//! is the 2C02G/H's. The clones (Dendy, PAL-M, PAL-N) set their region only: no reference
//! documents their PPU revisions, so the revision options stay as they are.

use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::app_settings::{AppSettings, AppSettingsMut, TvRegionSetting};

/// `emu.ppu_revision`: 2C02G (or later; also the PAL 2C07 for palette reads).
const REVISION_G: i32 = 0;
/// `emu.ppu_revision`: a 2C02 before G.
const REVISION_BEFORE_G: i32 = 1;

/// One console model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleModel {
    /// NTSC NES or Famicom with a 2C02G or 2C02H.
    Ntsc2C02G = 0,
    /// NTSC NES or Famicom with a 2C02 before G.
    Ntsc2C02BeforeG = 1,
    /// PAL NES (2C07).
    Pal2C07 = 2,
    /// Dendy and the other 50 Hz famiclones of its family.
    Dendy = 3,
    /// PAL-M famiclone (Brazil).
    PalM = 4,
    /// PAL-N famiclone (Argentina).
    PalN = 5,
}

impl ConsoleModel {
    /// Every model, in the order of the menu.
    pub const ALL: [Self; 6] = [
        Self::Ntsc2C02G,
        Self::Ntsc2C02BeforeG,
        Self::Pal2C07,
        Self::Dendy,
        Self::PalM,
        Self::PalN,
    ];

    /// The model of number `raw` (the menu command's argument), `None` otherwise.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(Self::Ntsc2C02G),
            1 => Some(Self::Ntsc2C02BeforeG),
            2 => Some(Self::Pal2C07),
            3 => Some(Self::Dendy),
            4 => Some(Self::PalM),
            5 => Some(Self::PalN),
            _ => None,
        }
    }

    /// The name the menu shows: chip and region names, the same in every language.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ntsc2C02G => "NES / Famicom (2C02G, 2C02H)",
            Self::Ntsc2C02BeforeG => "NES / Famicom (2C02 before G)",
            Self::Pal2C07 => "PAL NES (2C07)",
            Self::Dendy => "Dendy",
            Self::PalM => "Famiclone PAL-M",
            Self::PalN => "Famiclone PAL-N",
        }
    }

    /// The region the model runs.
    #[must_use]
    pub const fn region(self) -> TvRegionSetting {
        match self {
            Self::Ntsc2C02G | Self::Ntsc2C02BeforeG => TvRegionSetting::Ntsc,
            Self::Pal2C07 => TvRegionSetting::Pal,
            Self::Dendy => TvRegionSetting::Dendy,
            Self::PalM => TvRegionSetting::PalM,
            Self::PalN => TvRegionSetting::PalN,
        }
    }

    /// The `emu.ppu_revision` the model implies, `None` when no reference names it.
    #[must_use]
    pub const fn ppu_revision(self) -> Option<i32> {
        match self {
            Self::Ntsc2C02G | Self::Pal2C07 => Some(REVISION_G),
            Self::Ntsc2C02BeforeG => Some(REVISION_BEFORE_G),
            Self::Dendy | Self::PalM | Self::PalN => None,
        }
    }

    /// The `emu.oamaddr_bug` the model implies, `None` when no reference names it.
    #[must_use]
    pub const fn oamaddr_bug(self) -> Option<bool> {
        match self {
            Self::Ntsc2C02G => Some(true),
            Self::Ntsc2C02BeforeG | Self::Pal2C07 => Some(false),
            Self::Dendy | Self::PalM | Self::PalN => None,
        }
    }

    /// Writes the model's values into `model`, region detection off.
    pub fn apply(self, model: &mut SettingsModel) {
        model.set_auto_tv_mode(false);
        model.set_tv_mode(self.region());
        if let Some(revision) = self.ppu_revision() {
            model.set(SettingId::EmuPpuRevision, SettingValue::Enum(revision));
        }
        if let Some(bug) = self.oamaddr_bug() {
            model.set(SettingId::EmuOamaddrBug, SettingValue::Bool(bug));
        }
    }

    /// Whether every value the model sets is the one in force in `model`.
    #[must_use]
    pub fn is_selected(self, model: &SettingsModel) -> bool {
        !model.auto_tv_mode()
            && model.tv_mode() == self.region()
            && self
                .ppu_revision()
                .is_none_or(|revision| model.get(SettingId::EmuPpuRevision).as_int() == revision)
            && self
                .oamaddr_bug()
                .is_none_or(|bug| model.get(SettingId::EmuOamaddrBug).as_bool() == bug)
    }
}
