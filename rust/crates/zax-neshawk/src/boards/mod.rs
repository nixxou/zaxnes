//! The cartridge boards.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/Boards/NesBoardBase.cs` and
//! `Boards/INesBoard.cs`; each board module names its own C# file.
//! MIT; see `LICENSE-BizHawk`.
//!
//! # How the back-reference to `NES` is kept
//!
//! A C# board reaches the machine through `NES` (`NES.CIRAM`, `NES.DB`). Rust cannot hand a board
//! a reference to the struct that owns it, so the two fields boards actually touch travel in
//! [`BoardEnv`], built from disjoint fields of `Nes` at each call. Nothing else about the shape
//! changes: every method is the C# method, in the C#'s order.
//!
//! # Scope
//!
//! Not every NesHawk board is here. The owner's decision of 2026-11-12 is to port faithfully the
//! boards covering about 80-95 % of the commercial library and to fall back on Zaxnes's own
//! boards for the rest, marked as not NesHawk-exact. [`board_kind_for`] is the registry, and
//! [`find_board`] is `NES.BoardSystem.cs`'s `FindBoard`.

use crate::cart::{CartInfo, DetectionOrigin};

mod axrom;
mod cnrom;
mod mmc3;
mod nrom;
mod sxrom;
mod txrom;
mod uxrom;

pub use mmc3::Mmc3;

/// `NesBoardBase.cs`'s `EMirrorType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorType {
    Vertical,
    Horizontal,
    OneScreenA,
    OneScreenB,
}

/// The two pieces of `NES` a board touches on a read or a write.
pub struct BoardEnv<'a> {
    /// `NES.CIRAM`, the 2 KiB of nametable RAM on the console side.
    pub ciram: &'a mut [u8; 0x800],
    /// `NES.DB`, the data bus's last value, which an unmapped read returns.
    pub db: u8,
}

/// The state `NesBoardBase` holds for every board.
#[derive(Debug, Clone, Default)]
pub struct BoardBase {
    /// `NesBoardBase.Cart`, which in the C# is `NES.cart` by reference. Boards both read it and
    /// write it from `Configure`; the machine copies it back afterwards so that the two agree.
    pub cart: CartInfo,

    pub rom: Vec<u8>,
    pub vrom: Vec<u8>,
    pub wram: Vec<u8>,
    pub vram: Vec<u8>,

    mirroring: [i32; 4],
    wram_mask: i32,

    /// `NesBoardBase.IrqSignal`.
    pub irq_signal: bool,
}

impl BoardBase {
    /// `SetMirroring`.
    pub fn set_mirroring(&mut self, a: i32, b: i32, c: i32, d: i32) {
        self.mirroring[0] = a;
        self.mirroring[1] = b;
        self.mirroring[2] = c;
        self.mirroring[3] = d;
    }

    /// `CalculateMirrorType`.
    #[must_use]
    pub fn calculate_mirror_type(pad_h: i32, pad_v: i32) -> MirrorType {
        if pad_h == 0 {
            return if pad_v == 0 {
                MirrorType::OneScreenA
            } else {
                MirrorType::Horizontal
            };
        }
        if pad_v == 0 {
            return MirrorType::Vertical;
        }
        MirrorType::OneScreenB
    }

    /// `SetMirrorType(int, int)`.
    pub fn set_mirror_type_pads(&mut self, pad_h: i32, pad_v: i32) {
        self.set_mirror_type(Self::calculate_mirror_type(pad_h, pad_v));
    }

    /// `SetMirrorType(EMirrorType)`.
    pub fn set_mirror_type(&mut self, mirror_type: MirrorType) {
        match mirror_type {
            MirrorType::Horizontal => self.set_mirroring(0, 0, 1, 1),
            MirrorType::Vertical => self.set_mirroring(0, 1, 0, 1),
            MirrorType::OneScreenA => self.set_mirroring(0, 0, 0, 0),
            MirrorType::OneScreenB => self.set_mirroring(1, 1, 1, 1),
        }
    }

    /// `ApplyMirroring`.
    #[must_use]
    pub fn apply_mirroring(&self, addr: i32) -> i32 {
        let block = (addr >> 10) & 3;
        let block = self.mirroring[block as usize];
        let ofs = addr & 0x3FF;
        (block << 10) | ofs
    }

    /// `PostConfigure`: `_wramMask = (Cart.WramSize * 1024) - 1`.
    pub fn post_configure(&mut self) {
        self.wram_mask = (self.cart.wram_size * 1024) - 1;
    }

    /// `NesBoardBase.ReadWram`: `_wram?[addr & _wramMask] ?? NES.DB`.
    #[must_use]
    pub fn read_wram(&self, addr: i32, db: u8) -> u8 {
        if self.wram.is_empty() {
            return db;
        }
        self.wram
            .get((addr & self.wram_mask) as usize)
            .copied()
            .unwrap_or(db)
    }

    /// `NesBoardBase.WriteWram`.
    pub fn write_wram(&mut self, addr: i32, value: u8) {
        if self.wram.is_empty() {
            return;
        }
        if let Some(slot) = self.wram.get_mut((addr & self.wram_mask) as usize) {
            *slot = value;
        }
    }

    /// `NesBoardBase.ReadPpu`.
    #[must_use]
    pub fn read_ppu(&self, addr: i32, env: &BoardEnv<'_>) -> u8 {
        if addr < 0x2000 {
            if !self.vrom.is_empty() {
                return self.vrom[addr as usize];
            }
            return self.vram.get(addr as usize).copied().unwrap_or(0);
        }
        env.ciram[self.apply_mirroring(addr) as usize]
    }

    /// `NesBoardBase.WritePpu`.
    pub fn write_ppu(&mut self, addr: i32, value: u8, env: &mut BoardEnv<'_>) {
        if addr < 0x2000 {
            if !self.vram.is_empty() {
                self.vram[addr as usize] = value;
            }
        } else {
            env.ciram[self.apply_mirroring(addr) as usize] = value;
        }
    }

    /// `AssertMemType`, reduced to a boolean: a board whose assert fails does not configure.
    /// The C# throws out of `FindBoard`, which aborts the load; refusing the board instead lets a
    /// later one try, and the load fails with "no class implements the board type" if none does.
    #[must_use]
    pub fn assert_mem(value: i32, valid: &[i32]) -> bool {
        valid.contains(&value)
    }
}

/// Every board hook `INesBoard` declares that the ported machine calls.
///
/// The read hooks take `&self` where every board in scope leaves no trace on a read, which is
/// what lets `peek_cart` share their bodies. A board that needs a mutable read (MMC5's
/// `ReadReg2xxx` does) will have to widen this.
pub trait Board {
    fn base(&self) -> &BoardBase;
    fn base_mut(&mut self) -> &mut BoardBase;

    /// The C# class name, for the load report and the harness.
    fn name(&self) -> &'static str;

    /// `Configure(origin)`. Reads and writes `self.base_mut().cart`.
    fn configure(&mut self, origin: DetectionOrigin) -> bool;

    /// `PostConfigure`.
    fn post_configure(&mut self) {
        self.base_mut().post_configure();
    }

    /// `HasClockPPU`: whether the class overrides `ClockPpu`.
    fn has_clock_ppu(&self) -> bool {
        false
    }

    fn clock_ppu(&mut self) {}
    fn clock_cpu(&mut self) {}
    fn at_vsync_nmi(&mut self) {}
    fn nes_soft_reset(&mut self) {}

    fn read_prg(&self, addr: i32) -> u8 {
        self.base().rom[addr as usize]
    }
    fn write_prg(&mut self, _addr: i32, _value: u8) {}

    fn read_wram(&self, addr: i32, db: u8) -> u8 {
        self.base().read_wram(addr, db)
    }
    fn write_wram(&mut self, addr: i32, value: u8) {
        self.base_mut().write_wram(addr, value);
    }

    fn read_exp(&self, _addr: i32, db: u8) -> u8 {
        db
    }
    fn write_exp(&mut self, _addr: i32, _value: u8) {}

    fn read_ppu(&mut self, addr: i32, env: &mut BoardEnv<'_>) -> u8 {
        self.base().read_ppu(addr, env)
    }
    fn write_ppu(&mut self, addr: i32, value: u8, env: &mut BoardEnv<'_>) {
        self.base_mut().write_ppu(addr, value, env);
    }
    fn address_ppu(&mut self, _addr: i32) {}

    /// `NesBoardBase.PeekCart`, which the debugger and the trace use.
    fn peek_cart(&self, addr: i32, db: u8) -> u8 {
        if addr >= 0x8000 {
            self.read_prg(addr - 0x8000)
        } else if addr < 0x6000 {
            self.read_exp(addr - 0x4000, db)
        } else {
            self.read_wram(addr - 0x6000, db)
        }
    }

    /// `IrqSignal`, which `RunCpuOne` merges into the CPU's IRQ pin every cycle.
    fn irq_signal(&self) -> bool {
        self.base().irq_signal
    }

    /// `SaveRam`: the battery-backed WRAM, or nothing.
    fn save_ram(&self) -> Option<&[u8]> {
        let b = self.base();
        if b.cart.wram_battery && !b.wram.is_empty() {
            Some(&b.wram)
        } else {
            None
        }
    }

    fn save_ram_mut(&mut self) -> Option<&mut [u8]> {
        let b = self.base_mut();
        if b.cart.wram_battery && !b.wram.is_empty() {
            Some(&mut b.wram)
        } else {
            None
        }
    }
}

/// Which of the ported board classes a cartridge gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardKind {
    Nrom,
    UxRom,
    CnRom,
    AxRom,
    SxRom,
    SuRom,
    SxRom512,
    SoRom,
    TxRom,
}

/// The order `FindBoard` walks, which is BizHawk's reflection order: the classes carrying
/// `[NesBoardImplPriority]` first, then the rest. Within each group the order only matters where
/// two boards would both accept a board type, and the comments say where that is.
const BOARD_ORDER: &[BoardKind] = &[
    // --- [NesBoardImplPriority] ---
    BoardKind::Nrom,
    BoardKind::UxRom,
    BoardKind::CnRom,
    BoardKind::AxRom,
    // SxROM must come before SuROM/SXROM/SoROM: it takes `MAPPER001` with PRG <= 256 and refuses
    // the rest, and SuROM takes `MAPPER001` with PRG > 256. Both carry the priority attribute via
    // their base class in the C#; only SxROM has it directly, and SuROM/SXROM/SoROM are normal
    // priority, so this order is the C#'s.
    BoardKind::SxRom,
    BoardKind::TxRom,
    // --- normal priority ---
    BoardKind::SuRom,
    BoardKind::SxRom512,
    BoardKind::SoRom,
];

/// Make an empty instance of a board class.
#[must_use]
pub fn create_board(kind: BoardKind) -> Box<dyn Board> {
    match kind {
        BoardKind::Nrom => Box::new(nrom::Nrom::default()),
        BoardKind::UxRom => Box::new(uxrom::UxRom::default()),
        BoardKind::CnRom => Box::new(cnrom::CnRom::new()),
        BoardKind::AxRom => Box::new(axrom::AxRom::default()),
        BoardKind::SxRom => Box::new(sxrom::SxRom::new(sxrom::SxVariant::SxRom)),
        BoardKind::SuRom => Box::new(sxrom::SxRom::new(sxrom::SxVariant::SuRom)),
        BoardKind::SxRom512 => Box::new(sxrom::SxRom::new(sxrom::SxVariant::SxRom512)),
        BoardKind::SoRom => Box::new(sxrom::SxRom::new(sxrom::SxVariant::SoRom)),
        BoardKind::TxRom => Box::new(txrom::TxRom::default()),
    }
}

/// `NES.BoardSystem.cs`'s `FindBoard`: the first class whose `Configure` accepts the cart.
///
/// As in the C#, `Configure` may **change** the cart (`Cart.VramSize = 8`, `Cart.WramSize = 0`),
/// and those changes stay: the C# hands every candidate the same `CartInfo` object. The accepted
/// board's own `base.cart` is therefore the cart to carry forward, which is what this returns.
#[must_use]
pub fn find_board(
    cart: &CartInfo,
    origin: DetectionOrigin,
) -> Option<(BoardKind, Box<dyn Board>, CartInfo)> {
    let mut running = cart.clone();
    for &kind in BOARD_ORDER {
        let mut board = create_board(kind);
        board.base_mut().cart = running.clone();
        let accepted = board.configure(origin);
        running = board.base().cart.clone();
        if accepted {
            return Some((kind, board, running));
        }
    }
    None
}
