using Zax.Engine3D;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// The 256 × 240 pixel frame of the core, uploaded as an RGBA texture for the 2D mode of
/// SPEC-06 REQ-SCN-055 and AMD-009 (Q-022).
/// </summary>
/// <remarks>
/// The core hands out one NES colour index per pixel (SPEC-02 REQ-CORE-048); they are resolved
/// here through the fixed 64-colour table of SPEC-04 §4.1, which is also the table the tiles
/// use. The texture is a direct-colour one for the scene shader, sampled nearest without
/// mipmap like every other NES texture (REQ-E3D-563).
/// </remarks>
internal sealed class NesPictureTexture : IDisposable
{
    /// <summary>Width of the pixel frame.</summary>
    public const int Width = 256;

    /// <summary>Height of the pixel frame.</summary>
    public const int Height = 240;

    private readonly GL _gl;
    private readonly byte[] _pixels = new byte[Width * Height * 4];

    private uint _handle;
    private bool _uploaded;

    /// <summary>Creates the texture for the current context.</summary>
    public NesPictureTexture(GL gl)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
    }

    /// <summary>
    /// The quad of the NES plane: <c>x ∈ [0, 256]</c>, <c>y ∈ [8, 248]</c>, <c>z = 0</c>, facing
    /// the default camera, with texture coordinates in texels so that NES pixel
    /// <c>(px, py)</c> lands on <c>[px, px + 1] × [247 − py, 248 − py]</c> (REQ-E3D-410).
    /// </summary>
    public static TileMesh Quad { get; } = BuildQuad();

    /// <summary>
    /// Uploads <paramref name="indices"/> and returns the texture name, or 0 when the frame has
    /// the wrong size.
    /// </summary>
    /// <param name="indices">256 × 240 NES colour indices, row 0 first.</param>
    public unsafe uint Upload(ReadOnlySpan<byte> indices)
    {
        if (indices.Length < Width * Height)
        {
            return 0;
        }

        for (int i = 0; i < Width * Height; i++)
        {
            Rgb24 color = NesPalette.Color(indices[i]);
            int at = i * 4;
            _pixels[at + 0] = color.R;
            _pixels[at + 1] = color.G;
            _pixels[at + 2] = color.B;
            _pixels[at + 3] = 255;
        }

        if (_handle == 0)
        {
            _handle = _gl.GenTexture();
        }

        _gl.BindTexture(TextureTarget.Texture2D, _handle);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 1);
        fixed (byte* data = _pixels)
        {
            if (_uploaded)
            {
                _gl.TexSubImage2D(
                    TextureTarget.Texture2D, 0, 0, 0, Width, Height,
                    PixelFormat.Rgba, PixelType.UnsignedByte, data);
            }
            else
            {
                _gl.TexImage2D(
                    TextureTarget.Texture2D, 0, InternalFormat.Rgba8, Width, Height, 0,
                    PixelFormat.Rgba, PixelType.UnsignedByte, data);
                _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Nearest);
                _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Nearest);
                _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
                _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
                _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureBaseLevel, 0);
                _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMaxLevel, 0);
                _uploaded = true;
            }
        }

        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 4);
        _gl.BindTexture(TextureTarget.Texture2D, 0);
        return _handle;
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_handle != 0)
        {
            _gl.DeleteTexture(_handle);
            _handle = 0;
            _uploaded = false;
        }
    }

    private static TileMesh BuildQuad()
    {
        const float top = SceneConstants.SceneHeight;
        const float bottom = SceneConstants.SceneHeight - Height;
        Vec3 normal = new(0f, 0f, -1f);

        // Same winding as the −Z face of the unit box (REQ-E3D-505): bottom-left, top-left,
        // top-right, bottom-right.
        MeshVertex[] vertices =
        [
            new MeshVertex(new Vec3(0f, bottom, 0f), normal, new Vec2(0f, Height)),
            new MeshVertex(new Vec3(0f, top, 0f), normal, new Vec2(0f, 0f)),
            new MeshVertex(new Vec3(Width, top, 0f), normal, new Vec2(Width, 0f)),
            new MeshVertex(new Vec3(Width, bottom, 0f), normal, new Vec2(Width, Height)),
        ];

        return new TileMesh(vertices, [0, 1, 2, 0, 2, 3]);
    }
}
