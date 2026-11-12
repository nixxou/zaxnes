//! The PPU's state and its internal counters.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/PPU.cs`, `PPU.regs.cs` (the register
//! objects) and the state declarations of `PPU.run.cs`. The methods that drive the machine —
//! `runppu`, `TickPPU_*`, `NewDeadPPU` and the `$2000-$2007` handlers — are `impl Nes` in
//! [`run`] and [`regs`], because in the C# every one of them calls `nes.RunCpuOne()` or
//! `nes.Board.*`.
//! MIT; see `LICENSE-BizHawk`.

pub(crate) mod regs;
pub(crate) mod run;

/// `PPU.Region`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Region {
    #[default]
    Ntsc,
    Pal,
    Dendy,
    Rgb,
}

/// `PPU.PPUSTATUS`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PpuStatus {
    pub sl: i32,
    pub cycle: i32,
}

impl PpuStatus {
    /// `PPUSTATUS.rendering`.
    #[must_use]
    pub fn rendering(&self) -> bool {
        self.sl >= 0 && self.sl < 241
    }
}

/// `PPU.PPUREGS`: the internal counters of
/// <http://nesdev.icequake.net/PPU%20addressing.txt>.
#[derive(Debug, Clone, Default)]
#[allow(non_snake_case)]
pub struct PpuRegs {
    //normal clocked regs
    pub fv: i32, //3
    pub v: i32,  //1
    pub h: i32,  //1
    pub vt: i32, //5
    pub ht: i32, //5

    //temp unlatched regs
    pub _fv: i32,
    pub _vt: i32,
    pub _v: i32,
    pub _h: i32,
    pub _ht: i32,

    //other regs that need savestating
    pub fh: i32, //3 (horz scroll)

    pub status: PpuStatus,
}

impl PpuRegs {
    pub fn reset(&mut self) {
        self.fv = 0;
        self.v = 0;
        self.h = 0;
        self.vt = 0;
        self.ht = 0;
        self.fh = 0;
        self._fv = 0;
        self._v = 0;
        self._h = 0;
        self._vt = 0;
        self._ht = 0;
        self.status.cycle = 0;
        self.status.sl = 0;
    }

    pub fn install_latches(&mut self) {
        self.fv = self._fv;
        self.v = self._v;
        self.h = self._h;
        self.vt = self._vt;
        self.ht = self._ht;
    }

    pub fn install_h_latches(&mut self) {
        self.ht = self._ht;
        self.h = self._h;
    }

    /// The horizontal scroll counter: HT daisy-chained to H, clocked every 8 dots.
    pub fn increment_hsc(&mut self) {
        self.ht += 1;
        self.h += self.ht >> 5;
        self.ht &= 31;
        self.h &= 1;
    }

    pub fn increment_vs(&mut self) {
        self.fv += 1;
        let fv_overflow = self.fv >> 3;
        self.vt += fv_overflow;
        self.vt &= 31; //fixed tecmo super bowl
        if self.vt == 30 && fv_overflow == 1 {
            //caution here (only do it at the exact instant of overflow) fixes p'radikus conflict
            self.v += 1;
            self.vt = 0;
        }
        self.fv &= 7;
        self.v &= 1;
    }

    #[must_use]
    pub fn get_ntread(&self) -> i32 {
        0x2000 | (self.v << 0xB) | (self.h << 0xA) | (self.vt << 5) | self.ht
    }

    #[must_use]
    pub fn get_2007access(&self) -> i32 {
        ((self.fv & 3) << 0xC) | (self.v << 0xB) | (self.h << 0xA) | (self.vt << 5) | self.ht
    }

    #[must_use]
    pub fn get_atread(&self) -> i32 {
        0x2000
            | (self.v << 0xB)
            | (self.h << 0xA)
            | 0x3C0
            | ((self.vt & 0x1C) << 1)
            | ((self.ht & 0x1C) >> 2)
    }

    /// `increment2007`.
    pub fn increment2007(&mut self, rendering: bool, by32: bool) {
        if rendering {
            //don't do this:
            //if (by32) increment_vs(); else increment_hsc();
            //do this instead:
            self.increment_vs(); //yes, even if we're moving by 32
            return;
        }

        if by32 {
            self.vt += 1;
        } else {
            self.ht += 1;
            self.vt += (self.ht >> 5) & 1;
        }
        self.h += self.vt >> 5;
        self.v += self.h >> 1;
        self.fv += self.v >> 1;
        self.ht &= 31;
        self.vt &= 31;
        self.h &= 1;
        self.v &= 1;
        self.fv &= 7;
    }
}

/// `PPU.Reg_2000`. The two bits that go straight into `PPUREGS` live there, as in the C# —
/// `Value`'s setter writes `_regs._h` and `_regs._v`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Reg2000 {
    pub vram_incr32: i32,
    pub obj_pattern_hi: i32,
    pub bg_pattern_hi: i32,
    pub obj_size_16: i32,
    pub ppu_layer: i32,
    pub vblank_nmi_gen: i32,
}

/// `PPU.Reg_2001`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Reg2001 {
    pub color_disable: i32,
    pub show_bg_leftmost: i32,
    pub show_obj_leftmost: i32,
    pub show_bg: i32,
    pub show_obj: i32,
    pub intense_green: i32,
    pub intense_blue: i32,
    pub intense_red: i32,
    /// an optimization..
    pub intensity_lsl_6: i32,
}

impl Reg2001 {
    #[must_use]
    pub fn value(&self) -> u8 {
        (self.color_disable
            | (self.show_bg_leftmost << 1)
            | (self.show_obj_leftmost << 2)
            | (self.show_bg << 3)
            | (self.show_obj << 4)
            | (self.intense_green << 5)
            | (self.intense_blue << 6)
            | (self.intense_red << 7)) as u8
    }

    pub fn set_value(&mut self, value: u8) {
        let v = i32::from(value);
        self.color_disable = v & 1;
        self.show_bg_leftmost = (v >> 1) & 1;
        self.show_obj_leftmost = (v >> 2) & 1;
        self.show_bg = (v >> 3) & 1;
        self.show_obj = (v >> 4) & 1;
        self.intense_blue = (v >> 6) & 1;
        self.intense_red = (v >> 7) & 1;
        self.intense_green = (v >> 5) & 1;
        self.intensity_lsl_6 = ((v >> 5) & 7) << 6;
    }
}

/// `PPU.run.cs`'s `BGDataRecord`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BgDataRecord {
    pub nt: u8,
    pub at: u8,
    pub pt_0: u8,
    pub pt_1: u8,
}

/// `PPU.run.cs`'s `TempOAM`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TempOam {
    pub oam_y: u8,
    pub oam_ind: u8,
    pub oam_attr: u8,
    pub oam_x: u8,
    pub patterns_0: u8,
    pub patterns_1: u8,
}

/// `PPU.ppuphase`'s three values.
pub const PPU_PHASE_VBL: i32 = 0;
pub const PPU_PHASE_BG: i32 = 1;
pub const PPU_PHASE_OBJ: i32 = 2;

/// `PPU.regs.cs`'s `DecayType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecayType {
    /// if there is no action, decrement the timer
    None,
    /// reset the timer for all bits (reg 2004 / 2007 (non-palette))
    All,
    /// reset the timer for high 3 bits (reg 2002)
    High,
    /// reset the timer for all low 6 bits (reg 2007 (palette))
    Low,
}

/// About one second worth of cycles, which is the C#'s own comment on this number.
const DECAY_CYCLES: i32 = 1_786_840;

/// Everything `PPU` holds. The reference's field names are kept, including the ones that are only
/// loop variables hoisted out of `TickPPU_active` (`xt`, `xp`, `s`, `line`, `target`…): they are
/// fields in the C# too, and a savestate syncs them.
#[allow(non_snake_case)]
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Ppu {
    pub cpu_step: i32,
    pub cpu_stepcounter: i32,

    pub has_clock_ppu: bool,

    // this only handles region differences within the PPU
    pub(crate) pre_nmi_lines: i32,
    pub(crate) post_nmi_lines: i32,
    pub(crate) chopdot: bool,

    pub region: Region,

    pub ppuphase: i32,

    //state
    pub ppudead: i32, //measured in frames
    pub(crate) idle_synch: bool,
    pub(crate) nmi_pending_instructions: i32,
    pub(crate) ppu_gen_latch: u8,
    pub(crate) vtoggle: bool,
    pub(crate) vram_buffer: u8,
    pub oam: [u8; 0x100],
    pub palram: [u8; 32],

    pub(crate) total_cycles: i64,

    // ---- PPU.run.cs
    pub(crate) bgdata: [BgDataRecord; 34],
    /// `PPU.xbuf`. A `Box<[i16]>` rather than the C#'s fixed `short[256*240]` only so that the
    /// 120 KiB never sits on the stack while the box is built.
    pub xbuf: Box<[i16]>,

    // values here are used in sprite evaluation
    pub(crate) spr_true_count: i32,
    pub(crate) sprite_eval_write: bool,
    pub(crate) read_value: u8,
    pub(crate) soam_index: i32,
    pub(crate) soam_index_prev: i32,
    pub(crate) oam_index: i32,
    pub(crate) oam_index_aux: i32,
    pub(crate) soam_index_aux: i32,
    pub(crate) is_even_cycle: bool,
    pub(crate) sprite_zero_in_range: bool,
    pub(crate) sprite_zero_go: bool,
    pub(crate) yp: i32,
    pub(crate) target: i32,
    pub(crate) sprite_height: i32,
    /// in a real nes, this would only be 32, but we wish to allow more then 8 sprites per scanline
    pub(crate) soam: [u8; 256],
    pub(crate) ppu_was_on: bool,
    pub(crate) ppu_was_on_spr: bool,
    pub(crate) sl_sprites: [u8; 3 * 256],

    // installing vram address is delayed after second write to 2006
    pub(crate) install_2006: i32,
    pub(crate) race_2006: bool,
    pub(crate) race_2006_2: bool,
    pub(crate) install_2001: i32,
    pub(crate) show_bg_new: bool,
    pub(crate) show_obj_new: bool,

    pub(crate) t_oam: [TempOam; 64],
    pub(crate) ppu_addr_temp: i32,
    pub(crate) pixelcolor_latch_1: i32,

    // these are states for the ppu incrementer
    pub do_vbl: bool,
    pub do_active_sl: bool,
    pub do_pre_vbl: bool,

    pub(crate) nmi_destiny: bool,
    pub(crate) even_odd_destiny: bool,
    pub start_up_offset: i32,
    pub(crate) nmi_offset: i32,
    pub(crate) yp_shift: i32,
    pub(crate) sprite_eval_cycle: i32,
    pub(crate) xt: i32,
    pub(crate) xp: i32,
    pub(crate) xstart: i32,
    pub(crate) rasterpos: i32,
    pub(crate) renderspritenow: bool,
    pub(crate) s: i32,
    pub(crate) ppu_aux_index: i32,
    pub(crate) junksprite: bool,
    pub(crate) line: i32,
    pub(crate) pattern_number: i32,
    pub(crate) pattern_address: i32,
    pub(crate) temp_addr: i32,

    // ---- PPU.regs.cs
    pub ppu_open_bus: u8,
    /// emulates a hardware bug of back to back 2007 reads
    pub(crate) double_2007_read: i64,
    pub(crate) ppu_open_bus_decay_timer: [i32; 8],
    /// `PPU.glitchy_reads_2003`, which only the commented-out OAM-corruption loop of
    /// `write_2003` would fill. Kept because the C# savestates it.
    #[allow(dead_code)]
    pub(crate) glitchy_reads_2003: [u8; 8],

    pub(crate) reg2002_objoverflow: bool,
    pub(crate) reg2002_objhit: bool,
    pub reg2002_vblank_active: bool,
    pub(crate) reg2002_vblank_active_pending: bool,
    pub(crate) reg2002_vblank_clear_pending: bool,

    pub ppur: PpuRegs,
    pub reg_2000: Reg2000,
    pub reg_2001: Reg2001,
    pub reg_2003: u8,
    pub reg_2006_2: u8,

    // `nes.Settings` — the three the PPU reads, at their BizHawk defaults.
    pub disp_background: bool,
    pub disp_sprites: bool,
    pub allow_more_than_eight_sprites: bool,

    /// The 3D tile capture's state (`crate::tiles`). Not NesHawk's: Zaxnes's own addition, which
    /// reads the PPU and changes nothing in it.
    pub tiles: crate::tiles::TileCapture,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    /// `new PPU(nes)`: the power-up palette, then `Reset()`.
    #[must_use]
    pub fn new() -> Self {
        let mut ppu = Self {
            cpu_step: 0,
            cpu_stepcounter: 0,
            has_clock_ppu: false,
            pre_nmi_lines: 0,
            post_nmi_lines: 0,
            chopdot: false,
            region: Region::Ntsc,
            ppuphase: 0,
            ppudead: 0,
            idle_synch: false,
            nmi_pending_instructions: 0,
            ppu_gen_latch: 0,
            vtoggle: false,
            vram_buffer: 0,
            oam: [0; 0x100],
            //power-up palette verified by blargg's power_up_palette test.
            #[rustfmt::skip]
            palram: [
                0x09, 0x01, 0x00, 0x01, 0x00, 0x02, 0x02, 0x0D,
                0x08, 0x10, 0x08, 0x24, 0x00, 0x00, 0x04, 0x2C,
                0x09, 0x01, 0x34, 0x03, 0x00, 0x04, 0x00, 0x14,
                0x08, 0x3A, 0x00, 0x02, 0x00, 0x20, 0x2C, 0x08,
            ],
            total_cycles: 0,
            bgdata: [BgDataRecord::default(); 34],
            xbuf: vec![0i16; 256 * 240].into_boxed_slice(),
            spr_true_count: 0,
            sprite_eval_write: false,
            read_value: 0,
            soam_index: 0,
            soam_index_prev: 0,
            oam_index: 0,
            oam_index_aux: 0,
            soam_index_aux: 0,
            is_even_cycle: false,
            sprite_zero_in_range: false,
            sprite_zero_go: false,
            yp: 0,
            target: 0,
            sprite_height: 0,
            soam: [0; 256],
            ppu_was_on: false,
            ppu_was_on_spr: false,
            sl_sprites: [0; 3 * 256],
            install_2006: 0,
            race_2006: false,
            race_2006_2: false,
            install_2001: 0,
            show_bg_new: false,
            show_obj_new: false,
            t_oam: [TempOam::default(); 64],
            ppu_addr_temp: 0,
            pixelcolor_latch_1: 0,
            do_vbl: false,
            do_active_sl: false,
            do_pre_vbl: false,
            nmi_destiny: false,
            even_odd_destiny: false,
            start_up_offset: 0,
            nmi_offset: 0,
            yp_shift: 0,
            sprite_eval_cycle: 0,
            xt: 0,
            xp: 0,
            xstart: 0,
            rasterpos: 0,
            renderspritenow: false,
            s: 0,
            ppu_aux_index: 0,
            junksprite: false,
            line: 0,
            pattern_number: 0,
            pattern_address: 0,
            temp_addr: 0,
            ppu_open_bus: 0,
            double_2007_read: 0,
            ppu_open_bus_decay_timer: [0; 8],
            glitchy_reads_2003: [0; 8],
            reg2002_objoverflow: false,
            reg2002_objhit: false,
            reg2002_vblank_active: false,
            reg2002_vblank_active_pending: false,
            reg2002_vblank_clear_pending: false,
            ppur: PpuRegs::default(),
            reg_2000: Reg2000::default(),
            reg_2001: Reg2001::default(),
            reg_2003: 0,
            reg_2006_2: 0,
            disp_background: true,
            disp_sprites: true,
            allow_more_than_eight_sprites: false,
            tiles: crate::tiles::TileCapture::default(),
        };
        ppu.sync_region();
        ppu.reset();
        ppu
    }

    /// `PPU.region`'s setter.
    pub fn set_region(&mut self, region: Region) {
        self.region = region;
        self.sync_region();
    }

    /// `SyncRegion`.
    fn sync_region(&mut self) {
        match self.region {
            Region::Ntsc => {
                self.pre_nmi_lines = 1;
                self.post_nmi_lines = 20;
                self.chopdot = true;
            }
            Region::Pal => {
                self.pre_nmi_lines = 1;
                self.post_nmi_lines = 70;
                self.chopdot = false;
            }
            Region::Dendy => {
                self.pre_nmi_lines = 51;
                self.post_nmi_lines = 20;
                self.chopdot = false;
            }
            Region::Rgb => {
                self.pre_nmi_lines = 1;
                self.post_nmi_lines = 20;
                self.chopdot = false;
            }
        }
    }

    /// `PPU.Reset`.
    pub fn reset(&mut self) {
        self.regs_reset();
        self.ppudead = 1;
        self.idle_synch = true;
        self.ppu_open_bus = 0;
        self.ppu_open_bus_decay_timer = [0; 8];
        self.double_2007_read = 0;
        self.start_up_offset = 4;
    }

    /// `PPU.NESSoftReset`. "this hasn't been brought up to date since NEShawk was first made."
    pub fn nes_soft_reset(&mut self) {
        self.reset();
    }

    /// `regs_reset`.
    fn regs_reset(&mut self) {
        self.ppur = PpuRegs::default();
        self.reg_2000 = Reg2000::default();
        self.reg_2001 = Reg2001::default();
        self.reg2002_objoverflow = false;
        self.reg2002_objhit = false;
        self.reg2002_vblank_active = false;
        self.ppu_gen_latch = 0;
        self.reg_2003 = 0;
        self.vtoggle = false;
        self.vram_buffer = 0;
    }

    /// `PPU.PPUON`.
    #[must_use]
    pub fn ppuon(&self) -> bool {
        self.show_bg_new || self.show_obj_new
    }

    #[must_use]
    pub fn total_cycles(&self) -> i64 {
        self.total_cycles
    }

    /// `Reg_2000.Value`'s getter, which reads `_h`/`_v` back out of `PPUREGS`.
    #[must_use]
    pub fn reg_2000_value(&self) -> u8 {
        (self.ppur._h
            | (self.ppur._v << 1)
            | (self.reg_2000.vram_incr32 << 2)
            | (self.reg_2000.obj_pattern_hi << 3)
            | (self.reg_2000.bg_pattern_hi << 4)
            | (self.reg_2000.obj_size_16 << 5)
            | (self.reg_2000.ppu_layer << 6)
            | (self.reg_2000.vblank_nmi_gen << 7)) as u8
    }

    /// `Reg_2000.Value`'s setter.
    pub fn set_reg_2000_value(&mut self, value: u8) {
        let v = i32::from(value);
        self.ppur._h = v & 1;
        self.ppur._v = (v >> 1) & 1;
        self.reg_2000.vram_incr32 = (v >> 2) & 1;
        self.reg_2000.obj_pattern_hi = (v >> 3) & 1;
        self.reg_2000.bg_pattern_hi = (v >> 4) & 1;
        self.reg_2000.obj_size_16 = (v >> 5) & 1;
        self.reg_2000.ppu_layer = (v >> 6) & 1;
        self.reg_2000.vblank_nmi_gen = (v >> 7) & 1;
    }

    /// `PpuOpenBusDecay`.
    pub(crate) fn ppu_open_bus_decay(&mut self, action: DecayType) {
        match action {
            DecayType::None => {
                for i in 0..8 {
                    if self.ppu_open_bus_decay_timer[i] == 0 {
                        self.ppu_open_bus &= 0xFF - (1u8 << i);
                        self.ppu_open_bus_decay_timer[i] = DECAY_CYCLES;
                    } else {
                        self.ppu_open_bus_decay_timer[i] -= 1;
                    }
                }
            }
            DecayType::All => {
                for i in 0..8 {
                    self.ppu_open_bus_decay_timer[i] = DECAY_CYCLES;
                }
            }
            DecayType::High => {
                self.ppu_open_bus_decay_timer[7] = DECAY_CYCLES;
                self.ppu_open_bus_decay_timer[6] = DECAY_CYCLES;
                self.ppu_open_bus_decay_timer[5] = DECAY_CYCLES;
            }
            DecayType::Low => {
                for i in 0..6 {
                    self.ppu_open_bus_decay_timer[i] = DECAY_CYCLES;
                }
            }
        }
    }

    /// `ppu_init_frame`.
    pub fn ppu_init_frame(&mut self) {
        self.ppur.status.sl = 241 + self.pre_nmi_lines;
        self.ppur.status.cycle = 0;

        // These things happen at the start of every frame
        self.ppuphase = PPU_PHASE_VBL;
        self.bgdata = [BgDataRecord::default(); 34];
    }

    /// `pipeline`, the experimental graphics-pipeline latch.
    pub(crate) fn pipeline(&mut self, pixelcolor: i32, row_check: i32) {
        if row_check > 0 {
            if self.reg_2001.color_disable != 0 {
                self.pixelcolor_latch_1 &= 0x30;
            }

            //tack on the deemph bits. THESE MAY BE ORDERED WRONG.
            self.xbuf[(self.target - 1) as usize] =
                (self.pixelcolor_latch_1 | self.reg_2001.intensity_lsl_6) as i16;
        }

        self.pixelcolor_latch_1 = pixelcolor;
    }
}

// -------------------------------------------------------------------- the savestate

// `PPU.SyncState`, `PPUREGS.SyncState` and the register sub-objects'. Nothing here is skipped
// except the frame buffer, which is handled below for the reason given there.
crate::state::sync_enum!(Region {
    0 => Region::Ntsc,
    1 => Region::Pal,
    2 => Region::Dendy,
    3 => Region::Rgb,
});

crate::state::sync_struct!(PpuStatus { sl, cycle });

crate::state::sync_struct!(PpuRegs {
    fv,
    v,
    h,
    vt,
    ht,
    _fv,
    _vt,
    _v,
    _h,
    _ht,
    fh,
    status,
});

crate::state::sync_struct!(Reg2000 {
    vram_incr32,
    obj_pattern_hi,
    bg_pattern_hi,
    obj_size_16,
    ppu_layer,
    vblank_nmi_gen,
});

crate::state::sync_struct!(Reg2001 {
    color_disable,
    show_bg_leftmost,
    show_obj_leftmost,
    show_bg,
    show_obj,
    intense_green,
    intense_blue,
    intense_red,
    intensity_lsl_6,
});

crate::state::sync_struct!(BgDataRecord { nt, at, pt_0, pt_1 });

crate::state::sync_struct!(TempOam {
    oam_y,
    oam_ind,
    oam_attr,
    oam_x,
    patterns_0,
    patterns_1
});

// Every field of `PPU` but the frame buffer, which `SyncState for Ppu` below handles itself.
crate::state::sync_struct!(inherent Ppu {
    cpu_step, cpu_stepcounter, has_clock_ppu,
    pre_nmi_lines, post_nmi_lines, chopdot, region,
    ppuphase, ppudead, idle_synch, nmi_pending_instructions,
    ppu_gen_latch, vtoggle, vram_buffer,
    oam, palram, total_cycles, bgdata,
    spr_true_count, sprite_eval_write, read_value,
    soam_index, soam_index_prev, oam_index, oam_index_aux, soam_index_aux,
    is_even_cycle, sprite_zero_in_range, sprite_zero_go,
    yp, target, sprite_height, soam, ppu_was_on, ppu_was_on_spr, sl_sprites,
    install_2006, race_2006, race_2006_2, install_2001, show_bg_new, show_obj_new,
    t_oam, ppu_addr_temp, pixelcolor_latch_1,
    do_vbl, do_active_sl, do_pre_vbl, nmi_destiny, even_odd_destiny,
    start_up_offset, nmi_offset, yp_shift, sprite_eval_cycle,
    xt, xp, xstart, rasterpos, renderspritenow, s, ppu_aux_index, junksprite,
    line, pattern_number, pattern_address, temp_addr,
    ppu_open_bus, double_2007_read, ppu_open_bus_decay_timer, glitchy_reads_2003,
    reg2002_objoverflow, reg2002_objhit, reg2002_vblank_active,
    reg2002_vblank_active_pending, reg2002_vblank_clear_pending,
    ppur, reg_2000, reg_2001, reg_2003, reg_2006_2,
    disp_background, disp_sprites, allow_more_than_eight_sprites,
    tiles,
} skip: {
    xbuf,
});

impl crate::state::SyncState for Ppu {
    /// Written by hand rather than generated for one field only: `xbuf`.
    ///
    /// The frame buffer is 120 KiB and **is** the frame just run, so a rollback block leaves it
    /// out — the next frame overwrites every pixel of it, and that is what REQ-CORE-121's own
    /// in-memory snapshot does ("leaves out the half of the PPU state that is the frame just
    /// run"). A state the user saves carries it, so that the picture comes back at once instead
    /// of on the next frame. Which of the two a block is, is written in its header and the
    /// reader is told before it starts, so the two paths cannot disagree about the layout.
    ///
    /// Everything else goes through the generated `write_sync_fields`/`read_sync_fields`, so the
    /// exhaustive check still covers it: a field added to `Ppu` and named in neither list will
    /// not compile.
    fn write_state(&self, w: &mut crate::state::StateWriter) {
        self.write_sync_fields(w);
        if w.full() {
            for pixel in &*self.xbuf {
                crate::state::SyncState::write_state(pixel, w);
            }
        }
    }

    fn read_state(&mut self, r: &mut crate::state::StateReader<'_>) {
        self.read_sync_fields(r);
        if r.full() {
            for pixel in &mut *self.xbuf {
                crate::state::SyncState::read_state(pixel, r);
            }
        }
    }
}
