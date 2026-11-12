//! UxROM, generally mapper 2 (Mega Man, Castlevania, Contra, Duck Tales, Metal Gear).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/UxROM.cs`.
//! MIT; see `LICENSE-BizHawk`.

use super::{Board, BoardBase, BoardEnv};
use crate::cart::DetectionOrigin;

/// `UxROM.adjust_prg`, which is a `Func<int,int>` in the C# and has exactly two values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AdjustPrg {
    /// `(x) => x`
    #[default]
    Identity,
    /// `(x) => ((x >> 2) & 7)` — HVC-UN1ROM
    Un1Rom,
}

#[derive(Debug, Clone, Default)]
pub struct UxRom {
    base: BoardBase,
    // configuration
    prg_mask: i32,
    vram_byte_mask: i32,
    adjust_prg: AdjustPrg,
    // state
    prg: i32,
}

impl Board for UxRom {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn clone_box(&self) -> Box<dyn Board> {
        Box::new(self.clone())
    }

    fn name(&self) -> &'static str {
        "UxROM"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        self.adjust_prg = AdjustPrg::Identity;

        let cart = &mut self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER0002-00" => {}
            "MAPPER002" => {
                if !BoardBase::assert_mem(cart.chr_size, &[0]) {
                    return false;
                }
                cart.vram_size = 8;
            }
            "NES-UNROM" | "HVC-UNROM" | "KONAMI-UNROM" | "NES-UNEPROM" | "IREM-UNROM"
            | "TAITO-UNROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                {
                    return false;
                }
            }
            "HVC-UN1ROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                {
                    return false;
                }
                self.adjust_prg = AdjustPrg::Un1Rom;
            }
            "NES-UOROM" | "HVC-UOROM" | "JALECO-JF-15" | "JALECO-JF-18" => {
                if !BoardBase::assert_mem(cart.prg_size, &[256])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            // NES-UNROM_VS sets NES._isVS, which this port does not emulate.
            _ => return false,
        }

        // these boards always have 8KB of VRAM
        self.vram_byte_mask = (self.base.cart.vram_size * 1024) - 1;
        self.prg_mask = (self.base.cart.prg_size / 16) - 1;
        let (h, v) = (self.base.cart.pad_h, self.base.cart.pad_v);
        self.base.set_mirror_type_pads(i32::from(h), i32::from(v));

        true
    }

    fn read_prg(&self, addr: i32) -> u8 {
        let block = addr >> 14;
        let page = if block == 1 { self.prg_mask } else { self.prg };
        let ofs = addr & 0x3FFF;
        self.base.rom[((page << 14) | ofs) as usize]
    }

    fn write_prg(&mut self, _addr: i32, value: u8) {
        let x = i32::from(value);
        let adjusted = match self.adjust_prg {
            AdjustPrg::Identity => x,
            AdjustPrg::Un1Rom => (x >> 2) & 7,
        };
        self.prg = adjusted & self.prg_mask;
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr < 0x2000 {
            self.base.vram[(addr & self.vram_byte_mask) as usize]
        } else {
            self.base.read_ppu(addr, env)
        }
    }

    fn write_ppu(&mut self, addr: i32, value: u8, env: &mut BoardEnv<'_>) {
        if addr < 0x2000 {
            let i = (addr & self.vram_byte_mask) as usize;
            self.base.vram[i] = value;
        } else {
            self.base.write_ppu(addr, value, env);
        }
    }
}
