//! The 6502's micro-operation machine: one micro-operation a cycle.
//!
//! Ported from BizHawk 2.11.1 `src/BizHawk.Emulation.Cores/CPUs/MOS 6502X/Execute.cs`
//! (everything but the `Uop` enum and the `Microcode` table, which are in
//! [`super::uop`]). MIT; see `LICENSE-BizHawk`.
//!
//! # The method names are the C#'s
//!
//! `Abs_RMW_Stage5_SLO`, `IndIdx_WRITE_Stage5_SHA`, `_Adc`, `NZ_A` — the brief asks for a port
//! that can be read line by line beside `Execute.cs`, and renaming 250 micro-operations would
//! defeat that. The `non_snake_case` lint is allowed for the module rather than per method.
//!
//! # How the C#'s idioms are translated
//!
//! | `Execute.cs` | here | why |
//! |---|---|---|
//! | `rdy_freeze = !RDY; if (RDY) {…}` | `if self.rdy_gate() {…}` | the pair is one idiom and appears ~150 times; the helper is exactly it |
//! | `_link.ReadMemory(a)` | `bus.read_memory(a)` | the link is passed in, not stored |
//! | `(ushort)ea` | `self.ea16()` | `ea` is `int` in the C# and holds a vector as well as an address |
//! | `(opcode3 << 8) + opcode2` | `self.abs_addr()` | the absolute-addressing operand |
//! | `S-- + 0x100` | `self.push_addr()` | post-decrement, so the write uses the *old* `S` |
//! | `PC++` | `self.pc_pp()` | post-increment, so the read uses the *old* `PC` |
//! | `x.Bit(n)` | `bit(x, n)` | `(x >> n) & 1 != 0` on a signed `int`, sign-extending as the C# does |
//!
//! Every C# cast of an `int` to `byte` or `ushort` is a truncation, which is what Rust's `as`
//! does on the same widths — and C#'s arithmetic here is `unchecked` throughout (the file says
//! so where it matters), so `wrapping_*` is used wherever a value could leave its type.
//!
//! # The recursion is the C#'s
//!
//! `Fetch1`, `End`, `End_ISpecial`, `End_SuppressInterrupt`, `IndIdx_READ_Stage5` and
//! `AbsIdx_READ_Stage4` all call `ExecuteOneRetry` again inside the same cycle, to run the
//! micro-operation they have just switched to without spending another cycle on it. That is
//! kept: it is bounded (an `End` reaches a `Fetch1` which reaches at most one more
//! micro-operation) and it is the mechanism by which an interrupt costs no extra cycle.

// The C# names, deliberately: see the module documentation.
#![allow(non_snake_case)]
// `ea`, `alu_temp` and `tempint` are the C#'s `int`, and the ports between `i32`, `u8` and `u16`
// are the C#'s own casts. Spelling each one `try_from(...).unwrap_or(...)` would both be noise
// and change the behaviour, since the C# truncates.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap
)]
// Each micro-operation is a method whose body is its C# counterpart's. Many are two lines and
// some are one; that is the shape of the reference and the reason the port is auditable.
#![allow(clippy::similar_names, clippy::too_many_lines)]
// `_Adc`, `_Cmp`, `_Ora`, `_Bit`… are `Execute.cs`'s own names for the ALU sub-operations, and
// the leading underscore is part of the name there rather than a "this is unused" marker.
#![allow(clippy::used_underscore_items)]
// `JSR` and `NOP` are the C#'s micro-operation names, not acronyms this port chose.
#![allow(clippy::upper_case_acronyms)]
// `(x >> n) & 1 != 0` is `BizHawk.Common.NumberExtensions.Bit`, which is what the C# calls and
// which `bit` below is. Clippy would rather see `x & 0x80 == 0x80`; that is the same test
// written so that it no longer looks like the reference.
#![allow(clippy::verbose_bit_mask)]

use super::uop::{
    Uop, VOP_FETCH1, VOP_FETCH1_NO_INTERRUPT, VOP_IRQ, VOP_NMI, VOP_RELATIVE_STUFF,
    VOP_RELATIVE_STUFF2,
};
use super::{Mos6502x, Mos6502xLink, BRK_VECTOR, IRQ_VECTOR, NMI_VECTOR, RESET_VECTOR};

/// `BizHawk.Common.NumberExtensions.Bit` on an `int`: `(value >> bit) & 1 != 0`.
///
/// The shift is arithmetic, as C#'s is on a signed `int`. `RelBranch_Stage4` tests bit 31 and
/// `_Axs` tests bit 8 of a value that may be negative, so this must not be a logical shift.
const fn bit(value: i32, index: u32) -> bool {
    (value >> index) & 1 != 0
}

impl Mos6502x {
    // ------------------------------------------------------------- the idioms

    /// `rdy_freeze = !RDY; if (RDY)` — the gate almost every micro-operation opens with.
    /// Returns whether the cycle may do its work.
    fn rdy_gate(&mut self) -> bool {
        self.rdy_freeze = !self.rdy;
        self.rdy
    }

    /// `PC++`: the old `PC`, with `PC` advanced.
    fn pc_pp(&mut self) -> u16 {
        let pc = self.PC;
        self.PC = self.PC.wrapping_add(1);
        pc
    }

    /// `S-- + 0x100`: the stack address to write, with `S` decremented.
    fn push_addr(&mut self) -> u16 {
        let s = self.S;
        self.S = self.S.wrapping_sub(1);
        0x100 | u16::from(s)
    }

    /// `S++ + 0x100`: the stack address to read, with `S` incremented.
    fn pull_addr(&mut self) -> u16 {
        let s = self.S;
        self.S = self.S.wrapping_add(1);
        0x100 | u16::from(s)
    }

    /// `S + 0x100`: the stack address, `S` untouched.
    fn stack_addr(&self) -> u16 {
        0x100 | u16::from(self.S)
    }

    /// `(ushort)ea`.
    fn ea16(&self) -> u16 {
        self.ea as u16
    }

    /// `(opcode3 << 8) + opcode2`, the absolute operand.
    fn abs_addr(&self) -> u16 {
        (u16::from(self.opcode3) << 8).wrapping_add(u16::from(self.opcode2))
    }

    /// `ea = (ushort)(ea + 0x100)`, the page-crossing fix-up.
    fn ea_add_page(&mut self) {
        self.ea = i32::from(self.ea16().wrapping_add(0x100));
    }

    /// `alu_temp = value8 = (byte)…`, which the C# writes as a chained assignment.
    fn set_alu_from_value8(&mut self) {
        self.alu_temp = i32::from(self.value8);
    }

    // ------------------------------------------------------- the cycle itself

    /// `MOS6502X.Execute(int cycles)`.
    pub fn execute(&mut self, bus: &mut impl Mos6502xLink, cycles: i32) {
        for _ in 0..cycles {
            self.execute_one(bus);
        }
    }

    /// `MOS6502X.ExecuteOne`: one CPU cycle.
    pub fn execute_one(&mut self, bus: &mut impl Mos6502xLink) {
        // total cycles now increments every time a cycle is called to accurately count during RDY
        self.total_executed_cycles += 1;
        self.interrupt_pending |= self.interrupted();
        self.rdy_freeze = false;

        self.execute_one_retry(bus);

        if !self.rdy_freeze {
            self.mi += 1;
        }

        if self.current_uop() == Uop::End {
            // If the next cycle is the start of a new instruction, the address bus needs to be
            // set to the PC now, so a DMC DMA's halt cycles don't use the wrong address bus
            // value.
            self.address_bus = self.PC;
        }
    }

    /// `MOS6502X.ExecuteOneRetry`: run the micro-operation `mi` points at.
    ///
    /// The C#'s `default:` arm throws. It cannot be reached: of the 253 micro-operations, the
    /// twenty listed in [`super::uop`] are named by no row of the table, and every one that *is*
    /// named has an arm here — which was checked mechanically against the table rather than
    /// assumed. This crate may not panic, so the unreachable arms do nothing.
    fn execute_one_retry(&mut self, bus: &mut impl Mos6502xLink) {
        match self.current_uop() {
            Uop::Fetch1 => self.Fetch1(bus),
            Uop::Fetch1_Real => self.Fetch1_Real(bus),
            Uop::Fetch2 => self.Fetch2(bus),
            Uop::Fetch3 => self.Fetch3(bus),
            Uop::FetchDummy => self.FetchDummy(bus),
            Uop::PushPCH => self.PushPCH(bus),
            Uop::PushPCL => self.PushPCL(bus),
            Uop::PushP_BRK => self.PushP_BRK(bus),
            Uop::PushP_IRQ => self.PushP_IRQ(bus),
            Uop::PushP_NMI => self.PushP_NMI(bus),
            Uop::PushP_Reset => self.PushP_Reset(bus),
            Uop::PushDummy => self.PushDummy(bus),
            Uop::FetchPCLVector => self.FetchPCLVector(bus),
            Uop::FetchPCHVector => self.FetchPCHVector(bus),
            Uop::Imp_INY => self.Imp_INY(bus),
            Uop::Imp_DEY => self.Imp_DEY(bus),
            Uop::Imp_INX => self.Imp_INX(bus),
            Uop::Imp_DEX => self.Imp_DEX(bus),
            Uop::NZ_A => self.NZ_A(),
            Uop::NZ_X => self.NZ_X(),
            Uop::NZ_Y => self.NZ_Y(),
            Uop::Imp_TSX => self.Imp_TSX(bus),
            Uop::Imp_TXS => self.Imp_TXS(bus),
            Uop::Imp_TAX => self.Imp_TAX(bus),
            Uop::Imp_TAY => self.Imp_TAY(bus),
            Uop::Imp_TYA => self.Imp_TYA(bus),
            Uop::Imp_TXA => self.Imp_TXA(bus),
            Uop::Imp_SEI => self.Imp_SEI(bus),
            Uop::Imp_CLI => self.Imp_CLI(bus),
            Uop::Imp_SEC => self.Imp_SEC(bus),
            Uop::Imp_CLC => self.Imp_CLC(bus),
            Uop::Imp_SED => self.Imp_SED(bus),
            Uop::Imp_CLD => self.Imp_CLD(bus),
            Uop::Imp_CLV => self.Imp_CLV(bus),
            Uop::Abs_WRITE_STA => self.Abs_WRITE_STA(bus),
            Uop::Abs_WRITE_STX => self.Abs_WRITE_STX(bus),
            Uop::Abs_WRITE_STY => self.Abs_WRITE_STY(bus),
            Uop::Abs_WRITE_SAX => self.Abs_WRITE_SAX(bus),
            Uop::ZP_WRITE_STA => self.ZP_WRITE_STA(bus),
            Uop::ZP_WRITE_STY => self.ZP_WRITE_STY(bus),
            Uop::ZP_WRITE_STX => self.ZP_WRITE_STX(bus),
            Uop::ZP_WRITE_SAX => self.ZP_WRITE_SAX(bus),
            Uop::IndIdx_Stage3 => self.IndIdx_Stage3(bus),
            Uop::IndIdx_Stage4 => self.IndIdx_Stage4(bus),
            Uop::IndIdx_WRITE_Stage5 => self.IndIdx_WRITE_Stage5(bus),
            Uop::IndIdx_WRITE_Stage5_SHA => self.IndIdx_WRITE_Stage5_SHA(bus),
            Uop::IndIdx_READ_Stage5 => self.IndIdx_READ_Stage5(bus),
            Uop::IndIdx_RMW_Stage5 => self.IndIdx_RMW_Stage5(bus),
            Uop::IndIdx_WRITE_Stage6_STA => self.IndIdx_WRITE_Stage6_STA(bus),
            Uop::IndIdx_WRITE_Stage6_SHA => self.IndIdx_WRITE_Stage6_SHA(bus),
            Uop::IndIdx_READ_Stage6_LDA => self.IndIdx_READ_Stage6_LDA(bus),
            Uop::IndIdx_READ_Stage6_CMP => self.IndIdx_READ_Stage6_CMP(bus),
            Uop::IndIdx_READ_Stage6_AND => self.IndIdx_READ_Stage6_AND(bus),
            Uop::IndIdx_READ_Stage6_EOR => self.IndIdx_READ_Stage6_EOR(bus),
            Uop::IndIdx_READ_Stage6_LAX => self.IndIdx_READ_Stage6_LAX(bus),
            Uop::IndIdx_READ_Stage6_ADC => self.IndIdx_READ_Stage6_ADC(bus),
            Uop::IndIdx_READ_Stage6_SBC => self.IndIdx_READ_Stage6_SBC(bus),
            Uop::IndIdx_READ_Stage6_ORA => self.IndIdx_READ_Stage6_ORA(bus),
            Uop::IndIdx_RMW_Stage6 => self.IndIdx_RMW_Stage6(bus),
            Uop::IndIdx_RMW_Stage7_SLO => self.IndIdx_RMW_Stage7_SLO(bus),
            Uop::IndIdx_RMW_Stage7_SRE => self.IndIdx_RMW_Stage7_SRE(bus),
            Uop::IndIdx_RMW_Stage7_RRA => self.IndIdx_RMW_Stage7_RRA(bus),
            Uop::IndIdx_RMW_Stage7_ISC => self.IndIdx_RMW_Stage7_ISC(bus),
            Uop::IndIdx_RMW_Stage7_DCP => self.IndIdx_RMW_Stage7_DCP(bus),
            Uop::IndIdx_RMW_Stage7_RLA => self.IndIdx_RMW_Stage7_RLA(bus),
            Uop::IndIdx_RMW_Stage8 => self.IndIdx_RMW_Stage8(bus),
            Uop::RelBranch_Stage2_BVS => self.RelBranch_Stage2_BVS(bus),
            Uop::RelBranch_Stage2_BVC => self.RelBranch_Stage2_BVC(bus),
            Uop::RelBranch_Stage2_BMI => self.RelBranch_Stage2_BMI(bus),
            Uop::RelBranch_Stage2_BPL => self.RelBranch_Stage2_BPL(bus),
            Uop::RelBranch_Stage2_BCS => self.RelBranch_Stage2_BCS(bus),
            Uop::RelBranch_Stage2_BCC => self.RelBranch_Stage2_BCC(bus),
            Uop::RelBranch_Stage2_BEQ => self.RelBranch_Stage2_BEQ(bus),
            Uop::RelBranch_Stage2_BNE => self.RelBranch_Stage2_BNE(bus),
            Uop::RelBranch_Stage2 => self.RelBranch_Stage2(bus),
            Uop::RelBranch_Stage3 => self.RelBranch_Stage3(bus),
            Uop::RelBranch_Stage4 => self.RelBranch_Stage4(bus),
            Uop::NOP => self.NOP(bus),
            Uop::DecS => self.DecS(bus),
            Uop::IncS => self.IncS(bus),
            Uop::JSR => self.JSR(bus),
            Uop::PullP => self.PullP(bus),
            Uop::PullPCL => self.PullPCL(bus),
            Uop::PullPCH_NoInc => self.PullPCH_NoInc(bus),
            Uop::Abs_READ_LDA => self.Abs_READ_LDA(bus),
            Uop::Abs_READ_LDY => self.Abs_READ_LDY(bus),
            Uop::Abs_READ_LDX => self.Abs_READ_LDX(bus),
            Uop::Abs_READ_BIT => self.Abs_READ_BIT(bus),
            Uop::Abs_READ_LAX => self.Abs_READ_LAX(bus),
            Uop::Abs_READ_AND => self.Abs_READ_AND(bus),
            Uop::Abs_READ_EOR => self.Abs_READ_EOR(bus),
            Uop::Abs_READ_ORA => self.Abs_READ_ORA(bus),
            Uop::Abs_READ_ADC => self.Abs_READ_ADC(bus),
            Uop::Abs_READ_CMP => self.Abs_READ_CMP(bus),
            Uop::Abs_READ_CPY => self.Abs_READ_CPY(bus),
            Uop::Abs_READ_NOP => self.Abs_READ_NOP(bus),
            Uop::Abs_READ_CPX => self.Abs_READ_CPX(bus),
            Uop::Abs_READ_SBC => self.Abs_READ_SBC(bus),
            Uop::ZpIdx_Stage3_X => self.ZpIdx_Stage3_X(bus),
            Uop::ZpIdx_Stage3_Y => self.ZpIdx_Stage3_Y(bus),
            Uop::ZpIdx_RMW_Stage4 => self.ZpIdx_RMW_Stage4(bus),
            Uop::ZpIdx_RMW_Stage6 => self.ZpIdx_RMW_Stage6(bus),
            Uop::ZP_READ_EOR => self.ZP_READ_EOR(bus),
            Uop::ZP_READ_BIT => self.ZP_READ_BIT(bus),
            Uop::ZP_READ_LDA => self.ZP_READ_LDA(bus),
            Uop::ZP_READ_LDY => self.ZP_READ_LDY(bus),
            Uop::ZP_READ_LDX => self.ZP_READ_LDX(bus),
            Uop::ZP_READ_LAX => self.ZP_READ_LAX(bus),
            Uop::ZP_READ_CPY => self.ZP_READ_CPY(bus),
            Uop::ZP_READ_CMP => self.ZP_READ_CMP(bus),
            Uop::ZP_READ_CPX => self.ZP_READ_CPX(bus),
            Uop::ZP_READ_ORA => self.ZP_READ_ORA(bus),
            Uop::ZP_READ_NOP => self.ZP_READ_NOP(bus),
            Uop::ZP_READ_SBC => self.ZP_READ_SBC(bus),
            Uop::ZP_READ_ADC => self.ZP_READ_ADC(bus),
            Uop::ZP_READ_AND => self.ZP_READ_AND(bus),
            Uop::_Cpx => self._Cpx(),
            Uop::_Cpy => self._Cpy(),
            Uop::_Cmp => self._Cmp(),
            Uop::_Eor => self._Eor(),
            Uop::_And => self._And(),
            Uop::_Ora => self._Ora(),
            Uop::_Anc => self._Anc(),
            Uop::_Asr => self._Asr(),
            Uop::_Axs => self._Axs(),
            Uop::_Arr => self._Arr(),
            Uop::_Lxa => self._Lxa(),
            Uop::_Sbc => self._Sbc(),
            Uop::_Adc => self._Adc(),
            Uop::Imm_EOR => self.Imm_EOR(bus),
            Uop::Imm_ANC => self.Imm_ANC(bus),
            Uop::Imm_ASR => self.Imm_ASR(bus),
            Uop::Imm_AXS => self.Imm_AXS(bus),
            Uop::Imm_ARR => self.Imm_ARR(bus),
            Uop::Imm_LXA => self.Imm_LXA(bus),
            Uop::Imm_ORA => self.Imm_ORA(bus),
            Uop::Imm_CPY => self.Imm_CPY(bus),
            Uop::Imm_CPX => self.Imm_CPX(bus),
            Uop::Imm_CMP => self.Imm_CMP(bus),
            Uop::Imm_SBC => self.Imm_SBC(bus),
            Uop::Imm_AND => self.Imm_AND(bus),
            Uop::Imm_ADC => self.Imm_ADC(bus),
            Uop::Imm_LDA => self.Imm_LDA(bus),
            Uop::Imm_LDX => self.Imm_LDX(bus),
            Uop::Imm_LDY => self.Imm_LDY(bus),
            Uop::Imm_ANE => self.Imm_ANE(bus),
            Uop::Imm_Unsupported => self.Imm_Unsupported(bus),
            Uop::IdxInd_Stage3 => self.IdxInd_Stage3(bus),
            Uop::IdxInd_Stage4 => self.IdxInd_Stage4(bus),
            Uop::IdxInd_Stage5 => self.IdxInd_Stage5(bus),
            Uop::IdxInd_Stage6_READ_LDA => self.IdxInd_Stage6_READ_LDA(bus),
            Uop::IdxInd_Stage6_READ_ORA => self.IdxInd_Stage6_READ_ORA(bus),
            Uop::IdxInd_Stage6_READ_LAX => self.IdxInd_Stage6_READ_LAX(bus),
            Uop::IdxInd_Stage6_READ_CMP => self.IdxInd_Stage6_READ_CMP(bus),
            Uop::IdxInd_Stage6_READ_ADC => self.IdxInd_Stage6_READ_ADC(bus),
            Uop::IdxInd_Stage6_READ_AND => self.IdxInd_Stage6_READ_AND(bus),
            Uop::IdxInd_Stage6_READ_EOR => self.IdxInd_Stage6_READ_EOR(bus),
            Uop::IdxInd_Stage6_READ_SBC => self.IdxInd_Stage6_READ_SBC(bus),
            Uop::IdxInd_Stage6_WRITE_STA => self.IdxInd_Stage6_WRITE_STA(bus),
            Uop::IdxInd_Stage6_WRITE_SAX => self.IdxInd_Stage6_WRITE_SAX(bus),
            Uop::IdxInd_Stage6_RMW => self.IdxInd_Stage6_RMW(bus),
            Uop::IdxInd_Stage7_RMW_SLO => self.IdxInd_Stage7_RMW_SLO(bus),
            Uop::IdxInd_Stage7_RMW_ISC => self.IdxInd_Stage7_RMW_ISC(bus),
            Uop::IdxInd_Stage7_RMW_DCP => self.IdxInd_Stage7_RMW_DCP(bus),
            Uop::IdxInd_Stage7_RMW_SRE => self.IdxInd_Stage7_RMW_SRE(bus),
            Uop::IdxInd_Stage7_RMW_RRA => self.IdxInd_Stage7_RMW_RRA(bus),
            Uop::IdxInd_Stage7_RMW_RLA => self.IdxInd_Stage7_RMW_RLA(bus),
            Uop::IdxInd_Stage8_RMW => self.IdxInd_Stage8_RMW(bus),
            Uop::PushP => self.PushP(bus),
            Uop::PushA => self.PushA(bus),
            Uop::PullA_NoInc => self.PullA_NoInc(bus),
            Uop::PullP_NoInc => self.PullP_NoInc(bus),
            Uop::Imp_ASL_A => self.Imp_ASL_A(bus),
            Uop::Imp_ROL_A => self.Imp_ROL_A(bus),
            Uop::Imp_ROR_A => self.Imp_ROR_A(bus),
            Uop::Imp_LSR_A => self.Imp_LSR_A(bus),
            Uop::JMP_abs => self.JMP_abs(bus),
            Uop::IncPC => self.IncPC(bus),
            Uop::ZP_RMW_Stage3 => self.ZP_RMW_Stage3(bus),
            Uop::ZP_RMW_Stage5 => self.ZP_RMW_Stage5(bus),
            Uop::ZP_RMW_INC => self.ZP_RMW_INC(bus),
            Uop::ZP_RMW_DEC => self.ZP_RMW_DEC(bus),
            Uop::ZP_RMW_ASL => self.ZP_RMW_ASL(bus),
            Uop::ZP_RMW_SRE => self.ZP_RMW_SRE(bus),
            Uop::ZP_RMW_RRA => self.ZP_RMW_RRA(bus),
            Uop::ZP_RMW_DCP => self.ZP_RMW_DCP(bus),
            Uop::ZP_RMW_LSR => self.ZP_RMW_LSR(bus),
            Uop::ZP_RMW_ROR => self.ZP_RMW_ROR(bus),
            Uop::ZP_RMW_ROL => self.ZP_RMW_ROL(bus),
            Uop::ZP_RMW_SLO => self.ZP_RMW_SLO(bus),
            Uop::ZP_RMW_ISC => self.ZP_RMW_ISC(bus),
            Uop::ZP_RMW_RLA => self.ZP_RMW_RLA(bus),
            Uop::AbsIdx_Stage3_Y => self.AbsIdx_Stage3_Y(bus),
            Uop::AbsIdx_Stage3_X => self.AbsIdx_Stage3_X(bus),
            Uop::AbsIdx_READ_Stage4 => self.AbsIdx_READ_Stage4(bus),
            Uop::AbsIdx_Stage4 => self.AbsIdx_Stage4(bus),
            Uop::AbsIdx_Stage4_SHX => self.AbsIdx_Stage4_SHX(bus),
            Uop::AbsIdx_Stage4_SHY => self.AbsIdx_Stage4_SHY(bus),
            Uop::AbsIdx_Stage4_SHA => self.AbsIdx_Stage4_SHA(bus),
            Uop::AbsIdx_Stage4_SHS => self.AbsIdx_Stage4_SHS(bus),
            Uop::AbsIdx_WRITE_Stage5_STA => self.AbsIdx_WRITE_Stage5_STA(bus),
            Uop::AbsIdx_WRITE_Stage5_SHY => self.AbsIdx_WRITE_Stage5_SHY(bus),
            Uop::AbsIdx_WRITE_Stage5_SHX => self.AbsIdx_WRITE_Stage5_SHX(bus),
            Uop::AbsIdx_WRITE_Stage5_SHA => self.AbsIdx_WRITE_Stage5_SHA(bus),
            Uop::AbsIdx_WRITE_Stage5_SHS => self.AbsIdx_WRITE_Stage5_SHS(bus),
            Uop::AbsIdx_RMW_Stage5 => self.AbsIdx_RMW_Stage5(bus),
            Uop::AbsIdx_RMW_Stage7 => self.AbsIdx_RMW_Stage7(bus),
            Uop::AbsIdx_RMW_Stage6_DEC => self.AbsIdx_RMW_Stage6_DEC(bus),
            Uop::AbsIdx_RMW_Stage6_DCP => self.AbsIdx_RMW_Stage6_DCP(bus),
            Uop::AbsIdx_RMW_Stage6_ISC => self.AbsIdx_RMW_Stage6_ISC(bus),
            Uop::AbsIdx_RMW_Stage6_INC => self.AbsIdx_RMW_Stage6_INC(bus),
            Uop::AbsIdx_RMW_Stage6_ROL => self.AbsIdx_RMW_Stage6_ROL(bus),
            Uop::AbsIdx_RMW_Stage6_LSR => self.AbsIdx_RMW_Stage6_LSR(bus),
            Uop::AbsIdx_RMW_Stage6_SLO => self.AbsIdx_RMW_Stage6_SLO(bus),
            Uop::AbsIdx_RMW_Stage6_SRE => self.AbsIdx_RMW_Stage6_SRE(bus),
            Uop::AbsIdx_RMW_Stage6_RRA => self.AbsIdx_RMW_Stage6_RRA(bus),
            Uop::AbsIdx_RMW_Stage6_RLA => self.AbsIdx_RMW_Stage6_RLA(bus),
            Uop::AbsIdx_RMW_Stage6_ASL => self.AbsIdx_RMW_Stage6_ASL(bus),
            Uop::AbsIdx_RMW_Stage6_ROR => self.AbsIdx_RMW_Stage6_ROR(bus),
            Uop::AbsIdx_READ_Stage5_LDA => self.AbsIdx_READ_Stage5_LDA(bus),
            Uop::AbsIdx_READ_Stage5_LDX => self.AbsIdx_READ_Stage5_LDX(bus),
            Uop::AbsIdx_READ_Stage5_LAX => self.AbsIdx_READ_Stage5_LAX(bus),
            Uop::AbsIdx_READ_Stage5_LDY => self.AbsIdx_READ_Stage5_LDY(bus),
            Uop::AbsIdx_READ_Stage5_ORA => self.AbsIdx_READ_Stage5_ORA(bus),
            Uop::AbsIdx_READ_Stage5_NOP => self.AbsIdx_READ_Stage5_NOP(bus),
            Uop::AbsIdx_READ_Stage5_CMP => self.AbsIdx_READ_Stage5_CMP(bus),
            Uop::AbsIdx_READ_Stage5_SBC => self.AbsIdx_READ_Stage5_SBC(bus),
            Uop::AbsIdx_READ_Stage5_ADC => self.AbsIdx_READ_Stage5_ADC(bus),
            Uop::AbsIdx_READ_Stage5_EOR => self.AbsIdx_READ_Stage5_EOR(bus),
            Uop::AbsIdx_READ_Stage5_AND => self.AbsIdx_READ_Stage5_AND(bus),
            Uop::AbsIdx_READ_Stage5_ERROR => self.AbsIdx_READ_Stage5_ERROR(bus),
            Uop::AbsInd_JMP_Stage4 => self.AbsInd_JMP_Stage4(bus),
            Uop::AbsInd_JMP_Stage5 => self.AbsInd_JMP_Stage5(bus),
            Uop::Abs_RMW_Stage4 => self.Abs_RMW_Stage4(bus),
            Uop::Abs_RMW_Stage5_INC => self.Abs_RMW_Stage5_INC(bus),
            Uop::Abs_RMW_Stage5_DEC => self.Abs_RMW_Stage5_DEC(bus),
            Uop::Abs_RMW_Stage5_DCP => self.Abs_RMW_Stage5_DCP(bus),
            Uop::Abs_RMW_Stage5_ISC => self.Abs_RMW_Stage5_ISC(bus),
            Uop::Abs_RMW_Stage5_ASL => self.Abs_RMW_Stage5_ASL(bus),
            Uop::Abs_RMW_Stage5_ROR => self.Abs_RMW_Stage5_ROR(bus),
            Uop::Abs_RMW_Stage5_SLO => self.Abs_RMW_Stage5_SLO(bus),
            Uop::Abs_RMW_Stage5_RLA => self.Abs_RMW_Stage5_RLA(bus),
            Uop::Abs_RMW_Stage5_SRE => self.Abs_RMW_Stage5_SRE(bus),
            Uop::Abs_RMW_Stage5_RRA => self.Abs_RMW_Stage5_RRA(bus),
            Uop::Abs_RMW_Stage5_ROL => self.Abs_RMW_Stage5_ROL(bus),
            Uop::Abs_RMW_Stage5_LSR => self.Abs_RMW_Stage5_LSR(bus),
            Uop::Abs_RMW_Stage6 => self.Abs_RMW_Stage6(bus),
            Uop::End_ISpecial => self.End_ISpecial(bus),
            Uop::End_SuppressInterrupt => self.End_SuppressInterrupt(bus),
            Uop::End => self.End(bus),
            Uop::Jam => self.Jam(),
            // `Unsupported`, `_Bit` and `RelBranch_Stage2`'s unreferenced siblings: no row of
            // the table names them, so the C#'s `default: throw` is unreachable too.
            Uop::Unsupported | Uop::_Bit => {}
        }
    }

    // ------------------------------------------------------------ fetch cycle

    fn FetchDummy(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
        }
    }

    fn Fetch1(&mut self, bus: &mut impl Mos6502xLink) {
        self.my_iflag = self.flag_i();
        self.set_flag_i(self.iflag_pending);
        if !self.branch_irq_hack {
            self.interrupt_pending = false;
            if self.nmi {
                self.ea = i32::from(NMI_VECTOR);
                self.opcode = VOP_NMI;
                self.nmi = false;
                self.mi = 0;
                self.execute_one_retry(bus);
                return;
            }

            if self.irq && !self.my_iflag {
                self.ea = i32::from(IRQ_VECTOR);
                self.opcode = VOP_IRQ;
                self.mi = 0;
                self.execute_one_retry(bus);
                return;
            }
        }

        self.mi = 0;
        self.opcode = VOP_FETCH1_NO_INTERRUPT;

        self.Fetch1_Real(bus);
    }

    fn Fetch1_Real(&mut self, bus: &mut impl Mos6502xLink) {
        self.rdy_freeze = !self.rdy;
        if self.rdy {
            self.branch_irq_hack = false;
            bus.on_exec_fetch(self.PC);
            let pc = self.pc_pp();
            self.opcode = usize::from(bus.read_memory(pc));
            self.mi = -1;
        }
        self.address_bus = self.PC;
    }

    fn Fetch2(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.opcode2 = bus.read_memory(pc);
        }
    }

    fn Fetch3(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.opcode3 = bus.read_memory(pc);
            self.address_bus = (u16::from(self.opcode3) << 8) | u16::from(self.opcode2);
        }
    }

    // ------------------------------------------------- the stack and vectors

    fn PushPCH(&mut self, bus: &mut impl Mos6502xLink) {
        let addr = self.push_addr();
        bus.write_memory(addr, (self.PC >> 8) as u8);
    }

    fn PushPCL(&mut self, bus: &mut impl Mos6502xLink) {
        let addr = self.push_addr();
        bus.write_memory(addr, self.PC as u8);
    }

    fn PushP_BRK(&mut self, bus: &mut impl Mos6502xLink) {
        self.set_flag_b(true);
        let addr = self.push_addr();
        bus.write_memory(addr, self.P);
        self.set_flag_i(true);
        self.ea = i32::from(BRK_VECTOR);
    }

    fn PushP_IRQ(&mut self, bus: &mut impl Mos6502xLink) {
        self.set_flag_b(false);
        let addr = self.push_addr();
        bus.write_memory(addr, self.P);
        self.set_flag_i(true);
        self.ea = i32::from(IRQ_VECTOR);
    }

    fn PushP_NMI(&mut self, bus: &mut impl Mos6502xLink) {
        self.set_flag_b(false);
        let addr = self.push_addr();
        bus.write_memory(addr, self.P);
        self.set_flag_i(true); // is this right?
        self.ea = i32::from(NMI_VECTOR);
    }

    fn PushP_Reset(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.ea = i32::from(RESET_VECTOR);
            let addr = self.push_addr();
            bus.dummy_read_memory(addr);
            self.set_flag_i(true);
        }
    }

    fn PushDummy(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let addr = self.push_addr();
            bus.dummy_read_memory(addr);
        }
    }

    fn FetchPCLVector(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            if self.ea == i32::from(BRK_VECTOR) && self.flag_b() && self.nmi {
                self.nmi = false;
                self.ea = i32::from(NMI_VECTOR);
            }
            if self.ea == i32::from(IRQ_VECTOR) && !self.flag_b() && self.nmi {
                self.nmi = false;
                self.ea = i32::from(NMI_VECTOR);
            }
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
        }
    }

    fn FetchPCHVector(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let addr = (self.ea + 1) as u16;
            self.alu_temp += i32::from(bus.read_memory(addr)) << 8;
            self.PC = self.alu_temp as u16;
        }
    }

    // --------------------------------------------- implied and accumulator

    fn Imp_INY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.Y = self.Y.wrapping_add(1);
            self.NZ_Y();
        }
    }

    fn Imp_DEY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.Y = self.Y.wrapping_sub(1);
            self.NZ_Y();
        }
    }

    fn Imp_INX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.X = self.X.wrapping_add(1);
            self.NZ_X();
        }
    }

    fn Imp_DEX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.X = self.X.wrapping_sub(1);
            self.NZ_X();
        }
    }

    fn NZ_A(&mut self) {
        self.set_nz(self.A);
    }

    fn NZ_X(&mut self) {
        self.set_nz(self.X);
    }

    fn NZ_Y(&mut self) {
        self.set_nz(self.Y);
    }

    fn Imp_TSX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.X = self.S;
            self.NZ_X();
        }
    }

    fn Imp_TXS(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.S = self.X;
        }
    }

    fn Imp_TAX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.X = self.A;
            self.NZ_X();
        }
    }

    fn Imp_TAY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.Y = self.A;
            self.NZ_Y();
        }
    }

    fn Imp_TYA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.A = self.Y;
            self.NZ_A();
        }
    }

    fn Imp_TXA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.A = self.X;
            self.NZ_A();
        }
    }

    fn Imp_SEI(&mut self, bus: &mut impl Mos6502xLink) {
        // not affected by RDY
        self.iflag_pending = true;

        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
        }
    }

    fn Imp_CLI(&mut self, bus: &mut impl Mos6502xLink) {
        // not affected by RDY
        self.iflag_pending = false;

        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
        }
    }

    fn Imp_SEC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.set_flag_c(true);
        }
    }

    fn Imp_CLC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.set_flag_c(false);
        }
    }

    fn Imp_SED(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.set_flag_d(true);
        }
    }

    fn Imp_CLD(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.set_flag_d(false);
        }
    }

    fn Imp_CLV(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.set_flag_v(false);
        }
    }

    fn Imp_ASL_A(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.set_flag_c((self.A & 0x80) != 0);
            self.A <<= 1;
            self.NZ_A();
        }
    }

    fn Imp_ROL_A(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.temp8 = self.A;
            self.A = (self.A << 1) | (self.P & 1);
            self.set_flag_c((self.temp8 & 0x80) != 0);
            self.NZ_A();
        }
    }

    fn Imp_ROR_A(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.temp8 = self.A;
            self.A = (self.A >> 1) | ((self.P & 1) << 7);
            self.set_flag_c((self.temp8 & 1) != 0);
            self.NZ_A();
        }
    }

    fn Imp_LSR_A(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.set_flag_c((self.A & 1) != 0);
            self.A >>= 1;
            self.NZ_A();
        }
    }

    // --------------------------------------------------- absolute addressing

    fn Abs_WRITE_STA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.abs_addr(), self.A);
    }

    fn Abs_WRITE_STX(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.abs_addr(), self.X);
    }

    fn Abs_WRITE_STY(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.abs_addr(), self.Y);
    }

    fn Abs_WRITE_SAX(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.abs_addr(), self.X & self.A);
    }

    fn Abs_READ_LDA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.A = bus.read_memory(self.abs_addr());
            self.NZ_A();
        }
    }

    fn Abs_READ_LDY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.Y = bus.read_memory(self.abs_addr());
            self.NZ_Y();
        }
    }

    fn Abs_READ_LDX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.X = bus.read_memory(self.abs_addr());
            self.NZ_X();
        }
    }

    fn Abs_READ_BIT(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._Bit();
        }
    }

    /// The C# reads the operand **twice** here, which is a quirk of this micro-operation rather
    /// than of the instruction, and is kept.
    fn Abs_READ_LAX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self.A = bus.read_memory(self.abs_addr());
            self.X = self.A;
            self.NZ_A();
        }
    }

    fn Abs_READ_AND(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._And();
        }
    }

    fn Abs_READ_EOR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._Eor();
        }
    }

    fn Abs_READ_ORA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._Ora();
        }
    }

    fn Abs_READ_ADC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._Adc();
        }
    }

    fn Abs_READ_CMP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._Cmp();
        }
    }

    fn Abs_READ_CPY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._Cpy();
        }
    }

    fn Abs_READ_NOP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
        }
    }

    fn Abs_READ_CPX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._Cpx();
        }
    }

    fn Abs_READ_SBC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.abs_addr()));
            self._Sbc();
        }
    }

    fn Abs_RMW_Stage4(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.ea = i32::from(self.abs_addr());
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
        }
    }

    fn Abs_RMW_Stage5_INC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp + 1) as u8;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn Abs_RMW_Stage5_DEC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp - 1) as u8;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn Abs_RMW_Stage5_DCP(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp - 1) as u8;
        self.set_alu_from_value8();
        self._Cmp();
    }

    fn Abs_RMW_Stage5_ISC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp + 1) as u8;
        self.set_alu_from_value8();
        self._Sbc();
    }

    fn Abs_RMW_Stage5_ASL(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 0x80) != 0);
        self.value8 <<= 1;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn Abs_RMW_Stage5_ROR(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 >> 1) | ((self.P & 1) << 7);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self.set_nz(self.value8);
    }

    fn Abs_RMW_Stage5_SLO(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 0x80) != 0);
        self.value8 <<= 1;
        self.set_alu_from_value8();
        self.A |= self.value8;
        self.NZ_A();
    }

    fn Abs_RMW_Stage5_RLA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 << 1) | (self.P & 1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 0x80) != 0);
        self.A &= self.value8;
        self.NZ_A();
    }

    fn Abs_RMW_Stage5_SRE(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 1) != 0);
        self.value8 >>= 1;
        self.set_alu_from_value8();
        self.A ^= self.value8;
        self.NZ_A();
    }

    fn Abs_RMW_Stage5_RRA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 >> 1) | ((self.P & 1) << 7);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self._Adc();
    }

    fn Abs_RMW_Stage5_ROL(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 << 1) | (self.P & 1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 0x80) != 0);
        self.set_nz(self.value8);
    }

    fn Abs_RMW_Stage5_LSR(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 1) != 0);
        self.value8 >>= 1;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn Abs_RMW_Stage6(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
    }

    fn JMP_abs(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.PC =
                (u16::from(bus.read_memory(self.PC)) << 8).wrapping_add(u16::from(self.opcode2));
        }
    }

    fn AbsInd_JMP_Stage4(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.ea = i32::from(self.abs_addr());
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
        }
    }

    fn AbsInd_JMP_Stage5(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.ea = (i32::from(self.opcode3) << 8) + i32::from(self.opcode2.wrapping_add(1));
            self.alu_temp += i32::from(bus.read_memory(self.ea16())) << 8;
            self.PC = self.alu_temp as u16;
        }
    }

    // ------------------------------------------------------------- zero page

    fn ZP_WRITE_STA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.A);
    }

    fn ZP_WRITE_STY(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.Y);
    }

    fn ZP_WRITE_STX(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.X);
    }

    fn ZP_WRITE_SAX(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.X & self.A);
    }

    /// A bit sneaky to shove this into `opcode2`… but we can reuse all the zero page uops if we
    /// do that. (The C#'s own comment.)
    fn ZpIdx_Stage3_X(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.read_memory(u16::from(self.opcode2));
            self.opcode2 = self.opcode2.wrapping_add(self.X);
        }
    }

    fn ZpIdx_Stage3_Y(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.read_memory(u16::from(self.opcode2));
            self.opcode2 = self.opcode2.wrapping_add(self.Y);
        }
    }

    fn ZpIdx_RMW_Stage4(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
        }
    }

    fn ZpIdx_RMW_Stage6(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
    }

    fn ZP_READ_EOR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._Eor();
        }
    }

    fn ZP_READ_BIT(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._Bit();
        }
    }

    fn ZP_READ_LDA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.A = bus.read_memory(u16::from(self.opcode2));
            self.NZ_A();
        }
    }

    fn ZP_READ_LDY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.Y = bus.read_memory(u16::from(self.opcode2));
            self.NZ_Y();
        }
    }

    fn ZP_READ_LDX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.X = bus.read_memory(u16::from(self.opcode2));
            self.NZ_X();
        }
    }

    fn ZP_READ_LAX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            // ?? is this right?? (the C#'s own comment)
            self.X = bus.read_memory(u16::from(self.opcode2));
            self.A = self.X;
            self.NZ_A();
        }
    }

    fn ZP_READ_CPY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._Cpy();
        }
    }

    fn ZP_READ_CMP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._Cmp();
        }
    }

    fn ZP_READ_CPX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._Cpx();
        }
    }

    fn ZP_READ_ORA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._Ora();
        }
    }

    fn ZP_READ_NOP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.read_memory(u16::from(self.opcode2)); // just a dummy
        }
    }

    fn ZP_READ_SBC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._Sbc();
        }
    }

    fn ZP_READ_ADC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._Adc();
        }
    }

    fn ZP_READ_AND(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
            self._And();
        }
    }

    fn ZP_RMW_Stage3(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(u16::from(self.opcode2)));
        }
    }

    fn ZP_RMW_Stage5(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
    }

    fn ZP_RMW_INC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.alu_temp = i32::from((self.alu_temp + 1) as u8);
        self.set_nz(self.alu_temp as u8);
    }

    fn ZP_RMW_DEC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.alu_temp = i32::from((self.alu_temp - 1) as u8);
        self.set_nz(self.alu_temp as u8);
    }

    fn ZP_RMW_ASL(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 0x80) != 0);
        self.value8 <<= 1;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn ZP_RMW_SRE(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 1) != 0);
        self.value8 >>= 1;
        self.set_alu_from_value8();
        self.A ^= self.value8;
        self.NZ_A();
    }

    fn ZP_RMW_RRA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 >> 1) | ((self.P & 1) << 7);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self._Adc();
    }

    fn ZP_RMW_DCP(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = self.temp8.wrapping_sub(1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self._Cmp();
    }

    fn ZP_RMW_LSR(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 1) != 0);
        self.value8 >>= 1;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn ZP_RMW_ROR(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 >> 1) | ((self.P & 1) << 7);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self.set_nz(self.value8);
    }

    fn ZP_RMW_ROL(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 << 1) | (self.P & 1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 0x80) != 0);
        self.set_nz(self.value8);
    }

    fn ZP_RMW_SLO(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 0x80) != 0);
        self.value8 <<= 1;
        self.set_alu_from_value8();
        self.A |= self.value8;
        self.NZ_A();
    }

    fn ZP_RMW_ISC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.value8 = (self.alu_temp as u8).wrapping_add(1);
        self.set_alu_from_value8();
        self._Sbc();
    }

    fn ZP_RMW_RLA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(u16::from(self.opcode2), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 << 1) | (self.P & 1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 0x80) != 0);
        self.A &= self.value8;
        self.NZ_A();
    }

    // -------------------------------------------- indexed indirect (addr,X)

    fn IdxInd_Stage3(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(u16::from(self.opcode2));
            self.alu_temp = i32::from(self.opcode2.wrapping_add(self.X));
        }
    }

    fn IdxInd_Stage4(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.ea = i32::from(bus.read_memory(self.alu_temp as u16));
            self.address_bus = self.ea16();
        }
    }

    fn IdxInd_Stage5(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let addr = u16::from((self.alu_temp as u8).wrapping_add(1));
            self.ea += i32::from(bus.read_memory(addr)) << 8;
        }
    }

    fn IdxInd_Stage6_READ_LDA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.A = bus.read_memory(self.ea16());
            self.NZ_A();
        }
    }

    fn IdxInd_Stage6_READ_ORA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Ora();
        }
    }

    fn IdxInd_Stage6_READ_LAX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.A = bus.read_memory(self.ea16());
            self.X = self.A;
            self.NZ_A();
        }
    }

    fn IdxInd_Stage6_READ_CMP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Cmp();
        }
    }

    fn IdxInd_Stage6_READ_ADC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Adc();
        }
    }

    fn IdxInd_Stage6_READ_AND(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._And();
        }
    }

    fn IdxInd_Stage6_READ_EOR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Eor();
        }
    }

    fn IdxInd_Stage6_READ_SBC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Sbc();
        }
    }

    fn IdxInd_Stage6_WRITE_STA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.A);
    }

    fn IdxInd_Stage6_WRITE_SAX(&mut self, bus: &mut impl Mos6502xLink) {
        self.alu_temp = i32::from(self.A & self.X);
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        // flag writing skipped on purpose
    }

    fn IdxInd_Stage6_RMW(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
        }
    }

    fn IdxInd_Stage7_RMW_SLO(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 0x80) != 0);
        self.value8 <<= 1;
        self.set_alu_from_value8();
        self.A |= self.value8;
        self.NZ_A();
    }

    fn IdxInd_Stage7_RMW_ISC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp as u8).wrapping_add(1);
        self.set_alu_from_value8();
        self._Sbc();
    }

    fn IdxInd_Stage7_RMW_DCP(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = self.temp8.wrapping_sub(1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self._Cmp();
    }

    fn IdxInd_Stage7_RMW_SRE(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 1) != 0);
        self.value8 >>= 1;
        self.set_alu_from_value8();
        self.A ^= self.value8;
        self.NZ_A();
    }

    fn IdxInd_Stage7_RMW_RRA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 >> 1) | ((self.P & 1) << 7);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self._Adc();
    }

    fn IdxInd_Stage7_RMW_RLA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 << 1) | (self.P & 1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 0x80) != 0);
        self.A &= self.value8;
        self.NZ_A();
    }

    fn IdxInd_Stage8_RMW(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
    }

    // -------------------------------------------- indirect indexed (addr),Y

    fn IndIdx_Stage3(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.ea = i32::from(bus.read_memory(u16::from(self.opcode2)));
        }
    }

    fn IndIdx_Stage4(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = self.ea + i32::from(self.Y);
            let addr = u16::from(self.opcode2.wrapping_add(1));
            self.ea = (i32::from(bus.read_memory(addr)) << 8) | (self.alu_temp & 0xFF);
            self.address_bus = self.ea16();
            self.H = 0; // In preparation for SHA (indirect, X), set H to 0.
        }
    }

    fn IndIdx_WRITE_Stage5(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.read_memory(self.ea16());
            self.ea += (self.alu_temp >> 8) << 8;
        }
    }

    fn IndIdx_WRITE_Stage5_SHA(&mut self, bus: &mut impl Mos6502xLink) {
        self.rdy_freeze = !self.rdy;

        if self.rdy {
            self.H |= ((self.ea >> 8) + 1) as u8;
            bus.read_memory(self.ea16());

            if bit(self.alu_temp, 8) {
                self.ea = i32::from(
                    ((self.ea & 0xFF)
                        | ((self.ea + 0x100) & 0xFF00 & (i32::from(self.A & self.X) << 8)))
                        as u16,
                );
            }
        } else {
            // If the RDY line is low here, the SHA instruction omits the bitwise AND with H
            self.H = 0xFF;
        }
    }

    fn IndIdx_READ_Stage5(&mut self, bus: &mut impl Mos6502xLink) {
        if bit(self.alu_temp, 8) {
            if self.rdy_gate() {
                bus.read_memory(self.ea16());
                self.ea_add_page();
            }
        } else {
            self.mi += 1;
            self.execute_one_retry(bus);
        }
    }

    fn IndIdx_RMW_Stage5(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.read_memory(self.ea16());
            if bit(self.alu_temp, 8) {
                self.ea_add_page();
            }
        }
    }

    fn IndIdx_WRITE_Stage6_STA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.A);
    }

    fn IndIdx_WRITE_Stage6_SHA(&mut self, bus: &mut impl Mos6502xLink) {
        self.alu_temp = i32::from(self.A & self.X & self.H);
        bus.write_memory(self.ea16(), self.alu_temp as u8);
    }

    fn IndIdx_READ_Stage6_LDA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.A = bus.read_memory(self.ea16());
            self.NZ_A();
        }
    }

    fn IndIdx_READ_Stage6_CMP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Cmp();
        }
    }

    fn IndIdx_READ_Stage6_AND(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._And();
        }
    }

    fn IndIdx_READ_Stage6_EOR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Eor();
        }
    }

    fn IndIdx_READ_Stage6_LAX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.A = bus.read_memory(self.ea16());
            self.X = self.A;
            self.NZ_A();
        }
    }

    fn IndIdx_READ_Stage6_ADC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Adc();
        }
    }

    fn IndIdx_READ_Stage6_SBC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Sbc();
        }
    }

    fn IndIdx_READ_Stage6_ORA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Ora();
        }
    }

    fn IndIdx_RMW_Stage6(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
        }
    }

    fn IndIdx_RMW_Stage7_SLO(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 0x80) != 0);
        self.value8 <<= 1;
        self.set_alu_from_value8();
        self.A |= self.value8;
        self.NZ_A();
    }

    fn IndIdx_RMW_Stage7_SRE(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 1) != 0);
        self.value8 >>= 1;
        self.set_alu_from_value8();
        self.A ^= self.value8;
        self.NZ_A();
    }

    fn IndIdx_RMW_Stage7_RRA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 >> 1) | ((self.P & 1) << 7);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self._Adc();
    }

    fn IndIdx_RMW_Stage7_ISC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = self.temp8.wrapping_add(1);
        self.set_alu_from_value8();
        self._Sbc();
    }

    fn IndIdx_RMW_Stage7_DCP(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = self.temp8.wrapping_sub(1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self._Cmp();
    }

    fn IndIdx_RMW_Stage7_RLA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 << 1) | (self.P & 1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 0x80) != 0);
        self.A &= self.value8;
        self.NZ_A();
    }

    fn IndIdx_RMW_Stage8(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
    }

    // ----------------------------------------------------- absolute indexed

    fn AbsIdx_Stage3_Y(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.opcode3 = bus.read_memory(pc);
            self.alu_temp = i32::from(self.opcode2) + i32::from(self.Y);
            self.ea = (i32::from(self.opcode3) << 8) + (self.alu_temp & 0xFF);
            self.address_bus = self.ea16();
            self.H = 0; // In preparation for SHA, SHS, and SHX, set H to 0.
        }
    }

    fn AbsIdx_Stage3_X(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.opcode3 = bus.read_memory(pc);
            self.alu_temp = i32::from(self.opcode2) + i32::from(self.X);
            self.ea = (i32::from(self.opcode3) << 8) + (self.alu_temp & 0xFF);
            self.address_bus = self.ea16();
            self.H = 0; // In preparation for SHY, set H to 0.
        }
    }

    fn AbsIdx_READ_Stage4(&mut self, bus: &mut impl Mos6502xLink) {
        if bit(self.alu_temp, 8) {
            if self.rdy_gate() {
                self.alu_temp = i32::from(bus.read_memory(self.ea16()));
                self.ea_add_page();
            }
        } else {
            self.mi += 1;
            self.execute_one_retry(bus);
        }
    }

    fn AbsIdx_Stage4(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let adjust = bit(self.alu_temp, 8);
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));

            if adjust {
                self.ea_add_page();
            }
        }
    }

    fn AbsIdx_Stage4_SHX(&mut self, bus: &mut impl Mos6502xLink) {
        self.rdy_freeze = !self.rdy;

        if self.rdy {
            self.H |= ((self.ea >> 8) + 1) as u8;
            let adjust = bit(self.alu_temp, 8);
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));

            if adjust {
                self.ea = i32::from(
                    ((self.ea & 0xFF) | ((self.ea + 0x100) & 0xFF00 & (i32::from(self.X) << 8)))
                        as u16,
                );
            }
        } else {
            // If the RDY line is low here, the SHX instruction omits the bitwise AND with H
            self.H = 0xFF;
        }
    }

    fn AbsIdx_Stage4_SHY(&mut self, bus: &mut impl Mos6502xLink) {
        self.rdy_freeze = !self.rdy;

        if self.rdy {
            self.H |= ((self.ea >> 8) + 1) as u8;
            let adjust = bit(self.alu_temp, 8);
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));

            if adjust {
                self.ea = i32::from(
                    ((self.ea & 0xFF) | ((self.ea + 0x100) & 0xFF00 & (i32::from(self.Y) << 8)))
                        as u16,
                );
            }
        } else {
            // If the RDY line is low here, the SHY instruction omits the bitwise AND with H
            self.H = 0xFF;
        }
    }

    fn AbsIdx_Stage4_SHA(&mut self, bus: &mut impl Mos6502xLink) {
        self.rdy_freeze = !self.rdy;

        if self.rdy {
            self.H |= ((self.ea >> 8) + 1) as u8;
            let adjust = bit(self.alu_temp, 8);
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));

            if adjust {
                self.ea = i32::from(
                    ((self.ea & 0xFF)
                        | ((self.ea + 0x100) & 0xFF00 & (i32::from(self.A & self.X) << 8)))
                        as u16,
                );
            }
        } else {
            // If the RDY line is low here, the SHA instruction omits the bitwise AND with H
            self.H = 0xFF;
        }
    }

    fn AbsIdx_Stage4_SHS(&mut self, bus: &mut impl Mos6502xLink) {
        self.rdy_freeze = !self.rdy;

        if self.rdy {
            self.H |= ((self.ea >> 8) + 1) as u8;
            let adjust = bit(self.alu_temp, 8);
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));

            if adjust {
                self.ea = i32::from(
                    ((self.ea & 0xFF)
                        | ((self.ea + 0x100) & 0xFF00 & (i32::from(self.A & self.X) << 8)))
                        as u16,
                );
            }
        } else {
            // If the RDY line is low here, the SHS instruction omits the bitwise AND with H
            self.H = 0xFF;
        }
    }

    fn AbsIdx_WRITE_Stage5_STA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.A);
    }

    fn AbsIdx_WRITE_Stage5_SHY(&mut self, bus: &mut impl Mos6502xLink) {
        self.alu_temp = i32::from(self.Y & self.H);
        bus.write_memory(self.ea16(), self.alu_temp as u8);
    }

    fn AbsIdx_WRITE_Stage5_SHX(&mut self, bus: &mut impl Mos6502xLink) {
        self.alu_temp = i32::from(self.X & self.H);
        bus.write_memory(self.ea16(), self.alu_temp as u8);
    }

    fn AbsIdx_WRITE_Stage5_SHA(&mut self, bus: &mut impl Mos6502xLink) {
        self.alu_temp = i32::from(self.A & self.X & self.H);
        bus.write_memory(self.ea16(), self.alu_temp as u8);
    }

    fn AbsIdx_WRITE_Stage5_SHS(&mut self, bus: &mut impl Mos6502xLink) {
        self.S = self.X & self.A;
        bus.write_memory(self.ea16(), self.S & self.H);
    }

    fn AbsIdx_RMW_Stage5(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
        }
    }

    fn AbsIdx_RMW_Stage7(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
    }

    fn AbsIdx_RMW_Stage6_DEC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp - 1) as u8;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn AbsIdx_RMW_Stage6_DCP(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp - 1) as u8;
        self.set_alu_from_value8();
        self._Cmp();
    }

    fn AbsIdx_RMW_Stage6_ISC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp + 1) as u8;
        self.set_alu_from_value8();
        self._Sbc();
    }

    fn AbsIdx_RMW_Stage6_INC(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = (self.alu_temp + 1) as u8;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn AbsIdx_RMW_Stage6_ROL(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 << 1) | (self.P & 1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 0x80) != 0);
        self.set_nz(self.value8);
    }

    fn AbsIdx_RMW_Stage6_LSR(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 1) != 0);
        self.value8 >>= 1;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn AbsIdx_RMW_Stage6_SLO(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 0x80) != 0);
        self.value8 <<= 1;
        self.set_alu_from_value8();
        self.A |= self.value8;
        self.NZ_A();
    }

    fn AbsIdx_RMW_Stage6_SRE(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 1) != 0);
        self.value8 >>= 1;
        self.set_alu_from_value8();
        self.A ^= self.value8;
        self.NZ_A();
    }

    fn AbsIdx_RMW_Stage6_RRA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 >> 1) | ((self.P & 1) << 7);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self._Adc();
    }

    fn AbsIdx_RMW_Stage6_RLA(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 << 1) | (self.P & 1);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 0x80) != 0);
        self.A &= self.value8;
        self.NZ_A();
    }

    fn AbsIdx_RMW_Stage6_ASL(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.value8 = self.alu_temp as u8;
        self.set_flag_c((self.value8 & 0x80) != 0);
        self.value8 <<= 1;
        self.set_alu_from_value8();
        self.set_nz(self.value8);
    }

    fn AbsIdx_RMW_Stage6_ROR(&mut self, bus: &mut impl Mos6502xLink) {
        bus.write_memory(self.ea16(), self.alu_temp as u8);
        self.temp8 = self.alu_temp as u8;
        self.value8 = (self.temp8 >> 1) | ((self.P & 1) << 7);
        self.set_alu_from_value8();
        self.set_flag_c((self.temp8 & 1) != 0);
        self.set_nz(self.value8);
    }

    fn AbsIdx_READ_Stage5_LDA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.A = bus.read_memory(self.ea16());
            self.NZ_A();
        }
    }

    fn AbsIdx_READ_Stage5_LDX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.X = bus.read_memory(self.ea16());
            self.NZ_X();
        }
    }

    fn AbsIdx_READ_Stage5_LAX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.A = bus.read_memory(self.ea16());
            self.X = self.A;
            self.NZ_A();
        }
    }

    fn AbsIdx_READ_Stage5_LDY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.Y = bus.read_memory(self.ea16());
            self.NZ_Y();
        }
    }

    fn AbsIdx_READ_Stage5_ORA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Ora();
        }
    }

    fn AbsIdx_READ_Stage5_NOP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
        }
    }

    fn AbsIdx_READ_Stage5_CMP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Cmp();
        }
    }

    fn AbsIdx_READ_Stage5_SBC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Sbc();
        }
    }

    fn AbsIdx_READ_Stage5_ADC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Adc();
        }
    }

    fn AbsIdx_READ_Stage5_EOR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._Eor();
        }
    }

    fn AbsIdx_READ_Stage5_AND(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self._And();
        }
    }

    /// `LAS`/`LAE` (`$BB`). The C# calls this one `_ERROR` and implements it anyway.
    fn AbsIdx_READ_Stage5_ERROR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.alu_temp = i32::from(bus.read_memory(self.ea16()));
            self.S &= self.alu_temp as u8;
            self.X = self.S;
            self.A = self.S;
            self.set_nz(self.S);
        }
    }

    // ------------------------------------------------------------ immediate

    fn Imm_EOR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Eor();
        }
    }

    fn Imm_ANC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Anc();
        }
    }

    fn Imm_ASR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Asr();
        }
    }

    fn Imm_AXS(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Axs();
        }
    }

    fn Imm_ARR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Arr();
        }
    }

    fn Imm_LXA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Lxa();
        }
    }

    fn Imm_ORA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Ora();
        }
    }

    fn Imm_CPY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Cpy();
        }
    }

    fn Imm_CPX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Cpx();
        }
    }

    fn Imm_CMP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Cmp();
        }
    }

    fn Imm_SBC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Sbc();
        }
    }

    fn Imm_AND(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._And();
        }
    }

    fn Imm_ADC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Adc();
        }
    }

    fn Imm_LDA(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.A = bus.read_memory(pc);
            self.NZ_A();
        }
    }

    fn Imm_LDX(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.X = bus.read_memory(pc);
            self.NZ_X();
        }
    }

    fn Imm_LDY(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.Y = bus.read_memory(pc);
            self.NZ_Y();
        }
    }

    fn Imm_ANE(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.alu_temp = i32::from(bus.read_memory(pc));
            self._Ane();
        }
    }

    fn Imm_Unsupported(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            bus.read_memory(pc);
        }
    }

    // ----------------------------------------------------------- the branches

    fn RelBranch_Stage2_BVS(&mut self, bus: &mut impl Mos6502xLink) {
        self.branch_taken = self.flag_v();
        self.RelBranch_Stage2(bus);
    }

    fn RelBranch_Stage2_BVC(&mut self, bus: &mut impl Mos6502xLink) {
        self.branch_taken = !self.flag_v();
        self.RelBranch_Stage2(bus);
    }

    fn RelBranch_Stage2_BMI(&mut self, bus: &mut impl Mos6502xLink) {
        self.branch_taken = self.flag_n();
        self.RelBranch_Stage2(bus);
    }

    fn RelBranch_Stage2_BPL(&mut self, bus: &mut impl Mos6502xLink) {
        self.branch_taken = !self.flag_n();
        self.RelBranch_Stage2(bus);
    }

    fn RelBranch_Stage2_BCS(&mut self, bus: &mut impl Mos6502xLink) {
        self.branch_taken = self.flag_c();
        self.RelBranch_Stage2(bus);
    }

    fn RelBranch_Stage2_BCC(&mut self, bus: &mut impl Mos6502xLink) {
        self.branch_taken = !self.flag_c();
        self.RelBranch_Stage2(bus);
    }

    fn RelBranch_Stage2_BEQ(&mut self, bus: &mut impl Mos6502xLink) {
        self.branch_taken = self.flag_z();
        self.RelBranch_Stage2(bus);
    }

    fn RelBranch_Stage2_BNE(&mut self, bus: &mut impl Mos6502xLink) {
        self.branch_taken = !self.flag_z();
        self.RelBranch_Stage2(bus);
    }

    fn RelBranch_Stage2(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let pc = self.pc_pp();
            self.opcode2 = bus.read_memory(pc);
            if self.branch_taken {
                self.branch_taken = false;
                // if the branch is taken, we enter a different bit of microcode to calculate the
                // PC and complete the branch
                self.opcode = VOP_RELATIVE_STUFF;
                self.mi = -1;
            }
        }
    }

    fn RelBranch_Stage3(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            self.alu_temp = i32::from(self.PC as u8) + i32::from(self.opcode2 as i8);
            self.PC &= 0xFF00;
            self.PC |= (self.alu_temp & 0xFF) as u16;

            if bit(self.alu_temp, 8) {
                // we need to carry the add, and then we'll be ready to fetch the next instruction
                self.opcode = VOP_RELATIVE_STUFF2;
                self.mi = -1;
            } else if !self.interrupt_pending {
                // to pass cpu_interrupts_v2/5-branch_delays_irq we need to handle a quirk here:
                // if we decide to interrupt in the next cycle, this condition will cause it to
                // get deferred by one instruction
                self.branch_irq_hack = true;
            }
        }
    }

    fn RelBranch_Stage4(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
            if bit(self.alu_temp, 31) {
                self.PC = self.PC.wrapping_sub(0x100);
            } else {
                self.PC = self.PC.wrapping_add(0x100);
            }
        }
    }

    // ------------------------------------------- the stack instructions, misc

    fn NOP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.dummy_read_memory(self.PC);
        }
    }

    fn DecS(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.S = self.S.wrapping_sub(1);
            let addr = self.stack_addr();
            bus.dummy_read_memory(addr);
        }
    }

    fn IncS(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let addr = self.pull_addr();
            bus.dummy_read_memory(addr);
        }
    }

    fn JSR(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.PC =
                (u16::from(bus.read_memory(self.PC)) << 8).wrapping_add(u16::from(self.opcode2));
        }
    }

    fn PushP(&mut self, bus: &mut impl Mos6502xLink) {
        self.set_flag_b(true);
        let addr = self.push_addr();
        bus.write_memory(addr, self.P);
    }

    fn PushA(&mut self, bus: &mut impl Mos6502xLink) {
        let addr = self.push_addr();
        bus.write_memory(addr, self.A);
    }

    fn PullA_NoInc(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let addr = self.stack_addr();
            self.A = bus.read_memory(addr);
            self.NZ_A();
        }
    }

    fn PullP(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let addr = self.pull_addr();
            self.P = bus.read_memory(addr);
            self.set_flag_t(true); // force T always to remain true
        }
    }

    fn PullP_NoInc(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            self.my_iflag = self.flag_i();
            let addr = self.stack_addr();
            self.P = bus.read_memory(addr);
            self.iflag_pending = self.flag_i();
            self.set_flag_i(self.my_iflag);
            self.set_flag_t(true); // force T always to remain true
        }
    }

    fn PullPCL(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let addr = self.pull_addr();
            self.PC &= 0xFF00;
            self.PC |= u16::from(bus.read_memory(addr));
        }
    }

    fn PullPCH_NoInc(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            let addr = self.stack_addr();
            self.PC &= 0xFF;
            self.PC |= u16::from(bus.read_memory(addr)) << 8;
        }
    }

    fn IncPC(&mut self, bus: &mut impl Mos6502xLink) {
        if self.rdy_gate() {
            bus.read_memory(self.PC);
            self.PC = self.PC.wrapping_add(1);
        }
    }

    // --------------------------------------------------------- the ALU sub-ops

    fn _Cpx(&mut self) {
        self.value8 = self.alu_temp as u8;
        self.value16 = (i32::from(self.X) - i32::from(self.value8)) as u16;
        self.set_flag_c(self.X >= self.value8);
        self.set_nz(self.value16 as u8);
    }

    fn _Cpy(&mut self) {
        self.value8 = self.alu_temp as u8;
        self.value16 = (i32::from(self.Y) - i32::from(self.value8)) as u16;
        self.set_flag_c(self.Y >= self.value8);
        self.set_nz(self.value16 as u8);
    }

    fn _Cmp(&mut self) {
        self.value8 = self.alu_temp as u8;
        self.value16 = (i32::from(self.A) - i32::from(self.value8)) as u16;
        self.set_flag_c(self.A >= self.value8);
        self.set_nz(self.value16 as u8);
    }

    fn _Bit(&mut self) {
        self.set_flag_n((self.alu_temp & 0x80) != 0);
        self.set_flag_v((self.alu_temp & 0x40) != 0);
        self.set_flag_z((i32::from(self.A) & self.alu_temp) == 0);
    }

    fn _Eor(&mut self) {
        self.A ^= self.alu_temp as u8;
        self.NZ_A();
    }

    fn _And(&mut self) {
        self.A &= self.alu_temp as u8;
        self.NZ_A();
    }

    fn _Ora(&mut self) {
        self.A |= self.alu_temp as u8;
        self.NZ_A();
    }

    fn _Anc(&mut self) {
        self.A &= self.alu_temp as u8;
        self.set_flag_c((self.A >> 7) & 1 != 0);
        self.NZ_A();
    }

    fn _Asr(&mut self) {
        self.A &= self.alu_temp as u8;
        self.set_flag_c(self.A & 1 != 0);
        self.A >>= 1;
        self.NZ_A();
    }

    fn _Axs(&mut self) {
        self.X &= self.A;
        self.alu_temp = i32::from(self.X) - i32::from(self.alu_temp as u8);
        self.X = self.alu_temp as u8;
        self.set_flag_c(!bit(self.alu_temp, 8));
        self.NZ_X();
    }

    fn _Arr(&mut self) {
        self.A &= self.alu_temp as u8;

        if self.flag_d() && self.bcd_enabled {
            // Shift logic
            let mut next = i32::from(self.A >> 1) | if self.flag_c() { 0x80 } else { 0x00 };
            self.set_flag_v(((i32::from(self.A) ^ next) & 0x40) != 0);
            self.set_flag_n(self.flag_c());
            self.set_flag_z((next & 0xFF) == 0);

            // BCD fixup
            if (self.A & 0x0F) + (self.A & 0x01) > 0x05 {
                next = (next & 0xF0) | ((next + 0x06) & 0x0F);
            }
            if i32::from(self.A & 0xF0) + i32::from(self.A & 0x10) > 0x50 {
                next = (next & 0x0F) | ((next + 0x60) & 0xF0);
                self.set_flag_c(true);
            } else {
                self.set_flag_c(false);
            }

            self.A = next as u8;
        } else {
            self.booltemp = self.A & 1 != 0;
            self.A = (self.A >> 1) | if self.flag_c() { 0x80 } else { 0x00 };
            self.set_flag_c(self.booltemp);
            if (self.A >> 5) & 1 != 0 {
                if (self.A >> 6) & 1 != 0 {
                    self.set_flag_c(true);
                    self.set_flag_v(false);
                } else {
                    self.set_flag_v(true);
                    self.set_flag_c(false);
                }
            } else if (self.A >> 6) & 1 != 0 {
                self.set_flag_v(true);
                self.set_flag_c(true);
            } else {
                self.set_flag_v(false);
                self.set_flag_c(false);
            }
            self.NZ_A();
        }
    }

    /// Many varied reports on what this should be. A safe value is `0xFF`; Commodore 64 needs
    /// `0xEF`. (The C#'s own comment; NesHawk leaves `ane_constant` at `0xFF`.)
    fn _Ane(&mut self) {
        self.A |= self.ane_constant;
        self.A &= self.X & (self.alu_temp as u8);
        self.NZ_A();
    }

    /// There is some debate about what this should be; it may depend on the 6502 variant. This
    /// is suggested by qeed's doc for the NES and passes blargg's instruction test. (The C#'s
    /// own comment.)
    fn _Lxa(&mut self) {
        self.A |= self.lxa_constant;
        self.A &= self.alu_temp as u8;
        self.X = self.A;
        self.NZ_A();
    }

    fn _Sbc(&mut self) {
        self.value8 = self.alu_temp as u8;
        self.tempint = i32::from(self.A) - i32::from(self.value8) - i32::from(!self.flag_c());
        if self.flag_d() && self.bcd_enabled {
            self.lo = i32::from(self.A & 0x0F)
                - i32::from(self.value8 & 0x0F)
                - i32::from(!self.flag_c());
            self.hi = i32::from(self.A & 0xF0) - i32::from(self.value8 & 0xF0);
            if (self.lo & 0xF0) != 0 {
                self.lo -= 0x06;
            }
            if (self.lo & 0x80) != 0 {
                self.hi -= 0x10;
            }
            if (self.hi & 0x0F00) != 0 {
                self.hi -= 0x60;
            }
            self.set_flag_v(
                ((i32::from(self.A) ^ i32::from(self.value8))
                    & (i32::from(self.A) ^ self.tempint)
                    & 0x80)
                    != 0,
            );
            self.set_flag_z((self.tempint & 0xFF) == 0);
            self.set_flag_n((self.tempint & 0x80) != 0);
            self.set_flag_c((self.hi & 0xFF00) == 0);
            self.A = ((self.lo & 0x0F) | (self.hi & 0xF0)) as u8;
        } else {
            self.set_flag_v(
                ((i32::from(self.A) ^ i32::from(self.value8))
                    & (i32::from(self.A) ^ self.tempint)
                    & 0x80)
                    != 0,
            );
            self.set_flag_c(self.tempint >= 0);
            self.A = self.tempint as u8;
            self.NZ_A();
        }
    }

    fn _Adc(&mut self) {
        self.value8 = self.alu_temp as u8;
        if self.flag_d() && self.bcd_enabled {
            self.tempint =
                i32::from(self.A & 0x0F) + i32::from(self.value8 & 0x0F) + i32::from(self.flag_c());
            if self.tempint > 0x09 {
                self.tempint += 0x06;
            }
            self.tempint = (self.tempint & 0x0F)
                + i32::from(self.A & 0xF0)
                + i32::from(self.value8 & 0xF0)
                + i32::from(self.tempint > 0x0F) * 0x10;
            self.set_flag_v(
                (!(i32::from(self.A) ^ i32::from(self.value8))
                    & (i32::from(self.A) ^ self.tempint)
                    & 0x80)
                    != 0,
            );
            self.set_flag_z(
                ((i32::from(self.A) + i32::from(self.value8) + i32::from(self.flag_c())) & 0xFF)
                    == 0,
            );
            self.set_flag_n((self.tempint & 0x80) != 0);
            if (self.tempint & 0x1F0) > 0x090 {
                self.tempint += 0x060;
            }
            self.set_flag_c(self.tempint > 0xFF);
            self.A = (self.tempint & 0xFF) as u8;
        } else {
            self.tempint = i32::from(self.value8) + i32::from(self.A) + i32::from(self.flag_c());
            self.set_flag_v(
                (!(i32::from(self.A) ^ i32::from(self.value8))
                    & (i32::from(self.A) ^ self.tempint)
                    & 0x80)
                    != 0,
            );
            self.set_flag_c(self.tempint > 0xFF);
            self.A = self.tempint as u8;
            self.NZ_A();
        }
    }

    // ------------------------------------------------------- end of instruction

    fn End_ISpecial(&mut self, bus: &mut impl Mos6502xLink) {
        self.opcode = VOP_FETCH1;
        self.mi = 0;
        self.execute_one_retry(bus);
    }

    fn End_SuppressInterrupt(&mut self, bus: &mut impl Mos6502xLink) {
        self.opcode = VOP_FETCH1_NO_INTERRUPT;
        self.mi = 0;
        self.execute_one_retry(bus);
    }

    fn End(&mut self, bus: &mut impl Mos6502xLink) {
        self.opcode = VOP_FETCH1;
        self.mi = 0;
        self.iflag_pending = self.flag_i();
        self.execute_one_retry(bus);
    }

    fn Jam(&mut self) {
        self.rdy_freeze = true;
    }
}

/// The table's shape, checked here rather than trusted: 264 rows, the eight `VOP_*` last.
#[cfg(test)]
mod tests {
    use super::super::uop::{MICROCODE, VOP_NUM};
    use super::*;

    #[test]
    fn the_microcode_table_has_a_row_per_opcode_and_per_vop() {
        assert_eq!(MICROCODE.len(), VOP_NUM);
        assert_eq!(MICROCODE[VOP_FETCH1], &[Uop::Fetch1]);
        assert_eq!(MICROCODE[VOP_FETCH1_NO_INTERRUPT], &[Uop::Fetch1_Real]);
        // every row must be non-empty, or `current_uop` would have nothing to run
        assert!(MICROCODE.iter().all(|row| !row.is_empty()));
    }

    /// `VOP_RESET` is eight micro-operations, not nine: the C# has a *commented-out*
    /// `Uop.FetchDummy` in the middle of that row, and a transliteration that treats the row as
    /// text rather than as code picks it up. This test is the one that caught exactly that.
    #[test]
    fn the_reset_sequence_is_the_cs_eight_micro_operations() {
        use super::super::uop::VOP_RESET;
        assert_eq!(
            MICROCODE[VOP_RESET],
            &[
                Uop::FetchDummy,
                Uop::FetchDummy,
                Uop::PushDummy,
                Uop::PushDummy,
                Uop::PushP_Reset,
                Uop::FetchPCLVector,
                Uop::FetchPCHVector,
                Uop::End_SuppressInterrupt,
            ]
        );
    }

    /// `AtInstructionStart` is an *ordering* test on the enum, so the variant order is part of
    /// the port's behaviour.
    #[test]
    fn end_and_everything_after_it_counts_as_an_instruction_start() {
        assert!(Uop::End_ISpecial >= Uop::End);
        assert!(Uop::End_SuppressInterrupt >= Uop::End);
        assert!(Uop::Jam >= Uop::End);
        // the seven appended "More unofficial micro-ops" are above `End` too, which is a quirk
        // of where they were added and is kept deliberately
        assert!(Uop::Imm_ANE >= Uop::End);
        assert!(Uop::AbsIdx_Stage4_SHS >= Uop::End);
        // and a mid-instruction micro-operation is below it
        assert!(Uop::Fetch2 < Uop::End);
        assert!(Uop::Abs_RMW_Stage6 < Uop::End);
    }
}
