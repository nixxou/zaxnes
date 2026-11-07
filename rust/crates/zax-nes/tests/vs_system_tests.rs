#![cfg(feature = "board-vs-system")]
// A failed setup panics, which is how a test says the fixture is wrong.
#![allow(clippy::expect_used, clippy::panic)]

//! The Nintendo VS. System: detection, the per-ROM table and the load note (lot MF-VS).
//!
//! The claims under test are the nesdev wiki's: "NES 2.0" for byte 7's console type, byte 13's
//! two nibbles and byte 15's default expansion device, and "Vs. System" for the machine itself.
//! The header fields themselves are REQ-CORE-012's.

use zax_nes::cartridges::ChrFetch;
use zax_nes::rom::InesHeader;
use zax_nes::vs::{rom_table, VsHardware, VsPpuModel, VsSource, VsSystem, VsWiring, LUT_LEN};
use zax_nes::{NametableMirroring, NesCore};

/// The ten PPU models, in the order byte 13's low nibble names them.
const MODELS: [VsPpuModel; 10] = [
    VsPpuModel::Rc2c03,
    VsPpuModel::Rp2c04_0001,
    VsPpuModel::Rp2c04_0002,
    VsPpuModel::Rp2c04_0003,
    VsPpuModel::Rp2c04_0004,
    VsPpuModel::Rc2c05_01,
    VsPpuModel::Rc2c05_02,
    VsPpuModel::Rc2c05_03,
    VsPpuModel::Rc2c05_04,
    VsPpuModel::Rc2c05_05,
];

/// Byte 13's low nibble for `model`, so that a synthetic image can ask for it by name.
fn byte13_of(model: VsPpuModel) -> u8 {
    (0x0..=0x0Fu8)
        .find(|&nibble| VsPpuModel::from_nes20_byte13(nibble) == model)
        .unwrap_or_else(|| panic!("no byte 13 names {}", model.name()))
}

/// A 32 KiB-PRG, 8 KiB-CHR image whose header bytes the caller sets.
///
/// `mapper` is written the iNES way (low nibble of byte 6, high nibble of byte 7); `byte7_low`
/// carries the console type bits and the NES 2.0 identifier, and bytes 13 and 15 are only read
/// when that identifier says NES 2.0.
fn image(mapper: u8, byte7_low: u8, byte13: u8, byte15: u8) -> Vec<u8> {
    let mut image = vec![0u8; 16 + 32 * 1024 + 8 * 1024];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 2; // 32 KiB of PRG-ROM.
    image[5] = 1; // 8 KiB of CHR-ROM.
    image[6] = (mapper & 0x0F) << 4;
    image[7] = (mapper & 0xF0) | byte7_low;
    image[13] = byte13;
    image[15] = byte15;

    // Somewhere for the CPU to go: an endless loop at $8000 and the three vectors on it.
    image[16..19].copy_from_slice(&[0x4C, 0x00, 0x80]);
    let vectors = 16 + 32 * 1024 - 6;
    for slot in 0..3 {
        image[vectors + 2 * slot] = 0x00;
        image[vectors + 2 * slot + 1] = 0x80;
    }
    image
}

fn header_of(image: &[u8]) -> InesHeader {
    InesHeader::try_parse(image).expect("the fixture is a valid header")
}

fn detect(image: &[u8]) -> Option<VsSystem> {
    VsSystem::detect(&header_of(image), 0xDEAD_BEEF)
}

// ------------------------------------------------------------------ detection

/// REQ-CORE-012: byte 7's two low bits are read on a clean iNES header, and an ordinary NES
/// image is not a VS. System.
#[test]
fn a_plain_nes_image_is_not_a_vs_system() {
    let image = image(0, 0x00, 0, 0);
    assert_eq!(header_of(&image).console_type, 0);
    assert!(detect(&image).is_none());
}

/// nesdev, "NES 2.0": byte 7 bit 0 of an iNES 1.0 header is the Vs. Unisystem flag.
#[test]
fn an_ines_1_0_header_with_byte_7_bit_0_is_a_vs_system() {
    let vs = detect(&image(0, 0x01, 0, 0)).expect("byte 7 bit 0 is set");
    assert_eq!(vs.ppu(), VsPpuModel::Rc2c03);
    assert_eq!(vs.hardware(), VsHardware::Unisystem);
    assert_eq!(vs.source(), VsSource::Default);
}

/// The PlayChoice-10 bit is byte 7 bit 1 and is not the VS one.
#[test]
fn the_playchoice_bit_alone_is_not_a_vs_system() {
    assert!(detect(&image(0, 0x02, 0, 0)).is_none());
}

/// REQ-CORE-012: bytes 7 to 15 of a dirty header are not read at all, so a free-text comment
/// cannot turn an image into an arcade board.
#[test]
fn a_dirty_header_is_never_a_vs_system() {
    let mut raw = image(0, 0x01, 0, 0);
    raw[12..16].copy_from_slice(b"Dude");
    let header = header_of(&raw);
    assert!(header.is_dirty);
    assert_eq!(header.console_type, 0);
    assert!(VsSystem::detect(&header, 0xDEAD_BEEF).is_none());
}

/// nesdev, "NES 2.0": console type 1 is the Nintendo Vs. System, and byte 13 then carries the
/// PPU in its low nibble and the hardware in its high one.
#[test]
fn nes20_console_type_1_reads_byte_13() {
    // $5 is the RP2C04-0004 and $5 the Vs. DualSystem.
    let vs = detect(&image(0, 0x09, 0x55, 0)).expect("console type 1");
    assert_eq!(vs.ppu(), VsPpuModel::Rp2c04_0004);
    assert_eq!(vs.hardware(), VsHardware::DualSystem);
    assert_eq!(vs.source(), VsSource::Header);
}

/// Every value of byte 13's low nibble, as the page's Vs. PPU Type table gives it. `$0`, `$1`,
/// `$6` and `$7` are the four RP2C03/RC2C03 variants and share one entry, and `$D` to `$F` are
/// reserved.
#[test]
fn byte_13_names_every_ppu_model() {
    let expected = [
        (0x0, VsPpuModel::Rc2c03),
        (0x1, VsPpuModel::Rc2c03),
        (0x2, VsPpuModel::Rp2c04_0001),
        (0x3, VsPpuModel::Rp2c04_0002),
        (0x4, VsPpuModel::Rp2c04_0003),
        (0x5, VsPpuModel::Rp2c04_0004),
        (0x6, VsPpuModel::Rc2c03),
        (0x7, VsPpuModel::Rc2c03),
        (0x8, VsPpuModel::Rc2c05_01),
        (0x9, VsPpuModel::Rc2c05_02),
        (0xA, VsPpuModel::Rc2c05_03),
        (0xB, VsPpuModel::Rc2c05_04),
        (0xC, VsPpuModel::Rc2c05_05),
        (0xD, VsPpuModel::Rc2c03),
        (0xE, VsPpuModel::Rc2c03),
        (0xF, VsPpuModel::Rc2c03),
    ];

    for (nibble, model) in expected {
        assert_eq!(
            VsPpuModel::from_nes20_byte13(nibble),
            model,
            "byte 13 low nibble ${nibble:X}"
        );
        // The high nibble must not reach the PPU choice.
        assert_eq!(VsPpuModel::from_nes20_byte13(0xF0 | nibble), model);
    }
}

/// The same for the high nibble's Vs. Hardware Type table, `$7` to `$F` reserved.
#[test]
fn byte_13_names_every_hardware_type() {
    let expected = [
        (0x0, VsHardware::Unisystem),
        (0x1, VsHardware::RbiBaseball),
        (0x2, VsHardware::TkoBoxing),
        (0x3, VsHardware::SuperXevious),
        (0x4, VsHardware::IceClimberJapan),
        (0x5, VsHardware::DualSystem),
        (0x6, VsHardware::RaidOnBungelingBay),
        (0x7, VsHardware::Unisystem),
        (0xF, VsHardware::Unisystem),
    ];

    for (nibble, hardware) in expected {
        assert_eq!(
            VsHardware::from_nes20_byte13(nibble << 4),
            hardware,
            "byte 13 high nibble ${nibble:X}"
        );
        assert_eq!(
            VsHardware::from_nes20_byte13((nibble << 4) | 0x0F),
            hardware
        );
    }
}

/// nesdev, "NES 2.0": console type 3 puts the real type in byte 13's low nibble, and `$1` there
/// is the Vs. System. That form names no PPU, so the configuration falls through to the table.
#[test]
fn nes20_extended_console_type_1_is_a_vs_system() {
    let vs = detect(&image(0, 0x0B, 0x01, 0)).expect("extended console type 1");
    assert_eq!(vs.source(), VsSource::Default);
    assert_eq!(vs.ppu(), VsPpuModel::Rc2c03);

    // $0 there is a regular NES, and $7 is a V.R. Technology VT03: neither is a VS. System.
    assert!(detect(&image(0, 0x0B, 0x00, 0)).is_none());
    assert!(detect(&image(0, 0x0B, 0x07, 0)).is_none());
}

/// Mappers 99 and 151 exist only on this board, so the number alone settles it even when the
/// header's console type says "NES" ("INES Mapper 099"; 151 is the deprecated number for a VRC1
/// on a VS board).
#[test]
fn mapper_99_and_151_are_vs_systems_whatever_the_console_type_says() {
    for mapper in [99u8, 151] {
        let image = image(mapper, 0x00, 0, 0);
        assert_eq!(header_of(&image).mapper, i32::from(mapper));
        assert!(
            detect(&image).is_some(),
            "mapper {mapper} is a VS. System by its number"
        );
    }

    // And a neighbouring number is not.
    assert!(detect(&image(100, 0x00, 0, 0)).is_none());
    assert!(detect(&image(152, 0x00, 0, 0)).is_none());
}

/// nesdev, "NES 2.0", Default Expansion Device: `$04` is "Vs. System (1P via $4016)", `$05`
/// "Vs. System (1P via $4017)" and `$07` the Vs. Zapper. Only `$05` swaps the sticks.
#[test]
fn byte_15_gives_the_stick_wiring() {
    assert_eq!(
        VsWiring::from_nes20_byte15(0x04),
        Some(VsWiring::PlayerOneOnPort0)
    );
    assert_eq!(
        VsWiring::from_nes20_byte15(0x05),
        Some(VsWiring::PlayerOneOnPort1)
    );
    assert_eq!(VsWiring::from_nes20_byte15(0x07), Some(VsWiring::Zapper));
    assert_eq!(VsWiring::from_nes20_byte15(0x01), None);
    assert_eq!(VsWiring::from_nes20_byte15(0x08), None);

    assert!(!VsWiring::PlayerOneOnPort0.swaps_sticks());
    assert!(VsWiring::PlayerOneOnPort1.swaps_sticks());
    assert!(!VsWiring::Zapper.swaps_sticks());

    let swapped = detect(&image(0, 0x09, 0x00, 0x05)).expect("console type 1");
    assert_eq!(swapped.wiring(), VsWiring::PlayerOneOnPort1);
    let plain = detect(&image(0, 0x09, 0x00, 0x04)).expect("console type 1");
    assert_eq!(plain.wiring(), VsWiring::PlayerOneOnPort0);
}

/// The RC2C05 family, which step 3 of the lot gives its two register quirks to.
#[test]
fn only_the_rc2c05_models_are_rc2c05() {
    for model in [
        VsPpuModel::Rc2c05_01,
        VsPpuModel::Rc2c05_02,
        VsPpuModel::Rc2c05_03,
        VsPpuModel::Rc2c05_04,
        VsPpuModel::Rc2c05_05,
    ] {
        assert!(model.is_rc2c05(), "{}", model.name());
    }

    for model in [
        VsPpuModel::Rc2c03,
        VsPpuModel::Rp2c04_0001,
        VsPpuModel::Rp2c04_0002,
        VsPpuModel::Rp2c04_0003,
        VsPpuModel::Rp2c04_0004,
    ] {
        assert!(!model.is_rc2c05(), "{}", model.name());
    }
}

// ------------------------------------------------------------------ the per-ROM table

/// [`zax_nes::vs::rom_table`] is bisected, so it has to be sorted and free of duplicate keys.
#[test]
fn the_rom_table_is_sorted_and_has_no_duplicate_crc() {
    let table = rom_table();
    assert_eq!(table.len(), 74, "the 74 VS entries of the header database");
    for pair in table.windows(2) {
        assert!(
            pair[0].crc() < pair[1].crc(),
            "{:08X} then {:08X}: the table must be sorted by CRC and hold each key once",
            pair[0].crc(),
            pair[1].crc()
        );
    }
}

/// Every row is reachable through the detection path, with the row's own configuration.
#[test]
fn every_row_of_the_table_is_found_by_its_crc() {
    // An iNES 1.0 VS header: the form that has no PPU field and therefore needs the table.
    let image = image(0, 0x01, 0, 0);
    let header = header_of(&image);

    for row in rom_table() {
        let vs = VsSystem::detect(&header, row.crc()).expect("byte 7 bit 0 is set");
        assert_eq!(vs.source(), VsSource::RomTable, "{}", row.title());
        assert_eq!(vs.ppu(), row.ppu(), "{}", row.title());
        assert_eq!(vs.hardware(), row.hardware(), "{}", row.title());
        assert_eq!(vs.wiring(), row.wiring(), "{}", row.title());
        assert_eq!(vs.title(), Some(row.title()));
    }
}

/// Four rows written out, the two the owner runs among them, so that a change to the generated
/// table is visible in a diff.
#[test]
fn four_known_dumps_have_the_configuration_the_database_gives_them() {
    let expected = [
        (
            0xFFBE_F374u32,
            "Vs. Castlevania",
            VsPpuModel::Rp2c04_0002,
            VsHardware::Unisystem,
            VsWiring::PlayerOneOnPort0,
        ),
        (
            0xFB0D_DDE7,
            "Excitebike (EB4-4 A)",
            VsPpuModel::Rp2c04_0004,
            VsHardware::Unisystem,
            VsWiring::PlayerOneOnPort0,
        ),
        (
            0x07A2_F3B2,
            "Vs. Gumshoe",
            VsPpuModel::Rc2c05_03,
            VsHardware::Unisystem,
            VsWiring::Zapper,
        ),
        (
            0xEE8A_F512,
            "Vs. Gradius",
            VsPpuModel::Rp2c04_0001,
            VsHardware::Unisystem,
            VsWiring::PlayerOneOnPort1,
        ),
    ];

    for (crc, title, ppu, hardware, wiring) in expected {
        let row = rom_table()
            .iter()
            .find(|row| row.crc() == crc)
            .unwrap_or_else(|| panic!("{title} is missing from the table"));
        assert_eq!(row.title(), title);
        assert_eq!(row.ppu(), ppu, "{title}");
        assert_eq!(row.hardware(), hardware, "{title}");
        assert_eq!(row.wiring(), wiring, "{title}");
    }
}

/// Lot MF-VS-fix step 1: a row carries the **board** the database records, and the two rows the
/// defect is about are the two with CHR-RAM. REQ-CORE-012 is what makes them the hard case: an
/// iNES 1.0 image with no CHR-ROM gets 32 KiB of CHR-RAM by default and these boards have 8.
#[test]
fn the_rows_carry_the_board_the_database_records() {
    let expected = [
        // (CRC, title, mapper, submapper, PRG-ROM, CHR-ROM, CHR-RAM)
        (
            0xFFBE_F374u32,
            "Vs. Castlevania",
            2,
            2,
            128 * 1024,
            0,
            8 * 1024,
        ),
        (0x8C0C_2DF5, "Vs. Top Gun", 2, 2, 128 * 1024, 0, 8 * 1024),
        // The two 40 KiB PRG images of nesdev's "INES Mapper 099", the size that is not a whole
        // number of the 16 KiB units byte 4 counts (MF-VS step 4).
        (0x07A2_F3B2, "Vs. Gumshoe", 99, 0, 40 * 1024, 16 * 1024, 0),
        (
            0x70FF_B591,
            "Raid on Bungeling Bay",
            99,
            0,
            40 * 1024,
            16 * 1024,
            0,
        ),
        // The one row with a submapper that is not a bus-conflict bit.
        (
            0x21A6_53C7,
            "Super Sky Kid",
            206,
            1,
            32 * 1024,
            32 * 1024,
            0,
        ),
        // Mapper 75, the VRC1 the deprecated number 151 also names (MF-VS step 4).
        (0xEE8A_F512, "Vs. Gradius", 75, 0, 64 * 1024, 64 * 1024, 0),
    ];

    for (crc, title, mapper, submapper, prg, chr, chr_ram) in expected {
        let row = rom_table()
            .iter()
            .find(|row| row.crc() == crc)
            .unwrap_or_else(|| panic!("{title} is missing from the table"));
        let board = row.board();
        assert_eq!(row.title(), title);
        assert_eq!(board.mapper(), mapper, "{title}");
        assert_eq!(board.submapper(), submapper, "{title}");
        assert_eq!(board.prg_rom_size(), prg, "{title}");
        assert_eq!(board.chr_rom_size(), chr, "{title}");
        assert_eq!(board.chr_ram_size(), chr_ram, "{title}");
    }
}

/// What all 74 boards have in common, which is what step 2's correction relies on: a mapper this
/// build runs, four-screen VRAM, some PRG-ROM, and exactly one of the two CHR memories.
#[test]
fn every_board_of_the_table_is_one_this_build_can_run() {
    for row in rom_table() {
        let board = row.board();
        let title = row.title();
        assert!(
            NesCore::SUPPORTED_MAPPERS.contains(&board.mapper()),
            "{title}: mapper {} is not registered",
            board.mapper()
        );
        // Every VS. System board is wired for four-screen VRAM, which is also the condition the
        // VRC1 of mappers 75 and 151 ignores its mirroring bit under (MF-VS step 4).
        assert_eq!(board.mirroring(), NametableMirroring::FourScreen, "{title}");
        assert!(board.prg_rom_size() >= 8 * 1024, "{title}");
        assert_eq!(board.prg_rom_size() % (8 * 1024), 0, "{title}");
        assert_eq!(
            board.chr_rom_size() == 0,
            board.chr_ram_size() != 0,
            "{title}: a board has CHR-ROM or CHR-RAM and not both"
        );
    }
}

/// The header wins over the table when it carries a PPU, the table filling in only the title
/// and, if byte 15 says nothing, the wiring.
#[test]
fn the_header_wins_over_the_table() {
    // Vs. Castlevania's CRC, in an image whose byte 13 claims an RC2C05-01 instead.
    let vs = VsSystem::detect(&header_of(&image(2, 0x09, 0x08, 0)), 0xFFBE_F374)
        .expect("console type 1");
    assert_eq!(vs.source(), VsSource::Header);
    assert_eq!(vs.ppu(), VsPpuModel::Rc2c05_01);
    assert_eq!(vs.title(), Some("Vs. Castlevania"));
    // Byte 15 is zero, which names no device, so the row's own wiring stands.
    assert_eq!(vs.wiring(), VsWiring::PlayerOneOnPort0);
}

// ------------------------------------------------------------------ the load note

/// The owner's rule, literally: an unknown VS image "gets the RP2C03 (plain NES palette) and a
/// load message saying so".
#[test]
fn an_unknown_vs_image_says_it_takes_the_plain_nes_palette() {
    let vs = detect(&image(0, 0x01, 0, 0)).expect("byte 7 bit 0 is set");
    let note = vs.load_note();
    assert!(note.contains("RP2C03"), "{note}");
    assert!(note.contains("unknown VS image"), "{note}");
    assert!(note.contains("plain NES palette"), "{note}");
}

/// Everything the build does not emulate is named in the note rather than left silent.
#[test]
fn the_note_names_every_gap_of_the_image() {
    // Raid on Bungeling Bay: a DualSystem *and* a protection board, and its row's wiring is the
    // swapped one.
    let vs = VsSystem::detect(&header_of(&image(99, 0x01, 0, 0)), 0x70FF_B591)
        .expect("mapper 99 is a VS. System");
    assert_eq!(vs.hardware(), VsHardware::RaidOnBungelingBay);
    let note = vs.load_note();
    assert!(note.contains("Raid on Bungeling Bay"), "{note}");
    assert!(note.contains("single screen"), "{note}");
    assert!(note.contains("protection is not emulated"), "{note}");

    // Vs. Gumshoe is a Zapper game, which the note says too.
    let gumshoe = VsSystem::detect(&header_of(&image(99, 0x01, 0, 0)), 0x07A2_F3B2)
        .expect("mapper 99 is a VS. System");
    assert!(gumshoe.load_note().contains("Zapper is not emulated"));

    // A plain Unisystem names no gap at all.
    let battle_city = VsSystem::detect(&header_of(&image(99, 0x01, 0, 0)), 0x0135_7944)
        .expect("mapper 99 is a VS. System");
    let note = battle_city.load_note();
    assert!(note.contains("Battle City"), "{note}");
    assert!(!note.contains("not emulated"), "{note}");
}

// ------------------------------------------------------------------ through the core

/// A whole load: the core keeps the VS. System and hands the note back, and a NES image gets
/// neither.
#[test]
fn the_core_keeps_the_vs_system_and_reports_it() {
    let mut core = zax_nes::NesCore::new();

    let load = core.load_rom(&image(0, 0x01, 0, 0), "SyntheticVs.nes");
    assert!(load.success, "{:?}", load.error);
    let note = load.note.expect("a VS image carries a load note");
    assert!(note.starts_with("VS. System: "), "{note}");
    assert_eq!(
        core.vs_system().map(VsSystem::ppu),
        Some(VsPpuModel::Rc2c03)
    );

    let load = core.load_rom(&image(0, 0x00, 0, 0), "SyntheticNes.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(load.note, None);
    assert!(core.vs_system().is_none());

    // And closing the ROM unplugs it.
    let load = core.load_rom(&image(0, 0x01, 0, 0), "SyntheticVs.nes");
    assert!(load.success, "{:?}", load.error);
    assert!(core.vs_system().is_some());
    core.close_rom();
    assert!(core.vs_system().is_none());
}

// ------------------------------------------------------------------ the master palettes

/// nesdev, "PPU palettes": "All four 2C04 PPUs contain the same master palette, but in different
/// permutations", offered as a LUT into the ordered palette. So each of the four arrays holds 64
/// six-bit indices, the four are different from one another, none of them is the identity, and
/// each holds the same multiset of indices as the others.
#[test]
fn the_four_rp2c04_luts_are_four_different_remaps_of_the_ordered_palette() {
    let luts: Vec<&[u8; LUT_LEN]> = [
        VsPpuModel::Rp2c04_0001,
        VsPpuModel::Rp2c04_0002,
        VsPpuModel::Rp2c04_0003,
        VsPpuModel::Rp2c04_0004,
    ]
    .iter()
    .map(|model| model.palette_lut().expect("an RP2C04 has a remap"))
    .collect();

    for (index, lut) in luts.iter().enumerate() {
        assert!(
            lut.iter().all(|&entry| entry < 0x40),
            "RP2C04-000{} holds an index outside the 64 colours",
            index + 1
        );
        assert!(
            lut.iter()
                .enumerate()
                .any(|(from, &to)| u8::try_from(from).unwrap_or(0xFF) != to),
            "RP2C04-000{} is the identity",
            index + 1
        );
    }

    for first in 0..luts.len() {
        for second in (first + 1)..luts.len() {
            assert_ne!(
                luts[first],
                luts[second],
                "RP2C04-000{} and RP2C04-000{} are the same table",
                first + 1,
                second + 1
            );
        }
    }

    let sorted = |lut: &[u8; LUT_LEN]| {
        let mut entries = lut.to_vec();
        entries.sort_unstable();
        entries
    };
    for lut in &luts[1..] {
        assert_eq!(sorted(luts[0]), sorted(lut));
    }
}

/// Every PPU whose master palette *is* the ordered one has no remap at all, which is both the
/// 2C03's case and the brief's rule for an unidentified VS image ("the plain NES palette").
#[test]
fn the_2c03_and_the_2c05_have_no_remap() {
    for model in MODELS {
        let expected = matches!(
            model,
            VsPpuModel::Rp2c04_0001
                | VsPpuModel::Rp2c04_0002
                | VsPpuModel::Rp2c04_0003
                | VsPpuModel::Rp2c04_0004
        );
        assert_eq!(
            model.palette_lut().is_some(),
            expected,
            "{} should {}have a remap",
            model.name(),
            if expected { "" } else { "not " }
        );
    }
}

// ------------------------------------------------------------------ chosen at load

/// What entry `entry` of palette RAM holds after `$00`..`$1F` have been written in order into
/// `$3F00`..`$3F1F`.
///
/// `$3F10`, `$3F14`, `$3F18` and `$3F1C` **are** `$3F00`, `$3F04`, `$3F08` and `$3F0C`
/// (REQ-CORE-045), so the write to the mirror lands on the same cell and, coming second, is the
/// byte that stays: the four background entries and their four mirrors all hold `$10`, `$14`,
/// `$18` and `$1C`.
fn written_at(entry: usize) -> u8 {
    let entry = u8::try_from(entry).unwrap_or(0);
    if entry % 4 == 0 {
        entry | 0x10
    } else {
        entry
    }
}

/// Points `$2006` at `$3F00 | entry` and reads `$2007`, the way a game reads palette RAM back.
fn read_palette_through_2007(core: &mut NesCore, entry: u8) -> u8 {
    for byte in [0x3F, entry] {
        let (ppu, cartridge) = core.bus_mut().ppu_and_cartridge_mut();
        ppu.write(6, byte, cartridge);
    }
    let (ppu, cartridge) = core.bus_mut().ppu_and_cartridge_mut();
    ppu.read(7, cartridge) & 0x3F
}

/// Loads a synthetic VS image whose byte 13 asks for `model` and writes the 32 bytes `$00`..`$1F`
/// into palette RAM through `$2006`/`$2007`.
fn core_with_palette(model: VsPpuModel) -> NesCore {
    let mut core = NesCore::new();
    let load = core.load_rom(&image(0, 0x09, byte13_of(model), 0), "SyntheticVs.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(core.vs_system().map(VsSystem::ppu), Some(model));

    core.write_memory(0x2006, 0x3F);
    core.write_memory(0x2006, 0x00);
    for entry in 0..32u8 {
        core.write_memory(0x2007, entry);
    }
    core
}

/// A synthetic image per PPU model: the palette chosen at load is the one the frame batch
/// carries, and palette RAM still reads back exactly what the game wrote.
#[test]
fn the_palette_chosen_at_load_is_the_one_the_batch_carries() {
    for model in MODELS {
        let mut core = core_with_palette(model);
        let shown = core.ppu().snapshot_palette();

        for (entry, &actual) in shown.iter().enumerate() {
            let written = written_at(entry);
            let expected = match model.palette_lut() {
                Some(lut) => lut[usize::from(written)],
                None => written,
            };
            assert_eq!(
                actual,
                expected,
                "{}: entry {entry:#04X}, written {written:#04X}",
                model.name()
            );
        }

        // Palette RAM sits before the chip's colour decoder, so a `$2007` read gives the written
        // byte back whether there is a remap or not. `NesCore::read_memory` is the debugger's
        // side-effect-free peek and answers 0 for every register (REQ-CORE-080), so the read has
        // to go through the PPU itself.
        for entry in 0..32usize {
            assert_eq!(
                read_palette_through_2007(&mut core, u8::try_from(entry).unwrap_or(0)),
                written_at(entry),
                "{}: $3F{entry:02X} reads back what the game wrote",
                model.name()
            );
        }
    }
}

/// The same image without the VS bit: no remap anywhere. This is the brief's "a plain NES image
/// proving none of it is active", for the palette.
#[test]
fn a_plain_nes_image_has_no_remap_at_all() {
    let mut core = NesCore::new();
    // Byte 13 claims an RP2C04-0004, and byte 7 does not say Vs. System: the byte is not read.
    let load = core.load_rom(&image(0, 0x08, 0x05, 0), "SyntheticNes.nes");
    assert!(load.success, "{:?}", load.error);
    assert!(core.vs_system().is_none());

    core.write_memory(0x2006, 0x3F);
    core.write_memory(0x2006, 0x00);
    for entry in 0..32u8 {
        core.write_memory(0x2007, entry);
    }

    let shown = core.ppu().snapshot_palette();
    for (entry, &actual) in shown.iter().enumerate() {
        assert_eq!(actual, written_at(entry), "entry {entry:#04X}");
    }
}

/// The remap reaches the **pixel** image too, and for free: the picture of REQ-CORE-048 resolves
/// through the shown palette, so a backdrop written as `$01` comes out as the chip's colour for
/// `$01` with no per-pixel work at all.
#[test]
fn the_remap_reaches_the_pixel_image() {
    // Entry 1 of the page's `PaletteLUT_2C04_0002` is $27.
    let model = VsPpuModel::Rp2c04_0002;
    assert_eq!(
        model.palette_lut().expect("an RP2C04 has a remap")[0x01],
        0x27
    );

    let mut core = NesCore::new();
    let load = core.load_rom(&image(0, 0x09, byte13_of(model), 0), "SyntheticVs.nes");
    assert!(load.success, "{:?}", load.error);

    core.write_memory(0x2006, 0x3F);
    core.write_memory(0x2006, 0x00);
    core.write_memory(0x2007, 0x01);
    // Rendering off, so every visible dot is the backdrop.
    core.write_memory(0x2001, 0x00);
    core.run_frame(0);

    let mut picture = Vec::new();
    core.ppu().pixel_frame_indices_into(&mut picture);
    assert_eq!(picture.len(), 256 * 240, "the whole visible image");
    assert!(
        picture.iter().all(|&pixel| pixel == 0x27),
        "the backdrop should be the chip's colour for $01 and not $01 itself"
    );
}

// ------------------------------------------------------------------ the RC2C05

/// Loads a synthetic VS image whose byte 13 asks for `model`, with no palette written.
fn vs_core(model: VsPpuModel) -> NesCore {
    let mut core = NesCore::new();
    let load = core.load_rom(&image(0, 0x09, byte13_of(model), 0), "SyntheticVs.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(core.vs_system().map(VsSystem::ppu), Some(model));
    core
}

/// Writes PPU register `register` as the CPU would.
fn ppu_write(core: &mut NesCore, register: u16, value: u8) {
    let (ppu, cartridge) = core.bus_mut().ppu_and_cartridge_mut();
    ppu.write(register, value, cartridge);
}

/// Reads PPU register `register` as the CPU would, side effects included.
fn ppu_read(core: &mut NesCore, register: u16) -> u8 {
    let (ppu, cartridge) = core.bus_mut().ppu_and_cartridge_mut();
    ppu.read(register, cartridge)
}

/// nesdev, "PPU registers": "The 2C05 series of arcade PPUs returns an identifier in bits 4-0
/// instead of PPU open bus". So the value does not depend on what the data-bus latch holds —
/// which is the whole difference from the 2C02, whose bits 0 to 4 *are* that latch
/// (`ppu_open_bus` tests 6 and 7).
#[test]
fn an_rc2c05_drives_the_low_bits_of_2002_with_its_identifier() {
    let expected = [
        (VsPpuModel::Rc2c05_01, 0x1Bu8),
        (VsPpuModel::Rc2c05_02, 0x3D),
        (VsPpuModel::Rc2c05_03, 0x1C),
        (VsPpuModel::Rc2c05_04, 0x1B),
        (VsPpuModel::Rc2c05_05, 0x00),
    ];

    for (model, identifier) in expected {
        for latch in [0x00u8, 0xFF] {
            let mut core = vs_core(model);
            // A write to any register loads the whole bus latch (`ppu_open_bus` test 2);
            // `$2003` is the one that does nothing else of interest.
            ppu_write(&mut core, 3, latch);
            assert_eq!(
                ppu_read(&mut core, 2) & 0x3F,
                identifier,
                "{} with the latch at {latch:#04X}",
                model.name()
            );
        }
    }
}

/// Every other PPU — the 2C02, the RP2C03/RC2C03 and the four RP2C04 — leaves bits 0 to 4 of
/// `$2002` open, which is the behaviour the register always had. The brief's "a plain NES image
/// proving none of it is active", for `$2002`.
#[test]
fn every_other_ppu_leaves_the_low_bits_of_2002_open() {
    let mut nes = NesCore::new();
    let load = nes.load_rom(&image(0, 0x00, 0, 0), "SyntheticNes.nes");
    assert!(load.success, "{:?}", load.error);

    let mut cores = vec![nes];
    for model in [
        VsPpuModel::Rc2c03,
        VsPpuModel::Rp2c04_0001,
        VsPpuModel::Rp2c04_0002,
        VsPpuModel::Rp2c04_0003,
        VsPpuModel::Rp2c04_0004,
    ] {
        cores.push(vs_core(model));
    }

    for core in &mut cores {
        ppu_write(core, 3, 0xFF);
        assert_eq!(ppu_read(core, 2) & 0x1F, 0x1F, "the latch was all ones");
        ppu_write(core, 3, 0x00);
        assert_eq!(ppu_read(core, 2) & 0x1F, 0x00, "the latch was all zeroes");
    }
}

/// The three flags of bits 5 to 7 keep their places on an RC2C05, except the RC2C05-02's bit 5:
/// the page's table gives that variant a **six**-bit mask (`$3F`) with the value `$3D`, so the
/// bit the sprite-overflow flag lives in is driven to 1 for good.
#[test]
fn the_rc2c05_02_drives_bit_5_as_well() {
    let mut core = vs_core(VsPpuModel::Rc2c05_02);
    ppu_write(&mut core, 3, 0x00);
    assert_eq!(ppu_read(&mut core, 2) & 0x20, 0x20);

    // The other four leave bit 5 to the flag, which is clear on a machine that has not drawn.
    for model in [
        VsPpuModel::Rc2c05_01,
        VsPpuModel::Rc2c05_03,
        VsPpuModel::Rc2c05_04,
        VsPpuModel::Rc2c05_05,
    ] {
        let mut core = vs_core(model);
        ppu_write(&mut core, 3, 0x00);
        assert_eq!(ppu_read(&mut core, 2) & 0x20, 0x00, "{}", model.name());
    }
}

/// Paints one frame with rendering off and returns a visible pixel, which is the backdrop —
/// greyed when PPUMASK bit 0 is set (the colour is masked with `$30`).
fn backdrop_with(model: VsPpuModel, register: u16) -> u8 {
    let mut core = vs_core(model);
    // $3F00 = $21, a colour the grey mask changes: $21 & $30 is $20.
    ppu_write(&mut core, 6, 0x3F);
    ppu_write(&mut core, 6, 0x00);
    ppu_write(&mut core, 7, 0x21);
    // The register under test, with the greyscale bit of PPUMASK and nothing else.
    ppu_write(&mut core, register, 0x01);
    core.run_frame(0);

    let mut picture = Vec::new();
    core.ppu().pixel_frame_indices_into(&mut picture);
    picture[256 * 120 + 128]
}

/// nesdev, "Vs. System": the 2C05 "swaps the meanings of `$2000` and `$2001`". PPUMASK's
/// greyscale bit is the cheapest proof — on an RC2C05 it is `$2000` that greys the picture and
/// `$2001` that does not, and on every other PPU the other way round.
#[test]
fn the_rc2c05_swaps_2000_and_2001() {
    // Neither the RC2C05 nor the RP2C03 remaps the palette, so $21 stays $21.
    assert!(VsPpuModel::Rc2c05_01.palette_lut().is_none());
    assert!(VsPpuModel::Rc2c03.palette_lut().is_none());

    for model in [
        VsPpuModel::Rc2c05_01,
        VsPpuModel::Rc2c05_02,
        VsPpuModel::Rc2c05_03,
        VsPpuModel::Rc2c05_04,
        VsPpuModel::Rc2c05_05,
    ] {
        assert_eq!(
            backdrop_with(model, 0),
            0x20,
            "{}: a $2000 write should reach PPUMASK",
            model.name()
        );
        assert_eq!(
            backdrop_with(model, 1),
            0x21,
            "{}: a $2001 write should reach PPUCTRL and leave the colour alone",
            model.name()
        );
    }

    // A VS. System that is not an RC2C05 keeps the ordinary wiring.
    assert_eq!(backdrop_with(VsPpuModel::Rc2c03, 1), 0x20);
    assert_eq!(backdrop_with(VsPpuModel::Rc2c03, 0), 0x21);
}

/// And a plain NES image keeps it too: the brief's "a plain NES image proving none of it is
/// active", for the register swap.
#[test]
fn a_plain_nes_image_does_not_swap_2000_and_2001() {
    let mut core = NesCore::new();
    let load = core.load_rom(&image(0, 0x00, 0, 0), "SyntheticNes.nes");
    assert!(load.success, "{:?}", load.error);
    assert!(core.vs_system().is_none());

    ppu_write(&mut core, 6, 0x3F);
    ppu_write(&mut core, 6, 0x00);
    ppu_write(&mut core, 7, 0x21);
    ppu_write(&mut core, 1, 0x01);
    core.run_frame(0);

    let mut picture = Vec::new();
    core.ppu().pixel_frame_indices_into(&mut picture);
    assert_eq!(picture[256 * 120 + 128], 0x20, "$2001 greys a NES picture");
}

// ------------------------------------------------------------------ mapper 99 and mapper 151

/// A NES 2.0 VS image of `mapper` with `prg_8k` 8 KiB PRG banks and `chr_8k` 8 KiB CHR banks,
/// each PRG bank filled with `0x10 + index` and each CHR bank with `0xA0 + index`.
///
/// 40 KiB of PRG is not a whole number of the 16 KiB units byte 4 counts, so that size is written
/// in NES 2.0's exponent-multiplier form (byte 9's nibble `$F`): `5 << 13`, which is the shape
/// *Vs. Gumshoe*'s own header has.
fn vs_board_image(mapper: u8, prg_8k: usize, chr_8k: usize) -> Vec<u8> {
    let prg = prg_8k * 8 * 1024;
    let chr = chr_8k * 8 * 1024;
    let mut image = vec![0u8; 16 + prg + chr];
    image[..4].copy_from_slice(b"NES\x1A");
    image[6] = (mapper & 0x0F) << 4;
    // NES 2.0, console type 1, so that the image is a VS. System however it is detected.
    image[7] = (mapper & 0xF0) | 0x09;

    if prg % (16 * 1024) == 0 {
        image[4] = u8::try_from(prg / (16 * 1024)).expect("a small fixture");
    } else {
        assert_eq!(
            prg,
            40 * 1024,
            "only the 40 KiB layout needs the other form"
        );
        image[4] = (13 << 2) | 2; // 5 << 13 = 40960.
        image[9] = 0x0F;
    }
    image[5] = u8::try_from(chr / (8 * 1024)).expect("a small fixture");

    for bank in 0..prg_8k {
        let start = 16 + bank * 8 * 1024;
        image[start..start + 8 * 1024].fill(0x10 + u8::try_from(bank).unwrap_or(0));
    }
    for bank in 0..chr_8k {
        let start = 16 + prg + bank * 8 * 1024;
        image[start..start + 8 * 1024].fill(0xA0 + u8::try_from(bank).unwrap_or(0));
    }
    image
}

/// The byte the PPU fetches at `$0000`, which is the 8 KiB CHR bank the board selects.
fn chr_bank_byte(core: &NesCore) -> u8 {
    core.cartridge()
        .expect("a ROM is loaded")
        .ppu_peek(0x0000, ChrFetch::Background)
}

/// nesdev, "INES Mapper 099" and "Vs. System": the board's one register is **bit 2 of a `$4016`
/// write**, and it selects the 8 KiB CHR bank.
#[test]
fn mapper_99_selects_its_chr_bank_through_4016_bit_2() {
    let mut core = NesCore::new();
    let load = core.load_rom(&vs_board_image(99, 4, 2), "Mapper99.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(core.cartridge().expect("loaded").mapper_number(), 99);
    assert!(core.vs_system().is_some());

    assert_eq!(chr_bank_byte(&core), 0xA0, "bank 0 at power-on");

    core.write_memory(0x4016, 0x04);
    assert_eq!(chr_bank_byte(&core), 0xA1, "bit 2 selects bank 1");

    core.write_memory(0x4016, 0x00);
    assert_eq!(chr_bank_byte(&core), 0xA0, "and clearing it goes back");

    // The two low bits of that write are the controller strobe and the `DualSystem` line, which
    // is why the page says a game keeps its own copy of the value: neither moves the bank.
    for value in [0x01u8, 0x02, 0x03] {
        core.write_memory(0x4016, value);
        assert_eq!(
            chr_bank_byte(&core),
            0xA0,
            "{value:#04X} is not the select bit"
        );
    }
    for value in [0x05u8, 0x06, 0x07, 0xFF] {
        core.write_memory(0x4016, value);
        assert_eq!(
            chr_bank_byte(&core),
            0xA1,
            "{value:#04X} carries the select bit"
        );
    }
}

/// The page's 40 KiB variant: "Only *Vs. Gumshoe* uses the 40 KiB PRG configuration … the 8 KiB
/// banks are sequenced as 0, 1, 2, 3, 0 alternate, empty", so the same bit additionally swaps the
/// bank at `$8000-$9FFF` — and `$A000-$FFFF` is the fixed 24 KiB whatever it says.
#[test]
fn mapper_99_switches_the_8k_prg_bank_only_on_the_40k_layout() {
    let mut core = NesCore::new();
    let load = core.load_rom(&vs_board_image(99, 5, 2), "Mapper99Gumshoe.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(
        core.cartridge().expect("loaded").rom().prg_rom().len(),
        40960
    );

    let windows = |core: &NesCore| {
        [0x8000u16, 0xA000, 0xC000, 0xE000].map(|address| core.read_memory(address))
    };

    assert_eq!(windows(&core), [0x10, 0x11, 0x12, 0x13], "banks 0, 1, 2, 3");
    core.write_memory(0x4016, 0x04);
    assert_eq!(
        windows(&core),
        [0x14, 0x11, 0x12, 0x13],
        "the alternate bank 4 at $8000, the fixed 24 KiB untouched"
    );
    core.write_memory(0x4016, 0x00);
    assert_eq!(windows(&core), [0x10, 0x11, 0x12, 0x13]);

    // A 32 KiB image has no fifth bank for the bit to reach, and its PRG never moves.
    let mut core = NesCore::new();
    let load = core.load_rom(&vs_board_image(99, 4, 2), "Mapper99.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(windows(&core), [0x10, 0x11, 0x12, 0x13]);
    core.write_memory(0x4016, 0x04);
    assert_eq!(
        windows(&core),
        [0x10, 0x11, 0x12, 0x13],
        "32 KiB: PRG is fixed"
    );
    // The CHR bank still moves, the two halves of the register being one bit.
    assert_eq!(chr_bank_byte(&core), 0xA1);
}

/// The board's 2 KiB of work RAM, not the iNES default of 8 KiB ("CPU `$6000-$7FFF`: 2 KiB RAM"),
/// mirrored four times across the window because the board decodes no more address lines.
#[test]
fn mapper_99_has_two_kibibytes_of_work_ram_mirrored() {
    let mut core = NesCore::new();
    let load = core.load_rom(&vs_board_image(99, 4, 2), "Mapper99.nes");
    assert!(load.success, "{:?}", load.error);

    core.write_memory(0x6000, 0x5A);
    assert_eq!(core.read_memory(0x6000), 0x5A);
    for mirror in [0x6800u16, 0x7000, 0x7800] {
        assert_eq!(
            core.read_memory(mirror),
            0x5A,
            "{mirror:#06X} mirrors $6000"
        );
    }

    core.write_memory(0x77FF, 0xA5);
    assert_eq!(core.read_memory(0x67FF), 0xA5);
}

/// The select bit is in the savestate, so a restore puts the banks back (REQ-CORE-090).
#[test]
fn the_select_bit_is_in_the_savestate() {
    let mut core = NesCore::new();
    let load = core.load_rom(&vs_board_image(99, 5, 2), "Mapper99Gumshoe.nes");
    assert!(load.success, "{:?}", load.error);

    core.write_memory(0x4016, 0x04);
    let state = core.save_state(false).expect("a ROM is loaded");
    core.write_memory(0x4016, 0x00);
    assert_eq!(chr_bank_byte(&core), 0xA0);
    assert_eq!(core.read_memory(0x8000), 0x10);

    core.try_load_state(&state, 0)
        .expect("the state is this machine's");
    assert_eq!(chr_bank_byte(&core), 0xA1);
    assert_eq!(core.read_memory(0x8000), 0x14);
}

/// Mapper 151 is the VRC1 on a VS board: the same chip, the same registers, a different number.
#[test]
fn mapper_151_is_the_vrc1_and_a_vs_system() {
    let mut core = NesCore::new();
    let load = core.load_rom(&vs_board_image(151, 8, 8), "Mapper151.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(core.cartridge().expect("loaded").mapper_number(), 151);
    assert!(core.vs_system().is_some());

    // The VRC1's $8000 register: bits 3-0 are the 8 KiB PRG bank at $8000.
    core.write_memory(0x8000, 0x05);
    assert_eq!(core.read_memory(0x8000), 0x15);
    // $E000 is the low four bits of the 4 KiB CHR bank at PPU $0000; bank 2 of 4 KiB is the
    // second half of the 8 KiB bank 1.
    core.write_memory(0xE000, 0x02);
    assert_eq!(chr_bank_byte(&core), 0xA1);

    // And mapper 75, the same board off a VS board, is not a VS. System.
    let mut core = NesCore::new();
    let load = core.load_rom(&vs_board_image(75, 8, 8), "Mapper75.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(core.cartridge().expect("loaded").mapper_number(), 75);
    // The fixture's header says console type 1, so it *is* detected as a VS image — which is the
    // point: the number is not what makes 75 a VS board, the header is.
    assert!(core.vs_system().is_some());

    let mut core = NesCore::new();
    let load = core.load_rom(&image(75, 0x00, 0, 0), "Mapper75Nes.nes");
    assert!(load.success, "{:?}", load.error);
    assert!(
        core.vs_system().is_none(),
        "mapper 75 alone is not a VS image"
    );
}

/// A `$4016` write reaches the cartridge **only** on the one board whose register it is: the
/// brief's "a plain NES image proving none of it is active", for the board.
#[test]
fn a_4016_write_leaves_every_other_board_alone() {
    // A UxROM VS image — the shape *Vs. Castlevania* has — whose PRG bank is its own register.
    let mut core = NesCore::new();
    let load = core.load_rom(&vs_board_image(2, 8, 1), "Mapper2Vs.nes");
    assert!(load.success, "{:?}", load.error);
    assert!(
        core.vs_system().is_some(),
        "console type 1 is in the fixture"
    );

    let at_power_on = core.read_memory(0x8000);
    // The UxROM register banks 16 KiB at a time, so bank 3 is the pair of 8 KiB banks 6 and 7.
    core.write_memory(0x8000, 0x03);
    let before = core.read_memory(0x8000);
    assert_eq!(before, 0x16, "the UxROM register took its 16 KiB bank 3");
    assert_ne!(before, at_power_on);
    for value in [0x00u8, 0x04, 0xFF] {
        core.write_memory(0x4016, value);
        assert_eq!(
            core.read_memory(0x8000),
            before,
            "{value:#04X} at $4016 must not reach a UxROM"
        );
    }

    // And a plain NES image: its CHR and PRG are untouched by any $4016 write.
    let mut core = NesCore::new();
    let load = core.load_rom(&image(0, 0x00, 0, 0), "SyntheticNes.nes");
    assert!(load.success, "{:?}", load.error);
    let chr = chr_bank_byte(&core);
    for value in [0x00u8, 0x04, 0xFF] {
        core.write_memory(0x4016, value);
        assert_eq!(chr_bank_byte(&core), chr);
    }
}

// ------------------------------------------------------------------ the cabinet

/// Reads `$4016` (port 0) or `$4017` (port 1) with its side effects, the serial bit masked off:
/// what is left is what the cabinet drives.
fn cabinet_bits(core: &mut NesCore, port: usize) -> u8 {
    core.bus_mut().read_input_port(port, 0xFF) & 0xFE
}

/// nesdev, "Vs. System": `$4016` carries the service button in bit 2 and DIP switches 1 and 2 in
/// bits 3 and 4; `$4017` carries switches 3 to 8 in bits 2 to 7. Bit 1 is always 0 and `$4016`
/// bit 7 is the CPU identifier, 0 on the primary.
#[test]
fn the_eight_dip_switches_are_readable_at_4016_and_4017() {
    let mut core = vs_core(VsPpuModel::Rc2c03);

    // All off: nothing but the serial bit, which `cabinet_bits` masks away.
    assert_eq!(cabinet_bits(&mut core, 0), 0x00);
    assert_eq!(cabinet_bits(&mut core, 1), 0x00);

    let expected = [
        (1usize, 0, 0x08u8),
        (2, 0, 0x10),
        (3, 1, 0x04),
        (4, 1, 0x08),
        (5, 1, 0x10),
        (6, 1, 0x20),
        (7, 1, 0x40),
        (8, 1, 0x80),
    ];

    for (switch, port, bit) in expected {
        let vs = core.vs_system_mut().expect("a VS image");
        vs.set_dip_switches(0);
        vs.set_dip_switch(switch, true);
        assert!(core.vs_system().expect("a VS image").dip_switch(switch));

        assert_eq!(
            cabinet_bits(&mut core, port),
            bit,
            "DIP switch {switch} should be bit {bit:#04X} of port {port}"
        );
        // And it is on that port only.
        assert_eq!(
            cabinet_bits(&mut core, 1 - port),
            0x00,
            "DIP switch {switch}"
        );
    }

    // All eight at once: both halves of the byte, and bit 1 still 0 on either port.
    core.vs_system_mut()
        .expect("a VS image")
        .set_dip_switches(0xFF);
    assert_eq!(cabinet_bits(&mut core, 0), 0x18);
    assert_eq!(cabinet_bits(&mut core, 1), 0xFC);

    // Bit 7 of $4016 is the CPU identifier and this build is always the primary.
    assert_eq!(cabinet_bits(&mut core, 0) & 0x80, 0x00);
}

/// A coin in slot 1 shows in `$4016` bit 5 and one in slot 2 in bit 6, and the line is released
/// after [`VsSystem::COIN_FRAMES`] frames.
#[test]
fn a_coin_is_visible_at_4016_and_then_released() {
    for (slot, bit) in [(1usize, 0x20u8), (2, 0x40)] {
        let mut core = vs_core(VsPpuModel::Rc2c03);
        assert_eq!(cabinet_bits(&mut core, 0) & 0x60, 0x00);

        core.vs_system_mut().expect("a VS image").insert_coin(slot);
        assert!(core.vs_system().expect("a VS image").coin(slot));
        assert_eq!(cabinet_bits(&mut core, 0) & 0x60, bit, "slot {slot}");
        // The other slot stays open, and $4017 never carries a coin.
        assert_eq!(cabinet_bits(&mut core, 1) & 0x60, 0x00);

        // The line is held for four frames and released on the fifth.
        for frame in 0..u32::from(VsSystem::COIN_FRAMES) {
            assert_eq!(
                cabinet_bits(&mut core, 0) & 0x60,
                bit,
                "slot {slot} on frame {frame}"
            );
            core.run_frame(0);
        }

        assert_eq!(
            cabinet_bits(&mut core, 0) & 0x60,
            0x00,
            "slot {slot} released"
        );
    }
}

/// A line a host holds itself is never counted down, which is what [`VsSystem::set_coin`] is for.
#[test]
fn a_held_coin_line_stays_held() {
    let mut core = vs_core(VsPpuModel::Rc2c03);
    core.vs_system_mut().expect("a VS image").set_coin(1, true);
    for _ in 0..32 {
        core.run_frame(0);
        assert_eq!(cabinet_bits(&mut core, 0) & 0x20, 0x20);
    }

    core.vs_system_mut().expect("a VS image").set_coin(1, false);
    assert_eq!(cabinet_bits(&mut core, 0) & 0x20, 0x00);
}

/// The service button is `$4016` bit 2 and is held for as long as the host holds it.
#[test]
fn the_service_button_is_4016_bit_2() {
    let mut core = vs_core(VsPpuModel::Rc2c03);
    assert_eq!(cabinet_bits(&mut core, 0) & 0x04, 0x00);

    core.vs_system_mut()
        .expect("a VS image")
        .set_service_button(true);
    assert_eq!(cabinet_bits(&mut core, 0) & 0x04, 0x04);
    core.run_frame(0);
    assert_eq!(cabinet_bits(&mut core, 0) & 0x04, 0x04, "it is not a pulse");
    assert_eq!(cabinet_bits(&mut core, 1) & 0x04, 0x00, "nor on $4017");

    core.vs_system_mut()
        .expect("a VS image")
        .set_service_button(false);
    assert_eq!(cabinet_bits(&mut core, 0) & 0x04, 0x00);
}

/// A plain NES image keeps the stale bus contents in bits 5 to 7 of a `$4016` read (nesdev), so
/// nothing of the cabinet is active: the brief's "a plain NES image proving none of it is
/// active", for the inputs.
#[test]
fn a_plain_nes_image_still_reads_open_bus_above_the_serial_bit() {
    let mut core = NesCore::new();
    let load = core.load_rom(&image(0, 0x00, 0, 0), "SyntheticNes.nes");
    assert!(load.success, "{:?}", load.error);
    assert!(core.vs_system().is_none());

    for port in 0..2 {
        assert_eq!(
            core.bus_mut().read_input_port(port, 0xE0) & 0xFE,
            0xE0,
            "port {port} shows the stale bus"
        );
        assert_eq!(core.bus_mut().read_input_port(port, 0x00) & 0xFE, 0x00);
    }
}

/// The word of a frame, as the two ports see it after the cabinet's wiring.
fn wired_word(wiring: VsWiring, host_word: u16) -> u16 {
    // Byte 15 `$04` is "1P via $4016" and `$05` "1P via $4017".
    let byte15 = match wiring {
        VsWiring::PlayerOneOnPort0 => 0x04,
        VsWiring::PlayerOneOnPort1 => 0x05,
        VsWiring::Zapper => 0x07,
    };
    let mut core = NesCore::new();
    let load = core.load_rom(&image(0, 0x09, 0, byte15), "SyntheticVs.nes");
    assert!(load.success, "{:?}", load.error);
    assert_eq!(core.vs_system().expect("a VS image").wiring(), wiring);

    core.bus_mut().input_mut().begin_frame(host_word);
    core.bus_mut().input().input_word()
}

/// nesdev, "Vs. System": `$4016` shifts out "A, B, 1, 3, Up, Down, Left, Right", so what a NES
/// controller calls **Select** is the start button of that stick's player. The host's Start
/// therefore has to land in the Select slot.
#[test]
fn the_cabinet_puts_the_start_button_where_select_would_be() {
    // Player 1 presses Start (bit 3), player 2 presses Select (bit 2).
    let word = wired_word(VsWiring::PlayerOneOnPort0, 0x0408);
    assert_eq!(
        word, 0x0804,
        "player one's Start becomes button 1 (bit 2) and player two's Select button 4 (bit 3)"
    );

    // The six stick bits are untouched: A, B and the four directions.
    let word = wired_word(VsWiring::PlayerOneOnPort0, 0xF3F3);
    assert_eq!(word, 0xF3F3);
}

/// And which stick is player one's is the game's: for byte 15 `$05` the **sticks** swap while the
/// start buttons stay with their own port, "1" being wired to `$4016` whichever stick the game
/// reads first.
#[test]
fn byte_15_05_swaps_the_two_sticks_and_not_the_start_buttons() {
    // Player 1 holds Right (bit 7) and Start (bit 3); player 2 holds A (bit 0).
    let host = u16::from_le_bytes([0x88, 0x01]);

    let plain = wired_word(VsWiring::PlayerOneOnPort0, host);
    assert_eq!(
        plain.to_le_bytes(),
        [0x84, 0x01],
        "port 0 keeps player one's Right, with Start moved into the Select slot"
    );

    let swapped = wired_word(VsWiring::PlayerOneOnPort1, host);
    assert_eq!(
        swapped.to_le_bytes(),
        [0x05, 0x80],
        "the sticks swapped — player two's A on port 0, player one's Right on port 1 — and \
         player one's start button still on port 0"
    );

    // A VS. Zapper game takes the unswapped wiring, its light gun being a recorded gap.
    assert_eq!(wired_word(VsWiring::Zapper, host), plain);
}

/// The cabinet is in the savestate, appended only for a machine that has one (REQ-CORE-090,
/// REQ-CORE-091).
#[test]
fn the_cabinet_survives_a_save_and_restore() {
    let mut core = vs_core(VsPpuModel::Rc2c03);
    {
        let vs = core.vs_system_mut().expect("a VS image");
        vs.set_dip_switches(0xA5);
        vs.set_service_button(true);
        vs.insert_coin(2);
    }
    let state = core.save_state(false).expect("a ROM is loaded");

    {
        let vs = core.vs_system_mut().expect("a VS image");
        vs.set_dip_switches(0x00);
        vs.set_service_button(false);
        vs.set_coin(2, false);
    }
    assert_eq!(cabinet_bits(&mut core, 1), 0x00);

    core.try_load_state(&state, 0)
        .expect("the state is this machine's");
    let vs = core.vs_system().expect("a VS image");
    assert_eq!(vs.dip_switches(), 0xA5);
    assert!(vs.service_button());
    assert!(vs.coin(2));
    assert!(!vs.coin(1));
    assert_eq!(cabinet_bits(&mut core, 1), 0xA4, "switches 3 to 8 of $A5");
}

/// A NES image's savestate is byte for byte the block it was before the lot, which is what
/// "appended only for a machine that has a cabinet" buys (REQ-CORE-091).
#[test]
fn a_nes_savestate_carries_no_cabinet_bytes() {
    // **The two names are the same length on purpose**: a savestate carries the ROM name
    // (REQ-CORE-014), so a shorter one would hide a byte of the difference.
    let mut nes = NesCore::new();
    let load = nes.load_rom(&image(0, 0x00, 0, 0), "Synthetic0.nes");
    assert!(load.success, "{:?}", load.error);
    let nes_state = nes.save_state(false).expect("a ROM is loaded").len();

    let mut vs = NesCore::new();
    // The same image but for byte 7's VS bit, so the two blocks differ only by the cabinet.
    let load = vs.load_rom(&image(0, 0x01, 0, 0), "Synthetic1.nes");
    assert!(load.success, "{:?}", load.error);
    let vs_state = vs.save_state(false).expect("a ROM is loaded").len();

    assert_eq!(
        vs_state - nes_state,
        4,
        "the cabinet is one DIP byte, two coin counters and the service button"
    );
}

/// Out of range asks are ignored rather than panicking: a host's UI counts its switches from 1.
#[test]
fn the_cabinet_ignores_numbers_outside_its_own() {
    let mut core = vs_core(VsPpuModel::Rc2c03);
    let vs = core.vs_system_mut().expect("a VS image");

    vs.set_dip_switch(0, true);
    vs.set_dip_switch(9, true);
    assert_eq!(vs.dip_switches(), 0x00);
    assert!(!vs.dip_switch(0));
    assert!(!vs.dip_switch(9));

    vs.insert_coin(0);
    vs.insert_coin(3);
    assert!(!vs.coin(0));
    assert!(!vs.coin(3));
    assert_eq!(vs.port_bits(0) & 0x60, 0x00);
}

/// The savestate carries palette RAM as the game wrote it — the format is unchanged — and a
/// restore puts the shown palette back through the remap (REQ-CORE-090, REQ-CORE-091).
#[test]
fn a_savestate_restores_the_shown_palette_through_the_remap() {
    let mut core = core_with_palette(VsPpuModel::Rp2c04_0003);
    let shown = core.ppu().snapshot_palette();
    let state = core.save_state(false).expect("a ROM is loaded");

    core.write_memory(0x2006, 0x3F);
    core.write_memory(0x2006, 0x00);
    for _ in 0..32 {
        core.write_memory(0x2007, 0x30);
    }
    assert_ne!(core.ppu().snapshot_palette(), shown);

    core.try_load_state(&state, 0)
        .expect("the state is this machine's");
    assert_eq!(core.ppu().snapshot_palette(), shown);
}
