using Silk.NET.Maths;
using Silk.NET.OpenGL;
using Silk.NET.Windowing;
using Zax.Render;

namespace Zax.Harness;

/// <summary>
/// A hidden OpenGL 3.3 core context with an offscreen colour-and-depth framebuffer, so that the
/// renderer can be driven and read back without a visible window
/// (SPEC-01 REQ-ARCH-031, SPEC-06 REQ-SCN-057).
/// </summary>
/// <remarks>
/// The window is created invisible and is never shown nor run; only its context is made current.
/// It is the same path <see cref="SceneRenderer"/> takes in the application, which is what
/// REQ-SCN-057 asks for: the target comes from the outside.
/// </remarks>
internal sealed unsafe class OffscreenContext : IDisposable
{
    private readonly IWindow _window;
    private uint _framebuffer;
    private uint _colorTexture;
    private uint _depthBuffer;
    private bool _disposed;

    private OffscreenContext(IWindow window, GL gl, int width, int height)
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

    /// <summary>The render target to hand to the renderer.</summary>
    public RenderTarget Target => new(_framebuffer, Width, Height);

    /// <summary>
    /// Creates the context, or throws when the machine has no usable GL 3.3 core driver.
    /// </summary>
    public static OffscreenContext Create(int width, int height)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(width, 1);
        ArgumentOutOfRangeException.ThrowIfLessThan(height, 1);

        Window.PrioritizeGlfw();
        WindowOptions options = WindowOptions.Default with
        {
            IsVisible = false,
            Size = new Vector2D<int>(Math.Max(width, 16), Math.Max(height, 16)),
            Title = "Zax.Harness",
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
        GL gl = GL.GetApi(window);
        return new OffscreenContext(window, gl, width, height);
    }

    /// <summary>
    /// Reads the colour attachment back as rows of RGBA bytes, top row first — the order a PNG
    /// expects, which is the reverse of OpenGL's.
    /// </summary>
    public byte[] ReadPixels()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);

        int stride = Width * 4;
        byte[] bottomUp = new byte[stride * Height];
        Gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, _framebuffer);
        Gl.PixelStore(PixelStoreParameter.PackAlignment, 1);
        fixed (byte* data = bottomUp)
        {
            Gl.ReadPixels(
                0, 0, (uint)Width, (uint)Height,
                PixelFormat.Rgba, PixelType.UnsignedByte, data);
        }

        byte[] topDown = new byte[bottomUp.Length];
        for (int y = 0; y < Height; y++)
        {
            bottomUp.AsSpan((Height - 1 - y) * stride, stride).CopyTo(topDown.AsSpan(y * stride, stride));
        }

        return topDown;
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

    private void Allocate()
    {
        _framebuffer = Gl.GenFramebuffer();
        Gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffer);

        _colorTexture = Gl.GenTexture();
        Gl.BindTexture(TextureTarget.Texture2D, _colorTexture);
        Gl.TexImage2D(
            TextureTarget.Texture2D, 0, InternalFormat.Rgba8,
            (uint)Width, (uint)Height, 0,
            PixelFormat.Rgba, PixelType.UnsignedByte, null);
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
            throw new InvalidOperationException($"The {Width}x{Height} offscreen target is incomplete: {status}.");
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
