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
pub use roms::{rom_table, VsBoardSpec, VsRom};

use crate::rom::InesHeader;
use crate::state::{InvalidState, StateReader, StateWriter};

/// The header a VS. System image should load under, when the per-ROM table knows the dump and the
/// image's own header disagrees with it, and the sentence the load message then carries.
///
/// **The defect this exists for** (lot MF-VS-fix). Circulating iNES 1.0 dumps of
/// *Vs. Castlevania* and *Vs. Top Gun* announce **mapper 99 with 0 KiB of CHR-ROM**; the boards
/// are `UxROM` (mapper 2, submapper 2) with 8 KiB of CHR-RAM. Loaded as the header says, the image
/// comes up with the mapper-99 board fetching its patterns from a CHR-ROM that is not there and
/// the screen stays empty. The table has known the right answer since MF-VS step 1 — it was in a
/// comment. Many VS dumps carry mapper 99 by default in old header sets, so the rule is the
/// table's and not one dump's.
///
/// **Who wins.** An **iNES 1.0** header describes a board with four fields and a nibble, and the
/// database row keyed by the CRC of this very data describes it with six; every field of the row
/// that disagrees wins. A **NES 2.0** header is trusted — it has a field for all of this and a
/// dumper filled them — **unless it contradicts the table on the mapper**, in which case the
/// mapper and the submapper are the table's and the message says so. Nothing else of a NES 2.0
/// header is touched: getting the mapper wrong does not make a header wrong about a size
/// (**Q-138** records that reading).
///
/// **What it never does.** It is reached only for an image [`VsSystem::is_vs_image`] already
/// holds for, so a NES image whose data CRC happened to collide with a row is not touched. It
/// never grows the image: `announced_size` after the correction is checked against the bytes
/// there are, and sizes that would not fit are left as the header had them (which cannot happen
/// for a dump whose CRC matched the row — the CRC is of the data the header delimits — and is
/// refused rather than trusted all the same).
///
/// Returns `None` when there is nothing to correct, which is the case for every correctly
/// headered dump and for every VS image the table does not know.
#[must_use]
pub(crate) fn corrected_board(
    header: &InesHeader,
    data_crc: u32,
    image_len: usize,
) -> Option<(InesHeader, String)> {
    if !VsSystem::is_vs_image(header) {
        return None;
    }

    let row = roms::find(data_crc)?;
    let board = row.board();
    let mut corrected = header.clone();
    corrected.mapper = board.mapper();
    corrected.submapper = board.submapper();

    if header.is_nes20 {
        // Trusted on everything but the mapper, and only when it contradicts the table there.
        // The brief's "the table wins too" reaches the mapper and the submapper and stops: a
        // NES 2.0 header has a field for every one of the others and a dumper filled them, and a
        // wrong mapper does not make a size wrong. Q-138 is the reading and what it would cost.
        if header.mapper == board.mapper() {
            return None;
        }

        corrected.board_from_vs_table = true;
        return Some((
            corrected,
            format!(
                "the NES 2.0 header says mapper {}, the VS table says mapper {} for {} \
                 and the table wins",
                header.mapper,
                board.mapper(),
                row.title()
            ),
        ));
    }

    corrected.chr_ram_size = board.chr_ram_size();
    corrected.mirroring = board.mirroring();
    corrected.prg_rom_size = board.prg_rom_size();
    corrected.chr_rom_size = board.chr_rom_size();
    if image_len < corrected.announced_size() {
        // Defensive: the row's split is not the one these bytes can hold, so keep the header's.
        corrected.prg_rom_size = header.prg_rom_size;
        corrected.chr_rom_size = header.chr_rom_size;
    }

    if corrected.mapper == header.mapper
        && corrected.submapper == header.submapper
        && corrected.prg_rom_size == header.prg_rom_size
        && corrected.chr_rom_size == header.chr_rom_size
        && corrected.chr_ram_size == header.chr_ram_size
        && corrected.mirroring == header.mirroring
    {
        return None;
    }

    corrected.board_from_vs_table = true;
    let mut fields = Vec::with_capacity(4);
    if corrected.mapper != header.mapper || corrected.submapper != header.submapper {
        fields.push(format!(
            "mapper {} is mapper {} submapper {}",
            header.mapper, corrected.mapper, corrected.submapper
        ));
    }
    if corrected.prg_rom_size != header.prg_rom_size
        || corrected.chr_rom_size != header.chr_rom_size
    {
        fields.push(format!(
            "{} KiB of PRG-ROM and {} KiB of CHR-ROM",
            corrected.prg_rom_size / 1024,
            corrected.chr_rom_size / 1024
        ));
    }
    if corrected.chr_ram_size != header.chr_ram_size {
        fields.push(format!("{} KiB of CHR-RAM", corrected.chr_ram_size / 1024));
    }
    if corrected.mirroring != header.mirroring {
        fields.push(format!("{:?} mirroring", corrected.mirroring));
    }

    Some((
        corrected,
        format!(
            "the iNES 1.0 header of {} is corrected by the VS table: {}",
            row.title(),
            fields.join(", ")
        ),
    ))
}

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

    /// What a `$2002` read answers below the three flags: `(latch_mask, identifier)`.
    ///
    /// `latch_mask` is the bits that come from the PPU's own data-bus latch and `identifier` the
    /// bits the chip drives itself. On every PPU but an RC2C05 that is `(0x1F, 0x00)` — bits 0
    /// to 4 open, nothing driven — which is the 2C02's own behaviour (nesdev, "PPU registers";
    /// `ppu_open_bus` tests 6 and 7).
    ///
    /// **The RC2C05 drives them instead**: "The 2C05 series of arcade PPUs returns an identifier
    /// in bits 4-0 instead of PPU open bus" (nesdev, "PPU registers"), and that page's table
    /// gives a *mask* beside each value, because the RC2C05-02's covers **six** bits and not
    /// five:
    ///
    /// | PPU | mask | value |
    /// | --- | ---: | ---: |
    /// | RC2C05-01 | `$1F` | `$1B` |
    /// | RC2C05-02 | `$3F` | `$3D` |
    /// | RC2C05-03 | `$1F` | `$1C` |
    /// | RC2C05-04 | `$1F` | `$1B` |
    /// | RC2C05-05 | `$1F` | `$00` |
    ///
    /// The mask the page gives is the bits the chip drives, so the latch mask returned here is
    /// its complement inside bits 0 to 5 — `0x00` for the four variants whose mask is `$1F`,
    /// since nothing of bits 0 to 4 is left open. The RC2C05-02's `$3F`/`$3D` forces bit 5, the
    /// sprite-overflow flag's bit, to 1 for good, which the `or` below does exactly.
    ///
    /// Two of the five values are not on that page's table, which lists the `-02`, `-03` and
    /// `-04` only, and are the ones `third_party/mesen2`'s `NesPpu.cpp` has (`Ppu2C05A` `0x1B`,
    /// `Ppu2C05E` nothing driven); the three the page does give agree with it to the bit.
    /// **Q-136 question 2** records those two rows as a provisional choice.
    #[must_use]
    pub const fn status_identifier(self) -> (u8, u8) {
        match self {
            // The `-01` and the `-04` answer the same `$1B`, which the page's table gives twice
            // and which a game therefore cannot use to tell those two apart.
            Self::Rc2c05_01 | Self::Rc2c05_04 => (0x00, 0x1B),
            Self::Rc2c05_02 => (0x00, 0x3D),
            Self::Rc2c05_03 => (0x00, 0x1C),
            // The RC2C05-05 drives bits 0 to 4 with nothing: the identifier is zero, and the
            // latch stays shut, which is what makes it distinguishable from a 2C02 all the same.
            Self::Rc2c05_05 => (0x00, 0x00),
            // Every RP2C03, RC2C03 and RP2C04 answers like a 2C02 here.
            Self::Rc2c03
            | Self::Rp2c04_0001
            | Self::Rp2c04_0002
            | Self::Rp2c04_0003
            | Self::Rp2c04_0004 => (0x1F, 0x00),
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

/// The start and select bits of a host's port byte, swapped: the cabinet's start button of that
/// port's player sits where a NES controller puts Select, and the third- and fourth-player start
/// lines of a `DualSystem` sit where it puts Start (see [`VsSystem::input_word`]).
const fn start_in_select_slot(start_select: u8) -> u8 {
    ((start_select & 0x04) << 1) | ((start_select & 0x08) >> 1)
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
    /// The eight DIP switches, bit `n` being switch `n + 1`. All off at load: the brief takes the
    /// defaults "from the per-ROM table when the source gives them, otherwise all off", and the
    /// header database the table is built from records none (see `roms`).
    dip: u8,
    /// Frames each coin slot's line is still held for, counted down by [`Self::begin_frame`]; a
    /// host that drives the line itself uses [`Self::set_coin`] instead (lot MF-VS step 5).
    coin_frames: [u8; Self::COIN_SLOTS],
    /// Whether the service button is held.
    service: bool,
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
            dip: 0,
            coin_frames: [0; Self::COIN_SLOTS],
            service: false,
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

    // ----------------------------------------------------------------- the cabinet

    /// Number of coin slots a Unisystem cabinet has.
    pub const COIN_SLOTS: usize = 2;

    /// Number of DIP switches.
    pub const DIP_SWITCHES: usize = 8;

    /// How many frames [`Self::insert_coin`] holds a coin line for.
    ///
    /// A real coin passing the slot closes the switch for some tens of milliseconds, and a game
    /// samples it once a frame; one frame would be a race against where in the frame the host
    /// asked. Four is what `third_party/mesen2`'s `VsInputButtons` uses
    /// (`InsertCoinFrameCount = 4`) and what this takes. A host that wants to drive the line
    /// itself — a movie, a test — uses [`Self::set_coin`] and ignores this.
    pub const COIN_FRAMES: u8 = 4;

    /// The eight DIP switches as a byte, bit `n` being switch `n + 1`.
    #[must_use]
    pub const fn dip_switches(&self) -> u8 {
        self.dip
    }

    /// Sets all eight DIP switches at once.
    pub const fn set_dip_switches(&mut self, switches: u8) {
        self.dip = switches;
    }

    /// Whether DIP switch `switch` (**1** to 8, as the cabinet's manual numbers them) is on;
    /// `false` for any other number.
    #[must_use]
    pub const fn dip_switch(&self, switch: usize) -> bool {
        if switch == 0 || switch > Self::DIP_SWITCHES {
            return false;
        }

        self.dip & (1 << (switch - 1)) != 0
    }

    /// Turns DIP switch `switch` (1 to 8) on or off; any other number is ignored.
    pub const fn set_dip_switch(&mut self, switch: usize, on: bool) {
        if switch == 0 || switch > Self::DIP_SWITCHES {
            return;
        }

        let bit = 1u8 << (switch - 1);
        if on {
            self.dip |= bit;
        } else {
            self.dip &= !bit;
        }
    }

    /// Drops a coin into slot `slot` (**1** or 2): the line is held for [`Self::COIN_FRAMES`]
    /// frames and then released. Any other slot number is ignored.
    pub const fn insert_coin(&mut self, slot: usize) {
        if slot == 0 || slot > Self::COIN_SLOTS {
            return;
        }

        self.coin_frames[slot - 1] = Self::COIN_FRAMES;
    }

    /// Holds or releases slot `slot`'s line directly, for a host that times the coin itself.
    pub const fn set_coin(&mut self, slot: usize, held: bool) {
        if slot == 0 || slot > Self::COIN_SLOTS {
            return;
        }

        self.coin_frames[slot - 1] = if held { u8::MAX } else { 0 };
    }

    /// Whether slot `slot`'s line is held.
    #[must_use]
    pub const fn coin(&self, slot: usize) -> bool {
        if slot == 0 || slot > Self::COIN_SLOTS {
            return false;
        }

        self.coin_frames[slot - 1] != 0
    }

    /// Whether the service button is held.
    #[must_use]
    pub const fn service_button(&self) -> bool {
        self.service
    }

    /// Holds or releases the service button, which a cabinet's operator presses to give a free
    /// credit.
    pub const fn set_service_button(&mut self, held: bool) {
        self.service = held;
    }

    /// Releases the coin lines a frame older than [`Self::COIN_FRAMES`]; called once per frame by
    /// [`crate::input::InputPorts::begin_frame`].
    ///
    /// A line held with [`Self::set_coin`] stays at [`u8::MAX`] and is never counted down, which
    /// is what makes that call "hold it until I say otherwise".
    pub const fn begin_frame(&mut self) {
        let mut slot = 0;
        while slot < Self::COIN_SLOTS {
            if self.coin_frames[slot] != 0 && self.coin_frames[slot] != u8::MAX {
                self.coin_frames[slot] -= 1;
            }

            slot += 1;
        }
    }

    /// The bits a VS. System drives into a `$4016` or `$4017` read **above** the controller's
    /// serial bit, which on a NES would be the stale bus contents (nesdev, "Vs. System").
    ///
    /// `$4016` read:
    ///
    /// ```text
    /// 7  bit  0
    /// ---- ----
    /// Cba. .sD0
    /// |||| ||||
    /// |||| |||+- the right stick's serial data
    /// |||| ||+-- always 0
    /// |||| |+--- the service button
    /// ||++-+---- DIP switches 1 and 2
    /// |++------- coin inserted, slots 1 and 2
    /// +--------- CPU identifier: 0 on the primary, 1 on a `DualSystem`'s secondary
    /// ```
    ///
    /// `$4017` read: the left stick's serial data in bit 0, 0 in bit 1, and **DIP switches 3 to
    /// 8 in bits 2 to 7** — which makes that half the identity on this byte's bits 2 to 7.
    ///
    /// **One ambiguity of the page's wording, resolved and marked.** It gives the two ranges as
    /// "Bits 3-4: DIP switches '2' and '1'" and "Bits 2-7: DIP switches 8 through 3", which read
    /// left to right would put switch 2 in bit 3 and switch 8 in bit 2 — the reverse of what is
    /// below. The reading taken is that the page lists the bits of a range **high first**, as
    /// every register diagram on the wiki does, so bit 4 is switch 2 and bit 7 switch 8; it is
    /// also what `third_party/mesen2`'s `VsControlManager::ReadRam` does, bit for bit.
    /// **Q-136 question 1** records it as a provisional choice: the consequence of being wrong is
    /// a game reading its cabinet options mirrored and nothing worse.
    ///
    /// Bit 7 is 0: this build is always the primary CPU, the `DualSystem`'s second being a
    /// recorded gap.
    #[must_use]
    pub const fn port_bits(&self, port: usize) -> u8 {
        if port == 0 {
            let mut bits = (self.dip & 0x03) << 3;
            if self.service {
                bits |= 0x04;
            }

            if self.coin_frames[0] != 0 {
                bits |= 0x20;
            }

            if self.coin_frames[1] != 0 {
                bits |= 0x40;
            }

            bits
        } else {
            // Switches 3 to 8 are bits 2 to 7 of both bytes, so this half is a mask.
            self.dip & 0xFC
        }
    }

    /// The frame's input word as the cabinet's wiring presents it to the two ports, from the word
    /// the host latched for players 1 and 2 (nesdev, "Vs. System"; lot MF-VS step 5).
    ///
    /// **Two things are not what a NES does**, and both come from the page's own bit lists —
    /// `$4016` shifts out "A, B, 1, 3, Up, Down, Left, Right" and `$4017` "A, B, 2, 4, Up, Down,
    /// Left, Right", where a NES controller shifts A, B, **Select, Start**, Up, Down, Left,
    /// Right (nesdev, "Standard controller"):
    ///
    /// 1. **There is no Select on a cabinet.** What a NES reads as Select is the *start button*
    ///    of the player that stick belongs to — "1" on `$4016`, "2" on `$4017` — and what it
    ///    reads as Start is the start button of players 3 and 4, which only a `DualSystem` has.
    ///    So the two bits are swapped inside each port byte: the host's Start reaches the start
    ///    button and its Select lands on the unused third-player line.
    /// 2. **Which stick is player one's is a property of the game** ([`VsWiring`]). For
    ///    [`VsWiring::PlayerOneOnPort1`] the two **sticks** swap — A, B and the four directions —
    ///    while the start buttons do **not**, because "1" is wired to `$4016` whichever stick the
    ///    game reads first.
    ///
    /// Applied once per frame by [`crate::input::InputPorts::begin_frame`], never per read.
    #[must_use]
    pub const fn input_word(&self, host_word: u16) -> u16 {
        const STICK: u8 = 0xF3; // A, B, Up, Down, Left, Right.
        const START_SELECT: u8 = 0x0C;

        let [player_one, player_two] = host_word.to_le_bytes();
        let (stick_on_port_0, stick_on_port_1) = if self.wiring.swaps_sticks() {
            (player_two, player_one)
        } else {
            (player_one, player_two)
        };

        u16::from_le_bytes([
            (stick_on_port_0 & STICK) | start_in_select_slot(player_one & START_SELECT),
            (stick_on_port_1 & STICK) | start_in_select_slot(player_two & START_SELECT),
        ])
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

    // ----------------------------------------------------------------- the savestate

    /// Appends the **cabinet's** state to `writer`: the eight DIP switches, the two coin lines
    /// and the service button (REQ-CORE-090).
    ///
    /// Everything else here was chosen at load from the image and cannot differ between a save
    /// and its restore — the PPU model, the hardware type, the wiring — so none of it is in the
    /// block; the mapper-99 board's bank select is in that board's own registers, where every
    /// board's are.
    ///
    /// **The format version is untouched, and the reason is not an oversight.** These bytes are
    /// written only for a machine that *has* a VS. System plugged in, so a NES block is byte for
    /// byte the block it was and REQ-CORE-091's older versions keep loading. A block that carries
    /// them can only have been written by a build with `board-vs-system` that had loaded a VS
    /// image, and the reader is in exactly the same position — it knows the machine before it
    /// reads. A future lot that *grows* this block gates the growth on
    /// [`crate::state::StateReader::format_version`], which is what that accessor is for.
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.dip);
        for frames in self.coin_frames {
            writer.write_u8(frames);
        }

        writer.write_bool(self.service);
    }

    /// Restores the cabinet's state (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.dip = reader.read_u8()?;
        for slot in 0..Self::COIN_SLOTS {
            self.coin_frames[slot] = reader.read_u8()?;
        }

        self.service = reader.read_bool()?;
        Ok(())
    }
}
