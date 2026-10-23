using System.Numerics;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// The single shadow map of SPEC-06 REQ-SCN-030: hard shadows, no cascade, 4096 texels of side
/// by default (AMD-034 §2, which replaces the 1024 of DEC-R2-D4).
/// </summary>
/// <remarks>
/// OpenGL 3.3 only guarantees <c>GL_MAX_TEXTURE_SIZE</c> ≥ 1024, so 4096 is not a size every
/// implementation has to accept. AMD-034 §2 asks for the largest one it does accept: the
/// requested size is clamped to what the driver advertises, then halved until the depth
/// framebuffer comes out complete — 4096, 2048, 1024. <see cref="Requested"/> and
/// <see cref="Size"/> differ exactly when that happened, which is what the host reports once.
/// </remarks>
internal sealed class ShadowMap : IDisposable
{
    private readonly GL _gl;
    private uint _framebuffer;
    private uint _depthTexture;
    private int _maxTextureSize = -1;

    /// <summary>Creates the map at <paramref name="size"/> texels of side.</summary>
    public ShadowMap(GL gl, int size)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        Allocate(size);
    }

    /// <summary>Side of the map in texels, as the hardware granted it.</summary>
    public int Size { get; private set; }

    /// <summary>Side of the map in texels, as it was asked for.</summary>
    public int Requested { get; private set; }

    /// <summary>The depth texture.</summary>
    public uint DepthTexture => _depthTexture;

    /// <summary>Reallocates the map when the setting changes.</summary>
    public void Resize(int size)
    {
        if (Math.Max(SceneConstants.MinShadowMapSize, size) == Requested)
        {
            return;
        }

        Release();
        Allocate(size);
    }

    /// <summary>Binds the map as the draw target and clears its depth.</summary>
    public void BeginPass()
    {
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffer);
        _gl.Viewport(0, 0, (uint)Size, (uint)Size);
        _gl.DepthMask(true);
        _gl.ClearDepth(1.0);
        _gl.Clear(ClearBufferMask.DepthBufferBit);
    }

    /// <inheritdoc/>
    public void Dispose() => Release();

    /// <summary>
    /// Allocates the largest map the hardware accepts, starting at <paramref name="size"/>
    /// and halving on refusal (AMD-034 §2).
    /// </summary>
    /// <param name="size">The size asked for, in texels.</param>
    private void Allocate(int size)
    {
        Requested = Math.Max(SceneConstants.MinShadowMapSize, size);

        // A driver that answers nonsense must not shrink the map for ever; the completeness
        // check of TryAllocate is the real guard.
        if (_maxTextureSize < 0)
        {
            _gl.GetInteger(GLEnum.MaxTextureSize, out int max);
            _maxTextureSize = Math.Max(SceneConstants.MinShadowMapSize, max);
        }

        foreach (int side in SceneConstants.ShadowMapSizes(Requested, _maxTextureSize))
        {
            if (TryAllocate(side))
            {
                Size = side;
                return;
            }

            Release();
        }

        throw new InvalidOperationException(
            $"No shadow map from {Requested} down to {SceneConstants.MinShadowMapSize} texels is complete.");
    }

    /// <summary>Builds the depth texture and its framebuffer; false when the pair is refused.</summary>
    /// <param name="size">Side of the map to try, in texels.</param>
    private unsafe bool TryAllocate(int size)
    {
        _depthTexture = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, _depthTexture);
        _gl.TexImage2D(
            TextureTarget.Texture2D, 0, InternalFormat.DepthComponent24,
            (uint)size, (uint)size, 0,
            PixelFormat.DepthComponent, PixelType.Float, null);

        // Hard shadows: nearest sampling, no soft filtering (REQ-SCN-030).
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMinFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureMagFilter, (int)GLEnum.Nearest);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapS, (int)GLEnum.ClampToBorder);
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureWrapT, (int)GLEnum.ClampToBorder);
        float[] border = [1f, 1f, 1f, 1f];
        _gl.TexParameter(TextureTarget.Texture2D, TextureParameterName.TextureBorderColor, border);
        _gl.BindTexture(TextureTarget.Texture2D, 0);

        _framebuffer = _gl.GenFramebuffer();
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, _framebuffer);
        _gl.FramebufferTexture2D(
            FramebufferTarget.Framebuffer, FramebufferAttachment.DepthAttachment,
            TextureTarget.Texture2D, _depthTexture, 0);
        _gl.DrawBuffer(DrawBufferMode.None);
        GLEnum status = _gl.CheckFramebufferStatus(FramebufferTarget.Framebuffer);
        _gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);

        return status == GLEnum.FramebufferComplete;
    }

    private void Release()
    {
        if (_framebuffer != 0)
        {
            _gl.DeleteFramebuffer(_framebuffer);
            _framebuffer = 0;
        }

        if (_depthTexture != 0)
        {
            _gl.DeleteTexture(_depthTexture);
            _depthTexture = 0;
        }
    }
}

/// <summary>
/// The light's view volume for one frame: the orthographic box that covers the shadow casters
/// within 512 units of the camera (REQ-SCN-030).
/// </summary>
/// <param name="ViewProjection">Light view-projection matrix, OpenGL frame.</param>
/// <param name="DepthRange">Depth of the box in scene units, used to scale the depth bias.</param>
/// <param name="Extent">Largest side of the box across the light, in scene units.</param>
/// <param name="HasContent">False when nothing casts a shadow this frame.</param>
internal readonly record struct ShadowVolume(
    Matrix4x4 ViewProjection,
    float DepthRange,
    float Extent,
    bool HasContent)
{
    /// <summary>The empty volume.</summary>
    public static ShadowVolume Empty { get; } = new(Matrix4x4.Identity, 1f, 0f, false);

    /// <summary>The side of one texel of a <paramref name="mapSize"/>-texel map, in scene units.</summary>
    /// <param name="mapSize">Side of the shadow map in texels.</param>
    public float TexelSize(int mapSize) => mapSize <= 0 ? 0f : Extent / mapSize;

    /// <summary>
    /// Builds the volume from the scene-space bounds of the casters, the eye and the light
    /// direction. The bounds are first restricted to the 512-unit sphere around the eye
    /// (REQ-SCN-030), then fitted so that the texels are spent on what actually casts rather
    /// than on the empty space the sphere mostly contains (Q-019).
    /// </summary>
    /// <param name="boundsMin">Lowest corner of the casters' bounds, scene frame.</param>
    /// <param name="boundsMax">Highest corner of the casters' bounds, scene frame.</param>
    /// <param name="eye">The eye, scene frame.</param>
    /// <param name="lightDirection">Propagation direction of the light, scene frame.</param>
    public static ShadowVolume Fit(
        Vector3 boundsMin,
        Vector3 boundsMax,
        Vector3 eye,
        Vector3 lightDirection)
    {
        Vector3 limitMin = eye - new Vector3(SceneConstants.ShadowDistance);
        Vector3 limitMax = eye + new Vector3(SceneConstants.ShadowDistance);
        Vector3 min = Vector3.Max(boundsMin, limitMin);
        Vector3 max = Vector3.Min(boundsMax, limitMax);
        if (max.X < min.X || max.Y < min.Y || max.Z < min.Z)
        {
            return Empty;
        }

        // A little slack so that the bias and the normal offset never push a caster out.
        min -= new Vector3(1f);
        max += new Vector3(1f);

        Vector3 forward = lightDirection.LengthSquared() > 0f
            ? Vector3.Normalize(lightDirection)
            : new Vector3(0f, -1f, 0f);
        Vector3 forwardGl = SceneCamera.ToGl(forward);
        Vector3 up = MathF.Abs(forwardGl.Y) > 0.99f ? Vector3.UnitZ : Vector3.UnitY;

        Vector3 center = (min + max) * 0.5f;
        Vector3 centerGl = SceneCamera.ToGl(center);
        float radius = Vector3.Distance(min, max) * 0.5f;
        radius = MathF.Max(radius, 1f);

        Matrix4x4 view = Matrix4x4.CreateLookAt(centerGl - (forwardGl * radius), centerGl, up);

        // Exact extents of the eight corners in light space, so the texels are not wasted.
        float minX = float.MaxValue;
        float minY = float.MaxValue;
        float minZ = float.MaxValue;
        float maxX = float.MinValue;
        float maxY = float.MinValue;
        float maxZ = float.MinValue;
        for (int corner = 0; corner < 8; corner++)
        {
            Vector3 point = new(
                (corner & 1) == 0 ? min.X : max.X,
                (corner & 2) == 0 ? min.Y : max.Y,
                (corner & 4) == 0 ? min.Z : max.Z);
            Vector3 light = Vector3.Transform(SceneCamera.ToGl(point), view);
            minX = MathF.Min(minX, light.X);
            minY = MathF.Min(minY, light.Y);
            minZ = MathF.Min(minZ, light.Z);
            maxX = MathF.Max(maxX, light.X);
            maxY = MathF.Max(maxY, light.Y);
            maxZ = MathF.Max(maxZ, light.Z);
        }

        // Light space looks along −Z, so the box spans [−maxZ, −minZ] in front of the eye.
        float near = MathF.Max(0.01f, -maxZ);
        float far = -minZ;
        if (far <= near)
        {
            far = near + 1f;
        }

        Matrix4x4 projection = Matrix4x4.CreateOrthographicOffCenter(minX, maxX, minY, maxY, near, far);
        return new ShadowVolume(
            Matrix4x4.Multiply(view, projection),
            far - near,
            MathF.Max(maxX - minX, maxY - minY),
            true);
    }
}
