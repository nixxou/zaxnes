use super::{AddressingMode, CpuOperation};

/// How an instruction touches its operand, which decides the dummy cycles of the
/// addressing mode (nesdev, "6502 instruction reference" and "CPU unofficial opcodes").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperandAccess {
    /// No memory operand: implied, accumulator, branch, jump or stack.
    None,
    /// Reads the operand. Indexed modes cost one extra cycle only on a page cross.
    Read,
    /// Writes the operand. Indexed modes always pay the extra cycle.
    Write,
    /// Reads, modifies and writes back. One dummy write, and always the extra cycle.
    ReadModifyWrite,
}

/// Static description of one of the 256 opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpcodeInfo {
    /// The operation performed.
    pub operation: CpuOperation,
    /// The addressing mode.
    pub mode: AddressingMode,
    /// How the operand is touched.
    pub access: OperandAccess,
    /// `false` for the unofficial opcodes, which `nestest.log` prefixes with `*`.
    pub official: bool,
}

impl OpcodeInfo {
    /// Total length of the instruction in bytes, opcode included.
    #[must_use]
    pub const fn length(&self) -> usize {
        match self.mode {
            AddressingMode::Implied | AddressingMode::Accumulator => 1,
            AddressingMode::Immediate
            | AddressingMode::Relative
            | AddressingMode::ZeroPage
            | AddressingMode::ZeroPageX
            | AddressingMode::ZeroPageY
            | AddressingMode::IndirectX
            | AddressingMode::IndirectY => 2,
            _ => 3,
        }
    }

    /// Three-letter mnemonic, upper case.
    #[must_use]
    pub const fn mnemonic(&self) -> &'static str {
        self.operation.mnemonic()
    }
}

/// The opcode matrix of the 2A03's 6502 core: all 151 official opcodes and all 105
/// unofficial ones (REQ-CORE-110).
///
/// Built from the public nesdev documentation ("6502 instruction reference", "CPU
/// unofficial opcodes") and the mnemonics of the public `nestest.log`. The table is the
/// single source of truth: [`super::Cpu6502`] derives both the cycle count and the operand
/// fetch from the addressing mode and [`OperandAccess`], and [`super::CpuTracer`] derives
/// the disassembly from the same entries, so the two cannot drift apart.
///
/// Cycle counts are not stored: they follow from the explicit bus accesses that the
/// addressing mode performs, including the dummy reads of indexed modes and the dummy
/// write of a read-modify-write (see [`super::Cpu6502`]).
pub struct OpcodeTable;

static ENTRIES: [OpcodeInfo; 256] = build_entries();

impl OpcodeTable {
    /// Returns the description of `opcode`.
    #[inline]
    #[must_use]
    pub fn get(opcode: u8) -> OpcodeInfo {
        ENTRIES[opcode as usize]
    }

    /// All 256 entries, indexed by opcode.
    #[must_use]
    pub fn all() -> &'static [OpcodeInfo; 256] {
        &ENTRIES
    }
}

const fn e(
    operation: CpuOperation,
    mode: AddressingMode,
    access: OperandAccess,
    official: bool,
) -> OpcodeInfo {
    OpcodeInfo {
        operation,
        mode,
        access,
        official,
    }
}

// Every slot is filled below; the matrix of the NMOS 6502 has no gap once the unofficial
// opcodes are included.
#[allow(clippy::too_many_lines)] // One line per opcode, kept as a flat matrix.
const fn build_entries() -> [OpcodeInfo; 256] {
    use AddressingMode::{
        Absolute as Abs, AbsoluteX as AbX, AbsoluteY as AbY, Accumulator as Acc, Immediate as Imm,
        Implied as Imp, Indirect as Ind, IndirectX as IzX, IndirectY as IzY, Relative as Rel,
        ZeroPage as Zp, ZeroPageX as ZpX, ZeroPageY as ZpY,
    };
    use CpuOperation as Op;
    use OperandAccess::{None as No, Read as Rd, ReadModifyWrite as Rmw, Write as Wr};

    let mut t = [e(Op::Jam, Imp, No, false); 256];
    t[0x00] = e(Op::Brk, Imp, No, true);
    t[0x01] = e(Op::Ora, IzX, Rd, true);
    t[0x02] = e(Op::Jam, Imp, No, false);
    t[0x03] = e(Op::Slo, IzX, Rmw, false);
    t[0x04] = e(Op::Nop, Zp, Rd, false);
    t[0x05] = e(Op::Ora, Zp, Rd, true);
    t[0x06] = e(Op::Asl, Zp, Rmw, true);
    t[0x07] = e(Op::Slo, Zp, Rmw, false);
    t[0x08] = e(Op::Php, Imp, No, true);
    t[0x09] = e(Op::Ora, Imm, Rd, true);
    t[0x0A] = e(Op::Asl, Acc, No, true);
    t[0x0B] = e(Op::Anc, Imm, Rd, false);
    t[0x0C] = e(Op::Nop, Abs, Rd, false);
    t[0x0D] = e(Op::Ora, Abs, Rd, true);
    t[0x0E] = e(Op::Asl, Abs, Rmw, true);
    t[0x0F] = e(Op::Slo, Abs, Rmw, false);
    t[0x10] = e(Op::Bpl, Rel, No, true);
    t[0x11] = e(Op::Ora, IzY, Rd, true);
    t[0x12] = e(Op::Jam, Imp, No, false);
    t[0x13] = e(Op::Slo, IzY, Rmw, false);
    t[0x14] = e(Op::Nop, ZpX, Rd, false);
    t[0x15] = e(Op::Ora, ZpX, Rd, true);
    t[0x16] = e(Op::Asl, ZpX, Rmw, true);
    t[0x17] = e(Op::Slo, ZpX, Rmw, false);
    t[0x18] = e(Op::Clc, Imp, No, true);
    t[0x19] = e(Op::Ora, AbY, Rd, true);
    t[0x1A] = e(Op::Nop, Imp, No, false);
    t[0x1B] = e(Op::Slo, AbY, Rmw, false);
    t[0x1C] = e(Op::Nop, AbX, Rd, false);
    t[0x1D] = e(Op::Ora, AbX, Rd, true);
    t[0x1E] = e(Op::Asl, AbX, Rmw, true);
    t[0x1F] = e(Op::Slo, AbX, Rmw, false);
    t[0x20] = e(Op::Jsr, Abs, No, true);
    t[0x21] = e(Op::And, IzX, Rd, true);
    t[0x22] = e(Op::Jam, Imp, No, false);
    t[0x23] = e(Op::Rla, IzX, Rmw, false);
    t[0x24] = e(Op::Bit, Zp, Rd, true);
    t[0x25] = e(Op::And, Zp, Rd, true);
    t[0x26] = e(Op::Rol, Zp, Rmw, true);
    t[0x27] = e(Op::Rla, Zp, Rmw, false);
    t[0x28] = e(Op::Plp, Imp, No, true);
    t[0x29] = e(Op::And, Imm, Rd, true);
    t[0x2A] = e(Op::Rol, Acc, No, true);
    t[0x2B] = e(Op::Anc, Imm, Rd, false);
    t[0x2C] = e(Op::Bit, Abs, Rd, true);
    t[0x2D] = e(Op::And, Abs, Rd, true);
    t[0x2E] = e(Op::Rol, Abs, Rmw, true);
    t[0x2F] = e(Op::Rla, Abs, Rmw, false);
    t[0x30] = e(Op::Bmi, Rel, No, true);
    t[0x31] = e(Op::And, IzY, Rd, true);
    t[0x32] = e(Op::Jam, Imp, No, false);
    t[0x33] = e(Op::Rla, IzY, Rmw, false);
    t[0x34] = e(Op::Nop, ZpX, Rd, false);
    t[0x35] = e(Op::And, ZpX, Rd, true);
    t[0x36] = e(Op::Rol, ZpX, Rmw, true);
    t[0x37] = e(Op::Rla, ZpX, Rmw, false);
    t[0x38] = e(Op::Sec, Imp, No, true);
    t[0x39] = e(Op::And, AbY, Rd, true);
    t[0x3A] = e(Op::Nop, Imp, No, false);
    t[0x3B] = e(Op::Rla, AbY, Rmw, false);
    t[0x3C] = e(Op::Nop, AbX, Rd, false);
    t[0x3D] = e(Op::And, AbX, Rd, true);
    t[0x3E] = e(Op::Rol, AbX, Rmw, true);
    t[0x3F] = e(Op::Rla, AbX, Rmw, false);
    t[0x40] = e(Op::Rti, Imp, No, true);
    t[0x41] = e(Op::Eor, IzX, Rd, true);
    t[0x42] = e(Op::Jam, Imp, No, false);
    t[0x43] = e(Op::Sre, IzX, Rmw, false);
    t[0x44] = e(Op::Nop, Zp, Rd, false);
    t[0x45] = e(Op::Eor, Zp, Rd, true);
    t[0x46] = e(Op::Lsr, Zp, Rmw, true);
    t[0x47] = e(Op::Sre, Zp, Rmw, false);
    t[0x48] = e(Op::Pha, Imp, No, true);
    t[0x49] = e(Op::Eor, Imm, Rd, true);
    t[0x4A] = e(Op::Lsr, Acc, No, true);
    t[0x4B] = e(Op::Alr, Imm, Rd, false);
    t[0x4C] = e(Op::Jmp, Abs, No, true);
    t[0x4D] = e(Op::Eor, Abs, Rd, true);
    t[0x4E] = e(Op::Lsr, Abs, Rmw, true);
    t[0x4F] = e(Op::Sre, Abs, Rmw, false);
    t[0x50] = e(Op::Bvc, Rel, No, true);
    t[0x51] = e(Op::Eor, IzY, Rd, true);
    t[0x52] = e(Op::Jam, Imp, No, false);
    t[0x53] = e(Op::Sre, IzY, Rmw, false);
    t[0x54] = e(Op::Nop, ZpX, Rd, false);
    t[0x55] = e(Op::Eor, ZpX, Rd, true);
    t[0x56] = e(Op::Lsr, ZpX, Rmw, true);
    t[0x57] = e(Op::Sre, ZpX, Rmw, false);
    t[0x58] = e(Op::Cli, Imp, No, true);
    t[0x59] = e(Op::Eor, AbY, Rd, true);
    t[0x5A] = e(Op::Nop, Imp, No, false);
    t[0x5B] = e(Op::Sre, AbY, Rmw, false);
    t[0x5C] = e(Op::Nop, AbX, Rd, false);
    t[0x5D] = e(Op::Eor, AbX, Rd, true);
    t[0x5E] = e(Op::Lsr, AbX, Rmw, true);
    t[0x5F] = e(Op::Sre, AbX, Rmw, false);
    t[0x60] = e(Op::Rts, Imp, No, true);
    t[0x61] = e(Op::Adc, IzX, Rd, true);
    t[0x62] = e(Op::Jam, Imp, No, false);
    t[0x63] = e(Op::Rra, IzX, Rmw, false);
    t[0x64] = e(Op::Nop, Zp, Rd, false);
    t[0x65] = e(Op::Adc, Zp, Rd, true);
    t[0x66] = e(Op::Ror, Zp, Rmw, true);
    t[0x67] = e(Op::Rra, Zp, Rmw, false);
    t[0x68] = e(Op::Pla, Imp, No, true);
    t[0x69] = e(Op::Adc, Imm, Rd, true);
    t[0x6A] = e(Op::Ror, Acc, No, true);
    t[0x6B] = e(Op::Arr, Imm, Rd, false);
    t[0x6C] = e(Op::Jmp, Ind, No, true);
    t[0x6D] = e(Op::Adc, Abs, Rd, true);
    t[0x6E] = e(Op::Ror, Abs, Rmw, true);
    t[0x6F] = e(Op::Rra, Abs, Rmw, false);
    t[0x70] = e(Op::Bvs, Rel, No, true);
    t[0x71] = e(Op::Adc, IzY, Rd, true);
    t[0x72] = e(Op::Jam, Imp, No, false);
    t[0x73] = e(Op::Rra, IzY, Rmw, false);
    t[0x74] = e(Op::Nop, ZpX, Rd, false);
    t[0x75] = e(Op::Adc, ZpX, Rd, true);
    t[0x76] = e(Op::Ror, ZpX, Rmw, true);
    t[0x77] = e(Op::Rra, ZpX, Rmw, false);
    t[0x78] = e(Op::Sei, Imp, No, true);
    t[0x79] = e(Op::Adc, AbY, Rd, true);
    t[0x7A] = e(Op::Nop, Imp, No, false);
    t[0x7B] = e(Op::Rra, AbY, Rmw, false);
    t[0x7C] = e(Op::Nop, AbX, Rd, false);
    t[0x7D] = e(Op::Adc, AbX, Rd, true);
    t[0x7E] = e(Op::Ror, AbX, Rmw, true);
    t[0x7F] = e(Op::Rra, AbX, Rmw, false);
    t[0x80] = e(Op::Nop, Imm, Rd, false);
    t[0x81] = e(Op::Sta, IzX, Wr, true);
    t[0x82] = e(Op::Nop, Imm, Rd, false);
    t[0x83] = e(Op::Sax, IzX, Wr, false);
    t[0x84] = e(Op::Sty, Zp, Wr, true);
    t[0x85] = e(Op::Sta, Zp, Wr, true);
    t[0x86] = e(Op::Stx, Zp, Wr, true);
    t[0x87] = e(Op::Sax, Zp, Wr, false);
    t[0x88] = e(Op::Dey, Imp, No, true);
    t[0x89] = e(Op::Nop, Imm, Rd, false);
    t[0x8A] = e(Op::Txa, Imp, No, true);
    t[0x8B] = e(Op::Ane, Imm, Rd, false);
    t[0x8C] = e(Op::Sty, Abs, Wr, true);
    t[0x8D] = e(Op::Sta, Abs, Wr, true);
    t[0x8E] = e(Op::Stx, Abs, Wr, true);
    t[0x8F] = e(Op::Sax, Abs, Wr, false);
    t[0x90] = e(Op::Bcc, Rel, No, true);
    t[0x91] = e(Op::Sta, IzY, Wr, true);
    t[0x92] = e(Op::Jam, Imp, No, false);
    t[0x93] = e(Op::Sha, IzY, Wr, false);
    t[0x94] = e(Op::Sty, ZpX, Wr, true);
    t[0x95] = e(Op::Sta, ZpX, Wr, true);
    t[0x96] = e(Op::Stx, ZpY, Wr, true);
    t[0x97] = e(Op::Sax, ZpY, Wr, false);
    t[0x98] = e(Op::Tya, Imp, No, true);
    t[0x99] = e(Op::Sta, AbY, Wr, true);
    t[0x9A] = e(Op::Txs, Imp, No, true);
    t[0x9B] = e(Op::Shs, AbY, Wr, false);
    t[0x9C] = e(Op::Shy, AbX, Wr, false);
    t[0x9D] = e(Op::Sta, AbX, Wr, true);
    t[0x9E] = e(Op::Shx, AbY, Wr, false);
    t[0x9F] = e(Op::Sha, AbY, Wr, false);
    t[0xA0] = e(Op::Ldy, Imm, Rd, true);
    t[0xA1] = e(Op::Lda, IzX, Rd, true);
    t[0xA2] = e(Op::Ldx, Imm, Rd, true);
    t[0xA3] = e(Op::Lax, IzX, Rd, false);
    t[0xA4] = e(Op::Ldy, Zp, Rd, true);
    t[0xA5] = e(Op::Lda, Zp, Rd, true);
    t[0xA6] = e(Op::Ldx, Zp, Rd, true);
    t[0xA7] = e(Op::Lax, Zp, Rd, false);
    t[0xA8] = e(Op::Tay, Imp, No, true);
    t[0xA9] = e(Op::Lda, Imm, Rd, true);
    t[0xAA] = e(Op::Tax, Imp, No, true);
    t[0xAB] = e(Op::Lxa, Imm, Rd, false);
    t[0xAC] = e(Op::Ldy, Abs, Rd, true);
    t[0xAD] = e(Op::Lda, Abs, Rd, true);
    t[0xAE] = e(Op::Ldx, Abs, Rd, true);
    t[0xAF] = e(Op::Lax, Abs, Rd, false);
    t[0xB0] = e(Op::Bcs, Rel, No, true);
    t[0xB1] = e(Op::Lda, IzY, Rd, true);
    t[0xB2] = e(Op::Jam, Imp, No, false);
    t[0xB3] = e(Op::Lax, IzY, Rd, false);
    t[0xB4] = e(Op::Ldy, ZpX, Rd, true);
    t[0xB5] = e(Op::Lda, ZpX, Rd, true);
    t[0xB6] = e(Op::Ldx, ZpY, Rd, true);
    t[0xB7] = e(Op::Lax, ZpY, Rd, false);
    t[0xB8] = e(Op::Clv, Imp, No, true);
    t[0xB9] = e(Op::Lda, AbY, Rd, true);
    t[0xBA] = e(Op::Tsx, Imp, No, true);
    t[0xBB] = e(Op::Las, AbY, Rd, false);
    t[0xBC] = e(Op::Ldy, AbX, Rd, true);
    t[0xBD] = e(Op::Lda, AbX, Rd, true);
    t[0xBE] = e(Op::Ldx, AbY, Rd, true);
    t[0xBF] = e(Op::Lax, AbY, Rd, false);
    t[0xC0] = e(Op::Cpy, Imm, Rd, true);
    t[0xC1] = e(Op::Cmp, IzX, Rd, true);
    t[0xC2] = e(Op::Nop, Imm, Rd, false);
    t[0xC3] = e(Op::Dcp, IzX, Rmw, false);
    t[0xC4] = e(Op::Cpy, Zp, Rd, true);
    t[0xC5] = e(Op::Cmp, Zp, Rd, true);
    t[0xC6] = e(Op::Dec, Zp, Rmw, true);
    t[0xC7] = e(Op::Dcp, Zp, Rmw, false);
    t[0xC8] = e(Op::Iny, Imp, No, true);
    t[0xC9] = e(Op::Cmp, Imm, Rd, true);
    t[0xCA] = e(Op::Dex, Imp, No, true);
    t[0xCB] = e(Op::Sbx, Imm, Rd, false);
    t[0xCC] = e(Op::Cpy, Abs, Rd, true);
    t[0xCD] = e(Op::Cmp, Abs, Rd, true);
    t[0xCE] = e(Op::Dec, Abs, Rmw, true);
    t[0xCF] = e(Op::Dcp, Abs, Rmw, false);
    t[0xD0] = e(Op::Bne, Rel, No, true);
    t[0xD1] = e(Op::Cmp, IzY, Rd, true);
    t[0xD2] = e(Op::Jam, Imp, No, false);
    t[0xD3] = e(Op::Dcp, IzY, Rmw, false);
    t[0xD4] = e(Op::Nop, ZpX, Rd, false);
    t[0xD5] = e(Op::Cmp, ZpX, Rd, true);
    t[0xD6] = e(Op::Dec, ZpX, Rmw, true);
    t[0xD7] = e(Op::Dcp, ZpX, Rmw, false);
    t[0xD8] = e(Op::Cld, Imp, No, true);
    t[0xD9] = e(Op::Cmp, AbY, Rd, true);
    t[0xDA] = e(Op::Nop, Imp, No, false);
    t[0xDB] = e(Op::Dcp, AbY, Rmw, false);
    t[0xDC] = e(Op::Nop, AbX, Rd, false);
    t[0xDD] = e(Op::Cmp, AbX, Rd, true);
    t[0xDE] = e(Op::Dec, AbX, Rmw, true);
    t[0xDF] = e(Op::Dcp, AbX, Rmw, false);
    t[0xE0] = e(Op::Cpx, Imm, Rd, true);
    t[0xE1] = e(Op::Sbc, IzX, Rd, true);
    t[0xE2] = e(Op::Nop, Imm, Rd, false);
    t[0xE3] = e(Op::Isb, IzX, Rmw, false);
    t[0xE4] = e(Op::Cpx, Zp, Rd, true);
    t[0xE5] = e(Op::Sbc, Zp, Rd, true);
    t[0xE6] = e(Op::Inc, Zp, Rmw, true);
    t[0xE7] = e(Op::Isb, Zp, Rmw, false);
    t[0xE8] = e(Op::Inx, Imp, No, true);
    t[0xE9] = e(Op::Sbc, Imm, Rd, true);
    t[0xEA] = e(Op::Nop, Imp, No, true);
    t[0xEB] = e(Op::Sbc, Imm, Rd, false);
    t[0xEC] = e(Op::Cpx, Abs, Rd, true);
    t[0xED] = e(Op::Sbc, Abs, Rd, true);
    t[0xEE] = e(Op::Inc, Abs, Rmw, true);
    t[0xEF] = e(Op::Isb, Abs, Rmw, false);
    t[0xF0] = e(Op::Beq, Rel, No, true);
    t[0xF1] = e(Op::Sbc, IzY, Rd, true);
    t[0xF2] = e(Op::Jam, Imp, No, false);
    t[0xF3] = e(Op::Isb, IzY, Rmw, false);
    t[0xF4] = e(Op::Nop, ZpX, Rd, false);
    t[0xF5] = e(Op::Sbc, ZpX, Rd, true);
    t[0xF6] = e(Op::Inc, ZpX, Rmw, true);
    t[0xF7] = e(Op::Isb, ZpX, Rmw, false);
    t[0xF8] = e(Op::Sed, Imp, No, true);
    t[0xF9] = e(Op::Sbc, AbY, Rd, true);
    t[0xFA] = e(Op::Nop, Imp, No, false);
    t[0xFB] = e(Op::Isb, AbY, Rmw, false);
    t[0xFC] = e(Op::Nop, AbX, Rd, false);
    t[0xFD] = e(Op::Sbc, AbX, Rd, true);
    t[0xFE] = e(Op::Inc, AbX, Rmw, true);
    t[0xFF] = e(Op::Isb, AbX, Rmw, false);
    t
}
