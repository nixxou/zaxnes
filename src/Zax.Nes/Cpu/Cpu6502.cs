using Zax.Nes.State;

namespace Zax.Nes.Cpu;

/// <summary>
/// The 6502 core of the 2A03 (REQ-CORE-110, REQ-CORE-111).
/// </summary>
/// <remarks>
/// <para>
/// Written from the public nesdev documentation ("6502 instruction reference", "CPU
/// interrupts", "CPU unofficial opcodes"). Decimal mode is absent from the 2A03, so the
/// D flag is stored but never affects <c>ADC</c> or <c>SBC</c>.
/// </para>
/// <para>
/// <strong>Cycle model.</strong> Instructions are executed in one go, but every single
/// cycle is a real bus access through <see cref="ICpuBus"/>, dummy reads and the dummy
/// write of a read-modify-write included. The cycle count of an instruction is therefore
/// not stored in a table: it is the number of accesses the addressing mode performs, which
/// gives the page-cross and branch penalties for free.
/// </para>
/// <para>
/// <strong>Interrupts.</strong> Both lines are sampled once per cycle. For <strong>IRQ</strong>
/// the decision used at an instruction boundary is the sample of the <em>previous</em> cycle,
/// which is how the hardware polls (second-to-last cycle of the instruction) and is also
/// what produces the one-instruction lag of <c>CLI</c>, <c>SEI</c> and <c>PLP</c> on IRQ
/// recognition. The <strong>NMI</strong> edge is usable at the very next boundary, because
/// the line is already sampled near the start of the cycle rather than at its end
/// (<see cref="Ppu.Ppu2C02.NmiLineSample"/>).
/// </para>
/// <para>
/// Three details of the sequencing come from <c>cpu_interrupts_v2</c> (REQ-CORE-111): the
/// vector of a <c>BRK</c> or an IRQ is <strong>hijacked</strong> by an NMI latched before the
/// vector fetch, the interrupt sequence takes <strong>no decision for the boundary that
/// follows it</strong> so the first instruction of the handler always runs, and a taken
/// non-page-crossing <strong>branch</strong> polls one cycle earlier than any other
/// instruction. See <see cref="ServiceInterrupt"/> and <see cref="Branch"/>.
/// </para>
/// <para>
/// Still not modelled: NMI edges that both rise and fall while the CPU is halted by DMA — the
/// PPU holds /NMI low for the whole vblank, so a level sample after the DMA still sees it.
/// </para>
/// </remarks>
public sealed class Cpu6502
{
    /// <summary>Address of the NMI vector.</summary>
    public const ushort NmiVector = 0xFFFA;

    /// <summary>Address of the reset vector.</summary>
    public const ushort ResetVector = 0xFFFC;

    /// <summary>Address of the IRQ and BRK vector.</summary>
    public const ushort IrqVector = 0xFFFE;

    /// <summary>Carry.</summary>
    public const byte FlagC = 0x01;

    /// <summary>Zero.</summary>
    public const byte FlagZ = 0x02;

    /// <summary>Interrupt disable.</summary>
    public const byte FlagI = 0x04;

    /// <summary>Decimal mode; stored but inert on the 2A03.</summary>
    public const byte FlagD = 0x08;

    /// <summary>Break; exists only in the pushed copy of the status register.</summary>
    public const byte FlagB = 0x10;

    /// <summary>Unused; always 1 in the pushed copy and in the register as read back.</summary>
    public const byte FlagU = 0x20;

    /// <summary>Overflow.</summary>
    public const byte FlagV = 0x40;

    /// <summary>Negative.</summary>
    public const byte FlagN = 0x80;

    /// <summary>
    /// Constant OR-ed into A on the unstable opcodes <c>ANE</c> ($8B) and <c>LXA</c> ($AB).
    /// </summary>
    /// <remarks>
    /// On real hardware the value depends on the chip and its temperature. 0xEE is the
    /// value the public "NMOS 6510 unintended opcodes" document reports as the most
    /// common. No ROM required by SPEC-02 uses these two opcodes.
    /// </remarks>
    public const byte UnstableMagic = 0xEE;

    private readonly ICpuBus _bus;

    private byte _p = FlagI | FlagU;
    private bool _nmiLine;
    private bool _nmiPending;
    private bool _irqSample;
    private bool _pollNmi;
    private bool _pollIrq;

    /// <summary>Creates a CPU wired to <paramref name="bus"/>.</summary>
    public Cpu6502(ICpuBus bus)
    {
        ArgumentNullException.ThrowIfNull(bus);
        _bus = bus;
    }

    /// <summary>Accumulator.</summary>
    public byte A { get; set; }

    /// <summary>Index register X.</summary>
    public byte X { get; set; }

    /// <summary>Index register Y.</summary>
    public byte Y { get; set; }

    /// <summary>Stack pointer; the stack lives at $0100 + S.</summary>
    public byte S { get; set; }

    /// <summary>Program counter.</summary>
    public ushort PC { get; set; }

    /// <summary>
    /// Status register. Bit 5 always reads 1 and bit 4 always reads 0, which is what the
    /// <c>P</c> column of <c>nestest.log</c> shows.
    /// </summary>
    public byte P
    {
        get => _p;
        set => _p = (byte)((value & ~FlagB) | FlagU);
    }

    /// <summary>
    /// True once a <c>JAM</c> opcode has frozen the CPU. Only a reset clears it; the core
    /// keeps burning cycles so that the host loop does not hang.
    /// </summary>
    public bool Jammed { get; private set; }

    /// <summary>Master cycle count, taken from the bus (REQ-CORE-110).</summary>
    public long Cycles => _bus.CycleCount;

    /// <summary>Whether an NMI has been latched and will be serviced at the next boundary.</summary>
    public bool NmiPending => _nmiPending;

    /// <summary>Reads one flag of <see cref="P"/>.</summary>
    public bool GetFlag(byte flag) => (_p & flag) != 0;

    /// <summary>Sets or clears one flag of <see cref="P"/>.</summary>
    public void SetFlag(byte flag, bool value)
    {
        if (value)
        {
            _p |= flag;
        }
        else
        {
            _p &= (byte)~flag;
        }
    }

    /// <summary>
    /// Runs the power-on / reset sequence: 7 cycles, three stack pointer decrements and a
    /// jump through the reset vector (REQ-CORE-004).
    /// </summary>
    /// <remarks>
    /// The registers start at 0 rather than at an undefined value, so that two instances
    /// of the core behave identically (REQ-ARCH-012).
    /// </remarks>
    public void Reset()
    {
        Jammed = false;
        A = 0;
        X = 0;
        Y = 0;
        _p = FlagI | FlagU;
        S = 0x00;
        _nmiPending = false;
        _irqSample = false;
        _pollNmi = false;
        _pollIrq = false;

        Read(PC);
        Read(PC);
        Read((ushort)(0x0100 | S));
        S--;
        Read((ushort)(0x0100 | S));
        S--;
        Read((ushort)(0x0100 | S));
        S--;

        byte lo = Read(ResetVector);
        byte hi = Read((ushort)(ResetVector + 1));
        PC = (ushort)(lo | (hi << 8));

        // Take the current level as the reference for edge detection, so that an NMI that
        // was already asserted before the reset is not replayed.
        _nmiLine = _bus.NmiAsserted;
        _nmiPending = false;
        _pollNmi = false;
        _pollIrq = false;
    }

    /// <summary>
    /// Executes one instruction, or services a pending interrupt, and returns the number
    /// of CPU cycles it took.
    /// </summary>
    public int Step()
    {
        long before = _bus.CycleCount;

        if (Jammed)
        {
            Read(PC);
        }
        else if (_pollNmi)
        {
            // Cleared before the sequence runs, so that an edge arriving during the seven
            // cycles of the handler entry is still latched for the next boundary.
            _nmiPending = false;
            _pollNmi = false;
            ServiceInterrupt(NmiVector, fromBrk: false);
        }
        else if (_pollIrq)
        {
            ServiceInterrupt(IrqVector, fromBrk: false);
        }
        else
        {
            byte opcode = ReadPc();
            Execute(OpcodeTable.Get(opcode));
        }

        return (int)(_bus.CycleCount - before);
    }

    // ---------------------------------------------------------------- cycle plumbing

    private byte Read(ushort address)
    {
        byte value = _bus.Read(address);
        EndCycle();
        return value;
    }

    private void Write(ushort address, byte value)
    {
        _bus.Write(address, value);
        EndCycle();
    }

    private byte ReadPc() => Read(PC++);

    /// <summary>
    /// Closes a CPU cycle: promotes the previous cycle's IRQ sample to the decision used at
    /// the next instruction boundary, then samples the lines again.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The <strong>IRQ</strong> decision lags one cycle behind the sample, which is how the
    /// hardware polls (second-to-last cycle of the instruction) and is also what produces
    /// the one-instruction lag of <c>CLI</c>, <c>SEI</c> and <c>PLP</c>.
    /// </para>
    /// <para>
    /// The <strong>NMI</strong> edge, on the other hand, is usable at the very next
    /// instruction boundary. The line is already sampled near the start of the CPU cycle
    /// rather than at its end (<see cref="Ppu.Ppu2C02.NmiLineSample"/>), so adding the IRQ's
    /// extra cycle of lag on top would place the handler one CPU cycle too late — which is
    /// exactly what <c>ppu_vbl_nmi</c> test 5 and <c>vbl_nmi_timing</c> test 7 measure
    /// (REQ-CORE-116).
    /// </para>
    /// </remarks>
    private void EndCycle()
    {
        _pollIrq = _irqSample;

        bool nmi = _bus.NmiAsserted;
        if (nmi && !_nmiLine)
        {
            _nmiPending = true;
        }

        _nmiLine = nmi;
        _pollNmi = _nmiPending;
        _irqSample = _bus.IrqAsserted && (_p & FlagI) == 0;
    }

    private void Push(byte value)
    {
        Write((ushort)(0x0100 | S), value);
        S--;
    }

    private byte Pull()
    {
        S++;
        return Read((ushort)(0x0100 | S));
    }

    /// <summary>
    /// The seven-cycle interrupt sequence, shared by <c>BRK</c>, IRQ and NMI, with the vector
    /// hijacking of <c>cpu_interrupts_v2</c> parts 2 and 3.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The vector is not decided when the sequence starts: it is decided when the sequence
    /// reaches its vector fetch, on the state of /NMI at that moment. An NMI edge seen during
    /// any of the first five cycles — the two dummy reads and the three pushes — therefore sends
    /// a <c>BRK</c> or an IRQ through <c>$FFFA</c> instead of <c>$FFFE</c>, while the status byte
    /// already on the stack keeps the B bit a <c>BRK</c> gave it.
    /// </para>
    /// <para>
    /// That window of five cycles is what the ROM measures: "interrupts BRK for 5 clocks" in
    /// part 2, and in part 3 the seven rows that read the same are the two cycles of the
    /// <c>CLC</c> before the sequence plus those five. An edge seen during the two vector
    /// fetches comes too late and is serviced at the next boundary, which is the "IRQ occurs,
    /// then NMI occurs after SEC in IRQ handler" row.
    /// </para>
    /// </remarks>
    private void ServiceInterrupt(ushort vector, bool fromBrk)
    {
        if (fromBrk)
        {
            // BRK has already consumed its opcode; it also skips the signature byte.
            ReadPc();
        }
        else
        {
            Read(PC);
            Read(PC);
        }

        Push((byte)(PC >> 8));
        Push((byte)PC);
        Push(fromBrk ? (byte)(_p | FlagB) : _p);
        _p |= FlagI;

        // Hijacking: only a sequence heading for the IRQ vector can be diverted, and only by an
        // NMI, the two being the same vector otherwise.
        if (vector == IrqVector && _nmiPending)
        {
            vector = NmiVector;
            _nmiPending = false;
            _pollNmi = false;
        }

        byte lo = Read(vector);
        byte hi = Read((ushort)(vector + 1));
        PC = (ushort)(lo | (hi << 8));

        // The sequence takes no decision for the boundary that follows it: the first
        // instruction of the handler always runs. An edge seen during the two vector fetches
        // stays latched in _nmiPending and is serviced one instruction later, which is the
        // "NMI after SEC at beginning of IRQ handler" row of parts 2 and 3.
        _pollNmi = false;
        _pollIrq = false;
    }

    // ---------------------------------------------------------------- addressing

    private readonly record struct Operand(ushort Address, ushort BaseAddress, bool PageCrossed);

    private Operand Resolve(AddressingMode mode, OperandAccess access)
    {
        switch (mode)
        {
            case AddressingMode.ZeroPage:
            {
                byte zp = ReadPc();
                return new Operand(zp, zp, false);
            }

            case AddressingMode.ZeroPageX:
            case AddressingMode.ZeroPageY:
            {
                byte zp = ReadPc();
                Read(zp); // Dummy read at the un-indexed page-zero address.
                byte index = mode == AddressingMode.ZeroPageX ? X : Y;
                byte effective = (byte)(zp + index);
                return new Operand(effective, zp, false);
            }

            case AddressingMode.Absolute:
            {
                byte lo = ReadPc();
                byte hi = ReadPc();
                var address = (ushort)(lo | (hi << 8));
                return new Operand(address, address, false);
            }

            case AddressingMode.AbsoluteX:
            case AddressingMode.AbsoluteY:
            {
                byte lo = ReadPc();
                byte hi = ReadPc();
                var @base = (ushort)(lo | (hi << 8));
                byte index = mode == AddressingMode.AbsoluteX ? X : Y;
                var address = (ushort)(@base + index);
                bool crossed = (address & 0xFF00) != (@base & 0xFF00);

                // Reads only pay the extra cycle when the page changed; writes and
                // read-modify-writes always pay it.
                if (crossed || access != OperandAccess.Read)
                {
                    Read((ushort)((@base & 0xFF00) | (address & 0x00FF)));
                }

                return new Operand(address, @base, crossed);
            }

            case AddressingMode.IndirectX:
            {
                byte zp = ReadPc();
                Read(zp); // Dummy read before the index is added.
                var pointer = (byte)(zp + X);
                byte lo = Read(pointer);
                byte hi = Read((byte)(pointer + 1));
                var address = (ushort)(lo | (hi << 8));
                return new Operand(address, address, false);
            }

            case AddressingMode.IndirectY:
            {
                byte zp = ReadPc();
                byte lo = Read(zp);
                byte hi = Read((byte)(zp + 1));
                var @base = (ushort)(lo | (hi << 8));
                var address = (ushort)(@base + Y);
                bool crossed = (address & 0xFF00) != (@base & 0xFF00);

                if (crossed || access != OperandAccess.Read)
                {
                    Read((ushort)((@base & 0xFF00) | (address & 0x00FF)));
                }

                return new Operand(address, @base, crossed);
            }

            default:
                throw new InvalidOperationException($"Addressing mode {mode} has no memory operand.");
        }
    }

    // ---------------------------------------------------------------- execution

    private void Execute(OpcodeInfo info)
    {
        switch (info.Access)
        {
            case OperandAccess.None:
                ExecuteWithoutOperand(info);
                break;

            case OperandAccess.Read:
            {
                byte value = info.Mode == AddressingMode.Immediate
                    ? ReadPc()
                    : Read(Resolve(info.Mode, OperandAccess.Read).Address);
                ApplyRead(info.Operation, value);
                break;
            }

            case OperandAccess.Write:
            {
                Operand operand = Resolve(info.Mode, OperandAccess.Write);
                ApplyWrite(info.Operation, operand);
                break;
            }

            case OperandAccess.ReadModifyWrite:
            {
                Operand operand = Resolve(info.Mode, OperandAccess.ReadModifyWrite);
                byte value = Read(operand.Address);
                Write(operand.Address, value); // The real chip writes the old value back first.
                Write(operand.Address, Modify(info.Operation, value));
                break;
            }

            default:
                throw new InvalidOperationException($"Unknown operand access {info.Access}.");
        }
    }

    private void ExecuteWithoutOperand(OpcodeInfo info)
    {
        switch (info.Operation)
        {
            case CpuOperation.Jmp when info.Mode == AddressingMode.Absolute:
            {
                byte lo = ReadPc();
                byte hi = ReadPc();
                PC = (ushort)(lo | (hi << 8));
                return;
            }

            case CpuOperation.Jmp:
            {
                // Indirect JMP: the NMOS 6502 does not carry into the high byte of the
                // pointer, so ($xxFF) reads its high byte from $xx00.
                byte pLo = ReadPc();
                byte pHi = ReadPc();
                var pointer = (ushort)(pLo | (pHi << 8));
                byte lo = Read(pointer);
                byte hi = Read((ushort)((pointer & 0xFF00) | ((pointer + 1) & 0x00FF)));
                PC = (ushort)(lo | (hi << 8));
                return;
            }

            case CpuOperation.Jsr:
            {
                byte lo = ReadPc();
                Read((ushort)(0x0100 | S)); // Dummy stack read.
                Push((byte)(PC >> 8));
                Push((byte)PC);
                byte hi = ReadPc();
                PC = (ushort)(lo | (hi << 8));
                return;
            }

            case CpuOperation.Rts:
            {
                Read(PC);
                Read((ushort)(0x0100 | S));
                byte lo = Pull();
                byte hi = Pull();
                var target = (ushort)(lo | (hi << 8));
                Read(target); // Dummy read while PC is incremented.
                PC = (ushort)(target + 1);
                return;
            }

            case CpuOperation.Rti:
            {
                Read(PC);
                Read((ushort)(0x0100 | S));
                P = Pull();
                byte lo = Pull();
                byte hi = Pull();
                PC = (ushort)(lo | (hi << 8));
                return;
            }

            case CpuOperation.Brk:
                ServiceInterrupt(IrqVector, fromBrk: true);
                return;

            case CpuOperation.Pha:
                Read(PC);
                Push(A);
                return;

            case CpuOperation.Php:
                Read(PC);
                Push((byte)(_p | FlagB));
                return;

            case CpuOperation.Pla:
                Read(PC);
                Read((ushort)(0x0100 | S));
                A = Pull();
                SetZeroNegative(A);
                return;

            case CpuOperation.Plp:
                Read(PC);
                Read((ushort)(0x0100 | S));
                P = Pull();
                return;

            case CpuOperation.Bcc:
                Branch(!GetFlag(FlagC));
                return;
            case CpuOperation.Bcs:
                Branch(GetFlag(FlagC));
                return;
            case CpuOperation.Bne:
                Branch(!GetFlag(FlagZ));
                return;
            case CpuOperation.Beq:
                Branch(GetFlag(FlagZ));
                return;
            case CpuOperation.Bpl:
                Branch(!GetFlag(FlagN));
                return;
            case CpuOperation.Bmi:
                Branch(GetFlag(FlagN));
                return;
            case CpuOperation.Bvc:
                Branch(!GetFlag(FlagV));
                return;
            case CpuOperation.Bvs:
                Branch(GetFlag(FlagV));
                return;

            case CpuOperation.Jam:
                Jammed = true;
                Read(PC);
                return;
        }

        // Everything left takes exactly two cycles: the opcode fetch and one dummy read.
        Read(PC);

        switch (info.Operation)
        {
            case CpuOperation.Asl:
                A = Modify(CpuOperation.Asl, A);
                break;
            case CpuOperation.Lsr:
                A = Modify(CpuOperation.Lsr, A);
                break;
            case CpuOperation.Rol:
                A = Modify(CpuOperation.Rol, A);
                break;
            case CpuOperation.Ror:
                A = Modify(CpuOperation.Ror, A);
                break;

            case CpuOperation.Tax:
                X = A;
                SetZeroNegative(X);
                break;
            case CpuOperation.Tay:
                Y = A;
                SetZeroNegative(Y);
                break;
            case CpuOperation.Txa:
                A = X;
                SetZeroNegative(A);
                break;
            case CpuOperation.Tya:
                A = Y;
                SetZeroNegative(A);
                break;
            case CpuOperation.Tsx:
                X = S;
                SetZeroNegative(X);
                break;
            case CpuOperation.Txs:
                S = X; // TXS touches no flag.
                break;

            case CpuOperation.Inx:
                X++;
                SetZeroNegative(X);
                break;
            case CpuOperation.Iny:
                Y++;
                SetZeroNegative(Y);
                break;
            case CpuOperation.Dex:
                X--;
                SetZeroNegative(X);
                break;
            case CpuOperation.Dey:
                Y--;
                SetZeroNegative(Y);
                break;

            case CpuOperation.Clc:
                SetFlag(FlagC, false);
                break;
            case CpuOperation.Cld:
                SetFlag(FlagD, false);
                break;
            case CpuOperation.Cli:
                SetFlag(FlagI, false);
                break;
            case CpuOperation.Clv:
                SetFlag(FlagV, false);
                break;
            case CpuOperation.Sec:
                SetFlag(FlagC, true);
                break;
            case CpuOperation.Sed:
                SetFlag(FlagD, true);
                break;
            case CpuOperation.Sei:
                SetFlag(FlagI, true);
                break;

            case CpuOperation.Nop:
                break;

            default:
                throw new InvalidOperationException(
                    $"Operation {info.Operation} is not handled without an operand.");
        }
    }

    /// <summary>
    /// A conditional branch, with the interrupt quirk of <c>cpu_interrupts_v2</c> part 5: "a
    /// taken non-page-crossing branch ignores IRQ during its last clock, so that the next
    /// instruction executes before the IRQ. The same occurs for NMI."
    /// </summary>
    /// <remarks>
    /// An instruction normally polls the lines on its second-to-last cycle, which
    /// <see cref="EndCycle"/> models by promoting the previous cycle's sample. A taken branch
    /// that stays in the page polls one cycle earlier still — where a two-cycle instruction
    /// would — so the decision taken before its third cycle is the one that stands. A taken
    /// branch that crosses a page is ordinary: its fourth cycle restores nothing, and the
    /// <c>test_branch_taken_pagecross</c> column of the ROM shows the normal pattern.
    /// </remarks>
    private void Branch(bool taken)
    {
        var offset = (sbyte)ReadPc();
        if (!taken)
        {
            return;
        }

        // The decision as it stands after the operand fetch: the sample of the opcode-fetch
        // cycle. It is what the boundary must see when the branch stays inside its page.
        bool pollIrqBeforeTaken = _pollIrq;
        bool pollNmiBeforeTaken = _pollNmi;

        Read(PC); // Dummy read: one extra cycle when the branch is taken.
        var target = (ushort)(PC + offset);

        if ((target & 0xFF00) != (PC & 0xFF00))
        {
            // Second extra cycle: the high byte of PC is fixed up. The poll is then the
            // ordinary one, so nothing is restored.
            Read((ushort)((PC & 0xFF00) | (target & 0x00FF)));
        }
        else
        {
            // The pending edge stays latched in _nmiPending, so the NMI is serviced one
            // instruction later rather than lost.
            _pollIrq = pollIrqBeforeTaken;
            _pollNmi = pollNmiBeforeTaken;
        }

        PC = target;
    }

    private void ApplyRead(CpuOperation operation, byte value)
    {
        switch (operation)
        {
            case CpuOperation.Lda:
                A = value;
                SetZeroNegative(A);
                break;
            case CpuOperation.Ldx:
                X = value;
                SetZeroNegative(X);
                break;
            case CpuOperation.Ldy:
                Y = value;
                SetZeroNegative(Y);
                break;
            case CpuOperation.Lax:
                A = value;
                X = value;
                SetZeroNegative(A);
                break;

            case CpuOperation.And:
                A &= value;
                SetZeroNegative(A);
                break;
            case CpuOperation.Ora:
                A |= value;
                SetZeroNegative(A);
                break;
            case CpuOperation.Eor:
                A ^= value;
                SetZeroNegative(A);
                break;

            case CpuOperation.Adc:
                AddWithCarry(value);
                break;
            case CpuOperation.Sbc:
                AddWithCarry((byte)~value);
                break;

            case CpuOperation.Cmp:
                Compare(A, value);
                break;
            case CpuOperation.Cpx:
                Compare(X, value);
                break;
            case CpuOperation.Cpy:
                Compare(Y, value);
                break;

            case CpuOperation.Bit:
                SetFlag(FlagZ, (A & value) == 0);
                SetFlag(FlagN, (value & FlagN) != 0);
                SetFlag(FlagV, (value & FlagV) != 0);
                break;

            case CpuOperation.Nop:
                break;

            case CpuOperation.Anc:
                A &= value;
                SetZeroNegative(A);
                SetFlag(FlagC, (A & 0x80) != 0);
                break;

            case CpuOperation.Alr:
                A &= value;
                SetFlag(FlagC, (A & 0x01) != 0);
                A >>= 1;
                SetZeroNegative(A);
                break;

            case CpuOperation.Arr:
            {
                // AND, then ROR — but the rotate is done by the adder, not by the shift
                // circuit, so the flags come out as if an addition had taken place: the carry
                // is bit 7 of the AND result (bit 6 of the rotated value) rather than the bit
                // the rotate shifted out, and V is bits 6 xor 5 of the result (public
                // "NMOS 6510 unintended opcodes"; pinned by instr_test-v5 03-immediate).
                byte masked = (byte)(A & value);
                var result = (byte)((masked >> 1) | (GetFlag(FlagC) ? 0x80 : 0x00));
                A = result;
                SetZeroNegative(result);
                SetFlag(FlagC, (masked & 0x80) != 0);
                SetFlag(FlagV, (((result >> 6) ^ (result >> 5)) & 0x01) != 0);
                break;
            }

            case CpuOperation.Sbx:
            {
                var left = (byte)(A & X);
                Compare(left, value);
                X = (byte)(left - value);
                SetZeroNegative(X);
                break;
            }

            case CpuOperation.Ane:
                A = (byte)((A | UnstableMagic) & X & value);
                SetZeroNegative(A);
                break;

            case CpuOperation.Lxa:
                // $AB behaves as a plain "load A and X with the operand" on the 2A03: the
                // magic constant that makes $8B unstable reads as all ones here, which is why
                // blargg's instr_test-v5 lists ATX as testable (U) and XAA as not (?).
                A = value;
                X = value;
                SetZeroNegative(A);
                break;

            case CpuOperation.Las:
                S = (byte)(S & value);
                A = S;
                X = S;
                SetZeroNegative(A);
                break;

            default:
                throw new InvalidOperationException($"Operation {operation} is not a read operation.");
        }
    }

    private void ApplyWrite(CpuOperation operation, Operand operand)
    {
        switch (operation)
        {
            case CpuOperation.Sta:
                Write(operand.Address, A);
                return;
            case CpuOperation.Stx:
                Write(operand.Address, X);
                return;
            case CpuOperation.Sty:
                Write(operand.Address, Y);
                return;
            case CpuOperation.Sax:
                Write(operand.Address, (byte)(A & X));
                return;
        }

        // The four unstable stores: the value is AND-ed with the high byte of the
        // un-indexed address plus one, and on a page cross that same value replaces the
        // high byte of the target address.
        var high = (byte)((operand.BaseAddress >> 8) + 1);
        byte source = operation switch
        {
            CpuOperation.Shx => X,
            CpuOperation.Shy => Y,
            CpuOperation.Sha => (byte)(A & X),
            CpuOperation.Shs => (byte)(A & X),
            _ => throw new InvalidOperationException($"Operation {operation} is not a write operation."),
        };

        if (operation == CpuOperation.Shs)
        {
            S = (byte)(A & X);
        }

        var value = (byte)(source & high);
        ushort address = operand.PageCrossed
            ? (ushort)((value << 8) | (operand.Address & 0x00FF))
            : operand.Address;

        Write(address, value);
    }

    private byte Modify(CpuOperation operation, byte value)
    {
        switch (operation)
        {
            case CpuOperation.Asl:
            {
                SetFlag(FlagC, (value & 0x80) != 0);
                var result = (byte)(value << 1);
                SetZeroNegative(result);
                return result;
            }

            case CpuOperation.Lsr:
            {
                SetFlag(FlagC, (value & 0x01) != 0);
                var result = (byte)(value >> 1);
                SetZeroNegative(result);
                return result;
            }

            case CpuOperation.Rol:
            {
                var result = (byte)((value << 1) | (GetFlag(FlagC) ? 0x01 : 0x00));
                SetFlag(FlagC, (value & 0x80) != 0);
                SetZeroNegative(result);
                return result;
            }

            case CpuOperation.Ror:
            {
                var result = (byte)((value >> 1) | (GetFlag(FlagC) ? 0x80 : 0x00));
                SetFlag(FlagC, (value & 0x01) != 0);
                SetZeroNegative(result);
                return result;
            }

            case CpuOperation.Inc:
            {
                var result = (byte)(value + 1);
                SetZeroNegative(result);
                return result;
            }

            case CpuOperation.Dec:
            {
                var result = (byte)(value - 1);
                SetZeroNegative(result);
                return result;
            }

            case CpuOperation.Slo:
            {
                byte shifted = Modify(CpuOperation.Asl, value);
                A |= shifted;
                SetZeroNegative(A);
                return shifted;
            }

            case CpuOperation.Rla:
            {
                byte rotated = Modify(CpuOperation.Rol, value);
                A &= rotated;
                SetZeroNegative(A);
                return rotated;
            }

            case CpuOperation.Sre:
            {
                byte shifted = Modify(CpuOperation.Lsr, value);
                A ^= shifted;
                SetZeroNegative(A);
                return shifted;
            }

            case CpuOperation.Rra:
            {
                byte rotated = Modify(CpuOperation.Ror, value);
                AddWithCarry(rotated);
                return rotated;
            }

            case CpuOperation.Dcp:
            {
                var result = (byte)(value - 1);
                Compare(A, result);
                return result;
            }

            case CpuOperation.Isb:
            {
                var result = (byte)(value + 1);
                AddWithCarry((byte)~result);
                return result;
            }

            default:
                throw new InvalidOperationException($"Operation {operation} is not a read-modify-write.");
        }
    }

    private void AddWithCarry(byte value)
    {
        // The 2A03 has no decimal mode, so D never takes part (nesdev, "CPU ALU").
        int sum = A + value + (GetFlag(FlagC) ? 1 : 0);
        var result = (byte)sum;

        SetFlag(FlagC, sum > 0xFF);
        SetFlag(FlagV, ((A ^ result) & (value ^ result) & 0x80) != 0);
        A = result;
        SetZeroNegative(A);
    }

    private void Compare(byte left, byte right)
    {
        SetFlag(FlagC, left >= right);
        SetZeroNegative((byte)(left - right));
    }

    private void SetZeroNegative(byte value)
    {
        SetFlag(FlagZ, value == 0);
        SetFlag(FlagN, (value & FlagN) != 0);
    }

    // ---------------------------------------------------------------- savestate

    /// <summary>Appends the CPU state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        ArgumentNullException.ThrowIfNull(writer);

        writer.WriteByte(A);
        writer.WriteByte(X);
        writer.WriteByte(Y);
        writer.WriteByte(S);
        writer.WriteUInt16(PC);
        writer.WriteByte(_p);
        writer.WriteBool(_nmiLine);
        writer.WriteBool(_nmiPending);
        writer.WriteBool(_irqSample);
        writer.WriteBool(_pollNmi);
        writer.WriteBool(_pollIrq);
        writer.WriteBool(Jammed);
    }

    /// <summary>Restores the CPU state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        ArgumentNullException.ThrowIfNull(reader);

        A = reader.ReadByte();
        X = reader.ReadByte();
        Y = reader.ReadByte();
        S = reader.ReadByte();
        PC = reader.ReadUInt16();
        _p = reader.ReadByte();
        _nmiLine = reader.ReadBool();
        _nmiPending = reader.ReadBool();
        _irqSample = reader.ReadBool();
        _pollNmi = reader.ReadBool();
        _pollIrq = reader.ReadBool();
        Jammed = reader.ReadBool();
    }
}
