namespace Zax.Nes.Cpu;

/// <summary>
/// Operation performed by an opcode, independently of its addressing mode.
/// </summary>
/// <remarks>
/// The unofficial names follow the mnemonics of the public <c>nestest.log</c> reference
/// trace (<c>SLO</c>, <c>RLA</c>, <c>SRE</c>, <c>RRA</c>, <c>SAX</c>, <c>LAX</c>,
/// <c>DCP</c>, <c>ISB</c>) and, for the ones nestest does not exercise, the names of the
/// public "NMOS 6510 unintended opcodes" documentation.
/// </remarks>
public enum CpuOperation
{
    // Load and store.
    Lda, Ldx, Ldy, Sta, Stx, Sty,

    // Transfers.
    Tax, Tay, Txa, Tya, Tsx, Txs,

    // Stack.
    Pha, Php, Pla, Plp,

    // Arithmetic and logic.
    Adc, Sbc, And, Ora, Eor, Cmp, Cpx, Cpy, Bit,

    // Increment and decrement.
    Inc, Dec, Inx, Iny, Dex, Dey,

    // Shifts and rotates.
    Asl, Lsr, Rol, Ror,

    // Jumps, calls and branches.
    Jmp, Jsr, Rts, Rti, Brk,
    Bcc, Bcs, Beq, Bmi, Bne, Bpl, Bvc, Bvs,

    // Flags.
    Clc, Cld, Cli, Clv, Sec, Sed, Sei,

    /// <summary>No operation, official (<c>$EA</c>) or unofficial.</summary>
    Nop,

    // Unofficial: combined read-modify-write operations.
    /// <summary><c>ASL</c> then <c>ORA</c>.</summary>
    Slo,

    /// <summary><c>ROL</c> then <c>AND</c>.</summary>
    Rla,

    /// <summary><c>LSR</c> then <c>EOR</c>.</summary>
    Sre,

    /// <summary><c>ROR</c> then <c>ADC</c>.</summary>
    Rra,

    /// <summary><c>DEC</c> then <c>CMP</c>.</summary>
    Dcp,

    /// <summary><c>INC</c> then <c>SBC</c>.</summary>
    Isb,

    // Unofficial: combined loads and stores.
    /// <summary>Loads A and X with the same value.</summary>
    Lax,

    /// <summary>Stores <c>A &amp; X</c>.</summary>
    Sax,

    // Unofficial: immediate operations.
    /// <summary><c>AND</c> with the carry copied from bit 7 of the result.</summary>
    Anc,

    /// <summary><c>AND</c> then <c>LSR</c>.</summary>
    Alr,

    /// <summary><c>AND</c> then <c>ROR</c>, with adder-like carry and overflow.</summary>
    Arr,

    /// <summary><c>(A &amp; X) - immediate</c> into X.</summary>
    Sbx,

    /// <summary>Unstable: <c>A = (A | magic) &amp; X &amp; immediate</c>.</summary>
    Ane,

    /// <summary>Unstable: <c>A = X = (A | magic) &amp; immediate</c>.</summary>
    Lxa,

    // Unofficial: unstable stores and the stack-pointer oddities.
    /// <summary>Unstable: stores <c>A &amp; X &amp; (high byte of the address + 1)</c>.</summary>
    Sha,

    /// <summary>Unstable: stores <c>X &amp; (high byte of the address + 1)</c>.</summary>
    Shx,

    /// <summary>Unstable: stores <c>Y &amp; (high byte of the address + 1)</c>.</summary>
    Shy,

    /// <summary>Unstable: <c>S = A &amp; X</c>, then stores <c>S &amp; (high byte + 1)</c>.</summary>
    Shs,

    /// <summary>Unstable: <c>A = X = S = S &amp; memory</c>.</summary>
    Las,

    /// <summary>Halts the CPU until reset (also known as KIL or HLT).</summary>
    Jam,
}
