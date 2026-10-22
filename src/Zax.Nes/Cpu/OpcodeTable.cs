namespace Zax.Nes.Cpu;

/// <summary>
/// How an instruction touches its operand, which decides the dummy cycles of the
/// addressing mode (nesdev, "6502 instruction reference" and "CPU unofficial opcodes").
/// </summary>
public enum OperandAccess
{
    /// <summary>No memory operand: implied, accumulator, branch, jump or stack.</summary>
    None,

    /// <summary>Reads the operand. Indexed modes cost one extra cycle only on a page cross.</summary>
    Read,

    /// <summary>Writes the operand. Indexed modes always pay the extra cycle.</summary>
    Write,

    /// <summary>Reads, modifies and writes back. One dummy write, and always the extra cycle.</summary>
    ReadModifyWrite,
}

/// <summary>Static description of one of the 256 opcodes.</summary>
/// <param name="Operation">The operation performed.</param>
/// <param name="Mode">The addressing mode.</param>
/// <param name="Access">How the operand is touched.</param>
/// <param name="Official">
/// <see langword="false"/> for the unofficial opcodes, which <c>nestest.log</c> prefixes
/// with <c>*</c>.
/// </param>
public readonly record struct OpcodeInfo(
    CpuOperation Operation,
    AddressingMode Mode,
    OperandAccess Access,
    bool Official)
{
    /// <summary>Total length of the instruction in bytes, opcode included.</summary>
    public int Length => Mode switch
    {
        AddressingMode.Implied or AddressingMode.Accumulator => 1,
        AddressingMode.Immediate or AddressingMode.Relative
            or AddressingMode.ZeroPage or AddressingMode.ZeroPageX or AddressingMode.ZeroPageY
            or AddressingMode.IndirectX or AddressingMode.IndirectY => 2,
        _ => 3,
    };

    /// <summary>Three-letter mnemonic, upper case.</summary>
    public string Mnemonic => Operation switch
    {
        CpuOperation.Isb => "ISB",
        CpuOperation.Jam => "JAM",
        _ => Operation.ToString().ToUpperInvariant(),
    };
}

/// <summary>
/// The opcode matrix of the 2A03's 6502 core: all 151 official opcodes and all 105
/// unofficial ones (REQ-CORE-110).
/// </summary>
/// <remarks>
/// <para>
/// Built from the public nesdev documentation ("6502 instruction reference", "CPU
/// unofficial opcodes") and the mnemonics of the public <c>nestest.log</c>. The table is
/// the single source of truth: <see cref="Cpu6502"/> derives both the cycle count and the
/// operand fetch from the addressing mode and <see cref="OperandAccess"/>, and
/// <see cref="CpuTracer"/> derives the disassembly from the same entries, so the two
/// cannot drift apart.
/// </para>
/// <para>
/// Cycle counts are not stored: they follow from the explicit bus accesses that the
/// addressing mode performs, including the dummy reads of indexed modes and the dummy
/// write of a read-modify-write (see <see cref="Cpu6502"/>).
/// </para>
/// </remarks>
public static class OpcodeTable
{
    private static readonly OpcodeInfo[] Entries = BuildEntries();

    /// <summary>Returns the description of <paramref name="opcode"/>.</summary>
    public static OpcodeInfo Get(byte opcode) => Entries[opcode];

    /// <summary>All 256 entries, indexed by opcode.</summary>
    public static ReadOnlySpan<OpcodeInfo> All => Entries;

    private static OpcodeInfo[] BuildEntries()
    {
        var table = new OpcodeInfo[256];

        // Every slot is filled below; the matrix of the NMOS 6502 has no gap once the
        // unofficial opcodes are included.
        void Set(byte opcode, CpuOperation op, AddressingMode mode, OperandAccess access, bool official = true)
            => table[opcode] = new OpcodeInfo(op, mode, access, official);

        const AddressingMode Imp = AddressingMode.Implied;
        const AddressingMode Acc = AddressingMode.Accumulator;
        const AddressingMode Imm = AddressingMode.Immediate;
        const AddressingMode Zp = AddressingMode.ZeroPage;
        const AddressingMode ZpX = AddressingMode.ZeroPageX;
        const AddressingMode ZpY = AddressingMode.ZeroPageY;
        const AddressingMode Abs = AddressingMode.Absolute;
        const AddressingMode AbX = AddressingMode.AbsoluteX;
        const AddressingMode AbY = AddressingMode.AbsoluteY;
        const AddressingMode Ind = AddressingMode.Indirect;
        const AddressingMode IzX = AddressingMode.IndirectX;
        const AddressingMode IzY = AddressingMode.IndirectY;
        const AddressingMode Rel = AddressingMode.Relative;

        const OperandAccess No = OperandAccess.None;
        const OperandAccess Rd = OperandAccess.Read;
        const OperandAccess Wr = OperandAccess.Write;
        const OperandAccess Rmw = OperandAccess.ReadModifyWrite;

        const bool U = false;

        // Row $00
        Set(0x00, CpuOperation.Brk, Imp, No);
        Set(0x01, CpuOperation.Ora, IzX, Rd);
        Set(0x02, CpuOperation.Jam, Imp, No, U);
        Set(0x03, CpuOperation.Slo, IzX, Rmw, U);
        Set(0x04, CpuOperation.Nop, Zp, Rd, U);
        Set(0x05, CpuOperation.Ora, Zp, Rd);
        Set(0x06, CpuOperation.Asl, Zp, Rmw);
        Set(0x07, CpuOperation.Slo, Zp, Rmw, U);
        Set(0x08, CpuOperation.Php, Imp, No);
        Set(0x09, CpuOperation.Ora, Imm, Rd);
        Set(0x0A, CpuOperation.Asl, Acc, No);
        Set(0x0B, CpuOperation.Anc, Imm, Rd, U);
        Set(0x0C, CpuOperation.Nop, Abs, Rd, U);
        Set(0x0D, CpuOperation.Ora, Abs, Rd);
        Set(0x0E, CpuOperation.Asl, Abs, Rmw);
        Set(0x0F, CpuOperation.Slo, Abs, Rmw, U);

        // Row $10
        Set(0x10, CpuOperation.Bpl, Rel, No);
        Set(0x11, CpuOperation.Ora, IzY, Rd);
        Set(0x12, CpuOperation.Jam, Imp, No, U);
        Set(0x13, CpuOperation.Slo, IzY, Rmw, U);
        Set(0x14, CpuOperation.Nop, ZpX, Rd, U);
        Set(0x15, CpuOperation.Ora, ZpX, Rd);
        Set(0x16, CpuOperation.Asl, ZpX, Rmw);
        Set(0x17, CpuOperation.Slo, ZpX, Rmw, U);
        Set(0x18, CpuOperation.Clc, Imp, No);
        Set(0x19, CpuOperation.Ora, AbY, Rd);
        Set(0x1A, CpuOperation.Nop, Imp, No, U);
        Set(0x1B, CpuOperation.Slo, AbY, Rmw, U);
        Set(0x1C, CpuOperation.Nop, AbX, Rd, U);
        Set(0x1D, CpuOperation.Ora, AbX, Rd);
        Set(0x1E, CpuOperation.Asl, AbX, Rmw);
        Set(0x1F, CpuOperation.Slo, AbX, Rmw, U);

        // Row $20
        Set(0x20, CpuOperation.Jsr, Abs, No);
        Set(0x21, CpuOperation.And, IzX, Rd);
        Set(0x22, CpuOperation.Jam, Imp, No, U);
        Set(0x23, CpuOperation.Rla, IzX, Rmw, U);
        Set(0x24, CpuOperation.Bit, Zp, Rd);
        Set(0x25, CpuOperation.And, Zp, Rd);
        Set(0x26, CpuOperation.Rol, Zp, Rmw);
        Set(0x27, CpuOperation.Rla, Zp, Rmw, U);
        Set(0x28, CpuOperation.Plp, Imp, No);
        Set(0x29, CpuOperation.And, Imm, Rd);
        Set(0x2A, CpuOperation.Rol, Acc, No);
        Set(0x2B, CpuOperation.Anc, Imm, Rd, U);
        Set(0x2C, CpuOperation.Bit, Abs, Rd);
        Set(0x2D, CpuOperation.And, Abs, Rd);
        Set(0x2E, CpuOperation.Rol, Abs, Rmw);
        Set(0x2F, CpuOperation.Rla, Abs, Rmw, U);

        // Row $30
        Set(0x30, CpuOperation.Bmi, Rel, No);
        Set(0x31, CpuOperation.And, IzY, Rd);
        Set(0x32, CpuOperation.Jam, Imp, No, U);
        Set(0x33, CpuOperation.Rla, IzY, Rmw, U);
        Set(0x34, CpuOperation.Nop, ZpX, Rd, U);
        Set(0x35, CpuOperation.And, ZpX, Rd);
        Set(0x36, CpuOperation.Rol, ZpX, Rmw);
        Set(0x37, CpuOperation.Rla, ZpX, Rmw, U);
        Set(0x38, CpuOperation.Sec, Imp, No);
        Set(0x39, CpuOperation.And, AbY, Rd);
        Set(0x3A, CpuOperation.Nop, Imp, No, U);
        Set(0x3B, CpuOperation.Rla, AbY, Rmw, U);
        Set(0x3C, CpuOperation.Nop, AbX, Rd, U);
        Set(0x3D, CpuOperation.And, AbX, Rd);
        Set(0x3E, CpuOperation.Rol, AbX, Rmw);
        Set(0x3F, CpuOperation.Rla, AbX, Rmw, U);

        // Row $40
        Set(0x40, CpuOperation.Rti, Imp, No);
        Set(0x41, CpuOperation.Eor, IzX, Rd);
        Set(0x42, CpuOperation.Jam, Imp, No, U);
        Set(0x43, CpuOperation.Sre, IzX, Rmw, U);
        Set(0x44, CpuOperation.Nop, Zp, Rd, U);
        Set(0x45, CpuOperation.Eor, Zp, Rd);
        Set(0x46, CpuOperation.Lsr, Zp, Rmw);
        Set(0x47, CpuOperation.Sre, Zp, Rmw, U);
        Set(0x48, CpuOperation.Pha, Imp, No);
        Set(0x49, CpuOperation.Eor, Imm, Rd);
        Set(0x4A, CpuOperation.Lsr, Acc, No);
        Set(0x4B, CpuOperation.Alr, Imm, Rd, U);
        Set(0x4C, CpuOperation.Jmp, Abs, No);
        Set(0x4D, CpuOperation.Eor, Abs, Rd);
        Set(0x4E, CpuOperation.Lsr, Abs, Rmw);
        Set(0x4F, CpuOperation.Sre, Abs, Rmw, U);

        // Row $50
        Set(0x50, CpuOperation.Bvc, Rel, No);
        Set(0x51, CpuOperation.Eor, IzY, Rd);
        Set(0x52, CpuOperation.Jam, Imp, No, U);
        Set(0x53, CpuOperation.Sre, IzY, Rmw, U);
        Set(0x54, CpuOperation.Nop, ZpX, Rd, U);
        Set(0x55, CpuOperation.Eor, ZpX, Rd);
        Set(0x56, CpuOperation.Lsr, ZpX, Rmw);
        Set(0x57, CpuOperation.Sre, ZpX, Rmw, U);
        Set(0x58, CpuOperation.Cli, Imp, No);
        Set(0x59, CpuOperation.Eor, AbY, Rd);
        Set(0x5A, CpuOperation.Nop, Imp, No, U);
        Set(0x5B, CpuOperation.Sre, AbY, Rmw, U);
        Set(0x5C, CpuOperation.Nop, AbX, Rd, U);
        Set(0x5D, CpuOperation.Eor, AbX, Rd);
        Set(0x5E, CpuOperation.Lsr, AbX, Rmw);
        Set(0x5F, CpuOperation.Sre, AbX, Rmw, U);

        // Row $60
        Set(0x60, CpuOperation.Rts, Imp, No);
        Set(0x61, CpuOperation.Adc, IzX, Rd);
        Set(0x62, CpuOperation.Jam, Imp, No, U);
        Set(0x63, CpuOperation.Rra, IzX, Rmw, U);
        Set(0x64, CpuOperation.Nop, Zp, Rd, U);
        Set(0x65, CpuOperation.Adc, Zp, Rd);
        Set(0x66, CpuOperation.Ror, Zp, Rmw);
        Set(0x67, CpuOperation.Rra, Zp, Rmw, U);
        Set(0x68, CpuOperation.Pla, Imp, No);
        Set(0x69, CpuOperation.Adc, Imm, Rd);
        Set(0x6A, CpuOperation.Ror, Acc, No);
        Set(0x6B, CpuOperation.Arr, Imm, Rd, U);
        Set(0x6C, CpuOperation.Jmp, Ind, No);
        Set(0x6D, CpuOperation.Adc, Abs, Rd);
        Set(0x6E, CpuOperation.Ror, Abs, Rmw);
        Set(0x6F, CpuOperation.Rra, Abs, Rmw, U);

        // Row $70
        Set(0x70, CpuOperation.Bvs, Rel, No);
        Set(0x71, CpuOperation.Adc, IzY, Rd);
        Set(0x72, CpuOperation.Jam, Imp, No, U);
        Set(0x73, CpuOperation.Rra, IzY, Rmw, U);
        Set(0x74, CpuOperation.Nop, ZpX, Rd, U);
        Set(0x75, CpuOperation.Adc, ZpX, Rd);
        Set(0x76, CpuOperation.Ror, ZpX, Rmw);
        Set(0x77, CpuOperation.Rra, ZpX, Rmw, U);
        Set(0x78, CpuOperation.Sei, Imp, No);
        Set(0x79, CpuOperation.Adc, AbY, Rd);
        Set(0x7A, CpuOperation.Nop, Imp, No, U);
        Set(0x7B, CpuOperation.Rra, AbY, Rmw, U);
        Set(0x7C, CpuOperation.Nop, AbX, Rd, U);
        Set(0x7D, CpuOperation.Adc, AbX, Rd);
        Set(0x7E, CpuOperation.Ror, AbX, Rmw);
        Set(0x7F, CpuOperation.Rra, AbX, Rmw, U);

        // Row $80
        Set(0x80, CpuOperation.Nop, Imm, Rd, U);
        Set(0x81, CpuOperation.Sta, IzX, Wr);
        Set(0x82, CpuOperation.Nop, Imm, Rd, U);
        Set(0x83, CpuOperation.Sax, IzX, Wr, U);
        Set(0x84, CpuOperation.Sty, Zp, Wr);
        Set(0x85, CpuOperation.Sta, Zp, Wr);
        Set(0x86, CpuOperation.Stx, Zp, Wr);
        Set(0x87, CpuOperation.Sax, Zp, Wr, U);
        Set(0x88, CpuOperation.Dey, Imp, No);
        Set(0x89, CpuOperation.Nop, Imm, Rd, U);
        Set(0x8A, CpuOperation.Txa, Imp, No);
        Set(0x8B, CpuOperation.Ane, Imm, Rd, U);
        Set(0x8C, CpuOperation.Sty, Abs, Wr);
        Set(0x8D, CpuOperation.Sta, Abs, Wr);
        Set(0x8E, CpuOperation.Stx, Abs, Wr);
        Set(0x8F, CpuOperation.Sax, Abs, Wr, U);

        // Row $90
        Set(0x90, CpuOperation.Bcc, Rel, No);
        Set(0x91, CpuOperation.Sta, IzY, Wr);
        Set(0x92, CpuOperation.Jam, Imp, No, U);
        Set(0x93, CpuOperation.Sha, IzY, Wr, U);
        Set(0x94, CpuOperation.Sty, ZpX, Wr);
        Set(0x95, CpuOperation.Sta, ZpX, Wr);
        Set(0x96, CpuOperation.Stx, ZpY, Wr);
        Set(0x97, CpuOperation.Sax, ZpY, Wr, U);
        Set(0x98, CpuOperation.Tya, Imp, No);
        Set(0x99, CpuOperation.Sta, AbY, Wr);
        Set(0x9A, CpuOperation.Txs, Imp, No);
        Set(0x9B, CpuOperation.Shs, AbY, Wr, U);
        Set(0x9C, CpuOperation.Shy, AbX, Wr, U);
        Set(0x9D, CpuOperation.Sta, AbX, Wr);
        Set(0x9E, CpuOperation.Shx, AbY, Wr, U);
        Set(0x9F, CpuOperation.Sha, AbY, Wr, U);

        // Row $A0
        Set(0xA0, CpuOperation.Ldy, Imm, Rd);
        Set(0xA1, CpuOperation.Lda, IzX, Rd);
        Set(0xA2, CpuOperation.Ldx, Imm, Rd);
        Set(0xA3, CpuOperation.Lax, IzX, Rd, U);
        Set(0xA4, CpuOperation.Ldy, Zp, Rd);
        Set(0xA5, CpuOperation.Lda, Zp, Rd);
        Set(0xA6, CpuOperation.Ldx, Zp, Rd);
        Set(0xA7, CpuOperation.Lax, Zp, Rd, U);
        Set(0xA8, CpuOperation.Tay, Imp, No);
        Set(0xA9, CpuOperation.Lda, Imm, Rd);
        Set(0xAA, CpuOperation.Tax, Imp, No);
        Set(0xAB, CpuOperation.Lxa, Imm, Rd, U);
        Set(0xAC, CpuOperation.Ldy, Abs, Rd);
        Set(0xAD, CpuOperation.Lda, Abs, Rd);
        Set(0xAE, CpuOperation.Ldx, Abs, Rd);
        Set(0xAF, CpuOperation.Lax, Abs, Rd, U);

        // Row $B0
        Set(0xB0, CpuOperation.Bcs, Rel, No);
        Set(0xB1, CpuOperation.Lda, IzY, Rd);
        Set(0xB2, CpuOperation.Jam, Imp, No, U);
        Set(0xB3, CpuOperation.Lax, IzY, Rd, U);
        Set(0xB4, CpuOperation.Ldy, ZpX, Rd);
        Set(0xB5, CpuOperation.Lda, ZpX, Rd);
        Set(0xB6, CpuOperation.Ldx, ZpY, Rd);
        Set(0xB7, CpuOperation.Lax, ZpY, Rd, U);
        Set(0xB8, CpuOperation.Clv, Imp, No);
        Set(0xB9, CpuOperation.Lda, AbY, Rd);
        Set(0xBA, CpuOperation.Tsx, Imp, No);
        Set(0xBB, CpuOperation.Las, AbY, Rd, U);
        Set(0xBC, CpuOperation.Ldy, AbX, Rd);
        Set(0xBD, CpuOperation.Lda, AbX, Rd);
        Set(0xBE, CpuOperation.Ldx, AbY, Rd);
        Set(0xBF, CpuOperation.Lax, AbY, Rd, U);

        // Row $C0
        Set(0xC0, CpuOperation.Cpy, Imm, Rd);
        Set(0xC1, CpuOperation.Cmp, IzX, Rd);
        Set(0xC2, CpuOperation.Nop, Imm, Rd, U);
        Set(0xC3, CpuOperation.Dcp, IzX, Rmw, U);
        Set(0xC4, CpuOperation.Cpy, Zp, Rd);
        Set(0xC5, CpuOperation.Cmp, Zp, Rd);
        Set(0xC6, CpuOperation.Dec, Zp, Rmw);
        Set(0xC7, CpuOperation.Dcp, Zp, Rmw, U);
        Set(0xC8, CpuOperation.Iny, Imp, No);
        Set(0xC9, CpuOperation.Cmp, Imm, Rd);
        Set(0xCA, CpuOperation.Dex, Imp, No);
        Set(0xCB, CpuOperation.Sbx, Imm, Rd, U);
        Set(0xCC, CpuOperation.Cpy, Abs, Rd);
        Set(0xCD, CpuOperation.Cmp, Abs, Rd);
        Set(0xCE, CpuOperation.Dec, Abs, Rmw);
        Set(0xCF, CpuOperation.Dcp, Abs, Rmw, U);

        // Row $D0
        Set(0xD0, CpuOperation.Bne, Rel, No);
        Set(0xD1, CpuOperation.Cmp, IzY, Rd);
        Set(0xD2, CpuOperation.Jam, Imp, No, U);
        Set(0xD3, CpuOperation.Dcp, IzY, Rmw, U);
        Set(0xD4, CpuOperation.Nop, ZpX, Rd, U);
        Set(0xD5, CpuOperation.Cmp, ZpX, Rd);
        Set(0xD6, CpuOperation.Dec, ZpX, Rmw);
        Set(0xD7, CpuOperation.Dcp, ZpX, Rmw, U);
        Set(0xD8, CpuOperation.Cld, Imp, No);
        Set(0xD9, CpuOperation.Cmp, AbY, Rd);
        Set(0xDA, CpuOperation.Nop, Imp, No, U);
        Set(0xDB, CpuOperation.Dcp, AbY, Rmw, U);
        Set(0xDC, CpuOperation.Nop, AbX, Rd, U);
        Set(0xDD, CpuOperation.Cmp, AbX, Rd);
        Set(0xDE, CpuOperation.Dec, AbX, Rmw);
        Set(0xDF, CpuOperation.Dcp, AbX, Rmw, U);

        // Row $E0
        Set(0xE0, CpuOperation.Cpx, Imm, Rd);
        Set(0xE1, CpuOperation.Sbc, IzX, Rd);
        Set(0xE2, CpuOperation.Nop, Imm, Rd, U);
        Set(0xE3, CpuOperation.Isb, IzX, Rmw, U);
        Set(0xE4, CpuOperation.Cpx, Zp, Rd);
        Set(0xE5, CpuOperation.Sbc, Zp, Rd);
        Set(0xE6, CpuOperation.Inc, Zp, Rmw);
        Set(0xE7, CpuOperation.Isb, Zp, Rmw, U);
        Set(0xE8, CpuOperation.Inx, Imp, No);
        Set(0xE9, CpuOperation.Sbc, Imm, Rd);
        Set(0xEA, CpuOperation.Nop, Imp, No);
        Set(0xEB, CpuOperation.Sbc, Imm, Rd, U);
        Set(0xEC, CpuOperation.Cpx, Abs, Rd);
        Set(0xED, CpuOperation.Sbc, Abs, Rd);
        Set(0xEE, CpuOperation.Inc, Abs, Rmw);
        Set(0xEF, CpuOperation.Isb, Abs, Rmw, U);

        // Row $F0
        Set(0xF0, CpuOperation.Beq, Rel, No);
        Set(0xF1, CpuOperation.Sbc, IzY, Rd);
        Set(0xF2, CpuOperation.Jam, Imp, No, U);
        Set(0xF3, CpuOperation.Isb, IzY, Rmw, U);
        Set(0xF4, CpuOperation.Nop, ZpX, Rd, U);
        Set(0xF5, CpuOperation.Sbc, ZpX, Rd);
        Set(0xF6, CpuOperation.Inc, ZpX, Rmw);
        Set(0xF7, CpuOperation.Isb, ZpX, Rmw, U);
        Set(0xF8, CpuOperation.Sed, Imp, No);
        Set(0xF9, CpuOperation.Sbc, AbY, Rd);
        Set(0xFA, CpuOperation.Nop, Imp, No, U);
        Set(0xFB, CpuOperation.Isb, AbY, Rmw, U);
        Set(0xFC, CpuOperation.Nop, AbX, Rd, U);
        Set(0xFD, CpuOperation.Sbc, AbX, Rd);
        Set(0xFE, CpuOperation.Inc, AbX, Rmw);
        Set(0xFF, CpuOperation.Isb, AbX, Rmw, U);

        return table;
    }
}
