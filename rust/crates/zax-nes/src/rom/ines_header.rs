use crate::{NametableMirroring, NesRegion};

/// A parsed 16-byte iNES or NES 2.0 header (REQ-CORE-010, REQ-CORE-012, REQ-CORE-013).
///
/// Parsing is pure: it never consults the mapper registry and never looks at the ROM
/// payload. Rejection of unsupported mappers and of truncated files happens in
/// [`super::RomImage`] (REQ-CORE-011), so that the header vectors of SPEC-02 §5.2 can be
/// checked on their own.
// Q-001: the failure phrases of REQ-CORE-011 are read as descriptions of the cause, not as
// literal strings, and the messages are written in English like the one literal message
// SPEC-02 does impose (REQ-CORE-120).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Four independent header flags.
pub struct InesHeader {
    /// True when the header carries the NES 2.0 extensions.
    pub is_nes20: bool,
    /// True for a "dirty" iNES header: not NES 2.0 and bytes 12 to 15 are not all zero (for
    /// instance the "`DiskDude!`" signature). The high nibble of byte 7 is then ignored, and
    /// the region is NTSC without reading bytes 7 to 15 (REQ-CORE-012, REQ-CORE-013).
    pub is_dirty: bool,
    /// iNES mapper number.
    pub mapper: i32,
    /// NES 2.0 submapper number, 0 when absent.
    pub submapper: i32,
    /// PRG-ROM size in bytes.
    pub prg_rom_size: usize,
    /// CHR-ROM size in bytes; 0 when the cartridge uses CHR-RAM.
    pub chr_rom_size: usize,
    /// CHR-RAM size in bytes; 0 when the cartridge has CHR-ROM.
    pub chr_ram_size: usize,
    /// PRG-RAM size in bytes at $6000-$7FFF.
    pub prg_ram_size: usize,
    /// Initial nametable layout (REQ-CORE-012).
    pub mirroring: NametableMirroring,
    /// Bit 1 of byte 6: the PRG-RAM is battery backed (REQ-CORE-095).
    pub has_battery: bool,
    /// Bit 2 of byte 6: a 512-byte trainer follows the header.
    pub has_trainer: bool,
    /// Region announced by the header (REQ-CORE-013).
    pub region: NesRegion,
}

const SIGNATURE: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];

impl InesHeader {
    /// Size in bytes of the header itself.
    pub const SIZE: usize = 16;

    /// Size in bytes of the optional trainer (REQ-CORE-012).
    pub const TRAINER_SIZE: usize = 512;

    /// CPU address the trainer is loaded at (REQ-CORE-012).
    pub const TRAINER_LOAD_ADDRESS: u16 = 0x7000;

    /// Default CHR-RAM size when the header announces no CHR-ROM (REQ-CORE-012).
    pub const DEFAULT_CHR_RAM_SIZE: usize = 32 * 1024;

    /// PRG-RAM always mapped at $6000-$7FFF unless the mapper disables it (REQ-CORE-012).
    pub const DEFAULT_PRG_RAM_SIZE: usize = 8 * 1024;

    /// Offset of the PRG-ROM inside the ROM image, trainer accounted for.
    #[must_use]
    pub const fn prg_rom_offset(&self) -> usize {
        Self::SIZE
            + if self.has_trainer {
                Self::TRAINER_SIZE
            } else {
                0
            }
    }

    /// Offset of the CHR-ROM inside the ROM image.
    #[must_use]
    pub const fn chr_rom_offset(&self) -> usize {
        self.prg_rom_offset().saturating_add(self.prg_rom_size)
    }

    /// Total number of bytes the header announces, header and trainer included.
    #[must_use]
    pub const fn announced_size(&self) -> usize {
        self.chr_rom_offset().saturating_add(self.chr_rom_size)
    }

    /// Parses the first 16 bytes of `image`.
    ///
    /// # Errors
    ///
    /// A message when the image is shorter than a header or when the `NES<EOF>` signature
    /// is absent (REQ-CORE-011).
    pub fn try_parse(image: &[u8]) -> Result<Self, String> {
        if image.len() < Self::SIZE {
            return Err(format!(
                "Corrupt header: a ROM image needs at least {} bytes, got {}.",
                Self::SIZE,
                image.len()
            ));
        }

        if image[..4] != SIGNATURE {
            return Err("Corrupt header: missing iNES signature \"NES\\x1A\".".to_owned());
        }

        let byte4 = image[4];
        let byte5 = image[5];
        let byte6 = image[6];
        let byte7 = image[7];
        let byte8 = image[8];
        let byte9 = image[9];
        let byte11 = image[11];
        let byte12 = image[12];

        let is_nes20 = (byte7 & 0x0C) == 0x08;

        // A header is "dirty" when it is not NES 2.0 and bytes 12 to 15 carry something else
        // than zero, which old tools used as a free-text comment (REQ-CORE-012).
        let is_dirty = !is_nes20 && (image[12] | image[13] | image[14] | image[15]) != 0;

        let mapper = if byte6 == 0x23 && byte7 == 0x64 {
            // Compatibility fix of the reference for one mislabelled dump (REQ-CORE-012).
            2
        } else {
            let mut mapper = i32::from(byte6 >> 4);
            if !is_dirty {
                mapper |= i32::from(byte7 >> 4) << 4;
                if is_nes20 {
                    mapper |= i32::from(byte8 & 0x0F) << 8;
                }
            }
            mapper
        };

        let submapper = if is_nes20 { i32::from(byte8 >> 4) } else { 0 };

        let (prg_size, chr_size) = if is_nes20 {
            (
                nes20_rom_size(byte4, byte9 & 0x0F, 16 * 1024),
                nes20_rom_size(byte5, byte9 >> 4, 8 * 1024),
            )
        } else {
            (
                usize::from(byte4) * 16 * 1024,
                usize::from(byte5) * 8 * 1024,
            )
        };

        let mut chr_ram = if is_nes20 {
            let shift = byte11 & 0x0F;
            if shift == 0 {
                0
            } else {
                64usize << shift
            }
        } else if chr_size == 0 {
            // No CHR-ROM means the cartridge carries 32 KiB of CHR-RAM (REQ-CORE-012).
            Self::DEFAULT_CHR_RAM_SIZE
        } else {
            0
        };

        if chr_size == 0 && chr_ram == 0 {
            chr_ram = Self::DEFAULT_CHR_RAM_SIZE;
        }

        let prg_ram = if is_nes20 {
            nes20_prg_ram_size(image[10])
        } else {
            Self::DEFAULT_PRG_RAM_SIZE
        };

        let mirroring = if (byte6 & 0x08) != 0 {
            NametableMirroring::FourScreen
        } else if (byte6 & 0x01) != 0 {
            NametableMirroring::Vertical
        } else {
            NametableMirroring::Horizontal
        };

        // REQ-CORE-013. A dirty header is NTSC, bytes 7 to 15 unread.
        let region = if is_dirty {
            NesRegion::Ntsc
        } else if is_nes20 {
            NesRegion::from_u8(byte12 & 0x03).unwrap_or_default()
        } else if (byte9 & 0x01) != 0 {
            NesRegion::Pal
        } else {
            NesRegion::Ntsc
        };

        Ok(Self {
            is_nes20,
            is_dirty,
            mapper,
            submapper,
            prg_rom_size: prg_size,
            chr_rom_size: chr_size,
            chr_ram_size: chr_ram,
            prg_ram_size: prg_ram,
            mirroring,
            has_battery: (byte6 & 0x02) != 0,
            has_trainer: (byte6 & 0x04) != 0,
            region,
        })
    }
}

/// NES 2.0 PRG-RAM size from byte 10: volatile plus non-volatile, at least the default.
fn nes20_prg_ram_size(byte10: u8) -> usize {
    let shift = byte10 & 0x0F;
    let eeprom_shift = byte10 >> 4;
    let volatile = if shift == 0 { 0 } else { 64usize << shift };
    let non_volatile = if eeprom_shift == 0 {
        0
    } else {
        64usize << eeprom_shift
    };
    (volatile + non_volatile).max(InesHeader::DEFAULT_PRG_RAM_SIZE)
}

/// NES 2.0 ROM size: the 12-bit value `msb_nibble:low` in units of `unit`, or the
/// exponent-multiplier form when the MSB nibble is 0xF (nesdev, "NES 2.0").
fn nes20_rom_size(low: u8, msb_nibble: u8, unit: usize) -> usize {
    if msb_nibble == 0x0F {
        // C# int arithmetic: the shift wraps at 32 bits, which a file this large could
        // never satisfy anyway.
        let exponent = u32::from(low >> 2);
        let multiplier = i32::from(low & 0x03) * 2 + 1;
        let size = multiplier.wrapping_shl(exponent);
        return usize::try_from(size).unwrap_or(usize::MAX);
    }

    ((usize::from(msb_nibble) << 8) | usize::from(low)) * unit
}
