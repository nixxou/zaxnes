using Zax.Nes.Cpu;

namespace Zax.Nes.Tests;

/// <summary>
/// The opcode matrix: completeness, official/unofficial split and instruction lengths
/// (REQ-CORE-110).
/// </summary>
public sealed class OpcodeTableTests
{
    [Fact]
    public void REQ_CORE_110_all_256_opcodes_are_described()
    {
        for (int opcode = 0; opcode < 256; opcode++)
        {
            OpcodeInfo info = OpcodeTable.Get((byte)opcode);
            Assert.True(Enum.IsDefined(info.Operation), $"Opcode ${opcode:X2} has no operation.");
            Assert.True(Enum.IsDefined(info.Mode), $"Opcode ${opcode:X2} has no addressing mode.");
            Assert.InRange(info.Length, 1, 3);
        }
    }

    [Fact]
    public void REQ_CORE_110_there_are_151_official_opcodes()
    {
        // The documented NMOS 6502 has 151 official opcodes; the other 105 are the
        // unofficial ones nestest prefixes with '*'.
        int official = 0;
        for (int opcode = 0; opcode < 256; opcode++)
        {
            if (OpcodeTable.Get((byte)opcode).Official)
            {
                official++;
            }
        }

        Assert.Equal(151, official);
    }

    [Fact]
    public void REQ_CORE_110_the_twelve_jam_opcodes_are_in_place()
    {
        byte[] expected = [0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xB2, 0xD2, 0xF2];

        var found = new List<byte>();
        for (int opcode = 0; opcode < 256; opcode++)
        {
            if (OpcodeTable.Get((byte)opcode).Operation == CpuOperation.Jam)
            {
                found.Add((byte)opcode);
            }
        }

        Assert.Equal(expected, found);
    }

    [Theory]
    // The ten unofficial mnemonics that appear in the public nestest.log.
    [InlineData(0x07, CpuOperation.Slo)]
    [InlineData(0x27, CpuOperation.Rla)]
    [InlineData(0x47, CpuOperation.Sre)]
    [InlineData(0x67, CpuOperation.Rra)]
    [InlineData(0x87, CpuOperation.Sax)]
    [InlineData(0xA7, CpuOperation.Lax)]
    [InlineData(0xC7, CpuOperation.Dcp)]
    [InlineData(0xE7, CpuOperation.Isb)]
    [InlineData(0xEB, CpuOperation.Sbc)]
    [InlineData(0x1A, CpuOperation.Nop)]
    public void REQ_CORE_110_nestest_unofficial_opcodes_are_mapped(byte opcode, CpuOperation expected)
    {
        OpcodeInfo info = OpcodeTable.Get(opcode);

        Assert.Equal(expected, info.Operation);
        Assert.False(info.Official);
    }

    [Theory]
    [InlineData(0xEA, AddressingMode.Implied, 1)]
    [InlineData(0x0A, AddressingMode.Accumulator, 1)]
    [InlineData(0xA9, AddressingMode.Immediate, 2)]
    [InlineData(0xA5, AddressingMode.ZeroPage, 2)]
    [InlineData(0xB5, AddressingMode.ZeroPageX, 2)]
    [InlineData(0xB6, AddressingMode.ZeroPageY, 2)]
    [InlineData(0xAD, AddressingMode.Absolute, 3)]
    [InlineData(0xBD, AddressingMode.AbsoluteX, 3)]
    [InlineData(0xB9, AddressingMode.AbsoluteY, 3)]
    [InlineData(0x6C, AddressingMode.Indirect, 3)]
    [InlineData(0xA1, AddressingMode.IndirectX, 2)]
    [InlineData(0xB1, AddressingMode.IndirectY, 2)]
    [InlineData(0xD0, AddressingMode.Relative, 2)]
    public void REQ_CORE_110_addressing_modes_and_lengths(byte opcode, AddressingMode mode, int length)
    {
        OpcodeInfo info = OpcodeTable.Get(opcode);

        Assert.Equal(mode, info.Mode);
        Assert.Equal(length, info.Length);
    }

    [Fact]
    public void REQ_CORE_110_read_modify_write_opcodes_are_tagged_as_such()
    {
        // Every ASL, LSR, ROL, ROR, INC, DEC and their unofficial combinations that have a
        // memory operand must be read-modify-write, which is what gives them their dummy
        // write and their unconditional indexing penalty.
        CpuOperation[] rmw =
        [
            CpuOperation.Asl, CpuOperation.Lsr, CpuOperation.Rol, CpuOperation.Ror,
            CpuOperation.Inc, CpuOperation.Dec,
            CpuOperation.Slo, CpuOperation.Rla, CpuOperation.Sre, CpuOperation.Rra,
            CpuOperation.Dcp, CpuOperation.Isb,
        ];

        for (int opcode = 0; opcode < 256; opcode++)
        {
            OpcodeInfo info = OpcodeTable.Get((byte)opcode);
            if (!rmw.Contains(info.Operation))
            {
                continue;
            }

            bool hasMemoryOperand = info.Mode is not (AddressingMode.Accumulator or AddressingMode.Implied);
            Assert.Equal(
                hasMemoryOperand ? OperandAccess.ReadModifyWrite : OperandAccess.None,
                info.Access);
        }
    }
}
