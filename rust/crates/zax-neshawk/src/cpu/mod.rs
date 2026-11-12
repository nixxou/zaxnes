//! The MOS 6502X: NesHawk's CPU.
//!
//! Ported from BizHawk 2.11.1 `src/BizHawk.Emulation.Cores/CPUs/MOS 6502X/MOS6502X.cs` and
//! `IMOS6502XLink.cs`; the micro-operation machine is in [`execute`] and the table in [`uop`].
//! MIT; see `LICENSE-BizHawk`.
//!
//! # Why the field names look like this
//!
//! `rdy_freeze`, `alu_temp`, `mi`, `iflag_pending`, `branch_irq_hack`, `ea`, `H` — these are
//! `MOS6502X.cs`'s own names, kept so that this file can be read beside the C#. The lot's brief
//! asks for exactly that.
//!
//! # The link
//!
//! The C# takes the bus as a generic type parameter (`MOS6502X<TLink>`) whose argument is a
//! struct, which is a .NET monomorphisation trick for inlining. Rust gets the same effect from a
//! trait taken by `&mut` on each call, so [`Mos6502xLink`] is passed *into* `execute_one` rather
//! than stored: that also keeps the CPU free of a borrow on the machine that owns it, which is
//! what lets `NES` hold both without a cell.

pub(crate) mod execute;
pub(crate) mod uop;

use uop::{Uop, MICROCODE, VOP_FETCH1, VOP_RESET};

/// Everything the CPU needs of the machine around it.
///
/// `IMOS6502XLink.cs`. `NES.CpuLink` is the implementation NesHawk gives it; the code-data-logger
/// variants of `ReadMemory`/`PeekMemory` are a debugging facility of `EmuHawk`'s and are not part
/// of this port.
pub trait Mos6502xLink {
    /// A real read, with every side effect the bus has.
    fn read_memory(&mut self, address: u16) -> u8;

    /// The read a cycle performs for timing alone. On the NES bus this is the same read —
    /// `NES.CpuLink.DummyReadMemory` forwards straight to `ReadMemory` — and it is a separate
    /// method because the 6502X is shared with cores where it is not.
    fn dummy_read_memory(&mut self, address: u16) -> u8;

    /// A read with no side effects at all, for a debugger or a trace.
    fn peek_memory(&self, address: u16) -> u8;

    /// A write, with every side effect the bus has.
    fn write_memory(&mut self, address: u16, value: u8);

    /// Called only when the **first** byte of an instruction is fetched.
    fn on_exec_fetch(&mut self, address: u16);
}

/// The interrupt vectors.
pub const NMI_VECTOR: u16 = 0xFFFA;
/// The reset vector.
pub const RESET_VECTOR: u16 = 0xFFFC;
/// `BRK`'s vector, which is the IRQ vector.
pub const BRK_VECTOR: u16 = 0xFFFE;
/// The IRQ vector, which is `BRK`'s.
pub const IRQ_VECTOR: u16 = 0xFFFE;

/// `MOS6502X.cs`'s `TableNZ`: the N and Z bits a value implies, ready to be OR-ed into `P`
/// after the other six bits have been masked off with `0x7D`.
#[rustfmt::skip]
static TABLE_NZ: [u8; 256] = {
    let mut t = [0u8; 256];
    t[0] = 0x02;
    let mut i = 0x80;
    while i < 0x100 {
        t[i] = 0x80;
        i += 1;
    }
    t
};

/// The 6502, as NesHawk runs it: one micro-operation a cycle out of [`MICROCODE`].
///
/// The field names are `MOS6502X.cs`'s; the module documentation says why.
#[allow(non_snake_case)]
// The 6502 has three input lines and eight one-bit pieces of sequencer state, and they are
// independent: `rdy`, `nmi` and `irq` are driven by the machine, `rdy_freeze` by the current
// cycle, `iflag_pending` by `CLI`/`SEI`/`PLP`, `branch_irq_hack` by one branch quirk. A state
// machine would claim they vary together, and `Execute.cs` is the proof that they do not.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Mos6502x {
    // ==== CPU State ==== (the C#'s own banner)
    /// The accumulator.
    pub A: u8,
    /// The X index register.
    pub X: u8,
    /// The Y index register.
    pub Y: u8,
    /// The processor status register. Bit 5 is always set.
    pub P: u8,
    /// The program counter.
    pub PC: u16,
    /// The stack pointer.
    pub S: u8,

    /// The IRQ line, asserted by the machine.
    pub irq: bool,
    /// The NMI line, asserted by the machine. Cleared by the CPU when it takes the interrupt.
    pub nmi: bool,
    /// The RDY line. While it is low the CPU is frozen: a cycle still passes and
    /// `total_executed_cycles` still counts it, but no micro-operation completes.
    pub rdy: bool,

    /// The PPU cycle, for the trace only. The machine keeps it up to date.
    pub ext_ppu_cycle: i32,

    /// Cycles since the last reset. Counts frozen cycles too — the C# comment says so.
    pub total_executed_cycles: i64,

    /// Whether decimal mode works. `false` on the NES's 2A03, whose BCD is not wired up.
    pub bcd_enabled: bool,

    /// The current opcode, or one of the `VOP_*` pseudo-opcodes above 255.
    pub opcode: usize,
    pub(crate) opcode2: u8,
    pub(crate) opcode3: u8,

    /// The effective address the instruction is working on. `int` in the C#, and it holds a
    /// vector as well as an address, so not a `u16`.
    pub(crate) ea: i32,
    /// The ALU's scratch register.
    pub(crate) alu_temp: i32,
    /// The microcode index: which micro-operation of the current opcode runs next. Goes to
    /// `-1` in `Fetch1_Real`, so it is signed.
    pub mi: i32,
    /// The I flag has to be stored *after* it is tested, for `CLI` and `SEI`.
    pub(crate) iflag_pending: bool,
    /// Set when the CPU must be frozen: RDY was low when the cycle wanted to do work.
    pub rdy_freeze: bool,
    /// The scratch byte of the "unstable high byte" group of unofficial instructions.
    pub(crate) H: u8,
    /// The 16-bit address bus, which a DMC DMA's halt cycles read.
    pub address_bus: u16,

    /// Whether an interrupt condition has come up recently. The C# is unsure whether this is
    /// real; it is what makes `branch_irq_hack` work.
    pub(crate) interrupt_pending: bool,
    /// See `RelBranch_Stage3`.
    pub(crate) branch_irq_hack: bool,

    // the C#'s instruction-scratch fields, which outlive a single micro-operation
    pub(crate) value8: u8,
    pub(crate) temp8: u8,
    pub(crate) value16: u16,
    pub(crate) branch_taken: bool,
    pub(crate) my_iflag: bool,
    pub(crate) booltemp: bool,
    pub(crate) tempint: i32,
    pub(crate) lo: i32,
    pub(crate) hi: i32,

    /// The constant the unofficial `ANE` (`$8B`) mixes in. NesHawk leaves it `0xFF`.
    pub ane_constant: u8,
    /// The constant the unofficial `LXA` (`$AB`) mixes in. NesHawk leaves it `0xFF`.
    pub lxa_constant: u8,
}

impl Default for Mos6502x {
    fn default() -> Self {
        Self::new()
    }
}

impl Mos6502x {
    /// A CPU in the state the C# constructor leaves it: `Reset()` has run.
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Self {
            A: 0,
            X: 0,
            Y: 0,
            P: 0,
            PC: 0,
            S: 0,
            irq: false,
            nmi: false,
            rdy: true,
            ext_ppu_cycle: 0,
            total_executed_cycles: 0,
            bcd_enabled: true,
            opcode: 0,
            opcode2: 0,
            opcode3: 0,
            ea: 0,
            alu_temp: 0,
            mi: 0,
            iflag_pending: false,
            rdy_freeze: false,
            H: 0,
            address_bus: 0,
            interrupt_pending: false,
            branch_irq_hack: false,
            value8: 0,
            temp8: 0,
            value16: 0,
            branch_taken: false,
            my_iflag: false,
            booltemp: false,
            tempint: 0,
            lo: 0,
            hi: 0,
            ane_constant: 0xFF,
            lxa_constant: 0xFF,
        };
        cpu.reset();
        cpu
    }

    /// `MOS6502X.Reset`. Not the NES's RESET line — that is [`Self::nes_soft_reset`]; this is
    /// the state the object starts in.
    pub fn reset(&mut self) {
        self.A = 0;
        self.X = 0;
        self.Y = 0;
        self.P = 0x20; // 5th bit always set
        self.S = 0;
        self.PC = 0;
        self.total_executed_cycles = 0;
        self.mi = 0;
        self.opcode = VOP_RESET;
        self.iflag_pending = true;
        self.rdy = true;
    }

    /// `MOS6502X.NESSoftReset`: the RESET line, which runs the reset micro-program without
    /// clearing the registers.
    pub fn nes_soft_reset(&mut self) {
        self.opcode = VOP_RESET;
        self.mi = 0;
        self.iflag_pending = true;
        self.set_flag_i(true);
    }

    /// The SO pin.
    pub fn set_overflow(&mut self) {
        self.set_flag_v(true);
    }

    /// Whether the next micro-operation starts an instruction, which is when a trace may
    /// disassemble. `MOS6502X.AtStart`.
    #[must_use]
    pub fn at_start(&self) -> bool {
        self.opcode == VOP_FETCH1 || self.current_uop() >= Uop::End
    }

    /// `MOS6502X.AtInstructionStart`.
    #[must_use]
    pub fn at_instruction_start(&self) -> bool {
        self.current_uop() >= Uop::End
    }

    /// The micro-operation `mi` points at. `mi` is `-1` only inside `Fetch1_Real`, between the
    /// opcode fetch and the increment at the end of the cycle, and the C# never indexes the
    /// table then; a negative index here reads row 0's first micro-operation rather than
    /// panicking, because this crate may not panic.
    pub(crate) fn current_uop(&self) -> Uop {
        let row = MICROCODE[self.opcode];
        let index = usize::try_from(self.mi).unwrap_or(0);
        row.get(index).copied().unwrap_or(Uop::End)
    }

    // ------------------------------------------------------------------ flags

    /// Carry.
    #[must_use]
    pub const fn flag_c(&self) -> bool {
        (self.P & 0x01) != 0
    }
    /// Zero.
    #[must_use]
    pub const fn flag_z(&self) -> bool {
        (self.P & 0x02) != 0
    }
    /// Interrupt disable.
    #[must_use]
    pub const fn flag_i(&self) -> bool {
        (self.P & 0x04) != 0
    }
    /// Decimal mode.
    #[must_use]
    pub const fn flag_d(&self) -> bool {
        (self.P & 0x08) != 0
    }
    /// Break.
    #[must_use]
    pub const fn flag_b(&self) -> bool {
        (self.P & 0x10) != 0
    }
    /// The unused flag, which is always set.
    #[must_use]
    pub const fn flag_t(&self) -> bool {
        (self.P & 0x20) != 0
    }
    /// Overflow.
    #[must_use]
    pub const fn flag_v(&self) -> bool {
        (self.P & 0x40) != 0
    }
    /// Negative.
    #[must_use]
    pub const fn flag_n(&self) -> bool {
        (self.P & 0x80) != 0
    }

    pub(crate) fn set_flag_c(&mut self, value: bool) {
        self.set_p_bit(0x01, value);
    }
    pub(crate) fn set_flag_z(&mut self, value: bool) {
        self.set_p_bit(0x02, value);
    }
    /// The I flag, which the machine sets directly as well (`FlagI` is the one public setter in
    /// the C#).
    pub fn set_flag_i(&mut self, value: bool) {
        self.set_p_bit(0x04, value);
    }
    pub(crate) fn set_flag_d(&mut self, value: bool) {
        self.set_p_bit(0x08, value);
    }
    pub(crate) fn set_flag_b(&mut self, value: bool) {
        self.set_p_bit(0x10, value);
    }
    /// The unused flag. `PullP` and `PullP_NoInc` force it back to `true` after pulling `P`,
    /// which is the C#'s "force T always to remain true".
    pub(crate) fn set_flag_t(&mut self, value: bool) {
        self.set_p_bit(0x20, value);
    }
    pub(crate) fn set_flag_v(&mut self, value: bool) {
        self.set_p_bit(0x40, value);
    }
    pub(crate) fn set_flag_n(&mut self, value: bool) {
        self.set_p_bit(0x80, value);
    }

    fn set_p_bit(&mut self, mask: u8, value: bool) {
        self.P = (self.P & !mask) | if value { mask } else { 0 };
    }

    /// `P = (P & 0x7D) | TableNZ[value]`, the C#'s own idiom for "set N and Z from this byte".
    pub(crate) fn set_nz(&mut self, value: u8) {
        self.P = (self.P & 0x7D) | TABLE_NZ[value as usize];
    }

    /// Whether an interrupt is being requested: `MOS6502X.Interrupted`.
    pub(crate) fn interrupted(&self) -> bool {
        self.nmi || (self.irq && !self.flag_i())
    }
}
