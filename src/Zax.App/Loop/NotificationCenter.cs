namespace Zax.App.Loop;

/// <summary>Severity of a notification (SPEC-10 REQ-UI-080).</summary>
public enum NotificationSeverity
{
    /// <summary>White text.</summary>
    Normal,

    /// <summary>Yellow text.</summary>
    Warning,

    /// <summary>Red text.</summary>
    Error,
}

/// <summary>One notification: a text, a duration in real seconds and a severity.</summary>
/// <param name="Text">Exact text to show.</param>
/// <param name="Severity">Severity, which decides the colour and the replacement rule.</param>
/// <param name="Seconds">Duration in real seconds, unaffected by pause or game speed.</param>
public readonly record struct Notification(string Text, NotificationSeverity Severity, double Seconds);

/// <summary>
/// The single notification area of SPEC-10 §3.10: one notification at a time, no stacking, and
/// a timer measured in real time (REQ-UI-080, REQ-UI-081).
/// </summary>
/// <remarks>
/// A new notification replaces the visible one — restarting its timer — when nothing is visible
/// or when its severity is at least that of the visible one; otherwise it is dropped. The texts
/// and durations the emulation loop uses are fixed by SPEC-07 §4 and SPEC-10 REQ-UI-082.
/// </remarks>
public sealed class NotificationCenter
{
    /// <summary>Default duration, 4 s (REQ-UI-080).</summary>
    public const double DefaultSeconds = 4.0;

    /// <summary>Duration of the rollback notification (SPEC-07 §4).</summary>
    public const double ShortSeconds = 1.0;

    private readonly List<Notification> _log = [];

    private Notification _current;
    private double _remaining;

    /// <summary>The visible notification, or null when none is.</summary>
    public Notification? Current => _remaining > 0.0 ? _current : null;

    /// <summary>Real seconds left before the visible notification disappears.</summary>
    public double Remaining => _remaining;

    /// <summary>Every notification that was accepted, oldest first. For tests and logs.</summary>
    public IReadOnlyList<Notification> Log => _log;

    /// <summary>
    /// Shows <paramref name="text"/>, applying the replacement rule of REQ-UI-081.
    /// </summary>
    /// <returns>True when the notification was accepted.</returns>
    public bool Show(string text, double seconds = DefaultSeconds, NotificationSeverity severity = NotificationSeverity.Normal)
    {
        ArgumentNullException.ThrowIfNull(text);

        if (Current is { } visible && severity < visible.Severity)
        {
            return false;
        }

        _current = new Notification(text, severity, seconds);
        _remaining = seconds;
        _log.Add(_current);
        return true;
    }

    /// <summary>Shows an error notification of the default duration (REQ-UI-083).</summary>
    public bool ShowError(string text) => Show(text, DefaultSeconds, NotificationSeverity.Error);

    /// <summary>Advances the timer by <paramref name="realSeconds"/> of real time.</summary>
    public void Advance(double realSeconds)
    {
        if (_remaining > 0.0)
        {
            _remaining = Math.Max(0.0, _remaining - realSeconds);
        }
    }

    /// <summary>Hides the visible notification immediately.</summary>
    public void Clear() => _remaining = 0.0;
}
