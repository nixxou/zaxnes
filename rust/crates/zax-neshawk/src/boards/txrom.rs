//! TxROM: the plain MMC3 board (Kirby's Adventure, SMB3, Mega Man 3, Super C).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/MMC3_family/TxROM.cs` with the
//! `MMC3Board_Base` hooks of `MMC3.cs`.
//! MIT; see `LICENSE-BizHawk`.

use super::mmc3::Mmc3BoardBase;
use super::{Board, BoardBase, BoardEnv};
use crate::cart::DetectionOrigin;

#[derive(Debug, Default)]
pub struct TxRom {
    base: BoardBase,
    pub m: Mmc3BoardBase,
}

impl TxRom {
    /// Copy `MMC3.irq_signal_out` into `NesBoardBase.IrqSignal`, which in the C# happens inside
    /// `board.SyncIRQ(flag)` the moment the chip calls it.
    fn pull_irq(&mut self) {
        self.base.irq_signal = self.m.mmc3.irq_signal_out;
    }
}

impl Board for TxRom {
    fn base(&self) -> &BoardBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BoardBase {
        &mut self.base
    }
    fn name(&self) -> &'static str {
        "TxROM"
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        let cart = &mut self.base.cart;
        let ck = BoardBase::assert_mem;
        //analyze board type
        match cart.board_type.as_str() {
            "MAPPER116_HACKY" => {}
            "TXROM-HOMEBREW" => {}
            "MAPPER004" | "MAPPER0004-00" => {
                if cart.ines_mirroring == Some(2) {
                    // send these to TVROM
                    return false;
                }
            }
            "NES-TBROM" | "HVC-TBROM" => {
                if !ck(cart.prg_size, &[64])
                    || !ck(cart.chr_size, &[64])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "NES-TEROM" | "HVC-TEROM" => {
                if !ck(cart.prg_size, &[32])
                    || !ck(cart.chr_size, &[32])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "NES-TFROM" | "HVC-TFROM" => {
                if !ck(cart.prg_size, &[128])
                    || !ck(cart.chr_size, &[32, 64])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "NES-TGROM" | "HVC-TGROM" => {
                if !ck(cart.prg_size, &[128, 256, 512])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "NES-TKROM" | "HVC-TKROM" => {
                if !ck(cart.prg_size, &[128, 256, 512])
                    || !ck(cart.chr_size, &[128, 256])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "NES-TKEPROM" => {
                if !ck(cart.prg_size, &[128, 256, 512])
                    || !ck(cart.chr_size, &[128, 256])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0, 8])
                {
                    return false;
                }
            }
            "NES-TLROM" | "KONAMI-TLROM" | "HVC-TLROM" | "ACCLAIM-TLROM" => {
                if !ck(cart.prg_size, &[128, 256, 512])
                    || !ck(cart.chr_size, &[64, 128, 256])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "UNIF_NES-TLROM" => {}
            "HVC-TL1ROM" | "NES-TL1ROM" => {
                if !ck(cart.prg_size, &[128])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "NES-TL2ROM" => {
                if !ck(cart.prg_size, &[128])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "HVC-TNROM" => {
                if !ck(cart.prg_size, &[128, 256, 512])
                    || !ck(cart.chr_size, &[0, 8])
                    || !ck(cart.vram_size, &[0, 8])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "NES-TSROM" | "HVC-TSROM" => {
                if !ck(cart.prg_size, &[128, 256, 512])
                    || !ck(cart.chr_size, &[128, 256])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[8])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "UNIF_TSROM" => {
                cart.wram_size = 8;
                cart.wram_battery = false;
            }
            "ACCLAIM-MC-ACC" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[128, 256])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            "NES-B4" => {
                if !ck(cart.prg_size, &[128])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                    || cart.wram_battery
                {
                    return false;
                }
            }
            _ => return false,
        }

        let mut base = std::mem::take(&mut self.base);
        self.m.base_setup(&mut base);
        self.base = base;

        true
    }

    fn has_clock_ppu(&self) -> bool {
        true
    }

    fn clock_ppu(&mut self) {
        self.m.mmc3.clock_ppu();
        self.pull_irq();
    }

    fn address_ppu(&mut self, addr: i32) {
        self.m.mmc3.address_ppu(addr);
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        self.m.mmc3.write_prg(addr, value);
        self.pull_irq();
        let m = self.m.mmc3.mirror_type();
        self.base.set_mirror_type(m); //often redundant, but gets the job done
    }

    fn read_prg(&self, addr: i32) -> u8 {
        let mut bank_8k = self.m.mmc3.get_prg_bank_8k(addr);
        bank_8k &= self.m.prg_mask;
        let addr = (bank_8k << 13) | (addr & 0x1FFF);
        self.base.rom[addr as usize]
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr < 0x2000 {
            let addr = self.m.map_chr(addr);
            if !self.base.vrom.is_empty() {
                return self.base.vrom[(addr + self.m.extra_vrom) as usize];
            }
            return self.base.vram[addr as usize];
        }

        self.base.read_ppu(addr, env)
    }

    fn write_ppu(&mut self, addr: i32, value: u8, env: &mut BoardEnv<'_>) {
        // `MMC3Board_Base.WritePpu`. Two details of the C# that a tidy-up would lose: with no
        // VRAM (a CHR-ROM board such as Kirby's) it returns **before** `base.WritePpu`, so a
        // write below $2000 does nothing at all; and when there is VRAM it reassigns `addr` to
        // the mapped address, so `base.WritePpu` writes the same mapped byte a second time.
        let mut addr = addr;
        if addr < 0x2000 {
            if self.base.vram.is_empty() {
                return;
            }
            addr = self.m.map_chr(addr);
            let i = addr as usize;
            self.base.vram[i] = value;
        }
        self.base.write_ppu(addr, value, env);
    }
}
