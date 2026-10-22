namespace Zax.Nes.Cpu;

/// <summary>
/// The bus the CPU is wired to. Every <see cref="Read"/> and <see cref="Write"/> is one
/// CPU cycle: the implementation advances the rest of the machine (PPU, APU, cycle-counted
/// mapper logic) inside the call, which is what makes the cycle counts of REQ-CORE-015 and
/// the interleaving of REQ-CORE-110 come out right.
/// </summary>
public interface ICpuBus
{
    /// <summary>Reads a byte, consuming one CPU cycle. Side effects of the read apply.</summary>
    byte Read(ushort address);

    /// <summary>Writes a byte, consuming one CPU cycle.</summary>
    void Write(ushort address, byte value);

    /// <summary>
    /// Reads a byte without consuming a cycle and without any side effect, for the host
    /// memory accessor and the tracer (REQ-CORE-080, REQ-CORE-081).
    /// </summary>
    byte Peek(ushort address);

    /// <summary>
    /// Whether the /NMI line is currently pulled low. The CPU detects its falling edge.
    /// </summary>
    bool NmiAsserted { get; }

    /// <summary>
    /// Whether any IRQ source (APU frame counter, DMC, mapper) is holding /IRQ low. This
    /// is a level, not an edge.
    /// </summary>
    bool IrqAsserted { get; }

    /// <summary>
    /// Master CPU cycle count since power-on. It includes the cycles the CPU is halted
    /// for, such as OAM DMA (REQ-CORE-110, <c>CYC</c> column of <c>nestest.log</c>).
    /// </summary>
    long CycleCount { get; }
}
