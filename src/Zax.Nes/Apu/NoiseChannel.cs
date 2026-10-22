using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// The noise channel: a 15-bit shift register with two tap modes, an envelope and a length
/// counter (nesdev, "APU Noise").
/// </summary>
internal sealed class NoiseChannel
{
    /// <summary>Timer periods in CPU cycles, NTSC (nesdev). MULTI and DENDY use these too.</summary>
    private static readonly int[] NtscPeriods =
        [4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068];

    /// <summary>Timer periods in CPU cycles, PAL (nesdev, REQ-CORE-015).</summary>
    private static readonly int[] PalPeriods =
        [4, 8, 14, 30, 60, 88, 118, 148, 188, 236, 354, 472, 708, 944, 1890, 3778];

    private int[] _periods = NtscPeriods;
    private int _periodIndex;
    private int _timer;
    private bool _shortMode;

    // The shift register is seeded with 1 at power-on; seeded with 0 it would never leave 0.
    private int _shiftRegister = 1;

    /// <summary>The envelope driving the channel's volume.</summary>
    public Envelope Envelope { get; } = new();

    /// <summary>The length counter $4015 reports on.</summary>
    public LengthCounter Length { get; } = new();

    /// <summary>Current output, 0 to 15.</summary>
    public int Output => (_shiftRegister & 1) != 0 || !Length.Active ? 0 : Envelope.Volume;

    /// <summary>Selects the NTSC or the PAL period table (REQ-CORE-015).</summary>
    public void SetTiming(RegionTiming timing) =>
        _periods = timing.UsesPalNoiseAndDmcPeriods ? PalPeriods : NtscPeriods;

    /// <summary>Writes register <paramref name="index"/> (0 to 3): $400C to $400F.</summary>
    /// <param name="index">0 = $400C, 1 = unused, 2 = $400E, 3 = $400F.</param>
    /// <param name="value">Byte written.</param>
    /// <param name="cycle">APU cycle the write lands on, for the length reload quirk.</param>
    public void Write(int index, byte value, long cycle)
    {
        switch (index)
        {
            case 0:
                Length.Halt = (value & 0x20) != 0;
                Envelope.Loop = (value & 0x20) != 0;
                Envelope.ConstantVolume = (value & 0x10) != 0;
                Envelope.Period = (byte)(value & 0x0F);
                break;

            case 2:
                _shortMode = (value & 0x80) != 0;
                _periodIndex = value & 0x0F;
                break;

            case 3:
                Length.Load(value, cycle);
                Envelope.Restart();
                break;

            default:
                break;
        }
    }

    /// <summary>Clocks the shift register's timer; called once per CPU cycle.</summary>
    public void ClockTimer()
    {
        if (_timer == 0)
        {
            _timer = _periods[_periodIndex] - 1;

            // Feedback is bit 0 xor bit 6 in short mode, bit 0 xor bit 1 otherwise (nesdev).
            int feedback = (_shiftRegister ^ (_shiftRegister >> (_shortMode ? 6 : 1))) & 1;
            _shiftRegister = (_shiftRegister >> 1) | (feedback << 14);
        }
        else
        {
            _timer--;
        }
    }

    /// <summary>Back to the power-on state.</summary>
    public void Reset()
    {
        Envelope.Reset();
        Length.Reset();

        _periodIndex = 0;
        _timer = 0;
        _shortMode = false;
        _shiftRegister = 1;
    }

    /// <summary>Appends the channel state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        Envelope.SaveState(writer);
        Length.SaveState(writer);

        writer.WriteInt32(_periodIndex);
        writer.WriteInt32(_timer);
        writer.WriteBool(_shortMode);
        writer.WriteInt32(_shiftRegister);
    }

    /// <summary>Restores the channel state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        Envelope.LoadState(reader);
        Length.LoadState(reader);

        _periodIndex = reader.ReadInt32();
        _timer = reader.ReadInt32();
        _shortMode = reader.ReadBool();
        _shiftRegister = reader.ReadInt32();
    }
}
