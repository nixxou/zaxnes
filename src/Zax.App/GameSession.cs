using Zax.App.Input;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Nes.Rom;
using Zax.Profiles;
using Zax.Render;
using Zax.Scripting;
using Zax.Settings;

namespace Zax.App;

/// <summary>
/// One running game: the NES core, the 2D→3D conversion, the geometry, the Lua runtime of the
/// profile and the scene settings the profile carries.
/// </summary>
/// <remarks>
/// <para>
/// One emulated frame is the sequence of SPEC-07 REQ-LOOP-060: read the inputs, call
/// <c>InputProcess</c> if the script pad is on, run the core, convert the produced frame — which
/// runs the frame hooks of SPEC-05 REQ-LUA-025 — and publish it. Nothing here has a clock of its
/// own (REQ-ARCH-010); the pacing belongs to <see cref="FramePacer"/>.
/// </para>
/// <para>
/// Loading follows REQ-LUA-015: the chunks run first, then the profile's settings are applied
/// and the patterns are built with their <c>BuildPattern</c> hooks, then <c>Start</c>.
/// </para>
/// </remarks>
internal sealed class GameSession : IDisposable
{
    private readonly CoreScriptInput _input;

    /// <summary>
    /// The state of the real frame of REQ-LOOP-065 step 2. One buffer for the whole session: it
    /// is rewritten at every real frame and read back at the end of the same frame, so run-ahead
    /// allocates nothing once the buffer has grown to its size (REQ-CORE-121).
    /// </summary>
    private readonly Zax.Nes.State.CoreState _realFrameState = new();

    private bool _started;

    private GameSession(
        NesCore core,
        ConversionEngine engine,
        GeometryBuilder geometry,
        ProfileScriptRuntime runtime,
        CoreScriptInput input,
        string title)
    {
        Core = core;
        Engine = engine;
        Geometry = geometry;
        Scripts = runtime;
        _input = input;
        Title = title;
    }

    /// <summary>The NES core.</summary>
    public NesCore Core { get; }

    /// <summary>The conversion engine.</summary>
    public ConversionEngine Engine { get; }

    /// <summary>The geometry builder of SPEC-04B.</summary>
    public GeometryBuilder Geometry { get; }

    /// <summary>The Lua runtime of the profile (SPEC-05).</summary>
    public ProfileScriptRuntime Scripts { get; }

    /// <summary>The title of the loaded profile, for the window and the overlay.</summary>
    public string Title { get; }

    /// <summary>
    /// Displayed frames run since the game started (REQ-E3D-043): one per call to
    /// <see cref="RunFrame"/>, so it counts <b>real</b> frames whatever
    /// <see cref="RunAheadFrames"/> says.
    /// </summary>
    /// <remarks>
    /// This is the counter the conversion stamps its frames with and the one the Lua global
    /// <c>frame</c> reads, and it is the displayed frame's number: the sequence of displayed
    /// frames is contiguous, which is what REQ-LOOP-066 relies on to call a savestate of the
    /// camera and of the learned conversion state coherent. The core's own counter
    /// (REQ-CORE-006) follows the real frame and is <see cref="NesCore.FrameCount"/>.
    /// </remarks>
    public int FrameCounter { get; private set; }

    /// <summary>
    /// Run-ahead frames N, 0 to 3 (AMD-041 REQ-LOOP-064). 0 — the value a freshly created
    /// session carries — runs the frame of REQ-LOOP-060 unchanged.
    /// </summary>
    /// <remarks>
    /// The setting <c>emu.run_ahead</c> belongs to the desktop loop, which writes this property
    /// before every real frame: that is how « applied at the next real frame » is honoured, and
    /// why the value is a property of the session rather than an argument of
    /// <see cref="RunFrame"/>. A host that drives the session itself gets no run-ahead unless it
    /// asks for it, which is what AMD-041 §4 requires of the libretro frontend, where the value
    /// is 0 and the host provides run-ahead from the serialisation instead.
    /// </remarks>
    public int RunAheadFrames
    {
        get => field;
        set => field = Zax.Settings.SettingsCatalogue.ClampRunAhead(value);
    }

    /// <summary>Names of the profile's Lua scripts, in the hook order of REQ-LUA-004.</summary>
    public IReadOnlyList<string> ScriptNames =>
        [.. Scripts.Scripts.Select(static script => script.Name)];

    /// <summary>Nominal emulated frame rate of the active region (DEC-R2-T3).</summary>
    public int NominalFrameRate => Core.Timing.NominalFramesPerSecond;

    /// <summary>
    /// Loads a ROM and a profile. Throws <see cref="InvalidOperationException"/> with the
    /// message of SPEC-07 REQ-LOOP-023 when the core refuses the ROM.
    /// </summary>
    /// <param name="romPath">Path of the <c>.nes</c> image.</param>
    /// <param name="profile">The profile; the empty profile plays "2D Mode" (REQ-SCN-055).</param>
    /// <param name="title">Title shown for the game.</param>
    /// <param name="scene">The scene the scripts drive (camera, light, backdrop, settings).</param>
    /// <param name="log">Where <c>print</c> and the script errors go.</param>
    /// <param name="automaticRegion">Whether the detected region replaces the setting.</param>
    public static GameSession Load(
        string romPath,
        GameProfile profile,
        string title,
        SceneState scene,
        IScriptLog? log = null,
        bool automaticRegion = true)
    {
        ArgumentNullException.ThrowIfNull(romPath);

        GameSession session = Create(profile, title, scene, log);
        RomLoadResult result = session.LoadRom(File.ReadAllBytes(romPath), Path.GetFileName(romPath));
        if (!result.Success)
        {
            session.Dispose();
            throw new InvalidOperationException($"Invalid rom file. ({result.Error})");
        }

        if (automaticRegion)
        {
            session.Core.Region = result.Region;
        }

        session.Core.Reset();
        return session;
    }

    /// <summary>
    /// Builds a session around a profile, with <b>no ROM</b> in the core yet: the launch flow of
    /// SPEC-07 REQ-LOOP-021 hands the ROM over at step 4, and a savestate load never reads a
    /// ROM at all because the core block is self-contained (SPEC-09 REQ-PERS-053).
    /// </summary>
    /// <param name="profile">The profile; the empty profile plays "2D Mode" (REQ-SCN-055).</param>
    /// <param name="title">Title shown for the game.</param>
    /// <param name="scene">The scene the scripts drive (camera, light, backdrop, settings).</param>
    /// <param name="log">Where <c>print</c> and the script errors go.</param>
    /// <param name="settingsModel">
    /// The settings of the emulator instance (SPEC-09); one of its own when the caller has none.
    /// </param>
    /// <param name="differentGame">
    /// Whether the profile's data is being (re)loaded: a game different from the previous one,
    /// or "2D Mode", which always reloads. True replaces every P setting with the profile's
    /// value (REQ-PERS-005); false keeps what the previous session of this same game left
    /// behind (REQ-PERS-006).
    /// </param>
    /// <param name="twoDimensionalMode">
    /// Overrides the 2D decision. Null — the default — keeps the rule of REQ-SCN-055: a profile
    /// with no pattern at all is the 2D game. False is what the built-in « 3D par défaut (sans
    /// profil) » entry of AMD-024 REQ-PUB-001 passes: the same empty profile, converted in 3D
    /// with the default geometry of SPEC-04 (Q-065).
    /// </param>
    // Q-065
    public static GameSession Create(
        GameProfile profile,
        string title,
        SceneState scene,
        IScriptLog? log = null,
        SettingsModel? settingsModel = null,
        bool differentGame = true,
        bool? twoDimensionalMode = null)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ArgumentNullException.ThrowIfNull(scene);

        NesCore core = new();
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        engine.TwoDimensionalMode = twoDimensionalMode
            ?? (profile.PermanentPatterns.Count == 0 && profile.TemporaryPatterns.Count == 0);

        CoreScriptInput input = new(core);
        SettingsModel model = settingsModel ?? new SettingsModel();
        ScriptSettings settings = new(model, engine.Settings);
        SceneSettingBridge.Connect(settings, scene);

        ScriptEnvironment environment = new()
        {
            Engine = engine,
            Settings = settings,
            Camera = new SceneCameraBridge(scene.Camera),
            Backdrop = new SceneBackdropBridge(scene.Backdrop),
            Input = input,
            Memory = new CoreScriptMemory(core),
            Log = log ?? NullScriptLog.Instance,
            Profile = new ScriptProfileInfo
            {
                Title = profile.Title ?? title,
                Maker = profile.Author ?? string.Empty,
                GameDev = profile.Publisher ?? string.Empty,
                Description = profile.Description ?? string.Empty,
                Id = profile.Id ?? string.Empty,
                Version = profile.VersionText ?? string.Empty,
            },
        };

        // REQ-LUA-015 step 1: every chunk runs before anything else.
        ProfileScriptRuntime runtime = ProfileScriptRuntime.Create(profile, environment);
        engine.ScriptHost = runtime;

        // REQ-LUA-015 step 2, REQ-LOOP-021 step 2 and REQ-PERS-005: the profile's P settings land
        // in the model, which mirrors them into the engine slice and into the scene through the
        // bridge above. This runs *after* the chunks, so an assignment made at the main level of a
        // chunk is overwritten, as REQ-LUA-015 demands. Relaunching the same game skips it, so the
        // scripts' changes survive (REQ-PERS-006).
        if (differentGame)
        {
            model.ApplyProfile(
                profile.Settings is { } embedded
                    ? ProfileSettingApplication.ToSettingValues(embedded)
                    : []);
        }

        // Both ports go back to Gamepad at every load of a different game (REQ-PERS-012,
        // DEC-09-Q6); the input layer stays the authority and takes its types from the model.
        input.SetPortTypes(
            model.GetInt(SettingKeys.InputPortType(0)),
            model.GetInt(SettingKeys.InputPortType(1)));

        // Step 3: the patterns with their BuildPattern hooks. The settings were applied above,
        // through the model, so the engine must not copy them a second time.
        engine.LoadProfile(profile, applySettings: false);

        GameSession session = new(core, engine, geometry, runtime, input, title);
        core.VideoCallback = session.OnVideoFrame;
        return session;
    }

    /// <summary>
    /// Hands a ROM image to the core and cold-starts it (REQ-LOOP-021 step 4). The caller
    /// decides what to do with a failure: SPEC-07 REQ-LOOP-023 shows
    /// <c>Invalid rom file.</c>, and AMD-002 REQ-LOOP-018 requires the file to be validated
    /// before the running game is interrupted.
    /// </summary>
    public RomLoadResult LoadRom(ReadOnlySpan<byte> image, string name)
    {
        RomLoadResult result = Core.LoadRom(image, name);
        if (result.Success)
        {
            FrameCounter = 0;
        }

        return result;
    }

    /// <summary>
    /// Whether a ROM is loaded in the core; false between <see cref="Create"/> and
    /// <see cref="LoadRom"/> or after a failed restore.
    /// </summary>
    public bool HasRom => Core.IsRunning;

    /// <summary>
    /// Puts the frame counter back to 0, which a cold start and a savestate restore both do
    /// (DEC-05-Q2).
    /// </summary>
    public void ResetFrameCounter() => FrameCounter = 0;

    /// <summary>
    /// Restores the core from a savestate block. The scene's frames are dropped and the
    /// pending audio with them, so nothing of the previous session is heard (REQ-AUD-010).
    /// </summary>
    /// <returns>False with <paramref name="error"/> set when the block is refused.</returns>
    public bool RestoreCore(ReadOnlyMemory<byte> block, out string? error)
    {
        bool restored = Core.TryLoadState(block, 0, out _, out _, out error);
        if (restored)
        {
            Engine.ClearFrames();
        }

        return restored;
    }

    /// <summary>
    /// Applies the scene settings the profile carries to <paramref name="scene"/>: clip margins,
    /// light direction and colour, and the NES background flag forced to true
    /// (REQ-SCN-003, REQ-SCN-026, REQ-SCN-046), then calls <c>Start</c> of every script
    /// (REQ-LUA-015 step 4, REQ-LUA-020).
    /// </summary>
    public void ApplyProfileSettings(SceneState scene, ProfileSettings? settings)
    {
        ApplyProfileScene(scene, settings);
        Start();
    }

    /// <summary>
    /// The scene half of <see cref="ApplyProfileSettings"/>, <b>without</b> the <c>Start</c>
    /// hook: a savestate load applies the profile's scene settings, then restores the core and
    /// the camera, and only then runs <c>Start</c> followed by <c>Load</c>
    /// (REQ-LUA-072, DEC-R2-T6).
    /// </summary>
    public void ApplyProfileScene(SceneState scene, ProfileSettings? settings)
    {
        ArgumentNullException.ThrowIfNull(scene);

        scene.Light.Reset();
        scene.Backdrop.Reset();

        // The profile's P settings were applied to the model by Create, before the ROM was read
        // and the region detected: the order Q-031 settles. Pushing them into the scene here
        // covers the clipping, the light and the local camera (REQ-SCN-009).
        _ = settings;
        SceneSettingBridge.Push(Scripts.Settings, scene);
    }

    /// <summary>
    /// Calls <c>Start</c> of every script that is not in error, once per game start
    /// (REQ-LUA-020). Calling it twice without an <see cref="End"/> has no effect.
    /// </summary>
    public void Start()
    {
        if (_started)
        {
            return;
        }

        _started = true;
        Scripts.InvokeStart();
    }

    /// <summary>
    /// Calls <c>End</c> of every script that is not in error and puts the end-of-game state
    /// back (REQ-LUA-021, REQ-LUA-090, REQ-PERS-012).
    /// </summary>
    public void End()
    {
        if (!_started)
        {
            return;
        }

        _started = false;
        Scripts.InvokeEnd();
        Scripts.Environment.Backdrop.Reset();
        Scripts.Settings.EndGame();
    }

    /// <summary>
    /// Restarts the same game without reloading the profile (reset, relaunch from the library):
    /// <c>End</c> then <c>Start</c>, the Lua state kept as it is — globals, error flags and
    /// <c>enable</c> survive (REQ-LUA-013, REQ-LUA-062, DEC-05-Q5, SPEC-07 REQ-LOOP-021). The
    /// frame counter goes back to 0 (DEC-05-Q2).
    /// </summary>
    public void Reset()
    {
        End();
        Core.Reset();
        FrameCounter = 0;
        Start();
    }

    /// <summary>
    /// The Lua part of a savestate (REQ-LUA-071): <c>Save</c> of every active script in script
    /// order, then the 64 <c>PData</c> entries as the 256-byte block of §4.6 (REQ-LUA-073).
    /// </summary>
    /// <remarks>
    /// This is also what <c>retro_serialize</c> captures (REQ-LUA-075, DEC-R2-D5). The rollback
    /// of SPEC-07 must <b>not</b> call it: it carries the 64 entries too (AMD-042
    /// REQ-LOOP-069), but reads and writes them directly, without a <c>Save</c> or a
    /// <c>Load</c> hook (REQ-LOOP-070, REQ-LUA-074) — see <see cref="CaptureRollbackData"/>.
    /// </remarks>
    public byte[] CaptureScriptState()
    {
        Scripts.InvokeSave();
        return Scripts.CapturePersistentData();
    }

    /// <summary>
    /// First half of a savestate load (REQ-LUA-072, DEC-R2-T6): <c>End</c> of every script, which
    /// happens <b>before</b> the profile is reloaded and before the core is restored. The caller
    /// restores the core, then calls <see cref="CompleteScriptRestore"/>.
    /// </summary>
    public void BeginScriptRestore() => End();

    /// <summary>
    /// Second half of a savestate load, once the core is back (REQ-LUA-072, DEC-R2-T6):
    /// <c>PData[1..64]</c> from <paramref name="block"/>, then <c>Start</c> of every script that
    /// is not in error, then <c>Load</c> of every active script, both in script order.
    /// </summary>
    /// <param name="block">The 256-byte block <see cref="CaptureScriptState"/> produced.</param>
    public void CompleteScriptRestore(ReadOnlySpan<byte> block)
    {
        Scripts.RestorePersistentData(block);
        FrameCounter = 0;
        Start();
        Scripts.InvokeLoad();
    }

    /// <summary>
    /// The 64 <c>PData</c> values a rollback snapshot carries (AMD-042 REQ-LOOP-069), read the
    /// way <c>GetPData(i)</c> reads them. The <c>Save</c> hook is <b>not</b> called: it stays
    /// reserved for savestates and for serialisation (REQ-LOOP-070, REQ-LUA-075).
    /// </summary>
    public byte[] CaptureRollbackData() => Scripts.CapturePersistentData();

    /// <summary>
    /// Writes the 64 <c>PData</c> values of a rollback snapshot back, exactly as
    /// <c>SetPData</c> would (AMD-042 REQ-LOOP-070). The <c>Load</c> hook is <b>not</b> called.
    /// </summary>
    public void RestoreRollbackData(ReadOnlySpan<byte> block) => Scripts.RestorePersistentData(block);

    /// <summary>
    /// The soft reset of AMD-044 REQ-CORE-122 and REQ-LOOP-072: the core imitates a press on
    /// the RESET button and the frame counter goes back to 0, but the profile is <b>not</b>
    /// relaunched — no <c>End</c>, no <c>Start</c>, the Lua state and <c>PData</c> untouched.
    /// </summary>
    /// <remarks>
    /// The converted frames in flight belong to before the reset and are dropped, as they are
    /// on a relaunch and on a savestate restore (REQ-E3D-281); the learned conversion state is
    /// kept, which is what distinguishes this from <see cref="Reset"/>.
    /// </remarks>
    public void SoftReset()
    {
        Core.SoftReset();
        FrameCounter = 0;
        Engine.ClearFrames();
    }

    /// <summary>
    /// The <c>retro_unserialize</c> path of REQ-LUA-075 and DEC-R2-D5: the profile stays loaded,
    /// so the 64 entries go back and <c>Load</c> of every active script runs, with no <c>End</c>
    /// and no <c>Start</c>.
    /// </summary>
    public void RestoreScriptStateInPlace(ReadOnlySpan<byte> block)
    {
        Scripts.RestorePersistentData(block);
        Scripts.InvokeLoad();
    }

    /// <summary>
    /// Runs one <b>real</b> frame and publishes the converted displayed frame (REQ-LOOP-060,
    /// REQ-LOOP-065, REQ-E3D-280).
    /// </summary>
    /// <param name="playerOneActions">
    /// The action mask of player 1, every source combined (SPEC-08 REQ-INP-002).
    /// </param>
    /// <param name="playerTwoActions">The action mask of player 2.</param>
    /// <param name="zapperByte">
    /// The byte a port of type <c>Zapper</c> carries (SPEC-08 §4.1): bit 4 set when the trigger
    /// is released, cleared while it is held (DEC-02-Q1).
    /// </param>
    /// <remarks>
    /// <para>
    /// With <see cref="RunAheadFrames"/> = 0 this is the frame of REQ-LOOP-060 as it was: read
    /// the inputs, run the core once, convert, run the frame hooks. With N ≥ 1 it is the sequence
    /// of REQ-LOOP-065 — the real frame silent and invisible, its state kept, N − 1 more silent
    /// frames, then the displayed frame with its image and its sound, then the real state back.
    /// </para>
    /// <para>
    /// The script pad of REQ-INP-071 is asked <b>once</b>, with the frame hooks, so every one of
    /// the N + 1 emulated frames sees the very same inputs E, as REQ-LOOP-065 requires.
    /// </para>
    /// </remarks>
    public void RunFrame(
        int playerOneActions,
        int playerTwoActions = 0,
        byte zapperByte = ZapperState.ReleasedByte)
    {
        // The script pad runs before the core sees the frame and is additive: it adds bits to
        // the player it is attached to and can never mask a physical input (REQ-INP-071, 072).
        int pad = Scripts.InvokeInputProcess(FrameCounter, FrameCounter / 60f);
        (InputAction first, InputAction second) = InputRouting.MergeScriptPad(
            (InputAction)playerOneActions,
            (InputAction)playerTwoActions,
            (InputAction)pad,
            Scripts.ScriptPadIndex);

        _input.SetPlayerStates((int)first, (int)second);

        // REQ-INP-004: a Zapper port carries the Zapper byte and no controller button at all.
        ushort word = InputRouting.CoreWord(
            first,
            second,
            Core.GetPortType(0),
            Core.GetPortType(1),
            zapperByte);

        RunAhead(word);
        Engine.Publish();
        FrameCounter++;
    }

    /// <summary>
    /// The five steps of REQ-LOOP-065 for the input word <paramref name="word"/>, or the single
    /// frame of REQ-LOOP-060 when run-ahead is off.
    /// </summary>
    private void RunAhead(ushort word)
    {
        int ahead = RunAheadFrames;
        if (ahead <= 0)
        {
            Core.RunFrame(word, FrameOutput.All);
            return;
        }

        // REQ-LOOP-068: with a Zapper on a port, every emulated frame renders its pixels and
        // detects the light on its own frame, exactly as it would without run-ahead. With no
        // Zapper the hidden frames skip the image, which costs nothing anywhere else.
        FrameOutput hidden = HasZapperPort ? FrameOutput.Pixels : FrameOutput.None;

        // Step 1: the real frame, no image for the display and no sound.
        Core.RunFrame(word, hidden);

        // Step 2: its complete state, in memory (REQ-CORE-121).
        Core.CaptureState(_realFrameState);

        // Step 3: N − 1 frames in advance, still silent and invisible.
        for (int frame = 1; frame < ahead; frame++)
        {
            Core.RunFrame(word, hidden);
        }

        // Step 4: the displayed frame, with its sound and its image. The video callback converts
        // it and runs the Lua frame hooks, which see this frame's RAM and counter
        // (REQ-LOOP-067) because step 5 has not happened yet.
        Core.RunFrame(word, FrameOutput.All);

        // Step 5: back to the real state, the sound just produced left untouched.
        Core.RestoreState(_realFrameState);
    }

    /// <summary>Whether a port carries a Zapper (REQ-INP-061, REQ-LOOP-068).</summary>
    private bool HasZapperPort
        => Core.GetPortType(0) == ControllerPortType.Zapper
        || Core.GetPortType(1) == ControllerPortType.Zapper;

    /// <summary>The device type of <paramref name="port"/> (0 or 1), REQ-INP-060.</summary>
    public ControllerPortType PortType(int port) => Core.GetPortType(port);

    /// <summary>
    /// Puts both port types back to <c>Controller</c>, which loading a game different from the
    /// previous one does before the profile's <c>Start</c> hook (REQ-INP-060, DEC-09-Q6, D).
    /// </summary>
    public void ResetPortTypes()
    {
        Core.SetPortType(0, ControllerPortType.Controller);
        Core.SetPortType(1, ControllerPortType.Controller);
    }

    /// <summary>
    /// The Zapper aim offset the profile's scripts own, in scene units
    /// (SPEC-05 <c>ZapperOffset</c>, SPEC-08 REQ-INP-063).
    /// </summary>
    public System.Numerics.Vector3 ZapperOffset
    {
        get
        {
            Vec3 offset = _input.ZapperOffset;
            return new System.Numerics.Vector3(offset.X, offset.Y, offset.Z);
        }
    }

    /// <summary>
    /// Connects <c>Setting:Save()</c> and <c>Setting:Load()</c> to the host's settings file
    /// (REQ-PERS-015).
    /// </summary>
    public void ConnectSettingsFile(Action? save, Action? load)
    {
        Scripts.Settings.SaveRequested = save;
        Scripts.Settings.LoadRequested = load;
    }

    /// <summary>
    /// Tells the core where to read the Zapper's aimed position from (REQ-CORE-060,
    /// REQ-INP-065). The core queries it once per frame, for each Zapper port.
    /// </summary>
    public void AttachZapper(ZapperState zapper)
    {
        ArgumentNullException.ThrowIfNull(zapper);
        Core.ZapperPositionSource = _ => zapper.Position;
    }

    /// <summary>The tile list of the published frame (SPEC-04B REQ-E3D-641).</summary>
    public FrameDrawList BuildDrawList() => Engine.BuildDrawList();

    /// <summary>
    /// True when the loaded profile holds no pattern at all, that is when the game runs in
    /// "2D Mode" (REQ-SCN-055).
    /// </summary>
    public bool TwoDimensionalMode => Engine.TwoDimensionalMode;

    /// <summary>
    /// The 256 × 240 pixel frame of the core, which the 2D mode shows on the NES plane
    /// (AMD-009, Q-022).
    /// </summary>
    public ReadOnlySpan<byte> NesPicture => Core.PixelFrame;

    /// <summary>
    /// The opacity mask of the same frame, one bit per pixel (SPEC-02 REQ-CORE-049): the 2D
    /// mode draws nothing where the bit is 0 (AMD-050 REQ-SCN-076).
    /// </summary>
    public ReadOnlySpan<byte> NesOpacityMask => Core.OpacityMask;

    /// <summary>The shape the subjective camera follows, or null (REQ-SCN-021).</summary>
    public FrameShape? FollowedShape => Scripts.FpvShape;

    /// <summary>
    /// The universal NES background colour of the displayed frame, entry 0 of its palette
    /// (REQ-SCN-046, REQ-SCN-052).
    /// </summary>
    public Rgb24 NesBackgroundColor
        => (Engine.PublishedFrame ?? Engine.ConvertingFrame).Palette.Color(0);

    /// <summary>
    /// The frame time of REQ-SCN-043: emulated frames divided by 60, which is what the animated
    /// deformation uses.
    /// </summary>
    public float EmulatedTime => (Engine.PublishedFrame ?? Engine.ConvertingFrame).Time;

    /// <inheritdoc/>
    public void Dispose()
    {
        End();
        Scripts.Dispose();
    }

    private void OnVideoFrame(in FrameBatch batch)
    {
        // The conversion is synchronous, inside the frame, as REQ-LOOP-060 (c) requires.
        Engine.Convert(batch, FrameCounter);

        // The subjective camera follows the shape a script picked, through the tracker of
        // SPEC-04 (REQ-LUA-088, REQ-SCN-021).
        Scripts.AdvanceCameraTracking();
    }
}
