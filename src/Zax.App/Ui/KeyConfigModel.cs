using Zax.App.Input;
using Zax.App.Loop;

namespace Zax.App.Ui;

/// <summary>One line of the key configuration window.</summary>
/// <param name="Label">What the line is called.</param>
/// <param name="Target">The action or the menu command the line binds.</param>
/// <param name="Player">The player whose sets the line reads and writes, 0 = J1.</param>
internal sealed record KeyConfigRow(string Label, BindingTarget Target, int Player);

/// <summary>A group of lines with a heading.</summary>
/// <param name="Title">The heading, in the interface language.</param>
/// <param name="Rows">Its lines, in order.</param>
internal sealed record KeyConfigSection(string Title, IReadOnlyList<KeyConfigRow> Rows);

/// <summary>
/// The key configuration of AMD-010 REQ-UI-203 as a UI-independent model: the 19 logical actions
/// of SPEC-08 §4.1 for both players <b>and</b> the menu commands, each bindable to a keyboard key
/// and to a gamepad control under the capture rules of SPEC-08 (REQ-INP-042 to REQ-INP-047).
/// </summary>
/// <remarks>
/// <para>
/// SPEC-10 REQ-UI-070 drives a single column with a <c>Player</c> and a <c>Type</c> selector;
/// AMD-010 replaces that presentation (§1) and the window shows the two device columns side by
/// side instead, which is why <see cref="Begin"/> takes the device kind: clicking a cell is what
/// chooses the set being edited.
/// </para>
/// <para>
/// A menu command can only be bound <b>without arguments</b>: a binding points at a command name
/// (<see cref="BindingTarget"/>) and the host invokes it with the default arguments, so the
/// commands that need a slot number, a speed or a port are not offered. The acceptance criterion
/// of REQ-UI-203 — bind a menu command to a pad button, press it, the command runs — is about the
/// argument-less ones.
/// </para>
/// </remarks>
internal sealed class KeyConfigModel
{
    /// <summary>
    /// The commands that take an argument, and so cannot be bound. Everything else the command
    /// model holds is offered.
    /// </summary>
    public static IReadOnlySet<string> CommandsNeedingArguments { get; } = new HashSet<string>(StringComparer.Ordinal)
    {
        ZaxCommands.OpenRecentRom,
        ZaxCommands.LaunchGame,
        ZaxCommands.SaveSlot,
        ZaxCommands.LoadSlot,
        ZaxCommands.DeleteSlot,
        ZaxCommands.SetSpeed,
        ZaxCommands.SetRegion,
        ZaxCommands.SetPortType,
        ZaxCommands.SetCameraMode,
        ZaxCommands.SetSkybox,
        ZaxCommands.SetVolume,
    };

    private readonly CommandModel _commands;
    private readonly List<KeyConfigSection> _sections = [];

    private int _builtFor = -1;

    /// <summary>Builds the window over <paramref name="commands"/>.</summary>
    /// <param name="commands">The command model, whose argument-less commands become lines.</param>
    public KeyConfigModel(CommandModel commands)
    {
        ArgumentNullException.ThrowIfNull(commands);
        _commands = commands;
        Rebuild();
    }

    /// <summary>
    /// The sections, in display order. The headings and the command labels follow the interface
    /// language, so the window switches without a restart (AMD-035 §1).
    /// </summary>
    public IReadOnlyList<KeyConfigSection> Sections
    {
        get
        {
            Rebuild();
            return _sections;
        }
    }

    private void Rebuild()
    {
        if (_builtFor == UiStrings.Generation)
        {
            return;
        }

        _builtFor = UiStrings.Generation;
        _sections.Clear();

        _sections.Add(new KeyConfigSection(
            UiStrings.Get(UiKeys.KeysPlayerOne),
            [.. NesButtons.Select(action => Row(action, player: 0))]));

        _sections.Add(new KeyConfigSection(
            UiStrings.Get(UiKeys.KeysPlayerTwo),
            [.. NesButtons.Select(action => Row(action, player: 1))]));

        _sections.Add(new KeyConfigSection(
            UiStrings.Get(UiKeys.KeysCamera),
            [.. CameraActions.Select(action => Row(action, player: 0))]));

        _sections.Add(new KeyConfigSection(
            UiStrings.Get(UiKeys.KeysHotkeys),
            [.. Hotkeys.Select(action => Row(action, player: 0))]));

        List<KeyConfigRow> commandRows = [];
        foreach (ZaxCommand command in _commands.Commands)
        {
            if (CommandsNeedingArguments.Contains(command.Name))
            {
                continue;
            }

            commandRows.Add(new KeyConfigRow(
                command.Label,
                BindingTarget.OfCommand(command.Name),
                Player: 0));
        }

        _sections.Add(new KeyConfigSection(UiStrings.Get(UiKeys.KeysCommands), commandRows));
    }

    /// <summary>Every line of every section.</summary>
    public IEnumerable<KeyConfigRow> AllRows()
    {
        foreach (KeyConfigSection section in Sections)
        {
            foreach (KeyConfigRow row in section.Rows)
            {
                yield return row;
            }
        }
    }

    /// <summary>The eight NES buttons, in the order of SPEC-08 §4.1.</summary>
    public static IReadOnlyList<InputAction> NesButtons { get; } =
    [
        InputAction.Left, InputAction.Right, InputAction.Up, InputAction.Down,
        InputAction.A, InputAction.B, InputAction.Select, InputAction.Start,
    ];

    /// <summary>The six camera actions of §3.9.</summary>
    public static IReadOnlyList<InputAction> CameraActions { get; } =
    [
        InputAction.RotLeft, InputAction.RotRight, InputAction.RotUp, InputAction.RotDown,
        InputAction.ZoomIn, InputAction.ZoomOut,
    ];

    /// <summary>The five system hotkeys of §3.11.</summary>
    public static IReadOnlyList<InputAction> Hotkeys { get; } =
    [
        InputAction.Menu, InputAction.RollBack, InputAction.FastForward,
        InputAction.QuickSave, InputAction.QuickLoad,
    ];

    /// <summary>
    /// The set a cell reads: the player's keyboard set, or the set of the pad assigned to them,
    /// which is null when they have none (REQ-INP-046).
    /// </summary>
    public static BindingSet? SetOf(
        InputBindings bindings,
        GamepadAssignment? pads,
        int player,
        BindingDeviceKind kind)
    {
        ArgumentNullException.ThrowIfNull(bindings);
        return kind == BindingDeviceKind.Keyboard
            ? bindings.Keyboard(player)
            : pads?.GamepadBindings(player);
    }

    /// <summary>
    /// Starts the capture of one cell (REQ-INP-042): the shown player and device kind follow the
    /// cell, the existing binding is cleared at once, and the next control pressed becomes the
    /// binding. Returns false when there is nothing to edit.
    /// </summary>
    public static bool Begin(RebindingSession session, KeyConfigRow row, BindingDeviceKind kind)
    {
        ArgumentNullException.ThrowIfNull(session);
        ArgumentNullException.ThrowIfNull(row);

        session.SelectPlayer(row.Player);
        session.SelectDeviceKind(kind);
        return session.Begin(row.Target);
    }

    /// <summary>Whether <paramref name="session"/> is listening for this very cell.</summary>
    public static bool IsListeningFor(RebindingSession session, KeyConfigRow row, BindingDeviceKind kind)
    {
        ArgumentNullException.ThrowIfNull(session);
        ArgumentNullException.ThrowIfNull(row);
        return session.IsListening
            && session.Player == row.Player
            && session.DeviceKind == kind
            && session.Target == row.Target;
    }

    private static KeyConfigRow Row(InputAction action, int player)
        => new(InputActions.NameOf(action), BindingTarget.Of(action), player);
}
