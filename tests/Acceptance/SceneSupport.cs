using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Silk.NET.Maths;
using Silk.NET.OpenGL;
using Silk.NET.Windowing;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Builds the synthetic scenes the SPEC-06 verification asserts on: a profile of one-cell or
/// block patterns, a batch of tile records, and the <see cref="SceneState"/> the renderer reads.
/// </summary>
public sealed class SceneFixture
{
    private readonly GameProfile _profile = GameProfile.CreateEmpty();
    private readonly List<TileRecord> _records = [];
    private readonly byte[] _palette = new byte[32];
    private int _spriteOrder;

    /// <summary>The engine of the last <see cref="Build"/>, which owns the draw list.</summary>
    public ConversionEngine? Engine { get; private set; }

    /// <summary>A slot where a test can park the scene it built, to keep a helper readable.</summary>
    public SceneState? Scene { get; set; }

    /// <summary>Whether the frame is converted in "2D Mode" (REQ-SCN-055).</summary>
    public bool TwoDimensionalMode { get; set; }

    /// <summary>Sets the universal background colour, entry 0 of the palette.</summary>
    public SceneFixture Background(byte nesColor)
    {
        _palette[0] = nesColor;
        return this;
    }

    /// <summary>Sets one of the 32 PPU palette bytes.</summary>
    public SceneFixture Palette(int entry, byte nesColor)
    {
        if (entry is > 0 and < 32)
        {
            _palette[entry] = nesColor;
        }

        return this;
    }

    /// <summary>Adds a pattern of <paramref name="columns"/> × <paramref name="rows"/> cells.</summary>
    public SceneFixture Pattern(
        (ulong Low, ulong High) tile,
        int depth,
        ProfileLayer layer,
        float alpha = 1f,
        VariantRenderStyle style = VariantRenderStyle.Normal,
        int columns = 1,
        int rows = 1,
        string? tags = null)
    {
        ProfileTile[] cells = [.. Enumerable
            .Range(0, columns * rows)
            .Select(_ => new ProfileTile(tile.Low, tile.High, TileFlip.None))];

        ProfilePattern pattern = new()
        {
            Grid = new PatternGrid(columns, rows, cells),
            TileSize = new Vec2I(columns, rows),
            Permanent = true,
            SelfConnected = true,
            ConstraintSize = Vec2I.Unconstrained,
        };
        pattern.Variants.Add(new PatternVariant
        {
            Depth = depth,
            Layer = layer,
            Alpha = alpha,
            RenderStyle = style,
            TagString = tags,
        });
        _profile.PermanentPatterns.Add(pattern);
        return this;
    }

    /// <summary>Adds a background tile record at the screen position (x, y).</summary>
    public SceneFixture Tile((ulong Low, ulong High) tile, int x, int y, int palette = 0)
    {
        _records.Add(new TileRecord(
            tile.Low, tile.High, (short)x, (short)y, _records.Count,
            mirrorH: false, mirrorV: false, hidden: false, (byte)palette, background: true));
        return this;
    }

    /// <summary>Adds a rectangle of background tile records, in tile cells.</summary>
    public SceneFixture Rect(
        (ulong Low, ulong High) tile, int column, int row, int columns, int rows, int palette = 0)
    {
        for (int y = 0; y < rows; y++)
        {
            for (int x = 0; x < columns; x++)
            {
                Tile(tile, (column + x) * 8, (row + y) * 8, palette);
            }
        }

        return this;
    }

    /// <summary>Adds a sprite tile record.</summary>
    public SceneFixture Sprite(
        (ulong Low, ulong High) tile, int x, int y, int palette = 0, bool behindBackground = false)
    {
        _records.Add(new TileRecord(
            tile.Low, tile.High, (short)x, (short)y, _spriteOrder++,
            mirrorH: false, mirrorV: false, hidden: behindBackground, (byte)palette, background: false));
        return this;
    }

    /// <summary>Converts the frame and returns a scene ready to render.</summary>
    public SceneState Build(Action<ConversionSettings>? configure = null)
    {
        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.TwoDimensionalMode = TwoDimensionalMode;
        engine.LoadProfile(_profile);
        configure?.Invoke(engine.Settings);

        engine.Convert(new FrameBatch([.. _records], _palette, 0, 0), frameCounter: 0);
        engine.Publish();

        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Clipping = engine.Settings.Clipping;
        scene.DrawList = engine.BuildDrawList();
        scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);
        scene.EmulatedTime = engine.PublishedFrame.Time;
        Engine = engine;
        return scene;
    }

    /// <summary>A full tile of the palette colour <paramref name="color"/>, identity <paramref name="id"/>.</summary>
    public static (ulong Low, ulong High) Solid(int color, int id = 0)
    {
        ulong low = (color & 1) != 0 ? ulong.MaxValue : 0UL;
        ulong high = (color & 2) != 0 ? ulong.MaxValue : 0UL;
        if ((color & 2) == 0)
        {
            high = 1UL << (id & 63);
        }
        else if ((color & 1) == 0)
        {
            low = 1UL << (id & 63);
        }

        return (low, high);
    }

    /// <summary>The packed RGB of a NES colour index.</summary>
    public static int NesPacked(int index) => NesPalette.Color(index).Packed;
}

/// <summary>
/// A hidden OpenGL 3.3 core context with an offscreen colour-and-depth target: the kind of
/// target REQ-SCN-057 says the host supplies.
/// </summary>
public sealed unsafe class GlTarget : IDisposable
{
    private static readonly object CreationLock = new();

    private readonly IWindow _window;
    private uint _framebuffer;
    private uint _colorTexture;
    private uint _depthBuffer;
    private bool _disposed;

    private GlTarget(IWindow window, GL gl, int width, int height)
    {
        _window = window;
        Gl = gl;
        Width = width;
        Height = height;
        Allocate();
    }

    /// <summary>The GL API of the context.</summary>
    public GL Gl { get; }

    /// <summary>Width of the target in pixels.</summary>
    public int Width { get; }

    /// <summary>Height of the target in pixels.</summary>
    public int Height { get; }

    /// <summary>The target to hand the renderer.</summary>
    public RenderTarget Target => new(_framebuffer, Width, Height);

    /// <summary>Creates a context, or throws when the machine has no usable GL 3.3 core driver.</summary>
    public static GlTarget Create(int width = 640, int height = 480)
    {
        lock (CreationLock)
        {
            Window.PrioritizeGlfw();
            WindowOptions options = WindowOptions.Default with
            {
                IsVisible = false,
                Size = new Vector2D<int>(Math.Max(width, 16), Math.Max(height, 16)),
                Title = "Zax.Acceptance.Tests",
                API = new GraphicsAPI(
                    ContextAPI.OpenGL,
                    ContextProfile.Core,
                    ContextFlags.ForwardCompatible,
                    new APIVersion(3, 3)),
                VSync = false,
                ShouldSwapAutomatically = false,
                IsEventDriven = false,
            };

            IWindow window = Window.Create(options);
            window.Initialize();
            window.MakeCurrent();
            return new GlTarget(window, GL.GetApi(window), width, height);
        }
    }

    /// <summary>
    /// Renders <paramref name="scene"/> and waits for the GPU to be done, <b>without</b> reading
    /// the image back. This is what a frame of the application costs; <see cref="Render"/> adds
    /// a full-frame read-back, which the application never does and which would dominate a
    /// frame-time measurement at 1920 × 1080.
    /// </summary>
    public void Draw(SceneRenderer renderer, SceneState scene)
    {
        ArgumentNullException.ThrowIfNull(renderer);
        renderer.Render(Target, scene);
        Gl.Finish();
    }

    /// <summary>Renders <paramref name="scene"/> and reads the image back, top row first.</summary>
    public Shot Render(SceneRenderer renderer, SceneState scene)
    {
        ArgumentNullException.ThrowIfNull(renderer);
        renderer.Render(Target, scene);
        return new Shot(ReadPixels(), Width, Height);
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Release();
        Gl.Dispose();
        _window.Dispose();
    }

    private byte[] ReadPixels()
    {
        int stride = Width * 4;
        byte[] bottomUp = new byte[stride * Height];
        Gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, _framebuffer);
        Gl.PixelStore(PixelStoreParameter.PackAlignment, 1);
        fixed (byte* data = bottomUp)
        {
            Gl.ReadPixels(0, 0, (uint)Width, (uint)Height, PixelFormat.Rgba, PixelType.UnsignedByte, data);
        }

        byte[] topDown = new byte[bottomUp.Length];
        for (int y = 0; y < Height; y++)
        {
            bottomUp.AsSpan((Height - 1 - y) * stride, stride).CopyTo(topDown.AsSpan(y * stride, stride));
        }

        return topDown;
    }

    private void Allocate()
    {
        _framebuffer = Gl.GenFramebuffer();
        Gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffer);

        _colorTexture = Gl.GenTexture();
        Gl.BindTexture(TextureTarget.Texture2D, _colorTexture);
        Gl.TexImage2D(
            TextureTarget.Texture2D, 0, InternalFormat.Rgba8,
            (uint)Width, (uint)Height, 0, PixelFormat.Rgba, PixelType.UnsignedByte, null);
        Gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Nearest);
        Gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Nearest);
        Gl.FramebufferTexture2D(
            FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment0,
            TextureTarget.Texture2D, _colorTexture, 0);
        Gl.BindTexture(TextureTarget.Texture2D, 0);

        _depthBuffer = Gl.GenRenderbuffer();
        Gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, _depthBuffer);
        Gl.RenderbufferStorage(
            RenderbufferTarget.Renderbuffer, InternalFormat.DepthComponent24, (uint)Width, (uint)Height);
        Gl.FramebufferRenderbuffer(
            FramebufferTarget.Framebuffer, FramebufferAttachment.DepthAttachment,
            RenderbufferTarget.Renderbuffer, _depthBuffer);
        Gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, 0);

        GLEnum status = Gl.CheckFramebufferStatus(FramebufferTarget.Framebuffer);
        Gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);
        if (status != GLEnum.FramebufferComplete)
        {
            Release();
            throw new InvalidOperationException($"the {Width}x{Height} offscreen target is incomplete: {status}");
        }
    }

    private void Release()
    {
        if (_depthBuffer != 0)
        {
            Gl.DeleteRenderbuffer(_depthBuffer);
            _depthBuffer = 0;
        }

        if (_colorTexture != 0)
        {
            Gl.DeleteTexture(_colorTexture);
            _colorTexture = 0;
        }

        if (_framebuffer != 0)
        {
            Gl.DeleteFramebuffer(_framebuffer);
            _framebuffer = 0;
        }
    }
}

/// <summary>One offscreen capture, as rows of RGBA bytes with the top row first.</summary>
public readonly record struct Shot(byte[] Pixels, int Width, int Height)
{
    /// <summary>The RGB of the pixel at (<paramref name="x"/>, <paramref name="y"/>).</summary>
    public (int R, int G, int B) At(int x, int y)
    {
        int at = ((y * Width) + x) * 4;
        return (Pixels[at], Pixels[at + 1], Pixels[at + 2]);
    }

    /// <summary>The packed RGB of the pixel.</summary>
    public int Packed(int x, int y)
    {
        (int r, int g, int b) = At(x, y);
        return (r << 16) | (g << 8) | b;
    }

    /// <summary>Relative luma of the pixel, 0 to 1.</summary>
    public double Luma(int x, int y)
    {
        (int r, int g, int b) = At(x, y);
        return ((0.299 * r) + (0.587 * g) + (0.114 * b)) / 255.0;
    }

    /// <summary>How many pixels carry the packed colour <paramref name="color"/>.</summary>
    public int Count(int color)
    {
        int found = 0;
        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                if (Packed(x, y) == color)
                {
                    found++;
                }
            }
        }

        return found;
    }

    /// <summary>How many pixels differ from <paramref name="color"/>.</summary>
    public int CountOther(int color) => (Width * Height) - Count(color);

    /// <summary>The distinct colours of the capture.</summary>
    public HashSet<int> DistinctColors()
    {
        HashSet<int> colors = [];
        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                colors.Add(Packed(x, y));
            }
        }

        return colors;
    }

    /// <summary>The first and last rows holding a pixel other than <paramref name="background"/>.</summary>
    public (int First, int Last) RowSpan(int background)
    {
        int first = -1;
        int last = -1;
        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                if (Packed(x, y) == background)
                {
                    continue;
                }

                first = first < 0 ? y : first;
                last = y;
                break;
            }
        }

        return (first, last);
    }

    /// <summary>The same for columns.</summary>
    public (int First, int Last) ColumnSpan(int background)
    {
        int first = -1;
        int last = -1;
        for (int x = 0; x < Width; x++)
        {
            for (int y = 0; y < Height; y++)
            {
                if (Packed(x, y) == background)
                {
                    continue;
                }

                first = first < 0 ? x : first;
                last = x;
                break;
            }
        }

        return (first, last);
    }

    /// <summary>Writes the capture as a PNG, for the captures archived with the report.</summary>
    public void SavePng(string path)
    {
        byte[] rgb = new byte[Width * Height * 3];
        for (int i = 0, j = 0; i < Pixels.Length; i += 4, j += 3)
        {
            rgb[j] = Pixels[i];
            rgb[j + 1] = Pixels[i + 1];
            rgb[j + 2] = Pixels[i + 2];
        }

        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllBytes(path, MinimalPng.Encode(rgb, Width, Height));
    }
}

/// <summary>A minimal PNG writer (RGB, 8 bits, no filtering) for the archived captures.</summary>
internal static class MinimalPng
{
    public static byte[] Encode(byte[] rgb, int width, int height)
    {
        using MemoryStream raw = new();
        for (int y = 0; y < height; y++)
        {
            raw.WriteByte(0);
            raw.Write(rgb, y * width * 3, width * 3);
        }

        byte[] scanlines = raw.ToArray();
        using MemoryStream deflated = new();
        using (System.IO.Compression.ZLibStream zlib = new(deflated, System.IO.Compression.CompressionLevel.Fastest, leaveOpen: true))
        {
            zlib.Write(scanlines);
        }

        using MemoryStream png = new();
        png.Write([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        WriteChunk(png, "IHDR", Header(width, height));
        WriteChunk(png, "IDAT", deflated.ToArray());
        WriteChunk(png, "IEND", []);
        return png.ToArray();
    }

    private static byte[] Header(int width, int height)
    {
        byte[] header = new byte[13];
        BinaryPrimitivesBigEndian(header, 0, width);
        BinaryPrimitivesBigEndian(header, 4, height);
        header[8] = 8;
        header[9] = 2;
        return header;
    }

    private static void WriteChunk(Stream stream, string type, byte[] data)
    {
        byte[] length = new byte[4];
        BinaryPrimitivesBigEndian(length, 0, data.Length);
        stream.Write(length);

        byte[] typeBytes = System.Text.Encoding.ASCII.GetBytes(type);
        stream.Write(typeBytes);
        stream.Write(data);

        uint crc = Crc32([.. typeBytes, .. data]);
        byte[] crcBytes = new byte[4];
        BinaryPrimitivesBigEndian(crcBytes, 0, (int)crc);
        stream.Write(crcBytes);
    }

    private static void BinaryPrimitivesBigEndian(byte[] destination, int offset, int value)
    {
        destination[offset] = (byte)(value >> 24);
        destination[offset + 1] = (byte)(value >> 16);
        destination[offset + 2] = (byte)(value >> 8);
        destination[offset + 3] = (byte)value;
    }

    private static uint Crc32(byte[] data)
    {
        uint crc = 0xFFFFFFFFu;
        foreach (byte value in data)
        {
            crc ^= value;
            for (int bit = 0; bit < 8; bit++)
            {
                crc = (crc & 1) != 0 ? (crc >> 1) ^ 0xEDB88320u : crc >> 1;
            }
        }

        return crc ^ 0xFFFFFFFFu;
    }
}

/// <summary>A fact skipped, not failed, when the machine gives no GL 3.3 core context.</summary>
public sealed class GlFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public GlFactAttribute()
    {
        if (GlAvailability.Reason is { } reason)
        {
            Skip = reason;
        }
    }
}

/// <summary>Probes once whether a GL 3.3 core context can be created in this process.</summary>
internal static class GlAvailability
{
    /// <summary>Why no context is available, or null when one is.</summary>
    public static string? Reason { get; } = Probe();

    private static string? Probe()
    {
        try
        {
            using GlTarget target = GlTarget.Create(16, 16);
            return null;
        }
        catch (Exception exception)
        {
            return $"no OpenGL 3.3 core context on this machine: {exception.Message}";
        }
    }
}

/// <summary>Where the captures of the final verification are archived.</summary>
public static class FinalCaptures
{
    /// <summary>Absolute path of <c>verify/round1/final</c>.</summary>
    public static string Directory { get; } = Locate();

    /// <summary>The directory, created if it is not there yet.</summary>
    public static string Ready()
    {
        System.IO.Directory.CreateDirectory(Directory);
        return Directory;
    }

    private static string Locate()
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            string candidate = Path.Combine(at.FullName, "verify", "round1");
            if (System.IO.Directory.Exists(candidate))
            {
                return Path.Combine(candidate, "final");
            }

            at = at.Parent;
        }

        return Path.Combine(Path.GetTempPath(), "zax-final");
    }
}

/// <summary>Where the part B captures are archived.</summary>
public static class PartBCaptures
{
    /// <summary>Absolute path of <c>verify/round1/part-b</c>.</summary>
    public static string Directory { get; } = Locate();

    private static string Locate()
    {
        DirectoryInfo? at = new(AppContext.BaseDirectory);
        while (at is not null)
        {
            string candidate = Path.Combine(at.FullName, "verify", "round1");
            if (System.IO.Directory.Exists(candidate))
            {
                return Path.Combine(candidate, "part-b");
            }

            at = at.Parent;
        }

        return Path.Combine(Path.GetTempPath(), "zax-part-b");
    }
}
