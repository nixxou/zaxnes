use super::{InesHeader, RomLoadFailure};
use crate::NesRegion;

/// Why a ROM image was refused: the typed cause and a technical message in English
/// (REQ-CORE-011, AMD-002 answer to Q-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomLoadError {
    /// Typed cause.
    pub failure: RomLoadFailure,
    /// A message naming the cause.
    pub message: String,
}

impl RomLoadError {
    pub(crate) const fn new(failure: RomLoadFailure, message: String) -> Self {
        Self { failure, message }
    }
}

impl std::fmt::Display for RomLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RomLoadError {}

/// A validated ROM image: its header, its PRG and CHR banks and the raw bytes it came from
/// (REQ-CORE-010, REQ-CORE-011).
///
/// The raw image is kept because a savestate must carry it (REQ-CORE-090): the reference
/// host restores a save slot without reloading the ROM, so the state block has to be
/// self-contained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomImage {
    raw: Vec<u8>,
    header: InesHeader,
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    trainer: Vec<u8>,
    name: String,
}

impl RomImage {
    /// Validates `image` and splits it into banks; `name` is the ROM name to keep
    /// (REQ-CORE-014).
    ///
    /// Rejects a missing signature, a file shorter than the size the header announces and a
    /// header that announces no PRG-ROM. Unsupported mapper numbers are rejected one level
    /// up, when the cartridge is built (REQ-CORE-011, REQ-CORE-102). Trailing bytes beyond
    /// the announced size are ignored: many dumps carry a title block there.
    ///
    /// # Errors
    ///
    /// The typed cause and its message (REQ-CORE-011).
    pub fn try_load(image: &[u8], name: &str) -> Result<Self, RomLoadError> {
        let header = InesHeader::try_parse(image)
            .map_err(|message| RomLoadError::new(RomLoadFailure::CorruptHeader, message))?;

        if header.prg_rom_size == 0 {
            return Err(RomLoadError::new(
                RomLoadFailure::CorruptHeader,
                "Corrupt header: the header announces no PRG-ROM.".to_owned(),
            ));
        }

        if image.len() < header.announced_size() {
            // The reference does not detect truncated files; failing here is an accepted
            // divergence (REQ-CORE-011, notes).
            return Err(RomLoadError::new(
                RomLoadFailure::TruncatedFile,
                format!(
                    "Truncated file: the header announces {} bytes, the image has {}.",
                    header.announced_size(),
                    image.len()
                ),
            ));
        }

        let trainer = if header.has_trainer {
            image[InesHeader::SIZE..InesHeader::SIZE + InesHeader::TRAINER_SIZE].to_vec()
        } else {
            Vec::new()
        };

        let prg_rom = image[header.prg_rom_offset()..header.chr_rom_offset()].to_vec();
        let chr_rom = image[header.chr_rom_offset()..header.announced_size()].to_vec();

        Ok(Self {
            raw: image.to_vec(),
            header,
            prg_rom,
            chr_rom,
            trainer,
            name: name.to_owned(),
        })
    }

    /// Re-reads the image under the board the VS. System table records for it, when the table
    /// knows the dump and the header disagrees; returns the sentence the load message carries
    /// (lot MF-VS-fix step 2).
    ///
    /// **Here and nowhere else**: this is the one point where the header has been parsed and
    /// nothing has been built from it yet, so [`crate::cartridges::Cartridge::create`] picks the
    /// board, the bus-conflict rule and the memory sizes from the corrected header exactly as it
    /// does for any image — no board, no registry arm and no mapper learns that a table exists.
    /// `crate::vs::corrected_board` is what decides, and it answers `None` for an image that is
    /// not a VS. System, for a VS image the table does not know and for a header that is right.
    ///
    /// The banks are re-split from [`Self::raw`] because the correction may move the PRG/CHR
    /// boundary: a header can be wrong about where its 128 KiB divide and right about the total.
    #[cfg(feature = "board-vs-system")]
    pub(crate) fn correct_vs_board(&mut self, data_crc: u32) -> Option<String> {
        let (header, note) = crate::vs::corrected_board(&self.header, data_crc, self.raw.len())?;
        self.header = header;
        self.prg_rom =
            self.raw[self.header.prg_rom_offset()..self.header.chr_rom_offset()].to_vec();
        self.chr_rom =
            self.raw[self.header.chr_rom_offset()..self.header.announced_size()].to_vec();
        Some(note)
    }

    /// The parsed header.
    #[must_use]
    pub const fn header(&self) -> &InesHeader {
        &self.header
    }

    /// ROM name supplied at load time, typically the file name. It is kept by the instance,
    /// stored in savestates and returned on restore (REQ-CORE-014).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// PRG-ROM bytes. Never empty.
    #[must_use]
    pub fn prg_rom(&self) -> &[u8] {
        &self.prg_rom
    }

    /// PRG-ROM bytes to write: a hack pack's hooks, turned on and off as its options go (see
    /// [`crate::hack_pack`]).
    pub(crate) fn prg_rom_mut(&mut self) -> &mut [u8] {
        &mut self.prg_rom
    }

    /// CHR-ROM bytes. Empty when the cartridge uses CHR-RAM.
    #[must_use]
    pub fn chr_rom(&self) -> &[u8] {
        &self.chr_rom
    }

    /// The 512 trainer bytes, or an empty slice when there is no trainer.
    #[must_use]
    pub fn trainer(&self) -> &[u8] {
        &self.trainer
    }

    /// The exact bytes the image was loaded from.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// iNES mapper number.
    #[must_use]
    pub const fn mapper(&self) -> i32 {
        self.header.mapper
    }

    /// Region announced by the header (REQ-CORE-013).
    #[must_use]
    pub const fn announced_region(&self) -> NesRegion {
        self.header.region
    }
}
