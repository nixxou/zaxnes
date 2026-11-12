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
        let hi = self.b.ppu.reg_2000.bg_pattern_hi;
        (hi << 0xC) | (par << 0x4) | self.b.ppu.ppur.fv
    }

    /// `PPU.Read_bgdata`.
    fn read_bgdata(&mut self, cycle: i32, i: i32) {
        let i = i as usize;
        match cycle {
            0 => {
                self.b.ppu.ppu_addr_temp = self.b.ppu.ppur.get_ntread();
                let a = self.b.ppu.ppu_addr_temp;
                self.b.ppu.bgdata[i].nt = self.b.ppubus_read(a, true, true);
            }
            1 => {}
            2 => {
                self.b.ppu.ppu_addr_temp = self.b.ppu.ppur.get_atread();
                let a = self.b.ppu.ppu_addr_temp;
                let mut at = self.b.ppubus_read(a, true, true);

                //modify at to get appropriate palette shift
                if (self.b.ppu.ppur.vt & 2) != 0 {
                    at >>= 4;
                }
                if (self.b.ppu.ppur.ht & 2) != 0 {
                    at >>= 2;
                }
                at &= 0x03;
                at <<= 2;
                self.b.ppu.bgdata[i].at = at;
            }
            3 => {}
            4 => {
                let nt = i32::from(self.b.ppu.bgdata[i].nt);
                self.b.ppu.ppu_addr_temp = self.get_ptread(nt);
                let a = self.b.ppu.ppu_addr_temp;
                self.b.ppu.bgdata[i].pt_0 = self.b.ppubus_read(a, true, true);
            }
            5 => {}
            6 => {
                self.b.ppu.ppu_addr_temp |= 8;
                let a = self.b.ppu.ppu_addr_temp;
                self.b.ppu.bgdata[i].pt_1 = self.b.ppubus_read(a, true, true);
            }
            _ => {}
        }
    }

    /// `PPU.runppu`: one PPU cycle, and every `cpu_sequence[cpu_step]` of them, one CPU cycle.
    pub(crate) fn runppu(&mut self) {
        //run one ppu cycle at a time so we can interact with the ppu and clockPPU at high
        //granularity
        if self.b.ppu.install_2006 > 0 {
            self.b.ppu.install_2006 -= 1;
            if self.b.ppu.install_2006 == 0 {
                if !self.b.ppu.race_2006 {
                    self.b.ppu.ppur.install_latches();
                } else {
                    self.b.ppu.race_2006_2 = true;
                }

                //normally the address isnt observed by the board till it gets clocked by a read
                //or write, but apparently this shoves it on the address bus too, or else blargg's
                //mmc3 tests don't pass — ONLY if the ppu is not rendering
                if self.b.ppu.ppur.status.sl >= 241 || !self.b.ppu.ppuon() {
                    let a = self.b.ppu.ppur.get_2007access();
                    self.b.board.address_ppu(a);
                }
            }
        }

        self.b.ppu.race_2006 = false;

        if self.b.ppu.install_2001 > 0 {
            self.b.ppu.install_2001 -= 1;
            if self.b.ppu.install_2001 == 0 {
                self.b.ppu.show_bg_new = self.b.ppu.reg_2001.show_bg != 0;
                self.b.ppu.show_obj_new = self.b.ppu.reg_2001.show_obj != 0;
            }
        }

        self.b.ppu.ppur.status.cycle += 1;
        self.b.ppu.is_even_cycle = !self.b.ppu.is_even_cycle;

        if self.b.ppu.ppur.status.cycle >= 257
            && self.b.ppu.ppur.status.cycle <= 320
            && self.b.ppu.ppur.status.sl <= 240
            && self.b.ppu.ppuon()
        {
            self.b.ppu.reg_2003 = 0;
        }

        // Here we execute a CPU instruction if enough PPU cycles have passed
        self.b.ppu.cpu_stepcounter += 1;
        if self.b.ppu.cpu_stepcounter
            == i32::from(self.b.cpu_sequence[self.b.ppu.cpu_step as usize])
        {
            self.b.ppu.cpu_step += 1;
            if self.b.ppu.cpu_step == 5 {
                self.b.ppu.cpu_step = 0;
            }
            self.b.ppu.cpu_stepcounter = 0;

            // this is where the CPU instruction is called
            self.run_cpu_one();

            // decay the ppu bus, approximating real behaviour
            self.b.ppu.ppu_open_bus_decay(DecayType::None);

            // Check for NMIs
            if self.b.ppu.nmi_pending_instructions > 0 {
                self.b.ppu.nmi_pending_instructions -= 1;
                if self.b.ppu.nmi_pending_instructions <= 0 {
                    self.cpu.nmi = true;
                }
            }
        }

        if self.b.ppu.has_clock_ppu {
            self.b.board.clock_ppu();
        }
        self.b.ppu.total_cycles += 1;
        self.cpu.ext_ppu_cycle += 1;
    }

    /// `PPU.TickPPU_VBL`.
    pub(crate) fn tick_ppu_vbl(&mut self) {
        let pre = self.b.ppu.pre_nmi_lines;
        let post = self.b.ppu.post_nmi_lines;

        if self.b.ppu.ppur.status.cycle == 0 && self.b.ppu.ppur.status.sl == 241 + pre {
            self.b.ppu.nmi_destiny =
                self.b.ppu.reg_2000.vblank_nmi_gen != 0 && self.b.ppu.reg2002_vblank_active;
            if self.b.ppu.cpu_stepcounter == 2 {
                self.b.ppu.nmi_offset = 1;
            } else if self.b.ppu.cpu_stepcounter == 1 {
                self.b.ppu.nmi_offset = 2;
            } else {
                self.b.ppu.nmi_offset = 0;
            }
        } else if self.b.ppu.ppur.status.cycle <= 2 && self.b.ppu.nmi_destiny {
            self.b.ppu.nmi_destiny &=
                self.b.ppu.reg_2000.vblank_nmi_gen != 0 && self.b.ppu.reg2002_vblank_active;
        } else if self.b.ppu.ppur.status.cycle == 3 + self.b.ppu.nmi_offset
            && self.b.ppu.ppur.status.sl == 241 + pre
        {
            if self.b.ppu.nmi_destiny {
                self.cpu.nmi = true;
            }
            self.b.board.at_vsync_nmi();
        }

        if self.b.ppu.ppur.status.cycle == 340 && self.b.ppu.ppur.status.sl == 241 + pre + post - 1
        {
            self.b.ppu.reg2002_vblank_clear_pending = true;
            self.b.ppu.idle_synch = !self.b.ppu.idle_synch;
            self.b.ppu.reg2002_objhit = false;
            self.b.ppu.reg2002_objoverflow = false;
        }

        self.runppu(); // note cycle ticks inside runppu

        if self.b.ppu.ppur.status.cycle == 341 {
            if self.b.ppu.reg2002_vblank_clear_pending {
                self.b.ppu.reg2002_vblank_active = false;
                self.b.ppu.reg2002_vblank_clear_pending = false;
            }

            self.b.ppu.ppur.status.cycle = 0;
            self.b.ppu.ppur.status.sl += 1;
            if self.b.ppu.ppur.status.sl == 241 + pre + post {
                self.b.ppu.do_vbl = false;
                self.b.ppu.ppur.status.sl = 0;
                self.b.ppu.do_active_sl = true;
            }
        }
    }

    /// The sprite-evaluation half of `TickPPU_active`'s visible dots.
    fn sprite_evaluation(&mut self) {
        let p = &mut self.b.ppu;
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
        let p = &mut self.b.ppu;
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
        if self.b.ppu.ppur.status.cycle < 256 {
            if self.b.ppu.ppur.status.cycle == 0 {
                self.b.ppu.ppur.status.cycle = 0;

                self.b.ppu.spr_true_count = 0;
                self.b.ppu.soam_index = 0;
                self.b.ppu.oam_index_aux = 0;
                self.b.ppu.oam_index = 0;
                self.b.ppu.is_even_cycle = true;
                self.b.ppu.sprite_eval_write = true;
                self.b.ppu.sprite_zero_go = self.b.ppu.sprite_zero_in_range;

                self.b.ppu.sprite_zero_in_range = false;

                self.b.ppu.yp = self.b.ppu.ppur.status.sl - 1;
                self.b.ppu.ppuphase = PPU_PHASE_BG;

                // "If PPUADDR is not less then 8 when rendering starts, the first 8 bytes in OAM
                // are written to from the current location of PPUADDR"
                if self.b.ppu.ppur.status.sl == 0
                    && self.b.ppu.ppuon()
                    && self.b.ppu.reg_2003 >= 8
                    && self.b.ppu.region == super::Region::Ntsc
                {
                    for i in 0..8usize {
                        self.b.ppu.oam[i] =
                            self.b.ppu.oam[(self.b.ppu.reg_2003 & 0xF8) as usize + i];
                    }
                }

                // set up intial values to use later
                self.b.ppu.yp_shift = self.b.ppu.yp << 8;
                self.b.ppu.xt = 0;
                self.b.ppu.xp = 0;

                self.b.ppu.sprite_eval_cycle = 0;

                self.b.ppu.xstart = self.b.ppu.xt << 3;
                self.b.ppu.target = self.b.ppu.yp_shift + self.b.ppu.xstart;
                self.b.ppu.rasterpos = self.b.ppu.xstart;

                self.b.ppu.sprite_height = if self.b.ppu.reg_2000.obj_size_16 != 0 {
                    16
                } else {
                    8
                };

                //check all the conditions that can cause things to render in these 8px
                self.b.ppu.renderspritenow = self.b.ppu.show_obj_new
                    && (self.b.ppu.xt > 0 || self.b.ppu.reg_2001.show_obj_leftmost != 0);
            }

            if self.b.ppu.ppur.status.sl != 0 {
                /////////////////////////////////////////////
                // Sprite Evaluation Start / End
                /////////////////////////////////////////////
                self.sprite_evaluation();

                let mut pixel = 0i32;
                let mut pixelcolor = i32::from(self.b.ppu.palram[pixel as usize]);

                //process the current clock's worth of bg data fetching
                //this needs to be split into 8 pieces or else exact sprite 0 hitting wont work
                if self.b.ppu.ppuon() {
                    let (xp, xt) = (self.b.ppu.xp, self.b.ppu.xt);
                    self.read_bgdata(xp, xt + 2);
                } else {
                    //according to qeed's doc, use palette 0 or $2006's value if it is & 0x3Fxx
                    let addr = self.b.ppu.ppur.get_2007access();
                    if (addr & 0x3F00) == 0x3F00 {
                        pixel = addr & 0x1F;
                    }
                    pixelcolor = i32::from(self.b.ppu.palram[pixel as usize]);
                    pixelcolor |= 0x8000;
                }

                //generate the BG data
                if self.b.ppu.show_bg_new
                    && (self.b.ppu.xt > 0 || self.b.ppu.reg_2001.show_bg_leftmost != 0)
                {
                    let bgtile = (self.b.ppu.rasterpos + self.b.ppu.ppur.fh) >> 3;
                    let pt_0 = self.b.ppu.bgdata[bgtile as usize].pt_0;
                    let pt_1 = self.b.ppu.bgdata[bgtile as usize].pt_1;
                    // the C#'s own precedence: `7 - (rasterpos + fh) & 7`, i.e. the subtraction
                    // binds tighter than the `&`.
                    let sel = (7 - (self.b.ppu.rasterpos + self.b.ppu.ppur.fh)) & 7;
                    pixel = ((i32::from(pt_0) >> sel) & 1) | (((i32::from(pt_1) >> sel) & 1) << 1);
                    if pixel != 0 {
                        pixel |= i32::from(self.b.ppu.bgdata[bgtile as usize].at);
                    }
                    pixelcolor = i32::from(self.b.ppu.palram[pixel as usize]);
                }

                if !self.b.ppu.disp_background {
                    pixelcolor = 0x8000;
                }

                //check if the pixel has a sprite in it
                let idx = (self.b.ppu.xt * 8 + self.b.ppu.xp) as usize;
                if self.b.ppu.sl_sprites[256 + idx] != 0 && self.b.ppu.renderspritenow {
                    let s = i32::from(self.b.ppu.sl_sprites[idx]);
                    let mut spixel = i32::from(self.b.ppu.sl_sprites[256 + idx]);
                    let temp_attr = i32::from(self.b.ppu.sl_sprites[512 + idx]);

                    //spritehit: is it sprite#0, and is the bg pixel nonzero?
                    self.b.ppu.reg2002_objhit |= self.b.ppu.sprite_zero_go
                        && s == 0
                        && pixel != 0
                        && self.b.ppu.rasterpos < 255
                        && self.b.ppu.show_bg_new
                        && self.b.ppu.show_obj_new;
                    //priority handling, if in front of BG:
                    let drawsprite = !((temp_attr & 0x20) != 0 && (pixel & 3) != 0);
                    if drawsprite && self.b.ppu.disp_sprites {
                        //bring in the palette bits and palettize
                        spixel |= (temp_attr & 3) << 2;
                        //save it for use in the framebuffer
                        pixelcolor = i32::from(self.b.ppu.palram[(0x10 + spixel) as usize]);
                    }
                }

                self.runppu();

                if self.b.ppu.xp == 6 && self.b.ppu.ppuon() {
                    self.b.ppu.ppu_was_on = true;
                    if self.b.ppu.ppur.status.cycle == 255 {
                        self.b.ppu.race_2006 = true;
                    }
                }

                if self.b.ppu.xp == 7 && self.b.ppu.ppuon() {
                    self.active_increment_block();
                }

                self.b.ppu.race_2006_2 = false;

                let row_check = self.b.ppu.xt * 8 + self.b.ppu.xp;
                self.b.ppu.pipeline(pixelcolor, row_check);
                self.b.ppu.target += 1;

                // clear out previous sprites from scanline buffer
                self.b.ppu.sl_sprites[256 + idx] = 0;

                // end of visible part of the scanline
                self.b.ppu.sprite_eval_cycle += 1;
                self.b.ppu.xp += 1;
                self.b.ppu.rasterpos += 1;

                if self.b.ppu.xp == 8 {
                    self.b.ppu.xp = 0;
                    self.b.ppu.xt += 1;

                    self.b.ppu.xstart = self.b.ppu.xt << 3;
                    self.b.ppu.target = self.b.ppu.yp_shift + self.b.ppu.xstart;
                    self.b.ppu.rasterpos = self.b.ppu.xstart;

                    self.b.ppu.sprite_height = if self.b.ppu.reg_2000.obj_size_16 != 0 {
                        16
                    } else {
                        8
                    };

                    self.b.ppu.renderspritenow = self.b.ppu.show_obj_new
                        && (self.b.ppu.xt > 0 || self.b.ppu.reg_2001.show_obj_leftmost != 0);
                }

                if self.b.ppu.ppur.status.cycle > 63
                    && self.b.ppu.ppu_was_on_spr
                    && !self.b.ppu.ppuon()
                {
                    self.b.ppu.reg_2003 = self.b.ppu.reg_2003.wrapping_add(1);
                }

                self.b.ppu.ppu_was_on_spr = self.b.ppu.ppuon();
            } else {
                // if scanline is the pre-render line, we just read BG data
                let (xp, xt) = (self.b.ppu.xp, self.b.ppu.xt);
                self.read_bgdata(xp, xt + 2);

                self.runppu();

                if self.b.ppu.xp == 6 && self.b.ppu.ppuon() {
                    self.b.ppu.ppu_was_on = true;
                    if self.b.ppu.ppur.status.cycle == 255 {
                        self.b.ppu.race_2006 = true;
                    }
                }

                if self.b.ppu.xp == 7 && self.b.ppu.ppuon() {
                    self.active_increment_block();
                }

                self.b.ppu.race_2006_2 = false;

                self.b.ppu.xp += 1;

                if self.b.ppu.xp == 8 {
                    self.b.ppu.xp = 0;
                    self.b.ppu.xt += 1;
                }
            }
        } else if self.b.ppu.ppur.status.cycle < 320 {
            self.tick_ppu_active_sprite_fetch();
        } else if self.b.ppu.ppur.status.cycle < 336 {
            if self.b.ppu.ppur.status.cycle == 320 {
                self.b.ppu.ppuphase = PPU_PHASE_BG;
                self.b.ppu.xt = 0;
                self.b.ppu.xp = 0;
            }

            // if scanline is the pre-render line, we just read BG data
            let (xp, xt) = (self.b.ppu.xp, self.b.ppu.xt);
            self.read_bgdata(xp, xt);

            self.runppu();

            if self.b.ppu.xp == 6 && self.b.ppu.ppuon() {
                self.b.ppu.ppu_was_on = true;
            }

            if self.b.ppu.xp == 7 && self.b.ppu.ppuon() {
                if !self.b.ppu.race_2006 {
                    self.b.ppu.ppur.increment_hsc();
                }

                if self.b.ppu.ppur.status.cycle == 256 && !self.b.ppu.race_2006 {
                    self.b.ppu.ppur.increment_vs();
                }

                self.b.ppu.ppu_was_on = false;
            }

            self.b.ppu.xp += 1;

            if self.b.ppu.xp == 8 {
                self.b.ppu.xp = 0;
                self.b.ppu.xt += 1;
            }
        } else if self.b.ppu.ppur.status.cycle < 340 {
            if self.b.ppu.ppur.status.cycle == 339 {
                self.b.ppu.even_odd_destiny = self.b.ppu.ppuon();
            }

            self.runppu();
        } else {
            // After memory access 170, the PPU simply rests for 4 cycles before repeating the
            // whole pixel/scanline rendering process. If the scanline being rendered is the very
            // first one on every second frame, then this delay simply doesn't exist.
            if self.b.ppu.ppur.status.sl == 0
                && self.b.ppu.idle_synch
                && self.b.ppu.even_odd_destiny
                && self.b.ppu.chopdot
            {
                self.b.ppu.ppur.status.cycle += 1; // increment cycle without running ppu
            } else {
                self.runppu();
            }

            self.b.ppu.ppur.status.cycle = 0;
            self.b.ppu.ppur.status.sl += 1;

            if self.b.ppu.ppur.status.sl == 241 {
                self.b.ppu.do_active_sl = false;
                self.b.ppu.do_pre_vbl = true;
            }
        }
    }

    /// Dots 256-319 of `TickPPU_active`: the transfer into the temporary OAM and the sprite
    /// pattern fetches.
    fn tick_ppu_active_sprite_fetch(&mut self) {
        if self.b.ppu.ppur.status.cycle == 256 {
            // do the more then 8 sprites stuff here where it is convenient
            self.b.ppu.soam_index_aux = 8;

            if self.b.ppu.allow_more_than_eight_sprites {
                while self.b.ppu.oam_index_aux < 64 && self.b.ppu.soam_index_aux < 64 {
                    //look for sprites
                    let oi = self.b.ppu.oam_index_aux as usize;
                    let si = self.b.ppu.soam_index_aux as usize;
                    self.b.ppu.soam[si * 4] = self.b.ppu.oam[oi * 4];
                    let y = i32::from(self.b.ppu.oam[oi * 4]);
                    if self.b.ppu.yp >= y && self.b.ppu.yp < y + self.b.ppu.sprite_height {
                        self.b.ppu.soam[si * 4 + 1] = self.b.ppu.oam[oi * 4 + 1];
                        self.b.ppu.soam[si * 4 + 2] = self.b.ppu.oam[oi * 4 + 2];
                        self.b.ppu.soam[si * 4 + 3] = self.b.ppu.oam[oi * 4 + 3];
                        self.b.ppu.soam_index_aux += 1;
                        self.b.ppu.oam_index_aux += 1;
                    } else {
                        self.b.ppu.oam_index_aux += 1;
                    }
                }
            }

            self.b.ppu.soam_index_prev = self.b.ppu.soam_index_aux;

            if self.b.ppu.soam_index_prev > 8 && !self.b.ppu.allow_more_than_eight_sprites {
                self.b.ppu.soam_index_prev = 8;
            }

            self.b.ppu.ppuphase = PPU_PHASE_OBJ;

            self.b.ppu.sprite_height = if self.b.ppu.reg_2000.obj_size_16 != 0 {
                16
            } else {
                8
            };

            self.b.ppu.s = 0;
            self.b.ppu.ppu_aux_index = 0;

            self.b.ppu.junksprite = !self.b.ppu.ppuon();

            self.load_t_oam_from_soam();
        }

        match self.b.ppu.ppu_aux_index {
            0 => {
                //8x16 sprite handling:
                if self.b.ppu.reg_2000.obj_size_16 != 0 {
                    let bank = (self.b.ppu.pattern_number & 1) << 12;
                    self.b.ppu.pattern_number &= !1;
                    self.b.ppu.pattern_number |= (self.b.ppu.line >> 3) & 1;
                    self.b.ppu.pattern_address = (self.b.ppu.pattern_number << 4) | bank;
                } else {
                    self.b.ppu.pattern_address = (self.b.ppu.pattern_number << 4)
                        | (self.b.ppu.reg_2000.obj_pattern_hi << 12);
                }

                //offset into the pattern for the current line.
                self.b.ppu.pattern_address += self.b.ppu.line & 7;

                let a = self.b.ppu.ppur.get_ntread();
                self.b.ppubus_read(a, true, true);

                self.b.ppu.read_value = self.b.ppu.t_oam[self.b.ppu.s as usize].oam_y;
                self.runppu();
            }
            1 => {
                if self.b.ppu.ppur.status.sl == 0
                    && self.b.ppu.ppur.status.cycle == 305
                    && self.b.ppu.ppuon()
                {
                    self.b.ppu.ppur.install_latches();

                    self.b.ppu.read_value = self.b.ppu.t_oam[self.b.ppu.s as usize].oam_ind;
                    self.runppu();
                } else if self.b.ppu.ppur.status.sl != 0
                    && self.b.ppu.ppur.status.cycle == 257
                    && self.b.ppu.ppuon()
                {
                    if self.b.ppu.target <= 61441 && self.b.ppu.target > 0 && self.b.ppu.s == 0 {
                        self.b.ppu.pipeline(0, 256); //  last pipeline call option 1 of 2
                        self.b.ppu.target += 1;
                    }

                    //at 257: 3d world runner is ugly if we do this at 256
                    if self.b.ppu.ppuon() {
                        self.b.ppu.ppur.install_h_latches();
                    }
                    self.b.ppu.race_2006_2 = false;
                    self.b.ppu.read_value = self.b.ppu.t_oam[self.b.ppu.s as usize].oam_ind;
                    self.runppu();
                } else {
                    if self.b.ppu.target <= 61441 && self.b.ppu.target > 0 && self.b.ppu.s == 0 {
                        self.b.ppu.pipeline(0, 256); //  last pipeline call option 2 of 2
                        self.b.ppu.target += 1;
                    }

                    self.b.ppu.read_value = self.b.ppu.t_oam[self.b.ppu.s as usize].oam_ind;
                    self.runppu();
                }
            }
            2 => {
                let a = self.b.ppu.ppur.get_atread();
                self.b.ppubus_read(a, true, true); //at or nt?
                self.b.ppu.read_value = self.b.ppu.t_oam[self.b.ppu.s as usize].oam_attr;
                self.runppu();
            }
            3 => {
                self.b.ppu.read_value = self.b.ppu.t_oam[self.b.ppu.s as usize].oam_x;
                self.runppu();
            }
            4 => {
                // if the PPU is off, we don't put anything on the bus
                if self.b.ppu.junksprite {
                    let a = self.b.ppu.pattern_address;
                    self.b.ppubus_read(a, true, false);
                    self.runppu();
                } else {
                    self.b.ppu.temp_addr = self.b.ppu.pattern_address;
                    let a = self.b.ppu.temp_addr;
                    let v = self.b.ppubus_read(a, true, true);
                    self.b.ppu.t_oam[self.b.ppu.s as usize].patterns_0 = v;
                    self.b.ppu.read_value = self.b.ppu.t_oam[self.b.ppu.s as usize].oam_x;
                    self.runppu();
                }
            }
            5 => self.runppu(),
            6 => {
                if self.b.ppu.junksprite {
                    let a = self.b.ppu.pattern_address;
                    self.b.ppubus_read(a, true, false);
                    self.runppu();
                } else {
                    self.b.ppu.temp_addr += 8;
                    let a = self.b.ppu.temp_addr;
                    let v = self.b.ppubus_read(a, true, true);
                    self.b.ppu.t_oam[self.b.ppu.s as usize].patterns_1 = v;
                    self.b.ppu.read_value = self.b.ppu.t_oam[self.b.ppu.s as usize].oam_x;
                    self.runppu();
                }
            }
            _ => {
                // case 7
                if self.b.ppu.junksprite {
                    self.runppu();
                } else {
                    self.runppu();

                    let s = self.b.ppu.s as usize;
                    // hflip
                    if (self.b.ppu.t_oam[s].oam_attr & 0x40) == 0 {
                        self.b.ppu.t_oam[s].patterns_0 =
                            BIT_REVERSE_BYTE8[self.b.ppu.t_oam[s].patterns_0 as usize];
                        self.b.ppu.t_oam[s].patterns_1 =
                            BIT_REVERSE_BYTE8[self.b.ppu.t_oam[s].patterns_1 as usize];
                    }

                    // if the sprite's attribute is 0xFF then this indicates a non-existent sprite
                    if self.b.ppu.t_oam[s].oam_attr == 0xFF {
                        self.b.ppu.t_oam[s].patterns_0 = 0;
                        self.b.ppu.t_oam[s].patterns_1 = 0;
                    }
                }
            }
        }

        self.b.ppu.ppu_aux_index += 1;
        if self.b.ppu.ppu_aux_index == 8 {
            // now that we have a sprite, we can fill in the next scanline's sprite pixels with it
            if self.b.ppu.s < self.b.ppu.soam_index_prev
                && self.b.ppu.ppur.status.sl != 0
                && self.b.ppu.ppur.status.sl != 240
            {
                self.fill_sl_sprites(self.b.ppu.s);
            }

            self.b.ppu.ppu_aux_index = 0;
            self.b.ppu.s += 1;

            if self.b.ppu.s < 8 {
                self.b.ppu.junksprite = !self.b.ppu.ppuon();
                self.load_t_oam_from_soam();
            } else if self.b.ppu.soam_index_aux > 8 {
                // repeat all the above steps for more then 8 sprites but don't run any cycles
                self.extra_sprites();
            }
        }
    }

    /// The four `t_oam[s]` loads plus `line` and `patternNumber`, which `TickPPU_active` does in
    /// two places with the same six statements.
    fn load_t_oam_from_soam(&mut self) {
        let s = self.b.ppu.s as usize;
        self.b.ppu.t_oam[s].oam_y = self.b.ppu.soam[s * 4];
        self.b.ppu.t_oam[s].oam_ind = self.b.ppu.soam[s * 4 + 1];
        self.b.ppu.t_oam[s].oam_attr = self.b.ppu.soam[s * 4 + 2];
        self.b.ppu.t_oam[s].oam_x = self.b.ppu.soam[s * 4 + 3];

        self.b.ppu.line = self.b.ppu.yp - i32::from(self.b.ppu.t_oam[s].oam_y);
        if (self.b.ppu.t_oam[s].oam_attr & 0x80) != 0 {
            //vflip
            self.b.ppu.line = self.b.ppu.sprite_height - self.b.ppu.line - 1;
        }

        self.b.ppu.pattern_number = i32::from(self.b.ppu.t_oam[s].oam_ind);
    }

    /// The `sl_sprites` fill, shared by the 8-sprite path and the extra-sprites path.
    fn fill_sl_sprites(&mut self, s: i32) {
        let p = &mut self.b.ppu;
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
        let count = self.b.ppu.soam_index_aux;
        for s in 8..count {
            let su = s as usize;
            self.b.ppu.t_oam[su].oam_y = self.b.ppu.soam[su * 4];
            self.b.ppu.t_oam[su].oam_ind = self.b.ppu.soam[su * 4 + 1];
            self.b.ppu.t_oam[su].oam_attr = self.b.ppu.soam[su * 4 + 2];
            self.b.ppu.t_oam[su].oam_x = self.b.ppu.soam[su * 4 + 3];

            let mut line = self.b.ppu.yp - i32::from(self.b.ppu.t_oam[su].oam_y);
            if (self.b.ppu.t_oam[su].oam_attr & 0x80) != 0 {
                //vflip
                line = self.b.ppu.sprite_height - line - 1;
            }

            let mut pattern_number = i32::from(self.b.ppu.t_oam[su].oam_ind);
            let mut pattern_address;

            //8x16 sprite handling:
            if self.b.ppu.reg_2000.obj_size_16 != 0 {
                let bank = (pattern_number & 1) << 12;
                pattern_number &= !1;
                pattern_number |= (line >> 3) & 1;
                pattern_address = (pattern_number << 4) | bank;
            } else {
                pattern_address =
                    (pattern_number << 4) | (self.b.ppu.reg_2000.obj_pattern_hi << 12);
            }

            pattern_address += line & 7;

            let a = self.b.ppu.ppur.get_ntread();
            self.b.ppubus_read(a, true, false);

            let a = self.b.ppu.ppur.get_atread();
            self.b.ppubus_read(a, true, false); //at or nt?

            let mut addr = pattern_address;
            let v = self.b.ppubus_read(addr, true, false);
            self.b.ppu.t_oam[su].patterns_0 = v;

            addr += 8;
            let v = self.b.ppubus_read(addr, true, false);
            self.b.ppu.t_oam[su].patterns_1 = v;

            // hflip
            if (self.b.ppu.t_oam[su].oam_attr & 0x40) == 0 {
                self.b.ppu.t_oam[su].patterns_0 =
                    BIT_REVERSE_BYTE8[self.b.ppu.t_oam[su].patterns_0 as usize];
                self.b.ppu.t_oam[su].patterns_1 =
                    BIT_REVERSE_BYTE8[self.b.ppu.t_oam[su].patterns_1 as usize];
            }

            if self.b.ppu.t_oam[su].oam_attr == 0xFF {
                self.b.ppu.t_oam[su].patterns_0 = 0;
                self.b.ppu.t_oam[su].patterns_1 = 0;
            }

            if self.b.ppu.ppur.status.sl != 0 && self.b.ppu.ppur.status.sl != 240 {
                self.fill_sl_sprites(s);
            }
        }
    }

    /// `PPU.TickPPU_preVBL`.
    pub(crate) fn tick_ppu_pre_vbl(&mut self) {
        let pre = self.b.ppu.pre_nmi_lines;
        if self.b.ppu.ppur.status.cycle == 340 && self.b.ppu.ppur.status.sl == 241 + pre - 1 {
            self.b.ppu.reg2002_vblank_active_pending = true;
        }

        self.runppu();

        if self.b.ppu.ppur.status.cycle == 341 {
            self.b.ppu.ppur.status.cycle = 0;
            self.b.ppu.ppur.status.sl += 1;

            if self.b.ppu.ppur.status.sl == 241 + pre {
                if self.b.ppu.reg2002_vblank_active_pending {
                    self.b.ppu.reg2002_vblank_active = true;
                    self.b.ppu.reg2002_vblank_active_pending = false;
                }

                self.b.ppu.do_pre_vbl = false;
                self.b.ppu.do_vbl = true;

                self.b.ppu.ppu_init_frame();
                self.b.frame_is_done = true;
            }
        }
    }

    /// `PPU.NewDeadPPU`: the start-up frame, during which the NES ignores register writes.
    pub(crate) fn new_dead_ppu(&mut self) {
        if self.b.ppu.ppur.status.cycle == 241 * 341 - self.b.ppu.start_up_offset - 1 {
            self.b.ppu.reg2002_vblank_active_pending = true;
        }

        self.runppu();

        if self.b.ppu.ppur.status.cycle == 241 * 341 - self.b.ppu.start_up_offset {
            if self.b.ppu.reg2002_vblank_active_pending {
                self.b.ppu.reg2002_vblank_active = true;
                self.b.ppu.reg2002_vblank_active_pending = false;
            }

            self.b.ppu.ppudead -= 1;

            self.b.ppu.ppu_init_frame();

            self.b.ppu.do_vbl = true;

            self.b.frame_is_done = true;
        }
    }
}
