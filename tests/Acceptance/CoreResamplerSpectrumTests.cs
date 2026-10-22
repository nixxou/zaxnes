using Zax.Nes;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The spectral half of REQ-CORE-070: on a sweep of the pulse channel from 100 Hz to
/// 10 kHz, the aliasing products — spectral components that are not harmonics of the
/// emulated signal — must stay at -60 dBFS or below.
/// </summary>
public sealed class CoreResamplerSpectrumTests
{
    private const int Size = 16384;          // FFT length: 2.93 Hz per bin at 48 kHz
    private const int SampleRate = 48_000;
    private const double Threshold = 0.001;  // -60 dBFS
    private const int LobeHalfWidth = 5;     // main-lobe half width of the window, in bins

    /// <summary>
    /// Pulse timer periods spanning the required range. The channel frequency is
    /// <c>CPU / (16 (t + 1))</c>, so these give roughly 100, 250, 440, 1 000, 2 000, 4 400
    /// and 10 000 Hz.
    /// </summary>
    [Theory] // REQ-CORE-070
    [InlineData(1117)]
    [InlineData(446)]
    [InlineData(253)]
    [InlineData(110)]
    [InlineData(54)]
    [InlineData(24)]
    [InlineData(10)]
    public void No_aliasing_product_rises_above_minus_60_dbfs(int timer)
    {
        (float[] samples, double cyclesPerSample) = CaptureSweepStep(timer);

        double[] window = BlackmanHarris(Size);
        double windowSum = window.Sum();

        double[] re = new double[Size];
        double[] im = new double[Size];
        for (int i = 0; i < Size; i++)
        {
            re[i] = samples[i] * window[i];
        }

        Fft(re, im);

        double[] amplitude = new double[(Size / 2) + 1];
        for (int bin = 0; bin < amplitude.Length; bin++)
        {
            amplitude[bin] = 2.0 * Math.Sqrt((re[bin] * re[bin]) + (im[bin] * im[bin])) / windowSum;
        }

        // The channel's period is 16 (t + 1) CPU cycles, which gives the fundamental to
        // about 0.1 % through the measured cycles-per-sample ratio. That is not accurate
        // enough at high harmonics, so the estimate is refined on the spectrum itself by
        // locking onto harmonics 1, 2, 4, 8 ... in turn: each step halves the relative
        // error of the previous one.
        double f0 = RefineFundamental(amplitude, SampleRate * cyclesPerSample / (16.0 * (timer + 1)));
        Assert.InRange(f0, 90.0, 11_000.0);

        bool[] expected = new bool[amplitude.Length];
        for (int bin = 0; bin <= LobeHalfWidth + 2; bin++)
        {
            expected[bin] = true; // DC and the DC-blocker's skirt
        }

        for (int harmonic = 1; harmonic * f0 < SampleRate / 2.0; harmonic++)
        {
            int centre = (int)Math.Round(harmonic * f0 * Size / SampleRate);
            for (int bin = centre - LobeHalfWidth; bin <= centre + LobeHalfWidth; bin++)
            {
                if (bin >= 0 && bin < expected.Length)
                {
                    expected[bin] = true;
                }
            }
        }

        double worst = 0;
        int worstBin = -1;
        for (int bin = 0; bin < amplitude.Length; bin++)
        {
            if (!expected[bin] && amplitude[bin] > worst)
            {
                worst = amplitude[bin];
                worstBin = bin;
            }
        }

        double dbfs = worst > 0 ? 20 * Math.Log10(worst) : double.NegativeInfinity;
        Assert.True(
            worst <= Threshold,
            $"f0 = {f0:F1} Hz: aliasing product of {dbfs:F1} dBFS at "
            + $"{(worstBin * (double)SampleRate / Size):F1} Hz "
            + $"(harmonic {(worstBin * (double)SampleRate / Size / f0):F3}; budget -60 dBFS)");
    }

    /// <summary>
    /// Locks the fundamental estimate onto the spectrum, doubling the harmonic each round so
    /// that the relative error shrinks geometrically.
    /// </summary>
    private static double RefineFundamental(double[] amplitude, double seed)
    {
        double f = seed;
        for (int harmonic = 1; harmonic * f < 16_000; harmonic *= 2)
        {
            int centre = (int)Math.Round(harmonic * f * Size / SampleRate);
            int span = Math.Max(3, (int)(0.3 * f * Size / SampleRate));
            int low = Math.Max(1, centre - span);
            int high = Math.Min(amplitude.Length - 2, centre + span);
            if (low >= high)
            {
                break;
            }

            int peak = low;
            for (int bin = low; bin <= high; bin++)
            {
                if (amplitude[bin] > amplitude[peak])
                {
                    peak = bin;
                }
            }

            if (amplitude[peak] < 2 * Threshold)
            {
                break;
            }

            // Parabolic interpolation on the log magnitude gives sub-bin accuracy.
            double a = Math.Log(amplitude[peak - 1]);
            double b = Math.Log(amplitude[peak]);
            double c = Math.Log(amplitude[peak + 1]);
            double delta = 0.5 * (a - c) / (a - (2 * b) + c);
            f = (peak + delta) * SampleRate / Size / harmonic;
        }

        return f;
    }

    private static (float[] Samples, double CyclesPerSample) CaptureSweepStep(int timer)
    {
        NesCore core = SpecRom.Warm();
        core.SampleRate = SampleRate;
        core.Region = NesRegion.Ntsc;

        core.WriteMemory(0x4015, 0x01);
        core.WriteMemory(0x4000, 0xBF);                 // 50 % duty, constant volume 15, halted
        core.WriteMemory(0x4001, 0x00);                 // sweep off
        core.WriteMemory(0x4002, (byte)(timer & 0xFF));
        core.WriteMemory(0x4003, (byte)(((timer >> 8) & 0x07) | 0x08));

        List<float> samples = [];
        float[] buffer = new float[1024];
        core.Apu.DiscardSamples();
        long startCycles = core.CycleCount;

        // Let the DC blocker settle, then collect.
        for (int frame = 0; frame < 120 && samples.Count < Size * 2; frame++)
        {
            core.RunFrame(0);
            int got;
            while ((got = core.ReadAudio(buffer)) > 0)
            {
                samples.AddRange(buffer.AsSpan(0, got).ToArray());
            }
        }

        Assert.True(samples.Count >= Size * 2, $"only {samples.Count} samples were produced");
        double cyclesPerSample = (core.CycleCount - startCycles) / (double)samples.Count;
        return ([.. samples.Skip(samples.Count - Size)], cyclesPerSample);
    }

    private static double[] BlackmanHarris(int n)
    {
        double[] window = new double[n];
        for (int i = 0; i < n; i++)
        {
            double t = 2 * Math.PI * i / (n - 1);
            window[i] = 0.35875
                - (0.48829 * Math.Cos(t))
                + (0.14128 * Math.Cos(2 * t))
                - (0.01168 * Math.Cos(3 * t));
        }

        return window;
    }

    /// <summary>In-place iterative radix-2 FFT.</summary>
    private static void Fft(double[] re, double[] im)
    {
        int n = re.Length;
        for (int i = 1, j = 0; i < n; i++)
        {
            int bit = n >> 1;
            for (; (j & bit) != 0; bit >>= 1)
            {
                j ^= bit;
            }

            j |= bit;
            if (i < j)
            {
                (re[i], re[j]) = (re[j], re[i]);
                (im[i], im[j]) = (im[j], im[i]);
            }
        }

        for (int length = 2; length <= n; length <<= 1)
        {
            double angle = -2 * Math.PI / length;
            double wRe = Math.Cos(angle);
            double wIm = Math.Sin(angle);
            for (int start = 0; start < n; start += length)
            {
                double curRe = 1;
                double curIm = 0;
                for (int k = 0; k < length / 2; k++)
                {
                    int a = start + k;
                    int b = a + (length / 2);
                    double tRe = (re[b] * curRe) - (im[b] * curIm);
                    double tIm = (re[b] * curIm) + (im[b] * curRe);
                    re[b] = re[a] - tRe;
                    im[b] = im[a] - tIm;
                    re[a] += tRe;
                    im[a] += tIm;

                    double nextRe = (curRe * wRe) - (curIm * wIm);
                    curIm = (curRe * wIm) + (curIm * wRe);
                    curRe = nextRe;
                }
            }
        }
    }
}
