//! GxROM, generally mapper 66 (Doraemon, Dragon Power, Gumshoe, Thunder & Lightning, SMB +
//! Duck Hunt).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/GxROM.cs`.
//! MIT; see `LICENSE-BizHawk`.

use super::{Board, BoardBase, BoardEnv};
use crate::cart::DetectionOrigin;

#[derive(Debug, Clone, Default)]
pub struct GxRom {
    base: BoardBase,
    // configuration
    prg_mask: i32,
    chr_mask: i32,
    // state
    prg: i32,
    chr: i32,
}

impl Board for GxRom {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn clone_box(&self) -> Box<dyn Board> {
        Box::new(self.clone())
    }

    crate::boards::board_sync!(GxRom);

    fn name(&self) -> &'static str {
        "GxROM"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &mut self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER066" => {
                if !BoardBase::assert_mem(cart.prg_size, &[32, 64, 128])
                    || !BoardBase::assert_mem(cart.chr_size, &[8, 16, 32, 64])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0, 8])
                {
                    return false;
                }
            }
            // Thunder & Lightning is 128 PRG; there are no known 64 PRG; U-Force Power Games
            // (U) (Prototype) uses 32 PRG.
            "NES-GNROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[32, 128])
                    || !BoardBase::assert_mem(cart.chr_size, &[8, 16, 32])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            // `NES-MHROM` is Super Mario Bros. / Duck Hunt.
            board @ ("BANDAI-GNROM" | "HVC-GNROM" | "NES-MHROM") => {
                let prg = if board == "NES-MHROM" { 64 } else { 128 };
                if !BoardBase::assert_mem(cart.prg_size, &[prg])
                    || !BoardBase::assert_mem(cart.chr_size, &[8, 16, 32])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            _ => return false,
        }

        self.prg_mask = (self.base.cart.prg_size / 32) - 1;
        self.chr_mask = (self.base.cart.chr_size / 8) - 1;
        let (h, v) = (self.base.cart.pad_h, self.base.cart.pad_v);
        self.base.set_mirror_type_pads(i32::from(h), i32::from(v));

        // The C# writes "Caution! This board (inferred from iNES) might have wrong mirr.type" to
        // the console for `EDetectionOrigin.INES`; the port has no console to write it to, and
        // `LoadReport` already records the detection origin.

        true
    }

    fn read_prg(&self, addr: i32) -> u8 {
        self.base.rom[(addr + (self.prg << 15)) as usize]
    }

    fn write_prg(&mut self, _addr: i32, value: u8) {
        // TODO of the reference: bus conflicts.
        let v = i32::from(value);
        self.chr = (v & 7) & self.chr_mask;
        self.prg = ((v >> 4) & 3) & self.prg_mask;
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr < 0x2000 {
            return self.base.vrom[(addr + (self.chr << 13)) as usize];
        }
        self.base.read_ppu(addr, env)
    }

    fn peek_ppu(&self, addr: i32, ciram: &[u8; 0x800]) -> u8 {
        if addr < 0x2000 {
            return self.base.vrom[(addr + (self.chr << 13)) as usize];
        }
        self.base.peek_ppu(addr, ciram)
    }
}

// `GxROM.SyncState`: `chr` and `prg`, in the C#'s order.
crate::state::sync_struct!(GxRom {
    chr, prg, prg_mask, chr_mask,
} skip: { base });
