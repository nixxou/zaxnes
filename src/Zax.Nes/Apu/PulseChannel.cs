using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// One of the two pulse channels: duty sequencer, envelope, sweep and length counter
/// (nesdev, "APU Pulse", "APU Sweep").
/// </summary>
internal sealed class PulseChannel
{
    /// <summary>
    /// The four duty sequences, read left to right over time (nesdev): 12.5 %, 25 %, 50 %
    /// and 25 % negated.
    /// </summary>
    private static readonly byte[] DutySequences =
    [
        0, 1, 0, 0, 0, 0, 0, 0,
        0, 1, 1, 0, 0, 0, 0, 0,
        0, 1, 1, 1, 1, 0, 0, 0,
        1, 0, 0, 1, 1, 1, 1, 1,
    ];

    private readonly bool _isFirst;

    private int _duty;
    private int _sequencePosition;
    private int _timerPeriod;
    private int _timer;

    private bool _sweepEnabled;
    private bool _sweepNegate;
    private bool _sweepReload;
    private int _sweepPeriod;
    private int _sweepShift;
    private int _sweepDivider;

    /// <summary>
    /// Creates a pulse channel. <paramref name="isFirst"/> selects the one-off difference
    /// between the two sweep units: pulse 1 negates with a one's complement, pulse 2 with a
    /// two's complement (nesdev).
    /// </summary>
    public PulseChannel(bool isFirst) => _isFirst = isFirst;

    /// <summary>The envelope driving the channel's volume.</summary>
    public Envelope Envelope { get; } = new();

    /// <summary>The length counter $4015 reports on.</summary>
    public LengthCounter Length { get; } = new();

    /// <summary>Current output, 0 to 15.</summary>
    public int Output =>
        Muted || !Length.Active || DutySequences[(_duty * 8) + _sequencePosition] == 0
            ? 0
            : Envelope.Volume;

    /// <summary>
    /// Whether the sweep unit silences the channel: the timer is too short for the sequencer
    /// to be audible, or the sweep target has run past the 11-bit range (nesdev).
    /// </summary>
    private bool Muted => _timerPeriod < 8 || SweepTarget() > 0x7FF;

    /// <summary>Writes register <paramref name="index"/> (0 to 3) of the channel.</summary>
    /// <param name="index">0 = $4000, 1 = $4001, 2 = $4002, 3 = $4003 (and $4004 upwards).</param>
    /// <param name="value">Byte written.</param>
    /// <param name="cycle">APU cycle the write lands on, for the length reload quirk.</param>
    public void Write(int index, byte value, long cycle)
    {
        switch (index)
        {
            case 0:
                _duty = value >> 6;
                Length.Halt = (value & 0x20) != 0;
                Envelope.Loop = (value & 0x20) != 0;
                Envelope.ConstantVolume = (value & 0x10) != 0;
                Envelope.Period = (byte)(value & 0x0F);
                break;

            case 1:
                _sweepEnabled = (value & 0x80) != 0;
                _sweepPeriod = (value >> 4) & 0x07;
                _sweepNegate = (value & 0x08) != 0;
                _sweepShift = value & 0x07;
                _sweepReload = true;
                break;

            case 2:
                _timerPeriod = (_timerPeriod & 0x700) | value;
                break;

            default:
                _timerPeriod = (_timerPeriod & 0x0FF) | ((value & 0x07) << 8);
                Length.Load(value, cycle);

                // The sequencer restarts at the first value of the sequence and the envelope
                // is restarted (nesdev). The timer itself is not reloaded.
                _sequencePosition = 0;
                Envelope.Restart();
                break;
        }
    }

    /// <summary>Clocks the duty sequencer's timer; called once per APU cycle (CPU / 2).</summary>
    public void ClockTimer()
    {
        if (_timer == 0)
        {
            _timer = _timerPeriod;
            _sequencePosition = (_sequencePosition + 1) & 7;
        }
        else
        {
            _timer--;
        }
    }

    /// <summary>Clocks the sweep unit on a half-frame signal.</summary>
    public void ClockSweep()
    {
        int target = SweepTarget();

        if (_sweepDivider == 0 && _sweepEnabled && _sweepShift > 0 && _timerPeriod >= 8 && target <= 0x7FF)
        {
            _timerPeriod = target;
        }

        if (_sweepDivider == 0 || _sweepReload)
        {
            _sweepDivider = _sweepPeriod;
            _sweepReload = false;
        }
        else
        {
            _sweepDivider--;
        }
    }

    /// <summary>
    /// The period the sweep unit would load. A negative target does not mute the channel;
    /// only a target above $7FF does (nesdev).
    /// </summary>
    private int SweepTarget()
    {
        int change = _timerPeriod >> _sweepShift;
        if (_sweepNegate)
        {
            change = -change;
            if (_isFirst)
            {
                change--;
            }
        }

        return _timerPeriod + change;
    }

    /// <summary>Back to the power-on state.</summary>
    public void Reset()
    {
        Envelope.Reset();
        Length.Reset();

        _duty = 0;
        _sequencePosition = 0;
        _timerPeriod = 0;
        _timer = 0;
        _sweepEnabled = false;
        _sweepNegate = false;
        _sweepReload = false;
        _sweepPeriod = 0;
        _sweepShift = 0;
        _sweepDivider = 0;
    }

    /// <summary>Appends the channel state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        Envelope.SaveState(writer);
        Length.SaveState(writer);

        writer.WriteInt32(_duty);
        writer.WriteInt32(_sequencePosition);
        writer.WriteInt32(_timerPeriod);
        writer.WriteInt32(_timer);
        writer.WriteBool(_sweepEnabled);
        writer.WriteBool(_sweepNegate);
        writer.WriteBool(_sweepReload);
        writer.WriteInt32(_sweepPeriod);
        writer.WriteInt32(_sweepShift);
        writer.WriteInt32(_sweepDivider);
    }

    /// <summary>Restores the channel state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        Envelope.LoadState(reader);
        Length.LoadState(reader);

        _duty = reader.ReadInt32();
        _sequencePosition = reader.ReadInt32();
        _timerPeriod = reader.ReadInt32();
        _timer = reader.ReadInt32();
        _sweepEnabled = reader.ReadBool();
        _sweepNegate = reader.ReadBool();
        _sweepReload = reader.ReadBool();
        _sweepPeriod = reader.ReadInt32();
        _sweepShift = reader.ReadInt32();
        _sweepDivider = reader.ReadInt32();
    }
}
