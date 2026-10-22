using System.Globalization;
using System.Numerics;

namespace Zax.Nes.Tests;

/// <summary>
/// The output path: the linear mix of REQ-CORE-073, the DC blocker, and the band-limited
/// resampling of REQ-CORE-070.
/// </summary>
public sealed class ApuOutputTests
{
    private static NesCore Core(int sampleRate = 48_000)
    {
        var core = new NesCore();
        Assert.True(core.LoadRom(TestRoms.Minimal(), "Audio.nes").Success);
        core.SampleRate = sampleRate;
        return core;
    }

    /// <summary>
    /// Starts pulse 1 at constant volume 15, 50 % duty, with a sweep setting that never mutes,
    /// and a timer period of <paramref name="period"/>. The length counter is halted, so the
    /// tone runs for as long as the test needs it and the captured signal is stationary.
    /// </summary>
    private static void StartPulse1(NesCore core, int period)
    {
        core.WriteMemory(0x4015, 0x01);
        core.WriteMemory(0x4000, 0xBF); // Duty 2 (50 %), length halted, constant volume 15.
        core.WriteMemory(0x4001, 0x08); // Negate with shift 0: the target never exceeds $7FF.
        core.WriteMemory(0x4002, (byte)period);
        core.WriteMemory(0x4003, (byte)(0xF8 | ((period >> 8) & 0x07)));
        core.WriteMemory(0x4017, 0x40); // Four-step, IRQ inhibited: nothing else perturbs it.
    }

    /// <summary>
    /// Frequency a pulse of timer period <paramref name="period"/> actually comes out at.
    /// </summary>
    /// <remarks>
    /// The duty sequencer takes <c>16 x (period + 1)</c> CPU cycles to go round, but the
    /// resampler is paced on the host's nominal cadence rather than on the hardware's exact
    /// frame rate (REQ-CORE-070), so one output second holds
    /// <c>nominal fps x cycles per frame</c> CPU cycles, not <c>CpuClockHz</c> of them. The
    /// difference is only 0.16 %, which is the whole point of that requirement, but over an
    /// 8192-sample window it is three cycles of phase at 10 kHz — enough to hide a tone from
    /// a transform that looks for it at the wrong frequency.
    /// </remarks>
    private static double PulseFrequency(NesCore core, int period)
    {
        double cyclesPerFrame =
            (double)RegionTiming.DotsPerScanline * core.Timing.ScanlinesPerFrame
            * core.Timing.DotsPerCycleDenominator / core.Timing.DotsPerCycleNumerator;

        return core.Timing.NominalFramesPerSecond * cyclesPerFrame / (16.0 * (period + 1));
    }

    /// <summary>Runs the APU and returns <paramref name="count"/> output samples.</summary>
    private static float[] Capture(NesCore core, int count, int skip = 0)
    {
        var samples = new List<float>(count + skip);
        var block = new float[64];

        while (samples.Count < count + skip)
        {
            core.Bus.Tick();
            if (core.Apu.AvailableSamples >= block.Length)
            {
                core.ReadAudio(block);
                samples.AddRange(block);
            }
        }

        return [.. samples.Skip(skip).Take(count)];
    }

    // ---------------------------------------------------------------- REQ-CORE-073

    [Theory]
    // The linear approximation of nesdev's "APU Mixer", scaled by 50 000 / 32 768
    // (REQ-CORE-073). One channel at a time, so each step value is read on its own.
    [InlineData(0x4000, 0x9F, 0x01, 15 * 0.00752)] // Pulse 1 at volume 15.
    [InlineData(0x4004, 0x9F, 0x02, 15 * 0.00752)] // Pulse 2 at volume 15.
    [InlineData(0x400C, 0x1F, 0x08, 15 * 0.00494)] // Noise at volume 15.
    public void REQ_CORE_073_each_channel_contributes_its_documented_step(
        ushort control, byte value, byte enable, double expectedMix)
    {
        NesCore core = Core();

        core.WriteMemory(0x4015, enable);
        core.WriteMemory(control, value);
        core.WriteMemory((ushort)(control + 1), 0x08);
        core.WriteMemory((ushort)(control + 2), 100);
        core.WriteMemory((ushort)(control + 3), 0xF8);

        double peak = double.NegativeInfinity;
        for (int i = 0; i < 4000; i++)
        {
            core.Bus.Tick();
            peak = Math.Max(peak, core.Apu.MixedLevel);
        }

        Assert.InRange(peak, (1.526 * expectedMix) - 1e-9, (1.526 * expectedMix) + 1e-9);
    }

    [Fact]
    public void REQ_CORE_073_the_triangle_and_the_dmc_contribute_their_documented_steps()
    {
        NesCore core = Core();

        // Triangle on its own, at the top of the staircase.
        core.WriteMemory(0x4015, 0x04);
        core.WriteMemory(0x4008, 0x7F);
        core.WriteMemory(0x400A, 50);
        core.WriteMemory(0x400B, 0xF8);
        core.WriteMemory(0x4017, 0x80);

        double peak = double.NegativeInfinity;
        for (int i = 0; i < 40 * 51; i++)
        {
            core.Bus.Tick();
            peak = Math.Max(peak, core.Apu.MixedLevel);
        }

        Assert.InRange(peak, (1.526 * 15 * 0.00851) - 1e-9, (1.526 * 15 * 0.00851) + 1e-9);

        // DMC on its own, level loaded straight through $4011.
        NesCore dmc = Core();
        dmc.WriteMemory(0x4011, 127);
        dmc.Bus.Tick();

        Assert.InRange(dmc.Apu.MixedLevel, (1.526 * 127 * 0.00335) - 1e-9, (1.526 * 127 * 0.00335) + 1e-9);
    }

    [Fact]
    public void REQ_CORE_073_the_mix_is_the_sum_of_the_channels_and_is_clamped_to_one()
    {
        NesCore core = Core();

        // Both pulses at volume 15 plus a DMC level of 127: 0.2256 + 0.42545, under the clamp.
        core.WriteMemory(0x4015, 0x03);
        core.WriteMemory(0x4000, 0x9F);
        core.WriteMemory(0x4001, 0x08);
        core.WriteMemory(0x4002, 100);
        core.WriteMemory(0x4003, 0xF8);
        core.WriteMemory(0x4004, 0x9F);
        core.WriteMemory(0x4005, 0x08);
        core.WriteMemory(0x4006, 100);
        core.WriteMemory(0x4007, 0xF8);
        core.WriteMemory(0x4011, 127);

        double peak = double.NegativeInfinity;
        for (int i = 0; i < 4000; i++)
        {
            core.Bus.Tick();
            peak = Math.Max(peak, core.Apu.MixedLevel);
        }

        double expected = 1.526 * ((2 * 15 * 0.00752) + (127 * 0.00335));
        Assert.InRange(peak, expected - 1e-9, expected + 1e-9);
        Assert.True(expected < 1.0);

        // Adding the triangle at the top of its staircase pushes the sum past full scale,
        // where REQ-CORE-073 asks for a clamp rather than a wrap.
        core.WriteMemory(0x4015, 0x07);
        core.WriteMemory(0x4008, 0x7F);
        core.WriteMemory(0x400A, 50);
        core.WriteMemory(0x400B, 0xF8);
        core.WriteMemory(0x4017, 0x80);

        peak = double.NegativeInfinity;
        for (int i = 0; i < 40 * 51; i++)
        {
            core.Bus.Tick();
            peak = Math.Max(peak, core.Apu.MixedLevel);
        }

        Assert.Equal(1.0, peak);
    }

    [Fact]
    public void REQ_CORE_073_a_lone_pulse_at_volume_fifteen_swings_by_zero_point_one_seven_two()
    {
        NesCore core = Core();
        StartPulse1(core, period: 111); // About 1 kHz: 24 output samples per half period.

        // The first samples carry the DC blocker's start-up transient.
        float[] samples = Capture(core, 8192, skip: 8192);

        // REQ-CORE-073 asks for the swing "after stabilisation": the two settled plateaus,
        // not the overshoot any band-limited edge rings with. The median of the samples on
        // each side of the mean is the plateau, and is immune to that ringing.
        double mean = samples.Average();
        double high = Median([.. samples.Where(sample => sample > mean)]);
        double low = Median([.. samples.Where(sample => sample < mean)]);

        Assert.InRange(high - low, 0.172 - 0.02, 0.172 + 0.02);
    }

    private static double Median(float[] values)
    {
        Array.Sort(values);
        return values[values.Length / 2];
    }

    [Fact]
    public void REQ_CORE_073_a_constant_level_decays_to_zero_in_less_than_two_hundred_milliseconds()
    {
        NesCore core = Core();

        // A DMC level written through $4011 is a pure step: every channel then sits still.
        core.WriteMemory(0x4011, 100);

        float[] samples = Capture(core, 48_000 / 5); // 200 ms at 48 kHz.

        // The step itself is in there; what matters is where it has got to by the end.
        Assert.True(Math.Abs(samples[32]) > 0.1, "the step should be audible to begin with");
        Assert.InRange(samples[^1], -0.001f, 0.001f);
    }

    [Fact]
    public void REQ_CORE_004_a_silent_apu_produces_digital_silence()
    {
        NesCore core = Core();

        float[] samples = Capture(core, 2048);

        Assert.All(samples, sample => Assert.Equal(0.0f, sample));
    }

    // ---------------------------------------------------------------- REQ-CORE-070

    [Theory]
    // Pulse timer periods whose fundamental sits between 4 and 11 kHz, so that every ROM of
    // odd harmonics lands above the output's Nyquist frequency and would fold back into the
    // audible band if the resampling were not band-limited.
    [InlineData(10)]
    [InlineData(13)]
    [InlineData(17)]
    [InlineData(22)]
    [InlineData(27)]
    public void REQ_CORE_070_resampling_a_pulse_leaves_no_aliasing_product_above_minus_sixty_dbfs(int period)
    {
        const int sampleRate = 48_000;
        const int size = 8192;

        NesCore core = Core(sampleRate);
        StartPulse1(core, period);

        float[] samples = Capture(core, size, skip: 8192);
        double[] spectrum = HannSpectrum(samples);

        double binHz = (double)sampleRate / size;
        double fundamental = PulseFrequency(core, period);

        // Bins within eight of a harmonic that is genuinely below Nyquist hold signal, not
        // aliasing: the window's own leakage lives there.
        var signalBins = new HashSet<int>();
        for (double harmonic = fundamental; harmonic < sampleRate / 2.0; harmonic += 2 * fundamental)
        {
            int centre = (int)Math.Round(harmonic / binHz);
            for (int offset = -8; offset <= 8; offset++)
            {
                signalBins.Add(centre + offset);
            }
        }

        // The lowest bins hold the DC blocker's own residue.
        for (int bin = 0; bin <= 8; bin++)
        {
            signalBins.Add(bin);
        }

        int worstBin = -1;
        double worst = 0.0;
        for (int bin = 0; bin < spectrum.Length; bin++)
        {
            if (!signalBins.Contains(bin) && spectrum[bin] > worst)
            {
                worst = spectrum[bin];
                worstBin = bin;
            }
        }

        double worstDb = 20.0 * Math.Log10(Math.Max(worst, 1e-12));

        Assert.True(
            worstDb <= -60.0,
            string.Create(
                CultureInfo.InvariantCulture,
                $"period {period} ({fundamental:F0} Hz): aliasing product of {worstDb:F1} dBFS "
                + $"at {worstBin * binHz:F0} Hz"));
    }

    [Theory]
    // The anti-aliasing filter must not swallow the signal it is protecting. A 50 % square of
    // 0.172 peak to peak carries 2/pi x 0.172/2 at its fundamental; the passband runs flat to
    // roughly 0.31 of the output rate, so a tone from 1 to 10 kHz has to come out at that
    // level.
    [InlineData(111)]
    [InlineData(27)]
    [InlineData(10)]
    public void REQ_CORE_070_the_passband_carries_the_fundamental_at_its_full_level(int period)
    {
        NesCore core = Core();
        StartPulse1(core, period);

        float[] samples = Capture(core, 8192, skip: 8192);

        double amplitude = AmplitudeAt(samples, PulseFrequency(core, period), 48_000);

        // A square alternating between 0 and h carries 2h/pi at its fundamental.
        double expected = 2.0 * 0.172 / Math.PI;

        Assert.InRange(amplitude, expected * 0.95, expected * 1.05);
    }

    /// <summary>
    /// Amplitude of the component of <paramref name="samples"/> at exactly
    /// <paramref name="frequency"/>, measured by evaluating the windowed transform at that
    /// frequency rather than at the nearest bin, which avoids the window's scalloping loss.
    /// </summary>
    private static double AmplitudeAt(float[] samples, double frequency, int sampleRate)
    {
        int size = samples.Length;
        var sum = Complex.Zero;

        for (int i = 0; i < size; i++)
        {
            double window = 0.5 - (0.5 * Math.Cos(2.0 * Math.PI * i / size));
            double angle = -2.0 * Math.PI * frequency * i / sampleRate;
            sum += samples[i] * window * Complex.FromPolarCoordinates(1.0, angle);
        }

        return 4.0 * sum.Magnitude / size;
    }

    /// <summary>
    /// Amplitude spectrum of <paramref name="samples"/> under a Hann window, normalised so
    /// that a sinusoid of amplitude <c>a</c> reads back as <c>a</c>.
    /// </summary>
    private static double[] HannSpectrum(float[] samples)
    {
        int size = samples.Length;
        var buffer = new Complex[size];

        for (int i = 0; i < size; i++)
        {
            double window = 0.5 - (0.5 * Math.Cos(2.0 * Math.PI * i / size));
            buffer[i] = new Complex(samples[i] * window, 0.0);
        }

        Fft(buffer);

        // A Hann window halves the coherent gain, and a real signal's energy is split between
        // the positive and the negative frequency.
        var spectrum = new double[size / 2];
        for (int bin = 0; bin < spectrum.Length; bin++)
        {
            spectrum[bin] = 4.0 * buffer[bin].Magnitude / size;
        }

        return spectrum;
    }

    /// <summary>In-place radix-2 FFT; <paramref name="buffer"/>'s length must be a power of two.</summary>
    private static void Fft(Complex[] buffer)
    {
        int size = buffer.Length;

        for (int i = 1, j = 0; i < size; i++)
        {
            int bit = size >> 1;
            for (; (j & bit) != 0; bit >>= 1)
            {
                j ^= bit;
            }

            j ^= bit;

            if (i < j)
            {
                (buffer[i], buffer[j]) = (buffer[j], buffer[i]);
            }
        }

        for (int length = 2; length <= size; length <<= 1)
        {
            double angle = -2.0 * Math.PI / length;
            var step = Complex.FromPolarCoordinates(1.0, angle);

            for (int start = 0; start < size; start += length)
            {
                Complex twiddle = Complex.One;
                for (int k = 0; k < length / 2; k++)
                {
                    Complex even = buffer[start + k];
                    Complex odd = buffer[start + k + (length / 2)] * twiddle;
                    buffer[start + k] = even + odd;
                    buffer[start + k + (length / 2)] = even - odd;
                    twiddle *= step;
                }
            }
        }
    }
}
