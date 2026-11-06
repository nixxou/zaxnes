use super::{Mapper, Vrc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The boards whose counter can pull /IRQ, **sharing one [`Board`] variant** between them
/// (MF-6b step 1).
///
/// [`Board`]: crate::cartridges::Board
///
/// # Why one variant for the whole lot
///
/// §4.2 of `docs/review/MF-6-inventory.md` calls `NesBus::irq_asserted` "the most expensive hook
/// in the inventory and the one a sub-lot is most likely to trip over, because it costs nothing
/// in the board's own code", and prices MF-6b at "five new `irq_asserted` arms that answer *yes*,
/// on a dispatch of thirty-two arms that outlined at the thirty-second". MF-5b step 1 is the
/// measurement behind that sentence: the thirty-second arm pushed the chain past the inliner's
/// budget and LLVM outlined it, at **+169 739 749 instructions over 300 frames on all thirteen
/// ROMs of lot PR-1's set**, twelve of which cannot assert /IRQ at all.
///
/// [`super::SmallBoard`] solved the same problem one dispatch along and its note says exactly why
/// it could not help here: it deliberately does **not** forward `irq_asserted` or `on_cpu_cycle`,
/// because "forwarding them would give each of those dispatches a *distinct body* — an inner
/// match returning `false` is not the same code as `false`", and the twenty-odd boards that
/// answer "no" share one arm only while their bodies stay identical.
///
/// This enum is that sentence read the other way. Every board in it answers **yes**, so none of
/// them could ever share the empty arm; what they can share is **each other's**. One inner match
/// is one body in `Board::irq_asserted`, one in `Board::on_cpu_cycle`, one in
/// `Board::on_ppu_address_bus` and one in `Board::ppu_irq_horizon`, **for the whole lot** instead
/// of one per board — and the price is paid by a cartridge that *is* one of these boards, which
/// is §4.10's own rule.
///
/// # What is forwarded, and what is deliberately not
///
/// [`Mapper::chr_view`] answers `None` for the whole variant rather than forwarding, for the
/// reason `SmallBoard`'s own `chr_view` gives: that dispatch is inlined into
/// `Ppu2C02::run_visible_span` and so into `Ppu2C02::run_dots`, where one more distinct body cost
/// MF-6a step 2 **+3.3 M to +6.9 M over 300 frames on eight of thirteen ROMs** in spill churn
/// alone. `None` is the body the MMC2, the MMC3, the MMC5, the RAMBO-1 and `SmallBoard` already
/// give, so the loop keeps its allocation and these boards take [`Mapper::ppu_peek`] one fetch at
/// a time. Everything a board of this enum actually implements is forwarded below, and nothing
/// else: a method no board here overrides must keep `Mapper`'s default, or it becomes a body in
/// a dispatch for nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrqBoard {
    /// Mapper 73, Konami VRC3.
    Vrc3(Vrc3),
}

/// Expands `$body` once per board with it bound to `$b`, as `Board`'s own `dispatch!` does.
macro_rules! inner {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            IrqBoard::Vrc3($b) => $body,
        }
    };
}

impl Mapper for IrqBoard {
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

    /// **`#[inline(never)]`, and it is the whole performance story of this lot.**
    ///
    /// Inlined, this one body is the thirteenth in the dispatch `Cpu6502::branch` folds
    /// `Board::irq_asserted` into, and it re-lays that function out: `branch` keeps its 418
    /// instructions and its dynamic count rises on **nine of lot PR-1's thirteen ROMs**, +0.013 %
    /// to +0.026 %, on boards with no IRQ at all. Out of line it is a `callq` per /IRQ sample for
    /// a cartridge of this family and **nothing for anyone else** — the same trade MF-6a2 step 2
    /// made with `Rambo1::ppu_irq_horizon`, where taking one body out of the
    /// `Board::ppu_irq_horizon` dispatch took `NesBus::catch_up_ppu` from 216 instructions to 203
    /// and bought −0.005 % to −0.082 % on every board in the library. MF-6b step 1 has the four
    /// shapes and their numbers.
    #[inline(never)]
    fn irq_asserted(&self) -> bool {
        inner!(self, b => b.irq_asserted())
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.write_register(memory, address, value));
    }

    /// `None` for the whole variant; see the type's own note on why this is not forwarded.
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
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

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        inner!(self, b => b.on_cpu_cycle(memory));
    }
}
