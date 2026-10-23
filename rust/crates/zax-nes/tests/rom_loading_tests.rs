//! ROM loading (REQ-CORE-010 to REQ-CORE-014) and the header vectors of SPEC-02 §5.2.
//!
//! `REQ_CORE_102_only_implemented_mappers_are_accepted` checks the full mapper table and
//! belongs with the mappers (T04); here the gate is checked through mapper 200.

// Test indices and bytes are small: the narrowing casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

mod common;

use common::{from_header_vector, minimal, minimal_rom};
use zax_nes::rom::{InesHeader, RomLoadFailure};
use zax_nes::{NametableMirroring, NesCore, NesRegion};

#[test]
fn req_core_010_minimal_test_rom_loads() {
    let mut core = NesCore::new();
    let result = core.load_rom(&minimal(), "Minimal.nes");
    assert!(result.success, "{:?}", result.error);
    assert_eq!(None, result.error);
    assert!(core.is_running());
}

#[test]
fn req_core_010_equivalent_nes20_header_loads() {
    let mut image = minimal();

    // Same cartridge announced as NES 2.0: byte 7 bits 2 and 3 become %10, and the size
    // MSB nibbles of byte 9 stay zero.
    image[7] = 0x08;
    let mut core = NesCore::new();
    let result = core.load_rom(&image, "Minimal20.nes");
    assert!(result.success, "{:?}", result.error);
    assert_eq!(NesRegion::Ntsc, result.region);
}

#[test]
fn req_core_011_missing_signature_is_rejected() {
    let mut core = NesCore::new();
    let result = core.load_rom(&[0; 16], "Zeroes.nes");
    assert!(!result.success);
    assert_eq!(RomLoadFailure::CorruptHeader, result.failure);
    assert!(result.error.unwrap().to_lowercase().contains("signature"));
    assert!(!core.is_running());
}

#[test]
fn req_core_011_truncated_image_is_rejected() {
    let image = minimal();
    let mut core = NesCore::new();
    let result = core.load_rom(&image[..image.len() - 1], "Truncated.nes");
    assert!(!result.success);
    assert_eq!(RomLoadFailure::TruncatedFile, result.failure);
    assert!(result.error.unwrap().to_lowercase().contains("truncated"));
    assert!(!core.is_running());
}

#[test]
fn req_core_011_unsupported_mapper_is_rejected_with_its_number() {
    // Mapper 200 = low nibble 8 in byte 6, high nibble C in byte 7.
    let mut image = minimal();
    image[6] = 0x80;
    image[7] = 0xC0;
    let mut core = NesCore::new();
    let result = core.load_rom(&image, "Mapper200.nes");
    assert!(!result.success);
    assert_eq!(RomLoadFailure::UnsupportedMapper, result.failure);
    assert!(result.error.unwrap().contains("200"));
    assert!(!core.is_running());
}

#[test]
fn req_core_011_header_shorter_than_16_bytes_is_rejected() {
    let mut core = NesCore::new();
    let result = core.load_rom(&[0x4E, 0x45, 0x53, 0x1A], "Short.nes");
    assert!(!result.success);
    assert!(result.error.is_some());
}

#[test]
fn req_core_041_random_bytes_never_throw() {
    // A corrupt file must never crash the application (REQ-ACC-041).
    let mut state: u64 = 1234;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as u32
    };
    let mut core = NesCore::new();
    for attempt in 0..400 {
        let length = (next() % 300) as usize;
        let mut image: Vec<u8> = (0..length).map(|_| next() as u8).collect();
        if attempt % 2 == 1 && image.len() >= 4 {
            // Half the attempts carry the signature so that the header parser is reached.
            image[..4].copy_from_slice(b"NES\x1A");
        }
        let result = core.load_rom(&image, "Fuzz.nes");
        assert!(result.success || result.error.is_some());
    }
}

// ------------------------------------------------------- SPEC-02 §5.2 vectors

#[allow(clippy::unwrap_used)] // Test helper: a vector that fails to parse fails the test.
fn parse(bytes4_to_15: [u8; 12]) -> InesHeader {
    InesHeader::try_parse(&from_header_vector(bytes4_to_15)).unwrap()
}

#[test]
fn req_core_012_vector1_mapper0_vertical_chr_rom_ntsc() {
    let header = parse([
        0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(0, header.mapper);
    assert_eq!(NametableMirroring::Vertical, header.mirroring);
    assert_eq!(8 * 1024, header.chr_rom_size);
    assert_eq!(0, header.chr_ram_size);
    assert_eq!(NesRegion::Ntsc, header.region);
}

#[test]
fn req_core_012_vector2_mapper1_horizontal_battery_chr_ram() {
    let header = parse([
        0x08, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(1, header.mapper);
    assert_eq!(NametableMirroring::Horizontal, header.mirroring);
    assert!(header.has_battery);
    assert_eq!(0, header.chr_rom_size);
    assert_eq!(32 * 1024, header.chr_ram_size);
    assert_eq!(128 * 1024, header.prg_rom_size);
}

#[test]
fn req_core_012_vector3_dirty_header_ignores_high_nibble_of_byte7() {
    // "DiskDude!" in bytes 7 to 15.
    let header = parse([
        0x02, 0x01, 0x41, 0x44, 0x69, 0x73, 0x6B, 0x44, 0x75, 0x64, 0x65, 0x21,
    ]);
    assert!(header.is_dirty);
    assert_eq!(4, header.mapper);
    assert_eq!(NametableMirroring::Vertical, header.mirroring);

    // Byte 9 is 's' (0x73), whose bit 0 is 1; the dirty rule still gives NTSC because bytes
    // 7 to 15 are not read for the region (REQ-CORE-013).
    assert_eq!(NesRegion::Ntsc, header.region);
}

#[test]
fn req_core_012_vector4_byte6_23_byte7_64_is_mapper2() {
    let header = parse([
        0x02, 0x00, 0x23, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(2, header.mapper);
}

#[test]
fn req_core_012_vector5_four_screen() {
    let header = parse([
        0x02, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(NametableMirroring::FourScreen, header.mirroring);
}

#[test]
fn req_core_013_vector6_nes20_byte12_is_dendy() {
    let header = parse([
        0x02, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    ]);
    assert!(header.is_nes20);
    assert_eq!(NesRegion::Dendy, header.region);
}

#[test]
fn req_core_013_vector7_ines1_byte9_bit0_is_pal() {
    let header = parse([
        0x02, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    assert!(!header.is_nes20);
    assert_eq!(NesRegion::Pal, header.region);
}

#[test]
fn req_core_013_nes20_region_bits() {
    for (byte12, expected) in [
        (0x00, NesRegion::Ntsc),
        (0x01, NesRegion::Pal),
        (0x02, NesRegion::Multi),
        (0x03, NesRegion::Dendy),
    ] {
        let header = parse([
            0x02, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, byte12, 0x00, 0x00, 0x00,
        ]);
        assert_eq!(expected, header.region);
    }
}

#[test]
fn req_core_013_detected_region_is_not_applied_by_the_core() {
    let mut image = minimal();
    image[9] = 0x01; // iNES 1.0 PAL flag.
    let mut core = NesCore::new();
    let result = core.load_rom(&image, "Pal.nes");
    assert!(result.success, "{:?}", result.error);
    assert_eq!(NesRegion::Pal, result.region);

    // The host decides; the core stays on its current region until told otherwise.
    assert_eq!(NesRegion::Ntsc, core.region());
}

#[test]
fn req_core_012_trainer_is_skipped_and_loaded_at_7000() {
    let minimal = minimal();
    let mut image = vec![0u8; minimal.len() + 512];
    image[..16].copy_from_slice(&minimal[..16]);
    image[6] |= 0x04; // Trainer present.
    for i in 0..512 {
        image[16 + i] = i as u8;
    }
    image[16 + 512..].copy_from_slice(&minimal[16..]);

    let mut core = NesCore::new();
    let result = core.load_rom(&image, "Trainer.nes");
    assert!(result.success, "{:?}", result.error);

    // The PRG must start after the trainer: $8000 still holds the JMP of §5.1.
    assert_eq!(0x4C, core.read_memory(0x8000));

    // And the trainer must have landed at $7000.
    assert_eq!(0x00, core.read_memory(0x7000));
    assert_eq!(0x7F, core.read_memory(0x707F));
}

#[test]
fn req_core_014_rom_name_is_kept() {
    let mut core = NesCore::new();
    assert!(core.load_rom(&minimal(), "Test.nes").success);
    assert_eq!(Some("Test.nes"), core.rom_name());
}

#[test]
fn req_core_012_trailing_bytes_beyond_the_announced_size_are_ignored() {
    let mut image = minimal_rom(0x01, None, None);
    image.extend_from_slice(&[0; 128]);
    let mut core = NesCore::new();
    assert!(core.load_rom(&image, "WithTitle.nes").success);
}
