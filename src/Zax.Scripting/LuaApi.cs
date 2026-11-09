using KeraLua;
using Zax.Engine3D;
using Zax.Settings;

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

        InstallInfoTable(binder, runtime);

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

    /// <summary>
    /// What <c>CS.Nes3D.Info.Version</c> reads (AMD-063 REQ-LUA-118, which amends AMD-040
    /// REQ-LUA-112): a string whose leading decimal integer is at least 2, which is how a 2.0
    /// script knows it may read <c>Camera.ResetDistance</c>.
    /// </summary>
    internal const string InfoVersion = "2.1";

    /// <summary>
    /// What <c>CS.Nes3D.Info.Title</c> reads (REQ-LUA-112): the product name of AMD-029, which
    /// a script compares against <c>"3dSenMaker"</c> — the profile editor — to pick a FOV.
    /// </summary>
    internal const string InfoTitle = "Zaxnes";

    /// <summary>
    /// What <c>CS.Nes3D.CameraControl.DistanceMax</c> reads (AMD-059 REQ-LUA-117): the upper
    /// bound the camera setpoint distance has in REQ-SCN-011. It is a constant of the table, not
    /// the live bound — <c>Camera.MaxDistance</c> is the one that follows the scene scale.
    /// </summary>
    internal const double CameraDistanceMax = 512d;

    /// <summary>
    /// What <c>CS.Nes3D.SettingManager.DefaultMsaa</c> reads (REQ-LUA-117): the default of
    /// <c>Setting.Msaa</c>, which REQ-LUA-116 fixes at 2.
    /// </summary>
    internal const double DefaultMsaa = 2d;

    /// <summary>
    /// Installs the global <c>CS</c> of AMD-040 REQ-LUA-112, completed by AMD-059 REQ-LUA-117
    /// and AMD-063 REQ-LUA-118 and REQ-LUA-119: <c>CS.Nes3D</c> carries four tables —
    /// <c>Info</c> (<c>Version</c>, <c>Title</c>), <c>CameraControl</c> (<c>DistanceMax</c> and
    /// the read-write <c>RotOverride</c>), <c>SettingManager</c> (<c>DefaultMsaa</c>) and
    /// <c>VisualStyle</c> (<c>Classic</c>, <c>SoftPixelLight</c>).
    /// </summary>
    /// <remarks>
    /// Each level is a shell whose fields live in a table only its metatable can reach. Reading
    /// a field the requirement does not name therefore gives nil without an error, and a write
    /// anywhere in these tables is dropped — it cannot reach the program, and it cannot
    /// disturb what the next script reads either. The metatable is hidden, so a script cannot
    /// take it off.
    /// <para>
    /// <c>RotOverride</c> is the single exception AMD-063 REQ-LUA-119 makes: it is the only
    /// field of <c>CS.Nes3D.CameraControl</c> whose write has an effect. Its leaf therefore
    /// carries a pair of bound functions rather than a plain table, and the rest of
    /// REQ-LUA-117 — every other field nil, every other write dropped — is enforced inside
    /// them.
    /// </para>
    /// </remarks>
    private static void InstallInfoTable(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        Lua state = runtime.State;
        ScriptCamera camera = runtime.Environment.Camera;

        // The four leaves of CS.Nes3D, pushed in the order Group names them.
        Shell(binder, state, s =>
        {
            s.PushString("Version");
            s.PushString(InfoVersion);
            s.RawSet(-3);
            s.PushString("Title");
            s.PushString(InfoTitle);
            s.RawSet(-3);
        });

        LiveShell(
            binder,
            state,
            (b, l) =>
            {
                // __index(t, k): the key is at index 2.
                string key = l.Type(2) == LuaType.String ? l.ToString(2) ?? string.Empty : string.Empty;
                switch (key)
                {
                    case "DistanceMax":
                        return Number(l, CameraDistanceMax);

                    // A copy, never a bound vector: writing through it would be a third way of
                    // steering the camera, and REQ-LUA-119 asks for « nil or a Vector3 copy ».
                    case "RotOverride" when camera.RotOverride is { } forced:
                        b.PushVector3(l, forced);
                        return 1;

                    default:
                        l.PushNil();
                        return 1;
                }
            },
            (b, l) =>
            {
                // __newindex(t, k, v): anything but RotOverride is dropped (REQ-LUA-117).
                if (l.Type(2) != LuaType.String || l.ToString(2) != "RotOverride")
                {
                    return 0;
                }

                if (l.IsNoneOrNil(3))
                {
                    camera.RotOverride = null;
                    return 0;
                }

                if (!b.TryGetVector3(l, 3, out Vec3 value))
                {
                    return b.InvalidArguments(l, "RotOverride");
                }

                camera.RotOverride = value;
                return 0;
            });

        Shell(binder, state, static s => Field(s, "DefaultMsaa", DefaultMsaa));

        // REQ-LUA-117: the two style identifiers as numbers, so that they compare with
        // Setting.VisualStyle and serve as arguments of the style-parameter methods. 4 is
        // Enhanced, which the original program calls « soft pixel light » (AMD-055 REQ-SCN-078).
        Shell(binder, state, static s =>
        {
            Field(s, "Classic", SettingsCatalogue.ClassicStyle);
            Field(s, "SoftPixelLight", SettingsCatalogue.EnhancedStyle);
        });

        Group(binder, state, ["Info", "CameraControl", "SettingManager", "VisualStyle"]);
        Wrap(binder, state, "Nes3D");
        state.SetGlobal("CS");
        return;

        // Leaves a table whose reads and writes are the two bound functions, with the metatable
        // guard of REQ-LUA-112 on it. Unlike Shell, 'read' is called for every key, so the
        // table can answer a live value, and 'write' decides itself what it accepts.
        static void LiveShell(LuaBinder binder, Lua state, LuaInvoke read, LuaInvoke write)
        {
            state.NewTable();
            state.NewTable();
            state.PushString("__index");
            binder.PushBoundFunction(state, read);
            state.RawSet(-3);
            state.PushString("__newindex");
            binder.PushBoundFunction(state, write);
            state.RawSet(-3);
            state.PushString("__metatable");
            state.PushString("CS");
            state.RawSet(-3);
            state.SetMetaTable(-2);
        }

        // Leaves a sealed table whose fields are the ones 'fill' writes into the table on top
        // of the stack.
        static void Shell(LuaBinder binder, Lua state, Action<Lua> fill)
        {
            state.NewTable();
            state.NewTable();
            state.PushString("__index");
            state.NewTable();
            fill(state);
            state.RawSet(-3);
            Seal(binder, state);
        }

        // One numeric field of the table on top of the stack.
        static void Field(Lua state, string name, double value)
        {
            state.PushString(name);
            state.PushNumber(value);
            state.RawSet(-3);
        }

        // Replaces the 'names.Length' values on top of the stack by one sealed table whose
        // fields name them, in order.
        static void Group(LuaBinder binder, Lua state, string[] names)
        {
            int first = state.GetTop() - names.Length + 1;

            state.NewTable();
            state.NewTable();
            state.PushString("__index");
            state.NewTable();
            for (int i = 0; i < names.Length; i++)
            {
                state.PushString(names[i]);
                state.PushCopy(first + i);
                state.RawSet(-3);
            }

            state.RawSet(-3);
            Seal(binder, state);

            // The shell is on top; the leaves below it have served their purpose.
            for (int i = 0; i < names.Length; i++)
            {
                state.Remove(first);
            }
        }

        // Replaces the value on top of the stack by a sealed table whose single field 'name'
        // holds it.
        static void Wrap(LuaBinder binder, Lua state, string name)
        {
            state.NewTable();
            state.NewTable();
            state.PushString("__index");
            state.NewTable();
            state.PushString(name);
            state.PushCopy(-6);
            state.RawSet(-3);
            state.RawSet(-3);
            Seal(binder, state);
            state.Remove(-2);
        }

        // Stack: the shell, then its metatable with __index already set. Adds the no-op
        // __newindex and the metatable guard, then attaches it.
        static void Seal(LuaBinder binder, Lua state)
        {
            state.PushString("__newindex");
            binder.PushBoundFunction(state, static (_, _) => 0);
            state.RawSet(-3);
            state.PushString("__metatable");
            state.PushString("CS");
            state.RawSet(-3);
            state.SetMetaTable(-2);
        }
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
