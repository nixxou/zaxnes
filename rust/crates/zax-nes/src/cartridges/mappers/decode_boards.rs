// `board-decode-boards` is the umbrella each board in this enum implies (`board-77 =
// ["board-decode-boards"]`), and Cargo cannot express the other direction: nothing stops
// `--features board-decode-boards` on its own, which would leave this enum with no variant and
// `Board::DecodeBoard` uninhabited. One clear error beats nine confusing ones. **A later member
// of this enum adds its feature to the `any` below**, and to nothing else -- the module, the
// `Board` variant, its `dispatch!` arm and the registry's `use` are all behind the umbrella.
#[cfg(not(any(
    feature = "board-77",
    feature = "board-96",
    feature = "board-111",
    feature = "board-jaleco-72-92-86",
    feature = "board-vs-system"
)))]
compile_error!(
    "feature `board-decode-boards` is implied by each of its boards (`board-77`, …) and is not \
     meant to be selected on its own"
);

#[cfg(feature = "board-111")]
use super::Cheapocabra;
#[cfg(feature = "board-77")]
use super::IremLrog017;
use super::Mapper;
#[cfg(feature = "board-96")]
use super::OekaKids;
#[cfg(feature = "board-vs-system")]
use super::VsBoard;
#[cfg(feature = "board-jaleco-72-92-86")]
use super::{JalecoJf13, JalecoJf17};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The **second** enum of boards that are a register decode and nothing else, sharing one
/// [`Board`] variant between them (MF-6c step 6).
///
/// [`Board`]: crate::cartridges::Board
/// [`SmallBoard`]: super::SmallBoard
/// [`IrqBoard`]: super::IrqBoard
///
/// # Why a second one exists at all
///
/// [`SmallBoard`] is where a board of this shape belonged from MF-6a step 2 until MF-6c step 5,
/// and that enum's own note has the four measurements that closed it: **one more variant in
/// either [`SmallBoard`] or [`IrqBoard`] costs `mmc5` +7 456 756 instructions over 300 frames,
/// +0.066 %**, against the owner's tolerance of +0.02 % per ROM. The figure is quantised —
/// `Cartridge::default_cpu_read` has two layouts on that ROM and a variant past each enum's own
/// limit buys one unit of the difference — and step 5 measured two units for two variants. So the
/// four remaining licensed boards of lot MF-6c (77; 72, 92 and 86; 96; 111) have no free home,
/// and R0's decision of 2026-11-06 names the first shape to measure instead:
///
/// > **One new `Board` variant holding all four boards** — a third shared enum beside
/// > `SmallBoard` and `IrqBoard` … This is **one** discriminant for all eight numbers, and the
/// > figure to beat is MF-6a3 step 1's: **two** new `Board` discriminants cost `mmc5`
/// > +8 994 278 (+0.080 %) … **One** entry has never been measured on its own and may well be
/// > free.
///
/// This enum is that experiment. It keeps [`SmallBoard`]'s and [`IrqBoard`]'s inner matches
/// exactly where they are — neither grows a variant — and pays one jump-table slot in each
/// `Board` dispatch instead. `docs/progress/MF-6c.md` step 6 has what that slot measured.
///
/// # Its entry condition, and what is forwarded
///
/// [`SmallBoard`]'s, unchanged: every board here answers *no* to [`Mapper::irq_asserted`] through
/// `Mapper`'s own default, so all of them share that dispatch's empty arm, and the per-cycle and
/// per-fetch methods are **deliberately not forwarded** for the reason that note gives — an inner
/// `match` returning `false` is not the same code as `false`, and a body too many in one of those
/// dispatches is what MF-5b step 1 priced at 170 M instructions.
///
/// Only the methods a board in here actually implements are forwarded, and [`Mapper::chr_view`]
/// answers `None` directly rather than forwarding, which is the one line MF-6a step 2's bar turned
/// on: see [`DecodeBoard::chr_view`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeBoard {
    /// Mapper 77, the Irem LROG017 of *Napoleon Senki* (MF-6c step 6).
    #[cfg(feature = "board-77")]
    IremLrog017(IremLrog017),
    // Appended by MF-6c step 7, which adds three numbers and **no** `Board` variant: step 6
    // paid for the discriminant and measured it at zero, so a board joining this enum is now
    // free to the rest of the library exactly as one joining [`SmallBoard`] was until step 2.
    //
    // [`SmallBoard`]: super::SmallBoard
    /// Mappers 72 and 92, Jaleco's JF-17 and JF-19, which differ by one `bool`.
    #[cfg(feature = "board-jaleco-72-92-86")]
    JalecoJf17(JalecoJf17),
    /// Mapper 86, Jaleco's JF-13: the same speech chip on a GNROM-like board.
    #[cfg(feature = "board-jaleco-72-92-86")]
    JalecoJf13(JalecoJf13),
    // Appended by MF-6c step 8.
    /// Mapper 96, the Bandai Oeka Kids board, whose CHR bank is latched off the PPU address bus.
    #[cfg(feature = "board-96")]
    OekaKids(OekaKids),
    // Appended by MF-6c step 9, the last board of the lot.
    /// Mapper 111, the Membler Industries GTROM / Cheapocabra board.
    #[cfg(feature = "board-111")]
    Cheapocabra(Cheapocabra),
    // Appended by MF-VS step 4, the lot's one board: the register is a `$4016` write and not a
    // cartridge write, which is the only thing that makes it unusual here.
    /// Mapper 99, the Nintendo VS. System board.
    #[cfg(feature = "board-vs-system")]
    VsBoard(VsBoard),
}

/// Expands `$body` once per board with it bound to `$b`, as [`SmallBoard`]'s own `inner!` does.
///
/// [`SmallBoard`]: super::SmallBoard
macro_rules! inner {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            #[cfg(feature = "board-77")]
            DecodeBoard::IremLrog017($b) => $body,
            #[cfg(feature = "board-jaleco-72-92-86")]
            DecodeBoard::JalecoJf17($b) => $body,
            #[cfg(feature = "board-jaleco-72-92-86")]
            DecodeBoard::JalecoJf13($b) => $body,
            #[cfg(feature = "board-96")]
            DecodeBoard::OekaKids($b) => $body,
            #[cfg(feature = "board-111")]
            DecodeBoard::Cheapocabra($b) => $body,
            #[cfg(feature = "board-vs-system")]
            DecodeBoard::VsBoard($b) => $body,
        }
    };
}

impl Mapper for DecodeBoard {
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

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.write_register(memory, address, value));
    }

    /// **`None`, and not a forward** — the line MF-6a step 2's measurement turned on, quoted in
    /// [`SmallBoard::chr_view`]'s own note: this dispatch's arms are inlined into
    /// `Ppu2C02::run_visible_span` and so into `Ppu2C02::run_dots`, and one more *distinct body*
    /// there cost **+3.3 M to +6.9 M over 300 frames on eight of lot PR-1's thirteen ROMs** in
    /// spill churn alone. A forward is such a body, because `Mapper::chr_view`'s default is
    /// monomorphised per board; `None` is the body the MMC2, the MMC3, the MMC5, the RAMBO-1 and
    /// [`SmallBoard`] already answer, so the dispatch gains nothing to lay out.
    ///
    /// The price is that the games of these numbers take [`Mapper::ppu_peek`] one fetch at a time
    /// instead of the span fast path — a cost on the boards that caused it and on no other game,
    /// which is §4.10's own rule. Mapper 77 could not have answered this dispatch anyway: its two
    /// CHR chips cannot be one [`ChrView`].
    ///
    /// [`SmallBoard`]: super::SmallBoard
    /// [`SmallBoard::chr_view`]: super::SmallBoard::chr_view
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    /// **Forwarded, unlike [`DecodeBoard::chr_view`] above**, and MF-6c step 7 is the step that
    /// turned it from a direct `None` into this: three of the four boards here are ordinary
    /// CHR-ROM boards that *can* show a view, and what the view buys is not speed but accuracy.
    /// `Ppu2C02::read_row_planes` captures `view.offsets()` when a background row is drawn and
    /// replays the row through them later (REQ-CORE-037); with `None` it falls back on
    /// `read_pattern_planes` **with the banks as they are now**, so a game that switches its
    /// 8 KiB CHR bank mid-frame would have its 3D tiles captured from the wrong bank.
    ///
    /// This dispatch is **not** inside `Ppu2C02::run_dots` — which is the whole reason
    /// [`DecodeBoard::chr_view`] may not be forwarded — and [`super::SmallBoard`] and
    /// [`super::IrqBoard`] both forward it already. Mapper 77 answers `None` from its own impl,
    /// because its two CHR chips are not one [`ChrView`].
    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        inner!(self, b => b.chr_peek_view(memory))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        inner!(self, b => b.ppu_peek(memory, address, fetch))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.ppu_write(memory, address, value));
    }

    /// Forwarded for mapper 96 alone (MF-6c step 8), the one board here whose whole mechanism
    /// is the PPU address bus. Behind `Cartridge::watches_ppu_reads` and
    /// `Cartridge::watches_ppu_bus_without_read`, so no board outside those two latches reaches
    /// this dispatch at all; `OekaKids::on_ppu_address_bus` carries `#[inline(never)]` so that
    /// the arm this adds beside the three A12 counters' is a `callq` and not a body.
    ///
    /// **Behaviour-neutral for the other three boards here**: none of them overrides it, so each
    /// takes `Mapper`'s empty default, which is what the un-forwarded dispatch did for the whole
    /// variant.
    #[cfg(feature = "board-96")]
    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        inner!(self, b => b.on_ppu_address_bus(memory, address, clock));
    }

    /// Forwarded for mapper 111 alone (MF-6c step 9), behind `Cartridge::nametable_pages_vary`
    /// so that no other board asks it. `#[inline(never)]`, for the reason
    /// [`super::IrqBoard::nametable_page`] and [`super::SmallBoard::nametable_page`] both carry
    /// it (MF-6b step 5): inlined, this body grows `Board::nametable_page`, which grows
    /// `Cartridge::refresh_nametable_pages`, which pushed `Cartridge::cpu_write` past the
    /// inliner's budget and cost two MMC1 ROMs about 1.2 M instructions on a dispatch no MMC1
    /// reaches.
    #[cfg(feature = "board-111")]
    #[inline(never)]
    fn nametable_page(&self, table: i32) -> i32 {
        inner!(self, b => b.nametable_page(table))
    }

    /// Forwarded for mapper 111 alone (MF-6c step 9), the board whose two 8 KiB nametable banks
    /// hold two sets of four distinct tables where this core's VRAM holds one. Reached only
    /// behind a `SERVED_BY_CARTRIDGE` answer from [`DecodeBoard::nametable_page`] above, so a
    /// board that does not serve its own nametables never calls either.
    #[cfg(feature = "board-111")]
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        inner!(self, b => b.read_nametable(memory, address))
    }

    /// The counterpart, for the same board and the same reason.
    #[cfg(feature = "board-111")]
    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.write_nametable(memory, address, value));
    }
}
