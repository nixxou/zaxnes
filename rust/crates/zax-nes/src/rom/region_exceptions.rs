//! The ROM dumps whose header gives the wrong TV region (Rust-only, owner request): by the CRC-32
//! of their data (`builtin_patches::data_crc`), the region they were made for. It stands for the
//! header's at load (`NesCore::load_rom`, so the host's automatic TV mode starts them right; a
//! region the player chose stays) and in the hack packs' region gate (`hack_pack::RomTraits`).
//! Official dumps only: a ROM hack of a PAL game made for NTSC consoles keeps its header's.

use crate::NesRegion;

/// The dumps and their region.
const EXCEPTIONS: &[(u32, NesRegion)] = &[
    // Castlevania (Europe) (Virtual Console): the European code, PAL, no PAL bit in its header.
    (0x534F_11AE, NesRegion::Pal),
];

/// The region of the dump whose data has CRC-32 `data_crc`, when its header is known to be
/// wrong about it.
#[must_use]
pub fn region_exception(data_crc: u32) -> Option<NesRegion> {
    EXCEPTIONS
        .iter()
        .find(|(crc, _)| *crc == data_crc)
        .map(|(_, region)| *region)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_european_virtual_console_castlevania_is_pal() {
        assert_eq!(Some(NesRegion::Pal), region_exception(0x534F_11AE));
        assert_eq!(None, region_exception(0x0AC1_AA8F), "Castlevania (USA)");
    }
}
