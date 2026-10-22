using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// The scene's own colour buffer, holding <em>linear</em> values, and the pass that encodes it
/// to sRGB into the framebuffer the host provided (AMD-021 R2-009).
/// </summary>
/// <remarks>
/// <para>
/// R2-009 requires the transparency of REQ-SCN-040 — objects, HUD and the FPV occlusion of
/// REQ-SCN-022 — to be blended on linear colours: displayed = sRGB(α × linear of the object +
/// (1 − α) × linear already present). OpenGL blends whatever the fragment shader wrote, so the
/// shaders have to write linear and the sRGB encode has to happen after the blending, not
/// inside them.
/// </para>
/// <para>
/// The host framebuffer is 8-bit sRGB (REQ-SCN-057: the renderer never creates it, and under a
/// libretro frontend it does not even own its format), so the renderer cannot ask for an
/// sRGB-encoded attachment and let the hardware decode on blend. It renders into this
/// half-float attachment instead — 8 bits of linear would band visibly in the darks — and
/// resolves it with one full-screen triangle. The alpha channel is copied through unchanged:
/// it is never a colour (REQ-SCN-035).
/// </para>
/// </remarks>
internal sealed class LinearColorBuffer : IDisposable
{
    private readonly GL _gl;
    private ShaderProgram? _resolve;
    private uint _framebuffer;
    private uint _colorTexture;
    private uint _depthBuffer;
    private uint _vertexArray;
    private bool _disposed;

    /// <summary>Creates the buffer; the attachments are allocated on the first use.</summary>
    public LinearColorBuffer(GL gl)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
    }

    /// <summary>Width of the attachments in pixels.</summary>
    public int Width { get; private set; }

    /// <summary>Height of the attachments in pixels.</summary>
    public int Height { get; private set; }

    /// <summary>Name of the framebuffer object, 0 before the first <see cref="BeginPass"/>.</summary>
    public uint Framebuffer => _framebuffer;

    /// <summary>
    /// Binds the buffer as the draw target at <paramref name="width"/> ×
    /// <paramref name="height"/>, reallocating it when the size changed.
    /// </summary>
    public void BeginPass(int width, int height)
    {
        int w = Math.Max(1, width);
        int h = Math.Max(1, height);
        if (_framebuffer == 0 || w != Width || h != Height)
        {
            Release();
            Allocate(w, h);
        }

        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffer);
        _gl.Viewport(0, 0, (uint)Width, (uint)Height);
    }

    /// <summary>
    /// Encodes the buffer to sRGB into <paramref name="target"/>, which becomes the bound
    /// framebuffer.
    /// </summary>
    public void ResolveTo(in RenderTarget target)
    {
        if (_framebuffer == 0)
        {
            return;
        }

        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, target.Framebuffer);
        _gl.Viewport(0, 0, (uint)target.Width, (uint)target.Height);
        _gl.Disable(EnableCap.ScissorTest);
        _gl.Disable(EnableCap.Blend);
        _gl.Disable(EnableCap.CullFace);
        _gl.Disable(EnableCap.DepthTest);
        _gl.DepthMask(false);
        _gl.ColorMask(true, true, true, true);

        ShaderProgram program = _resolve ??= ShaderProgram.Create(
            _gl, "resolve", SceneShaders.ResolveVertex, SceneShaders.ResolveFragment);
        program.Use();
        program.Set("uLinear", 0);
        _gl.ActiveTexture(TextureUnit.Texture0);
        _gl.BindTexture(TextureTarget.Texture2D, _colorTexture);

        _gl.BindVertexArray(VertexArray());
        _gl.DrawArrays(PrimitiveType.Triangles, 0, 3);
        _gl.BindVertexArray(0);

        _gl.Enable(EnableCap.DepthTest);
        _gl.DepthMask(true);
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _resolve?.Dispose();
        if (_vertexArray != 0)
        {
            _gl.DeleteVertexArray(_vertexArray);
            _vertexArray = 0;
        }

        Release();
    }

    /// <summary>The empty vertex array the full-screen triangle is drawn from.</summary>
    private uint VertexArray()
    {
        if (_vertexArray == 0)
        {
            _vertexArray = _gl.GenVertexArray();
        }

        return _vertexArray;
    }

    private unsafe void Allocate(int width, int height)
    {
        Width = width;
        Height = height;

        _framebuffer = _gl.GenFramebuffer();
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffer);

        _colorTexture = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, _colorTexture);

        // RGBA16F is colour-renderable in every GL 3.0 and later implementation, so no
        // extension is needed (REQ-SCN-056).
        _gl.TexImage2D(
            TextureTarget.Texture2D, 0, InternalFormat.Rgba16f,
            (uint)width, (uint)height, 0,
            PixelFormat.Rgba, PixelType.Float, null);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
        _gl.FramebufferTexture2D(
            FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment0,
            TextureTarget.Texture2D, _colorTexture, 0);
        _gl.BindTexture(TextureTarget.Texture2D, 0);

        _depthBuffer = _gl.GenRenderbuffer();
        _gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, _depthBuffer);
        _gl.RenderbufferStorage(
            RenderbufferTarget.Renderbuffer, InternalFormat.DepthComponent24, (uint)width, (uint)height);
        _gl.FramebufferRenderbuffer(
            FramebufferTarget.Framebuffer, FramebufferAttachment.DepthAttachment,
            RenderbufferTarget.Renderbuffer, _depthBuffer);
        _gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, 0);

        GLEnum status = _gl.CheckFramebufferStatus(FramebufferTarget.Framebuffer);
        if (status != GLEnum.FramebufferComplete)
        {
            Release();
            throw new InvalidOperationException(
                $"Linear colour buffer {width}x{height} is incomplete: {status}.");
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

        Width = 0;
        Height = 0;
    }
}
