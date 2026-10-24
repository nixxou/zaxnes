//! The widescreen extension: a small register window a ROM unlocks to have the core capture
//! tiles beyond the 256-pixel picture (see `docs/WIDESCREEN-EXTENSION.md`).
//!
//! Real hardware has no such thing: a ROM that uses it is made for this emulator. Nothing
//! changes for a ROM that never writes the unlock sequence: the window stays transparent (the
//! bus keeps reaching the cartridge and the open bus) and every capture is the standard one.
//!
//! Registers, once unlocked (`$5000`-`$5007` read back what was written unless noted):
//!
//! | Address | Access | Meaning |
//! |---|---|---|
//! | `$5000` | W | unlock sequence `W I D E` (`$57 $49 $44 $45`); R `$57` once unlocked |
//! | `$5001` | R | version, 1 |
//! | `$5002` | R | the margin the host offers on each side, in pixels (0 = widescreen off) |
//! | `$5003` | R/W | the margin the game asks for on each side, in pixels |
//! | `$5004` | R/W | how far left of x = 0 the picture may show, in pixels |
//! | `$5005` | R/W | how far right of x = 255 the picture may show, in pixels |
//! | `$5100`-`$51FF` | W | high byte of the x of sprite `(address & $FF) / 4`, taken at the next OAM DMA |
//! | `$5200`-`$5FFF` | R/W | 3.5 KiB of RAM for the ROM's own variables and code (the CPU runs code from it) |
//!
//! The margin in use is `min($5003, $5002)` rounded down to a multiple of 8. `$5004` and
//! `$5005` are read each time a band of tiles is captured, so a game can hide the sides behind
//! a status bar or past the end of a level by writing them mid-frame.

use crate::state::{InvalidState, StateReader, StateWriter};

/// The unlock sequence written to `$5000`.
const UNLOCK: [u8; 4] = *b"WIDE";

/// First address of the register window.
pub const FIRST_REGISTER: u16 = 0x5000;

/// Last address of the register window.
pub const LAST_REGISTER: u16 = 0x5FFF;

/// First address of the RAM of the window.
const FIRST_RAM: u16 = 0x5200;

/// Bytes of RAM in the window.
const RAM_SIZE: usize = 0x0E00;

/// Version the ROM reads at `$5001`.
const VERSION: u8 = 1;

/// The widest margin, in pixels on each side, the core will ever capture: twelve tile columns,
/// a 448 x 240 picture (about 1.87:1, wider than 16:9).
pub const MAX_MARGIN: i32 = 96;

/// Extra background columns on each side at the widest margin.
pub const MAX_EXTRA_COLUMNS: i32 = MAX_MARGIN / 8;

/// State of the widescreen extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WideExtension {
    /// Margin the host offers on each side, in pixels, a multiple of 8 (0 = off).
    host_margin: i32,
    /// How many bytes of [`UNLOCK`] have been written in a row.
    unlock_progress: u8,
    unlocked: bool,
    requested_margin: u8,
    left_extent: u8,
    right_extent: u8,
    /// Sprite x high bytes written since the last OAM DMA.
    pending_high: [u8; 64],
    /// Sprite x high bytes of the sprites now in OAM.
    active_high: [u8; 64],
    /// RAM at `$5200`-`$5FFF`.
    scratch: Vec<u8>,
}

impl Default for WideExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl WideExtension {
    /// The extension of a core with widescreen off.
    #[must_use]
    pub fn new() -> Self {
        Self {
            host_margin: 0,
            unlock_progress: 0,
            unlocked: false,
            requested_margin: 0,
            left_extent: 0xFF,
            right_extent: 0xFF,
            pending_high: [0; 64],
            active_high: [0; 64],
            scratch: vec![0; RAM_SIZE],
        }
    }

    /// Sets the margin the host offers on each side, in pixels; rounded down to a multiple of 8
    /// and capped at [`MAX_MARGIN`]. It survives resets and state loads: it is a host setting.
    pub fn set_host_margin(&mut self, pixels: i32) {
        self.host_margin = (pixels.clamp(0, MAX_MARGIN) / 8) * 8;
    }

    /// The margin the host offers on each side, in pixels.
    #[must_use]
    pub const fn host_margin(&self) -> i32 {
        self.host_margin
    }

    /// Whether the ROM wrote the unlock sequence since the last reset.
    #[must_use]
    pub const fn unlocked(&self) -> bool {
        self.unlocked
    }

    /// The margin captured on each side, in pixels: 0 unless the ROM unlocked the window and
    /// asked for one and the host offers one.
    #[must_use]
    pub fn margin(&self) -> i32 {
        if !self.unlocked {
            return 0;
        }

        (i32::from(self.requested_margin).min(self.host_margin) / 8) * 8
    }

    /// The range of screen x a tile may start at and still show: `[low, high)`, `low` being the
    /// leftmost pixel and `high` one past the rightmost, both in the standard frame of
    /// reference (0 = left edge of the 256-pixel picture).
    #[must_use]
    pub fn visible_range(&self) -> (i32, i32) {
        let margin = self.margin();
        (
            -margin.min(i32::from(self.left_extent)),
            256 + margin.min(i32::from(self.right_extent)),
        )
    }

    /// The screen x of OAM sprite `sprite` (0..64) whose OAM x byte is `low`.
    #[must_use]
    pub fn sprite_x(&self, sprite: usize, low: u8) -> i32 {
        let high = if self.margin() > 0 {
            self.active_high.get(sprite).copied().unwrap_or(0)
        } else {
            0
        };
        i32::from(i16::from_le_bytes([low, high]))
    }

    /// Puts the registers back to their power-on values; the host margin is kept.
    pub fn reset(&mut self) {
        *self = Self {
            host_margin: self.host_margin,
            ..Self::new()
        };
    }

    /// Whether `address` is in the register window.
    #[must_use]
    pub const fn claims(address: u16) -> bool {
        address >= FIRST_REGISTER && address <= LAST_REGISTER
    }

    /// The value a CPU read of `address` returns, or `None` when the window does not drive the
    /// bus there (still locked, or an address without a readable register).
    #[must_use]
    pub fn read(&self, address: u16) -> Option<u8> {
        if !self.unlocked {
            return None;
        }

        match address {
            0x5000 => Some(UNLOCK[0]),
            0x5001 => Some(VERSION),
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=96.
            0x5002 => Some(self.host_margin as u8),
            0x5003 => Some(self.requested_margin),
            0x5004 => Some(self.left_extent),
            0x5005 => Some(self.right_extent),
            FIRST_RAM..=LAST_REGISTER => {
                self.scratch.get(usize::from(address - FIRST_RAM)).copied()
            }
            _ => None,
        }
    }

    /// Takes a CPU write of `address`. The write still reaches the cartridge: the window only
    /// listens.
    pub fn write(&mut self, address: u16, value: u8) {
        if address == 0x5000 {
            self.advance_unlock(value);
            return;
        }

        if !self.unlocked {
            return;
        }

        match address {
            0x5003 => self.requested_margin = value,
            0x5004 => self.left_extent = value,
            0x5005 => self.right_extent = value,
            0x5100..=0x51FF => {
                self.pending_high[usize::from(address & 0xFF) / 4] = value;
            }
            FIRST_RAM..=LAST_REGISTER => {
                if let Some(byte) = self.scratch.get_mut(usize::from(address - FIRST_RAM)) {
                    *byte = value;
                }
            }
            _ => {}
        }
    }

    fn advance_unlock(&mut self, value: u8) {
        let progress = usize::from(self.unlock_progress);
        if value == UNLOCK[progress] {
            self.unlock_progress += 1;
            if usize::from(self.unlock_progress) == UNLOCK.len() {
                self.unlocked = true;
                self.unlock_progress = 0;
            }
        } else {
            self.unlock_progress = u8::from(value == UNLOCK[0]);
        }
    }

    /// An OAM DMA just copied a new sprite page: the high bytes written for it become the ones
    /// in use, and the next page starts from zero.
    pub fn latch_sprites(&mut self) {
        if !self.unlocked {
            return;
        }

        self.active_high = self.pending_high;
        self.pending_high = [0; 64];
    }

    /// Appends the register state (the host margin excluded) to `writer`.
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.unlock_progress);
        writer.write_bool(self.unlocked);
        writer.write_u8(self.requested_margin);
        writer.write_u8(self.left_extent);
        writer.write_u8(self.right_extent);
        writer.write_blob(&self.pending_high);
        writer.write_blob(&self.active_high);
        writer.write_blob(&self.scratch);
    }

    /// Restores the register state written by [`WideExtension::save_state`].
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.unlock_progress = reader.read_u8()?.min(3);
        self.unlocked = reader.read_bool()?;
        self.requested_margin = reader.read_u8()?;
        self.left_extent = reader.read_u8()?;
        self.right_extent = reader.read_u8()?;
        reader.read_blob_into(&mut self.pending_high)?;
        reader.read_blob_into(&mut self.active_high)?;
        reader.read_blob_into(&mut self.scratch)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlocked(host: i32) -> WideExtension {
        let mut wide = WideExtension::new();
        wide.set_host_margin(host);
        for byte in UNLOCK {
            wide.write(0x5000, byte);
        }
        wide
    }

    #[test]
    fn stays_transparent_until_unlocked() {
        let mut wide = WideExtension::new();
        wide.set_host_margin(88);
        wide.write(0x5003, 88);
        assert_eq!(wide.read(0x5000), None);
        assert_eq!(wide.margin(), 0);
        assert_eq!(wide.visible_range(), (0, 256));
    }

    #[test]
    fn unlock_tolerates_a_restart() {
        let mut wide = WideExtension::new();
        for byte in *b"WIWIDE" {
            wide.write(0x5000, byte);
        }
        assert!(wide.unlocked());
    }

    #[test]
    fn margin_is_the_smaller_of_both_rounded_to_a_tile() {
        let mut wide = unlocked(88);
        wide.write(0x5003, 100);
        assert_eq!(wide.margin(), 88);
        wide.write(0x5003, 60);
        assert_eq!(wide.margin(), 56);
        assert_eq!(wide.read(0x5002), Some(88));
    }

    #[test]
    fn extents_narrow_each_side() {
        let mut wide = unlocked(88);
        wide.write(0x5003, 88);
        wide.write(0x5004, 0);
        wide.write(0x5005, 40);
        assert_eq!(wide.visible_range(), (0, 296));
    }

    #[test]
    fn sprite_high_bytes_apply_from_the_next_dma_for_one_page() {
        let mut wide = unlocked(88);
        wide.write(0x5003, 88);
        wide.write(0x5100 + 4 * 5 + 3, 0xFF);
        assert_eq!(wide.sprite_x(5, 0xF0), 0xF0);
        wide.latch_sprites();
        assert_eq!(wide.sprite_x(5, 0xF0), -16);
        wide.latch_sprites();
        assert_eq!(wide.sprite_x(5, 0xF0), 0xF0);
    }

    #[test]
    fn scratch_ram_answers_once_unlocked() {
        let mut wide = WideExtension::new();
        wide.write(0x5280, 7);
        assert_eq!(wide.read(0x5280), None);
        let mut wide = unlocked(0);
        wide.write(0x5280, 7);
        assert_eq!(wide.read(0x5280), Some(7));
    }
}
