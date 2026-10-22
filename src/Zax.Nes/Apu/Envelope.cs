using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// The volume envelope of a pulse channel or of the noise channel (nesdev, "APU Envelope").
/// </summary>
/// <remarks>
/// The loop flag and the length counter's halt flag are the same bit of the same register;
/// they are kept separately here because they drive two different units.
/// </remarks>
internal sealed class Envelope
{
    private bool _start;
    private byte _divider;
    private byte _decay;

    /// <summary>Divider period, and the volume itself in constant-volume mode (0 to 15).</summary>
    public byte Period { get; set; }

    /// <summary>Constant volume flag: bit 4 of $4000 / $4004 / $400C.</summary>
    public bool ConstantVolume { get; set; }

    /// <summary>Loop flag: bit 5 of the same register.</summary>
    public bool Loop { get; set; }

    /// <summary>Volume the channel is driven at, 0 to 15.</summary>
    public byte Volume => ConstantVolume ? Period : _decay;

    /// <summary>Sets the start flag, which a write to the length register does.</summary>
    public void Restart() => _start = true;

    /// <summary>Clocks the envelope on a quarter-frame signal.</summary>
    public void Clock()
    {
        if (_start)
        {
            _start = false;
            _decay = 15;
            _divider = Period;
            return;
        }

        if (_divider == 0)
        {
            _divider = Period;

            if (_decay > 0)
            {
                _decay--;
            }
            else if (Loop)
            {
                _decay = 15;
            }
        }
        else
        {
            _divider--;
        }
    }

    /// <summary>Back to the power-on state.</summary>
    public void Reset()
    {
        Period = 0;
        ConstantVolume = false;
        Loop = false;
        _start = false;
        _divider = 0;
        _decay = 0;
    }

    /// <summary>Appends the envelope state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        writer.WriteByte(Period);
        writer.WriteBool(ConstantVolume);
        writer.WriteBool(Loop);
        writer.WriteBool(_start);
        writer.WriteByte(_divider);
        writer.WriteByte(_decay);
    }

    /// <summary>Restores the envelope state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        Period = reader.ReadByte();
        ConstantVolume = reader.ReadBool();
        Loop = reader.ReadBool();
        _start = reader.ReadBool();
        _divider = reader.ReadByte();
        _decay = reader.ReadByte();
    }
}
