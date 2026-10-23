use std::fmt::Write as _;

use super::{AddressingMode, Cpu6502, CpuBus, CpuOperation, OpcodeInfo, OpcodeTable};

/// One line of CPU trace: the machine state observed just before an instruction executes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTraceLine {
    /// Program counter.
    pub pc: u16,
    /// The instruction's bytes; only the first [`CpuTraceLine::length`] are meaningful.
    pub bytes: [u8; 3],
    /// Number of instruction bytes, 1 to 3.
    pub length: usize,
    /// Accumulator.
    pub a: u8,
    /// Index X.
    pub x: u8,
    /// Index Y.
    pub y: u8,
    /// Status register, with bit 5 set and bit 4 clear.
    pub p: u8,
    /// Stack pointer.
    pub s: u8,
    /// Master CPU cycle count (the `CYC` column).
    pub cycles: i64,
    /// Human-readable form, for diagnostics only.
    pub disassembly: String,
}

impl CpuTraceLine {
    /// The instruction's bytes, 1 to 3 of them.
    #[must_use]
    pub fn instruction_bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }

    /// Formats the line the way `nestest.log` does, minus the `PPU` column.
    #[must_use]
    pub fn format(&self) -> String {
        let mut text = String::with_capacity(96);
        let _ = write!(text, "{:04X}  ", self.pc);

        for i in 0..3 {
            if i < self.length {
                let _ = write!(text, "{:02X}", self.bytes[i]);
            } else {
                text.push_str("  ");
            }
            text.push(' ');
        }

        let _ = write!(
            text,
            " {:<32}A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
            self.disassembly, self.a, self.x, self.y, self.p, self.s, self.cycles
        );
        text
    }
}

/// Produces [`CpuTraceLine`] values in the format of the public `nestest.log`, so that a
/// run can be compared line by line with it (REQ-CORE-110).
///
/// Every read it performs goes through [`CpuBus::peek`]: tracing must not change what the
/// emulation does (REQ-CORE-081), and the comparison would be meaningless otherwise.
pub struct CpuTracer;

impl CpuTracer {
    /// Captures the state of `cpu` before its next instruction.
    #[must_use]
    pub fn capture<B: CpuBus + ?Sized>(cpu: &Cpu6502, bus: &B) -> CpuTraceLine {
        let pc = cpu.pc;
        let info = OpcodeTable::get(bus.peek(pc));
        let length = info.length();

        let mut bytes = [0u8; 3];
        for (i, byte) in (0u16..).zip(bytes.iter_mut().take(length)) {
            *byte = bus.peek(pc.wrapping_add(i));
        }

        CpuTraceLine {
            pc,
            bytes,
            length,
            a: cpu.a,
            x: cpu.x,
            y: cpu.y,
            p: cpu.p(),
            s: cpu.s,
            cycles: bus.cycle_count(),
            disassembly: Self::disassemble(cpu, bus, pc, info, &bytes[..length]),
        }
    }

    /// Renders the instruction at `pc` the way `nestest.log` does, resolved operand and
    /// pointed-at value included.
    #[must_use]
    pub fn disassemble<B: CpuBus + ?Sized>(
        cpu: &Cpu6502,
        bus: &B,
        pc: u16,
        info: OpcodeInfo,
        bytes: &[u8],
    ) -> String {
        let star = if info.official { ' ' } else { '*' };
        let name = info.mnemonic();
        let lo = bytes.get(1).copied().unwrap_or(0);
        let hi = bytes.get(2).copied().unwrap_or(0);
        let absolute = u16::from_le_bytes([lo, hi]);

        match info.mode {
            AddressingMode::Implied => format!("{star}{name}"),
            AddressingMode::Accumulator => format!("{star}{name} A"),
            AddressingMode::Immediate => format!("{star}{name} #${lo:02X}"),
            AddressingMode::ZeroPage => {
                format!("{star}{name} ${lo:02X} = {:02X}", bus.peek(u16::from(lo)))
            }

            AddressingMode::ZeroPageX | AddressingMode::ZeroPageY => {
                let (index, register) = if info.mode == AddressingMode::ZeroPageX {
                    (cpu.x, 'X')
                } else {
                    (cpu.y, 'Y')
                };
                let effective = lo.wrapping_add(index);
                format!(
                    "{star}{name} ${lo:02X},{register} @ {effective:02X} = {:02X}",
                    bus.peek(u16::from(effective))
                )
            }

            // JMP and JSR name their target; everything else shows the byte there.
            AddressingMode::Absolute => {
                if matches!(info.operation, CpuOperation::Jmp | CpuOperation::Jsr) {
                    format!("{star}{name} ${absolute:04X}")
                } else {
                    format!("{star}{name} ${absolute:04X} = {:02X}", bus.peek(absolute))
                }
            }

            AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
                let (index, register) = if info.mode == AddressingMode::AbsoluteX {
                    (cpu.x, 'X')
                } else {
                    (cpu.y, 'Y')
                };
                let effective = absolute.wrapping_add(u16::from(index));
                format!(
                    "{star}{name} ${absolute:04X},{register} @ {effective:04X} = {:02X}",
                    bus.peek(effective)
                )
            }

            AddressingMode::Indirect => {
                let target_lo = bus.peek(absolute);
                let target_hi = bus.peek((absolute & 0xFF00) | (absolute.wrapping_add(1) & 0x00FF));
                format!(
                    "{star}{name} (${absolute:04X}) = {:04X}",
                    u16::from_le_bytes([target_lo, target_hi])
                )
            }

            AddressingMode::IndirectX => {
                let pointer = lo.wrapping_add(cpu.x);
                let target_lo = bus.peek(u16::from(pointer));
                let target_hi = bus.peek(u16::from(pointer.wrapping_add(1)));
                let effective = u16::from_le_bytes([target_lo, target_hi]);
                format!(
                    "{star}{name} (${lo:02X},X) @ {pointer:02X} = {effective:04X} = {:02X}",
                    bus.peek(effective)
                )
            }

            AddressingMode::IndirectY => {
                let target_lo = bus.peek(u16::from(lo));
                let target_hi = bus.peek(u16::from(lo.wrapping_add(1)));
                let base = u16::from_le_bytes([target_lo, target_hi]);
                let effective = base.wrapping_add(u16::from(cpu.y));
                format!(
                    "{star}{name} (${lo:02X}),Y = {base:04X} @ {effective:04X} = {:02X}",
                    bus.peek(effective)
                )
            }

            AddressingMode::Relative => {
                #[allow(clippy::cast_possible_wrap)] // The operand is a signed displacement.
                let offset = i16::from(lo as i8);
                format!(
                    "{star}{name} ${:04X}",
                    pc.wrapping_add(2).wrapping_add_signed(offset)
                )
            }
        }
    }
}
