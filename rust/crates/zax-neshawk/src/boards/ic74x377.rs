//! `IC_74x377`, mapper 11 and friends (Color Dreams: Crystal Mines, Metal Fighter).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/IC_74x377.cs`.
//! MIT; see `LICENSE-BizHawk`.

use super::{Board, BoardBase, BoardEnv};
use crate::cart::DetectionOrigin;

#[derive(Debug, Clone)]
pub struct Ic74x377 {
    base: BoardBase,
    // configuration
    prg_bank_mask_32k: i32,
    chr_bank_mask_8k: i32,
    bus_conflict: bool,
    bus_conflict_50282: bool,
    // state
    prg_bank_32k: i32,
    chr_bank_8k: i32,
}

impl Default for Ic74x377 {
    /// `bus_conflict` is `true` at construction in the C# (`private bool bus_conflict = true;`),
    /// which `Configure` then clears for two board types. A `derive(Default)` would start it
    /// `false` and give every Color Dreams cartridge the wrong write path.
    fn default() -> Self {
        Self {
            base: BoardBase::default(),
            prg_bank_mask_32k: 0,
            chr_bank_mask_8k: 0,
            bus_conflict: true,
            bus_conflict_50282: false,
            prg_bank_32k: 0,
            chr_bank_8k: 0,
        }
    }
}

impl Board for Ic74x377 {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn clone_box(&self) -> Box<dyn Board> {
        Box::new(self.clone())
    }

    crate::boards::board_sync!(Ic74x377);

    fn name(&self) -> &'static str {
        "IC_74x377"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        match self.base.cart.board_type.as_str() {
            "MAPPER011" | "Discrete_74x377-FLEX" | "AGCI-47516" => {}
            "MAPPER011_HACKY" => self.bus_conflict = false,
            "COLORDREAMS-74*377" => {
                let cart = &self.base.cart;
                if !BoardBase::assert_mem(cart.prg_size, &[32, 64, 128])
                    || !BoardBase::assert_mem(cart.chr_size, &[16, 32, 64, 128])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            // AGCI-50282 is Death Race.
            "AGCI-50282" | "MAPPER144" => {
                self.bus_conflict_50282 = true;
                self.bus_conflict = false;
            }
            _ => return false,
        }

        let cart = &self.base.cart;
        if !BoardBase::assert_mem(cart.prg_size, &[32, 64, 128])
            || !BoardBase::assert_mem(cart.chr_size, &[8, 16, 32, 64, 128])
        {
            return false;
        }

        self.prg_bank_mask_32k = cart.prg_size / 32 - 1;
        self.chr_bank_mask_8k = cart.chr_size / 8 - 1;
        let (h, v) = (cart.pad_h, cart.pad_v);
        self.base.set_mirror_type_pads(i32::from(h), i32::from(v));

        true
    }

    fn read_prg(&self, addr: i32) -> u8 {
        self.base.rom[(addr + (self.prg_bank_32k << 15)) as usize]
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        let mut value = value;
        if self.bus_conflict_50282 {
            // "this is what nesdev wiki does. seems to give same results as [fceux's `if (addr ==
            // 0) return;`]" — the reference's own comment, and its own choice of the two.
            value = (value | 1) & self.read_prg(addr);
        }
        if self.bus_conflict {
            // The C# asserts here that the conflict never actually changes the value ("Found a
            // test case of Discrete_74x377 bus conflict. please report."). A `Debug.Assert` is a
            // debug-build diagnostic, not behaviour, and this crate may not panic — so the
            // masking is kept and the assertion is not.
            value &= self.read_prg(addr);
        }

        let v = i32::from(value);
        self.prg_bank_32k = (v & 3) & self.prg_bank_mask_32k;
        self.chr_bank_8k = ((v >> 4) & 0xF) & self.chr_bank_mask_8k;
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr < 0x2000 {
            return self.base.vrom[(addr + (self.chr_bank_8k << 13)) as usize];
        }
        self.base.read_ppu(addr, env)
    }

    fn peek_ppu(&self, addr: i32, ciram: &[u8; 0x800]) -> u8 {
        if addr < 0x2000 {
            return self.base.vrom[(addr + (self.chr_bank_8k << 13)) as usize];
        }
        self.base.peek_ppu(addr, ciram)
    }
}

// `IC_74x377.SyncState`: the two bank registers, in the C#'s order.
crate::state::sync_struct!(Ic74x377 {
    prg_bank_32k, chr_bank_8k,
    prg_bank_mask_32k, chr_bank_mask_8k, bus_conflict, bus_conflict_50282,
} skip: { base });
