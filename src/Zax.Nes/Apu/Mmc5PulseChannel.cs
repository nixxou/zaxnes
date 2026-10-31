using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// One of the MMC5's two square channels, $5000-$5003 and $5004-$5007 (REQ-CORE-075).
/// </summary>
/// <remarks>
/// <para>
/// REQ-CORE-075 takes the waveform, the duty, the timer, the envelope, the length counter and
/// the muting of periods below 8 straight from the APU's pulse channels, and drops the sweep
/// unit: the board has none, so nothing here recomputes a sweep target and nothing can mute
/// the channel on account of one. That is the one real difference from
/// <see cref="PulseChannel"/>, and it is why the two are separate classes rather than one with
/// a flag — an APU pulse with its sweep register left at 0 mutes itself above period $3FF,
/// which an MMC5 channel must not do.
/// </para>
/// <para>
/// The envelope and the length counter are <strong>not</strong> clocked from the APU's frame
/// sequencer: the board has no equivalent of it, and <see cref="Mmc5Audio"/> drives them at a
/// fixed cadence instead.
/// </para>
/// <para>
/// The public documentation notes that a period below 8 does <em>not</em> silence an MMC5
/// channel, as it does an APU pulse; REQ-CORE-075 asks for the APU's behaviour here, which is
/// the reference's, and records the divergence.
/// </para>
/// </remarks>
internal sealed class Mmc5PulseChannel
{
    private int _duty;
    private int _sequencePosition;
    private int _timerPeriod;
    private int _timer;

    /// <summary>The envelope driving the channel's volume.</summary>
    public Envelope Envelope { get; } = new();

    /// <summary>The length counter $5015 reports on.</summary>
    public LengthCounter Length { get; } = new();

    /// <summary>Current output, 0 to 15.</summary>
    public int Output =>
        _timerPeriod < 8 || !Length.Active || PulseChannel.DutySequences[(_duty * 8) + _sequencePosition] == 0
            ? 0
            : Envelope.Volume;

    /// <summary>Writes register <paramref name="index"/> (0 to 3) of the channel.</summary>
    /// <param name="index">0 = $5000, 1 = $5001, 2 = $5002, 3 = $5003 (and $5004 upwards).</param>
    /// <param name="value">Byte written.</param>
    /// <param name="cycle">CPU cycle the write lands on, for the length reload quirk.</param>
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
                // The APU's sweep register. The MMC5 has no sweep unit, so the board ignores
                // it (nesdev, "MMC5 audio").
                break;

            case 2:
                _timerPeriod = (_timerPeriod & 0x700) | value;
                break;

            default:
                _timerPeriod = (_timerPeriod & 0x0FF) | ((value & 0x07) << 8);
                Length.Load(value, cycle);

                // As on an APU pulse: the sequencer restarts at the first value of the
                // sequence and the envelope's start flag is set. The timer is not reloaded.
                _sequencePosition = 0;
                Envelope.Restart();
                break;
        }
    }

    /// <summary>Clocks the duty sequencer's timer, once every two CPU cycles as the APU does.</summary>
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

    /// <summary>Back to the power-on state.</summary>
    public void Reset()
    {
        Envelope.Reset();
        Length.Reset();

        _duty = 0;
        _sequencePosition = 0;
        _timerPeriod = 0;
        _timer = 0;
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
    }
}
