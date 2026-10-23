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
    palette_index, Cart, Ppu2C02, PpuBusAccess, SCREEN_WIDTH, SPRITES_PER_LINE_I32,
    VISIBLE_SCANLINES,
};
use crate::cartridges::ChrFetch;

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
    }

    /// Runs everything the current dot owes: the fetch pipeline, sprite evaluation and the
    /// pixel of the visible lines.
    pub(super) fn run_render_dot(&mut self, cart: Cart<'_, '_>) {
        let visible = self.scanline < VISIBLE_SCANLINES;
        let render_line = visible || self.scanline == self.pre_render_scanline();

        if render_line && self.rendering_enabled() {
            if self.dot == 1 {
                // Dots 1 to 64 clear the secondary OAM to $FF; the unused slots then make
                // the hardware fetch pattern $FF, which boards watching A12 can see.
                self.secondary_oam.fill(0xFF);
            }

            self.run_background_pipeline(cart);

            if visible {
                self.run_sprite_evaluation();
            } else if self.dot == 1 {
                // Sprite evaluation does not run on the pre-render line, so no sprite is
                // shown on line 0 (nesdev, "PPU sprite evaluation").
                self.eval_found = 0;
                self.sprite_zero_found = false;
            }

            self.run_sprite_fetches(cart);

            if self.scanline == self.pre_render_scanline() && (280..=304).contains(&self.dot) {
                self.transfer_address_y();
            }
        }

        if visible && (1..=256).contains(&self.dot) {
            #[allow(clippy::cast_sign_loss)] // dot is 1..=256 here.
            let x = (self.dot - 1) as usize;
            self.render_pixel(x);
        }
    }

    // ---------------------------------------------------------------- background

    fn run_background_pipeline(&mut self, cart: Cart<'_, '_>) {
        if (2..=257).contains(&self.dot) || (321..=337).contains(&self.dot) {
            self.shift_background();

            match (self.dot - 1) % 8 {
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

        if self.dot == 256 {
            self.increment_y();
        } else if self.dot == 257 {
            self.transfer_address_x();
        } else if self.dot == 338 || self.dot == 340 {
            // Two more nametable fetches the hardware performs and discards; they matter
            // only for boards that watch the PPU address bus.
            self.nametable_latch = self.read_vram_inner(
                0x2000 | (self.v & 0x0FFF),
                PpuBusAccess::Render,
                ChrFetch::Background,
                cart,
            );
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
    fn run_sprite_evaluation(&mut self) {
        if self.dot == 65 {
            self.eval_sprite = i32::from(self.oam_address >> 2);
            self.eval_byte = i32::from(self.oam_address & 0x03);
            self.eval_found = 0;
            self.overflow_copy = 0;
            self.eval_finished = false;
            self.sprite_zero_found = false;
        }

        if !(65..=256).contains(&self.dot) {
            return;
        }

        if (self.dot & 1) == 1 {
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
    fn run_sprite_fetches(&mut self, cart: Cart<'_, '_>) {
        if !(257..=320).contains(&self.dot) {
            return;
        }

        self.oam_address = 0;

        if self.dot == 257 {
            self.sprites_loaded = self.eval_found;
            self.sprite_zero_loaded = self.sprite_zero_found;
        }

        #[allow(clippy::cast_sign_loss)] // dot is 257..=320 here.
        let slot = ((self.dot - 257) >> 3) as usize;
        let step = (self.dot - 257) & 0x07;

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

    fn render_pixel(&mut self, x: usize) {
        let mut background_pixel = 0u8;
        let mut background_palette = 0u8;
        if self.rendering_enabled()
            && self.background_enabled()
            && (x >= 8 || (self.mask & 0x02) != 0)
        {
            let mux = 0x8000u16 >> self.fine_x;
            background_pixel = u8::from((self.pattern_shift_low & mux) != 0)
                | (u8::from((self.pattern_shift_high & mux) != 0) << 1);
            background_palette = u8::from((self.attribute_shift_low & mux) != 0)
                | (u8::from((self.attribute_shift_high & mux) != 0) << 1);
        }

        let mut sprite_pixel = 0u8;
        let mut sprite_palette = 0u8;
        let mut sprite_behind = false;
        let mut sprite_is_zero = false;
        if self.rendering_enabled() && self.sprites_enabled() && (x >= 8 || (self.mask & 0x04) != 0)
        {
            #[allow(clippy::cast_sign_loss)] // 0..=8.
            let loaded = self.sprites_loaded as usize;
            for slot in 0..loaded {
                let Some(column) = x.checked_sub(usize::from(self.sprite_x[slot])) else {
                    continue;
                };

                if column > 7 {
                    continue;
                }

                let attribute = self.sprite_attribute[slot];
                let bit = if (attribute & 0x40) != 0 {
                    column
                } else {
                    7 - column
                };
                let pixel = ((self.sprite_pattern_low[slot] >> bit) & 1)
                    | (((self.sprite_pattern_high[slot] >> bit) & 1) << 1);
                if pixel == 0 {
                    continue;
                }

                sprite_pixel = pixel;
                sprite_palette = attribute & 0x03;
                sprite_behind = (attribute & 0x20) != 0;
                sprite_is_zero = slot == 0 && self.sprite_zero_loaded;
                break;
            }
        }

        // Sprite 0 hit needs both layers on, a non-transparent pixel in each, and never
        // happens at x = 255 (nesdev, "PPU sprite 0 hits").
        if sprite_is_zero && sprite_pixel != 0 && background_pixel != 0 && x != SCREEN_WIDTH - 1 {
            self.sprite_zero_hit = true;
        }

        let entry: u16 = if sprite_pixel != 0 && (background_pixel == 0 || !sprite_behind) {
            0x10 + u16::from(sprite_palette * 4) + u16::from(sprite_pixel)
        } else if background_pixel != 0 {
            u16::from(background_palette * 4) + u16::from(background_pixel)
        } else {
            0
        };

        let mut color = self.palette[palette_index(0x3F00 | entry)];

        // The greyscale bit of PPUMASK is a video effect: it changes the image but neither
        // the tile records nor the palette handed to the engine (REQ-CORE-047).
        if (self.mask & 0x01) != 0 {
            color &= 0x30;
        }

        #[allow(clippy::cast_sign_loss)] // Visible line, 0..240.
        let line = self.scanline as usize;
        self.pixels[(line * SCREEN_WIDTH) + x] = color;
    }
}
