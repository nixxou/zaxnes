using Zax.Nes.Apu;
using Zax.Nes.Cartridges;
using Zax.Nes.Cpu;
using Zax.Nes.Input;
using Zax.Nes.Ppu;
using Zax.Nes.State;

namespace Zax.Nes;

/// <summary>
/// The CPU bus: memory map, open bus, OAM DMA and the master cycle clock (REQ-CORE-080 to
/// REQ-CORE-082).
/// </summary>
/// <remarks>
/// <para>
/// Map, after nesdev ("CPU memory map"):
/// $0000-$1FFF the 2 KiB of work RAM, mirrored four times; $2000-$3FFF the eight PPU
/// registers, mirrored every 8 bytes; $4000-$4017 the APU and I/O registers;
/// $4018-$401F disabled; $4020-$FFFF the cartridge.
/// </para>
/// <para>
/// Every <see cref="Read"/> and <see cref="Write"/> advances the clock by exactly one CPU
/// cycle, during which the PPU gets the dots of the current region and the APU one cycle.
/// Addresses nothing drives return the <strong>open bus</strong>: the last value that
/// travelled on the data bus.
/// </para>
/// </remarks>
public sealed class NesBus : ICpuBus
{
    /// <summary>Size of CPU work RAM.</summary>
    public const int RamSize = 2048;

    /// <summary>Cycles an OAM DMA steals from the CPU, on an even cycle (nesdev, "DMA").</summary>
    public const int OamDmaCyclesEven = 513;

    /// <summary>Cycles an OAM DMA steals from the CPU, on an odd cycle.</summary>
    public const int OamDmaCyclesOdd = 514;

    /// <summary>Cycles a DMC sample fetch steals from the CPU (nesdev, "DMA").</summary>
    public const int DmcDmaCycles = 4;

    private readonly byte[] _ram = new byte[RamSize];
    private readonly Ppu2C02 _ppu;
    private readonly Apu2A03 _apu;
    private readonly InputPorts _input;

    private Cartridge? _cartridge;
    private byte _openBus;
    private long _cycles;
    private bool _inDmcDma;

    /// <summary>Wires the bus to the other chips.</summary>
    public NesBus(Ppu2C02 ppu, Apu2A03 apu, InputPorts input)
    {
        ArgumentNullException.ThrowIfNull(ppu);
        ArgumentNullException.ThrowIfNull(apu);
        ArgumentNullException.ThrowIfNull(input);

        _ppu = ppu;
        _apu = apu;
        _input = input;
    }

    /// <inheritdoc />
    public long CycleCount => _cycles;

    /// <inheritdoc />
    /// <remarks>
    /// The CPU sees the /NMI line as it stood near the start of the cycle, not after the
    /// cycle's own register access (see <see cref="Ppu.Ppu2C02.NmiLineSample"/>).
    /// </remarks>
    public bool NmiAsserted => _ppu.NmiLineSample;

    /// <inheritdoc />
    public bool IrqAsserted => _apu.IrqAsserted || (_cartridge?.IrqAsserted ?? false);

    /// <summary>Last value seen on the data bus, returned by unmapped reads.</summary>
    public byte OpenBus => _openBus;

    /// <summary>CPU work RAM.</summary>
    public ReadOnlySpan<byte> Ram => _ram;

    /// <summary>Number of OAM DMA transfers performed, for the DMA timing tests.</summary>
    public long OamDmaCount { get; private set; }

    /// <summary>Number of DMC sample fetches performed, for the DMA tests.</summary>
    public long DmcDmaCount { get; private set; }

    /// <summary>Attaches <paramref name="cartridge"/>, or detaches it with <see langword="null"/>.</summary>
    public void Attach(Cartridge? cartridge)
    {
        _cartridge = cartridge;
        _ppu.Attach(cartridge);
    }

    /// <summary>Clears work RAM and the clock (part of power-on, REQ-CORE-004).</summary>
    public void Reset()
    {
        Array.Clear(_ram);
        _openBus = 0;
        _cycles = 0;
        _inDmcDma = false;
        OamDmaCount = 0;
        DmcDmaCount = 0;
    }

    /// <inheritdoc />
    public byte Read(ushort address)
    {
        Tick();
        byte value = ReadInternal(address);
        _openBus = value;
        return value;
    }

    /// <inheritdoc />
    public void Write(ushort address, byte value)
    {
        Tick();
        _openBus = value;
        WriteInternal(address, value);
    }

    /// <summary>
    /// Advances the whole machine by one CPU cycle.
    /// </summary>
    public void Tick()
    {
        _cycles++;
        _ppu.StepCpuCycle();
        _apu.StepCpuCycle();
        _cartridge?.StepCpuCycle();

        if (_apu.NeedsDmcDma && !_inDmcDma)
        {
            PerformDmcDma();
        }
    }

    /// <summary>
    /// Fetches one DMC sample byte, halting the CPU for <see cref="DmcDmaCycles"/> cycles
    /// (nesdev, "DMA": the DMC's memory reader takes the bus for up to four cycles, the last
    /// of which is the read).
    /// </summary>
    /// <remarks>
    /// The halt is placed on the cycle the APU asked for the byte rather than on the next
    /// cycle the CPU could actually be stopped at, so a fetch that collides with a read of
    /// $2007 or $4016 does not reproduce the hardware's double read. Those collisions are
    /// what <c>dmc_dma_during_read4</c> and <c>sprdma_and_dmc_dma</c> measure; REQ-CORE-113
    /// makes them P3.
    /// </remarks>
    private void PerformDmcDma()
    {
        _inDmcDma = true;
        DmcDmaCount++;

        for (int i = 1; i < DmcDmaCycles; i++)
        {
            Tick();
        }

        Tick();
        byte value = ReadInternal(_apu.DmcDmaAddress);
        _openBus = value;
        _apu.SupplyDmcByte(value);

        _inDmcDma = false;
    }

    private byte ReadInternal(ushort address)
    {
        if (address < 0x2000)
        {
            return _ram[address & (RamSize - 1)];
        }

        if (address < 0x4000)
        {
            // The PPU drives its own data-bus latch for the undefined bits, separate from
            // the CPU's open bus (nesdev, "PPU registers"; ppu_open_bus).
            return _ppu.Read(address & 0x07);
        }

        if (address < 0x4020)
        {
            return address switch
            {
                0x4015 => _apu.ReadStatus(_openBus),
                0x4016 => _input.Read(0, _openBus),
                0x4017 => _input.Read(1, _openBus),

                // $4000-$4014 are write-only and $4018-$401F are disabled: open bus.
                _ => _openBus,
            };
        }

        return _cartridge is not null && _cartridge.TryCpuRead(address, out byte value) ? value : _openBus;
    }

    private void WriteInternal(ushort address, byte value)
    {
        if (address < 0x2000)
        {
            _ram[address & (RamSize - 1)] = value;
            return;
        }

        if (address < 0x4000)
        {
            _ppu.Write(address & 0x07, value);
            return;
        }

        if (address < 0x4020)
        {
            switch (address)
            {
                case 0x4014:
                    PerformOamDma(value);
                    return;

                case 0x4016:
                    _input.Write(value);
                    return;

                default:
                    _apu.Write(address, value);
                    return;
            }
        }

        _cartridge?.CpuWrite(address, value);
    }

    /// <summary>
    /// Copies a 256-byte page into OAM and halts the CPU for 513 cycles, 514 when the
    /// write landed on an odd CPU cycle (nesdev, "DMA").
    /// </summary>
    /// <remarks>
    /// The write cycle itself is already counted by <see cref="Write"/>; the cycles below
    /// are the ones stolen from the CPU on top of it.
    /// </remarks>
    private void PerformOamDma(byte page)
    {
        OamDmaCount++;

        // Parity is read at the $4014 write, before the wait state.
        bool oddCycle = (_cycles & 1) != 0;

        Tick(); // Wait state while the DMA unit takes the bus.

        if (oddCycle)
        {
            Tick(); // Alignment cycle: the transfer only starts on a "get" cycle.
        }

        var baseAddress = (ushort)(page << 8);
        for (int i = 0; i < 256; i++)
        {
            Tick();
            byte value = ReadInternal((ushort)(baseAddress + i));
            _openBus = value;

            Tick();
            _ppu.WriteOamDma(value);
        }
    }

    /// <summary>
    /// Reads a byte with no side effect and without advancing time (REQ-CORE-080,
    /// REQ-CORE-081).
    /// </summary>
    /// <remarks>
    /// $2000-$401F reads back as 0, as REQ-CORE-081 asks, so that a host or a script
    /// peeking at $2002 cannot clear the VBlank flag under the game's feet.
    /// </remarks>
    public byte Peek(ushort address)
    {
        if (address < 0x2000)
        {
            return _ram[address & (RamSize - 1)];
        }

        if (address < 0x4020)
        {
            return 0;
        }

        return _cartridge is not null && _cartridge.TryCpuPeek(address, out byte value) ? value : (byte)0;
    }

    /// <summary>
    /// Writes a byte exactly as a CPU write would, without advancing emulated time
    /// (REQ-CORE-082).
    /// </summary>
    public void Poke(ushort address, byte value)
    {
        _openBus = value;
        WriteInternal(address, value);
    }

    /// <summary>Appends the bus state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        ArgumentNullException.ThrowIfNull(writer);

        writer.WriteBlob(_ram);
        writer.WriteByte(_openBus);
        writer.WriteInt64(_cycles);
        writer.WriteInt64(OamDmaCount);
        writer.WriteInt64(DmcDmaCount);
    }

    /// <summary>Restores the bus state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        ArgumentNullException.ThrowIfNull(reader);

        reader.ReadBlobInto(_ram);
        _openBus = reader.ReadByte();
        _cycles = reader.ReadInt64();
        OamDmaCount = reader.ReadInt64();
        DmcDmaCount = reader.ReadInt64();
        _inDmcDma = false;
    }
}
