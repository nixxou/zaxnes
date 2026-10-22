namespace Zax.Profiles;

/// <summary>
/// Pixel payload of a variant texture (SPEC-03 §4.5, <c>TexPayload</c>).
/// </summary>
/// <remarks>
/// <para>
/// For <see cref="TextureFormat.Indexed"/>, <see cref="Data"/> holds
/// <c>ceil(width × height / 16)</c> integers: bits <c>2(n mod 16)</c> and
/// <c>2(n mod 16) + 1</c> of <c>Data[n / 16]</c> are bits 0 and 1 of the index of pixel
/// <c>n</c>, and bit <c>n mod 16</c> of <c>Extra[n / 16]</c> is its bit 2, giving an index of
/// 0 to 7. An absent <see cref="Extra"/> means all its bits are zero.
/// </para>
/// <para>
/// For <see cref="TextureFormat.Direct"/>, <see cref="Data"/> holds <c>width × height</c>
/// packed ARGB integers, and the value 0 means "the background colour of the palette".
/// </para>
/// </remarks>
public sealed class TexturePixels
{
    /// <summary>Creates a payload.</summary>
    public TexturePixels(TextureFormat format, IEnumerable<int>? data = null, IEnumerable<int>? extra = null)
    {
        Format = format;
        Data = data is null ? [] : [.. data];
        Extra = extra is null ? [] : [.. extra];
    }

    /// <summary>Pixel format.</summary>
    public TextureFormat Format { get; set; }

    /// <summary>Packed pixel words; see the type remarks for the layout.</summary>
    public IList<int> Data { get; }

    /// <summary>
    /// Third bit plane of an indexed texture, one word per sixteen pixels. Empty means "all
    /// zero", which the canonical writer omits (REQ-PROF-051).
    /// </summary>
    public IList<int> Extra { get; }

    /// <summary>True when <see cref="Extra"/> carries no set bit, so the writer omits it.</summary>
    public bool ExtraIsEmpty
    {
        get
        {
            foreach (int word in Extra)
            {
                if (word != 0)
                {
                    return false;
                }
            }

            return true;
        }
    }

    /// <summary>
    /// Number of payload words a texture of <paramref name="width"/> × <paramref name="height"/>
    /// pixels needs in this format (REQ-PROF-035).
    /// </summary>
    public int ExpectedWordCount(int width, int height)
    {
        long pixels = (long)width * height;
        if (pixels <= 0)
        {
            return 0;
        }

        return Format == TextureFormat.Direct ? (int)pixels : (int)((pixels + 15) / 16);
    }
}

/// <summary>
/// Texture of a 3D variant (SPEC-03 REQ-PROF-035, §4.5, <c>TexRect</c>): a size and a pixel
/// payload.
/// </summary>
/// <remarks>
/// <para>
/// Three cases matter. The variant has <b>no texture at all</b> when the field is absent, which
/// the model shows as a null texture (the patterns of the editor's memorised slots are in that
/// case). It uses the <b>default texture computed from its tiles</b> when the stored value is
/// the <see cref="IsNullMarker">null marker</see> — width and height zero, format 0, no data.
/// Otherwise the texture is explicit and pixel (x, y) is at index <c>y × Width + x</c>, y = 0
/// at the top.
/// </para>
/// <para>
/// On the wire the payload is written <b>before</b> the width and the height, the only
/// exception to the ascending field order of the canonical writer (REQ-PROF-013); that is a
/// wire concern and does not show here.
/// </para>
/// </remarks>
public sealed class VariantTexture
{
    /// <summary>Creates a texture.</summary>
    public VariantTexture(int width, int height, TexturePixels? pixels)
    {
        Width = width;
        Height = height;
        Pixels = pixels;
    }

    /// <summary>Width in pixels.</summary>
    public int Width { get; set; }

    /// <summary>Height in pixels.</summary>
    public int Height { get; set; }

    /// <summary>Pixel payload, or <see langword="null"/> when the sub-message is absent.</summary>
    public TexturePixels? Pixels { get; set; }

    /// <summary>
    /// True when this is the null marker: the variant takes the default texture built from the
    /// pattern's tiles (REQ-PROF-035).
    /// </summary>
    public bool IsNullMarker
        => Width == 0 && Height == 0
        && (Pixels is null || (Pixels.Format == TextureFormat.Indexed && Pixels.Data.Count == 0));

    /// <summary>Builds the null marker written for a variant that uses the default texture.</summary>
    public static VariantTexture NullMarker() => new(0, 0, new TexturePixels(TextureFormat.Indexed));

    /// <summary>
    /// True when the payload length matches <see cref="Width"/> × <see cref="Height"/> for its
    /// format, which holds for all 13 681 explicit textures of the built-in profiles.
    /// </summary>
    public bool HasConsistentLength()
    {
        if (IsNullMarker)
        {
            return true;
        }

        if (Pixels is null)
        {
            return Width == 0 && Height == 0;
        }

        int expected = Pixels.ExpectedWordCount(Width, Height);
        if (Pixels.Data.Count != expected)
        {
            return false;
        }

        return Pixels.Extra.Count is 0 || Pixels.Extra.Count == expected;
    }
}
