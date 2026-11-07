//! The Nintendo VS. System, single-screen (lot MF-VS, behind `board-vs-system`).
//!
//! An arcade board built around a 2A03 and a *different* PPU: the palette order is not the
//! 2C02's, one PPU family swaps two registers and answers an identity, and the cabinet adds
//! two coin slots, a service button and eight DIP switches on `$4016`/`$4017`. The whole of
//! that is **chosen once at load** here and handed to its consumers as an answer — a palette
//! LUT, a byte of status bits, a `bool` — so that a plain NES image never asks which machine
//! it is running on.
//!
//! Sources, cited again at each register: the nesdev wiki pages "Vs. System" (the `$4016` and
//! `$4017` layouts and the controller wiring), "NES 2.0" (byte 7's console type, byte 13's two
//! nibbles and byte 15's default expansion device), "INES Mapper 099" (the board) and
//! "PPU palettes" (the ten master palettes).
//!
//! What is **not** here, recorded as gaps (`docs/progress/MF-VS.md`): the VS. `DualSystem`'s
//! second CPU and PPU, the four protection boards byte 13 names, and the VS. Zapper. Each is
//! *detected* and named in the load note, and the image then runs as a single screen with
//! ordinary controllers.

mod palettes;
mod roms;

pub use palettes::LUT_LEN;
pub use roms::{rom_table, VsRom};

use crate::rom::InesHeader;

/// Which PPU a VS. board carries (nesdev, "NES 2.0" byte 13 and "PPU palettes").
///
/// The variants keep the chips' own part numbers; `Rc2c03` stands for every RP2C03 and RC2C03
/// variant, whose master palette is the ordered one — the plain NES palette, as far as an index
/// remap is concerned (lot MF-VS step 2).
#[allow(non_camel_case_types)] // The part numbers are RP2C04-0001 and RC2C05-01; they are names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VsPpuModel {
    /// Any RP2C03 or RC2C03 variant: the ordered master palette. The default for an image
    /// nothing identifies, which is what the load note says.
    #[default]
    Rc2c03 = 0,
    /// RP2C04-0001.
    Rp2c04_0001 = 1,
    /// RP2C04-0002.
    Rp2c04_0002 = 2,
    /// RP2C04-0003.
    Rp2c04_0003 = 3,
    /// RP2C04-0004.
    Rp2c04_0004 = 4,
    /// RC2C05-01.
    Rc2c05_01 = 5,
    /// RC2C05-02.
    Rc2c05_02 = 6,
    /// RC2C05-03.
    Rc2c05_03 = 7,
    /// RC2C05-04.
    Rc2c05_04 = 8,
    /// RC2C05-05.
    Rc2c05_05 = 9,
}

impl VsPpuModel {
    /// The chip's part number, for the load note and the harness.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rc2c03 => "RP2C03/RC2C03",
            Self::Rp2c04_0001 => "RP2C04-0001",
            Self::Rp2c04_0002 => "RP2C04-0002",
            Self::Rp2c04_0003 => "RP2C04-0003",
            Self::Rp2c04_0004 => "RP2C04-0004",
            Self::Rc2c05_01 => "RC2C05-01",
            Self::Rc2c05_02 => "RC2C05-02",
            Self::Rc2c05_03 => "RC2C05-03",
            Self::Rc2c05_04 => "RC2C05-04",
            Self::Rc2c05_05 => "RC2C05-05",
        }
    }

    /// The model of byte 13's low nibble (nesdev, "NES 2.0", Vs. PPU Type).
    ///
    /// `$0`, `$1`, `$6` and `$7` are the four RP2C03/RC2C03 variants and share one entry here,
    /// their master palette being the ordered one. `$D` to `$F` are reserved and take the same
    /// default an unidentified image takes.
    #[must_use]
    pub const fn from_nes20_byte13(byte13: u8) -> Self {
        match byte13 & 0x0F {
            0x2 => Self::Rp2c04_0001,
            0x3 => Self::Rp2c04_0002,
            0x4 => Self::Rp2c04_0003,
            0x5 => Self::Rp2c04_0004,
            0x8 => Self::Rc2c05_01,
            0x9 => Self::Rc2c05_02,
            0xA => Self::Rc2c05_03,
            0xB => Self::Rc2c05_04,
            0xC => Self::Rc2c05_05,
            // $0 RP2C03B, $1 RP2C03G, $6 RC2C03B, $7 RC2C03C, and $D-$F reserved.
            _ => Self::Rc2c03,
        }
    }

    /// Whether this is an RC2C05, the family that swaps `$2000` and `$2001` and answers an
    /// identity in `$2002`'s low bits (nesdev, "Vs. System"). Step 3 of the lot is that pair.
    #[must_use]
    pub const fn is_rc2c05(self) -> bool {
        matches!(
            self,
            Self::Rc2c05_01 | Self::Rc2c05_02 | Self::Rc2c05_03 | Self::Rc2c05_04 | Self::Rc2c05_05
        )
    }
}

/// What byte 13's high nibble says the board is (nesdev, "NES 2.0", Vs. Hardware Type).
///
/// Only [`Self::Unisystem`] is emulated. The six others are **detected and named** so that the
/// load note can say what the image wants and what it will get instead; `docs/progress/MF-VS.md`
/// records each as a gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VsHardware {
    /// `$0`: Vs. Unisystem, one CPU and one PPU. The machine this lot builds.
    #[default]
    Unisystem = 0,
    /// `$1`: the RBI Baseball protection, a counter read at `$5E00`/`$5E01`. Not emulated.
    RbiBaseball = 1,
    /// `$2`: the TKO Boxing protection, the same mechanism with another table. Not emulated.
    TkoBoxing = 2,
    /// `$3`: the Super Xevious protection. Not emulated.
    SuperXevious = 3,
    /// `$4`: the Vs. Ice Climber (Japan) protection. Not emulated.
    IceClimberJapan = 4,
    /// `$5`: Vs. `DualSystem`, two CPUs and two PPUs sharing 2 KiB. Runs as a single screen.
    DualSystem = 5,
    /// `$6`: Vs. `DualSystem` with the Raid on Bungeling Bay protection. As above.
    RaidOnBungelingBay = 6,
}

impl VsHardware {
    /// The hardware type's name, for the load note and the harness.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unisystem => "Vs. Unisystem",
            Self::RbiBaseball => "Vs. Unisystem, RBI Baseball protection",
            Self::TkoBoxing => "Vs. Unisystem, TKO Boxing protection",
            Self::SuperXevious => "Vs. Unisystem, Super Xevious protection",
            Self::IceClimberJapan => "Vs. Unisystem, Vs. Ice Climber (Japan) protection",
            Self::DualSystem => "Vs. DualSystem",
            Self::RaidOnBungelingBay => "Vs. DualSystem, Raid on Bungeling Bay protection",
        }
    }

    /// The type of byte 13's high nibble; `$7` to `$F` are reserved and read as Unisystem.
    #[must_use]
    pub const fn from_nes20_byte13(byte13: u8) -> Self {
        match byte13 >> 4 {
            1 => Self::RbiBaseball,
            2 => Self::TkoBoxing,
            3 => Self::SuperXevious,
            4 => Self::IceClimberJapan,
            5 => Self::DualSystem,
            6 => Self::RaidOnBungelingBay,
            _ => Self::Unisystem,
        }
    }

    /// Whether this is one of the two `DualSystem` types, which this build runs as a single
    /// screen (a recorded gap).
    #[must_use]
    pub const fn is_dual_system(self) -> bool {
        matches!(self, Self::DualSystem | Self::RaidOnBungelingBay)
    }

    /// Whether the type names a protection board this build does not emulate.
    #[must_use]
    pub const fn has_protection(self) -> bool {
        matches!(
            self,
            Self::RbiBaseball
                | Self::TkoBoxing
                | Self::SuperXevious
                | Self::IceClimberJapan
                | Self::RaidOnBungelingBay
        )
    }
}

/// Which stick is player one's (nesdev, "Vs. System": "Left stick connects to `$4017`; right
/// stick connects to `$4016`. Games vary on which stick represents player one", and
/// "NES 2.0" byte 15, whose `$04` is "Vs. System (1P via `$4016`)" and `$05`
/// "Vs. System (1P via `$4017`)").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VsWiring {
    /// Byte 15 `$04`: player one reads on `$4016`, as on a NES. No swap.
    #[default]
    PlayerOneOnPort0 = 0,
    /// Byte 15 `$05`: player one reads on `$4017`. The two sticks are swapped.
    PlayerOneOnPort1 = 1,
    /// Byte 15 `$07`: a VS. Zapper game. The light gun is a recorded gap; the image runs with
    /// ordinary controllers on the `$4016` wiring.
    Zapper = 2,
}

impl VsWiring {
    /// Whether the two sticks are swapped.
    #[must_use]
    pub const fn swaps_sticks(self) -> bool {
        matches!(self, Self::PlayerOneOnPort1)
    }

    /// The wiring of byte 15, or `None` when the byte names no VS device (nesdev, "NES 2.0",
    /// Default Expansion Device).
    #[must_use]
    pub const fn from_nes20_byte15(byte15: u8) -> Option<Self> {
        match byte15 & 0x3F {
            0x04 => Some(Self::PlayerOneOnPort0),
            0x05 => Some(Self::PlayerOneOnPort1),
            0x07 => Some(Self::Zapper),
            _ => None,
        }
    }
}

/// Where the configuration below came from, which is what the load note reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VsSource {
    /// NES 2.0 byte 13 (and byte 15 for the wiring): the image declared it.
    Header,
    /// The per-ROM table, by the CRC-32 of the PRG and CHR data.
    RomTable,
    /// Neither. The RP2C03 — the plain NES palette — a Unisystem and no stick swap.
    Default,
}

/// The VS. System of the image loaded: what was chosen at load, and the cabinet's state.
///
/// Everything a consumer needs is an *answer* stored here, never a question asked per dot or
/// per register: the palette is a `&'static [u8; 64]` the PPU's palette snapshot is remapped
/// through, [`Self::is_rc2c05`] settles the two register quirks once, and the input surface is
/// read only by the `$4016`/`$4017` arm, which is already out of line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsSystem {
    ppu: VsPpuModel,
    hardware: VsHardware,
    wiring: VsWiring,
    source: VsSource,
    title: Option<&'static str>,
}

impl VsSystem {
    /// The VS. System of `header` and `data_crc`, or `None` for an image that is not one.
    ///
    /// An image is a VS. System when **any** of these holds:
    ///
    /// * an iNES 1.0 header has **byte 7 bit 0** set (nesdev, "NES 2.0": byte 7's two low bits
    ///   are the console type, and `1` is "Nintendo Vs. System" — on an iNES 1.0 header bit 1
    ///   is the PlayChoice-10 bit and bit 0 the VS one);
    /// * a NES 2.0 header has **console type 1**, or console type 3 with byte 13's **extended
    ///   console type** equal to 1;
    /// * the mapper is **99** or **151**, two numbers that exist only on this board, whatever
    ///   the header says about the console (`INES Mapper 099`; mapper 151 is the deprecated
    ///   number for a VRC1 on a VS board, which the public NES 2.0 database records as mapper
    ///   75 with console type 1).
    ///
    /// A **dirty** iNES header is excluded, because bytes 7 to 15 of one are not read at all
    /// (REQ-CORE-012): that is the existing rule of [`InesHeader`] and this does not bend it.
    ///
    /// The configuration then comes from the header when the header carries one, otherwise from
    /// the per-ROM table, otherwise from the defaults — in that order, reported by
    /// [`Self::source`].
    #[must_use]
    pub fn detect(header: &InesHeader, data_crc: u32) -> Option<Self> {
        if !Self::is_vs_image(header) {
            return None;
        }

        let entry = roms::find(data_crc);

        // A NES 2.0 console type of 1 is the one case where the image itself names the PPU and
        // the hardware. The extended console type (byte 7 = 3, byte 13 low nibble = 1) says
        // "Vs. System" and nothing more, so it falls through to the table like iNES 1.0.
        if header.is_nes20 && header.console_type == 1 {
            let wiring = VsWiring::from_nes20_byte15(header.byte15)
                .or_else(|| entry.map(VsRom::wiring))
                .unwrap_or_default();
            return Some(Self::new(
                VsPpuModel::from_nes20_byte13(header.byte13),
                VsHardware::from_nes20_byte13(header.byte13),
                wiring,
                VsSource::Header,
                entry.map(VsRom::title),
            ));
        }

        Some(match entry {
            Some(rom) => Self::new(
                rom.ppu(),
                rom.hardware(),
                rom.wiring(),
                VsSource::RomTable,
                Some(rom.title()),
            ),
            None => Self::new(
                VsPpuModel::Rc2c03,
                VsHardware::Unisystem,
                VsWiring::PlayerOneOnPort0,
                VsSource::Default,
                None,
            ),
        })
    }

    /// Whether `header` announces a VS. System image; see [`Self::detect`] for the three rules.
    #[must_use]
    pub const fn is_vs_image(header: &InesHeader) -> bool {
        if header.mapper == 99 || header.mapper == 151 {
            return true;
        }

        if header.is_dirty {
            return false;
        }

        if header.is_nes20 {
            // Console type 1, or the extended console type 1 of byte 13 behind console type 3.
            header.console_type == 1 || (header.console_type == 3 && (header.byte13 & 0x0F) == 1)
        } else {
            header.console_type & 0x01 != 0
        }
    }

    const fn new(
        ppu: VsPpuModel,
        hardware: VsHardware,
        wiring: VsWiring,
        source: VsSource,
        title: Option<&'static str>,
    ) -> Self {
        Self {
            ppu,
            hardware,
            wiring,
            source,
            title,
        }
    }

    /// The PPU model chosen at load.
    #[must_use]
    pub const fn ppu(&self) -> VsPpuModel {
        self.ppu
    }

    /// The hardware type chosen at load.
    #[must_use]
    pub const fn hardware(&self) -> VsHardware {
        self.hardware
    }

    /// The stick wiring chosen at load.
    #[must_use]
    pub const fn wiring(&self) -> VsWiring {
        self.wiring
    }

    /// Where the three above came from.
    #[must_use]
    pub const fn source(&self) -> VsSource {
        self.source
    }

    /// The title the per-ROM table knows the image under, when it knows it.
    #[must_use]
    pub const fn title(&self) -> Option<&'static str> {
        self.title
    }

    /// Whether the PPU is an RC2C05; see [`VsPpuModel::is_rc2c05`].
    #[must_use]
    pub const fn is_rc2c05(&self) -> bool {
        self.ppu.is_rc2c05()
    }

    /// The PPU's master palette as a remap into the ordered palette, or `None` when it **is**
    /// the ordered palette; see [`VsPpuModel::palette_lut`].
    #[must_use]
    pub const fn palette_lut(&self) -> Option<&'static [u8; LUT_LEN]> {
        self.ppu.palette_lut()
    }

    /// The message the host logs when a VS image is loaded: what was detected, where from, and
    /// every piece of it this build does not emulate.
    ///
    /// The owner's rule for an unidentified image is in it literally: such an image "gets the
    /// RP2C03 (plain NES palette)" and the note says so.
    #[must_use]
    pub fn load_note(&self) -> String {
        let mut note = String::from("VS. System: ");
        note.push_str(self.hardware.name());
        note.push_str(", ");
        note.push_str(self.ppu.name());

        match self.source {
            VsSource::Header => note.push_str(" (NES 2.0 byte 13)"),
            VsSource::RomTable => {
                note.push_str(" (known dump");
                if let Some(title) = self.title {
                    note.push_str(": ");
                    note.push_str(title);
                }
                note.push(')');
            }
            VsSource::Default => note.push_str(
                " (unknown VS image: no PPU in the header and no row in the table, \
                 so the plain NES palette)",
            ),
        }

        if self.wiring.swaps_sticks() {
            note.push_str("; player one on $4017");
        }

        if self.hardware.is_dual_system() {
            note.push_str("; DualSystem not emulated, running as a single screen");
        }

        if self.hardware.has_protection() {
            note.push_str("; the board's protection is not emulated");
        }

        if matches!(self.wiring, VsWiring::Zapper) {
            note.push_str("; the VS. Zapper is not emulated");
        }

        note
    }
}
