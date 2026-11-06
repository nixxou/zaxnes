//! The per-ROM table of VS. System configurations, keyed by the CRC-32 of the PRG and CHR
//! data (lot MF-VS step 1).
//!
//! **Why a table at all.** An iNES 1.0 header says only "this is a VS. System image" — byte 7
//! bit 0 — and carries no field for the PPU, the protection board or the stick wiring. NES 2.0
//! byte 13 does carry them, but most circulating VS dumps have an iNES 1.0 header, so the
//! configuration has to be recognised from the data. The key is the CRC-32 of the PRG-ROM and
//! CHR-ROM bytes with the header excluded, which is exactly what
//! [`crate::builtin_patches::data_crc`] already computes for the built-in patches, and which is
//! also the hash the source below is indexed by.
//!
//! **The source.** Every row is the `<vs hardware= ppu= />` and `<expansion type= />` of the
//! public **NES 2.0 ROM header database** (`nes20db.xml`, `NewRisingSun`; the build used is dated
//! 2026-09-26, 21 339 games of which these 74 have `<console type="1">`), with the CRC taken
//! from the same entry's `<rom crc32= />` — the hash of the data without the header. The three
//! enumerations the columns map onto are the nesdev wiki's own: "NES 2.0" byte 13's Vs. PPU
//! Type and Vs. Hardware Type, and byte 15's Default Expansion Device (`$04` "Vs. System
//! (1P via $4016)", `$05` "Vs. System (1P via $4017)", `$07` "Vs. Zapper").
//!
//! **No DIP switch column.** The owner's brief takes the DIP defaults "from the per-ROM table
//! when the source gives them, otherwise all off"; the header database records no DIP settings,
//! and the nesdev wiki has no per-game DIP list either, so every image starts with the eight
//! switches **off** and the host sets them (lot MF-VS step 5).
//!
//! The comment on each row is the mapper number and the database's own top-level category, kept
//! because the categories are what makes a surprising row readable: a `Bad Dumps` or
//! `Compatibility Hacks` entry is a real CRC of a circulating file, which is the only reason to
//! have it here.

use super::{VsHardware, VsPpuModel, VsWiring};

/// One row: a dump, and the VS. System it runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VsRom {
    crc: u32,
    title: &'static str,
    ppu: VsPpuModel,
    hardware: VsHardware,
    wiring: VsWiring,
}

impl VsRom {
    const fn new(
        crc: u32,
        title: &'static str,
        ppu: VsPpuModel,
        hardware: VsHardware,
        wiring: VsWiring,
    ) -> Self {
        Self {
            crc,
            title,
            ppu,
            hardware,
            wiring,
        }
    }

    /// CRC-32 of the PRG and CHR data, header excluded.
    #[must_use]
    pub const fn crc(self) -> u32 {
        self.crc
    }

    /// The dump's name in the header database, its folder dropped.
    #[must_use]
    pub const fn title(self) -> &'static str {
        self.title
    }

    /// The PPU the board carries.
    #[must_use]
    pub const fn ppu(self) -> VsPpuModel {
        self.ppu
    }

    /// The hardware type, protections and `DualSystem` included.
    #[must_use]
    pub const fn hardware(self) -> VsHardware {
        self.hardware
    }

    /// Which stick is player one's.
    #[must_use]
    pub const fn wiring(self) -> VsWiring {
        self.wiring
    }
}

/// The table, **sorted by CRC** so that [`find`] can bisect it; a test pins the order.
static TABLE: &[VsRom] = &[
    VsRom::new(
        0x0135_7944,
        "Battle City",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x071C_C00B,
        "Vs. Tennis (TE A-3)",
        VsPpuModel::Rc2c03,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x078C_981A,
        "Famidash vs. HUGE (v1.2.8)",
        VsPpuModel::Rc2c03,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 4, Homebrew
    VsRom::new(
        0x07A2_F3B2,
        "Vs. Gumshoe",
        VsPpuModel::Rc2c05_03,
        VsHardware::Unisystem,
        VsWiring::Zapper,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x0FA3_22C2,
        "Clu Clu Land",
        VsPpuModel::Rp2c04_0004,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x1201_2CD9,
        "Super Xevious: Gamp no Nazo",
        VsPpuModel::Rp2c04_0001,
        VsHardware::SuperXevious,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 206, Vs. System
    VsRom::new(
        0x159E_F3C1,
        "Star Luster",
        VsPpuModel::Rc2c03,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x1A0E_5D4C,
        "Vs. Balloon Fight [CHR overdump]",
        VsPpuModel::Rp2c04_0003,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0x1CA4_5A6D,
        "Ice Climber (IC4-4 B-1)",
        VsPpuModel::Rp2c04_0004,
        VsHardware::IceClimberJapan,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x2019_FE65,
        "Platoon",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 67, Vs. System
    VsRom::new(
        0x21A6_53C7,
        "Super Sky Kid",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 206, Vs. System
    VsRom::new(
        0x22BD_8A28,
        "Vs. BaseBall (BA E-1)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x2B85_420E,
        "Dr. Mario",
        VsPpuModel::Rp2c04_0003,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 1, Vs. System
    VsRom::new(
        0x2BF2_1F5F,
        "Vs. Wrecking Crew",
        VsPpuModel::Rp2c04_0002,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x2C1C_C6DA,
        "Vs. Balloon Fight",
        VsPpuModel::Rp2c04_0003,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x3058_6A4D,
        "Ice Climber (IC4-4 A-1)[doubled CHR]",
        VsPpuModel::Rp2c04_0004,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0x38C1_D042,
        "Vs. BaseBall (BA A-2)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x40EF_00FE,
        "Skate Kids {Vs. Super Mario Bros.}",
        VsPpuModel::Rp2c04_0004,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bootleg Hacks
    VsRom::new(
        0x412E_5A0D,
        "Vs. Gumshoe [padded]",
        VsPpuModel::Rc2c05_03,
        VsHardware::Unisystem,
        VsWiring::Zapper,
    ), // mapper 99, Compatibility Hacks
    VsRom::new(
        0x4836_1E79,
        "Stroke & Match Golf (GF4-2 F)",
        VsPpuModel::Rp2c04_0002,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x51C7_6943,
        "Vs. Mahjong",
        VsPpuModel::Rc2c03,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x52C5_01D0,
        "Vs. T.K.O. Boxing",
        VsPpuModel::Rp2c04_0003,
        VsHardware::TkoBoxing,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 206, Vs. System
    VsRom::new(
        0x535E_6152,
        "Mighty Bomb Jack",
        VsPpuModel::Rc2c05_02,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x5768_5E6C,
        "Vs. BaseBall (BA A-3)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x5FD6_F603,
        "Ice Climber (IC4-4 A-1)",
        VsPpuModel::Rp2c04_0004,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x63AB_F889,
        "Stroke & Match Golf (LG4 X)",
        VsPpuModel::Rp2c04_0002,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x6647_1EFE,
        "Pinball (PN3 B)",
        VsPpuModel::Rc2c03,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x67BA_C152,
        "Vs. Tennis (set 2)",
        VsPpuModel::Rc2c03,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x6B8E_7614,
        "Vs. BaseBall (BA E-1)[CHR overdump]",
        VsPpuModel::Rp2c04_0001,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0x70FF_B591,
        "Raid on Bungeling Bay",
        VsPpuModel::Rp2c04_0002,
        VsHardware::RaidOnBungelingBay,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x74F7_13B4,
        "Vs. The Goonies",
        VsPpuModel::Rp2c04_0003,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 75, Vs. System
    VsRom::new(
        0x766C_2CAC,
        "Soccer (SC4-3 X)",
        VsPpuModel::Rp2c04_0003,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x792C_F17F,
        "Vs. Mahjong [padded]",
        VsPpuModel::Rc2c03,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Compatibility Hacks
    VsRom::new(
        0x7CFF_0F84,
        "Vs. Super Mario Bros. (X)",
        VsPpuModel::Rp2c04_0004,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bootleg Hacks
    VsRom::new(
        0x832C_F592,
        "Freedom Force",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::Zapper,
    ), // mapper 206, Vs. System
    VsRom::new(
        0x8337_E123,
        "Atari R.B.I. Baseball (set 2)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::RbiBaseball,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 206, Vs. System
    VsRom::new(
        0x889A_02A9,
        "Stroke & Match Golf (LG4 E)",
        VsPpuModel::Rp2c04_0002,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x89A2_FD37,
        "Urban Champion",
        VsPpuModel::Rp2c04_0003,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x8C0C_2DF5,
        "Vs. Top Gun",
        VsPpuModel::Rc2c05_04,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 2, Vs. System
    VsRom::new(
        0x9768_E5E0,
        "Stroke & Match Golf (GF3 B)",
        VsPpuModel::Rc2c03,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x98E3_C75A,
        "Vs. Super Mario Bros. (SM4-4 E)",
        VsPpuModel::Rp2c04_0004,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0x994A_2711,
        "Vs. Mahjong [CHR overdump]",
        VsPpuModel::Rc2c03,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0x9AE2_BAA0,
        "Vs. Slalom",
        VsPpuModel::Rp2c04_0002,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xA213_30FE,
        "Vs. BaseBall (BA A-1)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xA3D6_0939,
        "Vs. BaseBall (BA A-1)[CHR overdump]",
        VsPpuModel::Rp2c04_0001,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0xABE1_A0C2,
        "Duck Hunt",
        VsPpuModel::Rc2c03,
        VsHardware::Unisystem,
        VsWiring::Zapper,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xABE8_E174,
        "Mach Rider (Fighting Course)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xAECA_9D42,
        "Tetris: The Soviet Mind Game [padded]",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Compatibility Hacks
    VsRom::new(
        0xB1C4_C508,
        "Stroke & Match Golf (GF4-2 X)",
        VsPpuModel::Rp2c04_0002,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xB281_6BF9,
        "Battle City (Vs.) [bad CHR data]",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0xB6AF_FFF8,
        "From Below (Vs. v0.10.0)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 0, Homebrew
    VsRom::new(
        0xBC92_75AB,
        "Famidash vs. (v1.2.8)",
        VsPpuModel::Rc2c03,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 4, Homebrew
    VsRom::new(
        0xBFD8_7C08,
        "Battle City (Vs.) [missing PRG and CHR data]",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 0, Bad Dumps
    VsRom::new(
        0xC20E_9CA7,
        "Ninja Jajamaru-kun",
        VsPpuModel::Rc2c05_01,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xC318_45AB,
        "Hogan's Alley",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::Zapper,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xC492_B4D1,
        "Tetris: The Soviet Mind Game",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xC953_21A8,
        "Excitebike (EB4-3 X)",
        VsPpuModel::Rp2c04_0003,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xCA3E_9B1A,
        "Pinball (PN4 E-1)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xCD05_9AEA,
        "Vs. Tennis (set 3)[right screen stays black]",
        VsPpuModel::Rc2c03,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0xD228_1882,
        "The Quest of Ki",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 206, Vs. System
    VsRom::new(
        0xE265_93E1,
        "Soccer (SC4-2 A)",
        VsPpuModel::Rp2c04_0002,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xE454_85A5,
        "Atari R.B.I. Baseball (set 1)",
        VsPpuModel::Rp2c04_0001,
        VsHardware::RbiBaseball,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 206, Vs. System
    VsRom::new(
        0xE587_809A,
        "Valkyrie no Bouken: Toki no Kagi Densetsu",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 206, Vs. System
    VsRom::new(
        0xE8B2_0197,
        "Mach Rider (Endurance Course)",
        VsPpuModel::Rp2c04_0002,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xEAA3_2008,
        "Super Xevious: Gamp no Nazo [music synchronization]",
        VsPpuModel::Rp2c04_0001,
        VsHardware::SuperXevious,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 206, Homebrew
    VsRom::new(
        0xEE8A_F512,
        "Vs. Gradius",
        VsPpuModel::Rp2c04_0001,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 75, Vs. System
    VsRom::new(
        0xEF7A_F338,
        "Ice Climber (IC4-4 X)",
        VsPpuModel::Rp2c04_0004,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xF06F_C822,
        "Excitebike (EB4-3 E)",
        VsPpuModel::Rp2c04_0003,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xF9DB_47BD,
        "Raid on Bungeling Bay [padded]",
        VsPpuModel::Rp2c04_0002,
        VsHardware::RaidOnBungelingBay,
        VsWiring::PlayerOneOnPort1,
    ), // mapper 99, Compatibility Hacks
    VsRom::new(
        0xFB0D_DDE7,
        "Excitebike (EB4-4 A)",
        VsPpuModel::Rp2c04_0004,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Vs. System
    VsRom::new(
        0xFC8E_252A,
        "Vs. BaseBall (BA A-3)[CHR overdump]",
        VsPpuModel::Rp2c04_0001,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0xFCFB_4236,
        "Vs. BaseBall (BA A-2)[CHR overdump]",
        VsPpuModel::Rp2c04_0001,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0xFD77_0F82,
        "Vs. Wrecking Crew [CHR overdump]",
        VsPpuModel::Rp2c04_0002,
        VsHardware::DualSystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 99, Bad Dumps
    VsRom::new(
        0xFFBE_F374,
        "Vs. Castlevania",
        VsPpuModel::Rp2c04_0002,
        VsHardware::Unisystem,
        VsWiring::PlayerOneOnPort0,
    ), // mapper 2, Vs. System
];

/// The whole table, for the harness and the tests.
#[must_use]
pub const fn rom_table() -> &'static [VsRom] {
    TABLE
}

/// The row for `crc`, or `None` when the dump is not one this table knows.
#[must_use]
pub fn find(crc: u32) -> Option<VsRom> {
    TABLE
        .binary_search_by(|row| row.crc.cmp(&crc))
        .ok()
        .map(|index| TABLE[index])
}
