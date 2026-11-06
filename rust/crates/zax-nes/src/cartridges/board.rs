//! The [`Board`] enum: one variant per mapper family, dispatching [`Mapper`] to it
//! (REQ-CORE-100). Created once at load time by the registry.

use super::mappers::{
    Action52, Action53, AxRom, BandaiFcg, Caltron6In1, CnRom, Codemasters, ColorDreams, CpRom,
    Daou306, Fme7, GxRom, IremG101, JalecoJ87, JalecoSs88006, Mapper, Mapper015, Mapper034,
    Mapper078, McAcc, Mmc1, Mmc2, Mmc3, Mmc5, Multicart225, Namco163, Namcot108, Nrom, Rambo1,
    UnRom512, UxRom, Vrc2Vrc4, Vrc6, Vrc7,
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
    Namcot108(Namcot108),
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
            Board::Namcot108($b) => $body,
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

    fn prg_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        dispatch!(self, b => b.prg_ram_offset(memory, address))
    }

    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        dispatch!(self, b => b.rom_windows(memory))
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

    fn write_nametable(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        dispatch!(self, b => b.write_nametable(memory, address, value));
    }

    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        dispatch!(self, b => b.on_cpu_cycle(memory));
    }
}
