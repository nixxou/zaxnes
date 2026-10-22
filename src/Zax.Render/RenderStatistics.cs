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

    /// <summary>Whether a skybox was drawn for this frame (REQ-SCN-049).</summary>
    public bool SkyboxPass { get; internal set; }

    /// <summary>Tunnel boxes drawn, 0 or 5 (REQ-SCN-053).</summary>
    public int TunnelBoxes { get; internal set; }

    /// <summary>Television meshes drawn (REQ-SCN-054).</summary>
    public int TvParts { get; internal set; }

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
        TunnelBoxes = 0;
        TvParts = 0;
        Occluders = 0;
    }
}
