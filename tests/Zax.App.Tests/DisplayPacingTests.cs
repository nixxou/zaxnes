using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Platform;
using Zax.Render;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// FIX-1, the two defects the user reported: « Synchronisation verticale » did nothing because
/// the setting never reached the window, and every launch overwrote the persisted settings with
/// the command-line defaults.
/// </summary>
/// <remarks>
/// Covers SPEC-06 REQ-SCN-061 (vsync and target frame rate are user settings the host applies),
/// SPEC-09 <c>video.vsync</c> / <c>video.frame_rate</c> and REQ-PERS-020 / REQ-PERS-021 with
/// Q-066 (an option overrides a persisted setting only when it is explicitly passed, and the
/// override is never written back).
/// </remarks>
public sealed class DisplayPacingTests
{
    /// <summary>
    /// REQ-SCN-061 with SPEC-09: each value of <c>video.vsync</c> has its swap interval, and the
    /// frame cap only exists when the driver is not pacing the swaps.
    /// </summary>
    [Fact]
    public void REQ_SCN_061_each_vsync_value_sets_its_swap_interval()
    {
        FakeDisplaySurface surface = new();

        DisplayPacingState off = DisplayPacing.Apply(surface, 0, 60);
        Assert.Equal(0, surface.SwapInterval);
        Assert.Equal(60, surface.FrameCap);
        Assert.Equal(new DisplayPacingState(0, 0, 60, false), off);

        DisplayPacingState every = DisplayPacing.Apply(surface, 1, 60);
        Assert.Equal(1, surface.SwapInterval);
        Assert.Equal(0, surface.FrameCap);
        Assert.Equal(new DisplayPacingState(1, 1, 0, false), every);

        DisplayPacingState other = DisplayPacing.Apply(surface, 2, 120);
        Assert.Equal(2, surface.SwapInterval);
        Assert.Equal(0, surface.FrameCap);
        Assert.Equal(new DisplayPacingState(2, 2, 0, false), other);

        // Out of domain is clamped, and an uncapped request stays uncapped.
        Assert.Equal(2, DisplayPacing.Apply(surface, 9, 0).SwapInterval);
        Assert.Equal(0, DisplayPacing.Apply(surface, 0, 0).FrameCap);
    }

    /// <summary>
    /// Q-066: with no swap-interval call there is no way to ask for one refresh out of two, and
    /// the documented fallback is every refresh — never "no synchronisation at all".
    /// </summary>
    [Fact]
    public void Q_066_vsync_2_falls_back_to_every_refresh_without_a_swap_interval_call()
    {
        FakeDisplaySurface surface = new() { SwapIntervalSupported = false };

        DisplayPacingState state = DisplayPacing.Apply(surface, 2, 90);

        Assert.True(state.FellBackToEveryRefresh);
        Assert.Equal(1, state.SwapInterval);
        Assert.Equal(1, surface.SwapInterval);
        Assert.Equal(0, surface.FrameCap);
        Assert.Contains("Q-066", state.Describe(), StringComparison.Ordinal);
    }

    /// <summary>
    /// FIX-1 (a): changing <c>video.vsync</c> at run time reaches the window's swap interval, and
    /// changing <c>video.frame_rate</c> reaches its frame cap — which is what the options window
    /// of AMD-010 REQ-UI-202 does when the user ticks « Synchronisation verticale ».
    /// </summary>
    [Fact]
    public void REQ_SCN_061_a_runtime_vsync_change_reaches_the_window()
    {
        FakeDisplaySurface surface = new();
        AppSettings settings = new();
        using VideoPacing pacing = new(surface, settings);

        pacing.Apply();
        Assert.Equal(0, surface.SwapInterval);
        Assert.Equal(90, surface.FrameCap);

        settings.VSync = 1;
        Assert.Equal(1, surface.SwapInterval);
        Assert.Equal(0, surface.FrameCap);

        settings.VSync = 2;
        Assert.Equal(2, surface.SwapInterval);

        settings.VSync = 0;
        Assert.Equal(0, surface.SwapInterval);
        Assert.Equal(90, surface.FrameCap);

        settings.FrameRate = 60;
        Assert.Equal(60, surface.FrameCap);

        // REQ-PERS-009: writing the value already in force changes nothing and applies nothing.
        int applications = pacing.Applications;
        settings.FrameRate = 60;
        Assert.Equal(applications, pacing.Applications);
    }

    /// <summary>
    /// A script's write on <c>video.vsync</c> is a session value (REQ-PERS-010) and it takes
    /// effect at once like any other change; the end of the game brings the preference back to
    /// the window.
    /// </summary>
    [Fact]
    public void REQ_PERS_010_a_script_write_on_vsync_reaches_the_window_and_is_undone()
    {
        FakeDisplaySurface surface = new();
        AppSettings settings = new();
        using VideoPacing pacing = new(surface, settings);

        settings.VSync = 1;
        settings.Model.Set(SettingKeys.VideoVSync, SettingValue.OfEnum(0), SettingLayer.Session);
        Assert.Equal(0, surface.SwapInterval);

        settings.EndOfGameResets();
        Assert.Equal(1, surface.SwapInterval);
    }

    /// <summary>
    /// FIX-1 (b): a launch with no flag at all keeps every persisted setting the command line
    /// could have overridden (SPEC-09 REQ-PERS-020, Q-066).
    /// </summary>
    [Fact]
    public void Q_066_a_launch_with_no_flag_keeps_every_persisted_setting()
    {
        using TempRoot root = new();

        // What the user chose in the options window during a previous run.
        AppSettings chosen = new();
        SettingsStore store = new(root.Paths);
        store.Load(chosen, DateTimeOffset.UnixEpoch);
        chosen.Fov = 55f;
        chosen.Shadow = false;
        chosen.Skybox = 2;
        chosen.TvBox = 1;
        chosen.SkipFrame = false;
        chosen.FrameRate = 120;
        chosen.VSync = 2;
        store.Save(chosen);

        // The next launch, with nothing on the command line.
        AppOptions options = AppOptions.Parse([]);
        AppSettings settings = new();
        new SettingsStore(root.Paths).Load(settings, DateTimeOffset.UnixEpoch);
        CommandLineSettings.Apply(options, settings);

        Assert.Equal(55f, settings.Fov);
        Assert.False(settings.Shadow);
        Assert.Equal(2, settings.Skybox);
        Assert.Equal(1, settings.TvBox);
        Assert.False(settings.SkipFrame);
        Assert.Equal(120, settings.FrameRate);
        Assert.Equal(2, settings.VSync);

        // No key was overridden, so the model holds no command-line value at all.
        Assert.All(
            CommandLineSettings.OverridableKeys,
            key => Assert.False(settings.Model.HasCommandLineValue(key)));
    }

    /// <summary>
    /// FIX-1 (c): an explicitly passed option overrides the persisted setting for this run and is
    /// not written back; a change the user then makes takes over and is persisted (Q-066).
    /// </summary>
    [Fact]
    public void Q_066_an_explicit_flag_overrides_this_run_only()
    {
        using TempRoot root = new();

        AppSettings chosen = new();
        SettingsStore first = new(root.Paths);
        first.Load(chosen, DateTimeOffset.UnixEpoch);
        chosen.VSync = 2;
        chosen.Fov = 55f;
        first.Save(chosen);

        // A launch that asks for no synchronisation and a 95° field of view.
        AppOptions options = AppOptions.Parse(["--vsync", "0", "--fov", "95"]);
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);
        CommandLineSettings.Apply(options, settings);

        Assert.Equal(0, settings.VSync);
        Assert.Equal(95f, settings.Fov);
        Assert.True(settings.Model.HasCommandLineValue(SettingKeys.VideoVSync));

        // REQ-PERS-021: an override is not a change by the user, so it schedules no write…
        Assert.False(store.HasPendingWrite);

        // …and the exit write of REQ-LOOP-052 keeps the user's own choices.
        store.Save(settings);
        AppSettings reread = new();
        new SettingsStore(root.Paths).Load(reread, DateTimeOffset.UnixEpoch);
        Assert.Equal(2, reread.VSync);
        Assert.Equal(55f, reread.Fov);

        // The user ticks the option during this run: that value is a preference again.
        settings.VSync = 1;
        Assert.Equal(1, settings.VSync);
        Assert.False(settings.Model.HasCommandLineValue(SettingKeys.VideoVSync));
        Assert.True(store.HasPendingWrite);

        store.Save(settings);
        AppSettings afterChange = new();
        new SettingsStore(root.Paths).Load(afterChange, DateTimeOffset.UnixEpoch);
        Assert.Equal(1, afterChange.VSync);

        // The field of view was never touched by the user, so the file still holds 55.
        Assert.Equal(55f, afterChange.Fov);
    }

    /// <summary>
    /// Q-066: the « Valeurs par défaut » button of REQ-UI-202 also ends an override — it is a
    /// choice by the user like any other.
    /// </summary>
    [Fact]
    public void Q_066_a_reset_to_the_default_ends_the_override()
    {
        AppSettings settings = new();
        CommandLineSettings.Apply(AppOptions.Parse(["--fps", "60"]), settings);
        Assert.Equal(60, settings.FrameRate);

        settings.Model.Reset(SettingKeys.VideoFrameRate);

        Assert.Equal(90, settings.FrameRate);
        Assert.False(settings.Model.HasCommandLineValue(SettingKeys.VideoFrameRate));
    }

    /// <summary>
    /// FIX-1 (d): the 2D/3D toggle of REQ-UI-200 changes nothing of the pacing, and the full
    /// screen of REQ-UI-209 keeps the swap interval the user asked for — the host re-applies it
    /// after the window-state change, which is where the tearing in full screen came from.
    /// </summary>
    [Fact]
    public void REQ_UI_209_the_2d_toggle_and_full_screen_keep_the_swap_interval()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Catalog.Add("a", harness.WriteRom(SyntheticRom.Counter()));

        FakeDisplaySurface surface = new();
        using VideoPacing pacing = new(surface, harness.Settings);
        harness.Settings.VSync = 2;
        Assert.Equal(2, surface.SwapInterval);

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("a"));
        harness.Run(ZaxCommands.ToggleTwoDimensional);
        Assert.True(harness.Host.ShowingTwoDimensional);
        Assert.Equal(2, surface.SwapInterval);
        Assert.Equal(0, surface.FrameCap);

        // REQ-UI-209: the command flips ui.fullscreen and the host re-applies the pacing to the
        // window it has just moved.
        harness.Run(ZaxCommands.ToggleFullscreen);
        Assert.True(harness.Settings.Fullscreen);
        pacing.Apply();

        Assert.Equal(2, surface.SwapInterval);
        Assert.Equal(0, surface.FrameCap);
        Assert.Equal(2, harness.Scene.Settings.VSync);
    }

    /// <summary>
    /// REQ-SCN-061: the window is opened on the persisted values, not on the command-line
    /// defaults — the swap interval of the first frame is already the right one.
    /// </summary>
    [Fact]
    public void REQ_SCN_061_the_window_opens_on_the_settings_values()
    {
        HostWindowSettings settings = new("Zaxnes", 1280, 720, VSync: 2, TargetFrameRate: 120);

        Assert.True(settings.IsVSyncEnabled);
        Assert.Equal(2, settings.VSync);
        Assert.Equal(120, settings.TargetFrameRate);
        Assert.False(new HostWindowSettings("Zaxnes", 1280, 720, VSync: 0).IsVSyncEnabled);
    }

    /// <summary>A window that records what the pacing asks of it.</summary>
    private sealed class FakeDisplaySurface : IDisplaySurface
    {
        /// <summary>Whether the backend offers a swap-interval call (Q-066).</summary>
        public bool SwapIntervalSupported { get; init; } = true;

        /// <summary>The swap interval in force, whichever call set it.</summary>
        public int SwapInterval { get; private set; }

        /// <summary>The frame cap in force, 0 for uncapped.</summary>
        public int FrameCap { get; private set; }

        /// <inheritdoc/>
        public void SetVSync(bool enabled) => SwapInterval = enabled ? 1 : 0;

        /// <inheritdoc/>
        public void SetFrameCap(int framesPerSecond) => FrameCap = framesPerSecond;

        /// <inheritdoc/>
        public bool TrySetSwapInterval(int interval)
        {
            if (!SwapIntervalSupported)
            {
                return false;
            }

            SwapInterval = interval;
            return true;
        }
    }

    /// <summary>A throw-away user root.</summary>
    private sealed class TempRoot : IDisposable
    {
        private readonly string _root = Path.Combine(
            Path.GetTempPath(),
            "zax-pacing-" + Guid.NewGuid().ToString("N"));

        public TempRoot()
        {
            Directory.CreateDirectory(_root);
            Paths = UserDataPaths.Resolve(_root);
        }

        public UserDataPaths Paths { get; }

        public void Dispose()
        {
            try
            {
                Directory.Delete(_root, recursive: true);
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // A leftover temporary folder is not a test failure.
            }
        }
    }
}
