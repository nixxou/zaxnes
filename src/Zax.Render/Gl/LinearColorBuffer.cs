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
/// <para>
/// AMD-032 §2: when anti-aliasing is on, the scene is drawn into a <em>multisampled</em> copy
/// of the same attachments and that copy is resolved into the single-sample texture before
/// anything samples it. Multisampling belongs here rather than on the host framebuffer because
/// the blending of R2-009 has to keep happening on the linear buffer, and because the renderer
/// does not own the host's format (REQ-SCN-057).
/// </para>
/// <para>
/// AMD-055 REQ-SCN-080, fourth bullet: the screen passes of the Enhanced style read the colour
/// and the depth <b>resolved</b>, so the depth is a sampleable texture — not a renderbuffer —
/// and <see cref="ResolveDepth"/> flattens the multisampled copy of it on demand, in the middle
/// of the frame, while the scene target stays bound for what is left to draw.
/// </para>
/// <para>
/// AMD-055 REQ-SCN-084: the floor reflections also need to know, per pixel, whether the surface
/// there is a <em>floor surface</em> and which way it faces. That is a second colour attachment,
/// allocated only for the frames where the reflections are on and written by the opaque pass
/// alone — <see cref="SetFloorMaskTarget"/> turns its draw buffer on and off, so no other pass
/// has to write an output for it.
/// </para>
/// </remarks>
internal sealed class LinearColorBuffer : IDisposable
{
    private readonly GL _gl;
    private ShaderProgram? _resolve;
    private uint _framebuffer;
    private uint _colorTexture;
    private uint _depthTexture;
    private uint _maskTexture;
    private uint _sampleFramebuffer;
    private uint _sampleColorBuffer;
    private uint _sampleDepthBuffer;
    private uint _sampleMaskBuffer;
    private bool _floorMask;
    private uint _vertexArray;
    private int _maxSamples = -1;
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

    /// <summary>
    /// Samples the scene is actually drawn with: 1 when anti-aliasing is off, and otherwise
    /// what the implementation granted, which may be fewer than what was asked for
    /// (AMD-032 §2).
    /// </summary>
    public int Samples { get; private set; } = 1;

    /// <summary>Name of the framebuffer object, 0 before the first <see cref="BeginPass"/>.</summary>
    public uint Framebuffer => _framebuffer;

    /// <summary>
    /// The single-sample depth texture of the scene, holding window-space depth. It is the one
    /// the screen passes of AMD-055 read; with anti-aliasing on it only holds the scene once
    /// <see cref="ResolveDepth"/> has run.
    /// </summary>
    public uint DepthTexture => _depthTexture;

    /// <summary>
    /// The single-sample floor mask of AMD-055 REQ-SCN-084 — the bent scene normal in
    /// <c>rgb</c>, the floor flag in <c>a</c> — or 0 when this frame has none. With
    /// anti-aliasing on it only holds the scene once <see cref="ResolveForScreenPass"/> has run.
    /// </summary>
    public uint FloorMaskTexture => _maskTexture;

    /// <summary>
    /// The single-sample linear colour of the scene. The reflections of REQ-SCN-084 read it, so
    /// with anti-aliasing on it only holds the scene once <see cref="ResolveForScreenPass"/> has
    /// run.
    /// </summary>
    public uint ColorTexture => _colorTexture;

    /// <summary>The name of the framebuffer the scene is being drawn into this frame.</summary>
    public uint SceneFramebuffer => Samples > 1 ? _sampleFramebuffer : _framebuffer;

    /// <summary>
    /// Flattens the multisampled depth into <see cref="DepthTexture"/> and binds the scene
    /// target again, so that a screen pass can read the resolved depth and go on drawing into
    /// the target (AMD-055 REQ-SCN-080). A no-op when anti-aliasing is off: the scene is then
    /// drawn into the depth texture itself.
    /// </summary>
    public void ResolveDepth()
    {
        if (Samples <= 1 || _sampleFramebuffer == 0)
        {
            return;
        }

        _gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, _sampleFramebuffer);
        _gl.BindFramebuffer(FramebufferTarget.DrawFramebuffer, _framebuffer);
        _gl.Disable(EnableCap.ScissorTest);

        // A depth blit between two framebuffers of the same size and format is the resolve
        // OpenGL 3.3 offers; the filter has to be Nearest for a depth buffer.
        _gl.BlitFramebuffer(
            0, 0, Width, Height,
            0, 0, Width, Height,
            (uint)ClearBufferMask.DepthBufferBit,
            BlitFramebufferFilter.Nearest);

        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _sampleFramebuffer);
        _gl.Viewport(0, 0, (uint)Width, (uint)Height);
    }

    /// <summary>
    /// Flattens the depth, the colour and — when this frame has one — the floor mask into their
    /// single-sample textures, then binds the scene target again. This is what REQ-SCN-080's
    /// fourth bullet asks of the reflections, which read the colour as well as the depth.
    /// </summary>
    public void ResolveForScreenPass()
    {
        if (Samples <= 1 || _sampleFramebuffer == 0)
        {
            return;
        }

        ResolveDepth();

        _gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, _sampleFramebuffer);
        _gl.BindFramebuffer(FramebufferTarget.DrawFramebuffer, _framebuffer);
        _gl.Disable(EnableCap.ScissorTest);

        BlitAttachment(0);
        if (_maskTexture != 0)
        {
            BlitAttachment(1);
        }

        // The read and draw buffers go back to the colour attachment the rest of the frame
        // expects (the mask's own draw buffer is turned on by SetFloorMaskTarget alone).
        _gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, _sampleFramebuffer);
        _gl.ReadBuffer(ReadBufferMode.ColorAttachment0);
        _gl.BindFramebuffer(FramebufferTarget.DrawFramebuffer, _framebuffer);
        DrawBuffers(false);

        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _sampleFramebuffer);
        _gl.Viewport(0, 0, (uint)Width, (uint)Height);
    }

    /// <summary>
    /// Turns the draw buffer of the floor mask on or off on the scene target. Only the opaque
    /// pass writes the mask (REQ-SCN-084: the transparent objects and the HUD are drawn after
    /// the reflections and carry none), so every other pass runs with it off and needs no
    /// second fragment output.
    /// </summary>
    /// <param name="on">True to write both attachments, false for the colour alone.</param>
    public void SetFloorMaskTarget(bool on)
    {
        if (_maskTexture == 0)
        {
            return;
        }

        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, SceneFramebuffer);
        DrawBuffers(on);
    }

    /// <summary>
    /// Clears the floor mask to zero — « no floor here » everywhere — without disturbing the
    /// background colour the scene target was just cleared with (REQ-SCN-046).
    /// </summary>
    public void ClearFloorMask()
    {
        if (_maskTexture == 0)
        {
            return;
        }

        // The mask alone: its draw buffer is turned on, the colour attachment is masked out of
        // the clear, and the colour the caller already wrote stays.
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, SceneFramebuffer);
        SelectDrawBuffers(GLEnum.None, GLEnum.ColorAttachment1);
        _gl.ClearColor(0f, 0f, 0f, 0f);
        _gl.Clear(ClearBufferMask.ColorBufferBit);
        DrawBuffers(false);
    }

    /// <summary>
    /// Binds the buffer as the draw target at <paramref name="width"/> ×
    /// <paramref name="height"/>, reallocating it when the size, the sample count or the need
    /// for a floor mask changed.
    /// </summary>
    /// <param name="width">Width in pixels.</param>
    /// <param name="height">Height in pixels.</param>
    /// <param name="samples">
    /// Requested samples per pixel, 1 for no anti-aliasing. Clamped to what
    /// <c>GL_MAX_SAMPLES</c> allows, then lowered one step at a time until the multisampled
    /// framebuffer is complete (AMD-032 §2).
    /// </param>
    /// <param name="floorMask">
    /// Whether this frame needs the floor mask of REQ-SCN-084. It costs an attachment, so it is
    /// only allocated for the frames whose reflections are on.
    /// </param>
    public void BeginPass(int width, int height, int samples = 1, bool floorMask = false)
    {
        int w = Math.Max(1, width);
        int h = Math.Max(1, height);
        int wanted = SceneConstants.ClampAntiAliasSamples(samples, MaxSamples());
        if (_framebuffer == 0 || w != Width || h != Height || wanted != Samples
            || floorMask != _floorMask)
        {
            Release();
            _floorMask = floorMask;
            Allocate(w, h);
            AllocateSamples(w, h, wanted);
        }

        _gl.BindFramebuffer(
            FramebufferTarget.Framebuffer, Samples > 1 ? _sampleFramebuffer : _framebuffer);
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

        ResolveSamples();

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

    /// <summary>
    /// Flattens the multisampled attachments into the single-sample texture the resolve pass
    /// and every later reader sample. A no-op when anti-aliasing is off.
    /// </summary>
    /// <remarks>
    /// The two framebuffers have the same size, so the filter the blit is given is never used;
    /// <c>Nearest</c> is the one a multisample resolve is allowed to take.
    /// </remarks>
    private void ResolveSamples()
    {
        if (Samples <= 1 || _sampleFramebuffer == 0)
        {
            return;
        }

        _gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, _sampleFramebuffer);
        _gl.BindFramebuffer(FramebufferTarget.DrawFramebuffer, _framebuffer);
        _gl.Disable(EnableCap.ScissorTest);
        BlitAttachment(0);
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);
    }

    /// <summary>
    /// Resolves one colour attachment of the multisampled target into the matching attachment of
    /// the single-sample one. The read and the draw buffer are both selected, because a colour
    /// blit copies <c>READ_BUFFER</c> into every draw buffer of the draw framebuffer.
    /// </summary>
    private void BlitAttachment(int index)
    {
        if (index == 0)
        {
            _gl.ReadBuffer(ReadBufferMode.ColorAttachment0);
            SelectDrawBuffers(GLEnum.ColorAttachment0, GLEnum.None);
        }
        else
        {
            _gl.ReadBuffer(ReadBufferMode.ColorAttachment1);
            SelectDrawBuffers(GLEnum.None, GLEnum.ColorAttachment1);
        }

        _gl.BlitFramebuffer(
            0, 0, Width, Height,
            0, 0, Width, Height,
            (uint)ClearBufferMask.ColorBufferBit,
            BlitFramebufferFilter.Nearest);
    }

    /// <summary>
    /// Selects the draw buffers of the bound draw framebuffer: the colour alone, or the colour
    /// and the floor mask of REQ-SCN-084.
    /// </summary>
    private void DrawBuffers(bool withMask)
        => SelectDrawBuffers(
            GLEnum.ColorAttachment0, withMask ? GLEnum.ColorAttachment1 : GLEnum.None);

    private void SelectDrawBuffers(GLEnum first, GLEnum second)
    {
        Span<GLEnum> buffers = [first, second];
        _gl.DrawBuffers(second == GLEnum.None && first != GLEnum.None ? 1u : 2u, buffers);
    }

    /// <summary>
    /// The highest sample count this implementation accepts, read once. OpenGL 3.3 core always
    /// answers at least 4 (REQ-SCN-056: no extension is needed for any of this).
    /// </summary>
    private int MaxSamples()
    {
        if (_maxSamples < 0)
        {
            _gl.GetInteger(GLEnum.MaxSamples, out int max);

            // A driver that answers nonsense must not disable anti-aliasing silently for ever;
            // the completeness check below is the real guard.
            _maxSamples = Math.Clamp(max, 1, SceneConstants.MaxAntiAliasSamples);
        }

        return _maxSamples;
    }

    /// <summary>
    /// Allocates the multisampled attachments, halving the sample count until the framebuffer
    /// is complete and falling back to no anti-aliasing at all (AMD-032 §2).
    /// </summary>
    private void AllocateSamples(int width, int height, int samples)
    {
        Samples = 1;
        for (int count = samples; count > 1; count /= 2)
        {
            if (TryAllocateSamples(width, height, count))
            {
                Samples = count;
                return;
            }

            ReleaseSamples();
        }
    }

    private bool TryAllocateSamples(int width, int height, int samples)
    {
        _sampleFramebuffer = _gl.GenFramebuffer();
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _sampleFramebuffer);

        _sampleColorBuffer = _gl.GenRenderbuffer();
        _gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, _sampleColorBuffer);
        _gl.RenderbufferStorageMultisample(
            RenderbufferTarget.Renderbuffer, (uint)samples, InternalFormat.Rgba16f,
            (uint)width, (uint)height);
        _gl.FramebufferRenderbuffer(
            FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment0,
            RenderbufferTarget.Renderbuffer, _sampleColorBuffer);

        if (_floorMask)
        {
            // REQ-SCN-084: the floor mask of a multisampled frame is multisampled too, and
            // resolved with the colour and the depth before the reflections read it.
            _sampleMaskBuffer = _gl.GenRenderbuffer();
            _gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, _sampleMaskBuffer);
            _gl.RenderbufferStorageMultisample(
                RenderbufferTarget.Renderbuffer, (uint)samples, InternalFormat.Rgba8,
                (uint)width, (uint)height);
            _gl.FramebufferRenderbuffer(
                FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment1,
                RenderbufferTarget.Renderbuffer, _sampleMaskBuffer);
        }

        _sampleDepthBuffer = _gl.GenRenderbuffer();
        _gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, _sampleDepthBuffer);
        _gl.RenderbufferStorageMultisample(
            RenderbufferTarget.Renderbuffer, (uint)samples, InternalFormat.DepthComponent24,
            (uint)width, (uint)height);
        _gl.FramebufferRenderbuffer(
            FramebufferTarget.Framebuffer, FramebufferAttachment.DepthAttachment,
            RenderbufferTarget.Renderbuffer, _sampleDepthBuffer);
        _gl.BindRenderbuffer(RenderbufferTarget.Renderbuffer, 0);

        DrawBuffers(false);
        GLEnum status = _gl.CheckFramebufferStatus(FramebufferTarget.Framebuffer);
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);
        return status == GLEnum.FramebufferComplete;
    }

    private void ReleaseSamples()
    {
        if (_sampleMaskBuffer != 0)
        {
            _gl.DeleteRenderbuffer(_sampleMaskBuffer);
            _sampleMaskBuffer = 0;
        }

        if (_sampleDepthBuffer != 0)
        {
            _gl.DeleteRenderbuffer(_sampleDepthBuffer);
            _sampleDepthBuffer = 0;
        }

        if (_sampleColorBuffer != 0)
        {
            _gl.DeleteRenderbuffer(_sampleColorBuffer);
            _sampleColorBuffer = 0;
        }

        if (_sampleFramebuffer != 0)
        {
            _gl.DeleteFramebuffer(_sampleFramebuffer);
            _sampleFramebuffer = 0;
        }
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

        if (_floorMask)
        {
            // AMD-055 REQ-SCN-084: the floor mask — the bent scene normal in rgb, the floor flag
            // of the requirement's « surface de sol » in a. Eight bits per channel are enough:
            // the flag is binary and the normal only steers a screen-space ray.
            _maskTexture = _gl.GenTexture();
            _gl.BindTexture(TextureTarget.Texture2D, _maskTexture);
            _gl.TexImage2D(
                TextureTarget.Texture2D, 0, InternalFormat.Rgba8,
                (uint)width, (uint)height, 0,
                PixelFormat.Rgba, PixelType.UnsignedByte, null);
            _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Nearest);
            _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Nearest);
            _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
            _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
            _gl.FramebufferTexture2D(
                FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment1,
                TextureTarget.Texture2D, _maskTexture, 0);
            _gl.BindTexture(TextureTarget.Texture2D, 0);
        }

        // A depth *texture* rather than a renderbuffer: the screen passes of AMD-055 sample it
        // (REQ-SCN-083). DepthComponent24 is required of every GL 3.0 implementation, so no
        // extension is needed (REQ-SCN-056).
        _depthTexture = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, _depthTexture);
        _gl.TexImage2D(
            TextureTarget.Texture2D, 0, InternalFormat.DepthComponent24,
            (uint)width, (uint)height, 0,
            PixelFormat.DepthComponent, PixelType.Float, null);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
        _gl.FramebufferTexture2D(
            FramebufferTarget.Framebuffer, FramebufferAttachment.DepthAttachment,
            TextureTarget.Texture2D, _depthTexture, 0);
        _gl.BindTexture(TextureTarget.Texture2D, 0);

        // Only the colour attachment is written by default; the mask's own draw buffer is turned
        // on by SetFloorMaskTarget around the opaque pass alone (REQ-SCN-084).
        DrawBuffers(false);

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
        ReleaseSamples();
        Samples = 1;

        if (_maskTexture != 0)
        {
            _gl.DeleteTexture(_maskTexture);
            _maskTexture = 0;
        }

        if (_depthTexture != 0)
        {
            _gl.DeleteTexture(_depthTexture);
            _depthTexture = 0;
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
