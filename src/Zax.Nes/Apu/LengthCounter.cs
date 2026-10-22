using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// The length counter the four tone channels share (nesdev, "APU Length Counter").
/// </summary>
/// <remarks>
/// <para>
/// Two write-ordering quirks are visible to the CPU and are measured by blargg's frame
/// counter suite (REQ-CORE-113), so they are modelled here rather than approximated.
/// </para>
/// <list type="bullet">
/// <item>
/// <strong>Halt.</strong> A write to the halt flag on the very cycle the counter is clocked
/// takes effect <em>after</em> the clock (<c>10.len_halt_timing</c>). Nothing is needed for
/// that: <see cref="NesBus"/> steps the APU for a cycle before handing the register write
/// over, so a write always lands after that cycle's clock.
/// </item>
/// <item>
/// <strong>Reload.</strong> A reload written on the clock cycle is ignored outright when the
/// counter was non-zero before the clock (<c>11.len_reload_timing</c>). That needs the
/// pre-clock value and the cycle the clock happened on, which is what
/// <see cref="Clock(long)"/> records.
/// </item>
/// </list>
/// </remarks>
internal sealed class LengthCounter
{
    /// <summary>Reload table, indexed by the top five bits of $4003 / $4007 / $400B / $400F.</summary>
    internal static readonly byte[] Table =
    [
        10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
        12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
    ];

    private long _clockedAtCycle = -1;
    private byte _valueBeforeClock;

    /// <summary>Current count; the channel is silent once it reaches 0.</summary>
    public byte Value { get; private set; }

    /// <summary>Halt flag: bit 5 of $4000 / $4004 / $400C, bit 7 of $4008.</summary>
    public bool Halt { get; set; }

    /// <summary>Whether $4015 enables the channel.</summary>
    public bool Enabled { get; private set; }

    /// <summary>Whether the counter is running, which is what $4015 reports.</summary>
    public bool Active => Value > 0;

    /// <summary>Enables or disables the channel; disabling clears the counter at once.</summary>
    public void SetEnabled(bool enabled)
    {
        Enabled = enabled;
        if (!enabled)
        {
            Value = 0;
        }
    }

    /// <summary>Counts down on a half-frame signal, at APU cycle <paramref name="cycle"/>.</summary>
    public void Clock(long cycle)
    {
        _clockedAtCycle = cycle;
        _valueBeforeClock = Value;

        if (!Halt && Value > 0)
        {
            Value--;
        }
    }

    /// <summary>
    /// Reloads from <paramref name="register"/> (the length register the game wrote), at
    /// cycle <paramref name="cycle"/>.
    /// </summary>
    public void Load(byte register, long cycle)
    {
        if (!Enabled)
        {
            // "When disabled via $4015, length shouldn't allow reloading" (apu_test 1-len_ctr).
            return;
        }

        if (_clockedAtCycle == cycle && _valueBeforeClock != 0)
        {
            // Reload written during a length clock, counter non-zero before it: ignored.
            return;
        }

        Value = Table[register >> 3];
    }

    /// <summary>Back to the power-on state.</summary>
    public void Reset()
    {
        Value = 0;
        Halt = false;
        Enabled = false;
        _clockedAtCycle = -1;
        _valueBeforeClock = 0;
    }

    /// <summary>Appends the counter state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        writer.WriteByte(Value);
        writer.WriteBool(Halt);
        writer.WriteBool(Enabled);
        writer.WriteInt64(_clockedAtCycle);
        writer.WriteByte(_valueBeforeClock);
    }

    /// <summary>Restores the counter state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        Value = reader.ReadByte();
        Halt = reader.ReadBool();
        Enabled = reader.ReadBool();
        _clockedAtCycle = reader.ReadInt64();
        _valueBeforeClock = reader.ReadByte();
    }
}
