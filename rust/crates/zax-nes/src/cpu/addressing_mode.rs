/// Addressing mode of a 6502 instruction (nesdev, "CPU addressing modes").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressingMode {
    /// No operand; the instruction still performs one dummy read.
    Implied,
    /// Operates on the accumulator; one dummy read.
    Accumulator,
    /// One operand byte, used as the value.
    Immediate,
    /// One operand byte, address in page zero.
    ZeroPage,
    /// Page-zero address plus X, wrapped inside page zero.
    ZeroPageX,
    /// Page-zero address plus Y, wrapped inside page zero.
    ZeroPageY,
    /// Two operand bytes forming a 16-bit address.
    Absolute,
    /// Absolute address plus X; page crossing costs one cycle on reads.
    AbsoluteX,
    /// Absolute address plus Y; page crossing costs one cycle on reads.
    AbsoluteY,
    /// Indirect, for `JMP` only, with the page-boundary bug of the NMOS 6502.
    Indirect,
    /// Indexed indirect: the pointer is (operand + X) inside page zero.
    IndirectX,
    /// Indirect indexed: the pointer is the operand, then Y is added to the address.
    IndirectY,
    /// Signed 8-bit branch displacement.
    Relative,
}
