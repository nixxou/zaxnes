// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`).
// The MIT licence is compatible with the GPL-3.0 this reimplementation is distributed under.

using System.Numerics;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// The <b>MSVO</b> mode of SPEC-06 REQ-SCN-083 (AMD-055), <c>spl.ao</c> = 1: a multi-scale
/// screen-space ambient occlusion ported from the public MIT-licensed effect of Microsoft's
/// MiniEngine and adapted to OpenGL 3.3 fragment passes. See <see cref="MsvoShaders"/> for what
/// is kept of the original and what is adapted.
/// </summary>
/// <remarks>
/// <para>
/// The pass sits exactly where the Depth occlusion of <see cref="OcclusionPass"/> does — after
/// the opaque objects and the reflections, before the sorted transparent ones (REQ-SCN-058
/// amended) — and writes the same thing: an occlusion <c>O</c> that the apply pass multiplies by
/// the ceiling <c>0.1 × ao.strength</c> and applies to the scene's linear colour before the sRGB
/// encode.
/// </para>
/// <para>
/// <c>ao.thickness</c> acts here and only here (REQ-SCN-079): it is the rejection fall-off of
/// the original, the assumed thickness of an occluder known only by its depth. At 1 a sample
/// nearer than the point is treated as a thin screen and is faded off; at 5 it is a solid volume
/// that occludes all the way back. The darkening can therefore only grow with it.
/// </para>
/// </remarks>
internal sealed class MsvoPass : IDisposable
{
    /// <summary>Levels of the depth pyramid for Low, Medium and High (REQ-SCN-083).</summary>
    private static readonly int[] LevelsByQuality = [2, 3, 3];

    /// <summary>
    /// The checkerboard pattern of <c>AoRenderCS.hlsli</c>: the offsets, in fifths of the
    /// radius, of the sample pairs. Low keeps the four cheapest entries.
    /// </summary>
    private static readonly (int X, int Y)[] Pattern =
        [(2, 0), (4, 0), (1, 1), (2, 2), (3, 3), (1, 3), (2, 4)];

    /// <summary>Entries of <see cref="Pattern"/> the Low quality keeps.</summary>
    private static readonly int[] LowPattern = [0, 2, 3, 5];

    private readonly GL _gl;

    private ShaderProgram? _linearize;
    private ShaderProgram? _downsample;
    private ShaderProgram? _occlusion;
    private ShaderProgram? _upsample;
    private ShaderProgram? _apply;
    private uint _vertexArray;

    // Per level: the linear depth, the level's own occlusion, and the combination of it with
    // everything coarser. The coarsest level needs no combination buffer.
    private uint[] _depthTextures = [];
    private uint[] _aoTextures = [];
    private uint[] _combinedTextures = [];
    private uint[] _depthBuffers = [];
    private uint[] _aoBuffers = [];
    private uint[] _combinedBuffers = [];
    private int[] _widths = [];
    private int[] _heights = [];
    private int _levels;
    private bool _disposed;

    /// <summary>Creates the pass; its buffers are allocated on the first use.</summary>
    public MsvoPass(GL gl)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
    }

    /// <summary>Width of the finest level in pixels, 0 before the first run.</summary>
    public int Width => _widths.Length > 0 ? _widths[0] : 0;

    /// <summary>Height of the finest level in pixels.</summary>
    public int Height => _heights.Length > 0 ? _heights[0] : 0;

    /// <summary>Levels of the pyramid the last run used.</summary>
    public int Levels => _levels;

    /// <summary>
    /// Runs the pyramid, the per-level occlusions, the upsample chain and the multiply into the
    /// scene, and leaves <paramref name="buffer"/>'s framebuffer bound at its own size.
    /// </summary>
    /// <param name="buffer">The scene's linear buffer, whose depth is read.</param>
    /// <param name="style">The style settings, read for the four <c>ao.*</c> parameters.</param>
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

        if (style.EffectiveOcclusion != AmbientOcclusionMode.Msvo
            || buffer.DepthTexture == 0
            || !Matrix4x4.Invert(view.Projection, out Matrix4x4 inverse))
        {
            return false;
        }

        // REQ-SCN-080, fourth bullet: the pass works on the resolved depth.
        buffer.ResolveDepth();

        // REQ-SCN-083: half of each dimension of the current render target except at High.
        int quality = Math.Clamp(style.AoQuality, 0, 2);
        bool full = quality >= 2;
        int width = Math.Max(1, full ? buffer.Width : buffer.Width / 2);
        int height = Math.Max(1, full ? buffer.Height : buffer.Height / 2);
        if (!Allocate(width, height, LevelsByQuality[quality]))
        {
            return false;
        }

        _gl.Disable(EnableCap.ScissorTest);
        _gl.Disable(EnableCap.Blend);
        _gl.Disable(EnableCap.CullFace);
        _gl.Disable(EnableCap.DepthTest);
        _gl.DepthMask(false);
        _gl.ColorMask(true, true, true, true);

        BuildPyramid(buffer, inverse);
        BuildOcclusion(style, view, sceneScale);
        Combine();
        Apply(buffer, style);

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
        _linearize?.Dispose();
        _downsample?.Dispose();
        _occlusion?.Dispose();
        _upsample?.Dispose();
        _apply?.Dispose();
        if (_vertexArray != 0)
        {
            _gl.DeleteVertexArray(_vertexArray);
            _vertexArray = 0;
        }

        Release();
    }

    /// <summary>The pyramid of linear depths, each level half the previous one.</summary>
    private void BuildPyramid(LinearColorBuffer buffer, Matrix4x4 inverse)
    {
        Bind(_depthBuffers[0], 0);
        ShaderProgram linearize = _linearize ??= ShaderProgram.Create(
            _gl, "msvo-linearize", SceneShaders.ResolveVertex, MsvoShaders.LinearizeFragment);
        linearize.Use();
        linearize.Set("uDepth", 0);
        linearize.Set("uInverseProjection", inverse);
        linearize.Set("uDepthTexel", new Vector2(1f / buffer.Width, 1f / buffer.Height));
        Texture(0, buffer.DepthTexture);
        FullScreen();

        ShaderProgram downsample = _downsample ??= ShaderProgram.Create(
            _gl, "msvo-downsample", SceneShaders.ResolveVertex, MsvoShaders.DownsampleFragment);
        for (int level = 1; level < _levels; level++)
        {
            Bind(_depthBuffers[level], level);
            downsample.Use();
            downsample.Set("uSource", 0);
            Texture(0, _depthTextures[level - 1]);
            FullScreen();
        }
    }

    /// <summary>The occlusion of each level, each on its own share of the radius.</summary>
    private void BuildOcclusion(StyleSettings style, in CameraView view, float sceneScale)
    {
        ShaderProgram occlusion = _occlusion ??= ShaderProgram.Create(
            _gl, "msvo-occlusion", SceneShaders.ResolveVertex, MsvoShaders.OcclusionFragment);
        occlusion.Use();
        occlusion.Set("uDepth", 0);
        occlusion.Set("uFalloff", -1f / style.OcclusionThickness);

        int quality = Math.Clamp(style.AoQuality, 0, 2);
        Vector4[] pattern = BuildPattern(quality);
        occlusion.Set("uPatternSize", pattern.Length);
        for (int i = 0; i < pattern.Length; i++)
        {
            occlusion.SetArray("uPattern", i, pattern[i]);
        }

        float radius = style.OcclusionRadius(sceneScale);
        for (int level = 0; level < _levels; level++)
        {
            Bind(_aoBuffers[level], level);

            // The coarsest level covers the whole radius r, each finer one half of the one
            // below it: the fine contact and the wide darkening the mode is about.
            occlusion.Set("uRadius", radius / (1 << (_levels - 1 - level)));

            // The focal length in pixels of this level: half its height over tan(fov / 2),
            // which the projection of REQ-SCN-004 carries in M22.
            occlusion.Set("uFocal", 0.5f * _heights[level] * view.Projection.M22);
            occlusion.Set("uTexel", new Vector2(1f / _widths[level], 1f / _heights[level]));
            Texture(0, _depthTextures[level]);
            FullScreen();
        }
    }

    /// <summary>The bilateral upsample chain, from the coarsest level to the finest.</summary>
    private void Combine()
    {
        ShaderProgram upsample = _upsample ??= ShaderProgram.Create(
            _gl, "msvo-upsample", SceneShaders.ResolveVertex, MsvoShaders.UpsampleFragment);
        upsample.Use();
        upsample.Set("uFineDepth", 0);
        upsample.Set("uCoarseDepth", 1);
        upsample.Set("uCoarseAo", 2);
        upsample.Set("uFineAo", 3);

        // REQ-SCN-083, scope: a pixel that holds no geometry is never darkened. The linearised
        // depth of such a pixel is the far plane itself; a hair under it is the test, so that
        // the interpolation of the pyramid cannot drag a background pixel back in.
        upsample.Set("uFarDistance", SceneConstants.FarPlane * 0.999f);

        for (int level = _levels - 2; level >= 0; level--)
        {
            int coarse = level + 1;
            Bind(_combinedBuffers[level], level);
            upsample.Set(
                "uCoarseTexel", new Vector2(1f / _widths[coarse], 1f / _heights[coarse]));
            upsample.Set("uHasFineAo", 1);
            upsample.Set("uFinal", level == 0 ? 1 : 0);

            Texture(0, _depthTextures[level]);
            Texture(1, _depthTextures[coarse]);
            Texture(2, coarse == _levels - 1 ? _aoTextures[coarse] : _combinedTextures[coarse]);
            Texture(3, _aoTextures[level]);
            FullScreen();
        }
    }

    /// <summary>
    /// The multiply into the scene, in linear space and before the sRGB encode: the same pass
    /// the Depth occlusion uses, so the ceiling <c>O ≤ 0.1 × ao.strength</c> is enforced in one
    /// place (REQ-SCN-083).
    /// </summary>
    private void Apply(LinearColorBuffer buffer, StyleSettings style)
    {
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, buffer.SceneFramebuffer);
        _gl.Viewport(0, 0, (uint)buffer.Width, (uint)buffer.Height);

        ShaderProgram apply = _apply ??= ShaderProgram.Create(
            _gl, "msvo-apply", SceneShaders.ResolveVertex, OcclusionShaders.ApplyFragment);
        apply.Use();
        apply.Set("uOcclusion", 0);
        apply.Set("uCeiling", style.OcclusionCeiling);
        Texture(0, _combinedTextures[0]);

        _gl.Enable(EnableCap.Blend);
        _gl.BlendFunc(BlendingFactor.DstColor, BlendingFactor.Zero);
        FullScreen();
        _gl.Disable(EnableCap.Blend);
        _gl.BlendFunc(BlendingFactor.SrcAlpha, BlendingFactor.OneMinusSrcAlpha);

        for (int unit = 3; unit >= 0; unit--)
        {
            Texture(unit, 0);
        }
    }

    /// <summary>
    /// The two tables of <c>SSAO.cpp</c>, built for the quality asked for: for each entry of the
    /// pattern, its offset in fifths of the radius, its normalised weight and the thickness of
    /// the sphere at that offset.
    /// </summary>
    /// <remarks>
    /// The weight of an entry is its sphere thickness times the number of samples it stands for
    /// — four for an axial or a diagonal entry, eight for an L-shaped one — and the whole table
    /// is normalised, so that a pair that nothing occludes contributes 2 and a lone plane
    /// contributes exactly 1.
    /// </remarks>
    private static Vector4[] BuildPattern(int quality)
    {
        int[] entries = quality == 0 ? LowPattern : [0, 1, 2, 3, 4, 5, 6];
        Vector4[] pattern = new Vector4[entries.Length];
        float total = 0f;
        for (int i = 0; i < entries.Length; i++)
        {
            (int x, int y) = Pattern[entries[i]];
            float u = x * 0.2f;
            float v = y * 0.2f;
            float thickness = MathF.Sqrt(MathF.Max(0f, 1f - (u * u) - (v * v)));

            // L-shaped entries stand for eight samples, axial and diagonal ones for four.
            float count = y != 0 && x != y ? 8f : 4f;
            float weight = count * thickness;
            total += weight;
            pattern[i] = new Vector4(x, y, weight, thickness);
        }

        for (int i = 0; i < pattern.Length; i++)
        {
            pattern[i].Z /= total;
        }

        return pattern;
    }

    private void Bind(uint framebuffer, int level)
    {
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, framebuffer);
        _gl.Viewport(0, 0, (uint)_widths[level], (uint)_heights[level]);
    }

    private void Texture(int unit, uint texture)
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

    private bool Allocate(int width, int height, int levels)
    {
        if (_levels == levels && _widths.Length > 0 && _widths[0] == width && _heights[0] == height)
        {
            return true;
        }

        Release();
        _levels = levels;
        _widths = new int[levels];
        _heights = new int[levels];
        _depthTextures = new uint[levels];
        _aoTextures = new uint[levels];
        _combinedTextures = new uint[levels];
        _depthBuffers = new uint[levels];
        _aoBuffers = new uint[levels];
        _combinedBuffers = new uint[levels];

        for (int level = 0; level < levels; level++)
        {
            _widths[level] = Math.Max(1, width >> level);
            _heights[level] = Math.Max(1, height >> level);

            if (!Attach(
                    ref _depthBuffers[level], ref _depthTextures[level], level,
                    InternalFormat.R16f, PixelFormat.Red, PixelType.Float)
                || !Attach(
                    ref _aoBuffers[level], ref _aoTextures[level], level,
                    InternalFormat.R8, PixelFormat.Red, PixelType.UnsignedByte))
            {
                Release();
                return false;
            }

            // The coarsest level is combined into the one below it, so it needs no buffer of
            // its own.
            if (level < levels - 1
                && !Attach(
                    ref _combinedBuffers[level], ref _combinedTextures[level], level,
                    InternalFormat.R8, PixelFormat.Red, PixelType.UnsignedByte))
            {
                Release();
                return false;
            }
        }

        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);
        return true;
    }

    private unsafe bool Attach(
        ref uint framebuffer,
        ref uint texture,
        int level,
        InternalFormat internalFormat,
        PixelFormat format,
        PixelType type)
    {
        framebuffer = _gl.GenFramebuffer();
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, framebuffer);

        texture = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, texture);
        _gl.TexImage2D(
            TextureTarget.Texture2D, 0, internalFormat,
            (uint)_widths[level], (uint)_heights[level], 0, format, type, null);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Linear);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Linear);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToEdge);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToEdge);
        _gl.FramebufferTexture2D(
            FramebufferTarget.Framebuffer, FramebufferAttachment.ColorAttachment0,
            TextureTarget.Texture2D, texture, 0);
        _gl.BindTexture(TextureTarget.Texture2D, 0);

        // A driver that refuses a buffer leaves the frame without the effect rather than
        // without an image (AMD-051 REQ-E3D-644, same spirit).
        return _gl.CheckFramebufferStatus(FramebufferTarget.Framebuffer) == GLEnum.FramebufferComplete;
    }

    private void Release()
    {
        foreach (uint[] textures in (uint[][])[_depthTextures, _aoTextures, _combinedTextures])
        {
            foreach (uint texture in textures)
            {
                if (texture != 0)
                {
                    _gl.DeleteTexture(texture);
                }
            }
        }

        foreach (uint[] buffers in (uint[][])[_depthBuffers, _aoBuffers, _combinedBuffers])
        {
            foreach (uint framebuffer in buffers)
            {
                if (framebuffer != 0)
                {
                    _gl.DeleteFramebuffer(framebuffer);
                }
            }
        }

        _depthTextures = [];
        _aoTextures = [];
        _combinedTextures = [];
        _depthBuffers = [];
        _aoBuffers = [];
        _combinedBuffers = [];
        _widths = [];
        _heights = [];
        _levels = 0;
    }
}
