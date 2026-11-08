// `board-decode-boards` is the umbrella each board in this enum implies (`board-77 =
// ["board-decode-boards"]`), and Cargo cannot express the other direction: nothing stops
// `--features board-decode-boards` on its own, which would leave this enum with no variant and
// `Board::DecodeBoard` uninhabited. One clear error beats nine confusing ones. **A later member
// of this enum adds its feature to the `any` below**, and to nothing else -- the module, the
// `Board` variant, its `dispatch!` arm and the registry's `use` are all behind the umbrella.
#[cfg(not(any(
    feature = "board-38",
    feature = "board-77",
    feature = "board-96",
    feature = "board-107",
    feature = "board-111",
    feature = "board-133",
    feature = "board-143",
    feature = "board-144",
    feature = "board-145",
    feature = "board-149",
    feature = "board-162",
    feature = "board-163",
    feature = "board-178",
    feature = "board-190",
    feature = "board-218",
    feature = "board-jaleco-72-92-86",
    feature = "board-jv001",
    feature = "board-nina03",
    feature = "board-sachen-8reg",
    feature = "board-sachen8259",
    feature = "board-vs-system"
)))]
compile_error!(
    "feature `board-decode-boards` is implied by each of its boards (`board-77`, …) and is not \
     meant to be selected on its own"
);

#[cfg(feature = "board-111")]
use super::Cheapocabra;
#[cfg(any(feature = "board-162", feature = "board-163"))]
use super::ChrWindowBoard;
#[cfg(feature = "board-77")]
use super::IremLrog017;
#[cfg(feature = "board-jv001")]
use super::Jv001;
#[cfg(feature = "board-218")]
use super::MagicFloor;
use super::Mapper;
#[cfg(feature = "board-nina03")]
use super::Nina03;
#[cfg(feature = "board-96")]
use super::OekaKids;
#[cfg(feature = "board-133")]
use super::Sachen72008;
#[cfg(feature = "board-sachen8259")]
use super::Sachen8259;
#[cfg(feature = "board-sachen-8reg")]
use super::Sachen8Reg;
#[cfg(any(
    feature = "board-38",
    feature = "board-107",
    feature = "board-143",
    feature = "board-144",
    feature = "board-145",
    feature = "board-149",
    feature = "board-190"
))]
use super::SingleLatch;
#[cfg(feature = "board-vs-system")]
use super::VsBoard;
#[cfg(feature = "board-178")]
use super::WaixingFs305;
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
    // Appended by MF-UNL-1 step 1, the first unlicensed board of this enum: six mapper numbers
    // and one variant, which is the shape `SmallBoard` and `Namcot108` already use for a family.
    /// Mappers 36, 132, 136, 147, 172 and 173, the TXC 05-00002-010 / Sachen JV001 protection
    /// ASIC.
    #[cfg(feature = "board-jv001")]
    Jv001(Jv001),
    // Appended by MF-UNL-1 step 2.
    /// Mappers 150 and 243, Sachen's eight-register ASIC.
    #[cfg(feature = "board-sachen-8reg")]
    Sachen8Reg(Sachen8Reg),
    // Appended by MF-UNL-1 step 3, the lot's last board and the smallest of the three.
    /// Mapper 133, the Sachen 72008.
    #[cfg(feature = "board-133")]
    Sachen72008(Sachen72008),
    // Appended by MF-UNL-3 step 1, the first board of a lot whose eight are all of this shape.
    /// Mappers 79 and 146, the AVE NINA-03/06 and the Sachen 3015 / SA-016.
    #[cfg(feature = "board-nina03")]
    Nina03(Nina03),
    // Appended by MF-UNL-3 step 2, and it is the last variant that lot adds: the seven
    // single-latch boards share **one** of them, with a runtime field saying which, exactly as
    // `Jv001` above holds six numbers and `Namcot108` five.
    /// Mappers 38, 107, 143, 144, 145, 149 and 190, lot MF-UNL-3's seven single-latch boards.
    #[cfg(any(
        feature = "board-38",
        feature = "board-107",
        feature = "board-143",
        feature = "board-144",
        feature = "board-145",
        feature = "board-149",
        feature = "board-190"
    ))]
    SingleLatch(SingleLatch),
    // Appended by MF-UNL-5 step 1, the first board of a lot whose three are all of this shape.
    /// Mappers 163 and 162, the 南晶 (Nánjīng) FC-001 and the 外星 (Wàixīng) FS304 — the second
    /// and third boards of this enum whose CHR window is latched off the PPU address bus, after
    /// mapper 96.
    #[cfg(any(feature = "board-162", feature = "board-163"))]
    ChrWindowBoard(ChrWindowBoard),
    // Appended by MF-UNL-5 step 3, the lot's last board and the one its section heading describes
    // least well: this one has no PPU-driven CHR window at all (Q-154).
    /// Mapper 178, the 外星 (Wàixīng) FS305 / 南晶 (Nánjīng) NJ0430 — four PRG modes and the
    /// enum's first **banked** PRG-RAM window.
    #[cfg(feature = "board-178")]
    WaixingFs305(WaixingFs305),
    // Appended by MF-UNL-7 step 1, the first board of a lot whose six are all of this shape: four
    // numbers and one variant, the fourth board of this enum to hold a whole family after `Jv001`
    // (six), `SingleLatch` (seven) and `Nina03` (two).
    /// Mappers 137, 138, 139 and 141, the Sachen 8259 — one chip with four wirings.
    #[cfg(feature = "board-sachen8259")]
    Sachen8259(Sachen8259),
    // Appended by MF-UNL-7 step 5: the first board of this enum with **no register at all**, and
    // the second after mapper 111 to serve its own nametables.
    /// Mapper 218, the single-chip homebrew board of *Magic Floor*.
    #[cfg(feature = "board-218")]
    MagicFloor(MagicFloor),
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
            #[cfg(feature = "board-jv001")]
            DecodeBoard::Jv001($b) => $body,
            #[cfg(feature = "board-sachen-8reg")]
            DecodeBoard::Sachen8Reg($b) => $body,
            #[cfg(feature = "board-133")]
            DecodeBoard::Sachen72008($b) => $body,
            #[cfg(feature = "board-nina03")]
            DecodeBoard::Nina03($b) => $body,
            #[cfg(any(
                feature = "board-38",
                feature = "board-107",
                feature = "board-143",
                feature = "board-144",
                feature = "board-145",
                feature = "board-149",
                feature = "board-190"
            ))]
            DecodeBoard::SingleLatch($b) => $body,
            #[cfg(any(feature = "board-162", feature = "board-163"))]
            DecodeBoard::ChrWindowBoard($b) => $body,
            #[cfg(feature = "board-178")]
            DecodeBoard::WaixingFs305($b) => $body,
            #[cfg(feature = "board-sachen8259")]
            DecodeBoard::Sachen8259($b) => $body,
            #[cfg(feature = "board-218")]
            DecodeBoard::MagicFloor($b) => $body,
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

    /// Forwarded for mapper 178 alone (MF-UNL-5 step 3), the one board of this enum whose
    /// $6000-$7FFF window is a **banked** RAM chip: "PRG-RAM is mapped to CPU $6000-$7FFF and is
    /// banked in 8 KiB amounts", from all eight bits of its `$4803`.
    ///
    /// Asked once per CPU write and **not** once per CPU read, which is lot MF-CART's whole point:
    /// `Cartridge` caches the answer beside `rom_windows`, so no `Board` dispatch is on the read
    /// path of that window for any board.
    ///
    /// **Behaviour-neutral for every other board here**: none of them overrides it, so each takes
    /// `Mapper`'s own default — the whole chip at offset zero — which is what the un-forwarded
    /// dispatch gave the variant.
    #[cfg(feature = "board-178")]
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        inner!(self, b => b.prg_ram_window(memory))
    }

    /// Forwarded for the JV001 family alone (MF-UNL-1 step 1), whose protection register is at
    /// $4100 — below $8000, where `Cartridge::default_cpu_read` falls through to this dispatch.
    ///
    /// **`#[inline(never)]`, and that is the whole reason the hook is this one**:
    /// [`super::SmallBoard::try_read_prg`]'s own note has the measurement, and it is MF-6c step
    /// 2's. `Board::try_read_prg` carries `#[inline(always)]` and folds into
    /// `Cartridge::try_cpu_read`; with the decode behind an `#[inline(never)]` forward the arm
    /// this variant contributes to that inlined jump table is **one `callq`**, whatever this
    /// enum grows to later. Answering from `Mapper::cpu_peek_override` instead was measured at
    /// +0.111 % on lot PR-1's set.
    ///
    /// **Behaviour-neutral for the five boards already here**: none of them overrides it, so
    /// each takes `Mapper`'s own default — `self.banks().and_then(BankWindows::read_prg)` —
    /// which is the answer the un-forwarded dispatch gave the whole variant.
    ///
    /// **Mapper 143 joins it in MF-UNL-3 step 3**, whose protection read is at the same $4100
    /// under the same mask and whose board has no write side at all. Nothing else of this lot
    /// overrides the hook, so the other eight numbers of it keep `Mapper`'s own default here.
    ///
    /// **And mapper 163 in MF-UNL-5 step 1**, whose feedback register is read at $5500-$5501 under
    /// mask $F300 — the fourth board of the enum to want the hook and the third publisher's
    /// protection scheme to be a read below $8000.
    #[cfg(any(
        feature = "board-143",
        feature = "board-163",
        feature = "board-jv001",
        feature = "board-sachen-8reg"
    ))]
    #[inline(never)]
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        inner!(self, b => b.try_read_prg(memory, address))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.ppu_write(memory, address, value));
    }

    /// Forwarded for mapper 96 (MF-6c step 8) and mappers 163 and 162 (MF-UNL-5 steps 1 and 2),
    /// the boards here whose whole CHR mechanism is the PPU address bus. Behind `Cartridge::watches_ppu_reads`
    /// and `Cartridge::watches_ppu_bus_without_read`, so no board outside those two latches
    /// reaches this dispatch at all; `OekaKids::on_ppu_address_bus` and
    /// `ChrWindowBoard::on_ppu_address_bus` both carry `#[inline(never)]` so that the arm this
    /// adds beside the three A12 counters' is a `callq` and not a body.
    ///
    /// **Behaviour-neutral for every other board here**: none of them overrides it, so each takes
    /// `Mapper`'s empty default, which is what the un-forwarded dispatch did for the whole
    /// variant.
    #[cfg(any(feature = "board-96", feature = "board-162", feature = "board-163"))]
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
    ///
    /// **And the Sachen 8259 in MF-UNL-7 step 1**, whose R7 mode 2 is `(0,1,1,1)` — one table on
    /// CIRAM page 0 and the other three on page 1, which is the eight-register ASIC's mode 0 read
    /// the other way round and the same §4.5 cheap level.
    ///
    /// **And mapper 218 in MF-UNL-7 step 5**, which answers `SERVED_BY_CARTRIDGE` for all four
    /// tables always: its 2 KiB *is* the console's nametable RAM, and the board maps the same
    /// 2 KiB into the pattern tables.
    #[cfg(any(
        feature = "board-111",
        feature = "board-218",
        feature = "board-sachen-8reg",
        feature = "board-sachen8259"
    ))]
    #[inline(never)]
    fn nametable_page(&self, table: i32) -> i32 {
        inner!(self, b => b.nametable_page(table))
    }

    /// Forwarded for mapper 111 alone (MF-6c step 9), the board whose two 8 KiB nametable banks
    /// hold two sets of four distinct tables where this core's VRAM holds one. Reached only
    /// behind a `SERVED_BY_CARTRIDGE` answer from [`DecodeBoard::nametable_page`] above, so a
    /// board that does not serve its own nametables never calls either.
    ///
    /// **Mapper 218 joins it in MF-UNL-7 step 5**, and it is the fifth distinct body of that
    /// dispatch in the core and the second inside this enum — so `Board::read_nametable`'s own
    /// jump table does not grow by an entry, which is what MF-6c step 7 established for a board
    /// joining here.
    #[cfg(any(feature = "board-111", feature = "board-218"))]
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        inner!(self, b => b.read_nametable(memory, address))
    }

    /// The counterpart, for the same two boards and the same reason.
    #[cfg(any(feature = "board-111", feature = "board-218"))]
    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        inner!(self, b => b.write_nametable(memory, address, value));
    }
}
