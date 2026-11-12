//! CNROM, generally mapper 3 (Solomon's Key, Arkanoid, Bump 'n' Jump, Cybernoid).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/CNROM.cs`.
//! MIT; see `LICENSE-BizHawk`.

use super::{Board, BoardBase, BoardEnv};
use crate::cart::DetectionOrigin;

/// `NES.RomChecksums.SeicrossRev2` — "yes this is meant to include the prefix".
const SEICROSS_REV2: &str = "SHA1:4C9C05FAD6F6F33A92A27C2EDC1E7DE12D7F216D";

#[derive(Debug)]
pub struct CnRom {
    base: BoardBase,
    // configuration
    prg_byte_mask: i32,
    chr_mask: i32,
    copyprotection: bool,
    bus_conflict: bool,
    seicross: bool,
    // state
    chr: i32,
    chr_enabled: bool,
}

impl CnRom {
    pub fn new() -> Self {
        Self {
            base: BoardBase::default(),
            prg_byte_mask: 0,
            chr_mask: 0,
            copyprotection: false,
            bus_conflict: false,
            seicross: false,
            chr: 0,
            chr_enabled: true,
        }
    }
}

impl Board for CnRom {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn name(&self) -> &'static str {
        "CNROM"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER185" | "HVC-CNROM+SECURITY" | "HVC-CNROM-256K-01" => {
                self.copyprotection = true;
                self.bus_conflict = true;
                if !BoardBase::assert_mem(cart.prg_size, &[16, 32])
                    || !BoardBase::assert_mem(cart.chr_size, &[8])
                {
                    return false;
                }
            }
            "MAPPER003" => {
                // we assume no bus conflicts for generic unknown cases.
                self.bus_conflict = false;
                if !BoardBase::assert_mem(cart.prg_size, &[8, 16, 32]) {
                    return false;
                }
            }
            "Sachen_CNROM" => {
                self.bus_conflict = false;
                if !BoardBase::assert_mem(cart.prg_size, &[16, 32]) {
                    return false;
                }
            }
            "NES-CNROM" | "UNIF_NES-CNROM" | "HVC-CNROM" | "TAITO-CNROM" | "BANDAI-CNROM"
            | "BANDAI-74*161/32" => {
                self.bus_conflict = true;
                if !BoardBase::assert_mem(cart.prg_size, &[16, 32]) {
                    return false;
                }
            }
            "KONAMI-CNROM" => {
                self.bus_conflict = true;
                if !BoardBase::assert_mem(cart.prg_size, &[32])
                    || !BoardBase::assert_mem(cart.chr_size, &[32])
                {
                    return false;
                }
            }
            "AVE-74*161" => {
                self.bus_conflict = true;
                if !BoardBase::assert_mem(cart.prg_size, &[32])
                    || !BoardBase::assert_mem(cart.chr_size, &[64])
                {
                    return false;
                }
            }
            "NTDEC-N715062" | "NTDEC-N715061" => {
                self.bus_conflict = true;
                if !BoardBase::assert_mem(cart.prg_size, &[16, 32])
                    || !BoardBase::assert_mem(cart.chr_size, &[8, 16, 32])
                {
                    return false;
                }
            }
            "BANDAI-PT-554" => {
                if !BoardBase::assert_mem(cart.prg_size, &[32])
                    || !BoardBase::assert_mem(cart.chr_size, &[32])
                {
                    return false;
                }
                self.bus_conflict = true;
            }
            "NAMCOT-CNROM+WRAM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[32])
                    || !BoardBase::assert_mem(cart.chr_size, &[32])
                    || !BoardBase::assert_mem(cart.wram_size, &[2])
                {
                    return false;
                }
            }
            _ => return false,
        }

        if self.base.cart.pcb.as_deref() == Some("9011-N02") {
            // othello
            self.copyprotection = true;
        }
        self.prg_byte_mask = self.base.cart.prg_size * 1024 - 1;
        self.chr_mask = (self.base.cart.chr_size / 8) - 1;
        let (h, v) = (self.base.cart.pad_h, self.base.cart.pad_v);
        self.base.set_mirror_type_pads(i32::from(h), i32::from(v));

        if self.base.cart.sha1.as_deref() == Some(SEICROSS_REV2) {
            self.seicross = true;
        }

        true
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        let mut value = value;
        if self.bus_conflict {
            // HandleNormalPRGConflict
            value &= self.read_prg(addr);
        }

        self.chr = i32::from(value) & self.chr_mask;

        if self.copyprotection {
            if self.seicross {
                self.chr_enabled = value != 0x21;
            } else {
                self.chr_enabled = (value & 0x0F) > 0 && (value != 0x13);
            }
        }
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if !self.chr_enabled {
            return 0x12;
        }
        if addr < 0x2000 {
            self.base.vrom[(addr + (self.chr << 13)) as usize]
        } else {
            self.base.read_ppu(addr, env)
        }
    }

    fn read_prg(&self, addr: i32) -> u8 {
        self.base.rom[(addr & self.prg_byte_mask) as usize]
    }
}
