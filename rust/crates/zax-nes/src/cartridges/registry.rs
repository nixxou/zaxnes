//! Maps an iNES mapper number to its board (port of `MapperRegistry`, REQ-CORE-100,
//! REQ-CORE-102).
//!
//! The registry only advertises the boards that are implemented, so that a ROM is never
//! accepted and then found unplayable. The table is the one of REQ-CORE-100; several numbers
//! share a board when they are the same silicon differently wired (9 and 10, 21/22/23/25/27,
//! 87 and 101). 156 is outside that table: it runs Metal Force (Korea), a widescreen target.

#[cfg(feature = "board-111")]
use super::mappers::Cheapocabra;
#[cfg(feature = "board-decode-boards")]
use super::mappers::DecodeBoard;
#[cfg(feature = "board-eeprom")]
use super::mappers::EepromPort;
#[cfg(feature = "board-77")]
use super::mappers::IremLrog017;
#[cfg(feature = "board-jv001")]
use super::mappers::Jv001;
#[cfg(feature = "board-mmc3-chrram")]
use super::mappers::Mmc3ChrRamBoard;
#[cfg(feature = "board-263")]
use super::mappers::Mmc3Kof97;
#[cfg(feature = "board-mmc3-multicart")]
use super::mappers::Mmc3Multicart;
#[cfg(feature = "board-mmc3-scramble")]
use super::mappers::Mmc3Scramble;
#[cfg(feature = "board-189")]
use super::mappers::Mmc3Txc189;
#[cfg(feature = "board-namcot108")]
use super::mappers::Namcot108;
#[cfg(feature = "board-105")]
use super::mappers::NesEvent;
#[cfg(feature = "board-nina03")]
use super::mappers::Nina03;
use super::mappers::Nrom;
#[cfg(feature = "board-96")]
use super::mappers::OekaKids;
#[cfg(feature = "board-133")]
use super::mappers::Sachen72008;
#[cfg(feature = "board-sachen-8reg")]
use super::mappers::Sachen8Reg;
#[cfg(feature = "board-116")]
use super::mappers::Somari;
#[cfg(feature = "board-68")]
use super::mappers::Sunsoft4;
#[cfg(feature = "board-158")]
use super::mappers::Tengen800037;
#[cfg(feature = "board-119")]
use super::mappers::Tqrom;
#[cfg(feature = "board-vs-system")]
use super::mappers::VsBoard;
use super::mappers::{
    Action52, Action53, AxRom, BandaiFcg, Caltron6In1, CnRom, Codemasters, ColorDreams, CpRom,
    Daou306, Fme7, GxRom, IremG101, IremH3001, IrqBoard, JalecoJ87, JalecoSs88006, Mapper015,
    Mapper034, Mapper078, McAcc, Mmc1, Mmc2, Mmc3, Mmc5, Multicart225, Namco163, NamcoVariant,
    Rambo1, Sunsoft3, TaitoTc0690, UnRom512, UxRom, Vrc2Vrc4, Vrc3, Vrc6, Vrc7,
};
#[cfg(feature = "board-small-boards")]
use super::mappers::{
    Bandai74161, Bf9096, CnRomProtect, IremTamS1, JalecoJf11, SmallBoard, Sunsoft1, Sunsoft2,
    TaitoTc0190, Vrc1,
};
#[cfg(feature = "board-jaleco-72-92-86")]
use super::mappers::{JalecoJf13, JalecoJf17};
#[cfg(any(
    feature = "board-38",
    feature = "board-107",
    feature = "board-143",
    feature = "board-144",
    feature = "board-145",
    feature = "board-149"
))]
use super::mappers::{LatchBoard, SingleLatch};
#[cfg(feature = "board-taito-x1")]
use super::mappers::{TaitoX1005, TaitoX1017};
use super::{BankWindows, Board, CartridgeMemory};
use crate::rom::InesHeader;

/// Mapper numbers this build can run, in ascending order (REQ-CORE-102).
///
/// **The table depends on the board features** (`docs/review/BOARD-FEATURES.md`): a number whose
/// board is behind a feature that is off is not advertised, so a ROM of it is refused with
/// REQ-CORE-011's message rather than accepted and found unplayable — which is the whole point
/// of the registry's own note above. `req_core_100_every_required_mapper_is_registered` holds in
/// every configuration, because REQ-CORE-100's own table is in no feature.
pub const SUPPORTED_MAPPERS: &[i32] = &[
    0,
    1,
    2,
    3,
    4,
    5,
    7,
    9,
    10,
    11,
    13,
    15,
    16,
    18,
    19,
    21,
    22,
    23,
    24,
    25,
    26,
    27,
    28,
    30,
    32,
    #[cfg(feature = "board-small-boards")]
    33,
    34,
    #[cfg(feature = "board-jv001")]
    36,
    #[cfg(feature = "board-mmc3-multicart")]
    37,
    #[cfg(feature = "board-38")]
    38,
    41,
    #[cfg(feature = "board-mmc3-multicart")]
    47,
    48,
    64,
    65,
    66,
    67,
    #[cfg(feature = "board-68")]
    68,
    69,
    #[cfg(feature = "board-small-boards")]
    70,
    71,
    #[cfg(feature = "board-jaleco-72-92-86")]
    72,
    73,
    #[cfg(feature = "board-mmc3-chrram")]
    74,
    #[cfg(feature = "board-small-boards")]
    75,
    #[cfg(feature = "board-namcot108")]
    76,
    #[cfg(feature = "board-77")]
    77,
    78,
    #[cfg(feature = "board-nina03")]
    79,
    #[cfg(feature = "board-taito-x1")]
    80,
    #[cfg(feature = "board-taito-x1")]
    82,
    85,
    #[cfg(feature = "board-jaleco-72-92-86")]
    86,
    87,
    #[cfg(feature = "board-namcot108")]
    88,
    #[cfg(feature = "board-small-boards")]
    89,
    #[cfg(feature = "board-jaleco-72-92-86")]
    92,
    #[cfg(feature = "board-small-boards")]
    93,
    #[cfg(feature = "board-94")]
    94,
    #[cfg(feature = "board-namcot108")]
    95,
    #[cfg(feature = "board-96")]
    96,
    #[cfg(feature = "board-small-boards")]
    97,
    #[cfg(feature = "board-vs-system")]
    99,
    101,
    #[cfg(feature = "board-105")]
    105,
    #[cfg(feature = "board-107")]
    107,
    #[cfg(feature = "board-111")]
    111,
    #[cfg(feature = "board-mmc3-scramble")]
    114,
    #[cfg(feature = "board-mmc3-scramble")]
    115,
    #[cfg(feature = "board-116")]
    116,
    118,
    #[cfg(feature = "board-119")]
    119,
    #[cfg(feature = "board-mmc3-scramble")]
    121,
    #[cfg(feature = "board-mmc3-scramble")]
    123,
    #[cfg(feature = "board-jv001")]
    132,
    #[cfg(feature = "board-133")]
    133,
    #[cfg(feature = "board-jv001")]
    136,
    #[cfg(feature = "board-small-boards")]
    140,
    #[cfg(feature = "board-143")]
    143,
    #[cfg(feature = "board-144")]
    144,
    #[cfg(feature = "board-145")]
    145,
    #[cfg(feature = "board-nina03")]
    146,
    #[cfg(feature = "board-jv001")]
    147,
    #[cfg(feature = "board-149")]
    149,
    #[cfg(feature = "board-sachen-8reg")]
    150,
    // Mapper 151 needs the VRC1 **and** the VS layer: the number is the deprecated one for that
    // board on a VS. System board, so it is advertised only when both are built, which
    // `board-vs-system` implying `board-small-boards` is what guarantees.
    #[cfg(feature = "board-vs-system")]
    151,
    #[cfg(feature = "board-small-boards")]
    152,
    #[cfg(feature = "board-153")]
    153,
    #[cfg(feature = "board-namcot108")]
    154,
    #[cfg(feature = "board-155")]
    155,
    156,
    #[cfg(feature = "board-eeprom")]
    157,
    #[cfg(feature = "board-158")]
    158,
    #[cfg(feature = "board-eeprom")]
    159,
    #[cfg(feature = "board-jv001")]
    172,
    #[cfg(feature = "board-jv001")]
    173,
    180,
    #[cfg(feature = "board-mmc3-scramble")]
    182,
    #[cfg(feature = "board-small-boards")]
    184,
    #[cfg(feature = "board-small-boards")]
    185,
    #[cfg(feature = "board-mmc3-scramble")]
    187,
    #[cfg(feature = "board-189")]
    189,
    #[cfg(feature = "board-mmc3-chrram")]
    192,
    #[cfg(feature = "board-mmc3-chrram")]
    194,
    #[cfg(feature = "board-mmc3-chrram")]
    195,
    #[cfg(feature = "board-namcot108")]
    206,
    #[cfg(feature = "board-taito-x1")]
    207,
    210,
    225,
    228,
    #[cfg(feature = "board-small-boards")]
    232,
    #[cfg(feature = "board-sachen-8reg")]
    243,
    #[cfg(feature = "board-263")]
    263,
    #[cfg(feature = "board-taito-x1")]
    552,
];

/// Whether `mapper` is implemented.
#[must_use]
pub fn is_supported(mapper: i32) -> bool {
    SUPPORTED_MAPPERS.contains(&mapper)
}

/// The sizes the *board* dictates for the volatile memories, settled once at load time
/// (REQ-CORE-100).
///
/// `prg_ram` and `chr_ram` are exact: the board carries that much RAM and the header cannot
/// say otherwise. `min_prg_ram` is the floor for boards that bank *more* than the header's
/// 8 KiB without fixing a size (MMC1 SOROM and SXROM, MMC5).
// The three fields name the three memories; the `_ram` they share is the point.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BoardMemorySizes {
    pub prg_ram: Option<usize>,
    pub chr_ram: Option<usize>,
    pub min_prg_ram: usize,
}

/// Which of the two mapper-34 boards a dump is (nesdev, "INES Mapper 034"): "Submapper 1
/// denotes the NINA-001/NINA-002 circuit board", "Submapper 2 denotes the I-IM/BNROM circuit
/// board", and "In the absence of valid submapper information, mapper 34 .NES files should be
/// considered BNROM when the CHR-ROM size is 0-8 KiB, and NINA-001/NINA-002 when the CHR-ROM
/// size is above 8 KiB". The submapper takes precedence over the size (MR-34-1).
#[must_use]
pub(crate) fn mapper_34_is_nina(header: &InesHeader) -> bool {
    if header.is_nes20 && (header.submapper == 1 || header.submapper == 2) {
        return header.submapper == 1;
    }

    header.chr_rom_size > 8 * 1024
}

/// Whether a mapper-157 cartridge carries the Datach's **second**, external EEPROM.
///
/// nesdev ("INES Mapper 157"): "The NES 2.0 header's PRG-NVRAM field will only denote whether the
/// game cartridge has an additional 128-byte serial EEPROM (byte 10 value 0x10) or not (byte 10
/// value 0x00)." So the header is the only indicator, and an iNES 1.0 dump cannot declare it.
///
/// **One divergence from Mesen 2, in the direction AMD-061 §1 requires.** Mesen gives the second
/// chip to *any* dump that is not NES 2.0 (`BandaiFcg.h:55`, `!IsNes20() || … == 128`), which
/// hands a phantom chip to the six Datach games that have none — and that chip's output is
/// wire-ANDed with the internal one (see [`super::mappers::EepromPort`]), so a game writing
/// $8000-$8003 could clock it onto the bus. The page makes byte 10 the test; this build uses only
/// byte 10.
#[cfg(feature = "board-eeprom")]
fn datach_has_external_eeprom(header: &InesHeader) -> bool {
    header.is_nes20 && header.byte10 == 0x10
}

/// The volatile-memory sizes of `mapper`'s board for `header`.
#[must_use]
// One arm per board that dictates a size, each with the page that says so: a flat table, like
// `create_board` above it and `opcode_table`'s matrix. MF-VS step 4's mapper-99 arm is the one
// that crossed the threshold.
#[allow(clippy::too_many_lines)]
pub(crate) fn board_memory_sizes(mapper: i32, header: &InesHeader) -> BoardMemorySizes {
    match mapper {
        // MR-34-2, nesdev ("INES Mapper 034"): the NINA boards have "8 KiB of unbanked
        // PRG-RAM" at $6000-$7FFF and the BNROM has none, so that window is open bus there.
        34 if !mapper_34_is_nina(header) => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // MR-4-6, nesdev ("MMC6"): the chip "contains 1 KB of internal PRG RAM", at "CPU
        // $7000-$7FFF: 1 KB PRG RAM, mirrored". The header of a StarTropics dump describes
        // the 8 KiB every other mapper-4 board has.
        // Mapper 118 shares the MMC3's board but never its submapper 1: a TxSROM is not an
        // MMC6, so this arm is on mapper 4 alone.
        4 if header.is_nes20 && header.submapper == 1 => BoardMemorySizes {
            prg_ram: Some(Mmc3::MMC6_PRG_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        5 => BoardMemorySizes {
            min_prg_ram: Mmc5::MAX_PRG_RAM_SIZE,
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 099"): "CPU $6000-$7FFF: 2 KiB RAM", not the iNES default of 8 KiB
        // (`VsBoard::PRG_RAM_SIZE` has the rest, the `DualSystem`'s sharing of it included).
        #[cfg(feature = "board-vs-system")]
        99 => BoardMemorySizes {
            prg_ram: Some(VsBoard::PRG_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 119"): "Both TQROM games used 8 KiB CHR RAM", beside the CHR-ROM
        // the same board carries. **No header field can state it**: byte 5 of an iNES 1.0
        // header is the CHR-*ROM* size, and REQ-CORE-012's CHR-RAM default only applies to an
        // image with no CHR-ROM at all -- so a TQROM dump would come up with none. Mesen 2
        // agrees (`MapperFactory.cpp:404`, `MMC3_ChrRam(0x40, 0x7F, 8)` -- chip-select bit $40,
        // 8 KiB).
        #[cfg(feature = "board-119")]
        119 => BoardMemorySizes {
            chr_ram: Some(Tqrom::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // The other four members of the TQROM's family, for the arm above's reason in the same
        // words: the `Size of RAM` column of nesdev's *MMC3 with CHR ROM and CHR RAM* is the
        // chip the board carries beside its CHR-ROM, and **no header field can state it** --
        // byte 5 of an iNES 1.0 header is the CHR-*ROM* size and REQ-CORE-012's CHR-RAM default
        // only applies to an image with no CHR-ROM at all. 74 and 194 carry 2 KiB, 192 4 KiB,
        // and 195 **8 KiB**, which is its page's "32 KiB of CHR-RAM, although address lines for
        // only 8 KiB are connected" and not the width of its window (MF-UNL-2 step 2).
        //
        // 192's page adds the one case where the header does speak: "If the NES 2.0 header is
        // present, 74 and 192 should be emulated identically, as NES 2.0 specifies CHR RAM size
        // elsewhere." This build does not take that path -- the board's chip is the board's,
        // which is the rule every arm of this function follows, and the two numbers then differ
        // by the one thing their pages differ by. Q-143 records it.
        #[cfg(feature = "board-mmc3-chrram")]
        74 | 192 | 194 | 195 => BoardMemorySizes {
            chr_ram: Mmc3ChrRamBoard::chr_ram_size(mapper),
            ..BoardMemorySizes::default()
        },
        // Mappers 114, 115 and 182: the `$600x` pair **is** the board's decode of $6000-$7FFF,
        // so there is no RAM behind that window. Both pages put the registers under a mask that
        // leaves no room for any -- 114's "probably $E001" repeats $6000/$6001 every two bytes
        // through the whole 8 KiB and 115's $E003 repeats $6000-$6003 every four -- and both say
        // in the same words that "the $600x registers are **not connected to the MMC3 clone's
        // WRAM interface** and thus function regardless of whether WRAM is enabled or not", so
        // the window cannot be RAM that a WRAM enable gates. The same arm, and the same reason,
        // that mappers 47, 37, 140, 184, 150 and 243 are on above (MF-UNL-2 step 3).
        //
        // **121, 123 and 187 are deliberately absent**: every register of those three is
        // **below** $6000 -- $5000-$5FFF, $5800-$5FFF and $5000 -- so nothing forces the
        // question and the header decides, which is mapper 97's precedent in this function.
        #[cfg(feature = "board-mmc3-scramble")]
        114 | 115 | 182 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // Mappers 47 and 37: the outer register *is* the board's decode of $6000-$7FFF, so
        // there is no RAM behind that window. 037's page says so of the register itself -- it
        // "is not readable and attempting to read it will return open bus" -- and with no RAM
        // there `Cartridge::default_cpu_read` falls through to `Mapper::try_read_prg`, which
        // answers `None` below $8000. The same arm 140 and 184 are on, for the same reason.
        #[cfg(feature = "board-mmc3-multicart")]
        37 | 47 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 077"): the Irem LROG017 carries a RAM chip on the PPU bus beside
        // its CHR-ROM, and the page's implementation note sizes what an emulator must provide --
        // "just provide 10KiB from $0800-$2FFF". 6 KiB of it is this CHR-RAM, at $0800-$1FFF; the
        // 4 KiB of $2000-$2FFF is the four-screen nametable the board forces and this core's
        // console VRAM already holds (see `IremLrog017`'s own note on the split).
        //
        // **No header field can state it**, which is the TQROM's arm's reason above in the same
        // words: byte 5 of an iNES 1.0 header is the CHR-*ROM* size and REQ-CORE-012's CHR-RAM
        // default only applies to an image with no CHR-ROM at all. Mesen 2 agrees
        // (`IremLrog017.h:10`, `GetChrRamSize` 0x1800).
        #[cfg(feature = "board-77")]
        77 => BoardMemorySizes {
            chr_ram: Some(IremLrog017::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // MR-13-1, nesdev ("CPROM") and REQ-CORE-100's mapper table: the board carries "16 KiB
        // RAM (in two 8KiB RAMs)", not the header's 32 KiB default.
        13 => BoardMemorySizes {
            chr_ram: Some(CpRom::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("VRC3"): "PPU $0000-$1FFF: 8 KiB CHR RAM", unbanked, and the board has no CHR
        // bank line at all. REQ-CORE-012's default would give an iNES 1.0 Salamander dump 32 KiB
        // of it, three quarters of which the chip cannot address. Mesen 2 agrees
        // (`VRC3.h:17`, `GetChrPageSize` 0x2000 with one page selected).
        73 => BoardMemorySizes {
            chr_ram: Some(Vrc3::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("NES-EVENT"): "8K of CHR RAM", and Disch "this mapper has 8k CHR-RAM, and it
        // is not swappable". The same reading as mapper 73 above, for the same REQ-CORE-012
        // reason, and Mesen 2 agrees (`MMC1.h`'s 8 KiB CHR with one page selected).
        #[cfg(feature = "board-105")]
        105 => BoardMemorySizes {
            chr_ram: Some(NesEvent::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // The Namco 163 carries the four 2 KiB windows of $6000-$7FFF whatever the header
        // says, plus the chip's own 128 bytes behind them -- that RAM lives at the end of
        // PRG-RAM so that REQ-CORE-095's single battery slice carries it, which is also
        // Mesen's `.sav` layout. See `Namco163`'s own note.
        19 => BoardMemorySizes {
            prg_ram: Some(Namco163::PRG_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 210"): the Namcot 175 has "0-8 KiB" of PRG-RAM and the 340
        // "contains no WRAM". Neither has the chip's internal RAM.
        210 => BoardMemorySizes {
            prg_ram: Some(
                match Namco163::variant_of(210, if header.is_nes20 { header.submapper } else { 0 })
                {
                    NamcoVariant::Namco340 => 0,
                    _ => 4 * 0x800,
                },
            ),
            ..BoardMemorySizes::default()
        },
        // MF-6c step 3: the EEPROM bytes of mappers 157 and 159, in PRG-RAM so that
        // REQ-CORE-095's single battery slice carries them -- the internal chip first and the
        // Datach's external one behind it. The board carries no other RAM at $6000-$7FFF, and
        // `BandaiFcg::prg_ram_enabled` answers `false` on these two numbers, so nothing the CPU
        // can address lives here: the bytes are reached only through the serial protocol.
        #[cfg(feature = "board-eeprom")]
        157 | 159 => BoardMemorySizes {
            prg_ram: EepromPort::ram_size(mapper, datach_has_external_eeprom(header)),
            ..BoardMemorySizes::default()
        },
        // The two Taito boards whose RAM is inside the ASIC, exactly as the Namco 163's 128
        // bytes are (§4.6 of `docs/review/MF-6-inventory.md`): the size is the chip's and the
        // header cannot say otherwise, and because there is no header RAM in front of it
        // REQ-CORE-095's single battery slice *is* the chip's RAM.
        //
        // * 80 and 207, nesdev ("Taito X1-005"): "CPU $7F00-$7F7F: 128 B internal RAM",
        //   mirrored at $7F80-$7FFF, and no other RAM in $6000-$7FFF;
        // * 82 and 552, nesdev ("Taito X1-017"): three windows of 2 KiB, 2 KiB and 1 KiB, each
        //   behind its own permission register -- 5 KiB in the window order $6000 / $6800 /
        //   $7000, which is Mesen's `.sav` layout (`TaitoX1017.h:29`, `0x1400`).
        #[cfg(feature = "board-taito-x1")]
        80 | 207 => BoardMemorySizes {
            prg_ram: Some(TaitoX1005::RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-taito-x1")]
        82 | 552 => BoardMemorySizes {
            prg_ram: Some(TaitoX1017::RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("GTROM"): "The board contains 32KB of RAM" on the PPU side -- two 8 KiB CHR
        // banks and two 8 KiB nametable banks, in one allocation because the board has one
        // chip and REQ-CORE-090 has one blob for it (see `Cheapocabra`'s own note). No
        // PRG-RAM: the register is at $5000-$5FFF and $7000-$7FFF, so that window is the
        // register and Mesen 2 gives the board neither work nor save RAM (MF-6c step 9).
        #[cfg(feature = "board-111")]
        111 => BoardMemorySizes {
            prg_ram: Some(0),
            chr_ram: Some(Cheapocabra::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 096"): "PRG RAM capacity: No", and "CHR capacity: 32 KiB RAM" --
        // eight of the 4 KiB pages the board banks in. The CHR figure is the same one
        // REQ-CORE-012's default gives an image with no CHR-ROM, and it is stated anyway
        // because the page states it and no header field can (MF-6c step 8).
        #[cfg(feature = "board-96")]
        96 => BoardMemorySizes {
            prg_ram: Some(0),
            chr_ram: Some(OekaKids::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 086"): "No PRG RAM (mapper ports occupy $6000-$7FFF instead)".
        // With no RAM behind that window `Cartridge::cpu_write` walks past its PRG-RAM branch and
        // `Mapper::write_register` sees the address intact, which is how the JF-13 decodes its two
        // ports without an `intercept_cpu_write` body -- the same arm, and the same reason, that
        // mappers 47 and 37 are on above (MF-6c step 7).
        #[cfg(feature = "board-jaleco-72-92-86")]
        86 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // The three boards that carry no PRG-RAM at all, each on its page's own words:
        //
        // * 30, MR-30-2, nesdev ("UNROM 512"): "PRG RAM capacity: None";
        // * 140 ("INES Mapper 140"): "Regs lie at $6000-7FFF, so there's no SRAM";
        // * 184 ("INES Mapper 184"): "This mapper precludes iNES-default RAM because the
        //   register is mapped into $6000-7FFF".
        //
        // The last two answer that window themselves through `Mapper::intercept_cpu_write`, so
        // the RAM must not exist behind it. **Mapper 97 is deliberately absent**, although §4.6
        // of the inventory grouped it with those two: its page documents no PRG-RAM but does not
        // say the board has none, and taking RAM away from an image that uses it is the worse of
        // the two errors.
        30 | 140 | 184 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("NINA-003-006"), the one page of mappers 79 and 146: "PRG RAM capacity:
        // **None**", which is mapper 30's reason above and not mapper 140's -- the board's
        // register is at $4100 under mask $E100, so `$6100 & $E100` is `$6100` and nothing
        // forces the question; the page states it, so this does (MF-UNL-3 step 1).
        #[cfg(feature = "board-nina03")]
        79 | 146 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 038"): "PRG RAM capacity: **None**", and here the window is in
        // the way as well -- the board's "Bank select ($7000-$7FFF)" is the upper half of it.
        // This is mapper 140's and 184's arm above for both of their reasons at once
        // (MF-UNL-3 step 2).
        #[cfg(feature = "board-38")]
        38 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 206"), the Namco 108 family: `PRG RAM: None`, the page adding
        // "with the rare exception" of the *Magical Puzzle Popils* prototype, which carries
        // "8 KiB battery-backed". So the window is open bus on these five numbers unless the
        // image declares a battery, which is the one board the page excepts.
        #[cfg(feature = "board-namcot108")]
        76 | 88 | 95 | 154 | 206 if !header.has_battery => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 150" and "INES Mapper 243"): both ports of Sachen's eight-register
        // ASIC are decoded under **mask $C101**, which reads A15, A14, A8 and A0 and nothing
        // else, so the index and data registers repeat through the whole of $4100-$7FFF --
        // `$6100 & $C101` is `$4100`. The window has to be the registers and not RAM, or a game
        // writing its CHR bank at $6101 would write work RAM instead; neither page gives the
        // board any PRG-RAM. The same arm, and the same reason, that mappers 47, 37, 140, 184
        // and the Jaleco JF-13 are on above (MF-UNL-1 step 2).
        #[cfg(feature = "board-sachen-8reg")]
        150 | 243 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // **Mapper 133 is deliberately absent too**, and for the same reason: its latch is at
        // $4100 under mask $E100, which `$6100 & $E100` cannot be, and the page gives the Sachen
        // 72008 no PRG-RAM line either way (MF-UNL-1 step 3).
        //
        // **The six JV001 numbers are deliberately absent** (36, 132, 136, 147, 172, 173), and
        // the reason is mapper 97's above in the same words: not one of the six pages documents
        // any PRG-RAM, and not one of them says the board has none either. Nothing forces the
        // question — the chip's window is $4100-$4103 under mask $E103 and the board's own CHR
        // latch on mapper 36 is $4200 under $E200, so `$6100 & $E103` is `$6100` and no address
        // in $6000-$7FFF can reach a register of this family. The header decides, as it does for
        // mapper 97 (MF-UNL-1 step 1).
        //
        // **Mappers 107, 143, 144, 145 and 149 are deliberately absent too** (MF-UNL-3 steps 2
        // and 3), and again for mapper 97's reason: not one of their pages documents any
        // PRG-RAM and not one says the board has none. Nothing forces the question — 143's and
        // 145's ports are at $4100 under mask $E100 and 107's, 144's and 149's are above $8000 —
        // so the header decides. Mapper 144 is mapper 11's board, which has no arm here either.
        _ => BoardMemorySizes::default(),
    }
}

/// Where in PRG-RAM the battery-backed region begins on `mapper`'s board, once the board has
/// `prg_ram_size` bytes of it. Everything from there to the end is what a host persists.
///
/// MR-1-5, nesdev ("MMC1"): "Even if the SOROM and SZROM boards utilizes a battery, it is
/// connected to only one PRG-RAM chip. The first RAM chip will not retain its data, but the
/// second one will." SOROM and SZROM are the 16 KiB boards, so the chip that retains is the
/// second 8 KiB.
///
/// **AMD-061 §8 closes Q-113 on the reading MF-1 chose**, and in these words: SOROM and SZROM
/// put "seule la seconde puce, $2000-$3FFF de la PRG-RAM" on the battery; **SXROM** carries
/// "une seule puce de 32 Kio, entièrement sur pile", so `battery_ram` exposes all 32 KiB and
/// *Final Fantasy I·II* keeps its saves; and "une image dont la taille de RAM ne permet pas de
/// distinguer la carte garde toute sa PRG-RAM sur pile". The function below is exactly that:
/// a 16 KiB mapper-1 RAM retains its second half, every other size retains all of itself. No
/// code changed in MF-2 — the amendment confirmed what was already here — and
/// `mr_1_5_only_the_second_sorom_ram_chip_is_battery_backed` and
/// `mr_1_5_an_sxrom_keeps_its_whole_ram_battery_backed` pin both halves.
#[must_use]
pub(crate) const fn battery_offset(mapper: i32, prg_ram_size: usize) -> usize {
    if mapper == 1 && prg_ram_size == 2 * BankWindows::PRG_WINDOW_SIZE {
        BankWindows::PRG_WINDOW_SIZE
    } else {
        0
    }
}

/// Whether `mapper`'s board lets the ROM drive the data bus during a write to $8000-$FFFF, so
/// that the value the register latches is "the bitwise AND of the written value and the value
/// in ROM" (MR-BUS-1, MR-66-1, the bus-conflict half of MR-34-2). Decided once at load.
///
/// Sources, all nesdev: *NES 2.0 submappers* gives mappers 2, 3 and 7 the AND on **submapper
/// 2** and says "Bus conflicts do not occur" on submapper 1; *INES Mapper 011* says "All
/// commercially-released games are subject to bus conflicts (submapper 2)", which with the
/// *Color Dreams* board page (only the *Free Fall* prototype has conflict-prevention logic)
/// makes it unconditional there; *UNROM 512* makes submapper 2 "Bus conflicts present" and for
/// submapper 0 says "The Battery bit must be specified if the game relies on bus conflicts not
/// being present", so a submapper-0 dump without battery has them; *INES Mapper 034* gives
/// BNROM's register "the value being written bitwise-AND'd with the PRG-ROM content", and only
/// BNROM — the NINA boards have conflict-free latches; and the `GxROM` board table gives mapper
/// 66 conflicts (MR-66-1), which Mesen does not implement.
///
/// **A submapper-0 dump of mappers 2, 3 and 7 has none**, which is what
/// `mr_bus_1_uxrom_without_a_submapper_has_no_conflict` pins and what MR-1b asked be kept: the
/// submapper is the gate, and most dumps of those boards carry no submapper at all.
///
/// The gate is [`InesHeader::has_submapper`] and not `is_nes20` because of the one other source
/// of a submapper this core has: the VS. System table, whose two `UxROM` rows the public NES 2.0
/// database records as **submapper 2** (lot MF-VS-fix step 2). In the default build that method
/// *is* `is_nes20`.
#[must_use]
pub(crate) fn has_bus_conflicts(mapper: i32, header: &InesHeader) -> bool {
    let submapper = if header.has_submapper() {
        header.submapper
    } else {
        0
    };
    match mapper {
        2 | 3 | 7 => submapper == 2,
        // *INES Mapper 180* gives the board "Bus conflicts: Yes" with no submapper clause: the
        // AND logic that makes its fixed bank the first one is what puts the ROM on the bus
        // during a write.
        // *INES Mapper 078* says the same of its board, "a CNROM and a UNROM" in discrete
        // logic with nothing to stop the ROM driving the bus.
        // *INES Mapper 089* and *INES Mapper 093* both carry the page's own "BUS CONFLICTS"
        // line against the register at $8000-$FFFF (MF-6a step 2).
        // *INES Mapper 070* and *INES Mapper 152* file the Bandai 74161/7432 board under
        // "Mappers with bus conflicts" too (MF-6a step 3).
        // Mesen 2 gives mapper 185 the CNROM board's conflicts (`CnromProtect.h:21`), which
        // the *CNROM* board page describes for the family (MF-6a step 4).
        // *INES Mapper 072* gives Jaleco's JF-17 "Bus conflicts: Yes" against its port at
        // $8000-$FFFF, and *INES Mapper 092* is "identical to mapper 072 except for the
        // different PRG Setup", so the JF-19 has them too (MF-6c step 7). Mesen 2 agrees for
        // both (`JalecoJf17_19.h:19`). **Mapper 86 is deliberately absent**: its page gives the
        // JF-13 no bus-conflict line and its ports are below $8000, where the PRG-ROM does not
        // drive -- the accidental copies at $E000-$FFFF are the board's missing address line and
        // not its output enable. Mesen 2 does not give it conflicts either.
        #[cfg(feature = "board-jaleco-72-92-86")]
        72 | 92 => true,
        // *INES Mapper 096* says "Bus conflicts: **partly**", and this build has one bool per
        // cartridge (MR-BUS-1), so "partly" has to become yes or no. **Yes**, which is Mesen
        // 2's reading (`OekaKids.h:14`, `HasBusConflicts` true); Q-135 holds the two readings
        // and the argument (MF-6c step 8).
        #[cfg(feature = "board-96")]
        96 => true, // Q-135
        // *INES Mapper 077* gives the Irem LROG017 "Bus conflicts: yes" against its one register
        // at $8000-$FFFF: two 74161s latch the write and nothing on the board disables the PRG
        // ROM's output enable while they do (MF-6c step 6). Mesen 2 agrees
        // (`IremLrog017.h:12`, `HasBusConflicts` true).
        #[cfg(feature = "board-77")]
        77 => true,
        // *INES Mapper 149* is "just like mapper 145, but moves the register to overlap with PRG
        // ROM and so **has bus conflicts**" -- the page's own words, with no submapper clause
        // and nothing qualifying them (MF-UNL-3 step 2).
        //
        // **Mappers 38, 79, 107, 143 and 145 are deliberately absent.** 38, 79 and 145 carry the
        // row "Bus conflicts: No" and keep their register below $8000, where the PRG-ROM does not
        // drive; 143 has no write side at all; 107's page says "it's currently **unknown**
        // whether this mapper suffers from bus conflicts or not", answered **no** here because
        // that is the reading which latches what a game wrote, and recorded in Q-147.
        //
        // **And mapper 144 is absent for the opposite reason**: that board *does* have bus
        // conflicts, of a shape this function cannot express. Its 300 Ω resistor makes the ROM's
        // **bit 0** win whatever the CPU drove, and the generic path below would AND that bit
        // away before the board saw it, so `SingleLatch` reads the ROM byte and does the
        // arithmetic itself (`LatchBoard::Agci144`, MF-UNL-3 step 3).
        #[cfg(feature = "board-149")]
        149 => true,
        11 | 66 | 70 | 78 | 89 | 93 | 152 | 180 | 185 => true,
        30 => submapper == 2 || (submapper == 0 && !header.has_battery),
        34 => !mapper_34_is_nina(header),
        _ => false,
    }
}

/// Builds the board of `mapper` for `memory`, or `None` when it is not implemented.
#[must_use]
// One arm per board, each with the lot that added it and the page it is built from: a flat
// table, as `board_memory_sizes` above is. MF-UNL-1 step 1's six-number arm is the one that
// crossed the threshold.
#[allow(clippy::too_many_lines)]
pub(crate) fn create_board(mapper: i32, memory: &CartridgeMemory) -> Option<Board> {
    Some(match mapper {
        0 => Board::Nrom(Nrom),
        // MF-6a step 4: mapper 155 is the MMC1 on a board with no PRG-RAM disable wire.
        #[cfg(feature = "board-155")]
        155 => Board::Mmc1(Mmc1::new(memory)),
        1 => Board::Mmc1(Mmc1::new(memory)),
        // MF-6a step 4: mapper 94 is the third wiring of the UxROM board, its bank field
        // shifted two bits (nesdev, "INES Mapper 094").
        #[cfg(feature = "board-94")]
        94 => Board::UxRom(UxRom::new(memory, mapper)),
        2 | 180 => Board::UxRom(UxRom::new(memory, mapper)),
        3 => Board::CnRom(CnRom::new(memory)),
        // MR-4-3: mapper 4 submapper 3 is Acclaim's MC-ACC, the same board with a ÷8 prescaler
        // on A12's falling edges. It is the same code, monomorphised with the difference as a
        // `const` parameter, so the MMC3 proper pays nothing (see `Mmc3Board`).
        4 if memory.rom().header().is_nes20 && memory.rom().header().submapper == 3 => {
            Board::McAcc(McAcc::new(memory, mapper))
        }
        4 | 118 => Board::Mmc3(Mmc3::new(memory, mapper)),
        5 => Board::Mmc5(Mmc5::new(memory)),
        7 => Board::AxRom(AxRom::new(memory)),
        9 | 10 => Board::Mmc2(Mmc2::new(memory, mapper)),
        11 => Board::ColorDreams(ColorDreams::new(memory)),
        13 => Board::CpRom(CpRom::new(memory)),
        15 => Board::Mapper015(Mapper015::new(memory)),
        // MF-6c step 1: mapper 153 is the same LZ93D50 on a board with 8 KiB of battery-backed
        // WRAM, an outer PRG bank on $8000-$8003 and no CHR banking. A parameter of mapper 16's
        // board and not a variant of its own; see `BandaiFcg`'s own note.
        #[cfg(feature = "board-153")]
        153 => Board::BandaiFcg(BandaiFcg::new(memory, mapper)),
        // MF-6c step 3: the two numbers with a serial EEPROM, which is one `Option` field on the
        // same board -- see `EepromPort`'s own note on the Datach's two chips and on the three
        // things this build records rather than implements.
        #[cfg(feature = "board-eeprom")]
        157 | 159 => Board::BandaiFcg(BandaiFcg::new(memory, mapper)),
        16 => Board::BandaiFcg(BandaiFcg::new(memory, mapper)),
        18 => Board::JalecoSs88006(JalecoSs88006::new(memory)),
        21 | 22 | 23 | 25 | 27 => Board::Vrc2Vrc4(Vrc2Vrc4::new(memory, mapper)),
        // Mapper 26 is the VRC6b, the same chip with A0 and A1 swapped (nesdev, "VRC6").
        19 | 210 => Board::Namco163(Namco163::new(memory, mapper)),
        24 | 26 => Board::Vrc6(Vrc6::new(memory, mapper)),
        28 => Board::Action53(Action53::new(memory)),
        30 => Board::UnRom512(UnRom512::new(memory)),
        32 => Board::IremG101(IremG101::new(memory)),
        34 => Board::Mapper034(Mapper034::new(memory)),
        41 => Board::Caltron6In1(Caltron6In1::new(memory)),
        64 => Board::Rambo1(Rambo1::new(memory)),
        66 => Board::GxRom(GxRom::new(memory)),
        69 => Board::Fme7(Fme7::new(memory)),
        // Mapper 85 is the VRC7; its submapper picks the register-select address line.
        85 => Board::Vrc7(Vrc7::new(memory)),
        78 => Board::Mapper078(Mapper078::new(memory)),
        71 => Board::Codemasters(Codemasters::new(memory)),
        87 | 101 => Board::JalecoJ87(JalecoJ87::new(memory, mapper)),
        156 => Board::Daou306(Daou306::new(memory)),
        225 => Board::Multicart225(Multicart225::new(memory)),
        228 => Board::Action52(Action52::new(memory)),
        // MF-6a step 1: the Namco 108 and its four parameters. One board, five numbers; see
        // `Namcot108`'s own note on why the differences are fields and not `const` parameters.
        #[cfg(feature = "board-namcot108")]
        76 | 88 | 95 | 154 | 206 => Board::Namcot108(Namcot108::new(memory, mapper)),
        // MF-6a step 2: five boards that are a register decode and nothing else, sharing one
        // `Board` discriminant. `SmallBoard`'s note has the measurement that put them there.
        #[cfg(feature = "board-small-boards")]
        33 => Board::SmallBoard(SmallBoard::TaitoTc0190(TaitoTc0190::new(memory))),
        #[cfg(feature = "board-small-boards")]
        97 => Board::SmallBoard(SmallBoard::IremTamS1(IremTamS1::new(memory))),
        #[cfg(feature = "board-small-boards")]
        140 => Board::SmallBoard(SmallBoard::JalecoJf11(JalecoJf11::new(memory))),
        #[cfg(feature = "board-small-boards")]
        184 => Board::SmallBoard(SmallBoard::Sunsoft1(Sunsoft1::new(memory))),
        // Mapper 89 is the Sunsoft-3 board and 93 the Sunsoft-3R; the chip is the same.
        #[cfg(feature = "board-small-boards")]
        89 | 93 => Board::SmallBoard(SmallBoard::Sunsoft2(Sunsoft2::new(memory, mapper))),
        // MF-6a step 3: four more numbers in the same variant, so no new discriminant.
        // Mapper 152 is the same board as 70 with its mirroring wire connected.
        #[cfg(feature = "board-small-boards")]
        70 | 152 => Board::SmallBoard(SmallBoard::Bandai74161(Bandai74161::new(memory, mapper))),
        #[cfg(feature = "board-small-boards")]
        // MF-VS step 4: mapper 151 is the **same** VRC1 on a VS. System board, and the public
        // NES 2.0 header database agrees -- it records *Vs. Gradius* and *Vs. The Goonies*, the
        // two titles the number exists for, as mapper 75 with console type 1. So the number is a
        // parameter of the board and not a board: `Vrc1` reports whichever of the two it was
        // built for, the VS layer comes from `crate::vs::VsSystem::detect` (which treats both
        // numbers as a VS image whatever the console type says), and `SmallBoard` grows no
        // variant.
        75 => Board::SmallBoard(SmallBoard::Vrc1(Vrc1::new(memory, mapper))),
        #[cfg(feature = "board-vs-system")]
        151 => Board::SmallBoard(SmallBoard::Vrc1(Vrc1::new(memory, mapper))),
        #[cfg(feature = "board-small-boards")]
        232 => Board::SmallBoard(SmallBoard::Bf9096(Bf9096::new(memory))),
        // MF-6a step 4: CNROM with the security diodes. `CnRomProtect`'s note says why it
        // is here and not a parameter of `CnRom`.
        #[cfg(feature = "board-small-boards")]
        185 => Board::SmallBoard(SmallBoard::CnRomProtect(CnRomProtect::new(memory))),
        // MF-6c step 4: the Sunsoft-4. Its CIRAM half is a register decode; its CHR-ROM
        // nametable mode is Q-123's third `read_nametable` body and has a feature of its own.
        #[cfg(feature = "board-68")]
        68 => Board::IrqBoard(IrqBoard::Sunsoft4(Sunsoft4::new(memory))),
        // MF-6c step 2: the two Taito boards whose WRAM is inside the ASIC. Mapper 207 is the
        // X1-005 with CHR A17 on CIRAM A10 and 552 the X1-017's corrected bank order, so each
        // chip is one board with one parameter. They are `SmallBoard` variants because both
        // pages say "IRQ: None", which is that enum's entry condition.
        #[cfg(feature = "board-taito-x1")]
        80 | 207 => Board::SmallBoard(SmallBoard::TaitoX1005(TaitoX1005::new(memory, mapper))),
        #[cfg(feature = "board-taito-x1")]
        82 | 552 => Board::SmallBoard(SmallBoard::TaitoX1017(TaitoX1017::new(memory, mapper))),
        // MF-6c step 6: the Irem LROG017, first member of the **second** enum of boards that are
        // a register decode and nothing else. `DecodeBoard`'s own note has R0's decision of
        // 2026-11-06 that put it in a new `Board` discriminant rather than in `SmallBoard`, which
        // steps 2 to 5 measured as full.
        #[cfg(feature = "board-77")]
        77 => Board::DecodeBoard(DecodeBoard::IremLrog017(IremLrog017::new(memory))),
        // MF-6c step 7: Jaleco's three boards with a uPD7756C speech chip, which this build does
        // not implement (§4.8). Mappers 72 and 92 are the JF-17 and the JF-19, one board whose
        // `is_jf19` bool picks which PRG window the latch drives; mapper 86 is the JF-13, a
        // different board with the same chip, so it is a second variant and not a parameter.
        #[cfg(feature = "board-jaleco-72-92-86")]
        72 | 92 => Board::DecodeBoard(DecodeBoard::JalecoJf17(JalecoJf17::new(memory, mapper))),
        #[cfg(feature = "board-jaleco-72-92-86")]
        86 => Board::DecodeBoard(DecodeBoard::JalecoJf13(JalecoJf13::new(memory))),
        // MF-6c step 8: the Oeka Kids board, whose 4 KiB CHR bank is latched off the PPU
        // address bus -- the first board in this core whose whole mechanism is that bus,
        // including a bare $2006 write with no fetch behind it. Its tablet is a recorded gap.
        #[cfg(feature = "board-96")]
        96 => Board::DecodeBoard(DecodeBoard::OekaKids(OekaKids::new(memory))),
        // MF-6c step 9: the GTROM / Cheapocabra, whose two 8 KiB nametable banks are the
        // fourth distinct body of the `Mapper::read_nametable` dispatch (4.5, Q-123) and
        // whose PRG-ROM is an SST39SF040 the game writes into (Q-117, MR-30-4).
        #[cfg(feature = "board-111")]
        111 => Board::DecodeBoard(DecodeBoard::Cheapocabra(Cheapocabra::new(memory))),
        // MF-UNL-1 step 1: the TXC 05-00002-010 / Sachen JV001 protection ASIC, one chip on six
        // boards from four publishers. The number picks the wiring -- the CPU data pins the
        // chip's six are on, the accumulator width, which Output bits reach which banking pin,
        // and whether a $8000 write drives the mirroring -- and `Jv001`'s own note has the
        // table. One `DecodeBoard` variant for all six, on the `Namcot108` precedent.
        #[cfg(feature = "board-jv001")]
        36 | 132 | 136 | 147 | 172 | 173 => {
            Board::DecodeBoard(DecodeBoard::Jv001(Jv001::new(memory, mapper)))
        }
        // MF-UNL-1 step 2: Sachen's eight-register ASIC. The number picks which CHR pins R2, R4
        // and R6 drive and nothing else, so it is one board with one `bool`.
        #[cfg(feature = "board-sachen-8reg")]
        150 | 243 => Board::DecodeBoard(DecodeBoard::Sachen8Reg(Sachen8Reg::new(memory, mapper))),
        // MF-UNL-1 step 3: the Sachen 72008's one latch. The page's 60-pin decode -- a Namco-108
        // clone with an analog feedback path -- is deliberately not built; `Sachen72008`'s own
        // note has the page's sentence that settles it.
        #[cfg(feature = "board-133")]
        133 => Board::DecodeBoard(DecodeBoard::Sachen72008(Sachen72008::new(memory))),
        // MF-UNL-3 step 1: the AVE NINA-03/06 and the Sachen 3015 / SA-016, which nesdev's one
        // page for both numbers calls "functionally identical". One latch at $4100 under mask
        // $E100, and FCEUX's extra CNROM switch at $8000-$FFFF deliberately left out --
        // `Nina03`'s own note has the page's last line, which attributes that latch to mapper
        // 148 and gives this board no bus conflicts.
        #[cfg(feature = "board-nina03")]
        79 | 146 => Board::DecodeBoard(DecodeBoard::Nina03(Nina03::new(memory, mapper))),
        // MF-UNL-3 step 2: the four plain latches of the lot's seven single-latch boards, one
        // `DecodeBoard` variant between them with a runtime field saying which. `SingleLatch`'s
        // own note has the table of the four pages.
        #[cfg(feature = "board-38")]
        38 => Board::DecodeBoard(DecodeBoard::SingleLatch(SingleLatch::new(
            memory,
            LatchBoard::BitCorp38,
        ))),
        #[cfg(feature = "board-107")]
        107 => Board::DecodeBoard(DecodeBoard::SingleLatch(SingleLatch::new(
            memory,
            LatchBoard::Magic107,
        ))),
        // MF-UNL-3 step 3: the two boards of that variant that are not a bank latch. 143 has no
        // write side at all -- its one register is a protection *read* at $4100 -- and 144 is
        // mapper 11's board with a resistor on CPU D0, so it does its own bus-conflict
        // arithmetic rather than taking `has_bus_conflicts`'s generic AND.
        #[cfg(feature = "board-143")]
        143 => Board::DecodeBoard(DecodeBoard::SingleLatch(SingleLatch::new(
            memory,
            LatchBoard::Tca143,
        ))),
        #[cfg(feature = "board-144")]
        144 => Board::DecodeBoard(DecodeBoard::SingleLatch(SingleLatch::new(
            memory,
            LatchBoard::Agci144,
        ))),
        #[cfg(feature = "board-145")]
        145 => Board::DecodeBoard(DecodeBoard::SingleLatch(SingleLatch::new(
            memory,
            LatchBoard::Sachen145,
        ))),
        #[cfg(feature = "board-149")]
        149 => Board::DecodeBoard(DecodeBoard::SingleLatch(SingleLatch::new(
            memory,
            LatchBoard::Sachen149,
        ))),
        // MF-VS step 4: the VS. System board, whose one register is bit 2 of a `$4016` write --
        // a CPU register, forwarded here by `NesBus::write_internal` for a VS image and by
        // nothing else.
        #[cfg(feature = "board-vs-system")]
        99 => Board::DecodeBoard(DecodeBoard::VsBoard(VsBoard::new(memory))),
        // MF-6b step 1: the boards whose counter can pull /IRQ, behind one discriminant.
        // `IrqBoard`'s note has the measurement that put them there.
        73 => Board::IrqBoard(IrqBoard::Vrc3(Vrc3::new(memory))),
        // MF-6b step 2: the two plain CPU-cycle down counters, in the same variant. They are
        // the same mechanism with different wrap rules; `Sunsoft3`'s own note has the two
        // differences.
        65 => Board::IrqBoard(IrqBoard::IremH3001(IremH3001::new(memory))),
        67 => Board::IrqBoard(IrqBoard::Sunsoft3(Sunsoft3::new(memory))),
        // MF-6b step 3: the NES-EVENT, which holds an `Mmc1` rather than parameterising one --
        // `NesEvent`'s own note has the measurement that decided that.
        #[cfg(feature = "board-105")]
        105 => Board::IrqBoard(IrqBoard::NesEvent(NesEvent::new(memory))),
        // MF-6b step 4: the TC0690, the TC0190 of mapper 33 with the MMC3's scanline counter
        // and a four-cycle delay on its assertion. `TaitoTc0690`'s own note has the three
        // readings where it leaves Mesen 2 for the page.
        48 => Board::IrqBoard(IrqBoard::TaitoTc0690(TaitoTc0690::new(memory))),
        // MF-6b step 5: Tengen's 800037 is the RAMBO-1 with CHR A17 on CIRAM A10, which is a
        // `const` parameter of the same board -- see `Rambo1Board`'s own note.
        #[cfg(feature = "board-158")]
        158 => Board::IrqBoard(IrqBoard::Tengen800037(Tengen800037::new(memory))),
        // MF-6a3 step 1: the TQROM, a `const` parameter of the MMC3's board -- see
        // `Mmc3Board`'s own note on the six shapes MF-6a2 step 3 measured for it. It goes in
        // `IrqBoard` rather than in a `Board` variant of its own, and `IrqBoard`'s note has the
        // +0.080 % on `mmc5` that two more discriminants cost.
        // MF-6a3 step 4: the Gouder SOMARI-P, three chips in one board. `Somari`'s own note
        // has the supervisor register, the four submappers and the two Q-132 readings.
        #[cfg(feature = "board-116")]
        116 => Board::IrqBoard(IrqBoard::Somari(Box::new(Somari::new(memory)))),
        #[cfg(feature = "board-119")]
        119 => Board::IrqBoard(IrqBoard::Tqrom(Tqrom::new(memory, mapper))),
        // MF-UNL-2 step 2: the other four numbers of that same family, one `const`
        // monomorphisation each behind one `IrqBoard` variant -- see `Mmc3ChrRamBoard`'s own
        // note. 195 is the only one whose window moves at run time (its GAL, Q-142).
        #[cfg(feature = "board-mmc3-chrram")]
        74 | 192 | 194 | 195 => Board::IrqBoard(IrqBoard::Mmc3ChrRam(Box::new(
            Mmc3ChrRamBoard::new(memory, mapper)?,
        ))),
        // MF-UNL-2 step 3: the six MMC3 clones with scrambled registers, an outer CHR bank and
        // an NROM-like PRG override. One board with six wirings -- the number picks the two
        // scramble rows, the override's port and bit order, whether CHR A18 follows PPU A12 and
        // whether the counter is the MMC3A's -- and `Mmc3Scramble`'s own note has the table.
        #[cfg(feature = "board-mmc3-scramble")]
        114 | 115 | 121 | 123 | 182 | 187 => Board::IrqBoard(IrqBoard::Mmc3Scramble(Box::new(
            Mmc3Scramble::new(memory, mapper)?,
        ))),
        // MF-UNL-2 step 4: the Rex Soft UNL-KOF97, an MMC3 with its register address and its
        // written data permuted and nothing else. A12 drives the chip's A0.
        #[cfg(feature = "board-263")]
        263 => Board::IrqBoard(IrqBoard::Mmc3Kof97(Mmc3Kof97::new(memory))),
        // MF-UNL-2 step 5: the TXC 01-22017/01-22018 and the Yoko FC-001EM, an MMC3 whose
        // PRG-ROM is banked in 32 KiB amounts from $4100. The generic pirate variants of the
        // same number are deliberately not decoded; `Mmc3Txc189`'s own note has the page's
        // paragraph that settles it.
        #[cfg(feature = "board-189")]
        189 => Board::IrqBoard(IrqBoard::Mmc3Txc189(Mmc3Txc189::new(memory))),
        // MF-6a3 step 1: Nintendo's two MMC3 multicarts in one monomorphisation. The two
        // numbers differ only in what the outer register's bits drive, which is a field test
        // in `Mmc3Board::multicart_prg` and nowhere on a fetch or a cycle path.
        #[cfg(feature = "board-mmc3-multicart")]
        37 | 47 => Board::IrqBoard(IrqBoard::Mmc3Multicart(Mmc3Multicart::new(memory, mapper))),
        _ => return None,
    })
}
