using KeraLua;
using Zax.Engine3D;

namespace Zax.Scripting;

/// <summary>
/// A host object that exists only to carry an identity for one of the API tables of SPEC-05
/// §4.1 (<c>Frame</c>, <c>Palette</c>, <c>Pattern</c>, <c>Script</c>, <c>NotiCenter</c>…).
/// </summary>
/// <remarks>
/// The settings, the light, the camera, the backdrop and the profile metadata are the real host
/// objects instead, which is what makes <c>Light == Setting.Light</c> true (REQ-LUA-089).
/// </remarks>
internal sealed class ApiTable(ProfileScriptRuntime runtime, HostTypeId kind)
{
    /// <summary>The runtime this table belongs to.</summary>
    public ProfileScriptRuntime Runtime { get; } = runtime;

    /// <summary>Which table it is.</summary>
    public HostTypeId Kind { get; } = kind;

    /// <inheritdoc/>
    public override string ToString() => Kind.ToString();
}

/// <summary>
/// Declares the complete script-visible API of SPEC-05 §4 against the binder: every global of
/// §4.1, every host type of §4.2, every bound vector, value type and list of §4.3 and the
/// constant tables of §4.4.
/// </summary>
/// <remarks>
/// The declarations are data: each member is a name, a getter and, when it is writable, a
/// setter. That is what REQ-LUA-051's static check verifies against, and what makes an unknown
/// member read as nil and an unknown assignment raise the exact message of REQ-LUA-042.
/// </remarks>
internal static partial class LuaApi
{
    /// <summary>Declares every host type.</summary>
    public static void Register(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        RegisterInstance(binder, runtime);
        RegisterReferences(binder, runtime);
        RegisterShape(binder, runtime);
        RegisterPattern(binder, runtime);
        RegisterLists(binder, runtime);
        RegisterFrame(binder, runtime);
        RegisterPalette(binder, runtime);
        RegisterPatternTable(binder, runtime);
        RegisterScriptTable(binder, runtime);
        RegisterCamera(binder, runtime);
        RegisterSettings(binder, runtime);
        RegisterScene(binder, runtime);
        RegisterInput(binder, runtime);
        RegisterValues(binder);
    }

    /// <summary>
    /// The host-object globals of §4.1, in the order they are installed. The same table feeds
    /// the API description, so the two can never drift apart (REQ-LUA-051).
    /// </summary>
    internal static readonly (string Name, HostTypeId Type)[] GlobalObjects =
    [
        ("Frame", HostTypeId.FrameApi),
        ("FrameManager", HostTypeId.FrameManagerApi),
        ("Palette", HostTypeId.PaletteApi),
        ("Pattern", HostTypeId.PatternApi),
        ("Script", HostTypeId.ScriptApi),
        ("NotiCenter", HostTypeId.NotiApi),
        ("Video", HostTypeId.VideoApi),
        ("Gamepad", HostTypeId.GamepadApi),
        ("ScriptPad", HostTypeId.ScriptPadApi),
        ("Layer", HostTypeId.LayerConfig),
        ("Clipping", HostTypeId.Clipping),
        ("Setting", HostTypeId.SettingApi),
        ("Light", HostTypeId.Light),
        ("Camera", HostTypeId.CameraApi),
        ("Backdrop", HostTypeId.BackdropApi),
        ("Profile", HostTypeId.ProfileApi),
        ("Vector3", HostTypeId.Vector3Class),
        ("Vector2", HostTypeId.Vector2Class),
        ("Quaternion", HostTypeId.QuaternionClass),
        ("Color", HostTypeId.ColorClass),
        ("IntVector2", HostTypeId.IntVector2Class),
        ("IntVector3", HostTypeId.IntVector3Class),
        ("Rect", HostTypeId.RectClass),
        ("Gamepad1", HostTypeId.Player),
        ("Gamepad2", HostTypeId.Player),
        ("ZapperOffset", HostTypeId.Vec3Ref),

        // Reassigned by the runtime at every hook call (REQ-LUA-025, REQ-LUA-026).
        ("shape", HostTypeId.Instance),
        ("shape2D", HostTypeId.Shape),
        ("pattern", HostTypeId.Instance),
        ("self", HostTypeId.ScriptObj),
    ];

    /// <summary>The constant tables of §4.4 and their entries (REQ-LUA-046).</summary>
    internal static readonly (string Name, (string Key, int Value)[] Entries)[] ConstantTables =
    [
        ("TLayer", [
            ("UNIDENTIFIED", 0), ("UI", 1), ("L1", 2), ("L2", 3), ("L3", 4), ("L4", 5),
            ("COUNT", 6),
        ]),
        ("Align", [("FRONT", 0), ("CENTER", 1), ("BACK", 2), ("COUNT", 3)]),
        ("CamMode", [("TPV", 0), ("FPV", 1)]),
    ];

    /// <summary>Installs the globals of §4.1 that are host objects or constant tables.</summary>
    public static void InstallGlobals(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        Lua state = runtime.State;
        ScriptEnvironment environment = runtime.Environment;

        foreach ((string name, HostTypeId id) in GlobalObjects)
        {
            switch (id)
            {
                case HostTypeId.SettingApi:
                    Push(id, runtime.Settings);
                    break;
                case HostTypeId.Light:
                    Push(id, runtime.Settings.Light);
                    break;
                case HostTypeId.CameraApi:
                    Push(id, environment.Camera);
                    break;
                case HostTypeId.BackdropApi:
                    Push(id, environment.Backdrop);
                    break;
                case HostTypeId.ProfileApi:
                    Push(id, environment.Profile);
                    break;

                // Gamepad1 and Gamepad2 are the two player objects (§4.1).
                case HostTypeId.Player:
                    binder.PushDescriptor(
                        state,
                        HostTypeId.Player,
                        environment.Input,
                        string.Equals(name, "Gamepad2", StringComparison.Ordinal) ? 1 : 0);
                    break;

                // ZapperOffset is a bound vector a script may change component by component.
                case HostTypeId.Vec3Ref:
                    binder.PushDescriptor(
                        state,
                        HostTypeId.Vec3Ref,
                        environment.Input,
                        (int)Vec3RefKind.ZapperOffset);
                    break;

                // shape, shape2D, pattern and self start out nil; the runtime assigns them.
                case HostTypeId.Instance:
                case HostTypeId.Shape:
                case HostTypeId.ScriptObj:
                    state.PushNil();
                    break;

                default:
                    Push(id, Table(runtime, id));
                    break;
            }

            state.SetGlobal(name);
        }

        // The constant values are pushed as floats like every other number the host hands to
        // Lua (REQ-LUA-040, REQ-LUA-046).
        foreach ((string name, (string Key, int Value)[] entries) in ConstantTables)
        {
            state.CreateTable(0, entries.Length);
            foreach ((string key, int value) in entries)
            {
                state.PushString(key);
                state.PushNumber(value);
                state.RawSet(-3);
            }

            state.SetGlobal(name);
        }

        return;

        void Push(HostTypeId id, object value) => binder.PushObject(state, id, value);
    }

    /// <summary>The single API table object of <paramref name="kind"/> for this runtime.</summary>
    internal static ApiTable Table(ProfileScriptRuntime runtime, HostTypeId kind)
        => runtime.TableOf(kind);

    /// <summary>The instance a method or a property was called on, or null.</summary>
    internal static PatternInstance? SelfInstance(LuaBinder binder, Lua state)
        => binder.ObjectAt(state, 1, HostTypeId.Instance) as PatternInstance;

    /// <summary>The frame instance a member was called on, or null for a pattern instance.</summary>
    internal static FrameInstance? SelfFrameInstance(LuaBinder binder, Lua state)
        => binder.ObjectAt(state, 1, HostTypeId.Instance) as FrameInstance;

    /// <summary>The shape a member was called on, or null.</summary>
    internal static FrameShape? SelfShape(LuaBinder binder, Lua state)
        => binder.ObjectAt(state, 1, HostTypeId.Shape) as FrameShape;

    /// <summary>Pushes a number, always as a Lua float (REQ-LUA-040).</summary>
    internal static int Number(Lua state, double value)
    {
        state.PushNumber(value);
        return 1;
    }

    /// <summary>Pushes a boolean.</summary>
    internal static int Boolean(Lua state, bool value)
    {
        state.PushBoolean(value);
        return 1;
    }

    /// <summary>Pushes a string.</summary>
    internal static int Text(Lua state, string value)
    {
        state.PushString(value);
        return 1;
    }

    /// <summary>The value being assigned, read as a number (index 3 of <c>__newindex</c>).</summary>
    internal static double AssignedNumber(LuaBinder binder, Lua state, string member)
        => binder.CheckNumber(state, 3, member);

    /// <summary>The value being assigned, read as an integer with truncation (REQ-LUA-040).</summary>
    internal static int AssignedInteger(LuaBinder binder, Lua state, string member)
        => LuaNumberFormat.ToInt32(binder.CheckNumber(state, 3, member));

    /// <summary>The value being assigned, read as a truth value.</summary>
    internal static bool AssignedBoolean(Lua state) => state.ToBoolean(3);
}
