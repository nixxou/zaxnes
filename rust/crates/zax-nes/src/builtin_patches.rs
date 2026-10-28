//! Patches built into the emulator and applied in memory to the ROMs they were made for.
//!
//! A known ROM is recognised by the CRC-32 of its data without the iNES header (the header of
//! one dump differs from another's), then patched with a BPS patch made against that data; the
//! player's header is kept. The patches carry the hacks' own bytes only: everything else is
//! copied from the player's ROM. A ROM nothing matches is loaded as it is.
//!
//! | ROM | Patch | Source |
//! |---|---|---|
//! | Zelda II - The Adventure of Link (USA) | widescreen (see `docs/WIDESCREEN-EXTENSION.md`) | a fork of the FiendsOfTheElements Zelda II disassembly (ca65), `tools/make_bps.py` |
//! | Mega Man 2 (USA) | widescreen objects (the margin decoder draws the background) | hooks and new code assembled into the original ROM (`mm2wide`, `tools/build.py`) |
//! | Super Mario Bros. (World) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and a 16 KiB PRG bank added for the extension's window (`smb1wide`, `tools/build.py`) |
//! | Ninja Gaiden III - The Ancient Ship of Doom (USA) | widescreen objects, advanced mode (the margin decoder alone draws the background) | hooks and new code assembled into the original ROM's free space (`ng3wide`, `tools/build.py`) |
//! | Super Mario Bros. 3 (USA) (Rev 1) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and code in its free space, part of it seen through the extension's window; the ROM keeps its size (`smb3wide`, `tools/build.py`) |
//! | Castlevania (USA), and its Rev 1 | widescreen objects and enemies, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and code in its free space, seen through the extension's window open from power-on; the ROM keeps its size; one source for both builds (`cvwide`, `tools/build.py`) |
//! | Adventure Island (USA) | widescreen sprites and objects, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and a 16 KiB PRG bank added for the extension's window (`aiwide`, `tools/build.py`) |
//! | Excitebike (Japan, USA) | widescreen riders, advanced mode (the margin decoder alone draws the background) | hooks into both copies of the original 16 KiB (the PRG laid out as the game, its copy for the $C000 mirror, then a 16 KiB bank for the extension's window) (`excitebikewide`, `tools/build.py`) |
//! | Final Fantasy (USA) | widescreen people and vehicles, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and code in its free space, seen through the extension's window open from power-on; the ROM keeps its size (`ff1wide`, `tools/build.py`) |
//! | Metal Force (Korea) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks and new code assembled into the original ROM (`metalforcewide`, `tools/build.py`) |
//! | Blaster Master (USA) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and a 16 KiB PRG bank added for the extension's window, the fixed bank copied to bank 15 (`blastermasterwide`, `tools/build.py`) |
//! | Metroid (USA) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks into the fixed bank and a 16 KiB PRG bank added for the extension's window, the fixed bank copied to bank 15 (`metroidwide`, `tools/build.py`) |
//! | Mega Man 4 (USA), and its Rev 1 | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and a 16 KiB PRG bank added for the extension's window, the fixed banks copied to the end; one source for both builds (`mm4wide`, `tools/build.py`) |
//! | Mega Man 5 (USA) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and a 16 KiB PRG bank added for the extension's window, the fixed banks copied to the end (`mm5wide`, `tools/build.py`) |
//! | Mega Man 6 (USA) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): hooks checked and applied by the core, the code in the extension's memory, the ROM keeping its size; "Invincible" a component of its own, with or without the advanced mode (`mm6wide`, `tools/build.py --pack`) |
//! | Mega Man 3 (USA) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and a 16 KiB PRG bank added for the extension's window, the fixed banks copied to the end (`mm3wide`, `tools/build.py`) |
//! | Mega Man (USA) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | hooks into the original ROM and a 16 KiB PRG bank added for the extension's window, the fixed bank copied to bank 15 (`mm1wide`, `tools/build.py`) |

/// One built-in patch.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinPatch {
    /// What the patch does, for the log.
    pub name: &'static str,
    /// The game, as the names of its ROMs and profiles start ("Zelda II").
    pub game: &'static str,
    /// The name of the ROM it is made for, as the No-Intro set has it.
    pub rom_name: &'static str,
    /// CRC-32 of the ROM data the patch applies to, iNES header excluded.
    pub source_crc: u32,
    /// The BPS patch.
    pub bps: &'static [u8],
    /// The options the player can turn on or off (a menu of the host), told to the ROM through
    /// the widescreen extension's `$500A`.
    pub options: &'static [HackOption],
    /// Puts right, in CPU RAM, a machine restored from a state made the other way (with the
    /// patch while it is off, or without it while it is on): the video memory of one is not laid
    /// out as the other expects, so the game is sent back to a step that redraws it.
    pub after_other_way_state: Option<fn(&mut [u8])>,
    /// An advanced mode of the game's widescreen (Rust-only, owner request): applied only when
    /// the player asks for it, the game widening without it through its margin decoder alone.
    pub advanced: bool,
    /// The widescreen extension unlocked at power-on and reset with its PRG window open on
    /// (`$5008`, `$5009`): for a ROM with no room left for the code that would unlock it, whose
    /// reset can then jump straight into the window.
    pub boot_window: Option<(u8, u8)>,
    /// With `boot_window`, the window stops at `$5FFF` from power-on (`$5007` bit 4 clear): for
    /// a game with PRG-RAM at `$6000-$7FFF` (Metroid), which it reads before the hack's code
    /// has run. Otherwise the boot window covers `$6000-$7FFF` too, a ROM with no room for
    /// anything there.
    pub boot_window_keeps_prg_ram: bool,
    /// The game's wait for the next frame, first to last address of its loop: the overclock
    /// (`$5006`) ends as soon as the CPU is there, the cycles left being of no use to the game
    /// (Rust-only, owner request: "yield"; the same game, the CPU time only when a frame needs it).
    pub idle_loop: Option<(u16, u16)>,
}

/// Name of the Zelda II (USA) widescreen patch.
pub const ZELDA2_USA: &str = "Zelda II (USA) widescreen";

/// Name of the Mega Man 2 (USA) widescreen patch.
pub const MEGAMAN2_USA: &str = "Mega Man 2 (USA) widescreen";

/// Name of the Super Mario Bros. (World) widescreen patch (advanced mode).
pub const SMB1_WORLD: &str = "Super Mario Bros. (World) widescreen";

/// Name of the Ninja Gaiden III (USA) widescreen patch (advanced mode).
pub const NG3_USA: &str = "Ninja Gaiden III (USA) widescreen";
/// Name of the Super Mario Bros. 3 (USA) (Rev 1) widescreen patch (advanced mode).
pub const SMB3_USA: &str = "Super Mario Bros. 3 (USA) (Rev 1) widescreen";
/// Name of the Castlevania (USA) widescreen patch (advanced mode).
pub const CASTLEVANIA_USA: &str = "Castlevania (USA) widescreen";
/// Name of the Castlevania (USA) (Rev 1) widescreen patch (advanced mode).
pub const CASTLEVANIA_USA_REV1: &str = "Castlevania (USA) (Rev 1) widescreen";
/// Name of the Adventure Island (USA) widescreen patch (advanced mode).
pub const ADVENTURE_ISLAND_USA: &str = "Adventure Island (USA) widescreen";
/// Name of the Excitebike (Japan, USA) widescreen patch (advanced mode).
pub const EXCITEBIKE: &str = "Excitebike (Japan, USA) widescreen";
/// Name of the Final Fantasy (USA) widescreen patch (advanced mode).
pub const FINAL_FANTASY_USA: &str = "Final Fantasy (USA) widescreen";
/// Name of the Metal Force (Korea) widescreen patch (advanced mode).
pub const METALFORCE_KOREA: &str = "Metal Force (Korea) widescreen";
/// Name of the Blaster Master (USA) widescreen patch (advanced mode).
pub const BLASTERMASTER_USA: &str = "Blaster Master (USA) widescreen";
/// Name of the Metroid (USA) widescreen patch (advanced mode).
pub const METROID_USA: &str = "Metroid (USA) widescreen";
/// Name of the Mega Man (USA) widescreen patch (advanced mode).
pub const MEGAMAN1_USA: &str = "Mega Man (USA) widescreen";
/// Name of the Mega Man 3 (USA) widescreen patch (advanced mode).
pub const MEGAMAN3_USA: &str = "Mega Man 3 (USA) widescreen";
/// Name of the Mega Man 4 (USA) widescreen patch (advanced mode).
pub const MEGAMAN4_USA: &str = "Mega Man 4 (USA) widescreen";
/// Name of the Mega Man 4 (USA) (Rev 1) widescreen patch (advanced mode).
pub const MEGAMAN4_USA_REV1: &str = "Mega Man 4 (USA) (Rev 1) widescreen";
/// Name of the Mega Man 5 (USA) widescreen patch (advanced mode).
pub const MEGAMAN5_USA: &str = "Mega Man 5 (USA) widescreen";
/// Name of the Mega Man 6 (USA) widescreen patch (advanced mode).
pub const MEGAMAN6_USA: &str = "Mega Man 6 (USA) widescreen";

/// The built-in patches.
pub const PATCHES: [BuiltinPatch; 19] = [
    BuiltinPatch {
        name: ZELDA2_USA,
        game: "Zelda II",
        rom_name: "Zelda II - The Adventure of Link (USA)",
        source_crc: 0xBA32_2865,
        bps: include_bytes!("../patches/zelda2-usa-widescreen.bps"),
        options: &[],
        after_other_way_state: Some(zelda2_after_other_way_state),
        advanced: false,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        // Everything runs in the NMI; the main loop waits for a game mode that loads an area.
        idle_loop: Some((0xC010, 0xC01A)),
    },
    BuiltinPatch {
        name: MEGAMAN2_USA,
        game: "Mega Man 2",
        rom_name: "Mega Man 2 (USA)",
        source_crc: 0x0FCF_C04D,
        bps: include_bytes!("../patches/megaman2-usa-widescreen.bps"),
        options: &[
            HackOption {
                key: "enemies_in_margins",
                label: "Enemies in the margins",
                label_fr: "Ennemis dans les marges",
                bit: 0,
                default: true,
            },
            // A testing aid: no damage, spikes harmless (pits still kill).
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        advanced: false,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        // The frame's wait: LDA $1D / BEQ (the NMI sets $1D).
        idle_loop: Some((0xC08E, 0xC091)),
    },
    BuiltinPatch {
        name: SMB1_WORLD,
        game: "Super Mario Bros.",
        rom_name: "Super Mario Bros. (World)",
        source_crc: 0xD445_F698,
        bps: include_bytes!("../patches/smb1-world-widescreen.bps"),
        options: &[
            // Testing aids: B on the title screen goes through every level; Mario never hurt
            // (falls and the timer still kill).
            HackOption {
                key: "level_select",
                label: "Level select (B on the title screen)",
                label_fr: "Sélecteur de niveau (B sur l'écran titre)",
                bit: 0,
                default: false,
            },
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (2)
        // is open in the window from power-on.
        boot_window: Some((0x82, 0x00)),
        boot_window_keeps_prg_ram: false,
        // Everything runs in the NMI; the main loop is a JMP to itself.
        idle_loop: Some((0x8057, 0x8059)),
    },
    BuiltinPatch {
        name: NG3_USA,
        game: "Ninja Gaiden III",
        rom_name: "Ninja Gaiden III - The Ancient Ship of Doom (USA)",
        source_crc: 0x902E_3168,
        bps: include_bytes!("../patches/ng3-usa-widescreen.bps"),
        options: &[
            HackOption {
                key: "enemies_in_margins",
                label: "Enemies in the margins",
                label_fr: "Ennemis dans les marges",
                bit: 0,
                default: true,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        // The frame's wait in bank 4: BIT $15 / BPL (the NMI sets $15).
        idle_loop: Some((0x9A87, 0x9A8A)),
    },
    BuiltinPatch {
        name: SMB3_USA,
        game: "Super Mario Bros. 3",
        rom_name: "Super Mario Bros. 3 (USA) (Rev 1)",
        source_crc: 0x2E63_01ED,
        bps: include_bytes!("../patches/smb3-usa-rev1-widescreen.bps"),
        options: &[
            // Testing aids: B on the 1P/2P menu opens the game's own debug menu (Up/Down: the
            // world); Mario never hurt (pits still kill).
            HackOption {
                key: "level_select",
                label: "Level select (B on the 1P/2P menu)",
                label_fr: "Sélecteur de niveau (B sur le menu 1P/2P)",
                bit: 0,
                default: false,
            },
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The NMI unlocks the extension and opens its window itself (code in bank 31's free
        // space).
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        // GraphicsBuf_Prep_And_WaitVSync: LDA VBlank_Tick / BPL, until the NMI counts it down.
        idle_loop: Some((0x96FB, 0x96FE)),
    },
    BuiltinPatch {
        name: CASTLEVANIA_USA,
        game: "Castlevania",
        rom_name: "Castlevania (USA)",
        source_crc: 0x0AC1_AA8F,
        bps: include_bytes!("../patches/castlevania-usa-widescreen.bps"),
        options: CASTLEVANIA_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        // No room in the fixed bank for code that would unlock the extension: the free end of
        // PRG bank 5 (from its page $1F) is open in the window from power-on.
        boot_window: Some((0x85, 0x1F)),
        boot_window_keeps_prg_ram: false,
        // Everything runs in the NMI; the main loop stirs the random number.
        idle_loop: Some((0xC030, 0xC03E)),
    },
    BuiltinPatch {
        name: CASTLEVANIA_USA_REV1,
        game: "Castlevania",
        rom_name: "Castlevania (USA) (Rev 1)",
        source_crc: 0xB668_C7FC,
        bps: include_bytes!("../patches/castlevania-usa-rev1-widescreen.bps"),
        options: CASTLEVANIA_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        // No room in the fixed bank for code that would unlock the extension: the free end of
        // PRG bank 5 (from its page $1F) is open in the window from power-on.
        boot_window: Some((0x85, 0x1F)),
        boot_window_keeps_prg_ram: false,
        // Everything runs in the NMI; the main loop stirs the random number.
        idle_loop: Some((0xC030, 0xC03E)),
    },
    BuiltinPatch {
        name: ADVENTURE_ISLAND_USA,
        game: "Adventure Island",
        rom_name: "Adventure Island (USA)",
        source_crc: 0xF8A7_13BE,
        bps: include_bytes!("../patches/adventure-island-usa-widescreen.bps"),
        options: &[
            // Testing aids: Select on the title screen goes through every round; no injury from
            // the enemies (falls, the water and hunger still kill).
            HackOption {
                key: "level_select",
                label: "Level select (Select on the title screen)",
                label_fr: "Sélecteur de niveau (Select sur l'écran titre)",
                bit: 0,
                default: false,
            },
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the sprites and objects.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (2)
        // is open in the window from power-on.
        boot_window: Some((0x82, 0x00)),
        boot_window_keeps_prg_ram: false,
        // The main loop's wait for the NMI: LDA $0B / BNE.
        idle_loop: Some((0xB759, 0xB75C)),
    },
    BuiltinPatch {
        name: EXCITEBIKE,
        game: "Excitebike",
        rom_name: "Excitebike (Japan, USA) (En)",
        source_crc: 0x49AE_B3A6,
        bps: include_bytes!("../patches/excitebike-widescreen.bps"),
        options: &[
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the riders.
        advanced: true,
        // No free byte in the game for code that would unlock the extension: its added PRG bank
        // (2, after the game and its copy) is open in the window from power-on.
        boot_window: Some((0x82, 0x00)),
        boot_window_keeps_prg_ram: false,
        // Everything runs in the NMI; the main loop draws random numbers while it waits:
        // JSR $D326 / LDA $48 / BMI / BEQ.
        idle_loop: Some((0xC1D8, 0xC1E0)),
    },
    BuiltinPatch {
        name: FINAL_FANTASY_USA,
        game: "Final Fantasy",
        rom_name: "Final Fantasy (USA)",
        source_crc: 0xCEBD_2A31,
        bps: include_bytes!("../patches/final-fantasy-usa-widescreen.bps"),
        options: &[
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 0,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the people and vehicles.
        advanced: true,
        // No room in the fixed bank for code that would unlock the extension: the free end of
        // PRG bank 6 (from its page $30) is open in the window from power-on.
        boot_window: Some((0x86, 0x30)),
        boot_window_keeps_prg_ram: false,
        // The wait for the NMI: JMP to itself, the NMI returning to the wait's caller.
        idle_loop: Some((0xFEBB, 0xFEBD)),
    },
    BuiltinPatch {
        name: METALFORCE_KOREA,
        game: "Metal Force",
        rom_name: "Metal Force (Korea) (Unl)",
        source_crc: 0xEC6D_910E,
        bps: include_bytes!("../patches/metalforce-korea-widescreen.bps"),
        options: &[
            // Testing aids: left / right on the mission screen pick the mission; no damage (pits
            // still kill).
            HackOption {
                key: "level_select",
                label: "Level select (left / right on the mission screen)",
                label_fr: "Sélecteur de niveau (gauche / droite sur l'écran de mission)",
                bit: 0,
                default: false,
            },
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        // Everything runs in the NMI; the reset ends in a JMP to itself.
        idle_loop: Some((0xFFC3, 0xFFC5)),
    },
    BuiltinPatch {
        name: BLASTERMASTER_USA,
        game: "Blaster Master",
        rom_name: "Blaster Master (USA)",
        source_crc: 0x3F0F_D764,
        bps: include_bytes!("../patches/blastermaster-usa-widescreen.bps"),
        options: &[
            // Testing aids: a new game starts with the game's own area select (up / down,
            // Start); no damage (the player only).
            HackOption {
                key: "level_select",
                label: "Level select (at a new game)",
                label_fr: "Sélecteur de niveau (en début de partie)",
                bit: 0,
                default: false,
            },
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (8)
        // is open in the window from power-on.
        boot_window: Some((0x88, 0x00)),
        boot_window_keeps_prg_ram: false,
        // The frame's wait: BIT $12 / BMI, until the NMI clears it.
        idle_loop: Some((0xE93A, 0xE93D)),
    },
    BuiltinPatch {
        name: METROID_USA,
        game: "Metroid",
        rom_name: "Metroid (USA)",
        source_crc: 0x7008_0810,
        bps: include_bytes!("../patches/metroid-usa-widescreen.bps"),
        options: &[
            // Testing aids: a new game starts in the area a direction held at START picks, as a
            // password starts one; no damage.
            HackOption {
                key: "level_select",
                label: "Level select (hold a direction at START: up Norfair, right Kraid, left Tourian, down Ridley)",
                label_fr: "Sélecteur de niveau (maintenir une direction sur START : haut Norfair, droite Kraid, gauche Tourian, bas Ridley)",
                bit: 0,
                default: false,
            },
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (8)
        // is open in the window from power-on (2 KiB: the game's PRG-RAM is at $6000).
        boot_window: Some((0x88, 0x00)),
        boot_window_keeps_prg_ram: true,
        // The main loop's wait: TAY / LDA $1A / BNE / JMP (the NMI sets $1A).
        idle_loop: Some((0xC0CB, 0xC0D2)),
    },
    BuiltinPatch {
        name: MEGAMAN1_USA,
        game: "Mega Man",
        rom_name: "Mega Man (USA)",
        source_crc: 0x6EE4_BB0A,
        bps: include_bytes!("../patches/megaman1-usa-widescreen.bps"),
        options: &[
            // A testing aid: no damage (pits and spikes still kill).
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (8)
        // is open in the window from power-on.
        boot_window: Some((0x88, 0x00)),
        boot_window_keeps_prg_ram: false,
        // The frame's wait: LDA $1F / BEQ, until the NMI sets it.
        idle_loop: Some((0xC02B, 0xC02E)),
    },
    BuiltinPatch {
        name: MEGAMAN3_USA,
        game: "Mega Man 3",
        rom_name: "Mega Man 3 (USA)",
        source_crc: 0x603A_AA57,
        bps: include_bytes!("../patches/megaman3-usa-widescreen.bps"),
        options: &[
            // A testing aid: no damage from an enemy's touch (pits and spikes still kill).
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (16
        // in 16 KiB units) is open in the window from power-on.
        boot_window: Some((0x90, 0x00)),
        boot_window_keeps_prg_ram: false,
        // The task scheduler's wait for a task to wake (the NMI wakes them).
        idle_loop: Some((0xFEAD, 0xFEC2)),
    },
    BuiltinPatch {
        name: MEGAMAN4_USA,
        game: "Mega Man 4",
        rom_name: "Mega Man 4 (USA)",
        source_crc: 0x2BC6_7AA8,
        bps: include_bytes!("../patches/megaman4-usa-widescreen.bps"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (32
        // in 16 KiB units) is open in the window from power-on.
        boot_window: Some((0xA0, 0x00)),
        boot_window_keeps_prg_ram: false,
        // The task scheduler's wait for a task to wake (the NMI wakes them).
        idle_loop: Some((0xFEA8, 0xFEBC)),
    },
    BuiltinPatch {
        name: MEGAMAN4_USA_REV1,
        game: "Mega Man 4",
        rom_name: "Mega Man 4 (USA) (Rev 1)",
        source_crc: 0x18A2_E74F,
        bps: include_bytes!("../patches/megaman4-usa-rev1-widescreen.bps"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (32
        // in 16 KiB units) is open in the window from power-on.
        boot_window: Some((0xA0, 0x00)),
        boot_window_keeps_prg_ram: false,
        // The task scheduler's wait for a task to wake (the NMI wakes them).
        idle_loop: Some((0xFEA8, 0xFEBC)),
    },
    BuiltinPatch {
        name: MEGAMAN5_USA,
        game: "Mega Man 5",
        rom_name: "Mega Man 5 (USA)",
        source_crc: 0x3EDC_F7E8,
        bps: include_bytes!("../patches/megaman5-usa-widescreen.bps"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: its added PRG bank (16
        // in 16 KiB units) is open in the window from power-on.
        boot_window: Some((0x90, 0x00)),
        boot_window_keeps_prg_ram: false,
        // The task scheduler's wait for a task to wake (the NMI wakes them).
        idle_loop: Some((0xFEAE, 0xFEC3)),
    },
    BuiltinPatch {
        name: MEGAMAN6_USA,
        game: "Mega Man 6",
        rom_name: "Mega Man 6 (USA)",
        source_crc: 0x9887_98A8,
        // A hack pack (see `crate::hack_pack`): the code in the extension's memory, the hooks
        // checked and applied by the core; "Invincible" a component of its own.
        bps: include_bytes!("../patches/megaman6-usa-widescreen.zxpk"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // No room in the game for code that would unlock the extension: the pack's code is open
        // in the window from power-on.
        boot_window: Some((0x80, 0x00)),
        boot_window_keeps_prg_ram: false,
        // The task scheduler's wait for the NMI (no task ready).
        idle_loop: Some((0xC581, 0xC584)),
    },
];

/// The options of both Mega Man 4 patches, the same hack built for each, and of the Mega Man 5 and 6
/// ones: a testing aid, off by default, and the overclock.
const MEGAMAN4_OPTIONS: &[HackOption] = &[
    // No damage from an enemy's touch (pits and spikes still kill).
    HackOption {
        key: "invincible",
        label: "Invincible",
        label_fr: "Invincible",
        bit: 1,
        default: false,
    },
    // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
    HackOption {
        key: "no_slowdown",
        label: "No slowdown",
        label_fr: "Sans ralentissements",
        bit: 2,
        default: true,
    },
];

/// The options of both Castlevania patches, the same hack built for each: testing aids, off by
/// default, and the overclock.
const CASTLEVANIA_OPTIONS: &[HackOption] = &[
    // Select in a stage: the next stage (after the last, the first).
    HackOption {
        key: "level_select",
        label: "Level select (Select: next stage)",
        label_fr: "Sélecteur de niveau (Select : stage suivant)",
        bit: 0,
        default: false,
    },
    // Simon never hurt (pits still kill).
    HackOption {
        key: "invincible",
        label: "Invincible",
        label_fr: "Invincible",
        bit: 1,
        default: false,
    },
    // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
    HackOption {
        key: "no_slowdown",
        label: "No slowdown",
        label_fr: "Sans ralentissements",
        bit: 2,
        default: true,
    },
];

/// Zelda II: the world map is laid out in four nametables by the hack and in two by the game, and
/// only redrawn when it is entered; a map state made the other way goes back through
/// the game's own map initialisation (game mode 2), which redraws it around Link.
fn zelda2_after_other_way_state(ram: &mut [u8]) {
    const GAME_MODE: usize = 0x0736;
    const WORLD_MAP: u8 = 0x05;
    if let Some(mode) = ram.get_mut(GAME_MODE).filter(|mode| **mode == WORLD_MAP) {
        *mode = 0x02;
    }
}

impl BuiltinPatch {
    /// The bits of `$500A` with every option as it is by default.
    #[must_use]
    pub fn default_options(&self) -> u8 {
        self.options
            .iter()
            .filter(|option| option.default)
            .fold(0, |bits, option| bits | (1 << (option.bit & 7)))
    }

    /// Whether option `key` is an aid of the patch's hack pack: a component of its own, applied
    /// with or without the advanced mode.
    #[must_use]
    pub fn is_aid(&self, key: &str) -> bool {
        crate::hack_pack::HackPack::read(self.bps).is_some_and(|pack| pack.component(key).is_some())
    }

    /// The patch's version: the CRC-32 of the ROM data it produces, which any change to the
    /// hack changes (read from the BPS footer); for a hack pack, the pack's CRC-32.
    #[must_use]
    pub fn version(&self) -> u32 {
        if crate::hack_pack::is_pack(self.bps) {
            return crc32(self.bps);
        }
        self.bps
            .len()
            .checked_sub(8)
            .and_then(|at| self.bps.get(at..at + 4))
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map_or(0, u32::from_le_bytes)
    }
}

/// An option of a built-in patch: bit `bit` of the extension's `$500A`, which the ROM reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HackOption {
    /// Stable key, kept in the host's settings.
    pub key: &'static str,
    /// Its label in English and in French.
    pub label: &'static str,
    pub label_fr: &'static str,
    /// Its bit of `$500A`.
    pub bit: u8,
    /// Whether it is on until the player says otherwise.
    pub default: bool,
}

/// A ROM image with the built-in patch it matched applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchedRom {
    /// The patched image, iNES header included when the original had one.
    pub image: Vec<u8>,
    /// The patch's name.
    pub name: &'static str,
    /// The code of a hack pack, which the extension's PRG window shows; empty for a BPS patch.
    pub code: &'static [u8],
}

/// Size of an iNES header.
const HEADER_SIZE: usize = 16;

/// The image with the built-in patch made for it, or `None` when no patch matches (or the one
/// that matches does not apply cleanly, which its checksums would tell).
#[must_use]
pub fn apply(image: &[u8]) -> Option<PatchedRom> {
    apply_with(image, None)
}

/// The same, and when no patch is made for `image`, `forced` applied all the same (Rust-only,
/// owner request: a ROM hack of the game, or another version of it, may well run with it): its
/// checksums are not asked for, only the size of the data it was made for.
#[must_use]
pub fn apply_with(image: &[u8], forced: Option<&'static BuiltinPatch>) -> Option<PatchedRom> {
    let (start, end) = data_bounds(image);
    let data = &image[start..end];
    let crc = crc32(data);
    let (patch, checked) = match PATCHES.iter().find(|patch| patch.source_crc == crc) {
        Some(patch) => (patch, true),
        None => match pack_matching(data) {
            Some((patch, _)) => (patch, true),
            None => (forced?, false),
        },
    };
    if crate::hack_pack::is_pack(patch.bps) {
        // A hack pack: its hooks all checked, the widescreen's applied; the ROM keeps its size,
        // the code goes to the extension's memory.
        let pack = crate::hack_pack::HackPack::read(patch.bps).filter(|pack| pack.matches(data))?;
        let mut patched_image = image[..end].to_vec();
        if let Some(widescreen) = pack.component(crate::hack_pack::WIDESCREEN) {
            widescreen.apply(&mut patched_image[start..], true);
        }
        return Some(PatchedRom {
            image: patched_image,
            name: patch.name,
            code: pack.code,
        });
    }
    let patched = apply_bps_checked(patch.bps, data, checked).ok()?;
    // What comes before the data (the header, a trainer) stays; bytes past the size the header
    // gives do not.
    let mut patched_image = Vec::with_capacity(start + patched.len());
    patched_image.extend_from_slice(&image[..start]);
    if patched.len() != data.len() {
        resize_prg(&mut patched_image, data.len(), patched.len());
    }
    patched_image.extend_from_slice(&patched);
    Some(PatchedRom {
        image: patched_image,
        name: patch.name,
        code: &[],
    })
}

/// The built-in hack pack whose hooks all find their bytes in `data` (a ROM's PRG-ROM and
/// CHR-ROM), whatever its CRC: another build of the game whose hooked code is the same.
fn pack_matching(data: &[u8]) -> Option<(&'static BuiltinPatch, crate::hack_pack::HackPack)> {
    PATCHES.iter().find_map(|patch| {
        let pack = crate::hack_pack::HackPack::read(patch.bps)?;
        pack.matches(data).then_some((patch, pack))
    })
}

/// The hack pack of the built-in patch made for `image`, when that patch is one and its hooks
/// all find their bytes: what the core takes the pack's aids from (their options), with or
/// without the advanced mode.
#[must_use]
pub fn pack_for(image: &[u8]) -> Option<(&'static BuiltinPatch, crate::hack_pack::HackPack)> {
    let data = rom_data(image);
    match for_data_crc(crc32(data)) {
        Some(patch) => {
            let pack = crate::hack_pack::HackPack::read(patch.bps)?;
            pack.matches(data).then_some((patch, pack))
        }
        None => pack_matching(data),
    }
}

/// A patch that adds PRG-ROM (16 KiB banks after the game's, for the widescreen extension's PRG
/// window; the CHR-ROM stays as it is, after them): the header in `header` (the bytes before the
/// data) gets the new PRG size, data of `before` bytes becoming `after`.
fn resize_prg(header: &mut [u8], before: usize, after: usize) {
    const BANK: usize = 16 * 1024;
    if header.len() < HEADER_SIZE || !header.starts_with(b"NES") {
        return;
    }
    let Some(added) = after.checked_sub(before).filter(|added| added % BANK == 0) else {
        return;
    };
    let nes2 = header[7] & 0x0C == 0x08;
    let high = if nes2 { header[9] & 0x0F } else { 0 };
    let banks = u16::from_le_bytes([header[4], high]) + u16::try_from(added / BANK).unwrap_or(0);
    let [low, high] = banks.to_le_bytes();
    header[4] = low;
    if nes2 {
        header[9] = (header[9] & 0xF0) | (high & 0x0F);
    }
}

/// The built-in patch made for `image` (iNES header or not), if any.
#[must_use]
pub fn patch_for(image: &[u8]) -> Option<&'static BuiltinPatch> {
    for_data_crc(data_crc(image)).or_else(|| pack_matching(rom_data(image)).map(|(patch, _)| patch))
}

/// The built-in patch called `name`, if any.
#[must_use]
pub fn by_name(name: &str) -> Option<&'static BuiltinPatch> {
    PATCHES.iter().find(|patch| patch.name == name)
}

/// CRC-32 of the data of `image`: its PRG-ROM and CHR-ROM, without the iNES header, a trainer
/// or bytes past the sizes the header gives (an overdump). It is the CRC the No-Intro set gives.
#[must_use]
pub fn data_crc(image: &[u8]) -> u32 {
    crc32(rom_data(image))
}

/// The data of `image`, as [`data_crc`] reads it.
#[must_use]
pub fn rom_data(image: &[u8]) -> &[u8] {
    let (start, end) = data_bounds(image);
    &image[start..end]
}

/// Where the data of `image` starts and ends: after the header and a trainer, up to the sizes
/// the header gives (iNES, or NES 2.0 with its high bits); a whole headerless image.
fn data_bounds(image: &[u8]) -> (usize, usize) {
    if !image.starts_with(b"NES\x1A") || image.len() <= HEADER_SIZE {
        return (0, image.len());
    }
    let trainer = if image[6] & 0x04 == 0 { 0 } else { 512 };
    let start = (HEADER_SIZE + trainer).min(image.len());
    let nes2 = image[7] & 0x0C == 0x08;
    let units = |low: u8, high: u8| {
        if nes2 && high == 0x0F {
            // The exponent form, never used by a game a patch is made for: the whole rest.
            None
        } else {
            Some(usize::from(low) | (usize::from(if nes2 { high } else { 0 }) << 8))
        }
    };
    let size = units(image[4], image[9] & 0x0F)
        .zip(units(image[5], image[9] >> 4))
        .map(|(prg, chr)| prg * 16 * 1024 + chr * 8 * 1024)
        .filter(|size| *size > 0);
    let end = size.map_or(image.len(), |size| (start + size).min(image.len()));
    (start, end)
}

/// The size of the data `patch` was made for, from its BPS header.
#[must_use]
pub fn source_size(patch: &BuiltinPatch) -> Option<usize> {
    let mut reader = Reader {
        bytes: patch.bps,
        position: 4,
    };
    reader.number().ok()
}

/// The built-in patch made for ROM data (iNES header excluded) of CRC-32 `crc`, if any.
#[must_use]
pub fn for_data_crc(crc: u32) -> Option<&'static BuiltinPatch> {
    PATCHES.iter().find(|patch| patch.source_crc == crc)
}

/// The built-in patch made for the game named `name` (a ROM file or a profile title), whichever
/// version of it the ROM is: the patch whose game's words the name starts with ("Zelda II - The
/// Adventure of Link (Europe)" is Zelda II's, "Mega Man 3" is no one's).
#[must_use]
pub fn for_game_name(name: &str) -> Option<&'static BuiltinPatch> {
    let wanted = words(name);
    // The longest game that names it, and not one whose number goes on ("Super Mario Bros. 3"
    // is not "Super Mario Bros.", "Castlevania II" not "Castlevania").
    PATCHES
        .iter()
        .filter(|patch| {
            let game = words(patch.game);
            !game.is_empty()
                && wanted.starts_with(&game)
                && !wanted.get(game.len()).is_some_and(|next| is_number(next))
        })
        .max_by_key(|patch| patch.game.len())
}

/// The words of `text`: its runs of letters and digits, lower case.
fn words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whether `word` is a number, in figures or in Roman numerals (the sequel of a game).
fn is_number(word: &str) -> bool {
    word.chars().all(|character| character.is_ascii_digit())
        || (word.len() <= 4
            && word
                .chars()
                .all(|character| matches!(character, 'i' | 'v' | 'x')))
}

/// Whether the game whose data (iNES header excluded) has CRC-32 `crc` can be widened: a
/// built-in patch is made for it, or a margin decoder reads it as it is.
#[must_use]
pub fn widescreen_capable(crc: u32) -> bool {
    for_data_crc(crc).is_some() || crate::wide_decode::for_rom(crc).is_some()
}

/// Whether a built-in patch is made for `image` (iNES header or not).
#[must_use]
pub fn matches(image: &[u8]) -> bool {
    patch_for(image).is_some()
}

/// Why a BPS patch did not apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpsError {
    /// Not a BPS patch, or one cut short.
    Malformed,
    /// The source is not the one the patch was made for.
    WrongSource,
    /// The result does not have the checksum the patch promises.
    WrongTarget,
}

/// Applies the BPS patch `patch` to `source` and returns the target.
///
/// # Errors
/// [`BpsError`] when the patch is malformed, made for another source, or produces a target
/// whose checksum differs from the one it carries.
pub fn apply_bps(patch: &[u8], source: &[u8]) -> Result<Vec<u8>, BpsError> {
    apply_bps_checked(patch, source, true)
}

/// [`apply_bps`], the checksums of the source and of the target asked for only when `checked`
/// (a forced patch: only the size of the source must be the one the patch was made for).
fn apply_bps_checked(patch: &[u8], source: &[u8], checked: bool) -> Result<Vec<u8>, BpsError> {
    if patch.len() < 4 + 12 || !patch.starts_with(b"BPS1") {
        return Err(BpsError::Malformed);
    }

    let footer_at = patch.len() - 12;
    let footer = |index: usize| {
        let bytes = patch.get(footer_at + (4 * index)..footer_at + (4 * index) + 4);
        bytes
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map(u32::from_le_bytes)
            .ok_or(BpsError::Malformed)
    };
    if crc32(&patch[..footer_at + 8]) != footer(2)? {
        return Err(BpsError::Malformed);
    }
    if checked && crc32(source) != footer(0)? {
        return Err(BpsError::WrongSource);
    }

    let mut reader = Reader {
        bytes: &patch[..footer_at],
        position: 4,
    };
    let source_size = reader.number()?;
    let target_size = reader.number()?;
    let metadata_size = reader.number()?;
    if source_size != source.len() {
        return Err(BpsError::WrongSource);
    }
    reader.skip(metadata_size)?;

    let mut target = Vec::with_capacity(target_size);
    let mut source_offset: usize = 0;
    let mut target_offset: usize = 0;
    while reader.position < reader.bytes.len() {
        let action = reader.number()?;
        let length = (action >> 2) + 1;
        match action & 3 {
            0 => {
                let start = target.len();
                let bytes = source
                    .get(start..start + length)
                    .ok_or(BpsError::Malformed)?;
                target.extend_from_slice(bytes);
            }
            1 => {
                let bytes = reader.take(length)?;
                target.extend_from_slice(bytes);
            }
            2 => {
                source_offset = offset(source_offset, reader.number()?)?;
                let bytes = source
                    .get(source_offset..source_offset + length)
                    .ok_or(BpsError::Malformed)?;
                target.extend_from_slice(bytes);
                source_offset += length;
            }
            _ => {
                target_offset = offset(target_offset, reader.number()?)?;
                for _ in 0..length {
                    // Byte by byte: the copy may overlap the bytes it is writing.
                    let byte = *target.get(target_offset).ok_or(BpsError::Malformed)?;
                    target.push(byte);
                    target_offset += 1;
                }
            }
        }
        if target.len() > target_size {
            return Err(BpsError::Malformed);
        }
    }

    if target.len() != target_size || (checked && crc32(&target) != footer(1)?) {
        return Err(BpsError::WrongTarget);
    }
    Ok(target)
}

/// A relative BPS offset: bit 0 is the sign, the rest the distance.
fn offset(base: usize, encoded: usize) -> Result<usize, BpsError> {
    let distance = encoded >> 1;
    if encoded & 1 == 0 {
        base.checked_add(distance)
    } else {
        base.checked_sub(distance)
    }
    .ok_or(BpsError::Malformed)
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, BpsError> {
        let byte = *self.bytes.get(self.position).ok_or(BpsError::Malformed)?;
        self.position += 1;
        Ok(byte)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], BpsError> {
        let bytes = self
            .bytes
            .get(self.position..self.position + length)
            .ok_or(BpsError::Malformed)?;
        self.position += length;
        Ok(bytes)
    }

    fn skip(&mut self, length: usize) -> Result<(), BpsError> {
        self.take(length).map(|_| ())
    }

    /// A BPS number: seven bits per byte, the last byte flagged, each continuation adding one.
    fn number(&mut self) -> Result<usize, BpsError> {
        let mut value: usize = 0;
        let mut shift: usize = 1;
        loop {
            let byte = self.byte()?;
            value = usize::from(byte & 0x7F)
                .checked_mul(shift)
                .and_then(|part| value.checked_add(part))
                .ok_or(BpsError::Malformed)?;
            if byte & 0x80 != 0 {
                return Ok(value);
            }
            shift = shift.checked_mul(128).ok_or(BpsError::Malformed)?;
            value = value.checked_add(shift).ok_or(BpsError::Malformed)?;
        }
    }
}

/// CRC-32 (IEEE, the one of zlib and of the ROM databases).
#[must_use]
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 0 {
                crc >> 1
            } else {
                (crc >> 1) ^ 0xEDB8_8320
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BPS number as the format writes it.
    fn number(mut value: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            #[allow(clippy::cast_possible_truncation)] // Masked to seven bits.
            let low = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                out.push(0x80 | low);
                return out;
            }
            out.push(low);
            value -= 1;
        }
    }

    fn patch(source: &[u8], target: &[u8], actions: &[u8]) -> Vec<u8> {
        let mut patch = b"BPS1".to_vec();
        patch.extend(number(source.len()));
        patch.extend(number(target.len()));
        patch.extend(number(0));
        patch.extend_from_slice(actions);
        patch.extend(crc32(source).to_le_bytes());
        patch.extend(crc32(target).to_le_bytes());
        let crc = crc32(&patch);
        patch.extend(crc.to_le_bytes());
        patch
    }

    #[test]
    fn a_game_is_found_by_the_start_of_its_name() {
        let zelda = for_game_name("Zelda II - The Adventure of Link (Europe)").unwrap();
        assert_eq!(ZELDA2_USA, zelda.name);
        assert_eq!(
            Some(MEGAMAN2_USA),
            for_game_name("Mega Man 2 (U)").map(|patch| patch.name)
        );
        assert_eq!(
            Some(MEGAMAN3_USA),
            for_game_name("Mega Man 3 (USA)").map(|patch| patch.name)
        );
        assert_eq!(
            Some(MEGAMAN1_USA),
            for_game_name("Mega Man (Europe)").map(|patch| patch.name)
        );
        assert_eq!(
            for_game_name("Mega Man 4 (Europe)").map(|patch| patch.game),
            Some("Mega Man 4")
        );
        assert!(for_game_name("Mega Man X (USA)").is_none());
        assert_eq!(
            for_game_name("Super Mario Bros. 3 (Europe)").map(|patch| patch.name),
            Some(SMB3_USA)
        );
        assert_eq!(
            for_game_name("Super Mario Bros. (Europe)").map(|patch| patch.name),
            Some(SMB1_WORLD)
        );
        assert!(for_game_name("Super Mario Bros. 2 (USA)").is_none());
        assert_eq!(
            for_game_name("Castlevania (Europe)").map(|patch| patch.game),
            Some("Castlevania")
        );
        assert!(for_game_name("Castlevania II - Simon's Quest (USA)").is_none());
        assert!(for_game_name("Castlevania III - Dracula's Curse (USA)").is_none());
        assert_eq!(
            Some(ADVENTURE_ISLAND_USA),
            for_game_name("Adventure Island (Europe)").map(|patch| patch.name)
        );
        assert!(for_game_name("Adventure Island II (USA)").is_none());
        assert!(for_game_name("Adventure Island 3 (USA)").is_none());
        assert!(for_game_name("Adventure Islands").is_none());
        assert_eq!(
            Some(FINAL_FANTASY_USA),
            for_game_name("Final Fantasy (Japan)").map(|patch| patch.name)
        );
        assert!(for_game_name("Final Fantasy II (Japan)").is_none());
        assert!(for_game_name("Final Fantasy III (Japan)").is_none());
        assert!(for_game_name("Legend of Zelda, The (USA)").is_none());
        assert!(for_game_name("").is_none());
    }

    #[test]
    fn crc32_matches_the_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn numbers_round_trip() {
        for value in [0, 1, 127, 128, 129, 16_511, 16_512, 1 << 20] {
            let bytes = number(value);
            let mut reader = Reader {
                bytes: &bytes,
                position: 0,
            };
            assert_eq!(reader.number(), Ok(value));
        }
    }

    #[test]
    fn source_read_target_read_and_copies() {
        let source = b"ABCDEFGH";
        let target = b"ABxyEFGHGH";
        let mut actions = Vec::new();
        actions.extend(number(1 << 2)); // SourceRead 2: "AB"
        actions.extend(number((1 << 2) | 1)); // TargetRead 2: "xy"
        actions.extend(b"xy");
        actions.extend(number((3 << 2) | 2)); // SourceCopy 4 from +4: "EFGH"
        actions.extend(number(4 << 1));
        actions.extend(number((1 << 2) | 3)); // TargetCopy 2 from +6: "GH"
        actions.extend(number(6 << 1));
        let patch = patch(source, target, &actions);
        assert_eq!(apply_bps(&patch, source), Ok(target.to_vec()));
        assert_eq!(apply_bps(&patch, b"ABCDEFGX"), Err(BpsError::WrongSource));
    }

    #[test]
    fn a_patch_adding_prg_banks_grows_the_header() {
        let mut header = b"NES".to_vec();
        header.extend_from_slice(&[2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        resize_prg(&mut header, 40 * 1024, 72 * 1024);
        assert_eq!(header[4], 4, "two banks added");
        assert_eq!(header[5], 1, "the CHR-ROM stays");
        resize_prg(&mut header, 40 * 1024, 41 * 1024);
        assert_eq!(header[4], 4, "not whole banks: left alone");
    }

    #[test]
    fn unknown_roms_are_left_alone() {
        let mut image = b"NES\x1A".to_vec();
        image.resize(16 + 1024, 0);
        assert_eq!(apply(&image), None);
    }

    /// Needs `ZAXNES_ROMS` with `Zelda II - The Adventure of Link (USA).nes`.
    #[test]
    #[ignore = "needs a commercial ROM (ZAXNES_ROMS)"]
    fn the_zelda_ii_patch_applies_to_the_usa_rom() {
        let Some(folder) = std::env::var_os("ZAXNES_ROMS") else {
            return;
        };
        let path = std::path::Path::new(&folder).join("Zelda II - The Adventure of Link (USA).nes");
        let Ok(image) = std::fs::read(path) else {
            return;
        };
        // `apply` checks the source and target checksums of the patch: a `Some` is the build.
        let patched = apply(&image).map(|rom| rom.image).unwrap_or_default();
        assert_eq!(patched.len(), image.len());
        assert_eq!(patched[..16], image[..16], "the player's header is kept");

        // An overdump and a trainer leave the data, and so the patch, as they are.
        let mut overdump = image.clone();
        overdump.extend_from_slice(&[0xFF; 1024]);
        assert_eq!(data_crc(&image), data_crc(&overdump));
        assert_eq!(
            Some(ZELDA2_USA),
            patch_for(&overdump).map(|patch| patch.name)
        );
        let mut trainer = image[..16].to_vec();
        trainer[6] |= 0x04;
        trainer.extend_from_slice(&[0x55; 512]);
        trainer.extend_from_slice(&image[16..]);
        assert_eq!(data_crc(&image), data_crc(&trainer));
        assert!(apply(&trainer).is_some());

        // A hack of the ROM (one byte changed here) takes the patch only when it is forced.
        let mut hack = image.clone();
        let last = hack.len() - 1;
        hack[last] ^= 0x01;
        assert!(apply(&hack).is_none());
        let zelda = by_name(ZELDA2_USA);
        let forced = apply_with(&hack, zelda)
            .map(|rom| rom.image)
            .unwrap_or_default();
        assert_eq!(forced.len(), hack.len());
        // The patch applies to data of the size it was made for only.
        assert_eq!(source_size(zelda.unwrap()), Some(image.len() - 16));
        assert!(apply_with(&hack[..hack.len() - 8192], zelda).is_none());
    }

    #[test]
    fn the_built_in_patches_are_well_formed() {
        for patch in PATCHES {
            // A hack pack: it reads, and its widescreen is one of its components.
            if crate::hack_pack::is_pack(patch.bps) {
                let pack = crate::hack_pack::HackPack::read(patch.bps);
                assert!(
                    pack.is_some_and(|pack| pack.component(crate::hack_pack::WIDESCREEN).is_some()),
                    "{}",
                    patch.name
                );
                continue;
            }
            assert!(patch.bps.starts_with(b"BPS1"), "{}", patch.name);
            let footer = patch.bps.len() - 12;
            let source_crc = u32::from_le_bytes([
                patch.bps[footer],
                patch.bps[footer + 1],
                patch.bps[footer + 2],
                patch.bps[footer + 3],
            ]);
            assert_eq!(source_crc, patch.source_crc, "{}", patch.name);
        }
    }
}
