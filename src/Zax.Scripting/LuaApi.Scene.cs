using KeraLua;
using Zax.Engine3D;

namespace Zax.Scripting;

/// <summary>
/// The globals <c>Camera</c>, <c>Backdrop</c>, <c>Light</c>, <c>Video</c>, <c>Profile</c> and
/// <c>NotiCenter</c> of SPEC-05 §4.2.
/// </summary>
internal static partial class LuaApi
{
    private static void RegisterCamera(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        HostType type = binder.DeclareType(HostTypeId.CameraApi, "CameraAPI");
        ScriptCamera camera = runtime.Environment.Camera;

        // The orientation of the current mode, as a bound vector a script may keep
        // (REQ-LUA-044, "Antarctic Adventure").
        type.Property(
            "Rot",
            (b, l) =>
            {
                b.PushDescriptor(l, HostTypeId.Vec3Ref, camera, (int)Vec3RefKind.CameraRotation);
                return 1;
            },
            result: HostTypeId.Vec3Ref);

        type.Property(
            "Target",
            (b, l) =>
            {
                b.PushDescriptor(l, HostTypeId.Vec3Ref, camera, (int)Vec3RefKind.CameraTarget);
                return 1;
            },
            result: HostTypeId.Vec3Ref);

        type.Property(
            "Distance",
            (_, l) => Number(l, camera.Distance),
            (b, l) => camera.Distance = (float)AssignedNumber(b, l, "Distance"));

        type.Property(
            "Mode",
            (_, l) => Number(l, camera.Mode),
            (b, l) => camera.Mode = Math.Clamp(AssignedInteger(b, l, "Mode"), 0, 1));

        type.Property(
            "LookAt",
            (b, l) =>
            {
                b.PushVector3(l, camera.LookAt);
                return 1;
            },
            (b, l) => camera.LookAt = b.CheckVector3(l, 3, "LookAt"),
            HostTypeId.Vector3);

        type.Property(
            "FpvShape",
            (b, l) =>
            {
                b.PushObject(l, HostTypeId.Shape, runtime.FpvShape);
                return 1;
            },
            (b, l) =>
            {
                if (l.IsNil(3))
                {
                    runtime.FpvShape = null;
                    return;
                }

                if (b.ObjectAt(l, 3, HostTypeId.Shape) is FrameShape shape)
                {
                    runtime.FpvShape = shape;
                    runtime.ApplyCameraTracking();
                    return;
                }

                b.InvalidArguments(l, "FpvShape");
            },
            HostTypeId.Shape);

        type.Property("IsFPV", (_, l) => Boolean(l, camera.Mode == 1));
        type.Property("IsTPV", (_, l) => Boolean(l, camera.Mode == 0));

        type.Function("SetTarget", (b, l) =>
        {
            Require(b, l, "SetTarget");
            if (b.TryGetVector3(l, 2, out Vec3 target))
            {
                camera.LookAt = target;
                return 0;
            }

            float x = (float)b.CheckNumber(l, 2, "SetTarget");
            float y = (float)b.CheckNumber(l, 3, "SetTarget");
            float z = (float)b.CheckNumber(l, 4, "SetTarget");
            camera.LookAt = new Vec3(x, y, z);
            return 0;
        });

        type.Function("GetTarget", (b, l) =>
        {
            Require(b, l, "GetTarget");
            b.PushVector3(l, camera.LookAt);
            return 1;
        }, HostTypeId.Vector3);

        type.Function("SetFPV", (b, l) =>
        {
            Require(b, l, "SetFPV");
            camera.Mode = 1;
            if (l.IsNoneOrNil(2))
            {
                return 0;
            }

            if (b.ObjectAt(l, 2, HostTypeId.Shape) is FrameShape shape)
            {
                runtime.FpvShape = shape;
                runtime.ApplyCameraTracking();
                return 0;
            }

            return b.InvalidArguments(l, "SetFPV");
        });

        type.Function("SetTPV", (b, l) =>
        {
            Require(b, l, "SetTPV");
            camera.Mode = 0;
            return 0;
        });

        type.Function("ToggleMode", (b, l) =>
        {
            Require(b, l, "ToggleMode");
            camera.Mode = camera.Mode == 0 ? 1 : 0;
            return 0;
        });

        type.Function("ResetView", (b, l) =>
        {
            Require(b, l, "ResetView");
            camera.ResetView();
            return 0;
        });
        return;

        static void Require(LuaBinder binder, Lua state, string member)
        {
            if (binder.TypeIdAt(state, 1) != HostTypeId.CameraApi)
            {
                binder.InvalidArguments(state, member);
            }
        }
    }

    private static void RegisterScene(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        ScriptEnvironment environment = runtime.Environment;

        // Backdrop (REQ-LUA-090).
        HostType backdrop = binder.DeclareType(HostTypeId.BackdropApi, "BackdropAPI");
        ScriptBackdrop panel = environment.Backdrop;
        backdrop.Property(
            "Enable",
            (_, l) => Boolean(l, panel.Enabled),
            (_, l) => panel.Enabled = AssignedBoolean(l));
        backdrop.Property(
            "Color",
            (b, l) =>
            {
                b.PushColor(l, panel.Color);
                return 1;
            },
            (b, l) =>
            {
                if (!b.TryGetColor(l, 3, out LuaColor colour))
                {
                    b.InvalidArguments(l, "Color");
                    return;
                }

                panel.Color = colour;
            },
            HostTypeId.Color);
        backdrop.Property(
            "RecvShadow",
            (_, l) => Boolean(l, panel.ReceiveShadow),
            (_, l) => panel.ReceiveShadow = AssignedBoolean(l));
        Axis(backdrop, "SizeX", 0, () => panel.Size, v => panel.Size = v);
        Axis(backdrop, "SizeY", 1, () => panel.Size, v => panel.Size = v);
        Axis(backdrop, "SizeZ", 2, () => panel.Size, v => panel.Size = v);
        Axis(backdrop, "OfsX", 0, () => panel.Offset, v => panel.Offset = v);
        Axis(backdrop, "OfsY", 1, () => panel.Offset, v => panel.Offset = v);
        Axis(backdrop, "OfsZ", 2, () => panel.Offset, v => panel.Offset = v);
        backdrop.Property(
            "Size",
            (b, l) =>
            {
                b.PushDescriptor(l, HostTypeId.Vec3Ref, panel, (int)Vec3RefKind.BackdropSize);
                return 1;
            },
            result: HostTypeId.Vec3Ref);
        backdrop.Property(
            "Offset",
            (b, l) =>
            {
                b.PushDescriptor(l, HostTypeId.Vec3Ref, panel, (int)Vec3RefKind.BackdropOffset);
                return 1;
            },
            result: HostTypeId.Vec3Ref);

        // Light (type Lumière, SPEC-09 §4.1.3).
        HostType light = binder.DeclareType(HostTypeId.Light, "Lumiere");
        ScriptLight lamp = runtime.Settings.Light;
        light.Property(
            "Direction",
            (b, l) =>
            {
                b.PushVector3(l, lamp.Direction);
                return 1;
            },
            (b, l) => lamp.Direction = b.CheckVector3(l, 3, "Direction"),
            HostTypeId.Vector3);
        light.Property(
            "Color",
            (b, l) =>
            {
                b.PushColor(l, lamp.Color);
                return 1;
            },
            (b, l) =>
            {
                if (!b.TryGetColor(l, 3, out LuaColor colour))
                {
                    b.InvalidArguments(l, "Color");
                    return;
                }

                lamp.Color = colour;
            },
            HostTypeId.Color);
        light.Property(
            "Intensity",
            (_, l) => Number(l, lamp.Intensity),
            (b, l) => lamp.Intensity = (float)AssignedNumber(b, l, "Intensity"));

        // Video (REQ-LUA-095): SceneScale reads 1 and a write triggers the view reset.
        HostType video = binder.DeclareType(HostTypeId.VideoApi, "VideoAPI");
        video.Property(
            "SceneScale",
            (_, l) => Number(l, runtime.Settings.SceneScale),
            (b, l) => runtime.Settings.SceneScale = (float)AssignedNumber(b, l, "SceneScale"));

        // Profile metadata (REQ-LUA-095): stored for the session, no effect on the library.
        HostType profile = binder.DeclareType(HostTypeId.ProfileApi, "ProfilAPI");
        ScriptProfileInfo info = environment.Profile;
        profile.Property(
            "Title",
            (_, l) => Text(l, info.Title),
            (b, l) => info.Title = b.CheckString(l, 3, "Title"));
        profile.Property(
            "Maker",
            (_, l) => Text(l, info.Maker),
            (b, l) => info.Maker = b.CheckString(l, 3, "Maker"));
        profile.Property(
            "GameDev",
            (_, l) => Text(l, info.GameDev),
            (b, l) => info.GameDev = b.CheckString(l, 3, "GameDev"));
        profile.Property(
            "Des",
            (_, l) => Text(l, info.Description),
            (b, l) => info.Description = b.CheckString(l, 3, "Des"));
        profile.Property("Id", (_, l) => Text(l, info.Id));
        profile.Property("Ver", (_, l) => Text(l, info.Version));
        profile.Property("IsV2Format", (_, l) => Boolean(l, info.IsV2Format));
        profile.Function("RenewId", (b, l) =>
        {
            if (b.TypeIdAt(l, 1) != HostTypeId.ProfileApi)
            {
                return b.InvalidArguments(l, "RenewId");
            }

            info.RenewId();
            return 0;
        });

        // Notifications (REQ-LUA-093): table functions, called with '.'.
        HostType noti = binder.DeclareType(HostTypeId.NotiApi, "NotiAPI");
        noti.Function("Notify", (b, l) =>
        {
            string message = b.CheckString(l, 1, "Notify");
            float seconds = (float)b.OptionalNumber(l, 2, "Notify", 4.0);
            int severity = (int)b.OptionalNumber(l, 3, "Notify", 0.0);
            Raise(message, seconds, severity);
            return 0;
        });
        noti.Function("Show", (b, l) =>
        {
            string message = b.CheckString(l, 1, "Show");
            int severity = (int)b.OptionalNumber(l, 2, "Show", 0.0);
            Raise(message, 4f, severity);
            return 0;
        });
        return;

        void Raise(string message, float seconds, int severity)
            => environment.Notify?.Invoke(new ScriptNotification(
                message,
                seconds,
                (ScriptNotificationSeverity)Math.Clamp(severity, 0, 2)));

        void Axis(HostType host, string name, int axis, Func<Vec3> read, Action<Vec3> write)
            => host.Property(
                name,
                (_, l) =>
                {
                    Vec3 value = read();
                    return Number(l, axis switch { 0 => value.X, 1 => value.Y, _ => value.Z });
                },
                (b, l) =>
                {
                    Vec3 value = read();
                    float component = (float)AssignedNumber(b, l, name);
                    switch (axis)
                    {
                        case 0: value.X = component; break;
                        case 1: value.Y = component; break;
                        default: value.Z = component; break;
                    }

                    write(value);
                });
    }
}
