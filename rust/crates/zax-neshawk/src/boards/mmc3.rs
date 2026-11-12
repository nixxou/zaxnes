//! The MMC3 chip and the base class its family of boards shares.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/MMC3_family/MMC3.cs`
//! (`MMC3` and `MMC3Board_Base`).
//! MIT; see `LICENSE-BizHawk`.

use super::{BoardBase, MirrorType};

/// `MMC3.EMMC3Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mmc3Type {
    None,
    Mmc3A,
    Mmc3BSharp,
    Mmc3BNonSharp,
    Mmc3C,
    Mmc6,
}

/// `MMC3.cs`'s `MMC3`.
///
/// The IRQ is where NesHawk's MMC3 earns its reputation, so the three counters stay exactly as the
/// C# has them: `separator_counter` and `irq_countdown` both count 15 and 5 PPU dots from an A12
/// rising edge and are deliberately **not** unified ("will someone else try to unify them?"), and
/// `just_cleared`/`just_cleared_pending` delay a `$C001` write by one dot (which "fixes Klax").
#[derive(Debug, Clone)]
pub struct Mmc3 {
    //state
    pub reg_addr: i32,
    pub chr_mode: bool,
    pub prg_mode: bool,
    pub regs: [u8; 8],

    pub mirror: u8,
    a12_old: i32,
    irq_reload: u8,
    irq_counter: u8,
    pub irq_pending: bool,
    pub irq_enable: bool,
    pub irq_reload_flag: bool,
    pub wram_enable: bool,
    pub wram_write_protect: bool,

    pub just_cleared_pending: bool,
    pub just_cleared: bool,

    separator_counter: i32,
    irq_countdown: i32,

    //volatile state
    pub chr_regs_1k: [u8; 8],
    pub prg_regs_8k: [u8; 4],

    //configuration
    mmc3type: Mmc3Type,
    old_irq_type: bool,

    pub cmd: u8,
    pub mirror_mask: u8,

    /// `board.SyncIRQ(flag)` writes `NesBoardBase.IrqSignal`; the owner of this chip copies it
    /// out after every call that can change it. Kept here so that `Mmc3` needs no back-pointer.
    pub irq_signal_out: bool,
}

impl Mmc3 {
    /// The C# constructor, including the chip-revision choice and the initial `regs`.
    #[must_use]
    pub fn new(cart_chips: &[String]) -> Self {
        let mut m = Self {
            reg_addr: 0,
            chr_mode: false,
            prg_mode: false,
            regs: [0; 8],
            mirror: 0,
            a12_old: 0,
            irq_reload: 0,
            irq_counter: 0,
            irq_pending: false,
            irq_enable: false,
            irq_reload_flag: false,
            wram_enable: false,
            wram_write_protect: false,
            just_cleared_pending: false,
            just_cleared: false,
            separator_counter: 0,
            irq_countdown: 0,
            chr_regs_1k: [0; 8],
            prg_regs_8k: [0; 4],
            mmc3type: Mmc3Type::None,
            old_irq_type: false,
            cmd: 0,
            mirror_mask: 1,
            irq_signal_out: false,
        };

        let has = |name: &str| cart_chips.iter().any(|c| c == name);
        // the last two arms are the same value: "arbitrary choice. is it the best choice?"
        #[allow(clippy::if_same_then_else)]
        let t = if has("MMC3A") {
            Mmc3Type::Mmc3A
        } else if has("MMC3B") {
            Mmc3Type::Mmc3BSharp
        } else if has("MMC3BNONSHARP") {
            Mmc3Type::Mmc3BNonSharp
        } else if has("MMC3C") {
            Mmc3Type::Mmc3C
        } else {
            Mmc3Type::Mmc3C //arbitrary choice. is it the best choice?
        };
        m.set_mmc3_type(t);

        //initial values seem necessary
        m.regs[0] = 0;
        m.regs[1] = 2;
        m.regs[2] = 4;
        m.regs[3] = 5;
        m.regs[4] = 6;
        m.regs[5] = 7;
        m.regs[6] = 0;
        m.regs[7] = 1;

        m.sync();
        m
    }

    /// `MMC3Type`'s setter, which also derives `oldIrqType`.
    pub fn set_mmc3_type(&mut self, value: Mmc3Type) {
        self.mmc3type = value;
        self.old_irq_type = matches!(
            value,
            Mmc3Type::Mmc3A | Mmc3Type::Mmc3BNonSharp | Mmc3Type::Mmc6
        );
    }

    #[must_use]
    pub fn mmc3_type(&self) -> Mmc3Type {
        self.mmc3type
    }

    /// `MirrorType`.
    #[must_use]
    pub fn mirror_type(&self) -> MirrorType {
        match self.mirror {
            1 => MirrorType::Horizontal,
            2 => MirrorType::OneScreenA,
            3 => MirrorType::OneScreenB,
            _ => MirrorType::Vertical,
        }
    }

    /// `MMC3.Sync`.
    pub fn sync(&mut self) {
        self.sync_irq();
        if self.prg_mode {
            self.prg_regs_8k[0] = 0xFE;
            self.prg_regs_8k[1] = self.regs[7];
            self.prg_regs_8k[2] = self.regs[6];
            self.prg_regs_8k[3] = 0xFF;
        } else {
            self.prg_regs_8k[0] = self.regs[6];
            self.prg_regs_8k[1] = self.regs[7];
            self.prg_regs_8k[2] = 0xFE;
            self.prg_regs_8k[3] = 0xFF;
        }

        let r0_0 = self.regs[0] & !1;
        let r0_1 = self.regs[0] | 1;
        let r1_0 = self.regs[1] & !1;
        let r1_1 = self.regs[1] | 1;

        if self.chr_mode {
            self.chr_regs_1k[0] = self.regs[2];
            self.chr_regs_1k[1] = self.regs[3];
            self.chr_regs_1k[2] = self.regs[4];
            self.chr_regs_1k[3] = self.regs[5];
            self.chr_regs_1k[4] = r0_0;
            self.chr_regs_1k[5] = r0_1;
            self.chr_regs_1k[6] = r1_0;
            self.chr_regs_1k[7] = r1_1;
        } else {
            self.chr_regs_1k[0] = r0_0;
            self.chr_regs_1k[1] = r0_1;
            self.chr_regs_1k[2] = r1_0;
            self.chr_regs_1k[3] = r1_1;
            self.chr_regs_1k[4] = self.regs[2];
            self.chr_regs_1k[5] = self.regs[3];
            self.chr_regs_1k[6] = self.regs[4];
            self.chr_regs_1k[7] = self.regs[5];
        }
    }

    /// `MMC3.SyncIRQ`, whose only implementation is `board.SyncIRQ(irq_pending)`.
    pub fn sync_irq(&mut self) {
        self.irq_signal_out = self.irq_pending;
    }

    /// `MMC3.WritePRG`. The caller re-reads [`Self::mirror_type`] and `irq_signal_out` after.
    pub fn write_prg(&mut self, addr: i32, value: u8) {
        match addr & 0x6001 {
            0x0000 => {
                //$8000
                self.cmd = value;
                self.chr_mode = (value & 0x80) != 0;
                self.prg_mode = (value & 0x40) != 0;
                self.reg_addr = i32::from(value) & 7;
                self.sync();
            }
            0x0001 => {
                //$8001
                self.regs[self.reg_addr as usize] = value;
                self.sync();
            }
            0x2000 => {
                //$A000 - mirroring
                self.mirror = value & self.mirror_mask;
                // board.SetMirrorType(MirrorType) — the caller does it
            }
            0x2001 => {
                //$A001 - wram enable/protect
                self.wram_write_protect = (value & 0x40) != 0;
                self.wram_enable = (value & 0x80) != 0;
            }
            0x4000 => {
                //$C000 - IRQ Reload value
                self.irq_reload = value;
            }
            0x4001 => {
                //$C001 - IRQ Clear
                // does not take immediate effect (fixes Klax)
                self.just_cleared_pending = true;
            }
            0x6000 => {
                //$E000 - IRQ Acknowledge / Disable
                self.irq_enable = false;
                self.irq_pending = false;
                self.sync_irq();
            }
            0x6001 => {
                //$E001 - IRQ Enable
                self.irq_enable = true;
                self.sync_irq();
            }
            _ => {}
        }
    }

    fn irq_eq_pass(&mut self) {
        if self.irq_enable {
            self.irq_pending = true;
        }
        self.sync_irq();
    }

    fn clock_irq(&mut self) {
        let last_irq_counter = self.irq_counter;
        if self.irq_reload_flag || self.irq_counter == 0 {
            self.irq_counter = self.irq_reload;
        } else {
            self.irq_counter -= 1;
        }
        if self.irq_counter == 0 {
            if self.old_irq_type {
                if last_irq_counter != 0 || self.irq_reload_flag {
                    self.irq_eq_pass();
                }
            } else {
                self.irq_eq_pass();
            }
        }

        self.irq_reload_flag = false;
    }

    /// `MMC3.ClockPPU`, called once per PPU dot.
    pub fn clock_ppu(&mut self) {
        if self.separator_counter > 0 {
            self.separator_counter -= 1;
        }

        if self.irq_countdown > 0 {
            self.irq_countdown -= 1;
            if self.irq_countdown == 0 {
                self.clock_irq();
            }
        }

        if self.just_cleared {
            self.irq_counter = 0;
            if self.old_irq_type {
                self.irq_reload_flag = true;
            }
        }

        self.just_cleared = self.just_cleared_pending;
        self.just_cleared_pending = false;
    }

    #[must_use]
    pub fn get_prg_bank_8k(&self, addr: i32) -> i32 {
        let bank_8k = addr >> 13;
        i32::from(self.prg_regs_8k[bank_8k as usize])
    }

    #[must_use]
    pub fn get_chr_bank_1k(&self, addr: i32) -> i32 {
        let bank_1k = addr >> 10;
        i32::from(self.chr_regs_1k[bank_1k as usize])
    }

    /// `MMC3.AddressPPU`, which sees every PPU bus address.
    pub fn address_ppu(&mut self, addr: i32) {
        // apparently mmc3 can't see internal pattern tables (fixes Recca)
        if addr < 0x3F00 {
            let a12 = (addr >> 12) & 1;
            let rising_edge = a12 == 1 && self.a12_old == 0;
            if rising_edge {
                if self.separator_counter > 0 {
                    self.separator_counter = 15;
                } else {
                    self.separator_counter = 15;
                    self.irq_countdown = 5;
                }
            }

            self.a12_old = a12;
        }
    }
}

/// The state `MMC3Board_Base` adds, with the mapping helpers it defines.
#[derive(Debug)]
pub struct Mmc3BoardBase {
    pub mmc3: Mmc3,
    pub extra_vrom: i32,
    //configuration
    pub prg_mask: i32,
    pub chr_mask: i32,
}

impl Default for Mmc3BoardBase {
    fn default() -> Self {
        Self {
            mmc3: Mmc3::new(&[]),
            extra_vrom: 0,
            prg_mask: 0,
            chr_mask: 0,
        }
    }
}

impl Mmc3BoardBase {
    /// `MMC3Board_Base.BaseSetup`.
    pub fn base_setup(&mut self, base: &mut BoardBase) {
        let num_prg_banks = base.cart.prg_size / 8;
        self.prg_mask = num_prg_banks - 1;

        let mut num_chr_banks = base.cart.chr_size;
        if num_chr_banks == 0 {
            // vram only board
            num_chr_banks = base.cart.vram_size;
        }
        self.chr_mask = num_chr_banks - 1;

        self.mmc3 = Mmc3::new(&base.cart.chips);
        base.irq_signal = self.mmc3.irq_signal_out;
        base.set_mirror_type(MirrorType::Vertical);
    }

    /// `MMC3Board_Base.MapCHR`.
    #[must_use]
    pub fn map_chr(&self, addr: i32) -> i32 {
        let mut bank_1k = self.mmc3.get_chr_bank_1k(addr);
        // allow NPOT chr sizes
        let modulus = self.chr_mask + 1;
        if modulus > 0 {
            bank_1k %= modulus;
        }
        (bank_1k << 10) | (addr & 0x3FF)
    }
}
