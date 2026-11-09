namespace Zax.Profiles.Hash;

/// <summary>
/// The default texture of a pattern — the pattern image itself, one indexed texel per tile pixel
/// (SPEC-03 REQ-PROF-035) — and the exclusive-or reconstruction a « 3dh » variant needs
/// (AMD-062 REQ-PROF-083).
/// </summary>
/// <remarks>
/// A cell that is empty, or whose tile is still pending, contributes zeros (REQ-PROF-081). The
/// conversion engine has its own, richer path to the same texels through the image registry;
/// this one works straight from the profile model, because reconstructing a variant's texture
/// is part of reading the profile and must not depend on the engine.
/// </remarks>
public static class PatternDefaultTexture
{
    /// <summary>Number of cells across, taken from the grid or from <c>tSize</c>.</summary>
    public static int CellsX(ProfilePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        return pattern.Grid?.Width ?? pattern.TileSize?.X ?? 0;
    }

    /// <summary>Number of cells down, taken from the grid or from <c>tSize</c>.</summary>
    public static int CellsY(ProfilePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        return pattern.Grid?.Height ?? pattern.TileSize?.Y ?? 0;
    }

    /// <summary>
    /// The texels of the default texture, <c>8W × 8H</c> values of 0 to 3, row by row.
    /// </summary>
    public static byte[] BuildTexels(ProfilePattern pattern)
    {
        int width = CellsX(pattern) * 8;
        int height = CellsY(pattern) * 8;
        byte[] texels = new byte[width * height];
        PatternGrid? grid = pattern.Grid;
        if (grid is null)
        {
            return texels;
        }

        for (int y = 0; y < height; y++)
        {
            for (int x = 0; x < width; x++)
            {
                ProfileTile? tile = grid[x >> 3, y >> 3];
                texels[x + (y * width)] = tile is null ? (byte)0 : (byte)tile.ColorAt(x & 7, y & 7);
            }
        }

        return texels;
    }

    /// <summary>
    /// The default texture of a pattern as a stored texture of format 0, which is the <c>B</c>
    /// of REQ-PROF-083. Its <c>extra</c> plane is as long as its <c>data</c>.
    /// </summary>
    public static VariantTexture Build(ProfilePattern pattern)
    {
        byte[] texels = BuildTexels(pattern);
        int words = (texels.Length + 15) / 16;
        int[] data = new int[words];
        int[] extra = new int[words];
        for (int n = 0; n < texels.Length; n++)
        {
            int texel = texels[n];
            data[n / 16] |= (texel & 3) << (2 * (n % 16));
            extra[n / 16] |= ((texel >> 2) & 1) << (n % 16);
        }

        return new VariantTexture(
            CellsX(pattern) * 8,
            CellsY(pattern) * 8,
            new TexturePixels(TextureFormat.Indexed, data, extra));
    }

    /// <summary>
    /// Reads texel <paramref name="n"/> of an indexed payload (SPEC-03 REQ-PROF-035): bits
    /// <c>2(n mod 16)</c> and <c>2(n mod 16) + 1</c> of <c>data[n / 16]</c>, plus bit
    /// <c>n mod 16</c> of <c>extra[n / 16]</c> as bit 2.
    /// </summary>
    public static int ReadIndexedTexel(TexturePixels pixels, int n)
    {
        ArgumentNullException.ThrowIfNull(pixels);
        ArgumentOutOfRangeException.ThrowIfNegative(n);

        int word = n / 16;
        int shift = 2 * (n % 16);
        int low = word < pixels.Data.Count ? (pixels.Data[word] >> shift) & 3 : 0;
        int high = word < pixels.Extra.Count ? (pixels.Extra[word] >> (n % 16)) & 1 : 0;
        return low | (high << 2);
    }

    /// <summary>
    /// Undoes the exclusive or of REQ-PROF-083: the result takes the dimensions and the format
    /// of <paramref name="stored"/> (field 24) and each word is combined with the word of the
    /// same index of <paramref name="defaultTexture"/>, array against array, whatever the
    /// formats and the sizes.
    /// </summary>
    public static VariantTexture UndoXor(VariantTexture stored, VariantTexture defaultTexture)
    {
        ArgumentNullException.ThrowIfNull(stored);
        ArgumentNullException.ThrowIfNull(defaultTexture);

        IList<int> storedData = stored.Pixels?.Data ?? [];
        IList<int> storedExtra = stored.Pixels?.Extra ?? [];
        IList<int> baseData = defaultTexture.Pixels?.Data ?? [];
        IList<int> baseExtra = defaultTexture.Pixels?.Extra ?? [];

        int[] data = new int[storedData.Count];
        int[] extra = new int[storedData.Count];
        for (int i = 0; i < storedData.Count; i++)
        {
            int baseWord = i < baseData.Count ? baseData[i] : 0;
            data[i] = storedData[i] ^ baseWord;

            // An absent element of X.extra counts as zero; the guard is on the length of
            // B.data, which is also the length of B.extra for a format 0 texture.
            int storedExtraWord = i < storedExtra.Count ? storedExtra[i] : 0;
            int baseExtraWord = i < baseData.Count && i < baseExtra.Count ? baseExtra[i] : 0;
            extra[i] = (short)(storedExtraWord ^ baseExtraWord);
        }

        TextureFormat format = stored.Pixels?.Format ?? TextureFormat.Indexed;
        return new VariantTexture(stored.Width, stored.Height, new TexturePixels(format, data, extra));
    }

    /// <summary>
    /// True when <paramref name="stored"/> has the same dimensions and format as the default
    /// texture, which is the condition REQ-PROF-083 needs before it undoes the exclusive or of
    /// a pattern that still has a pending tile.
    /// </summary>
    public static bool MatchesDefaultShape(VariantTexture stored, VariantTexture defaultTexture)
    {
        ArgumentNullException.ThrowIfNull(stored);
        ArgumentNullException.ThrowIfNull(defaultTexture);
        return stored.Width == defaultTexture.Width
            && stored.Height == defaultTexture.Height
            && (stored.Pixels?.Format ?? TextureFormat.Indexed)
                == (defaultTexture.Pixels?.Format ?? TextureFormat.Indexed);
    }
}
