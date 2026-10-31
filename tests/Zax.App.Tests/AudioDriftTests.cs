using Zax.App.Audio;
using Zax.Nes.Apu;

namespace Zax.App.Tests;

/// <summary>
/// Dynamic rate control of the core's output buffer (SPEC-11 REQ-AUD-004, REQ-AUD-005).
/// </summary>
/// <remarks>
/// <para>
/// The emulation runs on its own clock and the sound card on another. These tests drive
/// <see cref="AudioDriftControl"/> with a headless model of that pair: a producer that runs
/// one emulated frame every 1/60 s and enqueues <c>800 × (1 + adjustment)</c> samples, and a
/// consumer that takes 512-sample blocks at the device's own rate, which is detuned by the
/// drift under test. A block the buffer cannot fill entirely is a block of silence
/// (REQ-AUD-003), which is what a crackle sounds like, so the measurement is simply how many
/// of those happen over a minute.
/// </para>
/// <para>
/// The run is 60 s of simulated time with the first 10 s left out of the count: the core's
/// buffer starts empty (REQ-AUD-010) and a ±0.5 % correction needs a few seconds to lift it
/// to the target, which REQ-AUD-004 covers by asking for no recurring underrun
/// <em>in steady state</em>.
/// </para>
/// </remarks>
public sealed class AudioDriftTests
{
    private const int Rate = 48_000;
    private const double Fps = 60.0;
    private const double RunSeconds = 60.0;
    private const double WarmUpSeconds = 10.0;

    /// <summary>What a simulated minute of playback produced.</summary>
    /// <param name="SilentBlocks">Blocks the buffer could not fill, after the warm-up.</param>
    /// <param name="MinLatencyMs">Smallest smoothed latency seen, in milliseconds.</param>
    /// <param name="MaxLatencyMs">Largest smoothed latency seen, in milliseconds.</param>
    /// <param name="Dropped">Samples the core had to drop because its buffer was full.</param>
    private sealed record Run(int SilentBlocks, double MinLatencyMs, double MaxLatencyMs, int Dropped);

    /// <summary>
    /// REQ-AUD-004, REQ-AUD-005 — with the device clock 0.3 % fast or 0.3 % slow, the
    /// correction holds the buffer and the output never breaks up: no block of silence at all
    /// in steady state, where REQ-AUD-004 tolerates one a minute.
    /// </summary>
    [Theory]
    [InlineData(0.0)]
    [InlineData(0.003)]
    [InlineData(-0.003)]
    [InlineData(0.005)]
    [InlineData(-0.005)]
    public void REQ_AUD_005_a_clock_drift_of_up_to_half_a_percent_causes_no_underrun(double drift)
    {
        Run run = Simulate(drift);

        Assert.Equal(0, run.SilentBlocks);
        Assert.Equal(0, run.Dropped);

        // REQ-AUD-005: the latency stays put instead of creeping towards one of the two ends.
        Assert.True(
            run.MaxLatencyMs - run.MinLatencyMs < 20.0,
            $"latency swing of {run.MaxLatencyMs - run.MinLatencyMs:F1} ms.");

        // REQ-AUD-003: the core's buffer plus one block stays inside the 60 ms required —
        // about 40 ms without drift and 49 ms at the far end of the correction's range.
        double block = 1000.0 * AudioMixer.BlockFrames / Rate;
        Assert.True(run.MaxLatencyMs + block < 60.0, $"{run.MaxLatencyMs + block:F1} ms of latency.");
    }

    /// <summary>
    /// REQ-AUD-005 — the whole ±0.5 % the requirement allows is reachable. A law normalising
    /// the error by the target alone cannot: with a buffer that can only be 1 024 samples
    /// <em>below</em> a target of 1 024, its relative error saturates at −1, so a gain chosen
    /// to be gentle near the target caps the speed-up well under the 0.3 % a sound card can
    /// easily run fast, and the buffer then sits empty for good.
    /// </summary>
    [Fact]
    public void REQ_AUD_005_the_correction_reaches_half_a_percent_in_both_directions()
    {
        Assert.Equal(0.005, AudioDriftControl.MaxAdjustment, 1e-12);
        Assert.Equal(0.005, Apu2A03.MaxRatioAdjustment, 1e-12);

        var control = new AudioDriftControl(Rate);

        // An empty buffer asks for the largest speed-up there is.
        for (int tick = 0; tick < 120; tick++)
        {
            control.Update(0, 1.0 / Fps);
        }

        Assert.Equal(AudioDriftControl.MaxAdjustment, control.Adjustment, 1e-12);

        // A buffer well past the target asks for the largest slow-down.
        control.Reset();
        for (int tick = 0; tick < 120; tick++)
        {
            control.Update(8 * AudioMixer.BlockFrames, 1.0 / Fps);
        }

        Assert.Equal(-AudioDriftControl.MaxAdjustment, control.Adjustment, 1e-12);

        // And nothing beyond it, which is what the core would refuse anyway.
        Assert.InRange(control.Adjustment, -Apu2A03.MaxRatioAdjustment, Apu2A03.MaxRatioAdjustment);
    }

    /// <summary>
    /// REQ-AUD-005 — a buffer sitting at the target asks for no correction at all, so a device
    /// whose clock is exact is left at the pitch of REQ-CORE-070.
    /// </summary>
    [Fact]
    public void REQ_AUD_005_a_buffer_at_the_target_is_left_alone()
    {
        var control = new AudioDriftControl(Rate);

        for (int tick = 0; tick < 600; tick++)
        {
            control.Update(AudioDriftControl.TargetSamples, 1.0 / Fps);
        }

        Assert.Equal(0.0, control.Adjustment, 1e-12);
        Assert.Equal(AudioDriftControl.TargetSamples, control.Level, 1e-6);
    }

    /// <summary>
    /// REQ-AUD-005 — the smoothing keeps the sawtooth of the two granularities out of the
    /// ratio: over a steady minute the pitch wobble stays far under the nine cents the cap
    /// allows, so the correction is not audible as vibrato.
    /// </summary>
    [Fact]
    public void REQ_AUD_005_the_ratio_does_not_wobble_with_the_buffer_sawtooth()
    {
        double swing = AdjustmentSwing(0.003);

        // The cap, 0.5 %, is nine cents; a fifth of that is under two cents, and what the
        // smoothing actually leaves is about 0.06 %, or one cent.
        Assert.True(swing < 0.001, $"ratio swing of {swing * 100.0:F3} %.");
    }

    /// <summary>
    /// Runs the producer/consumer model for <see cref="RunSeconds"/> of simulated time.
    /// </summary>
    /// <param name="deviceDrift">
    /// Relative error of the device clock: positive means the sound card consumes faster than
    /// the nominal rate, which is the direction that empties the buffer.
    /// </param>
    private static Run Simulate(double deviceDrift)
    {
        var control = new AudioDriftControl(Rate);

        const double frameInterval = 1.0 / Fps;
        double samplesPerFrame = Rate / Fps;
        double blockInterval = AudioMixer.BlockFrames / (Rate * (1.0 + deviceDrift));

        int available = 0;
        int dropped = 0;
        int silent = 0;
        double carry = 0.0;
        double minLatency = double.MaxValue;
        double maxLatency = 0.0;

        double nextFrame = 0.0;
        double nextBlock = 0.0;

        while (Math.Min(nextFrame, nextBlock) < RunSeconds)
        {
            if (nextFrame <= nextBlock)
            {
                // One emulated frame: the core enqueues the samples of REQ-CORE-070, nudged
                // by the ratio the host asked for, and the host then takes its measurement.
                carry += samplesPerFrame * (1.0 + control.Adjustment);
                int whole = (int)carry;
                carry -= whole;

                available += whole;
                if (available > Apu2A03.MinBufferSamples)
                {
                    // REQ-CORE-071: the oldest samples go when the buffer is full.
                    dropped += available - Apu2A03.MinBufferSamples;
                    available = Apu2A03.MinBufferSamples;
                }

                control.Update(available, frameInterval);

                if (nextFrame >= WarmUpSeconds)
                {
                    double latency = 1000.0 * control.Level / Rate;
                    minLatency = Math.Min(minLatency, latency);
                    maxLatency = Math.Max(maxLatency, latency);
                }

                nextFrame += frameInterval;
                continue;
            }

            // One device block: all or nothing (REQ-AUD-003).
            if (available >= AudioMixer.BlockFrames)
            {
                available -= AudioMixer.BlockFrames;
            }
            else if (nextBlock >= WarmUpSeconds)
            {
                silent++;
            }

            nextBlock += blockInterval;
        }

        return new Run(silent, minLatency, maxLatency, dropped);
    }

    /// <summary>Peak-to-peak swing of the adjustment over the measured part of a run.</summary>
    private static double AdjustmentSwing(double deviceDrift)
    {
        var control = new AudioDriftControl(Rate);

        const double frameInterval = 1.0 / Fps;
        double samplesPerFrame = Rate / Fps;
        double blockInterval = AudioMixer.BlockFrames / (Rate * (1.0 + deviceDrift));

        int available = 0;
        double carry = 0.0;
        double low = double.MaxValue;
        double high = double.MinValue;
        double nextFrame = 0.0;
        double nextBlock = 0.0;

        while (Math.Min(nextFrame, nextBlock) < RunSeconds)
        {
            if (nextFrame <= nextBlock)
            {
                carry += samplesPerFrame * (1.0 + control.Adjustment);
                int whole = (int)carry;
                carry -= whole;
                available += whole;

                control.Update(available, frameInterval);

                if (nextFrame >= WarmUpSeconds)
                {
                    low = Math.Min(low, control.Adjustment);
                    high = Math.Max(high, control.Adjustment);
                }

                nextFrame += frameInterval;
                continue;
            }

            if (available >= AudioMixer.BlockFrames)
            {
                available -= AudioMixer.BlockFrames;
            }

            nextBlock += blockInterval;
        }

        return high - low;
    }
}
