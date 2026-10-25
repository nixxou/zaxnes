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
//! | `$5006` | R/W | overclock: extra CPU time each frame, in scanlines, before the vblank |
//! | `$5007` | R/W | bit 0: four separate nametables (2 x 2), whatever the cartridge's mirroring |
//! | `$5100`-`$51FF` | W | high byte of the x of sprite `(address & $FF) / 4`, taken at the next OAM DMA |
//! | `$5008` | R/W | PRG window: bit 7 on, bits 0-6 the 16 KiB PRG-ROM bank shown at `$5800`-`$5FFF` |
//! | `$5009` | R/W | PRG window: first 256-byte page of that bank it shows |
//! | `$5200`-`$5FFF` | R/W | 3.5 KiB of RAM for the ROM's own variables and code (the CPU runs code from it); `$5800`-`$5FFF` shows PRG-ROM instead while the window is on |
//!
//! The margin in use is `min($5003, $5002)` rounded down to a multiple of 8. `$5004` and
//! `$5005` are read each time a band of tiles is captured, so a game can hide the sides behind
//! a status bar or past the end of a level by writing them mid-frame.
//!
//! The overclock (`$5006`) holds the PPU and the APU still for that many
//! scanlines' worth of CPU cycles when the picture is done (line 240), so the game's frame
//! logic gets more time before the NMI while the picture and the sound stay as they were.

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

/// CPU cycles one overclock scanline stands for: 341 dots at three dots per cycle, rounded up.
const CYCLES_PER_SCANLINE: u32 = 114;

/// Version the ROM reads at `$5001`.
const VERSION: u8 = 1;

/// The widest margin, in pixels on each side, the core will ever capture: 64 tile columns, a
/// 1280 x 240 picture. Past 128 pixels only a margin decoder ([`crate::wide_decode`]) has
/// anything to show: the nametables wrap there.
pub const MAX_MARGIN: i32 = 512;

/// The widest margin the nametables can show: past it they wrap.
const NAMETABLE_MARGIN: i32 = 128;

/// The second sprite page in the extension's RAM (`$5007` bit 2), and the high bytes of its
/// sprites' x (sprite n at `EXTRA_HIGH + 4n`).
const EXTRA_OAM: u16 = 0x5300;
const EXTRA_HIGH: u16 = 0x5400;

/// The widest margin a ROM can ask for or be told of through its 8-bit registers.
const MAX_ROM_MARGIN: i32 = 248;

/// The tallest margin, in pixels above and below, the core will ever capture: 30 tile rows. Only
/// a margin decoder fills it, from rows past the ones the PPU drew.
pub const MAX_VERTICAL_MARGIN: i32 = 240;

/// Extra background rows above and below at the tallest margin.
pub const MAX_EXTRA_ROWS: i32 = MAX_VERTICAL_MARGIN / 8;

/// Extra background columns on each side at the widest margin.
pub const MAX_EXTRA_COLUMNS: i32 = MAX_MARGIN / 8;

/// State of the widescreen extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WideExtension {
    /// Margin the host offers on each side, in pixels, a multiple of 8 (0 = off).
    host_margin: i32,
    /// Margin the host offers above and below, in pixels, a multiple of 8 (0 = off).
    host_vertical_margin: i32,
    /// The options of the ROM's built-in patch the player turned on (`$500A`).
    host_options: u8,
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
    /// The second sprite page (`$5007` bit 2) latched at the last OAM DMA: 64 sprites in OAM
    /// format, then the high bytes of their x. Not saved: the next DMA latches it again.
    extra_oam: [u8; 256],
    extra_high: [u8; 64],
    /// RAM at `$5200`-`$5FFF`.
    scratch: Vec<u8>,
    /// Overclock register: extra scanlines of CPU time per frame.
    overclock: u8,
    /// Layout register: bit 0 asks for four separate nametables.
    layout: u8,
    /// Whether a margin decoder fills the margins: the host's margin then applies whatever
    /// the ROM asked. Set by the core, not saved.
    decoded: bool,
    /// PRG window: bit 7 on, bits 0-6 the 16 KiB bank.
    window_bank: u8,
    /// PRG window: first page of the bank.
    window_page: u8,
    /// CPU cycles of the current overclock still to run with the machine held.
    stall: u32,
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
            host_vertical_margin: 0,
            host_options: 0,
            unlock_progress: 0,
            unlocked: false,
            requested_margin: 0,
            left_extent: 0xFF,
            right_extent: 0xFF,
            pending_high: [0; 64],
            active_high: [0; 64],
            extra_oam: [0xFF; 256],
            extra_high: [0; 64],
            scratch: vec![0; RAM_SIZE],
            overclock: 0,
            layout: 0,
            decoded: false,
            window_bank: 0,
            window_page: 0,
            stall: 0,
        }
    }

    /// Sets the margin the host offers on each side, in pixels; rounded down to a multiple of 8
    /// and capped at [`MAX_MARGIN`]. It survives resets and state loads: it is a host setting.
    pub fn set_host_margin(&mut self, pixels: i32) {
        self.host_margin = (pixels.clamp(0, MAX_MARGIN) / 8) * 8;
    }

    /// Sets the margin the host offers above and below, in pixels: rounded down to a multiple of
    /// 8 and capped at [`MAX_VERTICAL_MARGIN`]. Only a margin decoder fills it.
    pub fn set_host_vertical_margin(&mut self, pixels: i32) {
        self.host_vertical_margin = (pixels.clamp(0, MAX_VERTICAL_MARGIN) / 8) * 8;
    }

    /// The margin captured above and below, in pixels: the host's when a margin decoder fills
    /// the margins, otherwise 0.
    #[must_use]
    pub const fn vertical_margin(&self) -> i32 {
        if self.decoded {
            self.host_vertical_margin
        } else {
            0
        }
    }

    /// Sets the options of the ROM's built-in patch the player turned on, which the ROM reads at
    /// `$500A` (one bit each, see `builtin_patches::HackOption`).
    pub fn set_host_options(&mut self, options: u8) {
        self.host_options = options;
    }

    /// Tells the extension a margin decoder fills the margins of this game.
    pub fn set_decoded(&mut self, decoded: bool) {
        self.decoded = decoded;
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

    /// The margin captured on each side, in pixels: the host's when a margin decoder fills the
    /// margins, otherwise [`WideExtension::rom_margin`].
    #[must_use]
    pub fn margin(&self) -> i32 {
        if self.decoded {
            return self.host_margin.max(self.rom_margin());
        }
        self.rom_margin()
    }

    /// The margin the nametables and sprites may show on each side, in pixels: 0 unless the
    /// ROM unlocked the window and asked for one and the host offers one, at most the 128 the
    /// nametables hold.
    #[must_use]
    pub fn rom_margin(&self) -> i32 {
        if !self.unlocked {
            return 0;
        }

        (i32::from(self.requested_margin)
            .min(self.host_margin)
            .min(NAMETABLE_MARGIN)
            / 8)
            * 8
    }

    /// The range of screen x a tile may start at and still show: `[low, high)`, `low` being the
    /// leftmost pixel and `high` one past the rightmost, both in the standard frame of
    /// reference (0 = left edge of the 256-pixel picture).
    #[must_use]
    pub fn visible_range(&self) -> (i32, i32) {
        let margin = self.rom_margin();
        (
            -margin.min(i32::from(self.left_extent)),
            256 + margin.min(i32::from(self.right_extent)),
        )
    }

    /// The range of screen x a sprite may start at and still show: [`WideExtension::visible_range`],
    /// or, when the ROM asked for sprites free of the background extents (`$5007` bit 1: a margin
    /// decoder draws the background), the margin the ROM asked for, up to 248.
    #[must_use]
    pub fn sprite_range(&self) -> (i32, i32) {
        if self.unlocked && self.layout & 2 != 0 {
            // Sprites are not read from the nametables: up to what the ROM can ask for.
            let margin = (i32::from(self.requested_margin)
                .min(self.host_margin)
                .min(MAX_ROM_MARGIN)
                / 8)
                * 8;
            return (-margin, 256 + margin);
        }
        self.visible_range()
    }

    /// The screen x of OAM sprite `sprite` (0..64) whose OAM x byte is `low`.
    #[must_use]
    pub fn sprite_x(&self, sprite: usize, low: u8) -> i32 {
        let high = if self.rom_margin() > 0 {
            self.active_high.get(sprite).copied().unwrap_or(0)
        } else {
            0
        };
        i32::from(i16::from_le_bytes([low, high]))
    }

    /// Unlocked with the PRG window open on `window_bank` (bit 7 set), `window_page`, as a
    /// built-in patch with a boot window asks at power-on and reset.
    pub fn boot(&mut self, window_bank: u8, window_page: u8) {
        self.unlocked = true;
        self.window_bank = window_bank;
        self.window_page = window_page;
    }

    /// Puts the registers back to their power-on values; the host margin is kept.
    pub fn reset(&mut self) {
        *self = Self {
            host_margin: self.host_margin,
            host_vertical_margin: self.host_vertical_margin,
            host_options: self.host_options,
            decoded: self.decoded,
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
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=128.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // 0..=248.
            0x5002 => Some(self.host_margin.min(MAX_ROM_MARGIN) as u8),
            0x5003 => Some(self.requested_margin),
            0x5004 => Some(self.left_extent),
            0x5005 => Some(self.right_extent),
            0x5006 => Some(self.overclock),
            0x5007 => Some(self.layout),
            0x5008 => Some(self.window_bank),
            0x5009 => Some(self.window_page),
            0x500A => Some(self.host_options),
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
            0x5006 => self.overclock = value,
            0x5007 => self.layout = value,
            0x5008 => self.window_bank = value,
            0x5009 => self.window_page = value,
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
        // Bit 3: the ROM writes the high byte of every sprite it places, so they stay with the
        // sprite page as it is, finished or not (a frame the game runs late); otherwise each page
        // starts from 0.
        if self.layout & 8 == 0 {
            self.pending_high = [0; 64];
        }
        if self.layout & 4 != 0 {
            let page = usize::from(EXTRA_OAM - FIRST_RAM);
            let high = usize::from(EXTRA_HIGH - FIRST_RAM);
            if let Some(sprites) = self.scratch.get(page..page + 256) {
                self.extra_oam.copy_from_slice(sprites);
            }
            for (n, byte) in self.extra_high.iter_mut().enumerate() {
                *byte = self.scratch.get(high + (4 * n)).copied().unwrap_or(0);
            }
        } else {
            self.extra_oam = [0xFF; 256];
        }
    }

    /// The second sprite page latched at the last OAM DMA (`$5007` bit 2), in OAM format.
    #[must_use]
    pub const fn extra_oam(&self) -> &[u8; 256] {
        &self.extra_oam
    }

    /// The screen x of sprite `sprite` (0..64) of the second page, whose x byte is `low`.
    #[must_use]
    pub fn extra_sprite_x(&self, sprite: usize, low: u8) -> i32 {
        let high = self.extra_high.get(sprite).copied().unwrap_or(0);
        i32::from(i16::from_le_bytes([low, high]))
    }

    /// The offset in PRG-ROM a CPU read of `address` sees through the PRG window, or `None`
    /// when the window is off or `address` is outside it. The offset may be past the end of
    /// a small PRG-ROM: the caller wraps it.
    #[must_use]
    pub fn prg_window(&self, address: u16) -> Option<usize> {
        if !self.unlocked || self.window_bank & 0x80 == 0 || !(0x5800..=0x5FFF).contains(&address) {
            return None;
        }

        Some(
            (usize::from(self.window_bank & 0x7F) * 0x4000)
                + (usize::from(self.window_page) * 0x100)
                + usize::from(address - 0x5800),
        )
    }

    /// Whether the ROM asked for four separate nametables (2 x 2): the 4 KiB of console VRAM
    /// are all used, whatever the mirroring the cartridge selects.
    #[must_use]
    pub const fn four_screen(&self) -> bool {
        self.unlocked && (self.layout & 1) != 0
    }

    /// The PPU just finished the picture (start of line 240): the overclock, if any, starts.
    pub fn arm_overclock(&mut self) {
        if self.unlocked && self.overclock > 0 {
            self.stall = u32::from(self.overclock) * CYCLES_PER_SCANLINE;
        }
    }

    /// Takes one CPU cycle of the overclock: true when the machine must stay still for it (the
    /// CPU alone runs it).
    #[inline]
    pub fn take_stall_cycle(&mut self) -> bool {
        if self.stall == 0 {
            return false;
        }

        self.stall -= 1;
        true
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
        writer.write_u8(self.overclock);
        writer.write_u8(self.layout);
        writer.write_u8(self.window_bank);
        writer.write_u8(self.window_page);
        writer.write_u32(self.stall);
    }

    /// Restores the register state written by [`WideExtension::save_state`]; `with_window`
    /// false reads the earlier block, without the PRG window registers (the window stays off).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn load_state(
        &mut self,
        reader: &mut StateReader,
        with_window: bool,
    ) -> Result<(), InvalidState> {
        self.unlock_progress = reader.read_u8()?.min(3);
        self.unlocked = reader.read_bool()?;
        self.requested_margin = reader.read_u8()?;
        self.left_extent = reader.read_u8()?;
        self.right_extent = reader.read_u8()?;
        reader.read_blob_into(&mut self.pending_high)?;
        reader.read_blob_into(&mut self.active_high)?;
        reader.read_blob_into(&mut self.scratch)?;
        self.overclock = reader.read_u8()?;
        self.layout = reader.read_u8()?;
        if with_window {
            self.window_bank = reader.read_u8()?;
            self.window_page = reader.read_u8()?;
        } else {
            self.window_bank = 0;
            self.window_page = 0;
        }
        self.stall = reader.read_u32()?;
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
    fn overclock_holds_the_machine_for_whole_scanlines() {
        let mut wide = unlocked(0);
        wide.arm_overclock();
        assert!(!wide.take_stall_cycle());
        wide.write(0x5006, 2);
        wide.arm_overclock();
        let held = std::iter::from_fn(|| wide.take_stall_cycle().then_some(())).count();
        assert_eq!(held, 228);
    }

    #[test]
    fn the_prg_window_maps_a_bank_page() {
        let mut wide = unlocked(0);
        assert_eq!(wide.prg_window(0x5800), None);
        wide.write(0x5008, 0x80);
        wide.write(0x5009, 0x2B);
        assert_eq!(wide.prg_window(0x5800), Some(0x2B00));
        assert_eq!(wide.prg_window(0x5FFF), Some(0x2B00 + 0x7FF));
        assert_eq!(wide.prg_window(0x57FF), None);
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
