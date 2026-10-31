using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// The MMC5's expansion audio: two square channels, a PCM channel and the $5010 / $5015
/// registers (REQ-CORE-075).
/// </summary>
/// <remarks>
/// <para>
/// Written from the public nesdev page "MMC5 audio" and from REQ-CORE-075, which settles the
/// points the page leaves open or where the reference differs.
/// </para>
/// <para>
/// <strong>No frame sequencer.</strong> The board has no equivalent of the APU's frame
/// counter: the envelopes of its two squares are clocked at a fixed cadence of about 240 Hz,
/// every <see cref="EnvelopeCycles"/> CPU cycles, and their length counters every other time,
/// so at about 120 Hz. Neither follows $4017, which is the difference a game would hear if the
/// APU's sequencer were reused — hence the dedicated divider below. The public documentation
/// gives the length counters 240 Hz as well; REQ-CORE-075 asks for 120 Hz, which is the
/// reference's, and records the divergence.
/// </para>
/// <para>
/// <strong>PCM channel.</strong> Write mode only. A non-zero write to $5011 sets the level; a
/// write of 0 leaves the level alone and raises the PCM IRQ flag instead. Reading $5010 returns
/// that flag in bit 7 and clears it. The flag never pulls the CPU's /IRQ line — REQ-CORE-075
/// records that as a divergence too, and it is what keeps a game that enables the IRQ from
/// looping in a handler the spec gives no other way out of. The read mode of $5010 (bit 0 set),
/// where reads of $8000-$BFFF feed the converter, is not emulated: the level simply stays at
/// the last value written.
/// </para>
/// <para>
/// <strong>Output.</strong> <c>M = r(Km · P(q1 + q2)) + 31 · pcm</c>, which
/// <see cref="Apu2A03"/> adds to its own <c>A</c> before clipping the sum to 32767
/// (REQ-CORE-073).
/// </para>
/// </remarks>
internal sealed class Mmc5Audio
{
    /// <summary>
    /// CPU cycles between two envelope clocks: 1 789 773 / 7 458 is 239.98 Hz, the "about
    /// 240 Hz" of REQ-CORE-075.
    /// </summary>
    public const int EnvelopeCycles = 7458;

    private readonly Mmc5PulseChannel _pulse1 = new();
    private readonly Mmc5PulseChannel _pulse2 = new();

    private long _cycles;
    private bool _halfRate;
    private int _divider;

    /// <summary>
    /// Whether the next envelope clock also clocks the length counters. It starts
    /// <see langword="true"/> so that the first one does <strong>not</strong>: the counters
    /// then fall on the even envelope clocks, every 2 × <see cref="EnvelopeCycles"/> cycles
    /// from power-on, which is the phase the APU's own half-frame signal has relative to its
    /// quarter-frame signal.
    /// </summary>
    private bool _lengthTurn = true;

    private byte _pcm;
    private bool _readMode;
    private bool _irqEnabled;
    private bool _irqFlag;

    /// <summary>
    /// <c>M</c> of REQ-CORE-073: the board's output in the mixer's integer units
    /// (REQ-CORE-075).
    /// </summary>
    public int Output
        => ApuMixer.Mmc5PulseTable[_pulse1.Output + _pulse2.Output] + (ApuMixer.Mmc5PcmStep * _pcm);

    /// <summary>Back to the power-on state, silent (REQ-CORE-004).</summary>
    public void Reset()
    {
        _pulse1.Reset();
        _pulse2.Reset();

        _cycles = 0;
        _halfRate = false;
        _divider = 0;
        _lengthTurn = true;

        _pcm = 0;
        _readMode = false;
        _irqEnabled = false;
        _irqFlag = false;
    }

    /// <summary>Advances the board's sound by one CPU cycle.</summary>
    public void StepCpuCycle()
    {
        _cycles++;

        _halfRate = !_halfRate;
        if (_halfRate)
        {
            // The square timers run at half the CPU clock, as the APU's pulses do.
            _pulse1.ClockTimer();
            _pulse2.ClockTimer();
        }

        if (++_divider < EnvelopeCycles)
        {
            return;
        }

        _divider = 0;
        _pulse1.Envelope.Clock();
        _pulse2.Envelope.Clock();

        _lengthTurn = !_lengthTurn;
        if (_lengthTurn)
        {
            _pulse1.Length.Clock(_cycles);
            _pulse2.Length.Clock(_cycles);
        }
    }

    /// <summary>Writes one of the board's sound registers, $5000 to $5015.</summary>
    public void Write(ushort address, byte value)
    {
        switch (address)
        {
            case >= 0x5000 and <= 0x5003:
                _pulse1.Write(address - 0x5000, value, _cycles);
                break;

            case >= 0x5004 and <= 0x5007:
                _pulse2.Write(address - 0x5004, value, _cycles);
                break;

            case 0x5010:
                _readMode = (value & 0x01) != 0;
                _irqEnabled = (value & 0x80) != 0;
                break;

            case 0x5011:
                // A write of 0 does not change the level; it raises the PCM IRQ flag
                // (REQ-CORE-075). Writes keep working in read mode, which is not emulated.
                if (value != 0)
                {
                    _pcm = value;
                }
                else
                {
                    _irqFlag = true;
                }

                break;

            case 0x5015:
                _pulse1.Length.SetEnabled((value & 0x01) != 0);
                _pulse2.Length.SetEnabled((value & 0x02) != 0);
                break;

            default:
                break;
        }
    }

    /// <summary>
    /// Reads one of the board's two readable sound registers.
    /// </summary>
    /// <param name="address">CPU address, $5010 or $5015 for anything to be returned.</param>
    /// <param name="value">Byte read.</param>
    /// <param name="acknowledge">
    /// Whether a read of $5010 clears the PCM IRQ flag, as a CPU read does. The host and Lua
    /// accessors pass <see langword="false"/>: REQ-CORE-081 wants them free of side effects,
    /// so peeking at $5010 must not acknowledge the flag under the game's feet.
    /// </param>
    /// <returns><see langword="false"/> when the board drives nothing at this address.</returns>
    public bool TryRead(ushort address, out byte value, bool acknowledge)
    {
        switch (address)
        {
            case 0x5010:
                value = _irqFlag ? (byte)0x80 : (byte)0x00;
                if (acknowledge)
                {
                    _irqFlag = false;
                }

                return true;

            case 0x5015:
                value = (byte)((_pulse1.Length.Active ? 0x01 : 0x00) | (_pulse2.Length.Active ? 0x02 : 0x00));
                return true;

            default:
                value = 0;
                return false;
        }
    }

    /// <summary>Appends the sound state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        _pulse1.SaveState(writer);
        _pulse2.SaveState(writer);

        writer.WriteInt64(_cycles);
        writer.WriteBool(_halfRate);
        writer.WriteInt32(_divider);
        writer.WriteBool(_lengthTurn);
        writer.WriteByte(_pcm);
        writer.WriteBool(_readMode);
        writer.WriteBool(_irqEnabled);
        writer.WriteBool(_irqFlag);
    }

    /// <summary>Restores the sound state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        _pulse1.LoadState(reader);
        _pulse2.LoadState(reader);

        _cycles = reader.ReadInt64();
        _halfRate = reader.ReadBool();
        _divider = reader.ReadInt32();
        _lengthTurn = reader.ReadBool();
        _pcm = reader.ReadByte();
        _readMode = reader.ReadBool();
        _irqEnabled = reader.ReadBool();
        _irqFlag = reader.ReadBool();
    }
}
