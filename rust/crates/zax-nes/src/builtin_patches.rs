//! Patches built into the emulator and applied in memory to the ROMs they were made for.
//!
//! A known ROM is recognised by the CRC-32 of its data without the iNES header (the header of
//! one dump differs from another's), or by the signatures of a hack pack, then patched; the
//! player's header is kept. Every patch is now a hack pack (`crate::hack_pack`): the hack's hooks,
//! each with the game's bytes it replaces, and its code for the extension's memory. The patches
//! carry the hacks' own bytes only. A ROM nothing matches is loaded as it is. A BPS patch is
//! still applied if one is listed (`apply_bps_checked`); the BPS of each converted hack stays in
//! `patches/` to compare.
//!
//! | ROM | Patch | Source |
//! |---|---|---|
//! | Zelda II - The Adventure of Link (USA) | widescreen (see `docs/WIDESCREEN-EXTENSION.md`) | a hack pack (`crate::hack_pack`): the code in the extension's memory, the hooks checked and applied by the core, the ROM keeping its size; written in a fork of the FiendsOfTheElements Zelda II disassembly (`z2wide`), moved out of the game by `z2pack` |
//! | Zelda II - The Adventure of Link (Europe), its Rev 1 and its Rev 2 | widescreen, the USA hack built for each European build's code | hack packs of the PAL builds (`crate::hack_pack`), from the same source with the game's moved addresses (`z2pack`, `tools/build.py`); one for Europe and its Rev 1, one for the Rev 2 |
//! | Mega Man 2 (USA) | widescreen objects (the margin decoder draws the background) | a hack pack (`crate::hack_pack`): the code in the extension's memory, the hooks checked and applied by the core, the ROM keeping its size; "Invincible" a component of its own, with or without the widescreen (`mm2wide`, `tools/build.py --pack`) |
//! | Mega Man 2 (Europe), Rockman 2 (Japan) | the same | the USA hack's pack built from the European ROM (PAL) and for the Japanese code (`mm2wide`, `tools/build.py --pack`), in the execution zone |
//! | Super Mario Bros. (World), and (Europe) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in three groups (the sprites, the enemies in the margins, the level select), the ROM keeping its size; the European one PAL with an NTSC twin; "Invincible" a component of its own, with or without the advanced mode (`smb1wide`, `tools/build.py --pack`) |
//! | Ninja Gaiden III - The Ancient Ship of Doom (USA), and Ninja Ryuuken Den III (Japan) | widescreen objects, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the objects in the margins, the enemy list's), the ROM keeping its size (`ng3wide`, `tools/build.py --pack`) |
//! | Super Mario Bros. 3 (USA) (Rev 1), (Japan), its Rev 1, and (Europe) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in three groups (the objects' sprites, the enemies in the margins, Mario in the margins), the ROM keeping its size; a pack per build whose code moved, the European one PAL with an NTSC twin; the level select and "Invincible" components of their own, without code (`smb3wide`, `tools/build.py --pack`) |
//! | Castlevania (USA), and its Rev 1 | widescreen objects and enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the extension's execution zone at `$2000` (`boot_zone`), the hooks checked and applied by the core, the ROM keeping its size; "Invincible" a component of its own, with or without the advanced mode; one source for both builds (`cvwide`, `tools/build.py --pack`) |
//! | Castlevania (Europe) | the same, PAL | the USA hack's pack built for the European code (`cvwide`, `tools/build.py --pack`), in the execution zone |
//! | Akumajou Dracula (Japan) | the same | the USA hack's pack built for the Japanese code (`cvwide`, `tools/build.py --pack`), in the execution zone |
//! | Adventure Island (USA), Classic (Europe), Takahashi Meijin no Bouken-jima (Japan) and Kiddy Sun in Fantasia (Asia) | widescreen sprites and objects, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in groups (the sprites, the ghosts, the previews, the two aids), the ROM keeping its size; the European one the same for PAL, the Japanese one built for its code, the Asian one the Japanese one but for its NMI's place, without the level selector (`aiwide`, `tools/build.py --pack`) |
//! | Excitebike (Japan, USA), and (Europe) | widescreen riders, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the settings, the riders in the margins), the ROM keeping its size; the European one the same for PAL (`excitebikewide`, `tools/build.py --pack`) |
//! | Final Fantasy (USA), and (Japan) | widescreen people and vehicles, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the sprites, the people and vehicles), the ROM keeping its size; the Japanese one for its code, which its Rev 1 takes (`ff1wide`, `tools/build.py --pack`) |
//! | Metal Force (Korea) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the sprites, the enemies), the ROM keeping its size; "Invincible" a component of its own, with or without the advanced mode (`metalforcewide`, `tools/build.py --pack`) |
//! | Blaster Master (USA) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the extension's memory, the hooks checked and applied by the core, the ROM keeping its size (`blastermasterwide`, `tools/build.py --pack`) |
//! | Metroid (USA) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the extension's memory, the hooks checked and applied by the core, the ROM keeping its size (`metroidwide`, `tools/build.py --pack`) |
//! | Mega Man 4 (USA), its Rev 1, and (Europe) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the sprites, the enemies in the margins), the ROM keeping its size; one pack for both USA builds and Rockman 4 (Japan), the same built from the European ROM, PAL; "Invincible" a component of its own, with or without the advanced mode (`mm4wide`, `tools/build.py --pack`) |
//! | Mega Man 5 (USA), and (Europe) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the sprites, the enemies in the margins), the ROM keeping its size; Rockman 5 (Japan) by its signatures, the European build's own pack, PAL; "Invincible" a component of its own, with or without the advanced mode (`mm5wide`, `tools/build.py --pack`) |
//! | Mega Man 6 (USA), and Rockman 6 (Japan) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the sprites, the enemies in the margins), the ROM keeping its size; the Japanese build's own pack; "Invincible" a component of its own, with or without the advanced mode (`mm6wide`, `tools/build.py --pack`) |
//! | Mega Man 3 (USA), Rockman 3 (Japan), (Europe) and its Rev 1 | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the sprites, the enemies in the margins), the ROM keeping its size; a pack per build, the European ones PAL with NTSC twins; "Invincible" a component of its own, with or without the advanced mode (`mm3wide`, `tools/build.py --pack`) |
//! | Mega Man (USA), and (Europe) | widescreen enemies, advanced mode (the margin decoder alone draws the background) | a hack pack (`crate::hack_pack`): the code in the zone fantôme (widescreen MMC5), hooks checked and applied by the core in two groups (the sprites, the enemies in the margins), the ROM keeping its size; Rockman (Japan) and the conversions by its signatures; the European build's pack, PAL, and the same as NTSC; "Invincible" a component of its own, with or without the advanced mode (`mm1wide`, `tools/build.py --pack`) |

/// One built-in patch.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // one flag per boot setting of the extension
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
    /// The widescreen extension unlocked at power-on, reset and restore with its execution zone
    /// on (`$500B` = `$80`): a hack pack whose code runs at `$2000`-`$3FFF`, shown to the CPU's
    /// instruction fetches only (`WideExtension::zone_offset`).
    pub boot_zone: bool,
    /// With `boot_zone`, the zone holds the extension's registers and RAM at `$3000`-`$3FFF` for
    /// the pack's own instructions (`$500B` bit 6), its code below `$3000`: a pack whose game
    /// uses `$5000`-`$5FFF` (MMC5, Namco 163), or any pack assembled for it.
    pub zone_data: bool,
    /// The game's wait for the next frame, first to last address of its loop: the overclock
    /// (`$5006`) ends as soon as the CPU is there, the cycles left being of no use to the game
    /// (Rust-only, owner request: "yield"; the same game, the CPU time only when a frame needs it).
    pub idle_loop: Option<(u16, u16)>,
}

/// Name of the Zelda II (USA) widescreen patch.
pub const ZELDA2_USA: &str = "Zelda II (USA) widescreen";
/// Name of the Zelda II (Europe) widescreen patch, and of its Rev 1's (the same pack).
pub const ZELDA2_EUROPE: &str = "Zelda II (Europe) widescreen";
pub const ZELDA2_EUROPE_REV1: &str = "Zelda II (Europe) (Rev 1) widescreen";
/// Name of the Zelda II (Europe) (Rev 2) widescreen patch.
pub const ZELDA2_EUROPE_REV2: &str = "Zelda II (Europe) (Rev 2) widescreen";
/// Name of the Zelda II (USA) MMC5 patch's widescreen patch (advanced mode).
pub const ZELDA2_USA_MMC5: &str = "Zelda II (USA) (MMC5) widescreen";

/// The options of the European builds of the Zelda II hack (those of the USA one).
const ZELDA2_OPTIONS: &[HackOption] = &[
    HackOption {
        key: "invincible",
        label: "Invincible",
        label_fr: "Invincible",
        bit: 1,
        default: false,
    },
    HackOption {
        key: "no_slowdown",
        label: "No slowdown",
        label_fr: "Sans ralentissements",
        bit: 2,
        default: true,
    },
];

/// Name of the Mega Man 2 (USA) widescreen patch.
pub const MEGAMAN2_USA: &str = "Mega Man 2 (USA) widescreen";
/// Name of the Mega Man 2 (Europe) widescreen patch (advanced mode).
pub const MEGAMAN2_EUROPE: &str = "Mega Man 2 (Europe) widescreen";
/// Name of the Rockman 2 (Japan) widescreen patch (advanced mode).
pub const ROCKMAN2_JAPAN: &str = "Rockman 2 (Japan) widescreen";

/// Name of the Super Mario Bros. (World) widescreen patch (advanced mode).
pub const SMB1_WORLD: &str = "Super Mario Bros. (World) widescreen";
/// Name of the Super Mario Bros. (Europe) widescreen patch (advanced mode).
pub const SMB1_EUROPE: &str = "Super Mario Bros. (Europe) widescreen";
/// Name of the same patch for an NTSC ROM of the European code (advanced mode).
pub const SMB1_EUROPE_NTSC: &str = "Super Mario Bros. (Europe) (NTSC) widescreen";

/// Name of the Ninja Gaiden III (USA) widescreen patch (advanced mode).
pub const NG3_USA: &str = "Ninja Gaiden III (USA) widescreen";
/// Name of the Ninja Ryuuken Den III (Japan) widescreen patch (advanced mode).
pub const NG3_JAPAN: &str = "Ninja Ryuuken Den III (Japan) widescreen";
/// Name of the Super Mario Bros. 3 (USA) (Rev 1) widescreen patch (advanced mode).
pub const SMB3_USA: &str = "Super Mario Bros. 3 (USA) (Rev 1) widescreen";
/// Name of the Super Mario Bros. 3 (Japan) (Rev 1) widescreen patch (advanced mode).
pub const SMB3_JAPAN_REV1: &str = "Super Mario Bros. 3 (Japan) (Rev 1) widescreen";
/// Name of the Super Mario Bros. 3 (Japan) widescreen patch (advanced mode).
pub const SMB3_JAPAN: &str = "Super Mario Bros. 3 (Japan) widescreen";
/// Name of the Super Mario Bros. 3 (Europe) widescreen patch (advanced mode).
pub const SMB3_EUROPE: &str = "Super Mario Bros. 3 (Europe) widescreen";
/// Name of the same patch for an NTSC ROM of the European code (advanced mode).
pub const SMB3_EUROPE_NTSC: &str = "Super Mario Bros. 3 (Europe) (NTSC) widescreen";
/// Name of the Castlevania (USA) widescreen patch (advanced mode).
pub const CASTLEVANIA_USA: &str = "Castlevania (USA) widescreen";
/// Name of the Castlevania (USA) (Rev 1) widescreen patch (advanced mode).
pub const CASTLEVANIA_USA_REV1: &str = "Castlevania (USA) (Rev 1) widescreen";
/// Name of the Castlevania (Europe) widescreen patch (advanced mode).
pub const CASTLEVANIA_EUROPE: &str = "Castlevania (Europe) widescreen";
/// Name of the same patch for an NTSC ROM of the European code (advanced mode).
pub const CASTLEVANIA_EUROPE_NTSC: &str = "Castlevania (Europe) (NTSC) widescreen";
/// Name of the Castlevania (PlayChoice-10) widescreen patch (advanced mode).
pub const CASTLEVANIA_PC10: &str = "Castlevania (PlayChoice-10) widescreen";
/// Name of the Akumajou Dracula (Japan) widescreen patch (advanced mode).
pub const AKUMAJOU_DRACULA_JAPAN: &str = "Akumajou Dracula (Japan) widescreen";
/// Name of the Adventure Island (USA) widescreen patch (advanced mode).
pub const ADVENTURE_ISLAND_USA: &str = "Adventure Island (USA) widescreen";
/// Name of the Adventure Island Classic (Europe) widescreen patch (advanced mode).
pub const ADVENTURE_ISLAND_EUROPE: &str = "Adventure Island Classic (Europe) widescreen";
/// Name of the Takahashi Meijin no Bouken-jima (Japan) widescreen patch (advanced mode).
pub const ADVENTURE_ISLAND_JAPAN: &str = "Takahashi Meijin no Bouken-jima (Japan) widescreen";
/// Name of the Kiddy Sun in Fantasia (Asia) widescreen patch (advanced mode).
pub const ADVENTURE_ISLAND_KIDDY_SUN: &str = "Kiddy Sun in Fantasia (Asia) widescreen";
/// Name of the Excitebike (Japan, USA) widescreen patch (advanced mode).
pub const EXCITEBIKE: &str = "Excitebike (Japan, USA) widescreen";
/// Name of the Excitebike (Europe) widescreen patch (advanced mode).
pub const EXCITEBIKE_EUROPE: &str = "Excitebike (Europe) widescreen";
/// Name of the Final Fantasy (USA) widescreen patch (advanced mode).
pub const FINAL_FANTASY_USA: &str = "Final Fantasy (USA) widescreen";
/// Name of the Final Fantasy (Japan) widescreen patch (advanced mode).
pub const FINAL_FANTASY_JAPAN: &str = "Final Fantasy (Japan) widescreen";
/// Name of the Metal Force (Korea) widescreen patch (advanced mode).
pub const METALFORCE_KOREA: &str = "Metal Force (Korea) widescreen";
/// Name of the Blaster Master (USA) widescreen patch (advanced mode).
pub const BLASTERMASTER_USA: &str = "Blaster Master (USA) widescreen";
/// Name of the Metroid (USA) widescreen patch (advanced mode).
pub const METROID_USA: &str = "Metroid (USA) widescreen";
/// Name of the Mega Man (USA) widescreen patch (advanced mode).
pub const MEGAMAN1_USA: &str = "Mega Man (USA) widescreen";
/// Name of the Mega Man (Europe) widescreen patch (advanced mode).
pub const MEGAMAN1_EUROPE: &str = "Mega Man (Europe) widescreen";
/// Name of the same patch for an NTSC ROM of the European code (advanced mode).
pub const MEGAMAN1_EUROPE_NTSC: &str = "Mega Man (Europe) (NTSC) widescreen";
/// Name of the Mega Man 3 (USA) widescreen patch (advanced mode).
pub const MEGAMAN3_USA: &str = "Mega Man 3 (USA) widescreen";
/// Name of the Rockman 3 (Japan) widescreen patch (advanced mode).
pub const ROCKMAN3_JAPAN: &str = "Rockman 3 (Japan) widescreen";
/// Name of the Mega Man 3 (Europe) widescreen patch (advanced mode).
pub const MEGAMAN3_EUROPE: &str = "Mega Man 3 (Europe) widescreen";
/// Name of the same patch for an NTSC ROM of the European code (advanced mode).
pub const MEGAMAN3_EUROPE_NTSC: &str = "Mega Man 3 (Europe) (NTSC) widescreen";
/// Name of the Mega Man 3 (Europe) (Rev 1) widescreen patch (advanced mode).
pub const MEGAMAN3_EUROPE_REV1: &str = "Mega Man 3 (Europe) (Rev 1) widescreen";
/// Name of the same patch for an NTSC ROM of the Rev 1's code (advanced mode).
pub const MEGAMAN3_EUROPE_REV1_NTSC: &str = "Mega Man 3 (Europe) (Rev 1) (NTSC) widescreen";
/// Name of the Mega Man 4 (USA) widescreen patch (advanced mode).
pub const MEGAMAN4_USA: &str = "Mega Man 4 (USA) widescreen";
/// Name of the Mega Man 4 (USA) (Rev 1) widescreen patch (advanced mode).
pub const MEGAMAN4_USA_REV1: &str = "Mega Man 4 (USA) (Rev 1) widescreen";
/// Name of the Mega Man 4 (Europe) widescreen patch (advanced mode).
pub const MEGAMAN4_EUROPE: &str = "Mega Man 4 (Europe) widescreen";
/// Name of the Mega Man 5 (USA) widescreen patch (advanced mode).
pub const MEGAMAN5_USA: &str = "Mega Man 5 (USA) widescreen";
/// Name of the Mega Man 5 (Europe) widescreen patch (advanced mode).
pub const MEGAMAN5_EUROPE: &str = "Mega Man 5 (Europe) widescreen";
/// Name of the Mega Man 6 (USA) widescreen patch (advanced mode).
pub const MEGAMAN6_USA: &str = "Mega Man 6 (USA) widescreen";
/// Name of the Rockman 6 (Japan) widescreen patch (advanced mode).
pub const ROCKMAN6_JAPAN: &str = "Rockman 6 (Japan) widescreen";

/// The built-in patches.
pub const PATCHES: [BuiltinPatch; 51] = [
    BuiltinPatch {
        name: ZELDA2_USA,
        game: "Zelda II",
        rom_name: "Zelda II - The Adventure of Link (USA)",
        source_crc: 0xBA32_2865,
        // A hack pack (see `crate::hack_pack`, built by `z2pack`): the code in the extension's
        // memory, the hooks checked and applied by the core; the ROM keeps its size.
        bps: include_bytes!("../patches/zelda2-usa-widescreen.zxpk"),
        options: &[
            // A testing aid, a component of its own (with or without the widescreen): no hit
            // hurts Link (pits and lava still kill).
            HackOption {
                key: "invincible",
                label: "Invincible",
                label_fr: "Invincible",
                bit: 1,
                default: false,
            },
            // Nearly a frame more of CPU time in the side view ($5006 = 255, ended once the game
            // waits for the next frame); off, the hack's 48 lines.
            HackOption {
                key: "no_slowdown",
                label: "No slowdown",
                label_fr: "Sans ralentissements",
                bit: 2,
                default: true,
            },
        ],
        after_other_way_state: Some(zelda2_after_other_way_state),
        // The margin decoder alone widens the game; the patch adds the enemies and the world map past the edges.
        advanced: true,
        // The pack's code runs in the execution zone ($2000-$3FFF, instruction fetches only),
        // open from power-on (the hack does not unlock the extension itself): no address the
        // game or a ROM hack of it uses, the PRG window and $6000 left alone.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop waits for a game mode that loads an area.
        idle_loop: Some((0xC010, 0xC01A)),
    },
    BuiltinPatch {
        name: ZELDA2_EUROPE,
        game: "Zelda II",
        rom_name: "Zelda II - The Adventure of Link (Europe)",
        source_crc: 0x47FD_88CF,
        // The USA hack built for the European code (`z2pack`, `tools/build.py`: the same source,
        // the game's moved addresses for this build; a PAL pack).
        bps: include_bytes!("../patches/zelda2-europe-widescreen.zxpk"),
        options: ZELDA2_OPTIONS,
        after_other_way_state: Some(zelda2_after_other_way_state),
        // The margin decoder alone widens the game; the patch adds the enemies and the world map
        // past the edges.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop waits for a game mode that loads an area.
        idle_loop: Some((0xC010, 0xC01A)),
    },
    BuiltinPatch {
        name: ZELDA2_EUROPE_REV1,
        game: "Zelda II",
        rom_name: "Zelda II - The Adventure of Link (Europe) (Rev 1)",
        source_crc: 0x97D5_2C06,
        // The USA hack built for the European code (`z2pack`, `tools/build.py`: the same source,
        // the game's moved addresses for this build; a PAL pack); the pack of the Europe build: this revision has the same code where the hack hooks it.
        bps: include_bytes!("../patches/zelda2-europe-widescreen.zxpk"),
        options: ZELDA2_OPTIONS,
        after_other_way_state: Some(zelda2_after_other_way_state),
        // The margin decoder alone widens the game; the patch adds the enemies and the world map
        // past the edges.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop waits for a game mode that loads an area.
        idle_loop: Some((0xC010, 0xC01A)),
    },
    BuiltinPatch {
        name: ZELDA2_EUROPE_REV2,
        game: "Zelda II",
        rom_name: "Zelda II - The Adventure of Link (Europe) (Rev 2)",
        source_crc: 0x47B6_A39F,
        // The USA hack built for the European code (`z2pack`, `tools/build.py`: the same source,
        // the game's moved addresses for this build; a PAL pack).
        bps: include_bytes!("../patches/zelda2-europe-rev2-widescreen.zxpk"),
        options: ZELDA2_OPTIONS,
        after_other_way_state: Some(zelda2_after_other_way_state),
        // The margin decoder alone widens the game; the patch adds the enemies and the world map
        // past the edges.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop waits for a game mode that loads an area.
        idle_loop: Some((0xC010, 0xC01A)),
    },
    BuiltinPatch {
        name: ZELDA2_USA_MMC5,
        game: "Zelda II",
        rom_name: "Zelda II - The Adventure of Link (USA) (MMC5 patch)",
        source_crc: 0x7D3A_2AEF,
        // The USA hack for the ROM hack that moves the game to MMC5 (512 KiB, its fixed bank left
        // at bank 7): the USA's code and addresses, its places counted from the start (`z2pack`,
        // `tools/build.py`); its translations take it by their signatures.
        bps: include_bytes!("../patches/zelda2-usa-mmc5-widescreen.zxpk"),
        options: ZELDA2_OPTIONS,
        after_other_way_state: Some(zelda2_after_other_way_state),
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the MMC5 keeps $5000-$5FFF, the extension is at $3000-$3FFF.
        zone_data: true,
        idle_loop: Some((0xC010, 0xC01A)),
    },
    BuiltinPatch {
        name: MEGAMAN2_USA,
        game: "Mega Man 2",
        rom_name: "Mega Man 2 (USA)",
        source_crc: 0x0FCF_C04D,
        // A hack pack (see `crate::hack_pack`): the code in the extension's execution zone at
        // $2000, the hooks checked and applied by the core; the ROM keeps its size. "Invincible"
        // a component of its own.
        bps: include_bytes!("../patches/megaman2-usa-widescreen.zxpk"),
        options: MEGAMAN2_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        // No room in the game for code that would unlock the extension: the pack's code runs in
        // the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Its code reaches the extension at $3000-$3FFF: an MMC5 conversion keeps $5000-$5FFF.
        zone_data: true,
        // The frame's wait: LDA $1D / BEQ (the NMI sets $1D).
        idle_loop: Some((0xC08E, 0xC091)),
    },
    BuiltinPatch {
        name: MEGAMAN2_EUROPE,
        game: "Mega Man 2",
        rom_name: "Mega Man 2 (Europe)",
        source_crc: 0xA663_8CBA,
        // The USA pack built from the European ROM, PAL (`mm2wide`, `tools/build.py --pack`): its
        // code is the same where the hack hooks and calls, its stage layouts too.
        bps: include_bytes!("../patches/megaman2-europe-widescreen.zxpk"),
        options: MEGAMAN2_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Its code reaches the extension at $3000-$3FFF: an MMC5 conversion keeps $5000-$5FFF.
        zone_data: true,
        // The frame's wait, at the same place.
        idle_loop: Some((0xC08E, 0xC091)),
    },
    BuiltinPatch {
        name: ROCKMAN2_JAPAN,
        game: "Mega Man 2",
        rom_name: "Rockman 2 - Dr. Wily no Nazo (Japan)",
        source_crc: 0x6150_517C,
        // The USA hack built for the Japanese code (`mm2wide`, `tools/build.py --pack`, -D JAPAN:
        // its fixed bank's places moved; banks 11 and 14, the RAM and the stage layouts as the
        // USA's).
        bps: include_bytes!("../patches/rockman2-japan-widescreen.zxpk"),
        options: MEGAMAN2_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Its code reaches the extension at $3000-$3FFF: an MMC5 conversion keeps $5000-$5FFF.
        zone_data: true,
        // The frame's wait, at the same place.
        idle_loop: Some((0xC08E, 0xC091)),
    },
    BuiltinPatch {
        name: SMB1_WORLD,
        game: "Super Mario Bros.",
        rom_name: "Super Mario Bros. (World)",
        source_crc: 0xD445_F698,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in three groups (the sprites, the enemies in the margins, the
        // level select); "Invincible" a component of its own (`smb1wide`, `tools/build.py
        // --pack`). The PlayChoice-10 ROM's data is the same.
        bps: include_bytes!("../patches/smb1-world-widescreen.zxpk"),
        options: SMB1_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop is a JMP to itself.
        idle_loop: Some((0x8057, 0x8059)),
    },
    BuiltinPatch {
        name: SMB1_EUROPE,
        game: "Super Mario Bros.",
        rom_name: "Super Mario Bros. (Europe)",
        source_crc: 0x9A2D_B086,
        // The World hack built for the European code (`smb1wide`, -D EUROPE: most of its code from
        // $C905 on 6 to 8 bytes later, some 56 earlier; the same RAM), PAL.
        bps: include_bytes!("../patches/smb1-europe-widescreen.zxpk"),
        options: SMB1_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop is a JMP to itself.
        idle_loop: Some((0x8057, 0x8059)),
    },
    BuiltinPatch {
        name: SMB1_EUROPE_NTSC,
        game: "Super Mario Bros.",
        rom_name: "Super Mario Bros. (Europe)",
        source_crc: 0x9A2D_B086,
        // The European build's CRC, its PAL entry above taking it first: this one is for the
        // ROMs of the European code whose header says NTSC, taken by their signatures
        // (`smb1wide`, `tools/build.py --pack <out> --ntsc`).
        bps: include_bytes!("../patches/smb1-europe-ntsc-widescreen.zxpk"),
        options: SMB1_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop is a JMP to itself.
        idle_loop: Some((0x8057, 0x8059)),
    },
    BuiltinPatch {
        name: NG3_USA,
        game: "Ninja Gaiden III",
        rom_name: "Ninja Gaiden III - The Ancient Ship of Doom (USA)",
        source_crc: 0x902E_3168,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in two groups (the objects in the margins, the enemy list's);
        // the ROM keeps its size (`ng3wide`, `tools/build.py --pack`).
        bps: include_bytes!("../patches/ng3-usa-widescreen.zxpk"),
        options: NG3_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The frame's wait in bank 4: BIT $15 / BPL (the NMI sets $15).
        idle_loop: Some((0x9A87, 0x9A8A)),
    },
    BuiltinPatch {
        name: NG3_JAPAN,
        game: "Ninja Gaiden III",
        rom_name: "Ninja Ryuuken Den III - Yomi no Hakobune (Japan)",
        source_crc: 0x2F2E_30F7,
        // The USA hack built for the Japanese code (`ng3wide`, -D JAPAN: the same code and RAM,
        // moved by a few bytes).
        bps: include_bytes!("../patches/ng3-japan-widescreen.zxpk"),
        options: NG3_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The frame's wait in bank 4: BIT $15 / BPL (the NMI sets $15).
        idle_loop: Some((0x9A77, 0x9A7A)),
    },
    BuiltinPatch {
        name: SMB3_USA,
        game: "Super Mario Bros. 3",
        rom_name: "Super Mario Bros. 3 (USA) (Rev 1)",
        source_crc: 0x2E63_01ED,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in three groups (the objects' sprites, the enemies in the
        // margins, Mario in the margins); the level select and "Invincible" components of their
        // own, without code (`smb3wide`, `tools/build.py --pack`). The first revision takes it
        // by its bytes.
        bps: include_bytes!("../patches/smb3-usa-rev1-widescreen.zxpk"),
        options: SMB3_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // GraphicsBuf_Prep_And_WaitVSync: LDA VBlank_Tick / BPL, until the NMI counts it down.
        idle_loop: Some((0x96FB, 0x96FE)),
    },
    BuiltinPatch {
        name: SMB3_JAPAN_REV1,
        game: "Super Mario Bros. 3",
        rom_name: "Super Mario Bros. 3 (Japan) (Rev 1)",
        source_crc: 0x52E2_B5E0,
        // The USA hack built for the Japanese code (`smb3wide`, -D JAPAN: the end of the frame's
        // work 15 bytes earlier, Mario's screen limit 23; the same RAM and level data).
        bps: include_bytes!("../patches/smb3-japan-rev1-widescreen.zxpk"),
        options: SMB3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, 15 bytes earlier.
        idle_loop: Some((0x96EC, 0x96EF)),
    },
    BuiltinPatch {
        name: SMB3_JAPAN,
        game: "Super Mario Bros. 3",
        rom_name: "Super Mario Bros. 3 (Japan)",
        source_crc: 0xE7DD_FEE3,
        // The Japanese pack with the first revision's title screen (its 1P/2P menu 39 bytes
        // earlier: the level select's place).
        bps: include_bytes!("../patches/smb3-japan-widescreen.zxpk"),
        options: SMB3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, 15 bytes earlier.
        idle_loop: Some((0x96EC, 0x96EF)),
    },
    BuiltinPatch {
        name: SMB3_EUROPE,
        game: "Super Mario Bros. 3",
        rom_name: "Super Mario Bros. 3 (Europe)",
        source_crc: 0x1ED5_C801,
        // The USA hack built for the European code (`smb3wide`, -D EUROPE: the end of the frame's
        // work and Mario's screen limit 7 bytes earlier), PAL.
        bps: include_bytes!("../patches/smb3-europe-widescreen.zxpk"),
        options: SMB3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, 7 bytes earlier.
        idle_loop: Some((0x96F4, 0x96F7)),
    },
    BuiltinPatch {
        name: SMB3_EUROPE_NTSC,
        game: "Super Mario Bros. 3",
        rom_name: "Super Mario Bros. 3 (Europe)",
        source_crc: 0x1ED5_C801,
        // The European build's CRC, its PAL entry above taking it first: this one is for the
        // ROMs of the European code whose header says NTSC, taken by their signatures
        // (`smb3wide`, `tools/build.py --pack <out> --ntsc`).
        bps: include_bytes!("../patches/smb3-europe-ntsc-widescreen.zxpk"),
        options: SMB3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, 7 bytes earlier.
        idle_loop: Some((0x96F4, 0x96F7)),
    },
    BuiltinPatch {
        name: CASTLEVANIA_USA,
        game: "Castlevania",
        rom_name: "Castlevania (USA)",
        source_crc: 0x0AC1_AA8F,
        // A hack pack (see `crate::hack_pack`): the code in the extension's memory, the hooks
        // checked and applied by the core; the ROM keeps its size.
        // "Invincible" a component of its own (the level selector needs the hack's code: it stays
        // with the widescreen).
        bps: include_bytes!("../patches/castlevania-usa-widescreen.zxpk"),
        options: CASTLEVANIA_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        // No room in the game for code that would unlock the extension: the pack's code is open
        // in the window from power-on (2 KiB: the window stops at $5FFF, as the hack's own code
        // did).
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop stirs the random number.
        idle_loop: Some((0xC030, 0xC03E)),
    },
    BuiltinPatch {
        name: CASTLEVANIA_USA_REV1,
        game: "Castlevania",
        rom_name: "Castlevania (USA) (Rev 1)",
        source_crc: 0xB668_C7FC,
        // A hack pack (see `crate::hack_pack`): the code in the extension's memory, the hooks
        // checked and applied by the core; the ROM keeps its size.
        // "Invincible" a component of its own (the level selector needs the hack's code: it stays
        // with the widescreen).
        bps: include_bytes!("../patches/castlevania-usa-rev1-widescreen.zxpk"),
        options: CASTLEVANIA_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        // No room in the game for code that would unlock the extension: the pack's code is open
        // in the window from power-on (2 KiB: the window stops at $5FFF, as the hack's own code
        // did).
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop stirs the random number.
        idle_loop: Some((0xC030, 0xC03E)),
    },
    BuiltinPatch {
        name: CASTLEVANIA_EUROPE,
        game: "Castlevania",
        rom_name: "Castlevania (Europe)",
        source_crc: 0xA935_27E2,
        // The USA hack built for the European code (`cvwide`, `tools/build.py --pack`: the same
        // source, the game's moved addresses for this build; the USA builds' data banks and RAM;
        // a PAL pack).
        bps: include_bytes!("../patches/castlevania-europe-widescreen.zxpk"),
        options: CASTLEVANIA_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the objects.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The Rev 1's main loop, at the same place.
        idle_loop: Some((0xC030, 0xC03E)),
    },
    BuiltinPatch {
        name: CASTLEVANIA_EUROPE_NTSC,
        game: "Castlevania",
        rom_name: "Castlevania (Europe)",
        // The European build's CRC, its PAL entry above taking it first: this one is for the
        // ROMs of the European code whose header says NTSC (most of its dumps, translations and
        // ROM hacks), taken by their signatures.
        source_crc: 0xA935_27E2,
        // The European pack as an NTSC one (`cvwide`, `tools/build.py --pack <out> --ntsc`).
        bps: include_bytes!("../patches/castlevania-europe-ntsc-widescreen.zxpk"),
        options: CASTLEVANIA_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        idle_loop: Some((0xC030, 0xC03E)),
    },
    BuiltinPatch {
        name: CASTLEVANIA_PC10,
        game: "Castlevania",
        rom_name: "Castlevania (PlayChoice-10)",
        source_crc: 0xCD65_49EA,
        // The USA hack built for the PlayChoice-10 code (`cvwide`, `tools/build.py --pack`,
        // -D PC10: the base build's code rebuilt, its routines moved; the same data banks, RAM
        // and states).
        bps: include_bytes!("../patches/castlevania-pc10-widescreen.zxpk"),
        options: CASTLEVANIA_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The base build's main loop, at the same place.
        idle_loop: Some((0xC030, 0xC03E)),
    },
    BuiltinPatch {
        name: AKUMAJOU_DRACULA_JAPAN,
        game: "Castlevania",
        rom_name: "Akumajou Dracula (Japan)",
        source_crc: 0xA0F9_9BB8,
        // The USA hack built for the Japanese code (`cvwide`, `tools/build.py --pack`, -D JAPAN:
        // its moved addresses, its shorter state dispatch, a hit's damage from its table; the
        // same RAM and states).
        bps: include_bytes!("../patches/akumajou-dracula-japan-widescreen.zxpk"),
        options: CASTLEVANIA_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Its main loop, which stirs the random number.
        idle_loop: Some((0xC056, 0xC064)),
    },
    BuiltinPatch {
        name: ADVENTURE_ISLAND_USA,
        game: "Adventure Island",
        rom_name: "Adventure Island (USA)",
        source_crc: 0xF8A7_13BE,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in groups (the sprites, the ghosts, the previews, and the two
        // aids, which need the hack's code; `aiwide`, `tools/build.py --pack`).
        bps: include_bytes!("../patches/adventure-island-usa-widescreen.zxpk"),
        options: ADVENTURE_ISLAND_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the sprites and objects.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The main loop's wait for the NMI: LDA $0B / BNE.
        idle_loop: Some((0xB759, 0xB75C)),
    },
    BuiltinPatch {
        name: ADVENTURE_ISLAND_EUROPE,
        game: "Adventure Island",
        rom_name: "Adventure Island Classic (Europe)",
        source_crc: 0x8686_7830,
        // The same pack for PAL: the European code is the same at the same places.
        bps: include_bytes!("../patches/adventure-island-europe-widescreen.zxpk"),
        options: ADVENTURE_ISLAND_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the sprites and objects.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The main loop's wait for the NMI: LDA $0B / BNE.
        idle_loop: Some((0xB759, 0xB75C)),
    },
    BuiltinPatch {
        name: ADVENTURE_ISLAND_JAPAN,
        game: "Adventure Island",
        rom_name: "Takahashi Meijin no Bouken-jima (Japan)",
        source_crc: 0x6A45_7A43,
        // The USA hack built for the Japanese code (`aiwide`, -D JAPAN: the same code and RAM,
        // moved).
        bps: include_bytes!("../patches/adventure-island-japan-widescreen.zxpk"),
        options: ADVENTURE_ISLAND_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the sprites and objects.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The main loop's wait for the NMI: LDA $0B / BNE.
        idle_loop: Some((0xB6ED, 0xB6F0)),
    },
    BuiltinPatch {
        name: ADVENTURE_ISLAND_KIDDY_SUN,
        game: "Adventure Island",
        rom_name: "Kiddy Sun in Fantasia (Asia)",
        source_crc: 0x6230_20FB,
        // The Japanese pack but for the NMI's place (4 bytes earlier) and the level selector, its
        // title screen being its own (`aiwide`, `tools/build.py --pack`).
        bps: include_bytes!("../patches/adventure-island-kiddy-sun-widescreen.zxpk"),
        options: ADVENTURE_ISLAND_KIDDY_SUN_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the sprites and objects.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The main loop's wait for the NMI: LDA $0B / BNE.
        idle_loop: Some((0xB6ED, 0xB6F0)),
    },
    BuiltinPatch {
        name: EXCITEBIKE,
        game: "Excitebike",
        rom_name: "Excitebike (Japan, USA) (En)",
        source_crc: 0x49AE_B3A6,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in two groups (the settings, the riders in the margins); the
        // game's 16 KiB as they are (NROM mirrors them; `excitebikewide`, `tools/build.py
        // --pack`).
        bps: include_bytes!("../patches/excitebike-widescreen.zxpk"),
        options: EXCITEBIKE_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the riders.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop draws random numbers while it waits:
        // JSR $D326 / LDA $48 / BMI / BEQ.
        idle_loop: Some((0xC1D8, 0xC1E0)),
    },
    BuiltinPatch {
        name: EXCITEBIKE_EUROPE,
        game: "Excitebike",
        rom_name: "Excitebike (Europe)",
        source_crc: 0x8A0C_7337,
        // The same pack for PAL: the European code is the same at the same places (a table of
        // speeds differs).
        bps: include_bytes!("../patches/excitebike-europe-widescreen.zxpk"),
        options: EXCITEBIKE_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the riders.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the main loop draws random numbers while it waits:
        // JSR $D326 / LDA $48 / BMI / BEQ.
        idle_loop: Some((0xC1D8, 0xC1E0)),
    },
    BuiltinPatch {
        name: FINAL_FANTASY_USA,
        game: "Final Fantasy",
        rom_name: "Final Fantasy (USA)",
        source_crc: 0xCEBD_2A31,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in two groups (the sprites, the people and vehicles in the
        // margins); the ROM keeps its size (`ff1wide`, `tools/build.py --pack`).
        bps: include_bytes!("../patches/final-fantasy-usa-widescreen.zxpk"),
        options: FINAL_FANTASY_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the people and vehicles.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The wait for the NMI: JMP to itself, the NMI returning to the wait's caller.
        idle_loop: Some((0xFEBB, 0xFEBD)),
    },
    BuiltinPatch {
        name: FINAL_FANTASY_JAPAN,
        game: "Final Fantasy",
        rom_name: "Final Fantasy (Japan)",
        source_crc: 0x466E_FDC2,
        // The USA hack built for the Japanese code, which its Rev 1 takes too: the same code and
        // RAM, the fixed bank's at other places (`ff1wide`, `tools/build.py --pack`).
        bps: include_bytes!("../patches/final-fantasy-japan-widescreen.zxpk"),
        options: FINAL_FANTASY_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the people and vehicles.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The wait for the NMI: JMP to itself, the NMI returning to the wait's caller.
        idle_loop: Some((0xFEEE, 0xFEF0)),
    },
    BuiltinPatch {
        name: METALFORCE_KOREA,
        game: "Metal Force",
        rom_name: "Metal Force (Korea) (Unl)",
        source_crc: 0xEC6D_910E,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in two groups (the sprites, the enemies in the margins); the ROM
        // keeps its size. "Invincible" a component of its own (the level selector needs the
        // hack's code: it stays with the widescreen; `metalforcewide`, `tools/build.py --pack`).
        bps: include_bytes!("../patches/metalforce-korea-widescreen.zxpk"),
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
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // Everything runs in the NMI; the reset ends in a JMP to itself.
        idle_loop: Some((0xFFC3, 0xFFC5)),
    },
    BuiltinPatch {
        name: BLASTERMASTER_USA,
        game: "Blaster Master",
        rom_name: "Blaster Master (USA)",
        source_crc: 0x3F0F_D764,
        // A hack pack (see `crate::hack_pack`): the code in the extension's memory, the hooks
        // checked and applied by the core (its aids need the hack's code: they stay with the
        // widescreen).
        bps: include_bytes!("../patches/blastermaster-usa-widescreen.zxpk"),
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
        // No room in the game for code that would unlock the extension: the pack's code is open
        // in the window from power-on.
        boot_window: Some((0x80, 0x00)),
        boot_window_keeps_prg_ram: false,
        boot_zone: false,
        zone_data: false,
        // The frame's wait: BIT $12 / BMI, until the NMI clears it.
        idle_loop: Some((0xE93A, 0xE93D)),
    },
    BuiltinPatch {
        name: METROID_USA,
        game: "Metroid",
        rom_name: "Metroid (USA)",
        source_crc: 0x7008_0810,
        // A hack pack (see `crate::hack_pack`): the code in the extension's memory, the hooks
        // checked and applied by the core (its aids need the hack's code: they stay with the
        // widescreen).
        bps: include_bytes!("../patches/metroid-usa-widescreen.zxpk"),
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
        // No room in the game for code that would unlock the extension: the pack's code is open
        // in the window from power-on (2 KiB: the game's PRG-RAM is at $6000).
        boot_window: Some((0x80, 0x00)),
        boot_window_keeps_prg_ram: true,
        boot_zone: false,
        zone_data: false,
        // The main loop's wait: TAY / LDA $1A / BNE / JMP (the NMI sets $1A).
        idle_loop: Some((0xC0CB, 0xC0D2)),
    },
    BuiltinPatch {
        name: MEGAMAN1_USA,
        game: "Mega Man",
        rom_name: "Mega Man (USA)",
        source_crc: 0x6EE4_BB0A,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core, in two groups (the sprites, the enemies in the margins);
        // "Invincible" a component of its own (`mm1wide`, `tools/build.py --pack`). It takes
        // Rockman (Japan), whose code is the USA's where the hack hooks and calls, by its
        // signatures; UxROM, MMC3, MMC5 and GxROM conversions too.
        bps: include_bytes!("../patches/megaman1-usa-widescreen.zxpk"),
        options: MEGAMAN1_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The frame's wait: LDA $1F / BEQ, until the NMI sets it.
        idle_loop: Some((0xC02B, 0xC02E)),
    },
    BuiltinPatch {
        name: MEGAMAN1_EUROPE,
        game: "Mega Man",
        rom_name: "Mega Man (Europe)",
        source_crc: 0x9447_6A70,
        // The USA hack built for the European code (`mm1wide`, `tools/build.py --pack`,
        // -D EUROPE: its fixed bank's hooks 6 bytes further; the same routines and RAM), PAL.
        bps: include_bytes!("../patches/megaman1-europe-widescreen.zxpk"),
        options: MEGAMAN1_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The frame's wait, at the same place.
        idle_loop: Some((0xC02B, 0xC02E)),
    },
    BuiltinPatch {
        name: MEGAMAN1_EUROPE_NTSC,
        game: "Mega Man",
        rom_name: "Mega Man (Europe)",
        // The European build's CRC, its PAL entry above taking it first: this one is for the
        // ROMs of the European code whose header says NTSC, taken by their signatures.
        source_crc: 0x9447_6A70,
        // The European pack as an NTSC one (`mm1wide`, `tools/build.py --pack <out> --ntsc`).
        bps: include_bytes!("../patches/megaman1-europe-ntsc-widescreen.zxpk"),
        options: MEGAMAN1_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        idle_loop: Some((0xC02B, 0xC02E)),
    },
    BuiltinPatch {
        name: MEGAMAN3_USA,
        game: "Mega Man 3",
        rom_name: "Mega Man 3 (USA)",
        source_crc: 0x603A_AA57,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in two groups (the sprites, the enemies in the margins);
        // "Invincible" a component of its own (`mm3wide`, `tools/build.py --pack`).
        bps: include_bytes!("../patches/megaman3-usa-widescreen.zxpk"),
        options: MEGAMAN3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The task scheduler's wait for a task to wake (the NMI wakes them).
        idle_loop: Some((0xFEAD, 0xFEC2)),
    },
    BuiltinPatch {
        name: ROCKMAN3_JAPAN,
        game: "Mega Man 3",
        rom_name: "Rockman 3 - Dr. Wily no Saigo!? (Japan)",
        source_crc: 0xD9F1_E47C,
        // The USA hack built for the Japanese code (`mm3wide`, -D JAPAN: its sprite code
        // moved in the fixed bank; the same RAM and routines).
        bps: include_bytes!("../patches/rockman3-japan-widescreen.zxpk"),
        options: MEGAMAN3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, at the same place.
        idle_loop: Some((0xFEAD, 0xFEC2)),
    },
    BuiltinPatch {
        name: MEGAMAN3_EUROPE,
        game: "Mega Man 3",
        rom_name: "Mega Man 3 (Europe)",
        source_crc: 0x837A_3D8A,
        // The USA hack built for the European code (`mm3wide`, -D EUROPE: its sprite code
        // moved in the fixed bank), PAL.
        bps: include_bytes!("../patches/megaman3-europe-widescreen.zxpk"),
        options: MEGAMAN3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, at the same place.
        idle_loop: Some((0xFEAD, 0xFEC2)),
    },
    BuiltinPatch {
        name: MEGAMAN3_EUROPE_NTSC,
        game: "Mega Man 3",
        rom_name: "Mega Man 3 (Europe)",
        source_crc: 0x837A_3D8A,
        // The European pack as an NTSC one, for the ROMs of that code whose header says NTSC,
        // taken by their signatures (its PAL entry above takes the CRC first).
        bps: include_bytes!("../patches/megaman3-europe-ntsc-widescreen.zxpk"),
        options: MEGAMAN3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, at the same place.
        idle_loop: Some((0xFEAD, 0xFEC2)),
    },
    BuiltinPatch {
        name: MEGAMAN3_EUROPE_REV1,
        game: "Mega Man 3",
        rom_name: "Mega Man 3 (Europe) (Rev 1)",
        source_crc: 0x0013_88B3,
        // The USA hack built for the code of the European Rev 1 (`mm3wide`, -D EUROPE_REV1:
        // its sprite code and the hiding of the rest of OAM moved), PAL.
        bps: include_bytes!("../patches/megaman3-europe-rev1-widescreen.zxpk"),
        options: MEGAMAN3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, at the same place.
        idle_loop: Some((0xFEAD, 0xFEC2)),
    },
    BuiltinPatch {
        name: MEGAMAN3_EUROPE_REV1_NTSC,
        game: "Mega Man 3",
        rom_name: "Mega Man 3 (Europe) (Rev 1)",
        source_crc: 0x0013_88B3,
        // The Rev 1's pack as an NTSC one, for the ROMs of that code whose header says NTSC.
        bps: include_bytes!("../patches/megaman3-europe-rev1-ntsc-widescreen.zxpk"),
        options: MEGAMAN3_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, at the same place.
        idle_loop: Some((0xFEAD, 0xFEC2)),
    },
    BuiltinPatch {
        name: MEGAMAN4_USA,
        game: "Mega Man 4",
        rom_name: "Mega Man 4 (USA)",
        source_crc: 0x2BC6_7AA8,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in two groups (the sprites, the enemies in the margins);
        // "Invincible" a component of its own (`mm4wide`, `tools/build.py --pack`). Rockman 4
        // (Japan), whose code is the USA's where the hack hooks and calls, takes it by its
        // signatures.
        bps: include_bytes!("../patches/megaman4-usa-widescreen.zxpk"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The task scheduler's wait for a task to wake (the NMI wakes them).
        idle_loop: Some((0xFEA8, 0xFEBC)),
    },
    BuiltinPatch {
        name: MEGAMAN4_USA_REV1,
        game: "Mega Man 4",
        rom_name: "Mega Man 4 (USA) (Rev 1)",
        source_crc: 0x18A2_E74F,
        // A hack pack (see `crate::hack_pack`): the code in the extension's memory, the hooks
        // checked and applied by the core; "Invincible" a component of its own.
        // The pack of Mega Man 4 (USA): its hooks find their bytes in this revision too.
        bps: include_bytes!("../patches/megaman4-usa-widescreen.zxpk"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The task scheduler's wait for a task to wake (the NMI wakes them).
        idle_loop: Some((0xFEA8, 0xFEBC)),
    },
    BuiltinPatch {
        name: MEGAMAN4_EUROPE,
        game: "Mega Man 4",
        rom_name: "Mega Man 4 (Europe)",
        source_crc: 0x1B93_2BEA,
        // The USA pack built from the European ROM, PAL (`mm4wide`, `tools/build.py --pack`): its
        // code is the same where the hack hooks and calls.
        bps: include_bytes!("../patches/megaman4-europe-widescreen.zxpk"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, at the same place.
        idle_loop: Some((0xFEA8, 0xFEBC)),
    },
    BuiltinPatch {
        name: MEGAMAN5_USA,
        game: "Mega Man 5",
        rom_name: "Mega Man 5 (USA)",
        source_crc: 0x3EDC_F7E8,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in two groups (the sprites, the enemies in the margins);
        // "Invincible" a component of its own (`mm5wide`, `tools/build.py --pack`). Rockman 5
        // (Japan), whose code is the USA's where the hack hooks and calls, takes it by its
        // signatures.
        bps: include_bytes!("../patches/megaman5-usa-widescreen.zxpk"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The task scheduler's wait for a task to wake (the NMI wakes them).
        idle_loop: Some((0xFEAE, 0xFEC3)),
    },
    BuiltinPatch {
        name: MEGAMAN5_EUROPE,
        game: "Mega Man 5",
        rom_name: "Mega Man 5 (Europe)",
        source_crc: 0xA4DC_F72E,
        // The USA hack built for the European code (`mm5wide`, -D EUROPE: its fixed banks' code
        // 2 bytes earlier from $C100; the same RAM and stage data), PAL.
        bps: include_bytes!("../patches/megaman5-europe-widescreen.zxpk"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, at the same place.
        idle_loop: Some((0xFEAE, 0xFEC3)),
    },
    BuiltinPatch {
        name: MEGAMAN6_USA,
        game: "Mega Man 6",
        rom_name: "Mega Man 6 (USA)",
        source_crc: 0x9887_98A8,
        // A hack pack (see `crate::hack_pack`): the code in the zone fantôme, the hooks checked
        // and applied by the core in two groups (the sprites, the enemies in the margins);
        // "Invincible" a component of its own (`mm6wide`, `tools/build.py --pack`).
        bps: include_bytes!("../patches/megaman6-usa-widescreen.zxpk"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        // The margin decoder alone widens the game; the patch adds the enemies.
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The task scheduler's wait for the NMI (no task ready).
        idle_loop: Some((0xC581, 0xC584)),
    },
    BuiltinPatch {
        name: ROCKMAN6_JAPAN,
        game: "Mega Man 6",
        rom_name: "Rockman 6 (Japan)",
        source_crc: 0x2D66_4D99,
        // The USA hack built for the Japanese code (`mm6wide`, -D JAPAN: its fixed banks' code 4
        // bytes later from $D0D9 to $FA8A; the same RAM and stage data).
        bps: include_bytes!("../patches/rockman6-japan-widescreen.zxpk"),
        options: MEGAMAN4_OPTIONS,
        after_other_way_state: None,
        advanced: true,
        // The pack's code runs in the execution zone, open from power-on.
        boot_window: None,
        boot_window_keeps_prg_ram: false,
        boot_zone: true,
        // Widescreen MMC5: the extension at $3000-$3FFF of the zone fantôme.
        zone_data: true,
        // The same wait, at the same place.
        idle_loop: Some((0xC581, 0xC584)),
    },
];

/// The options of the Mega Man 3 packs.
const MEGAMAN3_OPTIONS: &[HackOption] = &[
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
];

/// The options of the Mega Man packs.
const MEGAMAN1_OPTIONS: &[HackOption] = &[
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
];

/// The options of both Mega Man 2 patches.
const MEGAMAN2_OPTIONS: &[HackOption] = &[
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
];

/// The options of the Final Fantasy packs: the overclock.
const FINAL_FANTASY_OPTIONS: &[HackOption] = &[
    // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
    HackOption {
        key: "no_slowdown",
        label: "No slowdown",
        label_fr: "Sans ralentissements",
        bit: 0,
        default: true,
    },
];

/// The options of the Adventure Island packs: testing aids, off by default, and the overclock.
const ADVENTURE_ISLAND_OPTIONS: &[HackOption] = &[
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
];

/// The options of the Kiddy Sun in Fantasia pack: its title screen is its own (no level selector).
const ADVENTURE_ISLAND_KIDDY_SUN_OPTIONS: &[HackOption] = &[
    // A testing aid: no injury from the enemies (falls, the water and hunger still kill).
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

/// The options of the Excitebike packs: the overclock.
const EXCITEBIKE_OPTIONS: &[HackOption] = &[
    // Nearly a frame more of CPU time ($5006 = 255): no lag frames.
    HackOption {
        key: "no_slowdown",
        label: "No slowdown",
        label_fr: "Sans ralentissements",
        bit: 2,
        default: true,
    },
];

/// The options of the Ninja Gaiden III packs: the enemies in the margins and the overclock.
const NG3_OPTIONS: &[HackOption] = &[
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
];

/// The options of the Super Mario Bros. packs: testing aids, off by default, and the overclock.
const SMB1_OPTIONS: &[HackOption] = &[
    // Testing aids: B on the title screen goes through every level; Mario never hurt (falls and
    // the timer still kill).
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
];

/// The options of the Super Mario Bros. 3 packs: testing aids, off by default (components of
/// their own, without code), and the overclock.
const SMB3_OPTIONS: &[HackOption] = &[
    // Testing aids: B on the 1P/2P menu opens the game's own debug menu (Up/Down: the world);
    // Mario never hurt (pits still kill).
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
        None => match pack_matching(image, data) {
            Some((patch, _)) => (patch, true),
            None => (forced?, false),
        },
    };
    if crate::hack_pack::is_pack(patch.bps) {
        // A hack pack: the widescreen's hooks checked and applied (the groups of it that find
        // their bytes); the ROM keeps its size, the code goes to the extension's memory.
        let pack = crate::hack_pack::HackPack::read(patch.bps)?;
        let applied = pack.resolve(data, &crate::hack_pack::RomTraits::of(image, data));
        let widescreen = applied.component(crate::hack_pack::WIDESCREEN)?;
        let mut patched_image = image[..end].to_vec();
        widescreen.apply(&mut patched_image[start..], true);
        return Some(PatchedRom {
            image: patched_image,
            name: patch.name,
            code: applied.code,
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

/// The built-in hack pack that applies best to `image` (its data `data`, PRG-ROM then CHR-ROM),
/// whatever its CRC: another build of the game, or a ROM hack, whose hooked code (and the code
/// the hack relies on) is the same, of the region and on a mapper the pack is made for (see
/// [`crate::hack_pack`]). Best: its widescreen applies, then the most groups, then the first in
/// [`PATCHES`] (two builds of a game may share an aid's bytes but not the widescreen's).
fn pack_matching(
    image: &[u8],
    data: &[u8],
) -> Option<(&'static BuiltinPatch, crate::hack_pack::AppliedPack)> {
    let traits = crate::hack_pack::RomTraits::of(image, data);
    let score = |applied: &crate::hack_pack::AppliedPack| {
        let widescreen = applied.component(crate::hack_pack::WIDESCREEN).is_some();
        let groups: usize = applied
            .components
            .iter()
            .map(|component| component.groups.len())
            .sum();
        (widescreen, groups)
    };
    let mut best: Option<(&'static BuiltinPatch, crate::hack_pack::AppliedPack)> = None;
    for patch in &PATCHES {
        let Some(pack) = crate::hack_pack::HackPack::read(patch.bps) else {
            continue;
        };
        let applied = pack.resolve(data, &traits);
        if applied.applies()
            && best
                .as_ref()
                .is_none_or(|(_, kept)| score(&applied) > score(kept))
        {
            best = Some((patch, applied));
        }
    }
    best
}

/// The hack pack of the built-in patch made for `image`, as it applies to it (its components
/// that find their bytes, each judged apart): what the core takes the pack's aids from (their
/// options), with or without the advanced mode.
#[must_use]
pub fn pack_for(image: &[u8]) -> Option<(&'static BuiltinPatch, crate::hack_pack::AppliedPack)> {
    let data = rom_data(image);
    // The pack of the ROM's CRC first; when its gate refuses it (a header that says another
    // region), the pack its signatures take, another build's of the same code among them.
    for_data_crc(crc32(data))
        .and_then(|patch| {
            let applied = crate::hack_pack::HackPack::read(patch.bps)?
                .resolve(data, &crate::hack_pack::RomTraits::of(image, data));
            applied.applies().then_some((patch, applied))
        })
        .or_else(|| pack_matching(image, data))
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
    for_data_crc(data_crc(image))
        .or_else(|| pack_matching(image, rom_data(image)).map(|(patch, _)| patch))
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
    // is not "Super Mario Bros.", "Castlevania II" not "Castlevania"); of a game's several
    // builds, the first listed (`max_by_key` keeps the last of equals: the list reversed).
    PATCHES
        .iter()
        .rev()
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
