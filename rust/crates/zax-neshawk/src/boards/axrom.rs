//! AxROM, generally mapper 7 (Marble Madness, Battletoads, Time Lord).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/AxROM.cs`.
//! MIT; see `LICENSE-BizHawk`.

use super::{Board, BoardBase, MirrorType};
use crate::cart::DetectionOrigin;

#[derive(Debug, Default)]
pub struct AxRom {
    base: BoardBase,
    // configuration
    bus_conflict: bool,
    prg_mask_32k: i32,
    // state
    prg: i32,
}

impl Board for AxRom {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn name(&self) -> &'static str {
        "AxROM"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &mut self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER007" => {
                self.bus_conflict = false;
                cart.vram_size = 8;
            }
            "NES-ANROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
                self.bus_conflict = false;
            }
            "NES-AN1ROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[64])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
                self.bus_conflict = false;
            }
            "NES-AMROM" | "HVC-AMROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
                self.bus_conflict = false;
            }
            "NES-AOROM" | "HVC-AOROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
                self.bus_conflict = false;
            }
            "ACCLAIM-AOROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[256])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
                self.bus_conflict = true; // not enough chips on the pcb to disable bus conflicts?
            }
            _ => return false,
        }

        self.prg_mask_32k = self.base.cart.prg_size / 32 - 1;
        self.base.set_mirror_type(MirrorType::OneScreenA);

        true
    }

    fn read_prg(&self, addr: i32) -> u8 {
        if self.base.cart.prg_size == 16 {
            return self.base.rom[((addr & 0x3FFF) | (self.prg << 15)) as usize];
        }
        self.base.rom[(addr | (self.prg << 15)) as usize]
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        let mut value = value;
        if !self.base.rom.is_empty() && self.bus_conflict {
            value &= self.read_prg(addr);
        }

        self.prg = i32::from(value) & self.prg_mask_32k;
        self.base.set_mirror_type(if (value & 0x10) == 0 {
            MirrorType::OneScreenA
        } else {
            MirrorType::OneScreenB
        });
    }
}
