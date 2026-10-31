using System.Globalization;
using System.Numerics;
using Zax.Nes.Apu;

namespace Zax.Nes.Tests;

/// <summary>
/// The output path: the non-linear mix of REQ-CORE-073 and its test point (REQ-CORE-076), the
/// DC blocker, and the band-limited resampling of REQ-CORE-070.
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

    /// <summary>
    /// REQ-CORE-073 (AMD-054) — the four figures the requirement's first criterion names, read
    /// at the test point of REQ-CORE-076: a lone pulse at volume 15 gives 4 890, the two
    /// pulses 8 462 — a ratio of 1.73 rather than the 2 a linear law would give — the triangle
    /// alone 8 363, and every channel at its maximum exactly 32 767.
    /// </summary>
    [Fact] // REQ-CORE-073, REQ-CORE-076
    public void REQ_CORE_073_the_mixer_produces_the_documented_integers()
    {
        // A lone pulse at volume 15, 50 % duty, and both pulses on the same period in phase.
        Assert.Equal(4890, LivePeak(core => StartPulse1(core, period: 100)));
        Assert.Equal(8462, LivePeak(core =>
        {
            StartPulse1(core, period: 100);
            core.WriteMemory(0x4015, 0x03);
            core.WriteMemory(0x4004, 0xBF);
            core.WriteMemory(0x4005, 0x08);
            core.WriteMemory(0x4006, 100);
            core.WriteMemory(0x4007, 0xF8);
        }));

        // The ratio the note contrasts with the linear law's 2.
        Assert.InRange(8462.0 / 4890.0, 1.725, 1.735);

        // The triangle alone: its index runs 45 down to 0, so 8 363 is the excursion of the
        // staircase rather than a volume setting — the channel has none.
        Assert.Equal(8363, LivePeak(
            core =>
            {
                core.WriteMemory(0x4015, 0x04);
                core.WriteMemory(0x4008, 0x7F);
                core.WriteMemory(0x400A, 50);
                core.WriteMemory(0x400B, 0xF8);
                core.WriteMemory(0x4017, 0x80);
            },
            cycles: 40 * 51));
        Assert.Equal(8363, ApuMixer.TndTable[45]);

        // Every channel at its maximum: index 30 on one table, 3x15 + 2x15 + 127 = 202 on the
        // other. Both roundings land so that the two terms add up to full scale exactly.
        Assert.Equal(ApuMixer.FullScale, ApuMixer.Mix(15, 15, 15, 15, 127));
        Assert.Equal(8462, ApuMixer.PulseTable[30]);
        Assert.Equal(24305, ApuMixer.TndTable[202]);
    }

    /// <summary>
    /// REQ-CORE-073 (AMD-054) — the two tables are built from the real functions and scaled by
    /// <c>K</c>, which is computed on the unrounded values; the rounding goes to the nearest
    /// integer and to the even one on a tie, which the entry <c>K · T(196) = 23 905.4997</c> is
    /// near enough to be worth pinning. The numerator is 95.88, the analytic one, and not the
    /// 95.52 of the same page's lookup table, which would give 4 876 / 8 438 / 8 371.
    /// </summary>
    [Fact] // REQ-CORE-073
    public void REQ_CORE_073_the_tables_follow_the_documented_formulas()
    {
        Assert.Equal(31, ApuMixer.PulseTable.Length);
        Assert.Equal(203, ApuMixer.TndTable.Length);
        Assert.Equal(0, ApuMixer.PulseTable[0]);
        Assert.Equal(0, ApuMixer.TndTable[0]);

        for (int i = 1; i <= 30; i++)
        {
            Assert.Equal(95.88 / ((8128.0 / i) + 100.0), ApuMixer.Pulse(i), 12);
        }

        for (int i = 1; i <= 202; i++)
        {
            Assert.Equal(163.67 / ((24329.0 / i) + 100.0), ApuMixer.Tnd(i), 12);
        }

        Assert.Equal(32767.0 / (ApuMixer.Pulse(30) + ApuMixer.Tnd(202)), ApuMixer.Normalisation, 9);
        Assert.Equal(32_735.8776, ApuMixer.Normalisation, 3);

        // Round half to even, as the requirement asks.
        Assert.Equal(2, ApuMixer.Round(2.5));
        Assert.Equal(2, ApuMixer.Round(1.5));
        Assert.Equal(23_905, ApuMixer.TndTable[196]);
        Assert.InRange(ApuMixer.Normalisation * ApuMixer.Tnd(196), 23_905.499, 23_905.5);

        // The lookup-table numerator the requirement's note sets aside, with the K its own
        // table would give: 4 876 for a lone pulse instead of 4 890.
        static double LookupPulse(int i) => 95.52 / ((8128.0 / i) + 100.0);
        double lookupK = 32767.0 / (LookupPulse(30) + ApuMixer.Tnd(202));
        Assert.Equal(4876, ApuMixer.Round(lookupK * LookupPulse(15)));
        Assert.Equal(8438, ApuMixer.Round(lookupK * LookupPulse(30)));
    }

    /// <summary>
    /// REQ-CORE-073 (AMD-054) — the chain from the mixer's integer to the float the host reads:
    /// <c>S = min(A + M, 32 767)</c>, then <c>S / 32 768</c>. Without expansion audio
    /// <c>M = 0</c> and <c>S = A</c>.
    /// </summary>
    [Fact] // REQ-CORE-073, REQ-CORE-076
    public void REQ_CORE_073_the_integer_output_becomes_the_float_of_req_core_071()
    {
        NesCore core = Core();
        StartPulse1(core, period: 100);

        double peak = double.NegativeInfinity;
        int peakOutput = 0;
        for (int i = 0; i < 4000; i++)
        {
            core.Bus.Tick();
            Assert.Equal(0, core.Apu.MixerExpansion);
            Assert.Equal(core.Apu.MixerApu, core.Apu.MixerOutput);
            Assert.Equal(core.Apu.MixerOutput / 32_768.0, core.Apu.MixedLevel, 12);

            if (core.Apu.MixerOutput > peakOutput)
            {
                peakOutput = core.Apu.MixerOutput;
                peak = core.Apu.MixedLevel;
            }
        }

        Assert.Equal(4890, peakOutput);
        Assert.Equal(4890 / 32_768.0, peak, 12);

        // Full scale stays inside the [-1, 1] of REQ-CORE-071, with no clamping needed.
        Assert.InRange(ApuMixer.ToFloat(ApuMixer.FullScale), 0.99996, 1.0);
    }

    /// <summary>
    /// REQ-CORE-073 (AMD-054), second criterion, measured at the output of REQ-CORE-071 with
    /// the protocol Q-039 made normative and AMD-018 recorded: a pulse at 440 Hz (timer 253 in
    /// NTSC), 50 % duty, volume 15; the swing is the difference of the medians of the two
    /// plateaus, the first half of the samples discarded.
    /// </summary>
    /// <remarks>
    /// Taking raw min and max instead would measure the band-limited resampler's edge ringing,
    /// and taking the swing at another frequency would measure the DC blocker's droop: that is
    /// exactly the tension Q-039 resolved by fixing both the frequency and the estimator.
    /// </remarks>
    [Fact] // REQ-CORE-073, Q-039, AMD-018
    public void REQ_CORE_073_a_lone_pulse_at_volume_fifteen_swings_by_zero_point_one_four_nine()
    {
        NesCore core = Core();
        StartPulse1(core, period: 253); // 440 Hz in NTSC.

        float[] samples = Capture(core, 16_384);
        float[] tail = [.. samples.Skip(samples.Length / 2)];

        double mean = tail.Average();
        double high = Median([.. tail.Where(sample => sample > mean)]);
        double low = Median([.. tail.Where(sample => sample < mean)]);

        Assert.InRange(high - low, 0.149 - 0.002, 0.149 + 0.002);
    }

    private static double Median(float[] values)
    {
        Array.Sort(values);
        return values[values.Length / 2];
    }

    /// <summary>
    /// Largest <c>S</c> the test point reports over a run that <paramref name="start"/> sets
    /// up.
    /// </summary>
    private static int LivePeak(Action<NesCore> start, int cycles = 4000)
    {
        NesCore core = Core();
        start(core);

        int peak = 0;
        for (int i = 0; i < cycles; i++)
        {
            core.Bus.Tick();
            peak = Math.Max(peak, core.Apu.MixerOutput);
        }

        return peak;
    }

    // ---------------------------------------------------------------- REQ-CORE-076

    /// <summary>
    /// REQ-CORE-076 (AMD-054) — the test point itself: silence reads
    /// <c>A = M = S = 0</c>, and the three integers are what the acceptance criteria of
    /// REQ-CORE-073 and REQ-CORE-075 are expressed in.
    /// </summary>
    [Fact] // REQ-CORE-076
    public void REQ_CORE_076_silence_reads_zero_at_the_test_point()
    {
        NesCore core = Core();

        for (int i = 0; i < 2048; i++)
        {
            core.Bus.Tick();
            Assert.Equal(0, core.Apu.MixerApu);
            Assert.Equal(0, core.Apu.MixerExpansion);
            Assert.Equal(0, core.Apu.MixerOutput);
            Assert.Equal(0.0, core.Apu.MixedLevel);
        }
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
    // 4 890 / 32 768 peak to peak (REQ-CORE-073) carries 2/pi x that /2 at its fundamental;
    // the passband runs flat to roughly 0.31 of the output rate, so a tone from 1 to 10 kHz
    // has to come out at that level.
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
        double expected = 2.0 * ApuMixer.ToFloat(4890) / Math.PI;

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
