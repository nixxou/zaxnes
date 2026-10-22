using System.Globalization;

namespace Zax.Platform.Input;

/// <summary>
/// What kind of physical command a binding points at (SPEC-08 §2, "binding").
/// </summary>
public enum PhysicalControlKind
{
    /// <summary>No command: the action is unbound.</summary>
    None = 0,

    /// <summary>A keyboard key, identified by its physical position (DEC-08-5).</summary>
    Key = 1,

    /// <summary>A gamepad button, including the four hat directions.</summary>
    Button = 2,

    /// <summary>One half of a gamepad stick axis.</summary>
    Axis = 3,

    /// <summary>An analog trigger.</summary>
    Trigger = 4,
}

/// <summary>
/// One physical command: a key, a gamepad button, half of a stick axis or a trigger
/// (SPEC-08 §2).
/// </summary>
/// <remarks>
/// <para>
/// The descriptor carries no windowing or gamepad library type, which is what lets the binding
/// model of SPEC-08 §3.5 be serialised (SPEC-09 REQ-PERS-065) and lets a test drive a simulated
/// device. The numeric codes are those of <see cref="StandardGamepad"/> for a recognised pad and
/// the raw indices of the device for anything else (REQ-INP-021).
/// </para>
/// <para>
/// Key codes are the physical key positions the windowing layer reports; on the GLFW backend
/// those are the GLFW key codes, which is exactly the layout-independent numbering DEC-08-5
/// asks for.
/// </para>
/// </remarks>
/// <param name="Kind">Which family the command belongs to.</param>
/// <param name="Code">Key code, button index, axis index or trigger index.</param>
/// <param name="Direction">
/// Which half of an axis: +1 for the positive half, −1 for the negative half. Zero for the
/// other kinds.
/// </param>
public readonly record struct PhysicalControl(PhysicalControlKind Kind, int Code, int Direction = 0)
{
    /// <summary>The unbound command (REQ-INP-045: an action left empty).</summary>
    public static PhysicalControl None => default;

    /// <summary>Whether the descriptor points at a real command.</summary>
    public bool IsBound => Kind != PhysicalControlKind.None;

    /// <summary>Whether the command belongs to a keyboard.</summary>
    public bool IsKeyboard => Kind == PhysicalControlKind.Key;

    /// <summary>Whether the command belongs to a gamepad.</summary>
    public bool IsGamepad => Kind is PhysicalControlKind.Button
        or PhysicalControlKind.Axis
        or PhysicalControlKind.Trigger;

    /// <summary>The key at physical position <paramref name="code"/>.</summary>
    public static PhysicalControl Key(int code) => new(PhysicalControlKind.Key, code);

    /// <summary>Gamepad button <paramref name="code"/>.</summary>
    public static PhysicalControl Button(int code) => new(PhysicalControlKind.Button, code);

    /// <summary>The positive half of axis <paramref name="index"/>.</summary>
    public static PhysicalControl AxisPositive(int index) => new(PhysicalControlKind.Axis, index, 1);

    /// <summary>The negative half of axis <paramref name="index"/>.</summary>
    public static PhysicalControl AxisNegative(int index) => new(PhysicalControlKind.Axis, index, -1);

    /// <summary>Half <paramref name="direction"/> of axis <paramref name="index"/>.</summary>
    public static PhysicalControl Axis(int index, int direction)
        => new(PhysicalControlKind.Axis, index, direction >= 0 ? 1 : -1);

    /// <summary>Trigger <paramref name="index"/> (0 left, 1 right).</summary>
    public static PhysicalControl Trigger(int index) => new(PhysicalControlKind.Trigger, index);

    /// <summary>
    /// The persisted form: <c>key:256</c>, <c>button:0</c>, <c>axis:2+</c>, <c>trigger:1</c>, or
    /// the empty string when unbound (SPEC-09 REQ-PERS-065).
    /// </summary>
    public override string ToString() => Kind switch
    {
        PhysicalControlKind.Key => string.Create(CultureInfo.InvariantCulture, $"key:{Code}"),
        PhysicalControlKind.Button => string.Create(CultureInfo.InvariantCulture, $"button:{Code}"),
        PhysicalControlKind.Axis => string.Create(
            CultureInfo.InvariantCulture,
            $"axis:{Code}{(Direction < 0 ? '-' : '+')}"),
        PhysicalControlKind.Trigger => string.Create(CultureInfo.InvariantCulture, $"trigger:{Code}"),
        _ => string.Empty,
    };

    /// <summary>
    /// Reads back what <see cref="ToString"/> wrote. An unknown or malformed text yields
    /// <see cref="None"/>, which is how a corrupted entry degrades to "unbound" instead of
    /// failing the whole file (REQ-INP-052).
    /// </summary>
    public static PhysicalControl Parse(string? text)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return None;
        }

        ReadOnlySpan<char> span = text.AsSpan().Trim();
        int colon = span.IndexOf(':');
        if (colon <= 0 || colon == span.Length - 1)
        {
            return None;
        }

        ReadOnlySpan<char> kind = span[..colon];
        ReadOnlySpan<char> value = span[(colon + 1)..];

        int direction = 0;
        if (value[^1] is '+' or '-')
        {
            direction = value[^1] == '-' ? -1 : 1;
            value = value[..^1];
        }

        if (!int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int code) || code < 0)
        {
            return None;
        }

        if (kind.Equals("key", StringComparison.OrdinalIgnoreCase))
        {
            return Key(code);
        }

        if (kind.Equals("button", StringComparison.OrdinalIgnoreCase))
        {
            return Button(code);
        }

        if (kind.Equals("axis", StringComparison.OrdinalIgnoreCase))
        {
            // A half-axis without a sign is meaningless; treat it as unbound rather than
            // guessing a direction.
            return direction == 0 ? None : Axis(code, direction);
        }

        return kind.Equals("trigger", StringComparison.OrdinalIgnoreCase) ? Trigger(code) : None;
    }
}

/// <summary>
/// The control numbering of the standard Xbox-style layout SPEC-08 §4.3 is written against
/// (REQ-INP-020).
/// </summary>
/// <remarks>
/// A device the platform recognises reports its controls under these numbers, whatever the
/// underlying driver calls them, so the default gamepad table is one fixed set of descriptors.
/// A pad with no standard mapping reports raw indices instead and therefore has no useful
/// defaults (REQ-INP-021).
/// </remarks>
public static class StandardGamepad
{
    /// <summary>South face button (A on Xbox, Cross on PlayStation).</summary>
    public const int South = 0;

    /// <summary>East face button (B on Xbox, Circle).</summary>
    public const int East = 1;

    /// <summary>West face button (X on Xbox, Square).</summary>
    public const int West = 2;

    /// <summary>North face button (Y on Xbox, Triangle).</summary>
    public const int North = 3;

    /// <summary>Left bumper (LB / L1).</summary>
    public const int LeftBumper = 4;

    /// <summary>Right bumper (RB / R1).</summary>
    public const int RightBumper = 5;

    /// <summary>Back / View / Select / Share.</summary>
    public const int Back = 6;

    /// <summary>Start / Menu / Options.</summary>
    public const int Start = 7;

    /// <summary>Guide / Home / PS button.</summary>
    public const int Guide = 8;

    /// <summary>Left stick click (L3).</summary>
    public const int LeftStick = 9;

    /// <summary>Right stick click (R3).</summary>
    public const int RightStick = 10;

    /// <summary>Hat up.</summary>
    public const int DPadUp = 11;

    /// <summary>Hat right.</summary>
    public const int DPadRight = 12;

    /// <summary>Hat down.</summary>
    public const int DPadDown = 13;

    /// <summary>Hat left.</summary>
    public const int DPadLeft = 14;

    /// <summary>How many buttons the standard layout defines.</summary>
    public const int ButtonCount = 15;

    /// <summary>Left stick, horizontal axis; positive to the right.</summary>
    public const int LeftStickX = 0;

    /// <summary>Left stick, vertical axis; positive upwards.</summary>
    public const int LeftStickY = 1;

    /// <summary>Right stick, horizontal axis; positive to the right.</summary>
    public const int RightStickX = 2;

    /// <summary>Right stick, vertical axis; positive upwards.</summary>
    public const int RightStickY = 3;

    /// <summary>How many stick axes the standard layout defines.</summary>
    public const int AxisCount = 4;

    /// <summary>Left trigger (LT / L2).</summary>
    public const int LeftTrigger = 0;

    /// <summary>Right trigger (RT / R2).</summary>
    public const int RightTrigger = 1;

    /// <summary>How many triggers the standard layout defines.</summary>
    public const int TriggerCount = 2;
}
