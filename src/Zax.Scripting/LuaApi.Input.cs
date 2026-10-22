using KeraLua;
using Zax.Engine3D;

namespace Zax.Scripting;

/// <summary>
/// The globals <c>ScriptPad</c> and <c>Gamepad</c>, the player objects and the VR state of
/// SPEC-05 §4.2 (REQ-LUA-091, REQ-LUA-092).
/// </summary>
internal static partial class LuaApi
{
    private static void RegisterInput(LuaBinder binder, ProfileScriptRuntime runtime)
    {
        ScriptInput input = runtime.Environment.Input;

        HostType pad = binder.DeclareType(HostTypeId.ScriptPadApi, "ScriptPadAPI");
        pad.Function("Enable", (b, l) =>
        {
            RequirePad(b, l, "Enable");
            runtime.ScriptPadEnabled = LuaBinder.CheckBoolean(l, 2);
            return 0;
        });

        pad.Property(
            "PadIndex",
            (_, l) => Number(l, runtime.ScriptPadIndex),
            (b, l) => runtime.ScriptPadIndex = Math.Clamp(AssignedInteger(b, l, "PadIndex"), 0, 1));

        pad.Property("State", (_, l) => Number(l, runtime.ScriptPadState));

        pad.Function("Pressed", (b, l) =>
        {
            RequirePad(b, l, "Pressed");
            int mask = b.CheckInteger(l, 2, "Pressed");
            return Boolean(l, (runtime.ScriptPadState & mask) != 0);
        });

        // No VR on desktop: the hand and head data are zero vectors (REQ-LUA-091).
        pad.Function("HandPos", (b, l) =>
        {
            RequirePad(b, l, "HandPos");
            b.PushVector3(l, default);
            return 1;
        }, HostTypeId.Vector3);
        pad.Function("RotPos", (b, l) =>
        {
            RequirePad(b, l, "RotPos");
            b.PushVector3(l, default);
            return 1;
        }, HostTypeId.Vector3);
        pad.Property(
            "LeftHand",
            (b, l) =>
            {
                b.PushXrState(l, default);
                return 1;
            },
            result: HostTypeId.XrState);
        pad.Alias("LH", "LeftHand");
        pad.Property(
            "RightHand",
            (b, l) =>
            {
                b.PushXrState(l, default);
                return 1;
            },
            result: HostTypeId.XrState);
        pad.Alias("RH", "RightHand");
        pad.Property(
            "Head",
            (b, l) =>
            {
                b.PushXrState(l, default);
                return 1;
            },
            result: HostTypeId.XrState);

        // Gamepad (§4.2, type ManettesAPI).
        HostType pads = binder.DeclareType(HostTypeId.GamepadApi, "ManettesAPI");
        pads.Property("State", (_, l) => Number(l, input.GlobalState));
        pads.Function("Pressed", (b, l) =>
        {
            RequirePads(b, l, "Pressed");
            int mask = b.CheckInteger(l, 2, "Pressed");
            return Boolean(l, (input.GlobalState & mask) != 0);
        });
        pads.Function("GetInput", (b, l) =>
        {
            RequirePads(b, l, "GetInput");
            int port = b.CheckInteger(l, 2, "GetInput");
            b.CheckBounds(l, port, 2, "GetInput");
            return Number(l, input.PortByte(port));
        });
        pads.ElementType = HostTypeId.Player;
        pads.Indexer = (b, l) =>
        {
            int index = b.CheckInteger(l, 2, "Gamepad");
            b.CheckBounds(l, index, 2, "Gamepad");
            b.PushDescriptor(l, HostTypeId.Player, input, index);
            return 1;
        };

        // One player (§4.2, type Joueur).
        HostType player = binder.DeclareType(HostTypeId.Player, "Joueur");
        player.Property("State", (b, l) => Number(l, input.PlayerState(b.MemberAt(l, 1))));
        player.Function("Pressed", (b, l) =>
        {
            if (b.TypeIdAt(l, 1) != HostTypeId.Player)
            {
                return b.InvalidArguments(l, "Pressed");
            }

            int mask = b.CheckInteger(l, 2, "Pressed");
            return Boolean(l, (input.PlayerState(b.MemberAt(l, 1)) & mask) != 0);
        });

        // XRState (§4.2): a value type whose four vectors are zero on desktop.
        HostType state = binder.DeclareType(HostTypeId.XrState, "XRState", reference: false);
        XrVector(state, "pos", 0);
        XrVector(state, "vel", 1);
        XrVector(state, "acc", 2);
        XrVector(state, "rot", 3);
        return;

        static void XrVector(HostType host, string name, int slot)
            => host.Property(
                name,
                (b, l) =>
                {
                    b.TryGetXrState(l, 1, out LuaXrState value);
                    b.PushVector3(l, slot switch
                    {
                        0 => value.Position,
                        1 => value.Velocity,
                        2 => value.Acceleration,
                        _ => value.Rotation,
                    });
                    return 1;
                },
                static (_, _) =>
                {
                    // XRState is a copy (§4.3): writing a field changes only that copy, and
                    // desktop has no VR data to change anyway (REQ-LUA-091).
                },
                HostTypeId.Vector3);

        static void RequirePad(LuaBinder binder, Lua state, string member)
        {
            if (binder.TypeIdAt(state, 1) != HostTypeId.ScriptPadApi)
            {
                binder.InvalidArguments(state, member);
            }
        }

        static void RequirePads(LuaBinder binder, Lua state, string member)
        {
            if (binder.TypeIdAt(state, 1) != HostTypeId.GamepadApi)
            {
                binder.InvalidArguments(state, member);
            }
        }
    }
}
