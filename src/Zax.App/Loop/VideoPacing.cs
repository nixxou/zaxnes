using Zax.App.Persistence;
using Zax.Platform;
using Zax.Settings;

namespace Zax.App.Loop;

/// <summary>
/// Keeps the window's buffer-swap synchronisation and frame cap equal to what the settings say
/// (SPEC-06 REQ-SCN-061, SPEC-09 <c>video.vsync</c> and <c>video.frame_rate</c>).
/// </summary>
/// <remarks>
/// <para>
/// Before FIX-1 the two settings only reached <see cref="Render.SceneState"/>, which no one
/// paces with: the swap interval stayed at whatever the command line asked for at window
/// creation, so the option « Synchronisation verticale » changed nothing and the image tore.
/// This is the one place that applies them — at start-up from the persisted value, at every
/// change of either setting, and again after a window-state change, since Silk.NET re-applies
/// its own boolean vsync then and would undo the interval 2.
/// </para>
/// <para>
/// <see cref="VideoPacingRequest"/> is what the host asks for rather than the raw settings,
/// because a smoke run must not be paced by the display refresh rate.
/// </para>
/// </remarks>
internal sealed class VideoPacing : IDisposable
{
    private readonly IDisplaySurface _surface;
    private readonly AppSettings _settings;
    private readonly Func<VideoPacingRequest> _request;
    private readonly Action<string>? _log;

    private bool _detached;

    /// <summary>Attaches to <paramref name="settings"/> and applies nothing yet.</summary>
    /// <param name="surface">The window to drive.</param>
    /// <param name="settings">The settings to follow.</param>
    /// <param name="request">
    /// What to put in force, read on every application; null follows <c>video.vsync</c> and
    /// <c>video.frame_rate</c> as they stand.
    /// </param>
    /// <param name="log">Receives one line per application.</param>
    public VideoPacing(
        IDisplaySurface surface,
        AppSettings settings,
        Func<VideoPacingRequest>? request = null,
        Action<string>? log = null)
    {
        ArgumentNullException.ThrowIfNull(surface);
        ArgumentNullException.ThrowIfNull(settings);

        _surface = surface;
        _settings = settings;
        _request = request ?? (() => new VideoPacingRequest(settings.VSync, settings.FrameRate));
        _log = log;

        _settings.Changed += OnSettingChanged;
    }

    /// <summary>What the last application put in force.</summary>
    public DisplayPacingState State { get; private set; }

    /// <summary>How many times the pacing has been applied, which the tests watch.</summary>
    public int Applications { get; private set; }

    /// <summary>Applies the current request to the window and logs what it did.</summary>
    public DisplayPacingState Apply()
    {
        VideoPacingRequest request = _request();
        State = DisplayPacing.Apply(_surface, request.VSync, request.TargetFrameRate);
        Applications++;
        _log?.Invoke($"display pacing: {State.Describe()}");
        return State;
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_detached)
        {
            return;
        }

        _detached = true;
        _settings.Changed -= OnSettingChanged;
    }

    private void OnSettingChanged(string key)
    {
        // REQ-PERS-084: both take effect immediately, whoever changed them — the options window,
        // a menu, a libretro core option or a script.
        if (key is SettingKeys.VideoVSync or SettingKeys.VideoFrameRate)
        {
            Apply();
        }
    }
}

/// <summary>What the host wants in force on the window right now.</summary>
/// <param name="VSync">0 off, 1 every refresh, 2 every other refresh (SPEC-09).</param>
/// <param name="TargetFrameRate">The cap to use when vsync is off; 0 for uncapped.</param>
internal readonly record struct VideoPacingRequest(int VSync, int TargetFrameRate);
