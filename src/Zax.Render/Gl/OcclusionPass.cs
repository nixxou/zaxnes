using System.Numerics;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// The screen-space ambient occlusion of SPEC-06 REQ-SCN-083 (AMD-055): the occlusion is
/// estimated from the resolved depth of the scene, smoothed, and multiplied into the scene's
/// linear colour before the sRGB encode.
/// </summary>
/// <remarks>
/// <para>
/// The pass runs after the opaque objects and before the sorted transparent ones (REQ-SCN-058
/// amended, REQ-SCN-080), which is what decides its scope on its own: the skybox is drawn
/// before and is at infinity, so it never occludes; the transparent objects and the HUD are
/// drawn after, so they neither occlude nor are darkened.
/// </para>
/// <para>
/// <c>spl.ao</c> = 1 (MSVO) is rendered exactly as <c>spl.ao</c> = 2 (Depth) — the declared
/// fallback <b>(D)</b> of REQ-SCN-083, in force as long as the multi-scale port of the
/// MiniEngine is not available. <c>ao.thickness</c> has no effect under that fallback
/// (REQ-SCN-079).
/// </para>
/// </remarks>
internal sealed class OcclusionPass : IDisposable
{
    /// <summary>
    /// Samples per pixel for Low, Medium and High: « de l'ordre de 6, 10 et 16 »
    /// (REQ-SCN-083). The numbers are not observable.
    /// </summary>
    private static readonly int[] SamplesByQuality = [6, 10, OcclusionShaders.MaxSamples];

    private readonly GL _gl;
    private readonly Vector4[] _kernel = new Vector4[OcclusionShaders.MaxSamples];

    private ShaderProgram? _occlusion;
    private ShaderProgram? _blur;
    private ShaderProgram? _apply;
    private uint _vertexArray;
    private uint[] _framebuffers = [];
    private uint[] _textures = [];
    private int _width;
    private int _height;
    private bool _disposed;

    /// <summary>Creates the pass; its buffers are allocated on the first use.</summary>
    public OcclusionPass(GL gl)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        BuildKernel();
    }

    /// <summary>Width of the occlusion buffers in pixels, 0 before the first run.</summary>
    public int Width => _width;

    /// <summary>Height of the occlusion buffers in pixels.</summary>
    public int Height => _height;

    /// <summary>
    /// Runs the three sub-passes and leaves <paramref name="scene"/>'s framebuffer bound at its
    /// own size, ready for what the frame still has to draw.
    /// </summary>
    /// <param name="buffer">The scene's linear buffer, whose depth is read.</param>
    /// <param name="style">The style settings, read for the four <c>ao.*</c> parameters.</param>
    /// <param name="projection">The projection matrix of the frame (REQ-SCN-004).</param>
    /// <param name="sceneScale">The scene scale <c>s</c> of REQ-SCN-002.</param>
    /// <returns>Whether the pass ran.</returns>
    public bool Draw(
        LinearColorBuffer buffer,
        StyleSettings style,
        Matrix4x4 projection,
        float sceneScale)
    {
        ArgumentNullException.ThrowIfNull(buffer);
        ArgumentNullException.ThrowIfNull(style);

        if (style.EffectiveOcclusion == AmbientOcclusionMode.Off
            || buffer.DepthTexture == 0
            || !Matrix4x4.Invert(projection, out Matrix4x4 inverse))
        {
            return false;
        }

        // REQ-SCN-080, fourth bullet: the pass works on the resolved depth.
        buffer.ResolveDepth();

        // REQ-SCN-083: the occlusion is computed at half of each dimension of the current
        // render target except at the High quality. The numbers are not observable.
        bool full = style.AoQuality >= 2;
        int width = Math.Max(1, full ? buffer.Width : buffer.Width / 2);
        int height = Math.Max(1, full ? buffer.Height : buffer.Height / 2);
        if (!Allocate(width, height))
        {
            return false;
        }

        Vector2 sceneTexel = new(1f / buffer.Width, 1f / buffer.Height);
        Vector2 texel = new(1f / _width, 1f / _height);

        _gl.Disable(EnableCap.ScissorTest);
        _gl.Disable(EnableCap.Blend);
        _gl.Disable(EnableCap.CullFace);
        _gl.Disable(EnableCap.DepthTest);
        _gl.DepthMask(false);
        _gl.ColorMask(true, true, true, true);

        // 1. The occlusion itself.
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffers[0]);
        _gl.Viewport(0, 0, (uint)_width, (uint)_height);

        ShaderProgram occlusion = _occlusion ??= ShaderProgram.Create(
            _gl, "occlusion", SceneShaders.ResolveVertex, OcclusionShaders.OcclusionFragment);
        occlusion.Use();
        occlusion.Set("uDepth", 0);
        occlusion.Set("uInverseProjection", inverse);
        occlusion.Set("uProjection", projection);
        occlusion.Set("uDepthTexel", sceneTexel);
        occlusion.Set("uRadius", style.OcclusionRadius(sceneScale));
        occlusion.Set("uSamples", SamplesByQuality[Math.Clamp(style.AoQuality, 0, 2)]);
        for (int i = 0; i < _kernel.Length; i++)
        {
            occlusion.SetArray("uKernel", i, _kernel[i]);
        }

        _gl.ActiveTexture(TextureUnit.Texture0);
        _gl.BindTexture(TextureTarget.Texture2D, buffer.DepthTexture);
        FullScreen();

        // 2. The depth-aware blur, into the second buffer.
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffers[1]);
        _gl.Viewport(0, 0, (uint)_width, (uint)_height);

        ShaderProgram blur = _blur ??= ShaderProgram.Create(
            _gl, "occlusion-blur", SceneShaders.ResolveVertex, OcclusionShaders.BlurFragment);
        blur.Use();
        blur.Set("uDepth", 0);
        blur.Set("uOcclusion", 1);
        blur.Set("uInverseProjection", inverse);
        blur.Set("uDepthTexel", sceneTexel);
        blur.Set("uTexel", texel);

        // A wider blur at the lower qualities, whose kernel is noisier (REQ-SCN-083, review:
        // « Low pouvant être plus bruité », but never a band or a fixed pattern).
        blur.Set("uRadiusPixels", style.AoQuality >= 2 ? 2 : 3);

        _gl.ActiveTexture(TextureUnit.Texture0);
        _gl.BindTexture(TextureTarget.Texture2D, buffer.DepthTexture);
        _gl.ActiveTexture(TextureUnit.Texture1);
        _gl.BindTexture(TextureTarget.Texture2D, _textures[0]);
        FullScreen();

        // 3. The multiply into the scene, in linear space and before the sRGB encode
        // (REQ-SCN-083). The scene target may be multisampled, so the factor is written with a
        // blend rather than by reading the colour back (REQ-SCN-057).
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, buffer.SceneFramebuffer);
        _gl.Viewport(0, 0, (uint)buffer.Width, (uint)buffer.Height);

        ShaderProgram apply = _apply ??= ShaderProgram.Create(
            _gl, "occlusion-apply", SceneShaders.ResolveVertex, OcclusionShaders.ApplyFragment);
        apply.Use();
        apply.Set("uOcclusion", 0);
        apply.Set("uCeiling", style.OcclusionCeiling);

        _gl.ActiveTexture(TextureUnit.Texture0);
        _gl.BindTexture(TextureTarget.Texture2D, _textures[1]);
        _gl.Enable(EnableCap.Blend);
        _gl.BlendFunc(BlendingFactor.DstColor, BlendingFactor.Zero);
        FullScreen();
        _gl.Disable(EnableCap.Blend);

        _gl.ActiveTexture(TextureUnit.Texture1);
        _gl.BindTexture(TextureTarget.Texture2D, 0);
        _gl.ActiveTexture(TextureUnit.Texture0);
        _gl.BindTexture(TextureTarget.Texture2D, 0);

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
        _occlusion?.Dispose();
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
    /// The kernel of REQ-SCN-083: directions of the hemisphere about <c>+Z</c>, packed towards
    /// the point so that the samples near it weigh more than the ones at the rim (Chapman 2013).
    /// </summary>
    /// <remarks>
    /// Built once, with a fixed sequence rather than a random one: the image of a still camera
    /// must not move from frame to frame, and a test must be able to repeat a capture.
    /// </remarks>
    private void BuildKernel()
    {
        // The golden-angle spiral: it spreads the directions evenly over the hemisphere without
        // a lattice of its own, which a fixed pattern of the kernel would show as a texture.
        const float GoldenAngle = 2.39996323f;
        for (int i = 0; i < _kernel.Length; i++)
        {
            float cosine = 1f - ((i + 0.5f) / _kernel.Length);
            float sine = MathF.Sqrt(1f - (cosine * cosine));
            float angle = i * GoldenAngle;

            // Packed towards the origin: the scale grows as the square of the rank, so most of
            // the samples sit close to the point and the contact darkening is the sharpest part.
            float scale = 0.1f + (0.9f * ((i + 1f) / _kernel.Length) * ((i + 1f) / _kernel.Length));
            _kernel[i] = new Vector4(
                MathF.Cos(angle) * sine * scale,
                MathF.Sin(angle) * sine * scale,
                cosine * scale,
                0f);
        }
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
        if (_framebuffers.Length == 2 && width == _width && height == _height)
        {
            return true;
        }

        Release();
        _width = width;
        _height = height;
        _framebuffers = new uint[2];
        _textures = new uint[2];

        for (int i = 0; i < 2; i++)
        {
            _framebuffers[i] = _gl.GenFramebuffer();
            _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffers[i]);

            _textures[i] = _gl.GenTexture();
            _gl.BindTexture(TextureTarget.Texture2D, _textures[i]);

            // R8 holds the occlusion to a 256th, which is finer than the sRGB step the capture
            // is read on; linear filtering is what upsamples the half-size buffer.
            unsafe
            {
                _gl.TexImage2D(
                    TextureTarget.Texture2D, 0, InternalFormat.R8,
                    (uint)width, (uint)height, 0,
                    PixelFormat.Red, PixelType.UnsignedByte, null);
            }

            _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Linear);
            _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Linear);
            _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
            _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
            _gl.FramebufferTexture2D(
                FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment0,
                TextureTarget.Texture2D, _textures[i], 0);
            _gl.BindTexture(TextureTarget.Texture2D, 0);

            GLEnum status = _gl.CheckFramebufferStatus(FramebufferTarget.Framebuffer);
            if (status != GLEnum.FramebufferComplete)
            {
                // A driver that refuses the buffer leaves the frame without the effect rather
                // than without an image (AMD-051 REQ-E3D-644, same spirit).
                Release();
                return false;
            }
        }

        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);
        return true;
    }

    private void Release()
    {
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
