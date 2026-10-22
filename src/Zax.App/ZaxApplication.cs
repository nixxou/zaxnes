using System.Globalization;
using System.Numerics;
using ImGuiNET;
using Silk.NET.Input;
using Silk.NET.OpenGL;
using Silk.NET.OpenGL.Extensions.ImGui;
using Zax.App.Audio;
using Zax.App.Branding;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.App.Ui;
using Zax.Platform;
using Zax.Platform.Input;
using Zax.Profiles;
using Zax.Profiles.Library;
using Zax.Render;
using Zax.Render.Assets;

namespace Zax.App;

/// <summary>
/// The desktop host: the window, the GL context, the offscreen render target, the audio device
/// and the <see cref="EmulationHost"/> that owns the emulation loop.
/// </summary>
/// <remarks>
/// <para>
/// It owns the loop of SPEC-07 (states, command queue, pacing, speed, fast forward, rollback,
/// save states), the audio of SPEC-11, the persistence of SPEC-09 and the input layer of
/// SPEC-08. The interface itself — the menu bar, the options window, the key configuration, the
/// dialogs and the on-screen notifications of AMD-010 — is <see cref="ZaxUi"/>, which drives the
/// same command model (REQ-UI-205) the shortcuts and the tests drive.
/// </para>
/// <para>
/// Esc is the Menu action of SPEC-08: a short press pauses and resumes, a long press resets the
/// view. Quitting is Ctrl+Q or closing the window (REQ-LOOP-052).
/// </para>
/// </remarks>
internal sealed class ZaxApplication : IDisposable
{
    private readonly AppOptions _options;
    private readonly FrameRateCounter _frameRate = new();
    private readonly SceneState _scene = new();
    private readonly AppSettings _settings = new();

    private HostWindow? _host;
    private GL? _gl;
    private OffscreenFramebuffer? _offscreen;
    private ISceneRenderer? _renderer;
    private TvModel? _television;
    private ImGuiController? _imgui;
    private IKeyboard? _keyboard;
    private IMouse? _mouse;
    private AudioOutput? _audio;
    private AudioMixer? _mixer;
    private VideoPacing? _pacing;

    private UserDataPaths? _paths;
    private SettingsStore? _settingsStore;
    private InputBindingStore? _bindingStore;
    private LibraryGameCatalog? _catalog;
    private EmulationHost? _emulation;
    private string? _startupError;
    private string? _startupGameId;
    private byte[]? _picture;

    // SPEC-08: the logical action layer (REQ-INP-120) and the desktop device layer that feeds it
    // (REQ-INP-121). The bindings are global and shared by both players (REQ-INP-048).
    private readonly InputBindings _bindings = new();
    private readonly CursorAutoHide _cursor = new();
    private DesktopInputSource? _input;
    private SilkPointer? _pointer;

    private ZaxUi? _ui;
    private UiShots? _shots;

    private int _renderedFrames;
    private long _emulatedFrames;
    private int _sceneFrames;
    private int _skippedFrames;
    private (int Width, int Height) _lastRenderSize;
    private Vector2? _dragOrigin;
    private string _glRenderer = "unknown";
    private string _glVersion = "unknown";
    private int _viewInset;

    /// <summary>Creates the application for the given options.</summary>
    /// <remarks>
    /// Nothing of the settings is decided here: the file has not been read yet. The settings are
    /// loaded, then overridden by the options the command line really carries, then published to
    /// the scene, in <see cref="TryPrepare"/> (SPEC-09 REQ-PERS-020, Q-066).
    /// </remarks>
    public ZaxApplication(AppOptions options) => _options = options;

    /// <summary>Opens the window, runs the loop until it closes, and returns the exit code.</summary>
    public int Run()
    {
        if (!TryPrepare())
        {
            Console.Error.WriteLine(_startupError);
            return 1;
        }

        VideoPacingRequest pacing = PacingRequest();
        HostWindowSettings settings = new(
            "Zaxnes",
            _options.Width,
            _options.Height,
            VSync: pacing.VSync,
            TargetFrameRate: pacing.TargetFrameRate);

        _host = HostWindow.Create(settings);
        _host.Window.Load += OnLoad;
        _host.Window.Render += OnRender;
        _host.Window.FramebufferResize += OnFramebufferResize;
        _host.Window.Closing += OnClosing;

        if (_options.SmokeTest)
        {
            Console.Out.WriteLine(_options.SmokeEmulatedFrames > 0
                ? string.Create(
                    CultureInfo.InvariantCulture,
                    $"Zaxnes smoke test: playing {_options.SmokeEmulatedFrames} emulated frames.")
                : string.Create(
                    CultureInfo.InvariantCulture,
                    $"Zaxnes smoke test: rendering {_options.SmokeTestFrames} frames."));
        }

        _host.Run();

        if (!_options.SmokeTest)
        {
            return 0;
        }

        Console.Out.WriteLine(string.Create(
            CultureInfo.InvariantCulture,
            $"Zaxnes smoke test: rendered {_renderedFrames} frames and emulated "
                + $"{_emulatedFrames} on {_glRenderer} ({_glVersion})."));

        if (_options.SmokeEmulatedFrames > 0)
        {
            // AMD-024 REQ-PUB-001: what the run had to reach is a number of emulated frames,
            // that is a duration of play, not a number of redraws.
            if (_emulatedFrames >= _options.SmokeEmulatedFrames)
            {
                return 0;
            }

            Console.Error.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"Zaxnes smoke test: expected {_options.SmokeEmulatedFrames} emulated frames, "
                    + $"only {_emulatedFrames} were produced."));
            return 1;
        }

        if (_renderedFrames >= _options.SmokeTestFrames)
        {
            return 0;
        }

        Console.Error.WriteLine(string.Create(
            CultureInfo.InvariantCulture,
            $"Zaxnes smoke test: expected {_options.SmokeTestFrames} frames, only {_renderedFrames} rendered."));
        return 1;
    }

    /// <summary>
    /// Whether the smoke run has done what it was asked for: a number of emulated frames when
    /// <c>--smoke-emulated</c> was given, a number of rendered frames otherwise.
    /// </summary>
    private bool SmokeRunIsOver(EmulationHost emulation)
        => _options.SmokeEmulatedFrames > 0
            ? emulation.EmulatedFrames >= _options.SmokeEmulatedFrames
            : _renderedFrames >= _options.SmokeTestFrames;

    /// <summary>
    /// Resolves the user root, reads the settings and the library and decides which game the
    /// command line asks for, before any window exists.
    /// </summary>
    private bool TryPrepare()
    {
        string? assetRoot = NeutralAssets.Locate(_options.AssetRoot);

        _paths = UserDataPaths.Resolve(_options.UserDataRoot);

        // AMD-029 §2: the first run after the rename seeds the new root from the old one.
        if (_paths.MigratedFrom is { } legacyRoot)
        {
            Log($"user data copied from '{legacyRoot}' to '{_paths.Root}'; "
                + "the old folder was left untouched (AMD-029).");
        }

        _settingsStore = new SettingsStore(_paths);
        _settingsStore.Load(_settings, DateTimeOffset.UtcNow);

        // The command line wins over the file, but only for the options it really carries
        // (Q-066); then the scene gets the values that came out of the two.
        ApplyCommandLineOverrides();
        PublishSettingsToScene();

        if (!_paths.IsWritable)
        {
            Console.Error.WriteLine($"Zaxnes: {_paths.UnwritableMessage}");
        }

        _catalog = LibraryGameCatalog.Open(assetRoot, _paths, Log);

        if (_options.RomPath is not null && !File.Exists(_options.RomPath))
        {
            _startupError = $"Zaxnes: the ROM '{_options.RomPath}' does not exist.";
            return false;
        }

        _startupGameId = ResolveStartupGame();
        return true;
    }

    private void ApplyCommandLineOverrides() => CommandLineSettings.Apply(_options, _settings);

    /// <summary>
    /// Pushes the settings in force onto the scene, once, before the first frame. Afterwards
    /// every change travels on its own through <c>EmulationHost.OnSettingChanged</c>
    /// (REQ-PERS-009).
    /// </summary>
    private void PublishSettingsToScene()
    {
        _scene.Settings.Fov = _settings.Fov;
        _scene.Camera.Fov = _settings.Fov;
        _scene.Settings.Shadows = _settings.Shadow;
        _scene.Settings.Skybox = (SkyboxKind)_settings.Skybox;
        _scene.Settings.TvBox = TvBoxSetting.From(_settings.TvBox);
        _scene.Settings.SkipRender = _settings.SkipFrame;
        _scene.Settings.TargetFrameRate = _settings.FrameRate;
        _scene.Settings.VSync = _settings.VSync;
        _scene.Settings.LightIntensity = _settings.LightIntensity;
        _scene.Camera.SnapToSetpoints();
    }

    /// <summary>
    /// What the window should be paced at right now (REQ-SCN-061). A smoke run is never paced by
    /// the display refresh rate unless <c>--vsync</c> was given on purpose: it must render its
    /// frames as fast as the machine allows and exit.
    /// </summary>
    private VideoPacingRequest PacingRequest() => _options.SmokeTest
        ? new VideoPacingRequest(_options.VSync ?? 0, 0)
        : new VideoPacingRequest(_settings.VSync, _settings.FrameRate);

    /// <summary>
    /// Maps <c>--profile</c> onto a library entry: an exact identifier or title, an unambiguous
    /// title prefix, or the identifier a profile file declares. Without a match the "2D Mode"
    /// entry is used (REQ-SCN-055).
    /// </summary>
    private string? ResolveStartupGame()
    {
        GameLibrary library = _catalog!.Library;
        string? request = _options.Profile;

        if (!string.IsNullOrWhiteSpace(request) && File.Exists(request))
        {
            ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(request), sourceName: request);
            string? id = result.IsSuccess ? result.Profile?.Id : null;
            if (id is not null && library.Find(id) is { } matched)
            {
                return matched.Id;
            }

            Log($"the profile '{request}' is not in the library; copy it into "
                + $"'{_paths!.ProfilesDirectory}' to play it. Falling back to 2D Mode.");
            request = null;
        }

        if (!string.IsNullOrWhiteSpace(request))
        {
            foreach (GameLibraryEntry entry in library.Entries)
            {
                if (string.Equals(entry.Id, request, StringComparison.OrdinalIgnoreCase)
                    || string.Equals(entry.DisplayTitle, request, StringComparison.OrdinalIgnoreCase))
                {
                    return entry.Id;
                }
            }

            GameLibraryEntry? unique = null;
            foreach (GameLibraryEntry entry in library.Entries)
            {
                if (!entry.DisplayTitle.StartsWith(request, StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                if (unique is not null)
                {
                    Log($"'{request}' matches several games; use the exact title.");
                    return _catalog.TwoDimensionalModeId;
                }

                unique = entry;
            }

            if (unique is not null)
            {
                return unique.Id;
            }

            Log($"no game matches '{request}'. Use --list-profiles to see the titles.");
        }

        return _catalog.TwoDimensionalModeId;
    }

    private void OnLoad()
    {
        HostWindow host = _host ?? throw new InvalidOperationException("Window was not created.");

        // ICON-1: the native window exists from here on, which is the earliest the window icon
        // can be set. It is cosmetic — AppIcon reports instead of throwing, so a missing or
        // broken resource costs one log line and nothing more.
        AppIcon.Apply(host.Icon, Log);

        _gl = GL.GetApi(host.Window);
        _glRenderer = _gl.GetStringS(StringName.Renderer) ?? "unknown";
        _glVersion = _gl.GetStringS(StringName.Version) ?? "unknown";

        // FIX-1: the GL context exists from here on, so this is the first moment the swap
        // interval of REQ-SCN-061 can be set — from the persisted value, not from the command
        // line — and every later change of video.vsync or video.frame_rate follows.
        _pacing = new VideoPacing(host.Display, _settings, PacingRequest, Log);
        _pacing.Apply();

        _offscreen = new OffscreenFramebuffer(_gl, host.Window.FramebufferSize.X, host.Window.FramebufferSize.Y);

        // REQ-SCN-057: the host, not the renderer, decides where the assets of REQ-SCN-050,
        // REQ-SCN-051 and REQ-SCN-054 come from.
        string? assetRoot = NeutralAssets.Locate(_options.AssetRoot);

        // AMD-025: the television is the CC BY glTF file the release ships under assets/tv.
        // Without it the model is simply absent and the TV option does nothing (REQ-PUB-001:
        // the application runs with no asset at all).
        TvModel? television = TvModel.TryLoad(TelevisionAsset.Locate(_options.TvModelPath));
        if (television is null)
        {
            Log("no television model found; the TV box option is disabled.");
        }
        else
        {
            Log($"television model: {television.Describe()}");
        }

        _television = television;
        _renderer = new SceneRenderer(_gl, NeutralSceneAssets.TryOpen(assetRoot), television);
        _imgui = new ImGuiController(_gl, host.Window, host.Input, ConfigureImGuiIo);

        // REQ-UI-207: the controller has just built the font atlas and uploaded its texture, so
        // the glyph index tables exist; this is the earliest moment a missing code point can be
        // pointed at a glyph that is in the atlas.
        UiFonts.RemapMissingGlyphs();

        _keyboard = host.Input.Keyboards.Count > 0 ? host.Input.Keyboards[0] : null;
        _mouse = host.Input.Mice.Count > 0 ? host.Input.Mice[0] : null;

        OpenInput(host);
        OpenAudio(assetRoot);
        StartEmulation();
        OpenUserInterface();
        StartGameFromCommandLine();

        if (_options.UiShotDirectory is { } shots)
        {
            _shots = new UiShots(shots);
        }
    }

    /// <summary>
    /// Builds the classic interface of AMD-010 once the host, the library and the input layer
    /// exist, and gives it the window-level services it cannot provide itself.
    /// </summary>
    private void OpenUserInterface()
    {
        _ui = new ZaxUi(new ZaxUiOptions
        {
            Host = _emulation!,
            Catalog = _catalog!,
            Games = _settingsStore!.Games,
            Paths = _paths!,
            Scene = _scene,
            Bindings = _bindings,
            Input = _input,
            Mixer = _mixer,
            FramesPerSecond = () => (float)_frameRate.FramesPerSecond,
            Log = Log,
            RevealFolder = RevealFolder,
        });
    }

    /// <summary>
    /// Opens the folder of « Outils ▸ Dossier des données » (SPEC-09 REQ-PERS-072) in the
    /// desktop's file manager. A failure is logged and nothing else: the path is also printed.
    /// </summary>
    private static void RevealFolder(string path)
    {
        try
        {
            Directory.CreateDirectory(path);
            using System.Diagnostics.Process? _ = System.Diagnostics.Process.Start(
                new System.Diagnostics.ProcessStartInfo(path) { UseShellExecute = true });
        }
        catch (Exception exception) when (exception is System.ComponentModel.Win32Exception
            or IOException
            or UnauthorizedAccessException
            or System.PlatformNotSupportedException)
        {
            Log($"the data folder could not be opened: {exception.Message}");
        }
    }

    /// <summary>
    /// Builds the input layer of SPEC-08: the bindings read from <c>input.json</c>
    /// (REQ-PERS-065), the devices, and the host source that turns them into the snapshots of
    /// REQ-INP-120.
    /// </summary>
    private void OpenInput(HostWindow host)
    {
        _bindingStore = new InputBindingStore(_paths!);
        _bindingStore.Load(_bindings);

        _pointer = _mouse is not null ? new SilkPointer(_mouse) : null;
        _input = new DesktopInputSource(_bindings, new SilkGamepadHub(host.Input), _bindingStore)
        {
            Keyboard = _keyboard is not null ? new SilkKeyboard(_keyboard) : null,
            Pointer = _pointer,
        };
    }

    private void OpenAudio(string? assetRoot)
    {
        int rate = AudioOutput.PreferredSampleRate;

        if (_options.Audio)
        {
            _audio = new AudioOutput();
            if (_audio.TryOpen())
            {
                rate = _audio.SampleRate;
            }
            else
            {
                Log($"no audio output ({_audio.LastError}); the host runs silent.");
                _audio.Dispose();
                _audio = null;
            }
        }

        // AMD-024 REQ-PUB-005: the interface audio is synthesised. An asset folder given with
        // --assets may replace it with its own WAV files.
        IUiAudioLibrary library = AudioAssets.TryOpen(assetRoot, Log) is { } files
            ? files
            : new SynthAudioLibrary(rate);

        _mixer = new AudioMixer(rate, library)
        {
            Volume = _settings.EffectiveVolume,
            MenuMusicEnabled = _settings.MenuMusic,
        };
    }

    private void StartEmulation()
    {
        _emulation = new EmulationHost(new EmulationHostOptions
        {
            Paths = _paths!,
            Settings = _settings,
            Scene = _scene,
            Games = _catalog!,
            Mixer = _mixer,
            ScriptLog = new ConsoleScriptLog(),
            CaptureThumbnail = CaptureThumbnail,
            AudioDeviceOpen = _audio is { IsOpen: true },
            Log = Log,

            // AMD-010 REQ-UI-200: the ten recent ROMs, persisted beside the library.
            RomOpened = path =>
            {
                _settingsStore?.Games.NoteRomOpened(path);
                _settingsStore?.ScheduleWrite(TimeSpan.Zero);
            },

            // REQ-UI-201 with Q-036: the profile chosen per ROM file lives in games.json.
            ProfileForRom = path => _settingsStore?.Games.ProfileFor(path),
            RomProfileChosen = (path, gameId) =>
            {
                _settingsStore?.Games.SetProfileFor(path, gameId);
                _settingsStore?.ScheduleWrite(TimeSpan.Zero);
            },

            // REQ-UI-004 cases 2 and 3: the loop must know when the interface holds a modal
            // window, so that Escape goes to the window instead of pausing the game.
            ModalWindowOpen = () => _ui?.State.IsModal ?? false,

            // REQ-UI-110: the confirmation box E8 the deletions of SPEC-09 go through.
            Confirm = request => _ui?.AskForConfirmation(request) ?? false,

            // REQ-PERS-015: Setting:Save() and Setting:Load() from the Lua console.
            SaveSettingsRequested = () => _settingsStore?.Save(_settings),
            ReloadSettingsRequested = () => _settingsStore?.Load(_settings, DateTimeOffset.UtcNow),
        });

        _emulation.UiRequested += OnUiRequested;

        // AMD-025: with no model file the TV box setting is disabled and forced off.
        _emulation.TelevisionAvailable = _television is not null;
        if (_television is null)
        {
            _settings.TvBox = 0;
            _scene.Settings.TvBox = TvBoxKind.None;
        }

        if (_emulation.Slots.WasRecovered)
        {
            _emulation.Notifications.Show(
                "Save slots were rebuilt from the state files.",
                NotificationCenter.DefaultSeconds,
                NotificationSeverity.Warning);
        }

        foreach (string message in _settingsStore?.Messages ?? [])
        {
            _emulation.Notifications.Show(message, NotificationCenter.DefaultSeconds, NotificationSeverity.Warning);
        }

        if (_paths is { IsWritable: false, UnwritableMessage: { } unwritable })
        {
            _emulation.Notifications.ShowError(unwritable);
        }

    }

    /// <summary>
    /// What the command line asks for, once the interface exists (AMD-010 §7 of lot L9):
    /// without <c>--rom</c> the window opens on the welcome screen (REQ-UI-207); with
    /// <c>--rom</c> and <c>--profile</c> the pair is remembered and the game starts; with
    /// <c>--rom</c> alone the resolution of REQ-UI-201 runs, which means the profile chooser
    /// opens the first time that file is seen.
    /// </summary>
    private void StartGameFromCommandLine()
    {
        EmulationHost emulation = _emulation!;

        if (_options.RomPath is null)
        {
            Console.Out.WriteLine(
                "Zaxnes: no --rom given; the window opens on the welcome screen (AMD-010 REQ-UI-207).");
            return;
        }

        string rom = Path.GetFullPath(_options.RomPath);

        // A smoke test must never stop on a dialog, so it takes the resolved entry — "2D Mode"
        // when no profile was named — rather than the interactive flow.
        if (_options.Profile is not null || _options.SmokeTest)
        {
            if (_startupGameId is { } gameId)
            {
                emulation.LaunchRomWithProfile(rom, gameId);
                return;
            }
        }

        emulation.Enqueue(ZaxCommands.OpenRom, CommandArgs.Of(rom));
    }

    private int ReadEmulationAudio(Span<float> destination)
        => _emulation?.Session is { } session ? session.Core.ReadAudio(destination) : 0;

    private byte[]? CaptureThumbnail()
        => _offscreen?.ReadScaledRgb(EmulationHost.ThumbnailWidth, EmulationHost.ThumbnailHeight);

    private void OnUiRequested(string command, CommandArgs args)
    {
        // REQ-UI-209: the window state follows ui.fullscreen, whoever flipped it.
        if (command == ZaxCommands.ToggleFullscreen && _host is { } host)
        {
            host.Window.WindowState = _settings.Fullscreen
                ? Silk.NET.Windowing.WindowState.Fullscreen
                : Silk.NET.Windowing.WindowState.Normal;

            // FIX-1: a window-state change makes the windowing layer re-apply its own boolean
            // vsync, which would lose the interval 2 and, on some drivers, the interval
            // altogether. The tearing the user saw in full screen is this.
            _pacing?.Apply();
        }

        if (_ui is { } ui)
        {
            ui.HandleUiRequest(command, args);
            return;
        }

        Log($"the command '{command}' arrived before the interface existed.");
    }

    private unsafe void ConfigureImGuiIo()
    {
        // Dear ImGui would otherwise drop an imgui.ini next to the working directory.
        ImGui.GetIO().NativePtr->IniFilename = null;

        // REQ-UI-121 with AMD-024 REQ-PUB-006: the interface font is one the machine already
        // has, opened at run time and never redistributed. Without it the French labels of
        // AMD-010 lose their accents, so this is not only a matter of style.
        if (UiFonts.Load(_options.FontFile, _options.BuiltInFont))
        {
            Log($"interface font: {UiFonts.BodyFile}");
        }
        else
        {
            Log("no system font was loaded; the built-in ImGui font has no accents.");
        }
    }

    private void OnFramebufferResize(Silk.NET.Maths.Vector2D<int> size)
        => _offscreen?.Resize(size.X, size.Y);

    private void OnRender(double deltaSeconds)
    {
        HostWindow host = _host!;
        GL gl = _gl!;
        OffscreenFramebuffer offscreen = _offscreen!;
        ISceneRenderer renderer = _renderer!;
        ImGuiController imgui = _imgui!;
        EmulationHost emulation = _emulation!;

        _frameRate.Add(deltaSeconds);
        _renderedFrames++;

        float real = (float)Math.Clamp(deltaSeconds, 0.0, 0.25);
        InputSnapshot snapshot = SampleInput(host, real);

        HandleMenuShortcuts();
        HandleBoundCommands();
        HandleCameraInput(snapshot, real);
        UpdateCursor(real, snapshot);

        // One rendered frame of the loop: the Zapper, the hotkeys, the commands, then the
        // emulated frames due (REQ-INP-100, REQ-INP-005).
        int emulated = emulation.Tick(real, snapshot);
        _input?.EndSample();
        UpdateScene();

        // The camera keeps being smoothed whatever the emulation does (REQ-SCN-059).
        _scene.Camera.Advance(real);
        _scene.RealTime += real;

        // REQ-PERS-021: a change made from the UI reaches the disk at the latest two seconds
        // later, so a brutal stop does not lose it.
        _settingsStore?.Tick(_settings, TimeSpan.FromSeconds(_scene.RealTime));

        // REQ-UI-200 / REQ-UI-209: the game view sits under the menu bar, and takes the whole
        // window as soon as full screen hides it.
        int windowWidth = host.Window.FramebufferSize.X;
        int windowHeight = host.Window.FramebufferSize.Y;
        int viewHeight = Math.Max(1, windowHeight - _viewInset);
        offscreen.Resize(windowWidth, viewHeight);

        // REQ-SCN-060 / REQ-LOOP-014: with render skip on, a refresh that brought no new
        // emulated frame keeps the previous image.
        bool skip = RenderSkip.ShouldSkip(
            _scene.Settings.SkipRender,
            emulation.State == EmulationState.Running,
            emulated,
            _lastRenderSize == (windowWidth, viewHeight));
        if (!skip)
        {
            renderer.Render(offscreen.Target, _scene);
            _lastRenderSize = (windowWidth, viewHeight);
            _sceneFrames++;
        }
        else
        {
            _skippedFrames++;
        }

        gl.Viewport(0, 0, (uint)windowWidth, (uint)windowHeight);
        gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);
        gl.Clear((uint)ClearBufferMask.ColorBufferBit);
        offscreen.BlitToDefaultFramebuffer(windowWidth, viewHeight);

        imgui.Update((float)deltaSeconds);
        DrawUserInterface(real);
        imgui.Render();

        gl.BindFramebuffer(FramebufferTarget.Framebuffer, 0);

        PumpAudio(real);

        if (_shots is { } shots && _ui is { } shotUi)
        {
            shots.Advance(shotUi, path => CaptureWindow(gl, windowWidth, windowHeight, path));
            if (shots.Finished)
            {
                host.Window.Close();
                return;
            }
        }

        if (emulation.QuitRequested || (_options.SmokeTest && SmokeRunIsOver(emulation)))
        {
            // --smoke-shot writes the last frame of the run: the same picture the window
            // shows, captured through the application's own path rather than the harness's,
            // which is what a "the app looks like this" verification capture has to be.
            if (_options.SmokeShotPath is { } shot)
            {
                CaptureWindow(gl, windowWidth, windowHeight, shot);
            }

            host.Window.Close();
        }
    }

    /// <summary>
    /// Reads the window back and writes it as a PNG, for <c>--ui-shots</c>. GL hands the rows
    /// bottom-up, so they are flipped on the way out.
    /// </summary>
    private static unsafe void CaptureWindow(GL gl, int width, int height, string path)
    {
        byte[] rows = new byte[width * height * 3];
        gl.BindFramebuffer(FramebufferTarget.ReadFramebuffer, 0);
        gl.PixelStore(PixelStoreParameter.PackAlignment, 1);
        fixed (byte* pixels = rows)
        {
            gl.ReadPixels(0, 0, (uint)width, (uint)height, PixelFormat.Rgb, PixelType.UnsignedByte, pixels);
        }

        byte[] flipped = new byte[rows.Length];
        int stride = width * 3;
        for (int y = 0; y < height; y++)
        {
            Array.Copy(rows, (height - 1 - y) * stride, flipped, y * stride, stride);
        }

        Render.Assets.PngImage.WriteRgb(path, width, height, flipped);
        Log($"captured {path}");
    }

    /// <summary>
    /// The menu shortcuts of AMD-010 REQ-UI-204: the rising edge of every watched key is
    /// collected here, and <see cref="MenuShortcuts"/> decides which command each one invokes.
    /// </summary>
    private void HandleMenuShortcuts()
    {
        if (_keyboard is not { } keyboard || _ui is not { } ui)
        {
            return;
        }

        // While a binding capture is listening, the keyboard belongs to the capture: the fixed
        // shortcuts of REQ-UI-204 must not fire either, for the same reason REQ-INP-047 and
        // DEC-08-2 suspend the actions.
        if (_input is { Rebinding.SuppressesActions: true })
        {
            _heldKeys.Clear();
            return;
        }

        // A text field of the interface — the two search boxes — owns the keyboard while it has
        // the focus, so a shortcut must not fire from what is being typed.
        if (ImGui.GetIO().WantCaptureKeyboard)
        {
            _heldKeys.Clear();
            return;
        }

        _pressedKeys.Clear();
        foreach (Key key in MenuShortcuts.WatchedKeys)
        {
            bool down = keyboard.IsKeyPressed(key);
            if (down && _heldKeys.Add(key))
            {
                _pressedKeys.Add(key);
            }
            else if (!down)
            {
                _heldKeys.Remove(key);
            }
        }

        ui.HandleShortcuts(
            _pressedKeys,
            MenuShortcuts.ModifiersOf(
                keyboard.IsKeyPressed(Key.ControlLeft) || keyboard.IsKeyPressed(Key.ControlRight),
                keyboard.IsKeyPressed(Key.ShiftLeft) || keyboard.IsKeyPressed(Key.ShiftRight),
                keyboard.IsKeyPressed(Key.AltLeft) || keyboard.IsKeyPressed(Key.AltRight)));
    }

    /// <summary>
    /// The fixed shortcuts that act <b>without</b> a modifier. REQ-INP-106 lets an
    /// implementation add fixed shortcuts provided they use no key of the table of §4.2, which
    /// is what this list has to keep true.
    /// </summary>
    internal static IReadOnlyList<Key> BareShortcutKeys => MenuShortcuts.BareKeys;

    private readonly HashSet<Key> _heldKeys = [];
    private readonly HashSet<Key> _pressedKeys = [];

    /// <summary>
    /// Samples the devices into the platform-independent snapshot of REQ-INP-120 and tells the
    /// host layer what the interface is currently eating.
    /// </summary>
    private InputSnapshot SampleInput(HostWindow host, float realSeconds)
    {
        if (_input is not { } input)
        {
            return InputSnapshot.Empty;
        }

        input.KeyboardCapturedByUi = ImGui.GetIO().WantCaptureKeyboard;
        input.PointerOverUi = ImGui.GetIO().WantCaptureMouse;
        input.RenderTargetSize = new Vector2(
            host.Window.FramebufferSize.X,
            host.Window.FramebufferSize.Y);

        return input.Sample(realSeconds);
    }

    /// <summary>
    /// Queues the commands a key or a pad control is bound to (AMD-010 REQ-UI-203): the model is
    /// the single route, so a bound command behaves exactly like its menu entry.
    /// </summary>
    private void HandleBoundCommands()
    {
        if (_input is not { } input || _emulation is not { } emulation)
        {
            return;
        }

        foreach (string command in input.TriggeredCommands)
        {
            emulation.Enqueue(command);
        }
    }

    private void HandleCameraInput(in InputSnapshot snapshot, float realSeconds)
    {
        // REQ-SCN-018 / REQ-INP-083: the camera controls only act while the game is running.
        if (_emulation is not { State: EmulationState.Running } emulation)
        {
            _dragOrigin = null;
            return;
        }

        // REQ-INP-080: a player whose port is in Zapper does not drive the camera.
        CameraAction camera = InputActions.ToCameraActions(InputRouting.CameraSources(
            snapshot.PlayerOne,
            snapshot.PlayerTwo,
            emulation.PortType(0),
            emulation.PortType(1)));

        CameraControls.Apply(_scene.Camera, camera, realSeconds);

        // REQ-INP-090: the wheel is ignored over a piece of interface and while a zoom action is
        // held; 5 scene units per notch (DEC-08-3).
        if (snapshot.Wheel != 0f && !snapshot.PointerOverUi)
        {
            CameraControls.ApplyWheel(_scene.Camera, snapshot.Wheel, camera);
        }

        // REQ-INP-091: left-button drag orbits, 0.1° per pixel; Shift + drag has no effect on
        // desktop (it was the editor's selection gesture, out of scope).
        bool shift = _keyboard is not null
            && (_keyboard.IsKeyPressed(Key.ShiftLeft) || _keyboard.IsKeyPressed(Key.ShiftRight));
        bool dragging = snapshot.LeftButton && !snapshot.PointerOverUi && !shift;

        if (dragging)
        {
            if (_dragOrigin is { } origin)
            {
                // The screen y axis points down, so the vertical delta is negated
                // (REQ-SCN-017).
                CameraControls.ApplyDrag(
                    _scene.Camera,
                    snapshot.Pointer.X - origin.X,
                    origin.Y - snapshot.Pointer.Y);
            }

            _dragOrigin = snapshot.Pointer;
        }
        else
        {
            _dragOrigin = null;
        }
    }

    /// <summary>
    /// The cursor masking of REQ-INP-110 and REQ-INP-111: hidden after two still seconds while a
    /// game runs, always visible otherwise.
    /// </summary>
    private void UpdateCursor(float realSeconds, in InputSnapshot snapshot)
    {
        if (_pointer is not { } pointer)
        {
            return;
        }

        bool visible = _cursor.Update(
            _emulation?.State == EmulationState.Running,
            snapshot.PointerMoved,
            realSeconds);

        if (pointer.CursorVisible != visible)
        {
            pointer.CursorVisible = visible;
        }
    }

    private void UpdateScene()
    {
        if (_emulation?.Session is not { } session)
        {
            _scene.DrawList = null;
            _scene.TwoDimensionalMode = false;
            _scene.NesPicture = default;
            _scene.FollowedShape = null;
            return;
        }

        _scene.DrawList = session.BuildDrawList();
        _scene.NesBackgroundColor = session.NesBackgroundColor;
        _scene.EmulatedTime = session.EmulatedTime;
        _scene.Clipping = session.Engine.Settings.Clipping;

        // AMD-009 (Q-022): the 2D mode shows the core's pixel frame on the NES plane. AMD-017
        // (Q-037) adds the hot F11 toggle to the "2D Mode" profile of REQ-SCN-055: the host
        // combines the two, and nothing of the session is reloaded to switch.
        _scene.TwoDimensionalMode = _emulation!.ShowingTwoDimensional;
        if (_scene.TwoDimensionalMode)
        {
            _picture ??= new byte[256 * 240];
            session.NesPicture.CopyTo(_picture);
            _scene.NesPicture = _picture;
        }

        // REQ-SCN-022: the renderer needs the followed shape to find the occluders.
        _scene.FollowedShape = session.FollowedShape;
    }

    private void PumpAudio(float realSeconds)
    {
        if (_mixer is not { } mixer)
        {
            return;
        }

        mixer.EmulationSource = ReadEmulationAudio;

        if (_audio is { IsOpen: true } audio)
        {
            // The device pulls as many whole blocks as it has free buffers; the mixer's own
            // clock therefore advances with the audio it produced, which is real time.
            if (audio.Pump(buffer => mixer.Fill(buffer)) == 0)
            {
                mixer.Advance(realSeconds);
            }
        }
        else
        {
            // No device: the fades and the music selection still advance, so the state machine
            // behaves identically in silence.
            mixer.Advance(realSeconds);
        }
    }

    /// <summary>
    /// One frame of interface: the classic one of AMD-010, then the developer panel when it has
    /// been asked for (off by default, SPEC-10 REQ-UI-095 and DEC-10-Q7).
    /// </summary>
    private void DrawUserInterface(float realSeconds)
    {
        if (_ui is not { } ui)
        {
            return;
        }

        ui.Draw(realSeconds);
        _viewInset = (int)MathF.Round(ui.ChromeHeight);

        if (!ui.State.DebugOverlayVisible)
        {
            return;
        }

        ImGui.SetNextWindowSize(new Vector2(420.0f, 340.0f), ImGuiCond.FirstUseEver);
        ImGui.SetNextWindowPos(new Vector2(16.0f, ui.ChromeHeight + 16.0f), ImGuiCond.FirstUseEver);

        if (ImGui.Begin("Statistiques (F12)"))
        {
            EmulationHost emulation = _emulation!;
            RenderStatistics stats = _renderer!.Statistics;

            ImGui.Text(string.Create(
                CultureInfo.InvariantCulture,
                $"{_frameRate.FramesPerSecond:F1} FPS  ({_frameRate.FrameMilliseconds:F2} ms/frame)"));
            ImGui.Text(string.Create(
                CultureInfo.InvariantCulture,
                $"render {_renderedFrames}   emulated {emulation.EmulatedFrames}"));
            ImGui.Text(string.Create(
                CultureInfo.InvariantCulture,
                $"scene {_sceneFrames}   skipped {_skippedFrames}   target {_settings.FrameRate} fps"));

            ImGui.Separator();
            ImGui.Text(string.Create(
                CultureInfo.InvariantCulture,
                $"state {emulation.State}{(emulation.IsLoading ? $"  loading {emulation.LoadProgress * 100f:F0} pct" : string.Empty)}"));
            ImGui.Text(string.Create(
                CultureInfo.InvariantCulture,
                $"speed s={_settings.GameSpeed} (k={_settings.SpeedFactor:F2})"
                    + $"{(emulation.FastForward.IsActive ? $"  fast forward {emulation.FastForward.Remaining}" : string.Empty)}"));
            ImGui.Text(string.Create(
                CultureInfo.InvariantCulture,
                $"rollback {emulation.Rollback.Count}/{RollbackHistory.Capacity}   "
                    + $"quick slot {(emulation.Slots.Find(SaveSlotIndex.QuickSlotId) is { IsOccupied: true } q ? SaveStateFile.FormatTimestamp(q.SavedAt) : "empty")}"));

            if (emulation.CurrentGameTitle is { } title)
            {
                ImGui.Separator();
                ImGui.TextWrapped(title);
                if (emulation.Session is { } session)
                {
                    ImGui.Text(string.Create(
                        CultureInfo.InvariantCulture,
                        $"{session.Core.Region}  {session.NominalFrameRate} fps nominal  frame {session.Core.FrameCount}"));
                }
            }

            ImGui.Separator();
            ImGui.Text(string.Create(
                CultureInfo.InvariantCulture,
                $"tiles  opaque {stats.OpaqueTiles}  alpha {stats.TransparentTiles}  HUD {stats.HudTiles}"));
            ImGui.Text(string.Create(
                CultureInfo.InvariantCulture,
                $"casters {stats.ShadowCasters}   draw calls {stats.DrawCalls}   tris {stats.Triangles}"));

            if (_mixer is { } mixer)
            {
                ImGui.Separator();
                ImGui.Text(string.Create(
                    CultureInfo.InvariantCulture,
                    $"audio {mixer.SampleRate} Hz  emu gain {mixer.EmulationGain:F3}  music {mixer.MusicGain:F4}"
                        + $"  track {mixer.CurrentTrack}  underruns {mixer.UnderrunBlocks}"));
            }

            ImGui.Separator();
            ImGui.TextWrapped(_glRenderer);
        }

        ImGui.End();
    }

    private static void Log(string message) => Console.Out.WriteLine($"Zaxnes: {message}");

    private void OnClosing()
    {
        // REQ-UI-067: a binding capture in progress stops before anything is written.
        _ui?.CloseCapture();
        _ui = null;

        // REQ-LOOP-052: the current game is stopped (End hook), the settings and the slot
        // metadata are written, and nothing is auto-saved.
        if (_emulation is { } emulation)
        {
            _emulatedFrames = emulation.EmulatedFrames;
            emulation.Shutdown(_settingsStore);
            emulation.UiRequested -= OnUiRequested;
            emulation.Dispose();
            _emulation = null;
        }

        // REQ-INP-050 / REQ-PERS-065: the bindings and the gamepad assignment are written at
        // exit as well as after each change, and REQ-INP-111 asks for the cursor to come back.
        _bindingStore?.Save(_bindings);
        _input?.Detach();
        _input = null;
        _pointer?.Detach();
        _pointer = null;

        _catalog?.Save();
        _pacing?.Dispose();
        _pacing = null;
        _audio?.Dispose();
        _audio = null;
        _mixer = null;
        _imgui?.Dispose();
        _imgui = null;
        _renderer?.Dispose();
        _renderer = null;
        _offscreen?.Dispose();
        _offscreen = null;
        _gl?.Dispose();
        _gl = null;
    }

    /// <inheritdoc />
    public void Dispose()
    {
        OnClosing();
        _host?.Dispose();
        _host = null;
    }
}
