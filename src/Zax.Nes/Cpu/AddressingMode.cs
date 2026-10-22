namespace Zax.Nes.Cpu;

/// <summary>
/// Addressing mode of a 6502 instruction (nesdev, "CPU addressing modes").
/// </summary>
public enum AddressingMode
{
    /// <summary>No operand; the instruction still performs one dummy read.</summary>
    Implied,

    /// <summary>Operates on the accumulator; one dummy read.</summary>
    Accumulator,

    /// <summary>One operand byte, used as the value.</summary>
    Immediate,

    /// <summary>One operand byte, address in page zero.</summary>
    ZeroPage,

    /// <summary>Page-zero address plus X, wrapped inside page zero.</summary>
    ZeroPageX,

    /// <summary>Page-zero address plus Y, wrapped inside page zero.</summary>
    ZeroPageY,

    /// <summary>Two operand bytes forming a 16-bit address.</summary>
    Absolute,

    /// <summary>Absolute address plus X; page crossing costs one cycle on reads.</summary>
    AbsoluteX,

    /// <summary>Absolute address plus Y; page crossing costs one cycle on reads.</summary>
    AbsoluteY,

    /// <summary>Indirect, for <c>JMP</c> only, with the page-boundary bug of the NMOS 6502.</summary>
    Indirect,

    /// <summary>Indexed indirect: the pointer is (operand + X) inside page zero.</summary>
    IndirectX,

    /// <summary>Indirect indexed: the pointer is the operand, then Y is added to the address.</summary>
    IndirectY,

    /// <summary>Signed 8-bit branch displacement.</summary>
    Relative,
}
