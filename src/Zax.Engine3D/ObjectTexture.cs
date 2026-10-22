// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>
/// The texture of one 3D object, resolved once at build time: the pattern image by default, or
/// the custom texture the profile carries (SPEC-04B REQ-E3D-426, §2).
/// </summary>
/// <remarks>
/// <para>
/// Two formats exist (SPEC-03 REQ-PROF-035). An <see cref="TextureFormat.Indexed"/> texture
/// stores a texel value of 0 to 7 — two colour bits plus one extra bit; only the two colour
/// bits reach the palette (REQ-E3D-560), the extra bit serves transparency (REQ-E3D-424) and
/// the multicolour merge (REQ-E3D-503). A <see cref="TextureFormat.Direct"/> texture stores a
/// 24-bit colour per texel, used as is, with the integer value 0 standing for the universal
/// background colour <b>as it was when the texture was loaded</b> (REQ-E3D-562).
/// </para>
/// <para>
/// Every read is taken modulo the texture size (§2, DEC-04B-Q3): a texture smaller than the
/// pattern repeats over it, which four built-in profiles rely on.
/// </para>
/// </remarks>
public sealed class ObjectTexture
{
    private readonly int[] _texels;
    private readonly Rgb24[]? _colors;

    private ObjectTexture(int width, int height, TextureFormat format, int[] texels, Rgb24[]? colors)
    {
        Width = width;
        Height = height;
        Format = format;
        _texels = texels;
        _colors = colors;
    }

    /// <summary>Width in texels (<c>tw_tex</c>); at least 1.</summary>
    public int Width { get; }

    /// <summary>Height in texels (<c>th_tex</c>); at least 1.</summary>
    public int Height { get; }

    /// <summary>Texel format.</summary>
    public TextureFormat Format { get; }

    /// <summary>True when the texture carries 24-bit colours instead of palette indices.</summary>
    public bool IsDirect => Format == TextureFormat.Direct;

    /// <summary>
    /// The texel value <c>t(x, y)</c> at (<paramref name="x"/>, <paramref name="y"/>), read
    /// modulo the texture size: 0 to 7 for an indexed texture, the stored integer for a direct
    /// one (§2).
    /// </summary>
    public int Texel(int x, int y)
    {
        int tx = Wrap(x, Width);
        int ty = Wrap(y, Height);
        return _texels[tx + (ty * Width)];
    }

    /// <summary>
    /// The colour of a <b>direct</b> texel, the integer value 0 resolved to the universal
    /// background colour captured when the texture was built (REQ-E3D-562). Returns black for
    /// an indexed texture, whose colours the renderer resolves from the frame palette
    /// (REQ-E3D-560, REQ-E3D-565).
    /// </summary>
    public Rgb24 Color(int x, int y)
    {
        if (_colors is null)
        {
            return default;
        }

        int tx = Wrap(x, Width);
        int ty = Wrap(y, Height);
        return _colors[tx + (ty * Width)];
    }

    /// <summary>
    /// The palette column <c>t mod 4</c> an indexed texel addresses (REQ-E3D-560). The extra
    /// bit does not reach the colour.
    /// </summary>
    public static int PaletteColumn(int texel) => texel & 3;

    /// <summary>
    /// The transparent colour <c>e</c> of REQ-E3D-424: <c>emptyColor</c> when it is positive or
    /// zero, <c>emptyColor + 5</c> when it is negative.
    /// </summary>
    public static int EmptyColor(int emptyColor) => emptyColor >= 0 ? emptyColor : emptyColor + 5;

    /// <summary>The pattern image as an indexed texture: <c>8·tw × 8·th</c> texels of 0 to 3.</summary>
    public static ObjectTexture FromPattern(EnginePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        int width = Math.Max(1, pattern.Width * 8);
        int height = Math.Max(1, pattern.Height * 8);
        int[] texels = new int[width * height];
        for (int y = 0; y < height; y++)
        {
            for (int x = 0; x < width; x++)
            {
                TileImage? image = pattern.ImageAt(x >> 3, y >> 3);
                texels[x + (y * width)] = image is null ? 0 : image.ColorAt(x & 7, y & 7);
            }
        }

        return new ObjectTexture(width, height, TextureFormat.Indexed, texels, null);
    }

    /// <summary>
    /// The texture of a 3D object: its custom texture when the profile gives one of non-zero
    /// size, the pattern image otherwise (REQ-E3D-426).
    /// </summary>
    /// <param name="pattern">The pattern the object belongs to.</param>
    /// <param name="texture">The custom texture, or null.</param>
    /// <param name="background">
    /// The universal background colour used for the value-0 texels of a direct texture, read
    /// once here (REQ-E3D-562).
    /// </param>
    public static ObjectTexture For(EnginePattern pattern, VariantTexture? texture, Rgb24 background)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        if (texture is null
            || texture.IsNullMarker
            || texture.Width <= 0
            || texture.Height <= 0
            || texture.Pixels is not { } pixels)
        {
            return FromPattern(pattern);
        }

        int width = texture.Width;
        int height = texture.Height;
        int count = width * height;
        int[] texels = new int[count];

        if (pixels.Format == TextureFormat.Direct)
        {
            Rgb24[] colors = new Rgb24[count];
            for (int n = 0; n < count; n++)
            {
                int value = n < pixels.Data.Count ? pixels.Data[n] : 0;
                texels[n] = value;
                colors[n] = value == 0 ? background : Rgb24.FromPacked(value & 0xFFFFFF);
            }

            return new ObjectTexture(width, height, TextureFormat.Direct, texels, colors);
        }

        for (int n = 0; n < count; n++)
        {
            texels[n] = DefaultTexture.ReadIndexedTexel(pixels, n);
        }

        return new ObjectTexture(width, height, TextureFormat.Indexed, texels, null);
    }

    private static int Wrap(int value, int size)
    {
        int result = value % size;
        return result < 0 ? result + size : result;
    }
}
