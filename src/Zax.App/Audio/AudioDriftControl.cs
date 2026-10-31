using Zax.Nes.Apu;

namespace Zax.App.Audio;

/// <summary>
/// Dynamic rate control of the core's output buffer (SPEC-11 REQ-AUD-005).
/// </summary>
/// <remarks>
/// <para>
/// The emulation is paced by its own clock (REQ-AUD-004) and the audio device by the sound
/// card's: the two are never exactly equal, and a drift of a fraction of a percent is enough
/// to drain or to overflow the core's buffer within a minute. REQ-AUD-005 lets the frontend
/// correct it by nudging the resampling ratio by at most ±0.5 % — under nine cents of pitch,
/// which is inaudible — and the core takes that nudge through
/// <see cref="Zax.Nes.NesCore.AudioRatioAdjustment"/>.
/// </para>
/// <para>
/// <strong>The law.</strong> The buffer level is low-passed over
/// <see cref="SmoothingSeconds"/> and the adjustment is the rate error that would close the
/// remaining gap to <see cref="TargetSamples"/> in <see cref="WindowSeconds"/>:
/// <c>adjustment = (target − level) / (rate × window)</c>, clamped to ±0.5 %. Written that
/// way the loop crosses over at about one radian per second — slow enough that the smoothed
/// level carries none of the sawtooth the 800-sample-per-frame production and the
/// 512-sample blocks produce, and fast enough that a drift is absorbed in a few seconds.
/// </para>
/// <para>
/// <strong>Why the level is smoothed first.</strong> The raw count swings by about
/// ±650 samples over the sixteen-frame beat between the two granularities, which is more
/// than the whole authority of the correction. Feeding it in raw would swing the ratio from
/// one cap to the other several times a second — a 9-cent vibrato at about 4 Hz, which
/// <em>is</em> audible. A half-second time constant leaves a few tens of samples of ripple,
/// or under two cents.
/// </para>
/// <para>
/// <strong>Why proportional is enough.</strong> A proportional law settles with an offset of
/// <c>drift × rate × window</c> samples — 144 samples for a drift of 0,3 % at 48 kHz — so the
/// level stays within a quarter of a block of the target over the whole ±0.5 % range the cap
/// allows, and well above the one block a read needs (REQ-AUD-003). An integral term would
/// remove that offset at the price of a second-order loop that has to be damped; it buys
/// nothing here.
/// </para>
/// </remarks>
public sealed class AudioDriftControl
{
    /// <summary>Buffer level the control holds, in mono samples: three output blocks.</summary>
    /// <remarks>
    /// About 32 ms at 48 kHz, which with the block on top is inside the 60 ms REQ-AUD-003
    /// requires and near the 40 ms it asks for. It cannot usefully be lowered: a frame
    /// enqueues its 800 samples in one go while the device takes 512 at a time, so the count
    /// sweeps about 1 280 samples over the sixteen-frame beat between the two granularities,
    /// and a target under about 1 300 puts the bottom of that sweep below the block a read
    /// needs — the buffer then rides its own floor, where every few seconds a block comes out
    /// as silence.
    /// </remarks>
    public const int TargetSamples = 3 * AudioMixer.BlockFrames;

    /// <summary>Largest nudge of the resampling ratio, ±0.5 % (REQ-AUD-005).</summary>
    public const double MaxAdjustment = Apu2A03.MaxRatioAdjustment;

    /// <summary>Time constant of the low-pass on the measured level, in seconds.</summary>
    public const double SmoothingSeconds = 0.5;

    /// <summary>Time the correction aims to close the gap to the target in, in seconds.</summary>
    public const double WindowSeconds = 1.0;

    /// <summary>Longest step the loop integrates in one call, in seconds.</summary>
    private const double MaxStepSeconds = 0.1;

    private readonly double _gain;

    private double _level = TargetSamples;

    /// <summary>Creates a control for an output running at <paramref name="sampleRate"/> Hz.</summary>
    /// <param name="sampleRate">Effective output rate (REQ-AUD-001).</param>
    public AudioDriftControl(int sampleRate)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(sampleRate, 1);

        SampleRate = sampleRate;
        _gain = 1.0 / (sampleRate * WindowSeconds);
    }

    /// <summary>Output rate the gain is scaled to.</summary>
    public int SampleRate { get; }

    /// <summary>The nudge to hand the core, in [−0.005 ; 0.005].</summary>
    public double Adjustment { get; private set; }

    /// <summary>The smoothed buffer level the law acts on, in mono samples.</summary>
    public double Level => _level;

    /// <summary>
    /// Back to a settled loop: the level is assumed to be at the target and no correction is
    /// applied. Called whenever the core's buffer is emptied or the emulation stops, so that
    /// the measurement taken across the gap is not fed to the loop.
    /// </summary>
    public void Reset()
    {
        _level = TargetSamples;
        Adjustment = 0.0;
    }

    /// <summary>
    /// Folds one measurement in and returns the new adjustment.
    /// </summary>
    /// <param name="availableSamples">Mono samples waiting in the core's buffer.</param>
    /// <param name="realSeconds">Real time since the previous call.</param>
    public double Update(int availableSamples, double realSeconds)
    {
        if (realSeconds <= 0.0)
        {
            return Adjustment;
        }

        // A long step — a stall, a load, a breakpoint — is one measurement, not a licence to
        // jump the filter straight onto a stale count.
        double step = Math.Min(realSeconds, MaxStepSeconds);

        _level += (availableSamples - _level) * (1.0 - Math.Exp(-step / SmoothingSeconds));

        Adjustment = Math.Clamp((TargetSamples - _level) * _gain, -MaxAdjustment, MaxAdjustment);
        return Adjustment;
    }
}
