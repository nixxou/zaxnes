using System.Numerics;
using Zax.Engine3D;
using Zax.Render;
using Zax.Scripting;

namespace Zax.App;

/// <summary>
/// The camera of SPEC-06 seen through the <c>Camera</c> object of SPEC-05 REQ-LUA-088: the
/// setpoints of the current mode, so that a bound vector a script keeps keeps steering the
/// real camera (REQ-LUA-044).
/// </summary>
internal sealed class SceneCameraBridge(SceneCamera camera) : ScriptCamera
{
    /// <inheritdoc/>
    public override Vec3 Angles
    {
        get
        {
            Vector3 angles = camera.Current.Angles;
            return new Vec3(angles.X, angles.Y, angles.Z);
        }

        set => camera.Current.Angles = new Vector3(value.X, value.Y, value.Z);
    }

    /// <inheritdoc/>
    public override float Distance
    {
        // The setpoint bounds the distance to [0, 512] itself (REQ-SCN-011).
        get => camera.Current.Distance;
        set => camera.Current.Distance = value;
    }

    /// <inheritdoc/>
    public override int Mode
    {
        get => (int)camera.Mode;
        set => camera.SetMode(value == 1 ? CameraMode.Fpv : CameraMode.Tpv);
    }

    /// <inheritdoc/>
    public override Vec3 LookAt
    {
        get
        {
            Vector3 target = camera.Current.Target;
            return new Vec3(target.X, target.Y, target.Z);
        }

        set => camera.Current.Target = new Vector3(value.X, value.Y, value.Z);
    }

    /// <inheritdoc/>
    public override void ResetView() => camera.ResetView();
}

/// <summary>
/// The backdrop of SPEC-06 REQ-SCN-048 seen through the <c>Backdrop</c> object of SPEC-05
/// REQ-LUA-090.
/// </summary>
internal sealed class SceneBackdropBridge(BackdropState backdrop) : ScriptBackdrop
{
    /// <inheritdoc/>
    public override bool Enabled
    {
        get => backdrop.Enabled;
        set => backdrop.Enabled = value;
    }

    /// <inheritdoc/>
    public override LuaColor Color
    {
        get => LuaColor.FromBytes(backdrop.Color.R, backdrop.Color.G, backdrop.Color.B);
        set => backdrop.Color = value.ToRgb24();
    }

    /// <inheritdoc/>
    public override bool ReceiveShadow
    {
        get => backdrop.ReceiveShadow;
        set => backdrop.ReceiveShadow = value;
    }

    /// <inheritdoc/>
    public override Vec3 Size
    {
        get => new(backdrop.Size.X, backdrop.Size.Y, backdrop.Size.Z);
        set => backdrop.Size = new Vector3(value.X, value.Y, value.Z);
    }

    /// <inheritdoc/>
    public override Vec3 Offset
    {
        get => new(backdrop.Offset.X, backdrop.Offset.Y, backdrop.Offset.Z);
        set => backdrop.Offset = new Vector3(value.X, value.Y, value.Z);
    }

    /// <inheritdoc/>
    public override void Reset() => backdrop.Reset();
}

/// <summary>Writes the <c>print</c> output and the script errors to the console.</summary>
internal sealed class ConsoleScriptLog : IScriptLog
{
    /// <inheritdoc/>
    public void Print(string line) => Console.Out.WriteLine($"Lua: {line}");

    /// <inheritdoc/>
    public void Error(string line) => Console.Error.WriteLine(line);

    /// <inheritdoc/>
    public void Warning(string line) => Console.Out.WriteLine(line);
}

/// <summary>
/// Applies to the scene the settings a script writes (DEC-06-Q5, DEC-09-Q3): the session layer
/// of <see cref="ScriptSettings"/> holds the value and this pushes the ones the renderer owns.
/// </summary>
internal static class SceneSettingBridge
{
    /// <summary>Connects <paramref name="settings"/> to <paramref name="scene"/>.</summary>
    public static void Connect(ScriptSettings settings, SceneState scene)
    {
        ArgumentNullException.ThrowIfNull(settings);
        ArgumentNullException.ThrowIfNull(scene);

        settings.ViewReset = scene.Camera.ResetView;
        settings.Changed = key => Apply(key, settings, scene);
        Push(settings, scene);
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

    private static void PushLight(ScriptSettings settings, SceneState scene)
    {
        Vec3 direction = settings.Light.Direction;
        scene.Light.Direction = new Vector3(direction.X, direction.Y, direction.Z);
        scene.Light.Color = settings.Light.Color.ToRgb24();
        scene.Settings.LightIntensity = settings.Light.Intensity;
    }

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

                // The clip margins frame the camera and cut the tiles (REQ-E3D-410), so a Lua
                // write has to reach the scene at the next frame (REQ-PERS-007).
                scene.Clipping = settings.ClippingVector;
                break;
            default:
                // Every other setting is held in the session layer and read from there.
                break;
        }
    }
}
