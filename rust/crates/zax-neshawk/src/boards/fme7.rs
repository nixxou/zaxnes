//! `Sunsoft_FME7`, mapper 69 (Batman - Return of the Joker, Gimmick!, Barcode World).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/Mapper069.cs`.
//! MIT; see `LICENSE-BizHawk`.
//!
//! # `SUNSOFT-5B` is **not** here, on purpose
//!
//! The C# file holds two classes: `Sunsoft_FME7`, which is this one, and `Sunsoft_5` — the same
//! board plus the 5B's extra sound hardware (`Sunsoft5BAudio`, three square channels and a noise
//! channel on the cartridge). This port has no expansion audio at all, so a `SUNSOFT-5B`
//! cartridge (Gimmick! (J)) is **left to the Zaxnes fallback board**, which may have it, rather
//! than run here with its music missing. That is a deliberate scope line, not an oversight: a
//! board that silently drops a third of a game's audio is worse than one marked "not
//! NesHawk-exact". `configure` therefore refuses `SUNSOFT-5B`.

use super::{Board, BoardBase, BoardEnv, MirrorType};
use crate::cart::DetectionOrigin;

#[derive(Debug, Clone, Default)]
pub struct SunsoftFme7 {
    base: BoardBase,
    // configuration
    prg_bank_mask_8k: i32,
    chr_bank_mask_1k: i32,
    wram_bank_mask_8k: i32,
    // state
    addr_reg: i32,
    regs: [u8; 12],
    prg_banks_8k: [u8; 4],
    wram_bank: i32,
    wram_ram_selected: bool,
    wram_ram_enabled: bool,
    irq_counter: u16,
    irq_countdown: bool,
    irq_enabled: bool,
    irq_asserted: bool,
    clock_counter: i32,
}

impl SunsoftFme7 {
    /// `BaseConfigure`.
    fn base_configure(&mut self) {
        self.prg_bank_mask_8k = (self.base.cart.prg_size / 8) - 1;
        self.wram_bank_mask_8k = (self.base.cart.wram_size / 8) - 1;
        self.chr_bank_mask_1k = self.base.cart.chr_size - 1;
        self.prg_banks_8k[3] = 0xFF;
        self.base.set_mirror_type(MirrorType::Vertical);
    }

    /// `SyncPRG`.
    fn sync_prg(&mut self) {
        self.wram_ram_enabled = (self.regs[8] & 0x80) != 0;
        self.wram_ram_selected = (self.regs[8] & 0x40) != 0;
        self.wram_bank = i32::from(self.regs[8] & 0x7F);
        for i in 0..3 {
            self.prg_banks_8k[i] = self.regs[8 + i + 1];
        }
    }

    /// `SyncIrq`.
    fn sync_irq(&mut self) {
        self.base.irq_signal = self.irq_asserted;
    }

    /// `CalcWRAMAddress`.
    fn calc_wram_address(&self, addr: i32, bank_mask_8k: i32) -> i32 {
        let ofs = addr & ((1 << 13) - 1);
        let bank_8k = self.wram_bank & bank_mask_8k;
        (bank_8k << 13) | ofs
    }

    /// `CalcPPUAddress`.
    fn calc_ppu_address(&self, addr: i32) -> i32 {
        let bank_1k = addr >> 10;
        let ofs = addr & ((1 << 10) - 1);
        let bank_1k = i32::from(self.regs[bank_1k as usize]) & self.chr_bank_mask_1k;
        (bank_1k << 10) | ofs
    }
}

impl Board for SunsoftFme7 {
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

    /// `SyncState` ends with `SyncIrq()` on both directions in the C#; only the read direction
    /// needs it here, because the write direction changes nothing.
    fn read_state(&mut self, reader: &mut crate::state::StateReader<'_>) {
        let mut base = std::mem::take(&mut self.base);
        crate::state::SyncState::read_state(&mut base, reader);
        crate::state::SyncState::read_state(self, reader);
        self.base = base;
        self.sync_irq();
    }

    fn name(&self) -> &'static str {
        "Sunsoft_FME7"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &self.base.cart;
        match cart.board_type.as_str() {
            // Mr Gimmick
            "NES-JLROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[256])
                    || !BoardBase::assert_mem(cart.chr_size, &[128])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            // Batman (E)
            "NES-JSROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[256])
                    || !BoardBase::assert_mem(cart.wram_size, &[8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "MAPPER069" => {}
            // Batman (J)
            "SUNSOFT-5A" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.wram_size, &[0, 8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            // Barcode World (J)
            "SUNSOFT-FME-7" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.chr_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.wram_size, &[0, 8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                {
                    return false;
                }
            }
            // Batman - Return of the Joker (U), Mr Gimmick (Proto)
            "NES-BTR" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.chr_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.wram_size, &[0, 8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            // `SUNSOFT-5B` goes to `Sunsoft_5` in the C#, which is this board plus the 5B's
            // sound hardware. See the module header: it is left to the Zaxnes fallback.
            _ => return false,
        }

        self.base_configure();
        true
    }

    /// `ClockCpu`: the IRQ counter is a CPU-cycle counter on this board, not an A12 counter.
    fn clock_cpu(&mut self) {
        if !self.irq_countdown {
            return;
        }
        self.irq_counter = self.irq_counter.wrapping_sub(1);
        if self.irq_counter == 0xFFFF {
            self.irq_asserted = true;
            self.sync_irq();
        }
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        match addr & 0xE000 {
            // $8000: [.... AAAA] Address for use with $A000
            0x0000 => self.addr_reg = i32::from(value) & 0xF,
            // $A000: [DDDD DDDD] Data port
            0x2000 => match self.addr_reg {
                0..=7 => self.regs[self.addr_reg as usize] = value,
                8..=0xB => {
                    self.regs[self.addr_reg as usize] = value;
                    self.sync_prg();
                }
                0xC => {
                    let mirror = match value & 3 {
                        0 => MirrorType::Vertical,
                        1 => MirrorType::Horizontal,
                        2 => MirrorType::OneScreenA,
                        _ => MirrorType::OneScreenB,
                    };
                    self.base.set_mirror_type(mirror);
                }
                0xD => {
                    self.irq_countdown = (value >> 7) & 1 != 0;
                    self.irq_enabled = value & 1 != 0;
                    // "always ACK for reg 0xD and no other reg"
                    // <http://forums.nesdev.com/viewtopic.php?f=2&t=12436&start=15>
                    self.irq_asserted = false;
                    self.sync_irq();
                }
                0xE => {
                    self.irq_counter &= 0xFF00;
                    self.irq_counter |= u16::from(value);
                }
                _ => {
                    self.irq_counter &= 0x00FF;
                    self.irq_counter |= u16::from(value) << 8;
                }
            },
            _ => {}
        }
    }

    fn read_prg(&self, addr: i32) -> u8 {
        let bank_8k = addr >> 13;
        let ofs = addr & ((1 << 13) - 1);
        let bank_8k = i32::from(self.prg_banks_8k[bank_8k as usize]) & self.prg_bank_mask_8k;
        self.base.rom[((bank_8k << 13) | ofs) as usize]
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr < 0x2000 {
            return self.base.vrom[self.calc_ppu_address(addr) as usize];
        }
        self.base.read_ppu(addr, env)
    }

    fn peek_ppu(&self, addr: i32, ciram: &[u8; 0x800]) -> u8 {
        if addr < 0x2000 {
            return self.base.vrom[self.calc_ppu_address(addr) as usize];
        }
        self.base.peek_ppu(addr, ciram)
    }

    /// `WritePpu` drops a write below `$2000` entirely: this board is CHR-ROM only.
    fn write_ppu(&mut self, addr: i32, value: u8, env: &mut BoardEnv<'_>) {
        if addr < 0x2000 {
            return;
        }
        self.base.write_ppu(addr, value, env);
    }

    /// `ReadWram`: the `$6000-$7FFF` window is a **PRG-ROM** bank unless the register selects
    /// RAM, which is the one thing about this board that surprises a reader.
    fn read_wram(&self, addr: i32, _db: u8) -> u8 {
        if !self.wram_ram_selected {
            let addr = self.calc_wram_address(addr, self.prg_bank_mask_8k);
            return self.base.rom[addr as usize];
        }
        if !self.wram_ram_enabled || self.base.wram.is_empty() {
            return 0xFF; // empty bus
        }
        let addr = self.calc_wram_address(addr, self.wram_bank_mask_8k);
        self.base.wram[addr as usize]
    }

    fn write_wram(&mut self, addr: i32, value: u8) {
        if !self.wram_ram_selected {
            return;
        }
        if !self.wram_ram_enabled || self.base.wram.is_empty() {
            return; // empty bus
        }
        let addr = self.calc_wram_address(addr, self.wram_bank_mask_8k);
        self.base.wram[addr as usize] = value;
    }
}

// `Sunsoft_FME7.SyncState`, in the C#'s order.
crate::state::sync_struct!(SunsoftFme7 {
    addr_reg, regs, prg_banks_8k, wram_bank,
    wram_ram_selected, wram_ram_enabled,
    irq_counter, irq_countdown, irq_enabled, irq_asserted, clock_counter,
    prg_bank_mask_8k, chr_bank_mask_1k, wram_bank_mask_8k,
} skip: { base });
