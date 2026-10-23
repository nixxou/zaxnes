use super::{AddressingMode, CpuBus, CpuOperation, OpcodeInfo, OpcodeTable, OperandAccess};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The 6502 core of the 2A03 (REQ-CORE-110, REQ-CORE-111).
///
/// Written from the public nesdev documentation ("6502 instruction reference", "CPU
/// interrupts", "CPU unofficial opcodes"). Decimal mode is absent from the 2A03, so the D
/// flag is stored but never affects `ADC` or `SBC`.
///
/// **Cycle model.** Instructions are executed in one go, but every single cycle is a real
/// bus access through [`CpuBus`], dummy reads and the dummy write of a read-modify-write
/// included. The cycle count of an instruction is therefore not stored in a table: it is
/// the number of accesses the addressing mode performs, which gives the page-cross and
/// branch penalties for free.
///
/// **Interrupts.** Both lines are sampled once per cycle. For **IRQ** the decision used at
/// an instruction boundary is the sample of the *previous* cycle, which is how the hardware
/// polls (second-to-last cycle of the instruction) and is also what produces the
/// one-instruction lag of `CLI`, `SEI` and `PLP` on IRQ recognition. The **NMI** edge is
/// usable at the very next boundary, because the line is already sampled near the start of
/// the cycle rather than at its end (`Ppu2C02::nmi_line_sample`).
///
/// Three details of the sequencing come from `cpu_interrupts_v2` (REQ-CORE-111): the
/// vector of a `BRK` or an IRQ is **hijacked** by an NMI latched before the vector fetch,
/// the interrupt sequence takes **no decision for the boundary that follows it** so the
/// first instruction of the handler always runs, and a taken non-page-crossing **branch**
/// polls one cycle earlier than any other instruction. See `service_interrupt` and
/// `branch`.
///
/// Still not modelled: NMI edges that both rise and fall while the CPU is halted by DMA —
/// the PPU holds /NMI low for the whole vblank, so a level sample after the DMA still sees
/// it.
///
/// The CPU does not own its bus: every call that runs cycles takes it as an argument, so
/// the core can own both side by side without shared ownership.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the interrupt latches of the C# core.
pub struct Cpu6502 {
    /// Accumulator.
    pub a: u8,
    /// Index register X.
    pub x: u8,
    /// Index register Y.
    pub y: u8,
    /// Stack pointer; the stack lives at $0100 + S.
    pub s: u8,
    /// Program counter.
    pub pc: u16,
    p: u8,
    jammed: bool,
    nmi_line: bool,
    nmi_pending: bool,
    irq_sample: bool,
    poll_nmi: bool,
    poll_irq: bool,
}

/// A resolved memory operand.
#[derive(Clone, Copy)]
struct Operand {
    address: u16,
    base_address: u16,
    page_crossed: bool,
}

impl Operand {
    const fn plain(address: u16) -> Self {
        Self {
            address,
            base_address: address,
            page_crossed: false,
        }
    }
}

impl Default for Cpu6502 {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu6502 {
    /// Address of the NMI vector.
    pub const NMI_VECTOR: u16 = 0xFFFA;
    /// Address of the reset vector.
    pub const RESET_VECTOR: u16 = 0xFFFC;
    /// Address of the IRQ and BRK vector.
    pub const IRQ_VECTOR: u16 = 0xFFFE;

    /// Carry.
    pub const FLAG_C: u8 = 0x01;
    /// Zero.
    pub const FLAG_Z: u8 = 0x02;
    /// Interrupt disable.
    pub const FLAG_I: u8 = 0x04;
    /// Decimal mode; stored but inert on the 2A03.
    pub const FLAG_D: u8 = 0x08;
    /// Break; exists only in the pushed copy of the status register.
    pub const FLAG_B: u8 = 0x10;
    /// Unused; always 1 in the pushed copy and in the register as read back.
    pub const FLAG_U: u8 = 0x20;
    /// Overflow.
    pub const FLAG_V: u8 = 0x40;
    /// Negative.
    pub const FLAG_N: u8 = 0x80;

    /// Constant OR-ed into A on the unstable opcode `ANE` ($8B).
    ///
    /// On real hardware the value depends on the chip and its temperature. 0xEE is the
    /// value the public "NMOS 6510 unintended opcodes" document reports as the most common.
    /// No ROM required by SPEC-02 uses this opcode.
    pub const UNSTABLE_MAGIC: u8 = 0xEE;

    /// Creates a CPU in its pre-reset state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            s: 0,
            pc: 0,
            p: Self::FLAG_I | Self::FLAG_U,
            jammed: false,
            nmi_line: false,
            nmi_pending: false,
            irq_sample: false,
            poll_nmi: false,
            poll_irq: false,
        }
    }

    /// Status register. Bit 5 always reads 1 and bit 4 always reads 0, which is what the
    /// `P` column of `nestest.log` shows.
    #[must_use]
    pub const fn p(&self) -> u8 {
        self.p
    }

    /// Sets the status register, dropping bit 4 and forcing bit 5.
    pub fn set_p(&mut self, value: u8) {
        self.p = (value & !Self::FLAG_B) | Self::FLAG_U;
    }

    /// True once a `JAM` opcode has frozen the CPU. Only a reset clears it; the core keeps
    /// burning cycles so that the host loop does not hang.
    #[must_use]
    pub const fn jammed(&self) -> bool {
        self.jammed
    }

    /// Whether an NMI has been latched and will be serviced at the next boundary.
    #[must_use]
    pub const fn nmi_pending(&self) -> bool {
        self.nmi_pending
    }

    /// Reads one flag of the status register.
    #[must_use]
    pub const fn get_flag(&self, flag: u8) -> bool {
        (self.p & flag) != 0
    }

    /// Sets or clears one flag of the status register.
    pub fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.p |= flag;
        } else {
            self.p &= !flag;
        }
    }

    /// Runs the power-on / reset sequence: 7 cycles, three stack pointer decrements and a
    /// jump through the reset vector (REQ-CORE-004).
    ///
    /// The registers start at 0 rather than at an undefined value, so that two instances
    /// of the core behave identically (REQ-ARCH-012).
    pub fn reset<B: CpuBus + ?Sized>(&mut self, bus: &mut B) {
        self.jammed = false;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.p = Self::FLAG_I | Self::FLAG_U;
        self.s = 0x00;
        self.nmi_pending = false;
        self.irq_sample = false;
        self.poll_nmi = false;
        self.poll_irq = false;

        self.read(bus, self.pc);
        self.read(bus, self.pc);
        for _ in 0..3 {
            self.read(bus, 0x0100 | u16::from(self.s));
            self.s = self.s.wrapping_sub(1);
        }

        let lo = self.read(bus, Self::RESET_VECTOR);
        let hi = self.read(bus, Self::RESET_VECTOR + 1);
        self.pc = word(lo, hi);

        // Take the current level as the reference for edge detection, so that an NMI that
        // was already asserted before the reset is not replayed.
        self.nmi_line = bus.nmi_asserted();
        self.nmi_pending = false;
        self.poll_nmi = false;
        self.poll_irq = false;
    }

    /// Executes one instruction, or services a pending interrupt, and returns the number
    /// of CPU cycles it took.
    pub fn step<B: CpuBus + ?Sized>(&mut self, bus: &mut B) -> u32 {
        let before = bus.cycle_count();

        if self.jammed {
            self.read(bus, self.pc);
        } else if self.poll_nmi {
            // Cleared before the sequence runs, so that an edge arriving during the seven
            // cycles of the handler entry is still latched for the next boundary.
            self.nmi_pending = false;
            self.poll_nmi = false;
            self.service_interrupt(bus, Self::NMI_VECTOR, false);
        } else if self.poll_irq {
            self.service_interrupt(bus, Self::IRQ_VECTOR, false);
        } else {
            let opcode = self.read_pc(bus);
            self.execute(bus, OpcodeTable::get(opcode));
        }

        u32::try_from(bus.cycle_count() - before).unwrap_or(u32::MAX)
    }

    // ---------------------------------------------------------------- cycle plumbing

    #[inline]
    fn read<B: CpuBus + ?Sized>(&mut self, bus: &mut B, address: u16) -> u8 {
        let value = bus.read(address);
        self.end_cycle(bus);
        value
    }

    #[inline]
    fn write<B: CpuBus + ?Sized>(&mut self, bus: &mut B, address: u16, value: u8) {
        bus.write(address, value);
        self.end_cycle(bus);
    }

    #[inline]
    fn read_pc<B: CpuBus + ?Sized>(&mut self, bus: &mut B) -> u8 {
        let pc = self.pc;
        self.pc = pc.wrapping_add(1);
        self.read(bus, pc)
    }

    /// Closes a CPU cycle: promotes the previous cycle's IRQ sample to the decision used at
    /// the next instruction boundary, then samples the lines again.
    ///
    /// The **IRQ** decision lags one cycle behind the sample, which is how the hardware
    /// polls (second-to-last cycle of the instruction) and is also what produces the
    /// one-instruction lag of `CLI`, `SEI` and `PLP`.
    ///
    /// The **NMI** edge, on the other hand, is usable at the very next instruction
    /// boundary. The line is already sampled near the start of the CPU cycle rather than at
    /// its end, so adding the IRQ's extra cycle of lag on top would place the handler one
    /// CPU cycle too late — which is exactly what `ppu_vbl_nmi` test 5 and `vbl_nmi_timing`
    /// test 7 measure (REQ-CORE-116).
    #[inline]
    fn end_cycle<B: CpuBus + ?Sized>(&mut self, bus: &B) {
        self.poll_irq = self.irq_sample;

        let nmi = bus.nmi_asserted();
        if nmi && !self.nmi_line {
            self.nmi_pending = true;
        }

        self.nmi_line = nmi;
        self.poll_nmi = self.nmi_pending;
        self.irq_sample = bus.irq_asserted() && (self.p & Self::FLAG_I) == 0;
    }

    fn stack_address(&self) -> u16 {
        0x0100 | u16::from(self.s)
    }

    fn push<B: CpuBus + ?Sized>(&mut self, bus: &mut B, value: u8) {
        self.write(bus, self.stack_address(), value);
        self.s = self.s.wrapping_sub(1);
    }

    fn pull<B: CpuBus + ?Sized>(&mut self, bus: &mut B) -> u8 {
        self.s = self.s.wrapping_add(1);
        self.read(bus, self.stack_address())
    }

    /// The seven-cycle interrupt sequence, shared by `BRK`, IRQ and NMI, with the vector
    /// hijacking of `cpu_interrupts_v2` parts 2 and 3.
    ///
    /// The vector is not decided when the sequence starts: it is decided when the sequence
    /// reaches its vector fetch, on the state of /NMI at that moment. An NMI edge seen
    /// during any of the first five cycles — the two dummy reads and the three pushes —
    /// therefore sends a `BRK` or an IRQ through `$FFFA` instead of `$FFFE`, while the
    /// status byte already on the stack keeps the B bit a `BRK` gave it.
    ///
    /// That window of five cycles is what the ROM measures: "interrupts BRK for 5 clocks"
    /// in part 2, and in part 3 the seven rows that read the same are the two cycles of the
    /// `CLC` before the sequence plus those five. An edge seen during the two vector
    /// fetches comes too late and is serviced at the next boundary, which is the "IRQ
    /// occurs, then NMI occurs after SEC in IRQ handler" row.
    fn service_interrupt<B: CpuBus + ?Sized>(&mut self, bus: &mut B, vector: u16, from_brk: bool) {
        let mut vector = vector;
        if from_brk {
            // BRK has already consumed its opcode; it also skips the signature byte.
            self.read_pc(bus);
        } else {
            self.read(bus, self.pc);
            self.read(bus, self.pc);
        }

        let [pc_lo, pc_hi] = self.pc.to_le_bytes();
        self.push(bus, pc_hi);
        self.push(bus, pc_lo);
        let status = if from_brk {
            self.p | Self::FLAG_B
        } else {
            self.p
        };
        self.push(bus, status);
        self.p |= Self::FLAG_I;

        // Hijacking: only a sequence heading for the IRQ vector can be diverted, and only
        // by an NMI, the two being the same vector otherwise.
        if vector == Self::IRQ_VECTOR && self.nmi_pending {
            vector = Self::NMI_VECTOR;
            self.nmi_pending = false;
            self.poll_nmi = false;
        }

        let lo = self.read(bus, vector);
        let hi = self.read(bus, vector.wrapping_add(1));
        self.pc = word(lo, hi);

        // The sequence takes no decision for the boundary that follows it: the first
        // instruction of the handler always runs. An edge seen during the two vector
        // fetches stays latched in nmi_pending and is serviced one instruction later,
        // which is the "NMI after SEC at beginning of IRQ handler" row of parts 2 and 3.
        self.poll_nmi = false;
        self.poll_irq = false;
    }

    // ---------------------------------------------------------------- addressing

    fn resolve<B: CpuBus + ?Sized>(
        &mut self,
        bus: &mut B,
        mode: AddressingMode,
        access: OperandAccess,
    ) -> Operand {
        match mode {
            AddressingMode::ZeroPage => Operand::plain(u16::from(self.read_pc(bus))),

            AddressingMode::ZeroPageX | AddressingMode::ZeroPageY => {
                let zp = self.read_pc(bus);
                self.read(bus, u16::from(zp)); // Dummy read at the un-indexed page-zero address.
                let index = if mode == AddressingMode::ZeroPageX {
                    self.x
                } else {
                    self.y
                };
                let effective = zp.wrapping_add(index);
                Operand {
                    address: u16::from(effective),
                    base_address: u16::from(zp),
                    page_crossed: false,
                }
            }

            AddressingMode::Absolute => {
                let lo = self.read_pc(bus);
                let hi = self.read_pc(bus);
                Operand::plain(word(lo, hi))
            }

            AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => {
                let lo = self.read_pc(bus);
                let hi = self.read_pc(bus);
                let base = word(lo, hi);
                let index = if mode == AddressingMode::AbsoluteX {
                    self.x
                } else {
                    self.y
                };
                self.indexed(bus, base, index, access)
            }

            AddressingMode::IndirectX => {
                let zp = self.read_pc(bus);
                self.read(bus, u16::from(zp)); // Dummy read before the index is added.
                let pointer = zp.wrapping_add(self.x);
                let lo = self.read(bus, u16::from(pointer));
                let hi = self.read(bus, u16::from(pointer.wrapping_add(1)));
                Operand::plain(word(lo, hi))
            }

            AddressingMode::IndirectY => {
                let zp = self.read_pc(bus);
                let lo = self.read(bus, u16::from(zp));
                let hi = self.read(bus, u16::from(zp.wrapping_add(1)));
                self.indexed(bus, word(lo, hi), self.y, access)
            }

            // The table never pairs these modes with a memory operand.
            AddressingMode::Implied
            | AddressingMode::Accumulator
            | AddressingMode::Immediate
            | AddressingMode::Indirect
            | AddressingMode::Relative => Operand::plain(0),
        }
    }

    /// Adds an index to `base`. Reads only pay the extra cycle when the page changed;
    /// writes and read-modify-writes always pay it.
    fn indexed<B: CpuBus + ?Sized>(
        &mut self,
        bus: &mut B,
        base: u16,
        index: u8,
        access: OperandAccess,
    ) -> Operand {
        let address = base.wrapping_add(u16::from(index));
        let crossed = (address & 0xFF00) != (base & 0xFF00);

        if crossed || access != OperandAccess::Read {
            self.read(bus, (base & 0xFF00) | (address & 0x00FF));
        }

        Operand {
            address,
            base_address: base,
            page_crossed: crossed,
        }
    }

    // ---------------------------------------------------------------- execution

    fn execute<B: CpuBus + ?Sized>(&mut self, bus: &mut B, info: OpcodeInfo) {
        match info.access {
            OperandAccess::None => self.execute_without_operand(bus, info),

            OperandAccess::Read => {
                let value = if info.mode == AddressingMode::Immediate {
                    self.read_pc(bus)
                } else {
                    let operand = self.resolve(bus, info.mode, OperandAccess::Read);
                    self.read(bus, operand.address)
                };
                self.apply_read(info.operation, value);
            }

            OperandAccess::Write => {
                let operand = self.resolve(bus, info.mode, OperandAccess::Write);
                self.apply_write(bus, info.operation, operand);
            }

            OperandAccess::ReadModifyWrite => {
                let operand = self.resolve(bus, info.mode, OperandAccess::ReadModifyWrite);
                let value = self.read(bus, operand.address);
                self.write(bus, operand.address, value); // The real chip writes the old value back first.
                let result = self.modify(info.operation, value);
                self.write(bus, operand.address, result);
            }
        }
    }

    #[allow(clippy::too_many_lines)] // One arm per operation, as in the C# switch.
    fn execute_without_operand<B: CpuBus + ?Sized>(&mut self, bus: &mut B, info: OpcodeInfo) {
        match info.operation {
            CpuOperation::Jmp if info.mode == AddressingMode::Absolute => {
                let lo = self.read_pc(bus);
                let hi = self.read_pc(bus);
                self.pc = word(lo, hi);
                return;
            }

            CpuOperation::Jmp => {
                // Indirect JMP: the NMOS 6502 does not carry into the high byte of the
                // pointer, so ($xxFF) reads its high byte from $xx00.
                let p_lo = self.read_pc(bus);
                let p_hi = self.read_pc(bus);
                let pointer = word(p_lo, p_hi);
                let lo = self.read(bus, pointer);
                let hi = self.read(bus, (pointer & 0xFF00) | (pointer.wrapping_add(1) & 0x00FF));
                self.pc = word(lo, hi);
                return;
            }

            CpuOperation::Jsr => {
                let lo = self.read_pc(bus);
                self.read(bus, self.stack_address()); // Dummy stack read.
                let [pc_lo, pc_hi] = self.pc.to_le_bytes();
                self.push(bus, pc_hi);
                self.push(bus, pc_lo);
                let hi = self.read_pc(bus);
                self.pc = word(lo, hi);
                return;
            }

            CpuOperation::Rts => {
                self.read(bus, self.pc);
                self.read(bus, self.stack_address());
                let lo = self.pull(bus);
                let hi = self.pull(bus);
                let target = word(lo, hi);
                self.read(bus, target); // Dummy read while PC is incremented.
                self.pc = target.wrapping_add(1);
                return;
            }

            CpuOperation::Rti => {
                self.read(bus, self.pc);
                self.read(bus, self.stack_address());
                let status = self.pull(bus);
                self.set_p(status);
                let lo = self.pull(bus);
                let hi = self.pull(bus);
                self.pc = word(lo, hi);
                return;
            }

            CpuOperation::Brk => {
                self.service_interrupt(bus, Self::IRQ_VECTOR, true);
                return;
            }

            CpuOperation::Pha => {
                self.read(bus, self.pc);
                self.push(bus, self.a);
                return;
            }

            CpuOperation::Php => {
                self.read(bus, self.pc);
                self.push(bus, self.p | Self::FLAG_B);
                return;
            }

            CpuOperation::Pla => {
                self.read(bus, self.pc);
                self.read(bus, self.stack_address());
                self.a = self.pull(bus);
                self.set_zero_negative(self.a);
                return;
            }

            CpuOperation::Plp => {
                self.read(bus, self.pc);
                self.read(bus, self.stack_address());
                let status = self.pull(bus);
                self.set_p(status);
                return;
            }

            CpuOperation::Bcc => return self.branch(bus, !self.get_flag(Self::FLAG_C)),
            CpuOperation::Bcs => return self.branch(bus, self.get_flag(Self::FLAG_C)),
            CpuOperation::Bne => return self.branch(bus, !self.get_flag(Self::FLAG_Z)),
            CpuOperation::Beq => return self.branch(bus, self.get_flag(Self::FLAG_Z)),
            CpuOperation::Bpl => return self.branch(bus, !self.get_flag(Self::FLAG_N)),
            CpuOperation::Bmi => return self.branch(bus, self.get_flag(Self::FLAG_N)),
            CpuOperation::Bvc => return self.branch(bus, !self.get_flag(Self::FLAG_V)),
            CpuOperation::Bvs => return self.branch(bus, self.get_flag(Self::FLAG_V)),

            CpuOperation::Jam => {
                self.jammed = true;
                self.read(bus, self.pc);
                return;
            }

            _ => {}
        }

        // Everything left takes exactly two cycles: the opcode fetch and one dummy read.
        self.read(bus, self.pc);

        match info.operation {
            CpuOperation::Asl | CpuOperation::Lsr | CpuOperation::Rol | CpuOperation::Ror => {
                self.a = self.modify(info.operation, self.a);
            }

            CpuOperation::Tax => {
                self.x = self.a;
                self.set_zero_negative(self.x);
            }
            CpuOperation::Tay => {
                self.y = self.a;
                self.set_zero_negative(self.y);
            }
            CpuOperation::Txa => {
                self.a = self.x;
                self.set_zero_negative(self.a);
            }
            CpuOperation::Tya => {
                self.a = self.y;
                self.set_zero_negative(self.a);
            }
            CpuOperation::Tsx => {
                self.x = self.s;
                self.set_zero_negative(self.x);
            }
            CpuOperation::Txs => self.s = self.x, // TXS touches no flag.

            CpuOperation::Inx => {
                self.x = self.x.wrapping_add(1);
                self.set_zero_negative(self.x);
            }
            CpuOperation::Iny => {
                self.y = self.y.wrapping_add(1);
                self.set_zero_negative(self.y);
            }
            CpuOperation::Dex => {
                self.x = self.x.wrapping_sub(1);
                self.set_zero_negative(self.x);
            }
            CpuOperation::Dey => {
                self.y = self.y.wrapping_sub(1);
                self.set_zero_negative(self.y);
            }

            CpuOperation::Clc => self.set_flag(Self::FLAG_C, false),
            CpuOperation::Cld => self.set_flag(Self::FLAG_D, false),
            CpuOperation::Cli => self.set_flag(Self::FLAG_I, false),
            CpuOperation::Clv => self.set_flag(Self::FLAG_V, false),
            CpuOperation::Sec => self.set_flag(Self::FLAG_C, true),
            CpuOperation::Sed => self.set_flag(Self::FLAG_D, true),
            CpuOperation::Sei => self.set_flag(Self::FLAG_I, true),

            // NOP, and nothing else reaches here: the table only gives the operations
            // above an implied or accumulator mode.
            _ => {}
        }
    }

    /// A conditional branch, with the interrupt quirk of `cpu_interrupts_v2` part 5: "a
    /// taken non-page-crossing branch ignores IRQ during its last clock, so that the next
    /// instruction executes before the IRQ. The same occurs for NMI."
    ///
    /// An instruction normally polls the lines on its second-to-last cycle, which
    /// `end_cycle` models by promoting the previous cycle's sample. A taken branch that
    /// stays in the page polls one cycle earlier still — where a two-cycle instruction
    /// would — so the decision taken before its third cycle is the one that stands. A taken
    /// branch that crosses a page is ordinary: its fourth cycle restores nothing, and the
    /// `test_branch_taken_pagecross` column of the ROM shows the normal pattern.
    fn branch<B: CpuBus + ?Sized>(&mut self, bus: &mut B, taken: bool) {
        #[allow(clippy::cast_possible_wrap)] // The operand is a signed displacement.
        let offset = self.read_pc(bus) as i8;
        if !taken {
            return;
        }

        // The decision as it stands after the operand fetch: the sample of the opcode-fetch
        // cycle. It is what the boundary must see when the branch stays inside its page.
        let poll_irq_before_taken = self.poll_irq;
        let poll_nmi_before_taken = self.poll_nmi;

        self.read(bus, self.pc); // Dummy read: one extra cycle when the branch is taken.
        let target = self.pc.wrapping_add_signed(i16::from(offset));

        if (target & 0xFF00) == (self.pc & 0xFF00) {
            // The pending edge stays latched in nmi_pending, so the NMI is serviced one
            // instruction later rather than lost.
            self.poll_irq = poll_irq_before_taken;
            self.poll_nmi = poll_nmi_before_taken;
        } else {
            // Second extra cycle: the high byte of PC is fixed up. The poll is then the
            // ordinary one, so nothing is restored.
            self.read(bus, (self.pc & 0xFF00) | (target & 0x00FF));
        }

        self.pc = target;
    }

    fn apply_read(&mut self, operation: CpuOperation, value: u8) {
        match operation {
            CpuOperation::Lda => {
                self.a = value;
                self.set_zero_negative(self.a);
            }
            CpuOperation::Ldx => {
                self.x = value;
                self.set_zero_negative(self.x);
            }
            CpuOperation::Ldy => {
                self.y = value;
                self.set_zero_negative(self.y);
            }
            CpuOperation::Lax => {
                self.a = value;
                self.x = value;
                self.set_zero_negative(self.a);
            }

            CpuOperation::And => {
                self.a &= value;
                self.set_zero_negative(self.a);
            }
            CpuOperation::Ora => {
                self.a |= value;
                self.set_zero_negative(self.a);
            }
            CpuOperation::Eor => {
                self.a ^= value;
                self.set_zero_negative(self.a);
            }

            CpuOperation::Adc => self.add_with_carry(value),
            CpuOperation::Sbc => self.add_with_carry(!value),

            CpuOperation::Cmp => self.compare(self.a, value),
            CpuOperation::Cpx => self.compare(self.x, value),
            CpuOperation::Cpy => self.compare(self.y, value),

            CpuOperation::Bit => {
                self.set_flag(Self::FLAG_Z, (self.a & value) == 0);
                self.set_flag(Self::FLAG_N, (value & Self::FLAG_N) != 0);
                self.set_flag(Self::FLAG_V, (value & Self::FLAG_V) != 0);
            }

            CpuOperation::Anc => {
                self.a &= value;
                self.set_zero_negative(self.a);
                self.set_flag(Self::FLAG_C, (self.a & 0x80) != 0);
            }

            CpuOperation::Alr => {
                self.a &= value;
                self.set_flag(Self::FLAG_C, (self.a & 0x01) != 0);
                self.a >>= 1;
                self.set_zero_negative(self.a);
            }

            CpuOperation::Arr => {
                // AND, then ROR — but the rotate is done by the adder, not by the shift
                // circuit, so the flags come out as if an addition had taken place: the
                // carry is bit 7 of the AND result (bit 6 of the rotated value) rather than
                // the bit the rotate shifted out, and V is bits 6 xor 5 of the result
                // (public "NMOS 6510 unintended opcodes"; pinned by instr_test-v5
                // 03-immediate).
                let masked = self.a & value;
                let result = (masked >> 1)
                    | if self.get_flag(Self::FLAG_C) {
                        0x80
                    } else {
                        0x00
                    };
                self.a = result;
                self.set_zero_negative(result);
                self.set_flag(Self::FLAG_C, (masked & 0x80) != 0);
                self.set_flag(Self::FLAG_V, (((result >> 6) ^ (result >> 5)) & 0x01) != 0);
            }

            CpuOperation::Sbx => {
                let left = self.a & self.x;
                self.compare(left, value);
                self.x = left.wrapping_sub(value);
                self.set_zero_negative(self.x);
            }

            CpuOperation::Ane => {
                self.a = (self.a | Self::UNSTABLE_MAGIC) & self.x & value;
                self.set_zero_negative(self.a);
            }

            CpuOperation::Lxa => {
                // $AB behaves as a plain "load A and X with the operand" on the 2A03: the
                // magic constant that makes $8B unstable reads as all ones here, which is
                // why blargg's instr_test-v5 lists ATX as testable (U) and XAA as not (?).
                self.a = value;
                self.x = value;
                self.set_zero_negative(self.a);
            }

            CpuOperation::Las => {
                self.s &= value;
                self.a = self.s;
                self.x = self.s;
                self.set_zero_negative(self.a);
            }

            // NOP, and nothing else: the table tags only the operations above as reads.
            _ => {}
        }
    }

    fn apply_write<B: CpuBus + ?Sized>(
        &mut self,
        bus: &mut B,
        operation: CpuOperation,
        operand: Operand,
    ) {
        let source = match operation {
            CpuOperation::Sta => return self.write(bus, operand.address, self.a),
            CpuOperation::Stx => return self.write(bus, operand.address, self.x),
            CpuOperation::Sty => return self.write(bus, operand.address, self.y),
            CpuOperation::Sax => return self.write(bus, operand.address, self.a & self.x),
            CpuOperation::Shx => self.x,
            CpuOperation::Shy => self.y,
            CpuOperation::Shs => {
                self.s = self.a & self.x;
                self.a & self.x
            }
            // SHA, and nothing else: the table tags only the operations above as writes.
            _ => self.a & self.x,
        };

        // The four unstable stores: the value is AND-ed with the high byte of the
        // un-indexed address plus one, and on a page cross that same value replaces the
        // high byte of the target address.
        let high = operand.base_address.to_be_bytes()[0].wrapping_add(1);
        let value = source & high;
        let address = if operand.page_crossed {
            (u16::from(value) << 8) | (operand.address & 0x00FF)
        } else {
            operand.address
        };

        self.write(bus, address, value);
    }

    fn modify(&mut self, operation: CpuOperation, value: u8) -> u8 {
        match operation {
            CpuOperation::Asl => {
                self.set_flag(Self::FLAG_C, (value & 0x80) != 0);
                let result = value << 1;
                self.set_zero_negative(result);
                result
            }

            CpuOperation::Lsr => {
                self.set_flag(Self::FLAG_C, (value & 0x01) != 0);
                let result = value >> 1;
                self.set_zero_negative(result);
                result
            }

            CpuOperation::Rol => {
                let result = (value << 1) | u8::from(self.get_flag(Self::FLAG_C));
                self.set_flag(Self::FLAG_C, (value & 0x80) != 0);
                self.set_zero_negative(result);
                result
            }

            CpuOperation::Ror => {
                let result = (value >> 1)
                    | if self.get_flag(Self::FLAG_C) {
                        0x80
                    } else {
                        0x00
                    };
                self.set_flag(Self::FLAG_C, (value & 0x01) != 0);
                self.set_zero_negative(result);
                result
            }

            CpuOperation::Inc => {
                let result = value.wrapping_add(1);
                self.set_zero_negative(result);
                result
            }

            CpuOperation::Dec => {
                let result = value.wrapping_sub(1);
                self.set_zero_negative(result);
                result
            }

            CpuOperation::Slo => {
                let shifted = self.modify(CpuOperation::Asl, value);
                self.a |= shifted;
                self.set_zero_negative(self.a);
                shifted
            }

            CpuOperation::Rla => {
                let rotated = self.modify(CpuOperation::Rol, value);
                self.a &= rotated;
                self.set_zero_negative(self.a);
                rotated
            }

            CpuOperation::Sre => {
                let shifted = self.modify(CpuOperation::Lsr, value);
                self.a ^= shifted;
                self.set_zero_negative(self.a);
                shifted
            }

            CpuOperation::Rra => {
                let rotated = self.modify(CpuOperation::Ror, value);
                self.add_with_carry(rotated);
                rotated
            }

            CpuOperation::Dcp => {
                let result = value.wrapping_sub(1);
                self.compare(self.a, result);
                result
            }

            CpuOperation::Isb => {
                let result = value.wrapping_add(1);
                self.add_with_carry(!result);
                result
            }

            // The table tags only the operations above as read-modify-writes.
            _ => value,
        }
    }

    fn add_with_carry(&mut self, value: u8) {
        // The 2A03 has no decimal mode, so D never takes part (nesdev, "CPU ALU").
        let sum = u16::from(self.a) + u16::from(value) + u16::from(self.get_flag(Self::FLAG_C));
        let result = sum.to_le_bytes()[0];

        self.set_flag(Self::FLAG_C, sum > 0xFF);
        self.set_flag(
            Self::FLAG_V,
            ((self.a ^ result) & (value ^ result) & 0x80) != 0,
        );
        self.a = result;
        self.set_zero_negative(self.a);
    }

    fn compare(&mut self, left: u8, right: u8) {
        self.set_flag(Self::FLAG_C, left >= right);
        self.set_zero_negative(left.wrapping_sub(right));
    }

    fn set_zero_negative(&mut self, value: u8) {
        self.set_flag(Self::FLAG_Z, value == 0);
        self.set_flag(Self::FLAG_N, (value & Self::FLAG_N) != 0);
    }
}

#[inline]
const fn word(lo: u8, hi: u8) -> u16 {
    u16::from_le_bytes([lo, hi])
}

impl Cpu6502 {
    /// Appends the CPU state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.a);
        writer.write_u8(self.x);
        writer.write_u8(self.y);
        writer.write_u8(self.s);
        writer.write_u16(self.pc);
        writer.write_u8(self.p);
        writer.write_bool(self.nmi_line);
        writer.write_bool(self.nmi_pending);
        writer.write_bool(self.irq_sample);
        writer.write_bool(self.poll_nmi);
        writer.write_bool(self.poll_irq);
        writer.write_bool(self.jammed);
    }

    /// Restores the CPU state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.a = reader.read_u8()?;
        self.x = reader.read_u8()?;
        self.y = reader.read_u8()?;
        self.s = reader.read_u8()?;
        self.pc = reader.read_u16()?;
        self.p = reader.read_u8()?;
        self.nmi_line = reader.read_bool()?;
        self.nmi_pending = reader.read_bool()?;
        self.irq_sample = reader.read_bool()?;
        self.poll_nmi = reader.read_bool()?;
        self.poll_irq = reader.read_bool()?;
        self.jammed = reader.read_bool()?;
        Ok(())
    }
}
