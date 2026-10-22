namespace Zax.App;

/// <summary>
/// Exponentially smoothed frame time, so that the displayed FPS does not jitter.
/// </summary>
internal sealed class FrameRateCounter
{
    private const double Smoothing = 0.1;

    private double _smoothedSeconds;

    /// <summary>Records one frame duration.</summary>
    /// <param name="deltaSeconds">Duration of the frame that just finished, in seconds.</param>
    public void Add(double deltaSeconds)
    {
        if (deltaSeconds <= 0.0 || double.IsNaN(deltaSeconds))
        {
            return;
        }

        _smoothedSeconds = _smoothedSeconds <= 0.0
            ? deltaSeconds
            : _smoothedSeconds + ((deltaSeconds - _smoothedSeconds) * Smoothing);
    }

    /// <summary>Smoothed frames per second, or zero before the first frame.</summary>
    public double FramesPerSecond => _smoothedSeconds > 0.0 ? 1.0 / _smoothedSeconds : 0.0;

    /// <summary>Smoothed frame duration in milliseconds.</summary>
    public double FrameMilliseconds => _smoothedSeconds * 1000.0;
}
