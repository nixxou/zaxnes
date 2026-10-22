namespace Zax.Platform;

/// <summary>
/// Everything the platform layer needs in order to open the application window.
/// </summary>
/// <param name="Title">Window title.</param>
/// <param name="Width">Client width in pixels.</param>
/// <param name="Height">Client height in pixels.</param>
/// <param name="VSync">
/// Buffer-swap synchronisation (SPEC-06 REQ-SCN-061, SPEC-09 <c>video.vsync</c>): 0 off, 1 every
/// refresh, 2 every other refresh. The window opens with 0 or 1 — the only two values the
/// windowing layer carries — and <see cref="HostWindow.ApplyPacing"/> puts the exact interval in
/// force once the GL context exists.
/// </param>
/// <param name="TargetFrameRate">
/// Display rate the loop aims at, in frames per second, or 0 for "as fast as the machine
/// allows" (SPEC-06 REQ-SCN-061). It is ignored when <paramref name="VSync"/> is on, since the
/// driver then sets the pace.
/// </param>
public readonly record struct HostWindowSettings(
    string Title,
    int Width,
    int Height,
    int VSync,
    int TargetFrameRate = 0)
{
    /// <summary>The P0 defaults: a 1280x720 window titled "Zaxnes", with vsync on.</summary>
    public static HostWindowSettings Default { get; } = new("Zaxnes", 1280, 720, VSync: 1);

    /// <summary>Whether the swaps are synchronised at all.</summary>
    public bool IsVSyncEnabled => VSync > 0;
}
