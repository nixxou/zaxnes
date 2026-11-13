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
    map_nametable, Cart, Ppu2C02, PpuBusAccess, OPAQUE_PIXEL, SCREEN_WIDTH, SPRITES_PER_LINE,
    SPRITES_PER_LINE_I32, VISIBLE_SCANLINES,
};
use crate::cartridges::{Cartridge, ChrFetch};

impl Ppu2C02 {
    /// The 256 x 240 image of the frame just rendered, row-major: per pixel, the NES colour
    /// index of REQ-CORE-048 in bits 0-5 ([`super::PIXEL_COLOR`]) and the opacity bit of
    /// AMD-050 REQ-CORE-049 in bit 6 ([`super::OPAQUE_PIXEL`]). Needed by the 2D Mode of SPEC-06.
    ///
    /// The name says "with mask" because the two travel in one byte: that is the internal
    /// layout REQ-CORE-049's note allows, and the only one that costs the PPU nothing. A caller
    /// that wants the plain index image of REQ-CORE-048 masks with `PIXEL_COLOR`
    /// ([`Ppu2C02::pixel_frame_indices_into`]).
    #[must_use]
    pub fn pixel_frame_with_mask(&self) -> &[u8] {
        &self.pixels
    }

    /// The plain index image of REQ-CORE-048 — one NES colour index, 0 to 63, per pixel — copied
    /// into `out`.
    ///
    /// For a caller that needs the image without the opacity bit and is not already copying:
    /// the golden hashes and the debug view. The 2D mode does **not** use this — it wants both,
    /// and `gl/picture.rs` separates them while it resolves the picture to RGBA anyway.
    pub fn pixel_frame_indices_into(&self, out: &mut Vec<u8>) {
        out.clear();
        out.extend(self.pixels.iter().map(|pixel| pixel & super::PIXEL_COLOR));
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
        #[cfg(feature = "ppu-exact-sprite-eval")]
        {
            self.secondary_oam_addr = 0;
            self.sprite_in_range = false;
        }
        self.eval_found = 0;
        self.overflow_copy = 0;
        self.eval_finished = false;
        self.sprite_zero_found = false;
        self.sprites_loaded = 0;
        self.sprite_zero_loaded = false;
        self.sprite_line_stale = true;
        self.extra_loaded = 0;
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
        let layout = self.layout_of(Some(cartridge));
        let mut nametables = [0usize; 4];
        for (table, start) in (0u16..).zip(nametables.iter_mut()) {
            let page = self.nametable_page(0x2000 + (table << 10), Some(cartridge));
            if page == Cartridge::SERVED_BY_CARTRIDGE {
                return false;
            }

            *start = usize::try_from(page).map_or_else(
                |_| map_nametable(0x2000 + (table << 10), layout),
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

    /// [`Ppu2C02::run_visible_span`] for a cartridge whose fetches have side effects or another
    /// source (MMC2/MMC4 latches, MMC3 and Rambo1 watching A12, MMC5): the same loop, with each
    /// fetch through the full bus path ([`Ppu2C02::background_fetch`]), so the board sees the same
    /// addresses at the same PPU clock as with [`Ppu2C02::step_dot`].
    pub(super) fn run_visible_span_through_bus(&mut self, count: i64, cart: Cart<'_, '_>) {
        let pixels = SpanPixels::of(self.mask, i32::from(self.fine_x));
        #[allow(clippy::cast_sign_loss)] // Visible line, 0..240.
        let row = self.scanline as usize * SCREEN_WIDTH;

        for _ in 0..count {
            self.ppu_clock += 1;
            self.dot += 1;
            let dot = self.dot;

            self.shift_background();
            self.background_fetch(dot, cart);
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

        // AMD-050 REQ-CORE-049, and the reason the opacity mask is free: `entry` is non-zero
        // **exactly** where the pixel carries at least one opaque pixel. The sprite branch
        // yields `entry ≥ 0x10`; the background branch yields a `background` whose low two bits
        // are non-zero; the third branch is the only one that yields 0. And `background` and
        // `sprite` are held at 0 unless their layer is on and, for `x < 8`, unmasked, which are
        // the four PPUMASK conditions the requirement names. So `entry & 0x1F == 0` iff the
        // pixel is transparent, `palette_marked` carries `OPAQUE_PIXEL` on every other entry,
        // and `color_mask` is widened to keep it: the same indexed load and the same `AND` as
        // before the amendment, for both the colour and the mask.
        self.palette_marked[usize::from(entry & 0x1F)] & pixels.color_mask
    }

    /// The work of one dot of a line that renders (a visible line when `VISIBLE`, else the
    /// pre-render line) with rendering on, in the order of the nesdev timing diagram:
    /// background pipeline, sprite evaluation, sprite fetches, vertical reload.
    fn run_rendering_dot<const VISIBLE: bool>(&mut self, dot: i32, cart: Cart<'_, '_>) {
        match dot {
            1..=256 => {
                if dot == 1 {
                    if VISIBLE {
                        // Dots 1 to 64 clear the secondary OAM to $FF; the unused slots then
                        // make the hardware fetch pattern $FF, which boards watching A12 can
                        // see.
                        self.secondary_oam.fill(0xFF);
                    } else {
                        // **Not on the pre-render line**: "secondary OAM clear and sprite
                        // evaluation do not occur on the pre-render line" (nesdev, "PPU sprite
                        // evaluation"), so its sprite fetches read what line 239's evaluation
                        // left. With 8x16 sprites that decides whether A12 rises on this line,
                        // and so whether an MMC3 counts it (lot FIDELITY step 1: *Super Mario
                        // Bros. 3*'s first IRQ after power-on, one line early with the clear).
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
                if dot == 320 {
                    self.load_extra_sprites(cart.as_deref());
                }

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
    ///
    /// It goes through the **without-read** notification: this dot is the address lines alone,
    /// /RD going low on the next one, where `fetch_pattern`'s own `read_vram_inner` reports it.
    /// A board watching A12 is in both latch lists and sees no difference; a board waiting on
    /// /RD — the MMC5's in-frame detection — must not count this dot, and is spared a third of
    /// the bus events of a line for it (two of the six a tile produces).
    fn notify_pattern_address(&self, plane: u16, cart: Cart<'_, '_>) {
        self.notify_address_bus_without_read(self.pattern_address(plane), cart);
    }

    fn shift_background(&mut self) {
        // "The lowest bit (the new value shifted in on the right) is a 0 for the low bit plane,
        // and a 1 for the high bit plane" (`AccuracyCoin`, `BG Serial In`). The reload every eight
        // dots overwrites the low byte, so a line that renders normally never shows that 1; a
        // line whose reloads were skipped does.
        self.pattern_shift_low <<= 1;
        self.pattern_shift_high = (self.pattern_shift_high << 1) | 1;
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

    /// `pub(super)` for [`Ppu2C02::increment_vram_address`]: a `$2007` access made while the PPU
    /// is fetching borrows this increment and [`Self::increment_y`] at once (Q-137) — **in a build
    /// with `ppu-2007-render-read`**, which is out of `default` (Q-139). The visibility is not
    /// `#[cfg]`'d with it: the item is crate-private either way, so it is not a codegen input, and
    /// the fetch pipeline right here is this function's other caller in every build.
    pub(super) fn increment_coarse_x(&mut self) {
        if (self.v & 0x001F) == 0x001F {
            self.v = (self.v & !0x001F) ^ 0x0400;
        } else {
            self.v += 1;
        }
    }

    /// The vertical increment of dot 256. It touches **neither** `v_row_start` nor `v_row_next`:
    /// R2-005 holds the end-of-line vertical increment out of the row captured for this line, and
    /// Q-137 extends that to the copy of this increment a `$2007` access borrows — which only
    /// exists in a build with `ppu-2007-render-read`, out of `default` (Q-139). The dot-256 half
    /// of the sentence is the requirement's letter and is in every build.
    pub(super) fn increment_y(&mut self) {
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

    /// Sprite evaluation for the *next* line, **the default build's shape**: dots 1 to 64 clear
    /// the secondary OAM, dots 65 to 256 walk the 64 sprites four bytes at a time, with the range
    /// test taken on the byte whose index is 0 (nesdev, "PPU sprite evaluation"). The overflow
    /// flag follows the hardware, byte-increment bug included.
    ///
    /// This is `11407930`'s body to the instruction. The exact pipeline — the hardware's byte
    /// counter in two independently clocked halves, which wins three `AccuracyCoin` rows and
    /// measures +0.041 % to +0.133 % on PR-1's thirteen and +0.057 % to +0.127 % on all ten
    /// level-2 figures — is the `ppu-exact-sprite-eval` arm below. See `MF-4e.md` step 4 and
    /// `docs/review/BOARD-FEATURES.md`.
    #[cfg(not(feature = "ppu-exact-sprite-eval"))]
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
                if self.in_sprite_range(self.oam_eval_latch) && !self.sprite_in_margin() {
                    if self.eval_sprite == 0 {
                        self.sprite_zero_found = true;
                    }
                    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 0..64.
                    if let Some(source) = self.secondary_source.get_mut(self.eval_found as usize) {
                        *source = self.eval_sprite as u8;
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
        } else if self.in_sprite_range(self.oam_eval_latch) && !self.sprite_in_margin() {
            self.sprite_overflow = true;
            self.overflow_copy = 3;
            self.eval_byte = 1;
        } else {
            self.eval_byte = (self.eval_byte + 1) & 0x03;
            self.next_sprite();
        }
    }

    /// Sprite evaluation for the *next* line, dot by dot: dots 1 to 64 clear the secondary OAM,
    /// dots 65 to 256 scan the 64 sprites (nesdev, "PPU sprite evaluation"), odd dots reading one
    /// byte of primary OAM into the copy buffer and even dots deciding what to do with it.
    ///
    /// # Why it is a state machine over two address halves and not a loop over sprites
    ///
    /// The evaluation does not walk sprites; it walks a **byte counter** whose two halves —
    /// [`Ppu2C02::eval_sprite`] and [`Ppu2C02::eval_byte`] — are clocked by different rules at
    /// different times, while a second counter, [`Ppu2C02::secondary_oam_addr`], walks the
    /// secondary OAM in bytes. Almost every `AccuracyCoin` row on the two sprite-evaluation pages
    /// is a consequence of those three facts rather than of the nominal "copy four bytes per
    /// sprite":
    ///
    /// * the range test runs on **every even dot**, against whatever the copy buffer holds, so an
    ///   evaluation started at an address that is not a multiple of four reads its first byte at
    ///   that offset and treats it as a Y coordinate (`Misaligned OAM behavior`);
    ///   [`Ppu2C02::sprite_in_range`] latches the answer so the remaining bytes copy without
    ///   asking again;
    /// * the realignment at the end of a sprite is **conditional**: the low two bits of the
    ///   address are reset only if the last byte copied, read as a Y coordinate, is *not* itself
    ///   in range — so a misalignment can propagate from one sprite to the next;
    /// * once `secondary_oam_addr` reaches 32 the OAM write-disable signal turns the secondary
    ///   OAM's **writes into reads**, and the copy buffer starts reporting what is already there
    ///   (`Frozen OAM2 Increment`, `Misaligned OAM2 Address`);
    /// * sprite zero is whichever sprite dot 66 happens to look at (`Arbitrary Sprite zero`);
    /// * and past eight sprites the counter clocks *both* halves at once, which is the famous
    ///   overflow false positive.
    ///
    /// `third_party/mesen2`'s `NesPpu<T>::ProcessSpriteEvaluation` is the shape this follows, and
    /// the comments it carries from the nesdev page are reproduced at the branches they belong to.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    fn run_sprite_evaluation(&mut self, dot: i32) {
        if (dot & 1) == 1 {
            if dot == 65 {
                // The evaluation takes the OAM address as it stands, whatever `$2003` left in it,
                // and splits it into the two halves it then clocks separately.
                self.eval_sprite = i32::from(self.oam_address >> 2);
                self.eval_byte = i32::from(self.oam_address & 0x03);
                self.secondary_oam_addr = 0;
                self.sprite_in_range = false;
                self.eval_found = 0;
                self.overflow_copy = 0;
                self.eval_finished = false;
                self.sprite_zero_found = false;
            }

            // Odd dots read one byte of primary OAM into the copy buffer.
            #[allow(clippy::cast_sign_loss)] // Masked to 0..=255.
            let index = (((self.eval_sprite * 4) + self.eval_byte) & 0xFF) as usize;
            self.oam_eval_latch = self.oam[index];
            return;
        }

        if self.eval_finished {
            // The copy is over, but the sprite half of the counter keeps turning, and a write to
            // a full secondary OAM is a read from it.
            //
            // Full or not: once the copy is over the secondary OAM is no longer written, and the
            // even dots show the byte its address stands on — the Y of the last sprite looked at
            // when fewer than eight were found (`AccuracyCoin`, `$2004 Stress Test` 2: `$03` on
            // every even dot from the overflow of the OAM address to dot 256).
            self.eval_sprite = (self.eval_sprite + 1) & 0x3F;
            self.read_back_secondary_oam();

            return;
        }

        // The range test: on every even dot, against the copy buffer, latched until the four bytes
        // of the sprite it accepted have been copied.
        if !self.sprite_in_range
            && self.in_sprite_range(self.oam_eval_latch)
            && !self.sprite_in_margin()
        {
            self.sprite_in_range = true;
        }

        if self.secondary_oam_addr >= 4 * SPRITES_PER_LINE_I32 {
            self.evaluate_with_a_full_secondary_oam();
            return;
        }

        #[allow(clippy::cast_sign_loss)] // 0..32 by the test above.
        let slot = self.secondary_oam_addr as usize;
        self.secondary_oam[slot] = self.oam_eval_latch;

        if !self.sprite_in_range {
            // Nothing to copy: on to the next sprite, realigned.
            self.next_sprite();
            return;
        }

        // **Sprite zero is the first sprite the evaluation looks at, not OAM index 0**
        // (`AccuracyCoin`'s `Arbitrary Sprite zero`; mesen2 sets `_sprite0Added` on `_cycle == 66`
        // with the same comment, "this happens even if this isn't actually the first sprite in OAM
        // (i.e because OAMADDR was not 0 when evaluation started)"). Dot 66 is the first even dot
        // of the evaluation, so it is the dot that judges the first Y coordinate read — whichever
        // sprite `$2003` left the counter on.
        if dot == 66 {
            self.sprite_zero_found = true;
        }

        // The widescreen extension needs to know which primary sprite each slot came from, and a
        // slot begins where the byte counter is a multiple of four. `trailing_zeros() >= 2` is
        // `& 3 == 0` written the way `clippy::verbose_bit_mask` asks for, and is exact here
        // because 0 has 32 trailing zeros.
        if self.secondary_oam_addr.trailing_zeros() >= 2 {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 0..8, 0..64.
            if let Some(source) = self
                .secondary_source
                .get_mut((self.secondary_oam_addr >> 2) as usize)
            {
                *source = self.eval_sprite as u8;
            }
        }

        self.eval_byte += 1;
        self.secondary_oam_addr += 1;
        if self.eval_byte >= 4 {
            self.eval_sprite = (self.eval_sprite + 1) & 0x3F;
            self.eval_byte = 0;
            if self.eval_sprite == 0 {
                self.eval_finished = true;
            }
        }

        // The end of a slot is read off the **secondary** counter and not off `eval_byte`, which
        // mesen2 records as required "to replicate a hardware bug noticed in
        // oam_flicker_test_reenable when disabling & re-enabling rendering on a single scanline".
        if self.secondary_oam_addr.trailing_zeros() >= 2 {
            self.sprite_in_range = false;
            self.eval_found = self.secondary_oam_addr >> 2;

            // A misaligned evaluation would normally resync here and read the next entry's first
            // byte as its Y. It only does so if the last byte copied, read as a Y coordinate, is
            // **not** itself in range for this line — otherwise the low two bits stand and the
            // next sprite is misread as well.
            if self.eval_byte != 0 && !self.in_sprite_range(self.oam_eval_latch) {
                self.eval_byte = 0;
            }
        }
    }

    /// An even dot of the evaluation once eight sprites have been found: the ninth in-range sprite
    /// raises the overflow flag, and the hardware then clocks **both** halves of the address
    /// counter, which is the famous false positive.
    ///
    /// Out of line on purpose. It runs on no dot at all of a line carrying eight sprites or fewer,
    /// which is almost every line of almost every game, so inlining it would only lengthen the
    /// body that every dot of every visible line does pay for (MF-3b rule 4).
    #[cfg(feature = "ppu-exact-sprite-eval")]
    #[inline(never)]
    fn evaluate_with_a_full_secondary_oam(&mut self) {
        // "As seen above, a side effect of the OAM write disable signal is to turn writes to the
        // secondary OAM into reads from it."
        self.read_back_secondary_oam();

        if !self.sprite_in_range {
            // Not on this line: the evaluation bug, both halves at once.
            self.eval_sprite = (self.eval_sprite + 1) & 0x3F;
            self.eval_byte = (self.eval_byte + 1) & 0x03;
            if self.eval_sprite == 0 {
                self.eval_finished = true;
            }

            return;
        }

        self.sprite_overflow = true;
        self.eval_byte += 1;
        if self.eval_byte == 4 {
            self.eval_sprite = (self.eval_sprite + 1) & 0x3F;
            self.eval_byte = 0;
        }

        if self.overflow_copy == 0 {
            self.overflow_copy = 3;
        } else {
            self.overflow_copy -= 1;
            if self.overflow_copy == 0 {
                // "After it finishes 'fetching' this sprite (and setting the overflow flag), it
                // realigns back at the beginning of this line and then continues here on the next
                // sprite."
                self.eval_finished = true;
                self.eval_byte = 0;
            }
        }
    }

    /// A write to a full secondary OAM is a read from it, into the copy buffer.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    fn read_back_secondary_oam(&mut self) {
        #[allow(clippy::cast_sign_loss)] // Masked to 0..32.
        let slot = (self.secondary_oam_addr & 0x1F) as usize;
        self.oam_eval_latch = self.secondary_oam[slot];
    }

    /// On to the next sprite with the byte half realigned — the evaluation's commonest step, a
    /// sprite not on this line costing one even dot and nothing more. The copy ends when the
    /// sprite half wraps back to 0, which is how an evaluation that started misaligned stops
    /// part-way through a sprite.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    fn next_sprite(&mut self) {
        self.eval_sprite = (self.eval_sprite + 1) & 0x3F;
        self.eval_byte = 0;
        if self.eval_sprite == 0 {
            self.eval_finished = true;
        }
    }

    /// `11407930`'s own body: the byte index is the caller's business here, so this only moves the
    /// sprite index and stops the copy when it runs off the end of OAM.
    #[cfg(not(feature = "ppu-exact-sprite-eval"))]
    fn next_sprite(&mut self) {
        self.eval_sprite += 1;
        if self.eval_sprite >= 64 {
            self.eval_sprite = 0;
            self.eval_finished = true;
        }
    }

    /// Whether the sprite being evaluated is one the widescreen extension's high byte puts in a
    /// margin: not on the screen, it takes no place among the line's eight (nor raises the
    /// overflow), so that the sprites on the screen all show.
    fn sprite_in_margin(&self) -> bool {
        #[allow(clippy::cast_sign_loss)] // Masked to 0..64.
        let sprite = (self.eval_sprite & 63) as usize;
        let x = self.oam[(sprite * 4) + 3];
        !(0..256).contains(&self.wide.sprite_x(sprite, x))
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
            // The address of the coming fetch is already on the bus, /RD still high (see
            // notify_pattern_address).
            let address = self.sprite_pattern_address(slot) + if step == 3 { 0 } else { 8 };
            self.notify_address_bus_without_read(address, cart);
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
        // The widescreen extension: a sprite whose high byte puts it in a margin is not on the
        // screen; the fetch happens all the same (the bus activity of the hardware).
        let x = self.secondary_oam[(slot * 4) + 3];
        let in_margin = i32::try_from(slot).is_ok_and(|slot| slot < self.sprites_loaded)
            && !(0..256).contains(
                &self
                    .wide
                    .sprite_x(usize::from(self.secondary_source[slot]), x),
            );
        let data = if in_margin { 0 } else { data };
        if step == 4 {
            self.sprite_pattern_low[slot] = data;
            self.sprite_attribute[slot] = self.secondary_oam[(slot * 4) + 2];
            self.sprite_x[slot] = x;
        } else {
            self.sprite_pattern_high[slot] = data;
        }
    }

    /// The sprites of the next line past the eight the hardware loaded, when the picture has
    /// no limit (see [`super::SpriteLimit`]): those in range after the eighth, up to the end of
    /// the OAM where the evaluation stops. Their patterns are read without touching the
    /// cartridge, which saw the hardware's fetches only.
    fn load_extra_sprites(&mut self, cartridge: Option<&Cartridge>) {
        self.extra_loaded = 0;
        if self.sprites_loaded < SPRITES_PER_LINE_I32
            || self.sprite_limit == super::SpriteLimit::Kept
            || (self.sprite_limit == super::SpriteLimit::Smart && self.line_masks_sprites())
        {
            return;
        }

        let first = usize::from(self.secondary_source[SPRITES_PER_LINE - 1]) + 1;
        for n in first..64 {
            let y = self.oam[n * 4];
            let x = self.oam[(n * 4) + 3];
            if !self.in_sprite_range(y) || !(0..256).contains(&self.wide.sprite_x(n, x)) {
                continue;
            }

            let tile = u16::from(self.oam[(n * 4) + 1]);
            let attribute = self.oam[(n * 4) + 2];
            let address = self.sprite_row_address(tile, attribute, self.scanline - i32::from(y));
            let slot = self.extra_loaded;
            self.extra_pattern_low[slot] = self.peek_vram(address, ChrFetch::Sprite, cartridge);
            self.extra_pattern_high[slot] =
                self.peek_vram(address + 8, ChrFetch::Sprite, cartridge);
            self.extra_attribute[slot] = attribute;
            self.extra_x[slot] = x;
            self.extra_loaded += 1;
        }
        self.sprite_line_stale = true;
    }

    /// Whether a game hides sprites on the next line with the limit: eight sprites or more in
    /// range at the same position one after the other in OAM (other sprites out of range may
    /// lie between them). Mesen's test: eight sprites stacked on one spot have no other use.
    fn line_masks_sprites(&self) -> bool {
        let mut last = None;
        let mut run = 0;
        for n in 0..64 {
            let y = self.oam[n * 4];
            if !self.in_sprite_range(y) {
                continue;
            }

            let position = (y, self.oam[(n * 4) + 3]);
            if last == Some(position) {
                run += 1;
                if run >= SPRITES_PER_LINE {
                    return true;
                }
            } else {
                last = Some(position);
                run = 1;
            }
        }
        false
    }

    fn sprite_pattern_address(&self, slot: usize) -> u16 {
        let sprite_y = self.secondary_oam[slot * 4];
        let tile = u16::from(self.secondary_oam[(slot * 4) + 1]);
        let attribute = self.secondary_oam[(slot * 4) + 2];

        let mut row = self.scanline - i32::from(sprite_y);
        if i32::try_from(slot).unwrap_or(i32::MAX) >= self.sprites_loaded {
            // The unused slots still fetch, so that boards watching A12 see the same bus
            // activity as on hardware.
            row = 0;
        }
        self.sprite_row_address(tile, attribute, row)
    }

    /// The address of the low plane of row `row` of a sprite of tile `tile`.
    fn sprite_row_address(&self, tile: u16, attribute: u8, row: i32) -> u16 {
        let tall = (self.control & 0x20) != 0;
        let flip_v = (attribute & 0x80) != 0;

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
    /// the same whichever pixel of 1 to 256 asks for it. The sprites past the eighth, when the
    /// limit is removed, come after them.
    fn rebuild_sprite_line(&mut self) {
        self.sprite_line_stale = false;
        self.sprite_line.fill(0);

        #[allow(clippy::cast_sign_loss)] // 0..=8.
        let loaded = (self.sprites_loaded.clamp(0, SPRITES_PER_LINE_I32)) as usize;
        for slot in 0..loaded {
            let zero = slot == 0 && self.sprite_zero_loaded;
            paint_sprite(
                &mut self.sprite_line,
                self.sprite_x[slot],
                self.sprite_attribute[slot],
                (
                    self.sprite_pattern_low[slot],
                    self.sprite_pattern_high[slot],
                ),
                zero,
            );
        }
        for slot in 0..self.extra_loaded {
            paint_sprite(
                &mut self.sprite_line,
                self.extra_x[slot],
                self.extra_attribute[slot],
                (self.extra_pattern_low[slot], self.extra_pattern_high[slot]),
                false,
            );
        }
    }

    /// Draws dots `first` to `last` (1 to 256) of a visible line with rendering off: the
    /// backdrop, as [`Ppu2C02::render_pixel`] draws it when both layers are off.
    pub(super) fn fill_backdrop(&mut self, first: i32, last: i32) {
        // AMD-050 REQ-CORE-049: rendering is off here, so no opaque pixel is produced and the
        // bit stays 0 — entry 0 of `palette`, not of `palette_marked`, is the right source. The
        // amendment's note settles the hardware "palette background" case the same way (it is
        // rendering disabled, so the bit is 0).
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

        // AMD-050 REQ-CORE-049: the shadow palette, as in `span_pixel` — `entry` is non-zero
        // exactly where the pixel is opaque, so the bit comes with the colour.
        let mut color = self.palette_marked[usize::from(entry & 0x1F)];

        // The greyscale bit of PPUMASK is a video effect: it changes the image but neither
        // the tile records nor the palette handed to the engine (REQ-CORE-047). The mask keeps
        // bit 6, which is not a colour.
        if (mask & 0x01) != 0 {
            color &= 0x30 | OPAQUE_PIXEL;
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
    /// $30 with the greyscale bit (REQ-CORE-047), else $3F (palette entries are 6 bits), in both
    /// cases **plus** `OPAQUE_PIXEL`: the mask of AMD-050 REQ-CORE-049 rides in bit 6 of the
    /// stored byte and the greyscale effect must not clear it.
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
            color_mask: OPAQUE_PIXEL | if (mask & 0x01) != 0 { 0x30 } else { 0x3F },
            shift: 15 - (fine_x & 0x07),
        }
    }
}

/// [`Ppu2C02::sprite_line`] cell flags; the low four bits are the palette (bits 2-3) and the
/// 2-bit pixel, 0 when no sprite covers the column.
const SPRITE_OPAQUE: u8 = 0x40;
const SPRITE_BEHIND: u8 = 0x20;
const SPRITE_ZERO: u8 = 0x10;

/// Paints a sprite's row into `line`, at `x`, under the sprites painted before it (the first
/// opaque pixel of a column wins, as the hardware's priority multiplexer picks it).
fn paint_sprite(
    line: &mut [u8; SCREEN_WIDTH],
    x: u8,
    attribute: u8,
    (low, high): (u8, u8),
    zero: bool,
) {
    let flags = SPRITE_OPAQUE
        | ((attribute & 0x03) << 2)
        | if (attribute & 0x20) != 0 {
            SPRITE_BEHIND
        } else {
            0
        }
        | if zero { SPRITE_ZERO } else { 0 };
    for (column, cell) in line.iter_mut().skip(usize::from(x)).take(8).enumerate() {
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
