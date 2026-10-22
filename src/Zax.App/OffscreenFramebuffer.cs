using Silk.NET.OpenGL;
using Zax.Render;

namespace Zax.App;

/// <summary>
/// A colour plus depth framebuffer owned by the host and handed to <see cref="ISceneRenderer"/>.
/// </summary>
/// <remarks>
/// The render target lives on the host side on purpose (PLAN.md, libretro portability
/// section): under RetroArch the frontend supplies the FBO instead, and Zax.Render must not
/// care which of the two created it.
/// </remarks>
public sealed class OffscreenFramebuffer : IDisposable
{
    private readonly GL _gl;
    private uint _framebuffer;
    private uint _colorTexture;
    private uint _depthBuffer;
    private bool _disposed;

    /// <summary>Creates the framebuffer at the given size.</summary>
    /// <param name="gl">The GL API of the current context.</param>
    /// <param name="width">Initial width in pixels.</param>
    /// <param name="height">Initial height in pixels.</param>
    public OffscreenFramebuffer(GL gl, int width, int height)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        Allocate(Math.Max(1, width), Math.Max(1, height));
    }

    /// <summary>Current width in pixels.</summary>
    public int Width { get; private set; }

    /// <summary>Current height in pixels.</summary>
    public int Height { get; private set; }

    /// <summary>Name of the framebuffer object.</summary>
    public uint Handle => _framebuffer;

    /// <summary>The render target descriptor to pass to the renderer.</summary>
    public RenderTarget Target => new(_framebuffer, Width, Height);

    /// <summary>Reallocates the attachments if the requested size differs from the current one.</summary>
    /// <param name="width">Requested width in pixels.</param>
    /// <param name="height">Requested height in pixels.</param>
    public void Resize(int width, int height)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);

        int w = Math.Max(1, width);
        int h = Math.Max(1, height);
        if (w == Width && h == Height)
        {
            return;
        }

        Release();
        Allocate(w, h);
    }

    /// <summary>
    /// Copies the colour attachment onto the window's default framebuffer, scaling if needed.
    /// </summary>
    /// <param name="windowWidth">Width of the window framebuffer in pixels.</param>
    /// <param name="windowHeight">Height of the window framebuffer in pixels.</param>
    public void BlitToDefaultFramebuffer(int windowWidth, int windowHeight)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);

        _gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, _framebuffer);
        _gl.BindFramebuffer(FramebufferTarget.DrawFramebuffer, 0);
        _gl.BlitFramebuffer(
            0, 0, Width, Height,
            0, 0, Math.Max(1, windowWidth), Math.Max(1, windowHeight),
            (uint)ClearBufferMask.ColorBufferBit,
            BlitFramebufferFilter.Linear);
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);
    }

    /// <summary>
    /// Reads the colour attachment back and scales it to <paramref name="width"/> ×
    /// <paramref name="height"/> RGB, rows top to bottom: the pause capture of SPEC-07
    /// REQ-LOOP-006 that becomes a slot thumbnail (SPEC-09 REQ-PERS-051).
    /// </summary>
    /// <param name="width">Target width, 256 for a thumbnail.</param>
    /// <param name="height">Target height, 192 for a thumbnail.</param>
    /// <returns>Three bytes per pixel, or null when the framebuffer is empty.</returns>
    public unsafe byte[]? ReadScaledRgb(int width, int height)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        if (Width <= 0 || Height <= 0 || width <= 0 || height <= 0)
        {
            return null;
        }

        byte[] source = new byte[Width * Height * 4];
        _gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, _framebuffer);
        fixed (byte* pixels = source)
        {
            _gl.ReadPixels(
                0, 0, (uint)Width, (uint)Height,
                PixelFormat.Rgba, PixelType.UnsignedByte, pixels);
        }

        _gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, 0);

        // Nearest sampling is enough: the capture is a 256 × 192 thumbnail. GL reads rows
        // bottom-up, so the vertical axis is flipped here.
        byte[] target = new byte[width * height * 3];
        for (int y = 0; y < height; y++)
        {
            int sourceY = Height - 1 - (y * Height / height);
            sourceY = Math.Clamp(sourceY, 0, Height - 1);
            for (int x = 0; x < width; x++)
            {
                int sourceX = Math.Clamp(x * Width / width, 0, Width - 1);
                int from = ((sourceY * Width) + sourceX) * 4;
                int to = ((y * width) + x) * 3;
                target[to] = source[from];
                target[to + 1] = source[from + 1];
                target[to + 2] = source[from + 2];
            }
        }

        return target;
    }

    private unsafe void Allocate(int width, int height)
    {
        Width = width;
        Height = height;

        _framebuffer = _gl.GenFramebuffer();
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffer);

        _colorTexture = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, _colorTexture);
        _gl.TexImage2D(
            TextureTarget.Texture2D, 0, InternalFormat.Rgba8,
            (uint)width, (uint)height, 0,
            PixelFormat.Rgba, PixelType.UnsignedByte, null);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Linear);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Linear);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
        _gl.FramebufferTexture2D(
            FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment0,
            TextureTarget.Texture2D, _colorTexture, 0);
        _gl.BindTexture(TextureTarget.Texture2D, 0);

        _depthBuffer = _gl.GenRenderbuffer();
        _gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, _depthBuffer);
        _gl.RenderbufferStorage(
            RenderbufferTarget.Renderbuffer, InternalFormat.Depth24Stencil8, (uint)width, (uint)height);
        _gl.FramebufferRenderbuffer(
            FramebufferTarget.Framebuffer, FramebufferAttachment.DepthStencilAttachment,
            RenderbufferTarget.Renderbuffer, _depthBuffer);
        _gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, 0);

        GLEnum status = _gl.CheckFramebufferStatus(FramebufferTarget.Framebuffer);
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);

        if (status != GLEnum.FramebufferComplete)
        {
            Release();
            throw new InvalidOperationException(
                $"Offscreen framebuffer {width}x{height} is incomplete: {status}.");
        }
    }

    private void Release()
    {
        if (_depthBuffer != 0)
        {
            _gl.DeleteRenderbuffer(_depthBuffer);
            _depthBuffer = 0;
        }

        if (_colorTexture != 0)
        {
            _gl.DeleteTexture(_colorTexture);
            _colorTexture = 0;
        }

        if (_framebuffer != 0)
        {
            _gl.DeleteFramebuffer(_framebuffer);
            _framebuffer = 0;
        }
    }

    /// <inheritdoc />
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Release();
    }
}
