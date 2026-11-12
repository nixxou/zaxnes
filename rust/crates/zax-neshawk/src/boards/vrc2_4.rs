//! `VRC2_4`: Konami's VRC2 and VRC4, mappers 21, 22, 23, 25 (and 27, 116 as the reference has
//! them). Castlevania III (J), Gradius II, Contra (J), Crisis Force, Ganbare Goemon.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/VRC2_4.cs`. The reference's own
//! header note is kept because it explains the shape of the CHR registers: "TODO - for chr,
//! refactor to use 8 registers of 8 bits instead of 16 registers of 4 bits. more realistic, less
//! weird code." The port keeps the 16-of-4 form, because that is what the reference does and a
//! "more realistic" shape would be a redesign.
//! MIT; see `LICENSE-BizHawk`.
//!
//! # The address remap, which is the whole family's difficulty
//!
//! Every variety of VRC2/4 keys on `A14:A12` the same way, and then on **two further address
//! bits that differ per chip**. The C# holds that as a `Func<int,int>` chosen in `Configure`;
//! here it is an enum with one arm per remap, which is the same dispatch with the same names
//! (`AddrA1A0`, `AddrA0A1`, …) and no indirect call.

use super::{Board, BoardBase, BoardEnv, MirrorType};
use crate::cart::DetectionOrigin;

/// The C#'s `remap` field: `Func<int,int>`, with one value per chip revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Remap {
    /// `(addr) => addr` — mappers 27 and 116, which do not remap at all.
    #[default]
    Identity,
    /// `addr & 0x7000 | (addr >> 1) & 1 | (addr << 1) & 2`
    A1A0,
    /// `addr & 0x7003`
    A0A1,
    /// `addr & 0x7000 | (addr >> 1) & 3`
    A1A2,
    /// `addr & 0x7000 | (addr >> 6) & 3`
    A6A7,
    /// `addr & 0x7000 | (addr >> 2) & 3`
    A2A3,
    /// `addr & 0x7000 | (addr >> 3) & 1 | (addr >> 1) & 2`
    A3A2,
    /// `addr & 0x7000 | (addr >> 1) & 3 | (addr >> 6) & 3` — the iNES composite for mapper 21.
    A1A2A6A7,
    /// `addr & 0x7003 | (addr >> 2) & 3` — the iNES composite for mapper 23.
    A0A1A2A3,
    /// `addr & 0x7000 | (addr >> 3) & 1 | (addr >> 1) & 3 | (addr << 1) & 2` — mapper 25.
    A3A2A1A0,
}

impl Remap {
    /// The C#'s six `AddrXxYy` members and its three composites, each transcribed literally.
    const fn apply(self, addr: i32) -> i32 {
        match self {
            Self::Identity => addr,
            Self::A1A0 => addr & 0x7000 | (addr >> 1) & 1 | (addr << 1) & 2,
            Self::A0A1 => addr & 0x7003,
            Self::A1A2 => addr & 0x7000 | (addr >> 1) & 3,
            Self::A6A7 => addr & 0x7000 | (addr >> 6) & 3,
            Self::A2A3 => addr & 0x7000 | (addr >> 2) & 3,
            Self::A3A2 => addr & 0x7000 | (addr >> 3) & 1 | (addr >> 1) & 2,
            Self::A1A2A6A7 => addr & 0x7000 | (addr >> 1) & 3 | (addr >> 6) & 3,
            Self::A0A1A2A3 => addr & 0x7003 | (addr >> 2) & 3,
            Self::A3A2A1A0 => addr & 0x7000 | (addr >> 3) & 1 | (addr >> 1) & 3 | (addr << 1) & 2,
        }
    }
}

/// The C#'s `fix_chr` field: `Func<int,int>`, with two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum FixChr {
    /// `(b) => b`
    #[default]
    Identity,
    /// `(b) => (b >> 1)` — mapper 22 and the VRC2a, whose CHR registers are a bit smaller.
    Shr1,
}

impl FixChr {
    const fn apply(self, b: i32) -> i32 {
        match self {
            Self::Identity => b,
            Self::Shr1 => b >> 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Vrc24 {
    base: BoardBase,
    // configuration
    prg_bank_mask_8k: i32,
    chr_bank_mask_1k: i32,
    prg_reg_mask_8k: i32,
    remap: Remap,
    fix_chr: FixChr,
    /// 2 or 4: which chip family this is. Also used as a *mask* in the mirroring write
    /// (`value & (type - 1)`), which is why it is an `i32` and not an enum.
    board_type: i32,
    latch6k_exists: bool,
    /// "it's been verified that this should be true on all real VRC4 chips, and that some vrc4
    /// boards support it […] but no vrc4 game ever used it"
    extrabig_chr: bool,
    is_pirate: bool,
    /// "needed for 2-in-1 - Yuu Yuu + Dragonball Z [p1][!]"
    is_bmc: bool,
    // state
    prg_bank_reg_8k: [i32; 2],
    chr_bank_reg_1k: [i32; 16],
    prg_mode: bool,
    prg_banks_8k: [u8; 4],
    chr_banks_1k: [i32; 8],
    irq_mode: bool,
    irq_enabled: bool,
    irq_pending: bool,
    irq_autoen: bool,
    irq_reload: u8,
    irq_counter: u8,
    irq_prescaler: i32,
    pub extra_vrom: i32,
    latch6k_value: i32,
}

impl Vrc24 {
    /// `SyncPRG`. Does nothing at all on a BMC board, which drives `prg_banks_8k` directly.
    fn sync_prg(&mut self) {
        if self.is_bmc {
            return;
        }
        if self.prg_mode {
            self.prg_banks_8k[0] = 0xFE;
            self.prg_banks_8k[1] = self.prg_bank_reg_8k[1] as u8;
            self.prg_banks_8k[2] = self.prg_bank_reg_8k[0] as u8;
            self.prg_banks_8k[3] = 0xFF;
        } else {
            self.prg_banks_8k[0] = self.prg_bank_reg_8k[0] as u8;
            self.prg_banks_8k[1] = self.prg_bank_reg_8k[1] as u8;
            self.prg_banks_8k[2] = 0xFE;
            self.prg_banks_8k[3] = 0xFF;
        }
    }

    /// `SyncCHR`: eight 1 KiB banks out of sixteen 4-bit registers, low nibble then high.
    fn sync_chr(&mut self) {
        for i in 0..8 {
            let low = self.chr_bank_reg_1k[i * 2];
            let high = self.chr_bank_reg_1k[i * 2 + 1];
            let temp = low + high * 16;
            let temp = self.fix_chr.apply(temp);
            self.chr_banks_1k[i] = temp & self.chr_bank_mask_1k;
        }
    }

    /// `SyncIRQ`.
    fn sync_irq(&mut self) {
        self.base.irq_signal = self.irq_pending && self.irq_enabled;
    }

    /// `ClockIRQ`.
    fn clock_irq(&mut self) {
        if self.board_type == 2 {
            return;
        }
        if self.irq_counter == 0xFF {
            self.irq_pending = true;
            self.irq_counter = self.irq_reload;
        } else {
            self.irq_counter += 1;
        }
    }

    /// The BMC 2-in-1's own write path, which returns `true` when it handled the write.
    fn write_bmc(&mut self, addr: i32, value: u8) -> bool {
        if !self.is_bmc {
            return false;
        }
        if addr < 0x1000 {
            let index = usize::from(self.prg_mode);
            self.prg_banks_8k[index] = (self.prg_banks_8k[0] & 0x20) | (value & 0x1F);
            return true;
        }
        if (0x2000..0x3000).contains(&addr) {
            self.prg_banks_8k[1] = (self.prg_banks_8k[0] & 0x20) | (value & 0x1F);
            return true;
        }
        if (0x3000..0x7000).contains(&addr) {
            let value = (value << 2) & 0x20;
            for slot in &mut self.prg_banks_8k {
                *slot = value | (*slot & 0x1F);
            }
            return true;
        }
        false
    }
}

impl Board for Vrc24 {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn clone_box(&self) -> Box<dyn Board> {
        Box::new(self.clone())
    }

    fn write_state(&self, writer: &mut crate::state::StateWriter) {
        crate::state::SyncState::write_state(&self.base, writer);
        crate::state::SyncState::write_state(self, writer);
    }

    /// `SyncState` calls `SyncCHR()` and `SyncIRQ()` after reading — and deliberately **not**
    /// `SyncPRG()`, which the C# has commented out, because `prg_banks_8k` is in the block
    /// itself. Both details transcribed.
    fn read_state(&mut self, reader: &mut crate::state::StateReader<'_>) {
        let mut base = std::mem::take(&mut self.base);
        crate::state::SyncState::read_state(&mut base, reader);
        crate::state::SyncState::read_state(self, reader);
        self.base = base;
        self.sync_chr();
        self.sync_irq();
    }

    fn name(&self) -> &'static str {
        "VRC2_4"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        self.fix_chr = FixChr::Identity;
        // For iNES the reference assumes VRC4 in many cases where it might be VRC2.
        match self.base.cart.board_type.clone().as_str() {
            "MAPPER021" => {
                self.board_type = 4;
                self.remap = Remap::A1A2A6A7;
                self.base.cart.wram_size = 8;
            }
            "MAPPER022" => {
                self.board_type = 2;
                self.remap = Remap::A1A0;
                // "should be latch_exists = true but forget that" — the reference's own comment.
                self.base.cart.wram_size = 8;
                self.fix_chr = FixChr::Shr1;
            }
            "MAPPER023" => {
                self.board_type = 4;
                self.remap = Remap::A0A1A2A3;
                self.base.cart.wram_size = 8;
            }
            "MAPPER023_BMC" => {
                self.board_type = 4;
                self.remap = Remap::A0A1A2A3;
                self.base.cart.wram_size = 8;
                self.is_bmc = true;
                self.prg_banks_8k[0] = self.prg_bank_reg_8k[0] as u8;
                self.prg_banks_8k[1] = self.prg_bank_reg_8k[1] as u8;
                self.prg_banks_8k[2] = 0xFE;
                self.prg_banks_8k[3] = 0xFF;
            }
            "MAPPER025" => {
                self.board_type = 4;
                self.remap = Remap::A3A2A1A0;
                self.base.cart.wram_size = 8;
            }
            // `goto case "MAPPER023"` in the C#.
            "UNIF_UNL-T-230" => {
                self.is_pirate = true;
                self.board_type = 4;
                self.remap = Remap::A0A1A2A3;
                self.base.cart.wram_size = 8;
            }
            // "not exactly the same implementation as FCEUX, but we're taking functionality from
            // it step by step as we discover and document it. world hero (unl) is m027 and
            // depends on the extrabig_chr functionality to have correct graphics. otherwise,
            // cah4e3 says its the same as VRC4"
            "MAPPER027" => {
                self.extrabig_chr = true;
                self.remap = Remap::Identity;
                self.board_type = 4;
            }
            "MAPPER116_HACKY" => {
                self.remap = Remap::Identity;
                self.board_type = 2;
            }
            "KONAMI-VRC-4" => {
                let cart = &self.base.cart;
                if !BoardBase::assert_mem(cart.prg_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.chr_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0, 2, 8])
                {
                    return false;
                }
                self.board_type = 4;
                self.remap = match cart.pcb.as_deref() {
                    Some("352398") => Remap::A1A2, // vrc4a A1 A2
                    Some("351406") => Remap::A1A0, // vrc4b A1 A0
                    Some("352889") => Remap::A6A7, // vrc4c A6 A7
                    Some("352400") => Remap::A3A2, // vrc4d A3 A2
                    Some("352396") => Remap::A2A3, // vrc4e A2 A3
                    // The C# throws "Unknown PCB type for VRC4", which aborts the load. This
                    // crate may not panic, so the board refuses the cartridge instead and the
                    // load falls through to the Zaxnes board with its marker — which is a better
                    // outcome than a crash and is visible rather than silent.
                    _ => return false,
                };
            }
            "KONAMI-VRC-2" => {
                let cart = &self.base.cart;
                if !BoardBase::assert_mem(cart.prg_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.chr_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0, 8])
                {
                    return false;
                }
                self.board_type = 2;
                if cart.wram_size == 0 {
                    self.latch6k_exists = true;
                }
                match cart.pcb.as_deref() {
                    // vrc2a A1 A0, whose CHR registers are smaller
                    Some("351618") => {
                        self.remap = Remap::A1A0;
                        self.fix_chr = FixChr::Shr1;
                    }
                    // vrc2b A0 A1
                    Some("LROG009-00" | "350603" | "350926" | "350636" | "351179") => {
                        self.remap = Remap::A0A1;
                    }
                    // vrc2c A1 A0. The reference's note: "this is a weird PCB. it's the only
                    // vrc2 with wram, and the only one with 256 prg rom. it also has extra chips
                    // that no other vrc2\4 has (probably because the VRC2 doesn't support WRAM
                    // internally)"
                    Some("351948") => self.remap = Remap::A1A0,
                    // As above: the C# throws, this refuses.
                    _ => return false,
                }
            }
            _ => return false,
        }

        self.prg_bank_mask_8k = self.base.cart.prg_size / 8 - 1;
        self.chr_bank_mask_1k = self.base.cart.chr_size - 1;

        // "prg regs are 5 bits wide in all VRC2 and VRC4, believe it or not"
        self.prg_reg_mask_8k = 0x1F;

        self.prg_bank_reg_8k[0] = 0;
        self.prg_bank_reg_8k[1] = 1;
        self.sync_prg();
        self.sync_chr();
        self.base.set_mirror_type(MirrorType::Vertical);

        true
    }

    fn read_prg(&self, addr: i32) -> u8 {
        let bank_8k = addr >> 13;
        let ofs = addr & ((1 << 13) - 1);
        let bank_8k = i32::from(self.prg_banks_8k[bank_8k as usize]) & self.prg_bank_mask_8k;
        self.base.rom[((bank_8k << 13) | ofs) as usize]
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr < 0x2000 && !self.base.vrom.is_empty() {
            let bank_1k = self.chr_banks_1k[(addr >> 10) as usize];
            let ofs = addr & ((1 << 10) - 1);
            return self.base.vrom[(((bank_1k << 10) | ofs) + self.extra_vrom) as usize];
        }
        self.base.read_ppu(addr, env)
    }

    fn peek_ppu(&self, addr: i32, ciram: &[u8; 0x800]) -> u8 {
        if addr < 0x2000 && !self.base.vrom.is_empty() {
            let bank_1k = self.chr_banks_1k[(addr >> 10) as usize];
            let ofs = addr & ((1 << 10) - 1);
            return self.base.vrom[(((bank_1k << 10) | ofs) + self.extra_vrom) as usize];
        }
        self.base.peek_ppu(addr, ciram)
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        let addr = self.remap.apply(addr);

        let mut chr_value = i32::from(value) & 0xF;
        if (addr & 1) == 1 && self.extrabig_chr {
            chr_value = i32::from(value) & 0x1F;
        }

        if self.write_bmc(addr, value) {
            return;
        }

        match addr {
            // $8000
            0x0000..=0x0003 => {
                if !self.is_pirate {
                    self.prg_bank_reg_8k[0] = i32::from(value) & self.prg_reg_mask_8k;
                    self.sync_prg();
                }
            }
            // $9000, $9001 — and $9002/$9003 fall here on a VRC2 (`goto case 0x1000`).
            0x1000 | 0x1001 => {
                // "VRC2 only supports V, H, and not A, B"
                let mirror = match i32::from(value) & (self.board_type - 1) {
                    0 => MirrorType::Vertical,
                    1 => MirrorType::Horizontal,
                    2 => MirrorType::OneScreenA,
                    _ => MirrorType::OneScreenB,
                };
                self.base.set_mirror_type(mirror);
            }
            // $9002, $9003
            0x1002 | 0x1003 => {
                if self.board_type == 4 {
                    self.prg_mode = (value >> 1) & 1 != 0;
                    self.sync_prg();
                } else {
                    // The C#'s `goto case 0x1000`, which runs the mirroring arm and **then**
                    // falls out of the switch to `SyncPRG()` — so both happen, in that order.
                    let mirror = match i32::from(value) & (self.board_type - 1) {
                        0 => MirrorType::Vertical,
                        1 => MirrorType::Horizontal,
                        2 => MirrorType::OneScreenA,
                        _ => MirrorType::OneScreenB,
                    };
                    self.base.set_mirror_type(mirror);
                    self.sync_prg();
                }
            }
            // $A000
            0x2000..=0x2003 => {
                if self.is_pirate {
                    self.prg_bank_reg_8k[0] = (i32::from(value) & 0x1F) << 1;
                    self.prg_bank_reg_8k[1] = ((i32::from(value) & 0x1F) << 1) | 1;
                } else {
                    self.prg_bank_reg_8k[1] = i32::from(value) & self.prg_reg_mask_8k;
                }
                self.sync_prg();
            }
            // $B000 to $E003: the sixteen 4-bit CHR registers.
            0x3000..=0x3003 => {
                self.chr_bank_reg_1k[(addr - 0x3000) as usize] = chr_value;
                self.sync_chr();
            }
            0x4000..=0x4003 => {
                self.chr_bank_reg_1k[(addr - 0x4000 + 4) as usize] = chr_value;
                self.sync_chr();
            }
            0x5000..=0x5003 => {
                self.chr_bank_reg_1k[(addr - 0x5000 + 8) as usize] = chr_value;
                self.sync_chr();
            }
            0x6000..=0x6003 => {
                self.chr_bank_reg_1k[(addr - 0x6000 + 12) as usize] = chr_value;
                self.sync_chr();
            }
            // $F000: IRQ reload, low nibble
            0x7000 => {
                if self.board_type != 2 {
                    self.irq_reload = (self.irq_reload & 0xF0) | (value & 0x0F);
                }
            }
            // $F001: IRQ reload, high nibble
            0x7001 => {
                if self.board_type != 2 {
                    self.irq_reload = (self.irq_reload & 0x0F) | (value << 4);
                }
            }
            // $F002: IRQ control
            0x7002 => {
                if self.board_type != 2 {
                    self.irq_mode = (value >> 2) & 1 != 0;
                    self.irq_autoen = value & 1 != 0;
                    if (value >> 1) & 1 != 0 {
                        self.irq_enabled = true;
                        self.irq_counter = self.irq_reload;
                        self.irq_prescaler = 341 + 3;
                    } else {
                        self.irq_enabled = false;
                    }
                    // acknowledge
                    self.irq_pending = false;
                    self.sync_irq();
                }
            }
            // $F003: IRQ acknowledge. `if self.board_type != 2` in the C#'s own shape is folded
            // into the pattern guard, which is the same test on the same arm.
            0x7003 if self.board_type != 2 => {
                self.irq_pending = false;
                self.irq_enabled = self.irq_autoen;
                self.sync_irq();
            }
            // The C#'s `default:` writes "missed case: {addr:X4}" to the console. The port has
            // no console; the write is dropped, as it is there.
            _ => {}
        }
    }

    fn clock_cpu(&mut self) {
        if self.irq_pending {
            self.sync_irq();
        }

        if self.board_type == 2 || !self.irq_enabled {
            return;
        }

        if self.irq_mode {
            self.clock_irq();
        } else {
            self.irq_prescaler -= 3;
            if self.irq_prescaler <= 0 {
                self.irq_prescaler += 341;
                self.clock_irq();
            }
        }
    }

    /// "a single bit of data can be read at 6000:6fff on some VRC2 carts without wram. games
    /// will lock if they can't do this."
    fn read_wram(&self, addr: i32, db: u8) -> u8 {
        if !self.latch6k_exists {
            return self.base.read_wram(addr, db);
        }
        if addr >= 0x1000 {
            return db;
        }
        (db & 0xFE) | (self.latch6k_value as u8)
    }

    fn write_wram(&mut self, addr: i32, value: u8) {
        if !self.latch6k_exists {
            self.base.write_wram(addr, value);
        } else if addr < 0x1000 {
            self.latch6k_value = i32::from(value) & 1;
        }
    }
}

// `VRC2_4.SyncState`, in the C#'s order. `latch6k_value` is synced **only when
// `latch6k_exists`** in the C#, which makes the block's length depend on the board — kept, so
// that a block of this port is the same shape as the reference's own.
crate::state::sync_struct!(Vrc24 {
    prg_bank_reg_8k, chr_bank_reg_1k,
    irq_mode, irq_enabled, irq_pending, irq_autoen, irq_reload, irq_counter, irq_prescaler,
    extra_vrom, prg_mode, prg_banks_8k,
    is_pirate, is_bmc,
    chr_banks_1k, latch6k_value,
    prg_bank_mask_8k, chr_bank_mask_1k, prg_reg_mask_8k,
    remap, fix_chr, board_type, latch6k_exists, extrabig_chr,
} skip: { base });

crate::state::sync_enum!(Remap {
    0 => Remap::Identity,
    1 => Remap::A1A0,
    2 => Remap::A0A1,
    3 => Remap::A1A2,
    4 => Remap::A6A7,
    5 => Remap::A2A3,
    6 => Remap::A3A2,
    7 => Remap::A1A2A6A7,
    8 => Remap::A0A1A2A3,
    9 => Remap::A3A2A1A0,
});

crate::state::sync_enum!(FixChr {
    0 => FixChr::Identity,
    1 => FixChr::Shr1,
});
