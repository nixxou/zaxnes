using System.Numerics;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// The screen-space floor reflections of SPEC-06 REQ-SCN-084 (AMD-055): the reflected ray is
/// traced through the scene's resolved depth, the hit colour is blurred by the distance the ray
/// travelled, and the result is mixed into the scene's linear colour before the sRGB encode.
/// </summary>
/// <remarks>
/// <para>
/// The pass runs after the opaque objects and <b>before</b> the ambient occlusion and the sorted
/// transparent objects (REQ-SCN-058 amended, REQ-SCN-080), which is what decides its scope on
/// its own: the transparent objects and the HUD are drawn after, so they neither carry
/// reflections nor are reflected; only surfaces already in the depth and the colour can be.
/// </para>
/// <para>
/// A pixel reflects only where the floor mask of <see cref="LinearColorBuffer"/> says so — the
/// test of REQ-SCN-084: a face whose normal in the scene frame before curvature has
/// <c>+Y ≥ 0.9</c>, belonging to an object whose <c>xz</c> bounding box measures at least
/// <c>6 × s</c> units in both dimensions.
/// </para>
/// </remarks>
internal sealed class ReflectionPass : IDisposable
{
    /// <summary>
    /// March steps for Low, Medium, High and Ultra: « de l'ordre de 12, 24, 48 et 48 »
    /// (REQ-SCN-084). The numbers are not observable.
    /// </summary>
    private static readonly int[] StepsByQuality = [12, 24, 48, 48];

    /// <summary>Bisection refinements for the four qualities: « 2, 3, 5 et 5 ».</summary>
    private static readonly int[] RefinementsByQuality = [2, 3, 5, 5];

    private readonly GL _gl;

    private ShaderProgram? _trace;
    private ShaderProgram? _blur;
    private ShaderProgram? _apply;
    private uint _vertexArray;
    private uint[] _framebuffers = [];
    private uint[] _textures = [];
    private uint _travel;
    private int _width;
    private int _height;
    private bool _disposed;

    /// <summary>Creates the pass; its buffers are allocated on the first use.</summary>
    public ReflectionPass(GL gl)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
    }

    /// <summary>Width of the trace buffers in pixels, 0 before the first run.</summary>
    public int Width => _width;

    /// <summary>Height of the trace buffers in pixels.</summary>
    public int Height => _height;

    /// <summary>
    /// Runs the four sub-passes and leaves <paramref name="buffer"/>'s framebuffer bound at its
    /// own size, ready for what the frame still has to draw.
    /// </summary>
    /// <param name="buffer">The scene's linear buffer: colour, depth and floor mask.</param>
    /// <param name="style">The style settings, read for the four <c>ssr.*</c> parameters.</param>
    /// <param name="view">The camera view of the frame (REQ-SCN-004, REQ-SCN-007).</param>
    /// <param name="sceneScale">The scene scale <c>s</c> of REQ-SCN-002.</param>
    /// <returns>Whether the pass ran.</returns>
    public bool Draw(
        LinearColorBuffer buffer,
        StyleSettings style,
        in CameraView view,
        float sceneScale)
    {
        ArgumentNullException.ThrowIfNull(buffer);
        ArgumentNullException.ThrowIfNull(style);

        if (!style.EffectiveReflections
            || buffer.DepthTexture == 0
            || buffer.FloorMaskTexture == 0
            || !Matrix4x4.Invert(view.Projection, out Matrix4x4 inverse))
        {
            return false;
        }

        // REQ-SCN-080, fourth bullet: the pass works on the resolved colour, depth and mask.
        buffer.ResolveForScreenPass();

        // REQ-SCN-084: the trace runs at half of each dimension of the current render target
        // except in Ultra. The numbers are not observable.
        int quality = Math.Clamp(style.SsrQuality, 0, 3);
        bool full = quality >= 3;
        int width = Math.Max(1, full ? buffer.Width : buffer.Width / 2);
        int height = Math.Max(1, full ? buffer.Height : buffer.Height / 2);
        if (!Allocate(width, height))
        {
            return false;
        }

        Vector2 sceneTexel = new(1f / buffer.Width, 1f / buffer.Height);

        _gl.Disable(EnableCap.ScissorTest);
        _gl.Disable(EnableCap.Blend);
        _gl.Disable(EnableCap.CullFace);
        _gl.Disable(EnableCap.DepthTest);
        _gl.DepthMask(false);
        _gl.ColorMask(true, true, true, true);

        // 1. The trace itself, into the colour buffer and the travel buffer.
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffers[0]);
        _gl.Viewport(0, 0, (uint)_width, (uint)_height);

        ShaderProgram trace = _trace ??= ShaderProgram.Create(
            _gl, "reflection", SceneShaders.ResolveVertex, ReflectionShaders.TraceFragment);
        trace.Use();
        trace.Set("uDepth", 0);
        trace.Set("uColor", 1);
        trace.Set("uFloor", 2);
        trace.Set("uInverseProjection", inverse);
        trace.Set("uProjection", view.Projection);
        trace.Set("uDepthTexel", sceneTexel);
        trace.Set("uMaxLength", style.ReflectionLength(sceneScale));
        trace.Set("uIntensity", style.ReflectionIntensity);
        trace.Set("uSteps", StepsByQuality[quality]);
        trace.Set("uRefinements", RefinementsByQuality[quality]);

        Span<float> normal = stackalloc float[9];
        SceneToView(view.View, normal);
        trace.SetMatrix3("uViewNormal", normal);

        Bind(0, buffer.DepthTexture);
        Bind(1, buffer.ColorTexture);
        Bind(2, buffer.FloorMaskTexture);
        FullScreen();

        // 2 and 3. The separable blur, whose radius grows with the distance travelled.
        ShaderProgram blur = _blur ??= ShaderProgram.Create(
            _gl, "reflection-blur", SceneShaders.ResolveVertex, ReflectionShaders.BlurFragment);

        // The radius of REQ-SCN-084 is a fraction of the screen height; expressed in pixels of
        // this pass it is that same fraction of the pass's own height.
        float maxRadius = style.ReflectionBlur * _height;

        BlurAxis(blur, 1, _textures[0], new Vector2(1f / _width, 0f), maxRadius);
        BlurAxis(blur, 2, _textures[1], new Vector2(0f, 1f / _height), maxRadius);

        // 4. The mix into the scene, in linear space and before the sRGB encode (REQ-SCN-084).
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, buffer.SceneFramebuffer);
        _gl.Viewport(0, 0, (uint)buffer.Width, (uint)buffer.Height);

        ShaderProgram apply = _apply ??= ShaderProgram.Create(
            _gl, "reflection-apply", SceneShaders.ResolveVertex, ReflectionShaders.ApplyFragment);
        apply.Use();
        apply.Set("uReflection", 0);
        Bind(0, _textures[2]);

        _gl.Enable(EnableCap.Blend);
        _gl.BlendFuncSeparate(
            BlendingFactor.One, BlendingFactor.OneMinusSrcAlpha,
            BlendingFactor.Zero, BlendingFactor.One);
        FullScreen();
        _gl.Disable(EnableCap.Blend);
        _gl.BlendFunc(BlendingFactor.SrcAlpha, BlendingFactor.OneMinusSrcAlpha);

        for (int unit = 2; unit >= 0; unit--)
        {
            Bind(unit, 0);
        }

        _gl.Enable(EnableCap.DepthTest);
        _gl.DepthMask(true);
        return true;
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _trace?.Dispose();
        _blur?.Dispose();
        _apply?.Dispose();
        if (_vertexArray != 0)
        {
            _gl.DeleteVertexArray(_vertexArray);
            _vertexArray = 0;
        }

        Release();
    }

    /// <summary>
    /// The rotation that carries a <b>scene-frame</b> normal into the view frame. The scene
    /// frame is left-handed and the clip position mirrors <c>z</c> (REQ-SCN-001), so the mirror
    /// is folded in; the view matrix of REQ-SCN-007 is rigid, so its rotation is its own normal
    /// matrix. The cells are written row by row, the order <see cref="ShaderProgram.SetMatrix3"/>
    /// expects.
    /// </summary>
    private static void SceneToView(Matrix4x4 viewMatrix, Span<float> rowMajor)
    {
        // The view matrix applies to row vectors, so its upper-left 3 x 3 block has the camera
        // basis in its columns; the transpose takes a vector into the view frame.
        rowMajor[0] = viewMatrix.M11;
        rowMajor[1] = viewMatrix.M21;
        rowMajor[2] = -viewMatrix.M31;
        rowMajor[3] = viewMatrix.M12;
        rowMajor[4] = viewMatrix.M22;
        rowMajor[5] = -viewMatrix.M32;
        rowMajor[6] = viewMatrix.M13;
        rowMajor[7] = viewMatrix.M23;
        rowMajor[8] = -viewMatrix.M33;
    }

    private void BlurAxis(
        ShaderProgram blur, int targetIndex, uint source, Vector2 step, float maxRadius)
    {
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffers[targetIndex]);
        _gl.Viewport(0, 0, (uint)_width, (uint)_height);

        blur.Use();
        blur.Set("uSource", 0);
        blur.Set("uTravel", 1);
        blur.Set("uStep", step);
        blur.Set("uMaxRadius", maxRadius);

        Bind(0, source);
        Bind(1, _travel);
        FullScreen();
    }

    private void Bind(int unit, uint texture)
    {
        _gl.ActiveTexture(TextureUnit.Texture0 + unit);
        _gl.BindTexture(TextureTarget.Texture2D, texture);
    }

    private void FullScreen()
    {
        if (_vertexArray == 0)
        {
            _vertexArray = _gl.GenVertexArray();
        }

        _gl.BindVertexArray(_vertexArray);
        _gl.DrawArrays(PrimitiveType.Triangles, 0, 3);
        _gl.BindVertexArray(0);
    }

    private bool Allocate(int width, int height)
    {
        if (_framebuffers.Length == 3 && width == _width && height == _height)
        {
            return true;
        }

        Release();
        _width = width;
        _height = height;
        _framebuffers = new uint[3];
        _textures = new uint[3];

        for (int i = 0; i < 3; i++)
        {
            _framebuffers[i] = _gl.GenFramebuffer();
            _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffers[i]);

            // Half floats: the reflection is a premultiplied *linear* colour, which eight bits
            // would band in the darks like the scene buffer itself.
            _textures[i] = Texture(InternalFormat.Rgba16f, PixelFormat.Rgba, PixelType.Float, width, height, linear: true);
            _gl.FramebufferTexture2D(
                FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment0,
                TextureTarget.Texture2D, _textures[i], 0);

            if (i == 0)
            {
                // The travelled distance, as a fraction of the maximum length: one byte is
                // finer than the blur radius can resolve.
                _travel = Texture(InternalFormat.R8, PixelFormat.Red, PixelType.UnsignedByte, width, height, linear: false);
                _gl.FramebufferTexture2D(
                    FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment1,
                    TextureTarget.Texture2D, _travel, 0);

                Span<GLEnum> buffers = [GLEnum.ColorAttachment0, GLEnum.ColorAttachment1];
                _gl.DrawBuffers(2, buffers);
            }

            GLEnum status = _gl.CheckFramebufferStatus(FramebufferTarget.Framebuffer);
            if (status != GLEnum.FramebufferComplete)
            {
                // A driver that refuses the buffers leaves the frame without the effect rather
                // than without an image (AMD-051 REQ-E3D-644, same spirit).
                Release();
                return false;
            }
        }

        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);
        return true;
    }

    private unsafe uint Texture(
        InternalFormat internalFormat,
        PixelFormat format,
        PixelType type,
        int width,
        int height,
        bool linear)
    {
        uint texture = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, texture);
        _gl.TexImage2D(
            TextureTarget.Texture2D, 0, internalFormat,
            (uint)width, (uint)height, 0, format, type, null);

        GLEnum filter = linear ? GLEnum.Linear : GLEnum.Nearest;
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)filter);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)filter);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
        _gl.BindTexture(TextureTarget.Texture2D, 0);
        return texture;
    }

    private void Release()
    {
        if (_travel != 0)
        {
            _gl.DeleteTexture(_travel);
            _travel = 0;
        }

        foreach (uint texture in _textures)
        {
            if (texture != 0)
            {
                _gl.DeleteTexture(texture);
            }
        }

        foreach (uint framebuffer in _framebuffers)
        {
            if (framebuffer != 0)
            {
                _gl.DeleteFramebuffer(framebuffer);
            }
        }

        _textures = [];
        _framebuffers = [];
        _width = 0;
        _height = 0;
    }
}
