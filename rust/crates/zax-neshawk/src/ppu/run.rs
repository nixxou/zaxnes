//! The per-dot PPU loop.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/PPU.run.cs` and the `runppu` of `PPU.cs`.
//! `impl Nes` because the PPU drives the CPU: `runppu` is what calls `nes.RunCpuOne()`.
//! MIT; see `LICENSE-BizHawk`.

use super::{DecayType, PPU_PHASE_BG, PPU_PHASE_OBJ};
use crate::nes::Nes;

/// `BitReverse.Byte8`, which the sprite fetch uses for the horizontal flip.
static BIT_REVERSE_BYTE8: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut v = i as u8;
        let mut r = 0u8;
        let mut b = 0;
        while b < 8 {
            r = (r << 1) | (v & 1);
            v >>= 1;
            b += 1;
        }
        t[i] = r;
        i += 1;
    }
    t
};

/// `byte.Bit(i)`.
#[inline]
fn bit(v: u8, i: i32) -> bool {
    (v >> i) & 1 != 0
}

impl Nes {
    /// `PPU.get_ptread`: address line 3 relates to the pattern table fetch occuring.
    #[inline]
    fn get_ptread(&self, par: i32) -> i32 {
        let hi = self.ppu.reg_2000.bg_pattern_hi;
        (hi << 0xC) | (par << 0x4) | self.ppu.ppur.fv
    }

    /// `PPU.Read_bgdata`.
    fn read_bgdata(&mut self, cycle: i32, i: i32) {
        let i = i as usize;
        match cycle {
            0 => {
                self.ppu.ppu_addr_temp = self.ppu.ppur.get_ntread();
                let a = self.ppu.ppu_addr_temp;
                self.ppu.bgdata[i].nt = self.ppubus_read(a, true, true);
            }
            1 => {}
            2 => {
                self.ppu.ppu_addr_temp = self.ppu.ppur.get_atread();
                let a = self.ppu.ppu_addr_temp;
                let mut at = self.ppubus_read(a, true, true);

                //modify at to get appropriate palette shift
                if (self.ppu.ppur.vt & 2) != 0 {
                    at >>= 4;
                }
                if (self.ppu.ppur.ht & 2) != 0 {
                    at >>= 2;
                }
                at &= 0x03;
                at <<= 2;
                self.ppu.bgdata[i].at = at;
            }
            3 => {}
            4 => {
                let nt = i32::from(self.ppu.bgdata[i].nt);
                self.ppu.ppu_addr_temp = self.get_ptread(nt);
                let a = self.ppu.ppu_addr_temp;
                self.ppu.bgdata[i].pt_0 = self.ppubus_read(a, true, true);
            }
            5 => {}
            6 => {
                self.ppu.ppu_addr_temp |= 8;
                let a = self.ppu.ppu_addr_temp;
                self.ppu.bgdata[i].pt_1 = self.ppubus_read(a, true, true);
            }
            _ => {}
        }
    }

    /// `PPU.runppu`: one PPU cycle, and every `cpu_sequence[cpu_step]` of them, one CPU cycle.
    pub(crate) fn runppu(&mut self) {
        //run one ppu cycle at a time so we can interact with the ppu and clockPPU at high
        //granularity
        if self.ppu.install_2006 > 0 {
            self.ppu.install_2006 -= 1;
            if self.ppu.install_2006 == 0 {
                if !self.ppu.race_2006 {
                    self.ppu.ppur.install_latches();
                } else {
                    self.ppu.race_2006_2 = true;
                }

                //normally the address isnt observed by the board till it gets clocked by a read
                //or write, but apparently this shoves it on the address bus too, or else blargg's
                //mmc3 tests don't pass — ONLY if the ppu is not rendering
                if self.ppu.ppur.status.sl >= 241 || !self.ppu.ppuon() {
                    let a = self.ppu.ppur.get_2007access();
                    self.board.address_ppu(a);
                }
            }
        }

        self.ppu.race_2006 = false;

        if self.ppu.install_2001 > 0 {
            self.ppu.install_2001 -= 1;
            if self.ppu.install_2001 == 0 {
                self.ppu.show_bg_new = self.ppu.reg_2001.show_bg != 0;
                self.ppu.show_obj_new = self.ppu.reg_2001.show_obj != 0;
            }
        }

        self.ppu.ppur.status.cycle += 1;
        self.ppu.is_even_cycle = !self.ppu.is_even_cycle;

        if self.ppu.ppur.status.cycle >= 257
            && self.ppu.ppur.status.cycle <= 320
            && self.ppu.ppur.status.sl <= 240
            && self.ppu.ppuon()
        {
            self.ppu.reg_2003 = 0;
        }

        // Here we execute a CPU instruction if enough PPU cycles have passed
        self.ppu.cpu_stepcounter += 1;
        if self.ppu.cpu_stepcounter == i32::from(self.cpu_sequence[self.ppu.cpu_step as usize]) {
            self.ppu.cpu_step += 1;
            if self.ppu.cpu_step == 5 {
                self.ppu.cpu_step = 0;
            }
            self.ppu.cpu_stepcounter = 0;

            // this is where the CPU instruction is called
            self.run_cpu_one();

            // decay the ppu bus, approximating real behaviour
            self.ppu.ppu_open_bus_decay(DecayType::None);

            // Check for NMIs
            if self.ppu.nmi_pending_instructions > 0 {
                self.ppu.nmi_pending_instructions -= 1;
                if self.ppu.nmi_pending_instructions <= 0 {
                    self.cpu.nmi = true;
                }
            }
        }

        if self.ppu.has_clock_ppu {
            self.board.clock_ppu();
        }
        self.ppu.total_cycles += 1;
        self.cpu.ext_ppu_cycle += 1;
    }

    /// `PPU.TickPPU_VBL`.
    pub(crate) fn tick_ppu_vbl(&mut self) {
        let pre = self.ppu.pre_nmi_lines;
        let post = self.ppu.post_nmi_lines;

        if self.ppu.ppur.status.cycle == 0 && self.ppu.ppur.status.sl == 241 + pre {
            self.ppu.nmi_destiny =
                self.ppu.reg_2000.vblank_nmi_gen != 0 && self.ppu.reg2002_vblank_active;
            if self.ppu.cpu_stepcounter == 2 {
                self.ppu.nmi_offset = 1;
            } else if self.ppu.cpu_stepcounter == 1 {
                self.ppu.nmi_offset = 2;
            } else {
                self.ppu.nmi_offset = 0;
            }
        } else if self.ppu.ppur.status.cycle <= 2 && self.ppu.nmi_destiny {
            self.ppu.nmi_destiny &=
                self.ppu.reg_2000.vblank_nmi_gen != 0 && self.ppu.reg2002_vblank_active;
        } else if self.ppu.ppur.status.cycle == 3 + self.ppu.nmi_offset
            && self.ppu.ppur.status.sl == 241 + pre
        {
            if self.ppu.nmi_destiny {
                self.cpu.nmi = true;
            }
            self.board.at_vsync_nmi();
        }

        if self.ppu.ppur.status.cycle == 340 && self.ppu.ppur.status.sl == 241 + pre + post - 1 {
            self.ppu.reg2002_vblank_clear_pending = true;
            self.ppu.idle_synch = !self.ppu.idle_synch;
            self.ppu.reg2002_objhit = false;
            self.ppu.reg2002_objoverflow = false;
        }

        self.runppu(); // note cycle ticks inside runppu

        if self.ppu.ppur.status.cycle == 341 {
            if self.ppu.reg2002_vblank_clear_pending {
                self.ppu.reg2002_vblank_active = false;
                self.ppu.reg2002_vblank_clear_pending = false;
            }

            self.ppu.ppur.status.cycle = 0;
            self.ppu.ppur.status.sl += 1;
            if self.ppu.ppur.status.sl == 241 + pre + post {
                self.ppu.do_vbl = false;
                self.ppu.ppur.status.sl = 0;
                self.ppu.do_active_sl = true;
            }
        }
    }

    /// The sprite-evaluation half of `TickPPU_active`'s visible dots.
    fn sprite_evaluation(&mut self) {
        let p = &mut self.ppu;
        if p.sprite_eval_cycle < 64 {
            // the first 64 cycles of each scanline are used to initialize secondary OAM
            if !p.is_even_cycle {
                p.soam[p.soam_index as usize] = 0xFF;
                p.soam_index += 1;
            }
            return;
        }

        // otherwise, scan through OAM and test if sprites are in range
        if p.sprite_eval_cycle == 64 {
            p.soam_index = 0;
            p.oam_index = i32::from(p.reg_2003);
        }

        if p.oam_index >= 256 {
            p.oam_index = 0;
            p.sprite_eval_write = false;
        }

        if p.is_even_cycle {
            p.read_value = p.oam[(p.oam_index & 0xFF) as usize];
            return;
        }

        if !p.sprite_eval_write {
            // if we don't write sprites anymore, just scan through the oam
            p.read_value = p.soam[((p.soam_index * 4) & 0x1F) as usize];
            p.oam_index += 4;
            return;
        }

        let read_value = i32::from(p.read_value);
        let in_range = p.yp >= read_value && p.yp < read_value + p.sprite_height;

        if p.soam_index < 8 {
            //look for sprites
            if p.spr_true_count == 0 {
                p.soam[(p.soam_index * 4) as usize] = p.read_value;
            }

            if in_range && p.spr_true_count == 0 {
                //a flag gets set if sprite zero is in range
                if p.oam_index == i32::from(p.reg_2003) {
                    p.sprite_zero_in_range = true;
                }
                p.oam_index += 1;
                p.spr_true_count += 1;
            } else if p.spr_true_count > 0 && p.spr_true_count < 4 {
                p.soam[(p.soam_index * 4 + p.spr_true_count) as usize] = p.read_value;

                p.oam_index += 1;
                p.spr_true_count += 1;
                if p.spr_true_count == 4 {
                    p.soam_index += 1;
                    // The X coordinate makes the same checks as the Y position to see if
                    // "read_value" is in range of the scanline.
                    if !in_range {
                        // and if it isn't, clear the lower 2 bits of the OAM index.
                        // Bit 8 is used to check if the OAM address overflows.
                        p.oam_index &= 0x1FC;
                    }
                    if p.soam_index == 8 {
                        // oam_index could be pathologically misaligned at this point, so we have
                        // to find the next nearest actual sprite
                        p.oam_index_aux = (p.oam_index % 4) * 4;
                    }

                    p.spr_true_count = 0;
                }
            } else {
                // This object is out of the range of the scanline
                p.oam_index += 4;
                p.oam_index &= 0x1FC;
            }
        } else {
            if in_range && p.ppuon() {
                p.reg2002_objoverflow = true;
            }

            if in_range && p.spr_true_count == 0 {
                p.spr_true_count += 1;
                p.oam_index += 1;
            } else if p.spr_true_count > 0 && p.spr_true_count < 4 {
                p.oam_index += 1;
                p.spr_true_count += 1;
                if p.spr_true_count == 4 {
                    if !in_range {
                        p.oam_index &= 0x1FC;
                    }
                    // soam_index no longer increments now that Secondary OAM is full.
                    p.spr_true_count = 0;
                }
            } else {
                // glitchy increments
                p.oam_index += 1;

                if (p.oam_index & 3) != 0 {
                    p.oam_index += 4;
                }
            }

            p.read_value = p.soam[((p.soam_index * 4) & 0x1F) as usize]; //writes change to reads
        }
    }

    /// The `xp == 7` block, which both the visible and the pre-render path run verbatim.
    fn active_increment_block(&mut self) {
        let p = &mut self.ppu;
        p.ppur.increment_hsc();

        if p.ppur.status.cycle == 256 && p.ppu_was_on {
            p.ppur.increment_vs();
        }

        if p.race_2006_2 {
            if p.ppur.status.cycle == 256 {
                p.ppur.fv &= p.ppur._fv;
                p.ppur.v &= p.ppur._v;
                p.ppur.h &= p.ppur._h;
                p.ppur.vt &= p.ppur._vt;
                p.ppur.ht &= p.ppur._ht;
            } else {
                p.ppur.fv = p.ppur._fv;
                p.ppur.v = p.ppur._v;
                p.ppur.h &= p.ppur._h;
                p.ppur.vt = p.ppur._vt;
                p.ppur.ht &= p.ppur._ht;
            }
        }

        p.ppu_was_on = false;
    }

    /// `PPU.TickPPU_active`.
    pub(crate) fn tick_ppu_active(&mut self) {
        if self.ppu.ppur.status.cycle < 256 {
            if self.ppu.ppur.status.cycle == 0 {
                self.ppu.ppur.status.cycle = 0;

                self.ppu.spr_true_count = 0;
                self.ppu.soam_index = 0;
                self.ppu.oam_index_aux = 0;
                self.ppu.oam_index = 0;
                self.ppu.is_even_cycle = true;
                self.ppu.sprite_eval_write = true;
                self.ppu.sprite_zero_go = self.ppu.sprite_zero_in_range;

                self.ppu.sprite_zero_in_range = false;

                self.ppu.yp = self.ppu.ppur.status.sl - 1;
                self.ppu.ppuphase = PPU_PHASE_BG;

                // "If PPUADDR is not less then 8 when rendering starts, the first 8 bytes in OAM
                // are written to from the current location of PPUADDR"
                if self.ppu.ppur.status.sl == 0
                    && self.ppu.ppuon()
                    && self.ppu.reg_2003 >= 8
                    && self.ppu.region == super::Region::Ntsc
                {
                    for i in 0..8usize {
                        self.ppu.oam[i] = self.ppu.oam[(self.ppu.reg_2003 & 0xF8) as usize + i];
                    }
                }

                // set up intial values to use later
                self.ppu.yp_shift = self.ppu.yp << 8;
                self.ppu.xt = 0;
                self.ppu.xp = 0;

                self.ppu.sprite_eval_cycle = 0;

                self.ppu.xstart = self.ppu.xt << 3;
                self.ppu.target = self.ppu.yp_shift + self.ppu.xstart;
                self.ppu.rasterpos = self.ppu.xstart;

                self.ppu.sprite_height = if self.ppu.reg_2000.obj_size_16 != 0 {
                    16
                } else {
                    8
                };

                //check all the conditions that can cause things to render in these 8px
                self.ppu.renderspritenow = self.ppu.show_obj_new
                    && (self.ppu.xt > 0 || self.ppu.reg_2001.show_obj_leftmost != 0);
            }

            if self.ppu.ppur.status.sl != 0 {
                /////////////////////////////////////////////
                // Sprite Evaluation Start / End
                /////////////////////////////////////////////
                self.sprite_evaluation();

                let mut pixel = 0i32;
                let mut pixelcolor = i32::from(self.ppu.palram[pixel as usize]);

                //process the current clock's worth of bg data fetching
                //this needs to be split into 8 pieces or else exact sprite 0 hitting wont work
                if self.ppu.ppuon() {
                    let (xp, xt) = (self.ppu.xp, self.ppu.xt);
                    self.read_bgdata(xp, xt + 2);
                } else {
                    //according to qeed's doc, use palette 0 or $2006's value if it is & 0x3Fxx
                    let addr = self.ppu.ppur.get_2007access();
                    if (addr & 0x3F00) == 0x3F00 {
                        pixel = addr & 0x1F;
                    }
                    pixelcolor = i32::from(self.ppu.palram[pixel as usize]);
                    pixelcolor |= 0x8000;
                }

                //generate the BG data
                if self.ppu.show_bg_new
                    && (self.ppu.xt > 0 || self.ppu.reg_2001.show_bg_leftmost != 0)
                {
                    let bgtile = (self.ppu.rasterpos + self.ppu.ppur.fh) >> 3;
                    let pt_0 = self.ppu.bgdata[bgtile as usize].pt_0;
                    let pt_1 = self.ppu.bgdata[bgtile as usize].pt_1;
                    // the C#'s own precedence: `7 - (rasterpos + fh) & 7`, i.e. the subtraction
                    // binds tighter than the `&`.
                    let sel = (7 - (self.ppu.rasterpos + self.ppu.ppur.fh)) & 7;
                    pixel = ((i32::from(pt_0) >> sel) & 1) | (((i32::from(pt_1) >> sel) & 1) << 1);
                    if pixel != 0 {
                        pixel |= i32::from(self.ppu.bgdata[bgtile as usize].at);
                    }
                    pixelcolor = i32::from(self.ppu.palram[pixel as usize]);
                }

                if !self.ppu.disp_background {
                    pixelcolor = 0x8000;
                }

                //check if the pixel has a sprite in it
                let idx = (self.ppu.xt * 8 + self.ppu.xp) as usize;
                if self.ppu.sl_sprites[256 + idx] != 0 && self.ppu.renderspritenow {
                    let s = i32::from(self.ppu.sl_sprites[idx]);
                    let mut spixel = i32::from(self.ppu.sl_sprites[256 + idx]);
                    let temp_attr = i32::from(self.ppu.sl_sprites[512 + idx]);

                    //spritehit: is it sprite#0, and is the bg pixel nonzero?
                    self.ppu.reg2002_objhit |= self.ppu.sprite_zero_go
                        && s == 0
                        && pixel != 0
                        && self.ppu.rasterpos < 255
                        && self.ppu.show_bg_new
                        && self.ppu.show_obj_new;
                    //priority handling, if in front of BG:
                    let drawsprite = !((temp_attr & 0x20) != 0 && (pixel & 3) != 0);
                    if drawsprite && self.ppu.disp_sprites {
                        //bring in the palette bits and palettize
                        spixel |= (temp_attr & 3) << 2;
                        //save it for use in the framebuffer
                        pixelcolor = i32::from(self.ppu.palram[(0x10 + spixel) as usize]);
                    }
                }

                self.runppu();

                if self.ppu.xp == 6 && self.ppu.ppuon() {
                    self.ppu.ppu_was_on = true;
                    if self.ppu.ppur.status.cycle == 255 {
                        self.ppu.race_2006 = true;
                    }
                }

                if self.ppu.xp == 7 && self.ppu.ppuon() {
                    self.active_increment_block();
                }

                self.ppu.race_2006_2 = false;

                let row_check = self.ppu.xt * 8 + self.ppu.xp;
                self.ppu.pipeline(pixelcolor, row_check);
                self.ppu.target += 1;

                // clear out previous sprites from scanline buffer
                self.ppu.sl_sprites[256 + idx] = 0;

                // end of visible part of the scanline
                self.ppu.sprite_eval_cycle += 1;
                self.ppu.xp += 1;
                self.ppu.rasterpos += 1;

                if self.ppu.xp == 8 {
                    self.ppu.xp = 0;
                    self.ppu.xt += 1;

                    self.ppu.xstart = self.ppu.xt << 3;
                    self.ppu.target = self.ppu.yp_shift + self.ppu.xstart;
                    self.ppu.rasterpos = self.ppu.xstart;

                    self.ppu.sprite_height = if self.ppu.reg_2000.obj_size_16 != 0 {
                        16
                    } else {
                        8
                    };

                    self.ppu.renderspritenow = self.ppu.show_obj_new
                        && (self.ppu.xt > 0 || self.ppu.reg_2001.show_obj_leftmost != 0);
                }

                if self.ppu.ppur.status.cycle > 63 && self.ppu.ppu_was_on_spr && !self.ppu.ppuon() {
                    self.ppu.reg_2003 = self.ppu.reg_2003.wrapping_add(1);
                }

                self.ppu.ppu_was_on_spr = self.ppu.ppuon();
            } else {
                // if scanline is the pre-render line, we just read BG data
                let (xp, xt) = (self.ppu.xp, self.ppu.xt);
                self.read_bgdata(xp, xt + 2);

                self.runppu();

                if self.ppu.xp == 6 && self.ppu.ppuon() {
                    self.ppu.ppu_was_on = true;
                    if self.ppu.ppur.status.cycle == 255 {
                        self.ppu.race_2006 = true;
                    }
                }

                if self.ppu.xp == 7 && self.ppu.ppuon() {
                    self.active_increment_block();
                }

                self.ppu.race_2006_2 = false;

                self.ppu.xp += 1;

                if self.ppu.xp == 8 {
                    self.ppu.xp = 0;
                    self.ppu.xt += 1;
                }
            }
        } else if self.ppu.ppur.status.cycle < 320 {
            self.tick_ppu_active_sprite_fetch();
        } else if self.ppu.ppur.status.cycle < 336 {
            if self.ppu.ppur.status.cycle == 320 {
                self.ppu.ppuphase = PPU_PHASE_BG;
                self.ppu.xt = 0;
                self.ppu.xp = 0;
            }

            // if scanline is the pre-render line, we just read BG data
            let (xp, xt) = (self.ppu.xp, self.ppu.xt);
            self.read_bgdata(xp, xt);

            self.runppu();

            if self.ppu.xp == 6 && self.ppu.ppuon() {
                self.ppu.ppu_was_on = true;
            }

            if self.ppu.xp == 7 && self.ppu.ppuon() {
                if !self.ppu.race_2006 {
                    self.ppu.ppur.increment_hsc();
                }

                if self.ppu.ppur.status.cycle == 256 && !self.ppu.race_2006 {
                    self.ppu.ppur.increment_vs();
                }

                self.ppu.ppu_was_on = false;
            }

            self.ppu.xp += 1;

            if self.ppu.xp == 8 {
                self.ppu.xp = 0;
                self.ppu.xt += 1;
            }
        } else if self.ppu.ppur.status.cycle < 340 {
            if self.ppu.ppur.status.cycle == 339 {
                self.ppu.even_odd_destiny = self.ppu.ppuon();
            }

            self.runppu();
        } else {
            // After memory access 170, the PPU simply rests for 4 cycles before repeating the
            // whole pixel/scanline rendering process. If the scanline being rendered is the very
            // first one on every second frame, then this delay simply doesn't exist.
            if self.ppu.ppur.status.sl == 0
                && self.ppu.idle_synch
                && self.ppu.even_odd_destiny
                && self.ppu.chopdot
            {
                self.ppu.ppur.status.cycle += 1; // increment cycle without running ppu
            } else {
                self.runppu();
            }

            self.ppu.ppur.status.cycle = 0;
            self.ppu.ppur.status.sl += 1;

            if self.ppu.ppur.status.sl == 241 {
                self.ppu.do_active_sl = false;
                self.ppu.do_pre_vbl = true;
            }
        }
    }

    /// Dots 256-319 of `TickPPU_active`: the transfer into the temporary OAM and the sprite
    /// pattern fetches.
    fn tick_ppu_active_sprite_fetch(&mut self) {
        if self.ppu.ppur.status.cycle == 256 {
            // do the more then 8 sprites stuff here where it is convenient
            self.ppu.soam_index_aux = 8;

            if self.ppu.allow_more_than_eight_sprites {
                while self.ppu.oam_index_aux < 64 && self.ppu.soam_index_aux < 64 {
                    //look for sprites
                    let oi = self.ppu.oam_index_aux as usize;
                    let si = self.ppu.soam_index_aux as usize;
                    self.ppu.soam[si * 4] = self.ppu.oam[oi * 4];
                    let y = i32::from(self.ppu.oam[oi * 4]);
                    if self.ppu.yp >= y && self.ppu.yp < y + self.ppu.sprite_height {
                        self.ppu.soam[si * 4 + 1] = self.ppu.oam[oi * 4 + 1];
                        self.ppu.soam[si * 4 + 2] = self.ppu.oam[oi * 4 + 2];
                        self.ppu.soam[si * 4 + 3] = self.ppu.oam[oi * 4 + 3];
                        self.ppu.soam_index_aux += 1;
                        self.ppu.oam_index_aux += 1;
                    } else {
                        self.ppu.oam_index_aux += 1;
                    }
                }
            }

            self.ppu.soam_index_prev = self.ppu.soam_index_aux;

            if self.ppu.soam_index_prev > 8 && !self.ppu.allow_more_than_eight_sprites {
                self.ppu.soam_index_prev = 8;
            }

            self.ppu.ppuphase = PPU_PHASE_OBJ;

            self.ppu.sprite_height = if self.ppu.reg_2000.obj_size_16 != 0 {
                16
            } else {
                8
            };

            self.ppu.s = 0;
            self.ppu.ppu_aux_index = 0;

            self.ppu.junksprite = !self.ppu.ppuon();

            self.load_t_oam_from_soam();
        }

        match self.ppu.ppu_aux_index {
            0 => {
                //8x16 sprite handling:
                if self.ppu.reg_2000.obj_size_16 != 0 {
                    let bank = (self.ppu.pattern_number & 1) << 12;
                    self.ppu.pattern_number &= !1;
                    self.ppu.pattern_number |= (self.ppu.line >> 3) & 1;
                    self.ppu.pattern_address = (self.ppu.pattern_number << 4) | bank;
                } else {
                    self.ppu.pattern_address =
                        (self.ppu.pattern_number << 4) | (self.ppu.reg_2000.obj_pattern_hi << 12);
                }

                //offset into the pattern for the current line.
                self.ppu.pattern_address += self.ppu.line & 7;

                let a = self.ppu.ppur.get_ntread();
                self.ppubus_read(a, true, true);

                self.ppu.read_value = self.ppu.t_oam[self.ppu.s as usize].oam_y;
                self.runppu();
            }
            1 => {
                if self.ppu.ppur.status.sl == 0
                    && self.ppu.ppur.status.cycle == 305
                    && self.ppu.ppuon()
                {
                    self.ppu.ppur.install_latches();

                    self.ppu.read_value = self.ppu.t_oam[self.ppu.s as usize].oam_ind;
                    self.runppu();
                } else if self.ppu.ppur.status.sl != 0
                    && self.ppu.ppur.status.cycle == 257
                    && self.ppu.ppuon()
                {
                    if self.ppu.target <= 61441 && self.ppu.target > 0 && self.ppu.s == 0 {
                        self.ppu.pipeline(0, 256); //  last pipeline call option 1 of 2
                        self.ppu.target += 1;
                    }

                    //at 257: 3d world runner is ugly if we do this at 256
                    if self.ppu.ppuon() {
                        self.ppu.ppur.install_h_latches();
                    }
                    self.ppu.race_2006_2 = false;
                    self.ppu.read_value = self.ppu.t_oam[self.ppu.s as usize].oam_ind;
                    self.runppu();
                } else {
                    if self.ppu.target <= 61441 && self.ppu.target > 0 && self.ppu.s == 0 {
                        self.ppu.pipeline(0, 256); //  last pipeline call option 2 of 2
                        self.ppu.target += 1;
                    }

                    self.ppu.read_value = self.ppu.t_oam[self.ppu.s as usize].oam_ind;
                    self.runppu();
                }
            }
            2 => {
                let a = self.ppu.ppur.get_atread();
                self.ppubus_read(a, true, true); //at or nt?
                self.ppu.read_value = self.ppu.t_oam[self.ppu.s as usize].oam_attr;
                self.runppu();
            }
            3 => {
                self.ppu.read_value = self.ppu.t_oam[self.ppu.s as usize].oam_x;
                self.runppu();
            }
            4 => {
                // if the PPU is off, we don't put anything on the bus
                if self.ppu.junksprite {
                    let a = self.ppu.pattern_address;
                    self.ppubus_read(a, true, false);
                    self.runppu();
                } else {
                    self.ppu.temp_addr = self.ppu.pattern_address;
                    let a = self.ppu.temp_addr;
                    let v = self.ppubus_read(a, true, true);
                    self.ppu.t_oam[self.ppu.s as usize].patterns_0 = v;
                    self.ppu.read_value = self.ppu.t_oam[self.ppu.s as usize].oam_x;
                    self.runppu();
                }
            }
            5 => self.runppu(),
            6 => {
                if self.ppu.junksprite {
                    let a = self.ppu.pattern_address;
                    self.ppubus_read(a, true, false);
                    self.runppu();
                } else {
                    self.ppu.temp_addr += 8;
                    let a = self.ppu.temp_addr;
                    let v = self.ppubus_read(a, true, true);
                    self.ppu.t_oam[self.ppu.s as usize].patterns_1 = v;
                    self.ppu.read_value = self.ppu.t_oam[self.ppu.s as usize].oam_x;
                    self.runppu();
                }
            }
            _ => {
                // case 7
                if self.ppu.junksprite {
                    self.runppu();
                } else {
                    self.runppu();

                    let s = self.ppu.s as usize;
                    // hflip
                    if (self.ppu.t_oam[s].oam_attr & 0x40) == 0 {
                        self.ppu.t_oam[s].patterns_0 =
                            BIT_REVERSE_BYTE8[self.ppu.t_oam[s].patterns_0 as usize];
                        self.ppu.t_oam[s].patterns_1 =
                            BIT_REVERSE_BYTE8[self.ppu.t_oam[s].patterns_1 as usize];
                    }

                    // if the sprite's attribute is 0xFF then this indicates a non-existent sprite
                    if self.ppu.t_oam[s].oam_attr == 0xFF {
                        self.ppu.t_oam[s].patterns_0 = 0;
                        self.ppu.t_oam[s].patterns_1 = 0;
                    }
                }
            }
        }

        self.ppu.ppu_aux_index += 1;
        if self.ppu.ppu_aux_index == 8 {
            // now that we have a sprite, we can fill in the next scanline's sprite pixels with it
            if self.ppu.s < self.ppu.soam_index_prev
                && self.ppu.ppur.status.sl != 0
                && self.ppu.ppur.status.sl != 240
            {
                self.fill_sl_sprites(self.ppu.s);
            }

            self.ppu.ppu_aux_index = 0;
            self.ppu.s += 1;

            if self.ppu.s < 8 {
                self.ppu.junksprite = !self.ppu.ppuon();
                self.load_t_oam_from_soam();
            } else if self.ppu.soam_index_aux > 8 {
                // repeat all the above steps for more then 8 sprites but don't run any cycles
                self.extra_sprites();
            }
        }
    }

    /// The four `t_oam[s]` loads plus `line` and `patternNumber`, which `TickPPU_active` does in
    /// two places with the same six statements.
    fn load_t_oam_from_soam(&mut self) {
        let s = self.ppu.s as usize;
        self.ppu.t_oam[s].oam_y = self.ppu.soam[s * 4];
        self.ppu.t_oam[s].oam_ind = self.ppu.soam[s * 4 + 1];
        self.ppu.t_oam[s].oam_attr = self.ppu.soam[s * 4 + 2];
        self.ppu.t_oam[s].oam_x = self.ppu.soam[s * 4 + 3];

        self.ppu.line = self.ppu.yp - i32::from(self.ppu.t_oam[s].oam_y);
        if (self.ppu.t_oam[s].oam_attr & 0x80) != 0 {
            //vflip
            self.ppu.line = self.ppu.sprite_height - self.ppu.line - 1;
        }

        self.ppu.pattern_number = i32::from(self.ppu.t_oam[s].oam_ind);
    }

    /// The `sl_sprites` fill, shared by the 8-sprite path and the extra-sprites path.
    fn fill_sl_sprites(&mut self, s: i32) {
        let p = &mut self.ppu;
        let su = s as usize;
        let temp_x = i32::from(p.t_oam[su].oam_x);
        let mut i = 0i32;
        while (temp_x + i) < 256 && i < 8 {
            let at = (temp_x + i) as usize;
            if p.sl_sprites[256 + at] == 0 {
                let b0 = bit(p.t_oam[su].patterns_0, i);
                let b1 = bit(p.t_oam[su].patterns_1, i);
                if b0 || b1 {
                    let mut spixel = i32::from(b0);
                    spixel |= if b1 { 2 } else { 0 };

                    p.sl_sprites[at] = s as u8;
                    p.sl_sprites[256 + at] = spixel as u8;
                    p.sl_sprites[512 + at] = p.t_oam[su].oam_attr;
                }
            }
            i += 1;
        }
    }

    /// The sprites past the eighth, when `AllowMoreThanEightSprites` is on: all the same steps
    /// "but don't run any cycles".
    fn extra_sprites(&mut self) {
        let count = self.ppu.soam_index_aux;
        for s in 8..count {
            let su = s as usize;
            self.ppu.t_oam[su].oam_y = self.ppu.soam[su * 4];
            self.ppu.t_oam[su].oam_ind = self.ppu.soam[su * 4 + 1];
            self.ppu.t_oam[su].oam_attr = self.ppu.soam[su * 4 + 2];
            self.ppu.t_oam[su].oam_x = self.ppu.soam[su * 4 + 3];

            let mut line = self.ppu.yp - i32::from(self.ppu.t_oam[su].oam_y);
            if (self.ppu.t_oam[su].oam_attr & 0x80) != 0 {
                //vflip
                line = self.ppu.sprite_height - line - 1;
            }

            let mut pattern_number = i32::from(self.ppu.t_oam[su].oam_ind);
            let mut pattern_address;

            //8x16 sprite handling:
            if self.ppu.reg_2000.obj_size_16 != 0 {
                let bank = (pattern_number & 1) << 12;
                pattern_number &= !1;
                pattern_number |= (line >> 3) & 1;
                pattern_address = (pattern_number << 4) | bank;
            } else {
                pattern_address = (pattern_number << 4) | (self.ppu.reg_2000.obj_pattern_hi << 12);
            }

            pattern_address += line & 7;

            let a = self.ppu.ppur.get_ntread();
            self.ppubus_read(a, true, false);

            let a = self.ppu.ppur.get_atread();
            self.ppubus_read(a, true, false); //at or nt?

            let mut addr = pattern_address;
            let v = self.ppubus_read(addr, true, false);
            self.ppu.t_oam[su].patterns_0 = v;

            addr += 8;
            let v = self.ppubus_read(addr, true, false);
            self.ppu.t_oam[su].patterns_1 = v;

            // hflip
            if (self.ppu.t_oam[su].oam_attr & 0x40) == 0 {
                self.ppu.t_oam[su].patterns_0 =
                    BIT_REVERSE_BYTE8[self.ppu.t_oam[su].patterns_0 as usize];
                self.ppu.t_oam[su].patterns_1 =
                    BIT_REVERSE_BYTE8[self.ppu.t_oam[su].patterns_1 as usize];
            }

            if self.ppu.t_oam[su].oam_attr == 0xFF {
                self.ppu.t_oam[su].patterns_0 = 0;
                self.ppu.t_oam[su].patterns_1 = 0;
            }

            if self.ppu.ppur.status.sl != 0 && self.ppu.ppur.status.sl != 240 {
                self.fill_sl_sprites(s);
            }
        }
    }

    /// `PPU.TickPPU_preVBL`.
    pub(crate) fn tick_ppu_pre_vbl(&mut self) {
        let pre = self.ppu.pre_nmi_lines;
        if self.ppu.ppur.status.cycle == 340 && self.ppu.ppur.status.sl == 241 + pre - 1 {
            self.ppu.reg2002_vblank_active_pending = true;
        }

        self.runppu();

        if self.ppu.ppur.status.cycle == 341 {
            self.ppu.ppur.status.cycle = 0;
            self.ppu.ppur.status.sl += 1;

            if self.ppu.ppur.status.sl == 241 + pre {
                if self.ppu.reg2002_vblank_active_pending {
                    self.ppu.reg2002_vblank_active = true;
                    self.ppu.reg2002_vblank_active_pending = false;
                }

                self.ppu.do_pre_vbl = false;
                self.ppu.do_vbl = true;

                self.ppu.ppu_init_frame();
                self.frame_is_done = true;
            }
        }
    }

    /// `PPU.NewDeadPPU`: the start-up frame, during which the NES ignores register writes.
    pub(crate) fn new_dead_ppu(&mut self) {
        if self.ppu.ppur.status.cycle == 241 * 341 - self.ppu.start_up_offset - 1 {
            self.ppu.reg2002_vblank_active_pending = true;
        }

        self.runppu();

        if self.ppu.ppur.status.cycle == 241 * 341 - self.ppu.start_up_offset {
            if self.ppu.reg2002_vblank_active_pending {
                self.ppu.reg2002_vblank_active = true;
                self.ppu.reg2002_vblank_active_pending = false;
            }

            self.ppu.ppudead -= 1;

            self.ppu.ppu_init_frame();

            self.ppu.do_vbl = true;

            self.frame_is_done = true;
        }
    }
}
