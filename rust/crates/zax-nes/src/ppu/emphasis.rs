//! The colour emphasis bits (PPUMASK bits 5-7) of the 2D picture (lot FIDPLUS-D2b).
//!
//! The picture keeps one byte a pixel for the colour index and the opacity bit
//! ([`super::PIXEL_COLOR`], [`super::OPAQUE_PIXEL`]); REQ-CORE-047 keeps emphasis out of the tile
//! records and the palette. Emphasis is a property of the signal the 2C02 puts out, so a picture
//! without it shows a game that darkens or tints its screen through `$2001` at full brightness.
//!
//! It costs the PPU nothing per pixel: a write that changes the three bits is logged with the
//! first pixel it reaches, and the host expands the log into a plane only for a frame that has
//! emphasis at all. Delay after the write, in dots: 2 / 1 / 1 / 2 for alignments 0 / 1 / 2 / 3
//! (`TriCNES`, `Emulator.cs:10055-10066`, FIDELITY-LEADS C3; not measured on the netlist yet).

use super::{Ppu2C02, SCREEN_HEIGHT, SCREEN_WIDTH, VISIBLE_SCANLINES};

/// Dots from a `$2001` write to the first pixel drawn with its emphasis, by alignment.
const EMPHASIS_DELAY: [i32; 4] = [2, 1, 1, 2];

impl Ppu2C02 {
    /// Logs a write of `value` to PPUMASK whose emphasis bits differ from the mask's. Called
    /// before the mask takes the value; out of line, the write path only compares.
    #[cold]
    #[inline(never)]
    pub(super) fn note_emphasis(&mut self, value: u8) {
        if !(0..VISIBLE_SCANLINES).contains(&self.scanline) {
            // In vblank or on the pre-render line: the next frame starts with it
            // ([`Self::restart_emphasis`]).
            return;
        }
        let delay = EMPHASIS_DELAY[usize::from(self.alignment & 3)];
        let x = (self.dot - 1 + delay).clamp(0, 256);
        let index = self.scanline * 256 + x;
        self.emphasis_log
            .push((u32::try_from(index).unwrap_or(0), value & 0xE0));
    }

    /// The frame wrap: the new frame starts with the emphasis in force and no change yet.
    #[cold]
    #[inline(never)]
    pub(super) fn restart_emphasis(&mut self) {
        self.emphasis_start = self.mask & 0xE0;
        self.emphasis_log.clear();
    }

    /// The colour subcarrier's phase at the start of the frame just drawn (0 to 11), for
    /// [`crate::composite::RowDecoder`]: picture row `r` starts at `phase + 4 r` (mod 12) plus a
    /// constant, a constant that does not change the decoded colours. On the PAL 2C07,
    /// [`crate::composite::PAL_VIDEO`]: its phases depend on the line alone
    /// ([`crate::composite::RowDecoder::decode_row_pal`]).
    #[must_use]
    pub const fn video_phase(&self) -> usize {
        if matches!(self.timing.region, crate::NesRegion::Pal) {
            crate::composite::PAL_VIDEO
        } else {
            self.video_phase as usize
        }
    }

    /// Whether the frame just drawn showed any emphasis.
    #[must_use]
    pub fn frame_has_emphasis(&self) -> bool {
        self.emphasis_start != 0 || self.emphasis_log.iter().any(|&(_, bits)| bits != 0)
    }

    /// The emphasis of every pixel of the frame just drawn, row-major 256 x 240, as PPUMASK bits
    /// 5-7 shifted down (0 to 7), into `out`; `false` (and `out` empty) when the frame has none,
    /// which is every frame of most games.
    pub fn emphasis_plane_into(&self, out: &mut Vec<u8>) -> bool {
        out.clear();
        if !self.frame_has_emphasis() {
            return false;
        }
        let total = SCREEN_WIDTH * SCREEN_HEIGHT;
        out.resize(total, self.emphasis_start >> 5);
        let mut changes = self.emphasis_log.clone();
        changes.sort_by_key(|&(index, _)| index);
        for (position, &(index, bits)) in changes.iter().enumerate() {
            let from = (index as usize).min(total);
            let to = changes
                .get(position + 1)
                .map_or(total, |&(next, _)| (next as usize).min(total));
            out[from..to].fill(bits >> 5);
        }
        true
    }

    /// [`Self::emphasis_plane_into`] laid out as [`Self::wide_picture_into`]'s picture: the
    /// margins take the emphasis of the screen pixel nearest to them. `false` and empty `out`
    /// for a frame without emphasis.
    pub fn wide_emphasis_into(&self, out: &mut Vec<u8>) -> bool {
        let mut plane = Vec::new();
        if !self.emphasis_plane_into(&mut plane) {
            out.clear();
            return false;
        }
        let margin = usize::try_from(self.frame_margin().max(0)).unwrap_or(0);
        let margin_y = usize::try_from(self.frame_vertical_margin().max(0)).unwrap_or(0);
        if margin == 0 && margin_y == 0 {
            *out = plane;
            return true;
        }
        let width = SCREEN_WIDTH + 2 * margin;
        let height = SCREEN_HEIGHT + 2 * margin_y;
        out.clear();
        out.reserve(width * height);
        for row in 0..height {
            let line = row.saturating_sub(margin_y).min(SCREEN_HEIGHT - 1);
            let screen = &plane[line * SCREEN_WIDTH..(line + 1) * SCREEN_WIDTH];
            out.extend(std::iter::repeat_n(screen[0], margin));
            out.extend_from_slice(screen);
            out.extend(std::iter::repeat_n(screen[SCREEN_WIDTH - 1], margin));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::Ppu2C02;

    #[test]
    fn fidplus_d2b_a_frame_without_emphasis_has_no_plane() {
        let ppu = Ppu2C02::new();
        let mut plane = vec![1];
        assert!(!ppu.emphasis_plane_into(&mut plane));
        assert_eq!(plane, [0u8; 0]);
    }

    #[test]
    fn fidplus_d2b_a_mid_frame_write_reaches_the_pixels_after_it() {
        let mut ppu = Ppu2C02::new();
        ppu.alignment = 1;
        ppu.scanline = 100;
        ppu.dot = 50;
        ppu.note_emphasis(0x20);
        ppu.mask = 0x20;
        let mut plane = Vec::new();
        assert!(ppu.emphasis_plane_into(&mut plane));
        // Alignment 1: one dot of delay, so x = 50 is the first pixel with the new bits.
        assert_eq!(plane[100 * 256 + 48], 0);
        assert_eq!(plane[100 * 256 + 50], 1);
        assert_eq!(plane[239 * 256 + 255], 1);
        // The next frame starts red, whole.
        ppu.restart_emphasis();
        assert!(ppu.emphasis_plane_into(&mut plane));
        assert!(plane.iter().all(|&bits| bits == 1));
    }

    #[test]
    fn fidplus_d2b_a_write_in_vblank_waits_for_the_next_frame() {
        let mut ppu = Ppu2C02::new();
        ppu.scanline = 245;
        ppu.note_emphasis(0x80);
        let mut plane = Vec::new();
        assert!(!ppu.emphasis_plane_into(&mut plane));
    }
}
