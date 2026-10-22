using System.Globalization;
using System.Reflection;
using Zax.App;
using Zax.App.Audio;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Zax.Scripting;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// A minimal NROM image the verifier builds itself, so the loop tests of SPEC-07 need no
/// external file: 32 KiB of PRG, 8 KiB of CHR, all three vectors pointing at <c>$8000</c>.
/// </summary>
public static class LoopRom
{
    /// <summary>Builds an image running <paramref name="program"/> from <c>$8000</c>.</summary>
    /// <param name="program">6502 code; an endless <c>JMP $8000</c> by default.</param>
    public static byte[] Build(byte[]? program = null)
    {
        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2;
        image[5] = 1;
        image[6] = 0x01;

        byte[] code = program is { Length: > 0 } ? program : [0x4C, 0x00, 0x80];
        code.CopyTo(image, 16);

        for (int vector = 0; vector < 3; vector++)
        {
            image[16 + (32 * 1024) - 6 + (vector * 2)] = 0x00;
            image[16 + (32 * 1024) - 5 + (vector * 2)] = 0x80;
        }

        return image;
    }

    /// <summary>An image that increments <c>$0010</c> for ever: the core state is a function of N.</summary>
    public static byte[] Counter() => Build([0xE6, 0x10, 0x4C, 0x00, 0x80]);

    /// <summary>An image whose header declares a PAL region (REQ-LOOP-017).</summary>
    public static byte[] Pal()
    {
        byte[] image = Build();

        // iNES 2.0 with the PAL region nibble, which is the public rule REQ-CORE-013 follows.
        image[7] = 0x08;
        image[12] = 0x01;
        return image;
    }
}

/// <summary>
/// Drives the desktop emulation loop of SPEC-07 headless.
/// </summary>
/// <remarks>
/// <para>
/// <c>EmulationHost</c>, <c>EmulationHostOptions</c>, <c>GameSession</c> and
/// <c>IGameCatalog</c> are <b>internal</b> to <c>Zax.App</c>, whose internals are visible only
/// to the implementers' own test assembly. The verifier must not modify anything under
/// <c>src/</c>, so the loop is driven through reflection over the shipped assembly: no
/// production file is touched and the code under test is exactly the one the application runs.
/// </para>
/// <para>
/// The library is the real one — the embedded catalogue of <c>assets-neutral</c> plus
/// <c>library.json</c> under a throw-away user root — so the launch flow of REQ-LOOP-020 and the
/// slot flow of REQ-LOOP-044 are exercised end to end.
/// </para>
/// </remarks>
public sealed class HostDriver : IDisposable
{
    private static readonly Assembly App = typeof(AppOptions).Assembly;
    private static readonly Type HostType = App.GetType("Zax.App.Loop.EmulationHost", true)!;
    private static readonly Type OptionsType = App.GetType("Zax.App.Loop.EmulationHostOptions", true)!;
    private static readonly Type CatalogType = App.GetType("Zax.App.LibraryGameCatalog", true)!;
    private static readonly Type SessionType = App.GetType("Zax.App.GameSession", true)!;

    private readonly object _host;
    private readonly object _catalog;
    private double _clock;
    private bool _disposed;

    private HostDriver(string root, SceneState scene, AudioMixer mixer, bool audioDeviceOpen)
    {
        Root = root;
        Scene = scene;
        Mixer = mixer;
        Paths = UserDataPaths.Resolve(root);
        Settings = new AppSettings();

        Action<string> log = line => Log.Add(line);
        // AMD-024 REQ-PUB-001: without the extracted assets the host builds its library over the
        // built-in catalogue, so the loop still starts; the tests that need one of the 101
        // embedded profiles are the opt-in ones.
        _catalog = CatalogType
            .GetMethod("Open", BindingFlags.Public | BindingFlags.Static)!
            .Invoke(null, [NeutralAssets.Available ? NeutralAssets.Root : null, Paths, log])!;

        object options = Activator.CreateInstance(OptionsType, nonPublic: true)!;
        Set(options, "Paths", Paths);
        Set(options, "Settings", Settings);
        Set(options, "Scene", scene);
        Set(options, "Games", _catalog);
        Set(options, "Mixer", mixer);
        Set(options, "ScriptLog", ScriptLog);
        Set(options, "Clock", (Func<double>)(() => _clock));
        Set(options, "Now", (Func<DateTimeOffset>)(() => Now));
        Set(options, "CaptureThumbnail", (Func<byte[]?>)(() => Thumbnail));
        Set(options, "Log", log);
        Set(options, "RomOpened", (Action<string>)(path => RecentRoms.Add(path)));
        Set(options, "AudioDeviceOpen", audioDeviceOpen);

        // SPEC-10 REQ-UI-004 cases 2 and 3 and REQ-UI-110: the two hooks a frontend provides.
        // Both consult mutable state of the driver, so a test turns them on without rebuilding
        // the host: ModalOpen says a modal window is up, HoldConfirmations makes the fake
        // frontend keep the box open instead of letting the loop act at once.
        Set(options, "ModalWindowOpen", (Func<bool>)(() => ModalOpen));
        Set(options, "Confirm", ConfirmHook(options));

        _host = Activator.CreateInstance(HostType, options)!;
        HostType.GetEvent("UiRequested")!.AddEventHandler(
            _host,
            (Action<string, CommandArgs>)((command, args) => UiCalls.Add((command, args))));
    }

    /// <summary>The throw-away user root.</summary>
    public string Root { get; }

    /// <summary>The layout of the user root.</summary>
    public UserDataPaths Paths { get; }

    /// <summary>The settings the loop reads.</summary>
    public AppSettings Settings { get; }

    /// <summary>The scene the camera, the light and the scripts live in.</summary>
    public SceneState Scene { get; }

    /// <summary>The output mixer.</summary>
    public AudioMixer Mixer { get; }

    /// <summary>Where <c>print</c> and the script errors go.</summary>
    public CollectingScriptLog ScriptLog { get; } = new();

    /// <summary>Lines the host logged.</summary>
    public List<string> Log { get; } = [];

    /// <summary>ROM paths the host reported as opened (AMD-010 REQ-UI-200).</summary>
    public List<string> RecentRoms { get; } = [];

    /// <summary>Everything the host asked the frontend to do (<c>UiRequested</c>).</summary>
    public List<(string Command, CommandArgs Args)> UiCalls { get; } = [];

    /// <summary>
    /// The confirmation boxes E8 the loop asked for, in order (SPEC-10 REQ-UI-110).
    /// </summary>
    public List<Confirmation> Confirmations { get; } = [];

    /// <summary>
    /// Whether the fake frontend holds the box open instead of telling the loop there is nobody
    /// to ask. False — the default — reproduces a headless host: the loop acts at once, which is
    /// also what the command-line paths do.
    /// </summary>
    public bool HoldConfirmations { get; set; }

    /// <summary>
    /// Whether a modal interface window is up (REQ-UI-004 cases 2 and 3). The loop reads it
    /// through <c>EmulationHostOptions.ModalWindowOpen</c>.
    /// </summary>
    public bool ModalOpen { get; set; }

    /// <summary>The wall clock the host stamps slots and launch dates with.</summary>
    public DateTimeOffset Now { get; set; } = DateTimeOffset.FromUnixTimeSeconds(1_761_000_000);

    /// <summary>What the pause capture callback returns, or null for "cannot capture".</summary>
    public byte[]? Thumbnail { get; set; }

    /// <summary>The monotonic clock the pacer reads.</summary>
    public double Clock => _clock;

    /// <summary>Creates a driver on a fresh temporary user root.</summary>
    public static HostDriver Create(bool audio = false, int audioSeed = 1234, bool audioDeviceOpen = false)
        => Create(null, audio, audioSeed, audioDeviceOpen);

    /// <summary>
    /// Creates a driver whose library also holds <paramref name="userProfile"/>, written into
    /// <c>&lt;root&gt;/profiles</c> as a <c>.3dn</c> file before the catalogue is built. That is
    /// how the verifier puts a profile of its own — with its own Lua scripts — into a real
    /// library (SPEC-03 REQ-PROF-062).
    /// </summary>
    public static HostDriver Create(
        GameProfile? userProfile,
        bool audio = false,
        int audioSeed = 1234,
        bool audioDeviceOpen = false)
    {
        string root = Path.Combine(Path.GetTempPath(), $"zax-verify-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);

        if (userProfile is not null)
        {
            ProfileExport.Normalize(userProfile);
            string folder = Path.Combine(root, "profiles");
            Directory.CreateDirectory(folder);
            File.WriteAllBytes(
                Path.Combine(folder, (userProfile.Title ?? "verifier") + ".3dn"),
                ProfileFormat.Save(userProfile));
        }

        // AMD-024 REQ-PUB-005: the WAV files when they are there, the synthesised library
        // otherwise — which is what the published build always uses.
        IUiAudioLibrary? library = audio
            ? AudioAssets.TryOpen(NeutralAssets.Root) as IUiAudioLibrary ?? new SynthAudioLibrary(48_000)
            : null;
        return new HostDriver(root, new SceneState(), new AudioMixer(48_000, library, new Random(audioSeed)), audioDeviceOpen);
    }

    // ------------------------------------------------------------------ host surface

    /// <summary>The current state (REQ-LOOP-001).</summary>
    public EmulationState State => (EmulationState)Get("State")!;

    /// <summary>The loading indicator (REQ-LOOP-001).</summary>
    public bool IsLoading => (bool)Get("IsLoading")!;

    /// <summary>Mesh generation progress, 0 to 1 (REQ-LOOP-003).</summary>
    public float LoadProgress => (float)Get("LoadProgress")!;

    /// <summary>The frame clock of §3.2.</summary>
    public FramePacer Pacer => (FramePacer)Get("Pacer")!;

    /// <summary>The rollback history of §3.4.</summary>
    public RollbackHistory Rollback => (RollbackHistory)Get("Rollback")!;

    /// <summary>The fast-forward model of REQ-LOOP-016.</summary>
    public FastForwardState FastForward => (FastForwardState)Get("FastForward")!;

    /// <summary>The notification area (SPEC-10 §3.10).</summary>
    public NotificationCenter Notifications => (NotificationCenter)Get("Notifications")!;

    /// <summary>The command model of AMD-010 REQ-UI-205.</summary>
    public CommandModel Commands => (CommandModel)Get("Commands")!;

    /// <summary>The slot index.</summary>
    public SaveSlotIndex Slots => (SaveSlotIndex)Get("Slots")!;

    /// <summary>Identifier of the current game, or null (REQ-LOOP-003).</summary>
    public string? CurrentGameId => (string?)Get("CurrentGameId");

    /// <summary>Emulated frames run since the host was built.</summary>
    public long EmulatedFrames => (long)Get("EmulatedFrames")!;

    /// <summary>Whether a quit was asked for (REQ-LOOP-052).</summary>
    public bool QuitRequested => (bool)Get("QuitRequested")!;

    /// <summary>Commands waiting for the end of a load (REQ-LOOP-007).</summary>
    public int PendingCommands => (int)Get("PendingCommands")!;

    /// <summary>The running session, or null in Stopped.</summary>
    public SessionDriver? Session
    {
        get
        {
            object? session = Get("Session");
            return session is null ? null : new SessionDriver(session);
        }
    }

    /// <summary>
    /// The <c>EmulationHost</c> itself, as an object: the UI models of AMD-010 are internal to
    /// <c>Zax.App</c> and are built over it by reflection (see <c>UiDriver</c>).
    /// </summary>
    public object HostObject => _host;

    /// <summary>The bound of REQ-ARCH-013 and REQ-PERS-056 on a savestate file.</summary>
    public int MaxSaveStateSize => (int)Get("MaxSaveStateSize")!;

    /// <summary>Captures the four sections of a savestate without writing a file.</summary>
    public SaveStateContent CaptureSaveState(string slotId)
        => (SaveStateContent)HostType.GetMethod("CaptureSaveState")!.Invoke(_host, [slotId])!;

    /// <summary>The mouse Zapper of SPEC-08 §3.7.</summary>
    public ZapperState Zapper => (ZapperState)Get("Zapper")!;

    /// <summary>Whether the Zapper is active right now (REQ-INP-061).</summary>
    public bool ZapperActive => (bool)Get("ZapperActive")!;

    /// <summary>The device type of a port (REQ-INP-060).</summary>
    public ControllerPortType PortType(int port)
        => (ControllerPortType)HostType.GetMethod("PortType")!.Invoke(_host, [port])!;

    /// <summary>The scene scale <c>e</c> the Zapper aim uses (REQ-INP-063).</summary>
    public float SceneScale
    {
        get => (float)Get("SceneScale")!;
        set => HostType.GetProperty("SceneScale")!.SetValue(_host, value);
    }

    /// <summary>Queues a command (REQ-LOOP-007).</summary>
    public void Enqueue(string command, CommandArgs args = default)
        => HostType.GetMethod("Enqueue")!.Invoke(_host, [command, args]);

    /// <summary>Applies the queued commands without running an emulated frame.</summary>
    public int ApplyCommands() => (int)HostType.GetMethod("ApplyCommands")!.Invoke(_host, [])!;

    /// <summary>Advances the monotonic clock and runs one rendered frame.</summary>
    public int Tick(double seconds = 1.0 / 60.0, InputAction actions = InputAction.None)
    {
        _clock += seconds;
        MethodInfo tick = HostType.GetMethods()
            .First(m => m.Name == "Tick" && m.GetParameters().Length == 3);
        return (int)tick.Invoke(_host, [seconds, actions, InputAction.None])!;
    }

    /// <summary>
    /// Advances the monotonic clock and runs one rendered frame with a mask for each player,
    /// which is what the hotkeys of SPEC-08 §3.11 are read on (REQ-INP-100).
    /// </summary>
    public int Tick(double seconds, InputAction playerOne, InputAction playerTwo)
    {
        _clock += seconds;
        MethodInfo tick = HostType.GetMethods()
            .First(m => m.Name == "Tick" && m.GetParameters().Length == 3);
        return (int)tick.Invoke(_host, [seconds, playerOne, playerTwo])!;
    }

    /// <summary>
    /// Advances the monotonic clock and runs one rendered frame from a full input snapshot
    /// (REQ-INP-120), which is the only way to drive the mouse and the Zapper.
    /// </summary>
    public int Tick(double seconds, in InputSnapshot snapshot)
    {
        _clock += seconds;
        MethodInfo tick = HostType.GetMethods()
            .First(m => m.Name == "Tick" && m.GetParameters().Length == 2);
        return (int)tick.Invoke(_host, [seconds, snapshot])!;
    }

    /// <summary>Runs rendered frames until no load is in progress.</summary>
    public void Settle(int maximumTicks = 256)
    {
        for (int tick = 0; tick < maximumTicks && IsLoading; tick++)
        {
            Tick(1.0 / 1000.0);
        }
    }

    /// <summary>Queues a command, applies it, then waits out any load it started.</summary>
    public void Run(string command, CommandArgs args = default)
    {
        Enqueue(command, args);
        ApplyCommands();
        Settle();
    }

    /// <summary>Runs about <paramref name="frames"/> emulated frames; returns how many ran.</summary>
    public int RunFrames(int frames, InputAction actions = InputAction.None)
    {
        int ran = 0;
        double period = 1.0 / Math.Max(1.0, Pacer.EffectiveRate);
        for (int step = 0; step < frames * 8 && ran < frames; step++)
        {
            ran += Tick(period, actions);
        }

        return ran;
    }

    /// <summary>The exit of REQ-LOOP-052.</summary>
    public void Shutdown(SettingsStore? store = null)
        => HostType.GetMethod("Shutdown")!.Invoke(_host, [store]);

    /// <summary>Writes a ROM image into the user root and returns its path.</summary>
    public string WriteRom(byte[] image, string name = "game.nes")
    {
        string path = Path.Combine(Root, name);
        File.WriteAllBytes(path, image);
        return path;
    }

    // -------------------------------------------------------------- the library

    /// <summary>The library model of the embedded catalogue plus the user profiles.</summary>
    public Zax.Profiles.Library.GameLibrary Library
        => (Zax.Profiles.Library.GameLibrary)CatalogType.GetProperty("Library")!.GetValue(_catalog)!;

    /// <summary>
    /// Imports a <c>.3dn</c> file through the real library (SPEC-09 REQ-PERS-041): returns the
    /// outcome name and the message the interface would show.
    /// </summary>
    public (string Outcome, string Message) ImportProfile(string path)
    {
        object result = CatalogType.GetMethod("ImportProfile")!.Invoke(_catalog, [path])!;
        Type type = result.GetType();
        return (
            type.GetProperty("Outcome")!.GetValue(result)!.ToString()!,
            (string)type.GetProperty("Message")!.GetValue(result)!);
    }

    /// <summary>Writes <c>library.json</c> (REQ-PERS-037).</summary>
    public void SaveLibrary() => CatalogType.GetMethod("Save")!.Invoke(_catalog, []);

    /// <summary>Identifier of the generic "2D Mode" entry.</summary>
    public string? TwoDimensionalModeId
        => (string?)CatalogType.GetProperty("TwoDimensionalModeId")!.GetValue(_catalog);

    /// <summary>Associates <paramref name="romPath"/> with the library entry.</summary>
    public void SetRomPath(string gameId, string romPath)
        => CatalogType.GetMethod("SetRomPath")!.Invoke(_catalog, [gameId, romPath]);

    /// <summary>The identifier of a library entry whose title starts with the prefix.</summary>
    public string EntryIdByTitle(string prefix)
    {
        foreach (var entry in Library.Entries)
        {
            if (entry.Title is { } title
                && title.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                return entry.Id;
            }
        }

        throw new InvalidOperationException($"no library entry whose title starts with '{prefix}'.");
    }

    /// <summary>
    /// Launches a game: associates the ROM, queues the launch command and waits out the load.
    /// </summary>
    public void Launch(string gameId, string romPath)
    {
        SetRomPath(gameId, Path.GetFullPath(romPath));
        Run(ZaxCommands.LaunchGame, CommandArgs.Of(gameId));
    }

    /// <summary>Launches the "2D Mode" entry on a ROM image written into the user root.</summary>
    public string Launch2DMode(byte[]? image = null, string name = "game.nes")
    {
        string id = TwoDimensionalModeId
            ?? throw new InvalidOperationException("the catalogue has no \"2D Mode\" entry.");
        string path = WriteRom(image ?? LoopRom.Counter(), name);
        Launch(id, path);
        return id;
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        (_host as IDisposable)?.Dispose();

        try
        {
            Directory.Delete(Root, recursive: true);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // A locked temporary folder must not fail a verification run.
        }
    }

    private object? Get(string name) => HostType.GetProperty(name)!.GetValue(_host);

    private static void Set(object target, string property, object? value)
        => target.GetType().GetProperty(property)!.SetValue(target, value);

    /// <summary>
    /// A delegate of the host's own <c>Func&lt;ConfirmationRequest, bool&gt;</c> type that routes
    /// to <see cref="OnConfirm"/>. <c>ConfirmationRequest</c> is internal to <c>Zax.App</c>, so
    /// the delegate cannot be written in C# here and is built as an expression instead — the same
    /// reason the rest of this driver uses reflection.
    /// </summary>
    private Delegate ConfirmHook(object options)
    {
        Type delegateType = options.GetType().GetProperty("Confirm")!.PropertyType;
        Type requestType = delegateType.GetGenericArguments()[0];

        System.Linq.Expressions.ParameterExpression request =
            System.Linq.Expressions.Expression.Parameter(requestType, "request");

        return System.Linq.Expressions.Expression.Lambda(
            delegateType,
            System.Linq.Expressions.Expression.Invoke(
                System.Linq.Expressions.Expression.Constant((Func<object, bool>)OnConfirm),
                System.Linq.Expressions.Expression.Convert(request, typeof(object))),
            request)
            .Compile();
    }

    private bool OnConfirm(object request)
    {
        Confirmations.Add(Confirmation.From(request));
        return HoldConfirmations;
    }
}

/// <summary>
/// One confirmation box E8 the loop asked the frontend for (SPEC-10 REQ-UI-110): its title, its
/// message and the labels of its two buttons, read off the host's internal
/// <c>ConfirmationRequest</c>.
/// </summary>
/// <param name="Title">Title of the box.</param>
/// <param name="Message">The message, exactly as the requirement writes it.</param>
/// <param name="ConfirmLabel">Label of the button that acts and has the focus.</param>
/// <param name="CancelLabel">Label of the button that changes nothing.</param>
/// <param name="Confirmed">What the first button runs.</param>
public sealed record Confirmation(
    string Title,
    string Message,
    string ConfirmLabel,
    string CancelLabel,
    Action Confirmed)
{
    /// <summary>Reads a <c>ConfirmationRequest</c> of <c>Zax.App</c> by reflection.</summary>
    public static Confirmation From(object request)
    {
        ArgumentNullException.ThrowIfNull(request);
        Type type = request.GetType();

        return new Confirmation(
            (string)type.GetProperty("Title")!.GetValue(request)!,
            (string)type.GetProperty("Message")!.GetValue(request)!,
            (string)type.GetProperty("ConfirmLabel")!.GetValue(request)!,
            (string)type.GetProperty("CancelLabel")!.GetValue(request)!,
            (Action)type.GetProperty("Confirmed")!.GetValue(request)!);
    }
}

/// <summary>
/// The running game behind <c>EmulationHost.Session</c>, reached by reflection for the same
/// reason as <see cref="HostDriver"/>. Its core and its Lua runtime are public types, so only
/// the session wrapper itself needs reflecting.
/// </summary>
public sealed class SessionDriver : IDisposable
{
    private static readonly Type SessionType =
        typeof(AppOptions).Assembly.GetType("Zax.App.GameSession", true)!;

    private readonly object _session;

    internal SessionDriver(object session) => _session = session;

    /// <summary>The NES core.</summary>
    public NesCore Core => (NesCore)SessionType.GetProperty("Core")!.GetValue(_session)!;

    /// <summary>The conversion engine.</summary>
    public Zax.Engine3D.ConversionEngine Engine
        => (Zax.Engine3D.ConversionEngine)SessionType.GetProperty("Engine")!.GetValue(_session)!;

    /// <summary>The profile's Lua runtime.</summary>
    public ProfileScriptRuntime Scripts
        => (ProfileScriptRuntime)SessionType.GetProperty("Scripts")!.GetValue(_session)!;

    /// <summary>Emulated frames run since the game started.</summary>
    public int FrameCounter => (int)SessionType.GetProperty("FrameCounter")!.GetValue(_session)!;

    /// <summary>Whether a ROM is loaded in the core.</summary>
    public bool HasRom => (bool)SessionType.GetProperty("HasRom")!.GetValue(_session)!;

    /// <summary>Script names in the hook order of REQ-LUA-004.</summary>
    public IReadOnlyList<string> ScriptNames
        => (IReadOnlyList<string>)SessionType.GetProperty("ScriptNames")!.GetValue(_session)!;

    /// <summary>Runs exactly one emulated frame with these inputs (REQ-LOOP-062).</summary>
    public void RunFrame(int playerOne = 0, int playerTwo = 0)
    {
        MethodInfo run = SessionType.GetMethods().First(m => m.Name == "RunFrame");
        run.Invoke(_session, [playerOne, playerTwo, ZapperState.ReleasedByte]);
    }

    /// <summary>The Lua block of a savestate: <c>Save</c> of every active script, then PData.</summary>
    public byte[] CaptureScriptState()
        => (byte[])SessionType.GetMethod("CaptureScriptState")!.Invoke(_session, [])!;

    /// <summary>The <c>retro_unserialize</c> path of REQ-LUA-075: PData then <c>Load</c>.</summary>
    public void RestoreScriptStateInPlace(byte[] block)
        => SessionType.GetMethod("RestoreScriptStateInPlace")!.Invoke(_session, [block]);

    /// <summary>
    /// Builds a session on <paramref name="profile"/> exactly as the launch flow does, with no
    /// ROM yet: that is the production load order of REQ-LUA-015.
    /// </summary>
    public static SessionDriver CreateOnProfile(GameProfile profile, SceneState scene, IScriptLog? log = null)
    {
        MethodInfo create = SessionType.GetMethod("Create", BindingFlags.Public | BindingFlags.Static)!;

        // The last argument is the 2D override of AMD-024 REQ-PUB-001 (Q-065); null keeps the
        // rule of REQ-SCN-055, which is what a profile load does.
        object session = create.Invoke(
            null,
            [profile, "test", scene, log, new SettingsModel(), true, null])!;
        return new SessionDriver(session);
    }

    /// <summary>Applies the profile's scene settings and calls <c>Start</c> (REQ-LUA-015 step 4).</summary>
    public void ApplyProfileSettings(SceneState scene, ProfileSettings? settings)
        => SessionType.GetMethod("ApplyProfileSettings")!.Invoke(_session, [scene, settings]);

    /// <summary>Calls <c>Start</c> of every script that is not in error.</summary>
    public void Start() => SessionType.GetMethod("Start")!.Invoke(_session, []);

    /// <summary>Calls <c>End</c> and puts the end-of-game state back.</summary>
    public void End() => SessionType.GetMethod("End")!.Invoke(_session, []);

    /// <inheritdoc/>
    public void Dispose() => (_session as IDisposable)?.Dispose();
}

/// <summary>A fact skipped, not failed, when <c>assets-neutral</c> is missing.</summary>
public sealed class HostFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public HostFactAttribute()
    {
        if (!ProfileCorpus.Available)
        {
            Skip = "assets-neutral is not available next to the solution.";
        }
    }
}

/// <summary>A theory skipped, not failed, when <c>assets-neutral</c> is missing.</summary>
public sealed class HostTheoryAttribute : TheoryAttribute
{
    /// <summary>Creates the attribute.</summary>
    public HostTheoryAttribute()
    {
        if (!ProfileCorpus.Available)
        {
            Skip = "assets-neutral is not available next to the solution.";
        }
    }
}

/// <summary>Formats a double the invariant way, for test messages.</summary>
internal static class Fmt
{
    public static string N(double value) => value.ToString("0.####", CultureInfo.InvariantCulture);
}
