using Zax.Engine3D;
using Zax.Profiles;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// Uploads the object textures of SPEC-04B REQ-E3D-426 and the 64-colour frame palette of
/// REQ-E3D-561 to the GPU.
/// </summary>
/// <remarks>
/// An indexed texture becomes an <c>R8UI</c> texture whose texels keep their 0..7 value: the
/// colour is resolved at render time from the palette (REQ-E3D-565, REQ-SCN-034). A direct
/// texture becomes an <c>RGBA8</c> texture holding the sRGB colours. Sampling is nearest
/// neighbour without mipmap in both cases (REQ-E3D-563).
/// </remarks>
internal sealed class TextureCache : IDisposable
{
    /// <summary>Rendered frames an unused texture is kept for.</summary>
    public const int RetentionFrames = 600;

    private readonly GL _gl;
    private readonly Dictionary<ObjectTexture, Entry> _entries = new(ReferenceComparer.Instance);
    private readonly List<ObjectTexture> _expired = [];
    private readonly byte[] _paletteBytes = new byte[FramePalette.EntryCount * 3];

    private uint _palette;
    private uint _dummyIndexed;
    private uint _dummyColor;
    private long _frame;

    /// <summary>Creates the cache for the current context.</summary>
    public TextureCache(GL gl)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        _palette = CreatePaletteTexture();
        _dummyIndexed = CreateDummy(InternalFormat.R8ui, PixelFormat.RedInteger, PixelType.UnsignedByte);
        _dummyColor = CreateDummy(InternalFormat.Rgba8, PixelFormat.Rgba, PixelType.UnsignedByte);
    }

    /// <summary>Number of object textures resident on the GPU.</summary>
    public int Count => _entries.Count;

    /// <summary>The 64 × 1 palette texture of the current frame.</summary>
    public uint Palette => _palette;

    /// <summary>A 1 × 1 integer texture, bound to the unused indexed sampler.</summary>
    public uint DummyIndexed => _dummyIndexed;

    /// <summary>A 1 × 1 colour texture, bound to the unused colour sampler.</summary>
    public uint DummyColor => _dummyColor;

    /// <summary>Starts a rendered frame, for the retention bookkeeping.</summary>
    public void BeginFrame() => _frame++;

    /// <summary>
    /// Uploads the 64 resolved colours of the displayed frame (REQ-E3D-641). Called once per
    /// rendered frame: a palette change shows on the next frame without any mesh rebuild
    /// (REQ-E3D-565). It binds and unbinds on the active texture unit, so the caller sets up
    /// its sampler units afterwards.
    /// </summary>
    public unsafe void UploadPalette(ReadOnlySpan<Rgb24> colors)
    {
        int count = Math.Min(colors.Length, FramePalette.EntryCount);
        for (int i = 0; i < count; i++)
        {
            _paletteBytes[(i * 3) + 0] = colors[i].R;
            _paletteBytes[(i * 3) + 1] = colors[i].G;
            _paletteBytes[(i * 3) + 2] = colors[i].B;
        }

        for (int i = count; i < FramePalette.EntryCount; i++)
        {
            _paletteBytes[(i * 3) + 0] = 0;
            _paletteBytes[(i * 3) + 1] = 0;
            _paletteBytes[(i * 3) + 2] = 0;
        }

        _gl.BindTexture(TextureTarget.Texture2D, _palette);
        fixed (byte* data = _paletteBytes)
        {
            _gl.TexSubImage2D(
                TextureTarget.Texture2D, 0, 0, 0,
                FramePalette.EntryCount, 1,
                PixelFormat.Rgb, PixelType.UnsignedByte, data);
        }

        _gl.BindTexture(TextureTarget.Texture2D, 0);
    }

    /// <summary>The GPU texture of <paramref name="texture"/>, uploading it on first use.</summary>
    public uint Get(ObjectTexture texture)
    {
        ArgumentNullException.ThrowIfNull(texture);
        if (_entries.TryGetValue(texture, out Entry entry))
        {
            entry.LastUsed = _frame;
            _entries[texture] = entry;
            return entry.Handle;
        }

        Entry created = Upload(texture);
        _entries[texture] = created;
        return created.Handle;
    }

    /// <summary>Releases every texture whose last use is older than the retention window.</summary>
    public void Collect()
    {
        _expired.Clear();
        foreach ((ObjectTexture texture, Entry entry) in _entries)
        {
            if (_frame - entry.LastUsed > RetentionFrames)
            {
                _expired.Add(texture);
            }
        }

        foreach (ObjectTexture texture in _expired)
        {
            _gl.DeleteTexture(_entries[texture].Handle);
            _entries.Remove(texture);
        }

        _expired.Clear();
    }

    /// <summary>Releases every object texture, keeping the palette.</summary>
    public void Clear()
    {
        foreach (Entry entry in _entries.Values)
        {
            _gl.DeleteTexture(entry.Handle);
        }

        _entries.Clear();
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        Clear();
        if (_palette != 0)
        {
            _gl.DeleteTexture(_palette);
            _palette = 0;
        }

        if (_dummyIndexed != 0)
        {
            _gl.DeleteTexture(_dummyIndexed);
            _dummyIndexed = 0;
        }

        if (_dummyColor != 0)
        {
            _gl.DeleteTexture(_dummyColor);
            _dummyColor = 0;
        }
    }

    private unsafe Entry Upload(ObjectTexture texture)
    {
        uint handle = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, handle);

        int width = texture.Width;
        int height = texture.Height;

        if (texture.IsDirect)
        {
            byte[] pixels = new byte[width * height * 4];
            for (int y = 0; y < height; y++)
            {
                for (int x = 0; x < width; x++)
                {
                    Rgb24 color = texture.Color(x, y);
                    int at = ((y * width) + x) * 4;
                    pixels[at + 0] = color.R;
                    pixels[at + 1] = color.G;
                    pixels[at + 2] = color.B;
                    pixels[at + 3] = 255;
                }
            }

            fixed (byte* data = pixels)
            {
                _gl.TexImage2D(
                    TextureTarget.Texture2D, 0, InternalFormat.Rgba8,
                    (uint)width, (uint)height, 0,
                    PixelFormat.Rgba, PixelType.UnsignedByte, data);
            }
        }
        else
        {
            byte[] pixels = new byte[width * height];
            for (int y = 0; y < height; y++)
            {
                for (int x = 0; x < width; x++)
                {
                    pixels[(y * width) + x] = (byte)(texture.Texel(x, y) & 0xFF);
                }
            }

            _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 1);
            fixed (byte* data = pixels)
            {
                _gl.TexImage2D(
                    TextureTarget.Texture2D, 0, InternalFormat.R8ui,
                    (uint)width, (uint)height, 0,
                    PixelFormat.RedInteger, PixelType.UnsignedByte, data);
            }

            _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 4);
        }

        SetNearestClamp();
        _gl.BindTexture(TextureTarget.Texture2D, 0);

        return new Entry { Handle = handle, LastUsed = _frame };
    }

    private unsafe uint CreatePaletteTexture()
    {
        uint handle = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, handle);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 1);
        _gl.TexImage2D(
            TextureTarget.Texture2D, 0, InternalFormat.Rgb8,
            FramePalette.EntryCount, 1, 0,
            PixelFormat.Rgb, PixelType.UnsignedByte, null);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 4);
        SetNearestClamp();
        _gl.BindTexture(TextureTarget.Texture2D, 0);
        return handle;
    }

    private unsafe uint CreateDummy(InternalFormat internalFormat, PixelFormat format, PixelType type)
    {
        uint handle = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, handle);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 1);
        _gl.TexImage2D(TextureTarget.Texture2D, 0, internalFormat, 1, 1, 0, format, type, null);
        _gl.PixelStore(PixelStoreParameter.UnpackAlignment, 4);
        SetNearestClamp();
        _gl.BindTexture(TextureTarget.Texture2D, 0);
        return handle;
    }

    private void SetNearestClamp()
    {
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureBaseLevel, 0);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMaxLevel, 0);
    }

    private struct Entry
    {
        public uint Handle;
        public long LastUsed;
    }

    private sealed class ReferenceComparer : IEqualityComparer<ObjectTexture>
    {
        public static ReferenceComparer Instance { get; } = new();

        public bool Equals(ObjectTexture? x, ObjectTexture? y) => ReferenceEquals(x, y);

        public int GetHashCode(ObjectTexture obj)
            => System.Runtime.CompilerServices.RuntimeHelpers.GetHashCode(obj);
    }
}
