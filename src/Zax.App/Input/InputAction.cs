namespace Zax.App.Input;

/// <summary>
/// The 19 logical actions of SPEC-08 §4.1, with the bit values the Lua scripts see
/// (REQ-INP-003).
/// </summary>
/// <remarks>
/// These values are an interoperability fact: the profiles' scripts read a player's state and
/// return a mask from <c>InputProcess</c>, so they must never change (REQ-INP-003, SPEC-05
/// REQ-LUA-032). The eight low bits are the NES controller byte in the order A, B, Select,
/// Start, Up, Down, Left, Right.
/// </remarks>
[Flags]
public enum InputAction
{
    /// <summary>Nothing pressed.</summary>
    None = 0,

    /// <summary>NES A.</summary>
    A = 0x01,

    /// <summary>NES B.</summary>
    B = 0x02,

    /// <summary>NES Select.</summary>
    Select = 0x04,

    /// <summary>NES Start.</summary>
    Start = 0x08,

    /// <summary>NES Up.</summary>
    Up = 0x10,

    /// <summary>NES Down.</summary>
    Down = 0x20,

    /// <summary>NES Left.</summary>
    Left = 0x40,

    /// <summary>NES Right.</summary>
    Right = 0x80,

    /// <summary>Camera: move closer.</summary>
    ZoomIn = 0x100,

    /// <summary>Camera: move away.</summary>
    ZoomOut = 0x200,

    /// <summary>Camera: rotate left.</summary>
    RotLeft = 0x400,

    /// <summary>Camera: rotate right.</summary>
    RotRight = 0x800,

    /// <summary>Camera: rotate up.</summary>
    RotUp = 0x1000,

    /// <summary>Camera: rotate down.</summary>
    RotDown = 0x2000,

    /// <summary>Pause / menu; a long press resets the view (REQ-INP-101).</summary>
    Menu = 0x4000,

    /// <summary>Roll back.</summary>
    RollBack = 0x8000,

    /// <summary>Fast forward.</summary>
    FastForward = 0x10000,

    /// <summary>Quick save.</summary>
    QuickSave = 0x20000,

    /// <summary>Quick load.</summary>
    QuickLoad = 0x40000,

    /// <summary>The eight low bits, which form the NES controller byte (SPEC-08 §4.1).</summary>
    ControllerByte = 0xFF,

    /// <summary>The six camera actions (§3.9).</summary>
    CameraMask = ZoomIn | ZoomOut | RotLeft | RotRight | RotUp | RotDown,

    /// <summary>The five system hotkey actions (§3.11).</summary>
    HotkeyMask = Menu | RollBack | FastForward | QuickSave | QuickLoad,
}

/// <summary>
/// The action table of SPEC-08 §4.1 as data: the 19 actions in the display order of the key
/// configuration screen, with the English labels that screen shows (REQ-INP-040).
/// </summary>
public static class InputActions
{
    /// <summary>How many logical actions each player has (REQ-INP-001).</summary>
    public const int Count = 19;

    /// <summary>How many logical players there are: J1 and J2, never a J3 (REQ-INP-001).</summary>
    public const int PlayerCount = 2;

    /// <summary>
    /// The actions in the order of the table of §4.1, which is also the display order of the key
    /// configuration screen (REQ-INP-040).
    /// </summary>
    public static IReadOnlyList<InputAction> All { get; } =
    [
        InputAction.Left,
        InputAction.Right,
        InputAction.Up,
        InputAction.Down,
        InputAction.A,
        InputAction.B,
        InputAction.Select,
        InputAction.Start,
        InputAction.RotLeft,
        InputAction.RotRight,
        InputAction.RotUp,
        InputAction.RotDown,
        InputAction.ZoomIn,
        InputAction.ZoomOut,
        InputAction.Menu,
        InputAction.RollBack,
        InputAction.FastForward,
        InputAction.QuickSave,
        InputAction.QuickLoad,
    ];

    /// <summary>The label of §4.1 for <paramref name="action"/>, the English name of the table.</summary>
    public static string NameOf(InputAction action) => action switch
    {
        InputAction.Left => "Left",
        InputAction.Right => "Right",
        InputAction.Up => "Up",
        InputAction.Down => "Down",
        InputAction.A => "A",
        InputAction.B => "B",
        InputAction.Select => "Select",
        InputAction.Start => "Start",
        InputAction.RotLeft => "RotLeft",
        InputAction.RotRight => "RotRight",
        InputAction.RotUp => "RotUp",
        InputAction.RotDown => "RotDown",
        InputAction.ZoomIn => "ZoomIn",
        InputAction.ZoomOut => "ZoomOut",
        InputAction.Menu => "Menu",
        InputAction.RollBack => "RollBack",
        InputAction.FastForward => "FastForward",
        InputAction.QuickSave => "QuickSave",
        InputAction.QuickLoad => "QuickLoad",
        _ => action.ToString(),
    };

    /// <summary>The action of that §4.1 name, or null.</summary>
    public static InputAction? Parse(string? name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            return null;
        }

        foreach (InputAction action in All)
        {
            if (string.Equals(NameOf(action), name, StringComparison.OrdinalIgnoreCase))
            {
                return action;
            }
        }

        return null;
    }

    /// <summary>
    /// The NES controller byte of <paramref name="mask"/>: its eight low bits. The camera and
    /// system bits above <c>0x80</c> never reach the core (REQ-INP-006).
    /// </summary>
    public static byte ControllerByte(InputAction mask) => (byte)((int)mask & (int)InputAction.ControllerByte);

    /// <summary>The camera actions of <paramref name="mask"/>, as the renderer's controls see them.</summary>
    public static Zax.Render.CameraAction ToCameraActions(InputAction mask)
    {
        Zax.Render.CameraAction actions = Zax.Render.CameraAction.None;

        if (mask.HasFlag(InputAction.RotLeft))
        {
            actions |= Zax.Render.CameraAction.RotateLeft;
        }

        if (mask.HasFlag(InputAction.RotRight))
        {
            actions |= Zax.Render.CameraAction.RotateRight;
        }

        if (mask.HasFlag(InputAction.RotUp))
        {
            actions |= Zax.Render.CameraAction.RotateUp;
        }

        if (mask.HasFlag(InputAction.RotDown))
        {
            actions |= Zax.Render.CameraAction.RotateDown;
        }

        if (mask.HasFlag(InputAction.ZoomIn))
        {
            actions |= Zax.Render.CameraAction.ZoomIn;
        }

        if (mask.HasFlag(InputAction.ZoomOut))
        {
            actions |= Zax.Render.CameraAction.ZoomOut;
        }

        return actions;
    }
}
