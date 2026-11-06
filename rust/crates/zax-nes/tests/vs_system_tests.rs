#![cfg(feature = "board-vs-system")]
// A failed setup panics, which is how a test says the fixture is wrong.
#![allow(clippy::expect_used, clippy::panic)]

//! The Nintendo VS. System: detection, the per-ROM table and the load note (lot MF-VS).
//!
//! The claims under test are the nesdev wiki's: "NES 2.0" for byte 7's console type, byte 13's
//! two nibbles and byte 15's default expansion device, and "Vs. System" for the machine itself.
//! The header fields themselves are REQ-CORE-012's.

use zax_nes::rom::InesHeader;
use zax_nes::vs::{rom_table, VsHardware, VsPpuModel, VsSource, VsSystem, VsWiring};

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
