using Silk.NET.Maths;
using Silk.NET.OpenGL;
using Silk.NET.Windowing;
using Zax.Render;
using Xunit.Sdk;

namespace Zax.Render.Tests;

/// <summary>
/// A hidden OpenGL 3.3 core context with an offscreen colour-and-depth target, the same shape of
/// target the harness hands the renderer (SPEC-06 REQ-SCN-057, SPEC-01 REQ-ARCH-031).
/// </summary>
internal sealed unsafe class GlFixture : IDisposable
{
    private readonly IWindow _window;
    private uint _framebuffer;
    private uint _colorTexture;
    private uint _depthBuffer;
    private bool _disposed;

    private GlFixture(IWindow window, GL gl, int width, int height)
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

    /// <summary>The target to hand to the renderer.</summary>
    public RenderTarget Target => new(_framebuffer, Width, Height);

    /// <summary>
    /// Serialises context creation. The assembly runs its tests one at a time
    /// (<c>GlCollection.cs</c>), so this never blocks in practice; it is here so that a future
    /// test that does spin up a thread cannot race GLFW's own initialisation.
    /// </summary>
    private static readonly object CreationLock = new();

    /// <summary>Creates a context, or throws when the machine has no usable GL 3.3 core driver.</summary>
    public static GlFixture Create(int width = 640, int height = 480)
    {
        lock (CreationLock)
        {
            return CreateCore(width, height);
        }
    }

    private static GlFixture CreateCore(int width, int height)
    {
        Window.PrioritizeGlfw();
        WindowOptions options = WindowOptions.Default with
        {
            IsVisible = false,
            Size = new Vector2D<int>(Math.Max(width, 16), Math.Max(height, 16)),
            Title = "Zax.Render.Tests",
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
        return new GlFixture(window, GL.GetApi(window), width, height);
    }

    /// <summary>Renders <paramref name="scene"/> and reads the capture back, top row first.</summary>
    public Capture Render(SceneRenderer renderer, SceneState scene)
    {
        ArgumentNullException.ThrowIfNull(renderer);
        renderer.Render(Target, scene);
        return new Capture(ReadPixels(), Width, Height);
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

/// <summary>One capture, as rows of RGBA bytes with the top row first.</summary>
internal readonly record struct Capture(byte[] Pixels, int Width, int Height)
{
    /// <summary>The RGB of the pixel at (<paramref name="x"/>, <paramref name="y"/>).</summary>
    public (int R, int G, int B) At(int x, int y)
    {
        int at = (((y * Width) + x) * 4);
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

    /// <summary>How many pixels have the packed colour <paramref name="color"/>.</summary>
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

    /// <summary>
    /// The rows that hold at least one pixel other than <paramref name="background"/>, as
    /// (first, last); (−1, −1) when there is none.
    /// </summary>
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
}

/// <summary>
/// A fact skipped, rather than failed, when the machine cannot give a GL 3.3 core context.
/// </summary>
[XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
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
            using GlFixture fixture = GlFixture.Create(16, 16);
            return null;
        }
        catch (Exception ex)
        {
            return $"no OpenGL 3.3 core context on this machine: {ex.Message}";
        }
    }
}
