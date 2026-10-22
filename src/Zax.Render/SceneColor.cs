using System.Numerics;
using Zax.Engine3D;

namespace Zax.Render;

/// <summary>
/// The sRGB ↔ linear conversions of SPEC-06 §2 and REQ-SCN-035, and the scene constants that
/// depend on them.
/// </summary>
/// <remarks>
/// Shading is computed in linear space and re-encoded to sRGB for display (REQ-SCN-028). The
/// same curve is applied to the light intensity (DEC-06-Q2). The index and alpha channels are
/// never converted (REQ-SCN-035).
/// </remarks>
public static class SceneColor
{
    /// <summary>The ambient colour, sRGB (118, 118, 118) (REQ-SCN-027).</summary>
    public static Rgb24 Ambient { get; } = new(118, 118, 118);

    /// <summary>The ambient term in linear space, 0.1812 (REQ-SCN-027).</summary>
    public static float AmbientLinear { get; } = ToLinear(118f / 255f);

    /// <summary>The direct-light factor inside a shadow: 1 − 0.8 (REQ-SCN-031).</summary>
    public const float ShadowFactor = 0.2f;

    /// <summary>The standard sRGB → linear transfer function (IEC 61966-2-1).</summary>
    public static float ToLinear(float value)
        => value <= 0.04045f ? value / 12.92f : MathF.Pow((value + 0.055f) / 1.055f, 2.4f);

    /// <summary>The standard linear → sRGB transfer function.</summary>
    public static float ToSrgb(float value)
        => value <= 0.0031308f ? value * 12.92f : (1.055f * MathF.Pow(value, 1f / 2.4f)) - 0.055f;

    /// <summary>The linear colour of an sRGB colour given as bytes.</summary>
    public static Vector3 ToLinear(Rgb24 color) => new(
        ToLinear(color.R / 255f),
        ToLinear(color.G / 255f),
        ToLinear(color.B / 255f));

    /// <summary>The sRGB bytes of a linear colour, saturated to 1 without tone mapping.</summary>
    public static Rgb24 ToSrgbBytes(Vector3 linear) => new(
        Quantise(ToSrgb(Math.Clamp(linear.X, 0f, 1f))),
        Quantise(ToSrgb(Math.Clamp(linear.Y, 0f, 1f))),
        Quantise(ToSrgb(Math.Clamp(linear.Z, 0f, 1f))));

    /// <summary>The sRGB components of an sRGB colour, in the 0..1 range.</summary>
    public static Vector3 ToVector(Rgb24 color) => new(color.R / 255f, color.G / 255f, color.B / 255f);

    /// <summary>
    /// The diffuse Lambert shading of REQ-SCN-028:
    /// <c>albedo_lin × (ambient_lin + max(0, N·L) × light_lin × S)</c>, encoded back to sRGB.
    /// </summary>
    /// <param name="albedo">Albedo in sRGB.</param>
    /// <param name="normal">Mesh normal, scene frame (REQ-SCN-028: unchanged by the deformation).</param>
    /// <param name="lightDirection">Propagation direction of the rays (REQ-SCN-025).</param>
    /// <param name="lightColor">Light colour in sRGB.</param>
    /// <param name="intensity">Light intensity, 0.50 to 1.50 (REQ-SCN-026).</param>
    /// <param name="shadow">The shadow factor <c>S</c>: 1 in the light, 0.2 in shadow.</param>
    public static Rgb24 Shade(
        Rgb24 albedo,
        Vector3 normal,
        Vector3 lightDirection,
        Rgb24 lightColor,
        float intensity,
        float shadow)
    {
        Vector3 light = ToLinear(lightColor) * ToLinear(intensity);
        float ndl = 0f;
        if (normal.LengthSquared() > 0f && lightDirection.LengthSquared() > 0f)
        {
            ndl = MathF.Max(0f, Vector3.Dot(Vector3.Normalize(normal), Vector3.Normalize(-lightDirection)));
        }

        Vector3 factor = new Vector3(AmbientLinear) + (ndl * shadow * light);
        return ToSrgbBytes(ToLinear(albedo) * factor);
    }

    private static byte Quantise(float value) => (byte)Math.Clamp(MathF.Round(value * 255f), 0f, 255f);
}
