using System.Globalization;
using Zax.Engine3D;

namespace Zax.Scripting;

/// <summary>
/// A colour with four components in <c>[0, 1]</c> (SPEC-05 §4.3, type <c>Color</c>). Equality
/// is exact on all four components, as the type table requires.
/// </summary>
/// <param name="R">Red.</param>
/// <param name="G">Green.</param>
/// <param name="B">Blue.</param>
/// <param name="A">Alpha.</param>
public readonly record struct LuaColor(float R, float G, float B, float A)
{
    /// <summary>Opaque black.</summary>
    public static LuaColor Black => new(0f, 0f, 0f, 1f);

    /// <summary>Opaque white.</summary>
    public static LuaColor White => new(1f, 1f, 1f, 1f);

    /// <summary>The opaque colour of an 8-bit sRGB triple.</summary>
    public static LuaColor FromBytes(byte r, byte g, byte b)
        => new(r / 255f, g / 255f, b / 255f, 1f);

    /// <summary>The 8-bit sRGB triple of this colour, each component clamped and rounded.</summary>
    public Rgb24 ToRgb24() => new(Component(R), Component(G), Component(B));

    private static byte Component(float value)
        => (byte)Math.Clamp((int)MathF.Round(value * 255f), 0, 255);
}

/// <summary>
/// A three-component integer vector (SPEC-05 §4.3, type <c>IntVector3</c>).
/// </summary>
/// <param name="X">First component.</param>
/// <param name="Y">Second component.</param>
/// <param name="Z">Third component.</param>
public readonly record struct LuaIntVector3(int X, int Y, int Z);

/// <summary>
/// The state of one tracked VR device (SPEC-05 §4.2, type <c>XRState</c>). Every field is zero
/// on desktop (REQ-LUA-091).
/// </summary>
/// <param name="Position">Position.</param>
/// <param name="Velocity">Velocity.</param>
/// <param name="Acceleration">Acceleration.</param>
/// <param name="Rotation">Rotation in degrees.</param>
public readonly record struct LuaXrState(Vec3 Position, Vec3 Velocity, Vec3 Acceleration, Vec3 Rotation);

/// <summary>
/// How the runtime turns numbers into text, which has to match the dialect of the original:
/// a float whose value is integral is written without a decimal part, anything else with
/// <c>%.14g</c> (REQ-LUA-052, REQ-LUA-101 point 1).
/// </summary>
public static class LuaNumberFormat
{
    /// <summary>The text <c>print</c> and <c>tostring</c> produce for a number.</summary>
    public static string ToText(double value)
    {
        if (double.IsNaN(value))
        {
            return double.IsNegative(value) ? "-nan" : "nan";
        }

        if (double.IsPositiveInfinity(value))
        {
            return "inf";
        }

        if (double.IsNegativeInfinity(value))
        {
            return "-inf";
        }

        // An integral value that a 64-bit integer can hold is written without a decimal part,
        // as the single numeric type of the original did.
        if (value == Math.Floor(value) && Math.Abs(value) < 9.2233720368547758E18)
        {
            return ((long)value).ToString(CultureInfo.InvariantCulture);
        }

        string text = value.ToString("G14", CultureInfo.InvariantCulture);

        // C's %g writes "1e+20"; .NET's G writes "1E+20".
        int exponent = text.IndexOf('E', StringComparison.Ordinal);
        return exponent < 0 ? text : string.Concat(text.AsSpan(0, exponent), "e", text.AsSpan(exponent + 1));
    }

    /// <summary>
    /// Truncation towards zero, the conversion a numeric Lua value undergoes on its way to an
    /// integer parameter (REQ-LUA-040).
    /// </summary>
    public static long Truncate(double value)
    {
        if (double.IsNaN(value))
        {
            return 0;
        }

        double truncated = Math.Truncate(value);
        if (truncated >= 9.2233720368547758E18)
        {
            return long.MaxValue;
        }

        if (truncated <= -9.2233720368547758E18)
        {
            return long.MinValue;
        }

        return (long)truncated;
    }

    /// <summary>Truncation towards zero, then reduction modulo 256 (REQ-LUA-040).</summary>
    public static byte ToByte(double value) => (byte)(Truncate(value) & 0xFF);

    /// <summary>Truncation towards zero, then reduction modulo 65 536 (REQ-LUA-040).</summary>
    public static ushort ToAddress(double value) => (ushort)(Truncate(value) & 0xFFFF);

    /// <summary>Truncation towards zero, saturated to the range of <see cref="int"/>.</summary>
    public static int ToInt32(double value) => (int)Math.Clamp(Truncate(value), int.MinValue, int.MaxValue);
}
