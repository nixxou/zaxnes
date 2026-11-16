//! `PxROM_FxROM`: MMC2 (mapper 9, Mike Tyson's Punch-Out!!) and MMC4 (mapper 10, Fire Emblem,
//! Famicom Wars) — "similar enough to combine in one file", as the reference puts it.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/PxROM_FxROM.cs`.
//! MIT; see `LICENSE-BizHawk`.
//!
//! # The latches, and the one thing the 3D capture cannot do with them
//!
//! These two boards are the reason REQ-CORE-033 and REQ-CORE-039 exist. The pattern a tile shows
//! depends on the tiles fetched **before** it on the line, through the `$FD`/`$FE` latches that
//! `read_ppu` moves as it reads. The C# therefore has a `PeekPPU` that is `ReadPpu` *without the
//! latch logic*, and so does this port — `peek_ppu` here is that method, line for line.
//!
//! `zax-nes`'s capture goes further: it walks a line in display order moving the latch forward
//! tile by tile, and puts the real latch back afterwards. This port's capture reads through
//! `peek_ppu` and so sees the latch as the frame left it, which for an MMC2/MMC4 game can put a
//! tile in the other 4 KiB bank. It is the one named difference in the capture-parity measurement
//! of `docs/review/NESHAWK-REWRITE.md`, and it is a capture difference only: the *emulation* is
//! the reference's, latches and all.

use super::{Board, BoardBase, BoardEnv, MirrorType};
use crate::cart::DetectionOrigin;

#[derive(Debug, Clone, Default)]
pub struct PxRom {
    base: BoardBase,
    // configuration
    prg_bank_mask_8k: i32,
    chr_bank_mask_4k: i32,
    mmc4: bool,
    // state
    prg_reg: u8,
    prg_banks_8k: [i32; 4],
    chr_banks_4k: [i32; 4],
    chr_latches: [i32; 2],
}

impl PxRom {
    /// `SyncPRG`.
    fn sync_prg(&mut self) {
        let reg = i32::from(self.prg_reg);
        if self.mmc4 {
            self.prg_banks_8k[0] = (reg * 2) & self.prg_bank_mask_8k;
            self.prg_banks_8k[1] = (reg * 2 + 1) & self.prg_bank_mask_8k;
            self.prg_banks_8k[2] = 0xFE & self.prg_bank_mask_8k;
            self.prg_banks_8k[3] = 0xFF & self.prg_bank_mask_8k;
        } else {
            self.prg_banks_8k[0] = reg & self.prg_bank_mask_8k;
            self.prg_banks_8k[1] = 0xFD & self.prg_bank_mask_8k;
            self.prg_banks_8k[2] = 0xFE & self.prg_bank_mask_8k;
            self.prg_banks_8k[3] = 0xFF & self.prg_bank_mask_8k;
        }
    }

    /// The CHR address `addr` maps to, which `read_ppu` and `peek_ppu` share: the C# repeats the
    /// four lines in both, and they are identical.
    fn chr_address(&self, addr: i32) -> i32 {
        let side = addr >> 12;
        let reg = side * 2 + self.chr_latches[side as usize];
        let ofs = addr & ((1 << 12) - 1);
        let bank_4k = self.chr_banks_4k[reg as usize];
        (bank_4k << 12) | ofs
    }
}

impl Board for PxRom {
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

    /// `SyncState` ends with `if (ser.IsReader) SyncPRG();` — the C# does **not** serialise
    /// `prg_banks_8k`, it recomputes them from `prg_reg`. Transcribed, because a block of this
    /// board is two bytes shorter for it and because recomputing is what the reference does.
    fn read_state(&mut self, reader: &mut crate::state::StateReader<'_>) {
        let mut base = std::mem::take(&mut self.base);
        crate::state::SyncState::read_state(&mut base, reader);
        crate::state::SyncState::read_state(self, reader);
        self.base = base;
        self.sync_prg();
    }

    fn name(&self) -> &'static str {
        "PxROM_FxROM"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER009" => {}
            "MAPPER010" => self.mmc4 = true,
            // Punch-Out!!
            "NES-PNROM" | "HVC-PEEOROM" | "NES-PEEOROM" => {
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[128])
                    || !BoardBase::assert_mem(cart.wram_size, &[0])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                {
                    return false;
                }
            }
            // Fire Emblem
            "HVC-FKROM" => {
                self.mmc4 = true;
                if !BoardBase::assert_mem(cart.prg_size, &[256])
                    || !BoardBase::assert_mem(cart.chr_size, &[128])
                    || !BoardBase::assert_mem(cart.wram_size, &[8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                {
                    return false;
                }
            }
            // Famicom Wars
            "HVC-FJROM" => {
                self.mmc4 = true;
                if !BoardBase::assert_mem(cart.prg_size, &[128])
                    || !BoardBase::assert_mem(cart.chr_size, &[64])
                    || !BoardBase::assert_mem(cart.wram_size, &[8])
                    || !BoardBase::assert_mem(cart.vram_size, &[0])
                {
                    return false;
                }
            }
            _ => return false,
        }

        self.prg_bank_mask_8k = self.base.cart.prg_size / 8 - 1;
        self.chr_bank_mask_4k = self.base.cart.chr_size / 4 - 1;
        self.sync_prg();

        // The C#'s `Configure` sets no mirroring: `$F000` does, and until a game writes it the
        // board keeps `NesBoardBase`'s own default. Not "forgotten" — transcribed.

        true
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        match addr & 0xF000 {
            // $A000: PRG Reg
            0x2000 => {
                self.prg_reg = value;
                self.sync_prg();
            }
            // $B000: CHR Reg 0A
            0x3000 => self.chr_banks_4k[0] = i32::from(value) & self.chr_bank_mask_4k,
            // $C000: CHR Reg 0B
            0x4000 => self.chr_banks_4k[1] = i32::from(value) & self.chr_bank_mask_4k,
            // $D000: CHR Reg 1A
            0x5000 => self.chr_banks_4k[2] = i32::from(value) & self.chr_bank_mask_4k,
            // $E000: CHR Reg 1B
            0x6000 => self.chr_banks_4k[3] = i32::from(value) & self.chr_bank_mask_4k,
            // $F000: [.... ...M] Mirroring
            0x7000 => {
                let mirror = if value & 1 != 0 {
                    MirrorType::Horizontal
                } else {
                    MirrorType::Vertical
                };
                self.base.set_mirror_type(mirror);
            }
            _ => {}
        }
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr >= 0x2000 {
            return self.base.read_ppu(addr, env);
        }
        let side = addr >> 12;
        let tile = (addr >> 4) & 0xFF;
        let mapped = self.chr_address(addr);

        // "if we're grabbing the second byte of the tile, then apply the tile switching logic
        // (the next tile will be rendered with the fiddled register settings)" — and the test is
        // on the **mapped** address, which is the reference's `addr` after it was reassigned.
        if (mapped & 0xF) >= 0x8 {
            match tile {
                0xFD => self.chr_latches[side as usize] = 0,
                0xFE => self.chr_latches[side as usize] = 1,
                _ => {}
            }
        }
        self.base.vrom[mapped as usize]
    }

    /// `PeekPPU`: "same as readppu but without processing latches".
    fn peek_ppu(&self, addr: i32, ciram: &[u8; 0x800]) -> u8 {
        if addr >= 0x2000 {
            return self.base.peek_ppu(addr, ciram);
        }
        self.base.vrom[self.chr_address(addr) as usize]
    }

    fn read_prg(&self, addr: i32) -> u8 {
        let bank_8k = self.prg_banks_8k[(addr >> 13) as usize];
        let ofs = addr & ((1 << 13) - 1);
        self.base.rom[((bank_8k << 13) | ofs) as usize]
    }
}

// `PxROM_FxROM.SyncState`, in the C#'s order: `prg_reg`, `chr_banks_4k`, `chr_latches`.
// `prg_banks_8k` is **not** in it — `read_state` recomputes it with `SyncPRG`, as the C# does.
crate::state::sync_struct!(PxRom {
    prg_reg, chr_banks_4k, chr_latches,
    prg_bank_mask_8k, chr_bank_mask_4k, mmc4,
} skip: { base, prg_banks_8k });
