using Silk.NET.Input;
using Zax.Platform.Input;

namespace Zax.App.Input;

/// <summary>
/// The default binding tables of SPEC-08 §4.2 and §4.3 (REQ-INP-011, REQ-INP-020).
/// </summary>
/// <remarks>
/// The keyboard table names <b>physical positions</b>, not characters (DEC-08-5): the entry for
/// <c>B</c> is the key at the QWERTY <c>Z</c> position, which is the <c>W</c> key on an AZERTY
/// keyboard. Silk.NET's <see cref="Key"/> values are GLFW key codes, which are exactly that
/// numbering.
/// </remarks>
public static class DefaultBindings
{
    /// <summary>The default keyboard set of player 1 (SPEC-08 §4.2).</summary>
    public static IReadOnlyList<(InputAction Action, Key Key)> PlayerOneKeyboard { get; } =
    [
        (InputAction.Left, Key.Left),
        (InputAction.Right, Key.Right),
        (InputAction.Up, Key.Up),
        (InputAction.Down, Key.Down),
        (InputAction.A, Key.X),
        (InputAction.B, Key.Z),
        (InputAction.Select, Key.Space),
        (InputAction.Start, Key.Enter),
        (InputAction.RotLeft, Key.F5),
        (InputAction.RotRight, Key.F6),
        (InputAction.RotUp, Key.F7),
        (InputAction.RotDown, Key.F8),
        (InputAction.ZoomIn, Key.F9),
        (InputAction.ZoomOut, Key.F10),
        (InputAction.Menu, Key.Escape),
        (InputAction.RollBack, Key.Comma),
        (InputAction.FastForward, Key.Period),

        // QuickSave and QuickLoad have no default key: the combinations of REQ-INP-104 are the
        // only default route to the quick slot (SPEC-08 §4.2, §4.3).
    ];

    /// <summary>
    /// The default gamepad set of §4.3, the Xbox-style layout applied to every recognised pad
    /// (REQ-INP-020).
    /// </summary>
    /// <remarks>
    /// The left stick, the left stick click, both bumpers and the Guide button are deliberately
    /// unbound: the hat alone drives the NES directions (REQ-INP-032).
    /// </remarks>
    public static IReadOnlyList<(InputAction Action, PhysicalControl Control)> StandardGamepadSet { get; } =
    [
        (InputAction.Left, PhysicalControl.Button(StandardGamepad.DPadLeft)),
        (InputAction.Right, PhysicalControl.Button(StandardGamepad.DPadRight)),
        (InputAction.Up, PhysicalControl.Button(StandardGamepad.DPadUp)),
        (InputAction.Down, PhysicalControl.Button(StandardGamepad.DPadDown)),
        (InputAction.A, PhysicalControl.Button(StandardGamepad.South)),
        (InputAction.B, PhysicalControl.Button(StandardGamepad.West)),
        (InputAction.Select, PhysicalControl.Button(StandardGamepad.Back)),
        (InputAction.Start, PhysicalControl.Button(StandardGamepad.Start)),
        (InputAction.RollBack, PhysicalControl.Button(StandardGamepad.North)),
        (InputAction.FastForward, PhysicalControl.Button(StandardGamepad.East)),
        (InputAction.RotLeft, PhysicalControl.AxisNegative(StandardGamepad.RightStickX)),
        (InputAction.RotRight, PhysicalControl.AxisPositive(StandardGamepad.RightStickX)),
        (InputAction.RotUp, PhysicalControl.AxisPositive(StandardGamepad.RightStickY)),
        (InputAction.RotDown, PhysicalControl.AxisNegative(StandardGamepad.RightStickY)),
        (InputAction.ZoomIn, PhysicalControl.Trigger(StandardGamepad.LeftTrigger)),
        (InputAction.ZoomOut, PhysicalControl.Trigger(StandardGamepad.RightTrigger)),
        (InputAction.Menu, PhysicalControl.Button(StandardGamepad.RightStick)),
    ];

    /// <summary>A fresh keyboard set holding the table of §4.2.</summary>
    public static BindingSet CreatePlayerOneKeyboard()
    {
        BindingSet set = new(BindingDeviceKind.Keyboard);
        foreach ((InputAction action, Key key) in PlayerOneKeyboard)
        {
            set.Set(action, PhysicalControl.Key((int)key));
        }

        return set;
    }

    /// <summary>A fresh, empty keyboard set: the default of player 2 (REQ-INP-011).</summary>
    public static BindingSet CreatePlayerTwoKeyboard() => new(BindingDeviceKind.Keyboard);

    /// <summary>The default keyboard set of <paramref name="player"/> (0 = J1, 1 = J2).</summary>
    public static BindingSet CreateKeyboard(int player)
        => player == 0 ? CreatePlayerOneKeyboard() : CreatePlayerTwoKeyboard();

    /// <summary>
    /// A fresh gamepad set: the table of §4.3 for a recognised pad, empty for a pad with no
    /// standard mapping, which has no useful defaults (REQ-INP-021).
    /// </summary>
    public static BindingSet CreateGamepad(bool standard = true)
    {
        BindingSet set = new(BindingDeviceKind.Gamepad);
        if (!standard)
        {
            return set;
        }

        foreach ((InputAction action, PhysicalControl control) in StandardGamepadSet)
        {
            set.Set(action, control);
        }

        return set;
    }
}
