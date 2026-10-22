using Zax.Render.Assets;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// Uploads the PNG textures of <c>assets-neutral/textures</c> — the television materials of
/// SPEC-06 REQ-SCN-054 — and keeps them resident for as long as the renderer lives.
/// </summary>
/// <remarks>
/// These are ordinary art assets, not NES data: they are sampled with bilinear filtering and
/// repeat, unlike the tile atlases of REQ-E3D-563.
/// </remarks>
internal sealed class ImageTextureCache : IDisposable
{
    private readonly GL _gl;
    private readonly ISceneAssets? _assets;
    private readonly Dictionary<string, uint> _handles = [];
    private readonly Dictionary<string, ImageData> _provided = [];

    private uint _white;

    /// <summary>Creates the cache for the current context.</summary>
    public ImageTextureCache(GL gl, ISceneAssets? assets)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        _assets = assets;
    }

    /// <summary>Number of textures resident on the GPU.</summary>
    public int Count => _handles.Count(pair => pair.Value != 0);

    /// <summary>
    /// The handle of <paramref name="name"/>, uploading it on first use, or an opaque white
    /// 1 × 1 texture when the asset source has no such file.
    /// </summary>
    public uint Get(string? name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return White();
        }

        if (!_handles.TryGetValue(name, out uint handle))
        {
            handle = Upload(name);
            _handles[name] = handle;
        }

        return handle != 0 ? handle : White();
    }

    /// <summary>Whether <paramref name="name"/> could be uploaded from the asset source.</summary>
    public bool Has(string? name)
        => !string.IsNullOrWhiteSpace(name) && Get(name) != White();

    /// <summary>
    /// Registers an image the host already holds in memory under <paramref name="name"/>: the
    /// textures the glTF television of AMD-025 carries inside its own file, which no
    /// <see cref="ISceneAssets"/> can hand out by name. Uploading waits for the first use.
    /// </summary>
    public void Register(string name, ImageData image)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(name);
        ArgumentNullException.ThrowIfNull(image);
        _provided[name] = image;
        _handles.Remove(name);
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        foreach (uint handle in _handles.Values)
        {
            if (handle != 0)
            {
                _gl.DeleteTexture(handle);
            }
        }

        _handles.Clear();
        if (_white != 0)
        {
            _gl.DeleteTexture(_white);
            _white = 0;
        }
    }

    private unsafe uint Upload(string name)
    {
        if (!_provided.TryGetValue(name, out ImageData? image))
        {
            image = _assets?.Texture(name);
        }

        if (image is null)
        {
            return 0;
        }

        uint handle = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, handle);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 1);
        fixed (byte* data = image.Pixels)
        {
            _gl.TexImage2D(
                TextureTarget.Texture2D, 0, InternalFormat.Rgba8,
                (uint)image.Width, (uint)image.Height, 0,
                PixelFormat.Rgba, PixelType.UnsignedByte, data);
        }

        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 4);

        // The television textures are up to 2048 wide on a set that covers a few hundred
        // pixels; without mip levels the cabinet grain aliases into a shimmer as the camera
        // turns.
        _gl.GenerateMipmap(TextureTarget.Texture2D);
        _gl.TexParameter(
            TextureTarget.Texture2D,
            TextureParameterName.TextureMinFilter,
            (int)GLEnum.LinearMipmapLinear);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Linear);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.Repeat);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.Repeat);
        _gl.BindTexture(TextureTarget.Texture2D, 0);
        return handle;
    }

    private unsafe uint White()
    {
        if (_white != 0)
        {
            return _white;
        }

        _white = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, _white);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 1);
        Span<byte> pixel = [255, 255, 255, 255];
        fixed (byte* data = pixel)
        {
            _gl.TexImage2D(
                TextureTarget.Texture2D, 0, InternalFormat.Rgba8, 1, 1, 0,
                PixelFormat.Rgba, PixelType.UnsignedByte, data);
        }

        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 4);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Nearest);
        _gl.BindTexture(TextureTarget.Texture2D, 0);
        return _white;
    }
}
