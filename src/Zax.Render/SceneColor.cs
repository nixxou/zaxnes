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

    /// <summary>
    /// The perceptual roughness <c>ρ</c> of the backdrop material, 0.99 — almost matt
    /// (REQ-SCN-075, AMD-049).
    /// </summary>
    public const float BackdropRoughness = 0.99f;

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

    /// <summary>
    /// The backdrop shading of REQ-SCN-075 (AMD-049), in linear space and <b>before</b> the
    /// clamp: <c>c · 0.96 · (Amb + s · E · Fd · nl) + s · E · Spec · F</c>. It replaces, for the
    /// backdrop only, the diffuse term REQ-SCN-048 used to inherit from REQ-SCN-028; the tiles
    /// and the HUD keep <see cref="Shade"/>.
    /// </summary>
    /// <param name="albedoLinear">
    /// The albedo <c>c</c>, <c>Backdrop.Color</c> already converted to linear; its alpha has no
    /// part here, the box is opaque (REQ-SCN-058).
    /// </param>
    /// <param name="normal">
    /// The normal <c>N</c> of the face, transported by the curvature when it is active
    /// (AMD-043 REQ-SCN-068). Normalised here.
    /// </param>
    /// <param name="view">The vector <c>V</c> from the point towards the eye. Normalised here.</param>
    /// <param name="lightDirection">Propagation direction of the rays (REQ-SCN-025).</param>
    /// <param name="lightLinear">
    /// The illuminance <c>E</c>: the light colour multiplied by the intensity, converted to
    /// linear, the <c>lumière_lin</c> of REQ-SCN-028 (DEC-06-Q2, AMD-021 R2-015).
    /// </param>
    /// <param name="shadow">The shadow factor <c>s</c>: 1 in the light, 0.2 in shadow.</param>
    /// <remarks>
    /// Pure and platform-free, so that the two automatic vectors of the amendment can be checked
    /// without a GL context; the shader of <c>SceneShaders</c> computes the very same expression
    /// per fragment. There is no environment reflection: in shadow the specular part is reduced
    /// by the same <c>s</c> as the direct diffuse part.
    /// </remarks>
    public static Vector3 ShadeBackdropLinear(
        Vector3 albedoLinear,
        Vector3 normal,
        Vector3 view,
        Vector3 lightDirection,
        Vector3 lightLinear,
        float shadow)
    {
        // A degenerate vector falls back on a defined colour instead of a NaN: V = N (the face
        // seen head-on) and L = −N (the ambient alone, as REQ-SCN-028 does for a zero direction).
        Vector3 n = Unit(normal, -Vector3.UnitZ);
        Vector3 v = Unit(view, n);
        Vector3 l = Unit(-lightDirection, -n);

        // REQ-SCN-075: H = (L + V) normalised, and H = N when L + V = 0.
        Vector3 sum = l + v;
        Vector3 h = sum.LengthSquared() > 0f ? Vector3.Normalize(sum) : n;

        float nl = MathF.Max(Vector3.Dot(n, l), 0f);
        float nv = MathF.Abs(Vector3.Dot(n, v));
        float nh = MathF.Max(Vector3.Dot(n, h), 0f);
        float lh = MathF.Max(Vector3.Dot(l, h), 0f);

        const float rough = BackdropRoughness;
        const float alpha = rough * rough;
        const float alpha2 = alpha * alpha;

        float f90 = 0.5f + (2f * lh * lh * rough);
        float fd = (1f + ((f90 - 1f) * Pow5(1f - nl)))
            * (1f + ((f90 - 1f) * Pow5(1f - nv)));

        // The 1e-5 and the clamps above only cover degenerate cases; they change no measurable
        // result at ρ = 0.99 (R2-review-AMD-049, 49-8).
        float vis = 0.5f / ((nl * ((nv * (1f - alpha)) + alpha))
            + (nv * ((nl * (1f - alpha)) + alpha)) + 1e-5f);
        float denominator = (nh * nh * (alpha2 - 1f)) + 1f;
        float d = alpha2 / (MathF.PI * denominator * denominator);
        float spec = vis * d * MathF.PI * nl;
        float f = 0.04f + (0.96f * Pow5(1f - lh));

        return (albedoLinear * 0.96f * (new Vector3(AmbientLinear) + (shadow * lightLinear * fd * nl)))
            + (shadow * lightLinear * spec * f);
    }

    /// <summary>
    /// <see cref="ShadeBackdropLinear(Vector3, Vector3, Vector3, Vector3, Vector3, float)"/>
    /// taking <c>Backdrop.Color</c>, the light colour and the intensity as the renderer holds
    /// them, saturated to 1 and encoded back to sRGB exactly as REQ-SCN-028 does, without any
    /// tone mapping (REQ-SCN-075).
    /// </summary>
    /// <param name="albedo"><c>Backdrop.Color</c> in sRGB.</param>
    /// <param name="normal">The normal <c>N</c> of the face.</param>
    /// <param name="view">The vector <c>V</c> from the point towards the eye.</param>
    /// <param name="lightDirection">Propagation direction of the rays (REQ-SCN-025).</param>
    /// <param name="lightColor">Light colour in sRGB.</param>
    /// <param name="intensity">Light intensity, 0.50 to 1.50 (REQ-SCN-026).</param>
    /// <param name="shadow">The shadow factor <c>s</c>: 1 in the light, 0.2 in shadow.</param>
    public static Rgb24 ShadeBackdrop(
        Rgb24 albedo,
        Vector3 normal,
        Vector3 view,
        Vector3 lightDirection,
        Rgb24 lightColor,
        float intensity,
        float shadow)
        => ToSrgbBytes(ShadeBackdropLinear(
            ToLinear(albedo),
            normal,
            view,
            lightDirection,
            ToLinear(lightColor) * ToLinear(intensity),
            shadow));

    /// <summary>The fifth power, written as three multiplications so the GPU agrees to the bit.</summary>
    private static float Pow5(float value)
    {
        float square = value * value;
        return square * square * value;
    }

    /// <summary><paramref name="vector"/> normalised, or <paramref name="fallback"/> when it has no length.</summary>
    private static Vector3 Unit(Vector3 vector, Vector3 fallback)
        => vector.LengthSquared() > 0f ? Vector3.Normalize(vector) : fallback;

    private static byte Quantise(float value) => (byte)Math.Clamp(MathF.Round(value * 255f), 0f, 255f);
}
