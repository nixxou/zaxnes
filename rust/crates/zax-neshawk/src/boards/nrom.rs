//! NROM, generally mapper 0.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/NROM.cs`.
//! MIT; see `LICENSE-BizHawk`.

use super::{Board, BoardBase};
use crate::cart::DetectionOrigin;

#[derive(Debug, Clone, Default)]
pub struct Nrom {
    base: BoardBase,
    /// configuration
    prg_byte_mask: i32,
}

impl Board for Nrom {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn clone_box(&self) -> Box<dyn Board> {
        Box::new(self.clone())
    }

    crate::boards::board_sync!(Nrom);

    fn name(&self) -> &'static str {
        "NROM"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER000" => {}
            // MAPPER000_VS sets NES._isVS, which this port does not emulate.
            "MAPPER0000-00" => {
                if !BoardBase::assert_mem(cart.prg_size, &[8, 16, 32])
                    || !BoardBase::assert_mem(cart.chr_size, &[8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0, 2, 4, 8])
                {
                    return false;
                }
            }
            "BANDAI-NROM-128" | "BANDAI-NROM-256" | "HVC-HROM" | "HVC-NROM-128"
            | "HVC-NROM-256" | "HVC-RROM" | "HVC-RTROM" | "HVC-SROM" | "HVC-STROM"
            | "IREM-NROM-128" | "IREM-NROM-256" | "JALECO-JF-01" | "JALECO-JF-02"
            | "JALECO-JF-03" | "JALECO-JF-04" | "KONAMI-NROM-128" | "NAMCOT-3301"
            | "NAMCOT-3302" | "NAMCOT-3303" | "NAMCOT-3304" | "NAMCOT-3305" | "NAMCOT-3311"
            | "NAMCOT-3312" | "NAMCOT-NROM-128" | "NES-NROM-128" | "NES-NROM-256"
            | "NES-RROM-128" | "SACHEN-NROM" | "SETA-NROM-128" | "SUNSOFT-NROM-128"
            | "SUNSOFT-NROM-256" | "TAITO-NROM-128" | "TAITO-NROM-256" | "TENGEN-800003"
            | "UNIF_NES-NROM-128" | "UNIF_NES-NROM-256" => {
                if !BoardBase::assert_mem(cart.prg_size, &[8, 16, 32])
                    || !BoardBase::assert_mem(cart.chr_size, &[8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0, 8])
                {
                    return false;
                }
            }
            "AVE-NINA-03" => {
                // at least one game on this board has none of the mapper chips present,
                // and emulates as simple NROM
                if !cart.chips.is_empty() {
                    return false;
                }
                if !BoardBase::assert_mem(cart.prg_size, &[8, 16, 32])
                    || !BoardBase::assert_mem(cart.chr_size, &[8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "HVC-FAMILYBASIC" => {
                // we don't emulate the controller, so this won't work
                if !BoardBase::assert_mem(cart.prg_size, &[32])
                    || !BoardBase::assert_mem(cart.chr_size, &[8])
                    || !BoardBase::assert_mem(cart.wram_size, &[2, 4])
                {
                    return false;
                }
            }
            _ => return false,
        }

        self.prg_byte_mask = (self.base.cart.prg_size * 1024) - 1;
        let (h, v) = (self.base.cart.pad_h, self.base.cart.pad_v);
        self.base.set_mirror_type_pads(i32::from(h), i32::from(v));

        true
    }

    fn read_prg(&self, addr: i32) -> u8 {
        let addr = addr & self.prg_byte_mask;
        self.base.rom[addr as usize]
    }
}

// `NROM.SyncState`: the board has no state of its own beyond the base's. `prg_byte_mask` is
// `Configure`'s, restored by the load rather than by the block, but it is one `i32` and a block
// that carries it cannot disagree with the machine it came from.
crate::state::sync_struct!(Nrom { prg_byte_mask } skip: { base });
