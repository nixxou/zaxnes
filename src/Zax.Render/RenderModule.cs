namespace Zax.Render;

/// <summary>
/// P0 scaffolding marker for the renderer.
/// </summary>
public static class RenderModule
{
    /// <summary>Stable identifier of this module.</summary>
    public const string Name = "Zax.Render";

    /// <summary>Short human-readable role of this module.</summary>
    public const string Role = "OpenGL 3.3 core rendering into a host-provided target";

    /// <summary>Major version of the OpenGL core profile this renderer targets.</summary>
    public const int GlMajorVersion = 3;

    /// <summary>Minor version of the OpenGL core profile this renderer targets.</summary>
    public const int GlMinorVersion = 3;
}
