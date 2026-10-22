namespace Zax.Render.Assets;

/// <summary>
/// A decoded 8-bit RGBA image, rows stored top-down, as the PNG files of
/// <c>assets-neutral/textures</c> and <c>assets-neutral/cubemaps</c> hold them.
/// </summary>
/// <param name="Width">Width in pixels.</param>
/// <param name="Height">Height in pixels.</param>
/// <param name="Pixels">
/// <c>Width × Height × 4</c> bytes, four per pixel in R, G, B, A order, the first row first.
/// </param>
public sealed record ImageData(int Width, int Height, byte[] Pixels)
{
    /// <summary>The four channels of pixel (<paramref name="x"/>, <paramref name="y"/>).</summary>
    public (byte R, byte G, byte B, byte A) Pixel(int x, int y)
    {
        int at = (((y * Width) + x) * 4);
        return (Pixels[at], Pixels[at + 1], Pixels[at + 2], Pixels[at + 3]);
    }

    /// <summary>A one-pixel image of the given colour, used when an asset is missing.</summary>
    public static ImageData Single(byte r, byte g, byte b, byte a = 255)
        => new(1, 1, [r, g, b, a]);
}
