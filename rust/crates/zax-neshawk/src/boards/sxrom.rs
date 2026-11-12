//! SxROM, AKA MMC1 (Final Fantasy, Mega Man 2, Blaster Master, Metroid, Zelda, Faxanadu).
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/SxROM.cs`, which holds the `MMC1`
//! chip, its serial controller, and the four board classes `SxROM`, `SuROM`, `SXROM` and `SoROM`.
//! MIT; see `LICENSE-BizHawk`.

use super::{Board, BoardBase, BoardEnv, MirrorType};
use crate::cart::DetectionOrigin;

/// `SxROM.cs`'s `MMC1`.
#[derive(Debug, Clone)]
pub struct Mmc1 {
    // the serial controller's state (`MMC1_SerialController`)
    shift_count: i32,
    shift_val: i32,

    //register 0:
    pub chr_mode: i32,
    pub prg_mode: i32,
    /// complicated
    pub prg_slot: i32,
    pub mirror: MirrorType,

    //register 1,2:
    chr_0: i32,
    chr_1: i32,

    //register 3:
    pub wram_disable: bool,
    prg: i32,

    //regenerable state
    chr_banks_4k: [i32; 2],
    prg_banks_16k: [i32; 2],
}

/// `MMC1._mirrorTypes`.
const MIRROR_TYPES: [MirrorType; 4] = [
    MirrorType::OneScreenA,
    MirrorType::OneScreenB,
    MirrorType::Vertical,
    MirrorType::Horizontal,
];

impl Default for Mmc1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Mmc1 {
    /// The C# constructor: wire the delegates, `StandardReset()`, then `SyncCHR()`.
    #[must_use]
    pub fn new() -> Self {
        let mut mmc1 = Self {
            shift_count: 0,
            shift_val: 0,
            chr_mode: 0,
            prg_mode: 0,
            prg_slot: 0,
            mirror: MirrorType::Vertical,
            chr_0: 0,
            chr_1: 0,
            wram_disable: false,
            prg: 0,
            chr_banks_4k: [0; 2],
            prg_banks_16k: [0; 2],
        };
        mmc1.standard_reset();
        mmc1.sync_chr();
        mmc1
    }

    /// `SerialReset`, which is the serial controller's `Reset` delegate.
    fn serial_reset(&mut self) {
        self.prg_mode = 1;
        self.prg_slot = 1;
    }

    /// `StandardReset`.
    pub fn standard_reset(&mut self) {
        self.prg_mode = 1;
        self.prg_slot = 1;
        self.chr_mode = 1;
        self.serial_reset(); // scnt.Reset()
        self.mirror = MirrorType::Horizontal;
        self.sync_chr();
        self.sync_prg();
    }

    /// `MMC1.Write`: the serial controller, then both syncs.
    pub fn write(&mut self, addr: i32, value: u8) {
        self.scnt_write(addr, value);
        self.sync_chr();
        self.sync_prg();
    }

    /// `MMC1_SerialController.Write`.
    fn scnt_write(&mut self, addr: i32, value: u8) {
        let data = i32::from(value) & 1;
        let reset = (i32::from(value) >> 7) & 1;
        if reset == 1 {
            self.shift_count = 0;
            self.shift_val = 0;
            self.serial_reset();
        } else {
            self.shift_val >>= 1;
            self.shift_val |= data << 4;
            self.shift_count += 1;
            if self.shift_count == 5 {
                let v = self.shift_val;
                self.serial_write_register(addr >> 13, v);
                self.shift_count = 0;
                self.shift_val = 0;
            }
        }
    }

    /// `SerialWriteRegister`: the logical register writes.
    fn serial_write_register(&mut self, addr: i32, value: i32) {
        match addr {
            0 => {
                //8000-9FFF
                self.mirror = MIRROR_TYPES[(value & 3) as usize];
                self.prg_slot = (value >> 2) & 1;
                self.prg_mode = (value >> 3) & 1;
                self.chr_mode = (value >> 4) & 1;
            }
            1 => self.chr_0 = value & 0x1F, //A000-BFFF
            2 => self.chr_1 = value & 0x1F, //C000-DFFF
            3 => {
                //E000-FFFF
                self.prg = value & 0xF;
                self.wram_disable = (value & 0x10) != 0;
            }
            _ => {}
        }
    }

    fn sync_chr(&mut self) {
        if self.chr_mode == 0 {
            self.chr_banks_4k[0] = self.chr_0 & !1;
            self.chr_banks_4k[1] = (self.chr_0 & !1) + 1;
        } else {
            self.chr_banks_4k[0] = self.chr_0;
            self.chr_banks_4k[1] = self.chr_1;
        }
    }

    fn sync_prg(&mut self) {
        if self.prg_mode == 0 {
            //switch 32kb
            self.prg_banks_16k[0] = self.prg & !1;
            self.prg_banks_16k[1] = (self.prg & !1) + 1;
        } else {
            //switch 16KB at...
            if self.prg_slot == 0 {
                //...$C000:
                self.prg_banks_16k[0] = 0x00;
                self.prg_banks_16k[1] = self.prg;
            } else {
                //...$8000:
                self.prg_banks_16k[0] = self.prg;
                self.prg_banks_16k[1] = 0x0F;
            }
        }
    }

    #[must_use]
    pub fn get_prg_bank(&self, addr: i32) -> i32 {
        self.prg_banks_16k[(addr >> 14) as usize]
    }

    #[must_use]
    pub fn get_chr_bank_4k(&self, addr: i32) -> i32 {
        self.chr_banks_4k[(addr >> 12) as usize]
    }
}

/// Which of the four C# classes this instance is. They differ only in `Configure` and in a
/// handful of overrides, which is why one Rust type carries all four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SxVariant {
    /// `SxROM`
    SxRom,
    /// `SuROM` — CHR A16 drives PRG A18 over 512 KiB
    SuRom,
    /// `SXROM` — SuROM's PRG plus SoROM-style WRAM banking
    SxRom512,
    /// `SoROM` — a CHR bit selects the WRAM bank
    SoRom,
}

/// "i don't know if this is right, but anything lower will not boot Bill & Ted"
const PPUTIMEOUT: u32 = 4;

#[derive(Debug, Clone)]
pub struct SxRom {
    base: BoardBase,
    variant: SxVariant,
    // configuration
    prg_mask: i32,
    chr_mask: i32,
    vram_mask: i32,
    /// mapper 171: mmc1 without mirroring control
    disablemirror: bool,
    /// for snrom wram disable
    is_snrom: bool,
    chr_wram_enable: bool,
    // state
    pub mmc1: Mmc1,
    /// number of cycles since last WritePRG()
    ppuclock: u32,
}

impl SxRom {
    #[must_use]
    pub fn new(variant: SxVariant) -> Self {
        Self {
            base: BoardBase::default(),
            variant,
            prg_mask: 0,
            chr_mask: 0,
            vram_mask: 0,
            disablemirror: false,
            is_snrom: false,
            chr_wram_enable: true,
            mmc1: Mmc1::new(),
            ppuclock: 0,
        }
    }

    /// `SxROM.BaseConfigure`.
    fn base_configure(&mut self) {
        self.mmc1 = Mmc1::new();
        self.prg_mask = (self.base.cart.prg_size / 16) - 1;
        self.vram_mask = (self.base.cart.vram_size * 1024) - 1;
        self.chr_mask = (self.base.cart.chr_size / 8 * 2) - 1;
        //Chip n Dale (PC10) has a nonstandard chr size, which makes the mask nonsense
        if self.base.cart.chr_size == 136 {
            self.chr_mask = (128 / 8 * 2) - 1;
        }

        if !self.disablemirror {
            let m = self.mmc1.mirror;
            self.base.set_mirror_type(m);
        }
        self.ppuclock = PPUTIMEOUT;
    }

    /// `SxROM.Gen_CHR_Address`.
    fn gen_chr_address(&self, addr: i32) -> i32 {
        let bank = self.mmc1.get_chr_bank_4k(addr);
        ((bank & self.chr_mask) << 12) | (addr & 0x0FFF)
    }

    /// `SoROM.Map_WRAM`.
    fn map_wram_sorom(&self, addr: i32) -> i32 {
        //$A000-BFFF:  [...R ...C]
        //  R = PRG-RAM page select
        //  C = CHR reg 0
        //* BUT THIS IS WRONG ??? R IS ONE BIT LOWER !!!??? ?!? *
        let chr_bank = self.mmc1.get_chr_bank_4k(0);
        let ofs = addr & ((8 * 1024) - 1);
        let wram_bank_8k = (chr_bank >> 3) & 1;
        (wram_bank_8k << 13) | ofs
    }

    /// `SXROM.Map_WRAM`.
    fn map_wram_sxrom512(&self, addr: i32) -> i32 {
        //$A000-BFFF:  [...P RR..]
        //  P = PRG-ROM 256k block select (just like on SUROM)
        //  R = PRG-RAM page select (selects 8k @ $6000-7FFF, just like SOROM)
        let chr_bank = self.mmc1.get_chr_bank_4k(0);
        let ofs = addr & ((8 * 1024) - 1);
        let wram_bank_8k = (chr_bank >> 2) & 3;
        (wram_bank_8k << 13) | ofs
    }

    /// The WRAM address mapping of this variant. `SxROM` and `SuROM` do not remap.
    fn map_wram(&self, addr: i32) -> i32 {
        match self.variant {
            SxVariant::SoRom => self.map_wram_sorom(addr),
            SxVariant::SxRom512 => self.map_wram_sxrom512(addr),
            _ => addr,
        }
    }

    fn configure_sxrom(&mut self) -> bool {
        let cart = &mut self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER116_HACKY" => {}
            // MAPPER001_VS sets NES._isVS, which this port does not emulate.
            "MAPPER001" | "NES-SIEPROM" => {
                // there's no way to define PRG oversize for mapper001 due to how the MMC1 regs
                // work, so 512KB must mean SUROM or SXROM. SUROM is more common, so we try that
                if cart.prg_size > 256 {
                    return false;
                }
            }
            "MAPPER171" => {
                // Tui Do Woo Ma Jeung
                if !BoardBase::assert_mem(cart.prg_size, &[32])
                    || !BoardBase::assert_mem(cart.chr_size, &[32])
                {
                    return false;
                }
                cart.wram_size = 0;
                self.disablemirror = true;
                let (h, v) = (cart.pad_h, cart.pad_v);
                self.base.set_mirror_type_pads(i32::from(h), i32::from(v));
            }
            "NES-SAROM" => {
                if !ck(cart.prg_size, &[64])
                    || !ck(cart.chr_size, &[16, 32, 64])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "NES-SBROM" => {
                if !ck(cart.prg_size, &[64])
                    || !ck(cart.chr_size, &[16, 32, 64])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "NES-SCROM" | "NES-SC1ROM" => {
                if !ck(cart.prg_size, &[64])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "NES-SEROM" | "HVC-SEROM" => {
                if !ck(cart.prg_size, &[32])
                    || !ck(cart.chr_size, &[16, 32])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "NES-SFROM" | "HVC-SFROM" | "NES-SF1ROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[16, 32, 64])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "NES-SGROM" | "HVC-SGROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "NES-SHROM" | "NES-SH1ROM" => {
                if !ck(cart.prg_size, &[32])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "HVC-SIROM" => {
                if !ck(cart.prg_size, &[32])
                    || !ck(cart.chr_size, &[16, 64])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "HVC-SJROM" => {
                if !ck(cart.prg_size, &[128])
                    || !ck(cart.chr_size, &[32])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0, 8])
                {
                    return false;
                }
            }
            "NES-SJROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[16, 32, 64])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "NES-SKROM" | "HVC-SKROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "HVC-SKEPROM" | "NES-SKEPROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0, 8])
                {
                    return false;
                }
            }
            "NES-SLROM" | "KONAMI-SLROM" | "HVC-SLROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "HVC-SL1ROM" | "NES-SL1ROM" => {
                if !ck(cart.prg_size, &[64, 128, 256])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "NES-SL2ROM" => {
                if !ck(cart.prg_size, &[128])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "NES-SL3ROM" => {
                if !ck(cart.prg_size, &[256])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "NES-SLRROM" | "HVC-SLRROM" => {
                if !ck(cart.prg_size, &[128])
                    || !ck(cart.chr_size, &[128])
                    || !ck(cart.vram_size, &[0])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "HVC-SMROM" => {
                if !ck(cart.prg_size, &[256])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[0])
                {
                    return false;
                }
            }
            "HVC-SNROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[0, 8])
                    || !ck(cart.vram_size, &[0, 8])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "HVC-SNROM-03" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "NES-SNROM" => {
                self.is_snrom = true;
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "VIRGIN-SNROM" | "NES-SNWEPROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            "SxROM-JUNK" => {}
            _ => return false,
        }

        self.base_configure();
        true
    }

    fn configure_surom(&mut self) -> bool {
        //SUROM uses CHR A16 to control the upper address line (PRG A18) of its 512KB PRG ROM.
        let cart = &mut self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER001" => {
                // we try to heuristic match to iNES 001 roms with big PRG only
                if cart.prg_size <= 256 {
                    return false;
                }
                if !ck(cart.prg_size, &[512]) || !ck(cart.chr_size, &[0]) {
                    return false;
                }
                cart.vram_size = 8;
                cart.wram_size = 8;
                cart.wram_battery = true; // all SUROM boards had batteries
            }
            "NES-SUROM" | "HVC-SUROM" => {
                if !ck(cart.prg_size, &[512])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[8])
                {
                    return false;
                }
            }
            _ => return false,
        }

        self.base_configure();
        true
    }

    fn configure_sxrom512(&mut self) -> bool {
        let cart = &self.base.cart;
        match cart.board_type.as_str() {
            "MAPPER0001-00" => {
                if cart.wram_size < 32 {
                    return false;
                }
                if !ck(cart.prg_size, &[128, 256, 512])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[32])
                {
                    return false;
                }
            }
            "HVC-SXROM" => {
                if !ck(cart.prg_size, &[128, 256, 512])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[32])
                {
                    return false;
                }
            }
            _ => return false,
        }

        self.base_configure();
        true
    }

    fn configure_sorom(&mut self) -> bool {
        let cart = &self.base.cart;
        match cart.board_type.as_str() {
            "NES-SOROM" | "HVC-SOROM" => {
                if !ck(cart.prg_size, &[128, 256])
                    || !ck(cart.chr_size, &[0])
                    || !ck(cart.vram_size, &[8])
                    || !ck(cart.wram_size, &[16])
                {
                    return false;
                }
            }
            _ => return false,
        }

        self.base_configure();
        true
    }

    /// `SuROM.ReadPrg`, which `SXROM` inherits.
    fn read_prg_surom(&self, addr: i32) -> u8 {
        let mut bank = self.mmc1.get_prg_bank(addr);
        let chr_bank = self.mmc1.get_chr_bank_4k(0);
        let bank_bit18 = chr_bank >> 4;
        bank |= bank_bit18 << 4;
        bank &= self.prg_mask;
        self.base.rom[((bank << 14) | (addr & 0x3FFF)) as usize]
    }
}

/// `AssertPrg`/`AssertChr`/`AssertVram`/`AssertWram`, shortened because SxROM.cs has 25 of them.
fn ck(value: i32, valid: &[i32]) -> bool {
    BoardBase::assert_mem(value, valid)
}

impl Board for SxRom {
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
        match self.variant {
            SxVariant::SxRom => "SxROM",
            SxVariant::SuRom => "SuROM",
            SxVariant::SxRom512 => "SXROM",
            SxVariant::SoRom => "SoROM",
        }
    }

    fn configure(&mut self, _origin: DetectionOrigin) -> bool {
        match self.variant {
            SxVariant::SxRom => self.configure_sxrom(),
            SxVariant::SuRom => self.configure_surom(),
            SxVariant::SxRom512 => self.configure_sxrom512(),
            SxVariant::SoRom => self.configure_sorom(),
        }
    }

    fn has_clock_ppu(&self) -> bool {
        true
    }

    fn clock_ppu(&mut self) {
        if self.ppuclock < PPUTIMEOUT {
            self.ppuclock += 1;
        }
    }

    fn write_prg(&mut self, addr: i32, value: u8) {
        // mmc1 ignores subsequent writes that are very close together
        if self.ppuclock >= PPUTIMEOUT {
            self.ppuclock = 0;
            self.mmc1.write(addr, value);
            if !self.disablemirror {
                let m = self.mmc1.mirror;
                self.base.set_mirror_type(m); //often redundant, but gets the job done
            }
        }
    }

    fn read_wram(&self, addr: i32, db: u8) -> u8 {
        let addr = self.map_wram(addr);
        if self.is_snrom {
            if !self.mmc1.wram_disable && self.chr_wram_enable {
                self.base.read_wram(addr, db)
            } else {
                db
            }
        } else {
            self.base.read_wram(addr, db)
        }
    }

    fn write_wram(&mut self, addr: i32, value: u8) {
        let addr = self.map_wram(addr);
        self.base.write_wram(addr, value);
    }

    fn read_prg(&self, addr: i32) -> u8 {
        if matches!(self.variant, SxVariant::SuRom | SxVariant::SxRom512) {
            return self.read_prg_surom(addr);
        }
        let bank = self.mmc1.get_prg_bank(addr) & self.prg_mask;
        let addr = (bank << 14) | (addr & 0x3FFF);
        self.base.rom[addr as usize]
    }

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        if addr < 0x2000 {
            if self.is_snrom {
                // WRAM enable is tied to ppu a12
                self.chr_wram_enable = (self.gen_chr_address(addr) & (1 << 16)) == 0;
            }

            let addr = self.gen_chr_address(addr);

            if self.base.cart.vram_size != 0 {
                return self.base.vram[(addr & self.vram_mask) as usize];
            }
            return self.base.vrom[addr as usize];
        }
        self.base.read_ppu(addr, env)
    }

    fn write_ppu(&mut self, addr: i32, value: u8, env: &mut BoardEnv<'_>) {
        if addr < 0x2000 {
            if self.is_snrom {
                // WRAM enable is tied to ppu a12
                self.chr_wram_enable = (self.gen_chr_address(addr) & (1 << 16)) == 0;
            }

            if self.base.cart.vram_size != 0 {
                let i = (self.gen_chr_address(addr) & self.vram_mask) as usize;
                self.base.vram[i] = value;
            }
        } else {
            self.base.write_ppu(addr, value, env);
        }
    }
}
