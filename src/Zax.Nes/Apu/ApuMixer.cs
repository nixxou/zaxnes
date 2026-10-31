namespace Zax.Nes.Apu;

/// <summary>
/// The non-linear mixer of REQ-CORE-073 (AMD-054): the two integer tables the five APU
/// channels are summed through, and the companion table the MMC5's two square channels use
/// (REQ-CORE-075).
/// </summary>
/// <remarks>
/// <para>
/// <strong>The two functions.</strong> nesdev's "APU Mixer" page gives the mixer as two real
/// functions of a channel index, which REQ-CORE-073 restates:
/// <c>P(i) = 95.88 / (8128 / i + 100)</c> for the two pulses summed, <c>i</c> from 0 to 30,
/// and <c>T(i) = 163.67 / (24329 / i + 100)</c> for the triangle, the noise and the DMC
/// summed as <c>3t + 2n + d</c>, <c>i</c> from 0 to 202. Both are 0 at <c>i = 0</c>, which
/// the formulas do not define.
/// </para>
/// <para>
/// <strong>Why 95.88 and not 95.52.</strong> The same page gives two sets of constants: the
/// analytic forms (95.88 for the pulses, 159.79 for the rest) and the lookup-table forms
/// (95.52 and 163.67). REQ-CORE-073 asks for 95.88 together with <c>163.67 / 24329</c>,
/// which is the mix of the two the reference uses; 95.52 would give 4876 / 8438 / 8371
/// instead of the 4890 / 8462 / 8363 the requirement's criterion names. The choice is the
/// requirement's, and its own note records it.
/// </para>
/// <para>
/// <strong>Scaling.</strong> <see cref="Normalisation"/> is
/// <c>K = 32767 / (P(30) + T(202))</c>, computed on the unrounded values in double
/// precision, so that the two terms at full scale add up to exactly 32767 — 8462 + 24305,
/// which holds after both roundings and is what stops <c>A</c> on its own from ever
/// overflowing. The two tables are the rounded products <c>r(K · P(i))</c> and
/// <c>r(K · T(i))</c>, <c>r</c> rounding to the nearest integer and to the even one on a
/// tie, as REQ-CORE-073 prescribes. One entry sits close enough to a tie for the rule to be
/// worth pinning: <c>K · T(196) = 23905.4997</c>.
/// </para>
/// <para>
/// <strong>MMC5.</strong> <see cref="Mmc5Normalisation"/> is <c>Km = 8000 / P(30)</c>, so
/// that the board's two square channels together top out at 8000 rather than at the APU's
/// 8462, and its PCM channel steps by <see cref="Mmc5PcmStep"/> per unit. Both constants are
/// the reference's, and REQ-CORE-075 records that they put the PCM channel well below where
/// the public documentation would (an accepted divergence).
/// </para>
/// </remarks>
public static class ApuMixer
{
    /// <summary>Largest value the mixer produces, <c>A</c> and <c>S</c> alike (REQ-CORE-073).</summary>
    public const int FullScale = 32_767;

    /// <summary>Divisor taking the integer output to the [−1, 1] scale of REQ-CORE-071.</summary>
    public const double FloatScale = 32_768.0;

    /// <summary>Largest index of the pulse table: both pulses at volume 15.</summary>
    public const int PulseIndexMax = 30;

    /// <summary>Largest index of the triangle/noise/DMC table: 3×15 + 2×15 + 127.</summary>
    public const int TndIndexMax = 202;

    /// <summary>Value the MMC5's two square channels reach together at volume 15 (REQ-CORE-075).</summary>
    public const int Mmc5FullScale = 8_000;

    /// <summary>Mixer units one unit of the MMC5's PCM level contributes (REQ-CORE-075).</summary>
    public const int Mmc5PcmStep = 31;

    /// <summary>Numerator of <c>P</c>, nesdev's analytic form for the pulses.</summary>
    private const double PulseNumerator = 95.88;

    /// <summary>Denominator constant of <c>P</c>.</summary>
    private const double PulseDenominator = 8128.0;

    /// <summary>Numerator of <c>T</c>, nesdev's lookup-table form.</summary>
    private const double TndNumerator = 163.67;

    /// <summary>Denominator constant of <c>T</c>.</summary>
    private const double TndDenominator = 24329.0;

    private static readonly double[] PulseReal;
    private static readonly double[] TndReal;
    private static readonly int[] PulseScaled;
    private static readonly int[] TndScaled;
    private static readonly int[] Mmc5PulseScaled;

    /// <summary>
    /// Builds the two real tables, then <c>K</c>, <c>Km</c> and the three integer tables, in
    /// that order: the normalisations are computed on the unrounded values, never on rounded
    /// ones, which REQ-CORE-073 requires.
    /// </summary>
    static ApuMixer()
    {
        PulseReal = Real(PulseIndexMax, PulseNumerator, PulseDenominator);
        TndReal = Real(TndIndexMax, TndNumerator, TndDenominator);

        Normalisation = FullScale / (PulseReal[PulseIndexMax] + TndReal[TndIndexMax]);
        Mmc5Normalisation = Mmc5FullScale / PulseReal[PulseIndexMax];

        PulseScaled = Scale(PulseReal, Normalisation);
        TndScaled = Scale(TndReal, Normalisation);
        Mmc5PulseScaled = Scale(PulseReal, Mmc5Normalisation);
    }

    /// <summary><c>K = 32767 / (P(30) + T(202))</c> (REQ-CORE-073).</summary>
    public static double Normalisation { get; }

    /// <summary><c>Km = 8000 / P(30)</c> (REQ-CORE-075).</summary>
    public static double Mmc5Normalisation { get; }

    /// <summary>The 31 entries <c>r(K · P(i))</c> of REQ-CORE-073.</summary>
    public static ReadOnlySpan<int> PulseTable => PulseScaled;

    /// <summary>The 203 entries <c>r(K · T(i))</c> of REQ-CORE-073.</summary>
    public static ReadOnlySpan<int> TndTable => TndScaled;

    /// <summary>The 31 entries <c>r(Km · P(i))</c> of REQ-CORE-075.</summary>
    public static ReadOnlySpan<int> Mmc5PulseTable => Mmc5PulseScaled;

    /// <summary><c>P(i)</c>, the real pulse function of REQ-CORE-073, 0 at <c>i = 0</c>.</summary>
    public static double Pulse(int i)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(i);
        ArgumentOutOfRangeException.ThrowIfGreaterThan(i, PulseIndexMax);
        return PulseReal[i];
    }

    /// <summary><c>T(i)</c>, the real triangle/noise/DMC function, 0 at <c>i = 0</c>.</summary>
    public static double Tnd(int i)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(i);
        ArgumentOutOfRangeException.ThrowIfGreaterThan(i, TndIndexMax);
        return TndReal[i];
    }

    /// <summary>
    /// Rounds <paramref name="value"/> as the <c>r</c> of REQ-CORE-073 does: to the nearest
    /// integer, and to the even one when the two are equally near.
    /// </summary>
    public static int Round(double value) => (int)Math.Round(value, MidpointRounding.ToEven);

    /// <summary>
    /// <c>A</c> of REQ-CORE-073: the APU's own contribution, from the five channel outputs.
    /// </summary>
    /// <param name="pulse1">Pulse 1 output, 0 to 15.</param>
    /// <param name="pulse2">Pulse 2 output, 0 to 15.</param>
    /// <param name="triangle">Triangle output, 0 to 15.</param>
    /// <param name="noise">Noise output, 0 to 15.</param>
    /// <param name="dmc">DMC output, 0 to 127.</param>
    public static int Mix(int pulse1, int pulse2, int triangle, int noise, int dmc)
        => MixIndexed(pulse1 + pulse2, (3 * triangle) + (2 * noise) + dmc);

    /// <summary>
    /// <c>A</c> of REQ-CORE-073 from the two table indices already summed, which is what the
    /// APU itself tracks.
    /// </summary>
    internal static int MixIndexed(int pulseIndex, int tndIndex)
        => PulseScaled[pulseIndex] + TndScaled[tndIndex];

    /// <summary>
    /// <c>S = min(A + M, 32767)</c> of REQ-CORE-073, the value the test point of
    /// REQ-CORE-076 reports.
    /// </summary>
    public static int Clip(int apu, int expansion) => Math.Min(apu + expansion, FullScale);

    /// <summary>
    /// Takes an integer mixer output to the [−1, 1] scale of REQ-CORE-071, by
    /// <c>S / 32768</c>.
    /// </summary>
    public static double ToFloat(int output) => output / FloatScale;

    /// <summary>Tabulates <c>numerator / (denominator / i + 100)</c> for <c>i = 0..max</c>.</summary>
    private static double[] Real(int max, double numerator, double denominator)
    {
        var table = new double[max + 1];
        for (int i = 1; i <= max; i++)
        {
            table[i] = numerator / ((denominator / i) + 100.0);
        }

        return table;
    }

    /// <summary>Scales a real table by <paramref name="factor"/> and rounds every entry.</summary>
    private static int[] Scale(double[] real, double factor)
    {
        var table = new int[real.Length];
        for (int i = 0; i < real.Length; i++)
        {
            table[i] = Round(factor * real[i]);
        }

        return table;
    }
}
