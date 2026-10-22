namespace Zax.Nes;

/// <summary>
/// Brightness classification of the 64 NES colours, as the Zapper's photodiode sees it
/// (REQ-CORE-062).
/// </summary>
/// <remarks>
/// The RGB table itself belongs to the palette management of SPEC-04 (DEC-04B-Q1); only
/// the light/dark decision is a core concern. The threshold is very low: exactly nine
/// colours count as dark, the blacks.
/// </remarks>
public static class NesColors
{
    /// <summary>Number of colours in the NES palette.</summary>
    public const int Count = 64;

    /// <summary>
    /// The nine colour indices the Zapper reads as dark: $0D, $0E, $0F, $1E, $1F, $2E,
    /// $2F, $3E and $3F (REQ-CORE-062, SPEC-02 §5.4).
    /// </summary>
    public static ReadOnlySpan<byte> DarkColors => [0x0D, 0x0E, 0x0F, 0x1E, 0x1F, 0x2E, 0x2F, 0x3E, 0x3F];

    /// <summary>
    /// Whether <paramref name="colorIndex"/> (6 significant bits) is dark for the Zapper.
    /// </summary>
    public static bool IsDark(byte colorIndex) => (colorIndex & 0x3F) switch
    {
        0x0D or 0x0E or 0x0F or 0x1E or 0x1F or 0x2E or 0x2F or 0x3E or 0x3F => true,
        _ => false,
    };

    /// <summary>Whether <paramref name="colorIndex"/> is light for the Zapper.</summary>
    public static bool IsLight(byte colorIndex) => !IsDark(colorIndex);
}
