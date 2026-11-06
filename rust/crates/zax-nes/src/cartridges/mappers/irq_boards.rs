use super::{IremH3001, Mapper, NesEvent, Sunsoft3, TaitoTc0690, Tengen800037, Vrc3};
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
    /// Mapper 65, Irem H3001.
    IremH3001(IremH3001),
    /// Mapper 67, Sunsoft-3.
    Sunsoft3(Sunsoft3),
    /// Mapper 105, the Nintendo NES-EVENT board.
    NesEvent(NesEvent),
    /// Mapper 48, the Taito TC0690.
    TaitoTc0690(TaitoTc0690),
    /// Mapper 158, Tengen's 800037: the RAMBO-1 with CHR A17 wired to CIRAM A10.
    Tengen800037(Tengen800037),
}

/// Expands `$body` once per board with it bound to `$b`, as `Board`'s own `dispatch!` does.
macro_rules! inner {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            IrqBoard::Vrc3($b) => $body,
            IrqBoard::IremH3001($b) => $body,
            IrqBoard::Sunsoft3($b) => $body,
            IrqBoard::NesEvent($b) => $body,
            IrqBoard::TaitoTc0690($b) => $body,
            IrqBoard::Tengen800037($b) => $body,
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

    /// Forwarded for mapper 105 alone, and it is the **only** one of the three PRG-RAM methods
    /// that is: [`Mapper::prg_ram_writable`]'s default calls this one, and the NES-EVENT's 8 KiB
    /// of unbanked PRG-RAM is exactly what [`Mapper::prg_ram_offset`]'s default answers. One body
    /// in `Board::prg_ram_enabled` instead of three across the $6000-$7FFF decode, which is the
    /// path MF-6b step 2 found `Cartridge::default_cpu_read` on.
    ///
    /// **Inlined, and `#[inline(never)]` here is refuted** (MF-6b step 5, V4). The sixth inner
    /// variant pushed `Board::prg_ram_writable` out of line — 0 to 55 instructions — at
    /// **+430 380** over 600 level-2 frames of Super Mario Bros. 3, and taking *this* body out
    /// of line to shrink it instead put `Board::prg_ram_enabled` behind a call on the MMC5's
    /// read path: `Cartridge::default_cpu_read` went **+67 110 804 instructions over 300 frames
    /// of `mmc5exram`**, +0.504 % on that ROM, where the 430 k it was meant to save is 0.002 %
    /// of a game this board is not. The 55 instructions stay.
    fn prg_ram_enabled(&self) -> bool {
        inner!(self, b => b.prg_ram_enabled())
    }

    /// Forwarded for mapper 158 alone (MF-6b step 5). `Cartridge` caches the four answers and
    /// re-asks them only after a register write, so this dispatch is cold: the field
    /// `Mmc3Board` keeps for the TxSROM predates that cache and is not copied here.
    ///
    /// **`#[inline(never)]`, and the first shape of step 5 measured why.** Inlined, this body
    /// grows `Board::nametable_page`, which grows `Cartridge::refresh_nametable_pages`, which
    /// pushed `Cartridge::cpu_write` past the inliner's budget: that function went from **0 to
    /// 3 776 318 instructions on `m1`** and 5 089 659 on `m4` over 300 frames, against
    /// −2 658 017 and −3 537 021 in `NesBus::write_internal` as its body left — a net cost of
    /// about 1.2 M on the two MMC1 ROMs, on a dispatch no MMC1 reaches. Out of line it is one
    /// `callq` per *register write* of a mapper-158 cartridge, four times, and nothing for
    /// anybody else.
    #[inline(never)]
    fn nametable_page(&self, table: i32) -> i32 {
        inner!(self, b => b.nametable_page(table))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.write_register(memory, address, value));
    }

    /// **`#[inline(never)]`, and §4.3 of `docs/review/MF-6-inventory.md` is why**: this dispatch
    /// runs 57 600-odd times a frame on a board that watches the PPU's address bus, and mappers
    /// 4, 64 and 118 are in it already. Inlined, the `IrqBoard` body would be one more arm in
    /// `Board::on_ppu_address_bus` for all four boards of this variant, three of which never
    /// reach it; out of line it is a `callq` for the one board that does. MF-6b step 4 has the
    /// figures.
    #[inline(never)]
    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        inner!(self, b => b.on_ppu_address_bus(memory, address, clock));
    }

    /// **`None` for the whole variant, which is the body the MMC5 already gives**, and MF-6b
    /// step 4 measured why it is not the forward it looks like it should be.
    ///
    /// `NesBus::ppu_horizon` is inlined into `NesBus::catch_up_ppu`, and *any* forward here is
    /// one more distinct body in the `Board::ppu_irq_horizon` dispatch it folds in — even one
    /// whose four other boards answer `Mapper`'s default, even behind `#[inline(never)]`. The
    /// first shape of this step did exactly that, with the TC0690's own
    /// [`Mapper::ppu_irq_horizon`] out of line: `catch_up_ppu` went from **188 instructions to
    /// 190** and that pair of instructions cost `mmc5` **+9 214 379** and `readbuf` +2 245 658
    /// over 300 frames, taking `mmc5` and `z2` above the lot's reference. It is MF-6a2 step 2's
    /// finding read backwards — that step took the RAMBO-1's body *out* of this dispatch, took
    /// `catch_up_ppu` from 216 to 203 and bought every board in the library 0.005 % to 0.082 %.
    ///
    /// `None` is "no bound is known, so the PPU runs in step with the CPU": always correct, and
    /// already in the dispatch as the MMC5's arm, so it adds no body at all. The price is that a
    /// cartridge of this variant gives up the PPU's lag — the TC0690 could not have kept it
    /// anyway, and the other four could but pay for the family, which is §4.10's own rule and
    /// the same trade [`IrqBoard::chr_view`] below makes one dispatch further on.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        None
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
