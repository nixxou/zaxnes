using Zax.Render.Assets;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// Uploads the skybox cubemaps of SPEC-06 REQ-SCN-050 and REQ-SCN-051 to the GPU, one face at a
/// time, and keeps them resident for as long as the renderer lives.
/// </summary>
/// <remarks>
/// <para>
/// The faces are uploaded in the OpenGL cube-map order, which matches the left-handed, Y-up
/// convention <c>assets-neutral/cubemaps/cubemaps.json</c> documents: a direction given in
/// scene coordinates can therefore be used as the sampling vector unchanged. Rows are stored
/// top-down in the files, which is also what the cube-map targets expect.
/// </para>
/// <para>
/// The notes of REQ-SCN-050 say the cubemaps hold <b>linear</b> data, so the texels are
/// uploaded as plain <c>RGBA8</c> and never passed through an sRGB transfer.
/// </para>
/// </remarks>
internal sealed class CubemapCache : IDisposable
{
    private static readonly TextureTarget[] Targets =
    [
        TextureTarget.TextureCubeMapPositiveX,
        TextureTarget.TextureCubeMapNegativeX,
        TextureTarget.TextureCubeMapPositiveY,
        TextureTarget.TextureCubeMapNegativeY,
        TextureTarget.TextureCubeMapPositiveZ,
        TextureTarget.TextureCubeMapNegativeZ,
    ];

    private readonly GL _gl;
    private readonly ISceneAssets? _assets;
    private readonly Dictionary<string, uint> _handles = [];

    private uint _fallback;

    /// <summary>Creates the cache for the current context.</summary>
    /// <param name="gl">The GL API.</param>
    /// <param name="assets">Where the faces come from, or null when there are none.</param>
    public CubemapCache(GL gl, ISceneAssets? assets)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        _assets = assets;
    }

    /// <summary>Number of cubemaps resident on the GPU, the fallback excluded.</summary>
    public int Count => _handles.Count(pair => pair.Value != 0);

    /// <summary>
    /// The handle of <paramref name="name"/>, uploading it on first use. When the asset source
    /// has no such cubemap, a black 1 × 1 cubemap is returned, which makes every formula of
    /// REQ-SCN-050 and REQ-SCN-051 degrade to its "no cubemap" case.
    /// </summary>
    public uint Get(string name)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(name);
        if (_handles.TryGetValue(name, out uint cached))
        {
            return cached != 0 ? cached : Fallback();
        }

        uint handle = Upload(name);
        _handles[name] = handle;
        return handle != 0 ? handle : Fallback();
    }

    /// <summary>Whether <paramref name="name"/> could be uploaded from the asset source.</summary>
    public bool Has(string name) => _handles.TryGetValue(name, out uint handle) && handle != 0;

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
        if (_fallback != 0)
        {
            _gl.DeleteTexture(_fallback);
            _fallback = 0;
        }
    }

    private unsafe uint Upload(string name)
    {
        if (_assets is null)
        {
            return 0;
        }

        ImageData?[] faces = new ImageData?[6];
        for (int i = 0; i < 6; i++)
        {
            faces[i] = _assets.CubemapFace(name, (CubeFace)i);
            if (faces[i] is null)
            {
                return 0;
            }
        }

        uint handle = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.TextureCubeMap, handle);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 1);

        for (int i = 0; i < 6; i++)
        {
            ImageData face = faces[i]!;
            fixed (byte* data = face.Pixels)
            {
                _gl.TexImage2D(
                    Targets[i], 0, InternalFormat.Rgba8,
                    (uint)face.Width, (uint)face.Height, 0,
                    PixelFormat.Rgba, PixelType.UnsignedByte, data);
            }

            // Let the face go before the next one is decoded: the three nebula cubemaps are
            // 2048 x 2048 each and would otherwise hold a hundred megabytes at once.
            faces[i] = null;
        }

        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 4);
        SetSampling(TextureTarget.TextureCubeMap, linear: true);
        _gl.BindTexture(TextureTarget.TextureCubeMap, 0);
        return handle;
    }

    private unsafe uint Fallback()
    {
        if (_fallback != 0)
        {
            return _fallback;
        }

        _fallback = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.TextureCubeMap, _fallback);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 1);
        Span<byte> black = [0, 0, 0, 255];
        foreach (TextureTarget target in Targets)
        {
            fixed (byte* data = black)
            {
                _gl.TexImage2D(
                    target, 0, InternalFormat.Rgba8, 1, 1, 0,
                    PixelFormat.Rgba, PixelType.UnsignedByte, data);
            }
        }

        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 4);
        SetSampling(TextureTarget.TextureCubeMap, linear: true);
        _gl.BindTexture(TextureTarget.TextureCubeMap, 0);
        return _fallback;
    }

    private void SetSampling(TextureTarget target, bool linear)
    {
        int filter = (int)(linear ? GLEnum.Linear : GLEnum.Nearest);
        _gl.TexParameter(target, TextureParameterName.TextureMinFilter, filter);
        _gl.TexParameter(target, TextureParameterName.TextureMagFilter, filter);
        _gl.TexParameter(target, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(target, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(target, TextureParameterName.TextureWrapR, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(target, TextureParameterName.TextureBaseLevel, 0);
        _gl.TexParameter(target, TextureParameterName.TextureMaxLevel, 0);
    }
}
