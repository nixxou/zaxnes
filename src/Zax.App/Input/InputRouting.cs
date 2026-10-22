using Zax.Nes;

namespace Zax.App.Input;

/// <summary>
/// The combination rules of SPEC-08 §3.1, §3.9 and §3.11: how the two players' action masks
/// become the input word the core receives, the camera actions and the hotkey mask.
/// </summary>
/// <remarks>
/// Nothing here knows about a device: it is the portable half of the input layer REQ-INP-120
/// asks for, and the rules can be exercised by injecting masks.
/// </remarks>
public static class InputRouting
{
    /// <summary>
    /// The input word of REQ-INP-004: port 1 in the low byte, port 2 in the next one. A port of
    /// type <c>Controller</c> carries the eight low bits of its player's mask; a port of type
    /// <c>Zapper</c> carries <paramref name="zapperByte"/> and no controller button at all.
    /// </summary>
    /// <param name="playerOne">Action mask of J1, every source combined (REQ-INP-002).</param>
    /// <param name="playerTwo">Action mask of J2.</param>
    /// <param name="portOne">Device type of port 1.</param>
    /// <param name="portTwo">Device type of port 2.</param>
    /// <param name="zapperByte">The Zapper byte of §4.1 (<see cref="ZapperState.PortByte"/>).</param>
    public static ushort CoreWord(
        InputAction playerOne,
        InputAction playerTwo,
        ControllerPortType portOne = ControllerPortType.Controller,
        ControllerPortType portTwo = ControllerPortType.Controller,
        byte zapperByte = ZapperState.ReleasedByte)
    {
        byte low = portOne == ControllerPortType.Zapper
            ? zapperByte
            : InputActions.ControllerByte(playerOne);
        byte high = portTwo == ControllerPortType.Zapper
            ? zapperByte
            : InputActions.ControllerByte(playerTwo);

        return (ushort)(low | (high << 8));
    }

    /// <summary>
    /// The mask the camera is driven by (REQ-INP-080): the OR of the masks of the players whose
    /// port is of type <c>Controller</c>. A player whose port is in <c>Zapper</c> does not drive
    /// the camera with their actions.
    /// </summary>
    public static InputAction CameraSources(
        InputAction playerOne,
        InputAction playerTwo,
        ControllerPortType portOne = ControllerPortType.Controller,
        ControllerPortType portTwo = ControllerPortType.Controller)
    {
        InputAction mask = InputAction.None;

        if (portOne != ControllerPortType.Zapper)
        {
            mask |= playerOne;
        }

        if (portTwo != ControllerPortType.Zapper)
        {
            mask |= playerTwo;
        }

        return mask & InputAction.CameraMask;
    }

    /// <summary>
    /// Whether the Zapper is active (REQ-INP-061): a game is running and at least one port is of
    /// type <c>Zapper</c>.
    /// </summary>
    public static bool ZapperActive(bool gameRunning, ControllerPortType portOne, ControllerPortType portTwo)
        => gameRunning
            && (portOne == ControllerPortType.Zapper || portTwo == ControllerPortType.Zapper);

    /// <summary>
    /// Merges the script pad into a player's mask (REQ-INP-071, REQ-INP-072): the pad is
    /// additive and can never take a bit away.
    /// </summary>
    /// <param name="playerOne">Physical mask of J1.</param>
    /// <param name="playerTwo">Physical mask of J2.</param>
    /// <param name="padMask">What <c>InputProcess</c> returned.</param>
    /// <param name="padIndex">The player the pad is attached to, 0 or 1 (SPEC-05 REQ-LUA-091).</param>
    public static (InputAction PlayerOne, InputAction PlayerTwo) MergeScriptPad(
        InputAction playerOne,
        InputAction playerTwo,
        InputAction padMask,
        int padIndex)
        => padIndex == 0
            ? (playerOne | padMask, playerTwo)
            : (playerOne, playerTwo | padMask);
}
