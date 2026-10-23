using Silk.NET.Input;
using Zax.App.Loop;
using Zax.App.Persistence;

namespace Zax.App.Ui;

/// <summary>The modifier keys a menu shortcut may ask for (AMD-010 REQ-UI-204).</summary>
[Flags]
internal enum ShortcutModifiers
{
    /// <summary>No modifier is held.</summary>
    None = 0,

    /// <summary>Either Control key.</summary>
    Control = 1,

    /// <summary>Either Shift key.</summary>
    Shift = 2,

    /// <summary>Either Alt key.</summary>
    Alt = 4,
}

/// <summary>
/// One default shortcut: a key, the modifiers that must be held, and the command it invokes.
/// </summary>
/// <param name="Key">The physical key, in the US layout positions Silk.NET reports.</param>
/// <param name="Modifiers">The modifiers that must be held, and no others.</param>
/// <param name="Command">The command name of <see cref="ZaxCommands"/>.</param>
/// <param name="Args">The arguments the shortcut passes, for the slot shortcuts.</param>
/// <param name="Text">The text the menu shows next to the entry.</param>
internal readonly record struct MenuShortcut(
    Key Key,
    ShortcutModifiers Modifiers,
    string Command,
    CommandArgs Args,
    string Text);

/// <summary>
/// The default keyboard shortcuts of AMD-010 REQ-UI-204 as data, so that the host, the key
/// configuration window and the tests all read the same table.
/// </summary>
/// <remarks>
/// <para>
/// The slot shortcuts use <c>Ctrl+Fn</c> to load instead of the bare <c>Fn</c> the amendment's
/// table writes, because <c>F5</c> to <c>F10</c> are the camera keys of SPEC-08 §4.2 and the same
/// requirement asks for no conflict with the default game keys — see
/// <c>spec/changes/Q-030.md</c>, answered by AMD-013.
/// </para>
/// <para>
/// A shortcut matches only when the modifiers held are <b>exactly</b> the ones it asks for, which
/// is what keeps <c>P</c> from firing under <c>Ctrl+P</c> and keeps the table free of conflicts.
/// The four hotkeys the amendment leaves to SPEC-08 — fast forward, rollback, quick save and
/// quick load — are not here: they are logical actions with their own bindings (REQ-INP-105), and
/// the key configuration window is where they are changed.
/// </para>
/// </remarks>
// Q-030
internal static class MenuShortcuts
{
    /// <summary>The function keys of the ten exposed slots (AMD-010 REQ-UI-208).</summary>
    internal static readonly Key[] SlotKeys =
    [
        Key.F1, Key.F2, Key.F3, Key.F4, Key.F5,
        Key.F6, Key.F7, Key.F8, Key.F9, Key.F10,
    ];

    private static IReadOnlyList<MenuShortcut> _all = Build();
    private static int _builtFor = UiStrings.Generation;

    /// <summary>
    /// Every default shortcut, in the order of the table of REQ-UI-204. Two of the texts are
    /// words rather than key names — « Alt+Entrée », « Maj+Fn » — so the table is rebuilt when
    /// the interface language changes (AMD-035).
    /// </summary>
    public static IReadOnlyList<MenuShortcut> All
    {
        get
        {
            if (_builtFor != UiStrings.Generation)
            {
                _builtFor = UiStrings.Generation;
                _all = Build();
            }

            return _all;
        }
    }

    /// <summary>
    /// The keys that act with no modifier at all. REQ-INP-106 lets an implementation add fixed
    /// shortcuts provided they use no key of the default table of §4.2, which is what this list
    /// has to keep true.
    /// </summary>
    public static IReadOnlyList<Key> BareKeys { get; } =
        [.. All.Where(static s => s.Modifiers == ShortcutModifiers.None).Select(static s => s.Key).Distinct()];

    /// <summary>Every key any shortcut uses, which is the set the host watches for edges.</summary>
    public static IReadOnlyList<Key> WatchedKeys { get; } =
        [.. All.Select(static s => s.Key).Distinct()];

    /// <summary>The text of the shortcut bound to <paramref name="command"/>, or an empty string.</summary>
    public static string TextOf(string command)
    {
        foreach (MenuShortcut shortcut in All)
        {
            if (string.Equals(shortcut.Command, command, StringComparison.Ordinal))
            {
                return shortcut.Text;
            }
        }

        return string.Empty;
    }

    /// <summary>
    /// The commands the keys that have just been pressed invoke, given the modifiers currently
    /// held. A key used by two shortcuts with different modifiers yields at most one of them.
    /// </summary>
    /// <param name="pressed">The keys whose rising edge was seen in this frame.</param>
    /// <param name="modifiers">The modifiers held right now.</param>
    /// <param name="commands">Receives the (command, arguments) pairs, in table order.</param>
    public static void Resolve(
        IReadOnlyCollection<Key> pressed,
        ShortcutModifiers modifiers,
        ICollection<(string Command, CommandArgs Args)> commands)
    {
        ArgumentNullException.ThrowIfNull(pressed);
        ArgumentNullException.ThrowIfNull(commands);

        if (pressed.Count == 0)
        {
            return;
        }

        foreach (MenuShortcut shortcut in All)
        {
            if (shortcut.Modifiers == modifiers && pressed.Contains(shortcut.Key))
            {
                commands.Add((shortcut.Command, shortcut.Args));
            }
        }
    }

    /// <summary>The modifier set of a keyboard state.</summary>
    public static ShortcutModifiers ModifiersOf(bool control, bool shift, bool alt)
        => (control ? ShortcutModifiers.Control : ShortcutModifiers.None)
            | (shift ? ShortcutModifiers.Shift : ShortcutModifiers.None)
            | (alt ? ShortcutModifiers.Alt : ShortcutModifiers.None);

    private static MenuShortcut[] Build()
    {
        List<MenuShortcut> list =
        [
            new(Key.O, ShortcutModifiers.Control, ZaxCommands.OpenRom, default, "Ctrl+O"),
            new(Key.P, ShortcutModifiers.None, ZaxCommands.PauseResume, default, "P"),
            new(Key.R, ShortcutModifiers.Control, ZaxCommands.Reset, default, "Ctrl+R"),
            new(Key.F11, ShortcutModifiers.None, ZaxCommands.ToggleTwoDimensional, default, "F11"),
            new(Key.Enter, ShortcutModifiers.Alt, ZaxCommands.ToggleFullscreen, default,
                UiStrings.Get(UiKeys.ShortcutAltEnter)),
            new(Key.Comma, ShortcutModifiers.Control, ZaxCommands.ShowOptions, default, "Ctrl+,"),
            new(Key.Q, ShortcutModifiers.Control, ZaxCommands.Quit, default, "Ctrl+Q"),

            // The debug overlay of DEC-10-Q7's neighbourhood: an addition, off by default, on a
            // key no default game binding uses.
            new(Key.F12, ShortcutModifiers.None, UiCommands.ToggleDebugOverlay, default, "F12"),
        ];

        for (int slot = 1; slot <= SaveSlotIndex.ExposedSlotCount; slot++)
        {
            Key key = SlotKeys[slot - 1];
            string suffix = slot.ToString(System.Globalization.CultureInfo.InvariantCulture);

            // AMD-013 (Q-030): Ctrl+Fn loads, Shift (« Maj ») + Fn saves.
            list.Add(new MenuShortcut(
                key,
                ShortcutModifiers.Control,
                ZaxCommands.LoadSlot,
                CommandArgs.Of(slot),
                $"Ctrl+F{suffix}"));
            list.Add(new MenuShortcut(
                key,
                ShortcutModifiers.Shift,
                ZaxCommands.SaveSlot,
                CommandArgs.Of(slot),
                $"{UiStrings.Get(UiKeys.ShortcutShiftPrefix)}+F{suffix}"));
        }

        return [.. list];
    }
}
