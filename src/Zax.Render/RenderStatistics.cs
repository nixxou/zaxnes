namespace Zax.Render;

/// <summary>
/// What the last rendered frame cost, for the performance report of SPEC-06 REQ-SCN-064 and the
/// debug overlay.
/// </summary>
public sealed class RenderStatistics
{
    /// <summary>Draw calls of the scene passes, shadow pass included.</summary>
    public int DrawCalls { get; internal set; }

    /// <summary>Triangles submitted, counted once per draw call.</summary>
    public long Triangles { get; internal set; }

    /// <summary>Opaque tiles drawn.</summary>
    public int OpaqueTiles { get; internal set; }

    /// <summary>Transparent tiles drawn.</summary>
    public int TransparentTiles { get; internal set; }

    /// <summary>
    /// Transparent objects drawn: the shape instances those tiles are grouped into for the depth
    /// pass and the distance sort (AMD-012).
    /// </summary>
    public int TransparentObjects { get; internal set; }

    /// <summary>HUD tiles drawn.</summary>
    public int HudTiles { get; internal set; }

    /// <summary>Tiles that went into the shadow map.</summary>
    public int ShadowCasters { get; internal set; }

    /// <summary>Meshes resident on the GPU.</summary>
    public int ResidentMeshes { get; internal set; }

    /// <summary>Meshes uploaded since the renderer was created.</summary>
    public int MeshUploads { get; internal set; }

    /// <summary>Object textures resident on the GPU.</summary>
    public int ResidentTextures { get; internal set; }

    /// <summary>Whether a shadow map was produced for this frame.</summary>
    public bool ShadowPass { get; internal set; }

    /// <summary>
    /// Side of the shadow map in texels the hardware granted (AMD-034 §2): the requested size,
    /// or the largest smaller one whose depth framebuffer came out complete.
    /// </summary>
    public int ShadowMapSize { get; internal set; } = SceneConstants.ShadowMapSize;

    /// <summary>Side of the shadow map in texels that was asked for (AMD-034 §2).</summary>
    public int RequestedShadowMapSize { get; internal set; } = SceneConstants.ShadowMapSize;

    /// <summary>Whether a skybox was drawn for this frame (REQ-SCN-049).</summary>
    public bool SkyboxPass { get; internal set; }

    /// <summary>
    /// Samples per pixel the scene pass was granted (AMD-032 §2): 1 when anti-aliasing is off
    /// or when the implementation refused what was asked for.
    /// </summary>
    public int AntiAliasSamples { get; internal set; } = 1;

    /// <summary>
    /// Opaque tiles turned transparent this frame because they hide the followed shape
    /// (REQ-SCN-022).
    /// </summary>
    public int Occluders { get; internal set; }

    internal void BeginFrame()
    {
        DrawCalls = 0;
        Triangles = 0;
        OpaqueTiles = 0;
        TransparentTiles = 0;
        TransparentObjects = 0;
        HudTiles = 0;
        ShadowCasters = 0;
        ShadowPass = false;
        SkyboxPass = false;
        AntiAliasSamples = 1;
        Occluders = 0;
    }
}
