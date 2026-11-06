use super::{
    Bandai74161, Bf9096, CnRomProtect, IremTamS1, JalecoJf11, Mapper, Sunsoft1, Sunsoft2,
    TaitoTc0190, Vrc1,
};
#[cfg(feature = "board-taito-x1")]
use super::{TaitoX1005, TaitoX1017};
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
/// # **This enum is full, and MF-6c measured where its limit is**
///
/// Three measurements of lot MF-6c, and they agree: **one more variant here costs `mmc5`
/// +7 456 756 instructions over 300 frames, +0.066 %**, and nothing else in lot PR-1's thirteen
/// moves at all. `Cartridge::default_cpu_read` has two layouts on that ROM — 283 356 728 and
/// 290 813 484 — and a variant past the tenth selects the dearer one, in which LLVM hoists
/// `leaq 240(%rdi), %rax` above the jump table of the inlined `Board::prg_ram_enabled` so that it
/// runs on every CPU read of the one board that maps PRG-RAM into $8000-$DFFF.
///
/// * **step 2** added the two Taito X1 boards (variants eleven and twelve) with three forwards:
///   +7 456 756. Seven shapes of their own code could not move it, and the two numbers are
///   **parked** (`board-taito-x1`, off by default).
/// * **step 3** re-measured that on a tree where `default_cpu_read` had grown from 377
///   instructions to 425 for unrelated reasons: the same figure, to the instruction. So it is not
///   a size threshold.
/// * **step 4** added the Sunsoft-4 alone (variant eleven) with **one** forward: the same figure
///   again. Moved to [`super::IrqBoard`], which was nine variants deep and in the cheap layout, it
///   cost `mmc5` **−35 125** and the board shipped in `default`.
/// * **step 5** then tried the same move for the two Taito boards, which step 4 expected to work,
///   and it does not: `IrqBoard` at twelve variants put `default_cpu_read` at **298 270 240**,
///   +14 913 512 — **two** units of the same 7 456 756 — and `mmc5` at +0.132 %. The park stands.
///
/// So a board that would have come here has **nowhere free left in either existing enum**.
/// `docs/progress/MF-6c.md` steps 2 to 5 have the eleven measurements.
///
/// **What step 6 then found, and it is the way out**: *one* new `Board` discriminant is free.
/// [`super::DecodeBoard`] is a second enum of exactly this kind beside this one, and the twelve
/// ROMs plus five games that priced it came out at **worst `metroid` +12 780 (+0.000 %), set total
/// −136 318 and `mmc5` −40 814** — where MF-6a3 step 1 had measured **two** new discriminants at
/// `mmc5` +8 994 278 (+0.080 %). So a register-decode board goes to [`super::DecodeBoard`] now,
/// and this enum and [`super::IrqBoard`] stay at the variant counts MF-6c left them with.
/// Redesigning the $6000-$7FFF read path — a load-time cache of its decode on `Cartridge`, the one
/// shape none of this lot's measurements tried — is still the structural fix, and R0 has made it
/// its own sub-lot after MF-6c; it would lift the limit here rather than route around it.
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
    // Appended by MF-6a step 3, which added four more numbers and **no** `Board` variant: the
    // whole point of this enum is that a board joining it is free to the rest of the library.
    /// Mappers 70 and 152, the Bandai 74161/7432 board with and without mirroring control.
    Bandai74161(Bandai74161),
    /// Mapper 75, Konami VRC1.
    Vrc1(Vrc1),
    /// Mapper 232, the Camerica BF9096.
    Bf9096(Bf9096),
    // Appended by MF-6a step 4.
    /// Mapper 185, CNROM with the security diodes. Here and not on [`super::CnRom`] because a
    /// CHR chip select has to reach `chr_view`, and this variant has already paid for that.
    CnRomProtect(CnRomProtect),
    // Appended by MF-6c step 2, which adds four numbers and no `Board` variant. Both boards
    // belong here by this enum's entry condition — "IRQ: None" on both pages, so both share the
    // empty `Board::irq_asserted` arm — and the two methods they need
    // ([`Mapper::cpu_peek_override`] and [`Mapper::nametable_page`]) are forwarded **once for
    // the variant**, which is the whole reason this enum exists.
    /// Mappers 80 and 207, the Taito X1-005 and its single-screen wiring.
    #[cfg(feature = "board-taito-x1")]
    TaitoX1005(TaitoX1005),
    /// Mapper 82 and NES 2.0 mapper 552, the Taito X1-017 with its two bank orders.
    #[cfg(feature = "board-taito-x1")]
    TaitoX1017(TaitoX1017),
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
            SmallBoard::Bandai74161($b) => $body,
            SmallBoard::Vrc1($b) => $body,
            SmallBoard::Bf9096($b) => $body,
            SmallBoard::CnRomProtect($b) => $body,
            #[cfg(feature = "board-taito-x1")]
            SmallBoard::TaitoX1005($b) => $body,
            #[cfg(feature = "board-taito-x1")]
            SmallBoard::TaitoX1017($b) => $body,
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

    /// Forwarded for the two Taito X1 boards alone (MF-6c step 2), **and this is the hook their
    /// mapper-internal WRAM reads through rather than [`Mapper::cpu_peek_override`].** Step 2
    /// measured the difference and it is the whole performance story of that step:
    ///
    /// * with the decode in `cpu_peek_override`, `Board::cpu_read_override` — whose default
    ///   forwards to it — gained a distinct body, and `Cartridge::default_cpu_read` went
    ///   **283 356 728 → 290 813 484 on `mmc5`, +7 456 756 over 300 frames, +0.066 %**, with no
    ///   other ROM of PR-1's thirteen moving by more than ±23 000. The post-LTO dump names the
    ///   mechanism exactly: that function grew 377 → 391 instructions and LLVM hoisted one
    ///   `leaq 240(%rdi), %rax` **above** the jump table of the inlined `Board::prg_ram_enabled`,
    ///   so it is executed on every call — one instruction per CPU read of the one board that
    ///   maps PRG-RAM into $8000-$DFFF and therefore reaches `default_cpu_read` on all of them.
    ///   It is MF-6b steps 2-5's figure to the instruction, and MF-6a3 step 4's in reverse: a
    ///   two-state oscillation of one function on one ROM.
    /// * with the decode **here**, the same two boards and the same four numbers cost that
    ///   function nothing — `#[inline(never)]` on this forward bounds the arm
    ///   `Board::try_read_prg`'s `#[inline(always)]` folds into `default_cpu_read` to one
    ///   `callq`, whatever this enum grows to later.
    ///
    /// Two other shapes were tried and rejected, both in `docs/progress/MF-6c.md` step 2:
    /// `#[inline(never)]` on the `cpu_peek_override` forward (**+0.111 %** on the set, `mmc5`
    /// +78 M as `Cartridge::try_cpu_read` lost the inlined chain), and `#[inline(always)]` on
    /// `Cartridge::default_cpu_read` (which cured `mmc5` at −0.065 % and pushed
    /// `Board::prg_ram_enabled`, then `Cartridge::prg_ram_enabled`, out of line instead —
    /// +0.021 % to +0.024 % on `m0` and `m1`, two ROMs whose board the step does not touch).
    ///
    /// **Behaviour-neutral for the nine boards MF-6a put here**: none of them overrides it, so
    /// each one's own default — `self.banks().and_then(BankWindows::read_prg)` — is the answer the
    /// un-forwarded dispatch gave it.
    #[cfg(feature = "board-taito-x1")]
    #[inline(never)]
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        inner!(self, b => b.try_read_prg(memory, address))
    }

    /// Forwarded for the two Taito X1 boards alone (MF-6c step 2), which answer **`false`**: the
    /// generic $6000-$7FFF path of `Cartridge::default_cpu_read` must not take that window,
    /// because only part of it is decoded and only while a permission latch holds the chip's
    /// magic value. With it false, the read falls through to [`SmallBoard::try_read_prg`] above,
    /// which is where those boards answer, and `Mapper::prg_ram_writable`'s default makes the
    /// generic *write* path stand aside for the same reason (the board's own
    /// `intercept_cpu_write` has already consumed it).
    ///
    /// **Behaviour-neutral for the nine boards MF-6a put here**: every one of them takes
    /// `Mapper`'s default `true`, which is what the un-forwarded dispatch answered for the whole
    /// variant. Adding a body to `Board::prg_ram_enabled` was measured free in MF-6c step 1,
    /// which gave that dispatch mapper 153's `is_153 && prg_ram_on` and left `mmc5` at −46 670.
    #[cfg(feature = "board-taito-x1")]
    fn prg_ram_enabled(&self) -> bool {
        inner!(self, b => b.prg_ram_enabled())
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

    /// Forwarded for mapper 207 alone (MF-6c step 2), the X1-005 wiring whose CHR A17 drives
    /// CIRAM A10 — "the exact same way that TLSROM (118) differs from MMC3 (4)". Behind
    /// `Cartridge::nametable_pages_vary`, so no other board asks it.
    ///
    /// **`#[inline(never)]`, for the reason `IrqBoard::nametable_page` carries the same attribute**
    /// (MF-6b step 5): inlined, this body grows `Board::nametable_page`, which grows
    /// `Cartridge::refresh_nametable_pages`, which pushed `Cartridge::cpu_write` past the
    /// inliner's budget and cost two MMC1 ROMs about 1.2 M instructions over 300 frames on a
    /// dispatch no MMC1 reaches. Out of line it is one `callq` per register write of a mapper-207
    /// cartridge, four times, and nothing for anybody else.
    #[cfg(feature = "board-taito-x1")]
    #[inline(never)]
    fn nametable_page(&self, table: i32) -> i32 {
        inner!(self, b => b.nametable_page(table))
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
