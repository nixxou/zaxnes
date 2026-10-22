using System.Numerics;
using Zax.Engine3D;
using Zax.Render.Assets;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// Draws the Gradient and Fantasy skyboxes of SPEC-06 REQ-SCN-049 to REQ-SCN-051.
/// </summary>
/// <remarks>
/// <para>
/// The programs and the empty vertex array are built on first use, so a renderer that never
/// leaves the Default or Color skybox pays nothing.
/// </para>
/// <para>
/// Without an <see cref="ISceneAssets"/> the sky fields are computed in the shader
/// (AMD-024 REQ-PUB-003), which is what the publishable build does: no cubemap, no image file,
/// and no cubemap cache is ever created. With one, the cubemaps SPEC-06 names are sampled, which
/// is both the user-supplied asset folder of <c>--assets</c> and the uniform test cubemaps the
/// acceptance criteria of REQ-SCN-051 inject.
/// </para>
/// </remarks>
internal sealed class SkyboxPass : IDisposable
{
    private readonly GL _gl;
    private readonly CubemapCache? _cubemaps;
    private readonly string _fields;

    private ShaderProgram? _gradient;
    private ShaderProgram? _fantasy;
    private uint _vertexArray;
    private bool _disposed;

    /// <summary>Creates the pass for the current context.</summary>
    /// <param name="gl">The GL API.</param>
    /// <param name="assets">
    /// Where the cubemaps come from, or null to compute the sky fields in the shader.
    /// </param>
    public SkyboxPass(GL gl, ISceneAssets? assets)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        _cubemaps = assets is null ? null : new CubemapCache(gl, assets);
        _fields = assets is null
            ? ProceduralSkyShaders.Procedural
            : ProceduralSkyShaders.Textured;
    }

    /// <summary>Whether the sky fields are computed rather than sampled (REQ-PUB-003).</summary>
    public bool IsProcedural => _cubemaps is null;

    /// <summary>
    /// The handle of a cubemap, uploading it on first use, or 0 when there is no asset source.
    /// The environment reflection of TV 2 (REQ-SCN-054) shares the cache with the skyboxes.
    /// </summary>
    public uint Cubemap(string name) => _cubemaps?.Get(name) ?? 0;

    /// <summary>
    /// Draws the skybox of <paramref name="scene"/> over the whole target, with the depth test
    /// off so that everything drawn afterwards covers it (REQ-SCN-058).
    /// </summary>
    /// <param name="scene">The scene state.</param>
    /// <param name="view">The camera of the frame.</param>
    /// <returns>True when a skybox was drawn.</returns>
    public bool Draw(SceneState scene, in CameraView view)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        ArgumentNullException.ThrowIfNull(scene);

        SkyboxKind kind = scene.Settings.Skybox;
        if (kind is not (SkyboxKind.Gradient or SkyboxKind.Fantasy))
        {
            return false;
        }

        ShaderProgram program = kind == SkyboxKind.Gradient ? Gradient() : Fantasy();
        program.Use();

        // The direction is rebuilt in scene coordinates: f + x·tan(fov/2)·aspect·right +
        // y·tan(fov/2)·up (REQ-SCN-004).
        float tan = MathF.Tan(view.Fov * MathF.PI / 360f);
        program.Set("uRight", view.Right);
        program.Set("uUp", view.Up);
        program.Set("uForward", view.Forward);
        program.Set("uTanHalf", new Vector2(tan * view.Aspect, tan));

        BindBodies(program, scene.Settings.SkyBodies);

        if (kind == SkyboxKind.Gradient)
        {
            BindGradient(program, scene);
        }
        else
        {
            BindFantasy(program, scene);
        }

        _gl.Disable(EnableCap.DepthTest);
        _gl.DepthMask(false);
        _gl.Disable(EnableCap.Blend);
        _gl.Disable(EnableCap.CullFace);
        _gl.BindVertexArray(VertexArray());
        _gl.DrawArrays(PrimitiveType.Triangles, 0, 3);
        _gl.BindVertexArray(0);
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
        _gradient?.Dispose();
        _fantasy?.Dispose();
        _cubemaps?.Dispose();
        if (_vertexArray != 0)
        {
            _gl.DeleteVertexArray(_vertexArray);
            _vertexArray = 0;
        }
    }

    /// <summary>
    /// The sun, the moon and the clouds of REQ-PUB-003. Their gains are 0 unless the host has
    /// set them, which is the « disabled » of REQ-SCN-050 (Q-064).
    /// </summary>
    private static void BindBodies(ShaderProgram program, SkyBodySettings bodies)
    {
        program.Set("uSunDirection", bodies.SunDirection);
        program.Set("uSunLinear", SceneColor.ToLinear(bodies.SunColor));
        program.Set("uSunGain", bodies.SunGain);
        program.Set("uSunCos", DiscCosines(bodies.SunRadiusDegrees, bodies.DiscSoftness));

        program.Set("uMoonDirection", bodies.MoonDirection);
        program.Set("uMoonLinear", SceneColor.ToLinear(bodies.MoonColor));
        program.Set("uMoonGain", bodies.MoonGain);
        program.Set("uMoonCos", DiscCosines(bodies.MoonRadiusDegrees, bodies.DiscSoftness));

        program.Set("uCloudLinear", SceneColor.ToLinear(bodies.CloudColor));
        program.Set("uCloudGain", bodies.CloudGain);
        program.Set("uCloudCoverage", bodies.CloudCoverage);
        program.Set("uCloudScale", bodies.CloudScale);
    }

    /// <summary>
    /// The cosines of the inner and the outer cone of a disc of angular radius
    /// <paramref name="degrees"/>, feathered by <paramref name="softness"/>.
    /// </summary>
    private static Vector2 DiscCosines(float degrees, float softness)
    {
        float inner = degrees * MathF.PI / 180f;
        float outer = inner * (1f + Math.Clamp(softness, 0f, 4f));
        return new Vector2(MathF.Cos(inner), MathF.Cos(MathF.Min(outer, MathF.PI)));
    }

    private void BindGradient(ShaderProgram program, SceneState scene)
    {
        // REQ-SCN-052: the sky follows the colour of the **displayed** frame.
        Rgb24 m = scene.NesBackgroundColor;
        program.Set("uBackground", SceneColor.ToVector(m));
        program.Set("uStars", SkyGradient.StarsVisible(m) ? 1f : 0f);

        // REQ-SCN-050, REQ-SCN-059: the twinkle follows real time, 3° per second.
        float angle = scene.RealTime * SkyGradient.StarDegreesPerSecond * MathF.PI / 180f;
        program.Set("uStarAngle", angle % (2f * MathF.PI));

        if (_cubemaps is null)
        {
            return;
        }

        program.Set("uStarMap", 0);
        _gl.ActiveTexture(TextureUnit.Texture0);
        _gl.BindTexture(TextureTarget.TextureCubeMap, _cubemaps.Get("Stars_LP_01"));
    }

    private void BindFantasy(ShaderProgram program, SceneState scene)
    {
        Rgb24 f0 = scene.NesBackgroundColor;
        ReadOnlySpan<Rgb24> palette = scene.DrawList is { } list ? list.Palette : [];
        program.Set("uF0", SceneColor.ToVector(f0));
        program.Set("uF1", SceneColor.ToVector(Entry(palette, 1)));
        program.Set("uF2", SceneColor.ToVector(Entry(palette, 2)));
        program.Set("uF3", SceneColor.ToVector(Entry(palette, 3)));
        program.Set("uGamma", SkyGradient.Grey(f0));

        if (_cubemaps is null)
        {
            return;
        }

        program.Set("uStarfield", 0);
        program.Set("uDensity", 1);
        program.Set("uDiffusion", 2);
        _gl.ActiveTexture(TextureUnit.Texture0);
        _gl.BindTexture(TextureTarget.TextureCubeMap, _cubemaps.Get("Starfield"));
        _gl.ActiveTexture(TextureUnit.Texture1);
        _gl.BindTexture(TextureTarget.TextureCubeMap, _cubemaps.Get("NebulaDensity"));
        _gl.ActiveTexture(TextureUnit.Texture2);
        _gl.BindTexture(TextureTarget.TextureCubeMap, _cubemaps.Get("NebulaDiffusion"));
        _gl.ActiveTexture(TextureUnit.Texture0);
    }

    private static Rgb24 Entry(ReadOnlySpan<Rgb24> palette, int index)
        => index < palette.Length ? palette[index] : default;

    private ShaderProgram Gradient() => _gradient ??= ShaderProgram.Create(
        _gl, "skybox-gradient", SkyboxShaders.Vertex, SkyboxShaders.GradientFragment(_fields));

    private ShaderProgram Fantasy() => _fantasy ??= ShaderProgram.Create(
        _gl, "skybox-fantasy", SkyboxShaders.Vertex, SkyboxShaders.FantasyFragment(_fields));

    private uint VertexArray()
    {
        if (_vertexArray == 0)
        {
            _vertexArray = _gl.GenVertexArray();
        }

        return _vertexArray;
    }
}
