using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// The triangle channel: a 32-step sequencer gated by a linear counter and a length counter
/// (nesdev, "APU Triangle").
/// </summary>
internal sealed class TriangleChannel
{
    /// <summary>The 32-step staircase, 15 down to 0 and back up (nesdev).</summary>
    private static readonly byte[] Sequence =
    [
        15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    ];

    // Power-on step. nesdev does not fix the sequencer's power-on position; step 15 is picked
    // because its output is 0, so a machine that has just been powered on or reset is
    // genuinely silent (REQ-CORE-004) instead of holding the staircase's top step.
    private int _sequencePosition = 15;

    private int _timerPeriod;
    private int _timer;

    private bool _control;
    private byte _linearReloadValue;
    private byte _linearCounter;
    private bool _linearReload;

    /// <summary>The length counter $4015 reports on.</summary>
    public LengthCounter Length { get; } = new();

    /// <summary>
    /// Current output, 0 to 15. The sequencer is only clocked while both counters are
    /// running; when it is not, the channel keeps putting out its last step rather than
    /// falling to zero (nesdev).
    /// </summary>
    public int Output => Sequence[_sequencePosition];

    /// <summary>Writes register <paramref name="index"/> (0 to 3): $4008 to $400B.</summary>
    /// <param name="index">0 = $4008, 1 = unused, 2 = $400A, 3 = $400B.</param>
    /// <param name="value">Byte written.</param>
    /// <param name="cycle">APU cycle the write lands on, for the length reload quirk.</param>
    public void Write(int index, byte value, long cycle)
    {
        switch (index)
        {
            case 0:
                _control = (value & 0x80) != 0;
                Length.Halt = _control;
                _linearReloadValue = (byte)(value & 0x7F);
                break;

            case 2:
                _timerPeriod = (_timerPeriod & 0x700) | value;
                break;

            case 3:
                _timerPeriod = (_timerPeriod & 0x0FF) | ((value & 0x07) << 8);
                Length.Load(value, cycle);
                _linearReload = true;
                break;

            default:
                break;
        }
    }

    /// <summary>Clocks the sequencer's timer; called once per CPU cycle.</summary>
    /// <remarks>
    /// A period below 2 is "ultrasonic": the real channel then puts out a fixed level around
    /// the middle of the staircase instead of a tone no speaker could reproduce, and games
    /// use it to park the channel. The sequencer is held instead of being clocked, which
    /// keeps the output steady and keeps the resampler from being fed a delta every cycle.
    /// </remarks>
    public void ClockTimer()
    {
        if (_timerPeriod < 2 || _linearCounter == 0 || !Length.Active)
        {
            return;
        }

        if (_timer == 0)
        {
            _timer = _timerPeriod;
            _sequencePosition = (_sequencePosition + 1) & 31;
        }
        else
        {
            _timer--;
        }
    }

    /// <summary>Clocks the linear counter on a quarter-frame signal.</summary>
    public void ClockLinearCounter()
    {
        if (_linearReload)
        {
            _linearCounter = _linearReloadValue;
        }
        else if (_linearCounter > 0)
        {
            _linearCounter--;
        }

        if (!_control)
        {
            _linearReload = false;
        }
    }

    /// <summary>Back to the power-on state.</summary>
    public void Reset()
    {
        Length.Reset();

        _sequencePosition = 15;
        _timerPeriod = 0;
        _timer = 0;
        _control = false;
        _linearReloadValue = 0;
        _linearCounter = 0;
        _linearReload = false;
    }

    /// <summary>Appends the channel state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        Length.SaveState(writer);

        writer.WriteInt32(_sequencePosition);
        writer.WriteInt32(_timerPeriod);
        writer.WriteInt32(_timer);
        writer.WriteBool(_control);
        writer.WriteByte(_linearReloadValue);
        writer.WriteByte(_linearCounter);
        writer.WriteBool(_linearReload);
    }

    /// <summary>Restores the channel state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        Length.LoadState(reader);

        _sequencePosition = reader.ReadInt32();
        _timerPeriod = reader.ReadInt32();
        _timer = reader.ReadInt32();
        _control = reader.ReadBool();
        _linearReloadValue = reader.ReadByte();
        _linearCounter = reader.ReadByte();
        _linearReload = reader.ReadBool();
    }
}
