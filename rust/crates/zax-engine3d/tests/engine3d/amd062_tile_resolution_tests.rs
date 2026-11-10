//! Resolving the tiles of a "3dh" profile where the engine and the core can be seen together
//! (AMD-062 REQ-PROF-078 to REQ-PROF-081).

use zax_nes::rom::InesHeader;
use zax_profiles::hash::rom_zones::RomZones;

/// An iNES file of `prg_units` PRG banks and `chr_units` CHR banks, `byte6` and `byte7` as given.
fn ines(prg_units: u8, chr_units: u8, byte6: u8, byte7: u8, byte9: u8) -> Vec<u8> {
    let mut file = vec![0_u8; 16];
    file[..4].copy_from_slice(&[b'N', b'E', b'S', 0x1A]);
    file[4] = prg_units;
    file[5] = chr_units;
    file[6] = byte6;
    file[7] = byte7;
    file[9] = byte9;

    let header = InesHeader::try_parse(&file).unwrap();
    file.resize(header.announced_size(), 0);
    file
}

// REQ-PROF-078 point 1 reads the iNES header for itself, the profile module depending on no
// emulation core. What it reads has to be what the core reads: same offsets, same sizes, NES 2.0
// and the trainer included.
#[test]
fn the_zones_of_req_prof_078_are_the_cores_own_offsets() {
    let cases = [
        // (prg units, chr units, byte 6, byte 7, byte 9)
        (1, 1, 0x00, 0x00, 0x00),
        (2, 0, 0x01, 0x00, 0x00),
        (8, 16, 0x00, 0x00, 0x00),
        // Trainer.
        (1, 1, 0x04, 0x00, 0x00),
        (4, 2, 0x06, 0x00, 0x00),
        // NES 2.0, the high nibbles of byte 9 adding to both sizes.
        (1, 1, 0x00, 0x08, 0x00),
        (1, 1, 0x00, 0x08, 0x11),
        (0x20, 0x10, 0x04, 0x08, 0x01),
        // NES 2.0 exponent-multiplier notation, for the PRG then for the CHR.
        (0x12, 1, 0x00, 0x08, 0x0F),
        (1, 0x12, 0x00, 0x08, 0xF0),
    ];

    for (prg_units, chr_units, byte6, byte7, byte9) in cases {
        let file = ines(prg_units, chr_units, byte6, byte7, byte9);
        let header = InesHeader::try_parse(&file).unwrap();
        let zones = RomZones::of(&file).unwrap();
        let label = format!(
            "prg={prg_units} chr={chr_units} b6={byte6:#04x} b7={byte7:#04x} b9={byte9:#04x}"
        );

        assert_eq!(header.prg_rom_offset(), zones.prg.start, "{label}");
        assert_eq!(header.prg_rom_size, zones.prg.len(), "{label}");
        assert_eq!(header.chr_rom_offset(), zones.chr.start, "{label}");
        assert_eq!(header.chr_rom_size, zones.chr.len(), "{label}");
    }
}
