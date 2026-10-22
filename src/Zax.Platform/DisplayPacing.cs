using System.Globalization;

namespace Zax.Platform;

/// <summary>
/// What the display pacing of SPEC-06 REQ-SCN-061 needs to drive on a window: the buffer-swap
/// synchronisation and the frame cap of the loop.
/// </summary>
/// <remarks>
/// It exists so that the rule — which swap interval each value of <c>video.vsync</c> means, and
/// when the cap applies — is testable without a GL context: the real implementation over
/// Silk.NET is <see cref="HostWindow.Display"/>, and the tests pass a recording double.
/// </remarks>
public interface IDisplaySurface
{
    /// <summary>
    /// Turns the windowing layer's own synchronisation on or off, which is a swap interval of 1
    /// or 0 (Silk.NET <c>IWindow.VSync</c>).
    /// </summary>
    void SetVSync(bool enabled);

    /// <summary>
    /// Caps the render loop at <paramref name="framesPerSecond"/>, or 0 for "as fast as the
    /// machine allows" (REQ-SCN-061).
    /// </summary>
    void SetFrameCap(int framesPerSecond);

    /// <summary>
    /// Asks the GL context for an exact swap interval, which is the only way to express
    /// <c>video.vsync</c> = 2 — one refresh out of two. False means the backend cannot: no
    /// context, or no such call on this platform, and the caller falls back (Q-066).
    /// </summary>
    /// <param name="interval">Refreshes to wait between two swaps.</param>
    bool TrySetSwapInterval(int interval);
}

/// <summary>What <see cref="DisplayPacing.Apply"/> actually put in force.</summary>
/// <param name="VSync">The <c>video.vsync</c> value that was asked for: 0, 1 or 2.</param>
/// <param name="SwapInterval">The swap interval in force: refreshes between two swaps.</param>
/// <param name="FrameCap">The frame cap in force, 0 when the driver paces the swaps.</param>
/// <param name="FellBackToEveryRefresh">
/// Whether <c>video.vsync</c> = 2 had to fall back to every refresh because the backend has no
/// swap-interval call (Q-066).
/// </param>
public readonly record struct DisplayPacingState(
    int VSync,
    int SwapInterval,
    int FrameCap,
    bool FellBackToEveryRefresh)
{
    /// <summary>One log line describing what is in force.</summary>
    public string Describe() => string.Create(
        CultureInfo.InvariantCulture,
        $"swap interval {SwapInterval} (video.vsync {VSync}"
            + $"{(FellBackToEveryRefresh ? ", every other refresh unavailable, Q-066" : string.Empty)}), "
            + $"frame cap {(FrameCap > 0 ? $"{FrameCap} fps" : "off")}");
}

/// <summary>
/// The display pacing of SPEC-06 REQ-SCN-061: <c>video.vsync</c> (SPEC-09, 0 off, 1 every
/// refresh, 2 every other refresh) and <c>video.frame_rate</c> (60, 90 or 120) applied to a
/// window, at start-up and again whenever either setting changes.
/// </summary>
/// <remarks>
/// With vsync on the driver paces the swaps, so the cap is left off; with vsync off the cap is
/// what keeps the loop at the rate the user asked for.
/// </remarks>
public static class DisplayPacing
{
    /// <summary>Highest <c>video.vsync</c> value SPEC-09 defines.</summary>
    public const int MaximumVSync = 2;

    /// <summary>
    /// Puts <paramref name="vsync"/> and <paramref name="targetFrameRate"/> in force on
    /// <paramref name="surface"/>.
    /// </summary>
    /// <param name="surface">The window to drive.</param>
    /// <param name="vsync">The <c>video.vsync</c> value; anything outside 0…2 is clamped.</param>
    /// <param name="targetFrameRate">The cap to use when vsync is off; 0 for uncapped.</param>
    /// <returns>What was put in force, for the log and the tests.</returns>
    public static DisplayPacingState Apply(IDisplaySurface surface, int vsync, int targetFrameRate)
    {
        ArgumentNullException.ThrowIfNull(surface);

        int requested = Math.Clamp(vsync, 0, MaximumVSync);
        if (requested == 0)
        {
            surface.SetVSync(false);

            int cap = targetFrameRate > 0 ? targetFrameRate : 0;
            surface.SetFrameCap(cap);
            return new DisplayPacingState(requested, 0, cap, false);
        }

        // Every refresh first, through the windowing layer, so that its own idea of vsync stays
        // true — a window-state change re-applies it. Then the exact interval for the value 2.
        surface.SetVSync(true);

        // Q-066: without a swap-interval call there is no way to ask for one refresh out of two,
        // and the documented fallback is every refresh.
        bool fellBack = requested == MaximumVSync && !surface.TrySetSwapInterval(MaximumVSync);
        surface.SetFrameCap(0);

        return new DisplayPacingState(requested, fellBack ? 1 : requested, 0, fellBack);
    }
}
