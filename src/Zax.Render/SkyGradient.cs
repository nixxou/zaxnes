using Zax.Engine3D;

namespace Zax.Render;

/// <summary>
/// The parts of the Gradient sky of SPEC-06 REQ-SCN-050 that do not need a GPU: the grey level
/// that decides whether stars appear, the star rotation speed and the two pole factors.
/// </summary>
public static class SkyGradient
{
    /// <summary>The stars of the Gradient sky turn 3° per second of real time.</summary>
    public const float StarDegreesPerSecond = 3f;

    /// <summary>Grey level of <c>M</c> at or below which stars are added.</summary>
    public const float StarGreyThreshold = 0.4f;

    /// <summary>Factor applied to <c>M</c> at the zenith, <c>Hc = 1.4 M</c>.</summary>
    public const float ZenithFactor = 1.4f;

    /// <summary>Factor applied to <c>M</c> at the nadir, <c>Bc = 0.6 M</c>.</summary>
    public const float NadirFactor = 0.6f;

    /// <summary>
    /// The linear factor the "×2" of REQ-SCN-051 stands for: what doubling an sRGB value does to
    /// its linear value, under the sRGB curve SPEC-06 §2 declares normative.
    /// </summary>
    /// <remarks>
    /// REQ-SCN-051 writes two numbers in the same sentence: « multiplier la teinte linéarisée par
    /// <b>4,595</b> (doublement en sRGB) ». 4.595 is 2^2.2, the pure power law; the piecewise
    /// sRGB curve of §2 gives <c>lin(1) / lin(0.5) = 4.672</c>. AMD-019 (Q-045) settles it: the
    /// curve of §2 is the authority, the exact value is the one it gives, « ×2 » was an
    /// approximation, and the criterion carries a ±2 % tolerance. Both numbers are inside that
    /// tolerance of each other (1.65 % apart), so criterion (b) of REQ-SCN-051 holds either way;
    /// what the amendment fixes is which one is exact.
    /// </remarks>
    // Q-045, Q-055
    public const float SrgbDouble = 4.672f;

    /// <summary>The grey level of REQ-SCN-050: <c>0.299 R + 0.587 G + 0.114 B</c> on sRGB 0–1.</summary>
    public static float Grey(Rgb24 color)
        => ((0.299f * color.R) + (0.587f * color.G) + (0.114f * color.B)) / 255f;

    /// <summary>Whether the Gradient sky shows stars for the background colour <paramref name="m"/>.</summary>
    public static bool StarsVisible(Rgb24 m) => Grey(m) <= StarGreyThreshold;

    /// <summary>
    /// The colour of the Gradient sky in the direction whose vertical component is
    /// <paramref name="y"/>, before the stars are mixed in: <c>g</c> of REQ-SCN-050, in sRGB
    /// and <b>not</b> saturated.
    /// </summary>
    /// <param name="m">The universal NES background colour, sRGB 0–1 per channel.</param>
    /// <param name="y">Vertical component of the unit view direction.</param>
    public static (float R, float G, float B) Sky(Rgb24 m, float y)
    {
        static float Channel(float value, float y)
        {
            float up = MathF.Pow(MathF.Max(y, 0f), 2.5f);
            float down = MathF.Pow(MathF.Max(-y, 0f), 2.5f);
            float c = value + (((ZenithFactor * value) - value) * up);
            return c + (((NadirFactor * value) - c) * down);
        }

        return (
            Channel(m.R / 255f, y),
            Channel(m.G / 255f, y),
            Channel(m.B / 255f, y));
    }
}
