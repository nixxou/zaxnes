namespace Zax.App.Loop;

/// <summary>
/// Arguments of a command invocation (AMD-010 REQ-UI-205). A command takes at most a number, a
/// second number and a string, which covers slots, speeds, regions, ports and paths.
/// </summary>
/// <param name="Number">First numeric argument: slot, speed setting, region, port, percent.</param>
/// <param name="Text">String argument: a ROM path, a profile path, a game identifier.</param>
/// <param name="Number2">Second numeric argument, used by <see cref="ZaxCommands.SetPortType"/>.</param>
public readonly record struct CommandArgs(int Number = 0, string? Text = null, int Number2 = 0)
{
    /// <summary>Arguments carrying a single number.</summary>
    public static CommandArgs Of(int number) => new(number);

    /// <summary>Arguments carrying a single string.</summary>
    public static CommandArgs Of(string text) => new(0, text);
}

/// <summary>
/// One command of the model: a stable name, a label, the default shortcut shown next to it, and
/// the three things a menu needs — invoke, "is it available", "is it ticked".
/// </summary>
public sealed class ZaxCommand
{
    private readonly Action<CommandArgs> _execute;
    private readonly Func<CommandArgs, bool>? _canExecute;
    private readonly Func<bool>? _isChecked;

    internal ZaxCommand(
        string name,
        string label,
        string shortcut,
        Action<CommandArgs> execute,
        Func<CommandArgs, bool>? canExecute,
        Func<bool>? isChecked)
    {
        Name = name;
        Label = label;
        Shortcut = shortcut;
        _execute = execute;
        _canExecute = canExecute;
        _isChecked = isChecked;
    }

    /// <summary>The stable name (<see cref="ZaxCommands"/>).</summary>
    public string Name { get; }

    /// <summary>Label a menu shows.</summary>
    public string Label { get; }

    /// <summary>Default keyboard shortcut, as AMD-010 REQ-UI-204 lists it; may be empty.</summary>
    public string Shortcut { get; }

    /// <summary>Whether the command is a toggle, so that a menu can draw a tick.</summary>
    public bool IsToggle => _isChecked is not null;

    /// <summary>Whether the command can run right now with <paramref name="args"/>.</summary>
    public bool CanExecute(CommandArgs args = default) => _canExecute?.Invoke(args) ?? true;

    /// <summary>Whether the toggle is on; false for a command that is not a toggle.</summary>
    public bool IsChecked => _isChecked?.Invoke() ?? false;

    /// <summary>Runs the command, whatever <see cref="CanExecute"/> says.</summary>
    public void Execute(CommandArgs args = default) => _execute(args);
}

/// <summary>
/// The command model of AMD-010 REQ-UI-205: every action the UI can trigger, under a stable
/// name, invocable programmatically and bindable to a key or a gamepad button later.
/// </summary>
/// <remarks>
/// The model is deliberately independent of the UI. The round-1 tests drive the emulation
/// through it, the menu bar of lot L9 reads <see cref="Commands"/> to build itself, and the
/// keyboard layer of SPEC-08 maps hotkeys onto the same names.
/// </remarks>
public sealed class CommandModel
{
    private readonly Dictionary<string, ZaxCommand> _commands = [];
    private readonly List<ZaxCommand> _order = [];

    /// <summary>Every registered command, in registration order.</summary>
    public IReadOnlyList<ZaxCommand> Commands => _order;

    /// <summary>Registers a command.</summary>
    /// <param name="name">A stable name from <see cref="ZaxCommands"/>.</param>
    /// <param name="label">Label for a menu.</param>
    /// <param name="execute">What the command does.</param>
    /// <param name="shortcut">Default shortcut text, or empty.</param>
    /// <param name="canExecute">Availability test, or null for "always".</param>
    /// <param name="isChecked">Tick state, or null when the command is not a toggle.</param>
    public ZaxCommand Register(
        string name,
        string label,
        Action<CommandArgs> execute,
        string shortcut = "",
        Func<CommandArgs, bool>? canExecute = null,
        Func<bool>? isChecked = null)
    {
        ArgumentException.ThrowIfNullOrEmpty(name);
        ArgumentNullException.ThrowIfNull(execute);

        if (_commands.ContainsKey(name))
        {
            throw new InvalidOperationException($"The command '{name}' is already registered.");
        }

        ZaxCommand command = new(name, label, shortcut, execute, canExecute, isChecked);
        _commands.Add(name, command);
        _order.Add(command);
        return command;
    }

    /// <summary>The command of that name, or null.</summary>
    public ZaxCommand? Find(string name) => _commands.GetValueOrDefault(name);

    /// <summary>Whether a command of that name exists.</summary>
    public bool Contains(string name) => _commands.ContainsKey(name);

    /// <summary>
    /// Invokes a command by name. Returns false when the name is unknown or the command is not
    /// available, which is what a menu entry drawn greyed out does.
    /// </summary>
    public bool Invoke(string name, CommandArgs args = default)
    {
        if (_commands.GetValueOrDefault(name) is not { } command || !command.CanExecute(args))
        {
            return false;
        }

        command.Execute(args);
        return true;
    }
}
