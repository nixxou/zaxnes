using System.Diagnostics;
using System.Globalization;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Zax.Scripting;

namespace Zax.Harness;

/// <summary>
/// Wires the Lua runtime of a profile (SPEC-05) into the headless harness, so that a ROM
/// capture shows what the profile's scripts actually do, and reports the hook time of
/// REQ-LUA-110.
/// </summary>
/// <remarks>
/// The harness deliberately does not reference Zax.App (SPEC-01 REQ-ARCH-002), so the few
/// views the runtime needs on the scene are built here rather than reused from the host.
/// </remarks>
internal static class ScriptBridge
{
    /// <summary>
    /// Creates the runtime of <paramref name="profile"/>, with its camera, light, backdrop,
    /// settings, memory and input bound to <paramref name="scene"/> and
    /// <paramref name="core"/>. The chunks run here (REQ-LUA-015 step 1).
    /// </summary>
    /// <param name="profile">The profile whose scripts run.</param>
    /// <param name="engine">The conversion engine the scripts drive.</param>
    /// <param name="core">The core the memory and input objects read.</param>
    /// <param name="scene">The scene the settings, camera, light and backdrop reach.</param>
    /// <param name="mesh">
    /// The mesh algorithm <c>--mesh</c> asked for. A user setting (REQ-E3D-340), so it is written
    /// through the model like the options window writes it: the mirror into the engine slice is
    /// the only path, and a value written straight into that slice would be put back to the
    /// default by the first mirror.
    /// </param>
    public static ProfileScriptRuntime Create(
        GameProfile profile,
        ConversionEngine engine,
        NesCore core,
        SceneState scene,
        RenderAlgorithm mesh = RenderAlgorithm.Greedy)
    {
        // The light the model backs, which is what the application uses: the direction, the
        // colour and the intensity are settings of SPEC-09 §4.1.3, so a profile's embedded
        // direction reaches the scene by the same single path as every other P setting
        // (REQ-PROF-041, REQ-SCN-025).
        ScriptSettings settings = new(engine.Settings);

        // Before the chunks run, as the host does (Zax.App's SceneSettingBridge.Connect): from
        // here on every write of a scene-owned setting reaches the scene at once, wherever it
        // comes from — the main level of a chunk, Start or a frame hook (REQ-PERS-007).
        Connect(settings, scene);
        settings.RenderMode = (int)mesh;

        ScriptEnvironment environment = new()
        {
            Engine = engine,
            Settings = settings,
            Camera = new HarnessCamera(scene.Camera),
            Backdrop = new HarnessBackdrop(scene.Backdrop),
            Input = new CoreScriptInput(core),
            Memory = new CoreScriptMemory(core),
            Log = ConsoleLog.Instance,
            Profile = new ScriptProfileInfo
            {
                Title = profile.Title ?? string.Empty,
                Maker = profile.Author ?? string.Empty,
                GameDev = profile.Publisher ?? string.Empty,
                Description = profile.Description ?? string.Empty,
                Id = profile.Id ?? string.Empty,
                Version = profile.VersionText ?? string.Empty,
            },
        };

        return ProfileScriptRuntime.Create(profile, environment);
    }

    /// <summary>
    /// Connects <paramref name="settings"/> to <paramref name="scene"/> and pushes everything
    /// once, which is what <c>SceneSettingBridge.Connect</c> does in the application: the
    /// renderer's copy of a setting is refreshed on every real change (REQ-PERS-007,
    /// REQ-PERS-009), so a write made in <c>Start</c> or in a frame hook is not lost.
    /// </summary>
    public static void Connect(ScriptSettings settings, SceneState scene)
    {
        ArgumentNullException.ThrowIfNull(settings);
        ArgumentNullException.ThrowIfNull(scene);

        settings.ViewReset = scene.Camera.ResetView;
        settings.Changed = key => Apply(key, settings, scene);
        Push(settings, scene);
    }

    /// <summary>Pushes the scene-owned settings a script may have changed (DEC-06-Q5).</summary>
    public static void Push(ProfileScriptRuntime runtime, SceneState scene)
    {
        ArgumentNullException.ThrowIfNull(runtime);
        Push(runtime.Settings, scene);
    }

    /// <summary>Pushes every scene-owned setting at once, which a game start needs.</summary>
    public static void Push(ScriptSettings settings, SceneState scene)
    {
        ArgumentNullException.ThrowIfNull(settings);
        ArgumentNullException.ThrowIfNull(scene);

        scene.Settings.Fov = settings.Fov;
        scene.Camera.Fov = settings.Fov;
        scene.Camera.SmoothTime = settings.SmoothTime;
        scene.Camera.LocalCamera = settings.LocalCam;
        scene.Settings.Shadows = settings.Shadow;
        scene.Settings.Skybox = (SkyboxKind)settings.Skybox;
        scene.Settings.CustomBackgroundColor = settings.CustomBgColor.ToRgb24();
        scene.UseNesBackgroundColor = settings.BgColor;
        scene.Clipping = settings.ClippingVector;
        PushLight(settings, scene);
    }

    /// <summary>
    /// The three light values the model holds, pushed into the renderer's working copy, exactly
    /// as <c>SceneSettingBridge.PushLight</c> does in the application (SPEC-09 §4.1.3,
    /// REQ-SCN-025).
    /// </summary>
    private static void PushLight(ScriptSettings settings, SceneState scene)
    {
        Vec3 direction = settings.Light.Direction;
        scene.Light.Direction = new System.Numerics.Vector3(direction.X, direction.Y, direction.Z);
        scene.Light.Color = settings.Light.Color.ToRgb24();
        scene.Settings.LightIntensity = settings.Light.Intensity;
    }

    /// <summary>
    /// The one setting <paramref name="key"/> names, the per-key half of
    /// <see cref="Push(ScriptSettings, SceneState)"/>. The keys are the stable ones of SPEC-09
    /// §4.1 and the switch is the application's.
    /// </summary>
    private static void Apply(string key, ScriptSettings settings, SceneState scene)
    {
        switch (key)
        {
            case "light.direction":
            case "light.color":
            case "light.intensity":

                // The three light values are settings of §4.1.3; the renderer's light is a
                // working copy the model pushes into.
                PushLight(settings, scene);
                break;
            case "camera.fov":
                scene.Settings.Fov = settings.Fov;
                scene.Camera.Fov = settings.Fov;
                break;
            case "camera.smooth_time":
                scene.Camera.SmoothTime = settings.SmoothTime;
                break;

            // REQ-SCN-009: the local camera is a profile setting, so a script may turn it on in
            // Start or later and the capture has to show it.
            case "camera.local_cam":
                scene.Camera.LocalCamera = settings.LocalCam;
                break;
            case "render.shadow":
                scene.Settings.Shadows = settings.Shadow;
                break;
            case "scene.skybox":
                scene.Settings.Skybox = (SkyboxKind)settings.Skybox;
                break;
            case "scene.custom_bg_color":
                scene.Settings.CustomBackgroundColor = settings.CustomBgColor.ToRgb24();
                break;
            case "scene.use_nes_bg_color":
                scene.UseNesBackgroundColor = settings.BgColor;
                break;
            case "clip.left":
            case "clip.bottom":
            case "clip.right":
            case "clip.top":
                scene.Clipping = settings.ClippingVector;
                break;
            default:
                // Every other setting is held in the session layer and read from there.
                break;
        }
    }

    /// <summary>Prints what ran and the hook budget of REQ-LUA-110.</summary>
    public static void Report(ProfileScriptRuntime runtime, TimedScriptHost timed)
    {
        if (runtime.Scripts.Count == 0)
        {
            return;
        }

        string names = string.Join(", ", runtime.Scripts.Select(static script => script.Name));
        Console.Out.WriteLine($"scripts: {runtime.Scripts.Count} in hook order: {names}");

        int failed = runtime.Scripts.Count(static script => script.Failed);
        if (failed > 0)
        {
            Console.Out.WriteLine($"scripts: {failed} went into error during the run");
        }

        if (timed.Frames > 0)
        {
            Console.Out.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"scripts: hooks {timed.Mean:F3} ms mean, {timed.P99:F3} ms p99 over "
                    + $"{timed.Frames} frames (REQ-LUA-110: 2 ms / 6 ms)"));
        }
    }

    /// <summary>Writes <c>print</c> and the script errors to the console.</summary>
    private sealed class ConsoleLog : IScriptLog
    {
        public static ConsoleLog Instance { get; } = new();

        public void Print(string line) => Console.Out.WriteLine($"Lua: {line}");

        public void Error(string line) => Console.Error.WriteLine(line);

        public void Warning(string line) => Console.Out.WriteLine(line);
    }

    private sealed class HarnessCamera(SceneCamera camera) : ScriptCamera
    {
        public override Vec3 Angles
        {
            get
            {
                System.Numerics.Vector3 angles = camera.Current.Angles;
                return new Vec3(angles.X, angles.Y, angles.Z);
            }

            set => camera.Current.Angles = new System.Numerics.Vector3(value.X, value.Y, value.Z);
        }

        public override float Distance
        {
            get => camera.Current.Distance;
            set => camera.Current.Distance = value;
        }

        public override int Mode
        {
            get => (int)camera.Mode;
            set => camera.SetMode(value == 1 ? CameraMode.Fpv : CameraMode.Tpv);
        }

        public override Vec3 LookAt
        {
            get
            {
                System.Numerics.Vector3 target = camera.Current.Target;
                return new Vec3(target.X, target.Y, target.Z);
            }

            set => camera.Current.Target = new System.Numerics.Vector3(value.X, value.Y, value.Z);
        }

        public override void ResetView() => camera.ResetView();

        public override float ResetDistance => camera.ResetDistance;
    }

    private sealed class HarnessBackdrop(BackdropState backdrop) : ScriptBackdrop
    {
        public override bool Enabled
        {
            get => backdrop.Enabled;
            set => backdrop.Enabled = value;
        }

        public override LuaColor Color
        {
            get => LuaColor.FromBytes(backdrop.Color.R, backdrop.Color.G, backdrop.Color.B);
            set => backdrop.Color = value.ToRgb24();
        }

        public override bool ReceiveShadow
        {
            get => backdrop.ReceiveShadow;
            set => backdrop.ReceiveShadow = value;
        }

        public override Vec3 Size
        {
            get => new(backdrop.Size.X, backdrop.Size.Y, backdrop.Size.Z);
            set => backdrop.Size = new System.Numerics.Vector3(value.X, value.Y, value.Z);
        }

        public override Vec3 Offset
        {
            get => new(backdrop.Offset.X, backdrop.Offset.Y, backdrop.Offset.Z);
            set => backdrop.Offset = new System.Numerics.Vector3(value.X, value.Y, value.Z);
        }

        public override void Reset() => backdrop.Reset();
    }
}

/// <summary>
/// Forwards every hook to the real runtime and records how long steps 1 to 5 of REQ-LUA-025
/// take per frame, which is the budget REQ-LUA-110 sets.
/// </summary>
internal sealed class TimedScriptHost(IScriptHost inner) : IScriptHost
{
    private readonly List<double> _perFrame = [];
    private double _current;

    /// <summary>Frames measured.</summary>
    public int Frames => _perFrame.Count;

    /// <summary>Mean hook time of a frame, in milliseconds.</summary>
    public double Mean => _perFrame.Count == 0 ? 0 : _perFrame.Average();

    /// <summary>99th percentile of the hook time of a frame, in milliseconds.</summary>
    public double P99
    {
        get
        {
            if (_perFrame.Count == 0)
            {
                return 0;
            }

            double[] sorted = [.. _perFrame.Order()];
            int index = (int)Math.Ceiling(sorted.Length * 0.99) - 1;
            return sorted[Math.Clamp(index, 0, sorted.Length - 1)];
        }
    }

    /// <inheritdoc/>
    public bool HasFrameHooks => inner.HasFrameHooks;

    /// <inheritdoc/>
    public void SetFrameGlobals(int frameCounter, float time)
    {
        // Step 1 opens the frame, so the accumulator starts here.
        _current = 0;
        Measure(() => inner.SetFrameGlobals(frameCounter, time));
    }

    /// <inheritdoc/>
    public void InvokeUpdate() => Measure(inner.InvokeUpdate);

    /// <inheritdoc/>
    public void InvokeUpdateShapes(IReadOnlyList<FrameInstance> scriptableInstances)
        => Measure(() => inner.InvokeUpdateShapes(scriptableInstances));

    /// <inheritdoc/>
    public void InvokeLateUpdate()
    {
        Measure(inner.InvokeLateUpdate);

        // Step 5 closes the frame.
        _perFrame.Add(_current);
    }

    /// <inheritdoc/>
    public void InvokeBuildPattern(PatternInstance patternInstance)
        => inner.InvokeBuildPattern(patternInstance);

    private void Measure(Action step)
    {
        long before = Stopwatch.GetTimestamp();
        step();
        _current += Stopwatch.GetElapsedTime(before).TotalMilliseconds;
    }
}
