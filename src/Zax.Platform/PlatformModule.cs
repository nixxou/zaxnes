namespace Zax.Platform;

/// <summary>
/// P0 scaffolding marker for the platform layer.
/// </summary>
/// <remarks>
/// This is the only assembly in the solution that is allowed to own a window, a GL context,
/// physical input devices or an audio device. Everything above it (lots L6, L8, L9) talks to
/// abstractions so that a libretro frontend can replace this layer wholesale.
/// </remarks>
public static class PlatformModule
{
    /// <summary>Stable identifier of this module.</summary>
    public const string Name = "Zax.Platform";

    /// <summary>Short human-readable role of this module.</summary>
    public const string Role = "Window, GL context, input, audio device";
}
