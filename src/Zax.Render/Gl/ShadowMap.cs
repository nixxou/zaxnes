using System.Numerics;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// The single shadow map of SPEC-06 REQ-SCN-030: hard shadows, no cascade, 1024 texels of side
/// by default (DEC-R2-D4).
/// </summary>
internal sealed class ShadowMap : IDisposable
{
    private readonly GL _gl;
    private uint _framebuffer;
    private uint _depthTexture;

    /// <summary>Creates the map at <paramref name="size"/> texels of side.</summary>
    public ShadowMap(GL gl, int size)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        Size = Math.Max(64, size);
        Allocate();
    }

    /// <summary>Side of the map in texels.</summary>
    public int Size { get; private set; }

    /// <summary>The depth texture.</summary>
    public uint DepthTexture => _depthTexture;

    /// <summary>Reallocates the map when the setting changes.</summary>
    public void Resize(int size)
    {
        int clamped = Math.Max(64, size);
        if (clamped == Size)
        {
            return;
        }

        Release();
        Size = clamped;
        Allocate();
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

    private unsafe void Allocate()
    {
        _depthTexture = _gl.GenTexture();
        _gl.BindTexture(TextureTarget.Texture2D, _depthTexture);
        _gl.TexImage2D(
            TextureTarget.Texture2D, 0, InternalFormat.DepthComponent24,
            (uint)Size, (uint)Size, 0,
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

        if (status != GLEnum.FramebufferComplete)
        {
            Release();
            throw new InvalidOperationException($"The {Size}x{Size} shadow map is incomplete: {status}.");
        }
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
    /// (REQ-SCN-030), then fitted so that the 1024 texels are spent on what actually casts
    /// rather than on the empty space the sphere mostly contains (Q-019).
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
