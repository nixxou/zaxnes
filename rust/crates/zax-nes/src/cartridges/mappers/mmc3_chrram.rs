use super::{Mapper, Mmc3ChrRam192, Mmc3ChrRam194, Mmc3ChrRam74, WaixingFs303};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The four numbers of `board-mmc3-chrram` — **74, 192, 194 and 195** — behind one
/// [`super::IrqBoard`] variant (MF-UNL-2 step 2).
///
/// # Why an inner enum and not four variants
///
/// Each number is its own `const` monomorphisation of [`super::Mmc3Board`] — that is what the
/// `CHR_RAM_*` parameters are for, and `Mmc3Board`'s own note has the six shapes MF-6a2 step 3
/// measured for exactly this choice — so the four cannot share one Rust type. They *can* share
/// one `IrqBoard` arm, which is what this enum is: one inner match is **one body** in
/// `IrqBoard::ppu_peek`, `IrqBoard::write_register` and the rest, for the whole feature, instead
/// of four. That is [`super::IrqBoard`]'s own argument read one dispatch further down, and it is
/// what keeps the price of this feature on the four boards that need it.
///
/// Boxed in the variant that holds it, for the reason `IrqBoard::Somari` is boxed: MF-6b step 1
/// measured **one byte** of displacement in that enum's payload at +0.009 % to +0.027 % on
/// eleven of lot PR-1's thirteen ROMs, and this enum is an `Mmc3Board` plus a discriminant.
/// The price is one dereference per forwarded call for a cartridge of this family.
///
/// # What is forwarded
///
/// Everything [`super::Mmc3Board`] implements and nothing else, which for this family is the
/// whole of the MMC3 — the bank registers, the A12 counter, `$A001`'s PRG-RAM protection, the
/// two `#[inline(never)]` CHR bodies — plus mapper 195's GAL register. `ppu_irq_horizon` and
/// `chr_view` are **not** here: [`super::IrqBoard`] answers both for its whole variant, which is
/// the trade its note describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mmc3ChrRamBoard {
    /// Mapper 74, the Waixing 43-393/43-406/860908C: 2 KiB of CHR-RAM at banks `$08-$09`.
    Mapper074(Mmc3ChrRam74),
    /// Mapper 192, the Waixing FS308: 4 KiB at banks `$08-$0B`.
    Mapper192(Mmc3ChrRam192),
    /// Mapper 194: 2 KiB at banks `$00-$01`.
    Mapper194(Mmc3ChrRam194),
    /// Mapper 195, the Waixing FS303: 8 KiB wired, the window chosen at run time by the GAL.
    WaixingFs303(WaixingFs303),
}

impl Mmc3ChrRamBoard {
    /// The board of `mapper`, or `None` when the number is not one of this family's four.
    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Option<Self> {
        Some(match mapper {
            74 => Self::Mapper074(Mmc3ChrRam74::new(memory, mapper)),
            192 => Self::Mapper192(Mmc3ChrRam192::new(memory, mapper)),
            194 => Self::Mapper194(Mmc3ChrRam194::new(memory, mapper)),
            195 => Self::WaixingFs303(WaixingFs303::new(memory, mapper)),
            _ => return None,
        })
    }

    /// The CHR-RAM chip each number carries, in bytes: the `Size of RAM` column of nesdev's
    /// *MMC3 with CHR ROM and CHR RAM*, which `registry::board_memory_sizes` settles at load
    /// because no header field can state it.
    ///
    /// Mapper 195 is the one that is not its own window's width — see [`WaixingFs303`].
    pub(crate) const fn chr_ram_size(mapper: i32) -> Option<usize> {
        Some(match mapper {
            74 | 194 => Mmc3ChrRam74::CHR_RAM_SIZE,
            192 => Mmc3ChrRam192::CHR_RAM_SIZE,
            195 => WaixingFs303::CHR_RAM_SIZE,
            _ => return None,
        })
    }
}

/// Expands `$body` once per number with it bound to `$b`, as `IrqBoard`'s own `inner!` does.
macro_rules! inner {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            Mmc3ChrRamBoard::Mapper074($b) => $body,
            Mmc3ChrRamBoard::Mapper192($b) => $body,
            Mmc3ChrRamBoard::Mapper194($b) => $body,
            Mmc3ChrRamBoard::WaixingFs303($b) => $body,
        }
    };
}

impl Mapper for Mmc3ChrRamBoard {
    fn mapper_number(&self) -> i32 {
        inner!(self, b => b.mapper_number())
    }

    fn banks(&self) -> Option<&BankWindows> {
        inner!(self, b => b.banks())
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        inner!(self, b => b.banks_mut())
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        inner!(self, b => b.save_registers(writer));
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        inner!(self, b => b.load_registers(reader))
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        inner!(self, b => b.reset(memory));
    }

    fn irq_asserted(&self) -> bool {
        inner!(self, b => b.irq_asserted())
    }

    fn prg_ram_enabled(&self) -> bool {
        inner!(self, b => b.prg_ram_enabled())
    }

    fn prg_ram_writable(&self) -> bool {
        inner!(self, b => b.prg_ram_writable())
    }

    fn nametable_page(&self, table: i32) -> i32 {
        inner!(self, b => b.nametable_page(table))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.write_register(memory, address, value));
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        inner!(self, b => b.chr_peek_view(memory))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        inner!(self, b => b.ppu_peek(memory, address, fetch))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.ppu_write(memory, address, value));
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        inner!(self, b => b.on_ppu_address_bus(memory, address, clock));
    }
}
