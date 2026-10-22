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
    public static ProfileScriptRuntime Create(
        GameProfile profile,
        ConversionEngine engine,
        NesCore core,
        SceneState scene)
    {
        ScriptSettings settings = new(engine.Settings, new HarnessLight(scene.Light));
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

    /// <summary>Pushes the scene-owned settings a script may have changed (DEC-06-Q5).</summary>
    public static void Apply(ProfileScriptRuntime runtime, SceneState scene)
    {
        ScriptSettings settings = runtime.Settings;
        scene.Settings.Fov = settings.Fov;
        scene.Camera.Fov = settings.Fov;
        scene.Camera.SmoothTime = settings.SmoothTime;
        scene.Camera.LocalCamera = settings.LocalCam;
        scene.Settings.Shadows = settings.Shadow;
        scene.Settings.Skybox = (SkyboxKind)settings.Skybox;
        scene.Settings.TvBox = (TvBoxKind)settings.TVBox;
        scene.Settings.CustomBackgroundColor = settings.CustomBgColor.ToRgb24();
        scene.UseNesBackgroundColor = settings.BgColor;
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
    }

    private sealed class HarnessLight(LightState light) : ScriptLight
    {
        private float _intensity = 1f;

        public override Vec3 Direction
        {
            get => new(light.Direction.X, light.Direction.Y, light.Direction.Z);
            set => light.Direction = new System.Numerics.Vector3(value.X, value.Y, value.Z);
        }

        public override LuaColor Color
        {
            get => LuaColor.FromBytes(light.Color.R, light.Color.G, light.Color.B);
            set => light.Color = value.ToRgb24();
        }

        public override float Intensity
        {
            get => _intensity;
            set => _intensity = value;
        }
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
