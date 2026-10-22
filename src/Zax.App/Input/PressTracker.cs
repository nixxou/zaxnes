namespace Zax.App.Input;

/// <summary>What a press became (SPEC-08 §2).</summary>
public enum PressKind
{
    /// <summary>Nothing happened.</summary>
    None,

    /// <summary>Released in less than 0.3 s.</summary>
    Short,

    /// <summary>Held for 0.3 s or more.</summary>
    Long,
}

/// <summary>
/// Measures the press duration of one action so that the short / long press of SPEC-08 §2 can be
/// told apart (0.3 s).
/// </summary>
public sealed class PressTracker
{
    /// <summary>Boundary between a short and a long press, 0.3 s (SPEC-08 §2).</summary>
    public const float LongPressSeconds = 0.3f;

    private float _held;
    private bool _down;
    private bool _consumed;

    /// <summary>True while the action is held.</summary>
    public bool IsDown => _down;

    /// <summary>How long the action has been held, in seconds.</summary>
    public float HeldSeconds => _held;

    /// <summary>
    /// Feeds the current state of the action and returns what the press became, if anything.
    /// A long press is reported as soon as 0.3 s have elapsed, so that the view reset of
    /// REQ-INP-101 happens while the key is still down; a short press is reported on release.
    /// </summary>
    public PressKind Update(bool down, float realSeconds)
    {
        if (down)
        {
            _held = _down ? _held + realSeconds : 0f;
            _down = true;
            if (!_consumed && _held >= LongPressSeconds)
            {
                _consumed = true;
                return PressKind.Long;
            }

            return PressKind.None;
        }

        if (!_down)
        {
            return PressKind.None;
        }

        _down = false;
        bool wasConsumed = _consumed;
        _consumed = false;
        return wasConsumed ? PressKind.None : PressKind.Short;
    }

    /// <summary>Forgets the press in progress, without reporting anything.</summary>
    public void Reset()
    {
        _held = 0f;
        _down = false;
        _consumed = false;
    }
}

/// <summary>
/// A rising / falling edge detector on the display clock, which is what the hotkeys and the menu
/// navigation of SPEC-08 §3.11 are evaluated on (REQ-INP-100).
/// </summary>
public struct EdgeDetector
{
    private bool _previous;

    /// <summary>Whether the input was held at the previous sample.</summary>
    public bool WasDown => _previous;

    /// <summary>Feeds the current state and returns whether this is a rising edge.</summary>
    public bool Rising(bool current)
    {
        bool rising = current && !_previous;
        _previous = current;
        return rising;
    }

    /// <summary>Feeds the current state and returns whether this is a falling edge.</summary>
    public bool Falling(bool current)
    {
        bool falling = !current && _previous;
        _previous = current;
        return falling;
    }

    /// <summary>Forgets the previous sample, so the next press is a rising edge again.</summary>
    public void Reset() => _previous = false;
}
