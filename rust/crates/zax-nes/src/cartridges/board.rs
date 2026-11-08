//! The [`Board`] enum: one variant per mapper family, dispatching [`Mapper`] to it
//! (REQ-CORE-100). Created once at load time by the registry.

#[cfg(feature = "board-decode-boards")]
use super::mappers::DecodeBoard;
#[cfg(feature = "board-namcot108")]
use super::mappers::Namcot108;
#[cfg(feature = "board-small-boards")]
use super::mappers::SmallBoard;
use super::mappers::{
    Action52, Action53, AxRom, BandaiFcg, Caltron6In1, CnRom, Codemasters, ColorDreams, CpRom,
    CpuReadWindows, Daou306, Fme7, GxRom, IremG101, IrqBoard, JalecoJ87, JalecoSs88006, Mapper,
    Mapper015, Mapper034, Mapper078, McAcc, Mmc1, Mmc2, Mmc3, Mmc5, Multicart225, Namco163, Nrom,
    Rambo1, UnRom512, UxRom, Vrc2Vrc4, Vrc6, Vrc7,
};
use super::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The mapper of a cartridge, with its registers (REQ-CORE-100).
///
/// **A new board is appended, never inserted.** The variants up to `Action52` are in mapper-number
/// order because that is the order they were written in; from MF-3 on, a board added to this enum
/// goes at the **end**, whatever its number. Two reasons, both measured:
///
/// * `Mmc5` must stay at index 5. `Board`'s largest variant is the MMC5 (its 1 KiB of `ExRAM`),
///   so rustc gives the enum a *niche-filling* layout whose tag lives inside that variant, and
///   every dispatch prologue is `movzbl 1355(%rcx); subb $2; movl $5; cmovael` — a literal 5.
///   Inserting a variant before it rewrites that prologue for all 27 arms (MF-2 step 0 is the
///   story of what such a reshape costs).
/// * The discriminants of the existing boards are their slots in the jump table of every
///   dispatch below. Appending leaves them where they are, so LLVM lays the existing arms out in
///   the same order; inserting in the middle shuffles them. Checked on `Board::ppu_peek` for
///   MF-3's mapper 69: appended, the arms before it are instruction-identical and only the
///   function's shared bounds-check tail changes shape; inserted after `GxRom`, four arms move
///   and their operand offsets change with them.
// Created once at load time, so the size of the MMC5 variant (its ExRAM) costs nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Board {
    /// Mapper 0.
    Nrom(Nrom),
    /// Mapper 1.
    Mmc1(Mmc1),
    /// Mapper 2.
    UxRom(UxRom),
    /// Mapper 3.
    CnRom(CnRom),
    /// Mapper 4.
    Mmc3(Mmc3),
    /// Mapper 5.
    Mmc5(Mmc5),
    /// Mapper 7.
    AxRom(AxRom),
    /// Mappers 9 and 10.
    Mmc2(Mmc2),
    /// Mapper 11.
    ColorDreams(ColorDreams),
    /// Mapper 13.
    CpRom(CpRom),
    /// Mapper 15.
    Mapper015(Mapper015),
    /// Mapper 16.
    BandaiFcg(BandaiFcg),
    /// Mapper 18.
    JalecoSs88006(JalecoSs88006),
    /// Mappers 21, 22, 23, 25 and 27.
    Vrc2Vrc4(Vrc2Vrc4),
    /// Mapper 28.
    Action53(Action53),
    /// Mapper 30.
    UnRom512(UnRom512),
    /// Mapper 32.
    IremG101(IremG101),
    /// Mapper 34.
    Mapper034(Mapper034),
    /// Mapper 41.
    Caltron6In1(Caltron6In1),
    /// Mapper 64.
    Rambo1(Rambo1),
    /// Mapper 66.
    GxRom(GxRom),
    /// Mapper 71.
    Codemasters(Codemasters),
    /// Mappers 87 and 101.
    JalecoJ87(JalecoJ87),
    /// Mapper 156.
    Daou306(Daou306),
    /// Mapper 225.
    Multicart225(Multicart225),
    /// Mapper 228.
    Action52(Action52),
    // Appended from MF-3 on, in the order the boards were added; see the note above.
    /// Mapper 69, Sunsoft FME-7 / 5A / 5B.
    Fme7(Fme7),
    /// Mapper 78, the Irem IF-12 and the Jaleco board of Uchuusen.
    Mapper078(Mapper078),
    /// Mapper 4 submapper 3, the Acclaim MC-ACC: the MMC3's code, with its ÷8 prescaler on A12's
    /// falling edges folded in at compile time (MR-4-3).
    McAcc(McAcc),
    /// Mappers 24 and 26, Konami VRC6.
    Vrc6(Vrc6),
    /// Mapper 19, the Namco 163, and mapper 210, the Namcot 175 and 340 cut from it.
    Namco163(Namco163),
    /// Mapper 85, Konami VRC7.
    Vrc7(Vrc7),
    /// Mapper 206, the Namco 108 / Namcot 109, 118, 119 and the Tengen MIMIC-1, with mappers
    /// 76, 88, 95 and 154 as its four parameters (MF-6a step 1).
    #[cfg(feature = "board-namcot108")]
    Namcot108(Namcot108),
    /// Mappers 33, 89, 93, 97, 140 and 184: the Taito TC0190, both Sunsoft-2 boards, the Irem
    /// TAM-S1, the Jaleco JF-11/14 and Sunsoft-1, **behind one discriminant** (MF-6a step 2).
    /// [`SmallBoard`]'s own note has the measurement that put them there: five variants cost
    /// `Ppu2C02::run_dots` 0.067 % to 0.131 % on eight of lot PR-1's thirteen ROMs in register
    /// pressure alone, and one costs nothing.
    #[cfg(feature = "board-small-boards")]
    SmallBoard(SmallBoard),
    /// The boards whose counter can pull /IRQ, **behind one discriminant**: mapper 73 (Konami
    /// VRC3, MF-6b step 1), mappers 65 and 67 (the Irem H3001 and the Sunsoft-3, MF-6b step 2).
    /// [`IrqBoard`]'s own note has the reasoning, which is
    /// [`SmallBoard`]'s read the other way round — every board in that enum answers *no* to
    /// [`Mapper::irq_asserted`] and therefore shares the empty arm; every board in this one
    /// answers *yes* and can only share the others'.
    /// MF-6a3 step 1 adds the TQROM (119) and the two MMC3 multicarts (47 and 37) to it, and
    /// **this enum has not grown a variant since**, which is the measurement that put them
    /// there: two more `Board` discriminants cost `mmc5` **+8 994 278 instructions over 300
    /// frames (+0.080 %)**, all of it one taken `jmp` per expansion-audio sample, because the
    /// jump table of `Board::expansion_audio_output` — which `NesBus::push_expansion_audio`
    /// inlines whole — grew by two entries and LLVM moved the fall-through off the MMC5's arm.
    /// See [`IrqBoard`]'s own note.
    IrqBoard(IrqBoard),
    /// The **second** enum of boards that are a register decode and nothing else, behind one
    /// discriminant: mapper 77, the Irem LROG017 (MF-6c step 6).
    ///
    /// It exists because [`SmallBoard`] and [`IrqBoard`] are both **full**, which MF-6c steps 2 to
    /// 5 measured four times: one more variant in either puts `Cartridge::default_cpu_read` into
    /// the dearer of its two layouts on `mmc5exram`, **+7 456 756 instructions over 300 frames,
    /// +0.066 %**, against the owner's tolerance of +0.02 % per ROM. R0's decision of 2026-11-06
    /// is to measure **one** new `Board` discriminant for all four of the lot's remaining boards
    /// instead, against MF-6a3 step 1's figure for **two** (`mmc5` +8 994 278, +0.080 %, one taken
    /// `jmp` per expansion-audio sample as the jump table of `Board::expansion_audio_output`
    /// grew); one entry alone had never been measured. This variant is that experiment, and
    /// `docs/progress/MF-6c.md` step 6 has what it cost.
    #[cfg(feature = "board-decode-boards")]
    DecodeBoard(DecodeBoard),
}

/// Expands `$body` once per variant with the board bound to `$b`.
macro_rules! dispatch {
    ($self:ident, $b:ident => $body:expr) => {
        match $self {
            Board::Nrom($b) => $body,
            Board::Mmc1($b) => $body,
            Board::UxRom($b) => $body,
            Board::CnRom($b) => $body,
            Board::Mmc3($b) => $body,
            Board::Mmc5($b) => $body,
            Board::AxRom($b) => $body,
            Board::Mmc2($b) => $body,
            Board::ColorDreams($b) => $body,
            Board::CpRom($b) => $body,
            Board::Mapper015($b) => $body,
            Board::BandaiFcg($b) => $body,
            Board::JalecoSs88006($b) => $body,
            Board::Vrc2Vrc4($b) => $body,
            Board::Action53($b) => $body,
            Board::UnRom512($b) => $body,
            Board::IremG101($b) => $body,
            Board::Mapper034($b) => $body,
            Board::Caltron6In1($b) => $body,
            Board::Rambo1($b) => $body,
            Board::GxRom($b) => $body,
            Board::Codemasters($b) => $body,
            Board::JalecoJ87($b) => $body,
            Board::Daou306($b) => $body,
            Board::Multicart225($b) => $body,
            Board::Action52($b) => $body,
            Board::Fme7($b) => $body,
            Board::Mapper078($b) => $body,
            Board::McAcc($b) => $body,
            Board::Vrc6($b) => $body,
            Board::Namco163($b) => $body,
            Board::Vrc7($b) => $body,
            #[cfg(feature = "board-namcot108")]
            Board::Namcot108($b) => $body,
            #[cfg(feature = "board-small-boards")]
            Board::SmallBoard($b) => $body,
            Board::IrqBoard($b) => $body,
            #[cfg(feature = "board-decode-boards")]
            Board::DecodeBoard($b) => $body,
        }
    };
}

impl Mapper for Board {
    fn mapper_number(&self) -> i32 {
        dispatch!(self, b => b.mapper_number())
    }

    fn banks(&self) -> Option<&BankWindows> {
        dispatch!(self, b => b.banks())
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        dispatch!(self, b => b.banks_mut())
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        dispatch!(self, b => b.save_registers(writer));
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        dispatch!(self, b => b.load_registers(reader))
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        dispatch!(self, b => b.reset(memory));
    }

    /// **`#[inline(always)]`, and it has to stay: this is the last link of the per-CPU-cycle
    /// /IRQ sample, and the only one MF-5b left to the cost model.** `<NesBus as CpuBus>::`
    /// and `Cartridge::irq_asserted` above it carry the attribute; this dispatch did not, so
    /// when the thirty-second variant pushed it past the inliner's budget LLVM gave it a symbol
    /// and twenty-five call sites inside `Cpu6502::step`. MF-5b measured that as a gain — it is
    /// what let `Cpu6502::execute_without_operand` fold into `step` — and on lot PR-1's set, 300
    /// frames with no 3D pack, it is: all thirteen ROMs went down.
    ///
    /// With a pack loaded it is a loss. R0's level-2 run (five commercial games, their packs,
    /// 600 frames, inputs pressed) priced the `callq` at **+56 834 892 instructions on Zelda II
    /// and +97 104 372 on Super Mario Bros.** against the 45.1 M of this dispatch that left
    /// `step`, and both games ended above the baseline. Pinned, with
    /// [`WideExtension::zone_offset`](crate::wide::WideExtension::zone_offset) and
    /// `execute_without_operand`, the five games are 0.123 % to 0.663 % *below* it
    /// (`docs/progress/MF-5b-fix.md`, step 3).
    ///
    /// **Pinning it is not the same as shortening it.** An eleven-arm form with the twenty-one
    /// boards that cannot assert /IRQ folded into one arm is inlined by the cost model on its own,
    /// and LLVM then compiles it to a bit mask plus a table of field offsets rather than a jump
    /// table: cheaper for a board that cannot assert, **+0.698 % on `m4`, +0.464 % on `smb3` and
    /// +0.278 % on `mmc5`** for the ones that can. Measured as variant 7 of the same step; the
    /// thirty-two-arm `dispatch!` is what the boards with an IRQ want.
    #[allow(clippy::inline_always)] // Measured: MF-5b-fix step 3, +56.8 M on Zelda II's pack.
    #[inline(always)]
    fn irq_asserted(&self) -> bool {
        dispatch!(self, b => b.irq_asserted())
    }

    /// **`#[inline(always)]` since MF-6a step 2, and it has to stay.** `NesBus::tick` asks this
    /// of the cartridge once per CPU cycle on a board with expansion audio, through
    /// `Cartridge::expansion_audio_output` and the `expansion_audio` latch; the MMC5 is the one
    /// board of REQ-CORE-075 this build must sound, and `mmc5exram` is in lot PR-1's set.
    ///
    /// The thirty-seventh `Board` variant pushed this dispatch past the inliner's budget and
    /// LLVM gave it a symbol: **0 → 116 144 093 instructions over 300 frames of `mmc5exram`**,
    /// with `NesBus::push_expansion_audio` dropping 44 670 800 as its body left — a net
    /// **+75 937 647, +0.672 % on that ROM**, on a board this lot does not touch. It is the same
    /// story MF-5b step 1 tells about `irq_asserted` one dispatch along, and the same fix: the
    /// chain is pinned rather than shortened, because shortening a dispatch and pinning it have
    /// opposite beneficiaries (PERFORMANCE-HISTORY §5.10).
    #[allow(clippy::inline_always)] // Measured: MF-6a step 2, +75.9 M on an MMC5.
    #[inline(always)]
    fn expansion_audio_output(&self) -> Option<i32> {
        dispatch!(self, b => b.expansion_audio_output())
    }

    fn ppu_irq_horizon(&self) -> Option<i64> {
        dispatch!(self, b => b.ppu_irq_horizon())
    }

    fn prg_ram_enabled(&self) -> bool {
        dispatch!(self, b => b.prg_ram_enabled())
    }

    fn prg_ram_writable(&self) -> bool {
        dispatch!(self, b => b.prg_ram_writable())
    }

    /// Not on any read path since lot MF-CART: `Cartridge` asks this once per CPU write and
    /// caches the answer, where its predecessor `prg_ram_offset` was a dispatch inside
    /// `Cartridge::default_cpu_read`.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        dispatch!(self, b => b.prg_ram_window(memory))
    }

    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        dispatch!(self, b => b.rom_windows(memory))
    }

    /// The **one** dispatch `Cartridge::refresh_rom_windows` makes, and the reason
    /// [`Mapper::cpu_read_windows`] exists: inside an arm, `rom_windows`, `prg_ram_window` and
    /// `prg_ram_enabled` are that board's own monomorphised bodies and not three more jump
    /// tables (lot MF-CART step 4, which measured the three-dispatch shape at `mm2/core`
    /// +0.005 %).
    fn cpu_read_windows(&self, memory: &CartridgeMemory) -> CpuReadWindows {
        dispatch!(self, b => b.cpu_read_windows(memory))
    }

    /// `#[inline(always)]`, and it has to stay, for the reason [`Mapper::cpu_read_override`]
    /// carries one method below: this dispatch is on **every CPU read of an MMC5 game**, that
    /// board being the one that maps PRG-RAM into $8000-$DFFF so that
    /// `Cartridge::read_rom_window` cannot answer.
    ///
    /// Lot MF-5b step 1 is what pinned it. Adding a thirty-second `Board` variant pushed this
    /// dispatch past the inliner's budget: it left `Cartridge::default_cpu_read` (which went to
    /// **0** exclusive instructions, its body having moved here) and became a call from
    /// `Cartridge::try_cpu_read`, at **+216 245 924 instructions over 300 frames of
    /// `mmc5exram`** against +133 M in its caller and −283 M in `default_cpu_read` — a net
    /// +66 M, 0.58 % of that ROM, on a board the lot does not touch.
    #[allow(clippy::inline_always)] // Measured: MF-5b step 1, +66 M on an MMC5.
    #[inline(always)]
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        dispatch!(self, b => b.try_read_prg(memory, address))
    }

    /// `#[inline(always)]`, and it has to stay: the MMC5 maps PRG-**RAM** into `$8000`-`$DFFF`,
    /// so `Cartridge::read_rom_window` cannot answer and **every** CPU read of an MMC5 game
    /// reaches this dispatch — about one call per read cycle, where an MMC1 reaches it a few
    /// thousand times in 8.8 million reads. Letting it go out of line turns lot PR-1's −0.73 %
    /// on an MMC5 into **+0.59 %** and is the only thing that made the lot's arrangement lose on
    /// any ROM of its set (`docs/progress/PR-1.md`, step 3). No other board is affected either
    /// way; MF-3b rule 3's "the arm that matters is the empty one" does not apply here, because
    /// what is being priced is the call, not an arm.
    #[allow(clippy::inline_always)] // Measured: lot PR-1 step 3.
    #[inline(always)]
    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        dispatch!(self, b => b.cpu_read_override(memory, address))
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        dispatch!(self, b => b.cpu_peek_override(memory, address))
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        dispatch!(self, b => b.intercept_cpu_write(memory, address, value))
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        dispatch!(self, b => b.write_register(memory, address, value));
    }

    fn ppu_read(&mut self, memory: &mut CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        dispatch!(self, b => b.ppu_read(memory, address, fetch))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        dispatch!(self, b => b.ppu_peek(memory, address, fetch))
    }

    fn chr_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        dispatch!(self, b => b.chr_view(memory))
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        dispatch!(self, b => b.chr_peek_view(memory))
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        dispatch!(self, b => b.ppu_write(memory, address, value));
    }

    fn chr_latch_state(&self) -> i32 {
        dispatch!(self, b => b.chr_latch_state())
    }

    fn set_chr_latch_state(&mut self, state: i32) {
        dispatch!(self, b => b.set_chr_latch_state(state));
    }

    fn advance_chr_latch(&mut self, pattern_table: i32, tile: i32) {
        dispatch!(self, b => b.advance_chr_latch(pattern_table, tile));
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        dispatch!(self, b => b.on_ppu_address_bus(memory, address, clock));
    }

    fn on_ppu_control(&mut self, control: u8) {
        dispatch!(self, b => b.on_ppu_control(control));
    }

    fn on_ppu_scanline(&mut self, memory: &mut CartridgeMemory, scanline: i32, rendering: bool) {
        dispatch!(self, b => b.on_ppu_scanline(memory, scanline, rendering));
    }

    fn nametable_page(&self, table: i32) -> i32 {
        dispatch!(self, b => b.nametable_page(table))
    }

    /// **Exactly two boards may override [`Mapper::read_nametable`], and a third costs every
    /// game.** Lot MF-5a measured it: this dispatch is folded into
    /// `Ppu2C02::read_nametable_inner` behind its `page == SERVED_BY_CARTRIDGE` test, and with
    /// two distinct arm bodies — the MMC5's `ExRAM`/fill and the UNROM 512's four-screen
    /// CHR-RAM — LLVM answers "not me" with a bitset: `movl $imm, %r9d`, `btl %ecx, %r9d`,
    /// `jae`.
    ///
    /// Giving the VRC6 an arm of its own for the `N`-set CHR-ROM nametables made that a **jump
    /// table**, and the extra register pressure grew `read_nametable_inner`'s prologue from
    /// `pushq %rsi / subq $32` to `pushq %rsi / pushq %rbx / subq $40` — two instructions and
    /// 8 bytes of stack **on every nametable fetch of every board**. `#[inline(always)]` here
    /// and on `Cartridge::read_nametable` does not help; it only moves the boundary (132
    /// instructions → 83, then 80, then 116, never back to 132). Shrinking the new arm to one
    /// indexed load, the shape the UNROM 512's already has, does not help either: the trigger is
    /// the third distinct body, not its size.
    ///
    /// So the VRC6 does not override it, and `$B003`'s `N` is left as **Q-123**. A board that
    /// needs its own nametables from here on has to reuse one of the two existing arms, or the
    /// `SERVED_BY_CARTRIDGE` path needs redesigning so that the data source is a cached slice on
    /// `Cartridge` rather than an arm of this enum — which the MMC5's fill mode, being computed
    /// and not a slice, is what makes awkward.
    fn read_nametable(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        dispatch!(self, b => b.read_nametable(memory, address))
    }

    /// **Not a `dispatch!`, deliberately** (lot MF-LEG-2). The paragraph above is the reason:
    /// `read_nametable`'s dispatch holds exactly two distinct arm bodies and a third makes it a
    /// jump table whose register pressure costs *every* board two instructions per nametable
    /// fetch. So this one tests for the single board that overrides the method — the MMC5 with
    /// `board-mmc5-exattr-split` on — and hands everything else to `read_nametable` unchanged,
    /// which keeps that function and its bitset exactly as they are.
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn read_nametable_from_vram(&self, memory: &CartridgeMemory, address: u16, vram: &[u8]) -> u8 {
        match self {
            Board::Mmc5(board) => board.read_nametable_from_vram(memory, address, vram),
            _ => self.read_nametable(memory, address),
        }
    }

    /// The same shape and the same reason as [`Board::read_nametable_from_vram`]: one test for
    /// the MMC5, `None` for every other board.
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn nametable_write_offset(&self, address: u16) -> Option<usize> {
        match self {
            Board::Mmc5(board) => board.nametable_write_offset(address),
            _ => None,
        }
    }

    /// Likewise: the MMC5's split is the only subscriber, so the other boards do not reach a
    /// dispatch at all.
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn on_tile_row_start(&mut self) {
        if let Board::Mmc5(board) = self {
            board.on_tile_row_start();
        }
    }

    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        dispatch!(self, b => b.write_nametable(memory, address, value));
    }

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        dispatch!(self, b => b.on_cpu_cycle(memory));
    }
}
