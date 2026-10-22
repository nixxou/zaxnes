namespace Zax.App;

/// <summary>
/// The render-skip rule of SPEC-06 REQ-SCN-060, kept apart from the loop so that it can be
/// checked on its own.
/// </summary>
/// <remarks>
/// The camera is deliberately not part of the decision: the criterion of REQ-SCN-060 expects a
/// skipping host to redraw at the emulated rate and no more, which is also what makes the orbit
/// smoother once the setting is turned off.
/// </remarks>
internal static class RenderSkip
{
    /// <summary>
    /// Whether the scene may keep the previous image for this refresh.
    /// </summary>
    /// <param name="enabled">The user setting, true by default.</param>
    /// <param name="gameRunning">Whether a game is running (REQ-SCN-047).</param>
    /// <param name="emulatedFrames">Emulated frames produced since the last refresh.</param>
    /// <param name="targetUnchanged">
    /// False when the render target changed size, or when nothing has been drawn into it yet.
    /// </param>
    public static bool ShouldSkip(bool enabled, bool gameRunning, int emulatedFrames, bool targetUnchanged)
        => enabled && gameRunning && emulatedFrames == 0 && targetUnchanged;
}
