using System.Diagnostics;

namespace Zax.App;

/// <summary>
/// The emulated frame clock of SPEC-07 §3.2: frames are due at <c>t0 + n / f</c> on a monotonic
/// clock, late frames are caught up to a limit of five, and the clock can be reset without any
/// catch-up.
/// </summary>
/// <remarks>
/// The nominal rate is 60 frames/s in NTSC and 50 in PAL and Dendy (DEC-R2-T3); the speed factor
/// <c>k</c> of REQ-LOOP-015 multiplies it. The pacing is deliberately host-side only: the kernel
/// exposes "run exactly one emulated frame" and nothing else (REQ-LOOP-062).
/// </remarks>
public sealed class FramePacer
{
    /// <summary>Highest number of emulated frames that may be late (REQ-LOOP-012).</summary>
    public const int MaxBacklog = 5;

    /// <summary>A frame may start up to 1 ms early (REQ-LOOP-011).</summary>
    public const double EarlyStartSeconds = 0.001;

    private readonly Func<double> _clock;
    private double _nextDue;

    /// <summary>Creates a pacer at <paramref name="nominalRate"/> frames per second.</summary>
    /// <param name="nominalRate">Nominal rate of the region, 60 or 50 (DEC-R2-T3).</param>
    /// <param name="clock">
    /// Monotonic clock in seconds. A test passes its own so that the pacing is checked without
    /// waiting; the default is a <see cref="Stopwatch"/>.
    /// </param>
    public FramePacer(double nominalRate, Func<double>? clock = null)
    {
        if (clock is null)
        {
            Stopwatch watch = Stopwatch.StartNew();
            clock = () => watch.Elapsed.TotalSeconds;
        }

        _clock = clock;
        NominalRate = nominalRate;
        Reset();
    }

    /// <summary>Nominal rate of the active region, in emulated frames per second.</summary>
    public double NominalRate { get; set; }

    /// <summary>Speed factor <c>k</c> of REQ-LOOP-015, 1 by default.</summary>
    public double Speed { get; set; } = 1.0;

    /// <summary>The effective rate, <c>nominal × k</c>.</summary>
    public double EffectiveRate => Math.Max(1.0, NominalRate * Speed);

    /// <summary>Elapsed real time in seconds since the pacer was created.</summary>
    public double ElapsedSeconds => _clock();

    /// <summary>Emulated frames the pacer has handed out.</summary>
    public long FrameCount { get; private set; }

    /// <summary>Frames dropped because the backlog went past the limit (REQ-LOOP-012).</summary>
    public long DroppedFrames { get; private set; }

    /// <summary>
    /// Resets the clock, so that nothing is caught up: what a resume, a reset, the end of a
    /// load and a rollback restore do (REQ-LOOP-013).
    /// </summary>
    public void Reset() => _nextDue = _clock();

    /// <summary>
    /// How many emulated frames are due now, at most <see cref="MaxBacklog"/>. Any further
    /// backlog is abandoned, so the game slows down instead of suddenly speeding up.
    /// </summary>
    public int DueFrames()
    {
        double now = _clock();
        double period = 1.0 / EffectiveRate;

        int due = 0;
        while (now + EarlyStartSeconds >= _nextDue)
        {
            _nextDue += period;
            due++;
            if (due < MaxBacklog)
            {
                continue;
            }

            // REQ-LOOP-012: the backlog never exceeds five frames; the rest is dropped.
            while (now + EarlyStartSeconds >= _nextDue)
            {
                _nextDue += period;
                DroppedFrames++;
            }

            break;
        }

        FrameCount += due;
        return due;
    }
}
