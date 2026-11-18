//! The wide 2D picture of a frame with a widescreen margin (lot NESHAWK-HACKS step 4).
//!
//! **Not a port of anything**: `zax_nes::ppu::Ppu2C02::wide_picture_into`, done on what this
//! engine keeps of a frame. The centre 256 pixels are the PPU's own (`xbuf`); the margins, and
//! the left 8 columns when the game blanks them, are composed from the frame's tile records with
//! the frame's last palette, by the same painting rule: sprites behind the background first, the
//! background, then the sprites in front by OAM priority, a sprite in front of a lower-priority
//! sprite behind the background left out where that one is.
//!
//! Not composed yet, unlike Zaxnes: the margin decoders' rows and palette, the vertical margins,
//! and the sprites the PPU did not draw in the centre (second page, across the left edge).

use zax_core_api::TileRecord;

/// The first of the "asleep" tint's palette entries (`zax_nes`'s `TINT_ENTRIES`).
const TINT_ENTRIES: u8 = 0x30;

/// What the composition reads of a frame.
pub struct WideFrameView<'a> {
    pub records: &'a [TileRecord],
    /// The palette RAM as the frame ended.
    pub palette: &'a [u8; 32],
    /// The PPU's picture, one NES colour index a pixel (bits 0-5).
    pub pixels: &'a [u8],
    /// The margin, in pixels on each side.
    pub margin: i32,
    /// Whether the PPU showed its first 8 columns (background and sprites).
    pub left_column_shown: bool,
    /// The sprites in the "asleep" tint, one bit per OAM number.
    pub tinted: u64,
    /// What the margin decoder added: its palette and the centre columns it redraws.
    pub decoded: crate::wide_decode::Decoded,
}

/// The first of the margin palette's entries (`zax_nes`'s `MARGIN_ENTRIES`).
const MARGIN_ENTRIES: u8 = 0x20;

/// `zax_nes::ppu::palette_index`: the palette RAM index of a `$3F00`-`$3F1F` address.
const fn palette_index(address: u16) -> usize {
    let mut index = (address & 0x1F) as usize;
    if (index & 0x13) == 0x10 {
        index &= 0x0F;
    }
    index
}

/// The "asleep" tint of colour `colour`: its brightness in grey, the blacks kept.
const fn asleep_tint(colour: u8) -> u8 {
    if colour & 0x0F >= 0x0D {
        colour
    } else {
        colour & 0x30
    }
}

/// Drawing priority of a sprite record (lower first), `zax_nes`'s `sprite_priority`.
const fn sprite_priority(index: i32) -> i32 {
    if index >= 128 {
        index
    } else {
        index % 64
    }
}

impl WideFrameView<'_> {
    fn tinted(&self, record: &TileRecord) -> bool {
        record.index < 128
            && u32::try_from(record.index % 64).is_ok_and(|n| self.tinted & (1 << n) != 0)
    }

    /// `Ppu2C02::entry_colour`, with the opacity bit of AMD-050 REQ-CORE-049.
    fn entry_colour(&self, entry: u8) -> u8 {
        let colour = if let Some(sprite) = entry.checked_sub(TINT_ENTRIES) {
            asleep_tint(self.palette[palette_index(0x3F10 | u16::from(sprite))] & 0x3F)
        } else {
            match (
                self.decoded.margin_palette,
                entry.checked_sub(MARGIN_ENTRIES),
            ) {
                (Some(margin_palette), Some(entry)) => margin_palette[usize::from(entry & 15)],
                _ => self.palette[palette_index(0x3F00 | u16::from(entry))] & 0x3F,
            }
        };
        if entry == 0 {
            colour
        } else {
            colour | zax_nes::ppu::OPAQUE_PIXEL
        }
    }

    /// The picture line each background row's records stop at: the top of the next row kept
    /// below it (a split's status bar), by row number (`Ppu2C02::row_ends`).
    fn row_ends(&self, row_width: i32) -> [i16; 31] {
        let mut tops = [None; 31];
        for record in self.records.iter().filter(|record| record.background) {
            if let Some(slot) = usize::try_from(record.index / row_width)
                .ok()
                .and_then(|row| tops.get_mut(row))
            {
                *slot = Some(record.y);
            }
        }
        let mut ends = [i16::MAX; 31];
        let mut next = i16::MAX;
        for (end, top) in ends.iter_mut().zip(tops.iter()).rev() {
            *end = next;
            if let Some(top) = top {
                next = *top;
            }
        }
        ends
    }

    /// The wide picture into `out`: `256 + 2 * margin` pixels a row, 240 rows; returns 240.
    pub fn compose(&self, out: &mut Vec<u8>) -> usize {
        let margin = usize::try_from(self.margin.max(0)).unwrap_or(0);
        let width = 256 + (2 * margin);
        let height = 240;
        out.clear();
        out.resize(width * height, 0);
        let composed_from: i32 = if self.left_column_shown { 0 } else { 8 };
        // The centre's x range of each line: the PPU's own pixels, less the columns a decoder
        // redraws.
        let centres: Vec<(i32, i32)> = (0..height)
            .map(|line| {
                let band = line / 8;
                (
                    self.decoded
                        .redrawn_to
                        .get(band)
                        .map_or(composed_from, |&to| i32::from(to).max(composed_from)),
                    self.decoded
                        .redrawn_from
                        .get(band)
                        .map_or(256, |&from| i32::from(from)),
                )
            })
            .collect();
        let row_width = 33 + (2 * (self.margin / 8));
        let ends = self.row_ends(row_width);
        let mut behind = vec![i32::MAX; width * height];
        let records = self.records;
        for record in records.iter().filter(|r| !r.background && r.hidden).rev() {
            self.paint(
                record,
                0x10,
                out,
                width,
                &centres,
                &mut behind,
                &ends,
                row_width,
            );
        }
        for record in records.iter().filter(|r| r.background) {
            self.paint(
                record,
                0x00,
                out,
                width,
                &centres,
                &mut behind,
                &ends,
                row_width,
            );
        }
        let mut front: Vec<&TileRecord> = records
            .iter()
            .filter(|r| !r.background && !r.hidden)
            .collect();
        front.sort_by_key(|record| std::cmp::Reverse(sprite_priority(record.index)));
        for record in front {
            self.paint(
                record,
                0x10,
                out,
                width,
                &centres,
                &mut behind,
                &ends,
                row_width,
            );
        }
        for row in 0..height {
            let (from, to) = centres[row];
            let first = margin + usize::try_from(from).unwrap_or(0);
            let end = (margin + usize::try_from(to).unwrap_or(256)).max(first);
            let line = &mut out[row * width..(row + 1) * width];
            let source = (row * 256) + first - margin;
            for (slot, &pixel) in line[first..end]
                .iter_mut()
                .zip(&self.pixels[source..source + (end - first)])
            {
                *slot = (pixel & zax_nes::ppu::PIXEL_COLOR) | zax_nes::ppu::OPAQUE_PIXEL;
            }
            let (before, after) = line.split_at_mut(end);
            for slot in before[..first].iter_mut().chain(after.iter_mut()) {
                *slot = self.entry_colour(*slot);
            }
        }
        height
    }

    /// `Ppu2C02::paint_side_tile`, the centre being `composed_from..256` on every line.
    #[allow(clippy::too_many_arguments)] // The picture and the state of its composition.
    fn paint(
        &self,
        record: &TileRecord,
        base: u8,
        out: &mut [u8],
        width: usize,
        centres: &[(i32, i32)],
        behind: &mut [i32],
        ends: &[i16; 31],
        row_width: i32,
    ) {
        let left = i32::from(record.x);
        let top_line = i32::from(record.y);
        let hidden = (0..8i32).all(|py| {
            usize::try_from(top_line + py)
                .ok()
                .and_then(|row| centres.get(row))
                .is_none_or(|&(from, to)| from <= left && left + 8 <= to)
        });
        if hidden {
            return; // wholly in the centre
        }
        let priority = sprite_priority(record.index);
        let top = i32::from(record.y);
        let end = if record.background {
            usize::try_from(record.index / row_width)
                .ok()
                .and_then(|row| ends.get(row))
                .map_or(i32::MAX, |&end| i32::from(end))
        } else {
            i32::MAX
        };
        let lines = (end - top).clamp(0, 8);
        for py in 0..8i32 {
            let Ok(row) = usize::try_from(top + py) else {
                continue;
            };
            let Some(&(from, to)) = centres.get(row) else {
                continue;
            };
            for px in 0..8i32 {
                let x = left + px;
                if (from..to).contains(&x) || (py >= lines && !(0..256).contains(&x)) {
                    continue;
                }
                let Ok(column) = usize::try_from(x + self.margin) else {
                    continue;
                };
                if column >= width {
                    continue;
                }
                #[allow(clippy::cast_sign_loss)] // 0..8.
                let Some(pixel) = record.displayed_pixel(px as usize, py as usize) else {
                    continue;
                };
                let at = (row * width) + column;
                if pixel != 0 && !record.background {
                    if record.hidden {
                        behind[at] = behind[at].min(priority);
                    } else if behind[at] < priority {
                        continue;
                    }
                }
                if pixel != 0 {
                    out[at] = if record.background && record.palette >= 4 {
                        MARGIN_ENTRIES + ((record.palette - 4) * 4) + pixel
                    } else if !record.background && self.tinted(record) {
                        TINT_ENTRIES + (record.palette * 4) + pixel
                    } else {
                        base + (record.palette * 4) + pixel
                    };
                }
            }
        }
    }
}
