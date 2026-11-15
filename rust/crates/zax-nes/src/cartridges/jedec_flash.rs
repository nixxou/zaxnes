//! Lot MAPPERS-P: the JEDEC command machine of the flash chips some boards carry as their
//! PRG-ROM (mappers 406, 446 and 451), read from puNES 0.111 `SST39SF040.c` (read, not copied).
//!
//! The chip's bytes are the PRG-ROM itself; programs and erases go through
//! [`CartridgeMemory::flash_program`] and [`CartridgeMemory::flash_erase_sector`] so that
//! [`super::FlashRom`] records the changed sectors as the save file. A program `AND`s the byte,
//! which is what a flash cell does and what puNES (which stores it) does not (FIDELITY-LEADS
//! J37). An erase keeps the chip busy one CPU cycle per erased byte, as puNES counts it; reads
//! then toggle D6 and are masked with `$77`, and the software-ID mode answers the two IDs.
use super::CartridgeMemory;
use crate::state::{InvalidState, StateReader, StateWriter};

/// One board's chip: its manufacturer and model IDs, the two unlock addresses (compared with
/// the chip address `& $7FFF`) and the erase sector size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JedecChip {
    pub(crate) ids: (u8, u8),
    pub(crate) unlock: (usize, usize),
    pub(crate) sector: usize,
}

/// The command state (`sst39sf040.sequence`, `.time_out`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct JedecFlash {
    sequence: u8,
    busy: u32,
}

impl JedecFlash {
    /// `sst39sf040_write`: one write at chip address `chip` (a PRG-ROM offset) of a chip of
    /// `size` bytes.
    pub(crate) fn write(
        &mut self,
        memory: &mut CartridgeMemory,
        chip: &JedecChip,
        offset: usize,
        value: u8,
        size: usize,
    ) {
        let command = offset & 0x7FFF;
        let (first, second) = chip.unlock;
        match self.sequence {
            0x01 | 0x81 => {
                if command == second && value == 0x55 {
                    self.sequence += 1;
                }
            }
            0x02 => {
                if command == first {
                    self.sequence = value;
                }
            }
            0x82 => {
                if value == 0x30 {
                    if offset < size {
                        memory.flash_erase_sector(offset);
                        self.busy = u32::try_from(chip.sector).unwrap_or(u32::MAX);
                    }
                } else if command == first && value == 0x10 {
                    for sector in (0..size).step_by(chip.sector.max(1)) {
                        memory.flash_erase_sector(sector);
                    }
                    self.busy = u32::try_from(size).unwrap_or(u32::MAX);
                }
            }
            0x90 => {
                if value == 0xF0 {
                    self.sequence = 0;
                }
            }
            0xA0 => {
                memory.flash_program(offset, value);
                self.sequence = 0;
            }
            // `default`, `0x00` and `0x80`: the first unlock byte.
            _ => {
                if command == first && value == 0xAA {
                    self.sequence = self.sequence.wrapping_add(1);
                }
            }
        }
    }

    /// Whether the chip answers reads itself (software ID or an erase running), which takes the
    /// board's ROM windows off the cached path.
    pub(crate) const fn answers(self) -> bool {
        self.sequence == 0x90 || self.busy != 0
    }

    /// `sst39sf040_read` while [`JedecFlash::answers`]: the IDs by A0, or the array byte `byte`
    /// with D6 toggling and `& $77`.
    pub(crate) fn read(self, chip: &JedecChip, address: u16, byte: Option<u8>) -> Option<u8> {
        if self.sequence == 0x90 {
            return Some(if address & 0x0001 != 0 {
                chip.ids.1
            } else {
                chip.ids.0
            });
        }
        byte.map(|byte| (byte ^ (u8::from(self.busy & 0x01 != 0) << 6)) & 0x77)
    }

    /// `sst39sf040_tick`: the end of an erase ends the command. Returns whether it was busy.
    pub(crate) fn tick(&mut self) -> bool {
        if self.busy == 0 {
            return false;
        }
        self.busy -= 1;
        if self.busy == 0 {
            self.sequence = 0;
        }
        true
    }

    pub(crate) fn save_state(self, writer: &mut StateWriter) {
        writer.write_u8(self.sequence);
        writer.write_u32(self.busy);
    }

    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.sequence = reader.read_u8()?;
        self.busy = reader.read_u32()?;
        Ok(())
    }
}
