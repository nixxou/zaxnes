//! The cartridge contract and NROM (REQ-CORE-012, REQ-CORE-033, REQ-CORE-100,
//! REQ-CORE-102).

// Test indices and bytes are small: the narrowing casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

mod common;

use common::{machine_with, minimal, minimal_rom};
use zax_nes::cartridges::{Cartridge, ChrFetch};
use zax_nes::rom::RomImage;
use zax_nes::NametableMirroring;

#[test]
fn req_core_100_mapper_0_is_implemented() {
    assert!(Cartridge::is_supported(0));
    let rom = RomImage::try_load(&minimal(), "Minimal.nes").unwrap();
    let cartridge = Cartridge::create(rom).unwrap();
    assert_eq!(0, cartridge.mapper_number());
    assert_eq!(NametableMirroring::Vertical, cartridge.mirroring());
}

#[test]
fn req_core_102_an_unimplemented_mapper_is_refused_by_name() {
    // Everything the registry does not implement is refused with the message of REQ-CORE-011 —
    // which is everything outside the REQ-CORE-100 table except the numbers REQ-CORE-102 lets
    // this build add; see `req_core_100_every_required_mapper_is_registered` for that list.
    //
    // 19 stood here until lot MF-5a step 4 implemented the Namco 163, and 177 took its place:
    // a number in the same decade that no lot has claimed, so the test still proves that an
    // unimplemented two-digit mapper is refused and not merely that a wild one is.
    //
    // **68 had to move for the same reason in MF-6c step 4**, which implemented the Sunsoft-4,
    // and that is the second time this list has been edited by a lot that implemented one of its
    // numbers — which is precisely the fragility **Q-130 question 2** describes: "the test that
    // embodies the criterion sweeps 6, 116, 177 and 200, and all four are real boards Mesen
    // builds, so the test holds no number guaranteed never to become supported".
    //
    // Its replacement is the first number in this project with that guarantee written down.
    // §3.5 of `docs/review/MF-6-inventory.md` lists the numbers Mesen 2 has a `case` for and
    // `break`s out of, with "They are **not** candidates for any sub-lot", and §6 item 2 names
    // **186** among them: the Study Box, "which Mesen loads as a BIOS and not as a ROM". A number
    // that is not a cartridge board at all cannot be implemented by a later mapper lot.
    for mapper in [6, 177, 186, 200] {
        assert!(!Cartridge::is_supported(mapper));
        let mut image = minimal();
        image[6] = (image[6] & 0x0F) | (((mapper & 0x0F) as u8) << 4);
        image[7] = ((mapper >> 4) as u8) << 4;
        let rom = RomImage::try_load(&image, "X.nes").unwrap();
        assert_eq!(mapper, rom.mapper());

        let error = Cartridge::create(rom).unwrap_err();
        assert!(error.message.contains(&mapper.to_string()));
    }
}

#[test]
fn req_core_100_nrom_128_mirrors_its_16_kib_of_prg_at_8000_and_c000() {
    // One 16 KiB PRG bank: $8000-$BFFF and $C000-$FFFF show the same bytes.
    let mut image = vec![0u8; 16 + (16 * 1024) + (8 * 1024)];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 0x01; // 1 x 16 KiB PRG.
    image[5] = 0x01; // 1 x 8 KiB CHR.
    image[6] = 0x01;
    image[16] = 0x4C; // A marker at the start of the bank.
    image[16 + (16 * 1024) - 1] = 0x5A; // And one at its end.

    let core = machine_with(&image, "Nrom128.nes");
    assert_eq!(0x4C, core.read_memory(0x8000));
    assert_eq!(0x4C, core.read_memory(0xC000));
    assert_eq!(0x5A, core.read_memory(0xBFFF));
    assert_eq!(0x5A, core.read_memory(0xFFFF));
}

#[test]
fn req_core_100_nrom_256_maps_32_kib_linearly() {
    let mut image = minimal();
    image[16] = 0x11; // $8000.
    image[16 + (16 * 1024)] = 0x22; // $C000.

    let core = machine_with(&image, "Nrom256.nes");
    assert_eq!(0x11, core.read_memory(0x8000));
    assert_eq!(0x22, core.read_memory(0xC000));
}

#[test]
fn req_core_012_chr_rom_is_readable_and_not_writable() {
    let mut core = machine_with(&minimal(), "Chr.nes");
    let cartridge = core.cartridge_mut().unwrap();

    // Tile 3 of table $0000: plane 0 is eight bytes of 3, plane 1 eight of 252.
    assert_eq!(0x03, cartridge.ppu_peek(3 * 16, ChrFetch::Background));
    assert_eq!(252, cartridge.ppu_peek((3 * 16) + 8, ChrFetch::Background));

    cartridge.ppu_write(3 * 16, 0xFF);
    assert_eq!(0x03, cartridge.ppu_peek(3 * 16, ChrFetch::Background));
}

#[test]
fn req_core_012_chr_ram_is_writable() {
    let minimal = minimal();
    let mut image = minimal[..16 + (32 * 1024)].to_vec();
    image[5] = 0x00; // No CHR-ROM: 32 KiB of CHR-RAM.

    let mut core = machine_with(&image, "ChrRam.nes");
    let cartridge = core.cartridge_mut().unwrap();
    cartridge.ppu_write(0x0123, 0xA5);
    assert_eq!(0xA5, cartridge.ppu_peek(0x0123, ChrFetch::Background));
}

#[test]
fn req_core_033_ppu_peek_never_touches_the_mapper_state() {
    // NROM has no latch or IRQ counter, so this only pins the contract the other mappers
    // must honour: ppu_peek is what the tile emitter uses and it must be inert.
    let core = machine_with(&minimal(), "Chr.nes");
    let cartridge = core.cartridge().unwrap();
    let before = cartridge.clone();
    for address in 0..0x2000u16 {
        let _ = cartridge.ppu_peek(address, ChrFetch::Background);
    }

    assert_eq!(&before, cartridge);
    assert!(!cartridge.irq_asserted());
}

#[test]
fn req_core_038_nametable_mirroring_pairs_the_right_tables() {
    for (mirroring, first, second) in [
        (NametableMirroring::Horizontal, 0x2000, 0x2400),
        (NametableMirroring::Horizontal, 0x2800, 0x2C00),
        (NametableMirroring::Vertical, 0x2000, 0x2800),
        (NametableMirroring::Vertical, 0x2400, 0x2C00),
    ] {
        let header_byte6 = u8::from(mirroring == NametableMirroring::Vertical);
        let core = machine_with(&minimal_rom(header_byte6, None, None), "Mirror.nes");
        assert_eq!(mirroring, core.mirroring());
        assert_eq!(core.map_nametable(first), core.map_nametable(second));

        // And the other pair must land somewhere else.
        let other = if mirroring == NametableMirroring::Vertical {
            0x2400
        } else {
            0x2800
        };
        if first != other {
            assert_ne!(core.map_nametable(first), core.map_nametable(other));
        }
    }
}

#[test]
fn req_core_012_four_screen_gives_four_distinct_nametables() {
    let core = machine_with(&minimal_rom(0x08, None, None), "FourScreen.nes");
    assert_eq!(NametableMirroring::FourScreen, core.mirroring());

    let offsets = [0x2000, 0x2400, 0x2800, 0x2C00].map(|address| core.map_nametable(address));
    assert_eq!([0, 1024, 2048, 3072], offsets);
}
