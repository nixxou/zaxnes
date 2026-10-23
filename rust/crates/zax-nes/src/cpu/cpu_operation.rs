/// Operation performed by an opcode, independently of its addressing mode.
///
/// The unofficial names follow the mnemonics of the public `nestest.log` reference trace
/// (`SLO`, `RLA`, `SRE`, `RRA`, `SAX`, `LAX`, `DCP`, `ISB`) and, for the ones nestest does
/// not exercise, the names of the public "NMOS 6510 unintended opcodes" documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CpuOperation {
    // Load and store.
    Lda,
    Ldx,
    Ldy,
    Sta,
    Stx,
    Sty,
    // Transfers.
    Tax,
    Tay,
    Txa,
    Tya,
    Tsx,
    Txs,
    // Stack.
    Pha,
    Php,
    Pla,
    Plp,
    // Arithmetic and logic.
    Adc,
    Sbc,
    And,
    Ora,
    Eor,
    Cmp,
    Cpx,
    Cpy,
    Bit,
    // Increment and decrement.
    Inc,
    Dec,
    Inx,
    Iny,
    Dex,
    Dey,
    // Shifts and rotates.
    Asl,
    Lsr,
    Rol,
    Ror,
    // Jumps, calls and branches.
    Jmp,
    Jsr,
    Rts,
    Rti,
    Brk,
    Bcc,
    Bcs,
    Beq,
    Bmi,
    Bne,
    Bpl,
    Bvc,
    Bvs,
    // Flags.
    Clc,
    Cld,
    Cli,
    Clv,
    Sec,
    Sed,
    Sei,
    /// No operation, official (`$EA`) or unofficial.
    Nop,
    /// Unofficial: `ASL` then `ORA`.
    Slo,
    /// Unofficial: `ROL` then `AND`.
    Rla,
    /// Unofficial: `LSR` then `EOR`.
    Sre,
    /// Unofficial: `ROR` then `ADC`.
    Rra,
    /// Unofficial: `DEC` then `CMP`.
    Dcp,
    /// Unofficial: `INC` then `SBC`.
    Isb,
    /// Unofficial: loads A and X with the same value.
    Lax,
    /// Unofficial: stores `A & X`.
    Sax,
    /// Unofficial: `AND` with the carry copied from bit 7 of the result.
    Anc,
    /// Unofficial: `AND` then `LSR`.
    Alr,
    /// Unofficial: `AND` then `ROR`, with adder-like carry and overflow.
    Arr,
    /// Unofficial: `(A & X) - immediate` into X.
    Sbx,
    /// Unstable: `A = (A | magic) & X & immediate`.
    Ane,
    /// Unstable: `A = X = (A | magic) & immediate`.
    Lxa,
    /// Unstable: stores `A & X & (high byte of the address + 1)`.
    Sha,
    /// Unstable: stores `X & (high byte of the address + 1)`.
    Shx,
    /// Unstable: stores `Y & (high byte of the address + 1)`.
    Shy,
    /// Unstable: `S = A & X`, then stores `S & (high byte + 1)`.
    Shs,
    /// Unstable: `A = X = S = S & memory`.
    Las,
    /// Halts the CPU until reset (also known as KIL or HLT).
    Jam,
}

impl CpuOperation {
    /// Three-letter mnemonic, upper case.
    #[must_use]
    pub const fn mnemonic(self) -> &'static str {
        match self {
            Self::Lda => "LDA",
            Self::Ldx => "LDX",
            Self::Ldy => "LDY",
            Self::Sta => "STA",
            Self::Stx => "STX",
            Self::Sty => "STY",
            Self::Tax => "TAX",
            Self::Tay => "TAY",
            Self::Txa => "TXA",
            Self::Tya => "TYA",
            Self::Tsx => "TSX",
            Self::Txs => "TXS",
            Self::Pha => "PHA",
            Self::Php => "PHP",
            Self::Pla => "PLA",
            Self::Plp => "PLP",
            Self::Adc => "ADC",
            Self::Sbc => "SBC",
            Self::And => "AND",
            Self::Ora => "ORA",
            Self::Eor => "EOR",
            Self::Cmp => "CMP",
            Self::Cpx => "CPX",
            Self::Cpy => "CPY",
            Self::Bit => "BIT",
            Self::Inc => "INC",
            Self::Dec => "DEC",
            Self::Inx => "INX",
            Self::Iny => "INY",
            Self::Dex => "DEX",
            Self::Dey => "DEY",
            Self::Asl => "ASL",
            Self::Lsr => "LSR",
            Self::Rol => "ROL",
            Self::Ror => "ROR",
            Self::Jmp => "JMP",
            Self::Jsr => "JSR",
            Self::Rts => "RTS",
            Self::Rti => "RTI",
            Self::Brk => "BRK",
            Self::Bcc => "BCC",
            Self::Bcs => "BCS",
            Self::Beq => "BEQ",
            Self::Bmi => "BMI",
            Self::Bne => "BNE",
            Self::Bpl => "BPL",
            Self::Bvc => "BVC",
            Self::Bvs => "BVS",
            Self::Clc => "CLC",
            Self::Cld => "CLD",
            Self::Cli => "CLI",
            Self::Clv => "CLV",
            Self::Sec => "SEC",
            Self::Sed => "SED",
            Self::Sei => "SEI",
            Self::Nop => "NOP",
            Self::Slo => "SLO",
            Self::Rla => "RLA",
            Self::Sre => "SRE",
            Self::Rra => "RRA",
            Self::Dcp => "DCP",
            Self::Isb => "ISB",
            Self::Lax => "LAX",
            Self::Sax => "SAX",
            Self::Anc => "ANC",
            Self::Alr => "ALR",
            Self::Arr => "ARR",
            Self::Sbx => "SBX",
            Self::Ane => "ANE",
            Self::Lxa => "LXA",
            Self::Sha => "SHA",
            Self::Shx => "SHX",
            Self::Shy => "SHY",
            Self::Shs => "SHS",
            Self::Las => "LAS",
            Self::Jam => "JAM",
        }
    }
}
