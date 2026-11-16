//! Camerica/Codemasters: mapper 71 (`Camerica_Mapper071`) and mapper 232
//! (`Camerica_Mapper232`).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/Camerica.cs`.
//! MIT; see `LICENSE-BizHawk`.

use super::{Board, BoardBase, MirrorType};
use crate::cart::DetectionOrigin;

/// Mapper 71. The reference's own note: "apparently this mapper contains good nes timing test
/// cases".
#[derive(Debug, Clone, Default)]
pub struct Camerica071 {
    base: BoardBase,
    // configuration
    prg_bank_mask_16k: i32,
    mirror_control_enabled: bool,
    // state
    prg_banks_16k: [i32; 2],
}

impl Board for Camerica071 {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn clone_box(&self) -> Box<dyn Board> {
        Box::new(self.clone())
    }

    crate::boards::board_sync!(Camerica071);

    fn name(&self) -> &'static str {
        "Camerica_Mapper071"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER071" => {}
            // Linus Spacehead's Cosmic Crusade (U). The C#'s own note on the 16: "zero
            // 22-mar-2012 - added 16 here as a hack to make micro machines (aladdin) load.
            // should remove this after it is fixed in the DB".
            "CAMERICA-ALGN" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128, 256])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8, 16])
                {
                    return false;
                }
            }
            // Big Nose Freaks Out (U)
            "CAMERICA-BF9093" => {
                if !BoardBase::assert_mem(cart.prg_size, &[64, 128, 256])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                {
                    return false;
                }
            }
            // Fire Hawk, the one board whose $8000-$9FFF controls the mirroring.
            "CAMERICA-BF9097" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                {
                    return false;
                }
                self.mirror_control_enabled = true;
            }
            // "untested", says the reference.
            "CODEMASTERS-NR8N" => {
                if !BoardBase::assert_mem(cart.prg_size, &[256])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                {
                    return false;
                }
            }
            _ => return false,
        }

        self.prg_bank_mask_16k = self.base.cart.prg_size / 16 - 1;
        self.prg_banks_16k[0] = 0x00;
        self.prg_banks_16k[1] = 0xFF & self.prg_bank_mask_16k;
        let (h, v) = (self.base.cart.pad_h, self.base.cart.pad_v);
        self.base.set_mirror_type_pads(i32::from(h), i32::from(v));

        true
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        match addr & 0x7000 {
            // $8000-$9FFF: [...M ....] Mirroring (for Fire Hawk only!)
            0x0000 | 0x1000 => {
                if self.mirror_control_enabled {
                    let mirror = if (value >> 4) & 1 != 0 {
                        MirrorType::OneScreenB
                    } else {
                        MirrorType::OneScreenA
                    };
                    self.base.set_mirror_type(mirror);
                }
            }
            // $C000-$FFFF: PRG Select (16k @ $8000)
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                self.prg_banks_16k[0] = i32::from(value) & self.prg_bank_mask_16k;
            }
            _ => {}
        }
    }

    fn read_prg(&self, addr: i32) -> u8 {
        let bank_16k = self.prg_banks_16k[(addr >> 14) as usize];
        let ofs = addr & ((1 << 14) - 1);
        self.base.rom[((bank_16k << 14) | ofs) as usize]
    }
}

/// Mapper 232: Quattro Adventure (U), Quattro Arcade (U).
#[derive(Debug, Clone, Default)]
pub struct Camerica232 {
    base: BoardBase,
    // configuration
    prg_bank_mask_16k: i32,
    // state
    prg_banks_16k: [i32; 2],
    prg_block: i32,
    prg_page: i32,
}

impl Camerica232 {
    /// `SyncPRG`.
    fn sync_prg(&mut self) {
        self.prg_banks_16k[0] = (self.prg_block << 2) | self.prg_page;
        self.prg_banks_16k[1] = (self.prg_block << 2) | 3;
        self.prg_banks_16k[0] &= self.prg_bank_mask_16k;
        self.prg_banks_16k[1] &= self.prg_bank_mask_16k;
    }
}

impl Board for Camerica232 {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn clone_box(&self) -> Box<dyn Board> {
        Box::new(self.clone())
    }

    crate::boards::board_sync!(Camerica232);

    fn name(&self) -> &'static str {
        "Camerica_Mapper232"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER232" => {}
            "CAMERICA-ALGQ" | "CAMERICA-BF9096" => {
                if !BoardBase::assert_mem(cart.prg_size, &[256])
                    || !BoardBase::assert_mem(cart.chr_size, &[0])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[8])
                {
                    return false;
                }
            }
            _ => return false,
        }

        self.prg_bank_mask_16k = self.base.cart.prg_size / 16 - 1;
        let (h, v) = (self.base.cart.pad_h, self.base.cart.pad_v);
        self.base.set_mirror_type_pads(i32::from(h), i32::from(v));
        self.sync_prg();

        true
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        // `addr &= 0x4000` in the C#, so the switch has exactly two arms.
        if addr & 0x4000 == 0 {
            self.prg_block = (i32::from(value) >> 3) & 3;
        } else {
            self.prg_page = i32::from(value) & 3;
        }
        self.sync_prg();
    }

    fn read_prg(&self, addr: i32) -> u8 {
        let bank_16k = self.prg_banks_16k[(addr >> 14) as usize];
        let ofs = addr & ((1 << 14) - 1);
        self.base.rom[((bank_16k << 14) | ofs) as usize]
    }
}

// `Camerica_Mapper071.SyncState` and `Camerica_Mapper232.SyncState`.
crate::state::sync_struct!(Camerica071 {
    prg_banks_16k, prg_bank_mask_16k, mirror_control_enabled,
} skip: { base });

crate::state::sync_struct!(Camerica232 {
    prg_banks_16k, prg_block, prg_page, prg_bank_mask_16k,
} skip: { base });
