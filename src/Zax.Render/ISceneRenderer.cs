namespace Zax.Render;

/// <summary>
/// Draws one frame of the 3D scene into a host-provided render target.
/// </summary>
/// <remarks>
/// Implementations bind <see cref="RenderTarget.Framebuffer"/> themselves and leave it bound
/// on return; the host rebinds whatever it needs afterwards (blit, UI pass, swap). No
/// process-wide state is kept, so several independent instances can coexist
/// (SPEC-06 REQ-SCN-057, SPEC-01 REQ-ARCH-003).
/// </remarks>
public interface ISceneRenderer : IDisposable
{
    /// <summary>What the last rendered frame cost (REQ-SCN-064).</summary>
    RenderStatistics Statistics { get; }

    /// <summary>Draws one frame of <paramref name="scene"/> into <paramref name="target"/>.</summary>
    /// <param name="target">The framebuffer and size supplied by the host.</param>
    /// <param name="scene">
    /// The state of the scene for this frame: settings, camera, light, clip margins and the tile
    /// list of the displayed frame (SPEC-04B REQ-E3D-641).
    /// </param>
    void Render(in RenderTarget target, SceneState scene);

    /// <summary>
    /// Drops every cached GPU mesh and texture. The host calls it when the 3D engine rebuilds
    /// its geometry wholesale, for instance after a change of mesh algorithm (REQ-E3D-340).
    /// </summary>
    void InvalidateCaches();
}
