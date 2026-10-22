// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>
/// The default texture of a pattern: the pattern image itself, <c>8W × 8H</c> indexed texels
/// taken from the colour of each tile pixel (SPEC-03 REQ-PROF-035, SPEC-04B REQ-E3D-426).
/// </summary>
/// <remarks>
/// It belongs here because the engine owns the tile images and the palettes (DEC-04B-Q1). The
/// profile writer needs it for the deferred rule of REQ-PROF-052: a variant whose texture is
/// identical to the default texture is written as the null marker.
/// </remarks>
public static class DefaultTexture
{
    /// <summary>
    /// Builds the default texture of a pattern: one texel per pixel of the pattern image, the
    /// colour index 0 to 3 of the tile of the cell, and 0 where the cell is empty.
    /// </summary>
    public static byte[] Build(EnginePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        int width = pattern.Width * 8;
        int height = pattern.Height * 8;
        byte[] texels = new byte[width * height];
        for (int y = 0; y < height; y++)
        {
            for (int x = 0; x < width; x++)
            {
                TileImage? image = pattern.ImageAt(x >> 3, y >> 3);
                texels[x + (y * width)] = image is null ? (byte)0 : (byte)image.ColorAt(x & 7, y & 7);
            }
        }

        return texels;
    }

    /// <summary>
    /// Builds the default texture of a profile pattern, without going through the registry.
    /// </summary>
    public static byte[] Build(ProfilePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        PatternGrid? grid = pattern.Grid;
        int cellsX = grid?.Width ?? pattern.TileSize?.X ?? 0;
        int cellsY = grid?.Height ?? pattern.TileSize?.Y ?? 0;
        int width = cellsX * 8;
        int height = cellsY * 8;
        byte[] texels = new byte[width * height];
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
    /// Whether <paramref name="texture"/> holds exactly the default texture of
    /// <paramref name="pattern"/>, which is what REQ-PROF-052 replaces by the null marker. A
    /// null texture, or the null marker itself, already means "use the default", so it answers
    /// true as well.
    /// </summary>
    public static bool IsDefaultTexture(VariantTexture? texture, ProfilePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        if (texture is null || texture.IsNullMarker)
        {
            return true;
        }

        PatternGrid? grid = pattern.Grid;
        int cellsX = grid?.Width ?? pattern.TileSize?.X ?? 0;
        int cellsY = grid?.Height ?? pattern.TileSize?.Y ?? 0;
        if (texture.Width != cellsX * 8 || texture.Height != cellsY * 8)
        {
            return false;
        }

        if (texture.Pixels is not { Format: TextureFormat.Indexed } pixels)
        {
            return false;
        }

        byte[] expected = Build(pattern);
        for (int n = 0; n < expected.Length; n++)
        {
            if (ReadIndexedTexel(pixels, n) != expected[n])
            {
                return false;
            }
        }

        return true;
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
}
