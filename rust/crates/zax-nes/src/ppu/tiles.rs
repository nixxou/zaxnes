//! The 3dSen-specific tile emission contract (SPEC-02 §3.5) and the colour composition the
//! Zapper reads back from it (REQ-CORE-062).
//!
//! Every pattern byte needed here is read through the *inert* cartridge path (`ppu_peek`),
//! so capturing records cannot count an A12 rise, toggle an MMC2/MMC4 latch or disturb the
//! $2007 read buffer: the emulation is identical whether or not the host registered a video
//! callback (REQ-CORE-033). Capture therefore always runs.
//!
//! The records of a frame are cleared on the pre-render line, not when "run one frame"
//! starts: a frame runs from the end of one vblank to the start of the next, so the host
//! still receives the records of the frame that was just drawn during the vblank. The
//! Zapper no longer reads them there — AMD-021 R2-006 makes the vblank and the pre-render
//! line see the universal backdrop only.

use super::{
    palette_index, Cart, Ppu2C02, TileBuffer, SCREEN_WIDTH, SCREEN_WIDTH_I32, VISIBLE_SCANLINES,
};
use crate::cartridges::{Cartridge, ChrFetch};
use crate::{nes_colors, NametableMirroring, TileRecord};

impl Ppu2C02 {
    /// Upper bound on the bytes the tile records of one frame occupy in a savestate, so that
    /// the host can announce a state size before serialising (REQ-ARCH-013).
    pub const MAX_TILE_RECORD_STATE_SIZE: usize = 12 + (TileBuffer::CAPACITY * 26);

    /// First column of the right-hand band the Zapper always sees as the universal backdrop
    /// (AMD-021 R2-006).
    pub const ZAPPER_BACKDROP_COLUMN: i32 = 239;

    /// Width of the unreliable left and right bands, in NES pixels (REQ-CORE-046).
    #[must_use]
    pub const fn mask_x(&self) -> i32 {
        self.mask_x
    }

    /// Width of the unreliable top and bottom bands, in NES pixels (REQ-CORE-046).
    #[must_use]
    pub const fn mask_y(&self) -> i32 {
        self.mask_y
    }

    /// Background records dropped by the capacity limit in this frame (REQ-CORE-032).
    #[must_use]
    pub const fn dropped_background_records(&self) -> usize {
        self.tiles.dropped_background()
    }

    /// Sprite records dropped by the capacity limit in this frame (REQ-CORE-032).
    #[must_use]
    pub const fn dropped_sprite_records(&self) -> usize {
        self.tiles.dropped_sprites()
    }

    pub(super) fn reset_tile_state(&mut self) {
        self.tiles.clear();
        self.frame_margin = 0;
        self.visible_rows = crate::STANDARD_ROWS;
        self.tiles.set_extra_columns(0);
        self.mask_x = 0;
        self.mask_y = 0;
    }

    /// Starts a new rendered frame: the record list of the previous one is dropped, and the
    /// widescreen margin of the new one is latched.
    pub(super) fn begin_rendered_frame(&mut self) {
        self.tiles.clear();
        self.frame_margin = self.wide.margin();
        self.tiles.set_extra_columns(self.frame_margin / 8);
        #[allow(clippy::cast_possible_truncation)] // At most 128.
        let margin = self.frame_margin as i16;
        self.visible_rows = [(-margin, 256 + margin); 30];
    }

    /// Runs the captures owed by visible line `line` at its first dot: the sprite band, and
    /// the masks on line 0.
    ///
    /// Sprites still come before the background row of the same line (REQ-CORE-044): the
    /// row is now emitted at the *end* of that same line (R2-005), so nothing else can slip
    /// between the two.
    pub(super) fn begin_capture_line(&mut self, line: i32, cart: Cart<'_, '_>) {
        if line == 0 {
            // REQ-CORE-046 reads the masks from the state at the start of visible line 0;
            // R2-005 moved the background row, not this.
            self.compute_masks(cart.as_deref());
        }

        if self.sprites_enabled() && (line & 7) == 7 {
            // REQ-CORE-039: on MMC2 and MMC4 the pattern a tile shows depends on the tiles
            // fetched before it on the line, through the $FD/$FE latches. A capture
            // therefore walks the line in display order and moves the latch forward as it
            // goes — which is why the sprites of one capture must be emitted by increasing x
            // on those two boards (REQ-CORE-044) — and puts the real latch back afterwards,
            // so that the emulation is unchanged whether or not records were built
            // (REQ-CORE-033).
            let saved_latch = cart.as_deref().map_or(0, Cartridge::chr_latch_state);
            self.capture_sprites(line, cart);
            restore_chr_latch(cart, saved_latch);
        }
    }

    /// Emits the background row owed by visible line `line`, at its last dot (R2-005).
    pub(super) fn end_capture_line(&mut self, line: i32, cart: Cart<'_, '_>) {
        if self.background_enabled() {
            let saved_latch = cart.as_deref().map_or(0, Cartridge::chr_latch_state);
            self.capture_background_row(line, cart);
            restore_chr_latch(cart, saved_latch);
        }
    }

    /// maskX and maskY, decided from the state at the start of visible line 0
    /// (REQ-CORE-046).
    fn compute_masks(&mut self, cartridge: Option<&Cartridge>) {
        (self.mask_x, self.mask_y) = match self.layout_of(cartridge) {
            NametableMirroring::Horizontal => (8, 0),
            NametableMirroring::Vertical => (0, 8),
            _ => (0, 0),
        };

        if self.left_column_masked() {
            self.mask_x = self.mask_x.max(8);
        }
    }

    // ---------------------------------------------------------------- background rows

    /// Emits the background row of line `line` when the fine Y scroll used to draw it is 1
    /// (REQ-CORE-037, REQ-CORE-038).
    fn capture_background_row(&mut self, line: i32, cart: Cart<'_, '_>) {
        // AMD-021 R2-005 (supersedes point 3 of the answer to Q-004 and DEC-02-Q4 on this
        // point): the scroll is the left-edge shadow of the internal address read at the END
        // of the line, so a $2006 write made during line L reaches the row of line L itself,
        // while the vertical increment and the horizontal reload of the end of the line only
        // reach line L + 1. The one-line tolerance of REQ-CORE-037 no longer applies to the
        // instant of capture.
        let fine_y = (self.v_row_start >> 12) & 0x07;
        if fine_y != 1 {
            return;
        }

        // Exception of the original: no row is captured on line 0 under vertical mirroring
        // (REQ-CORE-037, justification).
        if line == 0 && self.layout_of(cart.as_deref()) == NametableMirroring::Vertical {
            return;
        }

        let coarse_x = self.v_row_start & 0x1F;
        let coarse_y = (self.v_row_start >> 5) & 0x1F;
        let nametable = (self.v_row_start >> 10) & 0x03;
        let fine_x_scroll = i32::from(self.fine_x);
        let pattern_table: u16 = if (self.control & 0x10) != 0 {
            0x1000
        } else {
            0x0000
        };
        let row_index = line / 8;
        #[allow(clippy::cast_possible_truncation)] // line is 0..240.
        let y = (line - 1) as i16;

        // Widescreen extension: `extra` more columns on each side, read on through the
        // horizontally neighbouring nametables (a 512-pixel plane that wraps), and only those
        // inside the visible range of this band kept. Standard frames have extra = 0.
        let extra = self.frame_margin / 8;
        let (visible_low, visible_high) = if extra > 0 {
            self.wide.visible_range()
        } else {
            (0, SCREEN_WIDTH_I32)
        };
        let row_width = 33 + (2 * extra);
        if extra > 0 {
            if let Some(row) = usize::try_from(line / 8)
                .ok()
                .and_then(|row| self.visible_rows.get_mut(row))
            {
                #[allow(clippy::cast_possible_truncation)] // -128 to 384.
                let range = (visible_low as i16, visible_high as i16);
                *row = range;
            }
        }

        for k in -extra..=(32 + extra) {
            let x = (8 * k) - fine_x_scroll;
            if x >= SCREEN_WIDTH_I32 + (8 * extra) {
                // k = 32 + extra with fine X at 0: a row has 32 records, otherwise 33
                // (REQ-CORE-038), plus the extra columns.
                continue;
            }

            if extra > 0 && (x + 8 <= visible_low || x >= visible_high) {
                continue;
            }

            let wrapped = (i32::from(coarse_x) + k).rem_euclid(64);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..64.
            let mut column = wrapped as u16;
            let mut table = nametable;
            if column > 31 {
                // The horizontally neighbouring nametable, mirroring then applied.
                table ^= 1;
                column -= 32;
            }

            let nametable_address = 0x2000 | (table << 10) | (coarse_y << 5) | column;
            let tile = self.read_nametable_inner(nametable_address, cart.as_deref());
            let attribute_address =
                0x23C0 | (table << 10) | ((coarse_y & 0x1C) << 1) | (column >> 2);
            let attribute = self.read_nametable_inner(attribute_address, cart.as_deref());
            let shift = ((coarse_y & 0x02) << 1) | (column & 0x02);
            let palette = (attribute >> shift) & 0x03;

            let (plane0, plane1) = self.read_pattern_planes(
                pattern_table + (u16::from(tile) * 16),
                ChrFetch::Background,
                cart.as_deref(),
            );
            if let Some(cartridge) = cart.as_deref_mut() {
                cartridge.advance_chr_latch(i32::from(pattern_table), i32::from(tile));
            }

            #[allow(clippy::cast_possible_truncation)] // x is -103..352.
            let record = TileRecord::new(
                plane0,
                plane1,
                x as i16,
                y,
                (row_index * row_width) + k + extra,
                false,
                false,
                false,
                palette,
                true,
            );
            self.tiles.add(record);
        }
    }

    // ---------------------------------------------------------------- sprites

    /// Emits one record per OAM sprite whose 8-pixel band starts in the band of line `line`
    /// (REQ-CORE-041, REQ-CORE-042).
    ///
    /// The hardware limit of eight sprites per line does *not* apply: all 64 sprites are
    /// emitted, which is what keeps 3D sprites from flickering (REQ-CORE-041, notes).
    /// Left-column masking removes no record either; it is only reported through maskX.
    fn capture_sprites(&mut self, line: i32, cart: Cart<'_, '_>) {
        let band = line / 8;
        let mut count = 0;
        for n in 0..64 {
            let top = i32::from(self.oam[n * 4]) + 1;
            if top >= VISIBLE_SCANLINES || top / 8 != band {
                continue;
            }

            self.sprite_order[count] = n;
            count += 1;
        }

        if count == 0 {
            return;
        }

        // Mappers 9 and 10 emit the sprites of one capture by increasing x, OAM number
        // breaking ties (REQ-CORE-044). The reference uses an unstable sort there; the
        // deterministic tie-break is required by REQ-ARCH-012.
        if matches!(cart.as_deref().map(Cartridge::mapper_number), Some(9 | 10)) {
            self.sort_sprites_by_x(count);
        }

        let tall = (self.control & 0x20) != 0;
        let sprite_table: u16 = if (self.control & 0x08) != 0 {
            0x1000
        } else {
            0x0000
        };

        for i in 0..count {
            let n = self.sprite_order[i];
            let oam_y = self.oam[n * 4];
            let tile = u16::from(self.oam[(n * 4) + 1]);
            let attribute = self.oam[(n * 4) + 2];
            let oam_x = self.oam[(n * 4) + 3];
            let x = if self.frame_margin > 0 {
                let x = self.wide.sprite_x(n, oam_x);
                let (visible_low, visible_high) = self.wide.visible_range();
                if x + 8 <= visible_low || x >= visible_high {
                    continue;
                }
                x
            } else {
                i32::from(oam_x)
            };
            let sprite = SpriteAttributes {
                #[allow(clippy::cast_possible_truncation)] // -32768..32767 by construction.
                x: x as i16,
                mirror_h: (attribute & 0x40) != 0,
                mirror_v: (attribute & 0x80) != 0,
                hidden: (attribute & 0x20) != 0,
                palette: attribute & 0x03,
            };
            let top = i16::from(oam_y) + 1;
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // n < 64.
            let index = n as i32;

            if !tall {
                self.emit_sprite(index, top, sprite_table + (tile * 16), &sprite, cart);
                continue;
            }

            // 8x16: the table comes from bit 0 of the tile number and the pair is
            // (T & $FE, T | 1); vertical flip swaps the halves as well as flipping each.
            let tall_table: u16 = if (tile & 1) != 0 { 0x1000 } else { 0x0000 };
            let upper = if sprite.mirror_v {
                tile | 1
            } else {
                tile & 0xFE
            };
            let lower = if sprite.mirror_v {
                tile & 0xFE
            } else {
                tile | 1
            };
            self.emit_sprite(index, top, tall_table + (upper * 16), &sprite, cart);

            if i32::from(top) + 8 < VISIBLE_SCANLINES {
                self.emit_sprite(
                    64 + index,
                    top + 8,
                    tall_table + (lower * 16),
                    &sprite,
                    cart,
                );
            }
        }
    }

    fn sort_sprites_by_x(&mut self, count: usize) {
        // Insertion sort: the lists are at most 64 long and the order must be stable so
        // that equal x values stay in OAM order (REQ-CORE-044, REQ-ARCH-012).
        for i in 1..count {
            let current = self.sprite_order[i];
            let current_x = self.oam[(current * 4) + 3];
            let mut j = i;
            while j > 0 && self.oam[(self.sprite_order[j - 1] * 4) + 3] > current_x {
                self.sprite_order[j] = self.sprite_order[j - 1];
                j -= 1;
            }

            self.sprite_order[j] = current;
        }
    }

    fn emit_sprite(
        &mut self,
        index: i32,
        y: i16,
        pattern_address: u16,
        sprite: &SpriteAttributes,
        cart: Cart<'_, '_>,
    ) {
        let (plane0, plane1) =
            self.read_pattern_planes(pattern_address, ChrFetch::Sprite, cart.as_deref());
        if let Some(cartridge) = cart.as_deref_mut() {
            cartridge.advance_chr_latch(
                i32::from(pattern_address & 0x1000),
                i32::from((pattern_address >> 4) & 0xFF),
            );
        }

        self.tiles.add(TileRecord::new(
            plane0,
            plane1,
            sprite.x,
            y,
            index,
            sprite.mirror_h,
            sprite.mirror_v,
            sprite.hidden,
            sprite.palette,
            false,
        ));
    }

    /// Reads the 16 CHR bytes of the pattern at `address` through the side-effect-free path
    /// and packs them as REQ-CORE-034 requires.
    fn read_pattern_planes(
        &self,
        address: u16,
        fetch: ChrFetch,
        cartridge: Option<&Cartridge>,
    ) -> (u64, u64) {
        let mut plane0 = 0u64;
        let mut plane1 = 0u64;
        for row in 0..8u16 {
            plane0 |=
                u64::from(self.peek_vram(address.wrapping_add(row), fetch, cartridge)) << (8 * row);
            plane1 |= u64::from(self.peek_vram(address.wrapping_add(8 + row), fetch, cartridge))
                << (8 * row);
        }

        (plane0, plane1)
    }

    // ---------------------------------------------------------------- Zapper

    /// Universal backdrop colour, entry 0 of the palette ($3F00), as the Zapper sees it
    /// (REQ-CORE-062, AMD-021 R2-006).
    #[must_use]
    pub const fn universal_backdrop_color(&self) -> u8 {
        self.palette[palette_index(0x3F00)]
    }

    /// Whether the Zapper sees light at (`x`, `y`) (REQ-CORE-062, AMD-021 R2-006).
    ///
    /// The three conditions of REQ-CORE-062: the aim is inside the image, the scan has
    /// already passed that pixel in the frame being drawn, and the colour displayed there is
    /// light.
    ///
    /// R2-006 replaces what "the colour displayed there" means in two cases, and in doing so
    /// supersedes points 1 and 3 of the answer to Q-005 and the notes of REQ-CORE-062.
    /// Outside the visible scan — the vblank and the pre-render line — the records of the
    /// frame just drawn are no longer consulted at all: the colour considered is the
    /// universal backdrop, so a game that polls during the vblank sees light exactly when
    /// entry 0 of the palette is light. And from column 239 on, the colour considered is the
    /// universal backdrop as well, whatever the records say.
    #[must_use]
    pub fn sees_light_at(&self, x: i32, y: i32) -> bool {
        if !(0..SCREEN_WIDTH_I32).contains(&x) || !(0..VISIBLE_SCANLINES).contains(&y) {
            return false;
        }

        // R2-006: every line past the last visible one — the post-render line, the vblank
        // and the pre-render line — reads the backdrop and nothing else. Q-059: the
        // post-render line is neither of the two R2-006 names, and is read with them.
        if self.scanline >= VISIBLE_SCANLINES {
            return nes_colors::is_light(self.universal_backdrop_color());
        }

        if self.scanline <= y {
            return false;
        }

        // R2-006: the right-hand band is always seen with the universal backdrop colour —
        // light when that colour is light, which the previous reading of Q-005 point 3 never
        // allowed.
        nes_colors::is_light(if x >= Self::ZAPPER_BACKDROP_COLUMN {
            self.universal_backdrop_color()
        } else {
            self.compose_recorded_color(x, y)
        })
    }

    /// Colour displayed at (`x`, `y`), composed from the records already emitted in the
    /// current frame (REQ-CORE-062).
    ///
    /// NES composition order: a non-transparent pixel of the first sprite (lowest index)
    /// that is not "hidden"; otherwise a non-transparent background pixel; otherwise a
    /// non-transparent pixel of a hidden sprite; otherwise the universal backdrop.
    #[must_use]
    pub fn compose_recorded_color(&self, x: i32, y: i32) -> u8 {
        let mut front_index = i32::MAX;
        let mut front_entry: Option<u16> = None;
        let mut behind_index = i32::MAX;
        let mut behind_entry: Option<u16> = None;
        let mut background_entry: Option<u16> = None;

        for record in self.tiles.records() {
            let (Ok(column), Ok(row)) = (
                usize::try_from(x - i32::from(record.x)),
                usize::try_from(y - i32::from(record.y)),
            ) else {
                continue;
            };

            let Some(pixel) = record.displayed_pixel(column, row) else {
                continue;
            };

            if pixel == 0 {
                continue;
            }

            if record.background {
                // Q-005: two background rows can cover the same pixel after a mid-frame
                // scroll change; the one captured later wins, which is the order the scan
                // drew them in.
                background_entry = Some(u16::from(record.palette * 4 + pixel));
                continue;
            }

            let entry = 0x10 + u16::from(record.palette * 4 + pixel);
            if record.hidden {
                if record.index < behind_index {
                    behind_index = record.index;
                    behind_entry = Some(entry);
                }
            } else if record.index < front_index {
                front_index = record.index;
                front_entry = Some(entry);
            }
        }

        let chosen = front_entry
            .or(background_entry)
            .or(behind_entry)
            .unwrap_or(0);
        self.palette[palette_index(0x3F00 | chosen)]
    }
}

impl Ppu2C02 {
    /// The picture of the frame just drawn, as wide as its widescreen margin: one NES colour
    /// index per pixel, row 0 first, `256 + 2 * margin` pixels a row (the 2D mode). The 256
    /// middle columns are the PPU's own picture; the sides are composed from the tile records
    /// in the NES order (a sprite in front, then the background, then a sprite behind, a lower
    /// sprite number winning) over the universal backdrop.
    pub fn wide_picture_into(&self, out: &mut Vec<u8>) {
        let margin = usize::try_from(self.frame_margin.max(0)).unwrap_or(0);
        let width = SCREEN_WIDTH + (2 * margin);
        let height = super::SCREEN_HEIGHT;
        out.clear();
        if margin == 0 {
            out.extend_from_slice(&self.pixels);
            return;
        }

        // The PPU blanks its first 8 columns when the game asks for it, which in a wide picture
        // would be a black band in the middle: they are composed too.
        let composed_from = if self.left_column_masked() { 8 } else { 0 };
        // Palette entries of the sides (0 = nothing), composed first, then turned into colours. A
        // sprite behind is painted first so that an opaque background pixel covers it.
        out.resize(width * height, 0);
        let paint = |record: &TileRecord, base: u8, out: &mut Vec<u8>| {
            for py in 0..8i32 {
                let y = i32::from(record.y) + py;
                let Ok(row) = usize::try_from(y) else {
                    continue;
                };
                if row >= height {
                    continue;
                }
                for px in 0..8i32 {
                    let x = i32::from(record.x) + px;
                    if (composed_from..SCREEN_WIDTH_I32).contains(&x) {
                        continue;
                    }
                    let Ok(column) = usize::try_from(x + self.frame_margin) else {
                        continue;
                    };
                    if column >= width {
                        continue;
                    }
                    #[allow(clippy::cast_sign_loss)] // 0..8.
                    let Some(pixel) = record.displayed_pixel(px as usize, py as usize) else {
                        continue;
                    };
                    let slot = &mut out[(row * width) + column];
                    if pixel != 0 {
                        *slot = base + (record.palette * 4) + pixel;
                    }
                }
            }
        };
        let records = self.tiles.records();
        for record in records.iter().filter(|r| !r.background && r.hidden).rev() {
            paint(record, 0x10, out);
        }
        for record in records.iter().filter(|r| r.background) {
            paint(record, 0x00, out);
        }
        let mut front: Vec<&TileRecord> = records
            .iter()
            .filter(|r| !r.background && !r.hidden)
            .collect();
        front.sort_by_key(|record| std::cmp::Reverse(record.index % 64));
        for record in front {
            paint(record, 0x10, out);
        }

        for row in 0..height {
            let line = &mut out[row * width..(row + 1) * width];
            for (column, slot) in line.iter_mut().enumerate() {
                #[allow(clippy::cast_sign_loss)] // 0 or 8.
                let first = margin + composed_from as usize;
                if (first..margin + SCREEN_WIDTH).contains(&column) {
                    *slot = self.pixels[(row * SCREEN_WIDTH) + column - margin];
                } else {
                    *slot = self.palette[palette_index(0x3F00 | u16::from(*slot))];
                }
            }
        }
    }
}

/// The OAM fields one sprite capture copies into its records.
#[allow(clippy::struct_excessive_bools)] // The three attribute flags of the OAM byte.
struct SpriteAttributes {
    x: i16,
    mirror_h: bool,
    mirror_v: bool,
    hidden: bool,
    palette: u8,
}

fn restore_chr_latch(cart: Cart<'_, '_>, saved_latch: i32) {
    if let Some(cartridge) = cart.as_deref_mut() {
        cartridge.set_chr_latch_state(saved_latch);
    }
}
