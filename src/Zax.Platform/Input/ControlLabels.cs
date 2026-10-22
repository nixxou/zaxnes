using System.Globalization;
using Silk.NET.Input;

namespace Zax.Platform.Input;

/// <summary>
/// Readable labels for the physical commands a binding points at, which the key configuration
/// screen shows next to each action (DEC-08-5, SPEC-10; the screen itself is lot L9).
/// </summary>
/// <remarks>
/// Bindings are stored as physical positions, so the label is the one of the <b>US layout</b>
/// position unless the backend can tell us the character the active layout produces. That is the
/// "comprehensible label" DEC-08-5 asks SPEC-10 for, and it is why the default table of §4.2
/// documents <c>Z</c> as the AZERTY <c>W</c>.
/// </remarks>
public static class ControlLabels
{
    /// <summary>The label of <paramref name="control"/>, or "—" when it is unbound.</summary>
    public static string Of(PhysicalControl control) => control.Kind switch
    {
        PhysicalControlKind.Key => KeyLabel(control.Code),
        PhysicalControlKind.Button => ButtonLabel(control.Code),
        PhysicalControlKind.Axis => AxisLabel(control.Code, control.Direction),
        PhysicalControlKind.Trigger => control.Code == StandardGamepad.LeftTrigger
            ? "Left trigger"
            : "Right trigger",
        _ => "—",
    };

    private static string KeyLabel(int code)
    {
        Key key = (Key)code;
        return key switch
        {
            Key.Left => "Left arrow",
            Key.Right => "Right arrow",
            Key.Up => "Up arrow",
            Key.Down => "Down arrow",
            Key.Enter => "Enter",
            Key.KeypadEnter => "Keypad Enter",
            Key.Space => "Space",
            Key.Escape => "Esc",
            Key.Comma => ",",
            Key.Period => ".",
            Key.Minus => "-",
            Key.Equal => "=",
            Key.Semicolon => ";",
            Key.Apostrophe => "'",
            Key.Slash => "/",
            Key.BackSlash => "\\",
            Key.GraveAccent => "`",
            Key.LeftBracket => "[",
            Key.RightBracket => "]",
            Key.ShiftLeft => "Left Shift",
            Key.ShiftRight => "Right Shift",
            Key.ControlLeft => "Left Ctrl",
            Key.ControlRight => "Right Ctrl",
            Key.AltLeft => "Left Alt",
            Key.AltRight => "Right Alt",
            Key.SuperLeft => "Left Win",
            Key.SuperRight => "Right Win",
            Key.Backspace => "Backspace",
            Key.Tab => "Tab",
            Key.CapsLock => "Caps Lock",
            Key.PageUp => "Page Up",
            Key.PageDown => "Page Down",
            Key.Home => "Home",
            Key.End => "End",
            Key.Insert => "Insert",
            Key.Delete => "Delete",
            _ when Enum.IsDefined(key) => key.ToString(),
            _ => string.Create(CultureInfo.InvariantCulture, $"Key {code}"),
        };
    }

    private static string ButtonLabel(int code) => code switch
    {
        StandardGamepad.South => "South (A)",
        StandardGamepad.East => "East (B)",
        StandardGamepad.West => "West (X)",
        StandardGamepad.North => "North (Y)",
        StandardGamepad.LeftBumper => "Left bumper",
        StandardGamepad.RightBumper => "Right bumper",
        StandardGamepad.Back => "Back",
        StandardGamepad.Start => "Start",
        StandardGamepad.Guide => "Guide",
        StandardGamepad.LeftStick => "Left stick click",
        StandardGamepad.RightStick => "Right stick click",
        StandardGamepad.DPadUp => "D-pad up",
        StandardGamepad.DPadRight => "D-pad right",
        StandardGamepad.DPadDown => "D-pad down",
        StandardGamepad.DPadLeft => "D-pad left",
        _ => string.Create(CultureInfo.InvariantCulture, $"Button {code}"),
    };

    private static string AxisLabel(int axis, int direction) => axis switch
    {
        StandardGamepad.LeftStickX => direction < 0 ? "Left stick left" : "Left stick right",
        StandardGamepad.LeftStickY => direction < 0 ? "Left stick down" : "Left stick up",
        StandardGamepad.RightStickX => direction < 0 ? "Right stick left" : "Right stick right",
        StandardGamepad.RightStickY => direction < 0 ? "Right stick down" : "Right stick up",
        _ => string.Create(
            CultureInfo.InvariantCulture,
            $"Axis {axis}{(direction < 0 ? '-' : '+')}"),
    };
}
