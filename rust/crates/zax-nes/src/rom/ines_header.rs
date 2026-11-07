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
    /// Byte 6 itself. [`Self::mirroring`] collapses its bits 0 and 3 the way iNES does, where
    /// one board reads them as a pair: UNROM 512 uses "`%....0..0` Vertical, `%....0..1`
    /// Horizontal, `%....1..0` 1-Screen switchable, `%....1..1` 4-Screen with cartridge VRAM"
    /// (MR-30-1).
    pub byte6: u8,
    /// Byte 10 itself, zero on an iNES 1.0 header. [`Self::prg_ram_size`] collapses its two
    /// nibbles into one total with a floor, where one board needs the **non-volatile** nibble on
    /// its own: nesdev ("INES Mapper 157") gives the Datach's subcartridge EEPROM "byte 10 value
    /// 0x10" and its absence "byte 10 value 0x00", the internal 256-byte chip being excluded
    /// because it is "not part of the game cartridge". Exposed for the reason [`Self::byte6`] is
    /// (MF-6c step 3).
    pub byte10: u8,
    /// Byte 7's two low bits, the **console type** (nesdev, "NES 2.0"): 0 the NES and Famicom,
    /// 1 the Nintendo Vs. System, 2 the PlayChoice-10, 3 "extended", which puts the real type in
    /// byte 13's low nibble. On an **iNES 1.0** header the same two bits are the two older flags,
    /// bit 0 "Vs. Unisystem" and bit 1 "PlayChoice-10", which is why a VS image of that vintage is
    /// recognised by bit 0 alone (`crate::vs::VsSystem::detect`).
    ///
    /// Zero on a dirty header, whose bytes 7 to 15 are not read at all (REQ-CORE-012).
    pub console_type: u8,
    /// Byte 13 itself, zero on an iNES 1.0 or dirty header. Its meaning depends on the console
    /// type: for the Vs. System it is the Vs. PPU type in the low nibble and the Vs. hardware
    /// type in the high one, and for console type 3 the low nibble is the extended console type
    /// (nesdev, "NES 2.0"). Exposed for the reason [`Self::byte6`] is.
    pub byte13: u8,
    /// Byte 15 itself, zero on an iNES 1.0 or dirty header: the **default expansion device**
    /// (nesdev, "NES 2.0"), whose `$04`, `$05` and `$07` are the three VS. System wirings
    /// (`crate::vs::VsWiring`). Exposed for the reason [`Self::byte6`] is.
    pub byte15: u8,
    /// True when the board below came from the VS. System table and not from the header
    /// (`crate::vs::corrected_board`, lot MF-VS-fix step 2).
    ///
    /// It makes [`Self::submapper`] meaningful on an **iNES 1.0** header, which has no submapper
    /// field: the public NES 2.0 database records *Vs. Castlevania* and *Vs. Top Gun* as mapper 2
    /// **submapper 2**, "Bus conflicts present", and a corrected header carries that number from a
    /// source that has it. Nothing else changes: a header this flag is set on has been rewritten
    /// field by field already, and [`Self::is_nes20`] still says what the *file* carried.
    #[cfg(feature = "board-vs-system")]
    pub board_from_vs_table: bool,
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

    /// Whether [`Self::submapper`] is a number some source actually gave, which is the condition
    /// every submapper-dependent rule in this core is written under.
    ///
    /// It is `is_nes20` and nothing else, because an iNES 1.0 header has no such field — plus,
    /// behind `board-vs-system`, a header the VS. System table corrected
    /// ([`Self::board_from_vs_table`]).
    #[must_use]
    pub const fn has_submapper(&self) -> bool {
        #[cfg(feature = "board-vs-system")]
        {
            self.is_nes20 || self.board_from_vs_table
        }
        #[cfg(not(feature = "board-vs-system"))]
        {
            self.is_nes20
        }
    }

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

        let chr_ram = chr_ram_size(is_nes20, byte11, chr_size);
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
            byte10: if is_nes20 { image[10] } else { 0 },
            // A dirty header's bytes 7 to 15 are unread (REQ-CORE-012), so the console type and
            // the two bytes below are zero for one: an old tool's free-text comment must not be
            // able to turn an image into a Vs. System.
            console_type: if is_dirty { 0 } else { byte7 & 0x03 },
            byte13: if is_nes20 { image[13] } else { 0 },
            byte15: if is_nes20 { image[15] } else { 0 },
            mirroring,
            byte6,
            // Parsing never consults the table; `crate::vs::corrected_board` is what sets it.
            #[cfg(feature = "board-vs-system")]
            board_from_vs_table: false,
            has_battery: (byte6 & 0x02) != 0,
            has_trainer: (byte6 & 0x04) != 0,
            region,
        })
    }
}

/// CHR-RAM size: NES 2.0's byte 11 shift, or REQ-CORE-012's "no CHR-ROM means 32 KiB of
/// CHR-RAM", with that default as the floor for an image that has neither.
fn chr_ram_size(is_nes20: bool, byte11: u8, chr_rom_size: usize) -> usize {
    let announced = if is_nes20 {
        let shift = byte11 & 0x0F;
        if shift == 0 {
            0
        } else {
            64usize << shift
        }
    } else if chr_rom_size == 0 {
        InesHeader::DEFAULT_CHR_RAM_SIZE
    } else {
        0
    };

    if chr_rom_size == 0 && announced == 0 {
        InesHeader::DEFAULT_CHR_RAM_SIZE
    } else {
        announced
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
