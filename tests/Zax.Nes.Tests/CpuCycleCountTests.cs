using Zax.Nes.Cpu;

namespace Zax.Nes.Tests;

/// <summary>
/// Cycle counts of every opcode, and the page-cross and branch penalties
/// (REQ-CORE-110, REQ-CORE-111).
/// </summary>
/// <remarks>
/// The expected counts are the published 6502 cycle table (nesdev "6502 instruction
/// reference" and "CPU unofficial opcodes"), transcribed here as an independent reference:
/// the core derives its counts from its bus accesses, not from a table, so a mistake in
/// either one shows up.
/// </remarks>
public sealed class CpuCycleCountTests
{
    /// <summary>
    /// Base cycle count per opcode, indexed by opcode. 0 marks a <c>JAM</c>, which halts
    /// the chip and has no documented count. Branches carry their <em>not taken</em>
    /// count and are checked separately.
    /// </summary>
    private static readonly byte[] BaseCycles =
    [
        /*       0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F */
        /* 00 */ 7, 6, 0, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6,
        /* 10 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
        /* 20 */ 6, 6, 0, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6,
        /* 30 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
        /* 40 */ 6, 6, 0, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6,
        /* 50 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
        /* 60 */ 6, 6, 0, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6,
        /* 70 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
        /* 80 */ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
        /* 90 */ 2, 6, 0, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,
        /* A0 */ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
        /* B0 */ 2, 5, 0, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,
        /* C0 */ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
        /* D0 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
        /* E0 */ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
        /* F0 */ 2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    ];

    [Fact]
    public void REQ_CORE_110_every_opcode_takes_the_documented_number_of_cycles()
    {
        var failures = new List<string>();

        for (int value = 0; value < 256; value++)
        {
            var opcode = (byte)value;
            OpcodeInfo info = OpcodeTable.Get(opcode);

            if (info.Operation == CpuOperation.Jam || info.Mode == AddressingMode.Relative)
            {
                // JAM freezes the chip; branches depend on the flags and are checked below.
                continue;
            }

            var bus = new TestBus();

            // Operands chosen so that no index crosses a page: $10 in page zero, $2010 for
            // absolute, and both index registers at 0.
            bus.Load(0x1000, opcode, 0x10, 0x20);
            Cpu6502 cpu = bus.CreateCpu();

            int actual = cpu.Step();
            int expected = BaseCycles[opcode];

            if (actual != expected)
            {
                failures.Add(
                    $"${opcode:X2} {(info.Official ? " " : "*")}{info.Mnemonic} {info.Mode}: "
                    + $"expected {expected}, got {actual}");
            }
        }

        Assert.Empty(failures);
    }

    [Fact]
    public void REQ_CORE_110_jam_freezes_the_cpu_without_hanging_the_host()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x02);
        Cpu6502 cpu = bus.CreateCpu();

        cpu.Step();
        Assert.True(cpu.Jammed);
        Assert.Equal(0x1001, cpu.PC);

        // Further steps keep burning cycles without advancing PC.
        long before = bus.CycleCount;
        cpu.Step();
        Assert.True(bus.CycleCount > before);
        Assert.Equal(0x1001, cpu.PC);
    }

    [Theory]
    [InlineData(0xBD, 1, 4)] // LDA abs,X  — read, no page cross.
    [InlineData(0xBD, 0xFF, 5)] // LDA abs,X  — read, page crossed: +1.
    [InlineData(0xB9, 0xFF, 5)] // LDA abs,Y  — read, page crossed: +1.
    [InlineData(0x9D, 1, 5)] // STA abs,X  — write always pays the extra cycle.
    [InlineData(0x9D, 0xFF, 5)] // STA abs,X  — and nothing more on a cross.
    [InlineData(0xFE, 1, 7)] // INC abs,X  — read-modify-write, always 7.
    [InlineData(0xFE, 0xFF, 7)]
    public void REQ_CORE_110_absolute_indexed_page_cross_penalty(byte opcode, byte index, int expected)
    {
        var bus = new TestBus();

        // Base address $2010: index $FF crosses into page $21.
        bus.Load(0x1000, opcode, 0x10, 0x20);
        Cpu6502 cpu = bus.CreateCpu();
        cpu.X = index;
        cpu.Y = index;

        Assert.Equal(expected, cpu.Step());
    }

    [Theory]
    [InlineData(0xB1, 0x01, 5)] // LDA (ind),Y — no page cross.
    [InlineData(0xB1, 0xFF, 6)] // LDA (ind),Y — page crossed: +1.
    [InlineData(0x91, 0x01, 6)] // STA (ind),Y — write always pays it.
    [InlineData(0x91, 0xFF, 6)]
    public void REQ_CORE_110_indirect_indexed_page_cross_penalty(byte opcode, byte y, int expected)
    {
        var bus = new TestBus();
        bus.Load(0x1000, opcode, 0x10);
        bus[0x10] = 0x10;
        bus[0x11] = 0x20; // Pointer → $2010.

        Cpu6502 cpu = bus.CreateCpu();
        cpu.Y = y;

        Assert.Equal(expected, cpu.Step());
    }

    [Theory]
    [InlineData(0xD0, true, 0x10, 2)] // BNE with Z set: not taken.
    [InlineData(0xD0, false, 0x10, 3)] // BNE with Z clear: taken, same page.
    [InlineData(0xF0, false, 0x10, 2)] // BEQ with Z clear: not taken.
    [InlineData(0xF0, true, 0x10, 3)] // BEQ with Z set: taken, same page.
    public void REQ_CORE_111_branch_costs_two_three_or_four_cycles(
        byte opcode, bool zeroFlag, byte offset, int expected)
    {
        var bus = new TestBus();
        bus.Load(0x1000, opcode, offset);

        Cpu6502 cpu = bus.CreateCpu();
        cpu.SetFlag(Cpu6502.FlagZ, zeroFlag);

        Assert.Equal(expected, cpu.Step());
    }

    [Fact]
    public void REQ_CORE_111_branch_across_a_page_costs_four_cycles()
    {
        var bus = new TestBus();

        // At $10F0, a +$20 branch lands at $1112: the high byte of PC changes.
        bus.Load(0x10F0, 0xD0, 0x20);
        Cpu6502 cpu = bus.CreateCpu(0x10F0);

        Assert.Equal(4, cpu.Step());
        Assert.Equal(0x1112, cpu.PC);
    }

    [Fact]
    public void REQ_CORE_111_backward_branch_across_a_page_costs_four_cycles()
    {
        var bus = new TestBus();

        // At $1001, a -$10 branch lands at $0FF3.
        bus.Load(0x1001, 0xD0, 0xF0);
        Cpu6502 cpu = bus.CreateCpu(0x1001);

        Assert.Equal(4, cpu.Step());
        Assert.Equal(0x0FF3, cpu.PC);
    }

    [Fact]
    public void REQ_CORE_110_read_modify_write_writes_the_old_value_back_first()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xE6, 0x10); // INC $10
        bus[0x10] = 0x41;

        Cpu6502 cpu = bus.CreateCpu();
        Assert.Equal(5, cpu.Step());
        Assert.Equal(0x42, bus[0x10]);

        // Opcode, operand, read, dummy write, write: five accesses, the last two at $10.
        Assert.Equal<ushort[]>([0x1000, 0x1001, 0x0010, 0x0010, 0x0010], [.. bus.AccessLog]);
    }

    [Fact]
    public void REQ_CORE_110_zero_page_indexed_wraps_inside_page_zero()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xB5, 0xFF); // LDA $FF,X
        bus[0x7F] = 0x5A;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.X = 0x80; // $FF + $80 = $7F, not $017F.

        Assert.Equal(4, cpu.Step());
        Assert.Equal(0x5A, cpu.A);
    }

    [Fact]
    public void REQ_CORE_110_indexed_indirect_pointer_wraps_inside_page_zero()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0xA1, 0xFF); // LDA ($FF,X)
        bus[0xFF] = 0x34;
        bus[0x00] = 0x12; // High byte comes from $00, not $0100.
        bus[0x1234] = 0x7E;

        Cpu6502 cpu = bus.CreateCpu();
        cpu.X = 0x00;

        Assert.Equal(6, cpu.Step());
        Assert.Equal(0x7E, cpu.A);
    }

    [Fact]
    public void REQ_CORE_110_indirect_jmp_does_not_carry_into_the_high_byte()
    {
        var bus = new TestBus();
        bus.Load(0x1000, 0x6C, 0xFF, 0x20); // JMP ($20FF)
        bus[0x20FF] = 0x34;
        bus[0x2000] = 0x12; // The NMOS bug: the high byte is read from $2000.
        bus[0x2100] = 0xFF;

        Cpu6502 cpu = bus.CreateCpu();

        Assert.Equal(5, cpu.Step());
        Assert.Equal(0x1234, cpu.PC);
    }
}
