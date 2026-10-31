using Zax.App.Audio;
using Zax.App.Input;
using Zax.App.Persistence;
using Zax.Nes;
using Zax.Render;
using Zax.Scripting;

namespace Zax.App.Loop;

/// <summary>What an <see cref="EmulationHost"/> is built from.</summary>
internal sealed class EmulationHostOptions
{
    /// <summary>The user root and its layout (SPEC-09 §3.8).</summary>
    public required UserDataPaths Paths { get; init; }

    /// <summary>The settings the loop reads: speed, region, volume, menu music…</summary>
    public required AppSettings Settings { get; init; }

    /// <summary>The scene the profile's scripts and the camera live in.</summary>
    public required SceneState Scene { get; init; }

    /// <summary>The library the loop resolves game identifiers against.</summary>
    public required IGameCatalog Games { get; init; }

    /// <summary>The output mixer, or null to run without audio.</summary>
    public AudioMixer? Mixer { get; init; }

    /// <summary>Where <c>print</c> and script errors go.</summary>
    public IScriptLog? ScriptLog { get; init; }

    /// <summary>
    /// Produces a 256 × 192 RGB capture of the game view (REQ-PERS-051), or null when the host
    /// cannot capture — a headless test, for instance.
    /// </summary>
    public Func<byte[]?>? CaptureThumbnail { get; init; }

    /// <summary>Wall clock, for the <c>lastPlayed</c> and slot timestamps.</summary>
    public Func<DateTimeOffset>? Now { get; init; }

    /// <summary>Monotonic clock in seconds, for the frame pacer.</summary>
    public Func<double>? Clock { get; init; }

    /// <summary>Receives one line per notable event.</summary>
    public Action<string>? Log { get; init; }

    /// <summary>
    /// Receives one line per emulated frame when <c>-emustats</c> was given (AMD-053
    /// REQ-UI-213). Null switches the measurement off, which is what every normal run wants.
    /// The host writes English (AMD-035 §2) and the frontend sends it to the <b>log file</b>,
    /// never to the parent console.
    /// </summary>
    public Action<string>? FrameStatsLog { get; init; }

    /// <summary>
    /// Whether <c>-noresume</c> — or <c>-slot</c>, which implies it — was given. The resume slot
    /// of REQ-LOOP-076 is then neither read, nor written, nor erased for the whole run
    /// (Q-088).
    /// </summary>
    // Q-088
    public bool SuspendResume { get; init; }

    /// <summary>
    /// Called with the absolute path of a ROM the host has just started, so that the frontend
    /// can keep the ten recent ROMs of AMD-010 REQ-UI-200.
    /// </summary>
    public Action<string>? RomOpened { get; init; }

    /// <summary>
    /// The profile identifier remembered for a ROM file in <c>games.json</c>, or null when the
    /// file has never been started — step 1 of AMD-010 REQ-UI-201 (Q-036).
    /// </summary>
    public Func<string, string?>? ProfileForRom { get; init; }

    /// <summary>
    /// Called with the absolute ROM path and the profile identifier it was started on, so that
    /// the frontend remembers the pair (AMD-010 REQ-UI-201, Q-036).
    /// </summary>
    public Action<string, string>? RomProfileChosen { get; init; }

    /// <summary>What <c>Setting:Save()</c> does, or null to make the call inert (REQ-PERS-015).</summary>
    public Action? SaveSettingsRequested { get; init; }

    /// <summary>What <c>Setting:Load()</c> does, or null (REQ-PERS-015).</summary>
    public Action? ReloadSettingsRequested { get; init; }

    /// <summary>
    /// Whether the interface currently holds a <b>modal window</b> — the file browser E7, the
    /// profile chooser or a confirmation dialog E8 (SPEC-10 §3.11, §3.12). Null means "never",
    /// which is what a headless host wants.
    /// </summary>
    /// <remarks>
    /// REQ-UI-004 puts a modal window above the emulation in the priority order of the Menu
    /// button: cases 2 and 3 come before case 4. The loop cannot know by itself whether one is
    /// open, so the frontend tells it through this predicate, and the frontend is also the one
    /// that decides what Escape does to the window it owns — the browser closes as with
    /// « Cancel », a confirmation dialog does nothing (REQ-UI-110).
    /// </remarks>
    public Func<bool>? ModalWindowOpen { get; init; }

    /// <summary>
    /// Opens the confirmation box E8 of SPEC-10 REQ-UI-110 and returns true when it took the
    /// request. Null — or false — means there is no frontend to ask, and the loop acts at once:
    /// that is what a headless host and the command-line paths want.
    /// </summary>
    public Func<ConfirmationRequest, bool>? Confirm { get; init; }

    /// <summary>
    /// Whether a real audio device is consuming the mixer's blocks. Only then does the host
    /// correct the drift between the emulation clock and the device clock (REQ-AUD-005): with
    /// no consumer there is nothing to track, and leaving the resampling ratio alone keeps the
    /// replays of REQ-ARCH-012 bit-exact.
    /// </summary>
    public bool AudioDeviceOpen { get; init; }
}

/// <summary>
/// The emulation loop of SPEC-07: the state machine, the command queue, the frame pacing, the
/// rollback history, the save states and the launch flow.
/// </summary>
/// <remarks>
/// <para>
/// Nothing here touches OpenGL, the window or the audio device: the host is driven by
/// <see cref="Tick(double, in InputSnapshot)"/>, once per rendered frame, and everything it needs from the outside world
/// arrives through <see cref="EmulationHostOptions"/>. That is what lets the round-1 tests drive
/// a whole session headless, and it is the separation REQ-LOOP-062 asks for — the pacing is
/// frontend-side, the kernel only knows "run exactly one emulated frame".
/// </para>
/// <para>
/// Commands are applied between two emulated frames, in the order they were sent, and are
/// deferred — never dropped — while the loading indicator is up (REQ-LOOP-007). They are also
/// handled in Paused and Stopped.
/// </para>
/// </remarks>
internal sealed partial class EmulationHost : IDisposable
{
    private readonly EmulationHostOptions _options;
    private readonly UserDataPaths _paths;
    private readonly SceneState _scene;
    private readonly IGameCatalog _games;
    private readonly AudioMixer? _mixer;
    private readonly Func<DateTimeOffset> _now;
    private readonly Action<string> _log;
    private readonly Queue<(string Name, CommandArgs Args)> _pending = new();
    private readonly HotkeyTracker _hotkeys = new();
    private readonly BatteryRamStore _battery;
    private readonly AudioDriftControl _drift;

    private GameSession? _session;
    private ResolvedGame? _currentGame;
    private byte[]? _pauseThumbnail;
    private LoadJob? _job;
    private bool _disposed;

    // AMD-051 REQ-LOOP-073: the blocked-load watchdog. One idle clock covers both halves of the
    // condition, because a step either exposes the progress of REQ-E3D-600 or ends by moving on
    // to the next one — see WatchLoadTimeout. The job is part of the key, so a new load always
    // starts the clock afresh.
    private LoadJob? _watchedJob;
    private double _loadIdleSeconds;
    private int _watchedStage = -1;
    private float _watchedProgress = -1f;

    /// <summary>Creates a host. No game is loaded: the state is Stopped (REQ-LOOP-001).</summary>
    public EmulationHost(EmulationHostOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);

        _options = options;
        _paths = options.Paths;
        _scene = options.Scene;
        _games = options.Games;
        _mixer = options.Mixer;
        _now = options.Now ?? (static () => DateTimeOffset.UtcNow);
        _log = options.Log ?? (static _ => { });
        _battery = new BatteryRamStore(options.Paths);
        _drift = new AudioDriftControl(_mixer?.SampleRate ?? Zax.Nes.Apu.Apu2A03.DefaultSampleRate);

        Settings = options.Settings;
        Pacer = new FramePacer(RegionTiming.For(NesRegion.Ntsc).NominalFramesPerSecond, options.Clock);
        Slots = SaveSlotIndex.Load(options.Paths, _now());

        // AMD-053 REQ-LOOP-076 rule 3 and REQ-PERS-055 as §3 point 7 extends it: what the
        // reconciliation does to the resume slot has to be journalled, so its state before the
        // pass is noted here. With -noresume the slot is left strictly alone (Q-088).
        bool resumeWasOccupied = Slots.Find(SaveSlotIndex.ResumeSlotId) is { IsOccupied: true };
        Slots.Reconcile(options.Paths, _games.Exists, skipResumeSlot: options.SuspendResume);
        PrepareResumeSlot(resumeWasOccupied);

        Settings.Changed += OnSettingChanged;

        // AMD-035 §1: the language in force is the setting's, read once here and kept in step by
        // OnSettingChanged afterwards. There is no detection of the system's language.
        Ui.UiStrings.Language = Ui.UiStrings.Parse(Settings.Language);

        ApplySpeed();
        RegisterCommands();
        PublishState();
    }

    /// <summary>The current state; exactly one of the three at every instant (REQ-LOOP-001).</summary>
    public EmulationState State { get; private set; } = EmulationState.Stopped;

    /// <summary>The loading indicator, orthogonal to the state (REQ-LOOP-001).</summary>
    public bool IsLoading => _job is not null;

    /// <summary>Progress of the mesh generation, 0 to 1 (REQ-LOOP-003).</summary>
    public float LoadProgress { get; private set; }

    /// <summary>The settings the loop reads and writes.</summary>
    public AppSettings Settings { get; }

    /// <summary>The frame clock of SPEC-07 §3.2.</summary>
    public FramePacer Pacer { get; }

    /// <summary>The rollback history of SPEC-07 §3.4.</summary>
    public RollbackHistory Rollback { get; } = new();

    /// <summary>The fast-forward model of REQ-LOOP-016.</summary>
    public FastForwardState FastForward { get; } = new();

    /// <summary>The notification area of SPEC-10 §3.10.</summary>
    public NotificationCenter Notifications { get; } = new();

    /// <summary>The command model of AMD-010 REQ-UI-205.</summary>
    public CommandModel Commands { get; } = new();

    /// <summary>The 36 numbered slots, the quick slot and the resume slot.</summary>
    public SaveSlotIndex Slots { get; }

    /// <summary>
    /// Whether the <c>Resume</c> entry of AMD-053 REQ-LOOP-076 is offered this run: the resume
    /// slot is occupied, its game is still in the library, its file passed the whole-file check
    /// of the start-up, and <c>-noresume</c> was not given. Ignoring the entry during the
    /// session does <b>not</b> clear it (rule 2); only the next exit does.
    /// </summary>
    public bool ResumeAvailable { get; private set; }

    /// <summary>
    /// Whether the « Favorites only » filter of AMD-053 REQ-UI-210 is ticked. A session state:
    /// the amendment persists the mark, not the filter.
    /// </summary>
    public bool FavoritesOnly { get; private set; }

    /// <summary>The title search of AMD-053 REQ-UI-211, empty when nothing is typed.</summary>
    public string LibrarySearchText { get; private set; } = string.Empty;

    /// <summary>
    /// Whether « Mode 2D » is switched on by hand (AMD-010 REQ-UI-200, shortcut <c>F11</c>).
    /// </summary>
    /// <remarks>
    /// AMD-017 (Q-037) makes the entry a <b>hot</b> toggle of the rendering: the view shows the
    /// core's pixel frame on the NES plane (AMD-009) and goes back to the 3D rendering of the
    /// current profile at once. The ROM is not relaunched, and the profile, the game state, the
    /// scripts and the conversion are untouched — which is why this is a flag of the host and
    /// not a library entry. <b>(D)</b>
    /// </remarks>
    // Q-037
    public bool TwoDimensionalView { get; private set; }

    /// <summary>
    /// Whether the view shows the core's pixel frame rather than the 3D scene: the hot toggle of
    /// <see cref="TwoDimensionalView"/>, or a profile with no pattern at all — the "2D Mode"
    /// entry of REQ-SCN-055, which is always drawn that way.
    /// </summary>
    public bool ShowingTwoDimensional
        => TwoDimensionalView || _session is { TwoDimensionalMode: true };

    /// <summary>The running session, or null in Stopped.</summary>
    public GameSession? Session => _session;

    /// <summary>The current game, or null when there is none (REQ-LOOP-003).</summary>
    public string? CurrentGameId => _currentGame?.Id;

    /// <summary>Title of the current game, or null.</summary>
    public string? CurrentGameTitle => _currentGame?.Title;

    /// <summary>
    /// Real frames run since the host was created (REQ-LOOP-065): one per frame the clock said
    /// was due, whatever the run-ahead setting says the core emulated behind it.
    /// </summary>
    public long EmulatedFrames { get; private set; }

    /// <summary>Whether a quit was asked for (REQ-LOOP-052).</summary>
    public bool QuitRequested { get; private set; }

    /// <summary>Commands waiting for the end of a load (REQ-LOOP-007).</summary>
    public int PendingCommands => _pending.Count;

    /// <summary>
    /// Raised with the name of a command the UI owns — the options window, the key
    /// configuration, the about block, full screen (AMD-010 REQ-UI-202, 203, 209).
    /// </summary>
    public event Action<string, CommandArgs>? UiRequested;

    /// <summary>
    /// The capture taken at the last Running → Paused transition (REQ-LOOP-006), or null.
    /// </summary>
    public ReadOnlySpan<byte> PauseThumbnail => _pauseThumbnail;

    /// <summary>
    /// Queues a command. It runs between two emulated frames, in order, and waits for the end
    /// of a load if one is under way (REQ-LOOP-007).
    /// </summary>
    public void Enqueue(string command, CommandArgs args = default)
    {
        ArgumentException.ThrowIfNullOrEmpty(command);
        _pending.Enqueue((command, args));
    }

    /// <summary>
    /// One rendered frame: the Zapper, the hotkeys, the notification timers, the load in
    /// progress or the queued commands, then the emulated frames that are due.
    /// </summary>
    /// <param name="realSeconds">Real time since the previous call.</param>
    /// <param name="snapshot">The platform-independent input snapshot (REQ-INP-120).</param>
    /// <returns>How many emulated frames ran.</returns>
    public int Tick(double realSeconds, in InputSnapshot snapshot)
    {
        // The notification timer is real time: neither the pause nor the game speed affect it
        // (REQ-UI-080).
        Notifications.Advance(realSeconds);

        // REQ-INP-061: the Zapper is refreshed once per displayed frame, before the hotkeys and
        // before any emulated frame reads its position.
        UpdateZapper(snapshot);
        ReadHotkeys(snapshot.PlayerOne, snapshot.PlayerTwo, (float)realSeconds);

        if (_job is not null)
        {
            AdvanceLoad();

            // REQ-LOOP-021 step 6: no emulated frame runs before the loading indicator clears.
            PublishState();

            // AMD-051 REQ-LOOP-073: a load that stops advancing is given up instead of freezing
            // the program. Checked after PublishState, so the progress it reads is this frame's.
            WatchLoadTimeout(realSeconds);
            return 0;
        }

        ApplyCommands();

        int frames = RunDueFrames(snapshot.PlayerOne, snapshot.PlayerTwo);
        AdvanceBattery(realSeconds);
        CorrectAudioDrift(realSeconds);
        PublishState();
        return frames;
    }

    /// <summary>
    /// One rendered frame driven by action masks alone, which is all the loop tests of
    /// SPEC-07 need.
    /// </summary>
    /// <param name="realSeconds">Real time since the previous call.</param>
    /// <param name="playerOne">Action mask of J1 (REQ-INP-002).</param>
    /// <param name="playerTwo">Action mask of J2.</param>
    public int Tick(
        double realSeconds,
        InputAction playerOne = InputAction.None,
        InputAction playerTwo = InputAction.None)
        => Tick(realSeconds, InputSnapshot.Of(playerOne, playerTwo));

    /// <summary>
    /// Applies the queued commands without running any emulated frame — the "between two
    /// emulated frames" moment of REQ-LOOP-007. <see cref="Tick(double, in InputSnapshot)"/> calls it; a host that wants
    /// a command to take effect before the next frame can call it directly.
    /// </summary>
    /// <returns>How many commands ran.</returns>
    public int ApplyCommands()
    {
        if (_job is not null)
        {
            return 0;
        }

        int ran = DrainCommands();
        PublishState();
        return ran;
    }

    /// <summary>
    /// Runs the emulated frames the clock says are due, in the order of REQ-LOOP-060, with the
    /// rollback snapshot taken before the inputs are read.
    /// </summary>
    /// <param name="playerOne">Action mask of J1, every physical source combined.</param>
    /// <param name="playerTwo">Action mask of J2.</param>
    public int RunDueFrames(
        InputAction playerOne,
        InputAction playerTwo = InputAction.None)
    {
        if (_session is not { HasRom: true } session || State != EmulationState.Running)
        {
            return 0;
        }

        ApplySpeed();

        int due = Pacer.DueFrames();
        for (int frame = 0; frame < due; frame++)
        {
            // REQ-LOOP-064: the setting is read again here, so a change made during the game
            // applies at the next real frame and never in the middle of one.
            session.RunAheadFrames = Settings.RunAheadFrames;

            // REQ-LOOP-030 and REQ-LOOP-066: the snapshot is taken at the start of a frame whose
            // core frame counter is a multiple of 60, so before that frame executes — and the
            // counter is the real one, the core being in the real state R at this point.
            // AMD-042 REQ-LOOP-069: it carries the 64 PData values and the camera state as well,
            // read as they stand at that instant and with no script hook involved (AMD-042 §3).
            if (RollbackHistory.IsSnapshotFrame(session.Core.FrameCount))
            {
                Rollback.Capture(new RollbackSnapshot(
                    session.Core.SaveState(includeRomImage: false),
                    session.CaptureRollbackData(),
                    _scene.Camera.Capture()));
            }

            // AMD-053 REQ-UI-213: with -emustats the emulation time of each frame is measured.
            // Without the option the sink is null and not a single timestamp is taken.
            long startedAt = _options.FrameStatsLog is null
                ? 0L
                : System.Diagnostics.Stopwatch.GetTimestamp();

            try
            {
                // REQ-INP-005: one emulated frame is one sample, fast forward included.
                session.RunFrame((int)playerOne, (int)playerTwo, Zapper.PortByte);
            }
            catch (Exception exception) when (exception is InvalidOperationException
                or IndexOutOfRangeException
                or ArgumentException
                or NotSupportedException)
            {
                // REQ-LOOP-004, last rows: an exception raised by the core during a frame goes
                // back to Stopped. The original shows nothing; a notification is the tolerated
                // improvement the requirement's note allows.
                _log($"the core raised an exception: {exception.Message}");
                StopCurrentGame(clearCurrentGame: true);
                Notifications.ShowError("Emulation stopped after a core error.");
                return frame;
            }

            EmulatedFrames++;

            if (_options.FrameStatsLog is { } stats)
            {
                // One line per emulated frame, in English (AMD-035 §2), for the log file.
                stats(string.Create(
                    System.Globalization.CultureInfo.InvariantCulture,
                    $"emu frame {EmulatedFrames}: "
                        + $"{System.Diagnostics.Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds:F3} ms"));
            }

            if (FastForward.IsActive)
            {
                FastForward.OnFrameRun();
                if (!FastForward.IsActive)
                {
                    // The factor goes back to the setting as soon as the burst ends, so a held
                    // key re-arms without a visible step (REQ-LOOP-016).
                    ApplySpeed();
                }
            }
        }

        return due;
    }

    /// <summary>
    /// The exit of REQ-LOOP-052 in the four steps AMD-053 REQ-LOOP-076 fixes: the <c>Save</c>
    /// hook of every active script and the resume savestate, then the <c>End</c> hook and the
    /// stop of the current game, then the settings and the slot metadata. No numbered slot and
    /// no quick slot is ever written here (REQ-LOOP-052 as §3 points 1 and 2 amend it).
    /// </summary>
    public void Shutdown(SettingsStore? settings = null)
    {
        // Steps 1 and 2: the Save hooks run inside the capture, before the state is written.
        WriteOrClearResumeSlot();

        // Step 3: the End hook of every active script, then the stop of the emulation loop.
        StopCurrentGame(clearCurrentGame: true);

        // Step 4: the settings and the slot metadata, the `resume` entry among them.
        Slots.Save(_paths);
        settings?.Save(Settings);
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Settings.Changed -= OnSettingChanged;
        _session?.Dispose();
        _session = null;
    }

    // ----------------------------------------------------------------- states

    private void SetState(EmulationState next)
    {
        if (State == next)
        {
            return;
        }

        // REQ-LOOP-006: the capture of the game view is taken before the menu appears.
        if (State == EmulationState.Running && next == EmulationState.Paused)
        {
            _pauseThumbnail = _options.CaptureThumbnail?.Invoke() ?? _pauseThumbnail;

            // REQ-PERS-036, third source: the cover of the "2D Mode" entry follows the pause
            // capture every time the menu appears.
            RefreshTwoDimensionalCover();
        }

        State = next;
        PublishState();
    }

    private void PublishState()
    {
        // REQ-SCN-047 wants the 3D scene hidden outside a running game. With the classic menu
        // bar of AMD-010 there is no opaque menu covering it, so the scene stays visible while
        // the game is paused — the frozen frame a desktop emulator shows. Q-029.
        _scene.GameRunning = State != EmulationState.Stopped;
        LoadProgress = _session?.Engine.LoadProgress ?? (State == EmulationState.Stopped ? 0f : 1f);

        if (_mixer is { } mixer)
        {
            // REQ-AUD-010 / REQ-AUD-020: the console is audible only in Running, and the menu
            // music is in its active phase in every other state.
            mixer.EmulationAudible = State == EmulationState.Running;
            mixer.Volume = Settings.EffectiveVolume;
            mixer.MenuMusicEnabled = Settings.MenuMusic;
        }
    }

    private void ApplySpeed()
    {
        // REQ-LOOP-016: during a burst the factor is 1.5 whatever the setting says, which means
        // a setting above +2 is slowed down by fast forward. That is faithful (DEC-07-Q2).
        double factor = FastForward.IsActive ? SpeedSetting.FastForwardFactor : Settings.SpeedFactor;
        Pacer.Speed = factor;

        if (_session is { } session)
        {
            Pacer.NominalRate = session.Core.Timing.NominalFramesPerSecond;

            // REQ-AUD-011: the audio follows k by time compression with a proportional pitch
            // change; the core retimes its resampler, it never drops samples for that.
            session.Core.Speed = factor;
        }
    }

    private void OnSettingChanged(string key)
    {
        switch (key)
        {
            case "emu.game_speed":
                ApplySpeed();

                // AMD-052 REQ-LOOP-075: a change of the speed setting — the options window, a
                // script, a libretro core option or the ±1 shortcuts — resets the cadence clock,
                // so the frames the old period had made late are abandoned rather than run in a
                // burst at the new one. The fast-forward cycle of REQ-LOOP-016 goes through
                // ApplySpeed() without coming here, which is exactly what the requirement asks
                // for: its bursts expire and re-arm about six times a second.
                Pacer.Reset();
                break;

            case "emu.tv_mode":
                // REQ-LOOP-017: a region change is applied at once, to the core and to the
                // cadence, even in the middle of a game.
                if (_session is { HasRom: true } session)
                {
                    session.Core.Region = Settings.Region;
                }

                ApplySpeed();
                break;

            case "audio.volume":
            case "audio.mute":
            case "audio.bg_music":
                PublishState();
                break;

            case Zax.Settings.SettingKeys.UiLanguage:
                // AMD-035 §1: the change applies immediately. Everything the interface draws
                // reads the table through UiStrings, and the models that cache their labels
                // watch its generation counter.
                Ui.UiStrings.Language = Ui.UiStrings.Parse(Settings.Language);
                break;

            case "video.skip_frame":
                _scene.Settings.SkipRender = Settings.SkipFrame;
                break;

            case "video.frame_rate":
                _scene.Settings.TargetFrameRate = Settings.FrameRate;
                break;

            case "video.vsync":
                _scene.Settings.VSync = Settings.VSync;
                break;

            case "video.anti_alias":
                // AMD-032 §2: the setting takes effect on the next rendered frame, the
                // renderer reallocating its multisampled target by itself. No restart.
                _scene.Settings.AntiAliasSamples = SceneConstants.AntiAliasSamples(Settings.AntiAlias);
                break;

            case "render.shadow":
                _scene.Settings.Shadows = Settings.Shadow;
                break;

            case "scene.skybox":
                _scene.Settings.Skybox = (SkyboxKind)Settings.Skybox;
                break;

            // AMD-043 REQ-SCN-066: the curvature applies hot, at the next rendered frame.
            case Zax.Settings.SettingKeys.SceneBendLeftRight:
            case Zax.Settings.SettingKeys.SceneBendTopBottom:
            case Zax.Settings.SettingKeys.SceneArcHorizontal:
            case Zax.Settings.SettingKeys.SceneArcCenter:

            // AMD-046 REQ-SCN-074: « Bend 2D Mode » applies hot too, at the next rendered frame.
            case Zax.Settings.SettingKeys.SceneBend2dMode:
                PublishBending();
                break;

            case "camera.local_cam":
                // REQ-SCN-009: the top-down camera is a profile setting the "Mode caméra"
                // submenu also writes, so it is applied here as well as by the script bridge —
                // otherwise the menu entry would do nothing outside a running game.
                _scene.Camera.LocalCamera = Settings.Model.GetBool(Zax.Settings.SettingKeys.CameraLocalCam);
                break;

            case "camera.fov":
                _scene.Settings.Fov = Settings.Fov;
                _scene.Camera.Fov = Settings.Fov;
                break;

            case "light.intensity":
                _scene.Settings.LightIntensity = Settings.LightIntensity;
                break;

            default:
                break;
        }
    }

    /// <summary>
    /// Pushes the four curvature settings of AMD-043 REQ-SCN-066, and the 2D-mode switch of
    /// AMD-046 REQ-SCN-074, onto the scene. The renderer rebuilds <c>B</c> from them, so a
    /// change is in force at the next rendered frame.
    /// </summary>
    internal void PublishBending()
    {
        _scene.Settings.SceneScale = SceneScale;
        _scene.Settings.BendLeftRight = Settings.BendLeftRight;
        _scene.Settings.BendTopBottom = Settings.BendTopBottom;
        _scene.Settings.ArcHorizontal = Settings.ArcHorizontal;
        _scene.Settings.ArcAtEdge = Settings.ArcCenter != 0;
        _scene.Settings.Bend2dMode = Settings.Bend2dMode;
    }

    // ---------------------------------------------------------------- Zapper

    /// <summary>The mouse Zapper of SPEC-08 §3.7.</summary>
    public ZapperState Zapper { get; } = new();

    /// <summary>The device type of <paramref name="port"/> (0 or 1), REQ-INP-060.</summary>
    public ControllerPortType PortType(int port)
        => _session is { } session ? session.PortType(port) : ControllerPortType.Controller;

    /// <summary>
    /// Whether the Zapper is active right now: a game is running and at least one port is of
    /// type <c>Zapper</c> (REQ-INP-061).
    /// </summary>
    public bool ZapperActive => InputRouting.ZapperActive(
        State == EmulationState.Running,
        PortType(0),
        PortType(1));

    /// <summary>
    /// Refreshes the Zapper for one displayed frame (REQ-INP-061): the ray cast of REQ-INP-063
    /// through the game camera, and the trigger on the right mouse button (REQ-INP-062).
    /// </summary>
    private void UpdateZapper(in InputSnapshot snapshot)
    {
        if (!ZapperActive)
        {
            // In pause, or with no Zapper port, moving the mouse changes nothing of what the
            // core sees (REQ-INP-061).
            return;
        }

        float aspect = snapshot.PointerSize.Y > 0f
            ? snapshot.PointerSize.X / snapshot.PointerSize.Y
            : 1f;

        Zapper.Update(
            active: true,
            _scene.Camera.BuildView(_scene.Clipping, aspect),
            snapshot.Pointer,
            snapshot.PointerSize,
            snapshot.RightButton,
            _session?.ZapperOffset ?? System.Numerics.Vector3.Zero,
            SceneScale,

            // REQ-INP-124: aiming on the bent scene, with the curvature of the frame — except
            // on a 2D mode kept flat, where REQ-SCN-074 sends the aim back to REQ-INP-063.
            ZapperAim.AimBend(ShowingTwoDimensional, Settings.Bend2dMode, _scene.Settings.Bend));
    }

    /// <summary>
    /// The scene scale <c>e</c> of REQ-INP-063, which is 1 on desktop (DEC-08-4, notes). It is a
    /// property so that a host that scales the scene can tell the aim about it.
    /// </summary>
    public float SceneScale { get; set; } = 1f;

    // --------------------------------------------------------------- hotkeys

    /// <summary>
    /// Whether a modal interface window is up (REQ-UI-004 cases 2 and 3).
    /// </summary>
    public bool ModalWindowOpen => _options.ModalWindowOpen?.Invoke() ?? false;

    private void ReadHotkeys(InputAction playerOne, InputAction playerTwo, float realSeconds)
    {
        HotkeyEvents events = _hotkeys.Update(playerOne, playerTwo, realSeconds);

        if (events.HasFlag(HotkeyEvents.MenuLong) && !IsLoading)
        {
            // REQ-INP-101 and REQ-UI-005: a long press resets the view without changing the pause
            // state, and is inactive while the loading indicator is up.
            Enqueue(ZaxCommands.ResetView);
        }

        if (events.HasFlag(HotkeyEvents.MenuShort))
        {
            // REQ-INP-101 and REQ-UI-004 case 1: a Menu press made during a load is **ignored**,
            // not deferred. REQ-LOOP-007 defers the commands that are already in the queue, which
            // is right; the point here is that this one is never issued at all, so the game does
            // not pause the instant the load ends.
            if (!IsLoading)
            {
                // Cases 2 and 3: a modal window takes the press before the emulation does. What
                // it does with it belongs to the frontend — the browser closes as with « Cancel »,
                // a confirmation dialog swallows it (REQ-UI-110).
                Enqueue(ModalWindowOpen ? ZaxCommands.CancelModal : ZaxCommands.PauseResume);
            }
        }

        if (events.HasFlag(HotkeyEvents.Rollback))
        {
            Enqueue(ZaxCommands.Rollback);
        }

        if (events.HasFlag(HotkeyEvents.FastForward))
        {
            Enqueue(ZaxCommands.FastForward);
        }

        if (events.HasFlag(HotkeyEvents.QuickSave))
        {
            Enqueue(ZaxCommands.QuickSave);
        }

        if (events.HasFlag(HotkeyEvents.QuickLoad))
        {
            Enqueue(ZaxCommands.QuickLoad);
        }

        // AMD-044 REQ-INP-125: the bound action triggers the command of REQ-LOOP-072.
        if (events.HasFlag(HotkeyEvents.SoftReset))
        {
            Enqueue(ZaxCommands.SoftReset);
        }
    }

    // -------------------------------------------------------------- commands

    private int DrainCommands()
    {
        // A command may queue another one (a reset asked for by a failed launch, for
        // instance): the newly queued command waits for the next drain, which keeps the FIFO
        // order of REQ-LOOP-007 observable.
        int budget = _pending.Count;
        int ran = 0;
        while (budget-- > 0 && _job is null && _pending.Count > 0)
        {
            (string name, CommandArgs args) = _pending.Dequeue();
            ran++;
            if (Commands.Find(name) is { } command)
            {
                command.Execute(args);
            }
            else
            {
                _log($"unknown command '{name}'.");
            }
        }

        return ran;
    }

    /// <summary>
    /// The drift correction of REQ-AUD-005: the emulation clock and the device clock are never
    /// exactly equal, so the resampling ratio is nudged by at most ±0.5 % — less than nine
    /// cents of pitch — to hold the latency around two output blocks instead of letting the
    /// core run dry or drop its oldest samples. The law itself is
    /// <see cref="AudioDriftControl"/>.
    /// </summary>
    private void CorrectAudioDrift(double realSeconds)
    {
        if (!_options.AudioDeviceOpen
            || _session is not { HasRom: true } session
            || State != EmulationState.Running)
        {
            // Outside Running the buffer drains or is emptied (REQ-AUD-010); the loop starts
            // again from a settled state rather than from a measurement taken across the gap.
            if (_drift.Adjustment != 0.0 && _session is { HasRom: true } idle)
            {
                idle.Core.AudioRatioAdjustment = 0.0;
            }

            _drift.Reset();
            return;
        }

        session.Core.AudioRatioAdjustment = _drift.Update(session.Core.Apu.AvailableSamples, realSeconds);
    }

    private void AdvanceBattery(double realSeconds)
    {
        if (_session is { HasRom: true } session)
        {
            // REQ-PERS-060: written at most every 30 s, and only when the content changed.
            _battery.Advance(realSeconds, session.Core.ExportBatteryRam());
        }
    }
}
