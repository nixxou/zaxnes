//! Maps an iNES mapper number to its board (port of `MapperRegistry`, REQ-CORE-100,
//! REQ-CORE-102).
//!
//! The registry only advertises the boards that are implemented, so that a ROM is never
//! accepted and then found unplayable. The table is the one of REQ-CORE-100; several numbers
//! share a board when they are the same silicon differently wired (9 and 10, 21/22/23/25/27,
//! 87 and 101). 156 is outside that table: it runs Metal Force (Korea), a widescreen target.

#[cfg(feature = "board-530")]
use super::mappers::Ax5705;
#[cfg(feature = "board-188")]
use super::mappers::BandaiKaraoke;
#[cfg(feature = "board-111")]
use super::mappers::Cheapocabra;
#[cfg(feature = "board-266")]
use super::mappers::CityFighter;
#[cfg(feature = "board-cony")]
use super::mappers::Cony;
#[cfg(feature = "board-decode-boards")]
use super::mappers::DecodeBoard;
#[cfg(feature = "board-284")]
use super::mappers::Drip;
#[cfg(feature = "board-eeprom")]
use super::mappers::EepromPort;
#[cfg(feature = "board-323")]
use super::mappers::FaridSlrom;
#[cfg(feature = "board-176")]
use super::mappers::Fk23c;
#[cfg(feature = "board-huang1")]
use super::mappers::Huang1;
#[cfg(feature = "board-77")]
use super::mappers::IremLrog017;
#[cfg(feature = "board-jv001")]
use super::mappers::Jv001;
#[cfg(feature = "board-jy-asic")]
use super::mappers::JyCompany;
#[cfg(feature = "board-303")]
use super::mappers::Kaiser7017;
#[cfg(feature = "board-218")]
use super::mappers::MagicFloor;
#[cfg(feature = "board-91")]
use super::mappers::Mapper091;
#[cfg(feature = "board-234")]
use super::mappers::Maxi15;
#[cfg(feature = "board-111")]
use super::mappers::Mmc1Parallel;
#[cfg(feature = "board-219")]
use super::mappers::Mmc3A9461;
#[cfg(feature = "board-mmc3-chrram")]
use super::mappers::Mmc3ChrRamBoard;
#[cfg(feature = "board-292")]
use super::mappers::Mmc3Dragonfighter;
#[cfg(feature = "board-208")]
use super::mappers::Mmc3Gouder208;
#[cfg(feature = "board-263")]
use super::mappers::Mmc3Kof97;
#[cfg(feature = "board-mmc3-multicart")]
use super::mappers::Mmc3Multicart;
#[cfg(feature = "board-268")]
use super::mappers::Mmc3Outer;
#[cfg(feature = "board-mmc3-scramble")]
use super::mappers::Mmc3Scramble;
#[cfg(feature = "board-262")]
use super::mappers::Mmc3Shero;
#[cfg(feature = "board-189")]
use super::mappers::Mmc3Txc189;
#[cfg(feature = "board-215")]
use super::mappers::Mmc3Unl8237;
#[cfg(feature = "board-namcot108")]
use super::mappers::Namcot108;
#[cfg(feature = "board-105")]
use super::mappers::NesEvent;
#[cfg(feature = "board-nina03")]
use super::mappers::Nina03;
use super::mappers::Nrom;
#[cfg(feature = "board-96")]
use super::mappers::OekaKids;
#[cfg(feature = "board-164")]
use super::mappers::Pec9588;
#[cfg(feature = "board-168")]
use super::mappers::Racermate;
#[cfg(feature = "board-682")]
use super::mappers::Rainbow;
#[cfg(feature = "board-133")]
use super::mappers::Sachen72008;
#[cfg(feature = "board-sachen8259")]
use super::mappers::Sachen8259;
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
#[cfg(feature = "board-252")]
use super::mappers::Waixing252;
#[cfg(feature = "board-253")]
use super::mappers::Waixing253;
#[cfg(feature = "board-178")]
use super::mappers::WaixingFs305;
#[cfg(feature = "board-246")]
use super::mappers::G0151;
#[cfg(feature = "board-529")]
use super::mappers::T230;
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
#[cfg(feature = "board-chip-switch")]
use super::mappers::{ChipSwitch, ChipSwitchKind};
#[cfg(any(feature = "board-162", feature = "board-163"))]
use super::mappers::{ChrWindowBoard, ChrWindowKind};
#[cfg(feature = "board-fine-rom")]
use super::mappers::{FineRom, FineRomKind};
#[cfg(feature = "board-irq-latch")]
use super::mappers::{IrqLatch, IrqLatchKind};
#[cfg(feature = "board-irq-p")]
use super::mappers::{IrqP, IrqPKind};
#[cfg(feature = "board-jaleco-72-92-86")]
use super::mappers::{JalecoJf13, JalecoJf17};
#[cfg(any(
    feature = "board-29",
    feature = "board-38",
    feature = "board-107",
    feature = "board-143",
    feature = "board-144",
    feature = "board-145",
    feature = "board-149",
    feature = "board-190"
))]
use super::mappers::{LatchBoard, SingleLatch};
#[cfg(feature = "board-latch-multi")]
use super::mappers::{LatchKind, LatchMulti};
#[cfg(feature = "board-misc-n")]
use super::mappers::{MiscKind, MiscN};
#[cfg(feature = "board-mmc1-outer")]
use super::mappers::{Mmc1Outer, Mmc1OuterKind};
#[cfg(feature = "board-mmc3-clones")]
use super::mappers::{Mmc3Clone, Mmc3CloneKind};
#[cfg(feature = "board-mmc3-p")]
use super::mappers::{Mmc3P, Mmc3PKind};
#[cfg(feature = "board-multi-p")]
use super::mappers::{MultiP, MultiPKind};
#[cfg(feature = "board-taito-x1")]
use super::mappers::{TaitoX1005, TaitoX1017};
#[cfg(feature = "board-vrc-p")]
use super::mappers::{VrcP, VrcPKind};
#[cfg(feature = "board-111")]
use super::FlashRom;
use super::{BankWindows, Board, CartridgeMemory};
use crate::rom::InesHeader;

/// The number whose board a number runs (lot MAPPERS-P): puNES's `mappers.c` dispatches **248** to
/// `map_init_115` (nesdev, "INES Mapper 248": a duplicate of 115) and **536** to `map_init_195`
/// (the Waixing FS303 board). `Cartridge::create` builds the board, its memory and its latch
/// lists from this number and keeps the header's for itself.
#[must_use]
pub(crate) const fn board_number(mapper: i32) -> i32 {
    match mapper {
        #[cfg(feature = "board-248")]
        248 => 115,
        #[cfg(feature = "board-536")]
        536 => 195,
        // MAPPERS-P: puNES `map_init_532` is `map_init_019` and the font overlay
        // (`crate::ersan2_overlay`).
        #[cfg(feature = "board-532")]
        532 => 19,
        _ => mapper,
    }
}

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
    #[cfg(feature = "board-6")]
    6,
    7,
    #[cfg(feature = "board-8")]
    8,
    9,
    10,
    11,
    // The Gouder SL-5020B, the Huang-1 ASIC "exclusively as an MMC3A clone" plus a GAL that adds
    // an outer CHR register at $4132 (MF-UNL-8 step 1).
    #[cfg(feature = "board-huang1")]
    12,
    13,
    // The Gouder SL-1632, the same ASIC with a PAL that moves the supervisor register to $A131 and
    // gives it two modes and three CHR A18 bits (MF-UNL-8 step 2).
    #[cfg(feature = "board-huang1")]
    14,
    15,
    16,
    #[cfg(feature = "board-17")]
    17,
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
    #[cfg(feature = "board-29")]
    29,
    30,
    #[cfg(feature = "board-31")]
    31,
    32,
    #[cfg(feature = "board-small-boards")]
    33,
    34,
    #[cfg(feature = "board-jy-asic")]
    35,
    #[cfg(feature = "board-jv001")]
    36,
    #[cfg(feature = "board-mmc3-multicart")]
    37,
    #[cfg(feature = "board-38")]
    38,
    #[cfg(feature = "board-39")]
    39,
    #[cfg(feature = "board-40")]
    40,
    41,
    #[cfg(feature = "board-42")]
    42,
    #[cfg(feature = "board-43")]
    43,
    #[cfg(feature = "board-44")]
    44,
    #[cfg(feature = "board-45")]
    45,
    #[cfg(feature = "board-46")]
    46,
    #[cfg(feature = "board-mmc3-multicart")]
    47,
    48,
    #[cfg(feature = "board-49")]
    49,
    #[cfg(feature = "board-50")]
    50,
    #[cfg(feature = "board-51")]
    51,
    #[cfg(feature = "board-52")]
    52,
    #[cfg(feature = "board-53")]
    53,
    #[cfg(feature = "board-54")]
    54,
    #[cfg(feature = "board-55")]
    55,
    #[cfg(feature = "board-56")]
    56,
    #[cfg(feature = "board-57")]
    57,
    #[cfg(feature = "board-58")]
    58,
    #[cfg(feature = "board-59")]
    59,
    #[cfg(feature = "board-60")]
    60,
    #[cfg(feature = "board-61")]
    61,
    #[cfg(feature = "board-62")]
    62,
    #[cfg(feature = "board-63")]
    63,
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
    #[cfg(feature = "board-81")]
    81,
    #[cfg(feature = "board-taito-x1")]
    82,
    // The Cony / Yoko ASIC, with NES 2.0 mapper 264 at the end of this list (MF-UNL-8 step 3).
    #[cfg(feature = "board-cony")]
    83,
    85,
    #[cfg(feature = "board-jaleco-72-92-86")]
    86,
    87,
    #[cfg(feature = "board-namcot108")]
    88,
    #[cfg(feature = "board-small-boards")]
    89,
    #[cfg(feature = "board-jy-asic")]
    90,
    // The JY830623C / YY840238C and the EJ-006-1, two boards and two counters (MF-UNL-8 step 4).
    #[cfg(feature = "board-91")]
    91,
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
    #[cfg(feature = "board-100")]
    100,
    101,
    #[cfg(feature = "board-103")]
    103,
    #[cfg(feature = "board-104")]
    104,
    #[cfg(feature = "board-105")]
    105,
    #[cfg(feature = "board-106")]
    106,
    #[cfg(feature = "board-107")]
    107,
    #[cfg(feature = "board-108")]
    108,
    #[cfg(feature = "board-111")]
    111,
    #[cfg(feature = "board-112")]
    112,
    #[cfg(feature = "board-113")]
    113,
    #[cfg(feature = "board-mmc3-scramble")]
    114,
    #[cfg(feature = "board-mmc3-scramble")]
    115,
    #[cfg(feature = "board-116")]
    116,
    #[cfg(feature = "board-117")]
    117,
    118,
    #[cfg(feature = "board-119")]
    119,
    #[cfg(feature = "board-120")]
    120,
    #[cfg(feature = "board-mmc3-scramble")]
    121,
    #[cfg(feature = "board-122")]
    122,
    #[cfg(feature = "board-mmc3-scramble")]
    123,
    #[cfg(feature = "board-125")]
    125,
    #[cfg(feature = "board-126")]
    126,
    #[cfg(feature = "board-jv001")]
    132,
    #[cfg(feature = "board-133")]
    133,
    #[cfg(feature = "board-134")]
    134,
    #[cfg(feature = "board-jv001")]
    136,
    // The four wirings of the Sachen 8259, which are **not** in letter order: 137 is the 8259D,
    // 138 the B, 139 the C and 141 the A (MF-UNL-7 step 1).
    #[cfg(feature = "board-sachen8259")]
    137,
    #[cfg(feature = "board-sachen8259")]
    138,
    #[cfg(feature = "board-sachen8259")]
    139,
    #[cfg(feature = "board-small-boards")]
    140,
    #[cfg(feature = "board-sachen8259")]
    141,
    #[cfg(feature = "board-142")]
    142,
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
    #[cfg(feature = "board-148")]
    148,
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
    #[cfg(feature = "board-162")]
    162,
    #[cfg(feature = "board-163")]
    163,
    #[cfg(feature = "board-164")]
    164,
    #[cfg(feature = "board-165")]
    165,
    #[cfg(feature = "board-166")]
    166,
    #[cfg(feature = "board-167")]
    167,
    #[cfg(feature = "board-168")]
    168,
    #[cfg(feature = "board-170")]
    170,
    #[cfg(feature = "board-171")]
    171,
    #[cfg(feature = "board-jv001")]
    172,
    #[cfg(feature = "board-jv001")]
    173,
    #[cfg(feature = "board-174")]
    174,
    #[cfg(feature = "board-175")]
    175,
    #[cfg(feature = "board-176")]
    176,
    #[cfg(feature = "board-177")]
    177,
    #[cfg(feature = "board-178")]
    178,
    #[cfg(feature = "board-179")]
    179,
    180,
    #[cfg(feature = "board-mmc3-scramble")]
    182,
    #[cfg(feature = "board-183")]
    183,
    #[cfg(feature = "board-small-boards")]
    184,
    #[cfg(feature = "board-small-boards")]
    185,
    #[cfg(feature = "board-mmc3-scramble")]
    187,
    #[cfg(feature = "board-188")]
    188,
    #[cfg(feature = "board-189")]
    189,
    #[cfg(feature = "board-190")]
    190,
    #[cfg(feature = "board-191")]
    191,
    #[cfg(feature = "board-mmc3-chrram")]
    192,
    #[cfg(feature = "board-193")]
    193,
    #[cfg(feature = "board-mmc3-chrram")]
    194,
    #[cfg(feature = "board-mmc3-chrram")]
    195,
    #[cfg(feature = "board-196")]
    196,
    #[cfg(feature = "board-197")]
    197,
    #[cfg(feature = "board-198")]
    198,
    #[cfg(feature = "board-199")]
    199,
    #[cfg(feature = "board-200")]
    200,
    #[cfg(feature = "board-201")]
    201,
    #[cfg(feature = "board-202")]
    202,
    #[cfg(feature = "board-203")]
    203,
    #[cfg(feature = "board-204")]
    204,
    #[cfg(feature = "board-205")]
    205,
    #[cfg(feature = "board-namcot108")]
    206,
    #[cfg(feature = "board-taito-x1")]
    207,
    // The unnamed Gouder board of the Street Fighter IV backport, an MMC3 clone with a 256-byte
    // protection lookup table (MF-UNL-11 step 2).
    #[cfg(feature = "board-208")]
    208,
    #[cfg(feature = "board-jy-asic")]
    209,
    210,
    #[cfg(feature = "board-jy-asic")]
    211,
    #[cfg(feature = "board-212")]
    212,
    #[cfg(feature = "board-213")]
    213,
    #[cfg(feature = "board-214")]
    214,
    // The Sugar Softec UNL-8237 and UNL-8237A, an MMC3 clone with one of eight run-time
    // selectable register scrambles (MF-UNL-11 step 1).
    #[cfg(feature = "board-215")]
    215,
    #[cfg(feature = "board-216")]
    216,
    #[cfg(feature = "board-217")]
    217,
    #[cfg(feature = "board-218")]
    218,
    // The Kasheng A9461, an MMC3 clone with an extended banking mode (MF-UNL-11 step 3).
    #[cfg(feature = "board-219")]
    219,
    #[cfg(feature = "board-221")]
    221,
    #[cfg(feature = "board-222")]
    222,
    #[cfg(feature = "board-224")]
    224,
    225,
    #[cfg(feature = "board-226")]
    226,
    #[cfg(feature = "board-227")]
    227,
    228,
    #[cfg(feature = "board-229")]
    229,
    #[cfg(feature = "board-230")]
    230,
    #[cfg(feature = "board-231")]
    231,
    #[cfg(feature = "board-small-boards")]
    232,
    #[cfg(feature = "board-233")]
    233,
    #[cfg(feature = "board-234")]
    234,
    #[cfg(feature = "board-235")]
    235,
    #[cfg(feature = "board-236")]
    236,
    #[cfg(feature = "board-237")]
    237,
    #[cfg(feature = "board-238")]
    238,
    #[cfg(feature = "board-240")]
    240,
    #[cfg(feature = "board-241")]
    241,
    #[cfg(feature = "board-242")]
    242,
    #[cfg(feature = "board-sachen-8reg")]
    243,
    #[cfg(feature = "board-244")]
    244,
    #[cfg(feature = "board-245")]
    245,
    // The G0151-1 of 封神榜: eight bank registers, battery PRG-RAM at $6800-$6FFF and a forced
    // PRG A17 on four read ranges (MF-UNL-12 step 3).
    #[cfg(feature = "board-246")]
    246,
    #[cfg(feature = "board-248")]
    248,
    #[cfg(feature = "board-249")]
    249,
    #[cfg(feature = "board-250")]
    250,
    #[cfg(feature = "board-252")]
    252,
    #[cfg(feature = "board-253")]
    253,
    #[cfg(feature = "board-254")]
    254,
    #[cfg(feature = "board-255")]
    255,
    #[cfg(feature = "board-256")]
    256,
    #[cfg(feature = "board-258")]
    258,
    #[cfg(feature = "board-259")]
    259,
    #[cfg(feature = "board-260")]
    260,
    #[cfg(feature = "board-261")]
    261,
    // The Sachen UNL-SHERO, an MMC3 clone with 8 KiB of CHR-RAM, four CHR A18 lines at $4100 and
    // four-screen mirroring (MF-UNL-12 step 1).
    #[cfg(feature = "board-262")]
    262,
    #[cfg(feature = "board-263")]
    263,
    #[cfg(feature = "board-cony")]
    264,
    #[cfg(feature = "board-265")]
    265,
    #[cfg(feature = "board-266")]
    266,
    #[cfg(feature = "board-267")]
    267,
    #[cfg(feature = "board-268")]
    268,
    #[cfg(feature = "board-269")]
    269,
    #[cfg(feature = "board-271")]
    271,
    #[cfg(feature = "board-272")]
    272,
    #[cfg(feature = "board-274")]
    274,
    #[cfg(feature = "board-281")]
    281,
    #[cfg(feature = "board-282")]
    282,
    #[cfg(feature = "board-283")]
    283,
    #[cfg(feature = "board-284")]
    284,
    #[cfg(feature = "board-285")]
    285,
    #[cfg(feature = "board-286")]
    286,
    #[cfg(feature = "board-287")]
    287,
    #[cfg(feature = "board-288")]
    288,
    #[cfg(feature = "board-289")]
    289,
    #[cfg(feature = "board-290")]
    290,
    // The BMW8544 / UNL-DRAGONFIGHTER, an MMC3 clone whose CHR banking is XORed with two
    // registers a read of $6000-$7FFF fills (MF-UNL-12 step 2).
    #[cfg(feature = "board-291")]
    291,
    #[cfg(feature = "board-292")]
    292,
    #[cfg(feature = "board-295")]
    295,
    #[cfg(feature = "board-297")]
    297,
    #[cfg(feature = "board-298")]
    298,
    #[cfg(feature = "board-299")]
    299,
    #[cfg(feature = "board-300")]
    300,
    #[cfg(feature = "board-301")]
    301,
    #[cfg(feature = "board-302")]
    302,
    #[cfg(feature = "board-303")]
    303,
    #[cfg(feature = "board-304")]
    304,
    #[cfg(feature = "board-305")]
    305,
    #[cfg(feature = "board-306")]
    306,
    #[cfg(feature = "board-307")]
    307,
    #[cfg(feature = "board-308")]
    308,
    #[cfg(feature = "board-309")]
    309,
    #[cfg(feature = "board-311")]
    311,
    #[cfg(feature = "board-312")]
    312,
    #[cfg(feature = "board-313")]
    313,
    #[cfg(feature = "board-314")]
    314,
    #[cfg(feature = "board-315")]
    315,
    #[cfg(feature = "board-319")]
    319,
    #[cfg(feature = "board-320")]
    320,
    #[cfg(feature = "board-322")]
    322,
    #[cfg(feature = "board-323")]
    323,
    #[cfg(feature = "board-324")]
    324,
    #[cfg(feature = "board-325")]
    325,
    #[cfg(feature = "board-327")]
    327,
    #[cfg(feature = "board-328")]
    328,
    #[cfg(feature = "board-329")]
    329,
    #[cfg(feature = "board-331")]
    331,
    #[cfg(feature = "board-332")]
    332,
    #[cfg(feature = "board-333")]
    333,
    #[cfg(feature = "board-334")]
    334,
    #[cfg(feature = "board-335")]
    335,
    #[cfg(feature = "board-336")]
    336,
    #[cfg(feature = "board-337")]
    337,
    #[cfg(feature = "board-338")]
    338,
    #[cfg(feature = "board-339")]
    339,
    #[cfg(feature = "board-340")]
    340,
    #[cfg(feature = "board-341")]
    341,
    #[cfg(feature = "board-342")]
    342,
    #[cfg(feature = "board-343")]
    343,
    #[cfg(feature = "board-344")]
    344,
    #[cfg(feature = "board-345")]
    345,
    #[cfg(feature = "board-346")]
    346,
    #[cfg(feature = "board-347")]
    347,
    #[cfg(feature = "board-348")]
    348,
    #[cfg(feature = "board-349")]
    349,
    #[cfg(feature = "board-350")]
    350,
    #[cfg(feature = "board-351")]
    351,
    #[cfg(feature = "board-352")]
    352,
    #[cfg(feature = "board-353")]
    353,
    #[cfg(feature = "board-354")]
    354,
    #[cfg(feature = "board-355")]
    355,
    #[cfg(feature = "board-356")]
    356,
    #[cfg(feature = "board-357")]
    357,
    #[cfg(feature = "board-358")]
    358,
    #[cfg(feature = "board-359")]
    359,
    #[cfg(feature = "board-360")]
    360,
    #[cfg(feature = "board-361")]
    361,
    #[cfg(feature = "board-362")]
    362,
    #[cfg(feature = "board-366")]
    366,
    #[cfg(feature = "board-368")]
    368,
    #[cfg(feature = "board-369")]
    369,
    #[cfg(feature = "board-370")]
    370,
    #[cfg(feature = "board-372")]
    372,
    #[cfg(feature = "board-374")]
    374,
    #[cfg(feature = "board-375")]
    375,
    #[cfg(feature = "board-377")]
    377,
    #[cfg(feature = "board-380")]
    380,
    #[cfg(feature = "board-381")]
    381,
    #[cfg(feature = "board-382")]
    382,
    #[cfg(feature = "board-384")]
    384,
    #[cfg(feature = "board-386")]
    386,
    #[cfg(feature = "board-387")]
    387,
    #[cfg(feature = "board-388")]
    388,
    #[cfg(feature = "board-389")]
    389,
    #[cfg(feature = "board-390")]
    390,
    #[cfg(feature = "board-393")]
    393,
    #[cfg(feature = "board-394")]
    394,
    #[cfg(feature = "board-395")]
    395,
    #[cfg(feature = "board-396")]
    396,
    #[cfg(feature = "board-397")]
    397,
    #[cfg(feature = "board-398")]
    398,
    #[cfg(feature = "board-399")]
    399,
    #[cfg(feature = "board-400")]
    400,
    #[cfg(feature = "board-401")]
    401,
    #[cfg(feature = "board-403")]
    403,
    #[cfg(feature = "board-404")]
    404,
    #[cfg(feature = "board-406")]
    406,
    #[cfg(feature = "board-409")]
    409,
    #[cfg(feature = "board-410")]
    410,
    #[cfg(feature = "board-411")]
    411,
    #[cfg(feature = "board-412")]
    412,
    #[cfg(feature = "board-413")]
    413,
    #[cfg(feature = "board-414")]
    414,
    #[cfg(feature = "board-415")]
    415,
    #[cfg(feature = "board-416")]
    416,
    #[cfg(feature = "board-417")]
    417,
    #[cfg(feature = "board-420")]
    420,
    #[cfg(feature = "board-421")]
    421,
    #[cfg(feature = "board-422")]
    422,
    #[cfg(feature = "board-428")]
    428,
    #[cfg(feature = "board-429")]
    429,
    #[cfg(feature = "board-431")]
    431,
    #[cfg(feature = "board-432")]
    432,
    #[cfg(feature = "board-433")]
    433,
    #[cfg(feature = "board-434")]
    434,
    #[cfg(feature = "board-436")]
    436,
    #[cfg(feature = "board-437")]
    437,
    #[cfg(feature = "board-438")]
    438,
    #[cfg(feature = "board-442")]
    442,
    #[cfg(feature = "board-446")]
    446,
    #[cfg(feature = "board-447")]
    447,
    #[cfg(feature = "board-451")]
    451,
    #[cfg(feature = "board-452")]
    452,
    #[cfg(feature = "board-455")]
    455,
    #[cfg(feature = "board-456")]
    456,
    #[cfg(feature = "board-457")]
    457,
    #[cfg(feature = "board-471")]
    471,
    #[cfg(feature = "board-481")]
    481,
    #[cfg(feature = "board-487")]
    487,
    #[cfg(feature = "board-512")]
    512,
    #[cfg(feature = "board-513")]
    513,
    #[cfg(feature = "board-516")]
    516,
    #[cfg(feature = "board-517")]
    517,
    #[cfg(feature = "board-518")]
    518,
    #[cfg(feature = "board-519")]
    519,
    #[cfg(feature = "board-521")]
    521,
    #[cfg(feature = "board-522")]
    522,
    #[cfg(feature = "board-524")]
    524,
    #[cfg(feature = "board-525")]
    525,
    #[cfg(feature = "board-526")]
    526,
    #[cfg(feature = "board-527")]
    527,
    #[cfg(feature = "board-528")]
    528,
    #[cfg(feature = "board-529")]
    529,
    #[cfg(feature = "board-530")]
    530,
    #[cfg(feature = "board-532")]
    532,
    #[cfg(feature = "board-534")]
    534,
    #[cfg(feature = "board-536")]
    536,
    #[cfg(feature = "board-538")]
    538,
    #[cfg(feature = "board-539")]
    539,
    #[cfg(feature = "board-540")]
    540,
    #[cfg(feature = "board-541")]
    541,
    #[cfg(feature = "board-543")]
    543,
    #[cfg(feature = "board-547")]
    547,
    #[cfg(feature = "board-550")]
    550,
    #[cfg(feature = "board-551")]
    551,
    #[cfg(feature = "board-taito-x1")]
    552,
    #[cfg(feature = "board-554")]
    554,
    #[cfg(feature = "board-555")]
    555,
    #[cfg(feature = "board-556")]
    556,
    #[cfg(feature = "board-557")]
    557,
    #[cfg(feature = "board-558")]
    558,
    #[cfg(feature = "board-559")]
    559,
    #[cfg(feature = "board-560")]
    560,
    #[cfg(feature = "board-561")]
    561,
    #[cfg(feature = "board-562")]
    562,
    #[cfg(feature = "board-564")]
    564,
    #[cfg(feature = "board-682")]
    682,
];

/// Whether `mapper` is implemented.
#[must_use]
pub fn is_supported(mapper: i32) -> bool {
    SUPPORTED_MAPPERS.contains(&mapper)
}

/// Lot MAPPERS-P: whether `rom` is a Study Box tape packaged by
/// [`super::mappers::study_box::package`] — mapper 186 with a `STBX` misc ROM, the one way 186
/// loads; a bare iNES 186 (the BIOS alone) stays refused (AMD-MAPPERS-N-200).
#[must_use]
pub(crate) fn is_study_box(rom: &super::RomImage) -> bool {
    #[cfg(feature = "board-186")]
    if rom.mapper() == 186 {
        let misc = rom
            .raw()
            .get(rom.header().announced_size()..)
            .unwrap_or_default();
        return super::mappers::study_box::is_tape(misc);
    }
    // Lot FDS: mapper 20 is a packaged disk image (`fds::package`); a bare iNES 20 is not.
    #[cfg(feature = "board-fds")]
    if rom.mapper() == 20 {
        let misc = rom
            .raw()
            .get(rom.header().announced_size()..)
            .unwrap_or_default();
        return super::mappers::fds::is_disk(misc, rom.header().submapper == 1);
    }
    let _ = rom;
    false
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
    /// Sector size of the flash chip the board's PRG-ROM **is**, for a board whose game saves by
    /// writing it, and `None` for a board whose PRG-ROM is mask ROM (MF-UNL-9 step 1). The size
    /// is the chip's erase unit, so it is also the unit of the dirty set and of the save file;
    /// see [`super::FlashRom`].
    pub flash_sector: Option<usize>,
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

/// The MAPPERS-N boards whose iNES 1.0 CHR-RAM is Mesen2's 8 KiB default (see
/// [`board_memory_sizes`]).
const MAPPERS_N_CHR_RAM_8K: &[i32] = &[
    452, 442, 347, 558, 539, 368, 357, 311, 340, 564, 541, 538, 481, 437, 434, 433, 431, 415, 409,
    396, 382, 380, 354, 375, 350, 337, 381, 237, 39, 40, 42, 43, 44, 45, 46, 49, 50, 51, 52, 54,
    56, 57, 58, 59, 60, 61, 53, 62, 63, 103, 104, 106, 108, 113, 117, 120, 125, 126, 134, 142, 148,
    166, 167, 170, 171, 174, 177, 179, 183, 196, 197, 200, 201, 202, 203, 204, 205, 212, 213, 214,
    216, 217, 221, 222, 224, 226, 227, 229, 230, 231, 233, 235, 236, 238, 240, 241, 242, 244, 245,
    249, 250, 254, 255, 258, 259, 261, 265, 274, 283, 285, 286, 287, 288, 289, 290, 299, 300, 301,
    302, 303, 304, 305, 306, 307, 309, 312, 313, 314, 319, 320, 324, 325, 329, 331, 332, 333, 336,
    346, 348, 349, 366, 518, 521, 522,
];

/// The volatile-memory sizes of `mapper`'s board for `header`.
#[must_use]
// One arm per board that dictates a size, each with the page that says so: a flat table, like
// `create_board` above it and `opcode_table`'s matrix. MF-VS step 4's mapper-99 arm is the one
// that crossed the threshold.
#[allow(clippy::too_many_lines)]
pub(crate) fn board_memory_sizes(mapper: i32, header: &InesHeader) -> BoardMemorySizes {
    match mapper {
        // MAPPERS-P: puNES `map_init_269` copies the PRG-ROM into CHR when the image has no
        // CHR-ROM, so the CHR memory is the PRG-ROM's size (`Mmc3P::fill_chr_269`).
        // MAPPERS-P: puNES `chr_swap_mmc3_555` maps 1 KiB CHR-RAM windows beside the CHR-ROM
        // (`memmap_vram_1k`); the cartridge carries 8 KiB of it, which an iNES 1.0 header cannot
        // state (the TQROM's case, mapper 119 above).
        // MAPPERS-P: puNES `map_init_558` mounts the 93C66 when the NES 2.0 header declares
        // 512 bytes of PRG-NVRAM (byte 10's high nibble 3); its bytes follow 8 KiB of work RAM and
        // `battery_offset` puts the save there.
        #[cfg(feature = "board-558")]
        558 if header.is_nes20 && header.byte10 >> 4 == 3 => BoardMemorySizes {
            prg_ram: Some(IrqP::EEPROM_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-P: mapper 512's 8 KiB CHR-RAM beside its CHR-ROM (`vram_size(0)`), which an
        // iNES 1.0 header cannot state either.
        #[cfg(feature = "board-512")]
        512 if header.chr_rom_size != 0 => BoardMemorySizes {
            chr_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-P: the PRG-ROM of 406 and 451 is a flash chip with 64 KiB sectors, their save
        // medium (puNES `sst39sf040_init(…, 65536)`, `force_battery_io`); 406 has no WRAM
        // (`wram_fix_mmc3_406`).
        #[cfg(feature = "board-406")]
        406 => BoardMemorySizes {
            prg_ram: Some(0),
            flash_sector: Some(0x1_0000),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-451")]
        451 => BoardMemorySizes {
            flash_sector: Some(0x1_0000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-P: mapper 547's 8 KiB CHR-RAM beside its kanji CHR-ROM (`memmap_vram_4k`), which
        // an iNES 1.0 header cannot state.
        #[cfg(feature = "board-547")]
        547 if header.chr_rom_size != 0 => BoardMemorySizes {
            chr_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-P: 446's PRG-ROM is a flash chip with 128 KiB sectors, its menu's save medium
        // (puNES `sst39sf040_init(…, 131072)`, `force_battery_io`).
        // MAPPERS-P: 342 (COOLGIRL) has 32 KiB of work RAM in four pages and, when the header
        // states none, 256 KiB of CHR-RAM (puNES `map_init_342`).
        #[cfg(feature = "board-342")]
        342 => BoardMemorySizes {
            prg_ram: Some(0x8000),
            chr_ram: (header.chr_rom_size == 0).then_some(0x4_0000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-P: the Study Box's 64 KiB of work RAM and 8 KiB of CHR-RAM (Mesen `StudyBox.h`).
        // Lot FDS: the RAM adapter's 32 KiB of work RAM ($6000-$DFFF) and 8 KiB of CHR-RAM.
        #[cfg(feature = "board-fds")]
        20 => BoardMemorySizes {
            prg_ram: Some(0x8000),
            chr_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-186")]
        186 => BoardMemorySizes {
            prg_ram: Some(super::mappers::study_box::WORK_RAM_SIZE),
            chr_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-446")]
        446 => BoardMemorySizes {
            flash_sector: Some(0x2_0000),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-555")]
        555 => BoardMemorySizes {
            chr_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-269")]
        269 if header.chr_rom_size == 0 => BoardMemorySizes {
            chr_ram: Some(header.prg_rom_size),
            ..BoardMemorySizes::default()
        },
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
        // The VRC2's Microwire latch replaces the work RAM, it does not sit beside it (MF-LEG-1
        // step 2, report entry VRC2-1; Q-174). A cartridge whose NES 2.0 header declares no
        // PRG-RAM **has no RAM chip**, and what answers $6000-$6FFF there is the chip's own
        // one-bit latch: see `Vrc2Vrc4::has_microwire_latch` for the gate, which is Nestopia's and
        // puNES's, and `Vrc2Vrc4::microwire_latch` for the latch. Without this arm REQ-CORE-012's
        // 8 KiB floor would keep the RAM on top of the latch and $7000-$7FFF would answer RAM
        // where the page says "only ever return open bus".
        #[cfg(feature = "board-vrc2-latch")]
        22 | 23 | 25 if Vrc2Vrc4::has_microwire_latch(mapper, header) => BoardMemorySizes {
            prg_ram: Some(0),
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
        // Mapper 195's own row, ahead of the family's: its PRG-RAM is **12 KiB**, the FS303
        // compatible board's 4 KiB at CPU $5000-$5FFF followed by the ordinary 8 KiB of WRAM at
        // $6000-$7FFF. nesdev ("INES Mapper 195") gives the window to a board that "**adds** 4
        // KiB of non-battery-backed PRG-RAM … mapped to CPU $5000-$5FFF", and the two projects
        // that implement it put it past the WRAM the same way -- FCEUX gives this number, and no
        // other MMC3 clone, 16 KiB and maps 4 KiB bank 2 there (`mmc3.cpp:1036-1041`, `:1044`);
        // puNES maps the same bank 2 (`mapper_195.c:134-137`).
        //
        // The extra chip is the **head** and the WRAM the tail, which is what `battery_offset`
        // below needs to keep the save file 8 KiB and the page's "non-battery-backed" true; see
        // `Mmc3ChrRamBoard::EXTRA_RAM_SIZE` for the table and Q-173 for what R0 is asked.
        #[cfg(feature = "board-195-prg-ram-5000")]
        195 => BoardMemorySizes {
            prg_ram: Some(Mmc3ChrRamBoard::EXTRA_RAM_SIZE + 2 * BankWindows::PRG_WINDOW_SIZE),
            chr_ram: Mmc3ChrRamBoard::chr_ram_size(mapper),
            ..BoardMemorySizes::default()
        },
        // 195 again for the build with that feature off, where it is the plain FS303 of MF-UNL-2
        // step 2 and its row is the family's. The two arms are spelled out rather than left to
        // one `74 | 192 | 194 | 195` so that neither configuration has an unreachable pattern.
        #[cfg(all(feature = "board-mmc3-chrram", not(feature = "board-195-prg-ram-5000")))]
        195 => BoardMemorySizes {
            chr_ram: Mmc3ChrRamBoard::chr_ram_size(mapper),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-mmc3-chrram")]
        #[cfg(feature = "board-191")]
        191 => BoardMemorySizes {
            chr_ram: Mmc3ChrRamBoard::chr_ram_size(mapper),
            ..BoardMemorySizes::default()
        },
        74 | 192 | 194 => BoardMemorySizes {
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
        // Mapper 208, submapper 0: the PRG bank and mirroring register answers at **$6800-$6FFF**
        // as well as at $4800-$4FFF, so there is no RAM behind that window -- the same arm, and
        // the same reason, as 114, 115 and 182 above (MF-UNL-11 step 2). `Cartridge::cpu_write`
        // returns at $6000-$7FFF whenever the cartridge has any PRG-RAM, so without this row the
        // register at $6800 could not be written at all.
        //
        // **Submapper 1 is deliberately excluded**: "There are no additional registers" on the
        // copyright-stripped hack, so nothing forces the question there and the header decides --
        // mapper 97's precedent in this function, and the same tightening mapper 4's own arm above
        // makes for the MMC6.
        #[cfg(feature = "board-208")]
        208 if !(header.is_nes20 && header.submapper == 1) => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 246"): "CPU $6800-$6FFF: **2 KiB PRG-RAM, battery-banked**", not the
        // iNES default of 8 KiB -- the same reading, and the same reason, as mapper 99's arm above.
        // The window is 2 KiB and the chip is 2 KiB, so `Mapper::prg_ram_window`'s own default
        // mirrors it four times across $6000-$7FFF; `G0151`'s note has that choice and the board's
        // own register decode, which wins over the mirror (MF-UNL-12 step 3, Q-172).
        //
        // "battery-banked", and whether a host persists the bytes is still the dump's own battery
        // bit: `Cartridge::battery_ram` is what REQ-CORE-095 exposes and it reads
        // `header.has_battery`, as it does for every board in this function.
        #[cfg(feature = "board-246")]
        246 => BoardMemorySizes {
            prg_ram: Some(G0151::PRG_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // Mapper 292: "There are four extra registers that are filled by writing or reading (!)
        // from the CPU $6000-$7FFF address range", so that whole window **is** the board's decode
        // and there is no RAM behind it -- the same arm, and the same reason, as 114, 115, 182 and
        // 208 above (MF-UNL-12 step 2). `Cartridge::cpu_write` returns at $6000-$7FFF whenever the
        // cartridge has any PRG-RAM, so without this row the Extra Index Register could not be
        // written at all; and with no RAM there `Cartridge::default_cpu_read` never shadows the
        // board's own `cpu_read_override`, which is what fills a data register.
        //
        // The page's condition on the registers -- "when PRG-RAM is enabled in the MMC3's A001
        // register" -- is still read, from `Mmc3Board::prg_ram_enabled`: it is the chip's own
        // enable bit and does not need a chip behind it.
        #[cfg(feature = "board-292")]
        292 => BoardMemorySizes {
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
        // nesdev ("NES 2.0 Mapper 262"): the UNL-SHERO uses "an MMC3 clone with both CHR-ROM and
        // **8 KiB of CHR-RAM**" (MF-UNL-12 step 1). **No header field can state it**, which is the
        // TQROM's arm's reason in the same words: byte 5 of an iNES 1.0 header is the CHR-*ROM*
        // size and REQ-CORE-012's CHR-RAM default only applies to an image with no CHR-ROM at all,
        // so a UNL-SHERO dump would come up with none and its software-rendered text boxes would
        // have nowhere to go.
        //
        // The 4 KiB of "additional VRAM for four-screen mirroring" the same sentence gives the
        // board is **not** here: it is the console's own nametable RAM, `ppu::VRAM_SIZE` being
        // 4 KiB, which is mapper 77's arm's reading of the same shape.
        // MAPPERS-N: the Front Fareast formats give a CHR-RAM image 32 KiB (Mesen2
        // `FrontFareast.h:12`, `GetChrRamSize() { return 0x8000; }`).
        #[cfg(feature = "board-irq-latch")]
        6 | 8 | 17 if header.chr_rom_size == 0 => BoardMemorySizes {
            chr_ram: Some(0x8000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: the FK23C's iNES 1.0 defaults, "only used for iNES 1.0 files w/ no DB entry"
        // (Mesen2 `Fk23C.h`, `GetChrRamSize` 256 KiB and `GetWorkRamSize` 32 KiB).
        #[cfg(feature = "board-176")]
        176 if !header.is_nes20 => BoardMemorySizes {
            prg_ram: Some(Fk23c::INES1_WORK_RAM),
            chr_ram: Some(Fk23c::INES1_CHR_RAM),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: "GetWorkRamSize() { return 0x8000; }" (Mesen2 `Edu2000.h:9`).
        #[cfg(feature = "board-329")]
        329 => BoardMemorySizes {
            prg_ram: Some(0x8000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: the LH10's 8 KiB of work RAM at `$C000-$DFFF` (Mesen2 `Lh10.h:25`).
        #[cfg(feature = "board-522")]
        522 => BoardMemorySizes {
            prg_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: mapper 125's LH32 has 8 KiB of work RAM at `$C000-$DFFF` (Mesen2 `Lh32.h:18`).
        #[cfg(feature = "board-125")]
        125 => BoardMemorySizes {
            prg_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: "GetChrRamSize() { return 0x800; }" (Mesen2 `Mapper253.h:21`).
        // A CHR-RAM-only image keeps the header's size: the 2 KiB is the chip beside the CHR-ROM.
        // MAPPERS-N: the 2 KiB beside the CHR-ROM (cartridge database `8B7EE49B`, puNES
        // `mapper_252.c` mapping it through `vram`).
        // MAPPERS-N: nesdev ("INES Mapper 198"): 8 KiB of battery WRAM at `$6000`, "an
        // additional non-backed 4 KiB of WRAM at $5000-$5FFF" (the head, as on 195) and 8 KiB of
        // CHR-RAM.
        // MAPPERS-N: nesdev ("INES Mapper 199"): "32 KiB of battery-backed PRG RAM, of which 12
        // KiB are mapped", and 8 KiB of unbanked CHR-RAM.
        // MAPPERS-N: `Sachen9602.h`, "GetChrRamSize() … 0x8000", battery-backed there.
        #[cfg(feature = "board-513")]
        513 => BoardMemorySizes {
            chr_ram: Some(0x8000),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-199")]
        199 => BoardMemorySizes {
            prg_ram: Some(0x8000),
            chr_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-198")]
        198 => BoardMemorySizes {
            prg_ram: Some(Mmc3Clone::RAM_198_LOW + 0x2000),
            chr_ram: Some(0x2000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: "GetChrRamSize() { return 0x1000; }" (Mesen2 `MMC3_165.h:13`).
        #[cfg(feature = "board-165")]
        165 => BoardMemorySizes {
            chr_ram: Some(0x1000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: nesdev ("INES Mapper 168"): 64 KiB of CHR-RAM, "There is no PRG RAM".
        #[cfg(feature = "board-168")]
        168 => BoardMemorySizes {
            prg_ram: Some(0),
            chr_ram: Some(Racermate::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: the `FineRom` boards decode `$6000-$7FFF` themselves; 103 and 307 have
        // work RAM behind it (`Mapper103.h`, `Kaiser7037.h`), 31 none (nesdev "PRG RAM capacity:
        // None"), 175 none, and 31's CHR is "recommended" 8 KiB of RAM.
        #[cfg(feature = "board-103")]
        103 => BoardMemorySizes {
            prg_ram: Some(FineRom::RAM_103),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-307")]
        307 => BoardMemorySizes {
            prg_ram: Some(FineRom::RAM_307),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-31")]
        31 => BoardMemorySizes {
            prg_ram: Some(0),
            chr_ram: (header.chr_rom_size == 0).then_some(0x2000),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: `$6000-$7FFF` is 53's outer register and ROM (`Supervision.h`).
        #[cfg(feature = "board-53")]
        53 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-175")]
        175 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: no PRG-RAM, `$6000-$7FFF` is the microphone (nesdev "INES Mapper 188").
        #[cfg(feature = "board-188")]
        188 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-252")]
        252 if header.chr_rom_size != 0 => BoardMemorySizes {
            chr_ram: Some(Waixing252::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-253")]
        253 if header.chr_rom_size != 0 => BoardMemorySizes {
            chr_ram: Some(Waixing253::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        #[cfg(feature = "board-262")]
        262 => BoardMemorySizes {
            chr_ram: Some(Mmc3Shero::CHR_RAM_SIZE),
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
        111 if header.chr_rom_size == 0 => BoardMemorySizes {
            prg_ram: Some(0),
            chr_ram: Some(Cheapocabra::CHR_RAM_SIZE),
            // nesdev ("GTROM"): "a Microchip SST39SF040 flash chip as the PRG ROM storage
            // device", and "Both of these mappers lack PRG RAM for game saves but the
            // configuration of both boards make it possible to erase and rewrite sections of the
            // chip using NES software". This board's PRG-ROM **is** the save medium, so it is
            // the one row of this table with a flash sector (MF-UNL-9 step 1).
            flash_sector: Some(FlashRom::SST39SF040_SECTOR),
            ..BoardMemorySizes::default()
        },
        // MAPPERS-N: the Rainbow's PRG-ROM is an S29 flash the game programs (Mesen2
        // `Rainbow.cpp`, `FlashS29`); its smallest erase unit, 8 KiB, is the tracking sector.
        #[cfg(feature = "board-682")]
        682 => BoardMemorySizes {
            flash_sector: Some(super::mappers::Rainbow::FLASH_TRACK_SECTOR),
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
        // nesdev ("INES Mapper 190"), its *Banks* section's first line: "CPU $6000-$7FFF: 8 KiB
        // RAM". The page has no capacity table at all -- it is one of the shortest in the census
        // -- so the `Banks` list is the only statement of the board's memories, and it states
        // this one. The figure is what REQ-CORE-012's default would give an iNES 1.0 image
        // anyway; it is stated because the page states it and byte 8 of an iNES 1.0 header
        // cannot be trusted to (MF-UNL-3 step 4). The page says nothing about a battery, so a
        // dump's own battery bit decides, as it does for every board here.
        #[cfg(feature = "board-190")]
        190 => BoardMemorySizes {
            prg_ram: Some(BankWindows::PRG_WINDOW_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("J.Y. Company ASIC"): mapper 35 "is like Mapper 209 but **explicitly specifies
        // 8 KiB of WRAM**. In the presence of a valid NES 2.0 header, which has its own PRG-RAM
        // field, INES Mapper 035 is an exact duplicate of INES Mappers 209." That sentence is the
        // *whole* difference between the two numbers, so this row is the whole of mapper 35's
        // implementation (MF-UNL-4 step 2). The other three numbers take REQ-CORE-012's default,
        // which is the same 8 KiB for an iNES 1.0 image and the header's own figure for an
        // NES 2.0 one -- the page's `wrammax` is 8K for the chip either way. The page says nothing
        // about a battery, so a dump's own battery bit decides, as it does for every board here.
        #[cfg(feature = "board-jy-asic")]
        35 => BoardMemorySizes {
            prg_ram: Some(BankWindows::PRG_WINDOW_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 163"), its *Banks* section: "CPU $6000-$7FFF: 8 KiB unbanked
        // PRG-RAM, battery-backed" and "PPU $0000-$1FFF: 8 KiB unbanked CHR-RAM bank, 4 KiB can be
        // automatically switched". Both figures are what REQ-CORE-012's defaults would give an
        // iNES 1.0 image with no CHR-ROM anyway, and both are stated because the page states them
        // and no iNES 1.0 header field can be trusted to: byte 8 is unreliable and byte 5 is the
        // CHR-*ROM* size. The CHR row matters on its own account -- the board's automatic switch
        // selects a 4 KiB page of **this** chip, so a bigger allocation would put pages the
        // hardware cannot address behind PPU A12 (MF-UNL-5 step 1).
        //
        // The page says "battery-backed", and whether a host persists the bytes is still the
        // dump's own battery bit: `Cartridge::battery_ram` is what REQ-CORE-095 exposes and it
        // reads `header.has_battery`, as it does for every board in this function.
        //
        // nesdev ("INES Mapper 162") states the same two lines of the same two chips for the
        // 外星 FS304, so the two numbers are one arm (MF-UNL-5 step 2).
        #[cfg(any(feature = "board-162", feature = "board-163"))]
        162 | 163 => BoardMemorySizes {
            prg_ram: Some(ChrWindowBoard::PRG_RAM_SIZE),
            chr_ram: Some(ChrWindowBoard::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // MF-UNL-10 step 2, nesdev ("INES Mapper 164"): the one board here with **two** memories
        // in `prg_ram` at once. Its *Banks* section gives "CPU $6000-$7FFF: 0-8 KiB unbanked
        // PRG-RAM, not battery-backed" and "PPU $0000-$1FFF: 8 KiB unbanked CHR-RAM", and its
        // opening paragraph gives the save medium: "PRG-RAM is never battery-backed; all games
        // save the game state to a 93C66 512-byte serial EEPROM."
        //
        // So the allocation is the volatile chip -- 2 KiB, "all known carts mount either no or a
        // 2 KiB RAM chip which is then mirrored three times across the respective address range"
        // (Q-170) -- plus the EEPROM's 512 bytes behind it, and `battery_offset` below puts
        // REQ-CORE-095's battery slice at the EEPROM alone. That is the `Eeprom24C0X` layout of
        // mappers 157 and 159 with a volatile chip in front of it, which is what the MMC1's SOROM
        // gave `battery_offset` its second parameter for.
        #[cfg(feature = "board-164")]
        164 => BoardMemorySizes {
            prg_ram: Some(Pec9588::PRG_RAM_SIZE),
            chr_ram: Some(Pec9588::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 178"), whose *Banks* section is the only statement of this
        // board's memories -- the page has no capacity table at all:
        //
        //   CPU $6000-$7FFF: 8 KiB switchable PRG-RAM bank
        //   PPU $0000-$1FFF: 8 KiB unbanked CHR-RAM
        //
        // The CHR figure is the chip the page says the chipset "is designed for", and it is the
        // one REQ-CORE-012's default gives an image with no CHR-ROM; it is stated because the page
        // states it and byte 5 of an iNES 1.0 header is the CHR-*ROM* size.
        //
        // **The PRG-RAM figure is deliberately a floor and not an exact size**, which is the
        // MMC1-SOROM and MMC5 shape of this struct's `min_prg_ram` field and the one thing this
        // function can honestly say: the window is 8 KiB, the register driving it is eight bits
        // wide over "PRG-RAM A20?..A13" -- the question mark is the page's -- and nothing on the
        // page fixes how much RAM a given cartridge carries. So an image's own header decides the
        // size and the board is guaranteed its one window, the modulo in
        // `WaixingFs305::prg_ram_window` standing in for the address lines a smaller chip does not
        // have. Q-154 records it with the rest of what this number's page does not say
        // (MF-UNL-5 step 3).
        #[cfg(feature = "board-178")]
        178 => BoardMemorySizes {
            chr_ram: Some(WaixingFs305::CHR_RAM_SIZE),
            min_prg_ram: BankWindows::PRG_WINDOW_SIZE,
            ..BoardMemorySizes::default()
        },
        // nesdev ("NES 2.0 Mapper 268"): the AA6023 addresses "up to 32 MiB of PRG-ROM, 256 KiB
        // of CHR-ROM or -RAM, and **8 KiB of WRAM**", and its *WRAM usage* section is a whole
        // paragraph about a game keeping that WRAM behind the register window -- so the board has
        // it, and the row states the figure (MF-UNL-6 step 3).
        //
        // **A floor and not an exact size**, which is mapper 190's and 35's shape in this
        // function: the figure is what REQ-CORE-012's default gives an iNES 1.0 image anyway, and
        // an NES 2.0 dump that declares more (a MINDKIDS flash cartridge) keeps what it declares.
        //
        // **The CHR is deliberately the header's**, which is mapper 97's reason rather than the
        // TQROM's: the page gives no single figure to state. Its PCB table gives a *maximum* per
        // board -- "max. 256 KiB CHR-RAM" on the SMD72A, "max. 128 KiB" on the J-852C, "max. 1 MiB
        // CHR-ROM" on the JTH-813 -- and the row this build serves (submappers 0 and 1) is
        // CHR-RAM-only or mixed, which only an NES 2.0 byte 11 can describe. So the dump decides,
        // and `$xxx4`'s mixed mode reads whatever chip it declared.
        #[cfg(feature = "board-268")]
        268 => BoardMemorySizes {
            min_prg_ram: BankWindows::PRG_WINDOW_SIZE,
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
        // nesdev ("INES Mapper 029"), the Sealie RET-CUFROM: "It is hard-wired for vertical
        // mirroring, and contains 8KB of WRAM mounted in the usual place", and "4x banks worth of
        // full 8KB CHR ram". Two figures the page states flatly and no header field can be trusted
        // to: byte 8 of an iNES 1.0 header is unreliable and byte 5 is the CHR-*ROM* size, where
        // REQ-CORE-012's CHR-RAM default would give this board 8 KiB and leave three of its four
        // banks unaddressable (MF-UNL-7 step 6).
        //
        // The page says nothing about a battery, so a dump's own battery bit decides, as it does
        // for every board here.
        #[cfg(feature = "board-29")]
        29 => BoardMemorySizes {
            prg_ram: Some(BankWindows::PRG_WINDOW_SIZE),
            chr_ram: Some(SingleLatch::RET_CUFROM_CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 218"): "There is no CHR-ROM or CHR-RAM on the cartridge. Instead,
        // the console's internal 2KiB nametable RAM is mapped as CHR-RAM", and "the cartridge
        // board only contains a single PRG-ROM chip (up to 32KiB)" beside a CIC.
        //
        // So the board has **2 KiB of CHR-RAM and no PRG-RAM**, and both figures are the page's
        // own rather than anything a header can say: byte 5 of an iNES 1.0 header is the CHR-*ROM*
        // size, REQ-CORE-012's CHR-RAM default is 8 KiB, and byte 8 cannot be trusted. The 2 KiB
        // is the console's nametable RAM, which this board maps into the pattern tables as well --
        // see `MagicFloor`'s own note on why it is held here (MF-UNL-7 step 5).
        #[cfg(feature = "board-218")]
        218 => BoardMemorySizes {
            prg_ram: Some(0),
            chr_ram: Some(MagicFloor::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 193"), NTDEC's TC-112: "Range,Mask: $6000-7FFF, $E007", which
        // reads A15, A14, A13 and A2-A0 -- so the board's five registers repeat every eight bytes
        // through the whole of $6000-$7FFF and the window has to be the registers and not RAM.
        // This is mappers 114, 115 and 182's arm above in the same words, and the page gives the
        // board no PRG-RAM line either way (MF-UNL-7 step 3).
        #[cfg(feature = "board-193")]
        193 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // The Sachen 8259's two ports are the same pair under the same **mask $C101** ("Registers
        // should be masked with $C101"), so the row above's reason holds word for word: the index
        // and data registers repeat through the whole of $4100-$7FFF, `$6100 & $C101` is `$4100`,
        // and the window has to be the registers and not RAM or a game writing its CHR block at
        // $6101 would write work RAM instead. Neither the family page nor mapper 137's gives the
        // board any PRG-RAM (MF-UNL-7 step 1).
        //
        // **No CHR row**, which is mapper 268's reason rather than the TQROM's: the page gives no
        // single figure to state. Its *CHR Rewiring* table gives a *maximum* per variant — 128 KiB
        // on the 8259B, 256 KiB on the A, 512 KiB on the C, 32 KiB on the D — and the 2M-RAM-COB
        // board of *Q-Boy* "replaces CHR-ROM with 8 KiB of unbanked CHR-RAM", which only the dump
        // can say. So the dump decides, and `Sachen8259::new` reads which of the two it is.
        #[cfg(feature = "board-sachen8259")]
        137 | 138 | 139 | 141 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 083"), the *Banks* section: "CPU $6000-$7FFF: 8 KiB switchable
        // window into 32 KiB PRG-NVRAM (Mapper 83.2)/**into PRG-ROM (all others)**". So on the four
        // PCBs this build serves there is no RAM behind that window at all -- it is a PRG-ROM bank
        // when $8100 bit 5 enables one and open bus when it does not -- and `Cony::try_read_prg`
        // answers it. 83.2, the one variant that would want RAM here, is refused at load (Q-167).
        // This is mappers 47 and 37's arm above for a different reason: not a register in the way,
        // but a window the board drives from ROM (MF-UNL-8 step 3).
        #[cfg(feature = "board-cony")]
        83 | 264 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 091"): every register of the chip is in $6000-$7FFF under
        // "Mask: $F003/$F007 (submapper 0/1)", which reads A15-A12 and two or three low bits -- so
        // the register file repeats every four or eight bytes through the whole 8 KiB and there is
        // no address in that window which is not a register. Mappers 114, 115, 182 and 193's arm,
        // in the same words, and the page gives the board no PRG-RAM line either way
        // (MF-UNL-8 step 4).
        #[cfg(feature = "board-91")]
        91 => BoardMemorySizes {
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
        // **NES 2.0 mapper 298 is deliberately absent**, and the reason is mapper 97's: its page
        // has no capacity rows at all, so it documents no PRG-RAM and does not say the board has
        // none either, and nothing forces the question -- every register of a VRC4b is above
        // $8000. The header decides, as it does for the five Konami numbers, none of which has a
        // row here either (MF-UNL-7 step 4).
        //
        // **Mapper 112 is deliberately absent**, and the reason is mapper 97's: its page has no
        // capacity table at all, so it documents no PRG-RAM and does not say the board has none
        // either, and nothing forces the question -- all three of its ports are at $8000, $A000
        // and $E000, above the window. The header decides (MF-UNL-7 step 2). **It is not on the
        // five Namco numbers' arm above**, although it shares their board: that arm is the Namco
        // 108 page's `PRG RAM: None` and its *Magical Puzzle Popils* exception, and neither
        // sentence is on mapper 112's page.
        //
        // **Mapper 12 is deliberately absent**, and the reason is mapper 133's above in the same
        // words: its outer register is at $4132 under **mask $E100**, which `$6100 & $E100` cannot
        // be, so no address in $6000-$7FFF can reach it and nothing forces the question. The page
        // gives the SL-5020B no PRG-RAM line either way, and the board is an MMC3 clone whose
        // $A001 gates the window, so the header decides -- mapper 4's own answer (MF-UNL-8 step 1).
        //
        // **Mapper 14 is deliberately absent too**, and for mapper 97's reason rather than mapper
        // 133's: its page has no capacity rows at all, so it documents no PRG-RAM and does not say
        // the board has none, and nothing forces the question -- the supervisor register is at
        // $A131, above the window. The header decides (MF-UNL-8 step 2).
        //
        // **Mappers 107, 143, 144, 145 and 149 are deliberately absent too** (MF-UNL-3 steps 2
        // and 3), and again for mapper 97's reason: not one of their pages documents any
        // PRG-RAM and not one says the board has none. Nothing forces the question — 143's and
        // 145's ports are at $4100 under mask $E100 and 107's, 144's and 149's are above $8000 —
        // so the header decides. Mapper 144 is mapper 11's board, which has no arm here either.
        // MAPPERS-N: an iNES 1.0 image without CHR-ROM gets 8 KiB of CHR-RAM on these boards, as
        // Mesen2 gives it (`BaseMapper::GetChrRamSize`, 0x2000 unless the board overrides it,
        // which none of these does), not REQ-CORE-012's 32 KiB: their CHR bank registers wrap
        // inside 8 KiB there, and a game writing a stray bank number would show an empty page
        // here (mapper 108's Bubble Bobble conversion, which FCEUX `108.cpp` and BizHawk
        // `Mapper108.cs`, "Cart.VramSize = 8", also give 8 KiB).
        n if !header.is_nes20 && header.chr_rom_size == 0 && MAPPERS_N_CHR_RAM_8K.contains(&n) => {
            BoardMemorySizes {
                chr_ram: Some(0x2000),
                ..BoardMemorySizes::default()
            }
        }
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
    match mapper {
        1 if prg_ram_size == 2 * BankWindows::PRG_WINDOW_SIZE => BankWindows::PRG_WINDOW_SIZE,
        // MF-UNL-10 step 2: mapper 164's PRG-RAM is a volatile chip the CPU addresses followed by
        // the 512 bytes of its 93C66, which is the only non-volatile medium on the board -- "all
        // games save the game state to a 93C66 512-byte serial EEPROM". So the battery slice is
        // the EEPROM and not the RAM, and the save file is 512 bytes whatever the RAM chip is.
        #[cfg(feature = "board-164")]
        164 => Pec9588::VOLATILE_RAM_SIZE,
        // MF-FIX, CHANGE 2: mapper 195's FS303 compatible board carries two RAM chips, and the
        // page says which of them the battery is not on -- "adds 4 KiB of **non-battery-backed**
        // PRG-RAM … mapped to CPU $5000-$5FFF". That 4 KiB is the head of the allocation, so the
        // battery slice is the 8 KiB of WRAM behind it and the save file stays the 8 KiB the
        // eleven ordinary FS303 titles already wrote. Mapper 1's SOROM row above is the same
        // arrangement with the sizes swapped.
        #[cfg(feature = "board-195-prg-ram-5000")]
        195 if prg_ram_size > Mmc3ChrRamBoard::EXTRA_RAM_SIZE => Mmc3ChrRamBoard::EXTRA_RAM_SIZE,
        // MAPPERS-N: mapper 198's battery is on the 8 KiB at `$6000`, after the 4 KiB of `$5000`.
        #[cfg(feature = "board-198")]
        198 if prg_ram_size > Mmc3Clone::RAM_198_LOW => Mmc3Clone::RAM_198_LOW,
        // MAPPERS-P: mapper 558's 93C66 follows its 8 KiB of work RAM.
        #[cfg(feature = "board-558")]
        558 if prg_ram_size == IrqP::EEPROM_RAM_SIZE => 0x2000,
        _ => 0,
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
        // *CPROM* carries the same unqualified "Subject to bus conflicts: Yes" row, against a
        // bank register the page places at "$8000-$FFFF" -- where the PRG-ROM drives (MF-LEG-1
        // step 3, report entry 13-1). MR-BUS-1 enumerated the boards whose pages carry the row and
        // **missed this one**, which the cross-check records as a gap in that enumeration and not
        // as a reversal of a verdict. Nestopia maps the board through its conflict-aware CHR swap
        // (`NstBoardCxRom.cpp:81`, `CHR_SWAP_4K_1_BC`) and puNES ANDs the written value with the
        // PRG byte at the same address (`mapper_013.c:46`); FCEUX, Nintendulator and Mesen 2 do
        // not, three against three with the page on the other three's side. *Videomation* is the
        // one cartridge.
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
        // MAPPERS-P: `mapper_382.c` ANDs the data latch with the PRG byte ("bus conflict").
        #[cfg(feature = "board-382")]
        382 => true,
        // MAPPERS-P: `mapper_400.c` ANDs its `$C000-$FFFF` latch with the PRG byte.
        #[cfg(feature = "board-400")]
        400 => true,
        // MAPPERS-P: `mapper_414.c` and `mapper_431.c` AND their data latch with the PRG byte.
        #[cfg(feature = "board-414")]
        414 => true,
        #[cfg(feature = "board-431")]
        431 => true,
        // MAPPERS-P: `mapper_434.c` and `mapper_437.c` AND their `$8000` latch with the PRG byte.
        #[cfg(feature = "board-434")]
        434 => true,
        #[cfg(feature = "board-437")]
        437 => true,
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
        // MAPPERS-N: Mesen2 gives the Sachen SA-008-A conflicts (`Sachen_148.h:10`,
        // `HasBusConflicts` true), its register sitting over the PRG-ROM.
        #[cfg(feature = "board-148")]
        148 => true,
        // MAPPERS-N: `FaridUnrom.h:9`, `HasBusConflicts` true.
        #[cfg(feature = "board-324")]
        324 => true,
        // MAPPERS-N: Mesen2 gives the Maxi 15 conflicts (`Mapper234.h`, `HasBusConflicts`).
        #[cfg(feature = "board-234")]
        234 => true,
        // MAPPERS-N: nesdev "INES Mapper 188", "Subject to bus conflicts: Yes".
        #[cfg(feature = "board-188")]
        188 => true,
        #[cfg(feature = "board-149")]
        149 => true, // Q-147 for the two numbers of this lot that are absent by a choice
        11 | 13 | 66 | 70 | 78 | 89 | 93 | 152 | 180 | 185 => true,
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
        // MF-UNL-8 step 1: the Gouder SL-5020B. **Submapper 1 is refused**: it denotes "ROM images
        // that have been extracted from disk images for the Front Fareast Magic Card 4M RAM
        // cartridge", which is copier hardware whose images need a 512-byte trainer "initialized
        // by JSRing to $7003 before JMPing to the game's reset vector" and the Super Magic Card's
        // own register file -- §1.8 of `docs/review/UNL-census.md`'s reason for leaving mappers 6,
        // 8 and 17 out, and census §7.2 says so of this submapper by name. A dump of it is refused
        // with REQ-CORE-011's message rather than run as an MMC3A clone it is not (Q-164).
        #[cfg(feature = "board-huang1")]
        12 if memory.rom().header().is_nes20 && memory.rom().header().submapper == 1 => {
            return None
        }
        // The SL-5020B itself: the Huang-1 "exclusively as an MMC3A clone" plus the GAL16V8's
        // outer CHR register at $4132 and its language-bit read. `Huang1`'s own note has the table
        // of this board against mapper 14's and against mapper 116's, and why the board holds an
        // `Mmc3` rather than being a parameter of `Somari` (Q-164).
        #[cfg(feature = "board-huang1")]
        12 => Board::IrqBoard(IrqBoard::Huang1(Box::new(Huang1::new(memory, mapper)))),
        13 => Board::CpRom(CpRom::new(memory)),
        // MF-UNL-8 step 2: the Gouder SL-1632 (`UNL-SL1632`), the same Huang-1 with a PAL that
        // "moves the supervisor register address to $A131, overlapping the MMC3 and VRC2 registers
        // there" -- two modes and no MMC1, three independent CHR A18 bits over unequal window
        // groups, and the page's two Nestopia-sourced rules. MF-6b step 6 decided the number "a
        // second board, not a parameter" on exactly those differences; `Huang1`'s own note has the
        // table and Q-165 the four things the page does not say.
        #[cfg(feature = "board-huang1")]
        14 => Board::IrqBoard(IrqBoard::Huang1(Box::new(Huang1::new(memory, mapper)))),
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
        // MF-UNL-7 step 4: NES 2.0 mapper 298, the unlicensed *Lethal Weapon*. Its page gives no
        // register list because it needs none -- "a VRC4 clone in the VRC4b configuration" -- and
        // states one difference, at $F003. One `bool` on the board above and no variant of its
        // own; `Vrc2Vrc4`'s own note has why a monomorphisation would have cost `mmc5` +0.066 %.
        #[cfg(feature = "board-298")]
        298 => Board::Vrc2Vrc4(Vrc2Vrc4::new(memory, mapper)),
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
        // MF-UNL-7 step 2: mapper 112, which its page calls "quite similar to iNES Mapper 206 ...
        // however, the registers appears to be scrambled a bit". The scramble is in the **ports
        // and the register order** and nothing else, so it is one `bool` on the board above and
        // not a board: `Namcot108::write_register_112` has the rotation, the three ports and the
        // two things the page does not say (Q-161).
        #[cfg(feature = "board-112")]
        112 => Board::Namcot108(Namcot108::new(memory, mapper)),
        // MF-UNL-7 step 3: NTDEC's TC-112, five registers at $6000-$6004 under mask $E007 with
        // three CHR windows of unequal width. It shares the board above for the window table, the
        // eight-byte register file and the savestate block and for nothing else -- census §7.2's
        // "112 and 193 are near-206 boards" -- and the two decodes have no line in common.
        #[cfg(feature = "board-193")]
        193 => Board::Namcot108(Namcot108::new(memory, mapper)),
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
        // nesdev ("INES Mapper 111"): an image with CHR-ROM is the Chinese Ninja Ryukenden's
        // non-serialized MMC1 (MAPPERS-N), one without is GTROM.
        #[cfg(feature = "board-111")]
        111 if !memory.rom().chr_rom().is_empty() => {
            Board::DecodeBoard(DecodeBoard::Mmc1Parallel(Mmc1Parallel::new(memory)))
        }
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
        // MF-UNL-7 step 1: the Sachen 8259, one chip with four wirings. The number picks how far
        // the CHR register's six bits are shifted above the window the PPU's own address lines
        // choose -- and, on mapper 137 alone, a 1 KiB layout with a fixed upper half and an R4
        // that is one bit per block instead of three common upper bits. One `DecodeBoard` variant
        // for all four, on the `Jv001` precedent; `Sachen8259`'s own note has the table and the
        // two places its two pages disagree (Q-160).
        #[cfg(feature = "board-sachen8259")]
        137 | 138 | 139 | 141 => {
            Board::DecodeBoard(DecodeBoard::Sachen8259(Sachen8259::new(memory, mapper)))
        }
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
        // MF-UNL-7 step 6: the Sealie Computing RET-CUFROM revD, an **eighth** board of that
        // variant and the first one with CHR-RAM: one register over the whole of $8000-$FFFF,
        // `[...P PPCC]`, two bits of 8 KiB CHR-RAM bank and three of 16 KiB PRG bank, hard-wired
        // vertical mirroring and 8 KiB of WRAM. The CopyNES flash-write path is deliberately not
        // emulated; `SingleLatch`'s own note has the page's sentence and why.
        #[cfg(feature = "board-29")]
        29 => Board::DecodeBoard(DecodeBoard::SingleLatch(SingleLatch::new(
            memory,
            LatchBoard::RetCufrom,
        ))),
        // MF-UNL-3 step 4: the last of the seven, and the only one with more than one window.
        #[cfg(feature = "board-190")]
        190 => Board::DecodeBoard(DecodeBoard::SingleLatch(SingleLatch::new(
            memory,
            LatchBoard::Zemina190,
        ))),
        // MF-UNL-5 step 1: the 南晶 (Nánjīng) FC-001, whose 4 KiB CHR-RAM page is latched off the
        // **rise of PPU A13** and taken from PPU A9 -- the second board in this core whose CHR
        // window is the PPU address bus rather than a register, after mapper 96. Its feedback
        // latch at $5100-$5101, read back inverted at $5500-$5501, is the protection.
        #[cfg(feature = "board-163")]
        163 => Board::DecodeBoard(DecodeBoard::ChrWindowBoard(ChrWindowBoard::new(
            memory,
            ChrWindowKind::NanjingFc001,
        ))),
        // MF-UNL-5 step 2: the 外星 (Wàixīng) FS304, the same four ports and the same CHR-RAM
        // latch with a PRG composition of its own -- two mode bits where 163 has one, and a bank
        // line on `$5100` where 163 has its feedback latch. One `ChrWindowKind` and no variant.
        #[cfg(feature = "board-162")]
        162 => Board::DecodeBoard(DecodeBoard::ChrWindowBoard(ChrWindowBoard::new(
            memory,
            ChrWindowKind::WaixingFs304,
        ))),
        // MF-UNL-10 step 2: the 东达 (Dōngdá) PEC-9588 / 燕城 (Yànchéng) cy2000-3, whose five
        // ports are at $5000-$53FF and $5500 and whose save medium is a 93C66 serial EEPROM over
        // Microwire. The **fourth** board of this enum whose CHR addressing is latched off the
        // rise of PPU A13, after 96, 163 and 162 -- and the first to latch two lines on it, CHR
        // A12 from PPU A9 and CHR A3 from PPU A0, which is the page's 1 bpp video mode.
        #[cfg(feature = "board-164")]
        164 => Board::DecodeBoard(DecodeBoard::Pec9588(Pec9588::new(memory))),
        // MF-UNL-5 step 3: the 外星 (Wàixīng) FS305 / 南晶 (Nánjīng) NJ0430, whose four
        // registers are at **$4800-$4803** and whose CHR is unbanked -- the board of this lot that
        // its section heading does not describe (Q-154). Its own mechanism is the banked PRG-RAM
        // window at $6000-$7FFF, which is `Mapper::prg_ram_window`.
        #[cfg(feature = "board-178")]
        178 => Board::DecodeBoard(DecodeBoard::WaixingFs305(WaixingFs305::new(memory))),
        // MAPPERS-P: 551 is puNES's `mapper_178.c` with `$4803` banking CHR.
        #[cfg(feature = "board-551")]
        551 => Board::DecodeBoard(DecodeBoard::WaixingFs305(WaixingFs305::with_number(
            memory, mapper,
        ))),
        // MF-UNL-7 step 5: the single-chip homebrew board of *Magic Floor*, whose whole content
        // is a PRG-ROM chip -- no register, no latch, no bank line and **no CHR memory**. The
        // header's byte 6 picks which PPU address line drives CIRAM A10, and the 2 KiB of CIRAM
        // answers the pattern tables and the nametables both; `MagicFloor`'s own note has why that
        // 2 KiB lives on the cartridge's side of the connector here.
        #[cfg(feature = "board-218")]
        218 => Board::DecodeBoard(DecodeBoard::MagicFloor(MagicFloor::new(memory))),
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
        // MF-UNL-6: the AA6023 of the COOLBOY and MINDKIDS boards, an MMC3 with six outer bank
        // registers and up to 32 MiB of PRG-ROM. `Mmc3Outer::new` answers `None` for submappers
        // 2 to 11 -- the seven PCB codes whose register layout is not this one -- so a dump of
        // those is refused with REQ-CORE-011's message rather than run on the wrong lines
        // (Q-157).
        #[cfg(feature = "board-268")]
        268 => Board::IrqBoard(IrqBoard::Mmc3Outer(Box::new(Mmc3Outer::new(
            memory, mapper,
        )?))),
        // MF-UNL-2 step 5: the TXC 01-22017/01-22018 and the Yoko FC-001EM, an MMC3 whose
        // PRG-ROM is banked in 32 KiB amounts from $4100. The generic pirate variants of the
        // same number are deliberately not decoded; `Mmc3Txc189`'s own note has the page's
        // paragraph that settles it.
        #[cfg(feature = "board-189")]
        189 => Board::IrqBoard(IrqBoard::Mmc3Txc189(Mmc3Txc189::new(memory))),
        // MF-UNL-11 step 3: the Kasheng A9461, an MMC3 clone whose `$8002` bit 5 opens an
        // extended mode -- four independent 8 KiB PRG windows and eight 1 KiB CHR windows behind
        // a wider register index -- above a two-bit 128 KiB outer bank at $5002/$5003. The page
        // gives the number no submapper, so there is nothing to refuse.
        #[cfg(feature = "board-219")]
        219 => Board::IrqBoard(IrqBoard::Mmc3A9461(Box::new(Mmc3A9461::new(
            memory, mapper,
        )))),
        // MF-UNL-12 step 1: the Sachen UNL-SHERO, an MMC3 clone with 8 KiB of CHR-RAM beside its
        // CHR-ROM. `$4100` bit 6 picks the chip and bits 3-0 drive CHR A18 over the four 2 KiB
        // regions -- in the page's own order, which crosses the two bottom regions against the
        // bits -- and a read of the same port answers the board's title jumper. The board is
        // four-screen from its wiring, so the chip's `$A000` is inert (Q-172). The page gives the
        // number no submapper, so there is nothing to refuse.
        #[cfg(feature = "board-262")]
        262 => Board::IrqBoard(IrqBoard::Mmc3Shero(Box::new(Mmc3Shero::new(
            memory, mapper,
        )))),
        // MF-UNL-12 step 2: the BMW8544 / UNL-DRAGONFIGHTER, an MMC3 clone whose PRG banking,
        // mirroring and counter are the chip's and whose CHR banking is not. Two extra registers,
        // selected by a write to $6000-$7FFF and **filled by a read** of the same window from the
        // datum of the last CPU write to any address, carry the XOR of the 2 KiB bank at PPU
        // $0000-$07FF, CHR A18 of the 2 KiB bank at $0800-$0FFF and the whole 4 KiB bank at
        // $1000-$1FFF. The page gives the number no submapper, so there is nothing to refuse.
        #[cfg(feature = "board-292")]
        292 => Board::IrqBoard(IrqBoard::Mmc3Dragonfighter(Box::new(
            Mmc3Dragonfighter::new(memory, mapper),
        ))),
        // MF-UNL-12 step 3: the G0151-1, "used only for the Taiwanese game 封神榜". Four 8 KiB PRG
        // banks at $6000-$6003 and four 2 KiB CHR banks at $6004-$6007 under one mask, 2 KiB of
        // battery PRG-RAM at $6800-$6FFF, hard-wired mirroring from the header, and PRG A17 forced
        // high on a read of $FFE4-$FFE7, $FFEC-$FFEF, $FFF4-$FFF7 or $FFFC-$FFFF -- which is why
        // the $E000-$FFFF window is `NO_ROM_WINDOW`. No IRQ, so `DecodeBoard` and not `IrqBoard`.
        #[cfg(feature = "board-246")]
        246 => Board::DecodeBoard(DecodeBoard::G0151(G0151::new(memory))),
        // MF-UNL-11 step 2: the unnamed Gouder board of the Street Fighter IV backport, an
        // MMC3 clone whose PRG-ROM is banked in 32 KiB amounts from a register at $4800/$6800,
        // with a 256-byte protection lookup table behind four registers at $5800-$5FFF.
        // Submapper 1 is the copyright-stripped hack, which has none of those registers and takes
        // its one 32 KiB page from the chip's own R6 instead -- so there is nothing to refuse.
        #[cfg(feature = "board-208")]
        208 => Board::IrqBoard(IrqBoard::Mmc3Gouder208(Box::new(Mmc3Gouder208::new(
            memory, mapper,
        )))),
        // MF-UNL-11 step 1: the Sugar Softec UNL-8237 and UNL-8237A, an MMC3 clone whose
        // register addresses and bank indices are permuted by one of eight patterns `$5007`
        // selects at run time, with outer PRG and CHR banks at `$5001` and an NROM-like PRG
        // override at `$5000`. NES 2.0 submapper 1 is the UNL-8237A, which differs in the bit
        // assignment of `$5001` and in nothing else, so there is nothing to refuse.
        #[cfg(feature = "board-215")]
        215 => Board::IrqBoard(IrqBoard::Mmc3Unl8237(Box::new(Mmc3Unl8237::new(
            memory, mapper,
        )))),
        // MF-6a3 step 1: Nintendo's two MMC3 multicarts in one monomorphisation. The two
        // numbers differ only in what the outer register's bits drive, which is a field test
        // in `Mmc3Board::multicart_prg` and nowhere on a fetch or a cycle path.
        #[cfg(feature = "board-mmc3-multicart")]
        37 | 47 => Board::IrqBoard(IrqBoard::Mmc3Multicart(Mmc3Multicart::new(memory, mapper))),
        // MF-UNL-8 step 3: the Cony / Yoko chip-on-board ASIC, "the same chip-on-board ASIC" on
        // five PCBs that the mapper number and the NES 2.0 submapper pick between. One board with
        // four parameters -- the address remap of mapper 264, the 2 KiB CHR rewiring of 83.1 and
        // 264, the inner PRG bank's width and the PRG base's bit layout -- on the `Jv001`
        // precedent. `Cony::new` answers `None` for **submapper 2**, the one PCB whose
        // $6000-$7FFF is PRG-NVRAM rather than PRG-ROM, so a dump of it is refused with
        // REQ-CORE-011's message (Q-167).
        #[cfg(feature = "board-cony")]
        83 | 264 => Board::IrqBoard(IrqBoard::Cony(Box::new(Cony::new(memory, mapper)?))),
        // MF-UNL-8 step 4: mapper 91's two boards, which differ by **one mask bit** and the four
        // registers that bit makes reachable -- "submapper 0 has a PA12-based IRQ counter and
        // hard-wired mirroring; submapper 1 an M2-based IRQ counter and selectable mirroring". One
        // board with one `bool`; `Mapper091`'s own note has the table and Q-168 the four things the
        // page does not say, the outer bank register's own bit order among them.
        #[cfg(feature = "board-91")]
        91 => Board::IrqBoard(IrqBoard::Mapper091(Mapper091::new(memory))),
        // MF-UNL-4: the J.Y. Company ASIC, one chip and four PCB configurations. 209 is the
        // page's "standard implementation"; **211 and 35 are the same board here**, the page
        // calling 211 "a mere duplicate of INES Mapper 209 with correct emulation" (Q-149) and 35
        // "an exact duplicate" whose only difference is the 8 KiB of WRAM it states, which is this
        // file's `board_memory_sizes` row and not the board's code (Q-148). 90 is the one number
        // with a parameter: its jumper "inhibits … ROM nametables and Extended Mirroring".
        #[cfg(feature = "board-jy-asic")]
        35 | 90 | 209 | 211 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        // Lot MAPPERS-P: J.Y. multicarts, the same ASIC with a wider outer bank (`JyMulticart`).
        #[cfg(feature = "board-281")]
        281 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-282")]
        282 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-295")]
        295 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-358")]
        358 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-386")]
        386 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-387")]
        387 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-388")]
        388 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-397")]
        397 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-421")]
        421 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-394")]
        394 => Board::IrqBoard(IrqBoard::JyCompany(Box::new(JyCompany::new(
            memory, mapper,
        )))),
        #[cfg(feature = "board-44")]
        44 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-45")]
        45 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-49")]
        49 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-52")]
        52 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-134")]
        134 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-196")]
        196 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-197")]
        197 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-205")]
        205 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-238")]
        238 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-245")]
        245 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-249")]
        249 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-250")]
        250 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-168")]
        168 => Board::IrqBoard(IrqBoard::Racermate(Box::new(Racermate::new(memory)))),
        #[cfg(feature = "board-303")]
        303 => Board::IrqBoard(IrqBoard::Kaiser7017(Box::new(Kaiser7017::new(memory)))),
        #[cfg(feature = "board-530")]
        530 => Board::IrqBoard(IrqBoard::Ax5705(Box::new(Ax5705::new(memory)))),
        #[cfg(feature = "board-529")]
        529 => Board::IrqBoard(IrqBoard::T230(Box::new(T230::new(memory)))),
        #[cfg(feature = "board-266")]
        266 => Board::IrqBoard(IrqBoard::CityFighter(Box::new(CityFighter::new(memory)))),
        #[cfg(feature = "board-284")]
        284 => Board::IrqBoard(IrqBoard::Drip(Box::new(Drip::new(memory)))),
        #[cfg(feature = "board-682")]
        682 => Board::IrqBoard(IrqBoard::Rainbow(Box::new(Rainbow::new(memory)))),
        #[cfg(feature = "board-252")]
        252 => Board::IrqBoard(IrqBoard::Waixing252(Box::new(Waixing252::new(memory)))),
        #[cfg(feature = "board-253")]
        253 => Board::IrqBoard(IrqBoard::Waixing253(Box::new(Waixing253::new(memory)))),
        #[cfg(feature = "board-57")]
        57 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-58")]
        58 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-59")]
        59 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-63")]
        63 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-200")]
        200 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-201")]
        201 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-202")]
        202 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-203")]
        203 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-204")]
        204 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-212")]
        212 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-213")]
        213 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-214")]
        214 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-221")]
        221 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-226")]
        226 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-227")]
        227 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-229")]
        229 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-230")]
        230 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-231")]
        231 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-234")]
        234 => Board::DecodeBoard(DecodeBoard::Maxi15(Maxi15::new(memory))),
        #[cfg(feature = "board-53")]
        53 => Board::DecodeBoard(DecodeBoard::MiscN(MiscN::new(
            memory,
            MiscKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-487")]
        487 => Board::DecodeBoard(DecodeBoard::MiscN(MiscN::new(
            memory,
            MiscKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-518")]
        518 => Board::DecodeBoard(DecodeBoard::MiscN(MiscN::new(
            memory,
            MiscKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-328")]
        328 => Board::DecodeBoard(DecodeBoard::MiscN(MiscN::new(
            memory,
            MiscKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-519")]
        519 => Board::DecodeBoard(DecodeBoard::MiscN(MiscN::new(
            memory,
            MiscKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-323")]
        323 => Board::DecodeBoard(DecodeBoard::FaridSlrom(Box::new(FaridSlrom::new(memory)))),
        #[cfg(feature = "board-188")]
        188 => Board::DecodeBoard(DecodeBoard::BandaiKaraoke(BandaiKaraoke::new(memory))),
        #[cfg(feature = "board-233")]
        233 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-235")]
        235 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-236")]
        236 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-240")]
        240 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-241")]
        241 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-242")]
        242 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-244")]
        244 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-255")]
        255 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-46")]
        46 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-51")]
        51 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-60")]
        60 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-61")]
        61 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-62")]
        62 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-104")]
        104 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-108")]
        108 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-113")]
        113 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-120")]
        120 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-125")]
        125 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-170")]
        170 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-174")]
        174 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-177")]
        177 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-179")]
        179 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-216")]
        216 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-261")]
        261 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-274")]
        274 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-286")]
        286 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-126")]
        126 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-224")]
        224 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-259")]
        259 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-287")]
        287 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-313")]
        313 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-325")]
        325 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-191")]
        191 => Board::IrqBoard(IrqBoard::Mmc3ChrRam(Box::new(Mmc3ChrRamBoard::new(
            memory, mapper,
        )?))),
        #[cfg(feature = "board-40")]
        40 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-42")]
        42 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-43")]
        43 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-50")]
        50 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-106")]
        106 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-183")]
        183 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-117")]
        117 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-222")]
        222 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-6")]
        6 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-8")]
        8 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-17")]
        17 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-176")]
        176 => Board::IrqBoard(IrqBoard::Fk23c(Box::new(Fk23c::new(memory)))),
        #[cfg(feature = "board-39")]
        39 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-54")]
        54 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-148")]
        148 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-171")]
        171 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-312")]
        312 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-56")]
        56 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-142")]
        142 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-31")]
        31 => Board::DecodeBoard(DecodeBoard::FineRom(FineRom::new(
            memory,
            FineRomKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-103")]
        103 => Board::DecodeBoard(DecodeBoard::FineRom(FineRom::new(
            memory,
            FineRomKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-175")]
        175 => Board::DecodeBoard(DecodeBoard::FineRom(FineRom::new(
            memory,
            FineRomKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-307")]
        307 => Board::DecodeBoard(DecodeBoard::FineRom(FineRom::new(
            memory,
            FineRomKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-302")]
        302 => Board::DecodeBoard(DecodeBoard::FineRom(FineRom::new(
            memory,
            FineRomKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-305")]
        305 => Board::DecodeBoard(DecodeBoard::FineRom(FineRom::new(
            memory,
            FineRomKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-306")]
        306 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-217")]
        217 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-254")]
        254 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-165")]
        165 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-198")]
        198 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-199")]
        199 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-260")]
        260 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-513")]
        513 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-265")]
        265 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-283")]
        283 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-285")]
        285 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-288")]
        288 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-290")]
        290 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-55")]
        55 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-81")]
        81 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-122")]
        122 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-237")]
        237 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-389")]
        389 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-517")]
        517 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-271")]
        271 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-335")]
        335 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-338")]
        338 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-341")]
        341 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-343")]
        343 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-352")]
        352 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-360")]
        360 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-381")]
        381 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-337")]
        337 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-350")]
        350 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-375")]
        375 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-354")]
        354 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-380")]
        380 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-382")]
        382 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-390")]
        390 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-396")]
        396 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-400")]
        400 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-403")]
        403 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-409")]
        409 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-414")]
        414 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-415")]
        415 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-428")]
        428 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-429")]
        429 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-431")]
        431 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-433")]
        433 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-434")]
        434 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-437")]
        437 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-438")]
        438 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-481")]
        481 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-538")]
        538 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-541")]
        541 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-564")]
        564 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-525")]
        525 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-554")]
        554 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-297")]
        297 => Board::DecodeBoard(DecodeBoard::Mmc1Outer(Box::new(Mmc1Outer::new(
            memory,
            Mmc1OuterKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-374")]
        374 => Board::DecodeBoard(DecodeBoard::Mmc1Outer(Box::new(Mmc1Outer::new(
            memory,
            Mmc1OuterKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-404")]
        404 => Board::DecodeBoard(DecodeBoard::Mmc1Outer(Box::new(Mmc1Outer::new(
            memory,
            Mmc1OuterKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-550")]
        550 => Board::DecodeBoard(DecodeBoard::Mmc1Outer(Box::new(Mmc1Outer::new(
            memory,
            Mmc1OuterKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-267")]
        267 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-291")]
        291 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-315")]
        315 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-322")]
        322 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-327")]
        327 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-334")]
        334 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-339")]
        339 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-345")]
        345 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-361")]
        361 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-372")]
        372 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-377")]
        377 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-395")]
        395 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-399")]
        399 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-401")]
        401 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-410")]
        410 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-411")]
        411 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-420")]
        420 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-455")]
        455 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-456")]
        456 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-457")]
        457 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-516")]
        516 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-393")]
        393 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-412")]
        412 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-308")]
        308 => Board::IrqBoard(IrqBoard::VrcP(Box::new(VrcP::new(
            memory,
            VrcPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-384")]
        384 => Board::IrqBoard(IrqBoard::VrcP(Box::new(VrcP::new(
            memory,
            VrcPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-447")]
        447 => Board::IrqBoard(IrqBoard::VrcP(Box::new(VrcP::new(
            memory,
            VrcPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-524")]
        524 => Board::IrqBoard(IrqBoard::VrcP(Box::new(VrcP::new(
            memory,
            VrcPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-340")]
        340 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-311")]
        311 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-357")]
        357 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-368")]
        368 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-416")]
        416 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-417")]
        417 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-422")]
        422 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-534")]
        534 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-543")]
        543 => Board::DecodeBoard(DecodeBoard::Mmc1Outer(Box::new(Mmc1Outer::new(
            memory,
            Mmc1OuterKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-370")]
        370 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-344")]
        344 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-432")]
        432 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-356")]
        356 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-269")]
        269 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-527")]
        527 => Board::IrqBoard(IrqBoard::VrcP(Box::new(VrcP::new(
            memory,
            VrcPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-353")]
        353 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-100")]
        100 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-559")]
        559 => Board::IrqBoard(IrqBoard::VrcP(Box::new(VrcP::new(
            memory,
            VrcPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-557")]
        557 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-555")]
        555 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-471")]
        471 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-526")]
        526 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-539")]
        539 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-558")]
        558 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-272")]
        272 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-347")]
        347 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-442")]
        442 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-452")]
        452 => Board::DecodeBoard(DecodeBoard::MultiP(MultiP::new(
            memory,
            MultiPKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-362")]
        362 => Board::IrqBoard(IrqBoard::VrcP(Box::new(VrcP::new(
            memory,
            VrcPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-398")]
        398 => Board::IrqBoard(IrqBoard::VrcP(Box::new(VrcP::new(
            memory,
            VrcPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-359")]
        359 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-540")]
        540 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-528")]
        528 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-369")]
        369 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-556")]
        556 => Board::IrqBoard(IrqBoard::ChipSwitch(Box::new(ChipSwitch::new(
            memory,
            ChipSwitchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-351")]
        351 => Board::IrqBoard(IrqBoard::ChipSwitch(Box::new(ChipSwitch::new(
            memory,
            ChipSwitchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-512")]
        512 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-560")]
        560 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-413")]
        413 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-406")]
        406 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-451")]
        451 => Board::IrqBoard(IrqBoard::Mmc3P(Box::new(Mmc3P::new(
            memory,
            Mmc3PKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-547")]
        547 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-561")]
        561 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-256")]
        256 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-436")]
        436 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-446")]
        446 => Board::IrqBoard(IrqBoard::ChipSwitch(Box::new(ChipSwitch::new(
            memory,
            ChipSwitchKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-fds")]
        20 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-186")]
        186 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-342")]
        342 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-562")]
        562 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-355")]
        355 => Board::IrqBoard(IrqBoard::IrqP(Box::new(IrqP::new(
            memory,
            IrqPKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-299")]
        299 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-300")]
        300 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-336")]
        336 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-349")]
        349 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-289")]
        289 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-301")]
        301 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-314")]
        314 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-319")]
        319 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-258")]
        258 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-333")]
        333 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-348")]
        348 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-366")]
        366 => Board::IrqBoard(IrqBoard::Mmc3Clone(Box::new(Mmc3Clone::new(
            memory,
            Mmc3CloneKind::of(mapper)?,
        )))),
        #[cfg(feature = "board-309")]
        309 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-320")]
        320 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-329")]
        329 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-331")]
        331 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-332")]
        332 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-346")]
        346 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-521")]
        521 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-166")]
        166 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-167")]
        167 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-324")]
        324 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-522")]
        522 => Board::DecodeBoard(DecodeBoard::LatchMulti(LatchMulti::new(
            memory,
            LatchKind::of(mapper)?,
        ))),
        #[cfg(feature = "board-304")]
        304 => Board::IrqBoard(IrqBoard::IrqLatch(Box::new(IrqLatch::new(
            memory,
            IrqLatchKind::of(mapper)?,
        )))),
        _ => return None,
    })
}
