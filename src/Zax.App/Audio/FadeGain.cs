namespace Zax.App.Audio;

/// <summary>
/// A gain that moves linearly towards a target, as the fades of SPEC-11 REQ-AUD-020 and
/// REQ-AUD-022 do.
/// </summary>
/// <remarks>
/// <para>
/// A complete fade — from 0 to the full range or back — lasts <see cref="FullFadeSeconds"/> of
/// <b>real</b> time, 200/60 s ≈ 3.33 s, whatever the render rate (DEC-11-Q2, divergence D: the
/// original steps the gain once per rendered frame, so its fade lasts 2.2 s at 90 images/s). A
/// fade under way is replaced by the new one, starting from the current gain and keeping the
/// same slope.
/// </para>
/// <para>
/// When the gain has already reached its previous target, a new target is taken immediately:
/// that is what makes a change of <c>V</c> audible in under 100 ms (REQ-AUD-006) and what
/// REQ-AUD-023 asks for when the menu music is in its active phase, without skipping the
/// fade-in that is still running.
/// </para>
/// </remarks>
public sealed class FadeGain
{
    /// <summary>Duration of a complete fade in real seconds (DEC-11-Q2).</summary>
    public const double FullFadeSeconds = 200.0 / 60.0;

    private double _gain;
    private double _target;
    private double _range;

    /// <summary>Creates a gain already settled at <paramref name="value"/>.</summary>
    public FadeGain(double value = 0.0)
    {
        _gain = value;
        _target = value;
        _range = Math.Max(value, 0.0);
    }

    /// <summary>The current gain.</summary>
    public float Value => (float)_gain;

    /// <summary>The gain the fade is heading to.</summary>
    public double Target => _target;

    /// <summary>Whether the gain has reached its target.</summary>
    public bool IsSettled => Math.Abs(_gain - _target) <= 1e-9;

    /// <summary>
    /// Starts a fade towards <paramref name="target"/>: the gain always travels, from wherever
    /// it is. This is what a state change does (REQ-AUD-020, REQ-AUD-022).
    /// </summary>
    /// <param name="target">New target gain.</param>
    /// <param name="range">
    /// Full range of the fade, which fixes the slope <c>range / (200/60 s)</c>; pass the
    /// loudest gain the source reaches, that is <c>V</c> for the emulation and
    /// <c>0.02 × V</c> for the music.
    /// </param>
    public void FadeTo(double target, double range)
    {
        _target = target;
        _range = Math.Max(range, 0.0);
    }

    /// <summary>
    /// Moves the target without starting a new fade: when the gain had already reached the
    /// previous target it jumps to the new one, which is how a change of <c>V</c> becomes
    /// audible in under 100 ms (REQ-AUD-006); when a fade is under way it simply carries on
    /// towards the new target, so the fade of REQ-AUD-020 is never cut short.
    /// </summary>
    public void Retarget(double target, double range)
    {
        bool settled = IsSettled;
        _target = target;
        _range = Math.Max(range, 0.0);
        if (settled)
        {
            _gain = target;
        }
    }

    /// <summary>Moves the target and the gain at once, with no fade (REQ-AUD-023).</summary>
    public void Snap(double target, double range)
    {
        _target = target;
        _range = Math.Max(range, 0.0);
        _gain = target;
    }

    /// <summary>Advances the fade by <paramref name="realSeconds"/>.</summary>
    public void Advance(double realSeconds)
    {
        if (realSeconds <= 0.0 || IsSettled)
        {
            return;
        }

        double step = _range / FullFadeSeconds * realSeconds;
        if (step <= 0.0)
        {
            _gain = _target;
            return;
        }

        _gain = _gain < _target
            ? Math.Min(_target, _gain + step)
            : Math.Max(_target, _gain - step);
    }
}
