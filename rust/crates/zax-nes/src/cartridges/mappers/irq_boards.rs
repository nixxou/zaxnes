#[cfg(feature = "board-cony")]
use super::Cony;
#[cfg(feature = "board-huang1")]
use super::Huang1;
#[cfg(feature = "board-jy-asic")]
use super::JyCompany;
#[cfg(feature = "board-91")]
use super::Mapper091;
#[cfg(feature = "board-219")]
use super::Mmc3A9461;
#[cfg(feature = "board-mmc3-chrram")]
use super::Mmc3ChrRamBoard;
#[cfg(feature = "board-208")]
use super::Mmc3Gouder208;
#[cfg(feature = "board-263")]
use super::Mmc3Kof97;
#[cfg(feature = "board-mmc3-multicart")]
use super::Mmc3Multicart;
#[cfg(feature = "board-268")]
use super::Mmc3Outer;
#[cfg(feature = "board-mmc3-scramble")]
use super::Mmc3Scramble;
#[cfg(feature = "board-189")]
use super::Mmc3Txc189;
#[cfg(feature = "board-215")]
use super::Mmc3Unl8237;
#[cfg(feature = "board-105")]
use super::NesEvent;
#[cfg(feature = "board-116")]
use super::Somari;
#[cfg(feature = "board-68")]
use super::Sunsoft4;
#[cfg(feature = "board-158")]
use super::Tengen800037;
#[cfg(feature = "board-119")]
use super::Tqrom;
use super::{IremH3001, Mapper, Sunsoft3, TaitoTc0690, Vrc3};
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
///
/// # Why the TQROM and the two MMC3 multicarts are in here, and not variants of their own
///
/// They belong by their own description — all three *are* the MMC3's board, counter included, so
/// all three answer **yes** to [`Mapper::irq_asserted`], which is this enum's entry condition.
/// What put them here rather than beside [`super::Mmc3`] is a measurement, and it is the first of
/// this project to price a `Board` **variant** rather than a body.
///
/// MF-6a3 step 1 built them as two new `Board` discriminants, 35 → 37. Every pinned symbol held
/// — `NesBus::catch_up_ppu` **188 → 188**, the three claimants of `Cpu6502::step` 0 / 0 / 0,
/// `Mmc3Board::update_banks` byte-identical — and `mmc5` came out **+8 994 278 instructions over
/// 300 frames, +0.080 %**, four times the owner's tolerance, with the whole of it in one symbol:
/// `NesBus::push_expansion_audio`, **+8 934 160**. That function is **31 instructions in both
/// dumps** and `Board::expansion_audio_output` is inlined into it whole, as a 35-entry jump
/// table over the discriminants. Two more entries, and LLVM moved the fall-through into the
/// shared tail off the MMC5's arm: `movl 10776(%rdi), %eax` used to fall through and now ends
/// `jmp`. **One taken `jmp` per expansion-audio sample, 8.9 million of them.**
///
/// It is MF-6a2 step 1's residual — "the MMC3 and the RAMBO-1 swapped the fall-through … one
/// taken `jmp` per evaluation" — one dispatch along, and MF-6b step 1 measured the same figure
/// in the other direction: its thirty-fifth variant *bought* `push_expansion_audio`
/// **−8 934 160**, the same number to the instruction. So the two findings close on each other,
/// and the rule they give is narrower than §4.9's "a `Board` variant is free": a variant is free
/// in every dispatch **except** one whose jump table a hot function inlines whole, and
/// `push_expansion_audio` is that function. Adding the three numbers here leaves `Board` at
/// thirty-five discriminants and that jump table byte-identical.
///
/// The price the three boards pay for it is [`IrqBoard::ppu_irq_horizon`]'s `None`: an MMC3
/// could bound the PPU's lag and these three give it up, so their games run the PPU in step with
/// the CPU. That is exactly the trade MF-6b step 4 charged the TC0690, for the same reason — any
/// forward there is one more body in a dispatch `NesBus::catch_up_ppu` inlines, worth +9 214 379
/// on `mmc5` for two instructions — and it is §4.10's rule: the cost lands on the board that
/// needs the variant, not on the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrqBoard {
    /// Mapper 73, Konami VRC3.
    Vrc3(Vrc3),
    /// Mapper 65, Irem H3001.
    IremH3001(IremH3001),
    /// Mapper 67, Sunsoft-3.
    Sunsoft3(Sunsoft3),
    /// Mapper 105, the Nintendo NES-EVENT board.
    #[cfg(feature = "board-105")]
    NesEvent(NesEvent),
    /// Mapper 48, the Taito TC0690.
    TaitoTc0690(TaitoTc0690),
    /// Mapper 158, Tengen's 800037: the RAMBO-1 with CHR A17 wired to CIRAM A10.
    #[cfg(feature = "board-158")]
    Tengen800037(Tengen800037),
    /// Mapper 119, the Nintendo TQROM (MF-6a3 step 1).
    #[cfg(feature = "board-119")]
    Tqrom(Tqrom),
    /// Mappers 47 and 37, Nintendo's two MMC3 multicarts (MF-6a3 step 1).
    #[cfg(feature = "board-mmc3-multicart")]
    Mmc3Multicart(Mmc3Multicart),
    /// Mappers 74, 192, 194 and 195, the other four members of mapper 119's own family
    /// (MF-UNL-2 step 2) — **one variant for the four**, each of them a `const`
    /// monomorphisation of [`super::Mmc3Board`]. [`Mmc3ChrRamBoard`]'s own note has both halves
    /// of that: why four monomorphisations cannot be one Rust type, and why they can still be
    /// one arm here.
    ///
    /// **Boxed**, on the same measurement [`IrqBoard::Somari`] is boxed on: inline, an
    /// `Mmc3Board` plus a discriminant is the variant that would set this enum's size, and
    /// MF-6b step 1 priced one byte of displacement in that payload at +0.009 % to +0.027 % on
    /// eleven of lot PR-1's thirteen ROMs. Boxed, the enum is the size MF-6a3 step 1 measured.
    #[cfg(feature = "board-mmc3-chrram")]
    Mmc3ChrRam(Box<Mmc3ChrRamBoard>),
    /// Mappers 114, 115, 121, 123, 182 and 187, the six MMC3 clones with scrambled registers,
    /// an outer CHR bank and an NROM-like PRG override (MF-UNL-2 step 3).
    ///
    /// **Boxed**, on [`IrqBoard::Somari`]'s own measurement: the board *holds* an `Mmc3` and
    /// adds its own registers, so inline it would set this enum's size.
    #[cfg(feature = "board-mmc3-scramble")]
    Mmc3Scramble(Box<Mmc3Scramble>),
    /// NES 2.0 mapper 263, the Rex Soft UNL-KOF97 board: an MMC3 with its register address and
    /// its written data permuted (MF-UNL-2 step 4).
    ///
    /// **Not boxed**: the board is an `Mmc3` and one more word of nothing, so it is the size of
    /// the `Tqrom` variant this enum already has and sets no new high-water mark.
    #[cfg(feature = "board-263")]
    Mmc3Kof97(Mmc3Kof97),
    /// Mapper 189, the TXC 01-22017/01-22018 and the Yoko FC-001EM: an MMC3 whose PRG-ROM is
    /// banked in 32 KiB amounts from a register at $4100 (MF-UNL-2 step 5).
    ///
    /// **Not boxed**, for `IrqBoard::Mmc3Kof97`'s reason: the board is an `Mmc3` plus one byte,
    /// which lands in the padding that struct already has.
    #[cfg(feature = "board-189")]
    Mmc3Txc189(Mmc3Txc189),
    /// NES 2.0 mapper 268, the AA6023 ASIC of the COOLBOY and MINDKIDS boards: an MMC3 with six
    /// outer bank registers above the chip's own, up to 32 MiB of PRG-ROM (MF-UNL-6).
    ///
    /// **Boxed**, on [`IrqBoard::Mmc3Scramble`]'s own reason: the board *holds* an `Mmc3` and adds
    /// its own register file, so inline it would set this enum's size.
    #[cfg(feature = "board-268")]
    Mmc3Outer(Box<Mmc3Outer>),
    /// iNES mapper 215, the Sugar Softec **UNL-8237** and **UNL-8237A** (MF-UNL-11 step 1): an
    /// MMC3 clone whose register addresses and bank indices are permuted by one of **eight
    /// run-time selectable patterns**, with outer PRG and CHR banks and an NROM-like PRG
    /// override.
    ///
    /// **Boxed**, on [`IrqBoard::Mmc3Scramble`]'s own reason: the board *holds* an `Mmc3` and adds
    /// its own three registers, so inline it would set this enum's size. Every method it needs is
    /// one this variant already forwards for an earlier MMC3 clone, so `Board`'s own dispatches
    /// grow nothing.
    #[cfg(feature = "board-215")]
    Mmc3Unl8237(Box<Mmc3Unl8237>),
    /// iNES mapper 208, the unnamed Gouder board of the Street Fighter IV backport (MF-UNL-11
    /// step 2): an MMC3 clone banking PRG-ROM 32 KiB at a time from a register of its own, with a
    /// **256-byte protection lookup table** behind four registers at `$5800-$5FFF`.
    ///
    /// **Boxed**, on [`IrqBoard::Mmc3Scramble`]'s own reason: the board *holds* an `Mmc3` and adds
    /// its own register file, so inline it would set this enum's size. `try_read_prg` -- the
    /// protection read -- is a method this variant already forwards for mappers 121 and 187.
    #[cfg(feature = "board-208")]
    Mmc3Gouder208(Box<Mmc3Gouder208>),
    /// iNES mapper 219, the Kasheng **A9461** (MF-UNL-11 step 3): an MMC3 clone with a second
    /// **extended** banking mode -- four independent 8 KiB PRG windows and eight 1 KiB CHR
    /// windows reached through a wider register index -- above a 128 KiB outer bank of its own.
    ///
    /// **Boxed**, on [`IrqBoard::Mmc3Scramble`]'s own reason: the board *holds* an `Mmc3` and adds
    /// sixteen bytes of extended bank registers, so inline it would set this enum's size.
    #[cfg(feature = "board-219")]
    Mmc3A9461(Box<Mmc3A9461>),
    /// Mapper 116, the Gouder SOMARI-P and its Huang ASIC: three chips in one board, of which
    /// only the MMC3 has an IRQ (MF-6a3 step 4).
    ///
    /// **Boxed, and for two reasons that are the same reason.** A [`Somari`] holds an MMC3, an
    /// MMC1 and a VRC2 — about 550 bytes against this enum's 200 — so inline it would be the
    /// variant that sets the enum's size, and MF-6b step 1 measured what moving an IRQ board's
    /// fields costs: **one byte** of displacement on `VrcIrq::pending`, which
    /// `Board::irq_asserted` reads for the VRC3 arm of this very enum, re-laid out
    /// `Cpu6502::branch` and cost eleven of lot PR-1's thirteen ROMs +0.009 % to +0.027 % with
    /// no code added at all. Boxed, this enum is the size MF-6a3 step 1 measured and the six
    /// boards MF-6b put here do not move an instruction. It is also what keeps
    /// `clippy::large_enum_variant` satisfied without a new `allow`.
    ///
    /// The price is one pointer dereference per forwarded call **for mapper 116 and no other
    /// board**, on a board whose `Board` value is built once at load — §4.10's rule again.
    #[cfg(feature = "board-116")]
    Somari(Box<Somari>),
    /// Mapper 68, the Sunsoft-4 (MF-6c step 4) — **the one board here whose page says "IRQ:
    /// None"**, and the reason is a measurement, not a reclassification.
    ///
    /// By [`super::SmallBoard`]'s entry condition this board belongs there, and step 4 built it
    /// there first. The eleventh variant of that enum cost `mmc5` **+7 456 756 instructions over
    /// 300 frames, +0.066 %** — `Cartridge::default_cpu_read` flipping to the dearer of its two
    /// layouts, the same figure and the same function `board-taito-x1` was parked on, from **one**
    /// new variant and one forwarded method. `IrqBoard` was already nine variants deep and the
    /// same function was in its cheap layout, so the board was moved here and the figure went
    /// away; MF-6c step 4's write-up has both measurements side by side.
    ///
    /// **And the room that made it possible is now used up.** MF-6c step 5 tried the same move
    /// for the two Taito X1 boards `board-taito-x1` is parked on, which step 4 expected to work:
    /// at **twelve** variants this enum puts `Cartridge::default_cpu_read` at 298 270 240 on
    /// `mmc5`, +14 913 512 — **two** units of the same 7 456 756 — and that ROM at +0.132 %. So
    /// the figure is quantised in units of one layout step, both enums have a limit, and the Taito
    /// boards stay parked. See [`super::SmallBoard`]'s own note for all eleven measurements — and,
    /// at the end of it, what MF-6c step 6 found instead: **one new `Board` discriminant is free**,
    /// so the next register-decode board goes to [`super::DecodeBoard`] and this enum stays at ten.
    ///
    /// The price is this variant's [`IrqBoard::ppu_irq_horizon`] `None` — the Sunsoft-4 could have
    /// let the PPU run behind the CPU and gives that up — which is §4.10's rule and exactly the
    /// trade the TQROM and the two MMC3 multicarts already make. Its `irq_asserted` arm answers
    /// `false` through its own `Mapper` default, which costs this enum's inner match one more arm
    /// of the body it already has.
    #[cfg(feature = "board-68")]
    Sunsoft4(Sunsoft4),
    /// Mappers 35, 90, 209 and 211, the J.Y. Company ASIC (MF-UNL-4) — **the largest board in
    /// this enum and the one that needed the most of it**.
    ///
    /// `docs/review/UNL-census.md` §7.2 calls it "the only board in the plan needing **four**
    /// hooks at once — a CPU-cycle IRQ *and* an A12 scanline IRQ with a selectable clock source,
    /// an MMC4-style CHR latch, and nametables served from CHR-ROM". Every one of those four is
    /// a method **this variant already forwards** for an earlier board: `on_cpu_cycle` and
    /// `irq_asserted` for the six boards MF-6b put here, `on_ppu_address_bus` for the MMC3
    /// clones, and `nametable_page` plus `read_nametable` for mapper 68 (MF-6c step 4). So the
    /// four hooks cost `Board`'s dispatches **nothing**, and the only new bodies are inside this
    /// enum's own inner matches — the trade [`IrqBoard`]'s note calls "one inner match … for the
    /// whole lot instead of one per board".
    ///
    /// **Boxed**, on [`IrqBoard::Somari`]'s own measurement: a [`JyCompany`] is a `BankWindows`,
    /// twenty-eight register bytes, some thirty mode bits and a four-entry derived page cache,
    /// which inline would be the variant that sets this enum's size — and MF-6b step 1 priced one
    /// byte of displacement in that payload at +0.009 % to +0.027 % on eleven of lot PR-1's
    /// thirteen ROMs. Boxed, the enum is the size MF-6a3 step 1 measured.
    #[cfg(feature = "board-jy-asic")]
    JyCompany(Box<JyCompany>),
    /// Mappers 12 and 14, the **Huang-1** ASIC's other two PCBs (MF-UNL-8 steps 1 and 2) — the
    /// Gouder SL-5020B, where the chip "functions exclusively as an MMC3A clone", and the Gouder
    /// SL-1632, where a PAL gives it two modes and a supervisor register at `$A131`.
    ///
    /// Here for [`IrqBoard::Somari`]'s reason, which is the same chip's: the board holds an
    /// [`super::Mmc3`] whose counter is the chip's own, so it answers **yes** to
    /// [`Mapper::irq_asserted`]. Every method it needs is one this variant **already forwards** for
    /// an earlier board — `try_read_prg` for the scrambled MMC3 clones, `prg_ram_enabled` and
    /// `prg_ram_writable` for the TQROM, `on_ppu_address_bus` for every MMC3 clone in here — so
    /// `Board`'s own dispatches do not grow an arm.
    ///
    /// **Boxed**, on [`IrqBoard::Somari`]'s own measurement: mapper 14's board holds an `Mmc3`
    /// *and* a `Vrc2Vrc4`, which inline would be the variant that sets this enum's size, and MF-6b
    /// step 1 priced one byte of displacement in that payload at +0.009 % to +0.027 % on eleven of
    /// lot PR-1's thirteen ROMs. The price is one pointer dereference per forwarded call for these
    /// two numbers and no other board — §4.10's rule.
    #[cfg(feature = "board-huang1")]
    Huang1(Box<Huang1>),
    /// iNES mapper 83 and NES 2.0 mapper 264, the Cony / Yoko ASIC (MF-UNL-8 step 3): a 16-bit
    /// counter whose clock is **selectable between CPU M2 and PPU A12**, so the board answers
    /// **yes** to [`Mapper::irq_asserted`] and belongs here.
    ///
    /// Every hook it needs is one this variant already forwards for the J.Y. ASIC, which is the
    /// other board in here with two clock sources: `on_cpu_cycle`, `on_ppu_address_bus`,
    /// `irq_asserted` and `try_read_prg`.
    ///
    /// **Boxed**, on [`IrqBoard::Somari`]'s own measurement: a [`Cony`] is a `BankWindows`, a
    /// register file of fifteen bytes and the counter, which inline would set this enum's size.
    #[cfg(feature = "board-cony")]
    Cony(Box<Cony>),
    /// iNES mapper 91, the JY830623C / YY840238C and the EJ-006-1 (MF-UNL-8 step 4): two boards
    /// with two counters — "submapper 0 has a PA12-based IRQ counter and hard-wired mirroring;
    /// submapper 1 an M2-based IRQ counter and selectable mirroring" — so both answer **yes** to
    /// [`Mapper::irq_asserted`].
    ///
    /// The two hooks the two submappers want, `on_cpu_cycle` and `on_ppu_address_bus`, are both
    /// already forwarded here for the J.Y. ASIC and the Cony board, so `Board` grows nothing.
    ///
    /// **Not boxed**, on [`IrqBoard::Mmc3Kof97`]'s reason: the board is a `BankWindows` and sixteen
    /// bytes, where this enum already carries a whole `Mmc3Board` inline for the TQROM, so it sets
    /// no new high-water mark.
    #[cfg(feature = "board-91")]
    Mapper091(Mapper091),
}

/// Expands `$body` once per board with it bound to `$b`, as `Board`'s own `dispatch!` does.
macro_rules! inner {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            IrqBoard::Vrc3($b) => $body,
            IrqBoard::IremH3001($b) => $body,
            IrqBoard::Sunsoft3($b) => $body,
            #[cfg(feature = "board-105")]
            IrqBoard::NesEvent($b) => $body,
            IrqBoard::TaitoTc0690($b) => $body,
            #[cfg(feature = "board-158")]
            IrqBoard::Tengen800037($b) => $body,
            #[cfg(feature = "board-119")]
            IrqBoard::Tqrom($b) => $body,
            #[cfg(feature = "board-mmc3-multicart")]
            IrqBoard::Mmc3Multicart($b) => $body,
            #[cfg(feature = "board-mmc3-chrram")]
            IrqBoard::Mmc3ChrRam($b) => $body,
            #[cfg(feature = "board-mmc3-scramble")]
            IrqBoard::Mmc3Scramble($b) => $body,
            #[cfg(feature = "board-263")]
            IrqBoard::Mmc3Kof97($b) => $body,
            #[cfg(feature = "board-189")]
            IrqBoard::Mmc3Txc189($b) => $body,
            #[cfg(feature = "board-268")]
            IrqBoard::Mmc3Outer($b) => $body,
            #[cfg(feature = "board-215")]
            IrqBoard::Mmc3Unl8237($b) => $body,
            #[cfg(feature = "board-208")]
            IrqBoard::Mmc3Gouder208($b) => $body,
            #[cfg(feature = "board-219")]
            IrqBoard::Mmc3A9461($b) => $body,
            #[cfg(feature = "board-116")]
            IrqBoard::Somari($b) => $body,
            #[cfg(feature = "board-68")]
            IrqBoard::Sunsoft4($b) => $body,
            #[cfg(feature = "board-jy-asic")]
            IrqBoard::JyCompany($b) => $body,
            #[cfg(feature = "board-huang1")]
            IrqBoard::Huang1($b) => $body,
            #[cfg(feature = "board-cony")]
            IrqBoard::Cony($b) => $body,
            #[cfg(feature = "board-91")]
            IrqBoard::Mapper091($b) => $body,
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
    /// of unbanked PRG-RAM is exactly what [`Mapper::prg_ram_window`]'s default answers. One body
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
    #[cfg(any(
        feature = "board-105",
        feature = "board-116",
        feature = "board-119",
        feature = "board-268",
        feature = "board-68",
        feature = "board-91",
        feature = "board-cony",
        feature = "board-huang1",
        feature = "board-jy-asic"
    ))]
    fn prg_ram_enabled(&self) -> bool {
        inner!(self, b => b.prg_ram_enabled())
    }

    /// Forwarded for the TQROM alone (mapper 119, MF-6a3 step 1), which is an MMC3 board and so
    /// has the MMC3's $A001: bit 7 enables the RAM and **bit 6 write-protects it**. Without the
    /// forward, `Mapper::prg_ram_writable`'s default would answer
    /// [`IrqBoard::prg_ram_enabled`] and bit 6 would do nothing on that board.
    ///
    /// It is **behaviour-neutral for the other boards of this enum**: none of the six MF-6b put
    /// here overrides `prg_ram_writable`, so each one's own default is `self.prg_ram_enabled()`
    /// — the same answer the un-forwarded dispatch gave them. The two multicarts need nothing
    /// from it either: they carry no PRG-RAM at all, and the condition their outer register puts
    /// on itself reads `Mmc3Board::prg_ram_writable` directly.
    #[cfg(any(
        feature = "board-116",
        feature = "board-119",
        feature = "board-268",
        feature = "board-huang1"
    ))]
    fn prg_ram_writable(&self) -> bool {
        inner!(self, b => b.prg_ram_writable())
    }

    /// Forwarded for the two MMC3 multicarts (47 and 37, MF-6a3 step 1), whose outer register *is*
    /// the board's decode of $6000-$7FFF — see `Mmc3Board::multicart_outer_write` — and for
    /// mapper 268, whose six outer registers are at $6000-$6FFF or $5000-$5FFF and whose window
    /// is the WRAM's as well (MF-UNL-6 step 1). No other board of this enum overrides it, so the
    /// arm this adds to `Board::intercept_cpu_write` answers `false` for all of them exactly as
    /// the un-forwarded dispatch did.
    #[cfg(any(feature = "board-mmc3-multicart", feature = "board-268"))]
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        inner!(self, b => b.intercept_cpu_write(memory, address, value))
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
    #[cfg(any(feature = "board-158", feature = "board-68", feature = "board-jy-asic"))]
    #[inline(never)]
    fn nametable_page(&self, table: i32) -> i32 {
        inner!(self, b => b.nametable_page(table))
    }

    /// Forwarded for mapper 68 behind `board-68-chr-nametables` and for the J.Y. Company ASIC
    /// behind `board-jy-asic`: this is the dispatch Q-123 measured a cap of two bodies on, and the
    /// owner's rule of 2026-11-06 is what put the first of the two here ("A costly *mechanism* of
    /// a board … is implemented too, behind its own feature if it costs more than the board
    /// itself").
    ///
    /// **MF-6c step 4 re-measured Q-123's ceiling and did not find it.** MF-5a built five shapes
    /// for a third arm on a tree seven lots older and the best of them took
    /// `Ppu2C02::read_nametable_inner` from 132 instructions to 109; on this tree that function is
    /// **114 → 75**, the body leaving by a tail call, and the thirteen ROMs of lot PR-1 move by
    /// **+17 007 in total** with `readbuf` — the CNROM whose whole run is fetches — **−78 628**.
    /// The cap is re-stated rather than quoted, which is what the step was asked to do.
    ///
    /// **MF-UNL-4 step 1 asked a third time, and this is where the answer lands.** `Board`'s own
    /// dispatch already had five distinct bodies; a sixth was measured in two shapes — as its own
    /// `Board` arm and as one more body *inside this forward* — and `read_nametable_inner` came out
    /// **80 instructions in all three binaries, identical to the instruction**, with PR-1's worst
    /// ROM at +0.002 % and all five level-2 `core` figures at +0. So the J.Y. ASIC's nametables
    /// cost `Board`'s dispatches nothing at all: the arm mapper 68 bought is the one they use, and
    /// the rule for the next lot is to put its body inside a shared variant's existing forward.
    /// `rust/docs/PERFORMANCE-HISTORY.md` §5.18 has the figures.
    ///
    /// **Behaviour-neutral for the other boards of this variant**: none of them overrides it, and
    /// none answers `SERVED_BY_CARTRIDGE` from `nametable_page`, so this body is never reached for
    /// them.
    #[cfg(any(feature = "board-68-chr-nametables", feature = "board-jy-asic"))]
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        inner!(self, b => b.read_nametable(memory, address))
    }

    /// Forwarded for `board-mmc3-scramble` and 121 and 187 are why: both answer a
    /// **protection read below `$6000`** -- 121 from its four-byte array, 187 with a byte whose
    /// bit 7 *The King of Fighters 96* requires -- and `Mapper::try_read_prg`'s default answers
    /// `None` there.
    ///
    /// It is **behaviour-neutral for the other boards of this enum**: none of the ten overrides
    /// it, so each one's own default is `self.banks().and_then(...read_prg...)`, which is the
    /// same answer the un-forwarded dispatch gave them.
    ///
    /// This is the dispatch lot MF-CART freed. MF-6c measured nine shapes against
    /// `Cartridge::default_cpu_read`'s inlined copy of its jump table at `mmc5` +7 456 756
    /// instructions over 300 frames a time; MF-CART step 4 inlined that function away and
    /// measured `Mmc5::try_read_prg` and `Cartridge::try_cpu_read` **identical to the
    /// instruction** with `Board` at thirty-six, thirty-eight and thirty-nine variants. Step 8
    /// of this lot re-measures it rather than taking that on trust.
    /// **Mapper 12 is the fourth board to need it** (MF-UNL-8 step 1): its GAL answers a read of
    /// `$4132` with a language bit, and `Mapper::try_read_prg`'s default answers `None` there.
    /// **Mapper 208 is the fifth** (MF-UNL-11 step 2): its four protection registers answer a read
    /// of `$5800-$5FFF`, which is what the game's own check reads. The `board-208`-alone
    /// configuration of step 5 is what caught that this list is not implied by the variant.
    #[cfg(any(
        feature = "board-mmc3-scramble",
        feature = "board-208",
        feature = "board-268",
        feature = "board-cony",
        feature = "board-huang1",
        feature = "board-jy-asic"
    ))]
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        inner!(self, b => b.try_read_prg(memory, address))
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

    /// The three CHR-latch methods, forwarded for the J.Y. Company ASIC alone (MF-UNL-4) and
    /// **only behind `board-jy-asic-tile-latch`** — a second feature for one mechanism of one
    /// board, which the owner's rule of 2026-11-06 is what allows ("A costly *mechanism* of a
    /// board … is implemented too, behind its own feature if it costs more than the board
    /// itself") and which MF-UNL-4 step 7 measured four shapes of.
    ///
    /// **The board is free and these three are not.** `$D003` bit 7's latch itself lives in
    /// [`IrqBoard::on_ppu_address_bus`] and is in `default`; what is behind the feature is the
    /// *tile emitter's* view of it — `Ppu2C02` reads the latch state once per captured line,
    /// walks the line with `advance_chr_latch` so that each record carries the pattern the
    /// hardware would show at that column, then puts the saved value back (REQ-CORE-039).
    ///
    /// Three bodies in those three dispatches re-lay `Ppu2C02::run_dots` out, and that function
    /// is **57 % to 62 % of `NesCore::run_frame`** (PERFORMANCE-HISTORY §5.17). Measured against
    /// `16a30564` over 300 frames of lot PR-1's thirteen ROMs:
    ///
    /// | shape | worst ROM | set total |
    /// | --- | --- | ---: |
    /// | the three forwards as first written | `mmc5` **+0.374 %**, `smb3` +0.229 % | +79 123 279 |
    /// | the same three `#[inline(never)]` | `metroid` **+0.030 %**, five ROMs over tolerance | +7 771 829 |
    /// | a `chr_latch` latch on `Cartridge`, §4.10's own mechanism | `mmc5` **+0.326 %** — and ten ROMs **faster**, by up to −0.103 % | +16 981 759 |
    /// | **not forwarded at all** | **no ROM slower**, `smb3` −48 914 | **−417 513** |
    ///
    /// The whole of every figure is `run_dots` and `emit_sprite`, in spill churn: it is MF-6a
    /// step 2's finding on `Board::chr_view` one dispatch along. The third row is the interesting
    /// one — gating the three calls makes the *library* measurably faster and the three boards
    /// that keep a latch measurably slower, so the structural fix is a lot of its own and not
    /// this one's.
    ///
    /// **With the feature off the emulation is identical**, which is the point: `Mapper`'s
    /// defaults are `0`, nothing and nothing, so the tile emitter saves 0, moves nothing and
    /// restores 0, and the board's own latch — which only `on_ppu_address_bus` moves — never
    /// sees it. REQ-CORE-033's "the emulation is unchanged whether or not records were built"
    /// holds exactly. What is lost is that a 3D tile **record** of a mapper-90 game carries the
    /// bank the latch holds at capture time rather than the one the hardware would show at that
    /// column — the same thing every board but the MMC2 and the MMC4 gets, and no 3D pack exists
    /// for any of the thirteen titles (§5 of `docs/review/UNL-census.md`: there is no ROM to run).
    ///
    /// They are **behaviour-neutral for the other boards of this enum** in either configuration:
    /// none of the eleven overrides any of the three.
    #[cfg(feature = "board-jy-asic-tile-latch")]
    #[inline(never)]
    fn chr_latch_state(&self) -> i32 {
        inner!(self, b => b.chr_latch_state())
    }

    #[cfg(feature = "board-jy-asic-tile-latch")]
    #[inline(never)]
    fn set_chr_latch_state(&mut self, state: i32) {
        inner!(self, b => b.set_chr_latch_state(state));
    }

    #[cfg(feature = "board-jy-asic-tile-latch")]
    #[inline(never)]
    fn advance_chr_latch(&mut self, pattern_table: i32, tile: i32) {
        inner!(self, b => b.advance_chr_latch(pattern_table, tile));
    }

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        inner!(self, b => b.on_cpu_cycle(memory));
    }
}
