namespace Zax.App.Input;

/// <summary>
/// What a binding points at: one of the 19 logical actions of SPEC-08 §4.1, or one of the menu
/// commands of the command model (AMD-010 REQ-UI-203).
/// </summary>
/// <remarks>
/// REQ-UI-203 asks the key configuration screen to bind "the actions of SPEC-08 <b>and</b> the
/// menu commands", under the capture rules of SPEC-08. A single target type keeps one set of
/// rules — one binding per target per device type, no conflict detection, the same capture — for
/// both families.
/// </remarks>
public readonly record struct BindingTarget
{
    private BindingTarget(InputAction action, string? command)
    {
        Action = action;
        Command = command;
    }

    /// <summary>The logical action, or <c>None</c> when the target is a command.</summary>
    public InputAction Action { get; }

    /// <summary>The command name (<c>ZaxCommands</c>), or null when the target is an action.</summary>
    public string? Command { get; }

    /// <summary>Whether the target is one of the 19 logical actions.</summary>
    public bool IsAction => Command is null && Action != InputAction.None;

    /// <summary>Whether the target is a menu command.</summary>
    public bool IsCommand => Command is not null;

    /// <summary>Whether the target points at nothing.</summary>
    public bool IsEmpty => Command is null && Action == InputAction.None;

    /// <summary>The target of <paramref name="action"/>.</summary>
    public static BindingTarget Of(InputAction action) => new(action, null);

    /// <summary>The target of the command named <paramref name="command"/>.</summary>
    public static BindingTarget OfCommand(string command)
    {
        ArgumentException.ThrowIfNullOrEmpty(command);
        return new BindingTarget(InputAction.None, command);
    }

    /// <summary>
    /// The persisted key of this target: the §4.1 name of an action, or <c>cmd:&lt;name&gt;</c>
    /// for a command (SPEC-09 REQ-PERS-065).
    /// </summary>
    public string Key => Command is { } command ? $"cmd:{command}" : InputActions.NameOf(Action);

    /// <summary>Reads back what <see cref="Key"/> wrote; an unknown key yields an empty target.</summary>
    public static BindingTarget Parse(string? key)
    {
        if (string.IsNullOrWhiteSpace(key))
        {
            return default;
        }

        if (key.StartsWith("cmd:", StringComparison.OrdinalIgnoreCase))
        {
            string command = key[4..];
            return command.Length == 0 ? default : OfCommand(command);
        }

        return InputActions.Parse(key) is { } action ? Of(action) : default;
    }

    /// <inheritdoc/>
    public override string ToString() => Key;
}

/// <summary>
/// The two kinds of binding set a player has: one keyboard set and one gamepad set
/// (SPEC-08 §2, REQ-INP-043).
/// </summary>
public enum BindingDeviceKind
{
    /// <summary>The keyboard set, shared by the physical keyboard of both players (REQ-INP-010).</summary>
    Keyboard = 0,

    /// <summary>The gamepad set, which belongs to a device identity rather than a player (REQ-INP-025).</summary>
    Gamepad = 1,
}
