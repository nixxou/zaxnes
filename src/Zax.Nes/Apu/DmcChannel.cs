using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// The delta modulation channel: memory reader, output unit and IRQ (nesdev, "APU DMC").
/// </summary>
/// <remarks>
/// The channel does not own the bus. When its one-byte sample buffer runs dry it raises
/// <see cref="NeedsSampleFetch"/>; <see cref="NesBus"/> then halts the CPU for the DMA and
/// hands the byte back through <see cref="SupplyByte(byte)"/>.
/// </remarks>
internal sealed class DmcChannel
{
    /// <summary>Timer periods in CPU cycles per output bit, NTSC (nesdev).</summary>
    private static readonly int[] NtscRates =
        [428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54];

    /// <summary>Timer periods in CPU cycles per output bit, PAL (nesdev, REQ-CORE-015).</summary>
    private static readonly int[] PalRates =
        [398, 354, 316, 298, 276, 236, 210, 198, 176, 148, 132, 118, 98, 78, 66, 50];

    private int[] _rates = NtscRates;
    private int _rateIndex;
    private int _timer;

    private bool _irqEnabled;
    private bool _loop;

    private ushort _sampleAddress = 0xC000;
    private int _sampleLength = 1;

    private ushort _currentAddress = 0xC000;
    private int _bytesRemaining;

    private byte _sampleBuffer;
    private bool _bufferEmpty = true;

    private byte _shiftRegister;
    private int _bitsRemaining = 8;
    private bool _silence = true;

    /// <summary>Output level, 0 to 127 ($4011, and the delta decoder).</summary>
    public int Output { get; private set; }

    /// <summary>Whether the channel is pulling /IRQ low (bit 7 of $4015).</summary>
    public bool IrqFlag { get; private set; }

    /// <summary>Whether a sample is still playing (bit 4 of $4015).</summary>
    public bool Active => _bytesRemaining > 0;

    /// <summary>
    /// Whether the memory reader wants a byte: the sample buffer is empty and bytes remain.
    /// </summary>
    public bool NeedsSampleFetch => _bufferEmpty && _bytesRemaining > 0;

    /// <summary>Address the next DMA fetch reads.</summary>
    public ushort FetchAddress => _currentAddress;

    /// <summary>Selects the NTSC or the PAL rate table (REQ-CORE-015).</summary>
    public void SetTiming(RegionTiming timing) =>
        _rates = timing.UsesPalNoiseAndDmcPeriods ? PalRates : NtscRates;

    /// <summary>Writes register <paramref name="index"/> (0 to 3): $4010 to $4013.</summary>
    public void Write(int index, byte value)
    {
        switch (index)
        {
            case 0:
                _irqEnabled = (value & 0x80) != 0;
                _loop = (value & 0x40) != 0;
                _rateIndex = value & 0x0F;

                // "Disabling IRQ flag should clear it" (apu_test 7-dmc_basics).
                if (!_irqEnabled)
                {
                    IrqFlag = false;
                }

                break;

            case 1:
                Output = value & 0x7F;
                break;

            case 2:
                _sampleAddress = (ushort)(0xC000 + (value << 6));
                break;

            default:
                _sampleLength = (value << 4) + 1;
                break;
        }
    }

    /// <summary>
    /// Applies bit 4 of a $4015 write: starts the sample when it is set and nothing is
    /// playing, stops it when it is clear. A $4015 write always clears the DMC IRQ flag
    /// (nesdev).
    /// </summary>
    public void SetEnabled(bool enabled)
    {
        IrqFlag = false;

        if (!enabled)
        {
            _bytesRemaining = 0;
            return;
        }

        if (_bytesRemaining == 0)
        {
            Restart();
        }
    }

    /// <summary>Clocks the output unit's timer; called once per CPU cycle.</summary>
    public void ClockTimer()
    {
        if (_timer > 0)
        {
            _timer--;
            return;
        }

        _timer = _rates[_rateIndex] - 1;

        if (!_silence)
        {
            if ((_shiftRegister & 1) != 0)
            {
                if (Output <= 125)
                {
                    Output += 2;
                }
            }
            else if (Output >= 2)
            {
                Output -= 2;
            }
        }

        _shiftRegister >>= 1;

        if (--_bitsRemaining == 0)
        {
            _bitsRemaining = 8;

            if (_bufferEmpty)
            {
                _silence = true;
            }
            else
            {
                _silence = false;
                _shiftRegister = _sampleBuffer;
                _bufferEmpty = true;
            }
        }
    }

    /// <summary>
    /// Hands the byte the DMA read over to the memory reader. Advances the address, which
    /// wraps from $FFFF back to $8000, and ends or loops the sample when the last byte has
    /// been consumed (nesdev).
    /// </summary>
    public void SupplyByte(byte value)
    {
        _sampleBuffer = value;
        _bufferEmpty = false;
        _currentAddress = _currentAddress == 0xFFFF ? (ushort)0x8000 : (ushort)(_currentAddress + 1);

        if (--_bytesRemaining > 0)
        {
            return;
        }

        if (_loop)
        {
            Restart();
        }
        else if (_irqEnabled)
        {
            IrqFlag = true;
        }
    }

    private void Restart()
    {
        _currentAddress = _sampleAddress;
        _bytesRemaining = _sampleLength;
    }

    /// <summary>Back to the power-on state.</summary>
    public void Reset()
    {
        _rateIndex = 0;
        _timer = 0;
        _irqEnabled = false;
        _loop = false;
        _sampleAddress = 0xC000;
        _sampleLength = 1;
        _currentAddress = 0xC000;
        _bytesRemaining = 0;
        _sampleBuffer = 0;
        _bufferEmpty = true;
        _shiftRegister = 0;
        _bitsRemaining = 8;
        _silence = true;
        Output = 0;
        IrqFlag = false;
    }

    /// <summary>Appends the channel state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        writer.WriteInt32(_rateIndex);
        writer.WriteInt32(_timer);
        writer.WriteBool(_irqEnabled);
        writer.WriteBool(_loop);
        writer.WriteUInt16(_sampleAddress);
        writer.WriteInt32(_sampleLength);
        writer.WriteUInt16(_currentAddress);
        writer.WriteInt32(_bytesRemaining);
        writer.WriteByte(_sampleBuffer);
        writer.WriteBool(_bufferEmpty);
        writer.WriteByte(_shiftRegister);
        writer.WriteInt32(_bitsRemaining);
        writer.WriteBool(_silence);
        writer.WriteInt32(Output);
        writer.WriteBool(IrqFlag);
    }

    /// <summary>Restores the channel state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        _rateIndex = reader.ReadInt32();
        _timer = reader.ReadInt32();
        _irqEnabled = reader.ReadBool();
        _loop = reader.ReadBool();
        _sampleAddress = reader.ReadUInt16();
        _sampleLength = reader.ReadInt32();
        _currentAddress = reader.ReadUInt16();
        _bytesRemaining = reader.ReadInt32();
        _sampleBuffer = reader.ReadByte();
        _bufferEmpty = reader.ReadBool();
        _shiftRegister = reader.ReadByte();
        _bitsRemaining = reader.ReadInt32();
        _silence = reader.ReadBool();
        Output = reader.ReadInt32();
        IrqFlag = reader.ReadBool();
    }
}
