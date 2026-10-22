using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// Turns the APU's output, which changes at CPU-cycle resolution, into samples at the host's
/// output rate, without aliasing (REQ-CORE-070).
/// </summary>
/// <remarks>
/// <para>
/// <strong>Why not plain decimation.</strong> The APU runs at about 1.79 MHz and the host
/// wants 48 kHz: a naive "take one cycle in 37" folds everything above 24 kHz back into the
/// audible band, and a square wave is nothing but content above 24 kHz. REQ-CORE-070 caps
/// those folded products at −60 dBFS, so the signal has to be low-pass filtered at the
/// output's Nyquist frequency before it is sampled.
/// </para>
/// <para>
/// <strong>Band-limited step synthesis.</strong> Filtering 1.79 million samples a second
/// through a kernel sharp enough for that would be absurd, and it is also unnecessary: the
/// APU's output is piecewise constant, so all the information it carries is the list of
/// <em>steps</em> (time, amplitude change). This class therefore adds, for every step, a
/// band-limited impulse into an output-rate accumulator at the step's exact fractional
/// position, and reads the signal back as the running sum of that accumulator — the running
/// sum of a band-limited impulse being a band-limited step. The cost is proportional to the
/// number of steps, not to the source rate. The technique is the publicly documented one
/// behind blargg's band-limited synthesis work, which the nesdev wiki points at for APU
/// resampling; no code was taken from it.
/// </para>
/// <para>
/// <strong>Kernel.</strong> <see cref="Taps"/> output samples of a sinc windowed by a
/// Blackman window, at <see cref="Phases"/> sub-sample offsets, linearly interpolated
/// between neighbouring offsets so that the residual timing error is second order rather
/// than first. Each offset's row is normalised to sum to exactly 1, so a step always ends up
/// exactly as tall as it should be and no DC error accumulates. With 32 taps the transition
/// band is about 0.17 of the output rate wide, so the cutoff sits at 0.4: flat to 0.31 of
/// the output rate (15 kHz at 48 kHz, which is also roughly where the real console's own
/// low-pass sits) and about 74 dB down well before Nyquist, which leaves margin on the
/// −60 dBFS of REQ-CORE-070.
/// </para>
/// <para>
/// Every arithmetic step is a fixed sequence of IEEE-754 double operations on integer
/// inputs, so two runs from the same state produce bit-identical samples (REQ-CORE-092).
/// </para>
/// </remarks>
internal sealed class BandLimitedResampler
{
    /// <summary>Cutoff of the DC-blocking high-pass, in Hz (REQ-CORE-073: 10 to 40 Hz).</summary>
    public const double HighPassHz = 20.0;

    /// <summary>Output samples the kernel spans.</summary>
    private const int Taps = 32;

    /// <summary>Taps to the left of the step's position.</summary>
    private const int LeadingTaps = 15;

    /// <summary>Sub-sample offsets the kernel is tabulated at.</summary>
    private const int Phases = 128;

    /// <summary>Bits of <see cref="_fraction"/> that select the offset.</summary>
    private const int PhaseBits = 7;

    /// <summary>Cutoff frequency, as a fraction of the output rate.</summary>
    private const double Cutoff = 0.40;

    /// <summary>Length of the accumulator ring; a power of two above <see cref="Taps"/>.</summary>
    private const int RingSize = 64;

    /// <summary>
    /// Safety cap on the ratio: half an output sample per source cycle. Reaching it would
    /// need a speed factor below 0.06 (REQ-CORE-072), far outside anything SPEC-07 offers,
    /// and the fixed-point step below has to stay inside 32 bits.
    /// </summary>
    private const double MaxRatio = 0.5;

    /// <summary>
    /// Kernel rows, <c>Phases + 1</c> of them so that interpolation towards the next offset
    /// never runs off the end. The extra row is the kernel at a whole-sample offset, which is
    /// the same shape one tap over, so the table is continuous across a sample boundary.
    /// </summary>
    private static readonly double[] Kernel = BuildKernel();

    private readonly double[] _ring = new double[RingSize];
    private readonly Action<float> _sink;

    /// <summary>Fractional part of the output position, as a 0.32 fixed-point value.</summary>
    private uint _fraction;

    /// <summary>Output sample the source is currently at. Starts past the kernel's lead-in.</summary>
    private long _position = LeadingTaps + 1;

    /// <summary>Next output sample to emit.</summary>
    private long _emitted = 1;

    /// <summary>Step added to <see cref="_fraction"/> per source cycle: output samples per CPU cycle.</summary>
    private uint _step;

    /// <summary>Running sum of <see cref="_ring"/>: the signal itself.</summary>
    private double _signal;

    /// <summary>Last level a step was emitted for.</summary>
    private double _level;

    private double _highPassCoefficient = 0.9974;
    private double _highPassPrevious;
    private double _highPassOutput;

    /// <summary>Creates a resampler that hands every finished sample to <paramref name="sink"/>.</summary>
    public BandLimitedResampler(Action<float> sink)
    {
        ArgumentNullException.ThrowIfNull(sink);
        _sink = sink;
    }

    /// <summary>
    /// Sets the resampling ratio.
    /// </summary>
    /// <param name="samplesPerSourceCycle">Output samples produced per source (CPU) cycle.</param>
    /// <param name="sampleRate">Output rate in Hz, for the high-pass coefficient.</param>
    public void SetRatio(double samplesPerSourceCycle, int sampleRate)
    {
        _step = (uint)Math.Round(Math.Min(samplesPerSourceCycle, MaxRatio) * 4294967296.0);
        _highPassCoefficient = Math.Exp(-2.0 * Math.PI * HighPassHz / sampleRate);
    }

    /// <summary>
    /// Advances the source clock by one CPU cycle, having been handed the output level for
    /// that cycle.
    /// </summary>
    /// <param name="level">Mixed output of the five channels, already inside [−1, 1].</param>
    public void StepCycle(double level)
    {
        if (level != _level)
        {
            AddStep(level - _level);
            _level = level;
        }

        ulong advanced = (ulong)_fraction + _step;
        _fraction = (uint)advanced;

        if (advanced > uint.MaxValue)
        {
            _position++;
            Drain();
        }
    }

    /// <summary>Back to silence: no pending step, no pending sample (REQ-CORE-074).</summary>
    public void Reset()
    {
        Array.Clear(_ring);
        _fraction = 0;
        _position = LeadingTaps + 1;
        _emitted = 1;
        _signal = 0.0;
        _level = 0.0;
        _highPassPrevious = 0.0;
        _highPassOutput = 0.0;
    }

    /// <summary>Appends the resampler state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        foreach (double value in _ring)
        {
            writer.WriteDouble(value);
        }

        writer.WriteInt32(unchecked((int)_fraction));
        writer.WriteInt64(_position);
        writer.WriteInt64(_emitted);
        writer.WriteDouble(_signal);
        writer.WriteDouble(_level);
        writer.WriteDouble(_highPassPrevious);
        writer.WriteDouble(_highPassOutput);
    }

    /// <summary>Restores the resampler state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        for (int i = 0; i < _ring.Length; i++)
        {
            _ring[i] = reader.ReadDouble();
        }

        _fraction = unchecked((uint)reader.ReadInt32());
        _position = reader.ReadInt64();
        _emitted = reader.ReadInt64();
        _signal = reader.ReadDouble();
        _level = reader.ReadDouble();
        _highPassPrevious = reader.ReadDouble();
        _highPassOutput = reader.ReadDouble();
    }

    /// <summary>Bytes a saved resampler state occupies, for <c>MaxStateSize</c>.</summary>
    public static int StateSize => (RingSize * 8) + 4 + 8 + 8 + (4 * 8);

    /// <summary>
    /// Adds a band-limited step of <paramref name="delta"/> at the current fractional
    /// position.
    /// </summary>
    private void AddStep(double delta)
    {
        int phase = (int)(_fraction >> (32 - PhaseBits));
        double interpolation = ((_fraction >> (32 - PhaseBits - 16)) & 0xFFFF) / 65536.0;

        int low = phase * Taps;
        int high = low + Taps;
        int start = (int)((_position - LeadingTaps) & (RingSize - 1));

        for (int tap = 0; tap < Taps; tap++)
        {
            double weight = Kernel[low + tap] + ((Kernel[high + tap] - Kernel[low + tap]) * interpolation);
            _ring[(start + tap) & (RingSize - 1)] += weight * delta;
        }
    }

    /// <summary>
    /// Emits every output sample no future step can still reach. A step at source position
    /// <c>p</c> writes to output samples <c>p − 15</c> to <c>p + 16</c>, and the position
    /// only moves forward, so everything below <c>_position − 15</c> is final.
    /// </summary>
    private void Drain()
    {
        while (_emitted < _position - LeadingTaps)
        {
            int slot = (int)(_emitted & (RingSize - 1));
            _signal += _ring[slot];
            _ring[slot] = 0.0;
            _emitted++;

            // One-pole DC blocker (REQ-CORE-073). The level the mixer produces is
            // non-negative, so without it every sample would sit on a large offset.
            _highPassOutput = _signal - _highPassPrevious + (_highPassCoefficient * _highPassOutput);
            _highPassPrevious = _signal;

            _sink((float)Math.Clamp(_highPassOutput, -1.0, 1.0));
        }
    }

    /// <summary>Integration sub-steps per kernel offset, for tabulating the step response.</summary>
    private const int IntegrationSubSteps = 8;

    /// <summary>
    /// Builds the kernel rows as successive differences of the band-limited step response.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The obvious choice — the windowed sinc itself — is wrong here, and audibly so. Reading
    /// the signal back as a running sum of the accumulator is a discrete integrator, whose
    /// transfer function is <c>1 / (1 − z⁻¹)</c>, while a step inserted as a Dirac is a
    /// continuous-time derivative. The two do not cancel: what is left over is
    /// <c>ω / (2 sin(ω/2))</c>, a treble boost worth 0.7 dB at 10 kHz and 3.9 dB at Nyquist.
    /// </para>
    /// <para>
    /// Tabulating <c>S(x) − S(x − 1)</c> instead, where <c>S</c> is the integral of the
    /// windowed sinc, removes the problem by construction: the running sum of a row
    /// telescopes back to <c>S</c>, so the output is exactly the band-limited step response
    /// and the transfer function is exactly the low-pass that was designed. It also makes
    /// each row sum to 1 on its own, which is what keeps a step landing at its own height
    /// whatever the sub-sample offset, and the output free of DC error.
    /// </para>
    /// </remarks>
    private static double[] BuildKernel()
    {
        // S tabulated from −17 to +17 output samples, at the kernel's offset resolution.
        const int span = 2 * (Taps + 1);
        const int origin = (Taps + 1) * Phases;
        double[] step = new double[(span * Phases) + 1];

        double accumulated = 0.0;
        double dx = 1.0 / (Phases * IntegrationSubSteps);

        for (int i = 1; i < step.Length; i++)
        {
            // Trapezoidal integration of the windowed sinc over one offset's worth of x.
            double from = (double)(i - 1 - origin) / Phases;
            for (int sub = 0; sub < IntegrationSubSteps; sub++)
            {
                double a = from + (sub * dx);
                accumulated += (Impulse(a) + Impulse(a + dx)) * 0.5 * dx;
            }

            step[i] = accumulated;
        }

        var kernel = new double[(Phases + 1) * Taps];

        for (int phase = 0; phase <= Phases; phase++)
        {
            int row = phase * Taps;
            double sum = 0.0;

            for (int tap = 0; tap < Taps; tap++)
            {
                // x = tap − LeadingTaps − phase / Phases, as an exact index into the table.
                int index = ((tap - LeadingTaps) * Phases) - phase + origin;
                double value = step[index] - step[index - Phases];

                kernel[row + tap] = value;
                sum += value;
            }

            for (int tap = 0; tap < Taps; tap++)
            {
                kernel[row + tap] /= sum;
            }
        }

        return kernel;
    }

    /// <summary>The windowed sinc the step response integrates, zero outside the window.</summary>
    private static double Impulse(double x)
        => Math.Abs(x) >= Taps / 2.0 ? 0.0 : 2.0 * Cutoff * Sinc(2.0 * Cutoff * x) * Blackman(x / Taps);

    private static double Sinc(double x)
        => x == 0.0 ? 1.0 : Math.Sin(Math.PI * x) / (Math.PI * x);

    /// <summary>Blackman window, <paramref name="x"/> running over [−0.5, 0.5].</summary>
    private static double Blackman(double x)
    {
        double t = x + 0.5;
        return 0.42 - (0.5 * Math.Cos(2.0 * Math.PI * t)) + (0.08 * Math.Cos(4.0 * Math.PI * t));
    }
}
