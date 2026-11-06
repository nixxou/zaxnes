use super::{IremTamS1, JalecoJf11, Mapper, Sunsoft1, Sunsoft2, TaitoTc0190};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The boards that are a register decode and nothing else, **sharing one [`Board`] variant**
/// between them (MF-6a step 2).
///
/// [`Board`]: crate::cartridges::Board
///
/// # Why one variant and not five
///
/// §4.9 of `docs/review/MF-6-inventory.md` says a new board is appended to `Board` and costs
/// "one jump-table slot to each dispatch". Step 2 measured what five slots cost and it is not
/// nothing: with the five boards as five variants, **`Ppu2C02::run_dots` kept its 1 906
/// instructions and cost +3.3 M to +6.9 M over 300 frames on eight of lot PR-1's thirteen
/// ROMs** — 0.067 % to 0.131 %, the whole of it spill and reload churn inside the loop, because
/// `Board::chr_view`'s arms are inlined into `Ppu2C02::run_visible_span` and that is inlined
/// into `run_dots`. The step's experiment Y (the same five variants with no PPU-side override
/// on any board) reproduced the deltas **to the instruction**, which is what says the cost is
/// the arms and not the boards; and `#[inline(never)]` on `Board::chr_view`, which would have
/// taken the arms out of the loop, is far worse (+0.297 % on the set, +0.849 % on `Duelito`)
/// because that dispatch is reached once per span and some ROMs have very many spans.
///
/// So the five boards go behind **one** discriminant. Every dispatch in `Board` keeps the arm
/// count it had after step 1, and the second match below is paid only by a cartridge that *is*
/// one of these five boards — the same trade as every other hook in §4.10, where a cost lands
/// on the board that needs it instead of on the library.
///
/// # What is forwarded, and what is deliberately not
///
/// Only the twelve methods one of the five actually implements. **The per-cycle and per-fetch
/// methods are left to `Mapper`'s defaults on purpose**: `irq_asserted`, `on_cpu_cycle`,
/// `on_ppu_address_bus`, `nametable_page`, `expansion_audio_output` and the rest. Forwarding
/// them would give each of those dispatches a *distinct body* — an inner match returning
/// `false` is not the same code as `false` — and the twenty-odd boards that answer "no" to
/// `irq_asserted` share one arm precisely because their bodies are identical (MF-6a step 1's
/// finding, and MF-5b step 1's 170 M regression is what a body too many costs there).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmallBoard {
    /// Mapper 33, the Taito TC0190.
    TaitoTc0190(TaitoTc0190),
    /// Mapper 97, the Irem TAM-S1.
    IremTamS1(IremTamS1),
    /// Mapper 140, the Jaleco JF-11 and JF-14.
    JalecoJf11(JalecoJf11),
    /// Mapper 184, Sunsoft-1.
    Sunsoft1(Sunsoft1),
    /// Mappers 89 and 93, Sunsoft-2 on its two boards.
    Sunsoft2(Sunsoft2),
}

/// Expands `$body` once per board with it bound to `$b`, as `Board`'s own `dispatch!` does.
macro_rules! inner {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            SmallBoard::TaitoTc0190($b) => $body,
            SmallBoard::IremTamS1($b) => $body,
            SmallBoard::JalecoJf11($b) => $body,
            SmallBoard::Sunsoft1($b) => $body,
            SmallBoard::Sunsoft2($b) => $body,
        }
    };
}

impl Mapper for SmallBoard {
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

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        inner!(self, b => b.intercept_cpu_write(memory, address, value))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.write_register(memory, address, value));
    }

    /// **`None`, and this is the line step 2's bar turned on.**
    ///
    /// `Board::chr_view`'s arms are inlined into `Ppu2C02::run_visible_span` and so into
    /// `Ppu2C02::run_dots`, and what that loop cannot afford is **one more distinct body**. Any
    /// body of this board's own is one: an inner `match` is one, and so — measured — is
    /// forwarding to five boards whose `banks` field sits at five different offsets, because
    /// `Mapper::chr_view`'s default is monomorphised per board and the "shared" arm is only
    /// shared while the code is identical. Either way `run_dots` kept its 1 906 instructions
    /// and cost **+3.3 M to +6.9 M over 300 frames on eight of lot PR-1's thirteen ROMs**, in
    /// spill churn alone.
    ///
    /// `None` is not a body of this board's own: it is the one MMC2, MMC3, MMC5 and the RAMBO-1
    /// already answer, so the dispatch gains nothing to lay out and the loop keeps its
    /// allocation. The price is that the dozen games of these six mapper numbers take
    /// [`Mapper::ppu_peek`] one fetch at a time instead of the span fast path — a cost on the
    /// boards that caused it and on no other game, which is §4.10's own rule.
    ///
    /// [`Mapper::chr_peek_view`] *is* forwarded below, so the tile emitter and the host
    /// accessors keep a view: that dispatch is not inside `run_dots`, and the MMC3 and the
    /// UNROM 512 already give it bodies of their own.
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
}
