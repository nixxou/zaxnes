//! The fast core's PPU paths (lot FAST-1, `docs/design/FAST-CORE.md`).
//!
//! Step 1 of the PPU part: the visible span of a rendering line (dots 2 to 255) eight dots at a
//! time instead of one. The result is **the same machine** as [`Ppu2C02::run_visible_span`]:
//! the same pixels, the same latches, shifters, `v` and sprite evaluation state after the span,
//! so `golden-core` stays byte-identical and a fast span can stop anywhere a dot can.
//!
//! # Why eight dots at a time is exact
//!
//! Inside the span a dot does: shift the four background shifters, one step of the fetch
//! pipeline (phase `(dot - 1) & 7`: 0 reloads the shifters' low bytes and reads the nametable,
//! 2 the attribute, 4 and 6 the two pattern planes, 7 moves `v` to the next tile), one step of
//! sprite evaluation from dot 65, and one pixel. Take a group of eight dots starting at a phase-0
//! dot `d`. With `r` a shifter before dot `d` and `l` its latch, dot `d` makes it
//! `n = ((r << 1) & 0xFF00) | l`, and dot `d + j` shows bit `15 - fine_x` of `n << j`, that is bit
//! `15 - fine_x - j` of `n` (`fine_x + j` is at most 14). After the group the shifter is
//! `n << 7`. The fetches of the group only write the latches and `v`, which no pixel of the group
//! reads (the pixels read the shifters, loaded at dot `d` from the latches of the group before).
//! So the group's eight pixels come from `n`, its fetches run in their own order, and the sprite
//! evaluation, which looks at neither, runs its dots as before.

use super::{Cart, Ppu2C02, OPAQUE_PIXEL, SCREEN_WIDTH};
use crate::cartridges::{BoardKind, Cartridge};

/// `SPREAD[b]`: the eight bits of `b` spread over the eight bytes of a `u64`, the leftmost pixel
/// (bit 7) in the lowest byte, each byte 0 or 1.
const SPREAD: [u64; 256] = {
    let mut table = [0u64; 256];
    let mut byte = 0;
    while byte < 256 {
        let mut value = 0u64;
        let mut pixel = 0;
        while pixel < 8 {
            value |= (((byte >> (7 - pixel)) & 1) as u64) << (pixel * 8);
            pixel += 1;
        }
        table[byte] = value;
        byte += 1;
    }
    table
};

/// The eight 4-bit background entries (`palette << 2 | pixel`) of one byte of each of the four
/// shifters, the leftmost pixel in the lowest byte.
#[allow(clippy::inline_always)] // Once per eight pixels on the fast path.
#[inline(always)]
fn decode(low: u8, high: u8, attribute_low: u8, attribute_high: u8) -> u64 {
    SPREAD[usize::from(low)]
        | (SPREAD[usize::from(high)] << 1)
        | (SPREAD[usize::from(attribute_low)] << 2)
        | (SPREAD[usize::from(attribute_high)] << 3)
}

impl Ppu2C02 {
    /// [`Ppu2C02::run_visible_span`] for the fast core: the dots before the next phase-0 dot and
    /// after the last whole group run as before, the whole groups between run eight dots at a
    /// time. Same returns: `false`, having run nothing, when the cartridge's fetches cannot be
    /// read directly.
    pub(super) fn run_visible_span_fast<K: BoardKind>(
        &mut self,
        count: i64,
        cart: Cart<'_, '_, K>,
    ) -> bool {
        if !cart.as_deref().is_some_and(|cartridge| {
            self.fast_nametables(cartridge).is_some() && fast_chr::<K>(cartridge).is_some()
        }) {
            return false;
        }

        // Dots up to the next phase-0 dot (`dot + 1 ≡ 1 mod 8`, so `dot ≡ 0 mod 8`).
        let head = i64::from((8 - (self.dot & 7)) & 7).min(count);
        if head > 0 {
            self.run_span_dots::<K>(head, cart);
        }
        let mut groups = (count - head) / 8;
        let mut tail = count - head - groups * 8;
        // The line's last seven span dots (249 to 255, phases 0 to 6) run as a group short of
        // its phase-7 dot: its eighth pixel is x = 255, which never makes a sprite 0 hit and
        // which dot 256 paints again before anything reads it.
        let short_last = tail == 7 && i64::from(self.dot) + groups * 8 == 248;
        if short_last {
            groups += 1;
            tail = 0;
        }
        if groups > 0 {
            let mut events = A12Events::new();
            if let Some(cartridge) = cart.as_deref() {
                self.run_span_groups::<K>(groups, short_last, cartridge, &mut events);
            }
            events.emit::<K>(cart.as_deref_mut());
        }
        if tail > 0 {
            self.run_span_dots::<K>(tail, cart);
        }
        true
    }

    /// Dots of a visible span one at a time, through the bus for a board that watches A12.
    fn run_span_dots<K: BoardKind>(&mut self, count: i64, cart: Cart<'_, '_, K>) {
        if K::A12_ONLY {
            self.run_visible_span_through_bus(count, cart);
        } else {
            let _ = self.run_visible_span::<K>(count, cart.as_deref());
        }
    }

    /// The start of each logical nametable in console VRAM, as `run_visible_span` finds them;
    /// `None` when the cartridge serves one itself.
    fn fast_nametables(&self, cartridge: &Cartridge) -> Option<[usize; 4]> {
        let layout = self.layout_of(Some(cartridge));
        let mut nametables = [0usize; 4];
        for (table, start) in (0u16..).zip(nametables.iter_mut()) {
            let page = self.nametable_page(0x2000 + (table << 10), Some(cartridge));
            if page == Cartridge::SERVED_BY_CARTRIDGE {
                return None;
            }
            *start = usize::try_from(page).map_or_else(
                |_| super::map_nametable(0x2000 + (table << 10), layout),
                |page| page << 10,
            );
        }
        Some(nametables)
    }

    /// `groups` whole groups of eight dots of a visible line, from a phase-0 dot.
    #[allow(clippy::too_many_lines)] // One loop over the groups, its locals shared.
    fn run_span_groups<K: BoardKind>(
        &mut self,
        groups: i64,
        short_last: bool,
        cartridge: &Cartridge,
        events: &mut A12Events,
    ) {
        let (Some(chr), Some(nametables)) =
            (fast_chr::<K>(cartridge), self.fast_nametables(cartridge))
        else {
            return;
        };
        let nametable_byte = |vram: &[u8], address: u16| {
            vram[nametables[usize::from((address >> 10) & 0x03)] | (usize::from(address) & 0x03FF)]
        };

        let background = (self.mask & 0x08) != 0;
        let sprites = (self.mask & 0x10) != 0;
        let color_mask = OPAQUE_PIXEL | if (self.mask & 0x01) != 0 { 0x30 } else { 0x3F };
        let fine_x = u32::from(self.fine_x & 0x07);
        #[allow(clippy::cast_sign_loss)] // Visible line, 0..240.
        let row = self.scanline as usize * SCREEN_WIDTH;

        // The colour of a background entry with no sprite over it, for the whole span.
        let mut background_colors = [0u8; 16];
        for (entry, color) in (0u8..).zip(background_colors.iter_mut()) {
            let entry = if background && (entry & 0x03) != 0 {
                entry
            } else {
                0
            };
            *color = self.palette_marked[usize::from(entry)] & color_mask;
        }
        let evaluation_from = (self.dot + 1).max(65);
        let pattern_table: u16 = if (self.control & 0x10) != 0 {
            0x1000
        } else {
            0x0000
        };
        // Fine Y cannot change inside a span: only coarse X moves until dot 256.
        let fine_y = (self.v >> 12) & 0x07;

        // The high bytes the first group's reload keeps (`(r << 1) & 0xFF00`); after it, each
        // group's high bytes are the latches the group before loaded, so only the latches are
        // decoded from then on.
        let high_byte = |shifter: u16| ((shifter >> 7) & 0xFF) as u8;
        let mut high = [
            high_byte(self.pattern_shift_low),
            high_byte(self.pattern_shift_high),
            high_byte(self.attribute_shift_low),
            high_byte(self.attribute_shift_high),
        ];
        let mut current = decode(high[0], high[1], high[2], high[3]);
        let mut loaded = [0u16; 4];

        // The sprite line cannot go stale inside a span (only the sprite fetches of dots 257 to
        // 320 make it so), so it is rebuilt here or not at all.
        if sprites && self.sprite_line_stale {
            self.rebuild_sprite_line();
        }
        // The fields the groups move, held in locals for the loop and stored back after it.
        let mut v = self.v;
        let mut nametable_latch = self.nametable_latch;
        let mut attribute_latch = self.attribute_latch;
        let mut pattern_low_latch = self.pattern_low_latch;
        let mut pattern_high_latch = self.pattern_high_latch;
        let mut clock = self.ppu_clock;
        let mut zero_hit = false;
        let palette = self.palette_marked;
        let skip_pixels = self.skip_pixels;
        #[allow(clippy::cast_sign_loss)] // The span is inside dots 2..=255.
        let mut x0 = self.dot as usize;
        let vram = &self.vram;
        let sprite_line = &self.sprite_line;
        let pixels = &mut self.pixels;

        for group in 0..groups {
            let short = short_last && group + 1 == groups;
            // The group's reload, then its eight pixels from the two tiles it shows.
            let attribute_low: u8 = if (attribute_latch & 1) != 0 {
                0xFF
            } else {
                0x00
            };
            let attribute_high: u8 = if (attribute_latch & 2) != 0 {
                0xFF
            } else {
                0x00
            };
            let latches = [
                pattern_low_latch,
                pattern_high_latch,
                attribute_low,
                attribute_high,
            ];
            for ((shifter, high), latch) in loaded.iter_mut().zip(high).zip(latches) {
                *shifter = (u16::from(high) << 8) | u16::from(latch);
            }
            let next = decode(latches[0], latches[1], latches[2], latches[3]);
            let window = if fine_x == 0 {
                current
            } else {
                (current >> (8 * fine_x)) | (next << (64 - 8 * fine_x))
            };
            current = next;
            high = latches;

            let entries = window.to_le_bytes();
            let mut cells = [0u8; 8];
            if sprites {
                cells.copy_from_slice(&sprite_line[x0..x0 + 8]);
            }
            if skip_pixels {
                // An unshown frame: no colour, but sprite 0 still hits where it would.
                if u64::from_le_bytes(cells) != 0 && background {
                    for (j, (cell, entry)) in cells.iter().zip(entries).enumerate() {
                        zero_hit |= (cell & super::render::SPRITE_ZERO) != 0
                            && (entry & 0x03) != 0
                            && x0 + j != SCREEN_WIDTH - 1;
                    }
                }
            } else {
                let mut colors = [0u8; 8];
                if u64::from_le_bytes(cells) == 0 {
                    for (color, entry) in colors.iter_mut().zip(entries) {
                        *color = background_colors[usize::from(entry & 0x0F)];
                    }
                } else {
                    for (j, color) in colors.iter_mut().enumerate() {
                        let entry = if background { entries[j] } else { 0 };
                        let (value, hit) =
                            compose_pixel(&palette, entry, cells[j], x0 + j, color_mask);
                        *color = value;
                        zero_hit |= hit;
                    }
                }
                if let Some(out) = pixels.get_mut(row + x0..row + x0 + 8) {
                    out.copy_from_slice(&colors);
                }
            }

            // The group's fetches (phases 0, 2, 4, 6, 7), into the latches and `v`.
            let nametable_address = 0x2000 | (v & 0x0FFF);
            nametable_latch = nametable_byte(vram, nametable_address);
            let address = 0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
            let shift = ((v >> 4) & 0x04) | (v & 0x02);
            attribute_latch = (nametable_byte(vram, address) >> shift) & 0x03;
            let pattern = pattern_table + (u16::from(nametable_latch) * 16) + fine_y;
            if K::A12_ONLY {
                // A12 moves twice in the group at most: down with the nametable fetch on its
                // first dot, up (with background patterns at $1000) with the low plane's address
                // three dots later. The other fetches repeat the level, which a watcher of A12
                // does not see.
                events.push(nametable_address, clock + 1, true);
                events.push(pattern, clock + 4, false);
            }
            (pattern_low_latch, pattern_high_latch) = chr.read_pair(pattern);
            if short {
                // Seven dots: the coarse X increment is dot 256's.
                x0 += 7;
                clock += 7;
                break;
            }
            if (v & 0x001F) == 0x001F {
                v = (v & !0x001F) ^ 0x0400;
            } else {
                v += 1;
            }

            x0 += 8;
            clock += 8;
        }
        self.v = v;
        self.nametable_latch = nametable_latch;
        self.attribute_latch = attribute_latch;
        self.pattern_low_latch = pattern_low_latch;
        self.pattern_high_latch = pattern_high_latch;
        self.ppu_clock = clock;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // At most 255.
        {
            self.dot = x0 as i32;
        }
        if zero_hit {
            self.sprite_zero_hit = true;
        }
        // The shifters after the last group: its reload shifted seven times more (six for a
        // short group).
        let shift = if short_last { 6 } else { 7 };
        self.pattern_shift_low = loaded[0] << shift;
        self.pattern_shift_high = loaded[1] << shift;
        self.attribute_shift_low = loaded[2] << shift;
        self.attribute_shift_high = loaded[3] << shift;

        // Sprite evaluation looks at none of the above (it reads OAM and writes the secondary
        // OAM, which no pixel of this line reads): its dots run after the groups, in order.
        if self.dot >= evaluation_from {
            self.run_sprite_evaluation_range(evaluation_from, self.dot);
        }
    }

    /// Dots `first` to `last` (both at least 65) of the sprite evaluation, as that many
    /// [`Ppu2C02::run_sprite_evaluation`] calls would run them. A whole pair of dots (odd, then
    /// even) of an aligned evaluation takes a sprite at once: one pair for a sprite not on the
    /// line, four for one that is copied. Anything else (dot 65's start, a misaligned address, a
    /// full secondary OAM, a pair cut by the range) runs dot by dot.
    #[cfg(feature = "ppu-exact-sprite-eval")]
    fn run_sprite_evaluation_range(&mut self, first: i32, last: i32) {
        let mut dot = first;
        while dot <= last {
            let pair = (dot & 1) == 1 && dot > 65 && dot < last;
            if pair && self.eval_finished {
                // The copy is over: odd dots read OAM into the buffer, even dots move the sprite
                // half and read the secondary OAM back into it, so only the last pair shows.
                let pairs = (last - dot + 1) / 2;
                self.eval_sprite = (self.eval_sprite + pairs) & 0x3F;
                #[allow(clippy::cast_sign_loss)] // Masked to 0..32.
                let slot = (self.secondary_oam_addr & 0x1F) as usize;
                self.oam_eval_latch = self.secondary_oam[slot];
                dot += pairs * 2;
                continue;
            }
            if pair
                && self.eval_byte == 0
                && !self.sprite_in_range
                && self.secondary_oam_addr < 32
                && self.secondary_oam_addr.trailing_zeros() >= 2
            {
                #[allow(clippy::cast_sign_loss)] // 0..64.
                let sprite = self.eval_sprite as usize;
                let y = self.oam[sprite * 4];
                #[allow(clippy::cast_sign_loss)] // 0..32.
                let slot = self.secondary_oam_addr as usize;
                if !self.in_sprite_range(y) || self.sprite_in_margin() {
                    // A run of sprites not on the line, one pair each: each writes its Y over the
                    // same slot and moves on; the run stops before a sprite in range, at the end
                    // of OAM or of the range.
                    let height = if (self.control & 0x20) != 0 { 16 } else { 8 };
                    let line = self.scanline;
                    let mut next = sprite;
                    let mut last_y = y;
                    loop {
                        next = (next + 1) & 0x3F;
                        dot += 2;
                        if next == 0 || dot >= last {
                            break;
                        }
                        let y = i32::from(self.oam[next * 4]);
                        if line >= y && line < y + height {
                            break;
                        }
                        last_y = self.oam[next * 4];
                    }
                    self.oam_eval_latch = last_y;
                    self.secondary_oam[slot] = last_y;
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // 0..64.
                    {
                        self.eval_sprite = next as i32;
                    }
                    self.eval_byte = 0;
                    if next == 0 {
                        self.eval_finished = true;
                    }
                    continue;
                }
                if dot + 7 <= last {
                    if dot + 1 == 66 {
                        self.sprite_zero_found = true;
                    }
                    self.secondary_oam[slot..slot + 4]
                        .copy_from_slice(&self.oam[sprite * 4..sprite * 4 + 4]);
                    #[allow(clippy::cast_possible_truncation)] // 0..64.
                    if let Some(source) = self.secondary_source.get_mut(slot >> 2) {
                        *source = sprite as u8;
                    }
                    self.oam_eval_latch = self.oam[sprite * 4 + 3];
                    self.secondary_oam_addr += 4;
                    self.eval_copy_dot = dot + 7;
                    self.eval_sprite = (self.eval_sprite + 1) & 0x3F;
                    self.eval_finished = self.eval_sprite == 0;
                    self.sprite_in_range = false;
                    self.eval_found = self.secondary_oam_addr >> 2;
                    dot += 8;
                    continue;
                }
            }
            self.run_sprite_evaluation(dot);
            dot += 1;
        }
    }

    /// The evaluation of a build without `ppu-exact-sprite-eval` has its own shape; it runs dot
    /// by dot here.
    #[cfg(not(feature = "ppu-exact-sprite-eval"))]
    fn run_sprite_evaluation_range(&mut self, first: i32, last: i32) {
        for dot in first..=last {
            self.run_sprite_evaluation(dot);
        }
    }
}

impl Ppu2C02 {
    /// The fast core's stretch of the end of a visible rendering line, when the board watches no
    /// part of the PPU bus and its CHR can be read directly: the sprite fetches of dots 258 to
    /// 320 and the two-tile prefetch of dots 321 to 336, at most `count` dots from the current
    /// one. Returns the dots run, 0 when the stretch does not apply here (the caller then steps
    /// the dot as usual). The work is the per-dot path's, without its dispatch: on these dots of
    /// a visible line `process_dot` does nothing but `run_rendering_dot`, whose arms are the
    /// sprite fetch (and the extra sprites on dot 320) and the shift-and-fetch.
    pub(super) fn run_line_end_fast<K: BoardKind>(
        &mut self,
        count: i64,
        cart: Cart<'_, '_, K>,
    ) -> i64 {
        if (K::WATCHES_BUS && !K::A12_ONLY)
            || self.scanline >= super::VISIBLE_SCANLINES
            || !self.rendering_enabled()
        {
            return 0;
        }
        if !(257..336).contains(&self.dot) {
            return 0;
        }
        let mut events = A12Events::new();
        let run = match cart.as_deref() {
            Some(cartridge) => self.run_line_end_dots::<K>(count, cartridge, &mut events),
            None => 0,
        };
        events.emit::<K>(cart.as_deref_mut());
        run
    }

    /// The body of [`Self::run_line_end_fast`], the A12 events recorded in `events`.
    fn run_line_end_dots<K: BoardKind>(
        &mut self,
        count: i64,
        cartridge: &Cartridge,
        events: &mut A12Events,
    ) -> i64 {
        if self.dot < 320 {
            let Some(chr) = fast_chr::<K>(cartridge) else {
                return 0;
            };
            #[allow(clippy::cast_possible_truncation)] // At most a line.
            let last = (i64::from(self.dot) + count).min(320) as i32;
            let first_clock = self.ppu_clock - i64::from(self.dot);
            for dot in self.dot + 1..=last {
                self.oam_address = 0;
                let step = (dot - 257) & 0x07;
                if K::A12_ONLY {
                    // The slot's garbage nametable fetch (dot 257's is the background's own),
                    // then the low plane's address: the two places A12 can move in a slot.
                    #[allow(clippy::cast_sign_loss)] // dot is 258..=320, the slot 0..=7.
                    let slot = ((dot - 257) >> 3) as usize;
                    if step == 0 {
                        events.push(
                            0x2000 | (self.v & 0x0FFF),
                            first_clock + i64::from(dot),
                            true,
                        );
                    } else if step == 3 {
                        events.push(
                            self.sprite_pattern_address(slot),
                            first_clock + i64::from(dot),
                            false,
                        );
                    }
                }
                if step == 4 || step == 6 {
                    #[allow(clippy::cast_sign_loss)] // dot is 258..=320, the slot 0..=7.
                    let slot = ((dot - 257) >> 3) as usize;
                    let address = self.sprite_pattern_address(slot);
                    let data = chr.read(address + if step == 4 { 0 } else { 8 });
                    self.sprite_line_stale = true;
                    let x = self.secondary_oam[(slot * 4) + 3];
                    let data = if self.fetched_sprite_in_margin(slot) {
                        0
                    } else {
                        data
                    };
                    if step == 4 {
                        self.sprite_pattern_low[slot] = data;
                        self.sprite_attribute[slot] = self.secondary_oam[(slot * 4) + 2];
                        self.sprite_x[slot] = x;
                    } else {
                        self.sprite_pattern_high[slot] = data;
                    }
                }
            }
            let run = last - self.dot;
            self.dot = last;
            self.ppu_clock += i64::from(run);
            if last == 320 {
                self.load_extra_sprites(Some(cartridge));
            }
            return i64::from(run);
        }

        // Dot 320 done: the two prefetch groups (dots 321-328, 329-336), whole or not at all.
        if self.dot != 320 || count < 16 {
            return 0;
        }
        if fast_chr::<K>(cartridge).is_none() || self.fast_nametables(cartridge).is_none() {
            return 0;
        }
        self.run_prefetch_groups::<K>(cartridge, events);
        16
    }

    /// The two groups of eight dots of the prefetch (321-328, 329-336): the shift-and-fetch of
    /// the span groups, with no pixel.
    fn run_prefetch_groups<K: BoardKind>(&mut self, cartridge: &Cartridge, events: &mut A12Events) {
        let (Some(chr), Some(nametables)) =
            (fast_chr::<K>(cartridge), self.fast_nametables(cartridge))
        else {
            return;
        };
        for _ in 0..2 {
            let attribute_low = if (self.attribute_latch & 1) != 0 {
                0xFF
            } else {
                0x00
            };
            let attribute_high = if (self.attribute_latch & 2) != 0 {
                0xFF
            } else {
                0x00
            };
            let low = ((self.pattern_shift_low << 1) & 0xFF00) | u16::from(self.pattern_low_latch);
            let high =
                ((self.pattern_shift_high << 1) & 0xFF00) | u16::from(self.pattern_high_latch);
            let attr_low = ((self.attribute_shift_low << 1) & 0xFF00) | attribute_low;
            let attr_high = ((self.attribute_shift_high << 1) & 0xFF00) | attribute_high;
            self.pattern_shift_low = low << 7;
            self.pattern_shift_high = high << 7;
            self.attribute_shift_low = attr_low << 7;
            self.attribute_shift_high = attr_high << 7;

            let nametable_byte = |vram: &[u8], address: u16| {
                vram[nametables[usize::from((address >> 10) & 0x03)]
                    | (usize::from(address) & 0x03FF)]
            };
            let nametable_address = 0x2000 | (self.v & 0x0FFF);
            self.nametable_latch = nametable_byte(&self.vram, nametable_address);
            let address =
                0x23C0 | (self.v & 0x0C00) | ((self.v >> 4) & 0x38) | ((self.v >> 2) & 0x07);
            let shift = ((self.v >> 4) & 0x04) | (self.v & 0x02);
            self.attribute_latch = (nametable_byte(&self.vram, address) >> shift) & 0x03;
            if K::A12_ONLY {
                events.push(nametable_address, self.ppu_clock + 1, true);
                events.push(self.pattern_address(0), self.ppu_clock + 4, false);
            }
            self.pattern_low_latch = chr.read(self.pattern_address(0));
            self.pattern_high_latch = chr.read(self.pattern_address(8));
            self.increment_coarse_x();
            self.dot += 8;
            self.ppu_clock += 8;
        }
    }
}

impl Ppu2C02 {
    /// How many turns of a `$2002` loop waiting for the sprite 0 hit cannot see it (lot FAST-1):
    /// the PPU is caught up to the current cycle, a turn lasts `period` cycles and reads `$2002`
    /// on its `read_offset`-th. A hit can only be set on a dot of a visible line whose loaded
    /// sprite 0 covers it (`sprite_x[0]` to `+ 7`), with both layers on; a turn whose read finds
    /// the PPU before the first such dot reads 0, whatever the picture. The next line's sprite 0
    /// is not known before its fetches, so the bound stops at its first dot. 0 when anything
    /// out of the ordinary is on (prepaid dots, disturbed sprite units).
    pub(crate) fn zero_hit_safe_turns(&self, period: i64, read_offset: i64) -> i64 {
        if self.sprite_zero_hit || self.prepaid_dots != 0 || self.sprite_units.disturbed {
            return 0;
        }
        if (self.mask & 0x18) != 0x18 {
            // A layer is off, and nothing in the loop can turn it on: no hit before the horizon.
            return i64::MAX;
        }
        let line = i64::from(super::DOTS_PER_SCANLINE);
        let dot = i64::from(self.dot);
        let earliest = if self.scanline < super::VISIBLE_SCANLINES {
            let sprite_x = i64::from(self.sprite_x[0]);
            // Dots `x + 1` for the sprite's columns; column 255 never hits.
            let last = (sprite_x + 8).min(255);
            if self.sprite_zero_loaded && self.sprites_loaded > 0 && dot < last {
                (dot + 1).max(sprite_x + 1) - dot
            } else {
                line - dot + 1
            }
        } else {
            // To dot 1 of line 0, one dot early for a skipped dot of an odd frame.
            let lines = i64::from(self.timing.scanlines_per_frame - self.scanline);
            (lines * line - dot).max(1)
        };
        let numerator = i64::from(self.timing.dots_per_cycle_numerator);
        let denominator = i64::from(self.timing.dots_per_cycle_denominator);
        let accumulator = i64::from(self.dot_accumulator);
        // The fewest cycles after which the PPU has run `earliest` dots, then the first turn
        // whose read comes at or after them.
        let cycles = (earliest * denominator - accumulator + numerator - 1).div_euclid(numerator);
        ((cycles - read_offset + period - 1).div_euclid(period)).max(0)
    }
}

/// The colour of pixel `x` of a span from its background entry and its sprite cell, as
/// `span_pixel` composes it, and whether it is a sprite 0 hit.
#[allow(clippy::inline_always)] // Once a pixel under a sprite on the fast path.
#[inline(always)]
fn compose_pixel(
    palette: &[u8; super::PALETTE_SIZE],
    background: u8,
    sprite: u8,
    x: usize,
    color_mask: u8,
) -> (u8, bool) {
    let background_opaque = (background & 0x03) != 0;
    let hit =
        (sprite & super::render::SPRITE_ZERO) != 0 && background_opaque && x != SCREEN_WIDTH - 1;
    let entry = if (sprite & super::render::SPRITE_OPAQUE) != 0
        && (!background_opaque || (sprite & super::render::SPRITE_BEHIND) == 0)
    {
        0x10 | (sprite & 0x0F)
    } else if background_opaque {
        background
    } else {
        0
    };
    (palette[usize::from(entry & 0x1F)] & color_mask, hit)
}

/// The CHR the fast paths read: the fetch view, or for a board that only watches A12 (whose
/// fetches read what its peek reads), the peek view; the A12 events go to it apart.
fn fast_chr<K: BoardKind>(cartridge: &Cartridge) -> Option<crate::cartridges::ChrView<'_>> {
    if K::A12_ONLY {
        cartridge.chr_peek_view_as::<K>()
    } else {
        cartridge.chr_view_as::<K>()
    }
}

/// The A12 events of a stretch of fast dots, in order, handed to the cartridge once the stretch
/// has read its CHR: an A12 watcher only sees changes of level, and nothing in the stretch reads
/// the counter it clocks.
struct A12Events {
    list: [(u16, i64, bool); 80],
    len: usize,
}

impl A12Events {
    const fn new() -> Self {
        Self {
            list: [(0, 0, false); 80],
            len: 0,
        }
    }

    fn push(&mut self, address: u16, clock: i64, read: bool) {
        if let Some(slot) = self.list.get_mut(self.len) {
            *slot = (address, clock, read);
            self.len += 1;
        }
    }

    fn emit<K: BoardKind>(&self, cartridge: Option<&mut Cartridge>) {
        let Some(cartridge) = cartridge else {
            return;
        };
        for &(address, clock, read) in &self.list[..self.len] {
            if read {
                cartridge.on_ppu_address_bus_as::<K>(address & 0x3FFF, clock);
            } else {
                cartridge.on_ppu_address_bus_without_read_as::<K>(address & 0x3FFF, clock);
            }
        }
    }
}
