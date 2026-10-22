using Zax.Nes.Cpu;

namespace Zax.Nes.Tests;

/// <summary>
/// A flat 64 KiB memory with no device on it, used to measure instruction behaviour and
/// cycle counts in isolation. Every access costs exactly one cycle, like the real bus.
/// </summary>
internal sealed class TestBus : ICpuBus
{
    private readonly byte[] _memory = new byte[0x10000];

    public long CycleCount { get; private set; }

    public bool NmiAsserted { get; set; }

    public bool IrqAsserted { get; set; }

    /// <summary>Every address read or written, in order, for dummy-cycle assertions.</summary>
    public List<ushort> AccessLog { get; } = [];

    public byte this[int address]
    {
        get => _memory[address];
        set => _memory[address] = value;
    }

    public byte Read(ushort address)
    {
        CycleCount++;
        AccessLog.Add(address);
        return _memory[address];
    }

    public void Write(ushort address, byte value)
    {
        CycleCount++;
        AccessLog.Add(address);
        _memory[address] = value;
    }

    public byte Peek(ushort address) => _memory[address];

    /// <summary>Writes <paramref name="bytes"/> starting at <paramref name="address"/>.</summary>
    public void Load(int address, params byte[] bytes) => bytes.CopyTo(_memory, address);

    /// <summary>
    /// Builds a CPU whose PC is at <paramref name="origin"/>, past the reset sequence, with
    /// the cycle counter back at zero so that a single <see cref="Cpu6502.Step"/> can be
    /// measured.
    /// </summary>
    public Cpu6502 CreateCpu(ushort origin = 0x1000, byte stackPointer = 0xFD)
    {
        var cpu = new Cpu6502(this) { PC = origin, S = stackPointer };
        CycleCount = 0;
        AccessLog.Clear();
        return cpu;
    }
}
