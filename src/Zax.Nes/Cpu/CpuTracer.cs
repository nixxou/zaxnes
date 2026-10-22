using System.Globalization;
using System.Text;

namespace Zax.Nes.Cpu;

/// <summary>
/// One line of CPU trace: the machine state observed just before an instruction executes.
/// </summary>
/// <param name="Pc">Program counter.</param>
/// <param name="Bytes">The instruction's bytes, 1 to 3 of them.</param>
/// <param name="A">Accumulator.</param>
/// <param name="X">Index X.</param>
/// <param name="Y">Index Y.</param>
/// <param name="P">Status register, with bit 5 set and bit 4 clear.</param>
/// <param name="S">Stack pointer.</param>
/// <param name="Cycles">Master CPU cycle count (the <c>CYC</c> column).</param>
/// <param name="Disassembly">Human-readable form, for diagnostics only.</param>
public readonly record struct CpuTraceLine(
    ushort Pc,
    byte[] Bytes,
    byte A,
    byte X,
    byte Y,
    byte P,
    byte S,
    long Cycles,
    string Disassembly)
{
    /// <summary>
    /// Formats the line the way <c>nestest.log</c> does, minus the <c>PPU</c> column.
    /// </summary>
    public string Format()
    {
        var text = new StringBuilder(96);
        text.Append(Pc.ToString("X4", CultureInfo.InvariantCulture)).Append("  ");

        for (int i = 0; i < 3; i++)
        {
            text.Append(i < Bytes.Length
                ? Bytes[i].ToString("X2", CultureInfo.InvariantCulture)
                : "  ");
            text.Append(' ');
        }

        text.Append(' ').Append(Disassembly.PadRight(32));
        text.Append("A:").Append(A.ToString("X2", CultureInfo.InvariantCulture));
        text.Append(" X:").Append(X.ToString("X2", CultureInfo.InvariantCulture));
        text.Append(" Y:").Append(Y.ToString("X2", CultureInfo.InvariantCulture));
        text.Append(" P:").Append(P.ToString("X2", CultureInfo.InvariantCulture));
        text.Append(" SP:").Append(S.ToString("X2", CultureInfo.InvariantCulture));
        text.Append(" CYC:").Append(Cycles.ToString(CultureInfo.InvariantCulture));
        return text.ToString();
    }
}

/// <summary>
/// Produces <see cref="CpuTraceLine"/> values in the format of the public
/// <c>nestest.log</c>, so that a run can be compared line by line with it
/// (REQ-CORE-110).
/// </summary>
/// <remarks>
/// Every read it performs goes through <see cref="ICpuBus.Peek"/>: tracing must not change
/// what the emulation does (REQ-CORE-081), and the comparison would be meaningless
/// otherwise.
/// </remarks>
public static class CpuTracer
{
    /// <summary>Captures the state of <paramref name="cpu"/> before its next instruction.</summary>
    public static CpuTraceLine Capture(Cpu6502 cpu, ICpuBus bus)
    {
        ArgumentNullException.ThrowIfNull(cpu);
        ArgumentNullException.ThrowIfNull(bus);

        ushort pc = cpu.PC;
        byte opcode = bus.Peek(pc);
        OpcodeInfo info = OpcodeTable.Get(opcode);

        var bytes = new byte[info.Length];
        for (int i = 0; i < bytes.Length; i++)
        {
            bytes[i] = bus.Peek((ushort)(pc + i));
        }

        return new CpuTraceLine(
            pc,
            bytes,
            cpu.A,
            cpu.X,
            cpu.Y,
            cpu.P,
            cpu.S,
            cpu.Cycles,
            Disassemble(cpu, bus, pc, info, bytes));
    }

    /// <summary>
    /// Renders the instruction at <paramref name="pc"/> the way <c>nestest.log</c> does,
    /// resolved operand and pointed-at value included.
    /// </summary>
    public static string Disassemble(Cpu6502 cpu, ICpuBus bus, ushort pc, OpcodeInfo info, ReadOnlySpan<byte> bytes)
    {
        ArgumentNullException.ThrowIfNull(cpu);
        ArgumentNullException.ThrowIfNull(bus);

        string name = (info.Official ? " " : "*") + info.Mnemonic;
        byte lo = bytes.Length > 1 ? bytes[1] : (byte)0;
        byte hi = bytes.Length > 2 ? bytes[2] : (byte)0;
        var absolute = (ushort)(lo | (hi << 8));

        string Hex2(int value) => value.ToString("X2", CultureInfo.InvariantCulture);
        string Hex4(int value) => value.ToString("X4", CultureInfo.InvariantCulture);

        switch (info.Mode)
        {
            case AddressingMode.Implied:
                return name;

            case AddressingMode.Accumulator:
                return $"{name} A";

            case AddressingMode.Immediate:
                return $"{name} #${Hex2(lo)}";

            case AddressingMode.ZeroPage:
                return $"{name} ${Hex2(lo)} = {Hex2(bus.Peek(lo))}";

            case AddressingMode.ZeroPageX:
            {
                var effective = (byte)(lo + cpu.X);
                return $"{name} ${Hex2(lo)},X @ {Hex2(effective)} = {Hex2(bus.Peek(effective))}";
            }

            case AddressingMode.ZeroPageY:
            {
                var effective = (byte)(lo + cpu.Y);
                return $"{name} ${Hex2(lo)},Y @ {Hex2(effective)} = {Hex2(bus.Peek(effective))}";
            }

            case AddressingMode.Absolute:
                // JMP and JSR name their target; everything else shows the byte there.
                return info.Operation is CpuOperation.Jmp or CpuOperation.Jsr
                    ? $"{name} ${Hex4(absolute)}"
                    : $"{name} ${Hex4(absolute)} = {Hex2(bus.Peek(absolute))}";

            case AddressingMode.AbsoluteX:
            {
                var effective = (ushort)(absolute + cpu.X);
                return $"{name} ${Hex4(absolute)},X @ {Hex4(effective)} = {Hex2(bus.Peek(effective))}";
            }

            case AddressingMode.AbsoluteY:
            {
                var effective = (ushort)(absolute + cpu.Y);
                return $"{name} ${Hex4(absolute)},Y @ {Hex4(effective)} = {Hex2(bus.Peek(effective))}";
            }

            case AddressingMode.Indirect:
            {
                byte targetLo = bus.Peek(absolute);
                byte targetHi = bus.Peek((ushort)((absolute & 0xFF00) | ((absolute + 1) & 0x00FF)));
                return $"{name} (${Hex4(absolute)}) = {Hex4(targetLo | (targetHi << 8))}";
            }

            case AddressingMode.IndirectX:
            {
                var pointer = (byte)(lo + cpu.X);
                byte targetLo = bus.Peek(pointer);
                byte targetHi = bus.Peek((byte)(pointer + 1));
                var effective = (ushort)(targetLo | (targetHi << 8));
                return $"{name} (${Hex2(lo)},X) @ {Hex2(pointer)} = {Hex4(effective)} = {Hex2(bus.Peek(effective))}";
            }

            case AddressingMode.IndirectY:
            {
                byte targetLo = bus.Peek(lo);
                byte targetHi = bus.Peek((byte)(lo + 1));
                var @base = (ushort)(targetLo | (targetHi << 8));
                var effective = (ushort)(@base + cpu.Y);
                return $"{name} (${Hex2(lo)}),Y = {Hex4(@base)} @ {Hex4(effective)} = {Hex2(bus.Peek(effective))}";
            }

            case AddressingMode.Relative:
                return $"{name} ${Hex4((ushort)(pc + 2 + (sbyte)lo))}";

            default:
                return name;
        }
    }
}
