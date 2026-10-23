using System.Collections;
using System.Reflection;
using Zax.App.Loop;
using Zax.Profiles.Library;
using Zax.Render;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>One row of the menu bar of AMD-010 REQ-UI-200, flattened for the tests.</summary>
/// <param name="Menu">Title of the menu the row belongs to.</param>
/// <param name="Label">Label of the row.</param>
/// <param name="Command">Command the row invokes, or null for a rule or a submenu.</param>
/// <param name="Shortcut">Default shortcut text shown on the right.</param>
/// <param name="Kind">Command, Toggle, Choice, Submenu or Separator.</param>
/// <param name="Depth">0 for a row of the menu itself, 1 and more inside a submenu.</param>
public readonly record struct UiMenuRow(
    string Menu,
    string Label,
    string? Command,
    string Shortcut,
    string Kind,
    int Depth);

/// <summary>One default shortcut of AMD-010 REQ-UI-204.</summary>
/// <param name="Key">The physical key.</param>
/// <param name="Modifiers">The modifiers that must be held.</param>
/// <param name="Command">The command it invokes.</param>
/// <param name="Number">Its numeric argument, for the slot shortcuts.</param>
/// <param name="Text">The text the menu shows.</param>
public readonly record struct UiShortcut(
    Silk.NET.Input.Key Key,
    string Modifiers,
    string Command,
    int Number,
    string Text);

/// <summary>
/// Drives the interface models of AMD-010 through reflection.
/// </summary>
/// <remarks>
/// <c>MenuBarModel</c>, <c>OptionsModel</c>, <c>KeyConfigModel</c>, <c>MenuShortcuts</c>,
/// <c>UiState</c>, <c>UiText</c>, <c>FileBrowserModel</c> and <c>ProfileChooserModel</c> are all
/// <b>internal</b> to <c>Zax.App</c>, whose internals are visible only to the implementers' own
/// test assembly. The verifier must not touch anything under <c>src/</c>, so the models are
/// driven over the shipped assembly: no production file is modified and the code exercised is the
/// one the application runs. REQ-UI-205 is what makes this possible — every control goes through
/// the command model.
/// </remarks>
public sealed class UiDriver
{
    private static readonly Assembly App = typeof(Zax.App.AppOptions).Assembly;

    private readonly object _menuBar;

    private UiDriver(object menuBar)
    {
        _menuBar = menuBar;
    }

    /// <summary>Builds the menu bar over a driven host.</summary>
    public static UiDriver MenuBar(HostDriver host, IReadOnlyList<string>? recentRoms = null)
    {
        ArgumentNullException.ThrowIfNull(host);

        Type type = Type("Zax.App.Ui.MenuBarModel");
        MethodInfo build = type.GetMethod("Build", BindingFlags.Public | BindingFlags.Static)!;

        Func<IReadOnlyList<string>> roms = () => recentRoms ?? [];
        object model = build.Invoke(null, [host.HostObject, roms, host.Scene.Camera])!;
        return new UiDriver(model);
    }

    /// <summary>The five menu titles, in the order of REQ-UI-200.</summary>
    public IReadOnlyList<string> MenuTitles
    {
        get
        {
            List<string> titles = [];
            foreach (object menu in (IEnumerable)Get(_menuBar, "Menus")!)
            {
                titles.Add((string)Get(menu, "Title")!);
            }

            return titles;
        }
    }

    /// <summary>Every row of every menu, submenus expanded.</summary>
    public IReadOnlyList<UiMenuRow> Rows()
    {
        List<UiMenuRow> rows = [];
        foreach (object menu in (IEnumerable)Get(_menuBar, "Menus")!)
        {
            string title = (string)Get(menu, "Title")!;
            Walk(title, (IEnumerable)Get(menu, "Items")!, 0, rows);
        }

        return rows;
    }

    /// <summary>Whether the row of <paramref name="command"/> can be activated right now.</summary>
    public bool IsEnabled(string command, CommandArgs args = default)
    {
        foreach (object menu in (IEnumerable)Get(_menuBar, "Menus")!)
        {
            if (FindItem((IEnumerable)Get(menu, "Items")!, command, args) is { } item)
            {
                return (bool)_menuBar.GetType().GetMethod("IsEnabled")!.Invoke(_menuBar, [item])!;
            }
        }

        throw new InvalidOperationException($"no menu row invokes '{command}'");
    }

    /// <summary>Whether the row of <paramref name="command"/> shows a tick.</summary>
    public bool IsChecked(string command, CommandArgs args = default)
    {
        foreach (object menu in (IEnumerable)Get(_menuBar, "Menus")!)
        {
            if (FindItem((IEnumerable)Get(menu, "Items")!, command, args) is { } item)
            {
                return (bool)_menuBar.GetType().GetMethod("IsChecked")!.Invoke(_menuBar, [item])!;
            }
        }

        throw new InvalidOperationException($"no menu row invokes '{command}'");
    }

    // ------------------------------------------------------------------ shortcuts

    /// <summary>The default shortcut table of REQ-UI-204.</summary>
    public static IReadOnlyList<UiShortcut> Shortcuts()
    {
        List<UiShortcut> rows = [];
        Type type = Type("Zax.App.Ui.MenuShortcuts");
        foreach (object shortcut in (IEnumerable)type
            .GetProperty("All", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!)
        {
            CommandArgs args = (CommandArgs)Get(shortcut, "Args")!;
            rows.Add(new UiShortcut(
                (Silk.NET.Input.Key)Get(shortcut, "Key")!,
                Get(shortcut, "Modifiers")!.ToString()!,
                (string)Get(shortcut, "Command")!,
                args.Number,
                (string)Get(shortcut, "Text")!));
        }

        return rows;
    }

    // -------------------------------------------------------------------- options

    /// <summary>The six tab titles of the options window (REQ-UI-202).</summary>
    public static IReadOnlyList<string> OptionTabs()
    {
        Type type = Type("Zax.App.Ui.OptionsModel");
        MethodInfo title = type.GetMethod("TitleOf", BindingFlags.Public | BindingFlags.Static)!;

        List<string> titles = [];
        foreach (object tab in (IEnumerable)type
            .GetProperty("Tabs", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!)
        {
            titles.Add((string)title.Invoke(null, [tab])!);
        }

        return titles;
    }

    /// <summary>The settings the options window shows, with the tab and the widget bounds.</summary>
    public static IReadOnlyList<(string Tab, string Key, string Label, double Minimum, double Maximum)> OptionRows()
    {
        Type type = Type("Zax.App.Ui.OptionsModel");
        object model = Activator.CreateInstance(type)!;
        MethodInfo title = type.GetMethod("TitleOf", BindingFlags.Public | BindingFlags.Static)!;
        MethodInfo rowsOf = type.GetMethod("Rows")!;

        List<(string, string, string, double, double)> rows = [];
        foreach (object tab in (IEnumerable)type
            .GetProperty("Tabs", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!)
        {
            string tabTitle = (string)title.Invoke(null, [tab])!;
            foreach (object row in (IEnumerable)rowsOf.Invoke(model, [tab])!)
            {
                SettingDescriptor descriptor = (SettingDescriptor)Get(row, "Descriptor")!;
                rows.Add((
                    tabTitle,
                    descriptor.Key,
                    (string)Get(row, "Label")!,
                    (double)Get(row, "Minimum")!,
                    (double)Get(row, "Maximum")!));
            }
        }

        return rows;
    }

    /// <summary>Puts every row of a tab back to its default (the button of REQ-UI-202).</summary>
    public static void ResetOptionTab(SettingsModel settings, string tabTitle)
    {
        Type type = Type("Zax.App.Ui.OptionsModel");
        object model = Activator.CreateInstance(type)!;
        MethodInfo title = type.GetMethod("TitleOf", BindingFlags.Public | BindingFlags.Static)!;

        foreach (object tab in (IEnumerable)type
            .GetProperty("Tabs", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!)
        {
            if (string.Equals((string)title.Invoke(null, [tab])!, tabTitle, StringComparison.Ordinal))
            {
                type.GetMethod("ResetTab")!.Invoke(model, [settings, tab]);
                return;
            }
        }

        throw new InvalidOperationException($"no options tab is titled '{tabTitle}'");
    }

    /// <summary>
    /// The setting keys the options window changes <b>without</b> a sound (AMD-021 R2-013).
    /// </summary>
    public static IReadOnlyList<string> SilentOptionKeys()
        => [.. (IEnumerable<string>)Type("Zax.App.Ui.OptionsModel")
            .GetProperty("SilentKeys", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!];

    /// <summary>
    /// Whether changing the value of <paramref name="key"/> in the options window plays the
    /// validation sound of REQ-AUD-041 (AMD-021 R2-013).
    /// </summary>
    public static bool PlaysValidationSound(string key)
        => (bool)Type("Zax.App.Ui.OptionsModel")
            .GetMethod("PlaysValidationSound", BindingFlags.Public | BindingFlags.Static)!
            .Invoke(null, [key])!;

    // -------------------------------------------------------------- key configuration

    /// <summary>The sections and lines of the key configuration window (REQ-UI-203).</summary>
    public static IReadOnlyList<(string Section, string Label, string Target, int Player)> KeyConfigRows(
        HostDriver host)
    {
        ArgumentNullException.ThrowIfNull(host);

        Type type = Type("Zax.App.Ui.KeyConfigModel");
        object model = Activator.CreateInstance(type, host.Commands)!;

        List<(string, string, string, int)> rows = [];
        foreach (object section in (IEnumerable)Get(model, "Sections")!)
        {
            string title = (string)Get(section, "Title")!;
            foreach (object row in (IEnumerable)Get(section, "Rows")!)
            {
                object target = Get(row, "Target")!;
                rows.Add((
                    title,
                    (string)Get(row, "Label")!,
                    (string)Get(target, "Key")!,
                    (int)Get(row, "Player")!));
            }
        }

        return rows;
    }

    // ------------------------------------------------------------------ interface state

    /// <summary>A fresh interface state (REQ-UI-207, REQ-UI-209).</summary>
    public static object NewState() => Activator.CreateInstance(Type("Zax.App.Ui.UiState"))!;

    /// <summary>Reads a flag of the interface state.</summary>
    public static bool StateFlag(object state, string name) => (bool)Get(state, name)!;

    /// <summary>Writes a flag of the interface state.</summary>
    public static void SetStateFlag(object state, string name, bool value)
        => state.GetType().GetProperty(name)!.SetValue(state, value);

    /// <summary>Calls a method of the interface state.</summary>
    public static void StateCall(object state, string name)
        => state.GetType().GetMethod(name)!.Invoke(state, []);

    // --------------------------------------------------------------------- texts

    /// <summary>A constant or static property of <c>UiText</c>.</summary>
    public static string Text(string name)
    {
        Type type = Type("Zax.App.Ui.UiText");
        object? value = type.GetField(name, BindingFlags.Public | BindingFlags.Static)?.GetValue(null)
            ?? type.GetProperty(name, BindingFlags.Public | BindingFlags.Static)?.GetValue(null);
        return value?.ToString() ?? throw new InvalidOperationException($"UiText has no '{name}'");
    }

    /// <summary>
    /// The text of the key <paramref name="key"/> in the language in force (AMD-035 §3). A test
    /// that has to compare a label reads the expected one from here instead of spelling it out,
    /// so that it checks the function and not the language the interface happens to be in.
    /// </summary>
    public static string Label(string key)
        => (string)Type("Zax.App.Ui.UiStrings")
            .GetMethod("Get", BindingFlags.Public | BindingFlags.Static)!
            .Invoke(null, [key])!;

    /// <summary>The label of the menu row that invokes <paramref name="command"/> (REQ-UI-205).</summary>
    public static string CommandLabel(string command)
        => Label((string)Type("Zax.App.Ui.UiKeys")
            .GetMethod("Command", BindingFlags.Public | BindingFlags.Static)!
            .Invoke(null, [command])!);

    /// <summary>The lines of the about block (REQ-UI-076, DEC-10-Q3).</summary>
    public static IReadOnlyList<string> AboutLines()
    {
        object value = Type("Zax.App.Ui.UiText")
            .GetProperty("AboutLines", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!;

        return [.. ((IEnumerable)value).Cast<string>()];
    }

    // -------------------------------------------------------------- file browser

    /// <summary>A file browser opened on <paramref name="directory"/> with the ROM filters.</summary>
    /// <param name="title">Title of the dialog.</param>
    /// <param name="directory">The folder it opens on.</param>
    /// <param name="profileFilters">Whether to use the <c>.3dn</c> filters.</param>
    /// <param name="threeDSen">
    /// Whether to open it the way the 3dSen import does: the bundle filters and a folder as an
    /// acceptable answer (AMD-030 §2).
    /// </param>
    public static object OpenBrowser(
        string title,
        string? directory,
        bool profileFilters = false,
        bool threeDSen = false)
    {
        Type type = Type("Zax.App.Ui.FileBrowserModel");
        object model = Activator.CreateInstance(type)!;
        object filters = type
            .GetProperty(
                threeDSen ? "ThreeDSenFilters" : profileFilters ? "ProfileFilters" : "RomFilters",
                BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!;

        // Every parameter is given: Open has optional ones and reflection does not fill them in.
        type.GetMethod("Open")!.Invoke(model, [title, filters, directory, threeDSen, null]);
        return model;
    }

    /// <summary>The names the browser lists, folders first.</summary>
    public static IReadOnlyList<(string Name, string Kind)> BrowserEntries(object browser)
    {
        List<(string, string)> entries = [];
        foreach (object entry in (IEnumerable)Get(browser, "Entries")!)
        {
            entries.Add(((string)Get(entry, "Name")!, Get(entry, "Kind")!.ToString()!));
        }

        return entries;
    }

    /// <summary>Whether the browser's current filter accepts a file name.</summary>
    public static bool BrowserAccepts(object browser, string name)
        => (bool)browser.GetType().GetMethod("Accepts")!.Invoke(browser, [name])!;

    /// <summary>Sets the browser's search text (REQ-UI-103).</summary>
    public static void BrowserSearch(object browser, string text)
        => browser.GetType().GetProperty("Search")!.SetValue(browser, text);

    /// <summary>
    /// Validates the dialog and returns what it answers, or null when it refuses to answer
    /// (REQ-UI-104; AMD-030 §2 for the folder answer).
    /// </summary>
    public static string? BrowserConfirm(object browser)
        => (bool)browser.GetType().GetMethod("Confirm")!.Invoke(browser, null)!
            ? (string?)Get(browser, "Result")
            : null;

    /// <summary>Whether the dialog would answer something (AMD-030 §2).</summary>
    public static bool BrowserCanConfirm(object browser) => (bool)Get(browser, "CanConfirm")!;

    /// <summary>The browser's path trail (REQ-UI-101).</summary>
    public static IReadOnlyList<string> BrowserTrail(object browser)
    {
        List<string> trail = [];
        foreach (object step in (IEnumerable)Get(browser, "Trail")!)
        {
            // The trail is a list of value tuples: its label is Item1.
            trail.Add((string)step.GetType().GetField("Item1")!.GetValue(step)!);
        }

        return trail;
    }

    // ------------------------------------------------------------ profile chooser

    /// <summary>The profile chooser of REQ-UI-201, built over a library.</summary>
    public static object OpenChooser(GameLibrary library, string? romPath = null)
    {
        Type type = Type("Zax.App.Ui.ProfileChooserModel");
        object model = Activator.CreateInstance(type, library)!;
        type.GetMethod("Reset")!.Invoke(model, [romPath]);
        return model;
    }

    /// <summary>The titles the chooser offers, in order.</summary>
    public static IReadOnlyList<string> ChooserTitles(object chooser, bool filtered = false)
    {
        List<string> titles = [];
        foreach (object choice in (IEnumerable)Get(chooser, filtered ? "Filtered" : "All")!)
        {
            titles.Add((string)Get(choice, "Title")!);
        }

        return titles;
    }

    /// <summary>Sets the chooser's search text.</summary>
    public static void ChooserSearch(object chooser, string text)
        => chooser.GetType().GetProperty("Search")!.SetValue(chooser, text);

    // ------------------------------------------------------------------- helpers

    private static void Walk(string menu, IEnumerable items, int depth, List<UiMenuRow> rows)
    {
        foreach (object item in items)
        {
            string kind = Get(item, "Kind")!.ToString()!;
            rows.Add(new UiMenuRow(
                menu,
                (string)Get(item, "Label")!,
                (string?)Get(item, "Command"),
                (string)Get(item, "Shortcut")!,
                kind,
                depth));

            if (Get(item, "Children") is Delegate children)
            {
                Walk(menu, (IEnumerable)children.DynamicInvoke()!, depth + 1, rows);
            }
        }
    }

    private static object? FindItem(IEnumerable items, string command, CommandArgs args)
    {
        foreach (object item in items)
        {
            if ((string?)Get(item, "Command") == command && (CommandArgs)Get(item, "Args")! == args)
            {
                return item;
            }

            if (Get(item, "Children") is Delegate children
                && FindItem((IEnumerable)children.DynamicInvoke()!, command, args) is { } found)
            {
                return found;
            }
        }

        return null;
    }

    private static object? Get(object target, string name)
        => target.GetType().GetProperty(
            name,
            BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)?.GetValue(target);

    private static Type Type(string name) => App.GetType(name, true)!;
}
