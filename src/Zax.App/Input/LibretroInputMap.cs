namespace Zax.App.Input;

/// <summary>
/// The RetroPad buttons of the libretro input API, under the numbers libretro gives them.
/// </summary>
/// <remarks>
/// The values are those of <c>RETRO_DEVICE_ID_JOYPAD_*</c>. They are listed here as data so that
/// the default correspondence of SPEC-08 §4.4 can be written, checked and reviewed now; the
/// libretro DLL itself is target P7 (REQ-INP-122).
/// </remarks>
public enum RetroPadButton
{
    /// <summary>B, the south face button.</summary>
    B = 0,

    /// <summary>Y, the west face button.</summary>
    Y = 1,

    /// <summary>Select.</summary>
    Select = 2,

    /// <summary>Start.</summary>
    Start = 3,

    /// <summary>D-pad up.</summary>
    Up = 4,

    /// <summary>D-pad down.</summary>
    Down = 5,

    /// <summary>D-pad left.</summary>
    Left = 6,

    /// <summary>D-pad right.</summary>
    Right = 7,

    /// <summary>A, the east face button.</summary>
    A = 8,

    /// <summary>X, the north face button.</summary>
    X = 9,

    /// <summary>L shoulder.</summary>
    L = 10,

    /// <summary>R shoulder.</summary>
    R = 11,

    /// <summary>L2 trigger.</summary>
    L2 = 12,

    /// <summary>R2 trigger.</summary>
    R2 = 13,

    /// <summary>L3, left stick click.</summary>
    L3 = 14,

    /// <summary>R3, right stick click.</summary>
    R3 = 15,
}

/// <summary>Which analog stick half-axis a mapping entry refers to.</summary>
public enum RetroAnalogAxis
{
    /// <summary>Right stick, towards the left.</summary>
    RightStickLeft,

    /// <summary>Right stick, towards the right.</summary>
    RightStickRight,

    /// <summary>Right stick, upwards.</summary>
    RightStickUp,

    /// <summary>Right stick, downwards.</summary>
    RightStickDown,
}

/// <summary>
/// The default libretro correspondence of SPEC-08 §4.4 (REQ-INP-122), as data only.
/// </summary>
/// <remarks>
/// <para>
/// Port N drives player N. The convention <c>A = east</c> / <c>B = south</c> is the usual one of
/// the libretro NES cores and differs on purpose from the desktop layout of §4.3, where A is the
/// south button.
/// </para>
/// <para>
/// The system hotkeys — menu, rollback, fast forward, quick save — are deliberately absent: the
/// frontend provides its own menu, rewind, fast forward and save states, so they are not exposed
/// as buttons (REQ-INP-122). The optical gun or the pointer of port N drives the Zapper when
/// that port is in <c>Zapper</c>, through the same ray cast as REQ-INP-063 (REQ-INP-123).
/// </para>
/// </remarks>
public static class LibretroInputMap
{
    /// <summary>The digital part of the table of §4.4: RetroPad button to logical action.</summary>
    public static IReadOnlyList<(RetroPadButton Button, InputAction Action)> Buttons { get; } =
    [
        (RetroPadButton.Left, InputAction.Left),
        (RetroPadButton.Right, InputAction.Right),
        (RetroPadButton.Up, InputAction.Up),
        (RetroPadButton.Down, InputAction.Down),
        (RetroPadButton.A, InputAction.A),
        (RetroPadButton.B, InputAction.B),
        (RetroPadButton.Select, InputAction.Select),
        (RetroPadButton.Start, InputAction.Start),
        (RetroPadButton.L2, InputAction.ZoomIn),
        (RetroPadButton.R2, InputAction.ZoomOut),
    ];

    /// <summary>
    /// The analog part of the table of §4.4: each right-stick half-axis drives one camera
    /// rotation, with the same 0.34 threshold as the desktop layer (REQ-INP-030).
    /// </summary>
    public static IReadOnlyList<(RetroAnalogAxis Axis, InputAction Action)> Analog { get; } =
    [
        (RetroAnalogAxis.RightStickLeft, InputAction.RotLeft),
        (RetroAnalogAxis.RightStickRight, InputAction.RotRight),
        (RetroAnalogAxis.RightStickUp, InputAction.RotUp),
        (RetroAnalogAxis.RightStickDown, InputAction.RotDown),
    ];

    /// <summary>
    /// The actions the frontend host keeps for itself and which are therefore not mapped
    /// (REQ-INP-122).
    /// </summary>
    public static InputAction HostReservedActions => InputAction.HotkeyMask;

    /// <summary>The action of <paramref name="button"/>, or <c>None</c>.</summary>
    public static InputAction ActionOf(RetroPadButton button)
    {
        foreach ((RetroPadButton candidate, InputAction action) in Buttons)
        {
            if (candidate == button)
            {
                return action;
            }
        }

        return InputAction.None;
    }

    /// <summary>The action of <paramref name="axis"/>, or <c>None</c>.</summary>
    public static InputAction ActionOf(RetroAnalogAxis axis)
    {
        foreach ((RetroAnalogAxis candidate, InputAction action) in Analog)
        {
            if (candidate == axis)
            {
                return action;
            }
        }

        return InputAction.None;
    }
}
