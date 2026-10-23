//! Rendering half of the PPU: scroll registers, background fetch pipeline, sprite
//! evaluation, sprite 0 hit, sprite overflow and the optional 256 x 240 indexed image
//! (REQ-CORE-048, REQ-CORE-112, REQ-CORE-116).
//!
//! The dot schedule is the one of the nesdev "PPU rendering" timing diagram: on the visible
//! lines and the pre-render line, four two-dot fetches per tile, the coarse X increment on
//! the eighth dot of each group, the Y increment at dot 256, the horizontal reload at dot
//! 257, the vertical reload at dots 280 to 304 of the pre-render line, the sprite pattern
//! fetches at dots 257 to 320 and two more tiles prefetched at dots 321 to 336.

use super::{
    map_nametable, Cart, Ppu2C02, PpuBusAccess, SCREEN_WIDTH, SPRITES_PER_LINE_I32,
    VISIBLE_SCANLINES,
};
use crate::cartridges::{Cartridge, ChrFetch};

impl Ppu2C02 {
    /// The 256 x 240 indexed image of the frame just rendered: one NES colour index per
    /// pixel, row-major (REQ-CORE-048). Needed by the 2D Mode of SPEC-06.
    #[must_use]
    pub fn pixel_frame(&self) -> &[u8] {
        &self.pixels
    }

    /// Number of sprites the evaluation of the previous line found, 0 to 8.
    #[must_use]
    pub const fn sprites_on_line(&self) -> i32 {
        self.sprites_loaded
    }

    pub(super) fn reset_render_state(&mut self) {
        self.secondary_oam.fill(0xFF);
        self.sprite_pattern_low.fill(0);
        self.sprite_pattern_high.fill(0);
        self.sprite_attribute.fill(0);
        self.sprite_x.fill(0);
        self.nametable_latch = 0;
        self.attribute_latch = 0;
        self.pattern_low_latch = 0;
        self.pattern_high_latch = 0;
        self.pattern_shift_low = 0;
        self.pattern_shift_high = 0;
        self.attribute_shift_low = 0;
        self.attribute_shift_high = 0;
        self.oam_eval_latch = 0;
        self.eval_sprite = 0;
        self.eval_byte = 0;
        self.eval_found = 0;
        self.overflow_copy = 0;
        self.eval_finished = false;
        self.sprite_zero_found = false;
        self.sprites_loaded = 0;
        self.sprite_zero_loaded = false;
        self.sprite_line_stale = true;
    }

    /// Runs everything the current dot owes: the fetch pipeline, sprite evaluation and the
    /// pixel of the visible lines.
    pub(super) fn run_render_dot(&mut self, cart: Cart<'_, '_>) {
        let dot = self.dot;
        let visible = self.scanline < VISIBLE_SCANLINES;

        if self.rendering_enabled() {
            if visible {
                self.run_rendering_dot::<true>(dot, cart);
            } else if self.scanline == self.pre_render_scanline() {
                self.run_rendering_dot::<false>(dot, cart);
            }
        }

        if visible && (1..=256).contains(&dot) {
            #[allow(clippy::cast_sign_loss)] // dot is 1..=256 here.
            let x = (dot - 1) as usize;
            self.render_pixel(x);
        }
    }

    /// Runs `count` dots of a visible line with rendering on, all within dots 2 to 255, as
    /// that many [`Ppu2C02::step_dot`] calls would. Each of these dots does the same fixed
    /// work (shift, background fetch step, sprite evaluation from dot 65, pixel), so the
    /// per-dot dispatch is skipped and the fetches read CHR and console VRAM directly.
    ///
    /// Returns `false`, having run nothing, when the cartridge's fetches have side effects or
    /// another source ([`Cartridge::chr_view`], a nametable the cartridge serves itself).
    pub(super) fn run_visible_span(&mut self, count: i64, cartridge: Option<&Cartridge>) -> bool {
        let Some(cartridge) = cartridge else {
            return false;
        };
        let Some(chr) = cartridge.chr_view() else {
            return false;
        };

        // The start of each logical nametable in console VRAM, as `read_nametable_inner`
        // finds it.
        let mut nametables = [0usize; 4];
        for (table, start) in (0u16..).zip(nametables.iter_mut()) {
            let page = cartridge.nametable_page(i32::from(table));
            if page == Cartridge::SERVED_BY_CARTRIDGE {
                return false;
            }

            *start = usize::try_from(page).map_or_else(
                |_| map_nametable(0x2000 + (table << 10), cartridge.mirroring()),
                |page| page << 10,
            );
        }

        let nametable_byte = |vram: &[u8], address: u16| {
            vram[nametables[usize::from((address >> 10) & 0x03)] | (usize::from(address) & 0x03FF)]
        };

        // PPUMASK and fine X cannot change inside a batch of dots.
        let pixels = SpanPixels::of(self.mask, i32::from(self.fine_x));
        #[allow(clippy::cast_sign_loss)] // Visible line, 0..240.
        let row = self.scanline as usize * SCREEN_WIDTH;

        for _ in 0..count {
            self.ppu_clock += 1;
            self.dot += 1;
            let dot = self.dot;

            self.shift_background();
            match (dot - 1) & 0x07 {
                0 => {
                    self.load_background_shifters();
                    self.nametable_latch = nametable_byte(&self.vram, 0x2000 | (self.v & 0x0FFF));
                }
                2 => {
                    let address = 0x23C0
                        | (self.v & 0x0C00)
                        | ((self.v >> 4) & 0x38)
                        | ((self.v >> 2) & 0x07);
                    let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
                    self.attribute_latch = (nametable_byte(&self.vram, address) >> shift) & 0x03;
                }
                4 => self.pattern_low_latch = chr.read(self.pattern_address(0)),
                6 => self.pattern_high_latch = chr.read(self.pattern_address(8)),
                7 => self.increment_coarse_x(),
                _ => {}
            }

            if dot >= 65 {
                self.run_sprite_evaluation(dot);
            }

            #[allow(clippy::cast_sign_loss)] // dot is 2..=255 here.
            let x = (dot - 1) as usize & 0xFF;
            let color = self.span_pixel(x, &pixels);
            if let Some(pixel) = self.pixels.get_mut(row + x) {
                *pixel = color;
            }
        }

        true
    }

    /// The colour of pixel `x` of a span, as [`Ppu2C02::render_pixel`] composes it.
    fn span_pixel(&mut self, x: usize, pixels: &SpanPixels) -> u8 {
        let left = x >= 8;

        let mut background = 0u8;
        if pixels.background && (left || pixels.background_left) {
            let shift = pixels.shift;
            let pixel = ((self.pattern_shift_low >> shift) & 1)
                | (((self.pattern_shift_high >> shift) & 1) << 1);
            let palette = ((self.attribute_shift_low >> shift) & 1)
                | (((self.attribute_shift_high >> shift) & 1) << 1);
            #[allow(clippy::cast_possible_truncation)] // Two 2-bit fields.
            let entry = ((palette << 2) | pixel) as u8;
            background = entry;
        }

        let mut sprite = 0u8;
        if pixels.sprites && (left || pixels.sprites_left) {
            if self.sprite_line_stale {
                self.rebuild_sprite_line();
            }

            sprite = self.sprite_line[x];
        }

        let background_opaque = (background & 0x03) != 0;
        if (sprite & SPRITE_ZERO) != 0 && background_opaque && x != SCREEN_WIDTH - 1 {
            self.sprite_zero_hit = true;
        }

        let entry = if (sprite & SPRITE_OPAQUE) != 0
            && (!background_opaque || (sprite & SPRITE_BEHIND) == 0)
        {
            0x10 | (sprite & 0x0F)
        } else if background_opaque {
            background
        } else {
            0
        };

        self.palette[usize::from(entry & 0x1F)] & pixels.color_mask
    }

    /// The work of one dot of a line that renders (a visible line when `VISIBLE`, else the
    /// pre-render line) with rendering on, in the order of the nesdev timing diagram:
    /// background pipeline, sprite evaluation, sprite fetches, vertical reload.
    fn run_rendering_dot<const VISIBLE: bool>(&mut self, dot: i32, cart: Cart<'_, '_>) {
        match dot {
            1..=256 => {
                if dot == 1 {
                    // Dots 1 to 64 clear the secondary OAM to $FF; the unused slots then make
                    // the hardware fetch pattern $FF, which boards watching A12 can see.
                    self.secondary_oam.fill(0xFF);
                    if !VISIBLE {
                        // Sprite evaluation does not run on the pre-render line, so no
                        // sprite is shown on line 0 (nesdev, "PPU sprite evaluation").
                        self.eval_found = 0;
                        self.sprite_zero_found = false;
                    }
                } else {
                    self.shift_background();
                    self.background_fetch(dot, cart);
                }

                if dot == 256 {
                    self.increment_y();
                }

                if VISIBLE && dot >= 65 {
                    self.run_sprite_evaluation(dot);
                }
            }
            257..=320 => {
                if dot == 257 {
                    self.shift_background();
                    self.background_fetch(dot, cart);
                    self.transfer_address_x();
                }

                self.run_sprite_fetches(dot, cart);

                if !VISIBLE && (280..=304).contains(&dot) {
                    self.transfer_address_y();
                }
            }
            321..=337 => {
                self.shift_background();
                self.background_fetch(dot, cart);
            }
            // Two more nametable fetches the hardware performs and discards; they matter
            // only for boards that watch the PPU address bus.
            338 | 340 => {
                self.nametable_latch = self.read_vram_inner(
                    0x2000 | (self.v & 0x0FFF),
                    PpuBusAccess::Render,
                    ChrFetch::Background,
                    cart,
                );
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------- background

    /// The step of the four two-dot background fetches that falls on `dot` (2 to 257 and
    /// 321 to 337).
    fn background_fetch(&mut self, dot: i32, cart: Cart<'_, '_>) {
        match (dot - 1) & 0x07 {
            0 => {
                self.load_background_shifters();
                self.nametable_latch = self.read_vram_inner(
                    0x2000 | (self.v & 0x0FFF),
                    PpuBusAccess::Render,
                    ChrFetch::Background,
                    cart,
                );
            }
            2 => self.attribute_latch = self.fetch_attribute(cart),
            3 => self.notify_pattern_address(0, cart),
            4 => self.pattern_low_latch = self.fetch_pattern(0, cart),
            5 => self.notify_pattern_address(8, cart),
            6 => self.pattern_high_latch = self.fetch_pattern(8, cart),
            7 => self.increment_coarse_x(),
            _ => {}
        }
    }

    fn fetch_attribute(&self, cart: Cart<'_, '_>) -> u8 {
        let address = 0x23C0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07);
        let attribute =
            self.read_vram_inner(address, PpuBusAccess::Render, ChrFetch::Background, cart);
        let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
        (attribute >> shift) & 0x03
    }

    fn fetch_pattern(&self, plane: u16, cart: Cart<'_, '_>) -> u8 {
        self.read_vram_inner(
            self.pattern_address(plane),
            PpuBusAccess::Render,
            ChrFetch::Background,
            cart,
        )
    }

    fn pattern_address(&self, plane: u16) -> u16 {
        let table: u16 = if (self.control & 0x10) != 0 {
            0x1000
        } else {
            0x0000
        };
        table + (u16::from(self.nametable_latch) * 16) + ((self.v >> 12) & 0x07) + plane
    }

    /// Puts the address of the coming pattern fetch on the cartridge's bus, one dot before
    /// the byte is latched.
    ///
    /// Each fetch takes two dots and the address is driven for both of them, so a board
    /// watching A12 sees it one dot before the data arrives. That dot is measurable: it is
    /// what `mmc3_test_2` test 4 pins down, to PPU-clock accuracy, for both $2000 = $08 and
    /// $2000 = $10 (REQ-CORE-114).
    fn notify_pattern_address(&self, plane: u16, cart: Cart<'_, '_>) {
        self.notify_address_bus(self.pattern_address(plane), cart);
    }

    fn shift_background(&mut self) {
        self.pattern_shift_low <<= 1;
        self.pattern_shift_high <<= 1;
        self.attribute_shift_low <<= 1;
        self.attribute_shift_high <<= 1;
    }

    fn load_background_shifters(&mut self) {
        self.pattern_shift_low =
            (self.pattern_shift_low & 0xFF00) | u16::from(self.pattern_low_latch);
        self.pattern_shift_high =
            (self.pattern_shift_high & 0xFF00) | u16::from(self.pattern_high_latch);
        self.attribute_shift_low = (self.attribute_shift_low & 0xFF00)
            | if (self.attribute_latch & 1) != 0 {
                0xFF
            } else {
                0x00
            };
        self.attribute_shift_high = (self.attribute_shift_high & 0xFF00)
            | if (self.attribute_latch & 2) != 0 {
                0xFF
            } else {
                0x00
            };
    }

    fn increment_coarse_x(&mut self) {
        if (self.v & 0x001F) == 0x001F {
            self.v = (self.v & !0x001F) ^ 0x0400;
        } else {
            self.v += 1;
        }
    }

    fn increment_y(&mut self) {
        if (self.v & 0x7000) != 0x7000 {
            self.v += 0x1000;
            return;
        }

        self.v &= !0x7000;
        let mut coarse_y = (self.v >> 5) & 0x1F;
        if coarse_y == 29 {
            coarse_y = 0;
            self.v ^= 0x0800;
        } else if coarse_y == 31 {
            // Coarse Y 30 and 31 address the attribute tables; the hardware wraps without
            // switching nametable (REQ-CORE-038, notes).
            coarse_y = 0;
        } else {
            coarse_y += 1;
        }

        self.v = (self.v & !0x03E0) | (coarse_y << 5);
    }

    fn transfer_address_x(&mut self) {
        self.v = (self.v & !0x041F) | (self.t & 0x041F);

        // R2-005: the horizontal reload of dot 257 — and the vertical increment of dot 256
        // that precedes it — belong to the next line, not to the row captured for this one.
        self.v_row_next = self.v;
    }

    fn transfer_address_y(&mut self) {
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);

        // The vertical reload only happens on the pre-render line, which captures nothing,
        // so it may as well take effect at once.
        self.v_row_start = self.v;
        self.v_row_next = self.v;
    }

    // ---------------------------------------------------------------- sprites

    /// Sprite evaluation for the *next* line: dots 1 to 64 clear the secondary OAM, dots 65
    /// to 256 scan the 64 sprites (nesdev, "PPU sprite evaluation"). The overflow flag
    /// follows the hardware, byte-increment bug included.
    fn run_sprite_evaluation(&mut self, dot: i32) {
        if dot == 65 {
            self.eval_sprite = i32::from(self.oam_address >> 2);
            self.eval_byte = i32::from(self.oam_address & 0x03);
            self.eval_found = 0;
            self.overflow_copy = 0;
            self.eval_finished = false;
            self.sprite_zero_found = false;
        }

        if (dot & 1) == 1 {
            // Odd dots read primary OAM.
            #[allow(clippy::cast_sign_loss)] // Masked to 0..=255.
            let index = (((self.eval_sprite * 4) + self.eval_byte) & 0xFF) as usize;
            self.oam_eval_latch = self.oam[index];
            return;
        }

        if self.eval_finished {
            return;
        }

        if self.eval_found < SPRITES_PER_LINE_I32 {
            #[allow(clippy::cast_sign_loss)] // Both are small and non-negative.
            let slot = ((self.eval_found * 4) + self.eval_byte) as usize;
            self.secondary_oam[slot] = self.oam_eval_latch;
            if self.eval_byte == 0 {
                if self.in_sprite_range(self.oam_eval_latch) {
                    if self.eval_sprite == 0 {
                        self.sprite_zero_found = true;
                    }

                    self.eval_byte = 1;
                } else {
                    self.next_sprite();
                }
            } else {
                self.eval_byte += 1;
                if self.eval_byte == 4 {
                    self.eval_byte = 0;
                    self.eval_found += 1;
                    self.next_sprite();
                }
            }

            return;
        }

        // Eight sprites found: the ninth in-range sprite raises the overflow flag, and the
        // hardware then increments both the sprite and the byte index, which is the famous
        // false-positive bug.
        if self.overflow_copy > 0 {
            self.overflow_copy -= 1;
            self.eval_byte += 1;
            if self.eval_byte == 4 {
                self.eval_byte = 0;
                self.next_sprite();
            }
        } else if self.in_sprite_range(self.oam_eval_latch) {
            self.sprite_overflow = true;
            self.overflow_copy = 3;
            self.eval_byte = 1;
        } else {
            self.eval_byte = (self.eval_byte + 1) & 0x03;
            self.next_sprite();
        }
    }

    fn next_sprite(&mut self) {
        self.eval_sprite += 1;
        if self.eval_sprite >= 64 {
            self.eval_sprite = 0;
            self.eval_finished = true;
        }
    }

    fn in_sprite_range(&self, sprite_y: u8) -> bool {
        let height = if (self.control & 0x20) != 0 { 16 } else { 8 };
        let sprite_y = i32::from(sprite_y);
        self.scanline >= sprite_y && self.scanline < sprite_y + height
    }

    /// Dots 257 to 320 fetch the patterns of the eight sprites of the next line and hold
    /// OAMADDR at 0 (nesdev).
    fn run_sprite_fetches(&mut self, dot: i32, cart: Cart<'_, '_>) {
        self.oam_address = 0;

        if dot == 257 {
            self.sprites_loaded = self.eval_found;
            self.sprite_zero_loaded = self.sprite_zero_found;
            self.sprite_line_stale = true;
        }

        #[allow(clippy::cast_sign_loss)] // dot is 257..=320 here, so the slot is 0..=7.
        let slot = (((dot - 257) >> 3) & 0x07) as usize;
        let step = (dot - 257) & 0x07;

        if step == 3 || step == 5 {
            // The address of the coming fetch is already on the bus (see
            // notify_pattern_address).
            let address = self.sprite_pattern_address(slot) + if step == 3 { 0 } else { 8 };
            self.notify_address_bus(address, cart);
            return;
        }

        if step != 4 && step != 6 {
            return;
        }

        let address = self.sprite_pattern_address(slot);
        let data = self.read_vram_inner(
            address + if step == 4 { 0 } else { 8 },
            PpuBusAccess::Render,
            ChrFetch::Sprite,
            cart,
        );

        self.sprite_line_stale = true;
        if step == 4 {
            self.sprite_pattern_low[slot] = data;
            self.sprite_attribute[slot] = self.secondary_oam[(slot * 4) + 2];
            self.sprite_x[slot] = self.secondary_oam[(slot * 4) + 3];
        } else {
            self.sprite_pattern_high[slot] = data;
        }
    }

    fn sprite_pattern_address(&self, slot: usize) -> u16 {
        let sprite_y = self.secondary_oam[slot * 4];
        let tile = u16::from(self.secondary_oam[(slot * 4) + 1]);
        let attribute = self.secondary_oam[(slot * 4) + 2];
        let tall = (self.control & 0x20) != 0;
        let flip_v = (attribute & 0x80) != 0;

        let mut row = self.scanline - i32::from(sprite_y);
        if i32::try_from(slot).unwrap_or(i32::MAX) >= self.sprites_loaded {
            // The unused slots still fetch, so that boards watching A12 see the same bus
            // activity as on hardware.
            row = 0;
        }

        // `row & 7` and `row & 15` are in range even for a negative row, as in the C#.
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let row = (row & 0x0F) as u16;

        if !tall {
            let table: u16 = if (self.control & 0x08) != 0 {
                0x1000
            } else {
                0x0000
            };
            let line = if flip_v { 7 - (row & 7) } else { row & 7 };
            return table + (tile * 16) + line;
        }

        let tall_table: u16 = if (tile & 1) != 0 { 0x1000 } else { 0x0000 };
        let mut index = tile & 0xFE;
        let mut tall_row = if flip_v { 15 - row } else { row };
        if tall_row >= 8 {
            index |= 1;
            tall_row -= 8;
        }

        tall_table + (index * 16) + tall_row
    }

    // ---------------------------------------------------------------- composition

    /// Rebuilds [`Ppu2C02::sprite_line`] from the eight loaded sprite slots: per column, the
    /// first slot (in OAM order) whose pixel is opaque, as the hardware's priority
    /// multiplexer picks it. The slots only change during dots 257 to 320, so the line is
    /// the same whichever pixel of 1 to 256 asks for it.
    fn rebuild_sprite_line(&mut self) {
        self.sprite_line_stale = false;
        self.sprite_line.fill(0);

        #[allow(clippy::cast_sign_loss)] // 0..=8.
        let loaded = (self.sprites_loaded.clamp(0, SPRITES_PER_LINE_I32)) as usize;
        for slot in 0..loaded {
            let attribute = self.sprite_attribute[slot];
            let low = self.sprite_pattern_low[slot];
            let high = self.sprite_pattern_high[slot];
            let zero = slot == 0 && self.sprite_zero_loaded;
            let flags = SPRITE_OPAQUE
                | ((attribute & 0x03) << 2)
                | if (attribute & 0x20) != 0 {
                    SPRITE_BEHIND
                } else {
                    0
                }
                | if zero { SPRITE_ZERO } else { 0 };
            let start = usize::from(self.sprite_x[slot]);
            for (column, cell) in self.sprite_line.iter_mut().skip(start).take(8).enumerate() {
                if *cell != 0 {
                    continue;
                }

                let bit = if (attribute & 0x40) != 0 {
                    column
                } else {
                    7 - column
                };
                let pixel = ((low >> bit) & 1) | (((high >> bit) & 1) << 1);
                if pixel != 0 {
                    *cell = flags | pixel;
                }
            }
        }
    }

    /// Draws dots `first` to `last` (1 to 256) of a visible line with rendering off: the
    /// backdrop, as [`Ppu2C02::render_pixel`] draws it when both layers are off.
    pub(super) fn fill_backdrop(&mut self, first: i32, last: i32) {
        let mut color = self.palette[0];
        if (self.mask & 0x01) != 0 {
            color &= 0x30;
        }

        #[allow(clippy::cast_sign_loss)] // Visible line, dots 1..=256.
        let (line, first, last) = (self.scanline as usize, first as usize, last as usize);
        let start = line * SCREEN_WIDTH + first - 1;
        if let Some(row) = self.pixels.get_mut(start..start + (last + 1 - first)) {
            row.fill(color);
        }
    }

    fn render_pixel(&mut self, x: usize) {
        let mask = self.mask;
        let x = x & 0xFF;

        let mut background = 0u8;
        if (mask & 0x08) != 0 && (x >= 8 || (mask & 0x02) != 0) {
            let shift = 15 - (self.fine_x & 0x07);
            let pixel = ((self.pattern_shift_low >> shift) & 1)
                | (((self.pattern_shift_high >> shift) & 1) << 1);
            let palette = ((self.attribute_shift_low >> shift) & 1)
                | (((self.attribute_shift_high >> shift) & 1) << 1);
            #[allow(clippy::cast_possible_truncation)] // Two 2-bit fields.
            let entry = ((palette << 2) | pixel) as u8;
            background = entry;
        }

        let mut sprite = 0u8;
        if (mask & 0x10) != 0 && (x >= 8 || (mask & 0x04) != 0) {
            if self.sprite_line_stale {
                self.rebuild_sprite_line();
            }

            sprite = self.sprite_line[x];
        }

        let background_opaque = (background & 0x03) != 0;
        // Sprite 0 hit needs both layers on, a non-transparent pixel in each, and never
        // happens at x = 255 (nesdev, "PPU sprite 0 hits").
        if (sprite & SPRITE_ZERO) != 0 && background_opaque && x != SCREEN_WIDTH - 1 {
            self.sprite_zero_hit = true;
        }

        // Entries $00, $04, ... of a palette all show the backdrop: a transparent pixel of
        // either layer resolves to $3F00, and an opaque one never hits the $3F1x mirrors.
        let entry = if (sprite & SPRITE_OPAQUE) != 0
            && (!background_opaque || (sprite & SPRITE_BEHIND) == 0)
        {
            0x10 | (sprite & 0x0F)
        } else if background_opaque {
            background
        } else {
            0
        };

        let mut color = self.palette[usize::from(entry & 0x1F)];

        // The greyscale bit of PPUMASK is a video effect: it changes the image but neither
        // the tile records nor the palette handed to the engine (REQ-CORE-047).
        if (mask & 0x01) != 0 {
            color &= 0x30;
        }

        #[allow(clippy::cast_sign_loss)] // Visible line, 0..240.
        let line = self.scanline as usize;
        if let Some(pixel) = self.pixels.get_mut((line * SCREEN_WIDTH) + x) {
            *pixel = color;
        }
    }
}

/// What PPUMASK and fine X decide for every pixel of a span (they cannot change inside one).
#[allow(clippy::struct_excessive_bools)] // The four independent layer switches of PPUMASK.
struct SpanPixels {
    background: bool,
    background_left: bool,
    sprites: bool,
    sprites_left: bool,
    /// $30 with the greyscale bit (REQ-CORE-047), else $3F (palette entries are 6 bits).
    color_mask: u8,
    /// The background shifters' bit of the current pixel.
    shift: i32,
}

impl SpanPixels {
    fn of(mask: u8, fine_x: i32) -> Self {
        Self {
            background: (mask & 0x08) != 0,
            background_left: (mask & 0x02) != 0,
            sprites: (mask & 0x10) != 0,
            sprites_left: (mask & 0x04) != 0,
            color_mask: if (mask & 0x01) != 0 { 0x30 } else { 0x3F },
            shift: 15 - (fine_x & 0x07),
        }
    }
}

/// [`Ppu2C02::sprite_line`] cell flags; the low four bits are the palette (bits 2-3) and the
/// 2-bit pixel, 0 when no sprite covers the column.
const SPRITE_OPAQUE: u8 = 0x40;
const SPRITE_BEHIND: u8 = 0x20;
const SPRITE_ZERO: u8 = 0x10;
